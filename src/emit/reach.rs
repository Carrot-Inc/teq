//! What the program reaches from its entry point and its exports: the classes, methods, functions
//! and values the emitter writes. Anything else is left out of the output.
//!
//! A member call is dispatched by name at run time. A call of `m` through a receiver of static
//! class `O` can land on any reached class `D` below `O`, which takes its `m` from one of its
//! ancestors, so `m` is kept in every ancestor of every such `D`; the ancestors are one family
//! since a trait may implement what an unrelated trait declares (`IterableOps.filter` for
//! `SetOps.filter`). The runtime calls `toString`, `equals`, `hashCode` and `foreach` on any value
//! and `iterator`, `hasNext`, `next` and `drop` on the scrutinee of a sequence pattern: every
//! reached class keeps those. The indexes only hold reached classes, so the work is linear in what
//! is reached.
//!
//! The bodies of the standard library's methods are typed when the walk reaches them: the walk
//! stops when it needs one, the typer supplies it, and the walk goes on with the new IR.

use super::layout::{given_class_initialises_file, initialises_file, is_eager_top_val, Layout};
use crate::ast::{mods, ListRef};
use crate::intern::{FxMap, Interner, Name};
use crate::names;
use crate::source::FileId;
use crate::symbols::*;
use crate::tir::*;
use crate::typer::check::DeferredBody;
use crate::typer::Worker;
use crate::types::*;

pub struct Reach {
    pub classes: Vec<bool>,
    pub funs: Vec<bool>,
    /// The methods that are called through a bridge (`TClass::bridges`), by symbol.
    pub bridged: Vec<bool>,
    /// Top-level vals that stand on their own: lazy vals, givens, constants and compiler-made vals.
    pub vals: Vec<bool>,
    /// Files whose eager top-level vals run in the file's initialiser.
    pub files: Vec<bool>,
    /// Per symbol: a reached top-level definition of a file with an initialiser whose access
    /// runs the initialiser first (`layout::initialises_file`); a std file's initialisers are
    /// pure, and its triggers are the reads of its eager vals alone.
    pub triggers: Vec<bool>,
    /// Per class: a given class of such a file, whose instance's making runs the initialiser
    /// first (`layout::given_class_initialises_file`).
    pub trigger_classes: Vec<bool>,
    /// Per symbol: a top-level def that runs outside the initialised part of its file without a
    /// call there, as a function value made where the file is not known to be initialised
    /// (`(a) => f(a)`, written `f`) or as an export, which checks its file's initialiser itself.
    pub forwarded: Vec<bool>,
    pub imports: Vec<bool>,
    /// The library methods whose bodies the walk asked the typer for.
    pub asked: Vec<SymId>,
    /// Members read through a receiver, by symbol, other than through `this` in the body of
    /// their own class; a constructor parameter that is not among them needs no field, unless
    /// the body reads it (`init_read`) and runs apart from the constructor.
    pub fields_read: Vec<bool>,
    pub init_read: Vec<bool>,
    /// Under `--release`, the member names JavaScript code of the program uses by name: the
    /// `js.Dynamic` selections, the property accesses of `@js` templates outside the standard
    /// library, and the members of the classes that an exported definition takes or returns, or
    /// that such a class takes or returns in turn.
    pub js_names: Vec<String>,
    /// The `@js` templates the reached code writes and the globals it names (`JsGlobal`, a
    /// selection of the global scope), each once: what a local of the output may meet besides
    /// the definitions (`scope::Renderable`).
    pub templates: Vec<StrRef>,
    pub globals: Vec<Name>,
    /// Whether a reached `classOf` needs every class to carry its qualified name.
    pub uses_class_of: bool,
    /// Whether a reached `productElementName` needs every case class to carry its field names.
    pub uses_element_names: bool,
    /// Whether a reached `isAssignableFrom` or `isInstance` needs a trait's `classOf` to carry
    /// its number (`$traitClass`), which both read.
    pub uses_assignable_from: bool,
    /// Per symbol: whether it is an abstract member, as the typer sees it once the walk has
    /// entered every std file the program needs.
    pub abstract_syms: Vec<bool>,
    /// Per symbol: whether the class that declares it has to declare it on the JVM, where a call
    /// names the member of the class that declares it: a member called through a receiver
    /// (`invokeinterface` and `invokevirtual` resolve against the declaration), and in link and
    /// product output every member the output keeps for later callers (`link_root_members`).
    /// An abstract member outside it is left out of the class file (`jvm::classes`).
    pub declared: Vec<bool>,
    /// In link mode, the jar classes the walk met, as classes or as owners of members: the
    /// ones whose class files the JVM backend reads the descriptors of.
    pub linked: Vec<ClassId>,
    /// Per class, whether it came from a library on the class path.
    pub library_classes: Vec<bool>,
    /// Per class, whether it came from a directory of teq's products: another module's.
    pub product_classes: Vec<bool>,
    /// Those of them whose construction may run a body (not a companion scalac made).
    pub external_bodies: Vec<ClassId>,
    /// The inline expansions the walk met, with the definition whose body holds each.
    pub roots: Vec<(TExprId, Place)>,
    /// In link mode, the jar's `StringContext.s` and `raw` and the runtime's definitions that
    /// stand for them.
    pub interpolators: Vec<(SymId, SymId)>,
    /// The registrations of the reflectively instantiatable classes and objects, in the order of
    /// their classes, once a lookup of `scala.scalajs.reflect.Reflect` is reached.
    pub reflective: Vec<(ClassId, TExprId)>,
    /// Whether a lookup of `Reflect` is reached, registrations or none.
    pub reflective_reached: bool,
    /// In link mode, the forwarders of the program's `export` clauses, which the JVM writes as
    /// methods (`typer::export_plan`), made before the walk, whose roots their targets are.
    pub exports: std::sync::Arc<crate::typer::export_plan::JvmExports>,
}

impl Reach {
    fn empty() -> Reach {
        Reach {
            classes: Vec::new(),
            funs: Vec::new(),
            bridged: Vec::new(),
            vals: Vec::new(),
            files: Vec::new(),
            triggers: Vec::new(),
            trigger_classes: Vec::new(),
            forwarded: Vec::new(),
            imports: Vec::new(),
            asked: Vec::new(),
            fields_read: Vec::new(),
            init_read: Vec::new(),
            js_names: Vec::new(),
            templates: Vec::new(),
            globals: Vec::new(),
            uses_class_of: false,
            uses_element_names: false,
            uses_assignable_from: false,
            abstract_syms: Vec::new(),
            declared: Vec::new(),
            linked: Vec::new(),
            library_classes: Vec::new(),
            product_classes: Vec::new(),
            external_bodies: Vec::new(),
            roots: Vec::new(),
            interpolators: Vec::new(),
            reflective: Vec::new(),
            reflective_reached: false,
            exports: Default::default(),
        }
    }

    /// Covers the classes and symbols made after the walk (the pickles' writer completes library
    /// classes the program names), none of them reached.
    pub fn cover(&mut self, classes: usize, syms: usize) {
        for v in [&mut self.classes, &mut self.trigger_classes, &mut self.library_classes, &mut self.product_classes] {
            if v.len() < classes {
                v.resize(classes, false);
            }
        }
        for v in [&mut self.bridged, &mut self.vals, &mut self.triggers, &mut self.forwarded, &mut self.fields_read, &mut self.init_read, &mut self.abstract_syms, &mut self.declared] {
            if v.len() < syms {
                v.resize(syms, false);
            }
        }
    }
}

/// The definition a walked body belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Place {
    Fun(FunId),
    Class(ClassId),
    Val,
}

/// Types the bodies of the standard library that the program reaches and returns what it reaches.
/// `js_visible` asks for the names JavaScript code can see, which a release build keeps.
pub fn compute(typer: &mut Worker, array_seq: Option<ClassId>, js_visible: bool) -> Reach {
    compute_with_setup(typer, array_seq, js_visible).0
}

/// `compute`, with what the walk took from the typer before it began and looked up on the way,
/// which a kept reach's check walks with again (`check_kept`).
pub fn compute_with_setup(typer: &mut Worker, array_seq: Option<ClassId>, js_visible: bool) -> (Reach, Setup) {
    compute_unless(typer, array_seq, js_visible, &|| false).expect("a walk nothing stops")
}

/// `compute_with_setup`, given up where `stop` says so, which it asks between rounds and every
/// so many items of the walk: a session's settling walk, which a request takes the place of.
pub fn compute_unless(typer: &mut Worker, array_seq: Option<ClassId>, js_visible: bool, stop: &dyn Fn() -> bool) -> Option<(Reach, Setup)> {
    let p = typer.phase(crate::typer::profile::Phase::ReachWalk);
    let walked = compute_now(typer, array_seq, js_visible, stop);
    typer.phase_end(p);
    walked
}

/// What the walk takes from the typer before it begins, and the classes it looks up on the way.
#[derive(Clone)]
pub struct Setup {
    array_seq: Option<ClassId>,
    js_visible: bool,
    thrown: Vec<ClassId>,
    helpers: Option<JvmHelpers>,
    exported: Vec<String>,
    found_array_seq: Option<ClassId>,
    found_runtime_exceptions: Option<Vec<ClassId>>,
    /// The symbols the reflective registrations made, which every walk that reaches a lookup
    /// makes anew.
    registered_syms: usize,
    /// In link mode, the program's export forwarders, whose targets and qualifiers are roots.
    exports: std::sync::Arc<crate::typer::export_plan::JvmExports>,
}

impl Setup {
    pub fn array_seq(&self) -> Option<ClassId> {
        self.array_seq
    }

    pub fn js_visible(&self) -> bool {
        self.js_visible
    }

    pub fn registered_syms(&self) -> usize {
        self.registered_syms
    }
}

/// What the walk asks of the typer between its rounds: the typer itself, or for a kept reach's
/// check a stand-in over it that changes nothing and notes what it was asked (`Unchanged`).
trait Typing<'a> {
    fn w(&self) -> &Worker<'a>;
    fn array_seq_class(&mut self) -> Option<ClassId>;
    fn runtime_exceptions(&mut self) -> Vec<ClassId>;
    /// The errors stored for the reached members of a library body's anonymous class.
    fn take_poisoned(&mut self, reached: &[FunId]);
    fn demand_extenders(&mut self, c: ClassId);
    fn deferred_body(&mut self, s: SymId) -> Option<DeferredBody>;
    fn check_class(&mut self, c: ClassId);
    fn decode_templates_ahead(&mut self, owners: &[ClassId]) -> usize;
    fn bind_mixin_supers(&mut self);
    fn settle_dispatch_pending(&mut self);
    /// The roots renamed since the last walk, with their names now.
    fn renamed_roots(&mut self) -> Vec<(SymId, Name)>;
    fn reflective_registrations(&mut self) -> Vec<(ClassId, TExprId)>;
    fn complete_erased_jar_classes(&mut self);
}

impl<'a> Typing<'a> for Worker<'a> {
    fn w(&self) -> &Worker<'a> {
        self
    }

    fn array_seq_class(&mut self) -> Option<ClassId> {
        Worker::array_seq_class(self)
    }

    fn runtime_exceptions(&mut self) -> Vec<ClassId> {
        runtime_exceptions(self)
    }

    fn take_poisoned(&mut self, reached: &[FunId]) {
        for f in reached {
            if let Some(diags) = self.poisoned.remove(f) {
                self.diags.items.extend(diags);
            }
        }
    }

    fn demand_extenders(&mut self, c: ClassId) {
        self.demand_extenders_transitive(c);
    }

    fn deferred_body(&mut self, s: SymId) -> Option<DeferredBody> {
        Worker::deferred_body(self, s)
    }

    fn check_class(&mut self, c: ClassId) {
        if self.is_library_class(c) {
            self.check_library_class(c);
        } else {
            self.check_std_class(c);
        }
    }

    fn decode_templates_ahead(&mut self, owners: &[ClassId]) -> usize {
        Worker::decode_templates_ahead(self, owners)
    }

    fn bind_mixin_supers(&mut self) {
        Worker::bind_mixin_supers(self);
    }

    fn settle_dispatch_pending(&mut self) {
        Worker::settle_dispatch_pending(self);
    }

    fn renamed_roots(&mut self) -> Vec<(SymId, Name)> {
        std::mem::take(&mut self.renamed_roots).into_iter().map(|r| (r, self.output_name_of(r))).collect()
    }

    fn reflective_registrations(&mut self) -> Vec<(ClassId, TExprId)> {
        Worker::reflective_registrations(self)
    }

    fn complete_erased_jar_classes(&mut self) {
        Worker::complete_erased_jar_classes(self);
    }
}

/// The typer as a kept reach's check sees it: nothing typed, entered or bound, the classes the
/// kept reach's walk looked up given back, and every demand that would change the typer noted
/// as a divergence of the kept reach.
#[cfg(debug_assertions)]
struct Unchanged<'w, 'a> {
    w: &'w Worker<'a>,
    setup: &'w Setup,
    kept: &'w Reach,
    divergences: Vec<String>,
}

#[cfg(debug_assertions)]
impl<'w, 'a> Typing<'a> for Unchanged<'w, 'a> {
    fn w(&self) -> &Worker<'a> {
        self.w
    }

    fn array_seq_class(&mut self) -> Option<ClassId> {
        self.setup.found_array_seq.or(self.setup.array_seq)
    }

    fn runtime_exceptions(&mut self) -> Vec<ClassId> {
        self.setup.found_runtime_exceptions.clone().unwrap_or_else(|| self.setup.thrown.clone())
    }

    fn take_poisoned(&mut self, reached: &[FunId]) {
        for f in reached {
            self.divergences.push(format!("the errors of poisoned function {} are left", f.0));
        }
    }

    fn demand_extenders(&mut self, _c: ClassId) {}

    /// What `Worker::deferred_body` gives a body typed before: a std method's function, a
    /// library member's or a product's top-level definition's function or initialiser.
    fn deferred_body(&mut self, s: SymId) -> Option<DeferredBody> {
        let w = self.w;
        let product_top = w.loaded.as_ref().is_some_and(|l| l.product_package_members.contains_key(&s));
        if product_top || w.is_library_member(s) {
            if let Some(&init) = w.val_init.get(&s) {
                return Some(DeferredBody::Val(init));
            }
        }
        if let Some(&f) = w.fun_of_sym.get(&s) {
            return Some(DeferredBody::Fun(f));
        }
        if !self.kept.asked.contains(&s) {
            self.divergences.push(format!("asks for the body of {}, which the kept walk never asked for", self.w.sym_path(s)));
        }
        None
    }

    fn check_class(&mut self, _c: ClassId) {}

    fn decode_templates_ahead(&mut self, _owners: &[ClassId]) -> usize {
        0
    }

    fn bind_mixin_supers(&mut self) {}

    fn settle_dispatch_pending(&mut self) {
        if !self.w.dispatch_pending.is_empty() {
            self.divergences.push(format!("{} members' names are pending", self.w.dispatch_pending.len()));
        }
    }

    fn renamed_roots(&mut self) -> Vec<(SymId, Name)> {
        if !self.w.renamed_roots.is_empty() {
            self.divergences.push(format!("{} roots were renamed", self.w.renamed_roots.len()));
        }
        Vec::new()
    }

    fn reflective_registrations(&mut self) -> Vec<(ClassId, TExprId)> {
        self.kept.reflective.clone()
    }

    fn complete_erased_jar_classes(&mut self) {}
}

/// In link mode the output stands where scalac's class files would, which the jars' bytecode,
/// reflection and Java call by name: every class and object of the program's own files that
/// is not local is kept with its public members, and so is every top-level def and val, as scalac
/// writes them all. Inline defs and what names a type of `scala.quoted` run at compile time only.
fn link_root_members(typer: &Worker, std_files: &[bool], products: bool) -> (Vec<(ClassId, Vec<SymId>)>, Vec<SymId>) {
    let syms = &typer.syms;
    let program_file = |f: crate::source::FileId| !std_files.get(f.0 as usize).copied().unwrap_or(true);
    let runs = |m: SymId| {
        let info = syms.sym(m);
        let quoted = info.sig.as_ref().map_or(false, |s| {
            quoted_type(typer, s.ret, 4) || s.clauses.iter().any(|cl| cl.params.iter().any(|p| quoted_type(typer, p.ty, 4)))
        });
        // What `private[p]` opens to the rest of its package is kept, since a downstream module
        // may be in the package, whether it reads the products or builds over the sources.
        let private = info.mods & crate::ast::mods::PRIVATE != 0 && !info.scoped_private;
        matches!(info.kind, SymKind::Def | SymKind::Val | SymKind::Var | SymKind::Given)
            && info.mods & crate::ast::mods::INLINE == 0
            && !private
            && !quoted
    };
    let mut classes = Vec::new();
    for (i, info) in syms.classes.iter().enumerate() {
        let c = ClassId(i as u32);
        let enum_companion = info.kind == ClassKind::Object && info.companion.map_or(false, |e| syms.class(e).kind == ClassKind::Enum);
        let own = (info.def.is_some() || enum_companion)
            && info.owner != Owner::Local
            && (matches!(info.kind, ClassKind::Class | ClassKind::Object | ClassKind::Enum | ClassKind::EnumCase | ClassKind::Trait)
                || products && info.kind == ClassKind::GivenImpl)
            && program_file(info.file)
            && typer.loaded.as_ref().map_or(true, |l| !l.classes.contains_key(&c));
        if !own {
            continue;
        }
        let mut members: Vec<SymId> = Vec::new();
        for &m in info.members.values() {
            match syms.alternatives(m) {
                Some(alts) => members.extend(alts.iter().copied()),
                None => members.push(m),
            }
        }
        // The extension methods too, which a downstream module calls from its own class files
        // whether it reads the products or, as sbt's residents do, builds over the sources.
        members.extend(info.extensions.iter().copied());
        members.retain(|&m| syms.sym(m).owner == Owner::Class(c) && runs(m));
        classes.push((c, members));
    }
    let top_vals = syms.syms.iter().enumerate().filter(|(_, info)| {
        matches!(info.owner, Owner::Package(_)) && info.def.is_some() && !matches!(info.kind, SymKind::Def)
    });
    let top_extensions: Vec<SymId> =
        syms.syms.iter().enumerate().filter(|(_, info)| matches!(info.owner, Owner::Package(_)) && info.def.is_some() && info.is_extension).map(|(i, _)| SymId(i as u32)).collect();
    let top: Vec<SymId> = typer
        .prog
        .top_funs
        .iter()
        .map(|&f| typer.prog.funs[f.idx()].sym)
        .chain(top_vals.map(|(i, _)| SymId(i as u32)))
        .chain(top_extensions)
        .chain(typer.scala_mirror_vals())
        .filter(|&s| program_file(syms.sym(s).file) && runs(s))
        .collect();
    (classes, top)
}

/// A type of package `scala.quoted` (`Quotes`, `Expr`, `ToExpr`), or one taking such a type
/// as an argument: what only a macro's run at compile time calls.
fn quoted_type(typer: &Worker, t: TypeId, depth: u32) -> bool {
    let Type::Class(c, args) = typer.types.get(t) else { return false };
    let info = typer.syms.class(c);
    let quoted = matches!(info.owner, Owner::Package(p) if {
        let pkg = typer.syms.pkg(p);
        typer.interner.get(pkg.name) == "quoted"
            && pkg.parent.map_or(false, |s| typer.interner.get(typer.syms.pkg(s).name) == "scala" && typer.syms.pkg(s).parent.map_or(false, |r| typer.syms.pkg(r).parent.is_none()))
    });
    quoted || (depth > 0 && typer.types.items(args).iter().any(|&a| quoted_type(typer, a, depth - 1)))
}

/// The differences between a kept reach and the walk from the roots over the typer as it stands,
/// which reads and changes nothing of it but asks it through `Unchanged`: each demand the walk
/// would make of the typer, then each field of the reach the emitter reads that differs. The
/// order of `roots`, `templates`, `linked` and `external_bodies` is no field's: the output does not
/// depend on it, so they compare as sets, the templates by their text.
#[cfg(debug_assertions)]
pub fn check_kept(typer: &Worker, kept: &Reach, setup: &Setup) -> Vec<String> {
    let mut walked = setup.clone();
    let w = Walker::begin(typer, setup);
    let mut t = Unchanged { w: typer, setup, kept, divergences: Vec::new() };
    let (fresh, _) = rounds(&mut t, w, &mut walked, &mut crate::measure::Lap::off(crate::measure::Pass::Reach), &|| false).expect("a walk nothing stops");
    let mut out = t.divergences;
    differences(typer, kept, &fresh, &mut out);
    out
}

#[cfg(debug_assertions)]
fn differences(typer: &Worker, kept: &Reach, fresh: &Reach, out: &mut Vec<String>) {
    let syms = &typer.syms;
    let sym = |i: usize| typer.sym_path(SymId(i as u32));
    let class = |i: usize| typer.class_path(ClassId(i as u32));
    let fun = |i: usize| format!("function {} of {}", i, typer.sym_path(typer.prog.funs[i].sym));
    let flags = |name: &str, a: &[bool], b: &[bool], what: &dyn Fn(usize) -> String, out: &mut Vec<String>| {
        let n = a.len().max(b.len());
        let differ: Vec<usize> = (0..n).filter(|&i| a.get(i).copied().unwrap_or(false) != b.get(i).copied().unwrap_or(false)).collect();
        if !differ.is_empty() {
            let shown: Vec<String> = differ.iter().take(4).map(|&i| format!("{} ({} kept, {} walked)", what(i), a.get(i).copied().unwrap_or(false), b.get(i).copied().unwrap_or(false))).collect();
            out.push(format!("{}: {} differ, {}", name, differ.len(), shown.join(", ")));
        }
    };
    flags("classes", &kept.classes, &fresh.classes, &class, out);
    flags("functions", &kept.funs, &fresh.funs, &fun, out);
    flags("bridged", &kept.bridged, &fresh.bridged, &sym, out);
    flags("vals", &kept.vals, &fresh.vals, &sym, out);
    flags("files", &kept.files, &fresh.files, &|i| typer.source(crate::source::FileId(i as u32)).path.clone(), out);
    flags("triggers", &kept.triggers, &fresh.triggers, &sym, out);
    flags("trigger classes", &kept.trigger_classes, &fresh.trigger_classes, &class, out);
    flags("forwarded", &kept.forwarded, &fresh.forwarded, &sym, out);
    flags("imports", &kept.imports, &fresh.imports, &|i| format!("import {}", i), out);
    flags("fields read", &kept.fields_read, &fresh.fields_read, &sym, out);
    flags("initialiser reads", &kept.init_read, &fresh.init_read, &sym, out);
    // Abstractness is read only for the members of the classes reached. A dead local symbol of a
    // body typed again reads a tree the retype replaced (`Worker::is_abstract_member`), not the
    // same one at the kept walk's time and at this one's.
    let read = |i: usize| match syms.sym(SymId(i as u32)).owner {
        Owner::Class(c) => kept.classes.get(c.idx()).copied().unwrap_or(false) || fresh.classes.get(c.idx()).copied().unwrap_or(false),
        Owner::Package(_) => true,
        Owner::Local => false,
    };
    let abstract_read = |v: &[bool]| -> Vec<bool> { v.iter().enumerate().map(|(i, &a)| a && read(i)).collect() };
    flags("abstract", &abstract_read(&kept.abstract_syms), &abstract_read(&fresh.abstract_syms), &sym, out);
    flags("declared", &kept.declared, &fresh.declared, &sym, out);
    flags("library classes", &kept.library_classes, &fresh.library_classes, &class, out);
    flags("product classes", &kept.product_classes, &fresh.product_classes, &class, out);
    let mut same = |name: &str, equal: bool| {
        if !equal {
            out.push(format!("{} differ", name));
        }
    };
    same("JavaScript names", kept.js_names == fresh.js_names);
    same("globals", kept.globals == fresh.globals);
    same("the uses of classOf", kept.uses_class_of == fresh.uses_class_of);
    same("the uses of element names", kept.uses_element_names == fresh.uses_element_names);
    same("the uses of isAssignableFrom", kept.uses_assignable_from == fresh.uses_assignable_from);
    same("interpolators", kept.interpolators == fresh.interpolators);
    same("registrations", kept.reflective.iter().map(|&(c, e)| (c, e)).eq(fresh.reflective.iter().copied()) && kept.reflective_reached == fresh.reflective_reached);
    let set = |v: &[ClassId]| -> Vec<ClassId> {
        let mut v = v.to_vec();
        v.sort_unstable();
        v.dedup();
        v
    };
    same("linked classes", set(&kept.linked) == set(&fresh.linked));
    same("external bodies", set(&kept.external_bodies) == set(&fresh.external_bodies));
    let texts = |v: &[StrRef]| -> Vec<&str> {
        let mut v: Vec<&str> = v.iter().map(|t| typer.prog.strings[t.idx()].as_str()).collect();
        v.sort_unstable();
        v.dedup();
        v
    };
    same("templates", texts(&kept.templates) == texts(&fresh.templates));
    let roots = |v: &[(TExprId, Place)]| -> Vec<(TExprId, Place)> {
        let mut v = v.to_vec();
        v.sort_unstable();
        v.dedup();
        v
    };
    same("expansion roots", roots(&kept.roots) == roots(&fresh.roots));
}

fn compute_now(typer: &mut Worker, array_seq: Option<ClassId>, js_visible: bool, stop: &dyn Fn() -> bool) -> Option<(Reach, Setup)> {
    let mut lap = crate::measure::Lap::begin(crate::measure::Pass::Reach);
    let before = crate::measure::parts_on().then(|| LoadedCounts::of(typer));
    let diags_before = typer.diags.items.len();
    // The JVM backend names `MatchError` in every match; on JavaScript the runtime's exception
    // classes are looked up, and their std files entered, when the program first catches.
    let thrown = if typer.jvm { runtime_exceptions(typer) } else { Vec::new() };
    // Found first: finding them may enter a std file of the runtime.
    let helpers = if typer.jvm { Some(JvmHelpers::find(typer)) } else { None };
    // The forwarders of the program's exports, which the JVM writes and whose targets the walk
    // keeps: a scalac downstream calls them.
    let exports = if typer.link_mode() {
        let std_files: Vec<bool> = typer.files.as_slice().iter().map(|f| f.is_std).collect();
        std::sync::Arc::new(typer.plan_jvm_exports(&std_files))
    } else {
        Default::default()
    };
    let mut setup = Setup { array_seq, js_visible, thrown, helpers, exported: Vec::new(), found_array_seq: None, found_runtime_exceptions: None, registered_syms: 0, exports };
    let w = Walker::begin(typer, &setup);
    lap.done("setup", 0);
    // JavaScript may call any member of a class an exported definition takes or returns.
    setup.exported = exported_members(typer);
    lap.done("exported members", setup.exported.len());
    let (reach, w) = rounds(typer, w, &mut setup, &mut lap, stop)?;
    lap.count("rounds", lap.round as usize);
    lap.count("diagnostics added", typer.diags.items.len() - diags_before);
    if crate::measure::parts_on() {
        if let Some(before) = &before {
            LoadedCounts::of(typer).since(before, &mut lap);
        }
        lap.count("symbols", typer.syms.syms.len());
        lap.count("classes", typer.syms.classes.len());
        lap.count("functions", typer.prog.funs.len());
        lap.count("expressions", typer.prog.exprs.len());
        lap.count("reach bytes", reach.held());
        lap.count("walker bytes", w.held());
    }
    Some((reach, setup))
}

/// The walk from the roots over the typer's rounds, to the fixed point; the walker with it, for
/// its size.
fn rounds<'a, T: Typing<'a>>(t: &mut T, mut w: Walker, setup: &mut Setup, lap: &mut crate::measure::Lap, stop: &dyn Fn() -> bool) -> Option<(Reach, Walker)> {
    let exported_names: Vec<Name> = setup.exported.iter().map(|n| t.w().interner.intern(n)).collect();
    if setup.js_visible {
        w.reach.js_names = setup.exported.clone();
    }
    w.library = t.w().loaded.is_some();
    w.products = t.w().open_world;
    w.open_world = w.link || w.products;
    if w.open_world {
        (w.link_root_members, w.link_root_funs) = link_root_members(t.w(), &w.std_files, w.products);
        lap.done("open-world roots", w.link_root_members.len() + w.link_root_funs.len());
    }
    w.reach.exports = setup.exports.clone();
    w.roots(Cx::of(t.w()));
    for n in exported_names {
        w.call_name(Cx::of(t.w()), n, None);
    }
    lap.done("walk", 0);
    loop {
        lap.round += 1;
        // A std body typed in the last round may hold the first varargs literal.
        if w.array_seq.is_none() {
            w.array_seq = t.array_seq_class();
            setup.found_array_seq = w.array_seq;
        }
        if stop() || !w.run(Cx::of(t.w()), stop) {
            return None;
        }
        lap.done("walk", 0);
        // A member of a library body's anonymous class that did not type is an error once
        // the program reaches it.
        let reached: Vec<FunId> = t.w().poisoned.keys().copied().filter(|f| w.reach.funs.get(f.idx()).copied().unwrap_or(false)).collect();
        t.take_poisoned(&reached);
        lap.done("poisoned members' errors", reached.len());
        // A library class is compiled before its members are asked for, so that the methods
        // find the class they belong to.
        if w.wants_array_seq && w.array_seq.is_some() {
            w.wants_array_seq = false;
            w.reach_array_seq(Cx::of(t.w()));
        }
        if w.wants_runtime_exceptions && !w.runtime_exceptions_found {
            w.runtime_exceptions_found = true;
            lap.done("walk", 0);
            w.runtime_exceptions = t.runtime_exceptions();
            setup.found_runtime_exceptions = Some(w.runtime_exceptions.clone());
            lap.done("runtime exceptions looked up", w.runtime_exceptions.len());
            w.grow(Cx::of(t.w()));
            for c in w.runtime_exceptions.clone() {
                w.reach_class(Cx::of(t.w()), c);
            }
        }
        lap.done("walk", 0);
        let extenders = std::mem::take(&mut w.pending_extenders);
        for &c in &extenders {
            t.demand_extenders(c);
        }
        lap.done("std extenders entered", extenders.len());
        // A product's top-level definition reached: checked with its file (an eager val with
        // the file's group), then reached again as the whole program's.
        let packages = std::mem::take(&mut w.pending_packages);
        let bodies: Vec<(SymId, Option<DeferredBody>)> = packages.iter().map(|&(_, s)| (s, t.deferred_body(s))).collect();
        lap.done("product definitions typed", packages.len());
        if !packages.is_empty() {
            w.grow(Cx::of(t.w()));
            for (s, body) in bodies {
                if let Some(DeferredBody::Fun(f)) = body {
                    w.fun_of[s.idx()] = slot(f.idx());
                }
                w.reach_static(Cx::of(t.w()), s);
            }
        }
        lap.done("walk", 0);
        if w.library && !w.link {
            let mut owners: Vec<ClassId> = w.pending_classes.clone();
            owners.extend(w.pending.iter().filter_map(|&s| match t.w().syms.sym(s).owner {
                Owner::Class(c) => Some(c),
                _ => None,
            }));
            let decoded = t.decode_templates_ahead(&owners);
            lap.done("templates decoded ahead", decoded);
        }
        let classes = std::mem::take(&mut w.pending_classes);
        for &c in &classes {
            t.check_class(c);
        }
        lap.done("classes checked", classes.len());
        let pending = std::mem::take(&mut w.pending);
        w.reach.asked.extend_from_slice(&pending);
        let typed: Vec<(SymId, DeferredBody)> = pending.into_iter().filter_map(|s| t.deferred_body(s).map(|f| (s, f))).collect();
        lap.done("bodies typed", typed.len());
        // A library trait typed now, or in an earlier build of a watch session, may call
        // `super`: the classes that mix it in get the accessor, and its target is reached.
        t.bind_mixin_supers();
        lap.done("mixin supers bound", 0);
        w.grow(Cx::of(t.w()));
        let accessor_targets: Vec<SymId> = t.w().prog.classes.iter().flat_map(|tc| tc.super_accessors.iter().filter_map(|a| a.target)).collect();
        for target in accessor_targets {
            w.reach_fun_of(Cx::of(t.w()), target);
        }
        for c in classes {
            if t.w().is_library_class(c) {
                w.work.push(Work::Init(c));
            } else {
                w.std_checked.insert(c, ());
                w.reach_class(Cx::of(t.w()), c);
            }
        }
        for (sym, body) in typed {
            match body {
                DeferredBody::Fun(f) => {
                    w.fun_of[sym.idx()] = slot(f.idx());
                    w.reach_fun(f);
                }
                DeferredBody::Val(init) => {
                    w.reach.vals[sym.idx()] = true;
                    w.current_fun = None;
                    w.current_class = None;
                    w.current_val = Some(sym);
                    w.place = Some(Place::Val);
                    w.region = None;
                    w.walk(Cx::of(t.w()), init);
                    w.current_val = None;
                }
            }
        }
        lap.done("walk", 0);
        t.settle_dispatch_pending();
        lap.done("dispatch names settled", 0);
        for (r, renamed) in t.renamed_roots() {
            w.recall_renamed(Cx::of(t.w()), r, renamed);
        }
        lap.done("walk", 0);
        if w.wants_reflect && !w.reflect_found {
            w.reflect_found = true;
            let syms = t.w().syms.syms.len();
            let registrations = t.reflective_registrations();
            setup.registered_syms = t.w().syms.syms.len() - syms;
            lap.done("reflective registrations", registrations.len());
            w.grow(Cx::of(t.w()));
            for &(c, e) in &registrations {
                w.walk_registration(Cx::of(t.w()), c, e);
            }
            w.reach.reflective = registrations;
            w.reach.reflective_reached = true;
        }
        if w.work.is_empty() && w.pending.is_empty() && w.pending_classes.is_empty() && w.pending_extenders.is_empty() && w.pending_packages.is_empty() && !(w.wants_runtime_exceptions && !w.runtime_exceptions_found) && !(w.wants_reflect && !w.reflect_found) {
            w.reach.js_names.sort_unstable();
            w.reach.js_names.dedup();
            w.reach.globals.sort_unstable();
            w.reach.globals.dedup();
            lap.done("walk", 0);
            if t.w().jvm {
                t.complete_erased_jar_classes();
                lap.done("erased jar classes completed", 0);
                w.grow(Cx::of(t.w()));
            }
            let mut abstract_syms = std::mem::take(&mut w.abstract_syms);
            t.w().extend_abstract_members(&mut abstract_syms);
            debug_assert!(abstract_syms == t.w().abstract_members(), "a symbol's abstractness changed after it was read");
            w.reach.abstract_syms = abstract_syms;
            lap.done("abstract members", 0);
            // A closure's call is the JVM's `invokeinterface` of its function class's `apply`,
            // which no member call records.
            let typer = t.w();
            if typer.jvm {
                let arity: Vec<ClassId> = typer.b.functions.iter().chain(&typer.b.context_functions).flatten().copied().collect();
                for c in arity {
                    for &m in typer.syms.class(c).members.values() {
                        if typer.interner.get(typer.syms.sym(m).name) == "apply" {
                            w.reach.declared[m.idx()] = true;
                        }
                    }
                }
            }
            w.reach.library_classes = std::mem::take(&mut w.library_classes);
            w.reach.product_classes = std::mem::take(&mut w.product_classes);
            w.reach.external_bodies = std::mem::take(&mut w.external_bodies);
            lap.done("walk", 0);
            let reach = std::mem::replace(&mut w.reach, Reach::empty());
            return Some((reach, w));
        }
    }
}

/// What the typer and the loader hold, counted before and after the walk: the difference is
/// what the walk's typing made, inside its timed parts.
struct LoadedCounts {
    syms: usize,
    classes: usize,
    funs: usize,
    exprs: usize,
    classes_completed: usize,
    signatures_decoded: usize,
    files_read: usize,
    classes_converted: usize,
    members_typed: usize,
    library_time: std::time::Duration,
}

impl LoadedCounts {
    fn of(typer: &Worker) -> LoadedCounts {
        let loaded = typer.loaded.as_ref().map(|l| &***l);
        LoadedCounts {
            syms: typer.syms.syms.len(),
            classes: typer.syms.classes.len(),
            funs: typer.prog.funs.len(),
            exprs: typer.prog.exprs.len(),
            classes_completed: loaded.map_or(0, |l| l.classes_completed),
            signatures_decoded: loaded.map_or(0, |l| l.signatures_decoded),
            files_read: loaded.map_or(0, |l| l.cp.stats.files_inflated),
            classes_converted: loaded.map_or(0, |l| l.bodies.classes_converted),
            members_typed: loaded.map_or(0, |l| l.bodies.members_typed),
            library_time: loaded.map_or(std::time::Duration::ZERO, |l| l.bodies.time),
        }
    }

    fn since(&self, before: &LoadedCounts, lap: &mut crate::measure::Lap) {
        lap.count("symbols made", self.syms - before.syms);
        lap.count("classes made", self.classes - before.classes);
        lap.count("functions made", self.funs - before.funs);
        lap.count("expressions made", self.exprs - before.exprs);
        lap.count("jar classes completed", self.classes_completed - before.classes_completed);
        lap.count("jar signatures decoded", self.signatures_decoded - before.signatures_decoded);
        lap.count("jar files read", self.files_read - before.files_read);
        lap.count("jar classes converted", self.classes_converted - before.classes_converted);
        lap.count("jar members typed", self.members_typed - before.members_typed);
        lap.count("jar bodies' own clock, ms", (self.library_time - before.library_time).as_millis() as usize);
    }
}

impl Reach {
    /// The bytes its tables hold.
    pub fn held(&self) -> usize {
        let flags = [&self.classes, &self.funs, &self.bridged, &self.vals, &self.files, &self.triggers, &self.trigger_classes, &self.forwarded, &self.imports, &self.fields_read, &self.init_read, &self.abstract_syms, &self.declared, &self.library_classes, &self.product_classes];
        flags.iter().map(|v| v.capacity()).sum::<usize>()
            + self.asked.capacity() * 4
            + self.js_names.iter().map(|n| n.capacity() + 24).sum::<usize>()
            + self.templates.capacity() * 4
            + self.globals.capacity() * 4
            + (self.linked.capacity() + self.external_bodies.capacity()) * 4
            + self.roots.capacity() * std::mem::size_of::<(TExprId, Place)>()
            + self.interpolators.capacity() * 8
            + self.reflective.capacity() * 8
    }

    /// Whether `s` is an abstract member: a library symbol says so itself, a source one is in
    /// the typer's table.
    pub fn is_abstract(&self, syms: &Symbols, s: SymId) -> bool {
        let info = syms.sym(s);
        match info.def {
            None => info.mods & mods::ABSTRACT != 0,
            Some(_) => self.abstract_syms.get(s.idx()).copied().unwrap_or(false),
        }
    }
}

/// The exception classes the JavaScript runtime throws (`$exc` in `rt.js`), by their names in
/// `java.lang`, `java.util`, `scala` and `js`.
pub const RUNTIME_THROWN: [&str; 14] = [
    "Throwable",
    "JavaScriptException",
    "MatchError",
    "ArithmeticException",
    "NumberFormatException",
    "IndexOutOfBoundsException",
    "StringIndexOutOfBoundsException",
    "ArrayIndexOutOfBoundsException",
    "IllegalArgumentException",
    "NoSuchElementException",
    "RuntimeException",
    "UnsupportedOperationException",
    "AssertionError",
    "NotImplementedError",
];

fn runtime_exceptions(typer: &mut Worker) -> Vec<ClassId> {
    let mut out = Vec::with_capacity(RUNTIME_THROWN.len());
    let js = typer.syms.pkg(ROOT_PKG).entries.get(&names::JS).and_then(|e| e.pkg);
    let pkgs = [typer.java_lang_pkg(), typer.java_util_pkg(), Some(typer.b.scala_pkg), js];
    for name in RUNTIME_THROWN {
        let n = typer.interner.intern(name);
        for p in pkgs.into_iter().flatten() {
            // A class of the std or of scala-library's jar is entered when first named, which
            // a watch session's earlier build may have done: it is looked up the same way
            // either way.
            let found = if p == typer.b.scala_pkg { typer.std_class(name) } else { typer.demand_class(p, n) };
            if let Some(c) = found {
                out.push(c);
                break;
            }
        }
    }
    out
}

/// What code generated for the JVM calls in `std/jvm.scala` without the IR saying so: the
/// definitions every program needs, and the ones that a construct of the program brings in.
#[derive(Clone)]
struct JvmHelpers {
    /// The definitions and classes of package `scala.runtime` by name.
    terms: FxMap<String, SymId>,
    classes: FxMap<String, ClassId>,
    always: Vec<SymId>,
    seq_pattern: Option<SymId>,
    enum_lookups: Vec<SymId>,
    partial_function: Option<ClassId>,
    /// In link mode, the jar's `StringContext.s` and `raw` with the runtime's definitions that
    /// stand for them (`Gen::call_interpolator`).
    interpolators: Vec<(SymId, SymId)>,
    /// The std classes that are the JDK's own (`@jvmClass`): nothing of their bodies runs.
    jdk_classes: Vec<bool>,
}

impl JvmHelpers {
    fn find(typer: &mut Worker) -> JvmHelpers {
        let runtime = typer.interner.intern("runtime");
        let mut out = JvmHelpers {
            terms: FxMap::default(),
            classes: FxMap::default(),
            always: Vec::new(),
            seq_pattern: None,
            enum_lookups: Vec::new(),
            partial_function: None,
            interpolators: Vec::new(),
            jdk_classes: typer
                .syms
                .classes
                .iter()
                .map(|info| info.def.and_then(|d| typer.asts[info.file.0 as usize].defs.get(d.idx())).map_or(false, |d| d.annots.iter().any(|a| a.name == crate::names::JVM_CLASS)))
                .collect(),
        };
        let Some(pkg) = typer.syms.pkg(typer.b.scala_pkg).entries.get(&runtime).and_then(|e| e.pkg) else { return out };
        for (name, entry) in &typer.syms.pkg(pkg).entries {
            if let Some(s) = entry.term {
                out.terms.insert(typer.interner.get(*name).to_string(), s);
            }
            if let Some(c) = entry.class {
                out.classes.insert(typer.interner.get(*name).to_string(), c);
            }
        }
        let term = |name: &str| {
            let n = typer.interner.intern(name);
            typer.syms.pkg(pkg).entries.get(&n).and_then(|e| e.term)
        };
        out.always = ["equal", "anyHash", "mix", "finalizeHash"].iter().filter_map(|n| term(n)).collect();
        out.seq_pattern = term("seqPattern");
        out.enum_lookups = ["enumValueOf", "enumFromOrdinal"].iter().filter_map(|n| term(n)).collect();
        let n = typer.interner.intern("PartialFunctionImpl");
        out.partial_function = typer.syms.pkg(pkg).entries.get(&n).and_then(|e| e.class);
        if typer.link_mode() {
            out.interpolators = crate::jvm::interpolators(typer, pkg);
        }
        out
    }
}

/// The program as the walk reads it; taken afresh after the typer has added to it.
#[derive(Clone, Copy)]
pub(super) struct Cx<'a> {
    pub(super) prog: &'a Program,
    pub(super) syms: &'a Symbols,
    interner: &'a Interner,
    loaded: Option<&'a crate::typer::loader::Loaded>,
    pub(super) typer: &'a Worker<'a>,
}

impl<'a> Cx<'a> {
    pub(super) fn of(typer: &'a Worker) -> Cx<'a> {
        Cx { prog: &typer.prog, syms: &typer.syms, interner: &*typer.interner, loaded: typer.loaded.as_ref().map(|l| &***l), typer }
    }
}

/// Stands for a call through a receiver of any class.
const ANY_OWNER: ClassId = ClassId(u32::MAX);

/// Tables indexed by symbol or class hold an index plus one, so that an untouched table is zeroed
/// memory.
type Slot = u32;
const NONE: Slot = 0;

fn is_ctor_param(syms: &Symbols, c: ClassId, s: SymId) -> bool {
    syms.class(c).ctor_syms.iter().flatten().any(|&p| p == s)
}

fn slot(index: usize) -> Slot {
    index as Slot + 1
}

fn index(s: Slot) -> Option<usize> {
    s.checked_sub(1).map(|i| i as usize)
}

/// The std files of teq's JVM runtime, which a directory of products holds whole.
const RUNTIME_FILES: [&str; 6] = ["<std>/jvm.scala", "<std>/jvm_scala_library.scala", "<std>/jvm_buffer.scala", "<std>/library/product.scala", "<std>/library/mirrors.scala", "<std>/javalib/string.scala"];

/// The members the runtime calls on values of any class.
const RUNTIME_CALLED: [Name; 8] = [
    names::TO_STRING,
    names::EQUALS,
    names::HASH_CODE,
    names::FOREACH,
    names::ITERATOR,
    names::HAS_NEXT,
    names::NEXT,
    names::DROP,
];

enum Work {
    Class(ClassId),
    /// A library class the typer has checked since it was reached: its parents, which the
    /// class walk saw before they were known, and its IR.
    Init(ClassId),
    Fun(FunId),
}

/// Lists indexed by class, kept as linked nodes so that an empty list costs one word.
struct Links<T> {
    heads: Vec<Slot>,
    nodes: Vec<(T, Slot)>,
}

impl<T: Copy> Links<T> {
    fn new() -> Links<T> {
        Links { heads: Vec::new(), nodes: Vec::new() }
    }

    fn push(&mut self, at: ClassId, item: T) {
        self.nodes.push((item, self.heads[at.idx()]));
        self.heads[at.idx()] = slot(self.nodes.len() - 1);
    }

    fn iter(&self, at: ClassId) -> impl Iterator<Item = T> + '_ {
        let mut i = self.heads[at.idx()];
        std::iter::from_fn(move || {
            let (item, next) = self.nodes[index(i)?];
            i = next;
            Some(item)
        })
    }
}

struct Walker {
    jvm: Option<JvmHelpers>,
    /// Std classes reached, whose subclasses across the std's files the typer brings in.
    pending_extenders: Vec<ClassId>,
    /// Std classes the typer checked between two walks: reached on the second ask whether or
    /// not the check made a `TClass` (a native JS class gets none).
    std_checked: FxMap<ClassId, ()>,
    /// Whether the runtime's exception classes were looked up, and whether a catch asked for
    /// them before they were.
    runtime_exceptions_found: bool,
    wants_runtime_exceptions: bool,
    /// A varargs literal was met before the std's `ArraySeq` was known.
    wants_array_seq: bool,
    runtime_exceptions: Vec<ClassId>,
    runtime_exceptions_reached: bool,
    /// A lookup of `Reflect` was met, and whether the registrations it needs were made.
    wants_reflect: bool,
    /// The templates of `Reach::templates`, by bit.
    templates_seen: Vec<u64>,
    reflect_found: bool,
    /// Inside a registration, where a parameter's `classOf` is class data only.
    class_data_only: bool,
    reach: Reach,
    layout: Layout,
    array_seq: Option<ClassId>,
    std_files: Vec<bool>,
    /// The function or class being walked, for the detail of what reaches a class.
    current_fun: Option<FunId>,
    current_class: Option<ClassId>,
    current_val: Option<SymId>,
    /// The definition whose body is being walked, which holds the expansions met; a local def
    /// belongs to the definition around it.
    place: Option<Place>,
    local_places: FxMap<FunId, Place>,
    /// The file whose initialiser has run, or is running, wherever the code being walked runs:
    /// inside the body of one of its top-level defs or its initialiser (`Reach::forwarded`).
    region: Option<FileId>,
    /// The local defs of such a body, with its file.
    local_regions: FxMap<FunId, FileId>,
    /// Per class, whether it was read from a Java class file: a class below one may be called
    /// from Java through any member of the Java class it implements.
    java_classes: Vec<bool>,
    tclass_of: Vec<Slot>,
    /// The function registered for each def, method or given, by symbol.
    fun_of: Vec<Slot>,
    /// The initialiser of each top-level val, by symbol.
    top_val_of: Vec<Slot>,
    /// The symbols already reached by their path, the members already called and the bodies
    /// already asked of the typer, so that a repeated reference costs one lookup.
    seen: Vec<bool>,
    called_syms: Vec<bool>,
    asked: Vec<bool>,
    /// The member names called, each with the class of the receiver it was called through.
    called: FxMap<(Name, ClassId), ()>,
    /// Per class, the names called through a receiver of that class.
    called_through: Links<Name>,
    /// The names called through a receiver of unknown class.
    called_any: Vec<Name>,
    /// The source name of each called alternative of an overloaded method, by its name in the
    /// output, which is what calls are recorded under.
    overloaded: FxMap<Name, Name>,
    /// Per class, the reached classes that have it among their ancestors.
    descendants: Links<ClassId>,
    /// Per symbol, whether it is a member without a body; a symbol entered later is read from
    /// its modifiers.
    abstract_syms: Vec<bool>,
    /// The names called through `super`, whose every definition is kept.
    super_called: FxMap<Name, ()>,
    work: Vec<Work>,
    /// Defs of the standard library whose bodies the walk needs next.
    pending: Vec<SymId>,
    /// Classes of the classpath the walk reached, which the typer compiles from their TASTy.
    pending_classes: Vec<ClassId>,
    /// The products' files of top-level definitions the walk reached a definition of, before
    /// the typer converted them (`Worker::check_product_package`), each with the definition.
    pending_packages: Vec<(ClassId, SymId)>,
    /// Whether classes of the classpath are compiled, which the JavaScript target does.
    library: bool,
    /// Link mode (`Worker::link_mode`): a jar class is never compiled, its members are the jar's.
    link: bool,
    linked_seen: Vec<bool>,
    link_root_members: Vec<(ClassId, Vec<SymId>)>,
    link_root_funs: Vec<SymId>,
    /// Every definition of the program's files is a root (link mode, the product mode).
    open_world: bool,
    /// The product mode: every method of a class and every extension is a root too.
    products: bool,
    /// The std files whose top-level definitions a JVM product build has written whole: a
    /// directory of products holds teq's runtime complete, whatever its build reached, since a
    /// product of another build of the module may call any of it (docs/TARGETS.md, "A module's
    /// products").
    complete_std_files: FxMap<FileId, ()>,
    /// Per class, whether it came from a directory of teq's products.
    product_classes: Vec<bool>,
    /// The classes of product directories whose construction may run a body.
    external_bodies: Vec<ClassId>,
    /// Per class, whether it was read from a jar's TASTy.
    library_classes: Vec<bool>,
    /// Per class, how many of its base types the walk has registered it under.
    bases_registered: Vec<u32>,
    /// Whether the walk is inside the body of a class, where a local is read from a field
    /// when the body runs apart from the constructor.
    in_init: bool,
    /// The `@js` templates of the standard library, when JS-visible names are asked for, and
    /// how many symbols they cover.
    std_templates: Option<FxMap<String, ()>>,
    std_templates_upto: usize,
    /// How much of the program the tables cover.
    n_classes: usize,
    n_top_funs: usize,
    n_top_vals: usize,
    n_virtual_calls: usize,
}

impl Walker {
    fn new(cx: Cx, array_seq: Option<ClassId>, layout: Layout, std_files: Vec<bool>, link: bool) -> Walker {
        let n_files = layout.file_groups.iter().map(|(f, _)| f.0 as usize + 1).max().unwrap_or(0);
        let mut w = Walker {
            jvm: None,
            pending_extenders: Vec::new(),
            std_checked: FxMap::default(),
            runtime_exceptions_found: false,
            wants_runtime_exceptions: false,
            wants_reflect: false,
            templates_seen: Vec::new(),
            reflect_found: false,
            class_data_only: false,
            wants_array_seq: false,
            runtime_exceptions: Vec::new(),
            runtime_exceptions_reached: false,
            reach: Reach { files: vec![false; n_files], ..Reach::empty() },
            place: None,
            local_places: FxMap::default(),
            region: None,
            local_regions: FxMap::default(),
            layout,
            array_seq,
            std_files,
            current_fun: None,
            current_class: None,
            current_val: None,
            java_classes: Vec::new(),
            tclass_of: Vec::new(),
            fun_of: Vec::new(),
            top_val_of: Vec::new(),
            seen: Vec::new(),
            called_syms: Vec::new(),
            asked: Vec::new(),
            called: FxMap::default(),
            called_through: Links::new(),
            called_any: Vec::new(),
            overloaded: FxMap::default(),
            descendants: Links::new(),
            abstract_syms: Vec::new(),
            super_called: FxMap::default(),
            work: Vec::new(),
            pending: Vec::new(),
            pending_classes: Vec::new(),
            pending_packages: Vec::new(),
            library: false,
            link,
            linked_seen: Vec::new(),
            link_root_members: Vec::new(),
            link_root_funs: Vec::new(),
            open_world: false,
            products: false,
            complete_std_files: FxMap::default(),
            product_classes: Vec::new(),
            external_bodies: Vec::new(),
            library_classes: Vec::new(),
            bases_registered: Vec::new(),
            in_init: false,
            std_templates: None,
            std_templates_upto: 0,
            n_classes: 0,
            n_top_funs: 0,
            n_top_vals: 0,
            n_virtual_calls: 0,
        };
        w.grow(cx);
        w
    }

    /// The bytes its tables hold besides the `Reach`.
    fn held(&self) -> usize {
        let slots = [&self.tclass_of, &self.fun_of, &self.top_val_of, &self.called_through.heads, &self.descendants.heads];
        let flags = [&self.java_classes, &self.seen, &self.called_syms, &self.asked, &self.linked_seen, &self.std_files];
        slots.iter().map(|v| v.capacity() * 4).sum::<usize>()
            + flags.iter().map(|v| v.capacity()).sum::<usize>()
            + self.called_through.nodes.capacity() * 8
            + self.descendants.nodes.capacity() * 8
            + self.called.capacity() * 12
            + self.overloaded.capacity() * 12
            + self.local_places.capacity() * 16
            + self.local_regions.capacity() * 12
            + self.bases_registered.capacity() * 4
    }

    /// A walker over the program as `typer` holds it, with what `setup` took from the typer.
    fn begin(typer: &Worker, setup: &Setup) -> Walker {
        let std_files: Vec<bool> = typer.files.as_slice().iter().map(|f| f.is_std).collect();
        let layout = Layout::new(&typer.prog, &typer.syms, typer.interner);
        let mut w = Walker::new(Cx::of(typer), setup.array_seq, layout, std_files, typer.link_mode());
        w.abstract_syms = typer.abstract_members();
        if let Some(helpers) = &setup.helpers {
            w.reach.interpolators = helpers.interpolators.clone();
            w.jvm = Some(helpers.clone());
        }
        w.note_loaded_classes(Cx::of(typer));
        w.runtime_exceptions = setup.thrown.clone();
        w.runtime_exceptions_found = typer.jvm;
        if setup.js_visible {
            w.std_templates = Some(FxMap::default());
            w.note_std_templates(Cx::of(typer));
        }
        w
    }

    /// Extends the tables over what the typer added: symbols, classes, functions, and the
    /// registrations of new classes, top-level functions and vals.
    fn grow(&mut self, cx: Cx) {
        let n_syms = cx.syms.syms.len();
        for table in [&mut self.fun_of, &mut self.top_val_of] {
            table.resize(n_syms, NONE);
        }
        for table in [
            &mut self.reach.vals,
            &mut self.reach.triggers,
            &mut self.reach.forwarded,
            &mut self.reach.bridged,
            &mut self.reach.declared,
            &mut self.reach.fields_read,
            &mut self.reach.init_read,
            &mut self.seen,
            &mut self.called_syms,
            &mut self.asked,
        ] {
            table.resize(n_syms, false);
        }
        let n_classes = cx.syms.classes.len();
        self.reach.classes.resize(n_classes, false);
        self.reach.trigger_classes.resize(n_classes, false);
        self.layout.has_body.resize(n_classes, false);
        self.bases_registered.resize(n_classes, 0);
        self.tclass_of.resize(n_classes, NONE);
        self.called_through.heads.resize(n_classes, NONE);
        self.descendants.heads.resize(n_classes, NONE);
        self.reach.funs.resize(cx.prog.funs.len(), false);
        self.reach.imports.resize(cx.prog.js_imports.len(), false);
        for (i, tc) in cx.prog.classes.iter().enumerate().skip(self.n_classes) {
            self.tclass_of[tc.id.idx()] = slot(i);
            for &f in tc.methods.iter().chain(&tc.ctors) {
                self.fun_of[cx.prog.funs[f.idx()].sym.idx()] = slot(f.idx());
            }
            self.layout.note_class(cx.prog, cx.syms, tc);
        }
        self.note_loaded_classes(cx);
        self.n_classes = cx.prog.classes.len();
        for &f in &cx.prog.top_funs[self.n_top_funs..] {
            self.fun_of[cx.prog.funs[f.idx()].sym.idx()] = slot(f.idx());
        }
        self.n_top_funs = cx.prog.top_funs.len();
        for &(sym, init) in &cx.prog.top_vals[self.n_top_vals..] {
            self.top_val_of[sym.idx()] = slot(init.idx());
            self.layout.note_top_val(cx.prog, cx.syms, cx.interner, sym, init);
        }
        self.n_top_vals = cx.prog.top_vals.len();
        for i in self.n_virtual_calls..cx.prog.template_calls.len() {
            self.call_member(cx, cx.prog.template_calls[i].0);
        }
        self.n_virtual_calls = cx.prog.template_calls.len();
    }

    /// Which of the classes entered since the last call were read from a jar's TASTy or from a
    /// Java class file; the typer enters more of them while it types the bodies the walk asks for.
    fn note_loaded_classes(&mut self, cx: Cx) {
        let Some(loaded) = cx.loaded else { return };
        let n = cx.syms.classes.len();
        let first = self.library_classes.len();
        self.java_classes.resize(n, false);
        self.library_classes.resize(n, false);
        for i in first..n {
            let c = ClassId(i as u32);
            // A class of the platform layer that absorbed the JDK's members is implemented by
            // the std: only a Java class read for a jar has every member callable from Java.
            self.java_classes[i] = loaded.java.classes.contains_key(&c) && cx.syms.class(c).def.is_none();
            self.library_classes[i] = loaded.classes.contains_key(&c);
            if loaded.is_product_class(c) {
                if self.product_classes.len() <= i {
                    self.product_classes.resize(n, false);
                }
                self.product_classes[i] = true;
                // On the JVM another module's class is its class files, whose construction may
                // run anything; where its bodies are converted, they say what it runs.
                if self.link && !loaded.is_synthetic_product_class(c) {
                    self.layout.note_external(c);
                    self.external_bodies.push(c);
                }
            }
        }
    }

    /// A class of an upstream module's products: its class files are there, it is called and
    /// never compiled or written.
    fn is_product_class(&self, c: ClassId) -> bool {
        self.product_classes.get(c.idx()).copied().unwrap_or(false)
    }

    fn roots(&mut self, cx: Cx) {
        for s in self.jvm.as_ref().map_or(Vec::new(), |j| j.always.clone()) {
            self.reach_static(cx, s);
        }
        // A match that fails throws `scala.MatchError`, which the JVM backend names in every
        // match; the JavaScript runtime makes do without the class until the program catches.
        if self.jvm.is_some() {
            if let Some(c) = self.runtime_exceptions.iter().copied().find(|&c| cx.interner.get(cx.syms.class(c).name) == "MatchError") {
                self.reach_class(cx, c);
            }
        }
        if let Some(main) = cx.prog.main {
            let info = cx.syms.sym(main);
            self.reach_static(cx, main);
            if let Some(object) = cx.prog.main_object {
                self.reach_class(cx, object);
            }
            if matches!(info.owner, Owner::Package(_)) {
                self.reach_file(cx, info.file);
            }
            if has_repeated_param(cx, main) {
                self.reach_array_seq(cx);
            }
        }
        if self.open_world {
            self.link_roots(cx);
        }
        for &(sym, _) in &cx.prog.js_exports {
            let info = cx.syms.sym(sym);
            if let Some(i) = info.js_import {
                self.reach.imports[i as usize] = true;
                continue;
            }
            if info.kind == SymKind::Def && has_repeated_param(cx, sym) {
                self.reach_array_seq(cx);
            }
            self.reach.forwarded[sym.idx()] = true;
            self.reach_static(cx, sym);
        }
    }

    fn link_roots(&mut self, cx: Cx) {
        for s in std::mem::take(&mut self.link_root_funs) {
            self.reach_static(cx, s);
        }
        // What the export forwarders call: the qualifier's module, the member through it.
        let exports = self.reach.exports.clone();
        for f in exports.classes.values().chain(exports.files.values()).flatten() {
            if let crate::typer::exports::ExportQualifier::Object(o) = f.q {
                self.reach_class(cx, o);
            }
            match f.target {
                crate::typer::export_plan::JvmTarget::Object(k) => self.reach_class(cx, k),
                crate::typer::export_plan::JvmTarget::Member(s) => {
                    self.call_member(cx, s);
                    self.call_static(cx, s);
                }
            }
        }
        for (c, members) in std::mem::take(&mut self.link_root_members) {
            self.reach_class(cx, c);
            // Its secondary constructors, which scalac writes whoever calls them, with the
            // getters of their defaults on the companion.
            if self.link {
                for &s in &cx.syms.class(c).ctors {
                    if cx.syms.sym(s).mods & mods::PRIVATE == 0 {
                        self.reach_fun_of(cx, s);
                    }
                }
            }
            // A later caller may call an abstract extension through the class too, which its
            // class file declares as scalac's does, called here or not.
            for &m in &cx.syms.class(c).extensions {
                let info = cx.syms.sym(m);
                if info.owner == Owner::Class(c) && info.mods & mods::INLINE == 0 {
                    self.reach.declared[m.idx()] = true;
                }
            }
            for m in members {
                self.reach.declared[m.idx()] = true;
                let n = self.output_name(cx, m);
                self.call_name(cx, n, Some(c));
                // A module's products keep every method a class or trait defines, whether or
                // not a class of the module that runs it is made: a downstream one may be. A
                // given with type parameters or a clause is one (scalac's def, `Parsers.givenDef`).
                let def = match cx.syms.sym(m).kind {
                    SymKind::Def => true,
                    SymKind::Given => index(self.fun_of[m.idx()]).is_some(),
                    _ => false,
                };
                if self.products && def {
                    self.reach_fun_of(cx, m);
                }
            }
        }
    }

    /// Walks the work list out; false where `stop` said so, which it asks every 1,024 items.
    fn run(&mut self, cx: Cx, stop: &dyn Fn() -> bool) -> bool {
        let mut items = 0u32;
        while let Some(item) = self.work.pop() {
            items = items.wrapping_add(1);
            if items % 1024 == 0 && stop() {
                return false;
            }
            match item {
                Work::Class(c) => {
                    self.current_fun = None;
                    self.current_class = Some(c);
                    self.place = Some(Place::Class(c));
                    self.region = None;
                    self.process_class(cx, c)
                }
                Work::Init(c) => {
                    self.current_fun = None;
                    self.current_class = Some(c);
                    self.place = Some(Place::Class(c));
                    self.region = None;
                    let from = self.bases_registered[c.idx()] as usize;
                    self.process_class_from(cx, c, from);
                }
                Work::Fun(f) => {
                    self.current_fun = Some(f);
                    self.place = Some(self.local_places.get(&f).copied().unwrap_or(Place::Fun(f)));
                    let fun = &cx.prog.funs[f.idx()];
                    if matches!(cx.syms.sym(fun.sym).owner, Owner::Class(c) if self.is_jdk_class(c)) {
                        continue;
                    }
                    // A default of a parameter runs before the body, where a call from JavaScript
                    // may leave the file uninitialised.
                    self.region = None;
                    for &d in fun.defaults.iter().flatten() {
                        self.walk(cx, d);
                    }
                    let info = cx.syms.sym(fun.sym);
                    self.region = match info.owner {
                        Owner::Package(_) => Some(info.file),
                        _ => self.local_regions.get(&f).copied(),
                    };
                    if let Some(body) = fun.body {
                        self.walk(cx, body);
                    }
                }
            }
        }
        true
    }

    fn reach_class(&mut self, cx: Cx, c: ClassId) {
        if self.reach.classes[c.idx()] {
            return;
        }
        if cx.loaded.map_or(false, |l| l.detail) {
            let from = self.current_fun.map(|f| {
                let s = cx.prog.funs[f.idx()].sym;
                let owner = match cx.syms.sym(s).owner {
                    Owner::Class(k) => format!("{}.", cx.interner.get(cx.syms.class(k).name)),
                    _ => String::new(),
                };
                format!("{}{}", owner, cx.interner.get(cx.syms.sym(s).name))
            }).or_else(|| self.current_class.map(|k| format!("class {}", cx.interner.get(cx.syms.class(k).name))))
            .or_else(|| self.current_val.map(|v| {
                let owner = match cx.syms.sym(v).owner {
                    Owner::Class(k) => format!("{}.", cx.interner.get(cx.syms.class(k).name)),
                    _ => String::new(),
                };
                format!("val {}{}", owner, cx.interner.get(cx.syms.sym(v).name))
            })).unwrap_or_default();
            let tag = |k: ClassId| if cx.syms.class(k).kind == ClassKind::Object { "$" } else { "" };
            eprintln!("reach class {}{} from {}", cx.interner.get(cx.syms.class(c).name), tag(c), from);
        }
        // A library class behind a builtin of its name (`scala.Tuple2`) lends the builtin its
        // members and base types; the builtin is what runs.
        if cx.loaded.map_or(false, |l| l.builtin_of.contains_key(&c)) {
            return;
        }
        // A std class met before the typer checked it is checked between two walks, with the
        // names of its members settled and the std files with classes below it entered, and
        // reached again then: nothing marks its members before that.
        if index(self.tclass_of[c.idx()]).is_none() && self.std_class_unchecked(cx, c) && !self.std_checked.contains_key(&c) {
            if !self.pending_classes.contains(&c) {
                self.pending_classes.push(c);
                self.pending_extenders.push(c);
            }
            return;
        }
        self.reach.classes[c.idx()] = true;
        if self.link && (self.is_library_class(c) || self.is_product_class(c)) {
            self.note_linked(c);
            // Another module's case class has its companion touched before `new` where that
            // module's manifest says the companion runs code, as the whole build touches it.
            if self.is_product_class(c) {
                if let Some(companion) = super::touched_companion(cx.syms, &self.layout.has_body, c) {
                    self.reach_class(cx, companion);
                }
            }
            return;
        }
        if self.writes_runtime_whole(cx, cx.syms.class(c).file) {
            self.reach_members_whole(cx, c);
        }
        if let Some(JsBinding::Import(i)) = cx.syms.class(c).js_binding {
            self.reach.imports[i as usize] = true;
        }
        // A class of a jar is checked when first reached.
        if index(self.tclass_of[c.idx()]).is_none() && self.library && self.library_classes.get(c.idx()).copied().unwrap_or(false) {
            self.pending_classes.push(c);
        }
        self.work.push(Work::Class(c));
    }

    /// A class of a std file with a definition, which the reach pass checks rather than the
    /// walk over the files.
    fn std_class_unchecked(&self, cx: Cx, c: ClassId) -> bool {
        let info = cx.syms.class(c);
        info.def.is_some()
            && info.owner != Owner::Local
            && self.std_files.get(info.file.0 as usize).copied().unwrap_or(false)
            && !self.library_classes.get(c.idx()).copied().unwrap_or(false)
    }

    fn reach_fun(&mut self, f: FunId) {
        if self.reach.funs[f.idx()] {
            return;
        }
        self.reach.funs[f.idx()] = true;
        self.work.push(Work::Fun(f));
    }

    /// Whether the reach writes a std file's classes and top-level definitions whole: in a JVM
    /// build of products or over products, the files of teq's runtime, so that whichever copy a
    /// run's class path finds first answers every module's calls.
    fn writes_runtime_whole(&self, cx: Cx, file: FileId) -> bool {
        (self.products || cx.loaded.is_some_and(|l| l.cp.holds_products()))
            && cx.typer.jvm
            && self.std_files.get(file.0 as usize).copied().unwrap_or(false)
            && RUNTIME_FILES.contains(&cx.typer.files.as_slice()[file.0 as usize].path.as_str())
    }

    /// Every member of a std class a JVM build of products writes, as the open world's roots are.
    fn reach_members_whole(&mut self, cx: Cx, c: ClassId) {
        let info = cx.syms.class(c);
        let mut members: Vec<SymId> = Vec::new();
        for &m in info.members.values() {
            match cx.syms.alternatives(m) {
                Some(alts) => members.extend(alts.iter().copied()),
                None => members.push(m),
            }
        }
        members.retain(|&m| {
            let s = cx.syms.sym(m);
            s.owner == Owner::Class(c) && matches!(s.kind, SymKind::Def | SymKind::Val | SymKind::Var | SymKind::Given) && s.mods & mods::INLINE == 0 && s.intrinsic.is_none()
        });
        for m in members {
            self.reach.declared[m.idx()] = true;
            let n = self.output_name(cx, m);
            self.call_name(cx, n, Some(c));
            if cx.syms.sym(m).kind == SymKind::Def {
                self.reach_fun_of(cx, m);
            }
        }
    }

    /// Every top-level definition of a std file a JVM build of products writes, the first time
    /// one of them is reached.
    fn reach_file_whole(&mut self, cx: Cx, file: FileId) {
        if self.complete_std_files.insert(file, ()).is_some() {
            return;
        }
        let defs: Vec<SymId> = cx
            .syms
            .syms
            .iter()
            .enumerate()
            .filter(|(_, s)| s.file == file && matches!(s.owner, Owner::Package(_)) && s.def.is_some() && matches!(s.kind, SymKind::Def | SymKind::Val | SymKind::Var | SymKind::Given))
            .filter(|(_, s)| s.mods & mods::INLINE == 0 && s.intrinsic.is_none() && s.js_import.is_none() && s.js_global.is_none())
            // `@leanOnly`: a definition over the lean std's collections, which the JVM's std,
            // scala-library, does not have.
            .filter(|(_, s)| s.def.map_or(true, |d| !cx.typer.asts[file.0 as usize].def(d).annots.iter().any(|a| a.name == names::LEAN_ONLY)))
            .map(|(i, _)| SymId(i as u32))
            .collect();
        for s in defs {
            self.reach_static(cx, s);
        }
    }

    /// The function of a def, or a request to the typer for the body of a std def nobody typed yet.
    fn reach_fun_of(&mut self, cx: Cx, sym: SymId) {
        let file = cx.syms.sym(sym).file;
        if matches!(cx.syms.sym(sym).owner, Owner::Package(_)) && self.writes_runtime_whole(cx, file) {
            self.reach_file_whole(cx, file);
        }
        if let Some(f) = index(self.fun_of[sym.idx()]) {
            self.reach_fun(FunId(f as u32));
            return;
        }
        let info = cx.syms.sym(sym);
        if self.link_member(cx, sym) {
            return;
        }
        let in_std = self.std_files.get(info.file.0 as usize).copied().unwrap_or(true);
        let deferred = in_std && matches!(info.kind, SymKind::Def | SymKind::Given);
        if deferred && !std::mem::replace(&mut self.asked[sym.idx()], true) {
            self.pending.push(sym);
        }
    }

    fn is_library_class(&self, c: ClassId) -> bool {
        self.library_classes.get(c.idx()).copied().unwrap_or(false)
    }

    fn note_linked(&mut self, c: ClassId) {
        if self.linked_seen.len() <= c.idx() {
            self.linked_seen.resize(c.idx() + 1, false);
        }
        if !std::mem::replace(&mut self.linked_seen[c.idx()], true) {
            self.reach.linked.push(c);
        }
    }

    /// In link mode, whether `s` is a member of a jar class, which the jar's bytecode
    /// implements: its owner is noted for its descriptor and nothing is typed.
    fn link_member(&mut self, cx: Cx, s: SymId) -> bool {
        if !self.link {
            return false;
        }
        match cx.syms.sym(s).owner {
            Owner::Class(c) if self.is_product_class(c) => {
                self.note_linked(c);
                true
            }
            Owner::Class(c) if self.is_library_class(c) => {
                self.note_linked(c);
                if let Some(&(_, helper)) = self.jvm.as_ref().and_then(|j| j.interpolators.iter().find(|&&(m, _)| m == s)) {
                    self.reach_static(cx, helper);
                }
                true
            }
            _ => false,
        }
    }

    /// A val of a library class is typed when it is read, like a method: the typer supplies
    /// its initialiser and the walk goes on with it.
    fn reach_library_val(&mut self, cx: Cx, s: SymId) {
        if !self.library || self.link_member(cx, s) {
            return;
        }
        let in_library = match cx.syms.sym(s).owner {
            Owner::Class(c) => self.library_classes.get(c.idx()).copied().unwrap_or(false),
            _ => false,
        };
        if in_library && !std::mem::replace(&mut self.asked[s.idx()], true) {
            self.pending.push(s);
        }
    }

    fn reach_array_seq(&mut self, cx: Cx) {
        if let Some(c) = self.array_seq {
            self.reach_class(cx, c);
        } else {
            self.wants_array_seq = true;
        }
    }

    /// The eager top-level vals of a file are initialised together, so one read keeps them all.
    fn reach_file(&mut self, cx: Cx, file: FileId) {
        // A std file entered while bodies were typed, or a package block, may lie past the
        // files the table was sized for.
        if self.reach.files.len() <= file.0 as usize {
            self.reach.files.resize(file.0 as usize + 1, false);
        }
        let Some(slot) = self.reach.files.get_mut(file.0 as usize) else { return };
        if *slot {
            return;
        }
        *slot = true;
        let Some(g) = self.layout.file_groups.iter().position(|(f, _)| *f == file) else { return };
        let region = self.region.replace(file);
        for i in 0..self.layout.file_groups[g].1.len() {
            let (sym, init) = self.layout.file_groups[g].1[i];
            if is_eager_top_val(cx.syms, sym, &self.layout.const_vals) {
                let outer = self.place.replace(Place::Val);
                self.walk(cx, init);
                self.place = outer;
            }
        }
        self.region = region;
    }

    fn reach_val(&mut self, cx: Cx, sym: SymId, init: TExprId) {
        if self.reach.vals[sym.idx()] {
            return;
        }
        self.reach.vals[sym.idx()] = true;
        let outer = self.place.replace(Place::Val);
        let region = self.region.take();
        self.walk(cx, init);
        self.region = region;
        self.place = outer;
    }

    /// A top-level definition whose access runs its file's initialiser first reaches the
    /// initialiser (`Reach::triggers`). The JVM's class initialisation is the JVM's.
    fn note_trigger(&mut self, cx: Cx, sym: SymId) {
        let file = cx.syms.sym(sym).file;
        if self.has_initialiser(cx, file) && initialises_file(cx.syms, sym, cx.typer.constant_value(sym).is_some()) {
            self.reach.triggers[sym.idx()] = true;
            self.reach_file(cx, file);
        }
    }

    fn note_trigger_class(&mut self, cx: Cx, c: ClassId) {
        let file = cx.syms.class(c).file;
        if !self.reach.trigger_classes[c.idx()] && self.has_initialiser(cx, file) && given_class_initialises_file(cx.syms, c) {
            self.reach.trigger_classes[c.idx()] = true;
            self.reach_file(cx, file);
        }
    }

    /// A file with eager top-level vals whose initialiser the output runs on any access of the
    /// file's definitions. The JVM's class initialisation is the JVM's.
    fn has_initialiser(&self, cx: Cx, file: FileId) -> bool {
        self.jvm.is_none() && self.layout.file_inits.get(file.0 as usize).copied().unwrap_or(false) && !cx.typer.std_source(file)
    }

    /// A definition reached by its path: a top-level def or val, a member of an object, an enum
    /// value or an object itself.
    fn reach_static(&mut self, cx: Cx, sym: SymId) {
        // A product's top-level definition the typer has not checked yet: asked for between two
        // walks, then reached again.
        if let Some(l) = cx.loaded.filter(|l| !l.product_package_members.is_empty()) {
            if let Some(&obj) = l.product_package_members.get(&sym) {
                let checked = index(self.top_val_of[sym.idx()]).is_some() || index(self.fun_of[sym.idx()]).is_some();
                if !checked {
                    if !std::mem::replace(&mut self.asked[sym.idx()], true) {
                        self.pending_packages.push((obj, sym));
                    }
                    return;
                }
            }
        }
        if std::mem::replace(&mut self.seen[sym.idx()], true) {
            return;
        }
        self.note_trigger(cx, sym);
        let info = cx.syms.sym(sym);
        if let Some(init) = index(self.top_val_of[sym.idx()]) {
            if is_eager_top_val(cx.syms, sym, &self.layout.const_vals) {
                self.reach_file(cx, info.file);
            } else {
                self.reach_val(cx, sym, TExprId(init as u32));
            }
            return;
        }
        match (info.kind, info.owner) {
            (SymKind::EnumValue(case), _) => self.reach_enum_value(cx, case),
            (SymKind::Object(c), _) => self.reach_class(cx, c),
            (SymKind::Def | SymKind::Given, Owner::Class(c)) => {
                self.reach_class(cx, c);
                let name = self.output_name(cx, sym);
                self.call_name(cx, name, Some(c));
            }
            (SymKind::Def | SymKind::Given, _) => self.reach_fun_of(cx, sym),
            (SymKind::Val, Owner::Class(c)) => {
                self.reach_class(cx, c);
                self.reach_library_val(cx, sym);
            }
            (_, Owner::Class(c)) => self.reach_class(cx, c),
            _ => {}
        }
    }

    /// An enum value is read from its companion when the enum is stateful, and next to a
    /// companion with a body, which is touched first.
    fn reach_enum_value(&mut self, cx: Cx, case: ClassId) {
        self.reach_class(cx, case);
        let Owner::Class(companion) = cx.syms.class(case).owner else { return };
        let stateful = cx.syms.class(companion).companion.map_or(false, |e| cx.syms.class(e).stateful);
        if stateful || self.layout.has_body(companion) {
            self.reach_class(cx, companion);
        }
    }

    /// A call of the member `s` through a receiver of its static class. A field read of a def is
    /// one as well: a parameterless def of a JS class is a getter.
    fn call_member(&mut self, cx: Cx, s: SymId) {
        if std::mem::replace(&mut self.called_syms[s.idx()], true) {
            return;
        }
        self.reach.declared[s.idx()] = true;
        self.link_member(cx, s);
        let info = cx.syms.sym(s);
        let owner = match info.owner {
            Owner::Class(c) => Some(c),
            _ => None,
        };
        if info.kind == SymKind::Val {
            self.reach_library_val(cx, s);
            // A library class's val read through a parent answers with the val of the class
            // the instance has: an inner object implementing an abstract val is one.
            if let Some(c) = owner.filter(|c| self.library_classes.get(c.idx()).copied().unwrap_or(false)) {
                self.call_name(cx, info.name, Some(c));
            }
        }
        let abstract_var = info.kind == SymKind::Var && info.needs_accessor && !cx.syms.js_member(s) && crate::typer::setters::is_abstract_var(cx.syms, cx.interner, s);
        if !matches!(info.kind, SymKind::Def | SymKind::Given) && !abstract_var {
            return;
        }
        // A private constructor parameter is read as itself: no definition of the name above
        // or below answers for it (`Month.name` next to `java.lang.Enum.name()`).
        if info.mods & mods::PRIVATE != 0 && owner.map_or(false, |c| is_ctor_param(cx.syms, c, s)) {
            self.reach_fun_of(cx, s);
            return;
        }
        let name = self.output_name(cx, s);
        self.call_name(cx, name, owner);
    }

    /// The name a call of `s` is recorded under: that of the output.
    fn output_name(&mut self, cx: Cx, s: SymId) -> Name {
        let info = cx.syms.sym(s);
        match info.dispatch {
            Dispatch::Named => {
                let n = cx.syms.dispatch_name(s);
                self.overloaded.insert(n, info.name);
                n
            }
            _ => info.name,
        }
    }

    /// The calls recorded under the plain name of `root`, renamed since, through its class or
    /// a class below it, recorded again under its new name.
    fn recall_renamed(&mut self, cx: Cx, root: SymId, renamed: Name) {
        let plain = cx.syms.sym(root).name;
        let Owner::Class(rc) = cx.syms.sym(root).owner else { return };
        if renamed == plain {
            return;
        }
        self.overloaded.insert(renamed, plain);
        if self.called.contains_key(&(plain, ANY_OWNER)) {
            self.call_name(cx, renamed, None);
            return;
        }
        let owners: Vec<ClassId> = self
            .called
            .keys()
            .filter(|&&(n, k)| n == plain && k != ANY_OWNER && cx.syms.class(k).base_types.iter().any(|&(b, _)| b == rc))
            .map(|&(_, k)| k)
            .collect();
        for k in owners {
            self.call_name(cx, renamed, Some(k));
        }
    }

    /// A call of the member `name` through a receiver of class `owner`, or of any class.
    fn call_name(&mut self, cx: Cx, name: Name, owner: Option<ClassId>) {
        let owner = owner.unwrap_or(ANY_OWNER);
        if self.called.contains_key(&(name, ANY_OWNER)) || self.called.insert((name, owner), ()).is_some() {
            return;
        }
        if owner == ANY_OWNER {
            self.called_any.push(name);
            for c in 0..self.reach.classes.len() {
                if self.reach.classes[c] {
                    self.mark_member(cx, ClassId(c as u32), name);
                }
            }
            return;
        }
        self.called_through.push(owner, name);
        let mut i = self.descendants.heads[owner.idx()];
        while let Some(&(d, next)) = index(i).map(|i| &self.descendants.nodes[i]) {
            self.mark_dispatch(cx, d, name);
            self.mark_bridge(cx, d, name);
            i = next;
        }
    }

    /// A call of `name` on an instance of `d` runs the first definition with a body in the
    /// linearisation; on JavaScript the definitions above it stay out, unless a `super` call
    /// names them. The JVM keeps every definition: a call site names the interface it goes
    /// through, which has to declare the method.
    fn mark_dispatch(&mut self, cx: Cx, d: ClassId, name: Name) {
        let info = cx.syms.class(d);
        // A trait or an abstract class has no instances of its own, except the traits whose
        // instances the runtime makes from a function (`$pf`), which keep every definition.
        let runtime_made = cx.prog.partial_function == Some(d) || super::is_function_class(cx.syms, cx.interner, d);
        if (info.kind == ClassKind::Trait || info.mods & mods::ABSTRACT != 0) && !runtime_made {
            return;
        }
        let every = self.jvm.is_some() || self.super_called.contains_key(&name) || runtime_made;
        let mut found = false;
        for &(a, _) in &cx.syms.class(d).base_types {
            if found {
                self.mark_extensions(cx, a, name);
            } else {
                found = self.mark_member(cx, a, name) && !every;
            }
        }
    }

    /// `super.m(...)`: the definitions of `m` above the one that runs are targets too.
    fn note_super_call(&mut self, cx: Cx, s: SymId) {
        let name = self.output_name(cx, s);
        if self.super_called.insert(name, ()).is_none() {
            self.call_name(cx, name, None);
        }
    }

    fn concrete(&self, cx: Cx, s: SymId) -> bool {
        match self.abstract_syms.get(s.idx()) {
            Some(&abs) => !abs,
            None => cx.syms.sym(s).mods & mods::ABSTRACT == 0,
        }
    }

    /// A call of `name` on an instance of `c` may go through a bridge of `c` to a method of
    /// another name.
    fn mark_bridge(&mut self, cx: Cx, c: ClassId, name: Name) {
        if !cx.syms.class(c).has_overloads {
            return;
        }
        let Some(idx) = index(self.tclass_of[c.idx()]) else { return };
        for &(declared, implementation) in &cx.prog.classes[idx].bridges {
            if cx.syms.dispatch_name(declared) == name {
                self.reach.bridged[implementation.idx()] = true;
                self.reach_fun_of(cx, implementation);
            }
        }
    }

    /// Keeps the member that class `c` declares under the name `name` of the output, and its
    /// extensions of that name; whether a member with a body was kept. Of an overloaded name
    /// only the alternative called so is kept.
    fn mark_member(&mut self, cx: Cx, c: ClassId, name: Name) -> bool {
        // The member of the output's name is looked for under that name too, not only under the
        // source name the map gives it: the map is the program's, by output name alone, and
        // `@targetName("value") def f` makes it `value -> f` for an unrelated class's plain `value`.
        let source_name = match self.overloaded.is_empty() {
            true => name,
            false => self.overloaded.get(&name).copied().unwrap_or(name),
        };
        let mut found = false;
        if source_name != name {
            found = self.mark_member_named(cx, c, name, name);
        }
        found |= self.mark_member_named(cx, c, name, source_name);
        self.mark_extensions(cx, c, name);
        found
    }

    /// `mark_member`'s keeping of the members class `c` declares under the source name
    /// `source_name` that the output calls `name`.
    fn mark_member_named(&mut self, cx: Cx, c: ClassId, name: Name, source_name: Name) -> bool {
        let info = cx.syms.class(c);
        // The alternatives of a library class are named for the output once the class is
        // compiled, after this round: until then every alternative of the name counts as called
        // and the definitions above stay in.
        let unnamed = self.library_classes.get(c.idx()).copied().unwrap_or(false) && !info.named_for_output;
        let mut found = false;
        if let Some(&m) = info.members.get(&source_name) {
            // A private constructor parameter answers no call by name: a call goes past it to
            // the definition above (`java.lang.Enum.name` past an enum's private `name`).
            let called = |s: SymId| {
                !(cx.syms.sym(s).mods & mods::PRIVATE != 0 && is_ctor_param(cx.syms, c, s)) && (unnamed || cx.syms.dispatch_name(s) == name)
            };
            match cx.syms.alternatives(m) {
                None if called(m) => {
                    if cx.syms.sym(m).kind == SymKind::Val {
                        self.reach_library_val(cx, m);
                    }
                    self.reach_fun_of(cx, m);
                    found = self.concrete(cx, m) && !unnamed;
                }
                None => {}
                Some(alts) => {
                    for &a in alts {
                        if cx.syms.sym(a).owner == Owner::Class(c) && called(a) {
                            self.reach_fun_of(cx, a);
                            found |= self.concrete(cx, a) && !unnamed;
                        }
                    }
                }
            }
        }
        found
    }

    fn mark_extensions(&mut self, cx: Cx, c: ClassId, name: Name) {
        for &e in &cx.syms.class(c).extensions {
            if cx.syms.sym(e).name == name {
                self.reach_fun_of(cx, e);
            }
        }
    }

    /// In link mode, the jars' bytecode may call any member a jar class declares: a member of
    /// class `c` that overrides one is kept, with its bridges, as if called on `c`.
    fn keep_jar_overrides(&mut self, cx: Cx, c: ClassId) {
        let syms = cx.syms;
        let info = syms.class(c);
        let jar_bases: Vec<ClassId> = info.base_types.iter().skip(1).map(|&(b, _)| b).filter(|&b| self.is_library_class(b)).collect();
        if jar_bases.is_empty() {
            return;
        }
        let mut own: Vec<SymId> = Vec::new();
        for &m in info.members.values() {
            match syms.alternatives(m) {
                Some(alts) => own.extend(alts.iter().copied().filter(|&a| syms.sym(a).owner == Owner::Class(c))),
                None if syms.sym(m).owner == Owner::Class(c) => own.push(m),
                None => {}
            }
        }
        for m in own {
            let name = syms.sym(m).name;
            if jar_bases.iter().any(|&b| syms.class(b).members.contains_key(&name)) {
                let n = self.output_name(cx, m);
                self.call_name(cx, n, Some(c));
                self.mark_bridge(cx, c, n);
            }
        }
    }

    fn process_class(&mut self, cx: Cx, c: ClassId) {
        self.process_class_from(cx, c, 0);
    }

    /// The walk of a class from the `from`th of its base types on: a library class is walked
    /// once when it is reached, before the typer has checked it and its base types are known,
    /// and again after the check for the base types that were not registered then.
    fn process_class_from(&mut self, cx: Cx, c: ClassId, from: usize) {
        let syms = cx.syms;
        let info = syms.class(c);
        let mut names: Vec<Name> = Vec::new();
        for (i, &(a, _)) in info.base_types.iter().enumerate() {
            if i >= from {
                self.descendants.push(a, c);
            }
            for n in self.called_through.iter(a) {
                if !names.contains(&n) {
                    names.push(n);
                }
            }
        }
        self.bases_registered[c.idx()] = info.base_types.len() as u32;
        for &n in &names {
            self.mark_dispatch(cx, c, n);
            self.mark_bridge(cx, c, n);
        }
        for i in 0..self.called_any.len() {
            self.mark_member(cx, c, self.called_any[i]);
            self.mark_bridge(cx, c, self.called_any[i]);
        }
        // What the runtime calls runs the definition an instance takes, as a call does.
        for n in RUNTIME_CALLED {
            self.mark_dispatch(cx, c, n);
        }
        for &(b, _) in info.base_types.iter().skip(1) {
            self.reach_class(cx, b);
        }
        let callable = !matches!(info.kind, ClassKind::Trait | ClassKind::Enum)
            && info.base_types.iter().skip(1).any(|&(b, _)| super::is_function_class(syms, cx.interner, b));
        if callable {
            self.call_name(cx, names::APPLY, Some(c));
        }
        // The JDK's own code calls the members of its interfaces (`Optional.map` its
        // `Function.apply`), whether the interface was read from ct.sym or is a std trait that
        // stands for it (`@jvmClass`).
        let java_bases: Vec<ClassId> = info
            .base_types
            .iter()
            .skip(1)
            .map(|&(b, _)| b)
            .filter(|&b| self.java_classes.get(b.idx()).copied().unwrap_or(false) || self.is_jdk_class(b))
            .collect();
        for b in java_bases {
            let names: Vec<Name> = syms.class(b).members.keys().copied().collect();
            for n in names {
                self.call_name(cx, n, Some(c));
            }
        }
        if self.link && !self.is_library_class(c) {
            self.keep_jar_overrides(cx, c);
        }
        if info.kind == ClassKind::Object {
            for (_, nested) in super::implemented_nested_objects(syms, c) {
                self.reach_class(cx, nested);
            }
            if let Some(e) = info.companion.map(|e| syms.class(e)).filter(|e| e.stateful) {
                for &case in &e.children {
                    if syms.class(case).singleton.is_some() {
                        self.reach_class(cx, case);
                    }
                }
            }
        }
        if let Some(companion) = super::touched_companion(syms, &self.layout.has_body, c) {
            self.reach_class(cx, companion);
        }
        // A value of a stateful enum lives in the companion, which is known once the enum is
        // complete: a jar's enum completes after its value was first reached.
        if info.kind == ClassKind::EnumCase && info.singleton.is_some() {
            if let Owner::Class(companion) = info.owner {
                if syms.class(companion).companion.map_or(false, |e| syms.class(e).stateful) {
                    self.reach_class(cx, companion);
                }
            }
        }
        self.process_init(cx, c);
    }

    /// The IR of the class: constructor defaults, the parent call, the field initialisers and
    /// statements, forwarders and super accessors.
    fn is_jdk_class(&self, c: ClassId) -> bool {
        self.jvm.as_ref().map_or(false, |j| j.jdk_classes.get(c.idx()).copied().unwrap_or(false))
    }

    fn process_init(&mut self, cx: Cx, c: ClassId) {
        if self.is_jdk_class(c) {
            return;
        }
        let syms = cx.syms;
        let Some(idx) = index(self.tclass_of[c.idx()]) else { return };
        let tc = &cx.prog.classes[idx];
        for &d in tc.ctor_defaults.iter().flatten() {
            self.walk(cx, d);
        }
        if let Some(args) = tc.parent_args {
            self.walk_list(cx, args);
        }
        if let Some(via) = tc.parent_via {
            self.reach_fun_of(cx, via);
        }
        for stmt in &cx.prog.stmts[tc.parent_prelude.range()] {
            if let TStmt::Val(_, e) = *stmt {
                self.walk(cx, e);
            }
        }
        for init in &tc.init {
            // A lazy val runs as a getter, the rest as the constructor or `$body`.
            self.in_init = !matches!(*init, TInit::Field(s, _) if syms.sym(s).mods & mods::LAZY != 0);
            match *init {
                TInit::Field(_, e) | TInit::Stmt(e) => self.walk(cx, e),
                TInit::Parent(b, call) => {
                    self.reach_class(cx, b);
                    self.walk_list(cx, call.args);
                    for stmt in &cx.prog.stmts[call.prelude.range()] {
                        if let TStmt::Val(_, e) = *stmt {
                            self.walk(cx, e);
                        }
                    }
                }
            }
        }
        self.in_init = false;
        for &(_, target) in &tc.forwarders {
            self.reach_static(cx, target);
        }
        for target in tc.super_accessors.iter().filter_map(|a| a.target) {
            self.reach_fun_of(cx, target);
        }
    }

    fn walk_list(&mut self, cx: Cx, l: ListRef) {
        visit_list(self, cx, l);
    }

    fn walk(&mut self, cx: Cx, e: TExprId) {
        visit(self, cx, e);
    }

    /// A registration keeps its class and the constructors it offers; the classes of the
    /// parameters are named, not kept.
    fn walk_registration(&mut self, cx: Cx, c: ClassId, e: TExprId) {
        self.current_fun = None;
        self.current_class = Some(c);
        self.current_val = None;
        self.place = Some(Place::Class(c));
        self.reach_class(cx, c);
        self.class_data_only = true;
        self.walk(cx, e);
        self.class_data_only = false;
    }

    /// The wrapping of a caught value needs `Throwable` and `JavaScriptException`.
    fn reach_js_exception(&mut self, cx: Cx) {
        for c in [cx.prog.throwable, cx.prog.js_exception].into_iter().flatten() {
            self.reach_class(cx, c);
        }
    }

    /// A program that catches exceptions gets the classes the runtime throws, so that a caught
    /// failure of the runtime is the exception the JVM would give; a program without a catch
    /// cannot tell them from plain errors and is spared them.
    fn reach_runtime_exceptions(&mut self, cx: Cx) {
        if self.runtime_exceptions_reached {
            return;
        }
        if !self.runtime_exceptions_found {
            // Looked up between two walks, where the typer can enter their files.
            self.wants_runtime_exceptions = true;
            return;
        }
        self.runtime_exceptions_reached = true;
        for &c in &self.runtime_exceptions.clone() {
            self.reach_class(cx, c);
        }
    }

    /// The std's templates of the symbols entered since the last look.
    fn note_std_templates(&mut self, cx: Cx) {
        let Some(std) = self.std_templates.as_mut() else { return };
        let files = cx.typer.files.as_slice();
        for info in &cx.syms.syms[self.std_templates_upto..] {
            if let Some(t) = &info.intrinsic {
                if files.get(info.file.0 as usize).map_or(true, |f| f.is_std) {
                    std.insert(t.to_string(), ());
                }
            }
        }
        self.std_templates_upto = cx.syms.syms.len();
    }

    /// Whether a template is a std member's, among them those of a std file entered during the
    /// walk (a product's body typed as the walk reaches it).
    fn is_std_template(&mut self, cx: Cx, text: &str) -> bool {
        if self.std_templates.as_ref().map_or(false, |std| std.contains_key(text)) {
            return true;
        }
        if self.std_templates_upto == cx.syms.syms.len() {
            return false;
        }
        self.note_std_templates(cx);
        self.std_templates.as_ref().map_or(false, |std| std.contains_key(text))
    }

    /// The property names a template of the program accesses, `$0.name`.
    fn template_names(&mut self, text: &str) {
        let bytes = text.as_bytes();
        let mut i = 0;
        while i + 1 < bytes.len() {
            if bytes[i] == b'.' && (bytes[i + 1].is_ascii_alphabetic() || bytes[i + 1] == b'_' || bytes[i + 1] == b'$') {
                let start = i + 1;
                let mut end = start;
                while end < bytes.len() && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_' || bytes[end] == b'$') {
                    end += 1;
                }
                self.reach.js_names.push(text[start..end].to_string());
                i = end;
            } else {
                i += 1;
            }
        }
    }

    /// What a `@jvm` template names in `std/jvm.scala`: `rtcall name(desc)` a definition,
    /// `scala/runtime/Class` a class and `scala/runtime/Class.name(desc)` a member of it.
    fn reach_jvm_template(&mut self, cx: Cx, text: &str) {
        let mut after_rtcall = false;
        for token in text.split_whitespace() {
            let Some(j) = &self.jvm else { return };
            if after_rtcall {
                let name = token.split('(').next().unwrap_or(token);
                if let Some(&s) = j.terms.get(name) {
                    self.reach_static(cx, s);
                }
            } else if matches!(token, "array_apply" | "array_update" | "array_length" | "array_clone") {
                // An array whose kind only the value knows is read through the lean runtime's
                // definition of the name; link mode has scala-library's and none of its own.
                if let Some(&s) = j.terms.get(token) {
                    self.reach_static(cx, s);
                }
            } else if let Some(rest) = token.strip_prefix("scala/runtime/") {
                let (class, member) = match rest.split_once('.') {
                    Some((c, m)) => (c, m.split('(').next()),
                    None => (rest, None),
                };
                if let Some(&c) = j.classes.get(class) {
                    self.reach_class(cx, c);
                    let found = member.and_then(|m| {
                        cx.syms.class(c).member_order.iter().map(|&s| cx.syms.sym(s).name).find(|&n| cx.interner.get(n) == m)
                    });
                    if let Some(name) = found {
                        self.call_name(cx, name, Some(c));
                    }
                }
            }
            after_rtcall = token == "rtcall";
        }
    }
}

/// What the walk of an expression meets, in the order it meets it (`visit`): the walker acts on
/// each, the recorder of a unit's walk (`kept.rs`) notes it.
pub(super) trait Meet {
    /// An inline expansion's root.
    fn expansion(&mut self, e: TExprId);
    fn js_global(&mut self, name: Name);
    fn local(&mut self, s: SymId);
    /// A class named as a value, a `super` target, a type test or a pattern.
    fn class(&mut self, cx: Cx, c: ClassId);
    fn static_ref(&mut self, cx: Cx, s: SymId);
    fn class_of(&mut self, cx: Cx, c: ClassId);
    fn field(&mut self, cx: Cx, of_this: bool, s: SymId);
    fn call_static(&mut self, cx: Cx, s: SymId);
    fn call_method(&mut self, cx: Cx, through_super: bool, s: SymId);
    /// A call of a JavaScript member by name.
    fn call_js(&mut self, cx: Cx, name: Name);
    fn js_select(&mut self, cx: Cx, on: TExprId, name: Name);
    fn new(&mut self, cx: Cx, c: ClassId);
    fn new_via(&mut self, cx: Cx, s: SymId);
    fn lambda(&mut self, cx: Cx, forwarded: Option<SymId>);
    /// Whether an object in statement position is left out.
    fn object_left_out(&mut self, c: ClassId) -> bool;
    fn local_fun(&mut self, cx: Cx, f: FunId);
    fn assign_field(&mut self, s: SymId);
    fn seq_lit(&mut self, cx: Cx);
    fn template(&mut self, cx: Cx, template: StrRef, args: ListRef);
    fn test_array(&mut self, cx: Cx);
    fn js_import(&mut self, i: u32);
    fn throw_unwrap(&mut self, cx: Cx);
    fn try_end(&mut self, cx: Cx, wraps: bool);
    fn pat_seq(&mut self, cx: Cx);
    fn pat_rest(&mut self, cx: Cx);
}

pub(super) fn visit_list<M: Meet>(m: &mut M, cx: Cx, l: ListRef) {
    for &e in cx.prog.expr_list(l) {
        visit(m, cx, e);
    }
}

pub(super) fn visit<M: Meet>(m: &mut M, cx: Cx, e: TExprId) {
    let prog = cx.prog;
    if prog.is_expansion(e) {
        m.expansion(e);
    }
    match prog.expr(e) {
        TExpr::Int(_)
        | TExpr::Long(_)
        | TExpr::Double(_)
        | TExpr::Bool(_)
        | TExpr::Char(_)
        | TExpr::Str(_)
        | TExpr::Unit
        | TExpr::This
        | TExpr::Null => {}
        TExpr::JsGlobal(name, _) => m.js_global(name),
        TExpr::Local(s) => m.local(s),
        TExpr::Super(target) => {
            if let SuperTarget::Class(c) = target {
                m.class(cx, c);
            }
        }
        TExpr::Static(s) => m.static_ref(cx, s),
        TExpr::Module(c) => m.class(cx, c),
        TExpr::ClassOf(c) => m.class_of(cx, c),
        TExpr::Field(r, s) => {
            visit(m, cx, r);
            m.field(cx, matches!(prog.expr(r), TExpr::This), s);
        }
        TExpr::CallStatic(s, args) => {
            visit_list(m, cx, args);
            m.call_static(cx, s);
        }
        TExpr::CallMethod(r, s, args) => {
            visit(m, cx, r);
            visit_list(m, cx, args);
            m.call_method(cx, matches!(prog.expr(r), TExpr::Super(_)), s);
        }
        TExpr::CallClosure(f, args) => {
            if let TExpr::JsSelect(_, name) = prog.expr(f) {
                m.call_js(cx, name);
            }
            visit(m, cx, f);
            visit_list(m, cx, args);
        }
        TExpr::JsSelect(a, name) => {
            m.js_select(cx, a, name);
            visit(m, cx, a);
        }
        TExpr::New(c, args) => {
            visit_list(m, cx, args);
            m.new(cx, c);
        }
        TExpr::NewVia(s, args) => {
            visit_list(m, cx, args);
            m.new_via(cx, s);
        }
        TExpr::Lambda(params, body) => {
            visit(m, cx, body);
            m.lambda(cx, prog.forwarded_target(params, body));
        }
        TExpr::If(c, t, els) => {
            visit(m, cx, c);
            visit(m, cx, t);
            if let Some(x) = els {
                visit(m, cx, x);
            }
        }
        TExpr::While(c, body) => {
            visit(m, cx, c);
            visit(m, cx, body);
        }
        TExpr::Block(stmts, res) => {
            for s in &prog.stmts[stmts.range()] {
                match *s {
                    // An object without a body in statement position is left out of the output.
                    TStmt::Expr(x) if matches!(prog.expr(x), TExpr::Module(c) if m.object_left_out(c)) => {}
                    TStmt::Expr(x) | TStmt::Val(_, x) => visit(m, cx, x),
                    TStmt::Fun(f) => m.local_fun(cx, f),
                    TStmt::Pat(p, x) => {
                        visit_pat(m, cx, p);
                        visit(m, cx, x);
                    }
                }
            }
            visit(m, cx, res);
        }
        TExpr::Assign(t, v) => {
            visit(m, cx, t);
            visit(m, cx, v);
            if let TExpr::Field(_, s) = prog.expr(t) {
                m.assign_field(s);
            }
        }
        TExpr::Match(scrut, cases) => {
            visit(m, cx, scrut);
            for case in &prog.cases[cases.range()] {
                visit_pat(m, cx, case.pat);
                if let Some(g) = case.guard {
                    visit(m, cx, g);
                }
                visit(m, cx, case.body);
            }
        }
        TExpr::Prim(_, a, b) => {
            visit(m, cx, a);
            visit(m, cx, b);
        }
        TExpr::Unary(_, a) | TExpr::ToStr(a, _) | TExpr::Index(a, _) | TExpr::Spread(a) | TExpr::Return(a) => visit(m, cx, a),
        TExpr::StrConcat(items) | TExpr::ArrayLit(items) | TExpr::ObjLit(items) => visit_list(m, cx, items),
        TExpr::SeqLit(items) => {
            visit_list(m, cx, items);
            m.seq_lit(cx);
        }
        TExpr::Js(template, args) => {
            visit_list(m, cx, args);
            m.template(cx, template, args);
        }
        TExpr::TypeTest(inner, test) => {
            visit(m, cx, inner);
            visit_test(m, cx, test);
        }
        TExpr::JsImport(i) => m.js_import(i),
        TExpr::Throw(inner, unwrap) => {
            visit(m, cx, inner);
            if unwrap {
                m.throw_unwrap(cx);
            }
        }
        TExpr::Try(i) => {
            let t = &prog.tries[i as usize];
            visit(m, cx, t.body);
            for case in &prog.cases[t.cases.range()] {
                visit_pat(m, cx, case.pat);
                if let Some(g) = case.guard {
                    visit(m, cx, g);
                }
                visit(m, cx, case.body);
            }
            if let Some(f) = t.finalizer {
                visit(m, cx, f);
            }
            m.try_end(cx, t.wraps);
        }
        TExpr::Splice(_) => unreachable!("a stored inline body is emitted nowhere"),
    }
}

fn visit_test<M: Meet>(m: &mut M, cx: Cx, test: TestId) {
    match cx.prog.tests[test.idx()] {
        TypeTest::Class(c) => m.class(cx, c),
        TypeTest::Array => m.test_array(cx),
        TypeTest::Value(e) => visit(m, cx, e),
        TypeTest::Or(a, b) | TypeTest::And(a, b) => {
            visit_test(m, cx, a);
            visit_test(m, cx, b);
        }
        _ => {}
    }
}

fn visit_pats<M: Meet>(m: &mut M, cx: Cx, l: ListRef) {
    for &p in &cx.prog.pat_lists[l.range()] {
        visit_pat(m, cx, p);
    }
}

fn visit_pat<M: Meet>(m: &mut M, cx: Cx, pat: TPatId) {
    match cx.prog.pats[pat.idx()] {
        TPat::Wildcard => {}
        TPat::Bind(_, inner) => {
            if let Some(i) = inner {
                visit_pat(m, cx, i);
            }
        }
        TPat::Test(test, _, inner) => {
            visit_test(m, cx, test);
            visit_pat(m, cx, inner);
        }
        TPat::Equals(e, _) => visit(m, cx, e),
        TPat::Class(c, _, _, subs) => {
            m.class(cx, c);
            visit_pats(m, cx, subs);
        }
        TPat::Alt(alts) => visit_pats(m, cx, alts),
        TPat::Unapply(_, call, inner) => {
            visit(m, cx, call);
            visit_pat(m, cx, inner);
        }
        TPat::Seq(items, rest) => {
            m.pat_seq(cx);
            visit_pats(m, cx, items);
            if let Some(r) = rest {
                visit_pat(m, cx, r);
                m.pat_rest(cx);
            }
        }
    }
}

impl Meet for Walker {
    fn expansion(&mut self, e: TExprId) {
        if let Some(p) = self.place {
            self.reach.roots.push((e, p));
        }
    }

    fn js_global(&mut self, name: Name) {
        self.reach.globals.push(name);
    }

    fn local(&mut self, s: SymId) {
        if self.in_init {
            self.reach.init_read[s.idx()] = true;
        }
    }

    fn class(&mut self, cx: Cx, c: ClassId) {
        self.reach_class(cx, c);
    }

    fn static_ref(&mut self, cx: Cx, s: SymId) {
        self.reach_static(cx, s);
    }

    fn class_of(&mut self, cx: Cx, c: ClassId) {
        self.reach.uses_class_of = true;
        if !self.class_data_only {
            self.reach_class(cx, c);
        }
    }

    fn field(&mut self, cx: Cx, of_this: bool, s: SymId) {
        if self.in_init && of_this {
            self.reach.init_read[s.idx()] = true;
        } else {
            self.reach.fields_read[s.idx()] = true;
        }
        self.call_member(cx, s);
    }

    fn call_static(&mut self, cx: Cx, s: SymId) {
        if !self.seen[s.idx()] && cx.syms.sym(s).owner != Owner::Local {
            self.reach_static(cx, s);
        }
    }

    fn call_method(&mut self, cx: Cx, through_super: bool, s: SymId) {
        if through_super {
            self.note_super_call(cx, s);
        }
        self.call_member(cx, s);
    }

    fn call_js(&mut self, cx: Cx, name: Name) {
        self.call_name(cx, name, None);
    }

    fn js_select(&mut self, cx: Cx, on: TExprId, name: Name) {
        if self.std_templates.is_some() {
            self.reach.js_names.push(cx.interner.get(name).to_string());
        }
        if matches!(cx.prog.expr(on), TExpr::Module(c) if cx.syms.class(c).js_binding == Some(JsBinding::GlobalScope)) {
            self.reach.globals.push(name);
        }
    }

    fn new(&mut self, cx: Cx, c: ClassId) {
        self.reach_class(cx, c);
        self.note_trigger_class(cx, c);
    }

    fn new_via(&mut self, cx: Cx, s: SymId) {
        if let Owner::Class(c) = cx.syms.sym(s).owner {
            self.reach_class(cx, c);
        }
        self.reach_fun_of(cx, s);
    }

    fn lambda(&mut self, cx: Cx, forwarded: Option<SymId>) {
        if let Some(target) = forwarded {
            let info = cx.syms.sym(target);
            if matches!(info.owner, Owner::Package(_)) && self.region != Some(info.file) {
                self.reach.forwarded[target.idx()] = true;
            }
        }
    }

    fn object_left_out(&mut self, c: ClassId) -> bool {
        !self.layout.has_body(c)
    }

    fn local_fun(&mut self, _cx: Cx, f: FunId) {
        if let Some(p) = self.place {
            self.local_places.entry(f).or_insert(p);
        }
        if let Some(file) = self.region {
            self.local_regions.entry(f).or_insert(file);
        }
        self.reach_fun(f)
    }

    fn assign_field(&mut self, s: SymId) {
        self.reach.fields_read[s.idx()] = true;
    }

    fn seq_lit(&mut self, cx: Cx) {
        self.reach_array_seq(cx);
    }

    fn template(&mut self, cx: Cx, template: StrRef, args: ListRef) {
        let prog = cx.prog;
        let (word, bit) = (template.idx() / 64, 1u64 << (template.idx() % 64));
        if word >= self.templates_seen.len() {
            self.templates_seen.resize(word + 1, 0);
        }
        if self.templates_seen[word] & bit == 0 {
            self.templates_seen[word] |= bit;
            self.reach.templates.push(template);
        }
        let text = &prog.strings[template.idx()];
        if self.std_templates.is_some() {
            if !self.is_std_template(cx, text) {
                self.template_names(text);
            }
            // A string a template takes may be a property name, `js.get(o, "y")`.
            for &a in prog.expr_list(args) {
                if let TExpr::Str(k) = prog.expr(a) {
                    if super::is_js_identifier(&prog.strings[k.idx()]) {
                        self.reach.js_names.push(prog.strings[k.idx()].clone());
                    }
                }
            }
        }
        // The array sorts call `compare` on the comparator they are given.
        if text.contains("$sortArray(") || text.contains("$sortRange(") {
            self.call_name(cx, names::COMPARE, None);
        }
        if text.contains("$pf(") {
            if let Some(c) = prog.partial_function {
                self.reach_class(cx, c);
            }
            if let Some(c) = self.jvm.as_ref().and_then(|j| j.partial_function) {
                self.reach_class(cx, c);
            }
        }
        if self.jvm.is_some() && (text.contains("rtcall") || text.contains("scala/runtime/") || text.contains("array_")) {
            self.reach_jvm_template(cx, text);
        }
        if text.starts_with("$reflected") {
            self.wants_reflect = true;
        }
        // The runtime's `$withCause` puts `getCause` on an instance under that name.
        if text.starts_with("$withCause") {
            self.reach.js_names.push("getCause".to_string());
        }
        // A product's element: a class's own `productElement`, an override of a case class's
        // among them, is called by its name (`$productElement` in rt.js).
        if text.starts_with("$productElement(") {
            if let Some(n) = cx.interner.lookup("productElement") {
                self.call_name(cx, n, None);
            }
        }
        // A product's element names: the case classes carry theirs, a class of the
        // program that defines the method is called by its name.
        if text.starts_with("$productElementName") {
            self.reach.uses_element_names = true;
            self.reach.js_names.push("productElementName".to_string());
        }
        // A reflective structural call names the member with a literal: every method
        // of that name is kept.
        if text.starts_with("$memberByName") || text.starts_with("$callByName") {
            for &a in prog.expr_list(args) {
                if let TExpr::Str(k) = prog.expr(a) {
                    if let Some(n) = cx.interner.lookup(&prog.strings[k.idx()]) {
                        self.call_name(cx, n, None);
                    }
                }
            }
        }
        if text.starts_with("$isAssignableFrom") || text.starts_with("$isInstance") {
            self.reach.uses_assignable_from = true;
        }
        if text.starts_with("$enum") {
            for s in self.jvm.as_ref().map_or(Vec::new(), |j| j.enum_lookups.clone()) {
                self.reach_static(cx, s);
            }
        }
    }

    /// A test against an array of any kind asks the lean runtime on the JVM.
    fn test_array(&mut self, cx: Cx) {
        if let Some(s) = self.jvm.as_ref().and_then(|j| j.terms.get("isArray").copied()) {
            self.reach_static(cx, s);
        }
    }

    fn js_import(&mut self, i: u32) {
        self.reach.imports[i as usize] = true;
    }

    fn throw_unwrap(&mut self, cx: Cx) {
        self.reach_js_exception(cx);
    }

    fn try_end(&mut self, cx: Cx, wraps: bool) {
        if wraps {
            self.reach_js_exception(cx);
        }
        self.reach_runtime_exceptions(cx);
    }

    fn pat_seq(&mut self, cx: Cx) {
        if let Some(s) = self.jvm.as_ref().and_then(|j| j.seq_pattern) {
            self.reach_static(cx, s);
        }
    }

    fn pat_rest(&mut self, cx: Cx) {
        self.reach_array_seq(cx);
    }
}

/// The member names of the classes an exported definition takes or returns, and of the classes
/// those take or return in turn. The signatures are completed here, so that the set does not
/// depend on what an earlier build in watch mode happened to complete.
fn exported_members(typer: &mut Worker) -> Vec<String> {
    let mut classes: Vec<ClassId> = Vec::new();
    let mut seen = vec![false; typer.syms.classes.len()];
    let mut types: Vec<TypeId> = Vec::new();
    fn add_sig(typer: &mut Worker, s: SymId, types: &mut Vec<TypeId>) {
        if !matches!(typer.syms.sym(s).kind, SymKind::Def | SymKind::Val | SymKind::Given | SymKind::Param) {
            return;
        }
        let sig = typer.sig_of(s);
        types.push(sig.ret);
        types.extend(sig.clauses.iter().flat_map(|c| c.params.iter().map(|p| p.ty)));
    }
    let exports: Vec<SymId> = typer.prog.js_exports.iter().map(|&(s, _)| s).collect();
    for s in exports {
        add_sig(typer, s, &mut types);
        if let SymKind::Object(c) = typer.syms.sym(s).kind {
            classes.push(c);
        }
    }
    let mut names: Vec<String> = Vec::new();
    loop {
        while let Some(t) = types.pop() {
            match typer.types.get(t) {
                Type::Class(c, args) => {
                    // A signature completed here may have entered a std file with new classes.
                    if seen.len() <= c.idx() {
                        seen.resize(typer.syms.classes.len().max(c.idx() + 1), false);
                    }
                    if !std::mem::replace(&mut seen[c.idx()], true) {
                        classes.push(c);
                    }
                    types.extend(typer.types.items(args).iter().copied());
                }
                Type::AppVar(_, args) | Type::AppParam(_, args) => types.extend(typer.types.items(args).iter().copied()),
                Type::Union(a, b) | Type::Inter(a, b) => {
                    types.push(a);
                    types.push(b);
                }
                Type::Var(v) => types.extend(typer.tvars[v.idx()].inst),
                Type::Lambda(_, body) | Type::Poly(_, body) => types.push(body),
                Type::Alias(_, args) => types.extend(typer.types.items(args).iter().copied()),
                Type::Match(s, m) => {
                    types.push(s);
                    let info = typer.types.match_info(m);
                    types.extend(info.cases.iter().flat_map(|c| [c.pattern, c.body]));
                }
                _ => {}
            }
        }
        let Some(c) = classes.pop() else { break };
        typer.complete_class(c);
        if seen.len() < typer.syms.classes.len() {
            seen.resize(typer.syms.classes.len(), false);
        }
        let info = typer.syms.class(c);
        if info.js != JsKind::Scala {
            continue;
        }
        for &b in info.base_types.iter().map(|(b, _)| b) {
            if !std::mem::replace(&mut seen[b.idx()], true) {
                classes.push(b);
            }
        }
        let members: Vec<SymId> = info.member_order.iter().chain(info.extensions.iter()).copied().collect();
        for m in members {
            let sym = typer.syms.sym(m);
            if sym.mods & mods::PRIVATE != 0 {
                continue;
            }
            names.push(typer.interner.get(sym.name).to_string());
            add_sig(typer, m, &mut types);
        }
    }
    names
}

fn has_repeated_param(cx: Cx, sym: SymId) -> bool {
    let info = cx.syms.sym(sym);
    info.sig.as_ref().map_or(false, |sig| sig.clauses.iter().flat_map(|c| c.params.iter()).any(|p| p.repeated))
}

