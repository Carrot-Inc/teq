//! From the typed program to the trees of one TASTy file: the definitions of a top-level class
//! (or of a file's top-level definitions) with their signatures, the members scalac's
//! `PostTyper` adds to them, and the types of the grammar of `TastyFormat`.
//!
//! Every definition of the file gets its address when it is written; a reference to one not
//! yet written is reserved and filled when it is (scalac's forward references). A type
//! parameter is named by the address of its `TYPEPARAM`, a term parameter by its `PARAM`'s, the
//! parameter of a lambda type by its binder's address and its position: nothing is inferred
//! from a name.

use super::buf::{Slot, TreeBuf};
use super::names::{Names, SigParam};
use super::{Product, Unit, UnitKind};
use crate::ast::{mods, DefKind, ListRef, Mods};
use crate::intern::{FxMap, Name};
use crate::source::{FileId, Span};
use crate::symbols::*;
use crate::tasty::tags::*;
use crate::typer::Worker;
use crate::types::*;
use body::{Producer, Root, Withheld};
use synth::Synth;

mod annot;
mod body;
mod quoted;
mod reflect;
mod shapes;
pub(crate) use shapes::widened_library_members;
mod synth;
mod term;

pub const TOOLING: &str = concat!("teq ", env!("CARGO_PKG_VERSION"));

/// Asks `type_param_def` for the variance of a parameter outside a template (an alias's or
/// an opaque type's lambda); no modifier has this value.
const VARIANCE_MARK: u8 = 255;

/// Asks `write_flags` for `abstract override`, which `TreePickler.pickleFlags` writes as the
/// pair `ABSTRACT OVERRIDE` where a term's `ABSTRACT` stands, and not as `OVERRIDE` alone.
const ABSTRACT_OVERRIDE: u8 = 254;

pub struct Errors {
    pub errors: Vec<String>,
}

/// What a definition of the file is, for the addresses that name it.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum Key {
    Class(ClassId),
    /// The module val of an object (`val X: X.type`).
    Module(ClassId),
    Sym(SymId),
    Alias(AliasId),
    /// A companion scalac synthesizes where the program has none, by the class it goes with.
    SynthModule(ClassId),
    SynthModuleClass(ClassId),
    /// One binding of a type parameter by a `TYPEPARAM`: a method's, a constructor's, a
    /// synthesized member's copy of its class's, or an alias's lambda's.
    Binding(u32),
    /// An enum's `$new` and `$values` in its companion, by the enum.
    EnumNew(ClassId),
    EnumValues(ClassId),
    /// The `contextual$N` parameter of a hole's splice, by the hole.
    SpliceParam(SymId),
}

#[derive(Clone)]
enum Place {
    At(usize),
    Pending(Vec<Slot>),
}

/// What a body being written changed, which its rollback puts back newest first.
enum Undo {
    /// A definition's place, as it was.
    Place(Key, Option<Place>),
    Local(Key),
    Written(ClassId),
    ByName(SymId),
}

/// Where a body began: what its rollback truncates to.
struct BodyMark {
    buf: (usize, usize),
    names: usize,
    positions: usize,
    errors: usize,
    undo: usize,
    tparams: usize,
    params: usize,
    enclosing: usize,
    elided: u32,
    withheld: usize,
    class_origins: usize,
    inlined_callees: usize,
    inlined_leaves: usize,
    inlined_leaf_tests: usize,
    appended_parents: usize,
    counted: usize,
}

/// How a type parameter is referred to where it is in scope.
#[derive(Clone, Copy)]
enum TpRef {
    /// A method's or a constructor's: `TYPEREFdirect` to its binding, written or not yet.
    Direct(u32),
    /// A class's, in a member: `TYPEREFsymbol` with the class's `this` as the prefix.
    InClass(usize, ClassId),
    /// A lambda type's: `PARAMtype` with the binder's address and the position.
    Lambda(usize, u32),
}

/// What an `IMPORT` selects on.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ImportQual {
    Pkg(PkgId),
    Object(ClassId),
    Value(crate::typer::ValueImport),
}

/// A source file's lines as the `Positions` section counts them, made once per file: the
/// section's first part, and per byte offset its offset in UTF-16 units (empty for ASCII,
/// where the two agree).
pub struct Lines {
    sizes: Vec<u8>,
    units: Vec<u32>,
}

impl Lines {
    pub fn of(text: &str) -> Lines {
        let mut sizes = Vec::new();
        let lines: Vec<&str> = text.split('\n').collect();
        super::buf::write_nat(&mut sizes, lines.len() as u64);
        let ascii = text.is_ascii();
        for l in &lines {
            let n = if ascii { l.len() } else { l.encode_utf16().count() };
            super::buf::write_nat(&mut sizes, n as u64);
        }
        let units = if ascii {
            Vec::new()
        } else {
            let mut units = Vec::with_capacity(text.len() + 1);
            let mut n = 0u32;
            for (i, ch) in text.char_indices() {
                while units.len() < i {
                    units.push(n);
                }
                units.push(n);
                n += ch.len_utf16() as u32;
            }
            while units.len() <= text.len() {
                units.push(n);
            }
            units
        };
        Lines { sizes, units }
    }
}

/// A tree's entry of the `Positions` section: its address, its source and span in bytes of that
/// source, the byte offset of its point (none for a synthetic span), whether it is a
/// definition (whose point the origins' kind 1 records) and whether its source is not its
/// parent's, which a `SOURCE` after it says.
/// A term a call's block binds: the references to it written before its val, or the val's
/// address.
enum Lifted {
    Pending(Vec<Slot>),
    At(usize),
}

#[derive(Clone, Copy)]
struct ClassHead {
    kind: ClassKind,
    owner: Owner,
    name: Name,
    file: FileId,
}

#[derive(Clone, Copy)]
struct Pos {
    addr: usize,
    file: FileId,
    span: Span,
    point: Option<u32>,
    def: bool,
    switch: bool,
}

/// What the pickler looks up across the program, built once for all its files.
#[derive(Default)]
pub struct Index {
    /// The classes nested in each class, in the order they were made.
    pub nested: FxMap<ClassId, Vec<ClassId>>,
    /// The classes that name each class as a parent.
    pub children: FxMap<ClassId, Vec<ClassId>>,
    /// The class of each inner object's lazy val.
    pub inner_objects: FxMap<SymId, ClassId>,
    /// The typed class (`Program::classes`) of each class, by its place.
    pub tclasses: FxMap<ClassId, u32>,
    /// What a pickle's paths are relative to.
    pub sourceroot: std::path::PathBuf,
    /// The directory a relative source path is under.
    pub cwd: Option<std::path::PathBuf>,
    /// The files by the text of their tags, as generated names write them.
    pub files_by_tag: FxMap<String, FileId>,
    /// The sources of the upstream products' pickles by their tags, keys and tokens: what a
    /// name a converted body's class carries places it in (a quote's copy of an upstream's
    /// anonymous class).
    pub product_sources_by_tag: FxMap<String, (String, u64)>,
    /// The signatures of the std's and the jars' members selections carry, as they are made.
    pub library_signatures: std::cell::RefCell<FxMap<SymId, Result<Option<(Vec<SigParam>, String)>, String>>>,
    /// The context closures of each file, by their starts with their numbers of parameters,
    /// made when a pickle first numbers one (`quoted::contextual_closures`).
    pub contextual: std::cell::OnceCell<FxMap<FileId, Vec<(u32, u32)>>>,
}

impl Index {
    pub fn new(w: &Worker, sourceroot: &std::path::Path) -> Index {
        let mut index = Index { sourceroot: sourceroot.to_path_buf(), cwd: std::env::current_dir().ok(), ..Index::default() };
        for f in 0..w.prog.file_tags.len() {
            if let Some(&tag) = w.prog.file_tags.get(f) {
                index.files_by_tag.insert(crate::source::tag_text(tag), FileId(f as u32));
            }
        }
        for f in w.loaded.iter().flat_map(|l| l.files.as_slice().iter()) {
            if let crate::tasty::origins::Found::Read(o) = &f.provenance {
                for (key, token) in std::iter::once((&o.key, o.token)).chain(o.sources.iter().map(|(k, t)| (k, *t))) {
                    index.product_sources_by_tag.entry(crate::source::tag_text(token)).or_insert_with(|| (key.clone(), token));
                }
            }
        }
        for i in 0..w.syms.classes.len() as u32 {
            let k = ClassId(i);
            let info = w.syms.class(k);
            if let Owner::Class(o) = info.owner {
                index.nested.entry(o).or_default().push(k);
            }
            // A child registers with its parent's class whatever the parent's prefix
            // (`Namer.registerIfChildInCreationContext`, `parent.classSymbol`).
            for &p in &info.parents {
                if let Type::Class(pc, _) = w.types.get(w.types.strip_nested(p)) {
                    index.children.entry(pc).or_default().push(k);
                }
            }
            if let Some(v) = info.inner_object {
                index.inner_objects.insert(v, k);
            }
        }
        for (i, tc) in w.prog.classes.iter().enumerate() {
            index.tclasses.insert(tc.id, i as u32);
        }
        index
    }
}

struct P<'w, 'a> {
    w: &'w mut Worker<'a>,
    index: &'w Index,
    lines: &'w Lines,
    file: FileId,
    names: Names,
    buf: TreeBuf,
    places: FxMap<Key, Place>,
    /// The definitions this file holds, known before they are written.
    local: FxMap<Key, ()>,
    tparams: Vec<(TParamId, TpRef)>,
    params: Vec<(SymId, usize)>,
    /// The parameters of the method types being written (a refinement's method, a polymorphic
    /// function type's `apply`), innermost last, each with its `METHODtype`'s address and its
    /// position in the clause: a type names one by `PARAMtype`.
    method_params: Vec<(SymId, usize, u32)>,
    /// The classes whose bodies are being written, innermost last: the `this` a type may name.
    enclosing: Vec<ClassId>,
    /// Types written once in a form that does not depend on where they stand.
    shared: FxMap<TypeId, usize>,
    /// The address of an object's path written as a prefix, shared where it is written again.
    shared_prefixes: FxMap<ClassId, usize>,
    shared_paths: FxMap<String, usize>,
    /// The classes of other files written once, by package and name.
    shared_typerefs: FxMap<u64, (Box<str>, Box<str>, usize)>,
    /// Whether the type being written names something whose form depends on the place.
    placed: bool,
    /// The trees with a position.
    positions: Vec<Pos>,
    /// The line tables of the other sources a body's trees come from (an expansion of a
    /// transparent method of another file), made when first named.
    other_lines: FxMap<FileId, Lines>,
    /// The source of the tree being written, which a tree of another source switches from.
    src_ctx: FileId,
    /// The span of the innermost tree being written that has one: a node made without a span
    /// takes it.
    span_ctx: Option<(FileId, Span)>,
    /// The arguments the anonymous class being written passes to its superclass's constructor,
    /// which its creation evaluates.
    anon_parent_args: Option<Vec<crate::tir::TExprId>>,
    /// Inside a parent constructor call's arguments, where the class's parameters are named
    /// without `this` (`TERMREF m (THIS C)`, an identifier), as the class has no instance there.
    in_parent_args: bool,
    /// The classes the bodies define, by the wide address of their `TYPEDEF`, whose origins the
    /// section records.
    class_origins: Vec<(usize, ClassId)>,
    /// The other sources the origins' records name, in the order they are first named.
    origin_sources: Vec<(String, u64)>,
    /// The `INLINED`s written, by their wide addresses, with the methods they expand.
    inlined_callees: Vec<(usize, SymId)>,
    /// The `INLINED`s being written, innermost last, by their index in `inlined_callees`.
    inlined_open: Vec<usize>,
    /// What the expansions took from their call sites, each tree's wide address with its
    /// innermost `INLINED`: the leaves, and the type tests of the call's type arguments.
    inlined_leaves: Vec<(usize, usize)>,
    inlined_leaf_tests: Vec<(usize, usize)>,
    /// The module classes whose templates got parents after the declared ones, by their wide
    /// `TYPEDEF` addresses, with how many (TeqOrigins kind 11).
    appended_parents: Vec<(usize, u32)>,
    /// The terms of a call lifted into its block's vals: the references written before a val,
    /// or its address once written.
    lifted_terms: FxMap<crate::tir::TExprId, Lifted>,
    /// The census's keys counted inside the bodies being written, which a rollback takes back.
    counted: Vec<String>,
    /// The local classes of the body being written that are written, so that a block writes each
    /// once.
    written_locals: FxMap<ClassId, ()>,
    /// The by-name parameters of the methods written, whose reads the typer made calls.
    by_name_params: FxMap<SymId, ()>,
    /// The changes the bodies being written made, while one is (`bodies_open`).
    undo: Vec<Undo>,
    bodies_open: u32,
    /// Why each failure since the last body began failed, for the census of a withheld body.
    fail_reasons: Vec<String>,
    /// How many `ELIDED` right-hand sides the pickle holds: `OUTLINEattr` is written while it
    /// holds one.
    elided_bodies: u32,
    /// The withheld bodies, by the wide address of their `ELIDED` tree (a template statement left
    /// out, by its class's `TYPEDEF`), each with the census's reason: the section's kind 10.
    withheld: Vec<(usize, String)>,
    /// The reason the census counted last, which the next `ELIDED` written takes.
    pending_withheld: Option<String>,
    /// While a parent's call that cannot be stated is written again: the reason, which each of
    /// its arguments, written `ELIDED`, takes.
    elide_args: Option<String>,
    /// The next parent `Object()` is written `AnyRef()`.
    parent_as_anyref: bool,
    /// The binders of the pattern definitions' cases being written, by their `BIND`s, apart
    /// from the vals the definitions make of them, while `binders_apart` is set.
    case_binders: Vec<(SymId, usize)>,
    /// The type variables of the case being written, each with its binding and whether its
    /// `BIND` is written yet.
    pattern_binds: Vec<(TParamId, u32, bool)>,
    binders_apart: bool,
    /// Set by an application around its selection, which appends the Java varargs the inverse
    /// table names (`appended_varargs`): such a member selected otherwise is withheld.
    varargs_head: bool,
    /// The receivers of the member calls and fields being written, where a std member of a
    /// wider scala-library result may be selected (`wider_than_lean`).
    receivers: FxMap<crate::tir::TExprId, ()>,
    /// Numbers the vals the writer makes in a pickle (a pattern definition's tuple).
    fresh_vals: u32,
    /// Numbers the binders of `summonFrom`'s `_: T` cases in a pickle (`_$N`).
    summon_wildcards: u32,
    /// The function type the closure being written as an argument is expected to have.
    expected_fn: Option<TypeId>,
    /// The stored record of the inline method whose body is being written, whose reducible
    /// nodes are written with their markers (`IF INLINE`, `MATCH INLINE`, `MATCH IMPLICIT`).
    inline_body: Option<std::sync::Arc<crate::tir::InlineDefinition>>,
    /// The class of the inline method whose body is written, whose accessors its reads take.
    inline_owner: Option<ClassId>,
    /// The address of the result type's tree of the definition being written, which an inline
    /// method's body is ascribed by sharing it, as scalac does.
    result_tpt: Option<usize>,
    /// The parameters of the retained body being written, each read an `INLINED` of no call.
    retained_params: Vec<SymId>,
    /// The type the context expects of the anonymous class instance about to be written, which
    /// scalac ascribes it where its own refined type is not one (`ensureNoLocalRefs`).
    expected_anon: Option<(crate::tir::TExprId, TypeId)>,
    /// The name of the export forwarder the selection about to be written calls, where the
    /// export renames its member.
    forwarder_name: Option<Name>,
    /// The definitions with a point, by their final address, and the byte offsets of their
    /// names: the `TeqOrigins` section's `definitions` (`origins.rs`).
    definitions: Vec<(u32, u32)>,
    /// The source annotations written, by their file and `new`: the addresses of the class and
    /// the tree, which the annotation's other symbols share, or None where it was withheld.
    annotations_written: FxMap<(FileId, crate::ast::ExprId), Option<(usize, usize)>>,
    /// The meta annotations each annotation class carries (`annot::meta`).
    annotation_metas: FxMap<ClassId, u8>,
    /// The source annotations of the parameters and type parameters about to be written, by
    /// their symbols.
    param_annots: FxMap<SymId, (FileId, Vec<crate::ast::Annot>)>,
    tparam_annots: FxMap<TParamId, (FileId, Vec<crate::ast::Annot>)>,
    /// Set while a source definition's own parameters are written, which carry its parameters'
    /// annotations.
    source_params: bool,
    /// Set while an annotation's tree is written: scalac lifts no argument of an annotation's
    /// constructor (`Applications.isAnnotConstr`), nor does teq of any constructor in its tree.
    in_annotation: bool,
    errors: Vec<String>,
    /// The source's name for the `@SourceFile` annotation and the attributes.
    source_path: String,
    /// The span of the class being written, for the members scalac synthesizes.
    synth_span: Span,
    /// While a file's top-level definitions are written: the address of their object's class
    /// and its package, the `this` their references go through.
    package_members_this: Option<(usize, PkgId)>,
    positions_name: u32,
    attributes_name: u32,
    /// Numbers the anonymous parameters of higher-kinded type parameters, `_$1` on.
    hk_param_counter: u32,
    /// Numbers the bindings of type parameters (`Key::Binding`).
    bindings: u32,
    /// Whether a file's top-level definitions are being written, inside their object: a
    /// class or object among them is no top-level class of the pickle.
    in_package_object: bool,
    /// The definitions being written, outermost first, which a failure names.
    current: Vec<String>,
    /// The generated inline accessors (`typer::accessors`), by symbol: the class, the member and
    /// whether it is the setter's, of which `sym_name` derives the accessor's name.
    accessors: FxMap<SymId, (ClassId, SymId, bool)>,
    /// The shapes written as teq's typer holds them, by kind, for the report.
    approximated: &'w mut FxMap<String, u32>,
    /// The right-hand sides by producer, and those withheld by reason (`TEQ_BODIES_CENSUS`).
    bodies: &'w mut FxMap<String, u32>,
    /// The keys of the std's selections, collected instead of checked (`std_shapes`).
    shapes_seen: Option<Vec<String>>,
    /// `natural_type` of the nodes asked for, which a block's or a conditional's asks of its
    /// results again for every enclosing one.
    natural_types: FxMap<crate::tir::TExprId, Option<TypeId>>,
    /// The constructor call being written is a secondary constructor's call of another one,
    /// selected on `this` and not on a `new`.
    self_init: bool,
    /// The class of the receiver of the selection about to be written and its type, for a std
    /// member.
    select_receiver: Option<(ClassId, TypeId)>,
    /// The evidence of the call being written, which the lean std's `@jvmEvidence` member it
    /// calls leaves out of its signature (`Capture::evidence`): taken by the call's application.
    call_evidence: Vec<crate::tir::TExprId>,
    /// The places the pickle a method converted from a product gives its term parameters, in
    /// order, for the method being written; the next one taken by `method_param`.
    param_places: Vec<Option<(String, crate::tasty::positions::Pos)>>,
    param_place_next: usize,
    /// The method being written is a synthetic one (an export's forwarder), whose parameters are
    /// placed as its other synthetic trees, not where the method it forwards to has its own.
    synthetic_params: bool,
    /// A file's `$package` object written as a prefix, by its package and stem: shared as an
    /// object read from another module's pickle is (`shared_prefixes`).
    package_object_terms: FxMap<(PkgId, String), usize>,
    /// The holes of the quotes whose bodies are being written, with their code.
    open_holes: Vec<(SymId, crate::tir::TExprId)>,
    /// The holes whose splices' code is being written, innermost last: the `Quotes` the search
    /// found there is the innermost one's parameter.
    splice_params: Vec<(SymId, usize)>,
    /// The context closures the pickle numbered that its file's list does not hold.
    contextual_extra: u32,
    /// The holes of the quote patterns whose bodies are being written, with their binders.
    pattern_holes: Vec<(SymId, crate::tir::TPatId)>,
    /// The `Type` givens the type variables of the quote patterns being written bound, each
    /// with its variable and the pattern's `Quotes`: scalac's `Type.of[t](q)`.
    type_givens: Vec<(SymId, TParamId, crate::tir::TExprId)>,
    /// How many quote and quote pattern bodies enclose the tree being written, the splices'
    /// code apart.
    quoted_depth: u32,
    /// The std's object `scala.quoted.Reflect`, once looked up (`reflect::reflect_object`).
    reflect_object: Option<Option<ClassId>>,
    /// The sources converted pickles name the places of their trees in, by path: the files
    /// `u32::MAX - i` of the positions (`quoted::pickled_place_of`).
    pickled_sources: Vec<String>,
    /// The tree of a converted pickle the last tree placed by it stands for, by its address.
    placed_tree: Option<(usize, (u32, u32))>,
    /// The `Quotes` the source names for the reflection API's paths in the trees being written
    /// (an import `q.reflect.*` until its block ends, a parameter's declared `q.reflect.Term`),
    /// innermost last, each with how many splices were open when it began.
    reflect_scopes: Vec<(reflect::QuotesRef, usize)>,
    /// The source and start of the block's import being written, which its record keeps beside.
    import_at: Option<(FileId, u32)>,
    /// The parameters of using clauses written, among which a `Quotes` is the one `quotes` names.
    using_params: FxMap<SymId, ()>,
    /// The quotes cancelled with their only splice (`'{ $x }`), each with how many splices were
    /// open around it and its `Quotes`, which the splice's code names in the splice's stead.
    cancelled_quotes: Vec<(usize, crate::tir::TExprId)>,
}

impl<'w, 'a> P<'w, 'a> {
    fn new(w: &'w mut Worker<'a>, index: &'w Index, lines: &'w Lines, file: FileId, source_path: &str, approximated: &'w mut FxMap<String, u32>, bodies: &'w mut FxMap<String, u32>) -> P<'w, 'a> {
        let accessors = w.inline_accessor_syms.iter().map(|(&key, &a)| (a, key)).collect();
        P {
            accessors,
            w,
            index,
            lines,
            file,
            names: Names::default(),
            buf: TreeBuf::default(),
            places: FxMap::default(),
            local: FxMap::default(),
            tparams: Vec::new(),
            params: Vec::new(),
            method_params: Vec::new(),
            enclosing: Vec::new(),
            shared: FxMap::default(),
            shared_prefixes: FxMap::default(),
            shared_paths: FxMap::default(),
            shared_typerefs: FxMap::default(),
            placed: false,
            positions: Vec::new(),
            other_lines: FxMap::default(),
            src_ctx: file,
            span_ctx: None,
            anon_parent_args: None,
            in_parent_args: false,
            class_origins: Vec::new(),
            origin_sources: Vec::new(),
            inlined_callees: Vec::new(),
            inlined_open: Vec::new(),
            inlined_leaves: Vec::new(),
            inlined_leaf_tests: Vec::new(),
            appended_parents: Vec::new(),
            lifted_terms: FxMap::default(),
            counted: Vec::new(),
            written_locals: FxMap::default(),
            by_name_params: FxMap::default(),
            undo: Vec::new(),
            bodies_open: 0,
            fail_reasons: Vec::new(),
            elided_bodies: 0,
            withheld: Vec::new(),
            pending_withheld: None,
            elide_args: None,
            parent_as_anyref: false,
            case_binders: Vec::new(),
            pattern_binds: Vec::new(),
            binders_apart: false,
            varargs_head: false,
            receivers: FxMap::default(),
            fresh_vals: 0,
            summon_wildcards: 0,
            annotations_written: FxMap::default(),
            annotation_metas: FxMap::default(),
            param_annots: FxMap::default(),
            tparam_annots: FxMap::default(),
            source_params: false,
            in_annotation: false,
            expected_fn: None,
            inline_body: None,
            inline_owner: None,
            result_tpt: None,
            retained_params: Vec::new(),
            expected_anon: None,
            forwarder_name: None,
            definitions: Vec::new(),
            errors: Vec::new(),
            source_path: source_path.to_string(),
            synth_span: Span::default(),
            package_members_this: None,
            positions_name: 0,
            attributes_name: 0,
            hk_param_counter: 0,
            bindings: 0,
            in_package_object: false,
            current: Vec::new(),
            approximated,
            bodies,
            shapes_seen: None,
            natural_types: FxMap::default(),
            self_init: false,
            select_receiver: None,
            call_evidence: Vec::new(),
            param_places: Vec::new(),
            param_place_next: 0,
            synthetic_params: false,
            package_object_terms: FxMap::default(),
            open_holes: Vec::new(),
            splice_params: Vec::new(),
            contextual_extra: 0,
            pattern_holes: Vec::new(),
            type_givens: Vec::new(),
            quoted_depth: 0,
            reflect_object: None,
            pickled_sources: Vec::new(),
            placed_tree: None,
            cancelled_quotes: Vec::new(),
            reflect_scopes: Vec::new(),
            import_at: None,
            using_params: FxMap::default(),
        }
    }
}

/// Every key of a selection of a library member a body can write for a std definition
/// (`TEQ_STD_SHAPES`).
pub fn std_shapes(w: &mut Worker, sourceroot: &std::path::Path) -> Vec<String> {
    let index = Index::new(w, sourceroot);
    let lines = Lines::of("");
    let (mut approximated, mut bodies) = (FxMap::default(), FxMap::default());
    let mut p = P::new(w, &index, &lines, FileId(0), "", &mut approximated, &mut bodies);
    p.all_std_shapes()
}

pub fn pickle_unit(w: &mut Worker, index: &Index, lines: &Lines, unit: &Unit, source_path: &str, approximated: &mut FxMap<String, u32>, bodies: &mut FxMap<String, u32>) -> Result<Product, Errors> {
    let mut p = P::new(w, index, lines, unit.file, source_path, approximated, bodies);
    p.names.simple("ASTs");
    let (pkg, name) = match &unit.kind {
        UnitKind::Class { class, module } => {
            let c = class.or(*module).unwrap();
            let pkg = match p.w.syms.class(c).owner {
                Owner::Package(pk) => pk,
                _ => ROOT_PKG,
            };
            (pkg, p.w.interner.get(p.w.syms.class(c).name).to_string())
        }
        UnitKind::Package { pkg, name, .. } => (*pkg, name.clone()),
    };
    p.collect_local(unit);
    let root = p.buf.addr();
    p.buf.byte(PACKAGE);
    let len = p.buf.begin_length();
    p.package_ref(pkg);
    let first = match &unit.kind {
        UnitKind::Class { class, module } => class.or(*module).map_or(u32::MAX, |c| p.w.syms.class(c).span.start),
        UnitKind::Package { defs, .. } => {
            let ast = p.w.ast(unit.file);
            defs.iter().map(|&d| ast.def(d).span.start).min().unwrap_or(u32::MAX)
        }
    };
    p.file_imports_before(first);
    match &unit.kind {
        UnitKind::Class { class, module } => p.class_group(*class, *module),
        UnitKind::Package { pkg, name, defs } => p.package_object(*pkg, name, defs),
    }
    p.buf.end_length(len);
    let text_len = p.w.files.as_slice()[unit.file.0 as usize].text.len() as u32;
    p.positions.insert(0, Pos { addr: root, file: unit.file, span: Span { start: 0, end: text_len }, point: None, def: false, switch: true });
    let pending: Vec<String> = p
        .places
        .iter()
        .filter_map(|(k, pl)| match pl {
            Place::Pending(v) if !v.is_empty() => Some(match k {
                Key::Sym(s) => format!("{} ({:?}, {:?})", p.w.interner.get(p.w.syms.sym(*s).name), k, p.w.syms.sym(*s).owner),
                _ => format!("{:?}", k),
            }),
            _ => None,
        })
        .collect();
    if !pending.is_empty() {
        p.errors.push(format!("{}: references to definitions never written: {}", name, pending.join(", ")));
    }
    if !p.errors.is_empty() {
        return Err(Errors { errors: std::mem::take(&mut p.errors) });
    }
    let complete = p.elided_bodies == 0;
    let (bytes, uuid) = p.assemble();
    // The file is named as the class files are, operators encoded (`$plus.tasty` for `+`); the
    // pickle keeps the source names.
    let package = if pkg == ROOT_PKG { String::new() } else { p.pkg_path(pkg).split('.').map(crate::jvm::names::encode).collect::<Vec<_>>().join("/") };
    Ok(Product { package, name: crate::jvm::names::encode(&name), source: unit.file, bytes, uuid, complete })
}

impl<'w, 'a> P<'w, 'a> {
    fn approximated(&mut self, kind: &str) {
        *self.approximated.entry(kind.to_string()).or_insert(0) += 1;
    }

    fn name(&self, n: Name) -> String {
        self.w.interner.get(n).to_string()
    }

    /// The name table's entry of an interned name, as a simple name.
    fn simple_name(&mut self, n: Name) -> u32 {
        self.names.interned(n, self.w.interner.get(n))
    }

    /// The name of a member as its symbol has it: a generated inline accessor's is the derived
    /// name `PrepareInlineable.MakeInlineableMap.accessorNameOf` gives the accessor's symbol,
    /// every other the simple name of its text.
    fn sym_name(&mut self, s: SymId) -> u32 {
        match self.accessors.get(&s).copied() {
            Some((c, target, setter)) => {
                let (prefix, member) = self.w.inline_accessor_parts(c, target, setter);
                self.names.inline_accessor(&prefix, &member)
            }
            None => self.simple_name(self.w.syms.sym(s).name),
        }
    }

    fn class_info(&self, c: ClassId) -> ClassInfo {
        (*self.w.syms.class(c)).clone()
    }

    /// What a path to a class reads of it, without copying its members.
    fn class_head(&self, c: ClassId) -> ClassHead {
        let i = self.w.syms.class(c);
        ClassHead { kind: i.kind, owner: i.owner, name: i.name, file: i.file }
    }

    /// What a path to a member reads of it.
    fn sym_head(&self, s: SymId) -> (Owner, Name) {
        let i = self.w.syms.sym(s);
        (i.owner, i.name)
    }

    fn sym_info(&self, s: SymId) -> SymInfo {
        (*self.w.syms.sym(s)).clone()
    }

    fn alias_info(&self, a: AliasId) -> AliasInfo {
        (*self.w.syms.alias(a)).clone()
    }

    fn fail(&mut self, what: String) {
        let path = self.source_path.clone();
        let within = if self.current.is_empty() { String::new() } else { format!(" in {}", self.current.join(".")) };
        self.errors.push(format!("{}: cannot write TASTy for {}{}", path, what, within));
        self.fail_reasons.push(what);
    }

    // ---- the file's definitions ------------------------------------------------------------

    fn collect_local(&mut self, unit: &Unit) {
        match &unit.kind {
            UnitKind::Class { class, module } => {
                if let Some(c) = class {
                    self.collect_class(*c);
                    if module.is_none() && self.needs_synthetic_companion(*c) {
                        self.local.insert(Key::SynthModule(*c), ());
                        self.local.insert(Key::SynthModuleClass(*c), ());
                    }
                }
                if let Some(m) = module {
                    self.collect_class(*m);
                }
            }
            UnitKind::Package { defs, .. } => {
                let f = self.file.0 as usize;
                for d in defs {
                    if let Some(&s) = self.w.def_syms.get(f, d) {
                        self.local.insert(Key::Sym(s), ());
                    }
                    if let Some(&c) = self.w.def_classes.get(f, d) {
                        self.collect_class(c);
                    }
                    if let Some(&a) = self.w.def_aliases.get(f, d) {
                        self.local.insert(Key::Alias(a), ());
                    }
                }
            }
        }
    }

    fn collect_class(&mut self, c: ClassId) {
        self.local.insert(Key::Class(c), ());
        if self.w.syms.class(c).kind == ClassKind::Object {
            self.local.insert(Key::Module(c), ());
        }
        let info = self.w.syms.class(c);
        let members: Vec<SymId> = info.member_order.clone();
        let nested: Vec<ClassId> = info.nested.values().copied().collect();
        let aliases: Vec<AliasId> = info.type_aliases.values().copied().collect();
        let ctor: Vec<SymId> = info.ctor_syms.iter().flatten().copied().collect();
        for s in members.into_iter().chain(ctor) {
            self.local.insert(Key::Sym(s), ());
            if let SymKind::Object(o) = self.w.syms.sym(s).kind {
                self.collect_class(o);
            }
        }
        for a in aliases {
            self.local.insert(Key::Alias(a), ());
        }
        for n in nested {
            self.collect_class(n);
        }
        // Classes nested in the body that no table above lists: inner objects, given classes.
        let classes: Vec<ClassId> = self
            .index
            .nested
            .get(&c)
            .map_or(Vec::new(), |v| v.iter().copied().filter(|&k| !self.local.contains_key(&Key::Class(k))).collect());
        for k in classes {
            self.collect_class(k);
        }
        // The companion goes with the class, but an opaque type's companion object is
        // pickled with the file's top-level definitions, apart from the object.
        if let Some(co) = self.w.syms.class(c).companion {
            let ci = self.w.syms.class(co);
            if ci.owner == self.w.syms.class(c).owner && ci.kind != ClassKind::Opaque && self.w.syms.class(c).kind != ClassKind::Opaque && !self.local.contains_key(&Key::Class(co)) && ci.file == self.file {
                self.collect_class(co);
            }
        }
    }

    /// Records where a definition was written and fills the references that waited for it.
    fn define(&mut self, k: Key, addr: usize) {
        let prior = self.places.insert(k, Place::At(addr));
        if let Some(Place::Pending(slots)) = &prior {
            for &s in slots {
                self.buf.fill(s, addr);
            }
        }
        if self.bodies_open > 0 {
            self.undo.push(Undo::Place(k, prior));
        }
    }

    /// A reference to a definition of this file, written or not yet.
    fn def_ref(&mut self, k: Key) {
        if let Some(Place::At(a)) = self.places.get(&k) {
            let a = *a;
            self.buf.reference(a);
            return;
        }
        let s = self.buf.forward_reference();
        if self.bodies_open > 0 {
            let prior = self.places.get(&k).cloned();
            self.undo.push(Undo::Place(k, prior));
        }
        match self.places.get_mut(&k) {
            Some(Place::Pending(v)) => v.push(s),
            _ => {
                self.places.insert(k, Place::Pending(vec![s]));
            }
        }
    }

    /// Marks `k` a definition of this file, undone with a body that rolls back.
    fn add_local(&mut self, k: Key) {
        if self.local.insert(k, ()).is_none() && self.bodies_open > 0 {
            self.undo.push(Undo::Local(k));
        }
    }

    /// Begins a body that `body_end` keeps or rolls back.
    fn body_begin(&mut self) -> BodyMark {
        self.bodies_open += 1;
        BodyMark {
            buf: self.buf.mark(),
            names: self.names.len(),
            positions: self.positions.len(),
            errors: self.errors.len(),
            undo: self.undo.len(),
            tparams: self.tparams.len(),
            params: self.params.len(),
            enclosing: self.enclosing.len(),
            elided: self.elided_bodies,
            withheld: self.withheld.len(),
            class_origins: self.class_origins.len(),
            inlined_callees: self.inlined_callees.len(),
            inlined_leaves: self.inlined_leaves.len(),
            inlined_leaf_tests: self.inlined_leaf_tests.len(),
            appended_parents: self.appended_parents.len(),
            counted: self.counted.len(),
        }
    }

    /// Ends a body: kept where nothing failed, else rolled back to its mark, everything it
    /// wrote and defined forgotten, with the reason of its first failure.
    fn body_end(&mut self, m: BodyMark) -> Result<(), String> {
        self.bodies_open -= 1;
        // A reference to a definition of the body that the body does not write.
        if self.bodies_open == 0 && self.errors.len() == m.errors {
            let dangling = self.undo[m.undo..].iter().find_map(|u| match u {
                Undo::Place(k, _) if matches!(self.places.get(k), Some(Place::Pending(v)) if !v.is_empty()) => match *k {
                    Key::Sym(s) if self.w.syms.sym(s).owner == Owner::Local => Some(self.name(self.w.syms.sym(s).name)),
                    Key::Class(c) if self.w.syms.class(c).owner == Owner::Local => Some(self.name(self.w.syms.class(c).name)),
                    _ => None,
                },
                _ => None,
            });
            if let Some(name) = dangling {
                self.fail(format!("a reference to the body's {} the body does not define", name));
            }
        }
        if self.errors.len() == m.errors {
            if self.bodies_open == 0 {
                self.undo.clear();
                self.counted.clear();
            }
            return Ok(());
        }
        let reason = self.fail_reasons.get(m.errors).cloned().unwrap_or_default();
        self.errors.truncate(m.errors);
        self.fail_reasons.truncate(m.errors);
        self.natural_types.clear();
        while self.undo.len() > m.undo {
            match self.undo.pop().unwrap() {
                Undo::Place(k, prior) => {
                    // A definition's references from before the body were filled with its place.
                    if let Some(Place::Pending(slots)) = &prior {
                        for &s in slots {
                            if self.buf.slot_before(s, m.buf) {
                                self.buf.fill(s, usize::MAX);
                            }
                        }
                    }
                    match prior {
                        Some(p) => {
                            self.places.insert(k, p);
                        }
                        None => {
                            self.places.remove(&k);
                        }
                    }
                }
                Undo::Local(k) => {
                    self.local.remove(&k);
                }
                Undo::Written(c) => {
                    self.written_locals.remove(&c);
                }
                Undo::ByName(s) => {
                    self.by_name_params.remove(&s);
                }
            }
        }
        let at = m.buf.0;
        self.shared.retain(|_, &mut a| a < at);
        self.shared_paths.retain(|_, &mut a| a < at);
        self.shared_prefixes.retain(|_, &mut a| a < at);
        self.package_object_terms.retain(|_, &mut a| a < at);
        self.shared_typerefs.retain(|_, (_, _, a)| *a < at);
        self.buf.truncate(m.buf);
        self.names.truncate(m.names);
        self.positions.truncate(m.positions);
        self.tparams.truncate(m.tparams);
        self.params.truncate(m.params);
        self.enclosing.truncate(m.enclosing);
        self.class_origins.truncate(m.class_origins);
        self.withheld.truncate(m.withheld);
        self.inlined_callees.truncate(m.inlined_callees);
        self.inlined_leaves.truncate(m.inlined_leaves);
        self.inlined_leaf_tests.truncate(m.inlined_leaf_tests);
        self.appended_parents.truncate(m.appended_parents);
        for key in self.counted.drain(m.counted..) {
            if let Some(n) = self.bodies.get_mut(&key) {
                *n -= 1;
            }
        }
        self.elided_bodies = m.elided;
        Err(reason)
    }

    fn is_local(&self, k: Key) -> bool {
        self.local.contains_key(&k)
    }

    // ---- positions ------------------------------------------------------------------------

    /// A definition's span, its point the start of its name (`name_at`), as scalac's is.
    fn position(&mut self, addr: usize, span: Span, name_at: u32) {
        if span.end >= span.start && span != Span::default() {
            let point = if (span.start..=span.end).contains(&name_at) { name_at } else { span.start };
            // A definition of a body takes the source of the tree it stands in (an expansion of
            // another file's method).
            let file = self.src_ctx;
            self.positions.push(Pos { addr, file, span, point: Some(point), def: true, switch: false });
        }
    }

    /// A definition's span as `position` gives it, or where it stands for a definition of a
    /// body converted from a pickle (a class a quote of an upstream's macro made, copied into an
    /// expansion), the place that pickle gives it.
    /// The span returned, from the definition's point, is what its synthetic trees take, where
    /// the pickle placed it.
    fn position_def(&mut self, addr: usize, span: Span, name_at: u32, file: FileId, def: Option<crate::ast::DefId>) -> Option<Span> {
        let tree = def.and_then(|d| self.w.loaded.as_ref()?.product_def_trees.get(&(file, d)).copied());
        let placed = tree.and_then(|(f, a)| self.w.pickled_place(f, a));
        let Some((path, pos)) = placed else {
            self.position(addr, span, name_at);
            return None;
        };
        let file = self.pickled_source(path);
        let switch = file != self.src_ctx;
        let point = pos.point.unwrap_or(pos.start);
        self.positions.push(Pos { addr, file, span: Span { start: pos.start, end: pos.end }, point: Some(point), def: true, switch });
        Some(Span { start: point, end: pos.end })
    }

    fn synthetic_position(&mut self, addr: usize) {
        let s = self.synth_span;
        self.positions.push(Pos { addr, file: self.file, span: Span { start: s.start, end: s.start }, point: None, def: false, switch: false });
    }

    /// A leaf tree starts here (a type tree, `ELIDED`, a literal): scalac's unpickler gives a
    /// tree without an entry the envelope of its children's spans, which a leaf has none of, and
    /// its checker asserts a span on every tree it types.
    fn mark_tree(&mut self) {
        let addr = self.buf.addr();
        self.synthetic_position(addr);
    }

    /// A type as the tree a definition, a parent or a bound stands on.
    fn tpt(&mut self, t: TypeId) {
        self.mark_tree();
        self.ty(t);
    }

    /// `TYPEREF name (TERMREFpkg pkg)` as a tree.
    fn external_tpt(&mut self, pkg: &str, name: &str) {
        self.mark_tree();
        self.external_typeref(pkg, name);
    }

    // ---- names and paths ------------------------------------------------------------------

    fn pkg_path(&self, p: PkgId) -> String {
        let mut segs = Vec::new();
        let mut at = Some(p);
        while let Some(k) = at {
            if k == ROOT_PKG {
                break;
            }
            let info = self.w.syms.pkg(k);
            segs.push(self.w.interner.get(info.name).to_string());
            at = info.parent;
        }
        segs.reverse();
        segs.join(".")
    }

    fn package_ref(&mut self, p: PkgId) {
        if p == ROOT_PKG {
            // Top-level definitions without a package are in scalac's empty package.
            let n = self.names.simple("<empty>");
            self.buf.byte(TERMREFPKG);
            self.buf.nat(n as u64);
            return;
        }
        let path = self.pkg_path(p);
        self.package_path(&path);
    }

    /// `TYPEREFpkg a.b`, a package as the qualifier of `private[p]`.
    fn package_typeref(&mut self, p: PkgId) {
        let path = if p == ROOT_PKG { "<empty>".to_string() } else { self.pkg_path(p) };
        let n = self.names.qualified(&path);
        self.buf.byte(TYPEREFPKG);
        self.buf.nat(n as u64);
    }

    /// `TERMREFpkg a.b`, shared after its first time.
    fn package_path(&mut self, path: &str) {
        let path = if path.is_empty() { "<empty>" } else { path };
        if let Some(&a) = self.shared_paths.get(path) {
            self.buf.byte(SHAREDTYPE);
            self.buf.reference(a);
            return;
        }
        let at = self.buf.addr();
        let n = self.names.qualified(path);
        self.buf.byte(TERMREFPKG);
        self.buf.nat(n as u64);
        self.shared_paths.insert(path.to_string(), at);
    }

    // ---- a top-level class and its companion ----------------------------------------------

    fn class_group(&mut self, class: Option<ClassId>, module: Option<ClassId>) {
        if let Some(c) = class {
            self.class_def(c);
            if module.is_none() && self.needs_synthetic_companion(c) {
                self.synthetic_companion(c);
            }
        }
        if let Some(m) = module {
            self.object_def(m);
        }
    }

    /// A case class, a value class or a class whose constructor has defaults, without a
    /// companion in the source, has one in scalac's.
    fn needs_synthetic_companion(&self, c: ClassId) -> bool {
        let info = self.w.syms.class(c);
        let defaults = info.ctor.iter().any(|cl| cl.params.iter().any(|p| p.has_default));
        // A trait's parameters' defaults are its companion's, as a class's are.
        if info.kind != ClassKind::Class && !(info.kind == ClassKind::Trait && defaults) {
            return false;
        }
        let is_case = info.mods & mods::CASE != 0;
        // A class nested in a class has its companion as an object of the class, which teq
        // holds as a class with a module val.
        let nested_companion = info.companion.map_or(false, |co| self.w.syms.class(co).inner_object.is_some());
        (is_case || info.value_class || defaults) && !nested_companion && info.companion.map_or(true, |co| self.w.syms.class(co).kind != ClassKind::Object)
    }

    fn annotations_of_class(&mut self, c: ClassId, top: bool) {
        if top {
            self.source_file_annotation();
        }
        let _ = c;
    }

    /// `@SourceFile("path")`, which scalac puts on every top-level class.
    fn source_file_annotation(&mut self) {
        let path = self.source_path.clone();
        self.buf.byte(ANNOTATION);
        let len = self.buf.begin_length();
        let tpe = self.buf.addr();
        self.external_typeref("scala.annotation.internal", "SourceFile");
        self.buf.byte(APPLY);
        let app = self.buf.begin_length();
        self.buf.byte(SELECTIN);
        let sel = self.buf.begin_length();
        let n = self.names.signed("<init>", None, &[SigParam::Type("java.lang.String".to_string())], "scala.annotation.internal.SourceFile");
        self.buf.nat(n as u64);
        self.buf.byte(NEW);
        self.mark_tree();
        self.shared_type_at(tpe);
        self.shared_type_at(tpe);
        self.buf.end_length(sel);
        let s = self.names.simple(&path);
        self.mark_tree();
        self.buf.byte(STRINGCONST);
        self.buf.nat(s as u64);
        self.buf.end_length(app);
        self.buf.end_length(len);
    }

    fn shared_type_at(&mut self, addr: usize) {
        self.buf.byte(SHAREDTYPE);
        self.buf.reference(addr);
    }

    /// `TYPEREF name (TERMREFpkg pkg)` for a class of a package outside the file.
    fn external_typeref(&mut self, pkg: &str, name: &str) {
        let key = crate::shared::hash_of(&(pkg, name));
        if let Some((p, n, a)) = self.shared_typerefs.get(&key) {
            if &**p == pkg && &**n == name {
                let a = *a;
                self.shared_type_at(a);
                return;
            }
        }
        let at = self.buf.addr();
        let n = self.names.simple(name);
        self.buf.byte(TYPEREF);
        self.buf.nat(n as u64);
        self.package_path(pkg);
        self.shared_typerefs.insert(key, (pkg.into(), name.into(), at));
    }

    fn class_def(&mut self, c: ClassId) {
        self.w.complete_class(c);
        let info = self.class_info(c);
        self.current.push(self.name(info.name));
        self.class_def_now(c, &info);
        self.current.pop();
    }

    fn class_def_now(&mut self, c: ClassId, info: &ClassInfo) {
        let info = info.clone();
        if info.kind == ClassKind::Opaque {
            self.opaque_def(c);
            return;
        }
        let addr = self.buf.addr();
        self.define(Key::Class(c), addr);
        self.buf.byte(TYPEDEF);
        let len = self.buf.begin_length();
        let anon = info.kind == ClassKind::Anon;
        // A local object's class is its module class, `Pair$`, as scalac names it.
        let n = if anon {
            self.names.simple("$anon")
        } else if info.local_module.is_some() {
            self.names.object_class(&self.name(info.name))
        } else {
            self.simple_name(info.name)
        };
        self.buf.nat(n as u64);
        let top = matches!(info.owner, Owner::Package(_)) && !self.in_package_object;
        let span = self.def_span(info.file, info.def, info.span);
        let placed = self.position_def(addr, span, info.span.start, info.file, info.def);
        let saved_span = self.synth_span;
        self.synth_span = placed.unwrap_or(info.span);
        self.template(c, None);
        self.synth_span = saved_span;
        let mut flags = self.class_flags(&info);
        if anon {
            flags.push(SYNTHETIC);
        }
        if matches!(info.kind, ClassKind::Enum | ClassKind::EnumCase) {
            flags.push(ENUM);
        }
        if self.holds_opaque(c) {
            flags.push(OPAQUE);
        }
        self.write_flags(&flags);
        let qualified = self.class_access_qualifier(&info);
        self.write_access_qualifier(qualified);
        self.annotations_of_class(c, top);
        self.child_annotations(c);
        self.class_source_annotations(c);
        self.buf.end_length(len);
    }

    fn write_access_qualifier(&mut self, qualified: Option<(bool, Qualifier)>) {
        let Some((protected, q)) = qualified else { return };
        self.buf.byte(if protected { PROTECTEDQUALIFIED } else { PRIVATEQUALIFIED });
        match q {
            Qualifier::Pkg(p) => self.package_typeref(p),
            Qualifier::Class(k) => self.class_typeref(k),
        }
    }

    /// The scope of a class's qualified access, `private[p]` or `protected[p]`, with whether
    /// it is protected; a private top-level class is `private[p]` of its package.
    fn class_access_qualifier(&self, info: &ClassInfo) -> Option<(bool, Qualifier)> {
        let protected = info.mods & mods::PROTECTED != 0;
        if info.mods & (mods::PRIVATE | mods::PROTECTED) == 0 {
            return None;
        }
        let at = info.def.map(|d| self.w.ast(info.file).def(d).span.start);
        let within = at.and_then(|at| self.w.ast(info.file).access_scopes.iter().find(|&&(x, _)| x == at).map(|&(_, n)| n));
        match within {
            Some(q) => self.enclosing_named(info.owner, q).map(|x| (protected, x)),
            None => match info.owner {
                Owner::Package(p) if !protected && !self.in_package_object => Some((false, Qualifier::Pkg(p))),
                _ => None,
            },
        }
    }

    fn class_flags(&self, info: &ClassInfo) -> Vec<u8> {
        let m = info.mods;
        let mut out = Vec::new();
        // A qualified access, and a private top-level class's (`private[p]` for scalac's
        // `Namer`), is written as its qualifier instead (`class_access_qualifier`).
        if self.class_access_qualifier(info).is_none() {
            self.access_flags(m, &mut out);
            // A private class of a class is `private[this]`, as its private members are.
            if m & mods::PRIVATE != 0 && matches!(info.owner, Owner::Class(_)) {
                out.push(LOCAL);
            }
        }
        if info.local_module.is_some() {
            out.push(OBJECT);
        } else if (m & mods::FINAL != 0 || info.kind == ClassKind::EnumCase || info.value_class) && info.kind != ClassKind::Object {
            out.push(FINAL);
        }
        if m & mods::CASE != 0 || info.kind == ClassKind::EnumCase {
            out.push(CASE);
        }
        if m & mods::SEALED != 0 || info.kind == ClassKind::Enum {
            out.push(SEALED);
        }
        if (m & mods::ABSTRACT != 0 || info.kind == ClassKind::Enum) && info.kind != ClassKind::Trait {
            out.push(ABSTRACT);
        }
        if info.kind == ClassKind::Trait {
            out.push(TRAIT);
        }
        if m & mods::OPEN != 0 {
            out.push(OPEN);
        }
        if m & mods::IMPLICIT != 0 {
            out.push(IMPLICIT);
        }
        out
    }

    fn access_flags(&self, m: Mods, out: &mut Vec<u8>) {
        if m & mods::PRIVATE != 0 {
            out.push(PRIVATE);
        }
        if m & mods::PROTECTED != 0 {
            out.push(PROTECTED);
        }
    }

    /// Modifiers in the order `TreePickler.pickleFlags` writes them.
    fn write_flags(&mut self, flags: &[u8]) {
        const ORDER: [u8; 42] = [
            PRIVATE, PROTECTED, FINAL, CASE, OVERRIDE, INLINE, INLINEPROXY, MACRO, STATIC, OBJECT, ENUM, LOCAL, SYNTHETIC, ARTIFACT,
            TRANSPARENT, INFIX, INVISIBLE, ERASED, EXPORTED, GIVEN, IMPLICIT, TRACKED, LAZY, ABSTRACT_OVERRIDE, ABSTRACT, MUTABLE,
            FIELDACCESSOR, CASEACCESSOR, HASDEFAULT, STABLE, EXTENSION, PARAMSETTER, PARAMALIAS, SEALED, TRAIT, COVARIANT,
            CONTRAVARIANT, OPAQUE, OPEN, INTO, 0, 0,
        ];
        for t in ORDER {
            if t == ABSTRACT_OVERRIDE && flags.contains(&t) {
                self.buf.byte(ABSTRACT);
                self.buf.byte(OVERRIDE);
            } else if t != 0 && flags.contains(&t) {
                self.buf.byte(t);
            }
        }
    }

    /// `@Child[C]` for each direct subclass of a sealed class, the last found first, as
    /// scalac's `PostTyper` adds them.
    fn child_annotations(&mut self, c: ClassId) {
        if self.w.syms.class(c).mods & mods::SEALED == 0 && self.w.syms.class(c).kind != ClassKind::Enum {
            return;
        }
        let children = self.sealed_children(c);
        for child in children.into_iter().rev() {
            self.buf.byte(ANNOTATION);
            let len = self.buf.begin_length();
            let tpe = self.buf.addr();
            self.external_typeref("scala.annotation.internal", "Child");
            self.buf.byte(APPLY);
            let app = self.buf.begin_length();
            self.buf.byte(TYPEAPPLY);
            let ta = self.buf.begin_length();
            self.buf.byte(SELECTIN);
            let sel = self.buf.begin_length();
            let n = self.names.signed("<init>", None, &[], "scala.annotation.internal.Child");
            self.buf.nat(n as u64);
            self.buf.byte(NEW);
            self.mark_tree();
            self.shared_type_at(tpe);
            self.shared_type_at(tpe);
            self.buf.end_length(sel);
            self.mark_tree();
            self.child_type(child);
            self.buf.end_length(ta);
            self.buf.end_length(app);
            self.buf.end_length(len);
        }
    }

    /// The direct subclasses of a sealed class in the order they are defined, a singleton
    /// case or an object as its module val's type.
    fn sealed_children(&mut self, c: ClassId) -> Vec<ChildRef> {
        let info = self.class_info(c);
        let mut out = Vec::new();
        if info.kind == ClassKind::Enum {
            for &k in &info.children {
                let ki = self.w.syms.class(k);
                if let Some(s) = ki.singleton {
                    out.push(ChildRef::Value(s, k));
                } else {
                    out.push(ChildRef::Class(k));
                }
            }
            return out;
        }
        for &k in &info.subclasses {
            out.push(if self.w.syms.class(k).kind == ClassKind::Object { ChildRef::Object(k) } else { ChildRef::Class(k) });
        }
        // Subclasses by a trait are not `subclasses`; the parents say it.
        let direct: Vec<ClassId> = self.index.children.get(&c).cloned().unwrap_or_default();
        for k in direct {
            if out.iter().any(|r| r.class() == k) {
                continue;
            }
            let ki = self.w.syms.class(k);
            if ki.def.is_none() || ki.file != info.file {
                continue;
            }
            out.push(if ki.kind == ClassKind::Object { ChildRef::Object(k) } else { ChildRef::Class(k) });
        }
        out.sort_by_key(|r| {
            let k = r.class();
            let ki = self.w.syms.class(k);
            (ki.span.start, k.0)
        });
        // A local or anonymous child cannot be named from the pickle: scalac records the
        // sealed class itself in its place, once.
        let mut seen_local = false;
        out.retain_mut(|r| {
            let k = r.class();
            let ki = self.w.syms.class(k);
            if ki.owner == Owner::Local || ki.kind == ClassKind::Anon {
                if seen_local {
                    return false;
                }
                seen_local = true;
                *r = ChildRef::Class(c);
            }
            true
        });
        out
    }

    fn child_type(&mut self, child: ChildRef) {
        // A child nested in an object is named through the object's `this`, as scalac does.
        let owner = match child {
            ChildRef::Class(k) | ChildRef::Object(k) => self.w.syms.class(k).owner,
            ChildRef::Value(s, _) => self.w.syms.sym(s).owner,
        };
        let through = match owner {
            Owner::Class(o) if self.w.syms.class(o).kind == ClassKind::Object && !self.enclosing.contains(&o) => Some(o),
            _ => None,
        };
        if let Some(o) = through {
            self.enclosing.push(o);
        }
        self.child_type_now(child);
        if through.is_some() {
            self.enclosing.pop();
        }
    }

    fn child_type_now(&mut self, child: ChildRef) {
        match child {
            ChildRef::Class(k) => {
                // `@Child[C]` names the class unapplied, as a type tree would.
                self.class_typeref(k);
            }
            ChildRef::Object(k) => {
                self.module_termref(k);
            }
            ChildRef::Value(s, _) => {
                self.member_termref(s);
            }
        }
    }

    // ---- templates ------------------------------------------------------------------------

    /// `TEMPLATE`: the class's type parameters and constructor parameters, its parents, its
    /// self type, the constructor and the members.
    fn template(&mut self, c: ClassId, synth_of: Option<ClassId>) {
        let info = self.class_info(c);
        self.note_class_param_annotations(&info);
        self.buf.byte(TEMPLATE);
        let len = self.buf.begin_length();
        let tmark = self.tparams.len();
        let own: Vec<TParamId> = info.tparams.to_vec();
        for &tp in &own {
            let at = self.buf.addr();
            self.tparams.push((tp, TpRef::InClass(at, c)));
            self.type_param_def(tp, &[PRIVATE, LOCAL], None);
        }
        let ctor_syms: Vec<Vec<SymId>> = info.ctor_syms.clone();
        let ctor: Vec<ClauseSig> = info.ctor.clone();
        let is_case = info.mods & mods::CASE != 0;
        for (ci, clause) in ctor.iter().enumerate() {
            for (pi, param) in clause.params.iter().enumerate() {
                let sym = ctor_syms.get(ci).and_then(|cl| cl.get(pi)).copied();
                self.template_param(c, param, sym, clause, is_case && ci == 0);
            }
        }
        self.enclosing.push(c);
        self.parents(c, &info);
        if let Some(self_ty) = info.declared_self {
            self.buf.byte(SELFDEF);
            let n = if info.self_alias == crate::names::EMPTY { self.names.simple("_") } else { self.simple_name(info.self_alias) };
            self.buf.nat(n as u64);
            self.tpt(self_ty);
        } else if info.kind == ClassKind::Object || synth_of.is_some() {
            self.buf.byte(SELFDEF);
            let n = self.names.simple("_");
            self.buf.nat(n as u64);
            self.buf.byte(SINGLETONTPT);
            self.mark_tree();
            match synth_of {
                Some(k) => self.def_termref(Key::SynthModule(k), info.owner),
                None => self.module_self(c),
            }
        } else if let Some(m) = info.local_module {
            self.buf.byte(SELFDEF);
            let n = self.names.simple("_");
            self.buf.nat(n as u64);
            self.buf.byte(SINGLETONTPT);
            self.mark_tree();
            self.local_ref(m);
        }
        self.constructor(c, &info);
        self.param_setters(&info);
        self.secondary_constructors(c, &info);
        self.count_template_roots(c);
        self.members(c, &info);
        self.inline_accessor_defs(c);
        self.enclosing.pop();
        self.tparams.truncate(tmark);
        self.buf.end_length(len);
    }

    fn template_param(&mut self, c: ClassId, param: &ParamSig, sym: Option<SymId>, clause: &ClauseSig, case_field: bool) {
        let addr = self.buf.addr();
        if let Some(s) = sym {
            self.define(Key::Sym(s), addr);
        }
        if sym != Some(param.sym) && param.sym != SymId(u32::MAX) {
            self.define(Key::Sym(param.sym), addr);
        }
        self.buf.byte(PARAM);
        let len = self.buf.begin_length();
        let n = self.simple_name(param.name);
        self.buf.nat(n as u64);
        self.param_type(param);
        let smods = sym.map_or(0, |s| self.w.syms.sym(s).mods);
        let scoped = sym.map_or(false, |s| self.w.syms.sym(s).scoped_private);
        let mut flags = Vec::new();
        let is_field = smods & mods::FIELD != 0 || case_field;
        if !is_field {
            flags.push(PRIVATE);
            flags.push(LOCAL);
        } else {
            let private = smods & mods::PRIVATE != 0 && smods & mods::FIELD != 0 && !scoped;
            self.access_flags(smods & !mods::PRIVATE | if private { mods::PRIVATE } else { 0 }, &mut flags);
        }
        if case_field {
            flags.push(CASEACCESSOR);
        }
        if smods & mods::MUTABLE != 0 {
            flags.push(MUTABLE);
        }
        if clause.is_implicit {
            flags.push(IMPLICIT);
        } else if clause.is_using {
            flags.push(GIVEN);
        }
        if param.has_default {
            flags.push(HASDEFAULT);
        }
        if smods & mods::OVERRIDE != 0 {
            flags.push(OVERRIDE);
        }
        self.write_flags(&flags);
        if let (true, Some(s)) = (is_field && scoped, sym) {
            self.scoped_access(s);
        }
        self.param_source_annotations(param.sym, annot::Dest::Accessor);
        let span = sym.map_or(Span::default(), |s| self.w.syms.sym(s).span);
        self.position(addr, span, span.start);
        self.buf.end_length(len);
        let _ = c;
    }

    /// `PRIVATEqualified` of the qualifier a `private[q]` definition names.
    fn scoped_access(&mut self, s: SymId) {
        if let Some(q) = self.scoped_qualifier(s) {
            self.buf.byte(PRIVATEQUALIFIED);
            match q {
                Qualifier::Pkg(p) => self.package_typeref(p),
                Qualifier::Class(c) => self.class_typeref(c),
            }
        }
    }

    /// The setters of a class's `var` parameters, `x_=(x$1: T): Unit` without a body, which
    /// scalac pickles after the constructor (`PARAMsetter`).
    fn param_setters(&mut self, info: &ClassInfo) {
        let syms: Vec<SymId> = info.ctor_syms.iter().flatten().copied().collect();
        for s in syms {
            let si = self.sym_info(s);
            if si.mods & mods::MUTABLE == 0 || si.mods & mods::FIELD == 0 || !crate::typer::setters::setter_needed(&self.w.syms, s) {
                continue;
            }
            let ty = self.w.sig_of(s).ret;
            let addr = self.buf.addr();
            self.buf.byte(DEFDEF);
            let len = self.buf.begin_length();
            let n = self.names.simple(&format!("{}_=", self.name(si.name)));
            self.buf.nat(n as u64);
            let pa = self.buf.addr();
            self.buf.byte(PARAM);
            let pl = self.buf.begin_length();
            let pn = self.names.simple("x$1");
            self.buf.nat(pn as u64);
            self.tpt(ty);
            self.buf.byte(SYNTHETIC);
            self.synthetic_position(pa);
            self.buf.end_length(pl);
            let unit = self.w.b.t_unit;
            self.tpt(unit);
            let mut flags = Vec::new();
            if !si.scoped_private {
                self.access_flags(si.mods, &mut flags);
            }
            flags.extend_from_slice(&[MUTABLE, FIELDACCESSOR, PARAMSETTER]);
            self.write_flags(&flags);
            if si.scoped_private {
                self.scoped_access(s);
            }
            self.param_source_annotations(s, annot::Dest::Setter);
            self.synthetic_position(addr);
            self.buf.end_length(len);
        }
    }

    /// A class's parents: the first, a class, as its constructor call (`Object()` where no
    /// class is extended), the traits as type trees; a case class adds `Product` and
    /// `Serializable`, an enum `scala.reflect.Enum`, as `Desugar` does. A trait's parents are
    /// types.
    fn parents(&mut self, c: ClassId, info: &ClassInfo) {
        let parents: Vec<TypeId> = info.parents.iter().copied().filter(|&p| p != self.w.b.t_any_ref).collect();
        let first_is_class = parents.first().map_or(false, |&p| !self.is_trait_type(p));
        let calls = info.kind != ClassKind::Trait;
        if info.value_class && !parents.iter().any(|&p| p == self.w.b.t_any_val) {
            if calls {
                let t = self.w.b.t_any_val;
                self.parent_call(c, t);
            } else {
                self.external_tpt("scala", "AnyVal");
            }
        } else if !first_is_class {
            if calls {
                let t = self.w.b.t_any_ref;
                self.parent_call(c, t);
            } else {
                self.external_tpt("java.lang", "Object");
            }
        }
        for (i, &p) in parents.iter().enumerate() {
            if info.kind == ClassKind::Enum && self.is_scala_class(p, "Enum") {
                continue;
            }
            if calls && i == 0 && first_is_class {
                self.parent_or_elided(|w| w.parent_call(c, p));
            } else if let Some((args, via, prelude)) = self.trait_args(c, p).filter(|_| calls) {
                // A trait parent the class passes arguments to: its constructor call.
                self.parent_or_elided(|w| w.trait_parent_call(p, &args, via, prelude));
            } else {
                self.tpt(p);
            }
        }
        if info.kind == ClassKind::Enum {
            self.external_tpt("scala.reflect", "Enum");
        }
        if info.mods & mods::CASE != 0 && info.kind == ClassKind::Class {
            if !parents.iter().any(|&p| self.is_scala_class(p, "Product")) {
                self.external_tpt("scala", "Product");
            }
            self.external_tpt("java.io", "Serializable");
        }
        let _ = c;
    }

    /// A parent's constructor call, or, where its arguments cannot be stated, the call with
    /// each argument `ELIDED` of its parameter's type and the reason recorded: what the class
    /// passes its parent is a body, which is withheld and never fails the build. An anonymous
    /// class's parent is its enclosing body's to roll back.
    fn parent_or_elided(&mut self, call: impl Fn(&mut Self)) {
        if self.bodies_open > 0 {
            return call(self);
        }
        let mark = self.body_begin();
        call(self);
        if let Err(reason) = self.body_end(mark) {
            self.count_withheld(&format!("{} (a parent's call)", reason));
            self.elide_args = self.pending_withheld.take();
            call(self);
            self.elide_args = None;
        }
    }

    /// What the class `c` passes to the parameters of its trait parent `p`, where it passes
    /// any: the parent's arguments, or the evidence of a trait that takes nothing else, which
    /// the class sets as the trait's fields.
    fn trait_args(&mut self, c: ClassId, p: TypeId) -> Option<(Vec<crate::tir::TExprId>, Option<SymId>, Option<crate::ast::ListRef>)> {
        let p = self.w.zonk(p);
        let Type::Class(k, _) = self.w.types.get(self.w.types.strip_nested(p)) else { return None };
        let &i = self.index.tclasses.get(&c)?;
        let init = self.w.prog.classes[i as usize].init.clone();
        if let Some(pc) = init.iter().find_map(|x| match x {
            crate::tir::TInit::Parent(t, pc) if *t == k && pc.args.len > 0 => Some(*pc),
            _ => None,
        }) {
            return Some((self.w.prog.expr_list(pc.args).to_vec(), pc.via, Some(pc.prelude)));
        }
        let params: Vec<SymId> = match self.index.tclasses.get(&k) {
            Some(&j) => {
                let tk = &self.w.prog.classes[j as usize];
                tk.ctor_params.iter().skip(tk.captures).copied().collect()
            }
            None => return None,
        };
        if params.is_empty() {
            return None;
        }
        let args: Vec<crate::tir::TExprId> = params
            .iter()
            .map(|&s| init.iter().find_map(|x| match *x {
                crate::tir::TInit::Field(f, e) if f == s => Some(e),
                _ => None,
            }))
            .collect::<Option<Vec<_>>>()?;
        Some((args, None, None))
    }

    fn object_type(&mut self) -> TypeId {
        self.w.b.t_any_ref
    }

    fn is_scala_class(&self, t: TypeId, name: &str) -> bool {
        match self.w.types.get(t) {
            Type::Class(c, _) => {
                let info = self.w.syms.class(c);
                self.w.interner.get(info.name) == name && matches!(info.owner, Owner::Package(p) if self.pkg_path(p) == "scala")
            }
            _ => false,
        }
    }

    fn scala_trait(&mut self, name: &str) -> Option<TypeId> {
        let scala = self.w.b.scala_pkg;
        let n = self.w.interner.intern(name);
        let c = self.w.syms.pkg(scala).entries.get(&n).and_then(|e| e.class)?;
        Some(self.w.types.class(c, &[]))
    }

    // ---- constructors ---------------------------------------------------------------------

    fn constructor(&mut self, c: ClassId, info: &ClassInfo) {
        let addr = self.buf.addr();
        self.buf.byte(DEFDEF);
        let len = self.buf.begin_length();
        let n = self.names.simple("<init>");
        self.buf.nat(n as u64);
        let tmark = self.tparams.len();
        let pmark = self.params.len();
        let own: Vec<TParamId> = info.tparams.to_vec();
        let ids = self.bind_tparams(&own);
        self.write_bound_tparams(&own, &ids, false);
        let ctor = info.ctor.clone();
        let syms = info.ctor_syms.clone();
        if ctor.is_empty() {
            self.buf.byte(EMPTYCLAUSE);
        }
        let outer = std::mem::replace(&mut self.source_params, true);
        self.clauses(&ctor, Some(&syms), None);
        self.source_params = outer;
        let unit = self.w.b.t_unit;
        self.tpt(unit);
        self.tparams.truncate(tmark);
        self.params.truncate(pmark);
        let mut flags = Vec::new();
        // `class C private[q] (..)`: the qualifier filed under the end of the class's name.
        let scope = (info.mods & (mods::PRIVATE_CTOR | mods::PROTECTED_CTOR) != 0)
            .then(|| info.def.and_then(|_| self.w.ast(info.file).access_scopes.iter().find(|&&(at, _)| at == info.span.end).map(|&(_, n)| n)))
            .flatten()
            .and_then(|q| self.enclosing_named(Owner::Class(c), q));
        if info.mods & mods::PRIVATE_CTOR != 0 && scope.is_none() {
            flags.push(PRIVATE);
        }
        if info.mods & mods::PROTECTED_CTOR != 0 {
            flags.push(PROTECTED);
        }
        // A trait's constructor is stable, and so is a class's whose parameters are all fields
        // (`Base(val n: Int)`, a case class), as the probes show scalac pickling them.
        let fields = |s: &SymId| self.w.syms.sym(*s).mods & mods::FIELD != 0;
        let all_fields = syms.iter().flatten().next().is_some() && (info.mods & mods::CASE != 0 || syms.iter().flatten().all(fields));
        if info.kind == ClassKind::Trait || all_fields || info.local_module.is_some() {
            flags.push(STABLE);
        }
        self.write_flags(&flags);
        if let Some(q) = scope {
            self.buf.byte(if info.mods & mods::PRIVATE_CTOR != 0 { PRIVATEQUALIFIED } else { PROTECTEDQUALIFIED });
            match q {
                Qualifier::Pkg(p) => self.package_typeref(p),
                Qualifier::Class(k) => self.class_typeref(k),
            }
        }
        self.synthetic_position(addr);
        self.buf.end_length(len);
    }

    /// Puts type parameters in scope as direct references, each to a binding of its own that
    /// `write_bound_tparams` writes: a bound may name a parameter written after it.
    pub(super) fn bind_tparams(&mut self, tparams: &[TParamId]) -> Vec<u32> {
        tparams
            .iter()
            .map(|&tp| {
                self.bindings += 1;
                self.tparams.push((tp, TpRef::Direct(self.bindings)));
                self.bindings
            })
            .collect()
    }

    pub(super) fn write_bound_tparams(&mut self, tparams: &[TParamId], ids: &[u32], variance: bool) {
        for (&tp, &id) in tparams.iter().zip(ids) {
            let at = self.buf.addr();
            self.define(Key::Binding(id), at);
            let extra: &[u8] = if variance { &[VARIANCE_MARK] } else { &[] };
            self.type_param_def(tp, extra, None);
        }
    }

    /// Parameter clauses: `EMPTYCLAUSE` for `()`, `SPLITCLAUSE` between two clauses of one kind.
    /// A class's secondary constructors, `def this(..) = { this(..); stats }`, after its
    /// primary one.
    fn secondary_constructors(&mut self, c: ClassId, info: &ClassInfo) {
        let funs: Vec<crate::tir::FunId> = self.index.tclasses.get(&c).map(|&i| self.w.prog.classes[i as usize].ctors.clone()).unwrap_or_default();
        for &s in &info.ctors {
            let f = funs.iter().copied().find(|&f| self.w.prog.funs[f.idx()].sym == s);
            let addr = self.buf.addr();
            self.define(Key::Sym(s), addr);
            self.buf.byte(DEFDEF);
            let len = self.buf.begin_length();
            let n = self.names.simple("<init>");
            self.buf.nat(n as u64);
            let tmark = self.tparams.len();
            let pmark = self.params.len();
            let own: Vec<TParamId> = info.tparams.to_vec();
            let ids = self.bind_tparams(&own);
            self.write_bound_tparams(&own, &ids, false);
            let sig = self.w.sig_of(s).clone();
            if sig.clauses.is_empty() {
                self.buf.byte(EMPTYCLAUSE);
            }
            let si = self.sym_info(s);
            let source = self.note_method_param_annotations(&si, &sig);
            let outer = std::mem::replace(&mut self.source_params, source);
            self.clauses(&sig.clauses, None, None);
            self.source_params = outer;
            let unit = self.w.b.t_unit;
            self.tpt(unit);
            match f {
                Some(f) => {
                    let mark = self.body_begin();
                    self.secondary_body(c, f);
                    match self.body_end(mark) {
                        Ok(()) => self.count_body(Producer::Source),
                        Err(reason) => {
                            self.count_withheld(&reason);
                            self.elided(unit);
                        }
                    }
                }
                None => self.rhs(Producer::Elided(Withheld::Untyped), unit),
            }
            self.tparams.truncate(tmark);
            self.params.truncate(pmark);
            let si = self.sym_info(s);
            let mut flags = Vec::new();
            if si.mods & mods::PRIVATE != 0 {
                flags.push(PRIVATE);
            }
            if si.mods & mods::PROTECTED != 0 {
                flags.push(PROTECTED);
            }
            self.write_flags(&flags);
            if let Some(d) = si.def {
                let annots = self.w.ast(si.file).def(d).annots.clone();
                self.source_annotations(si.file, &annots, annot::Dest::Any, &mut |_, _| {});
            }
            let span = self.def_span(si.file, si.def, si.span);
            self.position_def(addr, span, si.span.start, si.file, si.def);
            self.buf.end_length(len);
        }
    }

    fn clauses(&mut self, clauses: &[ClauseSig], syms: Option<&[Vec<SymId>]>, _owner: Option<SymId>) {
        let mut prev_terms = false;
        for (ci, clause) in clauses.iter().enumerate() {
            if clause.params.is_empty() {
                self.buf.byte(EMPTYCLAUSE);
                prev_terms = false;
                continue;
            }
            if prev_terms {
                self.buf.byte(SPLITCLAUSE);
            }
            for (pi, p) in clause.params.iter().enumerate() {
                let sym = syms.and_then(|s| s.get(ci)).and_then(|cl| cl.get(pi)).copied().or(Some(p.sym));
                self.method_param(p, clause, sym);
            }
            prev_terms = true;
        }
    }

    fn method_param(&mut self, p: &ParamSig, clause: &ClauseSig, sym: Option<SymId>) {
        let addr = self.buf.addr();
        if p.by_name {
            for s in std::iter::once(p.sym).chain(sym) {
                if self.by_name_params.insert(s, ()).is_none() && self.bodies_open > 0 {
                    self.undo.push(Undo::ByName(s));
                }
            }
        }
        if let Some(s) = sym {
            self.params.push((s, addr));
        }
        self.params.push((p.sym, addr));
        if clause.is_using {
            self.using_params.extend(std::iter::once(p.sym).chain(sym).map(|s| (s, ())));
        }
        self.buf.byte(PARAM);
        let len = self.buf.begin_length();
        let n = self.simple_name(p.name);
        self.buf.nat(n as u64);
        // A type of the reflection API the parameter declares on a named `Quotes`
        // (`t: q.reflect.Term`): that one's.
        let declared = sym.or(Some(p.sym)).and_then(|s| {
            let info = self.w.syms.sym(s);
            let (file, start) = (info.file, info.span.start);
            self.declared_quotes(file, start)
        });
        let scopes = self.reflect_scopes.len();
        if let Some(q) = declared {
            self.reflect_scopes.push((q, self.splice_params.len()));
        }
        let inline = sym.map_or(false, |s| self.w.syms.sym(s).mods & mods::INLINE != 0);
        // A forwarder's inline parameter has the type scalac's `Exporter` copies, annotated.
        if self.synthetic_params && inline && !p.by_name && !p.repeated {
            self.mark_tree();
            self.buf.byte(ANNOTATEDTYPE);
            let l = self.buf.begin_length();
            self.ty(p.ty);
            self.annotation_tree("scala.annotation.internal", "InlineParam");
            self.buf.end_length(l);
        } else {
            self.param_type(p);
        }
        self.reflect_scopes.truncate(scopes);
        let mut flags = Vec::new();
        if clause.is_implicit {
            flags.push(IMPLICIT);
        } else if clause.is_using {
            flags.push(GIVEN);
        }
        if p.has_default {
            flags.push(HASDEFAULT);
        }
        if inline {
            flags.push(INLINE);
        }
        self.write_flags(&flags);
        if self.source_params {
            self.param_source_annotations(p.sym, annot::Dest::Param);
        }
        // A parameter of a method converted from a product's pickle where that pickle places it.
        let placed = self.param_places.get(self.param_place_next).cloned().flatten();
        self.param_place_next += 1;
        if self.synthetic_params {
            self.synthetic_position(addr);
            self.buf.end_length(len);
            return;
        }
        match placed {
            Some((path, pos)) => {
                let file = self.pickled_source(path);
                let switch = file != self.src_ctx;
                self.positions.push(Pos { addr, file, span: Span { start: pos.start, end: pos.end }, point: Some(pos.point.unwrap_or(pos.start)), def: true, switch });
            }
            None => {
                let span = sym.map_or(Span::default(), |s| self.w.syms.sym(s).span);
                self.position(addr, span, span.start);
            }
        }
        self.buf.end_length(len);
    }

    /// A parameter's type: `=> T` as `BYNAMEtpt`, `T*` as `Seq[T] @Repeated`.
    fn param_type(&mut self, p: &ParamSig) {
        if p.by_name {
            self.buf.byte(BYNAMETPT);
        }
        if p.repeated {
            self.buf.byte(ANNOTATEDTPT);
            let len = self.buf.begin_length();
            self.buf.byte(APPLIEDTPT);
            let app = self.buf.begin_length();
            self.external_tpt("scala.collection.immutable", "Seq");
            self.tpt(p.ty);
            self.buf.end_length(app);
            self.annotation_tree("scala.annotation.internal", "Repeated");
            self.buf.end_length(len);
        } else {
            self.tpt(p.ty);
        }
    }

    /// `Seq[T] @Repeated` as a type.
    fn repeated_type(&mut self, elem: TypeId) {
        self.buf.byte(ANNOTATEDTYPE);
        let len = self.buf.begin_length();
        self.buf.byte(APPLIEDTYPE);
        let app = self.buf.begin_length();
        self.external_typeref("scala.collection.immutable", "Seq");
        self.ty(elem);
        self.buf.end_length(app);
        self.annotation_tree("scala.annotation.internal", "Repeated");
        self.buf.end_length(len);
    }

    /// `new pkg.Name()` as the tree of an annotation without arguments.
    fn annotation_tree(&mut self, pkg: &str, name: &str) {
        self.buf.byte(APPLY);
        let app = self.buf.begin_length();
        self.buf.byte(SELECTIN);
        let sel = self.buf.begin_length();
        let n = self.names.signed("<init>", None, &[], &format!("{}.{}", pkg, name));
        self.buf.nat(n as u64);
        self.buf.byte(NEW);
        let at = self.buf.addr();
        self.external_tpt(pkg, name);
        self.shared_type_at(at);
        self.buf.end_length(sel);
        self.buf.end_length(app);
    }

    /// A type parameter; `extra` holds the template's `PRIVATE LOCAL`, and only a class's
    /// parameters, which the template declares, carry their variance.
    fn type_param_def(&mut self, tp: TParamId, extra: &[u8], _owner: Option<SymId>) {
        let info = self.w.syms.tparam(tp).clone();
        let in_template = extra.contains(&LOCAL) || extra.contains(&VARIANCE_MARK);
        let extra: Vec<u8> = extra.iter().copied().filter(|&t| t != VARIANCE_MARK).collect();
        let extra = extra.as_slice();
        self.buf.byte(TYPEPARAM);
        let len = self.buf.begin_length();
        let n = self.simple_name(info.name);
        self.buf.nat(n as u64);
        self.mark_tree();
        self.bounds(&info);
        let mut flags: Vec<u8> = extra.to_vec();
        if in_template && info.variance > 0 {
            flags.push(COVARIANT);
        } else if in_template && info.variance < 0 {
            flags.push(CONTRAVARIANT);
        }
        self.write_flags(&flags);
        if let Some((file, annots)) = self.tparam_annots.remove(&tp) {
            self.source_annotations(file, &annots, annot::Dest::Any, &mut |_, _| {});
        }
        self.buf.end_length(len);
    }

    /// `>: lo <: hi`, over a lambda of the parameter's own parameters when it takes some.
    fn bounds(&mut self, info: &TParamInfo) {
        if info.arity > 0 {
            // `F[_]`: its upper bound is a type lambda over parameters of its arity, and the
            // bounds end with the variances of those parameters.
            self.buf.byte(TYPEBOUNDS);
            let b = self.buf.begin_length();
            let lower = if info.lower == NOTHING || info.lower == ERROR { NOTHING } else { info.lower };
            match self.w.types.get(lower) {
                Type::Lambda(ps, body) => {
                    let ps: Vec<TypeId> = self.w.types.items(ps).to_vec();
                    self.lambda_type(TYPELAMBDATYPE, &ps, body);
                }
                _ => self.ty(lower),
            }
            let hi = if info.upper == ERROR { ANY } else { info.upper };
            match self.w.types.get(hi) {
                Type::Lambda(ps, body) => {
                    let ps: Vec<TypeId> = self.w.types.items(ps).to_vec();
                    self.lambda_type(TYPELAMBDATYPE, &ps, body);
                }
                _ => {
                    // `F[_] <: B`: a lambda whose result is the bound, over anonymous
                    // parameters.
                    self.placed = true;
                    let binder = self.buf.addr();
                    let _ = binder;
                    self.buf.byte(TYPELAMBDATYPE);
                    let len = self.buf.begin_length();
                    self.ty(hi);
                    // The parameters are named as the lower bound's lambda names them, if it has
                    // one, else as scalac names anonymous ones.
                    let lower_names: Vec<String> = match self.w.types.get(lower) {
                        Type::Lambda(ps, _) => self
                            .w
                            .types
                            .items(ps)
                            .iter()
                            .filter_map(|&p| match self.w.types.get(p) {
                                Type::Param(id) => Some(self.name(self.w.syms.tparam(id).name)),
                                _ => None,
                            })
                            .collect(),
                        _ => Vec::new(),
                    };
                    for i in 0..info.arity as usize {
                        self.buf.byte(TYPEBOUNDS);
                        let pb = self.buf.begin_length();
                        self.ty(NOTHING);
                        self.ty(ANY);
                        self.buf.end_length(pb);
                        let name = match lower_names.get(i) {
                            Some(n) => n.clone(),
                            None => {
                                self.hk_param_counter += 1;
                                format!("_${}", self.hk_param_counter)
                            }
                        };
                        let n = self.names.simple(&name);
                        self.buf.nat(n as u64);
                    }
                    self.buf.end_length(len);
                }
            }
            for i in 0..info.arity as usize {
                let v = info.hk_variances.get(i).copied().unwrap_or(0);
                self.buf.byte(if v > 0 { COVARIANT } else if v < 0 { CONTRAVARIANT } else { STABLE });
            }
            self.buf.end_length(b);
            return;
        }
        self.buf.byte(TYPEBOUNDS);
        let len = self.buf.begin_length();
        self.ty(info.lower);
        self.ty(info.upper);
        self.buf.end_length(len);
    }

    // ---- members --------------------------------------------------------------------------

    fn members(&mut self, c: ClassId, info: &ClassInfo) {
        if info.kind == ClassKind::Trait {
            self.super_accessors(c);
        }
        let order: Vec<SymId> = info.member_order.clone();
        let ctor_syms: Vec<SymId> = info.ctor_syms.iter().flatten().copied().collect();
        let case_synth = info.mods & mods::CASE != 0 && info.singleton.is_none() && matches!(info.kind, ClassKind::Class | ClassKind::EnumCase);
        if case_synth {
            self.case_class_members(c, info);
        } else if info.value_class {
            let int = self.w.b.t_int;
            let boolean = self.w.b.t_boolean;
            if !self.user_defines(c, "hashCode") {
                self.synth_def("hashCode", &[], true, int, &[OVERRIDE, SYNTHETIC], Synth::ValueHashCode(c));
            }
            if !self.user_defines(c, "equals") {
                self.synth_def("equals", &[("x$0", ANY)], false, boolean, &[OVERRIDE, SYNTHETIC], Synth::ValueEquals(c));
            }
        }
        // Members of the body in their source order, nested classes, aliases and statements
        // among them.
        let body = self.body_order(c, info);
        let mut stmts = self.template_stmts(c);
        for m in body {
            self.statements_before(c, &mut stmts, self.item_start(&m));
            match m {
                BodyItem::Sym(s) => {
                    if ctor_syms.contains(&s) || !(order.contains(&s) || info.extensions.contains(&s) || info.givens.contains(&s)) {
                        continue;
                    }
                    self.member(c, s);
                }
                BodyItem::Class(k) => {
                    self.nested_class(k);
                }
                BodyItem::Alias(a) => self.alias_def(a),
            }
        }
        self.statements_before(c, &mut stmts, None);
        if info.has_exports {
            self.export_forwarders(c);
        }
        // The implementations of the deferred givens the class inherits, after its body, as
        // dotty's `implementDeferredGivens` appends them.
        if let Some(&i) = self.index.tclasses.get(&c) {
            let implementations: Vec<SymId> = self.w.prog.classes[i as usize].deferred_givens.iter().map(|&(_, s)| s).collect();
            for s in implementations {
                self.value_or_method(s, &[]);
            }
        }
        if case_synth {
            self.case_class_tail(c, info);
        }
    }

    /// `super$<trait>$$m` for each member `m` a trait calls through `super`, which scalac's
    /// `SuperAccessors` adds before pickling: abstract, an artifact.
    fn super_accessors(&mut self, c: ClassId) {
        let called: Vec<SymId> = self.w.mixin_supers.get(&c).cloned().unwrap_or_default();
        if called.is_empty() {
            return;
        }
        let full = self.owner_full_name(self.w.syms.class(c).owner, &self.name(self.w.syms.class(c).name));
        for m in called {
            let info = self.sym_info(m);
            // The member as the trait sees it, scalac's `superInfo`: a generic parent's type
            // parameters bound as the trait's base type binds them.
            let sig = self.w.member_sig_seen_from(m, c);
            let addr = self.buf.addr();
            self.buf.byte(DEFDEF);
            let len = self.buf.begin_length();
            let n = self.super_accessor_name(&full, &self.name(info.name));
            self.buf.nat(n as u64);
            let tmark = self.tparams.len();
            let pmark = self.params.len();
            self.def_params(&sig.tparams, &sig.clauses, None);
            self.tpt(sig.ret);
            self.tparams.truncate(tmark);
            self.params.truncate(pmark);
            self.write_flags(&[ARTIFACT]);
            self.synthetic_position(addr);
            self.buf.end_length(len);
        }
    }

    /// `super$a$b$C$$m`: the super accessor of an expanded name, the trait's qualified name
    /// as expand-prefix names.
    fn super_accessor_name(&mut self, trait_full: &str, member: &str) -> u32 {
        use super::names::Key as NK;
        let mut segs = trait_full.split(['.', '$']).filter(|s| !s.is_empty());
        let first = segs.next().unwrap_or("");
        let mut prefix = self.names.simple(first);
        for seg in segs {
            let m = self.names.simple(seg);
            prefix = self.names.get(NK::ExpandPrefix(prefix, m));
        }
        let member = self.names.simple(member);
        let expanded = self.names.get(NK::Expanded(prefix, member));
        self.names.get(NK::SuperAccessor(expanded))
    }

    /// The forwarders an `export` clause makes, and the clause itself: a type as a final alias,
    /// a val or an object as a stable final def of its singleton type, a def as a final def of
    /// the same signature, all marked exported.
    fn export_forwarders(&mut self, c: ClassId) {
        let plans = self.w.class_export_plans(c);
        self.export_clauses(plans);
    }

    /// The forwarders of a file's top-level `export` clauses in package `pkg`, members of its
    /// `<file>$package` object as scalac makes them.
    fn package_export_forwarders(&mut self, pkg: PkgId) {
        let plans = self.w.package_export_plans(self.file, pkg);
        self.export_clauses(plans);
    }

    /// The export clauses of a class or a file's package, with the forwarders each makes, as
    /// the typer's plan selects them (`typer::export_plan`), which the JVM writes too.
    fn export_clauses(&mut self, plans: Vec<crate::typer::export_plan::ClausePlan>) {
        for plan in plans {
            let crate::typer::export_plan::ClausePlan { path, sels, q, types, objects, forwards } = plan;
            let Some(q) = q else {
                let path: Vec<String> = path.iter().map(|&n| self.name(n)).collect();
                self.fail(format!("an export whose qualifier is neither an object nor a package ({})", path.join(".")));
                continue;
            };
            // The clause.
            self.mark_tree();
            self.buf.byte(EXPORT);
            let len = self.buf.begin_length();
            self.mark_tree();
            self.export_qualifier_ref(q);
            for sel in &sels {
                match sel {
                    crate::ast::ImportSel::Name(n, rename) => {
                        let nn = self.simple_name(*n);
                        self.mark_tree();
                        self.buf.byte(IMPORTED);
                        self.buf.nat(nn as u64);
                        if let Some(r) = rename {
                            let rn = self.simple_name(*r);
                            self.mark_tree();
                            self.buf.byte(RENAMED);
                            self.buf.nat(rn as u64);
                        }
                    }
                    crate::ast::ImportSel::Wildcard => {
                        let nn = self.names.simple("_");
                        self.mark_tree();
                        self.buf.byte(IMPORTED);
                        self.buf.nat(nn as u64);
                    }
                    crate::ast::ImportSel::Given => {
                        let nn = self.names.simple("");
                        self.mark_tree();
                        self.buf.byte(IMPORTED);
                        self.buf.nat(nn as u64);
                    }
                }
            }
            self.buf.end_length(len);
            for (n, t) in types {
                let addr = self.buf.addr();
                self.buf.byte(TYPEDEF);
                let l = self.buf.begin_length();
                let nn = self.simple_name(n);
                self.buf.nat(nn as u64);
                self.mark_tree();
                self.buf.byte(TYPEBOUNDS);
                let b = self.buf.begin_length();
                self.exported_type(n, t);
                self.buf.end_length(b);
                self.write_flags(&[FINAL, EXPORTED]);
                self.synthetic_position(addr);
                self.buf.end_length(l);
            }
            for (n, original, _) in objects {
                self.export_object_forwarder(n, original, q);
            }
            for (n, sym, selected) in forwards {
                self.export_forwarder(n, sym, q, selected);
                self.export_default_forwarders(n, sym, q, selected);
            }
        }
    }

    /// The forwarder of an object, `final def n: q.original.type = q.original`, stable, as scalac
    /// writes it for an object and for a case class's companion.
    fn export_object_forwarder(&mut self, n: Name, original: Name, q: crate::typer::exports::ExportQualifier) {
        let addr = self.buf.addr();
        self.buf.byte(DEFDEF);
        let len = self.buf.begin_length();
        let nn = self.simple_name(n);
        self.buf.nat(nn as u64);
        let on = self.simple_name(original);
        self.mark_tree();
        self.buf.byte(TERMREF);
        self.buf.nat(on as u64);
        self.export_qualifier_ref(q);
        self.term_at(None);
        self.buf.byte(SELECT);
        self.buf.nat(on as u64);
        self.export_qualifier_term(q);
        self.count_body(Producer::Forwarder);
        self.write_flags(&[FINAL, EXPORTED, STABLE]);
        self.synthetic_position(addr);
        self.buf.end_length(len);
    }

    /// The qualifier of an export as a term: the object, or the package.
    fn export_qualifier_term(&mut self, q: crate::typer::exports::ExportQualifier) {
        use crate::typer::exports::ExportQualifier;
        self.term_at(None);
        match q {
            ExportQualifier::Object(o) => self.module_term(o),
            ExportQualifier::Package(p) => self.package_ref(p),
        }
    }

    /// The forwarders of the default getters of the exported method `sym`, as scalac writes them
    /// after its forwarder: `final def n$default$i: T @uncheckedVariance = q.m$default$i`.
    fn export_default_forwarders(&mut self, n: Name, sym: SymId, q: crate::typer::exports::ExportQualifier, selected: Name) {
        // An overloaded name forwards every alternative's getters.
        if let Some(alts) = self.w.syms.alternatives(sym).map(|a| a.to_vec()) {
            for a in alts {
                self.export_default_forwarders(n, a, q, selected);
            }
            return;
        }
        let info = self.sym_info(sym);
        if info.kind != SymKind::Def {
            return;
        }
        let sig = self.seen_through(sym, q);
        if !sig.clauses.iter().any(|c| c.params.iter().any(|p| p.has_default)) {
            return;
        }
        let (fwd, original) = (self.name(n), self.name(selected));
        let order = declared_clause_order(self.w, &info, &sig);
        let declared: Vec<ClauseSig> = order.iter().map(|&i| sig.clauses[i].clone()).collect();
        let mut index = 0u32;
        for (pos, &ci) in order.iter().enumerate() {
            for p in sig.clauses[ci].params.clone() {
                if p.has_default {
                    self.export_default_forwarder(&fwd, &original, index, &sig.tparams, &declared[..pos], p.ty, q);
                }
                index += 1;
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn export_default_forwarder(&mut self, fwd: &str, original: &str, index: u32, tparams: &[TParamId], before: &[ClauseSig], ty: TypeId, q: crate::typer::exports::ExportQualifier) {
        let addr = self.buf.addr();
        self.buf.byte(DEFDEF);
        let len = self.buf.begin_length();
        let n = self.names.default_getter(fwd, index);
        self.buf.nat(n as u64);
        let tmark = self.tparams.len();
        let pmark = self.params.len();
        self.synthetic_params = true;
        self.def_params(tparams, before, None);
        self.synthetic_params = false;
        self.mark_tree();
        self.unchecked_variance(ty);
        let mark = self.body_begin();
        if tparams.is_empty() && before.is_empty() {
            self.term_at(None);
            self.buf.byte(SELECT);
            let g = self.names.default_getter(original, index);
            self.buf.nat(g as u64);
            self.export_qualifier_term(q);
        } else {
            self.fail("a default getter's forwarder that takes parameters".to_string());
        }
        match self.body_end(mark) {
            Ok(()) => self.count_body(Producer::Forwarder),
            Err(reason) => {
                self.count_withheld(&reason);
                self.elided(ty);
            }
        }
        self.tparams.truncate(tmark);
        self.params.truncate(pmark);
        self.write_flags(&[FINAL, EXPORTED]);
        self.synthetic_position(addr);
        self.buf.end_length(len);
    }

    /// What an exported type forwarder aliases: the class, an alias as the reference to it
    /// (`Kinds.T`), a parameterized alias as the lambda of its right-hand side, as scalac writes
    /// them.
    fn exported_type(&mut self, n: Name, t: crate::typer::TypeRef) {
        use crate::typer::TypeRef;
        // An alias the qualifier inherits, read through its `this` (`TypeRef(Vocab.this, Label)`):
        // the forwarder refers to the alias it names, as for one the qualifier declares.
        let t = match t {
            TypeRef::Member(c, m) => {
                let bases: Vec<ClassId> = self.w.syms.class(c).base_types.iter().map(|&(b, _)| b).collect();
                bases.into_iter().find_map(|b| self.w.syms.class(b).type_aliases.get(&m).copied()).map_or(t, TypeRef::Alias)
            }
            _ => t,
        };
        match t {
            TypeRef::Class(k) => {
                let ty = if self.w.syms.class(k).tparams.is_empty() { self.w.types.class(k, &[]) } else { self.w.types.mk(Type::Ctor(k)) };
                self.ty(ty);
            }
            TypeRef::Alias(a) => {
                self.w.complete_alias(a);
                let info = self.alias_info(a);
                if info.tparams.is_empty() {
                    self.alias_ref(a);
                } else {
                    let params: Vec<TypeId> = info.tparams.iter().map(|&p| self.w.types.mk(Type::Param(p))).collect();
                    self.lambda_type(TYPELAMBDATYPE, &params, info.rhs);
                }
            }
            _ => {
                let name = self.name(n);
                self.fail(format!("the exported type {}, a member of a value", name));
                self.ty(ANY);
            }
        }
    }

    /// Whether a definition of `owner` is a member an export from `q` reaches: a member of the
    /// object or of one of its base classes, or a top-level definition of the package (in its
    /// own right or in one of its files' package objects).
    /// Whether the name is one the object an export takes members from exports itself: a
    /// forwarder of a forwarder, which scalac writes to the qualifier's own.
    pub(super) fn qualifier_has(&self, q: crate::typer::exports::ExportQualifier, owner: Owner) -> bool {
        self.w.qualifier_has(q, owner)
    }

    fn export_qualifier_ref(&mut self, q: crate::typer::exports::ExportQualifier) {
        use crate::typer::exports::ExportQualifier;
        match q {
            ExportQualifier::Object(o) => self.module_termref(o),
            ExportQualifier::Package(p) => self.package_ref(p),
        }
    }

    /// The forwarder `n` of `sym`, which the qualifier `q` has under the name `selected`: its own
    /// member's, or the name another export of `q` forwards it under.
    fn export_forwarder(&mut self, n: Name, sym: SymId, q: crate::typer::exports::ExportQualifier, selected: Name) {
        // An overloaded name forwards every alternative.
        if let Some(alts) = self.w.syms.alternatives(sym).map(|a| a.to_vec()) {
            for a in alts {
                self.export_forwarder(n, a, q, selected);
            }
            return;
        }
        let info = self.sym_info(sym);
        let addr = self.buf.addr();
        self.buf.byte(DEFDEF);
        let len = self.buf.begin_length();
        let nn = self.simple_name(n);
        self.buf.nat(nn as u64);
        // A given object (`given x: T with {...}`) is an object, its forwarder's result its singleton.
        let given_object = info.kind == SymKind::Given && info.impl_class.is_some() && {
            let sig = self.w.sig_of(sym);
            sig.tparams.is_empty() && sig.clauses.is_empty()
        };
        let stable = matches!(info.kind, SymKind::Val | SymKind::Object(_)) && info.mods & mods::LAZY == 0 || matches!(info.kind, SymKind::Object(_)) || given_object;
        let mut flags = vec![FINAL, EXPORTED];
        // An inline member's forwarder is inline, a transparent one's transparent, as scalac's
        // `Exporter` makes it.
        if info.mods & mods::INLINE != 0 {
            flags.push(INLINE);
        }
        if info.mods & mods::TRANSPARENT != 0 {
            flags.push(TRANSPARENT);
        }
        // A given's forwarder is a given, as scalac's export keeps it, which an import of the
        // object's givens takes.
        if self.w.syms.is_scala3_given(sym) {
            flags.push(GIVEN);
        } else if info.mods & mods::IMPLICIT != 0 {
            flags.push(IMPLICIT);
        }
        if stable {
            // `x.type` of the member through the object.
            self.mark_tree();
            let at = self.buf.addr();
            self.buf.byte(TERMREF);
            let m = self.simple_name(selected);
            self.buf.nat(m as u64);
            self.export_qualifier_ref(q);
            // `qualifier.x`, the singleton's term; the outline's right-hand side has its type.
            let mark = self.body_begin();
            self.forwarder_body(sym, q, &[], &[], selected);
            match self.body_end(mark) {
                Ok(()) => self.count_body(Producer::Forwarder),
                Err(reason) => {
                    self.count_withheld(&reason);
                    self.elided_bodies += 1;
                    self.mark_tree();
                    self.elided_byte();
                    self.shared_type_at(at);
                }
            }
            flags.push(STABLE);
        } else {
            let sig = self.seen_through(sym, q);
            let tmark = self.tparams.len();
            let pmark = self.params.len();
            if matches!(info.kind, SymKind::Def | SymKind::Given) {
                self.synthetic_params = true;
                self.method_params(&info, &sig);
                self.synthetic_params = false;
            }
            self.tpt(sig.ret);
            let tps: Vec<TypeId> = sig.tparams.iter().map(|&tp| self.w.types.param(tp)).collect();
            let clauses = self.param_addrs(pmark, &sig.clauses);
            let mark = self.body_begin();
            self.forwarder_body(sym, q, &tps, &clauses, selected);
            match self.body_end(mark) {
                Ok(()) => self.count_body(Producer::Forwarder),
                Err(reason) => {
                    self.count_withheld(&reason);
                    self.elided(sig.ret);
                }
            }
            self.tparams.truncate(tmark);
            self.params.truncate(pmark);
            if info.is_extension {
                flags.push(EXTENSION);
            }
        }
        self.write_flags(&flags);
        // The member's `@targetName`, which scalac's `addForwarder` copies with its other
        // annotations: the forwarder keeps the export's name, the class files the target's. Read
        // alike from a source and from products, so a split build writes the whole build's.
        if let Some(target) = self.target_name(sym) {
            self.target_name_annotation(&target);
        }
        self.synthetic_position(addr);
        self.buf.end_length(len);
    }

    /// A member's signature as the object it is exported from sees it: a member of a generic
    /// parent (`object Strings extends Impl[String]`) with the parent's type parameters bound.
    fn seen_through(&mut self, sym: SymId, q: crate::typer::exports::ExportQualifier) -> MethodSig {
        self.w.export_seen_through(sym, q)
    }

    /// The body's definitions in source order, as the AST lists them.
    fn body_order(&mut self, c: ClassId, info: &ClassInfo) -> Vec<BodyItem> {
        let mut out = Vec::new();
        let Some(d) = info.def else {
            // A var's setter is written with the var (`setter`).
            for &s in &info.member_order {
                if self.w.syms.sym(s).mods & mods::SETTER == 0 {
                    out.push(BodyItem::Sym(s));
                }
            }
            return out;
        };
        let ast = self.w.ast(info.file);
        let f = info.file.0 as usize;
        let body: Vec<crate::ast::DefId> = match &ast.def(d).kind {
            DefKind::Class(cls) => cls.body.iter().filter_map(|s| if let crate::ast::Stmt::Def(d) = s { Some(*d) } else { None }).collect(),
            DefKind::Given(g) => g.body.iter().filter_map(|s| if let crate::ast::Stmt::Def(d) = s { Some(*d) } else { None }).collect(),
            _ => Vec::new(),
        };
        for d in body {
            let def = ast.def(d);
            if let Some(&k) = self.w.def_classes.get(f, &d) {
                if self.w.syms.class(k).kind == ClassKind::EnumCase {
                    continue;
                }
                if matches!(def.kind, DefKind::Class(_)) || self.w.syms.class(k).kind == ClassKind::Opaque {
                    out.push(BodyItem::Class(k));
                    continue;
                }
            }
            if let Some(&s) = self.w.def_syms.get(f, &d) {
                // A class an expansion copies has members of its own for the definitions the
                // map gives the last copy's.
                let s = if self.w.syms.sym(s).owner == Owner::Class(c) {
                    s
                } else {
                    info.member_order.iter().chain(&info.extensions).chain(&info.givens).copied().find(|&m| self.w.syms.sym(m).def == Some(d) && self.w.syms.sym(m).owner == Owner::Class(c)).unwrap_or(s)
                };
                out.push(BodyItem::Sym(s));
                // A given with a body is a class next to its instance.
                continue;
            }
            if let Some(&a) = self.w.def_aliases.get(f, &d) {
                out.push(BodyItem::Alias(a));
                continue;
            }
            if let Some(&k) = self.w.def_classes.get(f, &d) {
                out.push(BodyItem::Class(k));
            }
        }
        out
    }

    fn nested_class(&mut self, k: ClassId) {
        let kind = self.w.syms.class(k).kind;
        match kind {
            ClassKind::Object => self.object_def(k),
            ClassKind::Opaque => self.opaque_def(k),
            ClassKind::EnumCase if self.w.syms.class(k).singleton.is_some() => {}
            _ => {
                if let Some(v) = self.w.syms.class(k).inner_object {
                    self.inner_object_def(k, v);
                    return;
                }
                self.class_def(k);
                if self.needs_synthetic_companion(k) {
                    self.synthetic_companion(k);
                }
                // An enum's companion, which holds its cases, is the typer's where the source
                // writes none: it goes with the enum.
                if kind == ClassKind::Enum {
                    if let Some(co) = self.w.syms.class(k).companion.filter(|&co| self.w.syms.class(co).def.is_none()) {
                        self.object_def(co);
                    }
                }
            }
        }
    }

    fn member(&mut self, c: ClassId, s: SymId) {
        let info = self.sym_info(s);
        match info.kind {
            SymKind::Object(o) => {
                if !self.places.contains_key(&Key::Class(o)) {
                    self.object_def(o);
                }
            }
            SymKind::Overloaded(_) => {
                let alts: Vec<SymId> = self.w.syms.alternatives(s).map(|a| a.to_vec()).unwrap_or_default();
                for a in alts {
                    if self.w.syms.sym(a).owner == Owner::Class(c) {
                        self.member(c, a);
                    }
                }
            }
            SymKind::EnumValue(_) => {}
            SymKind::Given => self.given_def(s),
            SymKind::Val | SymKind::Var | SymKind::Def => {
                if info.name == crate::names::INIT {
                    return;
                }
                if let Some(o) = self.inner_object_of(s) {
                    self.inner_object_def(o, s);
                    return;
                }
                self.value_or_method(s, &[]);
                if info.kind == SymKind::Var {
                    self.setter(s);
                }
            }
            SymKind::Param => {}
        }
    }

    fn inner_object_of(&self, s: SymId) -> Option<ClassId> {
        let owner = self.w.syms.sym(s).owner;
        let Owner::Class(c) = owner else { return None };
        self.index.inner_objects.get(&s).copied().filter(|&k| self.w.syms.class(k).owner == Owner::Class(c))
    }

    /// A val, var or def of a class or of a file's top level.
    fn value_or_method(&mut self, s: SymId, extra: &[u8]) {
        self.value_or_method_as(s, extra, None);
    }

    /// The same with the result type given: a given with a body returns its class.
    fn value_or_method_as(&mut self, s: SymId, extra: &[u8], ret: Option<ClassId>) {
        let name = self.name(self.w.syms.sym(s).name);
        self.current.push(name);
        self.value_or_method_now(s, extra, ret);
        self.current.pop();
    }

    fn value_or_method_now(&mut self, s: SymId, extra: &[u8], ret: Option<ClassId>) {
        let ret_class = ret;
        let info = self.sym_info(s);
        let mut sig = self.w.sig_of(s).clone();
        if let Some(&t) = self.w.inline_results.get(&s) {
            sig.ret = t;
        }
        if let Some(k) = ret {
            let args: Vec<TypeId> = sig.tparams.iter().map(|&t| self.w.types.param(t)).collect();
            sig.ret = self.w.types.class(k, &args);
        }
        // An inline given alias is an inline method (`Desugar`'s), whatever its parameters.
        let is_def = info.kind == SymKind::Def || (info.kind == SymKind::Given && (info.mods & mods::INLINE != 0 || !(sig.tparams.is_empty() && sig.clauses.is_empty())));
        let addr = self.buf.addr();
        self.define(Key::Sym(s), addr);
        self.buf.byte(if is_def { DEFDEF } else { VALDEF });
        let len = self.buf.begin_length();
        let n = self.sym_name(s);
        self.buf.nat(n as u64);
        let tmark = self.tparams.len();
        let pmark = self.params.len();
        if is_def {
            let tree = info.def.and_then(|d| self.w.loaded.as_ref()?.product_def_trees.get(&(info.file, d)).copied());
            self.param_places = tree.map_or(Vec::new(), |(f, a)| self.w.pickled_param_places(f, a));
            self.param_place_next = 0;
            self.method_params(&info, &sig);
            self.param_places.clear();
        }
        let ret = if is_def { sig.ret } else { self.constant_type(s, sig.ret).unwrap_or(sig.ret) };
        self.result_tpt = Some(self.buf.addr());
        self.tpt(ret);
        // `abstract override` has an implementation, which calls the member it overrides.
        let stackable = info.mods & (mods::ABSTRACT | mods::OVERRIDE) == mods::ABSTRACT | mods::OVERRIDE;
        let abstract_member = info.mods & mods::ABSTRACT != 0 && !stackable || self.is_abstract(s);
        match ret_class {
            _ if abstract_member => {}
            // `new C[A..](params..)`, the instance of the class a given with a body defines.
            Some(k) => {
                let targs: Vec<TypeId> = sig.tparams.iter().map(|&t| self.w.types.param(t)).collect();
                let clauses = self.param_addrs(pmark, &sig.clauses);
                let mark = self.body_begin();
                self.synth_new(k, &targs, &clauses);
                match self.body_end(mark) {
                    Ok(()) => self.count_body(Producer::Synthesized("given instance")),
                    Err(reason) => {
                        self.count_withheld(&reason);
                        self.elided(ret);
                    }
                }
            }
            None => {
                let producer = self.member_producer(s);
                self.member_rhs(s, producer, ret);
            }
        }
        self.tparams.truncate(tmark);
        self.params.truncate(pmark);
        let mut flags = self.member_flags(s, &info, is_def);
        flags.extend_from_slice(extra);
        self.write_flags(&flags);
        self.member_annotations(s, is_def);
        let span = self.def_span(info.file, info.def, info.span);
        self.position_def(addr, span, info.span.start, info.file, info.def);
        self.buf.end_length(len);
        self.default_getters(s, &info, &sig);
        if is_def && info.mods & mods::INLINE != 0 && self.w.is_retained_inline(s) {
            self.retained_body(s, &info, &sig);
        }
    }

    /// The type of a constant (`final val x = 1` without a type, `inline val`) that teq's typer
    /// widens: the constant's, as scalac pickles it, so that a reader and zinc's hash of the
    /// member see the value.
    fn constant_type(&self, s: SymId, widened: TypeId) -> Option<TypeId> {
        let b = &self.w.b;
        let (lit, class) = match self.w.constant_value(s)? {
            crate::tir::TExpr::Int(i) => (LitVal::Int(i), b.t_int),
            crate::tir::TExpr::Long(l) => (LitVal::Long(l), b.t_long),
            crate::tir::TExpr::Double(d) => (LitVal::Double(d.to_bits()), b.t_double),
            crate::tir::TExpr::Bool(v) => (LitVal::Bool(v), b.t_boolean),
            crate::tir::TExpr::Char(c) => (LitVal::Char(c), b.t_char),
            crate::tir::TExpr::Str(r) => (LitVal::Str(self.w.interner.intern(&self.w.prog.strings[r.idx()])), b.t_string),
            _ => return None,
        };
        (widened == class).then(|| self.w.types.lit(lit))
    }

    /// A method's type parameters and clauses, an extension's interleaved as scalac's
    /// `paramss` hold them.
    fn method_params(&mut self, info: &SymInfo, sig: &MethodSig) {
        let (ext_tparams, ext_clauses) = (info.ext_tparams as usize, info.ext_clauses as usize);
        let clauses: Vec<ClauseSig> = declared_clause_order(self.w, info, sig).into_iter().map(|i| sig.clauses[i].clone()).collect();
        let name_text = self.name(info.name);
        let ext = (info.is_extension && !name_text.ends_with(':')).then_some((ext_tparams, ext_clauses));
        let source = !self.synthetic_params && self.note_method_param_annotations(info, sig);
        let outer = std::mem::replace(&mut self.source_params, source);
        self.def_params(&sig.tparams, &clauses, ext);
        self.source_params = outer;
    }

    /// The annotations of a source method's parameters and type parameters, for their writing.
    fn note_method_param_annotations(&mut self, info: &SymInfo, sig: &MethodSig) -> bool {
        let Some(d) = info.def else { return false };
        let ast = self.w.ast(info.file);
        let (tparams, clauses) = match &ast.def(d).kind {
            DefKind::Fun(f) => (&f.tparams, &f.clauses),
            DefKind::Given(g) => (&g.tparams, &g.clauses),
            _ => return false,
        };
        if tparams.len() == sig.tparams.len() {
            for (tp, &id) in tparams.iter().zip(&sig.tparams) {
                if !tp.annots.is_empty() {
                    self.tparam_annots.insert(id, (info.file, tp.annots.clone()));
                }
            }
        }
        for p in clauses.iter().flat_map(|c| c.params.iter()) {
            let annots = ast.param_annots(p);
            if annots.is_empty() {
                continue;
            }
            if let Some(ps) = sig.clauses.iter().flat_map(|c| c.params.iter()).find(|q| q.name == p.name) {
                self.param_annots.insert(ps.sym, (info.file, annots.to_vec()));
            }
        }
        true
    }

    /// The annotations of a source class's parameters, its fields' and its constructor's, and
    /// of its type parameters, for their writing; a given's class has the given's.
    fn note_class_param_annotations(&mut self, info: &ClassInfo) {
        let Some(d) = info.def else { return };
        let ast = self.w.ast(info.file);
        let (tparams, clauses) = match &ast.def(d).kind {
            DefKind::Class(cls) => (&cls.tparams, &cls.clauses),
            DefKind::Given(g) => (&g.tparams, &g.clauses),
            _ => return,
        };
        let own = &info.tparams;
        if tparams.len() == own.len() {
            for (tp, &id) in tparams.iter().zip(own) {
                if !tp.annots.is_empty() {
                    self.tparam_annots.insert(id, (info.file, tp.annots.clone()));
                }
            }
        }
        for p in clauses.iter().flat_map(|c| c.params.iter()) {
            let annots = ast.param_annots(p);
            if annots.is_empty() {
                continue;
            }
            for (ci, clause) in info.ctor.iter().enumerate() {
                for (pi, q) in clause.params.iter().enumerate() {
                    if q.name != p.name {
                        continue;
                    }
                    self.param_annots.insert(q.sym, (info.file, annots.to_vec()));
                    if let Some(&field) = info.ctor_syms.get(ci).and_then(|c| c.get(pi)) {
                        self.param_annots.insert(field, (info.file, annots.to_vec()));
                    }
                }
            }
        }
    }

    /// The annotations noted for a parameter's symbol, kept where `dest` keeps them.
    fn param_source_annotations(&mut self, s: SymId, dest: annot::Dest) {
        if let Some((file, annots)) = self.param_annots.get(&s).cloned() {
            self.source_annotations(file, &annots, dest, &mut |_, _| {});
        }
    }

    /// A source class's own annotations, or an object's on its module val and class.
    fn class_source_annotations(&mut self, c: ClassId) {
        let info = self.w.syms.class(c);
        let (file, def) = (info.file, info.def);
        let Some(d) = def else { return };
        let def = self.w.ast(file).def(d);
        if !matches!(def.kind, DefKind::Class(_)) {
            return;
        }
        let annots = def.annots.clone();
        // An implicit class keeps what a given's class does (`PostTyper`).
        let dest = if self.w.syms.class(c).mods & mods::IMPLICIT != 0 { annot::Dest::GivenClass } else { annot::Dest::Any };
        self.source_annotations(file, &annots, dest, &mut |_, _| {});
    }

    /// `m$retainedBody`, which scalac adds beside an inline method that overrides or implements
    /// a member that is not inline: the body a call through that member runs, the method inlined
    /// into it with its own parameters, as teq's dispatch method is typed (`type_retained`): its
    /// transparent calls expanded, its other inline calls calls, inside the `INLINED` of the
    /// method's top-level class with every read of a parameter an `INLINED` of no call.
    fn retained_body(&mut self, s: SymId, info: &SymInfo, sig: &MethodSig) {
        use super::names::Key as NK;
        let addr = self.buf.addr();
        self.buf.byte(DEFDEF);
        let len = self.buf.begin_length();
        let u = self.simple_name(info.name);
        let n = self.names.get(NK::BodyRetainer(u));
        self.buf.nat(n as u64);
        let tmark = self.tparams.len();
        let pmark = self.params.len();
        self.def_params(&sig.tparams, &sig.clauses, None);
        let result = self.buf.addr();
        self.tpt(sig.ret);
        let body = self.w.fun_of_sym.get(&s).and_then(|&f| self.w.prog.funs[f.idx()].body);
        match body {
            Some(b) => {
                let transparent = info.mods & mods::TRANSPARENT != 0;
                let params: Vec<SymId> = sig.clauses.iter().flat_map(|c| c.params.iter().map(|p| p.sym)).collect();
                let mark = self.body_begin();
                let outer = std::mem::replace(&mut self.retained_params, params);
                self.buf.byte(INLINED);
                let l = self.buf.begin_length();
                if transparent {
                    self.body_term(b, sig.ret);
                } else {
                    self.buf.byte(TYPED);
                    let typed = self.buf.begin_length();
                    self.body_term(b, sig.ret);
                    self.buf.byte(SHAREDTERM);
                    self.buf.reference(result);
                    self.buf.end_length(typed);
                }
                self.inline_origin(s);
                self.buf.end_length(l);
                self.retained_params = outer;
                match self.body_end(mark) {
                    Ok(()) => self.count_body(Producer::Synthesized("retained body")),
                    Err(reason) => {
                        self.count_withheld(&reason);
                        self.elided(sig.ret);
                    }
                }
            }
            None => self.rhs(Producer::Elided(Withheld::InlineMethod), sig.ret),
        }
        self.tparams.truncate(tmark);
        self.params.truncate(pmark);
        self.write_flags(&[PRIVATE]);
        self.synthetic_position(addr);
        self.buf.end_length(len);
    }

    fn is_abstract(&self, s: SymId) -> bool {
        let info = self.w.syms.sym(s);
        if info.mods & mods::DEFERRED != 0 {
            return true;
        }
        let Some(d) = info.def else { return false };
        let ast = self.w.ast(info.file);
        match &ast.def(d).kind {
            DefKind::Val { rhs, .. } => rhs.is_none() && !matches!(info.owner, Owner::Package(_)),
            DefKind::Fun(f) => f.body.is_none() && info.intrinsic.is_none(),
            _ => false,
        }
    }

    /// A method's parameters: its type parameters and its clauses, an extension's own type
    /// parameters after the extension's clauses (`ext`: how many type parameters and clauses
    /// the extension gives), as scalac's `paramss` interleave them.
    fn def_params(&mut self, tparams: &[TParamId], clauses: &[ClauseSig], ext: Option<(usize, usize)>) {
        let ids = self.bind_tparams(tparams);
        let (et, ec) = ext.unwrap_or((tparams.len(), 0));
        let (et, ec) = (et.min(tparams.len()), ec.min(clauses.len()));
        if ext.is_none() || et == tparams.len() && ec == 0 {
            self.write_bound_tparams(tparams, &ids, false);
            self.clauses(clauses, None, None);
            return;
        }
        self.write_bound_tparams(&tparams[..et], &ids[..et], false);
        self.clauses(&clauses[..ec], None, None);
        if et < tparams.len() {
            self.write_bound_tparams(&tparams[et..], &ids[et..], false);
        } else if ec > 0 && clauses[ec..].first().map_or(false, |c| !c.params.is_empty()) && !clauses[ec - 1].params.is_empty() {
            self.buf.byte(SPLITCLAUSE);
        }
        self.clauses_from(clauses, ec);
    }

    /// The clauses from the `from`-th on.
    fn clauses_from(&mut self, clauses: &[ClauseSig], from: usize) {
        let mut prev_terms = false;
        for clause in clauses.iter().skip(from) {
            if clause.params.is_empty() {
                self.buf.byte(EMPTYCLAUSE);
                prev_terms = false;
                continue;
            }
            if prev_terms {
                self.buf.byte(SPLITCLAUSE);
            }
            for p in &clause.params {
                self.method_param(p, clause, Some(p.sym));
            }
            prev_terms = true;
        }
    }

    /// A right-hand side this stage leaves `ELIDED`, counted under its producer's reason.
    fn rhs(&mut self, p: Producer, t: TypeId) {
        self.count_withheld(p.name());
        self.elided(t);
    }

    /// A source definition's right-hand side: its typed body where this stage writes it.
    fn member_rhs(&mut self, s: SymId, p: Producer, t: TypeId) {
        if matches!(p, Producer::Elided(Withheld::Untyped)) && self.bodies_open > 0 {
            return self.withhold(Withheld::MacroClass);
        }
        if matches!(p, Producer::InlineBody | Producer::MacroBody) {
            return self.inline_rhs(s, t, p);
        }
        let body = match (p, self.root_of(s)) {
            (Producer::Source, Some(Root::Fun(f))) => self.w.prog.funs[f.idx()].body,
            (Producer::Source, Some(Root::Init(e))) => Some(e),
            _ => None,
        };
        match body {
            Some(b) => {
                let mark = self.body_begin();
                // An import of the reflection API the body makes (`import r.reflect.*`) names the
                // `Quotes` its paths take, which the pickle keeps no import of to tell.
                let scopes = self.reflect_scopes.len();
                if let Some(q) = self.body_reflect_import(s) {
                    self.reflect_scopes.push((q, self.splice_params.len()));
                }
                self.body_term(b, t);
                self.reflect_scopes.truncate(scopes);
                match self.body_end(mark) {
                    Ok(()) => self.count_body(p),
                    Err(reason) => {
                        self.count_withheld(&reason);
                        self.elided(t);
                    }
                }
            }
            None if p == Producer::Synthesized("js.native") => self.synth_rhs("js.native", t, Synth::JsNative, &[]),
            None => self.rhs(p, t),
        }
    }

    /// The accessors of the class's inline bodies (`typer::accessors`), after its members as
    /// scalac's `PrepareInlineable` appends them, each `<synthetic>`.
    fn inline_accessor_defs(&mut self, c: ClassId) {
        let mut accs: Vec<SymId> = self.w.inline_accessor_syms.values().copied().filter(|&a| self.w.syms.sym(a).owner == Owner::Class(c)).collect();
        accs.sort();
        for a in accs {
            self.value_or_method(a, &[SYNTHETIC]);
        }
    }

    /// The imports of the file that stand before `at`, as scalac's pickler keeps every import of a
    /// package's statements in each of the file's pickles (`Pickler.sliceTopLevel`): what an
    /// inline expansion's search and a regenerated body's see there.
    fn file_imports_before(&mut self, at: u32) {
        let file = self.file;
        let clauses: Vec<(Span, crate::typer::ResolvedImport)> = self.w.file_import_clauses(file).into_iter().filter(|(span, _)| span.start < at).collect();
        // A clause's selectors share its start (`import a.{b, c}`): one `IMPORT` of them all.
        let mut i = 0;
        while i < clauses.len() {
            let j = i + clauses[i..].iter().take_while(|(s, _)| s.start == clauses[i].0.start).count();
            let span = Span { start: clauses[i].0.start, end: clauses[j - 1].0.end };
            let imports: Vec<crate::typer::ResolvedImport> = clauses[i..j].iter().map(|&(_, r)| r).collect();
            self.import_group(Some(span), &imports);
            i = j;
        }
    }

    /// What a resolved import selects on and the member it names (`None` for a wildcard or the
    /// givens). An import of a class's members is none: the bodies name them by their paths.
    fn import_qual(&self, import: &crate::typer::ResolvedImport) -> Option<(ImportQual, Option<Name>)> {
        use crate::typer::ImportTarget as T;
        let (qual, member) = match import.target {
            T::PkgMember(p, m) => (ImportQual::Pkg(p), Some(m)),
            T::PkgAll(p) | T::PkgGivens(p) => (ImportQual::Pkg(p), None),
            T::ClassMember(c, m) => (ImportQual::Object(c), Some(m)),
            T::ClassAll(c) | T::ClassGivens(c) => (ImportQual::Object(c), None),
            T::ValueMember(v, m) => (ImportQual::Value(v), Some(m)),
            T::ValueAll(v) | T::ValueGivens(v) => (ImportQual::Value(v), None),
            _ => return None,
        };
        match qual {
            ImportQual::Object(c) if self.w.syms.class(c).kind != ClassKind::Object => None,
            _ => Some((qual, member)),
        }
    }

    pub(super) fn import_tree(&mut self, span: Option<Span>, import: &crate::typer::ResolvedImport) {
        self.import_group(span, std::slice::from_ref(import))
    }

    /// `IMPORT qual selector..` of resolved imports, one per run of a qualifier: its qualifier the
    /// package, object or stable value it selects on, a selector per member (`RENAMED` where the
    /// import renames it), `_` for a wildcard, the empty name for `given`.
    fn import_group(&mut self, span: Option<Span>, imports: &[crate::typer::ResolvedImport]) {
        use crate::typer::ImportTarget as T;
        let quals: Vec<(ImportQual, Option<Name>, crate::typer::ResolvedImport)> = imports.iter().filter_map(|i| self.import_qual(i).map(|(q, m)| (q, m, *i))).collect();
        let mut i = 0;
        while i < quals.len() {
            let qual = quals[i].0;
            let j = i + quals[i..].iter().take_while(|q| q.0 == qual).count();
            let addr = self.buf.addr();
            match span {
                Some(s) => self.positions.push(Pos { addr, file: self.file, span: s, point: Some(s.start), def: false, switch: self.file != self.src_ctx }),
                None => self.mark_tree(),
            }
            self.buf.byte(IMPORT);
            let len = self.buf.begin_length();
            self.mark_tree();
            match qual {
                ImportQual::Pkg(p) => self.package_ref(p),
                // `import q.reflect.*`, which the typer resolves to the std's object: the `Quotes`
                // the source names, which the trees after it in its block take too.
                ImportQual::Object(c) if self.is_reflect_class(c) => {
                    let at = span.map(|s| (if self.is_own_source(self.src_ctx) { self.src_ctx } else { self.file }, s.start)).or(self.import_at);
                    if let Some(q) = at.and_then(|(file, start)| self.declared_quotes(file, start)) {
                        self.reflect_scopes.push((q, self.splice_params.len()));
                    }
                    self.reflect_import(c)
                }
                ImportQual::Object(c) => self.module_term(c),
                ImportQual::Value(v) => self.value_import_path(v),
            }
            for &(_, member, import) in &quals[i..j] {
                // What a wildcard leaves out, `hidden as _` before it.
                let hidden: Vec<Name> = self.w.import_hidden.as_slice()[import.hidden.range()].to_vec();
                for h in hidden {
                    self.mark_tree();
                    self.buf.byte(IMPORTED);
                    let n = self.names.simple(&self.name(h));
                    self.buf.nat(n as u64);
                    self.mark_tree();
                    self.buf.byte(RENAMED);
                    let w = self.names.simple("_");
                    self.buf.nat(w as u64);
                }
                let selector = match (member, import.target) {
                    (Some(m), _) => self.name(m),
                    (None, T::PkgGivens(_) | T::ClassGivens(_) | T::ValueGivens(_)) => String::new(),
                    (None, _) => "_".to_string(),
                };
                self.mark_tree();
                self.buf.byte(IMPORTED);
                let n = self.names.simple(&selector);
                self.buf.nat(n as u64);
                if let (Some(m), Some(bound)) = (member, import.name) {
                    if bound != m {
                        self.mark_tree();
                        self.buf.byte(RENAMED);
                        let r = self.names.simple(&self.name(bound));
                        self.buf.nat(r as u64);
                    }
                }
            }
            self.buf.end_length(len);
            i = j;
        }
    }

    /// The stable value an import selects on: a local or parameter, a val of an object, or a val
    /// of the value before it (`import o.r.*`).
    fn value_import_path(&mut self, v: crate::typer::ValueImport) {
        let (val, through, prev) = self.w.import_values[v.0 as usize];
        match (prev, through) {
            (Some(p), _) => {
                self.buf.byte(SELECT);
                let n = self.simple_name(self.w.syms.sym(val).name);
                self.buf.nat(n as u64);
                self.value_import_path(p);
            }
            (None, Some(o)) => {
                self.buf.byte(SELECT);
                let n = self.simple_name(self.w.syms.sym(val).name);
                self.buf.nat(n as u64);
                self.module_term(o);
            }
            (None, None) if self.w.syms.sym(val).owner == Owner::Local => self.local_ref(val),
            // A val of a package or an object, defined in another file perhaps (`import p.n`
            // over the package's `val p`).
            (None, None) => self.member_termref(val),
        }
    }

    /// An inline method's body as the definition check stored it, before any expansion: its
    /// reducible nodes unreduced with their markers, the calls it keeps calls, the whole
    /// ascribed the result's tree unless the method is transparent (`PrepareInlineable.wrapRHS`).
    fn inline_rhs(&mut self, s: SymId, t: TypeId, p: Producer) {
        let record = self.w.inline_definitions.get(&s).cloned();
        let Some((record, body)) = record.and_then(|d| d.body.map(|b| (d, b))) else {
            return self.rhs(Producer::Elided(Withheld::InlineMethod), t);
        };
        let transparent = self.w.syms.sym(s).mods & mods::TRANSPARENT != 0;
        let result = self.result_tpt;
        let mark = self.body_begin();
        let outer = self.inline_body.replace(record);
        let outer_owner = std::mem::replace(&mut self.inline_owner, match self.w.syms.sym(s).owner {
            Owner::Class(c) => Some(c),
            _ => None,
        });
        match result.filter(|_| !transparent) {
            Some(at) => {
                self.buf.byte(TYPED);
                let l = self.buf.begin_length();
                self.body_term(body, t);
                self.buf.byte(SHAREDTERM);
                self.buf.reference(at);
                self.buf.end_length(l);
            }
            None => self.body_term(body, t),
        }
        self.inline_body = outer;
        self.inline_owner = outer_owner;
        match self.body_end(mark) {
            Ok(()) => self.count_body(p),
            Err(reason) => {
                self.count_withheld(&reason);
                self.elided(t);
            }
        }
    }

    /// The right-hand side scalac would have: the tree of the result type, marked elided.
    fn elided(&mut self, t: TypeId) {
        self.elided_bodies += 1;
        self.mark_tree();
        self.elided_byte();
        self.ty(t);
    }

    /// The `ELIDED` tag of a withheld right-hand side, recorded with the reason the census
    /// counted for it.
    fn elided_byte(&mut self) {
        let reason = self.pending_withheld.take().unwrap_or_else(|| "elided: no typed body".to_string());
        self.withheld.push((self.buf.addr(), reason));
        self.buf.byte(ELIDED);
    }

    fn member_flags(&self, s: SymId, info: &SymInfo, is_def: bool) -> Vec<u8> {
        let m = info.mods;
        let mut out = Vec::new();
        // A private member of a class is pickled `private[this]` where every access goes
        // through `this`, which teq does not record: it is written so. A private top-level
        // definition is `private[p]` (`Namer` widens it), written in `member_annotations`.
        if m & mods::PRIVATE != 0 && !info.scoped_private && !matches!(info.owner, Owner::Package(_)) {
            out.push(PRIVATE);
            out.push(LOCAL);
        }
        if m & mods::PROTECTED != 0 && !self.has_access_scope(s) {
            out.push(PROTECTED);
        }
        if m & mods::FINAL != 0 && !matches!(info.kind, SymKind::Object(_)) {
            out.push(FINAL);
        }
        if m & (mods::ABSTRACT | mods::OVERRIDE) == mods::ABSTRACT | mods::OVERRIDE {
            out.push(ABSTRACT_OVERRIDE);
        } else if m & mods::OVERRIDE != 0 {
            out.push(OVERRIDE);
        }
        if m & mods::INLINE != 0 {
            out.push(INLINE);
            // An inline method whose body is a splice is a macro.
            if let Some(d) = info.def {
                if let DefKind::Fun(f) = &self.w.ast(info.file).def(d).kind {
                    if f.body.map_or(false, |b| self.w.is_macro_body(info.file, b)) {
                        out.push(MACRO);
                    }
                }
            }
        }
        if m & mods::TRANSPARENT != 0 {
            out.push(TRANSPARENT);
        }
        if m & mods::INFIX != 0 {
            out.push(INFIX);
        }
        if info.kind == SymKind::Given || m & mods::GIVEN != 0 {
            out.push(GIVEN);
        }
        if m & mods::IMPLICIT != 0 && info.kind != SymKind::Given {
            out.push(IMPLICIT);
        }
        if m & mods::LAZY != 0 && !is_def {
            out.push(LAZY);
        }
        if info.kind == SymKind::Var {
            out.push(MUTABLE);
        }
        if info.is_extension {
            out.push(EXTENSION);
        }
        out
    }

    fn member_annotations(&mut self, s: SymId, is_def: bool) {
        let info = self.sym_info(s);
        self.member_access_qualifier(s, &info);
        // An implicit class's conversion carries the class's `@companionMethod` annotations, as
        // `PostTyper` adds them.
        if let Some(k) = is_def.then(|| self.implicit_class_of(s)).flatten() {
            self.given_class_annotations(k, annot::Dest::GivenMethod);
        }
        if let Some(d) = info.def {
            let annots = self.w.ast(info.file).def(d).annots.clone();
            let dest = match (is_def, info.impl_class) {
                (true, Some(_)) => annot::Dest::GivenMethod,
                (true, None) => annot::Dest::Any,
                (false, _) => annot::Dest::Field,
            };
            self.source_annotations(info.file, &annots, dest, &mut |p, a| p.target_name_fallback(info.file, a));
        }
    }

    /// The implicit class whose conversion the method `s` is: an implicit method of the class's
    /// name beside it.
    fn implicit_class_of(&self, s: SymId) -> Option<ClassId> {
        let info = self.w.syms.sym(s);
        if info.mods & mods::IMPLICIT == 0 {
            return None;
        }
        let k = match info.owner {
            Owner::Class(o) => self.w.syms.class(o).nested.get(&info.name).copied(),
            Owner::Package(p) => self.w.syms.pkg(p).entries.get(&info.name).and_then(|e| e.class),
            Owner::Local => None,
        }?;
        let ki = self.w.syms.class(k);
        (ki.mods & mods::IMPLICIT != 0 && ki.def.is_some() && ki.file == info.file).then_some(k)
    }

    /// A class's source annotations that `dest` keeps.
    fn given_class_annotations(&mut self, k: ClassId, dest: annot::Dest) {
        let info = self.w.syms.class(k);
        let (file, def) = (info.file, info.def);
        if let Some(d) = def {
            let annots = self.w.ast(file).def(d).annots.clone();
            self.source_annotations(file, &annots, dest, &mut |_, _| {});
        }
    }

    /// The annotations of the given `s` on the object or the class it defines.
    fn given_source_annotations(&mut self, s: SymId, dest: annot::Dest) {
        let info = self.sym_info(s);
        if let Some(d) = info.def {
            let annots = self.w.ast(info.file).def(d).annots.clone();
            self.source_annotations(info.file, &annots, dest, &mut |_, _| {});
        }
    }

    /// `@targetName` of a build that keeps no annotation's tree, from its string argument.
    fn target_name_fallback(&mut self, file: FileId, a: &crate::ast::Annot) {
        let ast = self.w.ast(file);
        if self.w.interner.get(a.name) == "targetName" {
            if let Some(&arg) = ast.annot_args(a).first() {
                let target = ast.str(arg).to_string();
                self.target_name_annotation(&target);
            }
        }
    }

    fn target_name_annotation(&mut self, target: &str) {
        self.buf.byte(ANNOTATION);
        let len = self.buf.begin_length();
        let tpe = self.buf.addr();
        self.external_typeref("scala.annotation", "targetName");
        self.buf.byte(APPLY);
        let app = self.buf.begin_length();
        self.buf.byte(SELECTIN);
        let sel = self.buf.begin_length();
        let n = self.names.signed("<init>", None, &[SigParam::Type("java.lang.String".to_string())], "scala.annotation.targetName");
        self.buf.nat(n as u64);
        self.buf.byte(NEW);
        self.mark_tree();
        self.shared_type_at(tpe);
        self.shared_type_at(tpe);
        self.buf.end_length(sel);
        let s = self.names.simple(target);
        self.mark_tree();
        self.buf.byte(STRINGCONST);
        self.buf.nat(s as u64);
        self.buf.end_length(app);
        self.buf.end_length(len);
    }

    /// Whether the definition names the scope of its access, `[p]`.
    /// A member's qualified access, `private[p]` or `protected[p]` with the qualifier as the
    /// package or class it names, and a private top-level definition's `private[p]`, which
    /// `member_flags` leaves out.
    fn member_access_qualifier(&mut self, s: SymId, info: &SymInfo) {
        let top_private = info.mods & mods::PRIVATE != 0 && !info.scoped_private && matches!(info.owner, Owner::Package(_));
        if top_private {
            if let Owner::Package(p) = info.owner {
                self.buf.byte(PRIVATEQUALIFIED);
                self.package_typeref(p);
            }
        }
        if info.scoped_private || info.mods & mods::PROTECTED != 0 && self.has_access_scope(s) {
            if let Some(q) = self.scoped_qualifier(s) {
                self.buf.byte(if info.scoped_private { PRIVATEQUALIFIED } else { PROTECTEDQUALIFIED });
                match q {
                    Qualifier::Pkg(p) => self.package_typeref(p),
                    Qualifier::Class(c) => self.class_typeref(c),
                }
            }
        }
    }

    fn has_access_scope(&self, s: SymId) -> bool {
        let d = self.access_declaration(s);
        if self.loaded_access_scope(d).is_some() {
            return true;
        }
        let info = self.w.syms.sym(d);
        let ast = self.w.ast(info.file);
        !ast.access_scopes.is_empty() && ast.access_scopes.iter().any(|&(at, _)| at == info.span.start)
    }

    fn scoped_qualifier(&self, s: SymId) -> Option<Qualifier> {
        let d = self.access_declaration(s);
        let info = self.w.syms.sym(d);
        let q = match self.loaded_access_scope(d) {
            Some(q) => q,
            None => {
                let ast = self.w.ast(info.file);
                ast.access_scopes.iter().find(|&&(at, _)| at == info.span.start).map(|&(_, n)| n)?
            }
        };
        self.enclosing_named(info.owner, q)
    }

    /// The boundary of `protected[q]` or `private[q]` of a definition read from a pickle (the
    /// declaration of an implementation the program's class was given), which the loader holds.
    fn loaded_access_scope(&self, d: SymId) -> Option<Name> {
        if self.w.syms.sym(d).mods & mods::QUALIFIED == 0 {
            return None;
        }
        self.w.loaded.as_ref()?.access_within.get(&d).map(|&(q, _)| q)
    }

    /// The definition whose access `s` has: the deferred given a class's implementation, made
    /// without one, implements (`TClass::deferred_givens`), as dotty's `dcl.copy` keeps its access
    /// and boundary; `s` itself otherwise.
    fn access_declaration(&self, s: SymId) -> SymId {
        let info = self.w.syms.sym(s);
        let (None, SymKind::Given, Owner::Class(c)) = (info.def, info.kind, info.owner) else { return s };
        let Some(&i) = self.index.tclasses.get(&c) else { return s };
        self.w.prog.classes[i as usize].deferred_givens.iter().find(|&&(_, d)| d == s).map_or(s, |&(m, _)| m)
    }

    /// The innermost class or package enclosing `owner`, itself included, named `q`.
    fn enclosing_named(&self, owner: Owner, q: Name) -> Option<Qualifier> {
        let mut owner = owner;
        loop {
            match owner {
                Owner::Class(c) => {
                    let ci = self.w.syms.class(c);
                    if ci.name == q {
                        return Some(Qualifier::Class(c));
                    }
                    owner = ci.owner;
                }
                Owner::Package(p) => {
                    let mut at = Some(p);
                    while let Some(k) = at {
                        if self.w.syms.pkg(k).name == q {
                            return Some(Qualifier::Pkg(k));
                        }
                        at = self.w.syms.pkg(k).parent;
                    }
                    return None;
                }
                Owner::Local => return None,
            }
        }
    }

    /// `m$default$N` for each parameter with a default, beside the method: the method's type
    /// parameters and the clauses before the parameter's, its type `@uncheckedVariance`.
    fn default_getters(&mut self, s: SymId, info: &SymInfo, sig: &MethodSig) {
        let name = self.name(info.name);
        // An inline method's defaults are its stored definition's, which a held body lacks.
        let producer = match self.w.inline_definitions.get(&s) {
            Some(d) if matches!(d.state, crate::tir::DefinitionState::Held(_)) => Producer::Elided(Withheld::HeldDefault),
            _ => Producer::Default,
        };
        // The defaults as the typer typed them, parallel to the parameters.
        let typed: Vec<Option<crate::tir::TExprId>> = match self.w.fun_of_sym.get(&s) {
            Some(&f) => self.w.prog.funs[f.idx()].defaults.clone(),
            None => self.w.inline_definitions.get(&s).map(|d| d.defaults.clone()).unwrap_or_default(),
        };
        // In the declaration's order of the clauses (a right-associative extension's first own
        // clause before its receiver's): the getter's index and earlier clauses are those.
        let order = declared_clause_order(self.w, info, sig);
        let declared: Vec<ClauseSig> = order.iter().map(|&i| sig.clauses[i].clone()).collect();
        let starts: Vec<usize> = sig.clauses.iter().scan(0, |acc, c| {
            let at = *acc;
            *acc += c.params.len();
            Some(at)
        }).collect();
        let mut index = 0u32;
        for (pos, &ci) in order.iter().enumerate() {
            for (k, p) in sig.clauses[ci].params.iter().enumerate() {
                if p.has_default {
                    let default = typed.get(starts[ci] + k).copied().flatten();
                    self.default_getter(&name, index, &sig.tparams, &declared[..pos], p.ty, info.mods & mods::PRIVATE != 0, false, producer, default);
                }
                index += 1;
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn default_getter(&mut self, method: &str, index: u32, tparams: &[TParamId], before: &[ClauseSig], ty: TypeId, private: bool, synthetic: bool, producer: Producer, default: Option<crate::tir::TExprId>) {
        let addr = self.buf.addr();
        self.buf.byte(DEFDEF);
        let len = self.buf.begin_length();
        let n = self.names.default_getter(method, index);
        self.buf.nat(n as u64);
        let tmark = self.tparams.len();
        let pmark = self.params.len();
        self.def_params(tparams, before, None);
        // A by-name parameter's default is the value, not the thunk the typer passes.
        let default = default.map(|e| match self.w.prog.expr(e) {
            crate::tir::TExpr::Lambda(ps, body) if ps.len == 0 && self.w.as_function(ty).is_none() => body,
            _ => e,
        });
        // Of a parameter whose type names the method's type parameters, scalac's getter has the
        // default's type (`foo$default$1[T]: String` of `x: T = "abc"`), from which its calls
        // infer the method's type arguments; teq's typer infers them without it.
        let own_typed = default.and_then(|e| self.w.prog.type_of(e)).map_or(false, |d| {
            !tparams.is_empty() && self.w.mentions_tparam_of(ty, Some(tparams)) && !self.w.mentions_tparam_of(d, Some(tparams))
        });
        self.mark_tree();
        self.unchecked_variance(ty);
        match default.filter(|_| producer == Producer::Default) {
            Some(e) => {
                let mark = self.body_begin();
                if own_typed {
                    self.fail("a default of a type parameter's type, which scalac's getter types as the default".to_string());
                } else {
                    self.body_term(e, ty);
                }
                match self.body_end(mark) {
                    Ok(()) => self.count_body(producer),
                    Err(reason) => {
                        self.count_withheld(&reason);
                        self.elided(ty);
                    }
                }
            }
            None => self.rhs(producer, ty),
        }
        self.tparams.truncate(tmark);
        self.params.truncate(pmark);
        let mut flags = Vec::new();
        if private {
            flags.push(PRIVATE);
        }
        if synthetic {
            flags.push(SYNTHETIC);
        }
        self.write_flags(&flags);
        self.synthetic_position(addr);
        self.buf.end_length(len);
    }

    /// `T @uncheckedVariance`, the type of a default getter.
    fn unchecked_variance(&mut self, t: TypeId) {
        self.buf.byte(ANNOTATEDTYPE);
        let len = self.buf.begin_length();
        self.ty(t);
        self.annotation_tree("scala.annotation.unchecked", "uncheckedVariance");
        self.buf.end_length(len);
    }

    /// A var's setter `x_=(x$1: T): Unit`, which scalac 3.8 pickles for a var of a class and
    /// for a top-level one in its file's `$package`, where a downstream's assignment calls it:
    /// where dotty's `Desugar.isSetterNeeded` makes one (`typer::setters::setter_needed`), with
    /// the var's access (`Desugar.valDef` takes the var's modifiers).
    fn setter(&mut self, s: SymId) {
        let info = self.sym_info(s);
        if !crate::typer::setters::setter_needed(&self.w.syms, s) {
            return;
        }
        let ty = self.w.sig_of(s).ret;
        let addr = self.buf.addr();
        self.buf.byte(DEFDEF);
        let len = self.buf.begin_length();
        let n = self.names.simple(&format!("{}_=", self.name(info.name)));
        self.buf.nat(n as u64);
        let pa = self.buf.addr();
        self.buf.byte(PARAM);
        let pl = self.buf.begin_length();
        let pn = self.names.simple("x$1");
        self.buf.nat(pn as u64);
        self.tpt(ty);
        self.buf.byte(SYNTHETIC);
        self.synthetic_position(pa);
        self.buf.end_length(pl);
        let unit = self.w.b.t_unit;
        self.tpt(unit);
        if !self.is_abstract(s) {
            self.synth_rhs("setter stub", unit, Synth::Unit, &[]);
        }
        let mut flags = Vec::new();
        if info.mods & mods::PRIVATE != 0 && !info.scoped_private && matches!(info.owner, Owner::Class(_)) {
            flags.extend_from_slice(&[PRIVATE, LOCAL]);
        }
        if info.mods & mods::PROTECTED != 0 && !self.has_access_scope(s) {
            flags.push(PROTECTED);
        }
        if info.mods & mods::OVERRIDE != 0 {
            flags.push(OVERRIDE);
        }
        flags.push(MUTABLE);
        flags.push(FIELDACCESSOR);
        self.write_flags(&flags);
        self.member_access_qualifier(s, &info);
        if let Some(d) = info.def {
            let annots = self.w.ast(info.file).def(d).annots.clone();
            self.source_annotations(info.file, &annots, annot::Dest::Setter, &mut |_, _| {});
        }
        self.synthetic_position(addr);
        self.buf.end_length(len);
    }

    /// A type alias, an abstract type member, or a match type's alias.
    fn alias_def(&mut self, a: AliasId) {
        let info = self.alias_info(a);
        self.current.push(self.name(info.name));
        self.alias_def_now(a, &info);
        self.current.pop();
    }

    fn alias_def_now(&mut self, a: AliasId, info: &AliasInfo) {
        let info = info.clone();
        let addr = self.buf.addr();
        self.define(Key::Alias(a), addr);
        self.buf.byte(TYPEDEF);
        let len = self.buf.begin_length();
        let n = self.simple_name(info.name);
        self.buf.nat(n as u64);
        let tmark = self.tparams.len();
        if let Some(DefKind::TypeAlias { tparams, .. }) = info.def.map(|d| &self.w.ast(info.file).def(d).kind) {
            if tparams.len() == info.tparams.len() {
                for (tp, &id) in tparams.iter().zip(&info.tparams) {
                    if !tp.annots.is_empty() {
                        self.tparam_annots.insert(id, (info.file, tp.annots.clone()));
                    }
                }
            }
        }
        if !info.tparams.is_empty() {
            // `type F[A] = ...`: a type lambda whose parameters the right-hand side names.
            self.buf.byte(LAMBDATPT);
            let l = self.buf.begin_length();
            let ids = self.bind_tparams(&info.tparams);
            self.write_bound_tparams(&info.tparams, &ids, true);
            self.alias_rhs(&info);
            self.buf.end_length(l);
        } else {
            self.alias_rhs(&info);
        }
        self.tparams.truncate(tmark);
        let mut flags = Vec::new();
        if let Some(d) = info.def {
            let m = self.w.ast(info.file).def(d).mods;
            self.access_flags(m, &mut flags);
            if m & mods::FINAL != 0 {
                flags.push(FINAL);
            }
            if m & mods::OVERRIDE != 0 {
                flags.push(OVERRIDE);
            }
        }
        if info.bounds.is_some() && matches!(info.owner, Owner::Class(_)) {
            // An abstract member is deferred by having bounds; no flag says so.
        }
        self.write_flags(&flags);
        if let Some(d) = info.def {
            let annots = self.w.ast(info.file).def(d).annots.clone();
            self.source_annotations(info.file, &annots, annot::Dest::Any, &mut |_, _| {});
        }
        let span = self.def_span(info.file, info.def, Span::default());
        let name_at = info.def.map_or(span.start, |d| self.w.ast(info.file).def(d).span.start);
        self.position(addr, span, name_at);
        self.buf.end_length(len);
    }

    fn alias_rhs(&mut self, info: &AliasInfo) {
        self.mark_tree();
        match info.bounds {
            Some((lo, hi)) => {
                // A higher-kinded member's bounds are the lambda's bodies over its parameters
                // (`type F[X] <: List[X]`), not lambdas of their own.
                let (lo, hi) = (self.lambda_body(lo, &info.tparams), self.lambda_body(hi, &info.tparams));
                self.buf.byte(TYPEBOUNDS);
                let len = self.buf.begin_length();
                self.ty(lo);
                self.ty(hi);
                self.buf.end_length(len);
            }
            None => {
                self.buf.byte(TYPEBOUNDS);
                let len = self.buf.begin_length();
                self.ty(info.rhs);
                self.buf.end_length(len);
            }
        }
    }

    /// The body of a type lambda of as many parameters as `tparams`, over them.
    fn lambda_body(&mut self, t: TypeId, tparams: &[TParamId]) -> TypeId {
        let Type::Lambda(ps, body) = self.w.types.get(t) else { return t };
        let ps: Vec<TypeId> = self.w.types.items(ps).to_vec();
        if tparams.is_empty() || ps.len() != tparams.len() {
            return t;
        }
        let mut subst: Subst = Subst::default();
        for (&p, &tp) in ps.iter().zip(tparams) {
            let Type::Param(lp) = self.w.types.get(p) else { return t };
            subst.push((lp, self.w.types.param(tp)));
        }
        self.w.types.subst(body, &subst)
    }

    /// `opaque type T = U`: an alias the defining scope sees through, with its bounds.
    fn opaque_def(&mut self, c: ClassId) {
        let info = self.class_info(c);
        let addr = self.buf.addr();
        self.define(Key::Class(c), addr);
        self.buf.byte(TYPEDEF);
        let len = self.buf.begin_length();
        let n = self.simple_name(info.name);
        self.buf.nat(n as u64);
        let tmark = self.tparams.len();
        let underlying = info.underlying.unwrap_or(ANY);
        let bound = info.parents.first().copied();
        if !info.tparams.is_empty() {
            self.buf.byte(LAMBDATPT);
            let l = self.buf.begin_length();
            let ids = self.bind_tparams(&info.tparams);
            self.write_bound_tparams(&info.tparams, &ids, true);
            self.opaque_rhs(underlying, bound);
            self.buf.end_length(l);
        } else {
            self.opaque_rhs(underlying, bound);
        }
        self.tparams.truncate(tmark);
        let mut flags = Vec::new();
        self.access_flags(info.mods, &mut flags);
        flags.push(OPAQUE);
        self.write_flags(&flags);
        let span = self.def_span(info.file, info.def, info.span);
        self.position_def(addr, span, info.span.start, info.file, info.def);
        self.buf.end_length(len);
    }

    /// The alias of an opaque type, and with a bound (`opaque type Name <: String = String`)
    /// the bounds before it, as `TreePickler` writes a `TypeBoundsTree` with an alias; teq keeps
    /// no lower bound, which is `Nothing`.
    fn opaque_rhs(&mut self, underlying: TypeId, bound: Option<TypeId>) {
        match bound {
            Some(hi) => {
                self.buf.byte(TYPEBOUNDSTPT);
                let b = self.buf.begin_length();
                self.tpt(NOTHING);
                self.tpt(hi);
                self.tpt(underlying);
                self.buf.end_length(b);
            }
            // The alias as a tree, as `TreePickler` writes it: the opaque flag keeps it from the
            // outside, where bounds of one type would read as a transparent alias.
            None => self.tpt(underlying),
        }
    }

    // ---- objects --------------------------------------------------------------------------

    /// An object: its module val `val X: X.type = new X$()` and its class `X$`.
    fn object_def(&mut self, o: ClassId) {
        self.w.complete_class(o);
        let info = self.class_info(o);
        let companion = info.companion.filter(|&k| self.w.syms.class(k).kind != ClassKind::Object);
        // A companion the source does not write (an enum's, one a `derives` clause makes at the
        // class's name) has its class's access, as `Desugar` gives it.
        let made = |p: &Self, k: ClassId| match info.def {
            None => true,
            Some(d) => p.w.ast(info.file).def(d).span == p.w.syms.class(k).span,
        };
        let made_companion = companion.map_or(false, |k| made(self, k));
        let access = match companion {
            Some(k) if made_companion => self.w.syms.class(k).mods & (mods::PRIVATE | mods::PROTECTED),
            // The source's own access of the object, which its class may not hold.
            _ => info.def.map_or(0, |d| self.w.ast(info.file).def(d).mods & (mods::PRIVATE | mods::PROTECTED)),
        };
        let obj = Obj {
            class: Some(o),
            companion,
            name: self.name(info.name),
            owner: info.owner,
            span: self.def_span(info.file, info.def, info.span),
            name_at: info.span.start,
            synthetic: info.def.is_none() || made_companion,
            mods: info.mods | access,
        };
        self.object_unit(&obj);
    }

    /// The companion scalac makes for a case class, a value class or a class whose
    /// constructor has defaults, where the source has none.
    fn synthetic_companion(&mut self, c: ClassId) {
        let info = self.class_info(c);
        let obj = Obj {
            class: None,
            companion: Some(c),
            name: self.name(info.name),
            owner: info.owner,
            span: Span { start: info.span.start, end: info.span.start },
            name_at: info.span.start,
            synthetic: true,
            mods: info.mods & mods::PRIVATE,
        };
        self.object_unit(&obj);
    }

    fn module_key(obj: &Obj) -> Key {
        match (obj.class, obj.companion) {
            (Some(o), _) => Key::Module(o),
            (None, Some(c)) => Key::SynthModule(c),
            (None, None) => unreachable!(),
        }
    }

    fn module_class_key(obj: &Obj) -> Key {
        match (obj.class, obj.companion) {
            (Some(o), _) => Key::Class(o),
            (None, Some(c)) => Key::SynthModuleClass(c),
            (None, None) => unreachable!(),
        }
    }

    fn object_unit(&mut self, obj: &Obj) {
        self.current.push(obj.name.clone());
        self.object_unit_now(obj);
        self.current.pop();
    }

    fn object_unit_now(&mut self, obj: &Obj) {
        let top = matches!(obj.owner, Owner::Package(_)) && !self.in_package_object;
        let (vkey, ckey) = (Self::module_key(obj), Self::module_class_key(obj));
        // The module val.
        let vaddr = self.buf.addr();
        self.define(vkey, vaddr);
        if let Some(ms) = obj.class.and_then(|o| self.w.syms.class(o).module_sym) {
            self.define(Key::Sym(ms), vaddr);
        }
        self.buf.byte(VALDEF);
        let vlen = self.buf.begin_length();
        let vn = self.names.simple(&obj.name);
        self.buf.nat(vn as u64);
        let tpt = self.buf.addr();
        let cn = self.names.object_class(&obj.name);
        self.mark_tree();
        self.buf.byte(IDENTTPT);
        self.buf.nat(cn as u64);
        let tref = self.buf.addr();
        self.placed = true;
        self.buf.byte(TYPEREFSYMBOL);
        self.def_ref(ckey);
        self.owner_prefix(obj.owner);
        self.new_module(&obj.name, obj.owner, tpt, tref);
        let qualified = obj.class.and_then(|o| self.class_access_qualifier(&self.w.syms.class(o).clone()));
        let mut vflags = Vec::new();
        if qualified.is_none() {
            self.access_flags(obj.mods, &mut vflags);
        }
        vflags.push(OBJECT);
        if obj.synthetic {
            vflags.push(SYNTHETIC);
        }
        if obj.mods & mods::CASE != 0 {
            vflags.push(CASE);
        }
        if obj.mods & mods::IMPLICIT != 0 {
            vflags.push(IMPLICIT);
        }
        self.write_flags(&vflags);
        self.write_access_qualifier(qualified);
        if let Some(o) = obj.class {
            self.class_source_annotations(o);
        }
        self.position(vaddr, obj.span, obj.name_at);
        self.buf.end_length(vlen);
        // The module class.
        let caddr = self.buf.addr();
        self.define(ckey, caddr);
        self.buf.byte(TYPEDEF);
        let len = self.buf.begin_length();
        self.buf.nat(cn as u64);
        let saved = self.synth_span;
        self.synth_span = obj.span;
        let appended = self.object_template(obj);
        if appended > 0 {
            self.appended_parents.push((caddr, appended));
        }
        self.synth_span = saved;
        let mut flags = Vec::new();
        if qualified.is_none() {
            self.access_flags(obj.mods, &mut flags);
        }
        flags.push(OBJECT);
        if obj.synthetic {
            flags.push(SYNTHETIC);
        }
        if obj.mods & mods::CASE != 0 {
            flags.push(CASE);
        }
        if obj.class.map_or(false, |o| self.holds_opaque(o)) {
            flags.push(OPAQUE);
        }
        self.write_flags(&flags);
        self.write_access_qualifier(qualified);
        if top {
            self.source_file_annotation();
        }
        if let Some(o) = obj.class {
            self.class_source_annotations(o);
        }
        self.position(caddr, obj.span, obj.name_at);
        self.buf.end_length(len);
    }

    /// Whether an object is static: its owners are packages and objects.
    fn is_static(&self, owner: Owner) -> bool {
        match owner {
            Owner::Package(_) => true,
            Owner::Class(c) => self.w.syms.class(c).kind == ClassKind::Object && self.is_static(self.w.syms.class(c).owner),
            Owner::Local => false,
        }
    }

    /// The module class's template; how many parents it appends after the declared ones.
    fn object_template(&mut self, obj: &Obj) -> u32 {
        let companion_case = obj.companion.filter(|&k| {
            let ki = self.w.syms.class(k);
            ki.mods & mods::CASE != 0 && matches!(ki.kind, ClassKind::Class | ClassKind::EnumCase) && ki.singleton.is_none()
        });
        let companion_enum = obj.companion.filter(|&k| self.w.syms.class(k).kind == ClassKind::Enum);
        let case_object = obj.mods & mods::CASE != 0;
        self.buf.byte(TEMPLATE);
        let len = self.buf.begin_length();
        // Parents.
        let declared: Vec<TypeId> = obj.class.filter(|_| !obj.synthetic).map(|o| self.w.syms.class(o).parents.clone()).unwrap_or_default();
        let declared: Vec<TypeId> = declared.into_iter().filter(|&t| t != self.w.b.t_any_ref).collect();
        if obj.synthetic {
            let t = self.w.b.t_any_ref;
            self.parent_as_anyref = true;
            self.parent_call(ClassId(u32::MAX), t);
        } else if declared.first().map_or(true, |&p| self.is_trait_type(p)) {
            let t = self.w.b.t_any_ref;
            self.parent_call(obj.class.unwrap_or(ClassId(u32::MAX)), t);
        }
        if let Some(o) = obj.class {
            self.enclosing.push(o);
        }
        for (i, &p) in declared.iter().enumerate() {
            if i == 0 && !self.is_trait_type(p) {
                self.parent_call(obj.class.unwrap_or(ClassId(u32::MAX)), p);
            } else if let Some((args, via, prelude)) = obj.class.and_then(|o| self.trait_args(o, p)) {
                self.trait_parent_call(p, &args, via, prelude);
            } else {
                self.tpt(p);
            }
        }
        let mut appended = 0;
        if case_object {
            self.external_tpt("scala", "Product");
            self.external_tpt("java.io", "Serializable");
            self.mark_tree();
            self.mirror_parent("Singleton");
            appended += 3;
        }
        if companion_case.map_or(false, |k| self.has_product_mirror(k)) {
            self.mark_tree();
            self.mirror_parent("Product");
            appended += 1;
        }
        if companion_enum.is_some() {
            self.mark_tree();
            self.mirror_parent("Sum");
            appended += 1;
        }
        // Self.
        self.buf.byte(SELFDEF);
        let us = self.names.simple("_");
        self.buf.nat(us as u64);
        self.buf.byte(SINGLETONTPT);
        self.mark_tree();
        let mut self_type = None;
        if matches!(obj.owner, Owner::Class(o) if self.w.syms.class(o).kind != ClassKind::Object) {
            // An object of a class is its own `this` type's member by name, as scalac writes it:
            // the module val by its address makes scalac's tree checker loop on the self type.
            let n = self.names.simple(&obj.name);
            self.buf.byte(SELECT);
            self.buf.nat(n as u64);
            self.mark_tree();
            self.owner_prefix(obj.owner);
        } else {
            self_type = Some(self.buf.addr());
            self.def_termref(Self::module_key(obj), obj.owner);
        }
        // The constructor.
        let ia = self.buf.addr();
        self.buf.byte(DEFDEF);
        let il = self.buf.begin_length();
        let init = self.names.simple("<init>");
        self.buf.nat(init as u64);
        self.buf.byte(EMPTYCLAUSE);
        let unit = self.w.b.t_unit;
        self.tpt(unit);
        if !self.is_static(obj.owner) || case_object {
            self.buf.byte(STABLE);
        }
        self.synthetic_position(ia);
        self.buf.end_length(il);
        if let Some(o) = obj.class {
            self.count_template_roots(o);
        }
        if self.is_static(obj.owner) {
            self.write_replace(self_type);
        }
        if let Some(e) = companion_enum {
            self.enum_cases(e);
        }
        if let Some(k) = companion_case {
            let info = self.class_info(k);
            self.case_companion_head(k, &info, obj.synthetic);
        }
        if case_object {
            self.case_object_members(obj.class);
        }
        if let Some(o) = obj.class {
            let info = self.class_info(o);
            let body = self.body_order(o, &info);
            let mut stmts = self.template_stmts(o);
            for m in body {
                self.statements_before(o, &mut stmts, self.item_start(&m));
                match m {
                    BodyItem::Sym(s) => self.member(o, s),
                    BodyItem::Class(k) => self.nested_class(k),
                    BodyItem::Alias(a) => self.alias_def(a),
                }
            }
            self.statements_before(o, &mut stmts, None);
            if info.has_exports {
                self.export_forwarders(o);
            }
        }
        if let Some(k) = obj.companion {
            self.constructor_default_getters(k);
        }
        if let Some(k) = companion_case {
            let ki = self.class_info(k);
            if self.has_product_mirror(k) && ki.mods & mods::ABSTRACT == 0 {
                let tparams: Vec<TParamId> = ki.tparams.to_vec();
                self.mirror_members(k, &tparams);
            }
        }
        if let Some(e) = companion_enum {
            self.enum_tail(e);
        }
        if let Some(o) = obj.class {
            self.inline_accessor_defs(o);
            self.enclosing.pop();
        }
        self.buf.end_length(len);
        appended
    }

    fn is_trait_type(&self, t: TypeId) -> bool {
        match self.w.types.get(self.w.types.strip_nested(t)) {
            Type::Class(c, _) => self.w.syms.class(c).kind == ClassKind::Trait && !self.library_class_of_std_trait(c),
            _ => false,
        }
    }

    /// A trait of the lean std that scala-library declares as a class: a parent of it is its
    /// constructor call (`given Conversion[A, B] with` extends `Conversion[A, B]()`).
    fn library_class_of_std_trait(&self, c: ClassId) -> bool {
        let info = self.w.syms.class(c);
        info.kind == ClassKind::Trait && self.w.files.as_slice().get(info.file.0 as usize).map_or(false, |f| f.is_std) && !self.w.in_jar(info.file) && STD_TRAITS_LIBRARY_CLASSES.contains(&scala_name(self.w, c).as_str())
    }

    /// The members `SyntheticMembers` gives a case object.
    fn case_object_members(&mut self, o: Option<ClassId>) {
        let int = self.w.b.t_int;
        let boolean = self.w.b.t_boolean;
        let string = self.w.b.t_string;
        let body = |f: fn(ClassId) -> Synth| o.map_or(Synth::Elided, f);
        self.synth_def("hashCode", &[], true, int, &[OVERRIDE, SYNTHETIC], body(Synth::HashCode));
        self.synth_def("toString", &[], true, string, &[OVERRIDE, SYNTHETIC], body(Synth::ToString));
        self.synth_def("canEqual", &[("that", ANY)], false, boolean, &[OVERRIDE, SYNTHETIC], body(Synth::CanEqual));
        self.synth_def("productArity", &[], false, int, &[OVERRIDE, SYNTHETIC], body(Synth::ProductArity));
        self.synth_def("productPrefix", &[], false, string, &[OVERRIDE, SYNTHETIC], body(Synth::ProductPrefix));
        self.synth_def("productElement", &[("n", int)], false, ANY, &[OVERRIDE, SYNTHETIC], body(Synth::ProductElement));
        self.synth_def("productElementName", &[("n", int)], false, string, &[OVERRIDE, SYNTHETIC], body(Synth::ProductElementName));
    }

    /// `<init>$default$N` in a class's companion for each constructor parameter with a default.
    fn constructor_default_getters(&mut self, k: ClassId) {
        let info = self.class_info(k);
        if !matches!(info.kind, ClassKind::Class | ClassKind::EnumCase | ClassKind::Enum | ClassKind::Trait) {
            return;
        }
        let tparams: Vec<TParamId> = info.tparams.to_vec();
        let clauses = info.ctor.clone();
        // The typed defaults after the captures, parallel to the constructor's parameters.
        let typed: Vec<Option<crate::tir::TExprId>> = self.index.tclasses.get(&k).map(|&i| {
            let tc = &self.w.prog.classes[i as usize];
            tc.ctor_defaults.iter().skip(tc.captures).copied().collect()
        }).unwrap_or_default();
        let mut index = 0u32;
        for (ci, clause) in clauses.iter().enumerate() {
            for p in &clause.params {
                if p.has_default {
                    let default = typed.get(index as usize).copied().flatten();
                    self.default_getter("<init>", index, &tparams, &clauses[..ci], p.ty, false, false, Producer::Default, default);
                }
                index += 1;
            }
        }
    }

    /// An enum's cases in its companion: a value case as a `case` val, a class case as a final
    /// case class with its companion; for an enum whose cases are all values, `$values`,
    /// `values`, `valueOf` and `$new`.
    fn enum_cases(&mut self, e: ClassId) {
        let info = self.class_info(e);
        let enum_ty = self.enum_wild_type(e);
        let mut all_values = true;
        let mut simple = false;
        for &k in &info.children {
            let ki = self.class_info(k);
            match ki.singleton {
                Some(s) => {
                    let ty = self.w.sig_of(s).ret;
                    let addr = self.buf.addr();
                    self.define(Key::Sym(s), addr);
                    self.buf.byte(VALDEF);
                    let len = self.buf.begin_length();
                    let n = self.simple_name(ki.name);
                    self.buf.nat(n as u64);
                    self.tpt(ty);
                    // `$new(ordinal, "Name")` of a simple case, an instance of its own class of another.
                    let body = if self.simple_enum_case(e, k) {
                        simple = true;
                        Synth::EnumValue(e, k)
                    } else {
                        Synth::EnumObject(e, k)
                    };
                    self.synth_rhs("enum value", ty, body, &[]);
                    self.write_flags(&[CASE, STATIC, ENUM]);
                    self.class_source_annotations(k);
                    let span = self.def_span(ki.file, ki.def, ki.span);
                    self.position_def(addr, span, ki.span.start, ki.file, ki.def);
                    self.buf.end_length(len);
                }
                None => {
                    all_values = false;
                    self.class_def(k);
                    self.synthetic_companion(k);
                }
            }
        }
        if all_values && !info.children.is_empty() {
            let array = self.w.b.array;
            let arr = self.w.types.class(array, &[enum_ty]);
            let string = self.w.b.t_string;
            let va = self.buf.addr();
            self.define(Key::EnumValues(e), va);
            self.synth_val_with("$values", arr, &[PRIVATE, LOCAL, SYNTHETIC], Synth::EnumArray(e));
            self.synth_def("values", &[], false, arr, &[SYNTHETIC], Synth::EnumValuesClone(e));
            self.synth_def("valueOf", &[("$name", string)], false, enum_ty, &[SYNTHETIC], Synth::EnumValueOf(e));
        }
        if simple {
            let string = self.w.b.t_string;
            let int = self.w.b.t_int;
            let na = self.buf.addr();
            self.define(Key::EnumNew(e), na);
            self.synth_def("$new", &[("_$ordinal", int), ("$name", string)], false, enum_ty, &[PRIVATE, LOCAL, SYNTHETIC], Synth::EnumNew(e));
        }
    }

    /// A value case scalac's `DesugarEnums` makes with `$new`: of an enum without type
    /// parameters, without a parent of its own.
    fn simple_enum_case(&self, e: ClassId, k: ClassId) -> bool {
        if !self.w.syms.class(e).tparams.is_empty() {
            return false;
        }
        let info = self.w.syms.class(k);
        let Some(d) = info.def else { return false };
        matches!(&self.w.ast(info.file).def(d).kind, DefKind::Class(cls) if cls.parents.is_empty())
    }

    /// `fromOrdinal`, the mirror's `MirroredMonoType` and `ordinal` of an enum's companion.
    fn enum_tail(&mut self, e: ClassId) {
        let enum_ty = self.enum_wild_type(e);
        let int = self.w.b.t_int;
        self.synth_def("fromOrdinal", &[("ordinal", int)], false, enum_ty, &[SYNTHETIC], Synth::FromOrdinal(e));
        let addr = self.buf.addr();
        self.buf.byte(TYPEDEF);
        let len = self.buf.begin_length();
        let n = self.names.simple("MirroredMonoType");
        self.buf.nat(n as u64);
        self.mark_tree();
        self.buf.byte(TYPEBOUNDS);
        let b = self.buf.begin_length();
        let n_tparams = self.w.syms.class(e).tparams.len();
        self.mono_type(e, n_tparams);
        self.buf.end_length(b);
        self.write_flags(&[SYNTHETIC]);
        self.synthetic_position(addr);
        self.buf.end_length(len);
        let oa = self.buf.addr();
        self.buf.byte(DEFDEF);
        let ol = self.buf.begin_length();
        let on = self.names.simple("ordinal");
        self.buf.nat(on as u64);
        let pa = self.buf.addr();
        self.buf.byte(PARAM);
        let pl = self.buf.begin_length();
        let pn = self.names.simple("x$0");
        self.buf.nat(pn as u64);
        self.placed = true;
        self.mark_tree();
        self.buf.byte(TYPEREFSYMBOL);
        self.buf.reference(addr);
        self.buf.byte(THIS);
        self.enclosing_this_typeref();
        self.synthetic_position(pa);
        self.buf.end_length(pl);
        self.tpt(int);
        self.synth_rhs("enum ordinal", int, Synth::SelectOrdinal(e), &[pa]);
        self.write_flags(&[SYNTHETIC]);
        self.synthetic_position(oa);
        self.buf.end_length(ol);
    }

    /// The enum's type with a wildcard for each type parameter.
    fn enum_wild_type(&mut self, e: ClassId) -> TypeId {
        let n = self.w.syms.class(e).tparams.len();
        let args: Vec<TypeId> = (0..n).map(|_| WILD).collect();
        self.w.types.class(e, &args)
    }

    fn synth_val_with(&mut self, name: &'static str, ty: TypeId, flags: &[u8], body: Synth) {
        let addr = self.buf.addr();
        self.buf.byte(VALDEF);
        let len = self.buf.begin_length();
        let n = self.names.simple(name);
        self.buf.nat(n as u64);
        self.tpt(ty);
        self.synth_rhs(name, ty, body, &[]);
        self.write_flags(flags);
        self.synthetic_position(addr);
        self.buf.end_length(len);
    }

    fn holds_opaque(&self, o: ClassId) -> bool {
        self.w.syms.class(o).nested.values().any(|&k| self.w.syms.class(k).kind == ClassKind::Opaque)
    }

    /// `new X$()` as a module val's right-hand side.
    fn new_module(&mut self, name: &str, owner: Owner, tpt: usize, tref: usize) {
        self.count_body(Producer::ModuleVal);
        self.buf.byte(APPLY);
        let app = self.buf.begin_length();
        self.buf.byte(SELECTIN);
        let sel = self.buf.begin_length();
        let full = format!("{}$", self.owner_full_name(owner, name));
        let n = self.names.signed("<init>", None, &[], &full);
        self.buf.nat(n as u64);
        self.buf.byte(NEW);
        self.buf.byte(SHAREDTERM);
        self.buf.reference(tpt);
        // The owner of the constructor: the module class's type, the tree's type above.
        self.buf.byte(SHAREDTYPE);
        self.buf.reference(tref);
        self.buf.end_length(sel);
        self.buf.end_length(app);
    }

    /// The qualified name of a definition `name` of `owner`, as a signature spells a class.
    fn owner_full_name(&self, owner: Owner, name: &str) -> String {
        match owner {
            Owner::Package(p) => {
                let path = self.pkg_path(p);
                if path.is_empty() { name.to_string() } else { format!("{}.{}", path, name) }
            }
            Owner::Class(c) => {
                let ci = self.w.syms.class(c);
                let outer = self.owner_full_name(ci.owner, &self.name(ci.name));
                let sep = if ci.kind == ClassKind::Object { "$." } else { "." };
                format!("{}{}{}", outer, sep, name)
            }
            Owner::Local => name.to_string(),
        }
    }

    /// An object nested in a class: in scalac a module val and a module class, as at the top.
    fn inner_object_def(&mut self, k: ClassId, v: SymId) {
        self.define(Key::Sym(v), self.buf.addr());
        self.add_local(Key::Module(k));
        self.object_def(k);
    }

    /// `private def writeReplace(): AnyRef`, which scalac gives every object: `new
    /// ModuleSerializationProxy(classOf[X.type])`, the singleton type the object's self type
    /// has at `self_type`.
    fn write_replace(&mut self, self_type: Option<usize>) {
        let addr = self.buf.addr();
        self.buf.byte(DEFDEF);
        let len = self.buf.begin_length();
        let n = self.names.simple("writeReplace");
        self.buf.nat(n as u64);
        self.buf.byte(EMPTYCLAUSE);
        let any_ref = self.w.b.t_any_ref;
        self.tpt(any_ref);
        match self_type {
            Some(at) => {
                let mark = self.body_begin();
                self.write_replace_body(|p| p.shared_type_at(at));
                match self.body_end(mark) {
                    Ok(()) => self.count_body(Producer::Synthesized("writeReplace")),
                    Err(reason) => {
                        self.count_withheld(&reason);
                        self.elided(any_ref);
                    }
                }
            }
            None => self.rhs(Producer::Synthesized("writeReplace"), any_ref),
        }
        self.write_flags(&[PRIVATE, SYNTHETIC]);
        self.synthetic_position(addr);
        self.buf.end_length(len);
    }

    fn mirror_parent(&mut self, kind: &str) {
        let key = format!("scala.deriving.Mirror.{}#type", kind);
        if let Some(&a) = self.shared_paths.get(&key) {
            self.shared_type_at(a);
            return;
        }
        let at = self.buf.addr();
        let n = self.names.simple(kind);
        self.buf.byte(TYPEREF);
        self.buf.nat(n as u64);
        self.buf.byte(THIS);
        let mn = self.names.object_class("Mirror");
        self.buf.byte(TYPEREF);
        self.buf.nat(mn as u64);
        self.package_path("scala.deriving");
        self.shared_paths.insert(key, at);
    }

    // ---- case classes ---------------------------------------------------------------------

    fn has_product_mirror(&self, c: ClassId) -> bool {
        let info = self.w.syms.class(c);
        // A case class with one parameter clause, not abstract, takes a product mirror.
        info.ctor.len() <= 1 && info.mods & mods::ABSTRACT == 0
    }

    /// Whether `owner` defines, or inherits concretely, a member `name` whose parameters match
    /// those of the one a case class would have synthesized (`params`, over `tparams`): the
    /// synthetic one is then left out (`Namer.invalidateIfClashingSynthetic`, which compares
    /// with `matchesLoosely`, so the result type does not count).
    fn clashing_member(&mut self, owner: ClassId, name: &str, tparams: &[TParamId], params: &[Vec<TypeId>]) -> bool {
        let n = self.w.interner.intern(name);
        let info = self.class_info(owner);
        let mut candidates: Vec<SymId> = Vec::new();
        let alternatives = |p: &Self, s: SymId| p.w.syms.alternatives(s).map(|a| a.to_vec()).unwrap_or_else(|| vec![s]);
        if let Some(&s) = info.members.get(&n) {
            candidates.extend(alternatives(self, s).into_iter().filter(|&a| self.w.syms.sym(a).def.is_some() && self.w.syms.sym(a).owner == Owner::Class(owner)));
        }
        for &(b, _) in info.base_types.iter().skip(1) {
            if let Some(&s) = self.w.syms.class(b).members.get(&n) {
                candidates.extend(alternatives(self, s).into_iter().filter(|&a| self.w.syms.sym(a).mods & mods::ABSTRACT == 0 && self.w.syms.sym(a).kind == SymKind::Def));
            }
        }
        candidates.into_iter().any(|a| {
            let sig = self.w.sig_of(a).clone();
            if sig.tparams.len() != tparams.len() || sig.clauses.len() != params.len() {
                return false;
            }
            let subst: Subst = sig.tparams.iter().copied().zip(tparams.iter().map(|&t| self.w.types.param(t))).collect();
            sig.clauses.iter().zip(params).all(|(cl, want)| {
                cl.params.len() == want.len()
                    && cl.params.iter().zip(want).all(|(p, &w)| {
                        let t = self.w.types.subst(p.ty, &subst);
                        t == w || self.w.is_same(t, w)
                    })
            })
        })
    }

    /// Whether the class defines the member itself: in its body, or as a field of its
    /// constructor (`override val toString: String`).
    fn user_defines(&self, c: ClassId, name: &str) -> bool {
        let n = self.w.interner.intern(name);
        let info = self.w.syms.class(c);
        let field = info.ctor_syms.iter().flatten().any(|&s| {
            let si = self.w.syms.sym(s);
            si.name == n && si.mods & mods::FIELD != 0
        });
        field
            || info.members.get(&n).map_or(false, |&s| {
                let si = self.w.syms.sym(s);
                si.def.is_some() && si.owner == Owner::Class(c)
            })
            || matches!(name, "hashCode" | "equals" | "toString") && self.inherits_concrete(c, n)
    }

    /// Whether a class inherits a concrete `hashCode`, `equals` or `toString` from a parent other
    /// than `Any`'s, which `SyntheticMembers`' `existingDef` takes for the class's own: a
    /// program's class's, or scala-library's function classes' `toString`.
    fn inherits_concrete(&self, c: ClassId, n: Name) -> bool {
        let info = self.w.syms.class(c);
        info.base_types.iter().map(|&(b, _)| b).filter(|&b| b != c).any(|b| {
            let bi = self.w.syms.class(b);
            let full = scala_name(self.w, b);
            if matches!(full.as_str(), "scala.Any" | "scala.AnyRef" | "java.lang.Object" | "scala.Product" | "scala.Equals" | "java.io.Serializable" | "scala.Serializable") {
                return false;
            }
            let library_function = full.strip_prefix("scala.Function").map_or(false, |k| k.parse::<u32>().is_ok()) && self.w.interner.get(n) == "toString";
            library_function
                || bi.members.get(&n).map_or(false, |&s| {
                    let si = self.w.syms.sym(s);
                    si.owner == Owner::Class(b) && si.def.is_some() && si.mods & mods::ABSTRACT == 0 && !self.w.files.as_slice().get(si.file.0 as usize).map_or(false, |f| f.is_std)
                })
        })
    }

    /// What `SyntheticMembers` adds to a case class after the constructor: the methods of
    /// `Any` and `Product` the class does not define itself.
    fn case_class_members(&mut self, c: ClassId, info: &ClassInfo) {
        let int = self.w.b.t_int;
        let boolean = self.w.b.t_boolean;
        let string = self.w.b.t_string;
        let any = ANY;
        // A value class's: of its field, and without `eq` (`SyntheticMembers`).
        if !self.user_defines(c, "hashCode") {
            let body = if info.value_class { Synth::ValueHashCode(c) } else { Synth::HashCode(c) };
            self.synth_def("hashCode", &[], true, int, &[OVERRIDE, SYNTHETIC], body);
        }
        if !self.user_defines(c, "equals") {
            let body = if info.value_class { Synth::ValueEquals(c) } else { Synth::Equals(c) };
            self.synth_def("equals", &[("x$0", any)], false, boolean, &[OVERRIDE, SYNTHETIC], body);
        }
        if !self.user_defines(c, "toString") {
            self.synth_def("toString", &[], true, string, &[OVERRIDE, SYNTHETIC], Synth::ToString(c));
        }
        if !self.user_defines(c, "canEqual") {
            self.synth_def("canEqual", &[("that", any)], false, boolean, &[OVERRIDE, SYNTHETIC], Synth::CanEqual(c));
        }
        if !self.user_defines(c, "productArity") {
            self.synth_def("productArity", &[], false, int, &[OVERRIDE, SYNTHETIC], Synth::ProductArity(c));
        }
        if !self.user_defines(c, "productPrefix") {
            self.synth_def("productPrefix", &[], false, string, &[OVERRIDE, SYNTHETIC], Synth::ProductPrefix(c));
        }
        if !self.user_defines(c, "productElement") {
            self.synth_def("productElement", &[("n", int)], false, any, &[OVERRIDE, SYNTHETIC], Synth::ProductElement(c));
        }
        if !self.user_defines(c, "productElementName") {
            self.synth_def("productElementName", &[("n", int)], false, string, &[OVERRIDE, SYNTHETIC], Synth::ProductElementName(c));
        }
    }

    /// What `Desugar` adds after the body: `copy` with its default getters, an enum case's
    /// `ordinal`, the `_N` accessors.
    fn case_class_tail(&mut self, c: ClassId, info: &ClassInfo) {
        let repeated = info.ctor.first().map_or(false, |cl| cl.params.last().map_or(false, |p| p.repeated));
        let private_ctor = info.mods & mods::PRIVATE_CTOR != 0;
        if !repeated && info.mods & mods::ABSTRACT == 0 {
            if !self.user_defines(c, "copy") {
                self.copy_method(c, info, private_ctor);
            }
            // The default getters stay when the class defines its own `copy`.
            let tparams: Vec<TParamId> = info.tparams.to_vec();
            let first: Vec<ParamSig> = info.ctor.first().map(|c| c.params.clone()).unwrap_or_default();
            let user_copy = self.user_defines(c, "copy");
            for (i, p) in first.iter().enumerate() {
                let tmark = self.tparams.len();
                self.default_getter_with(&"copy".to_string(), i as u32, &tparams, p.ty, private_ctor, !user_copy, Synth::Field(c, i as u32));
                self.tparams.truncate(tmark);
            }
        }
        if info.kind == ClassKind::EnumCase {
            let int = self.w.b.t_int;
            self.synth_def("ordinal", &[], false, int, &[SYNTHETIC], Synth::Int(info.ordinal as i32));
        }
        let first: Vec<ParamSig> = info.ctor.first().map(|c| c.params.clone()).unwrap_or_default();
        for (i, p) in first.iter().enumerate() {
            let name = format!("_{}", i + 1);
            if self.user_defines(c, &name) {
                continue;
            }
            let addr = self.buf.addr();
            self.buf.byte(DEFDEF);
            let len = self.buf.begin_length();
            let n = self.names.simple(&name);
            self.buf.nat(n as u64);
            if p.repeated {
                self.param_type(p);
                let mark = self.body_begin();
                self.synth_body_of(Synth::Field(c, i as u32));
                match self.body_end(mark) {
                    Ok(()) => self.count_body(Producer::Synthesized("case class _N")),
                    Err(reason) => {
                        // `ELIDED` of the accessor's type, `Seq[T] @Repeated`.
                        self.count_withheld(&reason);
                        self.elided_bodies += 1;
                        self.mark_tree();
                        self.elided_byte();
                        self.repeated_type(p.ty);
                    }
                }
            } else {
                self.tpt(p.ty);
                self.synth_rhs("case class _N", p.ty, Synth::Field(c, i as u32), &[]);
            }
            self.write_flags(&[SYNTHETIC]);
            self.synthetic_position(addr);
            self.buf.end_length(len);
        }
    }

    fn default_getter_with(&mut self, method: &str, index: u32, tparams: &[TParamId], ty: TypeId, private: bool, annotated: bool, body: Synth) {
        let addr = self.buf.addr();
        self.buf.byte(DEFDEF);
        let len = self.buf.begin_length();
        let n = self.names.default_getter(method, index);
        self.buf.nat(n as u64);
        let tmark = self.tparams.len();
        self.def_params(tparams, &[], None);
        // The getter's type and value are the field's, of the class's own type parameters, not
        // the getter's: `@uncheckedVariance` where it names none of them.
        self.tparams.truncate(tmark);
        let names_tparams = self.w.mentions_tparam_of(ty, Some(tparams));
        if annotated && !names_tparams {
            self.mark_tree();
            self.unchecked_variance(ty);
        } else {
            self.tpt(ty);
        }
        self.synth_rhs("copy default getter", ty, body, &[]);
        let mut flags = Vec::new();
        if private {
            flags.push(PRIVATE);
            flags.push(LOCAL);
        }
        flags.push(SYNTHETIC);
        self.write_flags(&flags);
        self.synthetic_position(addr);
        self.buf.end_length(len);
    }

    fn copy_method(&mut self, c: ClassId, info: &ClassInfo, private: bool) {
        let addr = self.buf.addr();
        self.buf.byte(DEFDEF);
        let len = self.buf.begin_length();
        let n = self.names.simple("copy");
        self.buf.nat(n as u64);
        let tmark = self.tparams.len();
        let pmark = self.params.len();
        let tparams: Vec<TParamId> = info.tparams.to_vec();
        let mut clauses = info.ctor.clone();
        if let Some(first) = clauses.first_mut() {
            for p in &mut first.params {
                p.has_default = true;
            }
        }
        self.def_params(&tparams, &clauses, None);
        let self_ty = self.class_applied(c, &tparams);
        self.tpt(self_ty);
        let addrs: Vec<usize> = self.param_addrs(pmark, &clauses).concat();
        self.synth_rhs("copy", self_ty, Synth::New(c), &addrs);
        self.tparams.truncate(tmark);
        self.params.truncate(pmark);
        let mut flags = Vec::new();
        if private {
            flags.push(PRIVATE);
            flags.push(LOCAL);
        }
        flags.push(SYNTHETIC);
        self.write_flags(&flags);
        self.synthetic_position(addr);
        self.buf.end_length(len);
    }

    fn class_applied(&mut self, c: ClassId, tparams: &[TParamId]) -> TypeId {
        let args: Vec<TypeId> = tparams.iter().map(|&p| self.w.types.param(p)).collect();
        self.w.types.class(c, &args)
    }

    /// `apply`, `unapply` (`unapplySeq` for a repeated last parameter) and, in a companion the
    /// source does not write, `toString`, which `Desugar` gives a case class's companion.
    fn case_companion_head(&mut self, c: ClassId, info: &ClassInfo, synthetic_companion: bool) {
        let tparams: Vec<TParamId> = info.tparams.to_vec();
        let private_ctor = info.mods & mods::PRIVATE_CTOR != 0;
        let abstract_class = info.mods & mods::ABSTRACT != 0;
        let companion = info.companion;
        let ctor_types: Vec<Vec<TypeId>> = info.ctor.iter().map(|cl| cl.params.iter().map(|p| p.ty).collect()).collect();
        let clashes = |p: &mut Self, name: &str, params: &[Vec<TypeId>]| companion.map_or(false, |co| p.clashing_member(co, name, &tparams, params));
        if !abstract_class && !clashes(self, "apply", &ctor_types) {
            let addr = self.buf.addr();
            self.buf.byte(DEFDEF);
            let len = self.buf.begin_length();
            let n = self.names.simple("apply");
            self.buf.nat(n as u64);
            let tmark = self.tparams.len();
            let pmark = self.params.len();
            self.def_params(&tparams, &info.ctor, None);
            let self_ty = self.class_applied(c, &tparams);
            self.tpt(self_ty);
            let addrs: Vec<usize> = self.param_addrs(pmark, &info.ctor).concat();
            self.synth_rhs("companion apply", self_ty, Synth::New(c), &addrs);
            self.tparams.truncate(tmark);
            self.params.truncate(pmark);
            let mut flags = Vec::new();
            if private_ctor {
                flags.push(PRIVATE);
                flags.push(LOCAL);
            }
            flags.push(SYNTHETIC);
            self.write_flags(&flags);
            self.synthetic_position(addr);
            self.buf.end_length(len);
        }
        let repeated = info.ctor.first().map_or(false, |cl| cl.params.last().map_or(false, |p| p.repeated));
        let unapply = if repeated { "unapplySeq" } else { "unapply" };
        let scrutinee = vec![vec![self.class_applied(c, &tparams)]];
        if !clashes(self, unapply, &scrutinee) {
            let addr = self.buf.addr();
            self.buf.byte(DEFDEF);
            let len = self.buf.begin_length();
            let n = self.names.simple(unapply);
            self.buf.nat(n as u64);
            let tmark = self.tparams.len();
            let ids = self.bind_tparams(&tparams);
            self.write_bound_tparams(&tparams, &ids, false);
            let self_ty = self.class_applied(c, &tparams);
            let pa = self.buf.addr();
            self.buf.byte(PARAM);
            let pl = self.buf.begin_length();
            let pn = self.names.simple("x$1");
            self.buf.nat(pn as u64);
            self.tpt(self_ty);
            self.buf.byte(SYNTHETIC);
            self.synthetic_position(pa);
            self.buf.end_length(pl);
            self.tpt(self_ty);
            self.synth_rhs("companion unapply", self_ty, Synth::Param, &[pa]);
            self.tparams.truncate(tmark);
            self.write_flags(&[SYNTHETIC]);
            self.synthetic_position(addr);
            self.buf.end_length(len);
        }
        if synthetic_companion {
            let string = self.w.b.t_string;
            self.synth_def("toString", &[], false, string, &[OVERRIDE, SYNTHETIC], Synth::Name(c));
        }
    }

    /// `type MirroredMonoType = C` and `def fromProduct(x$0: Product): MirroredMonoType`.
    fn mirror_members(&mut self, c: ClassId, tparams: &[TParamId]) {
        // dotty's `SyntheticMembers.addMirrorSupport`: a concrete `MirroredMonoType` the companion
        // has is the one the mirror names (`monoType`'s `existing`), and a `fromProduct` it
        // defines stands for the synthetic one (`addMethod`'s `existingDef`).
        let companion = self.w.syms.class(c).companion;
        let existing_mono = companion.and_then(|o| self.concrete_alias_member(o, "MirroredMonoType"));
        let product = self.scala_trait("Product").unwrap_or(ANY);
        let existing_from_product = companion.is_some_and(|o| self.clashing_member(o, "fromProduct", &[], &[vec![product]]));
        let mono = match existing_mono {
            Some(a) => Err(a),
            None => {
                let addr = self.buf.addr();
                self.buf.byte(TYPEDEF);
                let len = self.buf.begin_length();
                let n = self.names.simple("MirroredMonoType");
                self.buf.nat(n as u64);
                self.mark_tree();
                self.buf.byte(TYPEBOUNDS);
                let b = self.buf.begin_length();
                self.mono_type(c, tparams.len());
                self.buf.end_length(b);
                self.write_flags(&[SYNTHETIC]);
                self.synthetic_position(addr);
                self.buf.end_length(len);
                Ok(addr)
            }
        };
        if existing_from_product {
            return;
        }
        let fa = self.buf.addr();
        self.buf.byte(DEFDEF);
        let fl = self.buf.begin_length();
        let fname = self.names.simple("fromProduct");
        self.buf.nat(fname as u64);
        let pa = self.buf.addr();
        self.buf.byte(PARAM);
        let pl = self.buf.begin_length();
        let pn = self.names.simple("x$0");
        self.buf.nat(pn as u64);
        let product = self.scala_trait("Product").unwrap_or(ANY);
        self.tpt(product);
        self.synthetic_position(pa);
        self.buf.end_length(pl);
        self.mark_tree();
        self.mono_ref(c, mono);
        let mark = self.body_begin();
        self.from_product(c, pa);
        match self.body_end(mark) {
            Ok(()) => self.count_body(Producer::Synthesized("fromProduct")),
            Err(reason) => {
                self.count_withheld(&reason);
                self.elided_bodies += 1;
                self.mark_tree();
                self.elided_byte();
                self.mono_ref(c, mono);
            }
        }
        self.write_flags(&[SYNTHETIC]);
        self.synthetic_position(fa);
        self.buf.end_length(fl);
    }

    /// The mirror's `MirroredMonoType` of the case class `c`: the synthetic one written at its
    /// address, else the companion's own.
    fn mono_ref(&mut self, c: ClassId, mono: Result<usize, AliasId>) {
        match mono {
            Ok(addr) => {
                self.buf.byte(TYPEREFSYMBOL);
                self.buf.reference(addr);
                self.buf.byte(THIS);
                self.companion_this_typeref(c);
            }
            Err(a) => self.alias_ref(a),
        }
    }

    /// The type member `name` of the class `o` as a lookup finds it, the class's own first, then
    /// its bases' in linearization order, where it is an alias: an abstract one is none (dotty's
    /// `Deferred`).
    fn concrete_alias_member(&self, o: ClassId, name: &str) -> Option<AliasId> {
        let n = self.w.interner.lookup(name)?;
        let bases: Vec<ClassId> = std::iter::once(o).chain(self.w.syms.class(o).base_types.iter().skip(1).map(|&(b, _)| b)).collect();
        let a = bases.iter().find_map(|&b| self.w.syms.class(b).type_aliases.get(&n).copied())?;
        (!self.w.syms.aliases[a.idx()].is_abstract()).then_some(a)
    }

    /// A mirror's `MirroredMonoType`: the class, its type parameters as `? <: AnyKind`.
    fn mono_type(&mut self, c: ClassId, n: usize) {
        if n == 0 {
            return self.class_typeref(c);
        }
        self.buf.byte(APPLIEDTYPE);
        let len = self.buf.begin_length();
        self.class_typeref(c);
        for _ in 0..n {
            self.buf.byte(TYPEBOUNDS);
            let b = self.buf.begin_length();
            self.ty(NOTHING);
            self.external_typeref("scala", "AnyKind");
            self.buf.end_length(b);
        }
        self.buf.end_length(len);
    }

    /// The type of the class whose body is being written, as the prefix of its `this`.
    /// The module class of the companion being written of the class `c`: the object's own, or
    /// the one written for it where the source has none.
    fn companion_this_typeref(&mut self, c: ClassId) {
        // An object nested in a class is a class of the typer's: its companion link tells it.
        let explicit = self.enclosing.last().map_or(false, |&o| o != c && (self.w.syms.class(o).companion == Some(c) || self.w.syms.class(c).companion == Some(o)));
        if explicit {
            return self.enclosing_this_typeref();
        }
        self.buf.byte(TYPEREFSYMBOL);
        self.def_ref(Key::SynthModuleClass(c));
        let owner = self.w.syms.class(c).owner;
        self.owner_prefix(owner);
    }

    fn enclosing_this_typeref(&mut self) {
        match self.enclosing.last().copied() {
            Some(c) => self.class_typeref(c),
            None => self.ty(ANY),
        }
    }

    // ---- givens ---------------------------------------------------------------------------

    /// A given: an alias given as a lazy val (parameterless) or a def; a given with a body as
    /// an object (parameterless) or a class beside a def that makes it.
    fn given_def(&mut self, s: SymId) {
        let info = self.sym_info(s);
        let sig = self.w.sig_of(s).clone();
        let parameterless = sig.tparams.is_empty() && sig.clauses.is_empty();
        match info.impl_class {
            // A deferred given has no body and `HASDEFAULT`, and is not final (dotty's `Namer`).
            None if info.mods & mods::DEFERRED != 0 => self.value_or_method(s, &[HASDEFAULT]),
            None => self.value_or_method(s, &[FINAL]),
            Some(k) if parameterless => {
                // `given x: T with {...}` is `given object x`.
                self.given_object(s, k);
            }
            Some(k) => {
                self.given_class(s, k);
                self.value_or_method_as(s, &[FINAL], Some(k));
            }
        }
    }

    fn given_object(&mut self, s: SymId, k: ClassId) {
        let name = self.name(self.w.syms.sym(s).name);
        let info = self.class_info(k);
        let vaddr = self.buf.addr();
        self.define(Key::Sym(s), vaddr);
        self.define(Key::Module(k), vaddr);
        self.buf.byte(VALDEF);
        let vlen = self.buf.begin_length();
        let vn = self.names.simple(&name);
        self.buf.nat(vn as u64);
        let tpt = self.buf.addr();
        let cn = self.names.object_class(&name);
        self.mark_tree();
        self.buf.byte(IDENTTPT);
        self.buf.nat(cn as u64);
        let tref = self.buf.addr();
        self.module_class_typeref(k);
        self.new_module(&name, info.owner, tpt, tref);
        self.write_flags(&[OBJECT, GIVEN]);
        self.given_source_annotations(s, annot::Dest::Any);
        let span = self.def_span(info.file, info.def, info.span);
        self.position_def(vaddr, span, info.span.start, info.file, info.def);
        self.buf.end_length(vlen);
        let caddr = self.buf.addr();
        self.define(Key::Class(k), caddr);
        self.buf.byte(TYPEDEF);
        let len = self.buf.begin_length();
        self.buf.nat(cn as u64);
        self.add_local(Key::Module(k));
        self.given_template(k, true);
        self.write_flags(&[OBJECT]);
        self.given_source_annotations(s, annot::Dest::Any);
        self.position(caddr, span, info.span.start);
        self.buf.end_length(len);
    }

    fn given_class(&mut self, s: SymId, k: ClassId) {
        let info = self.class_info(k);
        let addr = self.buf.addr();
        self.define(Key::Class(k), addr);
        self.buf.byte(TYPEDEF);
        let len = self.buf.begin_length();
        let n = self.simple_name(info.name);
        self.buf.nat(n as u64);
        self.given_template(k, false);
        self.write_flags(&[SYNTHETIC, GIVEN]);
        self.given_source_annotations(s, annot::Dest::GivenClass);
        let span = self.def_span(info.file, info.def, info.span);
        self.position_def(addr, span, info.span.start, info.file, info.def);
        self.buf.end_length(len);
    }

    /// A given class's template: its parameters the given's, `protected` and `given` where the
    /// clause is a using clause.
    fn given_template(&mut self, k: ClassId, object: bool) {
        let info = self.class_info(k);
        self.note_class_param_annotations(&info);
        self.buf.byte(TEMPLATE);
        let len = self.buf.begin_length();
        let tmark = self.tparams.len();
        let own: Vec<TParamId> = info.tparams.to_vec();
        for &tp in &own {
            let at = self.buf.addr();
            self.tparams.push((tp, TpRef::InClass(at, k)));
            self.type_param_def(tp, &[PRIVATE, LOCAL], None);
        }
        let ctor = info.ctor.clone();
        let syms = info.ctor_syms.clone();
        for (ci, clause) in ctor.iter().enumerate() {
            for (pi, p) in clause.params.iter().enumerate() {
                let addr = self.buf.addr();
                if let Some(&s) = syms.get(ci).and_then(|c| c.get(pi)) {
                    self.define(Key::Sym(s), addr);
                }
                self.buf.byte(PARAM);
                let pl = self.buf.begin_length();
                let n = self.simple_name(p.name);
                self.buf.nat(n as u64);
                self.param_type(p);
                let mut flags = vec![PROTECTED];
                if clause.is_using {
                    flags.push(GIVEN);
                } else {
                    flags = vec![PRIVATE, LOCAL];
                }
                self.write_flags(&flags);
                self.param_source_annotations(p.sym, annot::Dest::Accessor);
                self.synthetic_position(addr);
                self.buf.end_length(pl);
            }
        }
        self.enclosing.push(k);
        let object_t = self.object_type();
        // A class parent is its constructor call, else `Object()` before the traits.
        let parents: Vec<TypeId> = info.parents.iter().copied().filter(|&p| p != object_t).collect();
        let first_class = parents.first().copied().filter(|&p| !self.is_trait_type(p));
        match first_class {
            Some(p) => self.parent_call(k, p),
            None => self.parent_call(k, object_t),
        }
        for (i, &p) in parents.iter().enumerate() {
            if i == 0 && first_class.is_some() {
                continue;
            }
            self.tpt(p);
        }
        let mut self_type = None;
        if object {
            self.buf.byte(SELFDEF);
            let us = self.names.simple("_");
            self.buf.nat(us as u64);
            self.buf.byte(SINGLETONTPT);
            self.mark_tree();
            self_type = self.is_static(info.owner).then(|| self.buf.addr());
            self.module_self(k);
        }
        self.constructor(k, &info);
        self.count_template_roots(k);
        if object {
            self.write_replace(self_type);
        }
        let body = self.body_order(k, &info);
        let ctor_syms: Vec<SymId> = info.ctor_syms.iter().flatten().copied().collect();
        let mut stmts = self.template_stmts(k);
        for m in body {
            self.statements_before(k, &mut stmts, self.item_start(&m));
            match m {
                BodyItem::Sym(s) if !ctor_syms.contains(&s) => self.member(k, s),
                BodyItem::Class(c) => self.nested_class(c),
                BodyItem::Alias(a) => self.alias_def(a),
                _ => {}
            }
        }
        self.statements_before(k, &mut stmts, None);
        self.enclosing.pop();
        self.tparams.truncate(tmark);
        self.buf.end_length(len);
    }

    // ---- the top-level definitions of a file ---------------------------------------------

    fn package_object(&mut self, pkg: PkgId, name: &str, defs: &[crate::ast::DefId]) {
        let f = self.file.0 as usize;
        let vaddr = self.buf.addr();
        self.buf.byte(VALDEF);
        let vlen = self.buf.begin_length();
        let vn = self.names.simple(name);
        self.buf.nat(vn as u64);
        let tpt = self.buf.addr();
        let cn = self.names.object_class(name);
        self.mark_tree();
        self.buf.byte(IDENTTPT);
        self.buf.nat(cn as u64);
        let class_at = self.buf.addr();
        self.buf.byte(TYPEREFSYMBOL);
        let fwd = self.buf.forward_reference();
        self.package_ref(pkg);
        self.new_module(name, Owner::Package(pkg), tpt, class_at);
        let synthetic = true;
        let mut vflags = vec![OBJECT];
        if synthetic {
            vflags.push(SYNTHETIC);
        }
        self.write_flags(&vflags);
        self.synthetic_position(vaddr);
        self.buf.end_length(vlen);
        let caddr = self.buf.addr();
        self.buf.fill(fwd, caddr);
        self.buf.byte(TYPEDEF);
        let len = self.buf.begin_length();
        self.buf.nat(cn as u64);
        self.buf.byte(TEMPLATE);
        let tl = self.buf.begin_length();
        let t = self.w.b.t_any_ref;
        self.parent_call(ClassId(u32::MAX), t);
        self.buf.byte(SELFDEF);
        let us = self.names.simple("_");
        self.buf.nat(us as u64);
        self.buf.byte(SINGLETONTPT);
        self.mark_tree();
        let self_type = self.buf.addr();
        self.buf.byte(TERMREFSYMBOL);
        self.buf.reference(vaddr);
        self.package_ref(pkg);
        let ia = self.buf.addr();
        self.buf.byte(DEFDEF);
        let il = self.buf.begin_length();
        let init = self.names.simple("<init>");
        self.buf.nat(init as u64);
        self.buf.byte(EMPTYCLAUSE);
        let unit = self.w.b.t_unit;
        self.tpt(unit);
        self.synthetic_position(ia);
        self.buf.end_length(il);
        self.write_replace(Some(self_type));
        self.package_members_this = Some((caddr, pkg));
        self.in_package_object = true;
        for d in defs {
            if let Some(&s) = self.w.def_syms.get(f, d) {
                let kind = self.w.syms.sym(s).kind;
                match kind {
                    SymKind::Given => self.given_def(s),
                    SymKind::Val | SymKind::Var | SymKind::Def => {
                        self.value_or_method(s, &[]);
                        if kind == SymKind::Var {
                            self.setter(s);
                        }
                    }
                    _ => {}
                }
                continue;
            }
            if let Some(&c) = self.w.def_classes.get(f, d) {
                self.nested_class(c);
                continue;
            }
            if let Some(&a) = self.w.def_aliases.get(f, d) {
                self.alias_def(a);
            }
        }
        self.package_export_forwarders(pkg);
        self.in_package_object = false;
        self.buf.end_length(tl);
        let mut flags = vec![OBJECT];
        if synthetic {
            flags.push(SYNTHETIC);
        }
        let holds_opaque = defs.iter().any(|d| self.w.def_classes.get(f, d).map_or(false, |&c| self.w.syms.class(c).kind == ClassKind::Opaque));
        if holds_opaque {
            flags.push(OPAQUE);
        }
        self.write_flags(&flags);
        self.source_file_annotation();
        self.synthetic_position(caddr);
        self.buf.end_length(len);
    }

    // ---- references -----------------------------------------------------------------------

    /// The prefix of a definition of `owner`: a package, an object's module val, or a class's
    /// `this` (a projection's class outside it).
    fn owner_prefix(&mut self, owner: Owner) {
        match owner {
            Owner::Package(p) => self.package_ref(p),
            Owner::Class(o) => {
                let oi = self.w.syms.class(o);
                if oi.kind == ClassKind::Object && oi.module_sym.is_some() && !self.enclosing.contains(&o) && self.is_java_class(o) {
                    // A static member of a Java class is its companion module class's, as
                    // scalac reads the class file.
                    self.placed = true;
                    self.buf.byte(THIS);
                    self.module_class_typeref(o);
                } else if oi.kind == ClassKind::Object && oi.module_sym.is_some() && !self.enclosing.contains(&o) {
                    if matches!(oi.owner, Owner::Package(_)) && self.name(oi.name).ends_with("$package") && !self.is_local(Key::Module(o)) {
                        return self.module_termref(o);
                    }
                    if let Some(&a) = self.shared_prefixes.get(&o) {
                        return self.shared_type_at(a);
                    }
                    let outer = std::mem::replace(&mut self.placed, false);
                    let at = self.buf.addr();
                    self.module_termref(o);
                    if !self.placed {
                        self.shared_prefixes.insert(o, at);
                    }
                    self.placed |= outer;
                } else if self.enclosing.contains(&o) {
                    self.buf.byte(THIS);
                    self.class_typeref(o);
                    self.placed = true;
                } else {
                    // `Outer#Inner` for a class nested in a class, seen from outside it.
                    self.approximated("class nested in a class, named outside it as a projection");
                    self.class_typeref(o);
                }
            }
            Owner::Local => {
                self.placed = true;
                self.buf.byte(THIS);
                self.package_ref(ROOT_PKG);
            }
        }
    }

    /// The stem of the `$package` object a file's top-level definitions are members of: the
    /// file's, scala-library's for the std's `iarray.scala` (`IArray$package`).
    fn package_stem(&self, file: FileId) -> String {
        let stem = super::file_stem(&self.w.files.as_slice()[file.0 as usize].path);
        let std = self.w.files.as_slice()[file.0 as usize].is_std && !self.w.in_jar(file);
        match stem.as_str() {
            "iarray" if std => "IArray".to_string(),
            _ => stem,
        }
    }

    /// A class nested in a class, through `prefix`: `TYPEREF name prefix`, or `TYPEREFsymbol
    /// addr prefix` for a class of this file.
    fn nested_typeref(&mut self, c: ClassId, prefix: TypeId) {
        self.placed = true;
        if self.is_local(Key::Class(c)) {
            self.buf.byte(TYPEREFSYMBOL);
            self.def_ref(Key::Class(c));
        } else {
            let n = self.simple_name(self.w.syms.class(c).name);
            self.buf.byte(TYPEREF);
            self.buf.nat(n as u64);
        }
        self.ty(prefix);
    }

    /// A class unapplied: `TYPEREF name prefix`, or `TYPEREFsymbol addr prefix` for a class of
    /// this file.
    fn class_typeref(&mut self, c: ClassId) {
        let info = self.class_head(c);
        if info.kind != ClassKind::Object && self.is_reflect_class(c) {
            return self.reflect_typeref(c);
        }
        // An object's type is its module class's.
        if info.kind == ClassKind::Object && info.owner != Owner::Local {
            return self.module_class_typeref(c);
        }
        if info.owner == Owner::Local {
            // A class of a body: by its definition's address, with no prefix.
            self.placed = true;
            self.buf.byte(TYPEREFDIRECT);
            self.def_ref(Key::Class(c));
            return;
        }
        // A top-level opaque type is a member of its file's `$package` object.
        if let (ClassKind::Opaque, Owner::Package(p)) = (info.kind, info.owner) {
            // Another file's is a static path, shared as any (`TYPEREF T (TERMREF F$package)`).
            let local = self.is_local(Key::Class(c));
            if local {
                self.placed = true;
                self.buf.byte(TYPEREFSYMBOL);
                self.def_ref(Key::Class(c));
            } else {
                let n = self.simple_name(info.name);
                self.buf.byte(TYPEREF);
                self.buf.nat(n as u64);
            }
            match self.package_members_this.filter(|&(_, pk)| pk == p && local) {
                Some((class_at, pk)) => {
                    self.buf.byte(THIS);
                    self.buf.byte(TYPEREFSYMBOL);
                    self.buf.reference(class_at);
                    self.package_ref(pk);
                }
                None => {
                    let stem = self.package_stem(info.file);
                    self.file_package_term(p, stem);
                }
            }
            return;
        }
        if self.is_local(Key::Class(c)) {
            self.placed = true;
            self.buf.byte(TYPEREFSYMBOL);
            self.def_ref(Key::Class(c));
            self.owner_prefix(info.owner);
            return;
        }
        let (pkg, name) = self.external_class_name(c);
        match pkg {
            Some(pkg) => self.external_typeref(&pkg, &name),
            None => {
                // A class nested in another class or object outside the file.
                let n = self.names.simple(&name);
                self.buf.byte(TYPEREF);
                self.buf.nat(n as u64);
                match self.java_static_nested_companion(c) {
                    Some(o) => self.module_termref(o),
                    None => self.owner_prefix(info.owner),
                }
            }
        }
    }

    /// The companion of the Java class a static nested class is in, which scalac enters it in
    /// reading the class file (`fix.Outer.Nested` the companion's path, `fix.Outer$.Nested` its
    /// signature name), where the loader entered it in the class.
    fn java_static_nested_companion(&self, c: ClassId) -> Option<ClassId> {
        let Owner::Class(o) = self.w.syms.class(c).owner else { return None };
        let java = &self.w.loaded.as_ref()?.java.classes;
        let nested = java.get(&c)?;
        if nested.inner || nested.companion_of.is_some() || self.w.syms.class(o).kind == ClassKind::Object {
            return None;
        }
        self.w.syms.class(o).companion.filter(|&k| self.w.syms.class(k).kind == ClassKind::Object)
    }

    /// The module class of an object: `TYPEREF X$ prefix`.
    fn module_class_typeref(&mut self, o: ClassId) {
        let info = self.class_head(o);
        if self.is_local(Key::Class(o)) {
            self.placed = true;
            self.buf.byte(TYPEREFSYMBOL);
            self.def_ref(Key::Class(o));
            self.owner_prefix(info.owner);
            return;
        }
        let (pkg, name) = self.external_class_name(o);
        let n = self.names.object_class(&name);
        self.buf.byte(TYPEREF);
        self.buf.nat(n as u64);
        if let Some(p) = self.opaque_companion_package(o) {
            let stem = self.package_stem(info.file);
            return self.file_package_term(p, stem);
        }
        match pkg {
            Some(p) => self.package_path(&p),
            None => self.owner_prefix(info.owner),
        }
    }

    /// The package of the program's object `o` where it is the companion of a top-level opaque
    /// type, which is a member of its file's `$package` object, as scalac places it beside the type.
    fn opaque_companion_package(&self, o: ClassId) -> Option<PkgId> {
        let info = self.w.syms.class(o);
        match (info.owner, info.companion) {
            (Owner::Package(p), Some(k)) if self.w.syms.class(k).kind == ClassKind::Opaque && self.w.syms.class(k).owner == Owner::Package(p) => Some(p),
            _ => None,
        }
    }

    /// An object's self type. An object of a class is its own `this` type's member by name, as
    /// scalac writes it: the module val by its address makes scalac's tree checker loop on it.
    fn module_self(&mut self, o: ClassId) {
        let owner = self.w.syms.class(o).owner;
        if matches!(owner, Owner::Class(c) if self.w.syms.class(c).kind != ClassKind::Object) {
            let name = self.name(self.w.syms.class(o).name);
            let n = self.names.simple(&name);
            self.buf.byte(SELECT);
            self.buf.nat(n as u64);
            self.mark_tree();
            self.owner_prefix(owner);
        } else {
            self.module_termref(o);
        }
    }

    /// An object's module val as a path: `TERMREF X prefix`.
    fn module_termref(&mut self, o: ClassId) {
        let info = self.class_head(o);
        if self.is_local(Key::Module(o)) {
            self.placed = true;
            self.buf.byte(TERMREFSYMBOL);
            self.def_ref(Key::Module(o));
            self.owner_prefix(info.owner);
            return;
        }
        let name = self.name(info.name);
        if let (ClassKind::Object, Owner::Package(p), Some(stem)) = (info.kind, info.owner, name.strip_suffix("$package")) {
            return self.file_package_term(p, stem.to_string());
        }
        let (pkg, _) = self.external_class_name(o);
        let n = self.names.simple(&name);
        self.buf.byte(TERMREF);
        self.buf.nat(n as u64);
        if let Some(p) = self.opaque_companion_package(o) {
            let stem = self.package_stem(info.file);
            return self.file_package_term(p, stem);
        }
        match pkg {
            Some(p) => self.package_path(&p),
            None => self.owner_prefix(info.owner),
        }
    }

    /// `TERMREF F$package p`, a file's package object as a prefix, shared once written.
    fn file_package_term(&mut self, p: PkgId, stem: String) {
        if let Some(&a) = self.package_object_terms.get(&(p, stem.clone())) {
            return self.shared_type_at(a);
        }
        let at = self.buf.addr();
        let n = self.names.simple(&format!("{}$package", stem));
        self.buf.byte(TERMREF);
        self.buf.nat(n as u64);
        self.package_ref(p);
        self.package_object_terms.insert((p, stem), at);
    }

    fn def_termref(&mut self, k: Key, owner: Owner) {
        self.placed = true;
        self.buf.byte(TERMREFSYMBOL);
        self.def_ref(k);
        self.owner_prefix(owner);
    }

    /// A member val as a path: `TERMREF x prefix`.
    fn member_termref(&mut self, s: SymId) {
        let (owner, _) = self.sym_head(s);
        if self.is_local(Key::Sym(s)) {
            self.placed = true;
            self.buf.byte(TERMREFSYMBOL);
            self.def_ref(Key::Sym(s));
            self.term_owner_prefix(s, owner);
            return;
        }
        let n = self.sym_name(s);
        self.buf.byte(TERMREF);
        self.buf.nat(n as u64);
        self.term_owner_prefix(s, owner);
    }

    /// The prefix of a term: a top-level one's is its file's `$package` object.
    fn term_owner_prefix(&mut self, s: SymId, owner: Owner) {
        match owner {
            Owner::Package(p) => {
                if let Some((class_at, pk)) = self.package_members_this.filter(|&(_, pk)| pk == p && self.is_local(Key::Sym(s))) {
                    self.placed = true;
                    self.buf.byte(THIS);
                    self.buf.byte(TYPEREFSYMBOL);
                    self.buf.reference(class_at);
                    self.package_ref(pk);
                    return;
                }
                let file = self.w.syms.sym(s).file;
                let stem = super::file_stem(&self.w.files.as_slice()[file.0 as usize].path);
                self.file_package_term(p, stem);
            }
            other => self.owner_prefix(other),
        }
    }

    /// Where a class lives as scala-library, the JDK or another file names it: its package and
    /// name when a package holds it, `None` with the simple name for a nested class.
    fn external_class_name(&self, c: ClassId) -> (Option<String>, String) {
        let full = scala_name(self.w, c);
        let info = self.w.syms.class(c);
        match info.owner {
            Owner::Package(_) => match full.rsplit_once('.') {
                Some((pkg, name)) => (Some(pkg.to_string()), name.to_string()),
                None => (Some(String::new()), full),
            },
            _ => (None, self.name(info.name)),
        }
    }

    // ---- types ----------------------------------------------------------------------------

    fn ty(&mut self, t: TypeId) {
        let t = self.w.zonk(t);
        if let Some(&a) = self.shared.get(&t) {
            self.shared_type_at(a);
            return;
        }
        let outer = std::mem::replace(&mut self.placed, false);
        let at = self.buf.addr();
        self.type_now(t);
        // A class of no arguments is shared where its prefix is a static path, the same
        // wherever the pickle names it.
        let shareable = match self.w.types.get(t) {
            Type::Class(c, l) if l == EMPTY_LIST => self.static_owner(self.w.syms.class(c).owner),
            _ => true,
        };
        if !self.placed && shareable {
            self.shared.insert(t, at);
        }
        self.placed |= outer;
    }

    fn type_now(&mut self, t: TypeId) {
        match self.w.types.get(t) {
            Type::Any => self.external_typeref("scala", "Any"),
            Type::Nothing => self.external_typeref("scala", "Nothing"),
            Type::Error => {
                self.fail("a type that did not check".to_string());
                self.external_typeref("scala", "Nothing");
            }
            Type::Class(c, args) => {
                let args: Vec<TypeId> = self.w.types.items(args).to_vec();
                if c == self.w.b.null {
                    return self.external_typeref("scala", "Null");
                }
                if c == self.w.b.any_ref {
                    return self.external_typeref("scala", "AnyRef");
                }
                if c == self.w.b.any_val {
                    return self.external_typeref("scala", "AnyVal");
                }
                if Some(c) == self.w.b.by_name {
                    self.buf.byte(BYNAMETYPE);
                    return self.ty(args.first().copied().unwrap_or(ANY));
                }
                if Some(c) == self.w.b.repeated {
                    // `<repeated>[T]` as scalac's `T*` type: `Seq[T] @Repeated`.
                    self.buf.byte(ANNOTATEDTYPE);
                    let len = self.buf.begin_length();
                    self.buf.byte(APPLIEDTYPE);
                    let app = self.buf.begin_length();
                    self.external_typeref("scala.collection.immutable", "Seq");
                    self.ty(args.first().copied().unwrap_or(ANY));
                    self.buf.end_length(app);
                    self.annotation_tree("scala.annotation.internal", "Repeated");
                    self.buf.end_length(len);
                    return;
                }
                if args.is_empty() {
                    return self.class_typeref(c);
                }
                // A tuple of more than 22 elements, which scalac has no class of: its `*:` chain.
                if args.len() > 22 && self.w.is_tuple_class(c) {
                    return self.tuple_chain(&args);
                }
                // A `*:` chain's empty tail as a signature spells it, scalac's alias `EmptyTuple`,
                // whose chain scalac's typer reads as the tuple (`t._1`) and its erasure as `Product`.
                if Some(c) == self.w.b.cons_tuple && args.len() == 2 && matches!(self.w.types.get(args[1]), Type::Class(e, _) if Some(e) == self.w.b.empty_tuple) {
                    self.buf.byte(APPLIEDTYPE);
                    let len = self.buf.begin_length();
                    self.class_typeref(c);
                    self.ty_arg(args[0]);
                    self.external_typeref("scala", "EmptyTuple");
                    self.buf.end_length(len);
                    return;
                }
                self.buf.byte(APPLIEDTYPE);
                let len = self.buf.begin_length();
                self.class_typeref(c);
                for a in args {
                    self.ty_arg(a);
                }
                self.buf.end_length(len);
            }
            Type::Param(p) => self.tparam_ref(p),
            Type::AppParam(p, args) => {
                let args: Vec<TypeId> = self.w.types.items(args).to_vec();
                self.buf.byte(APPLIEDTYPE);
                let len = self.buf.begin_length();
                self.tparam_ref(p);
                for a in args {
                    self.ty_arg(a);
                }
                self.buf.end_length(len);
            }
            Type::Ctor(c) => self.class_typeref(c),
            Type::Lambda(params, body) => {
                let params: Vec<TypeId> = self.w.types.items(params).to_vec();
                self.lambda_type(TYPELAMBDATYPE, &params, body);
            }
            // `[T] => (A, B) => R` as scalac has it: `PolyFunction { def apply[T](x$1: A, x$2: B): R }`,
            // `[T] => A ?=> R` with the clause a using one: the `apply` the function type is refined
            // by, its parameters' names and the result that names them (`[T] => (a: T) => a.type`).
            Type::Poly(..) => {
                // Its parameters holding the type's bounds (`poly_binders`).
                let Some((ids, fun)) = self.w.poly_binders(t) else { unreachable!() };
                let params: Vec<TypeId> = ids.iter().map(|&p| self.w.types.param(p)).collect();
                if let Some(sig) = self.named_apply(fun) {
                    self.placed = true;
                    self.buf.byte(REFINEDTYPE);
                    let len = self.buf.begin_length();
                    let n = self.names.simple("apply");
                    self.buf.nat(n as u64);
                    self.external_typeref("scala", "PolyFunction");
                    self.lambda_type_with(POLYTYPE, &params, |p| p.method_clauses(&sig));
                    self.buf.end_length(len);
                    return;
                }
                // Its `apply` is the type's whole (`makePolyFunctionType`, the reader's alike).
                let shown = self.w.show(fun);
                self.fail(format!("a polymorphic function type over {}", shown));
                self.external_typeref("scala", "Any");
            }
            Type::Var(_) | Type::AppVar(..) => {
                self.approximated("uninstantiated type variable, written as Any");
                self.external_typeref("scala", "Any");
            }
            Type::Union(a, b) => {
                self.buf.byte(ORTYPE);
                let len = self.buf.begin_length();
                self.ty(a);
                self.ty(b);
                self.buf.end_length(len);
            }
            Type::Inter(a, b) => {
                self.buf.byte(ANDTYPE);
                let len = self.buf.begin_length();
                self.ty(a);
                self.ty(b);
                self.buf.end_length(len);
            }
            Type::Lit(l) => self.constant(self.w.types.lit_val(l)),
            Type::Wild => {
                self.buf.byte(TYPEBOUNDS);
                let len = self.buf.begin_length();
                self.ty(NOTHING);
                self.ty(ANY);
                self.buf.end_length(len);
            }
            Type::BoundedWild(lo, hi) => {
                self.buf.byte(TYPEBOUNDS);
                let len = self.buf.begin_length();
                self.ty(lo);
                self.ty(hi);
                self.buf.end_length(len);
            }
            Type::Blocked(b) => {
                let d = self.w.types.blocked_description(b).to_string();
                self.fail(format!("a type teq reads but cannot state ({})", d));
                self.external_typeref("scala", "Nothing");
            }
            Type::This(c) => {
                self.placed = true;
                let local_module = self.w.syms.class(c).local_module.filter(|_| !self.enclosing.contains(&c));
                if self.w.syms.class(c).kind == ClassKind::Object {
                    self.module_termref(c);
                } else if let Some(v) = local_module {
                    // A local object's `this` outside its body: the local val that holds it.
                    self.buf.byte(TERMREFDIRECT);
                    self.def_ref(Key::Sym(v));
                } else {
                    self.buf.byte(THIS);
                    self.class_typeref(c);
                }
            }
            // A mirror teq synthesized, which scalac's pickle has as the companion cast to the
            // mirror's refinement (`product_mirror`): that refinement.
            Type::Term(s) if self.w.scala_mirror_class(s).is_some_and(|c| self.companion_mirror_of(c)) => {
                let c = self.w.scala_mirror_class(s).expect("a mirror's class");
                self.placed = true;
                self.product_mirror_type(c);
            }
            Type::Term(s) => self.term_ref(s),
            Type::Select(prefix, s) => {
                self.placed = true;
                let info = self.sym_info(s);
                if self.is_local(Key::Sym(s)) {
                    self.buf.byte(TERMREFSYMBOL);
                    self.def_ref(Key::Sym(s));
                } else {
                    let n = self.simple_name(info.name);
                    self.buf.byte(TERMREF);
                    self.buf.nat(n as u64);
                }
                self.ty(prefix);
            }
            Type::Member(prefix, name) => {
                self.placed = true;
                let n = self.simple_name(name);
                self.buf.byte(TYPEREF);
                self.buf.nat(n as u64);
                self.ty(prefix);
            }
            // `TYPEREF C p`, its own arguments applied, as scalac pickles `p.C[Ts]`.
            Type::Nested(prefix, class) => {
                let Type::Class(c, args) = self.w.types.get(class) else { return self.ty(class) };
                let args: Vec<TypeId> = self.w.types.items(args).to_vec();
                let len = (!args.is_empty()).then(|| {
                    self.buf.byte(APPLIEDTYPE);
                    self.buf.begin_length()
                });
                self.nested_typeref(c, prefix);
                if let Some(len) = len {
                    for a in args {
                        self.ty_arg(a);
                    }
                    self.buf.end_length(len);
                }
            }
            Type::AppMember(m, args) => {
                let args: Vec<TypeId> = self.w.types.items(args).to_vec();
                self.buf.byte(APPLIEDTYPE);
                let len = self.buf.begin_length();
                self.ty(m);
                for a in args {
                    self.ty_arg(a);
                }
                self.buf.end_length(len);
            }
            Type::Decl(a) => self.alias_ref(a),
            Type::Refined(parent, r) => self.refined(parent, r),
            Type::Match(scrutinee, m) => self.match_type(scrutinee, m),
            Type::Alias(a, args) => {
                let args: Vec<TypeId> = self.w.types.items(args).to_vec();
                if args.is_empty() {
                    return self.alias_ref(a);
                }
                // An alias of a type lambda applied past its own parameters, `~>[F, G][Int]`:
                // the alias's application applied to the rest.
                let own = self.w.syms.alias(a).tparams.len();
                let split = if own > 0 && args.len() > own { own } else { args.len() };
                let outer = (split < args.len()).then(|| {
                    self.buf.byte(APPLIEDTYPE);
                    self.buf.begin_length()
                });
                self.buf.byte(APPLIEDTYPE);
                let len = self.buf.begin_length();
                self.alias_ref(a);
                for &x in &args[..split] {
                    self.ty_arg(x);
                }
                self.buf.end_length(len);
                if let Some(o) = outer {
                    for &x in &args[split..] {
                        self.ty_arg(x);
                    }
                    self.buf.end_length(o);
                }
            }
        }
    }

    /// `h *: t`, ending in `EmptyTuple`.
    fn tuple_chain(&mut self, elems: &[TypeId]) {
        let (Some(cons), Some(empty)) = (self.w.b.cons_tuple, self.w.b.empty_tuple) else {
            return self.fail("a tuple of more than 22 elements without the std's `*:`".to_string());
        };
        match elems.split_first() {
            None => {
                let t = self.w.types.class(empty, &[]);
                self.ty(t);
            }
            Some((&head, rest)) => {
                self.buf.byte(APPLIEDTYPE);
                let len = self.buf.begin_length();
                self.class_typeref(cons);
                self.ty(head);
                self.tuple_chain(rest);
                self.buf.end_length(len);
            }
        }
    }

    /// A type argument: a wildcard is a `TYPEBOUNDS`, anything else the type. A wildcard the
    /// typer captured (`Box[?]`'s argument as a fresh `_`, `capture_wildcards`) is the wildcard
    /// again, `? >: lo <: hi`, as scalac's trees keep the argument.
    fn ty_arg(&mut self, a: TypeId) {
        if let Type::Param(p) = self.w.types.get(a) {
            if self.is_capture(p) {
                let (lo, hi) = (self.w.syms.tparam(p).lower, self.w.syms.tparam(p).upper);
                self.buf.byte(TYPEBOUNDS);
                let len = self.buf.begin_length();
                self.ty(lo);
                self.ty(hi);
                self.buf.end_length(len);
                return;
            }
        }
        self.ty(a);
    }

    /// Whether `p` is a wildcard the typer captured: a `_` no binder in scope declares, which
    /// stands for the unknown type of a value's wildcard argument (TASTY.md, "The binders").
    fn is_capture(&self, p: TParamId) -> bool {
        self.w.syms.tparam(p).name == crate::names::WILDCARD && !self.tparams.iter().any(|(q, _)| *q == p)
    }

    fn tparam_ref(&mut self, p: TParamId) {
        self.placed = true;
        let found = self.tparams.iter().rev().find(|(q, _)| *q == p).map(|&(_, r)| r);
        match found {
            Some(TpRef::Direct(id)) => {
                self.buf.byte(TYPEREFDIRECT);
                self.def_ref(Key::Binding(id));
            }
            Some(TpRef::InClass(at, c)) => {
                self.buf.byte(TYPEREFSYMBOL);
                self.buf.reference(at);
                self.buf.byte(THIS);
                self.class_typeref(c);
            }
            Some(TpRef::Lambda(binder, i)) => {
                self.buf.byte(PARAMTYPE);
                let len = self.buf.begin_length();
                self.buf.reference(binder);
                self.buf.nat(i as u64);
                self.buf.end_length(len);
            }
            // A captured wildcard outside a type argument: its upper bound, as dotty avoids a
            // capture where it would escape.
            None if self.is_capture(p) => {
                let upper = self.w.syms.tparam(p).upper;
                self.ty(upper);
            }
            None => {
                let name = self.name(self.w.syms.tparam(p).name);
                self.fail(format!("a reference to the type parameter {} outside its scope", name));
                self.external_typeref("scala", "Any");
            }
        }
    }

    /// `x.type` for a parameter, a local or a top-level val.
    fn term_ref(&mut self, s: SymId) {
        self.placed = true;
        // A method type's own parameter, which the type is written inside of.
        if let Some(&(_, binder, i)) = self.method_params.iter().rev().find(|(q, ..)| *q == s) {
            self.buf.byte(PARAMTYPE);
            let len = self.buf.begin_length();
            self.buf.reference(binder);
            self.buf.nat(i as u64);
            self.buf.end_length(len);
            return;
        }
        if let Some(&(_, at)) = self.params.iter().rev().chain(self.case_binders.iter().rev()).find(|(q, _)| *q == s) {
            self.buf.byte(TERMREFDIRECT);
            self.buf.reference(at);
            return;
        }
        let info = self.sym_info(s);
        match info.owner {
            // A local written before the type, a case's binder (`s.MirroredElemTypes`).
            Owner::Local if matches!(self.places.get(&Key::Sym(s)), Some(Place::At(_))) => {
                self.buf.byte(TERMREFDIRECT);
                self.def_ref(Key::Sym(s));
            }
            Owner::Local => {
                self.fail(format!("the singleton type of the local {}", self.name(info.name)));
                self.external_typeref("scala", "Any");
            }
            _ => self.member_termref(s),
        }
    }

    fn alias_ref(&mut self, a: AliasId) {
        let info = self.alias_info(a);
        if self.is_local(Key::Alias(a)) {
            self.placed = true;
            self.buf.byte(TYPEREFSYMBOL);
            self.def_ref(Key::Alias(a));
            self.alias_owner_prefix(a, info.owner);
            return;
        }
        let n = self.simple_name(info.name);
        self.buf.byte(TYPEREF);
        self.buf.nat(n as u64);
        self.alias_owner_prefix(a, info.owner);
    }

    fn alias_owner_prefix(&mut self, a: AliasId, owner: Owner) {
        match owner {
            Owner::Package(p) => {
                let info = self.alias_info(a);
                if let Some((class_at, pk)) = self.package_members_this.filter(|&(_, pk)| pk == p && self.is_local(Key::Alias(a))) {
                    self.placed = true;
                    self.buf.byte(THIS);
                    self.buf.byte(TYPEREFSYMBOL);
                    self.buf.reference(class_at);
                    self.package_ref(pk);
                    return;
                }
                if self.w.files.as_slice()[info.file.0 as usize].is_std {
                    // The std's aliases of Scala.js's classes are scalajs-library's classes.
                    if self.pkg_path(p) == "scala.scalajs.js" && js_class_alias(&self.name(info.name)) {
                        return self.package_ref(p);
                    }
                    // The std's top-level aliases are scala-library's package object's.
                    let n = self.names.simple("package");
                    self.buf.byte(TERMREF);
                    self.buf.nat(n as u64);
                    self.package_ref(p);
                    return;
                }
                let stem = super::file_stem(&self.w.files.as_slice()[info.file.0 as usize].path);
                self.file_package_term(p, stem);
            }
            other => self.owner_prefix(other),
        }
    }

    fn constant(&mut self, v: LitVal) {
        match v {
            LitVal::Int(i) => {
                self.buf.byte(INTCONST);
                self.buf.long_int(i as i64);
            }
            LitVal::Long(l) => {
                self.buf.byte(LONGCONST);
                self.buf.long_int(l);
            }
            LitVal::Double(bits) => {
                self.buf.byte(DOUBLECONST);
                self.buf.long_int(bits as i64);
            }
            LitVal::Char(c) => {
                self.buf.byte(CHARCONST);
                self.buf.nat(c as u64);
            }
            LitVal::Bool(b) => self.buf.byte(if b { TRUECONST } else { FALSECONST }),
            LitVal::Str(s) => {
                let n = self.simple_name(s);
                self.buf.byte(STRINGCONST);
                self.buf.nat(n as u64);
            }
        }
    }

    /// `TYPELAMBDAtype`, `POLYtype` or `METHODtype`: the result, then each parameter's bounds
    /// or type with its name. The parameters are `Param` types of teq, named in the result by
    /// the binder's address and their position.
    fn lambda_type(&mut self, tag: u8, params: &[TypeId], body: TypeId) {
        self.lambda_type_with(tag, params, |p| p.ty(body));
    }

    fn lambda_type_with(&mut self, tag: u8, params: &[TypeId], body: impl FnOnce(&mut Self)) {
        self.placed = true;
        let binder = self.buf.addr();
        self.buf.byte(tag);
        let len = self.buf.begin_length();
        let mark = self.tparams.len();
        let ids: Vec<Option<TParamId>> = params.iter().map(|&p| match self.w.types.get(p) {
            Type::Param(id) => Some(id),
            _ => None,
        }).collect();
        for (i, id) in ids.iter().enumerate() {
            if let Some(id) = id {
                self.tparams.push((*id, TpRef::Lambda(binder, i as u32)));
            }
        }
        body(self);
        for id in ids.iter() {
            match id {
                Some(id) => {
                    let info = self.w.syms.tparam(*id).clone();
                    self.buf.byte(TYPEBOUNDS);
                    let b = self.buf.begin_length();
                    self.ty(info.lower);
                    self.ty(info.upper);
                    self.buf.end_length(b);
                    let n = self.simple_name(info.name);
                    self.buf.nat(n as u64);
                }
                None => {
                    self.ty(ANY);
                    let n = self.names.simple("_");
                    self.buf.nat(n as u64);
                }
            }
        }
        self.tparams.truncate(mark);
        self.buf.end_length(len);
    }

    fn refined(&mut self, parent: TypeId, r: RefineId) {
        let refinement = self.w.types.refinement(r);
        self.buf.byte(REFINEDTYPE);
        let len = self.buf.begin_length();
        let n = self.simple_name(refinement.name());
        self.buf.nat(n as u64);
        self.ty(parent);
        match refinement {
            Refinement::Alias(_, t) => {
                self.buf.byte(TYPEBOUNDS);
                let b = self.buf.begin_length();
                self.ty(t);
                self.buf.end_length(b);
            }
            Refinement::Bounds(_, lo, hi) => {
                self.buf.byte(TYPEBOUNDS);
                let b = self.buf.begin_length();
                self.ty(lo);
                self.ty(hi);
                self.buf.end_length(b);
            }
            Refinement::Val(_, _, t) => self.ty(t),
            Refinement::Term(..) => {
                let sig = self.w.refinement_sig(refinement);
                self.refined_method(&sig);
            }
        }
        self.buf.end_length(len);
    }

    /// The signature of the `apply` that a function type with named parameters is refined by,
    /// as the refinement has it.
    pub(super) fn named_apply(&mut self, fun: TypeId) -> Option<std::sync::Arc<MethodSig>> {
        self.w.named_function(fun)?;
        let t = self.w.deref_alias(fun);
        let Type::Refined(_, r) = self.w.types.get(t) else { return None };
        let refinement = self.w.types.refinement(r);
        Some(self.w.refinement_sig(refinement))
    }

    /// A refinement's method: its clauses as nested `METHODtype`s over the refinement's own
    /// types, a parameterless one as `=> R`, a generic one inside the `POLYtype` of its type
    /// parameters.
    fn refined_method(&mut self, sig: &MethodSig) {
        if sig.clauses.is_empty() && sig.tparams.is_empty() {
            self.buf.byte(BYNAMETYPE);
            self.ty(sig.ret);
            return;
        }
        if sig.tparams.is_empty() {
            return self.method_clauses(sig);
        }
        let tparams: Vec<TypeId> = sig.tparams.iter().map(|&p| self.w.types.param(p)).collect();
        self.lambda_type_with(POLYTYPE, &tparams, |p| p.method_clauses(sig));
    }

    /// The clauses of `sig` as nested `METHODtype`s over its result. Each clause's parameters
    /// are its type's from the clause's own types to the result.
    fn method_clauses(&mut self, sig: &MethodSig) {
        self.method_clause(sig, 0);
    }

    fn method_clause(&mut self, sig: &MethodSig, ci: usize) {
        if ci == sig.clauses.len() {
            self.ty(sig.ret);
            return;
        }
        self.placed = true;
        let binder = self.buf.addr();
        self.buf.byte(METHODTYPE);
        let len = self.buf.begin_length();
        let clause = &sig.clauses[ci];
        let mark = self.method_params.len();
        for (i, p) in clause.params.iter().enumerate() {
            self.method_params.push((p.sym, binder, i as u32));
        }
        self.method_clause(sig, ci + 1);
        for p in clause.params.iter() {
            let t = p.ty;
            if p.repeated {
                // `T*` in a method type is `<repeated>[T]`, where a parameter's tree has
                // `Seq[T] @Repeated` (dotty's `MethodType.adaptParamInfo`, `annotatedToRepeated`).
                self.buf.byte(APPLIEDTYPE);
                let l = self.buf.begin_length();
                self.external_typeref("scala", "<repeated>");
                self.ty(t);
                self.buf.end_length(l);
            } else {
                // `=> T`, which a function type's parameter holds as its type already.
                if p.by_name && !matches!(self.w.types.get(t), Type::Class(c, _) if Some(c) == self.w.b.by_name) {
                    self.buf.byte(BYNAMETYPE);
                }
                self.ty(t);
            }
            let n = self.simple_name(p.name);
            self.buf.nat(n as u64);
        }
        self.method_params.truncate(mark);
        if clause.is_implicit {
            self.buf.byte(IMPLICIT);
        } else if clause.is_using {
            self.buf.byte(GIVEN);
        }
        self.buf.end_length(len);
    }

    /// `MATCHtype`: the bound, the scrutinee and each case, a case with pattern variables as a
    /// type lambda over them.
    fn match_type(&mut self, scrutinee: TypeId, m: MatchId) {
        let info = self.w.types.match_info(m).clone();
        self.buf.byte(MATCHTYPE);
        let len = self.buf.begin_length();
        self.ty(info.bound);
        self.ty(scrutinee);
        for case in info.cases.iter() {
            let binders: Vec<TypeId> = self.w.types.items(case.binders).to_vec();
            if binders.is_empty() {
                self.match_case(case.pattern, case.body);
            } else {
                let binder = self.buf.addr();
                self.placed = true;
                self.buf.byte(TYPELAMBDATYPE);
                let l = self.buf.begin_length();
                let mark = self.tparams.len();
                let ids: Vec<Option<TParamId>> = binders.iter().map(|&b| match self.w.types.get(b) {
                    Type::Param(id) => Some(id),
                    _ => None,
                }).collect();
                for (i, id) in ids.iter().enumerate() {
                    if let Some(id) = id {
                        self.tparams.push((*id, TpRef::Lambda(binder, i as u32)));
                    }
                }
                self.match_case(case.pattern, case.body);
                for id in ids.iter() {
                    self.buf.byte(TYPEBOUNDS);
                    let b = self.buf.begin_length();
                    self.ty(NOTHING);
                    self.ty(ANY);
                    self.buf.end_length(b);
                    let name = id.map(|i| self.name(self.w.syms.tparam(i).name)).unwrap_or_else(|| "_".to_string());
                    let n = self.names.simple(&name);
                    self.buf.nat(n as u64);
                }
                self.tparams.truncate(mark);
                self.buf.end_length(l);
            }
        }
        self.buf.end_length(len);
    }

    fn match_case(&mut self, pattern: TypeId, body: TypeId) {
        self.buf.byte(MATCHCASETYPE);
        let len = self.buf.begin_length();
        self.ty(pattern);
        self.ty(body);
        self.buf.end_length(len);
    }

    // ---- spans ----------------------------------------------------------------------------

    /// A definition's span in characters, from its modifiers to its end, or its name's.
    fn def_span(&self, file: FileId, def: Option<crate::ast::DefId>, fallback: Span) -> Span {
        match def {
            Some(d) => self.w.ast(file).def_range(d),
            None => fallback,
        }
    }

    // ---- assembly -------------------------------------------------------------------------

    /// What a class a body defines is named from, as `anon_name` and the emitters name it: an
    /// anonymous class's name read back into its parts, a named local class's definition.
    fn class_origin(&mut self, c: ClassId) -> Option<crate::tasty::origins::ClassOrigin> {
        use crate::tasty::origins::ClassOrigin;
        let (kind, owner, file, start, name) = {
            let i = self.w.syms.class(c);
            (i.kind, i.owner, i.file, i.span.start, self.w.interner.get(i.name).to_string())
        };
        if kind == ClassKind::Anon {
            let (prefix, rest) = name.rsplit_once("$$anon$")?;
            let mut parts = rest.split('$');
            let at = self.origin_place(parts.next()?)?;
            let (mut site, mut repeat) = (None, 1);
            for p in parts {
                if p.contains('_') {
                    site = Some(self.origin_place(p)?);
                } else {
                    repeat = p.parse().ok()?;
                }
            }
            return Some(ClassOrigin::Anonymous { prefix: prefix.to_string(), at, site, repeat });
        }
        if owner == Owner::Local {
            return Some(ClassOrigin::Local { at: (self.origin_source(file)?, start) });
        }
        None
    }

    /// The method an `INLINED` expands, as the outlining names its callee.
    fn callee_origin(&mut self, m: SymId) -> Option<crate::tasty::origins::Callee> {
        let (owner, name, file, span) = {
            let i = self.w.syms.sym(m);
            (i.owner, self.w.interner.get(i.name).to_string(), i.file, i.span)
        };
        let owner = match owner {
            Owner::Class(c) => self.w.library_class_name(c, ""),
            // A top-level method: its file's `$package` object's class, as `INLINED` names it.
            Owner::Package(p) => {
                let stem = super::file_stem(&self.w.files.as_slice()[file.0 as usize].path);
                let pkg = self.pkg_path(p);
                if pkg.is_empty() { format!("{}$package$", stem) } else { format!("{}.{}$package$", pkg, stem) }
            }
            Owner::Local => return None,
        };
        let signature = match self.member_signature(m) {
            Ok(Some((params, result))) => shapes::sig_text(&params, &result),
            _ => String::new(),
        };
        let at = match self.product_place(m) {
            Some(at) => at,
            None => (self.origin_source(file)?, span.start),
        };
        Some(crate::tasty::origins::Callee { owner, name, signature, at, leaves: Vec::new(), leaf_tests: Vec::new() })
    }

    /// The place of a method of an upstream product as its pickle records it: the source's key
    /// and token, the definition's offset (kind 1), which the outlining of a downstream names
    /// it by as the whole build names it by its file.
    fn product_place(&mut self, m: SymId) -> Option<crate::tasty::origins::Place> {
        let loaded = self.w.loaded.as_ref()?;
        let l = loaded.syms.get(&m)?;
        let crate::tasty::origins::Found::Read(o) = &loaded.files.as_slice().get(l.file as usize)?.provenance else { return None };
        let i = o.definitions.binary_search_by_key(&l.addr, |&(a, _)| a).ok()?;
        let (key, token, offset) = (o.key.clone(), o.token, o.definitions[i].1);
        Some((self.origin_source_keyed(key, token), offset))
    }

    /// A position as a generated name writes it, `<tag>_<offset>`, as a place of the origins.
    fn origin_place(&mut self, text: &str) -> Option<crate::tasty::origins::Place> {
        let (tag, offset) = text.rsplit_once('_')?;
        let offset = offset.parse().ok()?;
        if let Some(&file) = self.index.files_by_tag.get(tag) {
            return Some((self.origin_source(file)?, offset));
        }
        let (key, token) = self.index.product_sources_by_tag.get(tag)?.clone();
        Some((self.origin_source_keyed(key, token), offset))
    }

    /// The source of the file `f` among the origins' sources: a pseudo file of a product by its
    /// source's key and token, a jar's file by its key; none for a file no source stands for.
    fn origin_source(&mut self, f: FileId) -> Option<u32> {
        if f == self.file {
            return Some(0);
        }
        let key = self.w.files.as_slice().get(f.0 as usize)?.key.clone();
        let token = match self.w.prog.file_tags.get(f.0 as usize) {
            Some(&token) => token,
            None => match self.w.loaded.as_ref().and_then(|l| l.product_files.get(&f).and_then(|&s| l.product_sources.as_slice().get(s as usize))) {
                Some((key, token)) => return Some(self.origin_source_keyed(key.clone(), *token)),
                None => 0,
            },
        };
        Some(self.origin_source_keyed(key, token))
    }

    /// The source of the key and token among the origins' sources.
    fn origin_source_keyed(&mut self, key: String, token: u64) -> u32 {
        match self.origin_sources.iter().position(|(k, t)| *k == key && *t == token) {
            Some(i) => i as u32 + 1,
            None => {
                self.origin_sources.push((key, token));
                self.origin_sources.len() as u32
            }
        }
    }

    fn assemble(&mut self) -> (Vec<u8>, [u8; 16]) {
        let buf = std::mem::take(&mut self.buf);
        let (trees, moves) = buf.finish();
        let positions = self.positions_section(&moves);
        let attributes = self.attributes_section();
        // Last, so that its name is the name table's last and no reference to another moves.
        let origins_name = self.names.simple(crate::tasty::origins::SECTION);
        self.definitions.sort_unstable();
        let pending = std::mem::take(&mut self.class_origins);
        let mut classes: Vec<(u32, crate::tasty::origins::ClassOrigin)> = pending.into_iter().filter_map(|(addr, c)| Some((moves.map(addr) as u32, self.class_origin(c)?))).collect();
        classes.sort_by_key(|&(a, _)| a);
        let inlined = std::mem::take(&mut self.inlined_callees);
        let mut leaves: Vec<Vec<u32>> = vec![Vec::new(); inlined.len()];
        for (i, addr) in std::mem::take(&mut self.inlined_leaves) {
            leaves[i].push(moves.map(addr) as u32);
        }
        let mut leaf_tests: Vec<Vec<u32>> = vec![Vec::new(); inlined.len()];
        for (i, addr) in std::mem::take(&mut self.inlined_leaf_tests) {
            leaf_tests[i].push(moves.map(addr) as u32);
        }
        let mut callees: Vec<(u32, crate::tasty::origins::Callee)> = inlined
            .into_iter()
            .zip(leaves.into_iter().zip(leaf_tests))
            .filter_map(|((addr, m), (mut leaves, mut leaf_tests))| {
                let mut c = self.callee_origin(m)?;
                leaves.sort_unstable();
                leaf_tests.sort_unstable();
                (c.leaves, c.leaf_tests) = (leaves, leaf_tests);
                Some((moves.map(addr) as u32, c))
            })
            .collect();
        callees.sort_by_key(|&(a, _)| a);
        let sources: Vec<(String, u64)> = std::mem::take(&mut self.origin_sources);
        let key = &self.w.files.as_slice()[self.file.0 as usize].key;
        let mut withheld: Vec<(u32, String)> = std::mem::take(&mut self.withheld).into_iter().map(|(addr, reason)| (moves.map(addr) as u32, reason)).collect();
        withheld.sort();
        let mut appended: Vec<(u32, u32)> = std::mem::take(&mut self.appended_parents).into_iter().map(|(addr, n)| (moves.map(addr) as u32, n)).collect();
        appended.sort_unstable();
        let origins = crate::tasty::origins::write(key, self.w.prog.file_tags[self.file.0 as usize], &self.definitions, &sources, &classes, &callees, &withheld, &appended);
        let sections: Vec<(u32, Vec<u8>)> = {
            let asts = self.names.simple("ASTs");
            vec![(asts, trees), (self.positions_name, positions), (self.attributes_name, attributes), (origins_name, origins)]
        };
        let names = self.names.assemble();
        let name_bytes = strip_length(&names);
        let tree_hash = pjw_hash64(&sections[0].1);
        let name_hash = pjw_hash64(name_bytes);
        let low = name_hash ^ tree_hash;
        let high = sections[1..].iter().fold(0u64, |acc, (_, b)| acc ^ pjw_hash64(b));
        let mut uuid = [0u8; 16];
        uuid[..8].copy_from_slice(&low.to_be_bytes());
        uuid[8..].copy_from_slice(&high.to_be_bytes());
        let mut out = Vec::with_capacity(names.len() + sections.iter().map(|s| s.1.len() + 8).sum::<usize>() + 64);
        out.extend_from_slice(&[0x5c, 0xa1, 0xab, 0x1f]);
        super::buf::write_nat(&mut out, crate::tasty::MAJOR_VERSION as u64);
        super::buf::write_nat(&mut out, crate::tasty::MINOR_VERSION as u64);
        super::buf::write_nat(&mut out, 0);
        super::buf::write_nat(&mut out, TOOLING.len() as u64);
        out.extend_from_slice(TOOLING.as_bytes());
        out.extend_from_slice(&uuid);
        out.extend_from_slice(&names);
        for (name, bytes) in &sections {
            super::buf::write_nat(&mut out, *name as u64);
            super::buf::write_nat(&mut out, bytes.len() as u64);
            out.extend_from_slice(bytes);
        }
        (out, uuid)
    }

    /// `Positions`: the lines' sizes, then per definition its span as deltas from the one
    /// before, the source's path after the first.
    fn positions_section(&mut self, moves: &super::buf::Moves) -> Vec<u8> {
        self.positions_name = self.names.simple("Positions");
        let lines = self.lines;
        let mut out = lines.sizes.clone();
        // A position counts UTF-16 units of its tree's source.
        let chars = |p: &Self, file: FileId, b: u32| -> i64 {
            if p.is_pickled_source(file) {
                return b as i64;
            }
            let units = if file == p.file { &lines.units } else { &p.other_lines[&file].units };
            if units.is_empty() {
                return b as i64;
            }
            units[(b as usize).min(units.len() - 1)] as i64
        };
        let (mut last_addr, mut last_start, mut last_end) = (0i64, 0i64, 0i64);
        // In address order, the first entry of an address kept: the root's at 0 comes first.
        let mut entries: Vec<Pos> = self.positions.clone();
        entries.sort_by_key(|e| e.addr);
        entries.dedup_by_key(|e| e.addr);
        moves.map_ascending(entries.iter_mut().map(|e| &mut e.addr));
        self.definitions.clear();
        for e in entries.iter() {
            let (a, span, point) = (e.addr as i64, e.span, e.point);
            let (start, end) = (chars(self, e.file, span.start), chars(self, e.file, span.end));
            let (ds, de) = (start - last_start, end - last_end);
            let header = ((a - last_addr) << 3) | ((ds != 0) as i64) << 2 | ((de != 0) as i64) << 1 | point.is_some() as i64;
            super::buf::write_long_int(&mut out, header);
            if ds != 0 {
                super::buf::write_long_int(&mut out, ds);
            }
            if de != 0 {
                super::buf::write_long_int(&mut out, de);
            }
            if let Some(p) = point {
                super::buf::write_long_int(&mut out, chars(self, e.file, p) - start);
                if e.def && e.file == self.file {
                    self.definitions.push((a as u32, p));
                }
            }
            last_addr = a;
            last_start = start;
            last_end = end;
            // The tree's source, which holds for its subtree: the root's, and a tree's whose
            // source is not its parent's.
            if e.switch {
                let path = self.source_path_of(e.file);
                let source = self.names.simple(&path);
                super::buf::write_long_int(&mut out, 4);
                super::buf::write_long_int(&mut out, source as i64);
            }
        }
        out
    }

    /// The path a pickle names a source by, relative to the source root as scalac's
    /// `SourceFile.relativePath` gives it.
    fn source_path_of(&self, file: FileId) -> String {
        if file == self.file {
            return self.source_path.clone();
        }
        if self.is_pickled_source(file) {
            return self.pickled_sources[(u32::MAX - file.0) as usize].clone();
        }
        super::relative_path(&self.w.files.as_slice()[file.0 as usize].path, &self.index.sourceroot, self.index.cwd.as_deref())
    }

    fn attributes_section(&mut self) -> Vec<u8> {
        self.attributes_name = self.names.simple("Attributes");
        let mut out = Vec::new();
        // OUTLINEattr: a body is elided, which `TreeUnpickler` admits under it alone.
        if self.elided_bodies > 0 {
            out.push(6);
        }
        out.push(129);
        let n = self.names.simple(&self.source_path.clone());
        super::buf::write_nat(&mut out, n as u64);
        out
    }
}

/// An object as it is pickled: its class when the program has one, the class it is the
/// companion of, and what its module val and class are written with.
struct Obj {
    class: Option<ClassId>,
    companion: Option<ClassId>,
    name: String,
    owner: Owner,
    span: Span,
    /// Where its name is.
    name_at: u32,
    synthetic: bool,
    mods: Mods,
}

/// A sealed class's child as `@Child` names it.
#[derive(Clone, Copy)]
enum ChildRef {
    Class(ClassId),
    Object(ClassId),
    Value(SymId, ClassId),
}

impl ChildRef {
    fn class(self) -> ClassId {
        match self {
            ChildRef::Class(c) | ChildRef::Object(c) | ChildRef::Value(_, c) => c,
        }
    }
}

enum BodyItem {
    Sym(SymId),
    Class(ClassId),
    Alias(AliasId),
}

impl<'w, 'a> P<'w, 'a> {
    /// A template's statements (`TInit::Stmt`), by where they start, last first.
    fn template_stmts(&mut self, c: ClassId) -> Vec<(u32, crate::tir::TExprId)> {
        let Some(&i) = self.index.tclasses.get(&c) else { return Vec::new() };
        let mut out: Vec<(u32, crate::tir::TExprId)> = self.w.prog.classes[i as usize]
            .init
            .iter()
            .filter_map(|x| match *x {
                crate::tir::TInit::Stmt(e) => Some((self.w.prog.span_of(e).map_or(u32::MAX, |(_, s)| s.start), e)),
                _ => None,
            })
            .collect();
        out.sort_by_key(|&(at, _)| std::cmp::Reverse(at));
        out
    }

    /// Where a member of a template starts in its source.
    fn item_start(&self, m: &BodyItem) -> Option<u32> {
        match *m {
            BodyItem::Sym(s) => {
                let info = self.w.syms.sym(s);
                Some(self.def_span(info.file, info.def, info.span).start)
            }
            BodyItem::Class(k) => Some(self.w.syms.class(k).span.start),
            BodyItem::Alias(a) => {
                let info = self.w.syms.alias(a);
                info.def.map(|d| self.w.ast(info.file).def(d).span.start)
            }
        }
    }

    /// The template's statements that start before `at` (all of them for none), each a term of
    /// the template; one that cannot be stated is left out with its reason, the pickle an
    /// outline.
    fn statements_before(&mut self, c: ClassId, stmts: &mut Vec<(u32, crate::tir::TExprId)>, at: Option<u32>) {
        while let Some(&(start, e)) = stmts.last() {
            if at.map_or(false, |a| start >= a) {
                break;
            }
            stmts.pop();
            let mark = self.body_begin();
            let (src, ctx) = (self.src_ctx, self.span_ctx);
            self.src_ctx = self.file;
            self.term(e);
            self.src_ctx = src;
            self.span_ctx = ctx;
            match self.body_end(mark) {
                Ok(()) => self.count_body(Producer::Synthesized("template statement")),
                Err(reason) => {
                    self.count_withheld(&format!("{} (a template statement)", reason));
                    self.elided_bodies += 1;
                    let reason = self.pending_withheld.take().unwrap_or_default();
                    if let Some(&Place::At(class)) = self.places.get(&Key::Class(c)) {
                        self.withheld.push((class, reason));
                    }
                }
            }
        }
    }
}

#[derive(Clone, Copy)]
enum Qualifier {
    Pkg(PkgId),
    Class(ClassId),
}

fn strip_length(names: &[u8]) -> &[u8] {
    let mut r = crate::tasty::Reader::new(names);
    r.nat();
    &names[r.pos..]
}

/// scalac's `TastyHash.pjwHash64`.
pub fn pjw_hash64(data: &[u8]) -> u64 {
    let mut h: u64 = 0;
    for &b in data {
        h = (h << 8).wrapping_add(b as u64);
        let high = h & 0xFF00_0000_0000_0000;
        h ^= high >> 48;
        h &= !high;
    }
    h
}

thread_local! {
    /// The names `scala_name` gave classes while the products are written, which
    /// `with_scala_names` keeps for one write.
    static SCALA_NAMES: std::cell::RefCell<Option<FxMap<ClassId, String>>> = const { std::cell::RefCell::new(None) };
}

/// The clauses of a method as its declaration pickles them, by their index in the signature: a
/// right-associative extension's first explicit clause before its receiver's, as scalac's
/// `paramss` hold them, every other method's in order.
pub(super) fn declared_clause_order(w: &Worker, info: &SymInfo, sig: &MethodSig) -> Vec<usize> {
    let swapped = (info.is_extension && w.interner.get(info.name).ends_with(':')).then(|| sig.right_assoc_order(info.ext_clauses as usize)).flatten();
    swapped.unwrap_or_else(|| (0..sig.clauses.len()).collect())
}

/// Runs `f` with the classes' names kept as they are given: a class's name does not change
/// while the products are written.
pub fn with_scala_names<R>(f: impl FnOnce() -> R) -> R {
    SCALA_NAMES.with(|m| *m.borrow_mut() = Some(FxMap::default()));
    let r = f();
    SCALA_NAMES.with(|m| *m.borrow_mut() = None);
    r
}

/// A class's qualified name as scala-library and the JDK spell it: the std's classes by the
/// library's names (`scala.List` is `scala.collection.immutable.List`), the builtins by
/// scalac's (`java.lang.String`), a nested class with `.` after its enclosing class and `$.`
/// after an object's class, as signatures write it.
pub fn scala_name(w: &Worker, c: ClassId) -> String {
    if let Some(n) = SCALA_NAMES.with(|m| m.borrow().as_ref().and_then(|m| m.get(&c).cloned())) {
        return n;
    }
    let n = scala_name_now(w, c);
    SCALA_NAMES.with(|m| {
        if let Some(m) = m.borrow_mut().as_mut() {
            m.insert(c, n.clone());
        }
    });
    n
}

fn scala_name_now(w: &Worker, c: ClassId) -> String {
    let info = w.syms.class(c);
    let name = w.interner.get(info.name).to_string();
    if c == w.b.string {
        return "java.lang.String".to_string();
    }
    match info.owner {
        Owner::Package(p) => {
            let pkg = pkg_path_of(w, p);
            let is_std = w.files.as_slice().get(info.file.0 as usize).map_or(false, |f| f.is_std) || info.def.is_none() && info.kind == ClassKind::Builtin;
            if is_std {
                if let Some(lib) = std_library_name(&pkg, &name) {
                    return lib;
                }
            }
            if pkg.is_empty() { name } else { format!("{}.{}", pkg, name) }
        }
        Owner::Class(o) => {
            let oi = w.syms.class(o);
            let sep = if oi.kind == ClassKind::Object { "$." } else { "." };
            format!("{}{}{}", scala_name(w, o), sep, name)
        }
        Owner::Local => name,
    }
}

fn pkg_path_of(w: &Worker, p: PkgId) -> String {
    let mut segs = Vec::new();
    let mut at = Some(p);
    while let Some(k) = at {
        if k == ROOT_PKG {
            break;
        }
        let info = w.syms.pkg(k);
        segs.push(w.interner.get(info.name).to_string());
        at = info.parent;
    }
    segs.reverse();
    segs.join(".")
}

/// scala-library's name for a class the std defines under another package, the inverse of
/// `STD_BINDINGS` with the library's canonical choice where several of its names bind to one.
fn std_library_name(pkg: &str, name: &str) -> Option<String> {
    let preferred: &[(&str, &str, &str)] = &[
        ("scala", "Iterable", "scala.collection.Iterable"),
        ("scala", "SeqOps", "scala.collection.SeqOps"),
        ("scala", "SetOps", "scala.collection.SetOps"),
        ("scala", "StringBuilder", "scala.collection.mutable.StringBuilder"),
    ];
    if let Some(&(_, _, full)) = preferred.iter().find(|(p, n, _)| *p == pkg && *n == name) {
        return Some(full.to_string());
    }
    crate::typer::loader::compile::STD_BINDINGS
        .iter()
        .find(|(_, n, to)| *n == name && *to == pkg)
        .map(|(from, n, _)| format!("{}.{}", from, n))
}

/// The lean std's aliases in `scala.scalajs.js` of what scalajs-library declares as classes:
/// `js.Any`, `js.Array`, `js.FunctionN`, `js.JavaScriptException`.
fn js_class_alias(name: &str) -> bool {
    matches!(name, "Any" | "Array" | "JavaScriptException") || name.strip_prefix("Function").map_or(false, |n| n.parse::<u32>().is_ok())
}

/// The lean std's traits scala-library declares as abstract classes.
const STD_TRAITS_LIBRARY_CLASSES: &[&str] = &["scala.Conversion"];
