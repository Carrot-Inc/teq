//! What the shape of the output depends on across definitions: which top-level vals are
//! constants, which files run an initialiser, which classes run a body, which module of the
//! output each definition belongs to, and the number each class carries at run time.
//!
//! Nothing here follows the order of the arenas. Those grow in typing order, which depends on
//! what was demanded first, and in a watch session across builds; the output instead lays
//! definitions out by where they stand in the source and numbers classes the same way, so that
//! the same sources give the same text whatever was typed when.
//!
//! An anonymous class that is a closure in all but name (`lower_closures`) is written as an
//! arrow function where it is created and appears in no module's class list.

use super::names::sanitize;
use super::reach::Reach;
use super::is_function_class;
use crate::ast::mods;
use crate::intern::{FxMap, Interner};
use crate::source::{FileId, SourceFile};
use crate::symbols::*;
use crate::tir::*;
use crate::types::*;
use std::cmp::Ordering;

/// One file of the output and the definitions it holds, as indices into `Program::classes`,
/// `Program::top_funs` and `Layout::file_groups`, plus the enum cases whose value is a constant
/// of the module. A single-file build has one module holding everything; the split output has
/// the standard library in `std`, then one module per package of the program.
pub struct Module {
    pub name: String,
    /// The scope its definitions are named in (`names::Naming`): the standard library, or the
    /// package, which the modules of its files share under `--module-per-file`.
    pub scope: u32,
    pub classes: Vec<usize>,
    pub funs: Vec<usize>,
    pub groups: Vec<usize>,
    pub enum_values: Vec<ClassId>,
    /// The functions of outlined expansions it holds, as indices into `Outline::funs`.
    pub outlined: Vec<usize>,
    /// Whether the module holds one file of a `--module-per-file` package, which a hot swap may
    /// re-execute on its own: `--hot` gives it the eager-init footer that re-runs `$hot()` once
    /// the page has booted (`module.rs`).
    pub per_file: bool,
}

impl Module {
    fn new(name: String, scope: u32) -> Module {
        Module { name, scope, classes: Vec::new(), funs: Vec::new(), groups: Vec::new(), enum_values: Vec::new(), outlined: Vec::new(), per_file: false }
    }

    pub fn items(&self) -> usize {
        self.classes.len() + self.funs.len() + self.groups.len() + self.outlined.len()
    }

    fn sort(&mut self, prog: &Program, syms: &Symbols, interner: &Interner) {
        self.classes.sort_by(|&a, &b| compare_classes(syms, interner, prog.classes[a].id, prog.classes[b].id));
        self.funs.sort_by(|&a, &b| compare_syms(syms, interner, prog.funs[prog.top_funs[a].idx()].sym, prog.funs[prog.top_funs[b].idx()].sym));
        order_product_enum_values(syms, &mut self.enum_values);
    }
}

impl Module {
    /// `class B extends A` reads `A` when it is evaluated, so a class moves behind its
    /// superclass where the source has them the other way round.
    fn superclasses_first(&mut self, prog: &Program, syms: &Symbols) {
        let superclass = |i: usize| {
            let info = syms.class(prog.classes[i].id);
            info.superclass.filter(|_| info.kind != ClassKind::Trait)
        };
        if !self.classes.iter().any(|&i| superclass(i).is_some()) {
            return;
        }
        let mut slot_of: FxMap<ClassId, usize> = FxMap::default();
        for (slot, &i) in self.classes.iter().enumerate() {
            slot_of.insert(prog.classes[i].id, slot);
        }
        let mut placed = vec![false; self.classes.len()];
        let mut order = Vec::with_capacity(self.classes.len());
        for slot in 0..self.classes.len() {
            let mut chain = Vec::new();
            let mut at = Some(slot);
            while let Some(s) = at.filter(|&s| !placed[s]) {
                placed[s] = true;
                chain.push(self.classes[s]);
                at = superclass(self.classes[s]).and_then(|p| slot_of.get(&p).copied());
            }
            order.extend(chain.into_iter().rev());
        }
        self.classes = order;
    }
}

/// A class the output holds: reached, and neither a builtin nor an opaque type. The modules
/// list those alone, so that the order and the chunking of the output depend on what is written
/// and not on which classes happened to be checked.
fn is_written(syms: &Symbols, reach: &Reach, c: ClassId) -> bool {
    !matches!(syms.class(c).kind, ClassKind::Opaque | ClassKind::Builtin) && reach.classes.get(c.idx()).copied().unwrap_or(false)
}

pub fn is_literal_init(prog: &Program, init: TExprId) -> bool {
    matches!(prog.expr(init), TExpr::Int(_) | TExpr::Double(_) | TExpr::Bool(_) | TExpr::Str(_) | TExpr::Char(_) | TExpr::Long(_))
}

/// The module files of the split output whose names a package cannot take.
const RESERVED_MODULES: &[&str] = &["main", "rt", "std", "hot-refresh"];
pub const STD_MODULE: &str = "std";

/// A class that is not numbered: nothing reached tests for it or registers it.
pub const UNNUMBERED: u32 = u32::MAX;

pub struct Layout {
    /// Top-level vals whose initializer is a literal are plain consts.
    pub const_vals: Vec<bool>,
    /// The top-level vals of each file in source order, the files in their order; compiler-made
    /// vals join the file they were made for.
    pub file_groups: Vec<(FileId, Vec<(SymId, TExprId)>)>,
    /// Per file: whether it has top-level vals that its initialiser `$file<tag>()` runs together.
    pub file_inits: Vec<bool>,
    /// Per class: whether its constructor runs statements or initialisers of the body.
    pub has_body: Vec<bool>,
    /// Per class: the number its registration and type tests use, `UNNUMBERED` for the rest.
    pub class_numbers: Vec<u32>,
    /// The numbers of the parents a product by rule has (`Symbols::is_synthetic_parent`:
    /// `scala.Product`, `scala.Equals`, `java.io.Serializable`) where a test asks for them, which
    /// such a product (`ClassInfo::is_product_by_rule`) carries without extending them.
    pub product_numbers: Vec<u32>,
    /// The number of the partial functions' trait where a test asks for it, which a partial
    /// function value carries on its prototype.
    pub partial_function_number: Option<u32>,
    /// How many of the arguments that create an anonymous class go on to its superclass; they
    /// follow what the class captures.
    pub anon_parent_args: FxMap<ClassId, u32>,
    /// How many of the arguments that create a named local class are its captures, which lead
    /// its own parameters.
    pub local_captures: FxMap<ClassId, u32>,
    /// The anonymous classes the JavaScript output writes as closures where they are created,
    /// each with its index in `Program::classes` (`lower_closures`).
    pub closure_anons: FxMap<ClassId, usize>,
    /// The anonymous classes made per expansion that one class stands for (`share`).
    pub shared: super::share::Shared,
}

impl Layout {
    pub fn new(prog: &Program, syms: &Symbols, interner: &Interner) -> Layout {
        let mut const_vals = vec![false; syms.syms.len()];
        for (sym, init) in &prog.top_vals {
            if is_literal_init(prog, *init) && syms.sym(*sym).kind == SymKind::Val {
                const_vals[sym.idx()] = true;
            }
        }
        let mut file_groups: Vec<(FileId, Vec<(SymId, TExprId)>)> = Vec::new();
        let mut group_of: Vec<u32> = Vec::new();
        let mut file_inits: Vec<bool> = Vec::new();
        for &(sym, init) in &prog.top_vals {
            let file = syms.sym(sym).file;
            let f = file.0 as usize;
            if group_of.len() <= f {
                group_of.resize(f + 1, 0);
            }
            if group_of[f] == 0 {
                file_groups.push((file, Vec::new()));
                group_of[f] = file_groups.len() as u32;
            }
            file_groups[group_of[f] as usize - 1].1.push((sym, init));
            if is_eager_top_val(syms, sym, &const_vals) {
                if file_inits.len() <= f {
                    file_inits.resize(f + 1, false);
                }
                file_inits[f] = true;
            }
        }
        // By the files' ranks, which are their ids but for a product's file of top-level
        // definitions, ranked by its source.
        file_groups.sort_by_key(|(file, _)| (syms.file_rank(*file), *file));
        for (_, vals) in &mut file_groups {
            vals.sort_by(|a, b| compare_syms(syms, interner, a.0, b.0));
        }
        let mut layout = Layout {
            const_vals,
            file_groups,
            file_inits,
            has_body: vec![false; syms.classes.len()],
            class_numbers: Vec::new(),
            product_numbers: Vec::new(),
            partial_function_number: None,
            anon_parent_args: FxMap::default(),
            local_captures: FxMap::default(),
            closure_anons: FxMap::default(),
            shared: Default::default(),
        };
        for tc in &prog.classes {
            layout.note_class(prog, syms, tc);
        }
        layout
    }

    /// Records a top-level val the typer added after the layout was made: one of a std file
    /// entered while the reach pass typed bodies. It joins its file's group in source order.
    pub fn note_top_val(&mut self, prog: &Program, syms: &Symbols, interner: &Interner, sym: SymId, init: TExprId) {
        if self.const_vals.len() < syms.syms.len() {
            self.const_vals.resize(syms.syms.len(), false);
        }
        self.const_vals[sym.idx()] = is_literal_init(prog, init) && syms.sym(sym).kind == SymKind::Val;
        let file = syms.sym(sym).file;
        let at = match self.file_groups.iter().position(|(f, _)| *f == file) {
            Some(i) => i,
            None => {
                let key = (syms.file_rank(file), file);
                let i = self.file_groups.partition_point(|(f, _)| (syms.file_rank(*f), *f) < key);
                self.file_groups.insert(i, (file, Vec::new()));
                i
            }
        };
        let vals = &mut self.file_groups[at].1;
        if !vals.iter().any(|&(s, _)| s == sym) {
            vals.push((sym, init));
            vals.sort_by(|a, b| compare_syms(syms, interner, a.0, b.0));
        }
        if is_eager_top_val(syms, sym, &self.const_vals) {
            let f = file.0 as usize;
            if self.file_inits.len() <= f {
                self.file_inits.resize(f + 1, false);
            }
            self.file_inits[f] = true;
        }
    }

    /// Records what the construction of `tc` runs; for a class the typer added after the layout
    /// was made, when the reach pass compiled it from a library.
    pub fn note_class(&mut self, prog: &Program, syms: &Symbols, tc: &TClass) {
        if self.has_body.len() <= tc.id.idx() {
            self.has_body.resize(syms.classes.len().max(tc.id.idx() + 1), false);
        }
        self.has_body[tc.id.idx()] = runs_body(prog, syms, tc);
        if let Some(args) = tc.parent_args.filter(|_| syms.class(tc.id).kind == ClassKind::Anon) {
            self.anon_parent_args.insert(tc.id, args.len);
        }
        if tc.captures > 0 && syms.class(tc.id).kind != ClassKind::Anon {
            self.local_captures.insert(tc.id, tc.captures as u32);
        }
    }

    /// Another module's class: its construction may run anything, as a class of its own
    /// module compiled from source has a parent to call.
    pub fn note_external(&mut self, c: ClassId) {
        if self.has_body.len() <= c.idx() {
            self.has_body.resize(c.idx() + 1, false);
        }
        self.has_body[c.idx()] = true;
    }

    pub fn has_body(&self, c: ClassId) -> bool {
        self.has_body.get(c.idx()).copied().unwrap_or(false)
    }

    /// Finds the anonymous classes that are closures in all but name: no superclass, one
    /// member, the `apply` of a function type among the parents, the other parents marker
    /// traits nothing tests for, a body that never names its own `this`, and a creation that
    /// passes locals and `this` alone. JavaScript writes such a class as an arrow function at
    /// the place that creates it, with no class, registration or number of its own; what
    /// distinguished the instance from a lambda was its class name alone. The class-based
    /// targets keep the class.
    pub fn lower_closures(&mut self, prog: &Program, syms: &Symbols, interner: &Interner, reach: &Reach) {
        let mut tested = vec![false; syms.classes.len()];
        for (i, test) in prog.tests.iter().enumerate() {
            if let TypeTest::Class(c) | TypeTest::Trait(c) = *test {
                if !prog.stored_tests.contains_key(&TestId(i as u32)) {
                    tested[c.idx()] = true;
                }
            }
        }
        for (i, pat) in prog.pats.iter().enumerate() {
            if let TPat::Class(c, ..) = *pat {
                if !prog.stored_pats.contains_key(&TPatId(i as u32)) {
                    tested[c.idx()] = true;
                }
            }
        }
        let mut created = vec![Created::Never; syms.classes.len()];
        for (c, plain) in anon_creations(prog, syms) {
            let at = &mut created[c.idx()];
            *at = match (*at, plain) {
                (Created::Never, true) => Created::WithLocals,
                (Created::WithLocals, true) => Created::WithLocals,
                _ => Created::Otherwise,
            };
        }
        let candidates: Vec<usize> = (0..prog.classes.len()).filter(|&i| reach.classes[prog.classes[i].id.idx()] && created[prog.classes[i].id.idx()] == Created::WithLocals).collect();
        let shaped = |slice: &[usize]| -> Vec<usize> { slice.iter().copied().filter(|&i| is_closure_shaped(prog, syms, interner, reach, &prog.classes[i], &tested)).collect() };
        let workers = (candidates.len() / 1024).clamp(1, crate::workers());
        let lowered: Vec<usize> = if workers == 1 {
            shaped(&candidates)
        } else {
            std::thread::scope(|scope| {
                let shaped = &shaped;
                let handles: Vec<_> = candidates.chunks(candidates.len().div_ceil(workers)).map(|slice| crate::alloc::spawn_in(scope, move || shaped(slice))).collect();
                handles.into_iter().flat_map(|h| h.join().expect("closure shape thread panicked")).collect()
            })
        };
        for i in lowered {
            self.closure_anons.insert(prog.classes[i].id, i);
        }
    }

    fn is_written(&self, syms: &Symbols, reach: &Reach, c: ClassId) -> bool {
        is_written(syms, reach, c) && !self.closure_anons.contains_key(&c) && !self.shared.stands_for_another(c)
    }

    /// Numbers the classes the output refers to at run time, the reached ones and the traits of
    /// the reached type tests and casts (`Reach::tested_traits`), in their canonical order: what
    /// no reached code tests changes no number.
    pub fn number_classes(&mut self, prog: &Program, syms: &Symbols, interner: &Interner, reach: &Reach) {
        let tested = |c: ClassId| reach.tested_traits.get(c.idx()).copied().unwrap_or(false);
        let mut numbered: Vec<ClassId> = (0..syms.classes.len())
            .map(|i| ClassId(i as u32))
            .filter(|&c| (reach.classes[c.idx()] && !self.closure_anons.contains_key(&c) && !self.shared.stands_for_another(c)) || (tested(c) && !reach.classes[c.idx()]))
            .collect();
        numbered.sort_by(|&a, &b| compare_classes(syms, interner, a, b));
        numbered.dedup();
        self.class_numbers = vec![UNNUMBERED; syms.classes.len()];
        for (n, c) in numbered.into_iter().enumerate() {
            self.class_numbers[c.idx()] = n as u32;
        }
        // The parents of the products by rule and the partial functions' trait where a reached
        // test of the output asks for them.
        self.product_numbers.clear();
        self.partial_function_number = None;
        for c in (0..syms.classes.len()).map(|i| ClassId(i as u32)).filter(|&c| tested(c)) {
            if prog.partial_function == Some(c) {
                self.partial_function_number = Some(self.class_numbers[c.idx()]);
            }
            if syms.is_synthetic_parent(interner, c) && !self.product_numbers.contains(&self.class_numbers[c.idx()]) {
                self.product_numbers.push(self.class_numbers[c.idx()]);
            }
        }
        self.product_numbers.sort_unstable();
    }

    /// Everything in one module.
    pub fn single_module(&self, prog: &Program, syms: &Symbols, interner: &Interner, reach: &Reach) -> Vec<Module> {
        let mut m = Module::new(String::new(), 0);
        m.classes = (0..prog.classes.len()).filter(|&i| self.is_written(syms, reach, prog.classes[i].id)).collect();
        m.funs = (0..prog.top_funs.len()).collect();
        m.groups = (0..self.file_groups.len()).collect();
        m.enum_values = (0..syms.classes.len()).map(|i| ClassId(i as u32)).filter(|&c| is_enum_constant(syms, reach, c)).collect();
        m.sort(prog, syms, interner);
        m.superclasses_first(prog, syms);
        vec![m]
    }

    /// The standard library first, then the modules of the program in the order of their names:
    /// one per package, or one per file for the packages under a `--module-per-file` prefix (`a.b.File` for
    /// `File.scala` in package `a.b`); a definition belongs to the module of the file it stands
    /// in, and an anonymous class that an inline expansion made to the module of the call site.
    pub fn package_modules(
        &self,
        prog: &Program,
        syms: &Symbols,
        interner: &Interner,
        reach: &Reach,
        files: &[SourceFile],
        file_pkgs: &[PkgId],
        module_per_file: &[String],
    ) -> (Vec<Module>, Vec<usize>, FxMap<SymId, usize>) {
        let mut pkg_names: Vec<Option<String>> = vec![None; syms.pkgs.len()];
        // The pseudo files of library classes follow the program's files: each goes into the
        // module of its package.
        // A product's pseudo file stands for its owning source, as the whole program has it.
        let name_of_file: Vec<Option<(String, u32, bool)>> = (0..file_pkgs.len())
            .map(|f| {
                let file = files.get(f);
                if file.map_or(false, |file| file.is_std) {
                    return None;
                }
                let p = file_pkgs[f];
                let pkg = pkg_names[p.idx()].get_or_insert_with(|| package_name(syms, interner, p)).clone();
                let path = file.map(|file| file.path.as_str()).or_else(|| syms.product_file_key(FileId(f as u32)));
                let per_file = path.is_some() && is_module_per_file(&pkg, module_per_file);
                let name = match path {
                    Some(path) if per_file => format!("{}.{}", pkg, file_stem(path)),
                    _ => module_name(pkg),
                };
                Some((name, p.0 + 1, per_file))
            })
            .collect();
        // A product's method whose expansions a function is outlined for, where the build holds
        // no pseudo file of its source: the module of its package and source, as the whole
        // program has it.
        let callee_names: Vec<(SymId, (String, u32, bool))> = syms
            .product_callee_sources
            .iter()
            .filter(|&(_, &source)| !syms.product_files.values().any(|&s| s == source))
            .filter_map(|(&callee, &source)| {
                let p = top_package(syms, callee)?;
                let pkg = pkg_names[p.idx()].get_or_insert_with(|| package_name(syms, interner, p)).clone();
                let per_file = is_module_per_file(&pkg, module_per_file);
                let name = match per_file {
                    true => format!("{}.{}", pkg, file_stem(&syms.product_sources[source as usize].2)),
                    false => module_name(pkg),
                };
                Some((callee, (name, p.0 + 1, per_file)))
            })
            .collect();
        let mut names: Vec<(&str, u32, bool)> = name_of_file.iter().flatten().chain(callee_names.iter().map(|(_, n)| n)).map(|(n, s, small)| (n.as_str(), *s, *small)).collect();
        names.sort_unstable();
        names.dedup();
        let mut modules = vec![Module::new(STD_MODULE.to_string(), 0)];
        let mut slot_of_name: FxMap<&str, usize> = FxMap::default();
        for (name, scope, per_file) in names {
            slot_of_name.insert(name, modules.len());
            let mut module = Module::new(name.to_string(), scope);
            module.per_file = per_file;
            modules.push(module);
        }
        let module_of = |f: FileId| name_of_file.get(f.0 as usize).and_then(|n| n.as_ref()).map_or(0, |(n, ..)| slot_of_name[n.as_str()]);
        for (i, tc) in prog.classes.iter().enumerate().filter(|(_, tc)| self.is_written(syms, reach, tc.id)) {
            let home = match self.shared.groups.contains_key(&tc.id) {
                true => module_of(syms.class(tc.id).file),
                false => module_of(syms.class(tc.id).module_file()),
            };
            modules[home].classes.push(i);
        }
        for (i, &f) in prog.top_funs.iter().enumerate() {
            modules[module_of(syms.sym(prog.funs[f.idx()].sym).file)].funs.push(i);
        }
        for (i, (file, _)) in self.file_groups.iter().enumerate() {
            modules[module_of(*file)].groups.push(i);
        }
        for i in 0..syms.classes.len() {
            let c = ClassId(i as u32);
            if is_enum_constant(syms, reach, c) {
                modules[module_of(syms.class(c).file)].enum_values.push(c);
            }
        }
        for m in &mut modules {
            m.sort(prog, syms, interner);
        }
        let file_modules = (0..file_pkgs.len()).map(|f| module_of(FileId(f as u32))).collect();
        let callee_modules = callee_names.iter().map(|(callee, (n, ..))| (*callee, slot_of_name[n.as_str()])).collect();
        (modules, file_modules, callee_modules)
    }
}

/// Classes in their canonical order: the named ones by file and position, then the anonymous
/// classes the same way, then the tuple and function classes the typer makes on first use, two
/// at one position by kind and name. Two classes that share all of those share their emitted
/// name as well, which the output cannot hold (the JVM backend's "two classes are named"); every
/// caller sorts stably, so such a tie keeps the program's order, and the assertion-enabled builds
/// write it to the checks' log (`tie`).
pub fn compare_classes(syms: &Symbols, interner: &Interner, a: ClassId, b: ClassId) -> Ordering {
    let key = |c: ClassId| {
        let info = syms.class(c);
        let group = if is_arity_class(syms, interner, c) {
            2
        } else if info.kind == ClassKind::Anon {
            1
        } else {
            0
        };
        let (rank, start) = match syms.product_class_places.get(&c) {
            Some(&(source, offset)) => (syms.product_sources[source as usize].0, offset),
            None => syms.place(info.file, info.def, info.span.start),
        };
        (group, rank, start, info.kind as u8)
    };
    let order = key(a).cmp(&key(b)).then_with(|| interner.get(syms.class(a).name).cmp(interner.get(syms.class(b).name)));
    if order == Ordering::Equal && a != b {
        tie(|| format!("classes {} ({:?} and {:?})", interner.get(syms.class(a).name), a, b));
    }
    order
}

/// Two definitions the canonical order cannot tell apart, which no order may rest on the ids for
/// (the parallel typer's merge gives them).
fn tie(what: impl FnOnce() -> String) {
    crate::types::view::probe(|| format!("{} share their place in the output's order", what()));
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Created {
    Never,
    WithLocals,
    Otherwise,
}

/// Every creation of an anonymous class among the program's expressions, the dead ones of earlier
/// typings included, and whether its arguments are all locals or `this`. What `lower_closures`
/// makes of them does not depend on their order, so the expressions are read in slices on the
/// workers.
fn anon_creations(prog: &Program, syms: &Symbols) -> Vec<(ClassId, bool)> {
    const PER_WORKER: usize = 200_000;
    let exprs = prog.exprs.iter().as_slice();
    let scan = |slice: &[TExpr]| -> Vec<(ClassId, bool)> {
        let mut out = Vec::new();
        for e in slice {
            let TExpr::New(c, args) = *e else { continue };
            if syms.class(c).kind == ClassKind::Anon {
                out.push((c, prog.expr_list(args).iter().all(|&a| matches!(prog.expr(a), TExpr::Local(_) | TExpr::This))));
            }
        }
        out
    };
    let workers = (exprs.len() / PER_WORKER).clamp(1, crate::workers());
    if workers == 1 {
        return scan(exprs);
    }
    std::thread::scope(|scope| {
        let scan = &scan;
        let handles: Vec<_> = exprs.chunks(exprs.len().div_ceil(workers)).map(|slice| crate::alloc::spawn_in(scope, move || scan(slice))).collect();
        handles.into_iter().flat_map(|h| h.join().expect("closure scan thread panicked")).collect()
    })
}

/// Whether the anonymous class `tc` holds nothing a closure could not: see `lower_closures`.
/// A parent trait may redeclare the function's `apply` abstractly, as Magnolia's
/// `SerializableFunction0` does; it may declare nothing else.
fn is_closure_shaped(prog: &Program, syms: &Symbols, interner: &Interner, reach: &Reach, tc: &TClass, tested: &[bool]) -> bool {
    let info = syms.class(tc.id);
    if info.kind != ClassKind::Anon || info.js != JsKind::Scala || info.superclass.is_some() {
        return false;
    }
    let plain = tc.parent_args.is_none()
        && tc.init.is_empty()
        && tc.ctors.is_empty()
        && tc.forwarders.is_empty()
        && tc.bridges.is_empty()
        && tc.super_accessors.is_empty()
        && tc.methods.len() == 1;
    if !plain {
        return false;
    }
    let mut arity = None;
    for &(b, _) in info.base_types.iter().skip(1) {
        let base = syms.class(b);
        if is_function_class(syms, interner, b) {
            arity.get_or_insert(base.tparams.len() - 1);
            continue;
        }
        let redeclares_apply_only = base.member_order.iter().all(|&m| syms.sym(m).name == crate::names::APPLY && reach.is_abstract(syms, m));
        if base.kind != ClassKind::Trait || !redeclares_apply_only || tested[b.idx()] {
            return false;
        }
    }
    let Some(arity) = arity else { return false };
    let fun = &prog.funs[tc.methods[0].idx()];
    let Some(body) = fun.body else { return false };
    syms.sym(fun.sym).name == crate::names::APPLY
        && fun.params.len() == arity
        && fun.defaults.iter().all(|d| d.is_none())
        && !prog.descendants(body).any(|e| matches!(prog.expr(e), TExpr::This | TExpr::Super(_)))
}

/// Definitions in source order; a compiler-made one stands after the definition it shares its
/// position with, and among those the name and the kind tell them apart. A tie is two definitions
/// of one emitted name, kept in the program's order (`compare_classes`).
pub fn compare_syms(syms: &Symbols, interner: &Interner, a: SymId, b: SymId) -> Ordering {
    let key = |s: SymId| {
        let info = syms.sym(s);
        let (rank, start) = syms.place(info.file, info.def, info.span.start);
        (rank, start, info.def.map_or(u32::MAX, |d| d.0))
    };
    let order = key(a)
        .cmp(&key(b))
        .then_with(|| interner.get(syms.sym(a).name).cmp(interner.get(syms.sym(b).name)))
        .then_with(|| sym_kind_rank(syms.sym(a).kind).cmp(&sym_kind_rank(syms.sym(b).kind)));
    if order == Ordering::Equal && a != b {
        tie(|| format!("definitions {} ({:?} and {:?})", interner.get(syms.sym(a).name), a, b));
    }
    order
}

fn sym_kind_rank(k: SymKind) -> u8 {
    match k {
        SymKind::Val => 0,
        SymKind::Var => 1,
        SymKind::Def => 2,
        SymKind::Param => 3,
        SymKind::Object(_) => 4,
        SymKind::EnumValue(_) => 5,
        SymKind::Given => 6,
        SymKind::Overloaded(_) => 7,
    }
}

/// The place of a package-level extension method among those of its name in its package, in
/// source order: what tells them apart in the output, whatever was entered before them.
pub fn extension_rank(syms: &Symbols, interner: &Interner, s: SymId) -> usize {
    let info = syms.sym(s);
    let Owner::Package(p) = info.owner else { return 0 };
    let Some(entry) = syms.pkg(p).entries.get(&info.name) else { return 0 };
    // An inline extension is expanded where it is called and never written: it takes no place.
    let written = |e: SymId| syms.sym(e).mods & crate::ast::mods::INLINE == 0;
    entry.extensions.iter().filter(|&&e| written(e) && compare_syms(syms, interner, e, s) == Ordering::Less).count()
}

/// `scala.TupleN` or `scala.FunctionN`, made by the typer without a definition.
fn is_arity_class(syms: &Symbols, interner: &Interner, c: ClassId) -> bool {
    let info = syms.class(c);
    if info.def.is_some() {
        return false;
    }
    let Owner::Package(p) = info.owner else { return false };
    if interner.get(syms.pkg(p).name) != "scala" {
        return false;
    }
    let name = interner.get(info.name);
    let rest = name.strip_prefix("Tuple").or_else(|| name.strip_prefix("Function"));
    rest.map_or(false, |r| !r.is_empty() && r.bytes().all(|b| b.is_ascii_digit()))
}

/// `a.b.c` for the package, `_root_` for the empty one.
/// The package of the top-level definition a method is, or is a member of.
fn top_package(syms: &Symbols, m: SymId) -> Option<PkgId> {
    let mut owner = syms.sym(m).owner;
    for _ in 0..64 {
        match owner {
            Owner::Package(p) => return Some(p),
            Owner::Class(c) => owner = syms.class(c).owner,
            Owner::Local => return None,
        }
    }
    None
}

fn package_name(syms: &Symbols, interner: &Interner, p: PkgId) -> String {
    let mut segments = Vec::new();
    let mut at = Some(p);
    while let Some(p) = at {
        let info = syms.pkg(p);
        if info.parent.is_some() {
            segments.push(sanitize(interner.get(info.name)));
        }
        at = info.parent;
    }
    if segments.is_empty() {
        return "_root_".to_string();
    }
    segments.reverse();
    segments.join(".")
}

/// The module of a package: its name, with a `$` when a fixed module file of the output has it.
fn module_name(mut pkg: String) -> String {
    if RESERVED_MODULES.contains(&pkg.as_str()) {
        pkg.push('$');
    }
    pkg
}

/// Whether the package is one of the `--module-per-file` prefixes or lies below one.
fn is_module_per_file(pkg: &str, prefixes: &[String]) -> bool {
    prefixes.iter().any(|p| pkg.strip_prefix(p.as_str()).map_or(false, |rest| rest.is_empty() || rest.starts_with('.')))
}

fn file_stem(path: &str) -> &str {
    let name = path.rsplit(['/', '\\']).next().unwrap_or(path);
    name.strip_suffix(".scala").unwrap_or(name)
}

/// The value of a reached enum case without parameters is a constant of its module, unless the
/// enum is stateful, where the companion creates the values.
fn is_enum_constant(syms: &Symbols, reach: &Reach, c: ClassId) -> bool {
    let info = syms.class(c);
    info.singleton.is_some() && reach.classes[c.idx()] && !enum_of_case(syms, c).map_or(false, |e| e.stateful)
}

/// The enum that the case class `case` belongs to.
pub fn enum_of_case(syms: &Symbols, case: ClassId) -> Option<&ClassInfo> {
    let Owner::Class(companion) = syms.class(case).owner else { return None };
    Some(syms.class(syms.class(companion).companion?).info)
}

/// A top-level val that the initialiser of its file runs: lazy vals, givens, constants and the
/// vals the compiler makes (the mirrors of derivation) are read on their own.
pub fn is_eager_top_val(syms: &Symbols, sym: SymId, const_vals: &[bool]) -> bool {
    let info = syms.sym(sym);
    matches!(info.kind, SymKind::Val | SymKind::Var)
        && info.mods & mods::LAZY == 0
        && info.def.is_some()
        && !const_vals[sym.idx()]
}

/// Whether the construction of `tc` runs statements or initialisers of its body.
pub fn runs_body(prog: &Program, syms: &Symbols, tc: &TClass) -> bool {
    tc.init.iter().any(|i| match i {
        TInit::Field(s, _) => syms.sym(*s).mods & mods::LAZY == 0,
        TInit::Stmt(_) | TInit::Parent(..) => true,
    }) || tc.parent_via.map_or(false, |via| via_has_statements(prog, syms, via))
}

/// Whether an access of the top-level definition `s` initialises its file's eager vals before
/// anything else of it runs, its arguments or an assigned value included, as an access of
/// scalac's `<file>$package` object does: a def, a given that is no object
/// of its own, a val or var, lazy or not, but for a val that a use folds (`folded`: a `final
/// val` of a literal without a type). The definitions the compiler makes have no `<file>$package`
/// member.
pub fn initialises_file(syms: &Symbols, s: SymId, folded: bool) -> bool {
    let info = syms.sym(s);
    if !matches!(info.owner, Owner::Package(_)) || info.def.is_none() {
        return false;
    }
    match info.kind {
        SymKind::Def | SymKind::Var => true,
        SymKind::Given => {
            let parameterless = info.sig.as_ref().map_or(true, |sig| sig.tparams.is_empty() && sig.clauses.is_empty());
            !(parameterless && info.impl_class.is_some())
        }
        SymKind::Val => !folded,
        _ => false,
    }
}

/// Whether making an instance of `c` initialises its file's eager vals first: a given with
/// parameters and a body, which scalac makes through a def of the file's `<file>$package`, its
/// arguments evaluated after the file's initialiser.
pub fn given_class_initialises_file(syms: &Symbols, c: ClassId) -> bool {
    let info = syms.class(c);
    info.kind == ClassKind::GivenImpl
        && matches!(info.owner, Owner::Package(_))
        && info.def.is_some()
        && (!info.tparams.is_empty() || info.ctor.iter().any(|clause| !clause.params.is_empty()))
}

/// The secondary constructor `via` of the class it belongs to, with the ones its self call
/// chains to, in order: each with the body that holds its self call and the statements after.
pub fn ctor_chain(prog: &Program, syms: &Symbols, via: SymId) -> Vec<FunId> {
    let mut out = Vec::new();
    let mut cur = via;
    while out.len() <= syms.classes.len() {
        let Owner::Class(c) = syms.sym(cur).owner else { break };
        let Some(&f) = prog.classes.iter().find(|tc| tc.id == c).and_then(|tc| tc.ctors.iter().find(|&&f| prog.funs[f.idx()].sym == cur)) else { break };
        out.push(f);
        let Some(TExpr::Block(stmts, _)) = prog.funs[f.idx()].body.map(|b| prog.expr(b)) else { break };
        match stmts.len.checked_sub(1).map(|i| prog.stmts[stmts.start as usize + i as usize]) {
            Some(TStmt::Expr(call)) => match prog.expr(call) {
                TExpr::NewVia(next, _) => cur = next,
                _ => break,
            },
            _ => break,
        }
    }
    out
}

/// Whether a secondary constructor, or one its self call chains to, runs statements after the
/// call: what a subclass has to run once the superclass's body has.
pub fn via_has_statements(prog: &Program, syms: &Symbols, via: SymId) -> bool {
    ctor_chain(prog, syms, via).iter().any(|&f| {
        matches!(prog.funs[f.idx()].body.map(|b| prog.expr(b)), Some(TExpr::Block(_, after)) if !matches!(prog.expr(after), TExpr::Unit))
    })
}

/// The enum values stand in the order their classes were entered, which is their sources' for
/// the program's; a product's value cases, entered as the loader reaches their enums, are put in
/// their sources' order among the slots they take.
fn order_product_enum_values(syms: &Symbols, values: &mut [ClassId]) {
    if syms.product_class_places.is_empty() {
        return;
    }
    let slots: Vec<usize> = (0..values.len()).filter(|&i| syms.product_class_places.contains_key(&values[i])).collect();
    let mut placed: Vec<ClassId> = slots.iter().map(|&i| values[i]).collect();
    placed.sort_by_key(|c| {
        let (source, offset) = syms.product_class_places[c];
        (syms.product_sources[source as usize].0, offset)
    });
    for (slot, c) in slots.into_iter().zip(placed) {
        values[slot] = c;
    }
}
