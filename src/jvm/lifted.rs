//! The names of lifted methods as dotty's `LambdaLift` draws them (compiler/src/dotty/tools/dotc/transform/
//! LambdaLift.scala at 3.8.4): a closure is `$anonfun$N` where no method holds it (a val's right-hand side) and
//! `<owner>$$anonfun$N` where one does (`newName`, 66-71); a local def, a local lazy val's accessor and initialiser
//! and a local def's default getters are their names freshened, `g$N`. Each number comes from one counter per prefix
//! of the compilation unit (`FreshNameCreator.Default`, util/FreshNameCreator.scala 24-28), in the order
//! `Dependencies` meets the definitions (a walk that meets a definition before what it holds, Dependencies.scala
//! 276-283, whose order `liftLocals` names them in, LambdaLift.scala 97-128), over the tree the phases before it left:
//! a template's constructor holding its fields' initialisers and statements (`Constructors`), the complex arguments
//! of a superclass's call hoisted into methods (`HoistSuperArgs`), a value class's methods its companion's extension
//! methods (`ExtensionMethods`), a module class after its companion class (`FirstTransform.reorderAndComplete`).

use super::dependencies::{Callee, Deps, Kind};
use super::gen::*;
use super::lazy::{sanitize, UnitVisitor, UnitWalk};
use super::names::{encode, method_desc};
use crate::ast::{mods, ListRef};
use crate::intern::FxMap;
use crate::source::FileId;
use crate::symbols::*;
use crate::tir::*;
use crate::types::*;
use std::rc::Rc;

/// The names of a unit's lifted methods (`Gen::file_lifted_names`): each closure's by its body, each local def's and
/// each local lazy val's accessor's by its symbol, each local lazy val's initialiser's, each local def's default
/// getter's; and each one's place.
#[derive(Default)]
pub struct LiftedNames {
    /// Each lambda's name by its body and its place among the body's places in the typed tree (`LiftWalk::lambda`).
    lambdas: FxMap<(TExprId, u32), Box<str>>,
    defs: FxMap<SymId, Box<str>>,
    inits: FxMap<SymId, Box<str>>,
    defaults: FxMap<(SymId, usize), Box<str>>,
    places: FxMap<Placed, u32>,
    /// The lambdas and local defs (with their defaults' getters) inside a block dotty may replace by its constant
    /// (`LiftWalk::block`), with which it drops them.
    folded: FxMap<Folded, ()>,
}

/// What dotty may drop with a block it folds (`LiftedNames::folded`): a lambda by its body, a local def (and its
/// defaults' getters) by its symbol, a local or anonymous class (and what it holds) by its class.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum Folded {
    Lambda(TExprId),
    Def(SymId),
    Class(ClassId),
}

/// The lifted methods of the class being written (`Gen::lifted_entry`), by what each lifts: its name and its place by
/// its unit's walk (`LiftedNames`). Kept beside the class's plan, which holds such a method only once it is written.
/// An entry is made where the class first asks for it: the unit's names know no class (a class writes copies of local
/// defs another class's code declares, `Gen::copy_home`), and a class that lifts nothing walks no unit.
#[derive(Default)]
pub struct LiftedTable {
    entries: FxMap<Placed, (String, Option<u32>)>,
    /// What each method, by its name and its descriptor, lifts: how a call's `MethodRef` finds the body it names
    /// (`Gen::ask_lifted_call`). Two of one name and two descriptors are overloads, as dotty's counter numbers `+` and
    /// `$plus` alike; `None` for a name and descriptor two methods were given, refused where the second was named.
    named: FxMap<(String, String), Option<Placed>>,
}

impl LiftedTable {
    /// What the lifted method of the class named `name` with the descriptor `desc` lifts, `Some(None)` where two
    /// methods share both.
    pub(super) fn placed(&self, name: &str, desc: &str) -> Option<Option<Placed>> {
        self.named.get(&(name.to_string(), desc.to_string())).copied()
    }
}

/// Who makes an entry of the class's table: one of the four that make a lifted method where the class writes or holds
/// what asks for it, or the lowering of a call, which names the method it calls (`Gen::lifted_call_ref`).
#[derive(Clone, Copy)]
pub(super) enum Asker {
    LambdaMethod,
    LiftedFun,
    LazyMethod,
    LocalDefaultMethod,
    LiftedCallRef,
}

const ASKERS: [&str; 5] = ["lambda_method", "lifted_fun", "lazy_method", "local_default_method", "lifted_call_ref"];

/// The entries made, by asker, once their class's plan held a method (`TEQ_LIFTED_LATE`).
static LATE: [std::sync::atomic::AtomicU32; 5] = [const { std::sync::atomic::AtomicU32::new(0) }; 5];

/// Whether a build counts the entries of the classes' tables made once their class's plan holds a method, by asker,
/// and reports them as it ends (`TEQ_LIFTED_LATE` set; `report_late_entries`): what is left of a class's lifted methods
/// being in its plan before its code.
fn lifted_late() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("TEQ_LIFTED_LATE").is_some())
}

/// The build's line of the entries made late, `LIFTEDLATE<TAB>lambda_method N<TAB>...`, one count by asker, where
/// `TEQ_LIFTED_LATE` is set; the counts start again for the next build.
pub fn report_late_entries() {
    if !lifted_late() {
        return;
    }
    let counts: Vec<String> = ASKERS.iter().zip(&LATE).map(|(asker, n)| format!("{} {}", asker, n.swap(0, std::sync::atomic::Ordering::Relaxed))).collect();
    eprintln!("LIFTEDLATE\t{}", counts.join("\t"));
}

/// A lifted method of the unit by what it lifts, as `LiftedNames::places` keys its place: the order in which
/// `LambdaLift.transformDefDef` meets its definition (LambdaLift.scala 325-333), a definition after what it holds,
/// in which `Lifter.liftDef` (216-220) appends it to its class's lifted definitions, which `transformTemplate`
/// (342-347) puts after the template's body. Restricted to a class, the unit's order is the class's.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum Placed {
    /// A lambda, by its body and its place among the body's places in the typed tree (`LiftWalk::lambda`).
    Lambda(TExprId, u32),
    /// A local def, or a local lazy val's accessor.
    Def(SymId),
    LazyInit(SymId),
    Default(SymId, usize),
}

/// A definition whose code is walked, by its name as `newName` reads it (`raw`, the counter's key) and as the class
/// file spells it.
#[derive(Clone)]
struct OwnerName {
    raw: Rc<str>,
    spelled: Rc<str>,
}

impl OwnerName {
    fn new(raw: String, spelled: String) -> OwnerName {
        OwnerName { raw: Rc::from(raw), spelled: Rc::from(spelled) }
    }

    /// A method of a template, its source name its owner's.
    fn method(raw: &str) -> OwnerName {
        OwnerName::new(raw.to_string(), encode(raw))
    }

    /// A class's constructor, which holds its fields' initialisers and statements (`$init$$$anonfun$N`).
    fn constructor() -> OwnerName {
        OwnerName::new("<init>".to_string(), "$init$".to_string())
    }

    /// The initialiser of the lazy member `raw`, `x$lzyINITM` (`LazyVals.transformMemberDefThreadSafe`).
    fn lazy_initialiser(raw: &str, m: u32) -> OwnerName {
        OwnerName::new(format!("{}$lzyINIT{}", raw, m), format!("{}$lzyINIT{}", encode(raw), m))
    }

    /// The getter of the default `i` of a constructor (`DefaultGetterName` of `<init>`).
    fn constructor_default(i: usize) -> OwnerName {
        let name = format!("$lessinit$greater$default${}", i + 1);
        OwnerName::new(name.clone(), name)
    }
}

/// The walk of a unit (`Gen::walk_lifted_names`).
#[derive(Default)]
struct LiftWalk {
    unit: UnitWalk,
    counts: FxMap<String, u32>,
    /// `SuperArgName`'s counter of each class name (`HoistSuperArgs.newSuperArgMethod`).
    super_args: FxMap<String, u32>,
    out: LiftedNames,
    /// The definition holding the code being walked, none where a val does.
    owner: Option<OwnerName>,
    outer: Vec<Option<OwnerName>>,
    /// Set while a value class's methods are walked as its companion's extension methods.
    extension: bool,
    /// What the first walk records for the proxies of the unit's free variables (`dependencies.rs`).
    deps: Option<Box<Deps>>,
    /// The arguments teq's typer passes for what a callee captures (`LambdaLift.addFreeArgs`' work, which comes after
    /// `Dependencies`): a local trait's lifted member's or initialiser's proxies (`Worker::adapt_lifted_calls`,
    /// `add_trait_forwarders`), a local class's captures at its creation.
    proxy_args: FxMap<TExprId, ()>,
    /// The definitions the walk is inside of, each a lifted method's (`Placed`) or not, placed at its end.
    placing: Vec<Option<Placed>>,
    /// How many places of the typed tree each lambda's body has been met at.
    lambda_places: FxMap<TExprId, u32>,
    /// For each block the walk is inside of, whether dotty may replace it by its constant (`LiftWalk::block`).
    blocks: Vec<bool>,
}

impl LiftWalk {
    /// `FreshNameCreator.newName`: the counter of `sanitize(prefix) + "$"` (`UniqueName`).
    fn fresh(&mut self, raw: &str) -> u32 {
        let n = self.counts.entry(format!("{}$", sanitize(raw))).or_insert(0);
        *n += 1;
        *n
    }

    /// `raw` freshened (`NameOps.freshened`), `raw$N`, spelled `spelled$N`.
    fn freshened(&mut self, raw: &str, spelled: &str) -> OwnerName {
        let n = self.fresh(raw);
        OwnerName::new(format!("{}${}", raw, n), format!("{}${}", spelled, n))
    }

    fn closure(&mut self) -> OwnerName {
        match self.owner.clone() {
            Some(o) => self.freshened(&format!("{}$$anonfun", o.raw), &format!("{}$$anonfun", o.spelled)),
            None => self.freshened("$anonfun", "$anonfun"),
        }
    }

    fn enter(&mut self, owner: Option<OwnerName>) {
        let outer = std::mem::replace(&mut self.owner, owner);
        self.outer.push(outer);
    }

    fn leave(&mut self) {
        self.owner = self.outer.pop().flatten();
    }

    /// The definition ended takes the next place.
    fn place(&mut self, p: Placed) {
        let next = self.out.places.len() as u32;
        self.out.places.entry(p).or_insert(next);
    }

    fn end_placing(&mut self) {
        if let Some(p) = self.placing.pop().flatten() {
            self.place(p);
        }
    }

    /// Enters a method or a class for the dependencies (`Deps::push`), where the walk records them.
    fn push(&mut self, kind: Kind, callee: Option<Callee>, declares: &[SymId]) -> u32 {
        self.deps.as_deref_mut().map_or(0, |d| d.push(kind, callee, declares))
    }

    fn pop(&mut self) {
        if let Some(d) = self.deps.as_deref_mut() {
            d.pop();
        }
    }

    fn call(&mut self, callee: Callee) {
        if let Some(d) = self.deps.as_deref_mut() {
            d.call(callee);
        }
    }

    /// Enters the getter of the local trait's lazy val `s`, which holds its right-hand side and the initialiser teq's
    /// typer made of it: one definition, entered at the first of the two.
    fn trait_lazy(&mut self, s: SymId) {
        let Some(d) = self.deps.as_deref_mut() else { return };
        match d.callee(Callee::TraitMember(s)) {
            Some(e) => d.reenter(e),
            None => {
                d.push(Kind::Method, Some(Callee::TraitMember(s)), &[]);
            }
        }
    }
}

impl<'a> UnitVisitor<'a> for LiftWalk {
    const ANON_FIRST: bool = true;

    fn unit(&mut self) -> &mut UnitWalk {
        &mut self.unit
    }

    fn class(&mut self, g: &mut Gen<'a>, c: ClassId) {
        if self.blocks.contains(&true) {
            self.out.folded.insert(Folded::Class(c), ());
        }
        g.walk_class_lifted(self, c);
    }

    fn anonymous(&mut self, g: &mut Gen<'a>, c: ClassId, args: ListRef) -> bool {
        if self.blocks.contains(&true) {
            self.out.folded.insert(Folded::Class(c), ());
        }
        g.walk_anonymous_lifted(self, c, args);
        true
    }

    /// A lambda at one of its body's places in the typed tree. A tree the typer shares between two places (an
    /// op-assignment's left side, `b(i) += 2` as `b.update(i, b.apply(i) + 2)` with `i` by-name, its thunk in both)
    /// is met at each and lifted at each, as scalac's tree holds it twice (`Applications.typedOpAssign` writes the
    /// lifted left side twice, `e' = e' + args`, and `ElimByName` makes a closure at each): two closures, two names.
    /// A body the back end copies after the walk (a `finally`'s, which the writer writes on each exit) has one place,
    /// its lambda one method (`Gen::lambda_method`).
    /// A block dotty may replace by its constant, and with it what it defines: `FirstTransform.transformBlock`'s
    /// `constToLiteral` (FirstTransform.scala 202-203; TreeInfo.scala 753-780) of a pure block whose type is a
    /// constant, a definition a pure statement (`statPurity`). Read from the result's form alone
    /// (`Gen::result_may_be_constant`), without its type or the statements' purity: a block named here may be one
    /// dotty keeps, so that what is under it is checked as teq's and may pass where dotty refuses it, never the
    /// reverse.
    fn block(&mut self, g: &mut Gen<'a>, e: TExprId) {
        let TExpr::Block(_, result) = g.cx.input.prog.expr(e) else { return self.blocks.push(false) };
        self.blocks.push(g.result_may_be_constant(result));
    }

    fn block_end(&mut self) {
        self.blocks.pop();
    }

    fn lambda(&mut self, g: &mut Gen<'a>, params: ListRef, body: TExprId) -> bool {
        if self.blocks.contains(&true) {
            self.out.folded.insert(Folded::Lambda(body), ());
        }
        let place = *self.lambda_places.entry(body).and_modify(|n| *n += 1).or_insert(0);
        let name = self.closure();
        self.out.lambdas.insert((body, place), Box::from(&*name.spelled));
        self.enter(Some(name));
        self.push(Kind::Method, None, g.cx.input.prog.sym_list(params));
        self.placing.push(Some(Placed::Lambda(body, place)));
        true
    }

    fn lambda_end(&mut self) {
        self.pop();
        self.leave();
        self.end_placing();
    }

    fn local_fun(&mut self, g: &mut Gen<'a>, f: FunId) -> bool {
        let s = g.cx.input.prog.funs[f.idx()].sym;
        if self.out.defs.contains_key(&s) {
            return false;
        }
        if self.blocks.contains(&true) {
            self.out.folded.insert(Folded::Def(s), ());
        }
        let raw = g.cx.input.interner.get(g.cx.input.syms.sym(s).name);
        let name = self.freshened(raw, &encode(raw));
        self.out.defs.insert(s, Box::from(&*name.spelled));
        self.enter(Some(name));
        self.push(Kind::Method, Some(Callee::Fun(s)), &g.declared_params(f));
        self.placing.push(Some(Placed::Def(s)));
        true
    }

    fn local_fun_end(&mut self) {
        self.pop();
        self.leave();
        self.end_placing();
    }

    fn default_getter(&mut self, g: &mut Gen<'a>, f: FunId, i: usize, local: bool) {
        let s = g.cx.input.prog.funs[f.idx()].sym;
        let raw = g.cx.input.interner.get(g.cx.input.syms.sym(s).name);
        // Numbered as the getter is (`Gen::default_getter_shape`: a right-associative extension's in its compiled order).
        let index = g.default_getter_shape(s, i).index;
        let (getter, spelled) = (format!("{}$default${}", raw, index), format!("{}$default${}", encode(raw), index));
        self.placing.push(local.then_some(Placed::Default(s, i)));
        let owner = if local {
            let name = self.freshened(&getter, &spelled);
            self.out.defaults.insert((s, i), Box::from(&*name.spelled));
            name
        } else if self.extension {
            OwnerName::new(format!("{}$extension", getter), format!("{}$extension", spelled))
        } else {
            OwnerName::new(getter, spelled)
        };
        self.enter(Some(owner));
        match self.deps.as_deref_mut() {
            Some(d) if local => {
                d.push(Kind::Method, Some(Callee::Default(s, i)), &g.declared_params(f));
            }
            Some(d) => {
                d.push_beside(Kind::Method, g.local_trait_member(s).then_some(Callee::Default(s, i)), &g.declared_params(f));
            }
            None => {}
        }
    }

    fn default_getter_end(&mut self) {
        self.pop();
        self.leave();
        self.end_placing();
    }

    /// A local lazy val's initialiser `x$lzyINITM` is its right-hand side's owner, named before its accessor `x`,
    /// which follows it (`LazyVals.transformLocalDef`); another val's right-hand side has no method for an owner.
    /// The dependencies take the initialiser and the accessor as methods, the accessor calling the initialiser and a
    /// read of the val the accessor; they leave out its holder (`x$lzyN`), whose proxy draws from no counter of a
    /// lifted method.
    fn val(&mut self, g: &mut Gen<'a>, sym: Option<SymId>) -> bool {
        if let (Some(d), Some(s)) = (self.deps.as_deref_mut(), sym) {
            d.declare(s);
        }
        let Some(s) = sym.filter(|&s| g.is_lazy_local(s)) else {
            self.enter(None);
            self.placing.push(None);
            return true;
        };
        if self.out.inits.contains_key(&s) {
            return false;
        }
        self.placing.push(Some(Placed::LazyInit(s)));
        let raw = g.cx.input.interner.get(g.cx.input.syms.sym(s).name);
        let m = g.lazy_local_numbers(s).map_or(0, |(_, m)| m);
        let name = self.freshened(&format!("{}$lzyINIT{}", raw, m), &format!("{}$lzyINIT{}", encode(raw), m));
        self.out.inits.insert(s, Box::from(&*name.spelled));
        self.enter(Some(name));
        self.push(Kind::Method, Some(Callee::LazyInit(s)), &[]);
        // A local object's val, and a companion module's the namer or teq's typer makes: `new C$` its right-hand side.
        if let (Some(d), Some(TExpr::New(k, _))) = (self.deps.as_deref_mut(), g.cx.lazy_local_init(s).map(|e| g.cx.input.prog.expr(e))) {
            d.module_vals.insert(k, s);
        }
        true
    }

    /// A local lazy val's initialiser is placed at its end, its accessor after it (`LazyVals.transformLocalDef`'s
    /// thicket: the holder, the initialiser, the accessor).
    fn val_end(&mut self, g: &mut Gen<'a>, sym: Option<SymId>) {
        self.leave();
        self.end_placing();
        if let Some(s) = sym.filter(|&s| g.is_lazy_local(s)) {
            self.pop();
            self.push(Kind::Method, Some(Callee::Fun(s)), &[]);
            self.call(Callee::LazyInit(s));
            self.pop();
            let raw = g.cx.input.interner.get(g.cx.input.syms.sym(s).name);
            let name = self.freshened(raw, &encode(raw));
            self.out.defs.insert(s, Box::from(&*name.spelled));
            self.place(Placed::Def(s));
        }
    }

    fn expr(&mut self, g: &mut Gen<'a>, e: TExprId) {
        let Some(d) = self.deps.as_deref_mut() else { return };
        let cx = g.cx;
        let syms = cx.input.syms;
        let local_def = |s: SymId| syms.sym(s).owner == Owner::Local && syms.sym(s).kind == SymKind::Def;
        match cx.input.prog.expr(e) {
            TExpr::Local(_) | TExpr::CallStatic(..) if self.proxy_args.contains_key(&e) => {}
            TExpr::Local(s) | TExpr::CallStatic(s, _) => g.note_local(d, s),
            // What a local class captures its creation passes first (`Typer`'s captures, `LambdaLift`'s work after
            // `Dependencies`): the creation calls the constructor, whose free variables it takes.
            TExpr::New(c, args) => {
                d.call(Callee::Ctor(c));
                let list = cx.input.prog.expr_list(args);
                for &a in &list[..g.capture_count(c).min(list.len())] {
                    self.proxy_args.insert(a, ());
                }
            }
            TExpr::NewVia(s, args) => {
                if let Owner::Class(c) = syms.sym(s).owner {
                    d.call(Callee::Ctor(c));
                }
                d.call(Callee::Fun(s));
                let captured = g.captured_params(s);
                for (&a, _) in cx.input.prog.expr_list(args).iter().zip(&captured).filter(|&(_, &c)| c) {
                    self.proxy_args.insert(a, ());
                }
            }
            TExpr::Default(DefaultOf::Method(s), at, _) if local_def(s) || g.local_trait_member(s) => d.call(Callee::Default(s, (at & !DEFAULT_RECEIVER) as usize)),
            TExpr::Default(DefaultOf::Method(s), _, _) if g.local_class_constructor(s).is_some() => {
                d.call(Callee::Companion(g.local_class_constructor(s).expect("a local class's constructor")))
            }
            // A local class's getters are its companion module's: a call reads the module's lazy val.
            TExpr::Default(DefaultOf::Ctor(c), _, _) if syms.class(c).owner == Owner::Local => d.call(Callee::Companion(c)),
            TExpr::CallMethod(_, m, args) if g.local_trait_member(m) => {
                d.call(Callee::TraitMember(m));
                let n = g.proxy_count(m);
                for &a in &cx.input.prog.expr_list(args)[..n.min(args.len as usize)] {
                    self.proxy_args.insert(a, ());
                }
            }
            _ => {}
        }
    }

    /// A pattern's binders, declared where the pattern stands.
    fn pattern(&mut self, g: &mut Gen<'a>, p: TPatId) {
        let Some(d) = self.deps.as_deref_mut() else { return };
        let prog = g.cx.input.prog;
        let mut todo = vec![p];
        while let Some(p) = todo.pop() {
            match prog.pats[p.idx()] {
                TPat::Bind(s, inner) => {
                    d.declare(s);
                    todo.extend(inner);
                }
                TPat::Unapply(s, _, inner) => {
                    d.declare(s);
                    todo.push(inner);
                }
                TPat::Test(_, _, inner) => todo.push(inner),
                TPat::Class(_, _, _, subs) | TPat::Alt(subs) => todo.extend_from_slice(&prog.pat_lists[subs.range()]),
                TPat::Seq(items, rest) => {
                    todo.extend_from_slice(&prog.pat_lists[items.range()]);
                    todo.extend(rest);
                }
                TPat::Wildcard | TPat::Equals(..) => {}
            }
        }
    }
}

impl<'a> Gen<'a> {
    /// Whether the class `k` is the module class of an object: one the source writes, a companion the typer or the
    /// namer makes, an object nested in a class (a class of the program, named as its companion class and a `$`).
    fn is_object(&self, k: ClassId) -> bool {
        let cx = self.cx;
        let info = cx.input.syms.class(k);
        info.kind == ClassKind::Object
            || info.synthetic_companion
            || info.made_companion
            || info.companion.is_some_and(|c| cx.class_names[k.idx()].strip_suffix('$') == Some(cx.class_names[c.idx()].as_str()))
    }

    /// The companion object whose walk holds the class `c`'s constructor getters and extension methods: one with a
    /// template of its own. The one test both places ask, so that a getter is walked once whatever it answers.
    fn companion_holding(&self, c: ClassId) -> Option<ClassId> {
        let k = self.cx.input.syms.class(c).companion?;
        (!self.is_object(c) && self.is_object(k) && self.cx.tclass_of[k.idx()] != u32::MAX).then_some(k)
    }

    /// `FirstTransform.reorderAndComplete`: an object's module class stands after its companion class where that
    /// comes later. The places of `classes`, each with the place it is walked at and whether after the class there.
    fn companions_after(&self, classes: &[(u32, ClassId)]) -> Vec<(u32, bool, ClassId)> {
        let syms = self.cx.input.syms;
        classes
            .iter()
            .map(|&(at, c)| {
                let companion = syms.class(c).companion.filter(|&k| self.is_object(c) && !self.is_object(k));
                match companion.and_then(|k| classes.iter().find(|&&(_, x)| x == k)).filter(|&&(kat, _)| kat > at) {
                    Some(&(kat, _)) => (kat, true, c),
                    None => (at, false, c),
                }
            })
            .collect()
    }

    /// The lifted methods' names of the file `file`, walked once whichever class asks first (as its lazy names are,
    /// `file_lazy_names`): every worker draws the same. What the walk reads of other classes is no function
    /// arity of the class being written.
    fn file_lifted_names(&mut self, file: FileId) -> &'a LiftedNames {
        let cx = self.cx;
        match cx.lifted_names.get(file.0 as usize) {
            Some(cell) => cell.get_or_init(|| {
                let arities = self.function_arities;
                let names = self.walk_lifted_names(file);
                self.function_arities = arities;
                names
            }),
            None => &EMPTY,
        }
    }

    /// The unit whose code the class being written holds (`ClassInfo::module_file`, a file's module's own).
    fn writing_unit(&self) -> FileId {
        match self.this_class {
            Some(c) => self.cx.input.syms.class(c).module_file(),
            None => self.line_file,
        }
    }

    /// The unit whose code declares the local def or lazy val `s` (`Cx::local_file`), else the writing one's.
    fn declaring_unit(&self, s: SymId) -> FileId {
        self.cx.local_file(s).unwrap_or_else(|| self.writing_unit())
    }

    /// The name and the place of the lifted method of `p`, of the shape `shape`, in the class being written, by the
    /// class's table: a lookup of the unit's names the first time the class asks for it, which queues nothing. Two
    /// methods of one name and one descriptor in the class are an internal error, where the second is named.
    pub(super) fn lifted_entry(&mut self, p: Placed, asker: Asker, shape: &crate::lir::LiftedShape) -> (String, Option<u32>) {
        if let Some(entry) = self.lifted_table.entries.get(&p) {
            return entry.clone();
        }
        // A write finds the entries the class's holds made (the table checked by its key, not as a field).
        if super::gen::writing() {
            panic!("a write of its class makes a lifted method's entry in the class's table, asked by {}", ASKERS[asker as usize]);
        }
        if lifted_late() && !self.plan.methods.is_empty() {
            LATE[asker as usize].fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        let name = match p {
            Placed::Lambda(body, place) => self.lambda_name(body, place),
            Placed::Def(s) => self.def_name(s),
            Placed::LazyInit(s) => self.lazy_init_name(s),
            Placed::Default(s, i) => self.local_default_name(s, i),
        };
        let key = (name.clone(), method_desc(&shape.takes(), &shape.ret));
        if self.lifted_table.named.insert(key.clone(), Some(p)).is_some() {
            self.unsupported(&format!("internal error: two lifted methods named {}{} in the class", key.0, key.1));
            self.lifted_table.named.insert(key, None);
        }
        let entry = (name, self.lifted_place(p));
        self.lifted_table.entries.insert(p, entry.clone());
        entry
    }

    /// The lifted method of the lambda whose body is `body`, by its unit's walk.
    fn lambda_name(&mut self, body: TExprId, place: u32) -> String {
        let file = self.writing_unit();
        if let Some(name) = self.file_lifted_names(file).lambdas.get(&(body, place)) {
            return name.to_string();
        }
        self.unsupported(&format!("internal error: the lambda of body {} has no name in its unit at its place {}", body.0, place));
        format!("$anonfun$unnamed${}", body.0)
    }

    /// The lifted method of the local def `s`, or of the local lazy val `s`'s accessor.
    fn def_name(&mut self, s: SymId) -> String {
        let file = self.declaring_unit(s);
        if let Some(name) = self.file_lifted_names(file).defs.get(&s) {
            return name.to_string();
        }
        let name = self.cx.input.interner.get(self.cx.input.syms.sym(s).name);
        self.unsupported(&format!("internal error: the local definition {} has no name in its unit", name));
        format!("{}$unnamed${}", encode(name), s.0)
    }

    /// The lifted initialiser of the local lazy val `s`, `x$lzyINITM$N`.
    fn lazy_init_name(&mut self, s: SymId) -> String {
        let file = self.declaring_unit(s);
        let found = self.lazy_local_numbers(s).and_then(|_| self.file_lifted_names(file).inits.get(&s));
        if let Some(name) = found {
            return name.to_string();
        }
        let name = self.cx.input.interner.get(self.cx.input.syms.sym(s).name);
        self.unsupported(&format!("internal error: the local lazy val {} has no number in its unit", name));
        format!("{}$lzyINIT$unnamed${}", encode(name), s.0)
    }

    /// Whether dotty may drop the lifted method `f` with a block it folds (`LiftWalk::block`), by its unit's walk.
    pub(super) fn under_folded_block(&mut self, f: Folded) -> bool {
        let file = match f {
            Folded::Lambda(_) => self.writing_unit(),
            Folded::Def(s) => self.declaring_unit(s),
            Folded::Class(c) => self.cx.input.syms.class(c).module_file(),
        };
        self.file_lifted_names(file).folded.contains_key(&f)
    }

    /// Whether the block `e`'s result could have a constant type to dotty, by its form alone (dotty folds a pure tree
    /// whose type, its term references widened, is a `ConstantType`): a literal other than `()` and `null` (`UnitType`
    /// and `NullType` are none), a read of a value, a parameter or a value's getter, none lazy (a `final val`, an
    /// `inline val` or a value of a singleton type is one), an operator, a concatenation or a conversion over such, an
    /// `if` or a `match` with such a branch, a nested block's result, an ascription or a cast of such. A call of a method
    /// is no pure tree to dotty (TreeInfo's `exprPurity` of an application). A block whose result is a class literal
    /// dotty leaves alone (TreeInfo.scala 757-760). Wider than dotty's fold by form, never narrower.
    pub(super) fn result_may_be_constant(&self, e: TExprId) -> bool {
        let prog = self.cx.input.prog;
        let list = |l: ListRef| prog.expr_list(l).to_vec();
        match prog.expr(e) {
            TExpr::Unit | TExpr::Null | TExpr::ClassOf(..) => false,
            TExpr::Int(_) | TExpr::Long(_) | TExpr::Double(_) | TExpr::Bool(_) | TExpr::Char(_) | TExpr::Str(_) => true,
            TExpr::Local(v) | TExpr::Static(v) | TExpr::Field(_, v) => self.cx.input.syms.sym(v).mods & crate::ast::mods::LAZY == 0,
            // A read of a value's getter (`K.k` of an object's `final val`); a call of a method is no pure tree to dotty
            // (`exprPurity` of an application), nor a lazy val's read.
            TExpr::CallStatic(m, args) | TExpr::CallMethod(_, m, args) => {
                let info = self.cx.input.syms.sym(m);
                args.is_empty() && info.kind != SymKind::Def && info.mods & crate::ast::mods::LAZY == 0
            }
            TExpr::Prim(_, a, b) => self.result_may_be_constant(a) && self.result_may_be_constant(b),
            TExpr::Unary(_, a) | TExpr::ToStr(a, _) | TExpr::Cast(a, ..) | TExpr::Block(_, a) => self.result_may_be_constant(a),
            TExpr::StrConcat(parts) => list(parts).into_iter().all(|x| self.result_may_be_constant(x)),
            TExpr::If(_, t, els) => self.result_may_be_constant(t) || els.is_some_and(|x| self.result_may_be_constant(x)),
            TExpr::Match(..) => true,
            _ => false,
        }
    }

    /// The place of the lifted method of `p` among its class's lifted methods (`Placed`): a lambda's in the unit of
    /// the class being written, a local definition's in the unit of the code declaring it, as their names are.
    fn lifted_place(&mut self, p: Placed) -> Option<u32> {
        let file = match p {
            Placed::Lambda(..) => self.writing_unit(),
            Placed::Def(s) | Placed::LazyInit(s) | Placed::Default(s, _) => self.declaring_unit(s),
        };
        self.file_lifted_names(file).places.get(&p).copied()
    }

    /// The lifted getter of the default `i` of the local def `s`, `g$default$I$N`.
    fn local_default_name(&mut self, s: SymId, i: usize) -> String {
        let file = self.declaring_unit(s);
        if let Some(name) = self.file_lifted_names(file).defaults.get(&(s, i)) {
            return name.to_string();
        }
        let name = self.cx.input.interner.get(self.cx.input.syms.sym(s).name);
        self.unsupported(&format!("internal error: the default {} of the local def {} has no name in its unit", i + 1, name));
        format!("{}$default${}$unnamed${}", encode(name), i + 1, s.0)
    }

    /// The names of the unit's lifted methods. `LambdaLift` names its free variables' proxies first, from the same
    /// counters: where a proxy's name meets the counter of a lifted method's, the names are drawn again after the
    /// proxies' (`Deps::proxies`); the first walk's stand otherwise.
    fn walk_lifted_names(&mut self, file: FileId) -> LiftedNames {
        let mut first = LiftWalk { deps: Some(Box::new(Deps::new())), ..LiftWalk::default() };
        self.walk_unit_lifted(file, &mut first);
        let (syms, interner) = (self.cx.input.syms, self.cx.input.interner);
        let mut deps = first.deps.take().expect("the first walk records the unit's dependencies");
        let proxies = deps.proxies(|s| interner.get(syms.sym(s).name).to_string());
        if !proxies.keys().any(|k| first.counts.contains_key(k)) {
            return first.out;
        }
        let mut second = LiftWalk { counts: proxies, ..LiftWalk::default() };
        self.walk_unit_lifted(file, &mut second);
        second.out
    }

    /// A read of the local `s`: a local lazy val's calls its accessor and a local def's (as a value) the def, another
    /// local's or a constructor's parameter's (in its constructor's code) refers to it.
    fn note_local(&self, d: &mut Deps, s: SymId) {
        let syms = self.cx.input.syms;
        let info = syms.sym(s);
        match info.owner {
            Owner::Local if info.kind == SymKind::Def || self.is_lazy_local(s) => d.call(Callee::Fun(s)),
            Owner::Local => d.refer(s),
            Owner::Class(c) if syms.class(c).ctor_syms.iter().any(|clause| clause.contains(&s)) => d.refer(s),
            _ => {}
        }
    }

    /// The parameters the def `f` declares: neither a local trait's member's proxies (`TFun::captures`) nor a secondary
    /// constructor's parameters for its class's captures, which are the block's own locals.
    fn declared_params(&self, f: FunId) -> Vec<SymId> {
        let cx = self.cx;
        let fun = &cx.input.prog.funs[f.idx()];
        let own = &fun.params[(fun.captures as usize).min(fun.params.len())..];
        let captured: &[SymId] = match cx.input.syms.sym(fun.sym).owner {
            Owner::Class(c) => match cx.tclass_of.get(c.idx()) {
                Some(&i) if i != u32::MAX => {
                    let tc = &cx.input.prog.classes[i as usize];
                    &tc.ctor_params[..tc.captures.min(tc.ctor_params.len())]
                }
                _ => &[],
            },
            _ => &[],
        };
        own.iter().copied().filter(|s| !captured.contains(s)).collect()
    }

    /// For each parameter of the secondary constructor `s`, whether it is one of its class's captures.
    fn captured_params(&self, s: SymId) -> Vec<bool> {
        let cx = self.cx;
        let Owner::Class(c) = cx.input.syms.sym(s).owner else { return Vec::new() };
        let (Some(&f), Some(&i)) = (cx.fun_of_sym.get(s.idx()), cx.tclass_of.get(c.idx())) else { return Vec::new() };
        if f == u32::MAX || i == u32::MAX {
            return Vec::new();
        }
        let tc = &cx.input.prog.classes[i as usize];
        let captures = &tc.ctor_params[..tc.captures.min(tc.ctor_params.len())];
        cx.input.prog.funs[f as usize].params.iter().map(|p| captures.contains(p)).collect()
    }

    /// The local class whose secondary constructor `s` is.
    fn local_class_constructor(&self, s: SymId) -> Option<ClassId> {
        let cx = self.cx;
        let Owner::Class(c) = cx.input.syms.sym(s).owner else { return None };
        let f = *cx.fun_of_sym.get(s.idx())?;
        let i = *cx.tclass_of.get(c.idx())?;
        (cx.input.syms.class(c).owner == Owner::Local && f != u32::MAX && i != u32::MAX && cx.input.prog.classes[i as usize].ctors.contains(&FunId(f))).then_some(c)
    }

    /// How many of the def `m`'s parameters are a local trait's member's proxies (`TFun::captures`).
    fn proxy_count(&self, m: SymId) -> usize {
        match self.cx.fun_of_sym.get(m.idx()) {
            Some(&f) if f != u32::MAX => self.cx.input.prog.funs[f as usize].captures as usize,
            _ => 0,
        }
    }

    /// The lazy val of the trait `tc` whose initialiser teq's typer made the member `m` (`Worker::lazy_initialiser`:
    /// its proxies taken, the val's right-hand side a call of it): in dotty one definition with the val's getter.
    fn trait_lazy_of_initialiser(&self, tc: &TClass, m: SymId) -> Option<SymId> {
        let prog = self.cx.input.prog;
        tc.init.iter().find_map(|i| match *i {
            TInit::Field(s, e) if self.cx.is_lazy_member(s) && matches!(prog.expr(e), TExpr::CallMethod(_, x, _) if x == m) => Some(s),
            _ => None,
        })
    }

    /// Whether `m` is a member of a local trait: a local method, whose callers take its free variables.
    fn local_trait_member(&self, m: SymId) -> bool {
        let syms = self.cx.input.syms;
        matches!(syms.sym(m).owner, Owner::Class(t) if syms.class(t).kind == ClassKind::Trait && syms.class(t).owner == Owner::Local)
    }

    /// The local traits the class `c` mixes in that its superclass does not: those whose initialisers its
    /// constructor calls and whose members it forwards to (`Mixin`).
    fn mixed_local_traits(&self, c: ClassId) -> Vec<ClassId> {
        let syms = self.cx.input.syms;
        let info = syms.class(c);
        let inherited: Vec<ClassId> = info.superclass.map(|k| syms.class(k).base_types.iter().map(|&(b, _)| b).collect()).unwrap_or_default();
        info.base_types
            .iter()
            .skip(1)
            .map(|&(b, _)| b)
            .filter(|&b| syms.class(b).kind == ClassKind::Trait && syms.class(b).owner == Owner::Local && !inherited.contains(&b))
            .collect()
    }

    /// The members of local traits the class `c` gets a mixin forwarder to (`Mixin.mixinForwarders`, every member
    /// with a body under `-Xmixin-force-forwarders`, the default; `needsMixinForwarder`): each method and lazy val with
    /// a body of a trait of `mixed_local_traits`, not private, that no definition before it in `c`'s linearisation
    /// of that name and arity overrides.
    fn forwarded_members(&self, c: ClassId) -> Vec<SymId> {
        let cx = self.cx;
        let (syms, prog) = (cx.input.syms, cx.input.prog);
        let bodies = |k: ClassId| -> Vec<(SymId, usize)> {
            let Some(&idx) = cx.tclass_of.get(k.idx()).filter(|&&i| i != u32::MAX) else { return Vec::new() };
            let tc = &prog.classes[idx as usize];
            let mut out: Vec<(SymId, usize)> =
                tc.methods.iter().map(|&f| &prog.funs[f.idx()]).filter(|f| f.body.is_some()).map(|f| (f.sym, f.params.len() - f.captures as usize)).collect();
            out.extend(tc.init.iter().filter_map(|i| if let TInit::Field(s, _) = *i { Some(s) } else { None }).filter(|&s| cx.is_lazy_member(s)).map(|s| (s, 0)));
            out
        };
        let traits = self.mixed_local_traits(c);
        let order: Vec<ClassId> = syms.class(c).base_types.iter().map(|&(b, _)| b).collect();
        let mut out = Vec::new();
        for &t in &traits {
            for (m, arity) in bodies(t) {
                if syms.sym(m).mods & mods::PRIVATE != 0 {
                    continue;
                }
                let name = syms.sym(m).name;
                let first = order.iter().find(|&&k| bodies(k).iter().any(|&(o, a)| syms.sym(o).name == name && a == arity));
                if first == Some(&t) {
                    out.push(m);
                }
            }
        }
        out
    }

    /// The walk: per package clause the classes, traits and objects that stand at the top level in their order, a
    /// module class after its companion class, each class followed by its made companion; then the file's package
    /// object (`Desugar.packageDef`); then the classes it did not meet, by their binary names.
    fn walk_unit_lifted(&mut self, file: FileId, w: &mut LiftWalk) {
        let cx = self.cx;
        let syms = cx.input.syms;
        let (tops, opaque) = self.unit_classes(file, &mut w.unit);
        let mut order = self.companions_after(&tops);
        order.sort();
        for (_, _, c) in order {
            self.walk_class_lifted(w, c);
        }
        // The package object: its constructor's code (its vals' initialisers in their order), then its members by
        // place (its defs, its lazy vals' initialisers, its opaque types' companions).
        let (items, vals) = self.unit_package_items(file, &opaque);
        let lazy: FxMap<SymId, OwnerName> = items
            .iter()
            .filter(|&&(_, kind, _)| kind == 0)
            .filter_map(|&(_, _, id)| {
                let s = SymId(id);
                let name = self.field_name(s);
                let raw = cx.input.interner.get(syms.sym(s).name);
                self.lazy_member_numbers_in(file, None, &name).map(|(_, m)| (s, self.lazy_member_owner(s, raw, m)))
            })
            .collect();
        w.enter(Some(OwnerName::constructor()));
        w.push(Kind::Method, None, &[]);
        for &(_, kind, id) in &items {
            if let (0, Some(&e)) = (kind, vals.get(&SymId(id))) {
                if !lazy.contains_key(&SymId(id)) {
                    w.unit.scope = syms.sym(SymId(id)).span.start;
                    self.walk_expr(w, e);
                }
            }
        }
        w.pop();
        w.leave();
        for (_, kind, id) in items {
            match kind {
                0 => {
                    let s = SymId(id);
                    if let (Some(owner), Some(&e)) = (lazy.get(&s), vals.get(&s)) {
                        w.unit.scope = syms.sym(s).span.start;
                        w.enter(Some(owner.clone()));
                        w.push(Kind::Method, None, &[]);
                        self.walk_expr(w, e);
                        w.pop();
                        w.leave();
                    }
                }
                1 => {
                    let f = FunId(id);
                    w.enter(Some(OwnerName::method(cx.input.interner.get(syms.sym(cx.input.prog.funs[f.idx()].sym).name))));
                    w.push(Kind::Method, None, &self.declared_params(f));
                    self.walk_fun(w, f, false);
                    w.pop();
                    w.leave();
                }
                _ => self.walk_class_lifted(w, ClassId(id)),
            }
        }
        for c in self.unit_rest(file, &w.unit, &opaque) {
            self.walk_class_lifted(w, c);
        }
    }

    /// The owner of the code of the lazy member `s` (raw name `raw`, initialiser number `m` by the file's walk) as
    /// `LambdaLift` meets it: its initialiser `x$lzyINITM`, or the getter `x` for a `@threadUnsafe` one.
    fn lazy_member_owner(&self, s: SymId, raw: &str, m: u32) -> OwnerName {
        if self.is_thread_unsafe(s) { OwnerName::method(raw) } else { OwnerName::lazy_initialiser(raw, m) }
    }

    /// A class's template in the order `LambdaLift` meets it after the phases before it. A local class first takes
    /// its own name (`liftLocals` names it with the defs, `L$N`). Then its constructor's code: its superclass call's
    /// arguments that are not hoisted, its traits' arguments (`Mixin`'s initialiser calls after the superclass's),
    /// then its fields' initialisers and statements in their order (`Constructors`). Then its members by place, a
    /// method's default getters after it, a nested object's module class after its companion class
    /// (`FirstTransform`), an object's companion class's constructor getters first (`Desugar.classDef`'s
    /// `companionDefs`, merged before the object's body by `Namer.mergeModuleClass`) and its value class's extension
    /// methods last (`ExtensionMethods`). Then the arguments `HoistSuperArgs` hoisted (a top-level class's static
    /// methods at the end of its body, another's beside it in its owner), and where the class's companion is made,
    /// what that companion holds of it.
    fn walk_class_lifted(&mut self, w: &mut LiftWalk, c: ClassId) {
        if w.unit.walked.insert(c, ()).is_some() {
            return;
        }
        let cx = self.cx;
        let syms = cx.input.syms;
        let prog = cx.input.prog;
        let idx = cx.tclass_of[c.idx()];
        if idx == u32::MAX {
            return;
        }
        let tc = &prog.classes[idx as usize];
        let info = syms.class(c);
        let local = info.owner == Owner::Local;
        let name = if info.kind == ClassKind::Anon { "$anon" } else { cx.input.interner.get(info.name) };
        if local {
            w.fresh(name);
        }
        let is_trait = info.kind == ClassKind::Trait;
        // A class a dropped typing made is no class of dotty's tree (`ClassInfo::dropped`): its names are drawn last,
        // its dependencies none.
        let suspended = if syms.in_dropped_local(c) { w.deps.take() } else { None };
        // A companion module recorded before its walk (`companion_encl`) is entered again.
        let recorded = w.deps.as_deref().and_then(|d| d.classes.get(&c).copied());
        // A local class stands in a block and a member class in its owner's template, wherever the walk meets them (a
        // companion the typer makes, inside its class; a class the walk meets after the file's).
        let class_encl = match (recorded, w.deps.as_deref_mut()) {
            (Some((e, _)), Some(d)) => {
                d.reenter(e);
                e
            }
            (_, Some(d)) => {
                let at = match info.owner {
                    Owner::Local => d.block(),
                    Owner::Class(o) => d.classes.get(&o).map_or(d.current(), |&(e, _)| e),
                    Owner::Package(_) => d.current(),
                };
                d.push_at(at, Kind::Class { real: !is_trait }, Some(Callee::Class(c)), &[])
            }
            (_, None) => 0,
        };
        if !is_trait && w.deps.is_some() {
            for m in self.forwarded_members(c) {
                w.push(Kind::Method, None, &[]);
                w.call(Callee::TraitMember(m));
                w.pop();
            }
        }
        let outer_scope = std::mem::replace(&mut w.unit.scope, info.span.start);
        let file = info.module_file();
        // A lazy member's right-hand side is its initialiser's code, `x$lzyINITM` (`LazyVals`), a trait's and a
        // `@threadUnsafe` one's its getter's own (`LazyVals` leaves a trait's lazy val to the classes mixing it in, and
        // writes a `@threadUnsafe` one's right-hand side in the getter, `transformMemberDefThreadUnsafe`).
        let lazy: FxMap<SymId, OwnerName> = tc
            .init
            .iter()
            .filter_map(|i| if let TInit::Field(s, _) = *i { Some(s) } else { None })
            .filter_map(|s| {
                let raw = cx.input.interner.get(syms.sym(s).name);
                if info.kind == ClassKind::Trait {
                    return cx.is_lazy_member(s).then(|| (s, OwnerName::method(raw)));
                }
                let getter = self.field_name(s);
                self.lazy_member_numbers_in(file, Some(c), &getter).map(|(_, m)| (s, self.lazy_member_owner(s, raw, m)))
            })
            .collect();
        let mut hoisted: Vec<(TExprId, u32)> = Vec::new();
        w.enter(Some(OwnerName::constructor()));
        let ctor_encl = if is_trait {
            w.push(Kind::Method, Some(Callee::TraitInit(c)), &[])
        } else {
            let own = if info.kind == ClassKind::Anon { &[][..] } else { &tc.ctor_params[tc.captures.min(tc.ctor_params.len())..] };
            let e = match (recorded, w.deps.as_deref_mut()) {
                (Some((_, k)), Some(d)) => {
                    d.reenter(k);
                    k
                }
                _ => w.push(Kind::Ctor, Some(Callee::Ctor(c)), own),
            };
            if w.deps.is_some() {
                if local {
                    w.call(Callee::Class(c));
                }
                for t in self.mixed_local_traits(c) {
                    w.call(Callee::TraitInit(t));
                }
                if let Some(k) = info.superclass.filter(|&k| syms.class(k).owner == Owner::Local) {
                    w.call(tc.parent_via.map_or(Callee::Ctor(k), Callee::Fun));
                }
            }
            e
        };
        if let Some(d) = w.deps.as_deref_mut() {
            d.classes.insert(c, (class_encl, ctor_encl));
        }
        // An anonymous class whose first parent is a class has its superclass's arguments evaluated where it is made
        // (`tir::parent_call_at_creation`), which `walk_anonymous_lifted` walks; one over a trait makes the call in its
        // own constructor, as a named class does.
        let at_creation = info.kind == ClassKind::Anon && crate::tir::parent_call_at_creation(syms, cx.input.types, c);
        let parent = if at_creation { (ListRef::EMPTY, None) } else { (tc.parent_prelude, tc.parent_args) };
        for s in prog.stmts[parent.0.range()].to_vec() {
            if let (TStmt::Val(v, _), Some(d)) = (s, w.deps.as_deref_mut()) {
                d.declare(v);
            }
            match s {
                TStmt::Val(_, e) | TStmt::Expr(e) | TStmt::Pat(_, e, _) => self.super_arg(w, e, &mut hoisted),
                TStmt::Fun(_) => self.walk_stmt(w, s),
            }
        }
        if let Some(args) = parent.1 {
            for &a in prog.expr_list(args) {
                self.super_arg(w, a, &mut hoisted);
            }
        }
        for init in &tc.init {
            if let TInit::Parent(b, call) = *init {
                self.walk_stmts(w, call.prelude);
                let args = prog.expr_list(call.args);
                // A local trait's initialiser takes its proxies first (`add_trait_forwarders`).
                if w.deps.is_some() && syms.class(b).kind == ClassKind::Trait {
                    let n = cx.tclass_of.get(b.idx()).filter(|&&i| i != u32::MAX).map_or(0, |&i| prog.classes[i as usize].captures);
                    for &a in &args[..n.min(args.len())] {
                        w.proxy_args.insert(a, ());
                    }
                }
                for &a in args {
                    self.walk_expr(w, a);
                }
            }
        }
        let mut code: Vec<(u32, usize)> = Vec::new();
        for (i, init) in tc.init.iter().enumerate() {
            match *init {
                TInit::Field(s, _) if !lazy.contains_key(&s) => code.push((syms.sym(s).span.start, i)),
                TInit::Stmt(e) => code.push((self.expr_span(e).map_or(0, |sp| sp.start), i)),
                _ => {}
            }
        }
        code.sort();
        for (_, i) in code {
            if let TInit::Field(_, e) | TInit::Stmt(e) = tc.init[i] {
                self.walk_expr(w, e);
            }
        }
        w.pop();
        w.leave();
        // The members by their place: methods, secondary constructors, lazy members, member classes.
        let mut items: Vec<(u32, bool, u8, usize)> = Vec::new();
        if !info.value_class {
            for (i, &f) in tc.methods.iter().enumerate() {
                items.push((syms.sym(prog.funs[f.idx()].sym).span.start, false, 1, i));
            }
        }
        for (i, &f) in tc.ctors.iter().enumerate() {
            items.push((syms.sym(prog.funs[f.idx()].sym).span.start, false, 3, i));
        }
        for (i, init) in tc.init.iter().enumerate() {
            if let TInit::Field(s, _) = *init {
                if lazy.contains_key(&s) {
                    items.push((syms.sym(s).span.start, false, 4, i));
                }
            }
        }
        let mut nested = self.nested_classes(tc);
        for init in &tc.init {
            if let TInit::Field(s, _) = *init {
                if let SymKind::Object(k) = syms.sym(s).kind {
                    if !nested.contains(&k) {
                        nested.push(k);
                    }
                }
            }
        }
        let placed: Vec<(u32, ClassId)> = nested.iter().map(|&k| (syms.class(k).span.start, k)).collect();
        for (i, (at, after, _)) in self.companions_after(&placed).into_iter().enumerate() {
            items.push((at, after, 2, i));
        }
        items.sort();
        let object = self.is_object(c);
        let held = info.companion.filter(|&k| self.companion_holding(k) == Some(c));
        if let Some(k) = held {
            self.walk_constructor_defaults(w, k);
        }
        for (_, _, kind, i) in items {
            match kind {
                1 => {
                    let f = tc.methods[i];
                    let s = prog.funs[f.idx()].sym;
                    w.enter(Some(OwnerName::method(cx.input.interner.get(syms.sym(s).name))));
                    match self.trait_lazy_of_initialiser(tc, s).filter(|_| is_trait && local) {
                        Some(v) => w.trait_lazy(v),
                        None => {
                            w.push(Kind::Method, (is_trait && local).then_some(Callee::TraitMember(s)), &self.declared_params(f));
                        }
                    }
                    self.walk_fun(w, f, false);
                    w.pop();
                    w.leave();
                }
                3 => self.walk_secondary_constructor(w, tc.ctors[i], &mut hoisted),
                4 => {
                    if let TInit::Field(s, e) = tc.init[i] {
                        w.enter(Some(lazy[&s].clone()));
                        if is_trait && local {
                            w.trait_lazy(s);
                        } else {
                            w.push(Kind::Method, None, &[]);
                        }
                        self.walk_expr(w, e);
                        w.pop();
                        w.leave();
                    }
                }
                _ => self.walk_class_lifted(w, placed[i].1),
            }
        }
        if let Some(k) = held {
            self.walk_extension_methods(w, k);
        }
        // Its local classes the walk has not met yet, at their places.
        self.walk_local_classes_before(w, info.span.end);
        w.unit.scope = outer_scope;
        w.pop();
        let own = if info.kind == ClassKind::Anon { &[][..] } else { &tc.ctor_params[tc.captures.min(tc.ctor_params.len())..] };
        self.walk_hoisted(w, name, local, own, hoisted);
        // A companion the source does not write stands just after the class (`Desugar.classDef`'s `companionDefs`);
        // one with no template holds the class's getters and extension methods there.
        let made = |k: ClassId| syms.class(k).synthetic_companion || syms.class(k).made_companion || syms.class(k).def.is_none();
        if let Some(k) = info.companion.filter(|&k| !object && self.is_object(k) && made(k)) {
            self.walk_class_lifted(w, k);
        }
        if !object && self.companion_holding(c).is_none() {
            self.walk_constructor_defaults(w, c);
            self.walk_extension_methods(w, c);
        }
        if suspended.is_some() {
            w.deps = suspended;
        }
    }

    /// The creation of the anonymous class `c` (`{ class $anon ..; new $anon(..) }`): its template, then the
    /// superclass's arguments after its captures, those `HoistSuperArgs` hoists beside the class, then the captures.
    fn walk_anonymous_lifted(&mut self, w: &mut LiftWalk, c: ClassId, args: ListRef) {
        let items = self.cx.input.prog.expr_list(args);
        let captures = self.capture_count(c).min(items.len());
        if w.unit.walked.contains_key(&c) {
            for &a in items {
                self.walk_expr(w, a);
            }
            return;
        }
        self.walk_class_lifted(w, c);
        // The arguments are its constructor's code (`{ class $anon extends C(args) ..; new $anon }`).
        let ctor = w.deps.as_deref().and_then(|d| d.classes.get(&c).map(|&(_, k)| k));
        if let (Some(d), Some(k)) = (w.deps.as_deref_mut(), ctor) {
            d.reenter(k);
        }
        let mut hoisted: Vec<(TExprId, u32)> = Vec::new();
        for &a in &items[captures..] {
            self.super_arg(w, a, &mut hoisted);
        }
        if ctor.is_some() {
            w.pop();
        }
        self.walk_hoisted(w, "$anon", true, &[], hoisted);
        for &a in &items[..captures] {
            self.walk_expr(w, a);
        }
    }

    /// The arguments `HoistSuperArgs` hoisted out of the class `name`'s calls: each a method `C$superArg$N`
    /// (`SuperArgName`, numbered per class name as the phase meets the classes, after what they hold), a local
    /// class's freshened again by `LambdaLift`.
    /// Each takes the class's constructor's parameters `params` and is called by the constructor whose call held it.
    fn walk_hoisted(&mut self, w: &mut LiftWalk, name: &str, local: bool, params: &[SymId], hoisted: Vec<(TExprId, u32)>) {
        for (a, caller) in hoisted {
            let n = w.super_args.entry(name.to_string()).or_insert(0);
            *n += 1;
            let (raw, spelled) = (format!("{}$superArg${}", name, n), format!("{}$superArg${}", encode(name), n));
            let owner = if local { w.freshened(&raw, &spelled) } else { OwnerName::new(raw, spelled) };
            w.enter(Some(owner));
            if let Some(d) = w.deps.as_deref_mut() {
                let k = d.next_super_arg();
                d.call_from(caller, Callee::SuperArg(k));
                d.push(Kind::Method, Some(Callee::SuperArg(k)), params);
            }
            self.walk_expr(w, a);
            w.pop();
            w.leave();
        }
    }

    /// An argument of a superclass's or a self constructor's call: hoisted where `HoistSuperArgs.needsHoist` finds a
    /// definition (a lambda's, a local def's), a template or a class that is not static made in it, else the
    /// constructor's code.
    fn super_arg(&mut self, w: &mut LiftWalk, a: TExprId, hoisted: &mut Vec<(TExprId, u32)>) {
        let cx = self.cx;
        let prog = cx.input.prog;
        let complex = prog.descendants(a).any(|x| match prog.expr(x) {
            TExpr::Lambda(..) => true,
            TExpr::Block(stmts, _) => prog.stmts[stmts.range()].iter().any(|s| matches!(s, TStmt::Fun(_))),
            TExpr::New(k, _) => !self.static_class(k),
            TExpr::NewVia(ctor, _) => match cx.input.syms.sym(ctor).owner {
                Owner::Class(k) => !self.static_class(k),
                _ => false,
            },
            _ => false,
        });
        if complex {
            hoisted.push((a, w.deps.as_deref().map_or(0, |d| d.current())));
        } else {
            self.walk_expr(w, a);
        }
    }

    /// Whether the class `c` is static: every class around it an object (`Symbol.isStatic`).
    fn static_class(&self, c: ClassId) -> bool {
        let syms = self.cx.input.syms;
        let mut at = syms.class(c).owner;
        loop {
            match at {
                Owner::Package(_) => return true,
                Owner::Class(o) if syms.class(o).kind == ClassKind::Object => at = syms.class(o).owner,
                _ => return false,
            }
        }
    }

    /// A secondary constructor at its place: the arguments of its self constructor call, and the temporaries before
    /// it, hoisted where `HoistSuperArgs` hoists them (`hoistSuperArgsFromConstr`), then its code.
    fn walk_secondary_constructor(&mut self, w: &mut LiftWalk, f: FunId, hoisted: &mut Vec<(TExprId, u32)>) {
        let cx = self.cx;
        let prog = cx.input.prog;
        let fun = &prog.funs[f.idx()];
        let outer_scope = std::mem::replace(&mut w.unit.scope, cx.input.syms.sym(fun.sym).span.start);
        w.enter(Some(OwnerName::constructor()));
        w.push(Kind::Ctor, Some(Callee::Fun(fun.sym)), &self.declared_params(f));
        match fun.body.map(|b| (b, prog.expr(b))) {
            Some((b, TExpr::Block(stmts, res))) => {
                let list = prog.stmts[stmts.range()].to_vec();
                let call = list.iter().position(|s| matches!(*s, TStmt::Expr(x) if matches!(prog.expr(x), TExpr::New(..) | TExpr::NewVia(..))));
                for (j, s) in list.into_iter().enumerate() {
                    match (s, call) {
                        (TStmt::Expr(x), Some(k)) if j == k => {
                            // The self constructor's call, the class's captures passed first.
                            let captured: Vec<bool> = match prog.expr(x) {
                                TExpr::New(c, args) => {
                                    w.call(Callee::Ctor(c));
                                    (0..args.len as usize).map(|i| i < self.capture_count(c)).collect()
                                }
                                TExpr::NewVia(s, _) => {
                                    w.call(Callee::Fun(s));
                                    self.captured_params(s)
                                }
                                _ => Vec::new(),
                            };
                            if let TExpr::New(_, args) | TExpr::NewVia(_, args) = prog.expr(x) {
                                for (&a, _) in prog.expr_list(args).iter().zip(&captured).filter(|&(_, &c)| c) {
                                    w.proxy_args.insert(a, ());
                                }
                            }
                            if let TExpr::New(_, args) | TExpr::NewVia(_, args) = prog.expr(x) {
                                for &a in prog.expr_list(args) {
                                    self.super_arg(w, a, hoisted);
                                }
                            }
                        }
                        (TStmt::Val(v, e), Some(k)) if j < k => {
                            if let Some(d) = w.deps.as_deref_mut() {
                                d.declare(v);
                            }
                            self.super_arg(w, e, hoisted)
                        }
                        _ => self.walk_stmt(w, s),
                    }
                }
                if let Some(sp) = self.expr_span(res) {
                    self.walk_local_classes_before(w, sp.start);
                }
                self.walk_expr(w, res);
                if let Some(sp) = self.expr_span(b) {
                    self.walk_local_classes_before(w, sp.end);
                }
            }
            Some((b, _)) => self.walk_expr(w, b),
            None => {}
        }
        w.pop();
        w.leave();
        let companion = match cx.input.syms.sym(fun.sym).owner {
            Owner::Class(c) if fun.defaults.iter().any(Option::is_some) => self.companion_encl(w, c),
            _ => None,
        };
        for (i, d) in fun.own_defaults() {
            w.enter(Some(OwnerName::constructor_default(i)));
            if let (Some(e), Some(deps)) = (companion, w.deps.as_deref_mut()) {
                deps.reenter(e);
            }
            w.push(Kind::Method, None, &self.declared_params(f));
            self.walk_expr(w, self.by_name_default(fun.sym, i, d));
            w.pop();
            if companion.is_some() {
                w.pop();
            }
            w.leave();
        }
        w.unit.scope = outer_scope;
    }

    /// A default `d` of the parameter `i` of `s` as its getter returns it: a by-name parameter's the thunk's body.
    fn by_name_default(&self, s: SymId, i: usize, d: TExprId) -> TExprId {
        match self.cx.input.prog.expr(d) {
            TExpr::Lambda(ps, body) if ps.len == 0 && self.local_param_by_name(s, i) => body,
            _ => d,
        }
    }

    /// The getters of the class `c`'s constructor's defaults (`$lessinit$greater$default$N`), its companion's.
    fn walk_constructor_defaults(&mut self, w: &mut LiftWalk, c: ClassId) {
        let cx = self.cx;
        let idx = cx.tclass_of[c.idx()];
        if idx == u32::MAX {
            return;
        }
        let tc = &cx.input.prog.classes[idx as usize];
        let clauses = &cx.input.syms.class(c).ctor;
        let declared: usize = clauses.iter().map(|cl| cl.params.len()).sum();
        let captures = tc.ctor_params.len().saturating_sub(declared);
        if !tc.ctor_defaults.iter().skip(captures).any(Option::is_some) {
            return;
        }
        let params = &tc.ctor_params[captures..];
        let companion = self.companion_encl(w, c);
        for (i, d) in tc.ctor_defaults.iter().enumerate() {
            let Some(d) = *d else { continue };
            if i < captures {
                continue;
            }
            let d = match cx.input.prog.expr(d) {
                TExpr::Lambda(ps, body) if ps.len == 0 && clause_shape(clauses, i - captures).1.is_some() => body,
                _ => d,
            };
            w.enter(Some(OwnerName::constructor_default(i)));
            if let (Some(e), Some(deps)) = (companion, w.deps.as_deref_mut()) {
                deps.reenter(e);
            }
            w.push(Kind::Method, None, params);
            self.walk_expr(w, d);
            w.pop();
            if companion.is_some() {
                w.pop();
            }
            w.leave();
        }
    }

    /// The companion module holding the local class `c`'s constructors' default getters (dotty's namer makes one
    /// for a local class with defaults, a module a lazy val of the block holds): its class and constructor, recorded
    /// here before the module's own walk where it has one (which enters them again); the val that holds it is met by
    /// its right-hand side (`Deps::module_vals`), a module no val holds given an initialiser and an accessor of its
    /// own (`Deps::proxies`).
    fn companion_encl(&self, w: &mut LiftWalk, c: ClassId) -> Option<u32> {
        let syms = self.cx.input.syms;
        if syms.class(c).owner != Owner::Local {
            return None;
        }
        let d = w.deps.as_deref_mut()?;
        let module = syms.class(c).companion;
        if let Some(&(e, _)) = module.and_then(|k| d.classes.get(&k)) {
            d.companions.entry(c).or_insert((module, e));
            return Some(e);
        }
        if let Some(&(_, e)) = d.companions.get(&c) {
            return Some(e);
        }
        let parent = d.classes.get(&c).map_or(d.current(), |&(e, _)| d.parent_of(e));
        let class = d.push_at(parent, Kind::Class { real: true }, module.map(Callee::Class), &[]);
        let ctor = d.push(Kind::Ctor, module.map(Callee::Ctor), &[]);
        d.pop();
        d.pop();
        d.link(ctor, class);
        if let Some(k) = module {
            d.classes.insert(k, (class, ctor));
        }
        d.companions.insert(c, (module, class));
        Some(class)
    }

    /// The methods of the value class `c` as its companion's extension methods, `m$extension`, by their place.
    fn walk_extension_methods(&mut self, w: &mut LiftWalk, c: ClassId) {
        let cx = self.cx;
        let syms = cx.input.syms;
        let idx = cx.tclass_of[c.idx()];
        if !syms.class(c).value_class || idx == u32::MAX {
            return;
        }
        let prog = cx.input.prog;
        let mut funs: Vec<(u32, FunId)> = prog.classes[idx as usize].methods.iter().map(|&f| (syms.sym(prog.funs[f.idx()].sym).span.start, f)).collect();
        funs.sort();
        w.extension = true;
        for (_, f) in funs {
            let raw = cx.input.interner.get(syms.sym(prog.funs[f.idx()].sym).name);
            w.enter(Some(OwnerName::new(format!("{}$extension", raw), format!("{}$extension", encode(raw)))));
            w.push(Kind::Method, None, &self.declared_params(f));
            self.walk_fun(w, f, false);
            w.pop();
            w.leave();
        }
        w.extension = false;
    }
}

static EMPTY: std::sync::LazyLock<LiftedNames> = std::sync::LazyLock::new(LiftedNames::default);
