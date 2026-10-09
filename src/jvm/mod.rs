//! The JVM backend: class files from the typed IR.
//!
//! Every reached class of the program becomes a class file, the top-level definitions of a file
//! form the module class `file$package$` as under scalac, and lambdas are `invokedynamic` call
//! sites over synthetic static methods of the class they stand in. Classes are independent of
//! each other, so they are emitted on all cores like the chunks of the JS output.

mod callable;
pub mod classfile;
mod classes;
mod free;
mod gen;
mod intrinsic;
pub mod kept;
pub mod names;
mod pattern;
pub mod analysis;
pub mod api;
mod runtime;

use crate::ast::mods;
use crate::emit::layout::Layout;
use crate::emit::reach::Reach;
use crate::intern::{FxMap, Interner};
use crate::source::{FileId, SourceFile, Span};
use crate::symbols::*;
use crate::tir::*;
use crate::types::*;
use names::encode;
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct Input<'a> {
    pub prog: &'a Program,
    pub syms: &'a Symbols,
    pub types: &'a TypeStore,
    /// The solution of every inference variable, by variable.
    /// The type variables' instances, every worker's (`TVars`).
    pub insts: &'a crate::typer::TVars,
    pub interner: &'a Interner,
    pub reach: &'a Reach,
    pub array_seq: Option<ClassId>,
    pub sources: &'a [SourceFile],
    pub file_pkgs: &'a [PkgId],
    pub asts: &'a crate::ast::Asts,
    /// The Java classes of the classpath and the JDK, called through the descriptors their
    /// class files give.
    pub java: Option<&'a crate::typer::loader::Java>,
    /// The `@targetName`s of the members read from jars.
    pub target_names: Option<&'a crate::arena::SharedMap<SymId, crate::intern::Name>>,
    /// The `@targetName`s of the program's and the std's defs, resolved by the annotation's class
    /// as the typer resolves them (`Worker::source_target_names`), so that an imported alias names
    /// the method too.
    pub source_target_names: Option<&'a FxMap<SymId, crate::intern::Name>>,
    /// The `@targetName`s of the members read from other modules' products, whose class files
    /// teq wrote under those names too.
    pub product_target_names: Option<&'a crate::arena::SharedMap<SymId, crate::intern::Name>>,
    /// The vars and vals of jars' and products' traits that `@volatile` marks (`Loaded::volatile`).
    pub volatile: Option<&'a crate::arena::SharedMap<SymId, ()>>,
    /// The sources' vals, vars and class parameters that `@volatile` marks (`Worker::jvm_volatile`).
    pub source_volatile: Option<&'a FxMap<SymId, ()>>,
    pub link: Link<'a>,
    /// Per file, whether its classes are written (`--own`): the other program files are typed
    /// but their class files belong to another build. The std's files are always written.
    pub owned: Option<&'a [bool]>,
    /// `--all-mains`: no launcher, and every entry point of an owned file gets the class the
    /// `java` launcher runs, as scalac writes them.
    pub all_mains: Option<&'a [(SymId, Option<ClassId>)]>,
    /// `--java-output-version`: the Java release whose class file version is written.
    pub output_version: u32,
    /// The module-product mode: classes of other modules may extend the program's, so a val
    /// of a class that is not final is read through its accessor, as scalac reads it.
    pub open_world: bool,
    /// In the product mode, the members each trait calls through `super`, whose accessors it
    /// declares whether or not a class of the program mixes it in.
    pub mixin_supers: Option<&'a FxMap<ClassId, Vec<SymId>>>,
    /// In the product mode, the `abstract override` members of other modules' traits: marked
    /// abstract, they have an implementation a class mixing the trait in forwards to.
    pub stackable: Option<&'a FxMap<SymId, ()>>,
    /// The module classes of the class path's product directories whose initialisation runs
    /// code (their manifests' `inits`).
    pub product_inits: Option<&'a FxMap<String, ()>>,
}

/// Link mode (`Worker::link_mode`), the JVM's one mode: the output runs beside the jars of the
/// class path, which define the classes it calls.
pub struct Link<'a> {
    pub jar_classes: &'a crate::arena::SharedMap<ClassId, crate::typer::loader::LClass>,
}

impl<'a> Input<'a> {
    pub fn java_class(&self, c: ClassId) -> Option<&'a crate::typer::loader::javaclass::JClass> {
        self.java.and_then(|j| j.classes.get(&c))
    }

    /// A member read from a Java class file: the class file and the member's index in it.
    pub fn java_member(&self, s: SymId) -> Option<(&'a crate::classfile::ClassFile, crate::typer::loader::javaclass::JMember)> {
        let j = self.java?;
        let js = j.syms.get(&s)?;
        let jc = j.classes.get(&js.class)?;
        Some((&jc.cf, js.member))
    }
}

/// What one class file is made from.
#[derive(Clone, Copy, Debug)]
pub enum Unit {
    /// An index into `Program::classes`.
    Class(usize),
    /// The top-level definitions of a file: an index into `Layout::file_groups` when the file
    /// has vals, and the file.
    File(FileId),
}

/// The tables every emitter thread reads.
pub struct Cx<'a> {
    pub input: Input<'a>,
    pub layout: Layout,
    pub class_names: Vec<String>,
    pub class_by_name: FxMap<String, ClassId>,
    /// In the product mode, every class by its binary name, reached or not: whether a value of
    /// one class passes for another without a cast is not the rest of the program's to decide.
    pub all_by_name: FxMap<String, ClassId>,
    /// Per class its index in `Program::classes`, `u32::MAX` without one.
    pub tclass_of: Vec<u32>,
    /// Per symbol the function that defines it, `u32::MAX` without one.
    pub fun_of_sym: Vec<u32>,
    /// Per file the name of its module class.
    pub file_modules: Vec<String>,
    /// Per file the top-level functions and the index of its val group.
    pub file_funs: Vec<Vec<FunId>>,
    pub file_group: Vec<u32>,
    pub helpers: runtime::Helpers,
    /// Per trait the members its `super` calls name, which it declares as abstract accessors.
    pub super_called: FxMap<ClassId, Vec<SymId>>,
    /// Per file the offsets its lines start at, made when a line of the file is first asked.
    line_starts: Vec<std::sync::OnceLock<Vec<u32>>>,
    /// Per package its path as binary names begin with it, made when first asked.
    pkg_paths: Vec<std::sync::OnceLock<String>>,
    /// In link mode, per class whether the class path holds its binary name, and the class
    /// files of the jar classes the reach met.
    pub held: Vec<bool>,
    pub class_files: FxMap<ClassId, std::sync::Arc<crate::classfile::ClassFile>>,
    /// The companions (`C$`) of the jar's value classes among them, which hold `m$extension`.
    pub vc_companions: FxMap<ClassId, std::sync::Arc<crate::classfile::ClassFile>>,
    /// Those of a Scala 2 jar, each with the pickle (`ScalaSignature`) that describes it: its own
    /// class file's, or a nested class's owner's, which holds it (`owning_pickle`).
    pub scala2_pickles: FxMap<ClassId, std::sync::Arc<Vec<u8>>>,
    /// Every class by its binary name, made when an erasure first asks.
    named: std::sync::OnceLock<FxMap<String, ClassId>>,
    /// The given objects of a package or of objects (`given x: T with { ... }`), by their class
    /// and by their given: scalac's module classes `Owner$x$`, which every read reaches through
    /// `MODULE$` (`static_given_objects`).
    pub given_object_classes: FxMap<ClassId, SymId>,
    pub given_objects: FxMap<SymId, ClassId>,
    /// The given classes of a package, of objects and of class, trait and enum instances (`given
    /// listShow[A](using Show[A]): Show[List[A]] with { ... }`), by their class and by their
    /// given: scalac's class `Owner$name` and the def `name(params)` answering a new one, which
    /// the owner writes (`given_class_defs`).
    pub given_class_classes: FxMap<ClassId, SymId>,
    pub given_classes: FxMap<SymId, ClassId>,
    /// The given objects of class and trait instances (`given Show[Int] with { ... }` in a
    /// trait), by their class and by their given: scalac's inner module class `Owner$x$`, which
    /// the given's getter answers, as an inner object's (`static_given_objects`).
    pub inner_given_object_classes: FxMap<ClassId, SymId>,
    pub inner_given_objects: FxMap<SymId, ClassId>,
    /// The plain constructor parameters (`class C(x: Int)`, `(using Int)`) that only their
    /// class's own code reads: scalac's private final field, of no accessor (`private_params`).
    pub private_params: FxMap<SymId, ()>,
    /// Whether the class path holds `scala.runtime.BoxesRunTime`, whose `unboxToX` scalac's
    /// erasure unboxes through; without it an unboxing is spelled out.
    pub boxes_runtime: bool,
}

struct SyncCx<'a>(&'a Cx<'a>);
// The loader's `Java` table holds a `RefCell` memo, which the emitter's threads never touch.
unsafe impl Sync for SyncCx<'_> {}
unsafe impl Send for SyncCx<'_> {}

pub struct Output {
    pub classes: Vec<Emitted>,
    pub main_class: Option<String>,
    pub errors: Vec<String>,
    /// Under `--all-mains`, the classes the `java` launcher runs, by source file: the class of
    /// a `@main` method, or the class holding an object's static `main`.
    pub main_classes: Vec<(FileId, String)>,
    /// The module classes of the program's objects whose initialisation runs code.
    pub inits: Vec<String>,
    /// How a session's build used the class files it kept (`kept`).
    pub kept: kept::Use,
}

/// A class file by binary name, with the file and the class of the program it stands for: a
/// runtime class or a launcher has neither, a mirror or companion class has its module's. The
/// bytes are shared with what a session keeps of them (`kept`).
#[derive(Clone)]
pub struct Emitted {
    pub name: String,
    pub bytes: std::sync::Arc<Vec<u8>>,
    pub source: Option<FileId>,
    pub of: Option<ClassId>,
}

impl<'a> Cx<'a> {
    fn new(input: Input<'a>, lap: &mut crate::measure::Lap) -> Cx<'a> {
        let layout = Layout::new(input.prog, input.syms, input.interner);
        lap.done("tables: layout", 0);
        let syms = input.syms;
        let n_classes = syms.classes.len();
        let mut cx = Cx {
            layout,
            class_names: Vec::with_capacity(n_classes),
            class_by_name: FxMap::default(),
            all_by_name: FxMap::default(),
            tclass_of: vec![u32::MAX; n_classes],
            fun_of_sym: vec![u32::MAX; syms.syms.len()],
            file_modules: Vec::new(),
            file_funs: vec![Vec::new(); input.sources.len()],
            file_group: vec![u32::MAX; input.sources.len()],
            helpers: runtime::Helpers::default(),
            super_called: FxMap::default(),
            line_starts: input.sources.iter().map(|_| std::sync::OnceLock::new()).collect(),
            pkg_paths: (0..syms.pkgs.len()).map(|_| std::sync::OnceLock::new()).collect(),
            held: Vec::new(),
            class_files: FxMap::default(),
            vc_companions: FxMap::default(),
            scala2_pickles: FxMap::default(),
            named: std::sync::OnceLock::new(),
            given_object_classes: FxMap::default(),
            given_objects: FxMap::default(),
            given_class_classes: FxMap::default(),
            given_classes: FxMap::default(),
            inner_given_object_classes: FxMap::default(),
            inner_given_objects: FxMap::default(),
            private_params: FxMap::default(),
            boxes_runtime: false,
            input,
        };
        cx.static_given_objects();
        cx.private_params();
        // The files' module classes first: a top-level given object is named inside its file's.
        for (f, file) in cx.input.sources.iter().enumerate() {
            let pkg = cx.pkg_path(cx.input.file_pkgs[f]);
            let stem = file.path.rsplit(['/', '\\']).next().unwrap_or(&file.path);
            let stem = stem.strip_suffix(".scala").unwrap_or(stem);
            cx.file_modules.push(format!("{}{}$package$", pkg, encode(stem)));
        }
        for i in 0..n_classes {
            let name = cx.make_class_name(ClassId(i as u32));
            cx.class_names.push(name);
        }
        lap.done("tables: class names", n_classes);
        // Another module's object runs code when it is loaded where its manifest says so.
        for &c in &cx.input.reach.external_bodies {
            let object = syms.class(c).kind == ClassKind::Object;
            if !object || cx.input.product_inits.map_or(true, |inits| inits.contains_key(&cx.class_names[c.idx()])) {
                cx.layout.note_external(c);
            }
        }
        // Another module's classes are named where the program only names them as types: a
        // value of one of its classes is passed where its trait is expected.
        for (i, name) in cx.class_names.iter().enumerate() {
            if cx.input.reach.classes[i] || cx.input.reach.product_classes.get(i).copied().unwrap_or(false) {
                cx.class_by_name.insert(name.clone(), ClassId(i as u32));
            }
            if cx.input.open_world {
                cx.all_by_name.entry(name.clone()).or_insert(ClassId(i as u32));
            }
        }
        if cx.input.open_world {
            for (&t, members) in cx.input.mixin_supers.into_iter().flatten() {
                if cx.input.syms.class(t).def.is_some() {
                    cx.super_called.insert(t, members.clone());
                }
            }
        }
        for (i, tc) in cx.input.prog.classes.iter().enumerate() {
            cx.tclass_of[tc.id.idx()] = i as u32;
            for a in &tc.super_accessors {
                let members = cx.super_called.entry(a.of_trait).or_default();
                if !members.contains(&a.member) {
                    members.push(a.member);
                }
            }
        }
        for (i, f) in cx.input.prog.funs.iter().enumerate() {
            cx.fun_of_sym[f.sym.idx()] = i as u32;
        }
        for &f in &cx.input.prog.top_funs {
            let file = syms.sym(cx.input.prog.funs[f.idx()].sym).file;
            cx.file_funs[file.0 as usize].push(f);
        }
        for (g, (file, _)) in cx.layout.file_groups.iter().enumerate() {
            cx.file_group[file.0 as usize] = g as u32;
        }
        cx.helpers = runtime::Helpers::find(&cx);
        lap.done("tables: the rest", 0);
        cx
    }

    pub fn line_of(&self, file: FileId, offset: u32) -> u32 {
        let starts = self.line_starts[file.0 as usize].get_or_init(|| {
            let text = &self.input.sources[file.0 as usize].text;
            std::iter::once(0).chain(text.bytes().enumerate().filter(|(_, b)| *b == b'\n').map(|(i, _)| i as u32 + 1)).collect()
        });
        starts.partition_point(|&s| s <= offset) as u32
    }

    /// Where a member's name begins in `span`, past the receiver `recv` spans at its start, the
    /// whitespace, dots and parentheses between them skipped; the span's start when the receiver
    /// is not that prefix (a synthesized one, or one spanning the whole).
    pub fn point_after(&self, file: FileId, span: Span, recv: Option<(FileId, Span)>) -> u32 {
        let Some((rf, rs)) = recv else { return span.start };
        if rf != file || rs.start < span.start || rs.end >= span.end {
            return span.start;
        }
        let text = self.input.sources[file.0 as usize].text.as_bytes();
        let end = (span.end as usize).min(text.len());
        let mut i = rs.end as usize;
        while i < end && matches!(text[i], b' ' | b'\t' | b'\r' | b'\n' | b'.' | b')') {
            i += 1;
        }
        i as u32
    }

    fn pkg_path(&self, p: PkgId) -> String {
        self.pkg_paths[p.idx()]
            .get_or_init(|| {
                let info = self.input.syms.pkg(p);
                match info.parent {
                    Some(parent) => format!("{}{}/", self.pkg_path(parent), encode(self.input.interner.get(info.name))),
                    None => String::new(),
                }
            })
            .clone()
    }

    /// The binary name of a class: `pkg/Outer$Inner`, with a trailing `$` for an object.
    fn make_class_name(&self, c: ClassId) -> String {
        let syms = self.input.syms;
        let info = syms.class(c);
        let text = self.input.interner.get(info.name);
        if let Some(name) = self.jvm_class(c) {
            return name;
        }
        if let Some(jc) = self.input.java_class(c) {
            return match jc.companion_of {
                Some(_) => format!("{}$", jc.cf.name),
                None => jc.cf.name.clone(),
            };
        }
        if self.is_function_evidence(c) {
            return "scala/Function1".to_string();
        }
        // A tuple of more than 22 elements is scalac's TupleXXL, which the typer makes and reads
        // (`typer/arity.rs`): the class of the arity is a type alone, named as scalac erases it.
        if info.kind == ClassKind::Class
            && matches!(info.owner, Owner::Package(p) if self.pkg_path(p) == "scala/")
            && text.strip_prefix("Tuple").and_then(|n| n.parse::<usize>().ok()).map_or(false, |n| n > 22)
        {
            return "scala/runtime/TupleXXL".to_string();
        }
        if info.kind == ClassKind::Builtin {
            return match text {
                "String" => names::STRING.to_string(),
                "Array" => names::ARRAY.to_string(),
                "Unit" => names::BOXED_UNIT.to_string(),
                _ => names::OBJECT.to_string(),
            };
        }
        let mut out = match info.owner {
            // A top-level given object or class is scalac's class in its file's `X$package`. One of
            // a package object too, whose members teq's pickles and class files hold in the
            // file's `X$package` (`tasty::write`, units): a downstream finds it there, where
            // scalac's are the object's `package$`.
            Owner::Package(_) if (self.given_object_classes.contains_key(&c) || self.given_class_classes.contains_key(&c)) && (info.file.0 as usize) < self.file_modules.len() => {
                self.file_modules[info.file.0 as usize].clone()
            }
            Owner::Package(p) => self.pkg_path(p),
            // The companion of a top-level opaque type, which another module's pickle nests in its
            // file's `$package` object as scalac does: a class of the package, as teq's build of
            // that module names it (a jar's, scalac's, keeps scalac's name).
            Owner::Class(_) if self.input.reach.product_classes.get(c.idx()).copied().unwrap_or(false) && syms.opaque_companion_package(c, self.input.interner).is_some() => {
                self.pkg_path(syms.opaque_companion_package(c, self.input.interner).unwrap_or(crate::symbols::ROOT_PKG))
            }
            Owner::Class(o) => {
                let mut outer = self.class_names.get(o.idx()).cloned().unwrap_or_else(|| self.make_class_name(o));
                if !outer.ends_with('$') {
                    outer.push('$');
                }
                outer
            }
            Owner::Local => self.pkg_path(self.input.file_pkgs[info.file.0 as usize]),
        };
        out.push_str(&encode(text));
        let inner_object = info.inner_object.is_some();
        match info.kind {
            ClassKind::Object => out.push('$'),
            // The loader names a jar's inner object beside a companion class `R$` already; a
            // source identifier ending in `$` (``object `R$` ``) takes the suffix as any other.
            _ if inner_object && !(out.ends_with('$') && self.input.reach.library_classes.get(c.idx()).copied().unwrap_or(false)) => out.push('$'),
            // A given object of a package or of objects is scalac's module class `Owner$x$`.
            ClassKind::GivenImpl if self.given_object_classes.contains_key(&c) || self.inner_given_object_classes.contains_key(&c) => out.push('$'),
            ClassKind::GivenImpl if self.given_class_classes.contains_key(&c) => {}
            ClassKind::GivenImpl => out.push_str("$G"),
            ClassKind::EnumCase if info.singleton.is_some() => out.push_str("$V"),
            // Two local classes of one name in one file would share a binary name.
            _ if info.owner == Owner::Local && info.kind != ClassKind::Anon => match self.input.syms.product_position(info.file, info.def) {
                Some((token, offset)) => out.push_str(&format!("${}_{}", crate::source::tag_text(token), offset)),
                None => out.push_str(&format!("${}", self.input.prog.position(info.file, info.span.start))),
            },
            _ => {}
        }
        out
    }

    /// The given objects of a package or of objects: a given without parameters with a class of
    /// its own (`SymInfo::impl_class`), its owner a package or an object of a package or of
    /// objects. scalac writes each as a module class, `Show$given_Show_Int$` with `MODULE$`, and
    /// its owner declares it as a static field, where teq's model is a lazy given holding the one
    /// instance of its class; the JVM maps that model to scalac's layout, a product's given
    /// object read back as teq's model too. A given object of a class or trait instance is
    /// scalac's inner module class `Owner$x$`, its getter answering it, as teq lays out an inner
    /// object.
    ///
    /// A given class (a given with parameters and a class of its own) of a package or of objects
    /// is scalac's class `Owner$name` with the def `name(params): Owner$name` that makes one, where
    /// teq's calls make the class directly and write no def: the JVM writes the def for code teq
    /// did not compile. One of a class, trait or enum instance is the same, its constructor taking the
    /// instance first (`$outer`), the def a trait's default method with its static `name$`.
    fn static_given_objects(&mut self) {
        let syms = self.input.syms;
        for (i, info) in syms.classes.iter().enumerate() {
            if info.kind != ClassKind::GivenImpl {
                continue;
            }
            let c = ClassId(i as u32);
            // An instance owner as the typer's outer instance is (`Worker::outer_class`): a class, a
            // trait or an enum.
            let instance = matches!(info.owner, Owner::Class(o) if matches!(syms.class(o).kind, ClassKind::Class | ClassKind::Trait | ClassKind::Enum));
            let given = match info.owner {
                Owner::Class(o) if syms.class(o).kind == ClassKind::Object && self.statically_placed(o) => syms.class(o).members.get(&info.name).copied(),
                Owner::Class(o) if instance => syms.class(o).members.get(&info.name).copied(),
                Owner::Package(p) => syms.pkg(p).entries.get(&info.name).and_then(|e| e.term),
                _ => None,
            };
            let Some(g) = given.filter(|&g| syms.sym(g).kind == SymKind::Given && syms.sym(g).impl_class == Some(c)) else { continue };
            let parameterless = syms.sym(g).sig.as_ref().map_or(false, |s| s.tparams.is_empty() && s.clauses.is_empty());
            if parameterless && instance {
                self.inner_given_object_classes.insert(c, g);
                self.inner_given_objects.insert(g, c);
                continue;
            }
            if parameterless {
                self.given_object_classes.insert(c, g);
                self.given_objects.insert(g, c);
            } else {
                self.given_class_classes.insert(c, g);
                self.given_classes.insert(g, c);
            }
        }
    }

    /// The plain constructor parameters of the program's classes that no JVM class but their own
    /// reads, as scalac's `Getters` gives no getter to (`noGetterNeeded`) and its backend makes
    /// a `private final` field of: an immutable parameter of no `val`, of a class (no trait,
    /// object or value class) that is no case class, whose `copy` reads its later clauses where
    /// it is called (another module's code among them), defines no inline member, which an
    /// expansion elsewhere would read, no member class (an inner or given class, an object,
    /// which would read it through its outer instance) and whose code makes no local class (an
    /// anonymous class, a function as a SAM's class), which would read it through the instance
    /// it captures; and no code of another class or of the top level reads it. Decided by the
    /// module's own code, so that every build of the module lays it out alike.
    fn private_params(&mut self) {
        let syms = self.input.syms;
        let prog = self.input.prog;
        let mut has_members: FxMap<ClassId, ()> = FxMap::default();
        for info in syms.classes.iter() {
            if let Owner::Class(o) = info.owner {
                has_members.insert(o, ());
            }
        }
        let roots_of = |tc: &crate::tir::TClass| {
            let mut roots: Vec<TExprId> = Vec::new();
            for &f in tc.methods.iter().chain(&tc.ctors) {
                roots.extend(prog.funs[f.idx()].body);
                roots.extend(prog.funs[f.idx()].defaults.iter().flatten().copied());
            }
            roots.extend(tc.ctor_defaults.iter().flatten().copied());
            for init in &tc.init {
                match *init {
                    TInit::Field(_, e) | TInit::Stmt(e) => roots.push(e),
                    TInit::Parent(_, call) => roots.extend(prog.expr_list(call.args).iter().copied()),
                }
            }
            if let Some(l) = tc.parent_args {
                roots.extend(prog.expr_list(l).iter().copied());
            }
            roots
        };
        let local = |k: ClassId| syms.class(k).owner == Owner::Local;
        for tc in &prog.classes {
            let c = tc.id;
            let info = syms.class(c);
            if info.kind != ClassKind::Class || info.value_class || info.def.is_none() || info.js != JsKind::Scala || info.mods & crate::ast::mods::CASE != 0 || has_members.contains_key(&c) {
                continue;
            }
            if info.member_order.iter().any(|&m| syms.sym(m).mods & crate::ast::mods::INLINE != 0) {
                continue;
            }
            let makes_local_class = roots_of(tc).iter().any(|&r| {
                prog.descendants(r).any(|e| match prog.expr(e) {
                    TExpr::New(k, _) => local(k),
                    TExpr::NewVia(s, _) => matches!(syms.sym(s).owner, Owner::Class(k) if local(k)),
                    _ => false,
                })
            });
            if makes_local_class {
                continue;
            }
            for &p in &tc.ctor_params[tc.captures.min(tc.ctor_params.len())..] {
                let m = syms.sym(p).mods;
                if syms.sym(p).kind == SymKind::Val && m & crate::ast::mods::FIELD == 0 && m & crate::ast::mods::PRIVATE != 0 {
                    self.private_params.insert(p, ());
                }
            }
        }
        if self.private_params.is_empty() {
            return;
        }
        // A read by the code of another class or of the top level keeps the accessor.
        let mut read_elsewhere: Vec<SymId> = Vec::new();
        let scan = |at: Option<ClassId>, root: TExprId, out: &mut Vec<SymId>, params: &FxMap<SymId, ()>| {
            for e in prog.descendants(root) {
                if let TExpr::Field(_, s) | TExpr::CallMethod(_, s, _) = prog.expr(e) {
                    if params.contains_key(&s) && at.map_or(true, |k| syms.sym(s).owner != Owner::Class(k)) {
                        out.push(s);
                    }
                }
            }
        };
        for tc in &prog.classes {
            for root in roots_of(tc) {
                scan(Some(tc.id), root, &mut read_elsewhere, &self.private_params);
            }
        }
        for &f in prog.top_funs.iter() {
            for root in prog.funs[f.idx()].body.into_iter().chain(prog.funs[f.idx()].defaults.iter().flatten().copied()) {
                scan(None, root, &mut read_elsewhere, &self.private_params);
            }
        }
        for &(_, e) in prog.top_vals.iter() {
            scan(None, e, &mut read_elsewhere, &self.private_params);
        }
        for s in read_elsewhere {
            self.private_params.remove(&s);
        }
    }

    /// A class the jars define: one read from them, or one of the builtin layer under a binary
    /// name the class path holds (`scala/Tuple2`, `scala/Product`). The jar's is what runs.
    pub fn linked_class(&self, c: ClassId) -> bool {
        self.input.link.jar_classes.contains_key(&c) || self.held.get(c.idx()).copied().unwrap_or(false)
    }

    /// Reads what linking needs of the class path before the emitter threads start.
    fn read_classpath(&mut self, cp: &mut crate::classpath::Classpath) {
        self.boxes_runtime = cp.class_named("scala/runtime/BoxesRunTime").is_some();
        let reach = self.input.reach;
        // A class of a source of the build is the one, written whatever the class path holds
        // under its name (another source's product, a stale twin in the module's directory).
        let syms = self.input.syms;
        let sources = self.input.sources;
        let std_class = |i: usize| sources.get(syms.class(ClassId(i as u32)).file.0 as usize).map_or(true, |s| s.is_std);
        self.held = (0..self.class_names.len())
            .map(|i| (reach.classes[i] || self.tclass_of[i] != u32::MAX) && std_class(i) && cp.class_named(&self.class_names[i]).is_some())
            .collect();
        // The builtin layer's classes the class path holds (`ClassTag`, `Tuple2`) are called as
        // the jar declares them too: scala-library's `ClassTag` is an interface.
        let held: Vec<ClassId> = (0..self.held.len()).filter(|&i| self.held[i] && reach.classes[i]).map(|i| ClassId(i as u32)).collect();
        // And the jar classes a class of the program inherits from, whose methods its bridges
        // are made against (`Gen::jar_bridges`).
        let jar_classes = self.input.link.jar_classes;
        let mut ancestors: Vec<ClassId> = Vec::new();
        for i in (0..self.tclass_of.len()).filter(|&i| self.tclass_of[i] != u32::MAX && reach.classes[i]) {
            for &(b, _) in self.input.syms.class(ClassId(i as u32)).base_types.iter().skip(1) {
                if jar_classes.contains_key(&b) {
                    ancestors.push(b);
                }
            }
        }
        // And the jar value classes, which a type may name where the walk met none of their
        // members: a box is made and opened as the class file declares its constructor and
        // accessor.
        let mut value_classes: Vec<ClassId> = jar_classes.keys().copied().filter(|&c| self.input.syms.class(c).value_class).collect();
        value_classes.sort_unstable();
        for c in reach.linked.iter().copied().chain(held).chain(ancestors).chain(value_classes) {
            if self.class_files.contains_key(&c) {
                continue;
            }
            let Some(f) = cp.class_named(&self.class_names[c.idx()]) else { continue };
            if let Ok(cf) = cp.class_file(f) {
                if cp.is_scala2(f) {
                    let name = self.class_names[c.idx()].clone();
                    if let Some(raw) = cf.scala_sig.clone().or_else(|| owning_pickle(cp, &name)) {
                        self.scala2_pickles.insert(c, std::sync::Arc::new(raw));
                    }
                }
                self.class_files.insert(c, cf);
            }
            // A constructor's default getters are its companion's, which only a top-level class
            // forwards to statically.
            let ctor_defaults = self.input.syms.class(c).ctor.iter().any(|cl| cl.params.iter().any(|p| p.has_default));
            if let Some(k) = self.input.syms.class(c).companion.filter(|k| ctor_defaults && !self.class_files.contains_key(k)) {
                if let Some(cf) = cp.class_named(&self.class_names[k.idx()]).and_then(|f| cp.class_file(f).ok()) {
                    self.class_files.insert(k, cf);
                }
            }
            if self.input.syms.class(c).value_class {
                let companion = format!("{}$", self.class_names[c.idx()]);
                if let Some(cf) = cp.class_named(&companion).and_then(|f| cp.class_file(f).ok()) {
                    self.vc_companions.insert(c, cf);
                }
            }
        }
    }

    /// Whether class `c` of the program is written as a class file.
    pub fn emits(&self, c: ClassId) -> bool {
        let info = self.input.syms.class(c);
        self.tclass_of[c.idx()] != u32::MAX
            && !matches!(info.kind, ClassKind::Opaque | ClassKind::Builtin)
            && info.js == JsKind::Scala
            && self.input.reach.classes[c.idx()]
            && self.jvm_class(c).is_none()
            && !self.is_function_evidence(c)
            && !self.linked_class(c)
    }

    /// `<:<` and `=:=` of package `scala`, whose evidence is the identity function: they are
    /// `scala.Function1` in the class files, and nothing is emitted for them.
    fn is_function_evidence(&self, c: ClassId) -> bool {
        let info = self.input.syms.class(c);
        info.kind == ClassKind::Trait
            && matches!(self.input.interner.get(info.name), "<:<" | "=:=")
            && matches!(info.owner, Owner::Package(p) if self.input.interner.get(self.input.syms.pkg(p).name) == "scala")
    }

    /// `@jvmClass("java/util/LinkedHashMap")` on a class of the standard library: the JVM class
    /// its values are instances of. Nothing is emitted for such a class.
    pub fn jvm_class(&self, c: ClassId) -> Option<String> {
        let info = self.input.syms.class(c);
        let ast = &self.input.asts[info.file.0 as usize];
        let def = ast.def(info.def?);
        let a = def.annots.iter().find(|a| a.name == crate::names::JVM_CLASS)?;
        ast.annot_args(a).first().map(|&s| ast.str(s).to_string())
    }

    /// A member whose field is filled when first read: a lazy val, a given, an object nested
    /// in a class or trait.
    pub fn is_lazy_member(&self, s: SymId) -> bool {
        let info = self.input.syms.sym(s);
        info.mods & mods::LAZY != 0 || info.kind == SymKind::Given
    }

    /// The class of a binary name, the first of the program's classes so named.
    pub fn class_named(&self, name: &str) -> Option<ClassId> {
        let named = self.named.get_or_init(|| {
            let mut named = FxMap::default();
            for (i, n) in self.class_names.iter().enumerate() {
                named.entry(n.clone()).or_insert(ClassId(i as u32));
            }
            named
        });
        named.get(name).copied()
    }

    pub fn is_interface(&self, c: ClassId) -> bool {
        self.input.syms.class(c).kind == ClassKind::Trait
    }

    /// The enum that the case class `case` belongs to.
    pub fn enum_of_case(&self, case: ClassId) -> Option<ClassId> {
        let syms = self.input.syms;
        let info = syms.class(case);
        if info.kind != ClassKind::EnumCase {
            return None;
        }
        let Owner::Class(companion) = info.owner else { return None };
        syms.class(companion).companion
    }

    /// The members of `c`, the alternatives of an overloaded name apart, in the order they
    /// are declared: the members map is keyed by name, and the ids of names follow the order
    /// the parser's threads interned them in, which no output may depend on. A member the
    /// map alone holds stands after the declared ones, by its place in the source.
    pub fn members_in_order(&self, c: ClassId) -> Vec<SymId> {
        let syms = self.input.syms;
        let info = syms.class(c);
        let mut listed: FxMap<SymId, ()> = FxMap::default();
        let mut out: Vec<SymId> = Vec::with_capacity(info.member_order.len());
        for &m in &info.member_order {
            if listed.insert(m, ()).is_none() {
                out.push(m);
            }
        }
        let mut rest: Vec<SymId> = info
            .members
            .values()
            .flat_map(|&m| syms.alternatives(m).map_or(vec![m], |a| a.to_vec()))
            .filter(|m| !listed.contains_key(m))
            .collect();
        rest.sort_by(|&a, &b| crate::emit::layout::compare_syms(syms, self.input.interner, a, b));
        rest.dedup();
        out.extend(rest);
        out
    }

    /// An object of the std standing for a JDK class's statics (`@jvmClass`): its members are
    /// the class's static methods and fields.
    pub fn jdk_statics(&self, c: ClassId) -> bool {
        self.input.syms.class(c).kind == ClassKind::Object && self.jvm_class(c).is_some()
    }

    /// An enum whose superclass is `java.lang.Enum`.
    pub fn is_java_enum(&self, e: ClassId) -> bool {
        let info = self.input.syms.class(e);
        info.kind == ClassKind::Enum && info.superclass.map_or(false, |s| self.class_names[s.idx()] == "java/lang/Enum")
    }

    /// A case class, not a case object: a case object is its own module class, with scalac's
    /// `Mirror.Singleton`, and has no companion of a case class's (`apply`, `unapply`) nor `copy`.
    pub fn is_case_class(&self, c: ClassId) -> bool {
        let info = self.input.syms.class(c);
        info.mods & mods::CASE != 0 && info.singleton.is_none() && info.kind != ClassKind::Object
    }

    /// A case object: an object, with the product members of a case class of no parameters.
    pub fn is_case_object(&self, c: ClassId) -> bool {
        let info = self.input.syms.class(c);
        info.mods & mods::CASE != 0 && info.kind == ClassKind::Object
    }
}

/// Below this many units to emit, a `Gen` keeps its erasures in a map (`Gen::sparse`).
const SPARSE_UNITS: usize = 64;

pub fn emit(input: Input, classpath: &mut crate::classpath::Classpath, kept: Option<&mut kept::Kept>) -> Output {
    let mut lap = crate::measure::Lap::begin(crate::measure::Pass::Emit);
    let mut cx = Cx::new(input, &mut lap);
    cx.read_classpath(classpath);
    lap.done("class path read", cx.class_files.len());
    let held_by_classpath = |name: &str| classpath.class_named(name).is_some();
    let prog = cx.input.prog;
    let syms = cx.input.syms;
    let reach = cx.input.reach;
    let owned = |f: FileId| cx.input.sources[f.0 as usize].is_std || cx.input.owned.map_or(true, |o| o.get(f.0 as usize).copied().unwrap_or(true));
    let mut units: Vec<Unit> = Vec::new();
    // An anonymous class an inline expansion made for its caller is the caller's file's: with
    // `--own` it is written with the caller, not left to the inline body's build.
    for (i, tc) in prog.classes.iter().enumerate() {
        if cx.emits(tc.id) && owned(syms.class(tc.id).module_file()) {
            units.push(Unit::Class(i));
        }
    }
    // A module class whose companion class takes its static forwarders is written with it.
    units.retain(|u| match *u {
        Unit::Class(i) => cx.forwarding_class(prog.classes[i].id).is_none(),
        Unit::File(_) => true,
    });
    for f in 0..cx.input.sources.len() {
        let has_funs = cx.file_funs[f].iter().any(|&fun| reach.funs[fun.idx()]);
        let has_vals = cx.file_group[f] != u32::MAX && {
            let (_, vals) = &cx.layout.file_groups[cx.file_group[f] as usize];
            reach.files.get(f).copied().unwrap_or(false) || vals.iter().any(|&(s, _)| reach.vals[s.idx()])
        };
        // A file of top-level `export` clauses has its forwarders in its `<file>$package`, and a
        // file's top-level given class its def there.
        let has_exports = reach.exports.files.contains_key(&FileId(f as u32));
        let has_given_classes = cx.given_classes.iter().any(|(&g, &k)| syms.sym(g).file.0 as usize == f && matches!(syms.sym(g).owner, Owner::Package(_)) && cx.emits(k));
        if (has_funs || has_vals || has_exports || has_given_classes) && owned(FileId(f as u32)) {
            units.push(Unit::File(FileId(f as u32)));
        }
    }
    lap.done("units chosen", units.len());
    let origin = |unit: Unit| match unit {
        Unit::Class(i) => {
            let c = prog.classes[i].id;
            (Some(syms.class(c).module_file()), Some(c))
        }
        Unit::File(f) => (Some(f), None),
    };
    let mut usage = kept::Use::default();
    let (snapshot, keep, owns) = match kept.as_deref() {
        Some(k) => {
            let (snapshot, keep, owns) = k.keepable(&cx, &units, &mut usage);
            lap.done("facts compared", usage.kept);
            (Some(snapshot), keep, owns)
        }
        None => (None, vec![false; units.len()], Vec::new()),
    };
    let shared = SyncCx(&cx);
    let workers = if prog.exprs.len() < 100_000 { 1 } else { crate::workers() };
    // Each unit's class files, errors and function arities, by its index in `units`.
    let emit_units = |which: &[usize]| -> Vec<(usize, Vec<Emitted>, Vec<String>, u64)> {
        let next = AtomicUsize::new(0);
        let work = || {
            let shared = &shared;
            let mut g = gen::Gen::new(shared.0);
            g.sparse = which.len() < SPARSE_UNITS;
            let mut done = Vec::new();
            loop {
                let k = next.fetch_add(1, Ordering::Relaxed);
                let Some(&i) = which.get(k) else { break };
                let unit = units[i];
                let (source, of) = origin(unit);
                g.function_arities = 0;
                let (name, bytes) = g.emit_unit(unit);
                let mut classes = vec![Emitted { name, bytes: bytes.into(), source, of }];
                classes.extend(g.extra_classes.drain(..).map(|(name, bytes)| Emitted { name, bytes: bytes.into(), source, of }));
                // The two arities every build declares, whichever memo answered the unit's.
                done.push((i, classes, std::mem::take(&mut g.errors), g.function_arities | 0b11));
            }
            done
        };
        if workers == 1 || which.len() <= 1 {
            return work();
        }
        // The erasure expands applied aliases (`Gen::alias_expansion`), making types: the type
        // store takes the emitters' inserts serialised while they run, as the typer's workers'.
        let exclusive = cx.input.types.is_exclusive();
        cx.input.types.set_exclusive(false);
        let out = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..workers.min(which.len())).map(|_| crate::alloc::spawn_in(scope, &work)).collect();
            handles.into_iter().flat_map(|h| h.join().expect("emitter thread panicked")).collect()
        });
        cx.input.types.set_exclusive(exclusive);
        out
    };
    let fresh: Vec<usize> = (0..units.len()).filter(|&i| !keep[i]).collect();
    let mut per_unit: Vec<Option<(Vec<Emitted>, u64)>> = (0..units.len()).map(|_| None).collect();
    let mut errors = Vec::new();
    for (i, done, errs, used) in emit_units(&fresh) {
        errors.extend(errs);
        per_unit[i] = Some((done, used));
    }
    lap.done("units, in parallel", fresh.len());
    lap.count("emitter threads", if workers == 1 { 1 } else { workers.min(fresh.len().max(1)) });
    if let Some(k) = kept.as_deref() {
        if let Some(why) = usage.why {
            lap.count(why, 1);
        }
        if kept::checked() && usage.kept > 0 {
            let kept_units: Vec<usize> = (0..units.len()).filter(|&i| keep[i]).collect();
            for (i, done, errs, used) in emit_units(&kept_units) {
                let (classes, arities) = k.get(&cx, units[i]);
                let same = errs.is_empty()
                    && used == arities
                    && done.len() == classes.len()
                    && done.iter().zip(classes).all(|(a, b)| a.name == b.name && a.bytes == b.bytes && a.source == b.source && a.of == b.of);
                if !same {
                    let names = |cs: &[Emitted]| cs.iter().map(|c| format!("{} ({} bytes)", c.name, c.bytes.len())).collect::<Vec<_>>().join(", ");
                    panic!(
                        "the kept class files of {:?} differ from its emission now: kept {} arities {:#x}, emitted {} arities {:#x} errors {:?}",
                        units[i],
                        names(classes),
                        arities,
                        names(&done),
                        used,
                        errs
                    );
                }
            }
        }
    }
    let mut classes = Vec::with_capacity(units.len() + 8);
    let mut arities = 0u64;
    for (i, &unit) in units.iter().enumerate() {
        let (done, used) = match &per_unit[i] {
            Some((done, used)) => (done.as_slice(), *used),
            None => kept.as_deref().expect("a kept unit's store").get(&cx, unit),
        };
        classes.extend(done.iter().cloned());
        arities |= used;
    }
    if kept.is_some() {
        lap.done("units kept", usage.kept);
    }
    let standalone = |(name, bytes): (String, Vec<u8>)| Emitted { name, bytes: bytes.into(), source: None, of: None };
    let mut main_class = None;
    let mut main_classes = Vec::new();
    if let Some(main) = prog.main {
        let mut g = gen::Gen::new(&cx);
        g.sparse = true;
        classes.push(standalone(g.emit_launcher(main, prog.main_object)));
        if let Some(entry) = g.emit_main_class(main, &held_by_classpath) {
            classes.push(standalone(entry));
        }
        errors.extend(std::mem::take(&mut g.errors));
        arities |= g.function_arities;
        main_class = Some(runtime::LAUNCHER.to_string());
    }
    for &(main, object) in cx.input.all_mains.unwrap_or(&[]) {
        let mut g = gen::Gen::new(&cx);
        g.sparse = true;
        // An object's own `main` comes without the object, an inherited one with it; a `@main`
        // method of an object is neither: the launcher runs the class named after the method.
        let object = match (object, syms.sym(main).owner) {
            (Some(c), _) => Some(c),
            (None, Owner::Class(c)) if !syms.sym(main).is_main => Some(c),
            _ => None,
        };
        let file = object.map_or(syms.sym(main).file, |c| syms.class(c).file);
        if !owned(file) {
            continue;
        }
        match object {
            // The mirror or companion class of a top-level object carries the static forwarder.
            Some(c) if matches!(syms.class(c).owner, Owner::Package(_)) && cx.emits(c) => {
                let holder = cx.class_names[c.idx()].trim_end_matches('$').replace('/', ".");
                main_classes.push((file, holder));
            }
            Some(_) => {}
            // A package's own `main`: the mirror class of its file's module carries the forwarder.
            None if !syms.sym(main).is_main => {
                let holder = cx.file_modules[file.0 as usize].trim_end_matches('$').replace('/', ".");
                main_classes.push((file, holder));
            }
            None => {
                if let Some((name, bytes)) = g.emit_main_class(main, &held_by_classpath) {
                    main_classes.push((file, name.replace('/', ".")));
                    classes.push(Emitted { name, bytes: bytes.into(), source: Some(file), of: None });
                }
            }
        }
        errors.extend(std::mem::take(&mut g.errors));
        arities |= g.function_arities;
    }
    arities |= 0b11;
    for n in 0..63 {
        let name = format!("scala/Function{}", n);
        if arities & (1 << n) != 0 && !held_by_classpath(&name) && !classes.iter().any(|c| c.name == name) {
            classes.push(standalone(runtime::function_interface(n, cx.input.output_version)));
        }
    }
    lap.done("launchers and interfaces", 0);
    classes.sort_by(|a, b| a.name.cmp(&b.name));
    // Two classes under one binary name would be two jar entries, one of them unreachable.
    let mut twice: Vec<String> = Vec::new();
    classes.dedup_by(|later, kept| {
        let same = later.name == kept.name;
        if same {
            twice.push(later.name.clone());
        }
        same
    });
    errors.extend(twice.into_iter().map(|n| format!("two classes are named {}", n)));
    errors.sort();
    errors.dedup();
    // An object whose construction runs code, among them a companion its class's unit writes
    // (`forwarding_class`): a split build over these products touches it before `new` of the
    // class, as the whole build does (`touched_companion`).
    let object_of = |e: &Emitted| -> Option<ClassId> {
        let c = e.of?;
        let o = if e.name == cx.class_names[c.idx()] { c } else { cx.input.syms.class(c).companion.filter(|o| e.name == cx.class_names[o.idx()])? };
        (cx.input.syms.class(o).kind == ClassKind::Object && cx.layout.has_body(o)).then_some(o)
    };
    let inits: Vec<String> = classes.iter().filter(|e| object_of(e).is_some()).map(|e| e.name.clone()).collect();
    lap.done("sorted", classes.len());
    lap.count("class file bytes", classes.iter().map(|c| c.bytes.len()).sum());
    if let Some(k) = kept {
        match snapshot.filter(|_| errors.is_empty()) {
            Some(snapshot) => k.store(&cx, &units, per_unit, &owns, snapshot),
            None => k.clear(),
        }
        lap.done("store updated", usage.kept + usage.emitted);
    }
    Output { classes, main_class, errors, main_classes, inits, kept: usage }
}

/// Writes the classes as a jar when the path ends in `.jar`, else as a directory tree.
pub fn write(path: &str, out: &Output) -> std::io::Result<()> {
    if path.ends_with(".jar") {
        let entries: Vec<(String, Vec<u8>)> = out.classes.iter().map(|c| (c.name.clone(), c.bytes.to_vec())).collect();
        return classfile::write_jar(path, out.main_class.as_deref(), &entries);
    }
    // A directory tree of tens of thousands of class files (a module whose derivations copy a
    // library's quoted classes per expansion) is written on every worker thread, each
    // directory made once.
    let root = std::path::Path::new(path);
    let dirs: std::collections::BTreeSet<std::path::PathBuf> = out.classes.iter().filter_map(|c| root.join(format!("{}.class", c.name)).parent().map(|p| p.to_path_buf())).collect();
    for dir in &dirs {
        std::fs::create_dir_all(dir)?;
    }
    let workers = crate::workers().min(out.classes.len().max(1));
    let next = std::sync::Mutex::new(out.classes.iter());
    let failed: std::sync::Mutex<Option<std::io::Error>> = std::sync::Mutex::new(None);
    let work = || loop {
        let Some(c) = next.lock().unwrap().next() else { break };
        if let Err(e) = std::fs::write(root.join(format!("{}.class", c.name)), c.bytes.as_slice()) {
            failed.lock().unwrap().get_or_insert(e);
            break;
        }
    };
    if workers == 1 {
        work();
    } else {
        // Joined through their handles, so that no worker drops the last reference to its own
        // handle after its memory went back to the allocator (`alloc.rs`).
        std::thread::scope(|scope| {
            let handles: Vec<_> = (0..workers).map(|_| crate::alloc::spawn_in(scope, &work)).collect();
            for handle in handles {
                handle.join().expect("class file writer thread panicked");
            }
        });
    }
    match failed.into_inner().unwrap() {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

/// In link mode, scala-library's `StringContext.s` and `raw`, which have no bytecode, each with
/// the definition of the runtime (`std/jvm_scala_library.scala`) a call of it becomes.
pub fn interpolators(typer: &mut crate::typer::Worker, runtime: PkgId) -> Vec<(SymId, SymId)> {
    let Some(c) = typer.class_at(&["scala", "StringContext"]) else { return Vec::new() };
    typer.complete_class(c);
    let mut out = Vec::new();
    for (member, helper) in [("s", "interpolateS"), ("raw", "interpolateRaw")] {
        let (m, h) = (typer.interner.intern(member), typer.interner.intern(helper));
        typer.demand_std(runtime, h, crate::stdindex::TERM);
        let found = typer.syms.class(c).members.get(&m).copied().zip(typer.syms.pkg(runtime).entries.get(&h).and_then(|e| e.term));
        if let Some((m, h)) = found {
            let alts: Vec<SymId> = typer.syms.alternatives(m).map_or_else(|| vec![m], |a| a.to_vec());
            out.extend(alts.into_iter().filter(|&a| typer.syms.sym(a).kind == SymKind::Def).map(|a| (a, h)));
        }
    }
    out
}

/// The pickle of a Scala 2 jar's nested class, which its top-level class holds: the class file of
/// the longest prefix of `name` cut at a `$` whose signature has a class of this binary name
/// (`p/A$B` for `p/A$B$T`, a top-level object written `` `A$B` ``, before `p/A`).
fn owning_pickle(cp: &mut crate::classpath::Classpath, name: &str) -> Option<Vec<u8>> {
    let top = name.rfind('/').map_or(0, |i| i + 1);
    let simple = &name[top..];
    let cuts: Vec<usize> = simple.char_indices().filter(|&(i, ch)| ch == '$' && i > 0).map(|(i, _)| top + i).collect();
    for &cut in cuts.iter().rev() {
        let Some(f) = cp.class_named(&name[..cut]) else { continue };
        let Some(raw) = cp.class_file(f).ok().and_then(|t| t.scala_sig.clone()) else { continue };
        let describes = crate::scala2::pickle::Pickle::parse(crate::scala2::pickle::decode_signature(raw.clone())).map_or(false, |p| p.class_named(simple).is_some());
        if describes {
            return Some(raw);
        }
    }
    None
}
