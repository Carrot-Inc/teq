mod expr;
pub(crate) mod layout;
mod module;
mod names;
pub mod kept;
pub mod reach;
mod report;
mod runtime;
mod outline;
mod scope;
mod share;

use crate::ast::{mods, ListRef};
use crate::intern::{Interner, Name};
use crate::source::{FileId, SourceFile};
use crate::symbols::*;
use layout::is_eager_top_val;
use crate::tir::*;
use crate::types::*;
pub use names::is_js_identifier;
use names::{js_string, sanitize, Naming, Renames, NO_SCOPE};
use scope::{Renderable, Root, RootInfo, Scope};
use share::HoleKind;
use std::fmt::Write;
use std::rc::Rc;
use std::sync::atomic::{AtomicU8, AtomicUsize, Ordering};

pub const RUNTIME: &str = include_str!("../../runtime/rt.js");
/// React Fast Refresh for a development server, written unchanged into the split output
/// directory as `hot-refresh.mjs` under `--hot` (`module.rs`).
pub const HOT_REFRESH: &str = include_str!("../../runtime/hot-refresh.mjs");

/// What a split build needs to place a definition: the file it stands in tells the package, and
/// the packages under a `small` prefix get one module per file. `hot` adds what a development
/// server needs to re-execute a module in a page that has booted (`module.rs`).
#[derive(Clone, Copy)]
pub struct Split<'a> {
    pub files: &'a [SourceFile],
    pub file_pkgs: &'a [PkgId],
    pub module_per_file: &'a [String],
    pub hot: bool,
}

pub enum Output {
    Script(String),
    Modules(Modules),
}

/// The files of the output directory with their contents, `main.mjs` first. Under `--hot`
/// every text ends with a footer that names the text before it by its hash, which is not
/// known before the writer has compared the text with the last build's: `footers` has, per
/// file, the length of the text before the footer and the place in the text where the hash
/// goes, `HASH_LEN` characters that the writer fills in (`write::modules_after`), with the id
/// of the build `ID_AFTER_HASH` further on, and which then adds `hot-build.mjs`, the listing
/// of the hashes under that id (`module::hot_build`).
pub struct Modules {
    pub files: Vec<(String, String)>,
    pub footers: Vec<(usize, usize)>,
}

/// The length of a module's hash as the footers and `hot-build.mjs` spell it.
pub const HASH_LEN: usize = 16;
/// Where in a footer the id of the build that wrote the module follows its hash.
pub const ID_AFTER_HASH: usize = HASH_LEN + 4;
/// The listing of a `--hot` build's modules, the file that ends its publication.
pub const HOT_BUILD: &str = "hot-build.mjs";
pub use module::hot_build;

/// What `emit` is asked for besides the output.
#[derive(Clone, Copy, Default)]
pub struct Options<'a> {
    /// Short member names, no indentation or line breaks, a runtime without comments.
    pub release: bool,
    /// The bytes of the output per module, package and definition (`--size-report`), which
    /// names the files of the program.
    pub size_report: Option<&'a [SourceFile]>,
    /// Expansions of one shape written as one function and anonymous classes of one body as one
    /// class; off (`--no-outline`), each is written at its site.
    pub outline: bool,
}

/// What a watch session keeps from one build's emit for the next: the encodings of the
/// expansions and anonymous classes, which a rebuild takes again for the files it did not type.
#[derive(Default)]
pub struct Cache {
    classes: share::ClassCache,
    outline: outline::Cache,
}

impl Cache {
    /// The bytes the caches hold.
    pub fn held(&self) -> usize {
        self.classes.held() + self.outline.held()
    }
}

/// The bytes of one definition of the output, for the size report: a class is measured twice,
/// its body and its registration. The body of a class carries a hash of its text after the
/// header line, which tells anonymous classes of one shape apart.
pub(crate) struct Sized {
    def: Def,
    module: usize,
    bytes: usize,
    registration: bool,
    hash: u64,
}

/// A module-level binding of the output.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Def {
    /// A class, and the accessor of an object.
    Class(ClassId),
    /// A top-level def, the accessor (and setter) of a top-level val or var, a constant, or the
    /// value of an enum case.
    Sym(SymId),
    /// The initialiser `$file<tag>` of a file's vals.
    File(FileId),
    /// The function of expansions of one shape (`outline`).
    Outlined(u32),
}

/// The module-level bindings a piece of output refers to, by id as they are first named, and the
/// ones it defines: what a module of the split output imports and exports.
#[derive(Default)]
pub struct Bindings {
    pub classes: Vec<ClassId>,
    pub syms: Vec<SymId>,
    pub files: Vec<FileId>,
    pub imports: Vec<u32>,
    pub defs: Vec<Def>,
    pub outlined: Vec<u32>,
    /// Under `--hot`, the classes whose identity the output depends on, with the use that does.
    pub held: Vec<(ClassId, Held)>,
}

/// A use of a class that holds the output to the class as it is: a value made by the class of
/// before a hot swap is no value of the class the swap made. Every other use (a call, a
/// construction, a value read or passed on) takes what the swap made.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Held {
    /// A class of the output extends the class or mixes it in: it was linked to it once.
    Parent,
    /// A type test or a pattern over the class (`instanceof`).
    Tested,
    /// A comparison with the value of an enum case or with an object, in an expression or as
    /// a pattern.
    Compared,
}

/// Read-only view of the symbol table shared between the emitter's threads.
#[derive(Clone, Copy)]
struct SharedSymbols<'a>(&'a Symbols);

pub struct Emitter<'a> {
    prog: &'a Program,
    syms: &'a Symbols,
    interner: &'a Interner,
    out: String,
    indent: usize,
    sym_names: Vec<Option<Rc<str>>>,
    class_names: Vec<Option<Rc<str>>>,
    /// The qualified forms of the few names that are not bare everywhere.
    sym_quals: crate::intern::FxMap<SymId, Rc<str>>,
    class_quals: crate::intern::FxMap<ClassId, Rc<str>>,
    naming: &'a Naming,
    /// The scope of the module being emitted (`Naming`).
    scope: u32,
    /// The short names of the members under `--release`.
    renames: Option<&'a Renames>,
    /// `--release`: no indentation, no line breaks.
    release: bool,
    /// The chunk that last named each symbol and class: an emitter serves the chunks of one
    /// thread in turn, and each chunk records what it refers to once.
    sym_seen: Vec<u32>,
    class_seen: Vec<u32>,
    chunk: u32,
    simple_cache: &'a [AtomicU8],
    tmp_counter: u32,
    array_seq: Option<ClassId>,
    /// Top-level vals whose initializer is a literal are plain consts.
    const_vals: &'a [bool],
    /// Per file: whether it has top-level vals that its initialiser `$file<tag>()` runs together.
    file_inits: &'a [bool],
    /// Per class: whether its constructor runs statements or initialisers of the body.
    has_body: &'a [bool],
    /// Per class: the number its registration and type tests carry (`layout::Layout`).
    class_numbers: &'a [u32],
    anon_parent_args: &'a crate::intern::FxMap<ClassId, u32>,
    local_captures: &'a crate::intern::FxMap<ClassId, u32>,
    /// The anonymous classes written as arrow functions where they are created (`layout::Layout`).
    closure_anons: &'a crate::intern::FxMap<ClassId, usize>,
    /// The anonymous classes that one class of the same body stands for (`share`).
    shared: &'a share::Shared,
    /// The expansions written as calls of a shared function (`outline`).
    outline: &'a outline::Outline,
    /// The function of `outline` whose body is being written, from its root's tree.
    body: Option<u32>,
    /// What the program reaches from its entry point and exports; nothing else is written.
    reach: &'a reach::Reach,
    /// The locals that the anonymous class being emitted holds in fields.
    captures: Vec<SymId>,
    /// While the body of a lowered anonymous class is written as a closure: what the place
    /// that creates it passes for each local the body captures, innermost class last, other
    /// than a local passed as itself.
    closure_captures: Vec<(SymId, TExprId)>,
    /// The def whose body is being emitted as a loop, when it has self tail calls.
    tail: Option<TailLoop>,
    /// How many JS functions (lambdas, immediately invoked blocks) sit between the def being
    /// emitted and the code being written: a `return` at depth 0 is JavaScript's, deeper ones
    /// throw to the def's handler, whose key variable this names once one has been written.
    fn_depth: u32,
    nonlocal_return: Option<Rc<str>>,
    /// Inside such a loop with a tail call on another receiver, `this` reads the loop's copy.
    this_name: &'static str,
    /// The source names of the parameters and locals of the JS functions being emitted, innermost
    /// last. A further local of one of these names is renamed: declared in the same block it
    /// would redeclare the first, and in a nested block it would take the assignments meant for
    /// the outer one (`let x; { const x = ...; x = x; }`).
    scope_names: Vec<crate::intern::Name>,
    /// The locals named while the current definition is emitted; their names are its own.
    named_locals: Vec<SymId>,
    bindings: Bindings,
    /// The accessors of the objects the current chunk defines.
    accessors: Vec<String>,
    /// In the split output a class names its superclass when the modules have loaded (`$ext`).
    deferred_extends: bool,
    /// `--hot`: an object's accessor reports the object it constructs (`$hotObj`).
    hot: bool,
    /// The constructor parameters of the class whose `$body` is being emitted, which that method
    /// reads from their fields.
    body_params: Vec<SymId>,
    /// The constructor parameters without a field of the constructor being emitted, which it
    /// reads as its JS parameters.
    ctor_locals: Vec<SymId>,
    /// The arguments of the superclass constructor are being emitted: a capture is read as the
    /// constructor's parameter, since `this` does not exist yet.
    before_super: bool,
    /// The function bodies being emitted, innermost last (`Frame`).
    frames: Vec<Frame>,
    /// How many times `out` is swapped for a buffer that is spliced in later; positions taken
    /// while it is do not refer to `out`.
    swapped: u32,
    /// The file whose initialiser has run, or is running, where the code being written runs:
    /// inside the body of one of its top-level defs or its initialiser. An access of its
    /// definitions there needs no check (`Reach::triggers`).
    init_file: Option<FileId>,
    /// The check of its file's initialiser that the body about to be written starts with.
    entry_check: Option<String>,
    /// The access whose check was written as a statement in front of the statement holding it.
    checked: Option<TExprId>,
    /// The blocks and functions of the output open where the code being written stands,
    /// innermost last, each with the names declared in it (`scope.rs`).
    scopes: Vec<Scope>,
    scope_ids: u32,
    /// How many open scopes declare each name.
    open_names: crate::intern::FxMap<Rc<str>, u32>,
    /// The scope each local of the block being written is declared in, by the scope's id.
    decl_scope: crate::intern::FxMap<SymId, u32>,
    /// The walks of the roots of the current item's scopes.
    roots: crate::intern::FxMap<Root, Rc<RootInfo>>,
    /// The names given to the bindings of a case before the block that declares them opens, and
    /// those of them that are temporaries of the emitter.
    pending_names: crate::intern::FxMap<Root, (Vec<Rc<str>>, Vec<Rc<str>>)>,
    /// What the output can render as a bare identifier (`scope::Renderable`).
    renderable: &'a Renderable<'a>,
    /// The initialisers of the vals of the file being written (`Root::Vals`).
    group_inits: Vec<TExprId>,
    /// The scope of the def whose body is being written, where the key of its non-local returns
    /// is declared.
    def_scope: Option<usize>,
    /// The parameters of the constructor being written that its JavaScript parameter list names
    /// apart from their fields: a parameter named like a global the constructor reads.
    ctor_bindings: crate::intern::FxMap<SymId, Rc<str>>,
    /// A local is being named that the output does not declare (`Emitter::unbound_name`).
    unbound: bool,
    /// The targets of the assignments whose values are being written, innermost last.
    assign_targets: Vec<Rc<str>>,
    /// The texts of its own the constructor of the class being written writes (`CALLABLE`, ...).
    ctor_texts: Vec<&'static str>,
    /// A walk of a root is running (`scope::root_info`), which names nothing.
    walking: bool,
    /// The locals of the item by the names they go by (`Emitter::note_bound`).
    named_as: crate::intern::FxMap<Rc<str>, Vec<SymId>>,
}

/// A function body being emitted, recording where it calls the accessors of objects: an
/// accessor called often enough in a body for a binding to shorten it is bound once, `const
/// $$R = Registry$();`, before the statement of its first call, where that call is not inside a
/// construct that may not run (a lambda, a branch, a loop body, a default argument). The later
/// calls read the binding, nested lambdas included; the accessor is idempotent after its first
/// call, so the binding changes nothing about the order in which objects initialise. The binding
/// is named by the capitals of the object, numbered when the body binds two objects alike.
struct Frame {
    start: usize,
    /// The indentation of the statements of the body itself.
    top_indent: usize,
    /// Above zero inside a construct that may not run.
    cond: u32,
    /// Where the current statement of the body starts, and its indentation.
    mark: Option<(usize, usize)>,
    /// A body written on one line: a binding goes in front of the statement on that line.
    inline: bool,
    swapped: u32,
    uses: Vec<AccessorUse>,
    /// Replacements of a span, and insertions where the span is empty, ordered by the position
    /// of the use they stand for: two bindings in front of one statement follow its calls.
    edits: Vec<(usize, usize, usize, String)>,
    /// The spans of the frames nested in this one with the names each bound: a binding of this
    /// frame read inside one of them is named apart from its bindings, which would shadow it.
    nested: Vec<(usize, usize, String)>,
}

struct AccessorUse {
    class: ClassId,
    span: (usize, usize),
    form: UseForm,
    cond: bool,
    mark: Option<(usize, usize)>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum UseForm {
    /// `X$()`, which reads the binding instead.
    Call,
    /// `(X$(), ` in front of an expression, with the `)` behind it: an object touched for its
    /// body to run first. Bound, the touch goes.
    Touch(usize),
    /// A statement `X$();` on its own line, which goes as well.
    Stmt,
}

/// The texts a constructor writes of its own, which the walk of its scope reads the globals of
/// where the constructor writes them (`Emitter::ctor_texts`).
const CALLABLE: &str = "$callable($f, new.target.prototype);";
const WRITABLE_NAME: &str = " Object.defineProperty($f, \"name\", { writable: true });";
const SUPER_FIELDS: [&str; 3] = ["Object.assign(", ", Reflect.construct(Object.getPrototypeOf(", "), Array.of"];
pub(super) const OWN_FIELD: &str = "Object.prototype.hasOwnProperty.call(";
pub(super) const LAZY_FIELD: &str = "Object.defineProperty(this, ";
const CTOR_TEXTS: [&str; 7] = [CALLABLE, WRITABLE_NAME, SUPER_FIELDS[0], SUPER_FIELDS[1], SUPER_FIELDS[2], OWN_FIELD, LAZY_FIELD];

/// A def that cannot be overridden and calls itself in tail position is emitted as
/// `while (true)`: the JS parameters are named apart, each iteration copies them into `const`s
/// of the source names (so that closures keep the values of their iteration, as in scalac), and
/// a tail call assigns the parameters and continues.
struct TailLoop {
    fun: FunId,
    params: Vec<Rc<str>>,
    /// The parameter holding the receiver, when a tail call is made on another instance.
    receiver: Option<String>,
}

/// The output of one slice of a module, in the order the module lays the parts out.
#[derive(Default)]
pub struct Chunk {
    classes: String,
    registrations: String,
    /// The values of the module's enums; only the first chunk of a module has them.
    enum_values: String,
    enum_names: Vec<String>,
    funs: String,
    vals: String,
    /// The runtime helpers the chunk refers to.
    helpers: runtime::Used,
    bindings: Bindings,
    accessors: Vec<String>,
    sizes: Vec<Sized>,
}

impl Chunk {
    fn is_empty(&self) -> bool {
        self.classes.is_empty() && self.registrations.is_empty() && self.enum_values.is_empty() && self.funs.is_empty() && self.vals.is_empty()
    }
}

const PARALLEL_THRESHOLD_EXPRS: usize = 200_000;
/// A module of the split output is emitted in slices of about this many definitions, so that its
/// text depends on its own definitions alone and a large package still spreads over the cores.
const SPLIT_CHUNK_ITEMS: usize = 64;

pub fn emit(
    prog: &Program,
    syms: &Symbols,
    interner: &Interner,
    array_seq: Option<ClassId>,
    reach: &reach::Reach,
    split: Option<Split>,
    options: Options<'_>,
    mut cache: Option<&mut Cache>,
) -> (Output, Option<String>) {
    let mut lap = crate::measure::Lap::begin(crate::measure::Pass::Emit);
    let Options { release, size_report, outline } = options;
    let report_files = size_report;
    let size_report = size_report.is_some();
    let runtime = runtime::Runtime::parse(RUNTIME, release);
    let mut layout = layout::Layout::new(prog, syms, interner);
    layout.lower_closures(prog, syms, interner, reach);
    lap.done("closures lowered", layout.closure_anons.len());
    if outline {
        layout.shared = share::Shared::compute(prog, syms, interner, reach, &layout.closure_anons, cache.as_deref_mut().map(|c| &mut c.classes));
    }
    lap.done("sharing", layout.shared.groups.len());
    layout.number_classes(prog, syms, interner, reach);
    lap.done("classes numbered", 0);
    let file_groups = &layout.file_groups;
    let (mut modules, file_modules, callee_modules) = match split {
        None => (layout.single_module(prog, syms, interner, reach), Vec::new(), crate::intern::FxMap::default()),
        Some(s) => layout.package_modules(prog, syms, interner, reach, s.files, s.file_pkgs, s.module_per_file),
    };
    lap.done("layout", modules.len());
    let outline = outline::Outline::compute(
        prog,
        syms,
        interner,
        &layout.closure_anons,
        &layout.shared,
        reach,
        &mut modules,
        &|callee: SymId| callee_modules.get(&callee).copied().unwrap_or_else(|| module_of_callee(syms, &file_modules, callee)),
        cache.map(|c| &mut c.outline),
    );
    lap.done("outline", outline.funs.len());
    let outline = &outline;
    let naming = Naming::compute(
        prog,
        syms,
        interner,
        reach,
        &layout.const_vals,
        split.map(|s| (s.files, s.file_pkgs)),
        &runtime.helper_names(),
    );
    lap.done("naming", 0);
    let simple_cache: Vec<AtomicU8> = (0..prog.exprs.len()).map(|_| AtomicU8::new(0)).collect();
    let shared = SharedSymbols(syms);
    let renderable = Renderable::compute(
        prog,
        syms,
        interner,
        reach,
        &layout.file_inits,
        &runtime.helper_names(),
        expr::op_texts().chain(CTOR_TEXTS),
        outline.funs.iter().map(|f| f.name.clone()),
        layout.shared.groups.values().map(|g| g.name.clone()),
    );
    lap.done("renderable", 0);
    let renderable = &renderable;
    let renames = release.then(|| {
        let e = Emitter::new(prog, shared, interner, &simple_cache, array_seq, &layout, reach, &naming, outline, renderable);
        let primitive_trait = |c: ClassId| {
            let info = syms.class(c);
            matches!(interner.get(info.name), "CharSequence" | "Number") && matches!(info.owner, Owner::Package(p) if interner.get(syms.pkg(p).name) == "lang")
        };
        Renames::compute(syms, reach, |s| e.member_name(s), |c| e.extension_names(c), primitive_trait)
    });
    let new_emitter = || {
        let mut e = Emitter::new(prog, shared, interner, &simple_cache, array_seq, &layout, reach, &naming, outline, renderable);
        e.deferred_extends = split.is_some();
        e.hot = split.is_some_and(|s| s.hot);
        e.release = release;
        e.renames = renames.as_ref();
        e
    };
    lap.done("renames", 0);
    let workers = if prog.exprs.len() < PARALLEL_THRESHOLD_EXPRS {
        1
    } else {
        crate::workers()
    };
    let chunks_of: Vec<usize> = modules
        .iter()
        .map(|m| if split.is_none() { workers } else { (m.items() / SPLIT_CHUNK_ITEMS).max(1) })
        .collect();
    let emit_chunk = |e: &mut Emitter, m: usize, k: usize| -> Chunk {
        let module = &modules[m];
        let n = chunks_of[m];
        e.begin_chunk(module.scope);
        let range = |len: usize| (len * k / n)..(len * (k + 1) / n);
        let mut chunk = Chunk::default();
        let measure = |e: &Emitter, def: Def, from: usize, sizes: &mut Vec<Sized>| {
            if size_report {
                sizes.push(Sized { def, module: m, bytes: e.out.len() - from, registration: false, hash: 0 });
            }
        };
        for &i in &module.classes[range(module.classes.len())] {
            let at = e.out.len();
            e.begin_item();
            e.emit_class(i);
            if size_report {
                let text = &e.out[at..];
                let body = text.find('{').map_or(text, |open| &text[open..]);
                let mut h = crate::intern::FxHasher::default();
                std::hash::Hasher::write(&mut h, body.as_bytes());
                let hash = std::hash::Hasher::finish(&h);
                chunk.sizes.push(Sized { def: Def::Class(prog.classes[i].id), module: m, bytes: text.len(), registration: false, hash });
            }
        }
        chunk.classes = std::mem::take(&mut e.out);
        for &i in &module.classes[range(module.classes.len())] {
            let at = e.out.len();
            e.begin_item();
            e.emit_registration(i);
            if size_report {
                chunk.sizes.push(Sized { def: Def::Class(prog.classes[i].id), module: m, bytes: e.out.len() - at, registration: true, hash: 0 });
            }
        }
        chunk.registrations = std::mem::take(&mut e.out);
        if k == 0 {
            for &c in &module.enum_values {
                let at = e.out.len();
                e.begin_item();
                chunk.enum_names.extend(e.emit_enum_values(&[c], split.is_some()));
                measure(e, Def::Class(c), at, &mut chunk.sizes);
            }
            chunk.enum_values = std::mem::take(&mut e.out);
        }
        for &i in &module.funs[range(module.funs.len())] {
            if reach.funs[prog.top_funs[i].idx()] {
                let at = e.out.len();
                e.begin_item();
                e.emit_function(prog.top_funs[i], FunKind::TopLevel);
                measure(e, Def::Sym(prog.funs[prog.top_funs[i].idx()].sym), at, &mut chunk.sizes);
            }
        }
        for &f in &module.outlined[range(module.outlined.len())] {
            let at = e.out.len();
            e.begin_item();
            e.emit_outlined(f as u32);
            measure(e, Def::Outlined(f as u32), at, &mut chunk.sizes);
        }
        chunk.funs = std::mem::take(&mut e.out);
        for &i in &module.groups[range(module.groups.len())] {
            let at = e.out.len();
            e.begin_item();
            e.emit_top_vals(&file_groups[i].1);
            measure(e, Def::File(file_groups[i].0), at, &mut chunk.sizes);
        }
        chunk.vals = std::mem::take(&mut e.out);
        chunk.helpers = runtime.none_used();
        for text in [&chunk.classes, &chunk.registrations, &chunk.enum_values, &chunk.funs, &chunk.vals] {
            runtime.mark(text, &mut chunk.helpers);
        }
        chunk.bindings = std::mem::take(&mut e.bindings);
        chunk.accessors = std::mem::take(&mut e.accessors);
        chunk
    };
    // The largest modules first, so that the tail of the work is evenly spread.
    let mut order: Vec<usize> = (0..modules.len()).collect();
    order.sort_by_key(|&m| std::cmp::Reverse(modules[m].items()));
    let items: Vec<(usize, usize)> = order.iter().flat_map(|&m| (0..chunks_of[m]).map(move |k| (m, k))).collect();
    let mut chunks: Vec<Vec<Option<Chunk>>> = chunks_of.iter().map(|&n| (0..n).map(|_| None).collect()).collect();
    let next = AtomicUsize::new(0);
    let work = || {
        let mut e = new_emitter();
        let mut done = Vec::new();
        loop {
            let slot = next.fetch_add(1, Ordering::Relaxed);
            let Some(&(m, k)) = items.get(slot) else { break };
            done.push((m, k, emit_chunk(&mut e, m, k)));
        }
        done
    };
    let done: Vec<Vec<(usize, usize, Chunk)>> = if workers == 1 || items.len() == 1 {
        vec![work()]
    } else {
        std::thread::scope(|scope| {
            let handles: Vec<_> = (0..workers.min(items.len())).map(|_| crate::alloc::spawn_in(scope, &work)).collect();
            handles.into_iter().map(|h| h.join().expect("emitter thread panicked")).collect()
        })
    };
    for (m, k, chunk) in done.into_iter().flatten() {
        chunks[m][k] = Some(chunk);
    }
    let mut chunks: Vec<Vec<Chunk>> = chunks.into_iter().map(|cs| cs.into_iter().map(|c| c.unwrap()).collect()).collect();
    lap.done("chunks, in parallel", items.len());
    lap.count("emitter threads", workers.min(items.len()));

    let mut tail = new_emitter();
    if let Some(main) = prog.main {
        tail.emit_entry_point(main, prog.main_object);
    }
    let sizes: Vec<Sized> = if size_report {
        chunks.iter_mut().flat_map(|cs| cs.iter_mut().flat_map(|c| std::mem::take(&mut c.sizes))).collect()
    } else {
        Vec::new()
    };
    if let Some(s) = split {
        let output = tail.split_modules(&modules, chunks, &runtime, s.hot, &mut lap);
        lap.done("modules framed", output.files.len());
        let report = report_files.map(|paths| {
            let runtime = output.files.iter().find(|(name, _)| name == "rt.mjs").map_or(0, |(_, text)| text.len());
            new_emitter().size_report(&sizes, &modules, &output.files, runtime, paths)
        });
        return (Output::Modules(output), report);
    }
    let module = tail.module_parts();
    let chunks = &chunks[0];
    let mut helpers = runtime.none_used();
    for c in chunks {
        runtime::Runtime::merge(&mut helpers, &c.helpers);
    }
    for text in [&tail.out, &module.program_start] {
        runtime.mark(text, &mut helpers);
    }
    let body_len: usize = chunks
        .iter()
        .map(|c| c.classes.len() + c.registrations.len() + c.enum_values.len() + c.funs.len() + c.vals.len())
        .sum();
    let module_len = module.imports.len() + module.program_start.len() + module.exports.len();
    let mut out = String::with_capacity(runtime.len() + body_len + tail.out.len() + module_len + 64);
    out.push_str(&module.imports);
    runtime.write(&helpers, &mut out);
    out.push_str(&module.program_start);
    out.push_str("(function () {\n\"use strict\";");
    for c in chunks {
        out.push_str(&c.classes);
    }
    for c in chunks {
        out.push_str(&c.registrations);
    }
    for c in chunks {
        out.push_str(&c.enum_values);
    }
    for c in chunks {
        out.push_str(&c.funs);
    }
    for c in chunks {
        out.push_str(&c.vals);
    }
    out.push_str(&tail.out);
    out.push_str("\n})();\n");
    out.push_str(&module.exports);
    lap.done("file framed", 1);
    let report = report_files.map(|paths| {
        let mut runtime_text = String::new();
        runtime.write(&helpers, &mut runtime_text);
        tail.size_report(&sizes, &modules, &[(String::new(), out.clone())], runtime_text.len(), paths)
    });
    (Output::Script(out), report)
}

/// `scala.FunctionN`, which the typer makes on first use.
/// `classOf[T].getName` for a builtin, as the JVM names it; None for a class of the program.
pub(super) fn class_of_name(syms: &Symbols, interner: &Interner, c: ClassId) -> Option<String> {
    let info = syms.class(c);
    if info.kind != ClassKind::Builtin {
        return None;
    }
    let name = match interner.get(info.name) {
        "Int" => "int",
        "Long" => "long",
        "Double" => "double",
        "Float" => "float",
        "Boolean" => "boolean",
        "Char" => "char",
        "Byte" => "byte",
        "Short" => "short",
        "Unit" => "void",
        "String" => "java.lang.String",
        "Array" => "[Ljava.lang.Object;",
        "Nothing" => "scala.runtime.Nothing$",
        "Null" => "scala.runtime.Null$",
        _ => "java.lang.Object",
    };
    Some(name.to_string())
}

pub(super) fn is_function_class(syms: &Symbols, interner: &Interner, c: ClassId) -> bool {
    let info = syms.class(c);
    let Owner::Package(p) = info.owner else { return false };
    let name = interner.get(info.name);
    let name = name.strip_prefix("Context").unwrap_or(name);
    interner.get(syms.pkg(p).name) == "scala"
        && name.strip_prefix("Function").map_or(false, |rest| !rest.is_empty() && rest.bytes().all(|b| b.is_ascii_digit()))
}

/// An object nested in a class or trait beside a companion class of its name, which the output
/// names apart from the class; a jar's inner companion object carries scalac's `$` already.
pub(crate) fn beside_companion(interner: &Interner, info: &ClassInfo) -> bool {
    info.inner_object.is_some() && info.companion.is_some() && !interner.get(info.name).ends_with('$')
}

/// The nested objects of `c` that implement a member of a parent trait, with the member each one
/// stands for.
pub(crate) fn implemented_nested_objects(syms: &Symbols, c: ClassId) -> Vec<(SymId, ClassId)> {
    let info = syms.class(c);
    let mut out = Vec::new();
    for &m in &info.member_order {
        let SymKind::Object(nested) = syms.sym(m).kind else { continue };
        let name = syms.sym(m).name;
        let declared = info.base_types.iter().skip(1).find_map(|&(b, _)| syms.class(b).members.get(&name).copied());
        if let Some(declared) = declared {
            out.push((declared, nested));
        }
    }
    out
}

/// The companion whose body scalac's synthetic `apply` of a case class runs, when it has one.
pub(crate) fn touched_companion(syms: &Symbols, has_body: &[bool], c: ClassId) -> Option<ClassId> {
    let info = syms.class(c);
    if info.kind != ClassKind::Class || info.mods & mods::CASE == 0 {
        return None;
    }
    let companion = info.companion?;
    let touched = has_body[companion.idx()] && syms.class(companion).kind == ClassKind::Object;
    touched.then_some(companion)
}

impl<'a> Emitter<'a> {
    fn new(
        prog: &'a Program,
        shared: SharedSymbols<'a>,
        interner: &'a Interner,
        simple_cache: &'a [AtomicU8],
        array_seq: Option<ClassId>,
        layout: &'a layout::Layout,
        reach: &'a reach::Reach,
        naming: &'a Naming,
        outline: &'a outline::Outline,
        renderable: &'a Renderable<'a>,
    ) -> Emitter<'a> {
        let syms = shared.0;
        Emitter {
            prog,
            syms,
            interner,
            out: String::new(),
            indent: 0,
            sym_names: vec![None; syms.syms.len()],
            class_names: vec![None; syms.classes.len()],
            sym_quals: Default::default(),
            class_quals: Default::default(),
            naming,
            scope: NO_SCOPE,
            renames: None,
            release: false,
            sym_seen: vec![0; syms.syms.len()],
            class_seen: vec![0; syms.classes.len()],
            chunk: 1,
            simple_cache,
            tmp_counter: 0,
            array_seq,
            const_vals: &layout.const_vals,
            file_inits: &layout.file_inits,
            has_body: &layout.has_body,
            class_numbers: &layout.class_numbers,
            anon_parent_args: &layout.anon_parent_args,
            local_captures: &layout.local_captures,
            closure_anons: &layout.closure_anons,
            shared: &layout.shared,
            outline,
            body: None,
            reach,
            captures: Vec::new(),
            closure_captures: Vec::new(),
            tail: None,
            fn_depth: 0,
            nonlocal_return: None,
            this_name: "this",
            scope_names: Vec::new(),
            named_locals: Vec::new(),
            bindings: Bindings::default(),
            accessors: Vec::new(),
            deferred_extends: false,
            hot: false,
            body_params: Vec::new(),
            ctor_locals: Vec::new(),
            before_super: false,
            frames: Vec::new(),
            swapped: 0,
            init_file: None,
            entry_check: None,
            checked: None,
            scopes: Vec::new(),
            scope_ids: 0,
            open_names: Default::default(),
            decl_scope: Default::default(),
            roots: Default::default(),
            pending_names: Default::default(),
            renderable,
            group_inits: Vec::new(),
            def_scope: None,
            ctor_bindings: Default::default(),
            unbound: false,
            assign_targets: Vec::new(),
            ctor_texts: Vec::new(),
            walking: false,
            named_as: Default::default(),
        }
    }

    /// Starts recording the accessor calls of the function body that follows the `{` just
    /// written.
    fn open_frame(&mut self) {
        let at = self.out.len();
        self.frames.push(Frame {
            start: at,
            top_indent: self.indent,
            cond: 0,
            mark: Some((at, self.indent)),
            inline: false,
            swapped: self.swapped,
            uses: Vec::new(),
            edits: Vec::new(),
            nested: Vec::new(),
        });
    }

    /// A statement of the body of the innermost frame begins here.
    fn mark_stmt(&mut self) {
        let (at, indent) = (self.out.len(), self.indent);
        if let Some(f) = self.frames.last_mut() {
            if f.cond == 0 && f.top_indent == indent && f.swapped == self.swapped {
                f.mark = Some((at, indent));
            }
        }
    }

    fn enter_cond(&mut self) {
        if let Some(f) = self.frames.last_mut() {
            f.cond += 1;
        }
    }

    fn leave_cond(&mut self) {
        if let Some(f) = self.frames.last_mut() {
            f.cond -= 1;
        }
    }

    /// Writes the call of the accessor of the object `c` and records it.
    fn emit_accessor_call(&mut self, c: ClassId) {
        let accessor = self.module_accessor(c);
        let start = self.out.len();
        self.out.push_str(&accessor);
        self.out.push_str("()");
        self.record_accessor(c, start, UseForm::Call);
    }

    /// Writes `X$();` as a statement of its own and records it.
    fn emit_accessor_stmt(&mut self, c: ClassId) {
        let accessor = self.module_accessor(c);
        let start = self.out.len();
        self.line();
        let _ = write!(self.out, "{}();", accessor);
        self.record_accessor(c, start, UseForm::Stmt);
    }

    /// Writes `(X$(), ` and records the touch; `close_touch` writes its `)`.
    fn open_touch(&mut self, c: ClassId) -> Option<usize> {
        let accessor = self.module_accessor(c);
        let start = self.out.len();
        let _ = write!(self.out, "({}(), ", accessor);
        self.record_accessor(c, start, UseForm::Touch(0))
    }

    fn close_touch(&mut self, touch: Option<usize>) {
        let at = self.out.len();
        self.out.push(')');
        if let (Some(i), Some(f)) = (touch, self.frames.last_mut()) {
            f.uses[i].form = UseForm::Touch(at);
        }
    }

    fn record_accessor(&mut self, c: ClassId, start: usize, form: UseForm) -> Option<usize> {
        let end = self.out.len();
        let swapped = self.swapped;
        let f = self.frames.last_mut().filter(|f| f.swapped == swapped)?;
        f.uses.push(AccessorUse { class: c, span: (start, end), form, cond: f.cond > 0, mark: f.mark });
        Some(f.uses.len() - 1)
    }

    /// `$$JE` for the accessor `JsonEncoder$`.
    fn binding_name(accessor: &str) -> String {
        let mut name = String::from("$$");
        name.extend(accessor[..accessor.len() - 1].chars().filter(|c| c.is_ascii_uppercase() || c.is_ascii_digit()).take(3));
        if name.len() == 2 {
            name.push(accessor.chars().next().unwrap().to_ascii_uppercase());
        }
        name
    }

    /// Ends the innermost frame: binds the accessors called twice below it, hands the other
    /// calls on to the frame around it as calls that may not run, and applies the edits once
    /// no frame of the same buffer is left.
    fn close_frame(&mut self) {
        let mut frame = self.frames.pop().expect("a frame is open");
        let parent_same_buffer = self.frames.last().map_or(false, |p| p.swapped == frame.swapped);
        let mut uses = std::mem::take(&mut frame.uses);
        uses.sort_by_key(|u| (u.class.0, u.span.0));
        let mut handed: Vec<AccessorUse> = Vec::new();
        let mut bound: Vec<(usize, ClassId)> = Vec::new();
        let mut i = 0;
        while i < uses.len() {
            let class = uses[i].class;
            let mut j = i;
            while j < uses.len() && uses[j].class == class {
                j += 1;
            }
            let group = &uses[i..j];
            let binding = group.iter().find(|u| !u.cond).and_then(|u| u.mark);
            let covered: Vec<&AccessorUse> = match binding {
                Some((at, _)) => group.iter().filter(|u| u.span.0 >= at).collect(),
                None => Vec::new(),
            };
            let accessor = self.module_accessor(class);
            let name = Self::binding_name(&accessor);
            let saved: usize = covered.iter().map(|u| u.span.1 - u.span.0 + if u.form == UseForm::Call { 0 } else { 1 }).sum();
            let calls = covered.iter().filter(|u| u.form == UseForm::Call).count();
            let cost = "const  = ();".len() + name.len() + accessor.len() + calls * name.len() + !frame.inline as usize;
            if covered.len() < 2 || saved <= cost {
                if parent_same_buffer {
                    handed.extend(group.iter().map(|u| AccessorUse { cond: true, mark: None, ..*u }));
                }
                i = j;
                continue;
            }
            bound.push((covered[0].span.0, class));
            i = j;
        }
        // The bindings in the order of their first calls, named apart.
        bound.sort_unstable();
        let mut names: Vec<String> = Vec::new();
        for &(first, class) in &bound {
            let base = Self::binding_name(&self.module_accessor(class));
            let shadowed = |name: &str| {
                frame.nested.iter().any(|(start, end, n)| n == name && uses.iter().any(|u| u.class == class && u.span.0 >= first && u.span.0 >= *start && u.span.0 < *end))
            };
            let mut name = base.clone();
            let mut k = 1;
            // Nor does it take the name of a local of the item, which it would hide or redeclare.
            while names.contains(&name) || shadowed(&name) || self.named_as.contains_key(name.as_str()) {
                k += 1;
                name = format!("{}${}", base, k);
            }
            names.push(name);
        }
        for (k, &(first, class)) in bound.iter().enumerate() {
            let name = &names[k];
            let accessor = self.module_accessor(class);
            let group: Vec<&AccessorUse> = uses.iter().filter(|u| u.class == class).collect();
            let (at, indent) = group.iter().find(|u| !u.cond).and_then(|u| u.mark).unwrap();
            let mut decl = String::new();
            if !frame.inline {
                decl.push('\n');
                for _ in 0..indent {
                    decl.push_str("  ");
                }
            }
            let _ = write!(decl, "const {} = {}();", name, accessor);
            if frame.inline {
                decl.push(' ');
            }
            frame.edits.push((at, at, first, decl));
            for u in group {
                if u.span.0 < at {
                    if parent_same_buffer {
                        handed.push(AccessorUse { cond: true, mark: None, ..*u });
                    }
                    continue;
                }
                match u.form {
                    UseForm::Call => frame.edits.push((u.span.0, u.span.1, u.span.0, name.clone())),
                    UseForm::Touch(close) => {
                        frame.edits.push((u.span.0, u.span.1, u.span.0, String::new()));
                        frame.edits.push((close, close + 1, close, String::new()));
                    }
                    UseForm::Stmt => frame.edits.push((u.span.0, u.span.1, u.span.0, String::new())),
                }
            }
        }
        if parent_same_buffer {
            let end = self.out.len();
            let parent = self.frames.last_mut().unwrap();
            parent.uses.extend(handed);
            parent.edits.extend(frame.edits);
            parent.nested.extend(frame.nested);
            parent.nested.extend(names.into_iter().map(|n| (frame.start, end, n)));
            return;
        }
        if frame.edits.is_empty() {
            return;
        }
        frame.edits.sort_by_key(|e| (e.0, e.1, e.2));
        let tail = self.out.split_off(frame.start);
        let mut at = 0;
        for (pos, end, _, text) in frame.edits {
            self.out.push_str(&tail[at..pos - frame.start]);
            self.out.push_str(&text);
            at = end - frame.start;
        }
        self.out.push_str(&tail[at..]);
    }

    /// Starts the output of another chunk: the names given so far stay valid, since a
    /// module-level name depends on its definition alone; the temporaries and scopes start over.
    fn begin_chunk(&mut self, scope: u32) {
        self.chunk += 1;
        self.scope = scope;
        self.begin_item();
        self.indent = 0;
        self.captures.clear();
        self.closure_captures.clear();
        self.tail = None;
        self.this_name = "this";
        self.out.clear();
        self.bindings = Bindings::default();
    }

    /// Starts a definition of the module: the temporaries and the names of the locals start
    /// over, so that its text is a function of the definition alone and not of what the same
    /// emitter wrote before it.
    fn begin_item(&mut self) {
        self.tmp_counter = 0;
        self.scope_names.clear();
        for s in self.named_locals.drain(..) {
            self.sym_names[s.idx()] = None;
        }
        debug_assert!(self.scopes.is_empty(), "a scope is left open");
        self.scopes.clear();
        self.open_names.clear();
        self.decl_scope.clear();
        self.roots.clear();
        self.pending_names.clear();
        self.named_as.clear();
    }

    fn line(&mut self) {
        if self.release {
            return;
        }
        self.out.push('\n');
        for _ in 0..self.indent {
            self.out.push_str("  ");
        }
    }

    fn open(&mut self, s: &str) {
        self.out.push_str(s);
        self.indent += 1;
    }

    fn close(&mut self, s: &str) {
        self.indent -= 1;
        self.line();
        self.out.push_str(s);
    }

    fn pkg_prefix(&self, p: PkgId, out: &mut String) {
        let info = self.syms.pkg(p);
        if let Some(parent) = info.parent {
            self.pkg_prefix(parent, out);
            out.push_str(&sanitize(self.interner.get(info.name)));
            out.push('$');
        }
    }

    /// The classes a definition is nested in, `Outer$Inner$`, without the package.
    fn class_prefix(&self, owner: Owner, out: &mut String) {
        if let Owner::Class(c) = owner {
            let info = self.syms.class(c);
            self.class_prefix(info.owner, out);
            out.push_str(&sanitize(self.interner.get(info.name)));
            out.push('$');
        }
    }

    fn package_of(&self, owner: Owner) -> Option<PkgId> {
        match owner {
            Owner::Package(p) => Some(p),
            Owner::Class(c) => self.package_of(self.syms.class(c).owner),
            Owner::Local => None,
        }
    }

    /// The bare name behind the prefix of its package.
    fn qualify(&self, bare: &str, owner: Owner) -> Rc<str> {
        let mut full = String::new();
        if let Some(p) = self.package_of(owner) {
            self.pkg_prefix(p, &mut full);
        }
        full.push_str(bare);
        Rc::from(full)
    }

    /// `.name`, or `["name"]` for a property whose name is no identifier.
    fn js_prop_access(name: &str) -> String {
        if is_js_identifier(name) {
            format!(".{}", name)
        } else {
            let mut s = String::from("[");
            js_string(name, &mut s);
            s.push(']');
            s
        }
    }

    /// The access to the property that holds the member `s` of a JS type.
    fn js_prop(&self, s: SymId) -> String {
        let name = self.interner.get(self.syms.js_member_name(s));
        if self.syms.sym(s).js_symbol {
            return format!("[Symbol.{}]", name);
        }
        Self::js_prop_access(name)
    }

    fn js_global_ref(name: &str) -> String {
        if is_js_identifier(name) {
            name.to_string()
        } else {
            format!("globalThis{}", Self::js_prop_access(name))
        }
    }

    /// The JS expression of a native class or object.
    fn js_binding(&mut self, b: JsBinding) -> String {
        match b {
            JsBinding::Import(i) => self.js_import(i),
            _ => {
                let text = self.js_binding_text(b);
                self.note_name(scope::identifier_of(&text));
                text
            }
        }
    }

    fn js_binding_text(&self, b: JsBinding) -> String {
        match b {
            JsBinding::Import(i) if self.is_default_import(i) => format!("$imp{}.default", i),
            JsBinding::Import(i) => format!("$imp{}", i),
            JsBinding::Global(n) => Self::js_global_ref(self.interner.get(n)),
            JsBinding::GlobalScope => "globalThis".to_string(),
        }
    }

    /// The module-level binding of `Program::js_imports[i]`.
    pub fn js_import(&mut self, i: u32) -> String {
        self.bindings.imports.push(i);
        self.note_name(&format!("$imp{}", i));
        if self.is_default_import(i) {
            format!("$imp{}.default", i)
        } else {
            format!("$imp{}", i)
        }
    }

    fn is_default_import(&self, i: u32) -> bool {
        self.interner.get(self.prog.js_imports[i as usize].name) == "default"
    }

    /// Whether `r` is a `@JSGlobalScope` object, whose members are global identifiers.
    fn is_global_scope(&self, r: TExprId) -> bool {
        matches!(self.peek(r), TExpr::Module(c) if self.syms.class(c).js_binding == Some(JsBinding::GlobalScope))
    }

    /// The global identifier that the member `s` of the `@JSGlobalScope` object `r` stands for.
    fn global_scope_member(&self, r: TExprId, s: SymId) -> Option<String> {
        if !self.is_global_scope(r) {
            return None;
        }
        Some(Self::js_global_ref(self.interner.get(self.syms.js_member_name(s))))
    }

    /// Notes a use that holds the output to the class as it is (`Held`), where a hot swap
    /// could make the class anew.
    pub(super) fn hold(&mut self, c: ClassId, how: Held) {
        if !self.hot {
            return;
        }
        let held = (self.shared.rep_of(c), how);
        if !self.bindings.held.contains(&held) {
            self.bindings.held.push(held);
        }
    }

    /// Notes the comparison with `e` where it is the value of an enum case or an object.
    pub(super) fn hold_compared(&mut self, e: TExprId) {
        if !self.hot {
            return;
        }
        match self.peek(e) {
            TExpr::Static(s) => {
                if let SymKind::EnumValue(case) = self.syms.sym(s).kind {
                    self.hold(case, Held::Compared);
                }
            }
            TExpr::Module(c) => self.hold(c, Held::Compared),
            _ => {}
        }
    }

    /// Whether the class, or a class it extends, has a `val` or `var` of that name.
    fn has_field_named(&self, c: ClassId, name: &str) -> bool {
        let info = self.syms.class(c);
        std::iter::once(c).chain(info.base_types.iter().map(|&(b, _)| b)).any(|k| {
            self.syms.class(k).member_order.iter().any(|&m| {
                let s = self.syms.sym(m);
                matches!(s.kind, SymKind::Val | SymKind::Var) && self.interner.get(s.name) == name
            })
        })
    }

    pub fn class_name(&mut self, c: ClassId) -> Rc<str> {
        self.record_class(c);
        let name = self.class_text(c);
        self.note_name(scope::identifier_of(&name));
        name
    }

    /// Notes that the chunk refers to the class `c`: a module of the split output imports it.
    fn record_class(&mut self, c: ClassId) {
        let c = self.shared.rep_of(c);
        let info = self.syms.class(c);
        if self.class_seen[c.idx()] != self.chunk {
            self.class_seen[c.idx()] = self.chunk;
            match info.js_binding {
                None => self.bindings.classes.push(c),
                Some(JsBinding::Import(i)) => self.bindings.imports.push(i),
                Some(_) => {}
            }
        }
    }

    /// The name of the class `c` in the output, without noting the reference.
    pub(super) fn class_text(&mut self, c: ClassId) -> Rc<str> {
        let c = self.shared.rep_of(c);
        let info = self.syms.class(c);
        if let Some(b) = info.js_binding {
            if let Some(n) = &self.class_names[c.idx()] {
                return n.clone();
            }
            let rc: Rc<str> = Rc::from(self.js_binding_text(b));
            self.class_names[c.idx()] = Some(rc.clone());
            return rc;
        }
        let bare = match &self.class_names[c.idx()] {
            Some(n) => n.clone(),
            None if self.shared.groups.contains_key(&c) => {
                let rc: Rc<str> = Rc::from(self.shared.groups[&c].name.as_str());
                self.class_names[c.idx()] = Some(rc.clone());
                rc
            }
            None => {
                let mut s = String::new();
                // An opaque type's companion of another module's pickle, nested there in its file's
                // `$package` object: a class of the package, as the program's own (a jar's keeps
                // scalac's name).
                let product = self.reach.product_classes.get(c.idx()).copied().unwrap_or(false);
                if !product || self.syms.opaque_companion_package(c, self.interner).is_none() {
                    self.class_prefix(info.owner, &mut s);
                }
                s.push_str(&sanitize(self.interner.get(info.name)));
                // A named local class is known by its position, like an anonymous one.
                if info.owner == Owner::Local && info.kind != ClassKind::Anon {
                    match self.syms.product_position(info.file, info.def) {
                        Some((token, offset)) => {
                            let _ = write!(s, "${}_{}", crate::source::tag_text(token), offset);
                        }
                        None => {
                            let _ = write!(s, "${}", self.prog.position(info.file, info.span.start));
                        }
                    }
                }
                match info.kind {
                    ClassKind::Object => s.push_str("$M"),
                    // An object nested in a class is named apart from its companion class.
                    _ if beside_companion(self.interner, &info) => s.push_str("$M"),
                    ClassKind::GivenImpl => s.push_str("$G"),
                    ClassKind::EnumCase if info.singleton.is_some() => s.push_str("$V"),
                    _ => self.naming.avoid_taken(&mut s),
                }
                let rc: Rc<str> = Rc::from(s);
                self.class_names[c.idx()] = Some(rc.clone());
                rc
            }
        };
        if !self.naming.qualified(self.naming.classes[c.idx()], info.file, self.scope) {
            return bare;
        }
        if let Some(n) = self.class_quals.get(&c) {
            return n.clone();
        }
        let full = self.qualify(&bare, info.owner);
        self.class_quals.insert(c, full.clone());
        full
    }

    /// `$main` runs the entry point and flushes its output. A method of an object is called on
    /// the instance, and the command line reaches `main(args: Array[String])` as a JS array and a
    /// `@main def f(args: String*)` as a sequence.
    fn emit_entry_point(&mut self, main: SymId, object: Option<ClassId>) {
        let name = self.sym_name(main);
        let info = self.syms.sym(main);
        let owner = match (object, info.owner) {
            (Some(c), _) | (None, Owner::Class(c)) => Some(c),
            _ => None,
        };
        let param = info.sig.as_ref().and_then(|sig| sig.clauses.iter().flat_map(|c| c.params.iter()).next());
        let args = match (param, self.array_seq) {
            (None, _) => String::new(),
            (Some(p), Some(seq)) if p.repeated => format!("new {}($args())", self.class_name(seq)),
            (Some(_), _) => "$args()".to_string(),
        };
        self.line();
        let file_init = self.file_init(info.file).filter(|_| owner.is_none());
        match owner {
            // scalac reaches a top-level `@main` through its file's object, whose vals run first.
            None if file_init.is_some() => {
                let _ = write!(self.out, "$main(() => {{ {}(); return {}({}); }});", file_init.unwrap(), name, args);
            }
            None if args.is_empty() => {
                let _ = write!(self.out, "$main({});", name);
            }
            None => {
                let _ = write!(self.out, "$main(() => {}({}));", name, args);
            }
            Some(c) => {
                let accessor = self.module_accessor(c);
                let _ = write!(self.out, "$main(() => {}().{}({}));", accessor, name, args);
            }
        }
    }

    /// Accessor function returning the singleton instance of an object.
    fn module_accessor(&mut self, c: ClassId) -> String {
        self.record_class(c);
        let accessor = self.accessor_text(c);
        self.note_name(&accessor);
        accessor
    }

    pub(super) fn accessor_text(&mut self, c: ClassId) -> String {
        let name = self.class_text(c);
        format!("{}$", &name[..name.len() - 2])
    }

    pub fn sym_name(&mut self, s: SymId) -> Rc<str> {
        let info = self.syms.sym(s);
        if self.sym_seen[s.idx()] != self.chunk {
            self.sym_seen[s.idx()] = self.chunk;
            if matches!(info.owner, Owner::Package(_)) || matches!(info.kind, SymKind::EnumValue(_)) {
                self.bindings.syms.push(s);
            }
        }
        self.sym_text(s)
    }

    /// The name of `s` in the output, without noting the reference; a local is named here.
    pub(super) fn sym_text(&mut self, s: SymId) -> Rc<str> {
        let info = self.syms.sym(s);
        if let Owner::Package(_) = info.owner {
            return self.package_sym_name(s);
        }
        if let Some(n) = &self.sym_names[s.idx()] {
            return n.clone();
        }
        if let (Owner::Class(c), true) = (info.owner, info.is_extension) {
            self.name_class_extensions(c);
            if let Some(n) = &self.sym_names[s.idx()] {
                return n.clone();
            }
        }
        let name = match info.owner {
            Owner::Class(_) => {
                let dev = self.member_name(s);
                match self.renames {
                    Some(r) => r.get(&dev),
                    None => dev,
                }
            }
            Owner::Local => self.name_local(s),
            Owner::Package(_) => sanitize(self.interner.get(self.syms.dispatch_name(s))),
        };
        let rc: Rc<str> = Rc::from(name);
        if info.owner == Owner::Local {
            self.scope_names.push(info.name);
            self.named_locals.push(s);
            self.note_bound(s, rc.clone());
        }
        self.sym_names[s.idx()] = Some(rc.clone());
        rc
    }

    /// The name of the setter method of the var `s`, which implements an abstract setter: the
    /// name of the setter the typer recorded it as overriding, under which every call of it is
    /// made.
    fn setter_name(&mut self, s: SymId) -> String {
        let setter_name = self.interner.lookup(&format!("{}_=", self.interner.get(self.syms.sym(s).name)));
        let implemented = self.prog.overrides.get(&s).and_then(|bases| bases.iter().copied().find(|&b| Some(self.syms.sym(b).name) == setter_name));
        if let Some(setter) = implemented {
            return self.sym_name(setter).to_string();
        }
        let dev = format!("{}_$eq", sanitize(self.interner.get(self.syms.sym(s).name)));
        match self.renames {
            Some(r) => r.get(&dev),
            None => dev,
        }
    }

    /// The property name of the member `s` of a Scala class as the development output writes
    /// it; the release table maps these to short names.
    fn member_name(&self, s: SymId) -> String {
        let info = self.syms.sym(s);
        let Owner::Class(c) = info.owner else { unreachable!("a member") };
        if info.is_extension {
            return self.extension_names(c).into_iter().find(|(e, _)| *e == s).map(|(_, n)| n).expect("an extension");
        }
        let base = sanitize(self.interner.get(self.syms.dispatch_name(s)));
        // An implementing class may have a member of the same name next to the field it fills in.
        if info.mods & mods::PRIVATE != 0 && self.is_trait_state(c, s) {
            format!("{}$t{}", base, self.class_number(c))
        } else if info.mods & mods::PRIVATE != 0 && !info.scoped_private && self.syms.private_name_clashes(c, info.name) {
            format!("{}$p{}", base, self.class_number(c))
        } else {
            base
        }
    }

    /// A top-level def, val, var or given: its bare name in the scope of its package, the
    /// qualified one where the bare name is taken (`Naming`).
    fn package_sym_name(&mut self, s: SymId) -> Rc<str> {
        let info = self.syms.sym(s);
        let bare = match &self.sym_names[s.idx()] {
            Some(n) => n.clone(),
            None => {
                let mut name = sanitize(self.interner.get(self.syms.dispatch_name(s)));
                if info.is_extension {
                    let _ = write!(name, "$x{}", layout::extension_rank(self.syms, self.interner, s));
                }
                // The conversion of a top-level `implicit class` shares the class's name.
                let class_named_alike = info.kind == SymKind::Def
                    && matches!(info.owner, Owner::Package(p) if self.syms.pkg(p).entries.get(&info.name).map_or(false, |e| e.class.is_some()));
                if class_named_alike {
                    name.push_str("$f");
                }
                match info.kind {
                    SymKind::Val | SymKind::Var | SymKind::Given if !self.const_vals[s.idx()] => name.push('$'),
                    _ => self.naming.avoid_taken(&mut name),
                }
                let rc: Rc<str> = Rc::from(name);
                self.sym_names[s.idx()] = Some(rc.clone());
                rc
            }
        };
        if !self.naming.qualified(self.naming.syms[s.idx()], info.file, self.scope) {
            return bare;
        }
        if let Some(n) = self.sym_quals.get(&s) {
            return n.clone();
        }
        let full = self.qualify(&bare, info.owner);
        self.sym_quals.insert(s, full.clone());
        full
    }

    /// A private parameter, val or var of a trait: the class that holds it may have a member of
    /// the same name.
    fn is_trait_state(&self, c: ClassId, s: SymId) -> bool {
        let info = self.syms.class(c);
        if info.kind != ClassKind::Trait {
            return false;
        }
        let sym = self.syms.sym(s);
        let is_field = matches!(sym.kind, SymKind::Val | SymKind::Var) && sym.def.is_some() && sym.mods & mods::LAZY == 0;
        is_field || info.ctor.iter().any(|cl| cl.params.iter().any(|p| p.sym == s))
    }

    /// Extensions of one class may share a name and differ by receiver or arity. The first one
    /// keeps the plain name, which is also what ties an implementation to its trait's declaration.
    fn extension_names(&self, c: ClassId) -> Vec<(SymId, String)> {
        let syms = self.syms;
        let info = syms.class(c);
        let mut taken: crate::intern::FxMap<crate::intern::Name, u32> = Default::default();
        let mut out = Vec::with_capacity(info.extensions.len());
        for &e in &info.extensions {
            let name = syms.sym(e).name;
            let k = taken.entry(name).or_insert(info.members.contains_key(&name) as u32);
            let mut js = sanitize(self.interner.get(name));
            if *k > 0 {
                let _ = write!(js, "$x{}", k);
            }
            *k += 1;
            out.push((e, js));
        }
        out
    }

    fn name_class_extensions(&mut self, c: ClassId) {
        for (e, dev) in self.extension_names(c) {
            let name = match self.renames {
                Some(r) => r.get(&dev),
                None => dev,
            };
            self.sym_names[e.idx()] = Some(Rc::from(name));
        }
    }

    fn field_name(&mut self, s: SymId) -> String {
        let n = self.sym_name(s);
        if self.syms.sym(s).needs_accessor {
            format!("${}", n)
        } else {
            n.to_string()
        }
    }

    /// The access to the field of `this` that holds the member `s`.
    fn field_access(&mut self, s: SymId) -> String {
        if self.syms.js_member(s) {
            self.js_prop(s)
        } else {
            format!(".{}", self.field_name(s))
        }
    }

    /// The field of an anonymous class that holds the captured local `s`: by the local's source
    /// name, numbered since two captured locals may share one.
    fn capture_field(&self, index: usize, s: SymId) -> String {
        format!("{}$c{}", sanitize(self.interner.get(self.syms.sym(s).name)), index)
    }

    /// The JS expression that reads the local `s`: its name, or the field of the anonymous class
    /// being emitted that holds it. A captured `var` sits in a `$ref` cell.
    pub fn local_ref(&mut self, s: SymId) -> String {
        if let Some(&(_, passed)) = self.closure_captures.iter().rev().find(|&&(c, _)| c == s) {
            return match self.prog.expr(passed) {
                TExpr::Local(t) => self.local_ref(t),
                _ => self.this_name.to_string(),
            };
        }
        if !self.body_params.is_empty() && self.body_params.contains(&s) && !self.captures.contains(&s) {
            let field = self.field_access(s);
            return format!("{}{}", self.this_name, field);
        }
        match self.captures.iter().position(|&c| c == s) {
            Some(i) if self.before_super => {
                if self.syms.sym(s).kind == SymKind::Var {
                    format!("$c{}.v", i)
                } else {
                    format!("$c{}", i)
                }
            }
            Some(i) => {
                let field = self.capture_field(i, s);
                if self.syms.sym(s).kind == SymKind::Var {
                    format!("{}.{}.v", self.this_name, field)
                } else {
                    format!("{}.{}", self.this_name, field)
                }
            }
            None if self.ctor_bindings.contains_key(&s) => self.ctor_bindings[&s].to_string(),
            None => {
                let n = self.sym_name(s);
                if matches!(self.syms.sym(s).owner, Owner::Package(_)) {
                    self.note_name(&n);
                }
                n.to_string()
            }
        }
    }

    /// The file initialised where the body of the top-level definition `s` runs: its own, when
    /// every access of `s` runs the initialiser first (`Reach::triggers`), checked at the access
    /// or, for a function value made outside the file or an export, at the body's entry.
    fn initialised_in(&self, s: SymId) -> Option<FileId> {
        self.reach.triggers.get(s.idx()).copied().unwrap_or(false).then(|| self.syms.sym(s).file)
    }

    fn emit_entry_check(&mut self) {
        if let Some(check) = self.entry_check.take() {
            self.line();
            self.out.push_str(&check);
        }
    }

    /// The file whose initialiser an access of the top-level definition `s` runs first, where
    /// the code being written does not run inside the file's initialised part; `None` for the
    /// access `e` whose check stands in front of its statement.
    pub(super) fn needs_init(&self, s: SymId, e: Option<TExprId>) -> Option<FileId> {
        if !self.reach.triggers.get(s.idx()).copied().unwrap_or(false) || (e.is_some() && e == self.checked) {
            return None;
        }
        let file = self.syms.sym(s).file;
        (self.init_file != Some(file)).then_some(file)
    }

    /// The initialiser of a file with eager top-level vals.
    fn file_init(&mut self, file: FileId) -> Option<String> {
        let init = self.file_init_text(file)?;
        self.bindings.files.push(file);
        self.note_name(&init);
        Some(init)
    }

    pub(super) fn file_init_text(&self, file: FileId) -> Option<String> {
        let has = self.file_inits.get(file.0 as usize).copied().unwrap_or(false);
        has.then(|| self.prog.file_init_name(file))
    }

    /// The top-level vals of one file. As scalac's `file$package` object, the file initialises
    /// every plain val on the first read of any of them, in order; a lazy val, a given and a
    /// constant stand on their own.
    fn emit_top_vals(&mut self, vals: &[(SymId, TExprId)]) {
        let mut eager: Vec<(SymId, TExprId)> = Vec::new();
        for &(sym, init) in vals {
            if is_eager_top_val(self.syms, sym, self.const_vals) {
                eager.push((sym, init));
            } else if self.reach.vals[sym.idx()] {
                self.emit_top_val(sym, init);
            }
        }
        let file = match eager.first() {
            Some(&(s, _)) => self.syms.sym(s).file,
            None => return,
        };
        if !self.reach.files.get(file.0 as usize).copied().unwrap_or(false) {
            return;
        }
        let Some(init) = self.file_init(file) else { return };
        self.bindings.defs.push(Def::File(file));
        self.bindings.defs.extend(eager.iter().map(|&(sym, _)| Def::Sym(sym)));
        self.line();
        self.out.push_str("let ");
        for (i, &(sym, _)) in eager.iter().enumerate() {
            let name = self.sym_name(sym);
            let _ = write!(self.out, "{}{}v", if i > 0 { ", " } else { "" }, name);
        }
        let _ = write!(self.out, ", {}d = false;", init);
        self.line();
        let _ = write!(self.out, "function {}() ", init);
        self.open("{");
        self.line();
        // A copy of the initialiser that a module took before it rebound itself (`--hot`'s
        // `$hotUse`) still finds the flag.
        let _ = write!(self.out, "if ({0}d) return; {0}d = true; {0} = $nop;", init);
        self.group_inits = eager.iter().map(|&(_, e)| e).collect();
        self.enter_scope(Root::Vals);
        self.open_frame();
        let region = self.init_file.replace(file);
        for &(sym, e) in &eager {
            self.mark_stmt();
            let target = format!("{}v", self.sym_name(sym));
            if self.is_simple(e) {
                self.line();
                let _ = write!(self.out, "{} = ", target);
                self.emit_value(e);
                self.out.push(';');
            } else {
                self.emit_stmt(e, &Ctx::Assign(target));
            }
        }
        self.init_file = region;
        self.close_frame();
        self.leave_scope();
        self.close("}");
        for &(sym, _) in &eager {
            let name = self.sym_name(sym);
            self.line();
            let _ = write!(self.out, "function {0}() {{ {1}(); return {0}v; }}", name, init);
            if self.syms.sym(sym).kind == SymKind::Var {
                self.line();
                let _ = write!(self.out, "function {0}set(v) {{ {1}(); {0}v = v; }}", name, init);
            }
        }
    }

    fn emit_top_val(&mut self, sym: SymId, init: TExprId) {
        let name = self.sym_name(sym);
        self.bindings.defs.push(Def::Sym(sym));
        self.line();
        if self.const_vals[sym.idx()] {
            let _ = write!(self.out, "const {} = ", name);
            self.emit_value(init);
            self.out.push(';');
            return;
        }
        let _ = write!(self.out, "let {0}v, {0}d = false;", name);
        self.line();
        // Read where its file is initialised, as a def is called, but from JavaScript.
        let check = match self.reach.forwarded.get(sym.idx()).copied().unwrap_or(false) {
            true => self.needs_init(sym, None).and_then(|file| self.file_init(file)).map(|init| format!("{}(); ", init)),
            false => None,
        };
        let _ = write!(self.out, "function {0}() {{ {1}if (!{0}d) {{ {0}d = true; ", name, check.as_deref().unwrap_or(""));
        self.enter_scope(Root::Expr(init));
        self.open_frame();
        self.frames.last_mut().unwrap().inline = true;
        let region = self.initialised_in(sym);
        let region = std::mem::replace(&mut self.init_file, region);
        if self.is_simple(init) {
            let _ = write!(self.out, "{}v = ", name);
            self.emit_value(init);
            self.out.push(';');
        } else {
            let target = format!("{}v", name);
            self.emit_stmt(init, &Ctx::Assign(target));
        }
        self.init_file = region;
        self.close_frame();
        self.leave_scope();
        let _ = write!(self.out, " }} return {}v; }}", name);
    }

    /// Emits the parameter list and enters its names into `scope_names`; `leave_params` restores
    /// the names once the body is emitted.
    fn emit_params(&mut self, params: &[SymId], defaults: &[Option<TExprId>]) -> usize {
        self.emit_params_after_captures(0, params, defaults)
    }

    /// The parameter list of a constructor whose first `captures` parameters are the locals a
    /// lifted class captures, `$c0, $c1, ...`, followed by the declared ones.
    fn emit_params_after_captures(&mut self, captures: usize, params: &[SymId], defaults: &[Option<TExprId>]) -> usize {
        let mark = self.scope_names.len();
        self.out.push('(');
        for i in 0..captures {
            let _ = write!(self.out, "{}$c{}", if i > 0 { ", " } else { "" }, i);
            self.declare_fixed(&format!("$c{}", i));
        }
        for (i, &p) in params.iter().enumerate().skip(captures) {
            if i > 0 {
                self.out.push_str(", ");
            }
            let n = self.sym_name(p);
            // A constructor's parameter is a member of its class, which `sym_name` keeps out of
            // the scope's names: a local of the constructor (an expansion's binding in a parent's
            // call or the body) named alike is renamed apart from it, as from a method's.
            let info = self.syms.sym(p);
            if info.owner != Owner::Local && *n == *sanitize(self.interner.get(info.name)) {
                self.scope_names.push(info.name);
            }
            let n = match info.owner {
                Owner::Local => n,
                _ => self.name_ctor_param(p, n),
            };
            self.out.push_str(&n);
            if let Some(Some(d)) = defaults.get(i) {
                self.out.push_str(" = ");
                self.enter_cond();
                // A constructor's default runs before the body: a capture it reads is the
                // parameter; a method's reads it from the instance, as the body does.
                let outer = self.before_super;
                self.before_super = captures > 0 || outer;
                self.emit_value(*d);
                self.before_super = outer;
                self.leave_cond();
            }
        }
        self.out.push(')');
        mark
    }

    fn leave_params(&mut self, mark: usize) {
        self.scope_names.truncate(mark);
    }

    fn emit_function(&mut self, f: FunId, kind: FunKind) {
        let fun = &self.prog.funs[f.idx()];
        let Some(body) = fun.body else { return };
        let name = self.sym_name(fun.sym);
        self.line();
        match kind {
            FunKind::TopLevel => {
                self.bindings.defs.push(Def::Sym(fun.sym));
                let _ = write!(self.out, "function {}", name);
            }
            // A JS class keeps the verbatim names, and JavaScript reads a parameterless def as a
            // property, so it is a getter.
            FunKind::Method if self.syms.js_member(fun.sym) => {
                let info = self.syms.sym(fun.sym);
                if info.sig.as_ref().map_or(false, |sig| sig.clauses.is_empty()) {
                    self.out.push_str("get ");
                }
                let js = self.interner.get(self.syms.js_member_name(fun.sym));
                if is_js_identifier(js) {
                    self.out.push_str(js);
                } else {
                    js_string(js, &mut self.out);
                }
            }
            FunKind::Method => self.out.push_str(&name),
            FunKind::Local => {
                let _ = write!(self.out, "const {} = ", name);
            }
        }
        self.enter_scope(Root::Fun(f));
        let outer_def_scope = self.def_scope.replace(self.scopes.len() - 1);
        let tail = self.tail_loop_of(f, body);
        if let Some(t) = &tail {
            for (&p, name) in fun.params.iter().zip(&t.params) {
                self.sym_names[p.idx()] = Some(name.clone());
                self.note_bound(p, name.clone());
            }
        }
        // A top-level def runs with its file initialised, after the defaults of its parameters
        // where it checks the initialiser itself.
        let entry_check = match kind == FunKind::TopLevel && self.reach.forwarded.get(fun.sym.idx()).copied().unwrap_or(false) {
            true => self.needs_init(fun.sym, None).and_then(|file| self.file_init(file)).map(|init| format!("{}();", init)),
            false => None,
        };
        let (region, defaults_region) = match kind {
            FunKind::Local => (self.init_file, self.init_file),
            FunKind::TopLevel => (self.initialised_in(fun.sym), self.initialised_in(fun.sym).filter(|_| entry_check.is_none())),
            FunKind::Method => (None, None),
        };
        let outer_region = std::mem::replace(&mut self.init_file, defaults_region);
        let mark = self.emit_params(&fun.params, &fun.defaults);
        self.init_file = region;
        self.entry_check = entry_check;
        self.out.push_str(if kind == FunKind::Local { " => " } else { " " });
        let outer_depth = std::mem::replace(&mut self.fn_depth, 0);
        let outer_nonlocal = self.nonlocal_return.take();
        let body_start = self.out.len();
        match tail {
            Some(t) => self.emit_loop_body(t, body),
            None => self.emit_body(body),
        }
        if let Some(key) = self.nonlocal_return.take() {
            self.wrap_nonlocal_returns(body_start, &key);
        }
        self.init_file = outer_region;
        self.fn_depth = outer_depth;
        self.nonlocal_return = outer_nonlocal;
        self.leave_params(mark);
        self.def_scope = outer_def_scope;
        self.leave_scope();
        if kind == FunKind::Local {
            self.out.push(';');
        }
    }

    /// An anonymous class lowered to a closure (`layout::Layout::lower_closures`), created with
    /// `args`: the arrow function of its `apply`, written where the instance is made. The body
    /// reads a captured local as what the creation passes for it, which is the local itself or
    /// `this`; a `return` inside is the arrow function's own, as it was the method's.
    fn emit_closure(&mut self, e: TExprId, idx: usize, args: ListRef) {
        let prog = self.prog;
        let tc = &prog.classes[idx];
        let fun = &prog.funs[tc.methods[0].idx()];
        let body = fun.body.expect("a lowered class has a body");
        if let (true, Some((n, HoleKind::Thunk))) = (fun.params.is_empty(), self.hole_of(body)) {
            let _ = write!(self.out, "$k{}", n);
            return;
        }
        let outer_captures = self.closure_captures.len();
        for (&s, &a) in tc.ctor_params.iter().zip(prog.expr_list(args)) {
            if !matches!(prog.expr(a), TExpr::Local(t) if t == s) {
                self.closure_captures.push((s, a));
            }
        }
        self.enter_scope(Root::Closure(e));
        let outer_def_scope = self.def_scope.replace(self.scopes.len() - 1);
        let mark = self.emit_params(&fun.params, &fun.defaults);
        self.out.push_str(" => ");
        let outer_depth = std::mem::replace(&mut self.fn_depth, 0);
        let outer_nonlocal = self.nonlocal_return.take();
        let returns = self.hole_of(body).is_none() && prog.descendants(body).any(|e| matches!(prog.expr(e), TExpr::Return(_)));
        if !returns && self.is_simple(body) && !matches!(self.peek(body), TExpr::Unit) {
            let needs_parens = matches!(self.peek(body), TExpr::Js(..) | TExpr::Block(..));
            if needs_parens {
                self.out.push('(');
            }
            self.enter_cond();
            self.emit_value(body);
            self.leave_cond();
            if needs_parens {
                self.out.push(')');
            }
        } else {
            let start = self.out.len();
            self.emit_body(body);
            if let Some(key) = self.nonlocal_return.take() {
                self.wrap_nonlocal_returns(start, &key);
            }
        }
        self.fn_depth = outer_depth;
        self.nonlocal_return = outer_nonlocal;
        self.leave_params(mark);
        self.def_scope = outer_def_scope;
        self.leave_scope();
        self.closure_captures.truncate(outer_captures);
    }

    /// Whether the expression creates an anonymous class that is written as a closure.
    pub(super) fn is_closure(&self, e: TExprId) -> bool {
        matches!(self.peek(e), TExpr::New(c, _) if self.closure_anons.contains_key(&c))
    }

    /// A `return` inside a lambda of the def throws `{$nlr: key, v}`; the def's body, written
    /// from `start` on as `{ ... }`, catches what carries its own key.
    fn wrap_nonlocal_returns(&mut self, start: usize, key: &str) {
        let head = format!("{{ const {} = {{}}; try ", key);
        self.out.insert_str(start, &head);
        let _ = write!(self.out, " catch ($e) {{ if ($e.$nlr === {}) return $e.v; throw $e; }} }}", key);
        // The frame around a local def holds the accessor calls of its body by position.
        if let Some(f) = self.frames.last_mut() {
            let shift = |at: &mut usize| {
                if *at >= start {
                    *at += head.len();
                }
            };
            for u in &mut f.uses {
                shift(&mut u.span.0);
                shift(&mut u.span.1);
                if let UseForm::Touch(close) = &mut u.form {
                    shift(close);
                }
            }
            for (pos, end, first, _) in &mut f.edits {
                shift(pos);
                shift(end);
                shift(first);
            }
        }
    }

    /// Emits `{ ... }` for a function body whose value is returned. A `return` inside belongs to
    /// this function, so a tail loop of the enclosing def does not reach in.
    fn emit_body(&mut self, body: TExprId) {
        let tail = self.tail.take();
        self.open("{");
        self.emit_entry_check();
        self.open_frame();
        self.emit_stmt_unbraced(body, &Ctx::Return);
        self.close_frame();
        self.close("}");
        self.tail = tail;
    }

    fn tail_loop_of(&mut self, f: FunId, body: TExprId) -> Option<TailLoop> {
        let fun = &self.prog.funs[f.idx()];
        if !self.syms.is_effectively_final(fun.sym) {
            return None;
        }
        let (calls, on_other_receiver) = self.prog.tail_self_calls(fun.sym, fun.params.len(), body);
        if calls == 0 {
            return None;
        }
        let params = fun.params.clone().into_iter().map(|p| self.tail_param_name(p)).collect();
        let receiver = on_other_receiver.then(|| self.fresh("recv"));
        Some(TailLoop { fun: f, params, receiver })
    }

    fn emit_loop_body(&mut self, tail: TailLoop, body: TExprId) {
        let params = self.prog.funs[tail.fun.idx()].params.clone();
        self.open("{");
        self.emit_entry_check();
        if let Some(recv) = &tail.receiver {
            self.line();
            let _ = write!(self.out, "let {} = this;", recv);
        }
        self.line();
        self.out.push_str("while (true) ");
        self.open("{");
        // The loop's context first: the walk of its scope reads the parameters its jumps assign.
        let receiver = tail.receiver.clone();
        let outer = self.tail.replace(tail);
        let this = if receiver.is_some() { "$self" } else { self.this_name };
        let outer_this = std::mem::replace(&mut self.this_name, this);
        self.enter_scope(Root::Expr(body));
        if receiver.is_some() {
            self.declare_fixed("$self");
        }
        let tail_params = self.tail.as_ref().unwrap().params.clone();
        let mut copies: Vec<(Rc<str>, String)> = Vec::with_capacity(params.len() + 1);
        for (&p, param) in params.iter().zip(&tail_params) {
            let name = self.syms.sym(p).name;
            let plain = self.tail_copy_name(p);
            self.sym_names[p.idx()] = Some(plain.clone());
            // A local of the body named like the parameter (`case acc => loop(i + 1, acc)`)
            // is named apart from the copy.
            self.scope_names.push(name);
            copies.push((plain, param.to_string()));
        }
        if let Some(recv) = &receiver {
            copies.push((Rc::from("$self"), recv.clone()));
        }
        if !copies.is_empty() {
            self.line();
            self.out.push_str("const ");
            for (i, (copy, param)) in copies.iter().enumerate() {
                let _ = write!(self.out, "{}{} = {}", if i > 0 { ", " } else { "" }, copy, param);
            }
            self.out.push(';');
        }
        self.open_frame();
        self.emit_stmt_unbraced(body, &Ctx::Return);
        if !self.always_returns(body) {
            self.line();
            self.out.push_str("return;");
        }
        self.close_frame();
        self.tail = outer;
        self.this_name = outer_this;
        self.leave_scope();
        self.close("}");
        self.close("}");
    }

    fn emit_class(&mut self, idx: usize) {
        let region = self.init_file.take();
        self.emit_class_in(idx);
        self.init_file = region;
    }

    fn emit_class_in(&mut self, idx: usize) {
        let prog = self.prog;
        let tc = &prog.classes[idx];
        let info = self.syms.class(tc.id);
        if matches!(info.kind, ClassKind::Opaque | ClassKind::Builtin) || !self.reach.classes[tc.id.idx()] {
            return;
        }
        // A JS trait only describes plain objects; nothing refers to it at run time.
        if info.js != JsKind::Scala && info.kind == ClassKind::Trait {
            return;
        }
        let name = self.class_name(tc.id);
        self.bindings.defs.push(Def::Class(tc.id));
        self.line();
        let _ = write!(self.out, "class {} ", name);
        // A trait is mixed into its classes, which are the ones to extend its superclass.
        let superclass = info.superclass.filter(|_| info.kind != ClassKind::Trait);
        // Throwable is a native Error, so that instances carry a stack trace.
        let is_throwable = prog.throwable == Some(tc.id);
        if let Some(parent) = superclass {
            let parent = if self.deferred_extends { Rc::from("Object") } else { self.class_name(parent) };
            let _ = write!(self.out, "extends {} ", parent);
        } else if is_throwable {
            self.out.push_str("extends Error ");
        }
        self.open("{");
        let is_object = info.kind == ClassKind::Object;
        let is_anon = info.kind == ClassKind::Anon;
        let n_captures = if is_anon { tc.ctor_params.len() } else { tc.captures };
        if n_captures > 0 {
            self.captures = tc.ctor_params[..n_captures].to_vec();
        }
        let is_enum_value = info.kind == ClassKind::EnumCase && info.singleton.is_some();
        // Instances of a class with a function type among its parents are JS functions: the
        // constructor builds one that forwards to `apply`, gives it the class prototype and
        // returns it in place of `this`.
        let function_arity = info
            .base_types
            .iter()
            .skip(1)
            .find(|&&(b, _)| self.is_function_class(b))
            .map(|&(b, _)| self.syms.class(b).tparams.len() - 1);
        let callable = !matches!(info.kind, ClassKind::Trait | ClassKind::Enum) && function_arity.is_some();
        // The first callable class of a chain makes the function; a subclass continues with
        // the one its superclass constructor returned.
        let super_callable = superclass.map_or(false, |s| {
            let si = self.syms.class(s);
            !matches!(si.kind, ClassKind::Trait | ClassKind::Enum) && si.base_types.iter().skip(1).any(|&(b, _)| self.is_function_class(b))
        });
        let makes_function = callable && !super_callable;
        self.ctor_texts.clear();
        if makes_function {
            self.ctor_texts.push(CALLABLE);
            if self.has_field_named(tc.id, "name") {
                self.ctor_texts.push(WRITABLE_NAME);
            }
            if superclass.is_some() {
                self.ctor_texts.extend(SUPER_FIELDS);
            }
        }
        // A trait is never constructed: its body statements form `$traitInit`, which the
        // constructor of every class implementing it calls in linearisation order.
        let is_trait = info.kind == ClassKind::Trait;
        // The body of a trait with ordinary parameters takes them all as arguments.
        let takes_arguments = is_trait && info.ctor.iter().any(|cl| !cl.is_using);
        // Scala sets the parameter fields of a whole chain of classes before the first body of
        // the chain runs, so that the body of a superclass can read what a subclass takes as a
        // parameter. A class that is extended therefore keeps its body in `$body`, which calls
        // the one above; the constructor of the class being instantiated runs it once every
        // constructor of the chain has set its fields.
        let in_hierarchy = info.js == JsKind::Scala && !is_trait;
        let extended = in_hierarchy && !info.subclasses.is_empty();
        let body_above = in_hierarchy && self.body_above(tc.id);
        let defers_body = extended && self.has_body[tc.id.idx()];
        let runs_body = (body_above || defers_body) && info.mods & mods::ABSTRACT == 0;
        let needs_ctor = if is_trait {
            tc.init.iter().any(|i| match i {
                TInit::Stmt(_) => true,
                TInit::Field(s, _) => self.syms.sym(*s).mods & mods::LAZY == 0,
                TInit::Parent(..) => false,
            }) || takes_arguments
        } else {
            is_object
                || runs_body
                || is_enum_value
                || info.stateful
                || callable
                || !tc.ctor_params.is_empty()
                || (!tc.init.is_empty() && !defers_body)
                || tc.parent_args.is_some()
        };
        if needs_ctor {
            self.line();
            // Cases are no JS subclasses of their enum: they get `$init` mixed in and call it.
            // An enum that extends a class is the superclass of its cases, as any class.
            self.out.push_str(match info.kind {
                ClassKind::Enum if superclass.is_none() => "$init",
                ClassKind::Trait => "$traitInit",
                _ => "constructor",
            });
            let mark = self.scope_names.len();
            self.enter_scope(Root::Ctor(idx as u32));
            if is_enum_value {
                self.declare_fixed("$name");
                self.declare_fixed("$ordinal");
                self.out.push_str("($name, $ordinal) ");
            } else if is_trait && !takes_arguments {
                self.out.push_str("() ");
            } else if is_anon {
                self.out.push('(');
                for i in 0..tc.ctor_params.len() {
                    let _ = write!(self.out, "{}$c{}", if i > 0 { ", " } else { "" }, i);
                    self.declare_fixed(&format!("$c{}", i));
                }
                // What an anonymous class passes to its superclass is evaluated where it is created.
                for i in 0..tc.parent_args.map_or(0, |args| args.len) {
                    let _ = write!(self.out, "{}$p{}", if i > 0 || !tc.ctor_params.is_empty() { ", " } else { "" }, i);
                    self.declare_fixed(&format!("$p{}", i));
                }
                self.out.push_str(") ");
            } else if n_captures > 0 {
                self.emit_params_after_captures(n_captures, &tc.ctor_params, &tc.ctor_defaults);
                self.out.push(' ');
            } else {
                self.emit_params(&tc.ctor_params, &tc.ctor_defaults);
                self.out.push(' ');
            }
            self.open("{");
            if let Some(n) = function_arity.filter(|_| makes_function) {
                self.line();
                // Named parameters give the function the arity a type test reads from `length`.
                let params = (0..n).map(|i| format!("$a{}", i)).collect::<Vec<_>>().join(", ");
                // `new.target` is the class being constructed, which a subclass's constructor
                // continues with.
                let _ = write!(self.out, "const $f = ({0}) => $f.apply({0}); ", params);
                self.out.push_str(CALLABLE);
                self.declare_fixed("$f");
                // A field named like the function's own read-only `name` is assigned in its place.
                if self.has_field_named(tc.id, "name") {
                    self.out.push_str(WRITABLE_NAME);
                }
                self.this_name = "$f";
            }
            let this = self.this_name;
            if !is_anon {
                self.emit_block_stmts(tc.parent_prelude);
            }
            if is_throwable {
                // The message reaches `Error`, so JavaScript code and stack traces see it.
                let message = tc.ctor_params.first().map(|&p| self.ctor_param(p));
                self.line();
                match message {
                    Some(m) => {
                        let _ = write!(self.out, "super({} ?? undefined);", m);
                    }
                    None => self.out.push_str("super();"),
                }
            }
            let mut via_inits: Vec<String> = Vec::new();
            // Before `super()` a constructor reads its parameters and captures as such: `this`
            // does not exist yet.
            let outer_locals = std::mem::replace(&mut self.ctor_locals, tc.ctor_params.clone());
            self.before_super = true;
            if let (Some(via), true) = (tc.parent_via, superclass.is_some()) {
                via_inits = self.emit_super_via(tc, via, is_anon);
            } else if superclass.is_some() && !makes_function {
                // A callable class returns the function in place of `this`, which a
                // superclass constructor could not have made; it constructs nothing anyway.
                self.line();
                self.out.push_str("super");
                match tc.parent_args {
                    Some(args) if !is_anon => self.emit_ctor_args(superclass.unwrap(), args),
                    Some(args) => {
                        self.out.push('(');
                        for i in 0..args.len {
                            let _ = write!(self.out, "{}$p{}", if i > 0 { ", " } else { "" }, i);
                        }
                        self.out.push(')');
                    }
                    None => self.out.push_str("()"),
                }
                self.out.push(';');
            } else if let (Some(parent), true) = (superclass, makes_function) {
                // The superclass of the first callable class of a chain is constructed apart and
                // its fields copied onto the function (zio's `Callback extends AtomicBoolean(false)`);
                // it is read from the prototype chain, which a split build links after loading.
                let own_name = self.class_name(tc.id);
                self.line();
                let _ = write!(self.out, "{}{}{}{}{}", SUPER_FIELDS[0], this, SUPER_FIELDS[1], own_name, SUPER_FIELDS[2]);
                match tc.parent_args {
                    Some(args) if !is_anon => self.emit_ctor_args(parent, args),
                    Some(args) => {
                        self.out.push('(');
                        for i in 0..args.len {
                            let _ = write!(self.out, "{}$p{}", if i > 0 { ", " } else { "" }, i);
                        }
                        self.out.push(')');
                    }
                    None => self.out.push_str("()"),
                }
                self.out.push_str(", new.target));");
            }
            self.before_super = false;
            self.ctor_locals = outer_locals;
            if is_object {
                self.line();
                let _ = write!(self.out, "{}i = {};", &name[..name.len() - 1], this);
                self.emit_stateful_enum_values(tc.id);
            }
            if is_enum_value {
                self.line();
                let _ = write!(self.out, "{0}.$name = $name; {0}.$ordinal = $ordinal;", this);
            }
            for (i, &p) in tc.ctor_params.iter().enumerate().filter(|_| !is_trait || takes_arguments) {
                let captured = i < n_captures;
                if !captured && !info.value_class && !self.param_needs_field(p, defers_body) {
                    self.ctor_locals.push(p);
                    continue;
                }
                self.line();
                if captured {
                    let field = self.capture_field(i, p);
                    let _ = write!(self.out, "{}.{} = $c{};", this, field, i);
                } else {
                    let field = self.field_access(p);
                    let param = self.ctor_param(p);
                    let _ = write!(self.out, "{}{} = {};", this, field, param);
                }
            }
            if let Some(args) = tc.parent_args.filter(|_| superclass.is_none()) {
                self.line();
                let _ = write!(self.out, "{}.$init", this);
                self.emit_args(args);
                self.out.push(';');
            }
            // The statements after the self calls of the superclass's secondary constructors run
            // once its body has: with the deferred body of this class, or in its place.
            if defers_body {
                for (i, call) in via_inits.iter().enumerate() {
                    self.line();
                    let _ = write!(self.out, "{}.$via{} = () => {};", this, i, call.trim_end_matches(';'));
                }
            }
            if runs_body {
                self.line();
                if extended {
                    let _ = write!(self.out, "if (new.target === {}) ", name);
                }
                let _ = write!(self.out, "{}.$body();", this);
            }
            if !defers_body {
                for call in &via_inits {
                    self.line();
                    self.out.push_str(call);
                }
                self.emit_inits(tc, this);
            }
            if makes_function {
                self.line();
                self.out.push_str("return $f;");
                self.this_name = "this";
            }
            self.leave_scope();
            self.close("}");
            self.leave_params(mark);
            self.ctor_locals.clear();
            self.ctor_bindings.clear();
        }
        if defers_body {
            self.line();
            self.out.push_str("$body() ");
            self.open("{");
            self.enter_scope(Root::Ctor(idx as u32));
            if body_above {
                self.line();
                self.out.push_str("super.$body();");
            }
            if tc.parent_via.is_some() {
                let n = tc.parent_via.map_or(0, |via| layout::ctor_chain(self.prog, self.syms, via).len());
                for i in 0..n {
                    self.line();
                    let _ = write!(self.out, "this.$via{0}?.(); delete this.$via{0};", i);
                }
            }
            self.body_params = tc.ctor_params.clone();
            self.emit_inits(tc, "this");
            self.body_params.clear();
            self.leave_scope();
            self.close("}");
        }
        for &p in tc.ctor_params.iter().chain(tc.init.iter().filter_map(|i| match i {
            TInit::Field(s, _) => Some(s),
            _ => None,
        })) {
            if self.syms.sym(p).needs_accessor {
                let n = self.sym_name(p);
                self.line();
                let _ = write!(self.out, "{0}() {{ return this.${0}; }}", n);
                if self.syms.sym(p).needs_setter {
                    let setter = self.setter_name(p);
                    self.line();
                    let _ = write!(self.out, "{}(v) {{ this.${} = v; }}", setter, n);
                }
            }
        }
        // A lazy val is a getter that replaces itself with the computed value.
        for init in &tc.init {
            let TInit::Field(sym, e) = init else { continue };
            if self.syms.sym(*sym).mods & mods::LAZY == 0 {
                continue;
            }
            let field = self.field_name(*sym);
            self.line();
            let _ = write!(self.out, "get {}() ", field);
            self.open("{");
            self.enter_scope(Root::Getter(*e));
            self.declare_fixed("$v");
            self.line();
            self.out.push_str("let $v;");
            self.emit_stmt(*e, &Ctx::Assign("$v".to_string()));
            self.line();
            self.out.push_str(LAZY_FIELD);
            let _ = write!(self.out, "\"{}\", {{ value: $v }});", field);
            self.line();
            self.out.push_str("return $v;");
            self.leave_scope();
            self.close("}");
        }
        let mut methods = tc.methods.clone();
        methods.sort_by(|&a, &b| layout::compare_syms(self.syms, self.interner, prog.funs[a.idx()].sym, prog.funs[b.idx()].sym));
        for m in methods {
            if self.reach.funs[m.idx()] {
                self.emit_function(m, FunKind::Method);
            }
        }
        self.emit_object_members(tc.id);
        self.emit_forwarders(tc);
        self.emit_bridges(tc);
        self.emit_super_accessors(tc);
        self.emit_secondary_ctors(tc);
        if info.value_class && info.mods & mods::CASE == 0 {
            self.emit_value_class_equality(tc, &name);
        }
        self.close("}");
        self.captures.clear();
        if is_object {
            let accessor = self.module_accessor(tc.id);
            self.line();
            let _ = write!(self.out, "let {0}i; function {0}() {{ return {0}i ?? ", accessor);
            if self.hot {
                let _ = write!(self.out, "$hotObj(new {}(), ", name);
                js_string(&self.qualified_name(tc.id), &mut self.out);
                self.out.push_str("); }");
            } else {
                let _ = write!(self.out, "new {}(); }}", name);
            }
            self.accessors.push(accessor);
        }
    }

    /// A plain constructor parameter is kept in a field when something reads it as one: a
    /// method, or the body of the class where that runs apart from the constructor.
    fn param_needs_field(&self, p: SymId, defers_body: bool) -> bool {
        self.syms.sym(p).mods & mods::FIELD != 0
            || self.reach.fields_read[p.idx()]
            || (defers_body && self.reach.init_read[p.idx()])
    }

    /// Whether a class above `c` keeps a body in `$body`.
    fn body_above(&self, c: ClassId) -> bool {
        let mut at = self.syms.class(c).superclass;
        let mut steps = 0;
        while let Some(s) = at {
            if self.has_body[s.idx()] && self.syms.class(s).js == JsKind::Scala {
                return true;
            }
            steps += 1;
            if steps > self.syms.classes.len() {
                return false;
            }
            at = self.syms.class(s).superclass;
        }
        false
    }

    /// The field initialisers, statements and trait bodies that the construction of `tc` runs.
    fn emit_inits(&mut self, tc: &TClass, this: &str) {
        for init in &tc.init {
            match init {
                TInit::Field(sym, _) if self.syms.sym(*sym).mods & mods::LAZY != 0 => {}
                // The field may hold what a subclass took as a parameter in its place.
                TInit::Field(sym, e) if self.syms.sym(*sym).overridden_by_param => {
                    let field = self.field_name(*sym);
                    let value = self.fresh("o");
                    self.line();
                    let _ = write!(self.out, "let {};", value);
                    self.emit_stmt(*e, &Ctx::Assign(value.clone()));
                    self.line();
                    let _ = write!(self.out, "if (!{}{}, ", OWN_FIELD, this);
                    js_string(&field, &mut self.out);
                    let _ = write!(self.out, ")) {}.{} = {};", this, field, value);
                }
                TInit::Field(sym, e) => {
                    let field = self.field_access(*sym);
                    if self.is_simple(*e) {
                        self.line();
                        let _ = write!(self.out, "{}{} = ", this, field);
                        self.emit_value(*e);
                        self.out.push(';');
                    } else {
                        self.emit_stmt(*e, &Ctx::Assign(format!("{}{}", this, field)));
                    }
                }
                TInit::Stmt(e) => self.emit_stmt(*e, &Ctx::Discard),
                TInit::Parent(b, call) => {
                    let trait_name = self.class_name(*b);
                    let scoped = !call.prelude.is_empty();
                    if scoped {
                        self.line();
                        self.open("{");
                        let root = self.scopes.last().map_or(Root::Expr(TExprId(0)), |s| s.root);
                        self.enter_scope(root);
                        self.emit_block_stmts(call.prelude);
                    }
                    self.line();
                    let _ = write!(self.out, "{}.prototype.$traitInit.call({}", trait_name, this);
                    self.emit_more_args(call.args, ", ");
                    self.out.push(';');
                    if scoped {
                        self.leave_scope();
                        self.close("}");
                    }
                }
            }
        }
    }

    /// A nested object that implements a member of a parent trait is reached the way the trait
    /// declared it: as a property for a `val`, through a method for a `def`.
    fn emit_object_members(&mut self, c: ClassId) {
        for (declared, nested) in implemented_nested_objects(self.syms, c) {
            let js = self.sym_name(declared);
            let accessor = self.module_accessor(nested);
            let form = if self.syms.sym(declared).kind == SymKind::Def { "" } else { "get " };
            self.line();
            let _ = write!(self.out, "{}{}() {{ return {}(); }}", form, js, accessor);
        }
    }

    /// An abstract member that an export clause implements gets a real member, since the code of
    /// the trait reaches it through `this`: a method for a `def`, a getter for a `val`.
    fn emit_bridges(&mut self, tc: &TClass) {
        for &(declared, implementation) in &tc.bridges {
            if !self.reach.bridged[implementation.idx()] {
                continue;
            }
            let from = self.sym_name(declared);
            let to = self.sym_name(implementation);
            self.line();
            let _ = write!(self.out, "{}(...$a) {{ return this.{}(...$a); }}", from, to);
        }
    }

    /// The members an export clause makes, each a call of its target; a top-level target's
    /// access runs its file's initialiser first, after the forwarder's arguments, as the
    /// forwarder of scalac's does.
    fn emit_forwarders(&mut self, tc: &TClass) {
        for &(member, target) in &tc.forwarders {
            let name = self.sym_name(member);
            let value = self.static_ref(target);
            let member_is_def = self.syms.sym(member).kind == SymKind::Def;
            let target_is_def = self.syms.sym(target).kind == SymKind::Def;
            let check = match is_eager_top_val(self.syms, target, self.const_vals) {
                true => None,
                false => self.needs_init(target, None).and_then(|file| self.file_init(file)),
            };
            let check = check.map_or(String::new(), |init| format!("{}(); ", init));
            self.line();
            let _ = match (member_is_def, target_is_def) {
                (true, true) => write!(self.out, "{}(...$a) {{ {}return {}(...$a); }}", name, check, value),
                (true, false) => write!(self.out, "{}() {{ {}return {}; }}", name, check, value),
                (false, true) => write!(self.out, "get {}() {{ {}return {}(); }}", name, check, value),
                (false, false) => write!(self.out, "get {}() {{ {}return {}; }}", name, check, value),
            };
        }
    }

    /// A value class equals and hashes as its one field does, as it does on the JVM, where it
    /// is that field.
    fn emit_value_class_equality(&mut self, tc: &TClass, name: &str) {
        let Some(&field) = tc.ctor_params.first() else { return };
        let defines = |t: &Self, n: Name| t.syms.class(tc.id).members.contains_key(&n);
        let access = self.field_access(field);
        if !defines(self, crate::names::EQUALS) {
            self.line();
            let _ = write!(self.out, "equals(that) {{ return that instanceof {0} && $eq(this{1}, that{1}); }}", name, access);
        }
        if !defines(self, crate::names::HASH_CODE) {
            self.line();
            let _ = write!(self.out, "hashCode() {{ return $hash(this{}); }}", access);
        }
    }

    /// `C.$new3`: the static factory of the secondary constructor `s`, the third one of `C`.
    pub(super) fn ctor_factory(&mut self, s: SymId) -> String {
        let Owner::Class(c) = self.syms.sym(s).owner else { return String::new() };
        let k = self.ctor_number(c, s);
        format!("{}.$new{}", self.class_name(c), k)
    }

    fn ctor_number(&self, c: ClassId, s: SymId) -> usize {
        self.syms.class(c).ctors.iter().position(|&x| x == s).map_or(0, |i| i + 1)
    }

    /// A secondary constructor is a static factory that evaluates the arguments of its self
    /// call, makes the instance through the constructor the call goes to and runs the statements
    /// after the call through a method of the instance.
    fn emit_secondary_ctors(&mut self, tc: &TClass) {
        for (i, &f) in tc.ctors.iter().enumerate() {
            if !self.reach.funs[f.idx()] {
                continue;
            }
            let fun = &self.prog.funs[f.idx()];
            let Some(TExpr::Block(stmts, after)) = fun.body.map(|b| self.prog.expr(b)) else { continue };
            let (params, defaults) = (fun.params.clone(), fun.defaults.clone());
            let Some(TStmt::Expr(call)) = stmts.len.checked_sub(1).map(|j| self.prog.stmts[stmts.start as usize + j as usize]) else { continue };
            let has_after = !matches!(self.prog.expr(after), TExpr::Unit);
            let k = i + 1;
            self.line();
            let _ = write!(self.out, "static $new{}", k);
            self.enter_scope(Root::Fun(f));
            if has_after {
                self.declare_fixed("$t");
            }
            let mark = self.emit_params(&params, &defaults);
            self.out.push(' ');
            self.open("{");
            self.emit_block_stmts(ListRef { start: stmts.start, len: stmts.len - 1 });
            self.line();
            if has_after {
                self.out.push_str("const $t = ");
                self.emit_expr(call);
                let names: Vec<Rc<str>> = params.iter().map(|&p| self.sym_name(p)).collect();
                let _ = write!(self.out, "; $t.$init{}({}); return $t;", k, names.join(", "));
            } else {
                self.out.push_str("return ");
                self.emit_expr(call);
                self.out.push(';');
            }
            self.close("}");
            self.leave_params(mark);
            self.leave_scope();
            if has_after {
                self.line();
                let _ = write!(self.out, "$init{}", k);
                self.enter_scope(Root::Fun(f));
                let mark = self.emit_params(&params, &[]);
                self.out.push(' ');
                let outer_depth = std::mem::replace(&mut self.fn_depth, 0);
                self.open("{");
                self.emit_stmt(after, &Ctx::Discard);
                self.close("}");
                self.fn_depth = outer_depth;
                self.leave_params(mark);
                self.leave_scope();
            }
        }
    }

    /// `super(...)` for a parent call that goes to a secondary constructor of the superclass:
    /// the parameters of that constructor are bound to the arguments, then those of the
    /// constructor its self call goes to, down to the primary constructor's `super(...)`; a
    /// self call's arguments cannot refer to `this`, so they may run before it. Returns the
    /// calls that run the statements after each self call, innermost first, for the caller to
    /// place once the superclass's body has run.
    fn emit_super_via(&mut self, tc: &TClass, via: SymId, anon: bool) -> Vec<String> {
        let mut inits: Vec<String> = Vec::new();
        let mut placeholders: Vec<String> = Vec::new();
        let mut args: Vec<TExprId> = Vec::new();
        match tc.parent_args {
            Some(l) if anon => placeholders = (0..l.len).map(|i| format!("$p{}", i)).collect(),
            Some(l) => args = self.prog.expr_list(l).to_vec(),
            None => {}
        }
        let chain = layout::ctor_chain(self.prog, self.syms, via);
        let mut cur = via;
        // The temporaries for the parent constructor's parameters take fresh names next to
        // the class's own, which the parameters go by while the chain is emitted.
        let mut renamed: Vec<(SymId, Option<Rc<str>>)> = Vec::new();
        for f in chain {
            let fun = &self.prog.funs[f.idx()];
            let (params, defaults) = (fun.params.clone(), fun.defaults.clone());
            let Some(TExpr::Block(stmts, after)) = fun.body.map(|b| self.prog.expr(b)) else { break };
            let mut names: Vec<Rc<str>> = Vec::new();
            for (i, &p) in params.iter().enumerate() {
                // The parameter is no binding of the output: its name is the stem of the temporary's.
                let base = self.unbound_name(p);
                let n: Rc<str> = Rc::from(self.fresh(&base));
                renamed.push((p, self.sym_names[p.idx()].replace(n.clone())));
                self.line();
                let _ = write!(self.out, "const {} = ", n);
                let written = args.get(i).copied().filter(|&a| !matches!(self.prog.expr(a), TExpr::Unit));
                match (placeholders.get(i), written, defaults.get(i).copied().flatten()) {
                    (Some(text), _, _) => self.out.push_str(text),
                    (None, Some(a), _) => self.emit_expr(a),
                    (None, None, Some(d)) => self.emit_expr(d),
                    (None, None, None) => self.out.push_str("undefined"),
                }
                self.out.push(';');
                names.push(n);
            }
            placeholders.clear();
            self.emit_block_stmts(ListRef { start: stmts.start, len: stmts.len - 1 });
            if !matches!(self.prog.expr(after), TExpr::Unit) {
                let Owner::Class(owner) = self.syms.sym(cur).owner else { break };
                let k = self.ctor_number(owner, cur);
                inits.push(format!("this.$init{}({});", k, names.join(", ")));
            }
            let Some(TStmt::Expr(call)) = stmts.len.checked_sub(1).map(|j| self.prog.stmts[stmts.start as usize + j as usize]) else { break };
            match self.prog.expr(call) {
                TExpr::NewVia(next, l) => {
                    cur = next;
                    args = self.prog.expr_list(l).to_vec();
                }
                TExpr::New(_, l) => {
                    self.line();
                    self.out.push_str("super");
                    self.emit_args(l);
                    self.out.push(';');
                    break;
                }
                _ => break,
            }
        }
        for (p, saved) in renamed {
            self.sym_names[p.idx()] = saved;
        }
        inits.reverse();
        inits
    }

    /// The method through which the trait `tr` reaches `super.member`.
    pub(super) fn super_accessor_name(&mut self, tr: ClassId, member: SymId) -> String {
        format!("{}$super{}", self.sym_name(member), self.class_number(tr))
    }

    fn emit_super_accessors(&mut self, tc: &TClass) {
        let superclass = self.syms.class(tc.id).superclass.map(|s| self.syms.class(s));
        for a in &tc.super_accessors {
            let name = self.super_accessor_name(a.of_trait, a.member);
            let Some(target) = a.target else {
                let body = match self.interner.get(self.syms.sym(a.member).name) {
                    "equals" => "(that) { return this === that; }",
                    "hashCode" => "() { return $identityHash(this); }",
                    "clone" => "() { return $cloneObject(this); }",
                    _ => "() { return $anyStr(this); }",
                };
                self.line();
                let _ = write!(self.out, "{}{}", name, body);
                continue;
            };
            let Owner::Class(owner) = self.syms.sym(target).owner else { continue };
            let target = self.sym_name(target);
            self.line();
            if superclass.map_or(false, |s| s.base_types.iter().any(|&(b, _)| b == owner)) {
                let _ = write!(self.out, "{}(...$a) {{ return super.{}(...$a); }}", name, target);
            } else {
                let class = self.class_name(owner);
                let _ = write!(self.out, "{}(...$a) {{ return {}.prototype.{}.call(this, ...$a); }}", name, class, target);
            }
        }
    }

    /// The JS expression of a definition reached by its path: a member of an object, or a
    /// top-level one. A def is named, not called.
    fn static_ref(&mut self, s: SymId) -> String {
        let info = self.syms.sym(s);
        let n = self.sym_name(s);
        match info.owner {
            Owner::Class(a) => {
                let accessor = self.module_accessor(a);
                let read = if info.kind != SymKind::Def && info.needs_accessor { "()" } else { "" };
                format!("{}().{}{}", accessor, n, read)
            }
            _ if info.kind == SymKind::Def || self.const_vals[s.idx()] => n.to_string(),
            _ => format!("{}()", n),
        }
    }

    fn emit_registration(&mut self, idx: usize) {
        let prog = self.prog;
        let tc = &prog.classes[idx];
        let info = self.syms.class(tc.id);
        if matches!(info.kind, ClassKind::Opaque | ClassKind::Builtin) || !self.reach.classes[tc.id.idx()] {
            return;
        }
        let superclass = info.superclass.filter(|_| info.kind != ClassKind::Trait);
        if let (Some(parent), true) = (superclass, self.deferred_extends) {
            self.hold(parent, Held::Parent);
            let (name, parent) = (self.class_name(tc.id), self.class_name(parent));
            self.line();
            let _ = write!(self.out, "$ext({}, {});", name, parent);
        }
        // Instances of a JS class are plain objects: no class name, trait markers or equality.
        if info.js != JsKind::Scala {
            return;
        }
        let name = self.class_name(tc.id);
        // A subclass of a case class is a product of that class, whose name it prints and hashes.
        let case_ancestor = info.superclass.filter(|_| info.mods & mods::CASE == 0).and_then(|_| {
            info.base_types.iter().skip(1).map(|&(b, _)| self.syms.class(b)).find(|b| {
                b.kind == ClassKind::Class && b.mods & mods::CASE != 0
            })
        });
        // A class that stands for others goes by the name they share, which no edit of a site moves.
        let display = match self.shared.groups.get(&tc.id) {
            Some(group) if case_ancestor.is_none() => group.name.as_str(),
            _ => match case_ancestor {
                Some(b) => b.product_name(self.interner.get(b.name)),
                None => info.product_name(self.interner.get(info.name)),
            },
        };
        self.line();
        // What the superclass registered reaches its subclasses through the prototype chain.
        let superclass = superclass.map(|s| self.syms.class(s));
        // The linearisation of the superclass usually is the tail of the one of the class.
        let own_len = superclass.map_or(info.base_types.len(), |s| {
            let tail = info.base_types.len().saturating_sub(s.base_types.len());
            if info.base_types.get(tail).map(|&(b, _)| b) == s.base_types.first().map(|&(b, _)| b) { tail } else { 0 }
        });
        let own: Vec<ClassId> = match (superclass, own_len) {
            (Some(s), 0) => {
                let inherited = |b: ClassId| s.base_types.iter().any(|&(x, _)| x == b);
                info.base_types.iter().map(|&(b, _)| b).filter(|&b| !inherited(b)).collect()
            }
            _ => info.base_types[..own_len].iter().map(|&(b, _)| b).collect(),
        };
        let _ = write!(self.out, "$cls({}, ", name);
        let shown = if self.is_tuple_name(display) { "" } else { display };
        js_string(shown, &mut self.out);
        let is_case = info.mods & mods::CASE != 0 && info.singleton.is_none();
        let mut shape = String::new();
        if info.singleton.is_some() {
            shape.push('3');
        } else if matches!(info.kind, ClassKind::Trait | ClassKind::Enum) {
            shape.push('4');
        } else if is_case && (info.kind == ClassKind::Object || info.local_module.is_some() || info.inner_object.is_some()) {
            shape.push('2');
        } else if is_case {
            shape.push('[');
            let fields: Vec<SymId> = info.ctor.first().map(|c| c.params.iter().map(|p| p.sym).collect()).unwrap_or_default();
            for (i, f) in fields.iter().enumerate() {
                if i > 0 {
                    shape.push_str(", ");
                }
                let n = self.field_name(*f);
                js_string(&n, &mut shape);
            }
            shape.push(']');
        }
        let mut traits = String::new();
        let mut numbers: Vec<u32> = Vec::new();
        for (i, &b) in own[1..].iter().enumerate() {
            traits.push_str(if i > 0 { ", " } else { "[" });
            self.hold(b, Held::Parent);
            traits.push_str(&self.class_name(b));
            // A base no type test asks for (a builtin the library's class stands behind) has no
            // number, and no marker to set.
            if let Some(n) = self.numbered(b) {
                numbers.push(n);
            }
        }
        // An enum that extends a class is a JS class its cases extend, tested for like a trait:
        // its own number goes on its prototype, which the cases inherit.
        if info.kind == ClassKind::Enum && superclass.is_some() {
            numbers.push(self.class_number(tc.id));
            if traits.is_empty() {
                traits.push('[');
            }
        }
        let mut ids = String::new();
        if !traits.is_empty() {
            traits.push(']');
            ids.push('[');
            for (i, n) in numbers.iter().enumerate() {
                let _ = write!(ids, "{}{}", if i > 0 { ", " } else { "" }, n);
            }
            ids.push(']');
        }
        let ordinal = if info.kind == ClassKind::EnumCase && info.singleton.is_none() {
            info.ordinal.to_string()
        } else {
            String::new()
        };
        let has_fields = shape.starts_with('[');
        let mut args: Vec<(String, &str)> = vec![(shape, "0"), (traits, "[]"), (ids, "[]"), (ordinal, "")];
        while args.last().map_or(false, |(a, _)| a.is_empty()) {
            args.pop();
        }
        for (arg, empty) in &args {
            let _ = write!(self.out, ", {}", if arg.is_empty() { *empty } else { arg.as_str() });
        }
        self.out.push_str(");");
        // The registration gives a case class the three methods it does not define itself; where
        // the superclass has one, scalac makes none, so the inherited one shows again.
        for (bit, method) in ["toString", "equals", "hashCode"].into_iter().enumerate() {
            if tc.inherited_case_members & (1 << bit) != 0 {
                let _ = write!(self.out, " delete {}.prototype.{};", name, method);
            }
        }
        // An instance of a subclass may equal one of the case class itself, as under scalac.
        let defines_equals = info.base_types.iter().any(|&(b, _)| self.syms.class(b).members.contains_key(&crate::names::EQUALS));
        if is_case && !info.subclasses.is_empty() && !defines_equals {
            let _ = write!(self.out, " {0}.prototype.equals = $subEquals({0});", name);
        }
        if prog.partial_function == Some(tc.id) {
            let _ = write!(self.out, " $pfInit({});", name);
        }
        // The field names `productElementName` answers, by their Scala names.
        if has_fields && self.reach.uses_element_names {
            let _ = write!(self.out, " {}.prototype.$names = [", name);
            let fields: Vec<SymId> = info.ctor.first().map(|c| c.params.iter().map(|p| p.sym).collect()).unwrap_or_default();
            for (i, f) in fields.iter().enumerate() {
                if i > 0 {
                    self.out.push_str(", ");
                }
                js_string(self.interner.get(self.syms.sym(*f).name), &mut self.out);
            }
            self.out.push_str("];");
        }
        let throwable = prog.throwable.filter(|&t| info.base_types.iter().any(|&(b, _)| b == t));
        // The qualified name is what `getClass.getName` and an exception's `name` read.
        if throwable.is_some() || prog.uses_get_class() || self.reach.uses_class_of {
            let _ = write!(self.out, " {}.prototype.$qname = ", name);
            let qualified = self.qualified_name(tc.id);
            js_string(&qualified, &mut self.out);
            self.out.push(';');
        }
        if let Some(t) = throwable {
            if t == tc.id {
                let _ = write!(
                    self.out,
                    " Object.defineProperty({}.prototype, \"name\", {{ get() {{ return this.$qname; }} }});",
                    name
                );
            }
            if self.runtime_thrown(tc.id) {
                let _ = write!(self.out, " $exc.{} = {};", self.interner.get(info.name), name);
            }
        }
        if let Ok(i) = self.reach.reflective.binary_search_by_key(&tc.id, |&(c, _)| c) {
            let e = self.reach.reflective[i].1;
            self.out.push(' ');
            self.emit_value(e);
            self.out.push(';');
        }
    }

    /// `classOf[C]`: the `Class` of an emitted Scala class is the one `getClass` gives its
    /// instances; a builtin or a class the output has no constructor for is named.
    pub(super) fn emit_class_of(&mut self, c: ClassId) {
        let info = self.syms.class(c);
        let emitted = info.kind != ClassKind::Builtin && info.js == JsKind::Scala && self.reach.classes.get(c.idx()).copied().unwrap_or(false);
        if emitted {
            let name = self.class_name(c);
            if info.kind == ClassKind::Trait && self.reach.uses_assignable_from {
                let _ = write!(self.out, "$traitClass({}, {})", name, self.class_number(c));
            } else {
                let _ = write!(self.out, "$classValue({})", name);
            }
            return;
        }
        if let Some(builtin) = class_of_name(self.syms, self.interner, c) {
            self.out.push_str("$classNamed(");
            js_string(&builtin, &mut self.out);
            self.out.push(')');
            return;
        }
        let qualified = self.qualified_name(c);
        if info.js != JsKind::Scala {
            self.out.push_str("$classNamed(");
            js_string(&qualified, &mut self.out);
            self.out.push(')');
            return;
        }
        // A class a reflective registration names without reaching it keeps its ancestors.
        self.out.push_str("$classData(");
        js_string(&qualified, &mut self.out);
        self.out.push_str(", [");
        let ancestors: Vec<ClassId> = info.base_types.iter().skip(1).map(|&(b, _)| b).filter(|&b| self.syms.class(b).kind != ClassKind::Builtin).collect();
        for (i, b) in ancestors.into_iter().enumerate() {
            if i > 0 {
                self.out.push_str(", ");
            }
            let name = self.qualified_name(b);
            js_string(&name, &mut self.out);
        }
        self.out.push_str("])");
    }

    /// The name `getClass.getName` gives: packages joined with dots, the enclosing classes with
    /// `$`, as scalac names classes.
    fn qualified_name(&self, c: ClassId) -> String {
        let info = self.syms.class(c);
        let mut s = match info.owner {
            Owner::Package(p) => self.package_path(p),
            Owner::Class(o) => {
                let mut outer = self.qualified_name(o);
                if !outer.ends_with('$') {
                    outer.push('$');
                }
                outer
            }
            Owner::Local => String::new(),
        };
        match self.shared.groups.get(&c) {
            Some(group) => s.push_str(&group.name),
            None => s.push_str(self.interner.get(info.name)),
        }
        if info.kind == ClassKind::Object || beside_companion(self.interner, &info) {
            s.push('$');
        }
        s
    }

    fn package_path(&self, p: PkgId) -> String {
        let info = self.syms.pkg(p);
        match info.parent {
            Some(parent) => format!("{}{}.", self.package_path(parent), self.interner.get(info.name)),
            None => String::new(),
        }
    }

    /// Whether the runtime throws instances of this class of the standard library (`$exc`).
    fn runtime_thrown(&self, c: ClassId) -> bool {
        let info = self.syms.class(c);
        let Owner::Package(p) = info.owner else { return false };
        let pkg = self.interner.get(self.syms.pkg(p).name);
        matches!(pkg, "lang" | "scala" | "js" | "util")
            && reach::RUNTIME_THROWN.contains(&self.interner.get(info.name))
    }

    /// The run-time number of a class the output tests for, if it is one.
    fn numbered(&self, c: ClassId) -> Option<u32> {
        let n = self.class_numbers[c.idx()];
        (n != layout::UNNUMBERED).then_some(n)
    }

    /// The run-time number of a class, which its registration and the type tests carry.
    fn class_number(&self, c: ClassId) -> u32 {
        let n = self.class_numbers[c.idx()];
        debug_assert!(n != layout::UNNUMBERED, "class {} is referred to but not numbered", self.interner.get(self.syms.class(c).name));
        n
    }

    pub(super) fn is_tuple_name(&self, name: &str) -> bool {
        name.strip_prefix("Tuple").map_or(false, |rest| !rest.is_empty() && rest.bytes().all(|b| b.is_ascii_digit()))
    }

    fn is_function_class(&self, c: ClassId) -> bool {
        is_function_class(self.syms, self.interner, c)
    }

    fn enum_of_case(&self, case: ClassId) -> Option<&'a ClassInfo> {
        layout::enum_of_case(self.syms, case)
    }

    /// The module-level constant holding the value of an enum case, `Color$Red`.
    fn enum_value_name(&mut self, sym: SymId) -> String {
        self.sym_name(sym);
        self.enum_value_text(sym)
    }

    pub(super) fn enum_value_text(&mut self, sym: SymId) -> String {
        let info = self.syms.sym(sym);
        let (file, owner) = (info.file, info.owner);
        let mut bare = String::new();
        self.class_prefix(owner, &mut bare);
        bare.push_str(&self.sym_text(sym));
        if self.naming.qualified(self.naming.syms[sym.idx()], file, self.scope) {
            self.qualify(&bare, owner).to_string()
        } else {
            bare
        }
    }

    /// The values of a stateful enum are fields of its companion, created in declaration order
    /// before the companion's own initialisers run.
    fn emit_stateful_enum_values(&mut self, companion: ClassId) {
        let syms = self.syms;
        let Some(e) = syms.class(companion).companion.map(|e| syms.class(e)) else { return };
        if !e.stateful {
            return;
        }
        for &case in &e.children {
            let info = syms.class(case);
            let Some(sym) = info.singleton else { continue };
            let class = self.class_name(case);
            let value = self.sym_name(sym);
            self.line();
            let _ = write!(self.out, "{}.{} = new {}(", self.this_name, value, class);
            js_string(self.interner.get(info.name), &mut self.out);
            let _ = write!(self.out, ", {});", info.ordinal);
        }
    }

    /// The values of the given enum cases as constants of the module, created when it loads. In a
    /// module of the split output they are assigned instead, by `$enums` once every class of the
    /// program is registered, and the names are returned for the declarations.
    fn emit_enum_values(&mut self, cases: &[ClassId], assign: bool) -> Vec<String> {
        let mut names = Vec::new();
        for &c in cases {
            let info = self.syms.class(c);
            let Some(sym) = info.singleton else { continue };
            let class = self.class_name(c);
            let value = self.enum_value_name(sym);
            self.bindings.defs.push(Def::Sym(sym));
            self.line();
            let _ = write!(self.out, "{}{} = new {}(", if assign { "" } else { "const " }, value, class);
            js_string(self.interner.get(info.name), &mut self.out);
            let _ = write!(self.out, ", {});", info.ordinal);
            names.push(value);
        }
        names
    }

    /// Writes the JS expression referring to an enum singleton value. scalac reads it from the
    /// companion, whose body runs first; a companion with a body is touched here for the same
    /// effect.
    fn emit_enum_value_ref(&mut self, sym: SymId, case: ClassId) {
        let owner = self.syms.sym(sym).owner;
        if let (Owner::Class(companion), Some(true)) = (owner, self.enum_of_case(case).map(|e| e.stateful)) {
            let n = self.sym_name(sym);
            self.emit_accessor_call(companion);
            let _ = write!(self.out, ".{}", n);
            return;
        }
        let value = self.enum_value_name(sym);
        self.note_name(&value);
        match owner {
            Owner::Class(companion) if self.has_body[companion.idx()] => {
                let touch = self.open_touch(companion);
                self.out.push_str(&value);
                self.close_touch(touch);
            }
            _ => self.out.push_str(&value),
        }
    }

    /// The companion whose body scalac's synthetic `apply` of a case class runs, when it has
    /// one. A class case of an enum goes through a module of its own there.
    pub(super) fn companion_touch(&self, c: ClassId) -> Option<ClassId> {
        touched_companion(self.syms, self.has_body, c)
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum FunKind {
    TopLevel,
    Method,
    Local,
}

pub enum Ctx {
    Return,
    Discard,
    Assign(String),
}

/// The module an outlined function of `callee` stands in: its file's, or for a product's method
/// the module of a pseudo file of its source, where the whole program has the source's file.
fn module_of_callee(syms: &Symbols, file_modules: &[usize], callee: SymId) -> usize {
    let of_file = |f: FileId| file_modules.get(f.0 as usize).copied().unwrap_or(0);
    let Some(&source) = syms.product_callee_sources.get(&callee) else { return of_file(syms.sym(callee).file) };
    syms.product_files.iter().filter(|&(_, &s)| s == source).map(|(&f, _)| f).min().map_or_else(|| of_file(syms.sym(callee).file), of_file)
}
