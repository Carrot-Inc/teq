//! A session's reach kept between its builds (docs/TARGETS.md, "The kept reach"). A retype that
//! leaves the walk of every retyped unit as it was keeps the last reach, the units' ids moved to
//! the ones the retype made; any other retype walks from the roots again, and that reach is kept.
//!
//! A unit's walk (`UnitWalk`) is what the reach's walk meets in the unit's definitions, in the
//! order `reach::visit` meets it, beside what the walk reads of them: the layout's facts of the
//! top-level vals and the classes, the parts of the classes' records, the template calls, and for
//! each symbol, class and function the unit's typing made, its facts where the walk first meets
//! it. What the typing made is named by that order and the rest by its id, so that two typings of
//! a unit walk equal exactly where the reach acts alike on them; the comparison is per unit, and
//! whatever it does not cover sends the build to the walk from the roots.

use super::layout::{is_literal_init, runs_body};
#[cfg(debug_assertions)]
use super::reach::check_kept;
use super::reach::{visit, visit_list, Cx, Meet, Place, Reach, Setup};
use crate::ast::{mods, ListRef};
use crate::intern::{FxMap, Name};
use crate::source::FileId;
use crate::symbols::*;
use crate::tir::*;
use crate::typer::Worker;
use crate::types::*;

/// A symbol or a class as two typings of a unit compare: its id, or for what the unit's typing
/// made, the order in which the walk first met it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Id {
    Id(u32),
    Made(u32),
}

#[derive(Clone, PartialEq, Eq, Debug)]
enum Event {
    Expansion,
    JsGlobal(Name),
    Local(Id),
    Class(Id),
    Static(Id),
    ClassOf(Id),
    Field(bool, Id),
    CallStatic(Id),
    CallMethod(bool, Id),
    CallJs(Name),
    JsSelect(Name, Option<Id>),
    New(Id),
    NewVia(Id),
    Lambda(Option<Id>),
    Object(Id),
    LocalFun(u32),
    AssignField(Id),
    SeqLit,
    Template(String, Vec<Option<String>>),
    TestArray,
    JsImport(u32),
    ThrowUnwrap,
    TryEnd(bool),
    PatSeq,
    PatRest,
    /// A part of a definition the walk reads begins.
    Part(&'static str),
    Sym(Id),
    Fact(u64),
    Absent,
}

/// What the reach's walk meets in a unit's definitions, and the ids the unit's typing made, in
/// the order the comparison names them by.
pub struct UnitWalk {
    unit: FileId,
    events: Vec<Event>,
    syms: Vec<SymId>,
    classes: Vec<ClassId>,
    funs: Vec<FunId>,
    roots: Vec<TExprId>,
}

/// Whether the typing of a unit made `s`: a local, or a member of a class the typing made.
fn made_sym(syms: &Symbols, s: SymId) -> bool {
    match syms.sym(s).owner {
        Owner::Local => true,
        Owner::Class(c) => made_class(syms, c),
        Owner::Package(_) => false,
    }
}

/// Whether the typing of a unit made `c`: a local or anonymous class.
fn made_class(syms: &Symbols, c: ClassId) -> bool {
    syms.class(c).owner == Owner::Local
}

/// The walk of `unit` as `typer` holds it, for a reach that collects the names JavaScript sees
/// (`js_visible`, a release build's) or not.
pub fn unit_walk(typer: &Worker, unit: FileId, js_visible: bool) -> UnitWalk {
    let cx = Cx::of(typer);
    let (prog, syms) = (cx.prog, cx.syms);
    let mut records: FxMap<ClassId, u32> = FxMap::default();
    let mut own_classes: Vec<ClassId> = Vec::new();
    for (i, tc) in prog.classes.iter().enumerate() {
        records.insert(tc.id, i as u32);
        if !made_class(syms, tc.id) && syms.class_of_files(tc.id, &[unit]) {
            own_classes.push(tc.id);
        }
    }
    own_classes.sort_unstable();
    let mut r = Recorder {
        cx,
        js_visible,
        records,
        events: Vec::new(),
        made_syms: FxMap::default(),
        made_classes: FxMap::default(),
        made_funs: FxMap::default(),
        to_describe: Vec::new(),
        described: 0,
        walk: UnitWalk { unit, events: Vec::new(), syms: Vec::new(), classes: Vec::new(), funs: Vec::new(), roots: Vec::new() },
    };
    let mut vals: Vec<(SymId, TExprId)> = prog.top_vals.iter().copied().filter(|&(s, _)| syms.sym(s).file == unit).collect();
    vals.sort_unstable_by_key(|&(s, _)| s);
    for (s, init) in vals {
        r.events.push(Event::Part("top-level val"));
        let id = r.sym(s);
        r.events.push(Event::Sym(id));
        let literal = is_literal_init(prog, init) && syms.sym(s).kind == SymKind::Val;
        r.events.push(Event::Fact(literal as u64 | (typer.constant_value(s).is_some() as u64) << 1));
        visit(&mut r, cx, init);
    }
    let mut funs: Vec<FunId> = prog.top_funs.iter().copied().filter(|&f| syms.sym(prog.funs[f.idx()].sym).file == unit).collect();
    funs.sort_unstable_by_key(|&f| prog.funs[f.idx()].sym);
    for f in funs {
        r.events.push(Event::Part("top-level function"));
        r.fun(f);
    }
    for c in own_classes {
        r.events.push(Event::Part("class"));
        let id = r.class(c);
        r.events.push(Event::Class(id));
        r.record(c);
    }
    r.events.push(Event::Part("template calls"));
    for &(s, u) in prog.template_calls.iter() {
        if u == unit {
            let id = r.sym(s);
            r.events.push(Event::Sym(id));
        }
    }
    r.describe_met();
    // The walk reaches the targets of every record's super accessors, a local class's that nothing
    // makes among them: the unit's made classes nothing met are described too, in their order.
    let unmet: Vec<ClassId> = prog.classes.iter().map(|tc| tc.id).filter(|&c| made_class(syms, c) && syms.class_of_files(c, &[unit]) && !r.made_classes.contains_key(&c)).collect();
    for c in unmet {
        r.events.push(Event::Part("made class nothing met"));
        let id = r.class(c);
        r.events.push(Event::Class(id));
        r.describe_met();
    }
    r.walk.events = std::mem::take(&mut r.events);
    r.walk
}

#[derive(Clone, Copy)]
enum Thing {
    Class(ClassId),
    Fun(FunId),
}

struct Recorder<'c> {
    cx: Cx<'c>,
    js_visible: bool,
    /// Each class's record in the program, by class.
    records: FxMap<ClassId, u32>,
    events: Vec<Event>,
    made_syms: FxMap<SymId, u32>,
    made_classes: FxMap<ClassId, u32>,
    made_funs: FxMap<FunId, u32>,
    /// The classes and local functions the typing made, met, and how many of them are described.
    to_describe: Vec<Thing>,
    described: usize,
    walk: UnitWalk,
}

impl<'c> Recorder<'c> {
    /// Describes the classes and local functions met and not yet described, and those they meet.
    fn describe_met(&mut self) {
        while self.described < self.to_describe.len() {
            let thing = self.to_describe[self.described];
            self.described += 1;
            match thing {
                Thing::Class(c) => self.describe_class(c),
                Thing::Fun(f) => {
                    self.events.push(Event::Part("local function"));
                    self.fun(f);
                }
            }
        }
    }

    fn sym(&mut self, s: SymId) -> Id {
        let syms = self.cx.syms;
        if !made_sym(syms, s) {
            return Id::Id(s.0);
        }
        if let Some(&k) = self.made_syms.get(&s) {
            return Id::Made(k);
        }
        let k = self.walk.syms.len() as u32;
        self.walk.syms.push(s);
        self.made_syms.insert(s, k);
        let info = syms.sym(s);
        self.events.push(Event::Part("made symbol"));
        self.events.push(Event::Fact(info.name.0 as u64));
        self.events.push(Event::Fact(info.mods as u64));
        self.events.push(Event::Fact(info.is_extension as u64 | (info.scoped_private as u64) << 1 | (self.cx.typer.is_abstract_member(s) as u64) << 2));
        self.events.push(Event::Fact(syms.dispatch_name(s).0 as u64));
        match info.kind {
            SymKind::Object(c) | SymKind::EnumValue(c) => {
                self.events.push(Event::Fact(matches!(info.kind, SymKind::Object(_)) as u64));
                let id = self.class(c);
                self.events.push(Event::Class(id));
            }
            SymKind::Overloaded(_) => {
                self.events.push(Event::Part("alternatives"));
                for &a in syms.alternatives(s).unwrap_or(&[]) {
                    let id = self.sym(a);
                    self.events.push(Event::Sym(id));
                }
            }
            kind => self.events.push(Event::Fact(kind_code(kind))),
        }
        if let Owner::Class(c) = info.owner {
            let id = self.class(c);
            self.events.push(Event::Class(id));
        }
        Id::Made(k)
    }

    fn class(&mut self, c: ClassId) -> Id {
        if !made_class(self.cx.syms, c) {
            return Id::Id(c.0);
        }
        if let Some(&k) = self.made_classes.get(&c) {
            return Id::Made(k);
        }
        let k = self.walk.classes.len() as u32;
        self.walk.classes.push(c);
        self.made_classes.insert(c, k);
        self.to_describe.push(Thing::Class(c));
        Id::Made(k)
    }

    fn opt_sym(&mut self, s: Option<SymId>) {
        match s {
            Some(s) => {
                let id = self.sym(s);
                self.events.push(Event::Sym(id));
            }
            None => self.events.push(Event::Absent),
        }
    }

    fn opt_class(&mut self, c: Option<ClassId>) {
        match c {
            Some(c) => {
                let id = self.class(c);
                self.events.push(Event::Class(id));
            }
            None => self.events.push(Event::Absent),
        }
    }

    fn opt_expr(&mut self, e: Option<TExprId>) {
        match e {
            Some(e) => {
                self.events.push(Event::Part("expression"));
                let cx = self.cx;
                visit(self, cx, e);
            }
            None => self.events.push(Event::Absent),
        }
    }

    /// What the walk reads of a class the unit's typing made, beside its record.
    fn describe_class(&mut self, c: ClassId) {
        let info = self.cx.syms.class(c);
        self.events.push(Event::Part("made class"));
        let k = *self.made_classes.get(&c).expect("described once met");
        self.events.push(Event::Class(Id::Made(k)));
        // Not its name, which an anonymous class takes from its place; the walk reads its
        // members' names.
        self.events.push(Event::Fact(class_kind_code(info.kind)));
        self.events.push(Event::Fact(info.mods as u64));
        self.events.push(Event::Fact(js_code(info.js) | (info.stateful as u64) << 8 | (info.has_overloads as u64) << 9 | (info.named_for_output as u64) << 10));
        self.events.push(Event::Part("bases"));
        for (b, _) in info.base_types.clone() {
            let id = self.class(b);
            self.events.push(Event::Class(id));
        }
        let (superclass, companion, singleton, local_module) = (info.superclass, info.companion, info.singleton, info.local_module);
        self.opt_class(superclass);
        self.opt_class(companion);
        self.opt_sym(singleton);
        self.opt_sym(local_module);
        let mut members: Vec<(Name, SymId)> = info.members.iter().map(|(&n, &s)| (n, s)).collect();
        members.sort_unstable_by_key(|&(n, _)| n.0);
        let order = info.member_order.clone();
        let extensions = info.extensions.clone();
        let children = info.children.clone();
        let ctor_syms: Vec<SymId> = info.ctor_syms.iter().flatten().copied().collect();
        self.events.push(Event::Part("members"));
        for (n, s) in members {
            self.events.push(Event::Fact(n.0 as u64));
            let id = self.sym(s);
            self.events.push(Event::Sym(id));
        }
        for (part, list) in [("member order", order), ("extensions", extensions), ("constructor parameters", ctor_syms)] {
            self.events.push(Event::Part(part));
            for s in list {
                let id = self.sym(s);
                self.events.push(Event::Sym(id));
            }
        }
        self.events.push(Event::Part("children"));
        for k in children {
            let id = self.class(k);
            self.events.push(Event::Class(id));
        }
        self.record(c);
    }

    /// A class's record in the program, as the walk reads it, its methods' bodies included.
    fn record(&mut self, c: ClassId) {
        let Some(&i) = self.records.get(&c) else {
            self.events.push(Event::Absent);
            return;
        };
        let prog = self.cx.prog;
        let tc = &prog.classes[i as usize];
        self.events.push(Event::Part("record"));
        self.events.push(Event::Fact(runs_body(prog, self.cx.syms, tc) as u64 | (tc.captures as u64) << 1));
        for &d in &tc.ctor_defaults {
            self.opt_expr(d);
        }
        self.events.push(Event::Part("parent arguments"));
        match tc.parent_args {
            Some(args) => {
                let cx = self.cx;
                visit_list(self, cx, args)
            }
            None => self.events.push(Event::Absent),
        }
        self.opt_sym(tc.parent_via);
        self.prelude(tc.parent_prelude);
        for init in &tc.init {
            match *init {
                TInit::Field(s, e) => {
                    self.events.push(Event::Part("field"));
                    let id = self.sym(s);
                    self.events.push(Event::Sym(id));
                    self.events.push(Event::Fact((self.cx.syms.sym(s).mods & mods::LAZY != 0) as u64));
                    let cx = self.cx;
                visit(self, cx, e);
                }
                TInit::Stmt(e) => {
                    self.events.push(Event::Part("statement"));
                    let cx = self.cx;
                visit(self, cx, e);
                }
                TInit::Parent(b, call) => {
                    self.events.push(Event::Part("parent"));
                    let id = self.class(b);
                    self.events.push(Event::Class(id));
                    let cx = self.cx;
                    visit_list(self, cx, call.args);
                    self.prelude(call.prelude);
                }
            }
        }
        self.events.push(Event::Part("forwarders"));
        for &(a, b) in &tc.forwarders {
            for s in [a, b] {
                let id = self.sym(s);
                self.events.push(Event::Sym(id));
            }
        }
        self.events.push(Event::Part("super accessors"));
        for a in &tc.super_accessors {
            let id = self.class(a.of_trait);
            self.events.push(Event::Class(id));
            let id = self.sym(a.member);
            self.events.push(Event::Sym(id));
            self.opt_sym(a.target);
        }
        self.events.push(Event::Part("bridges"));
        for &(a, b) in &tc.bridges {
            for s in [a, b] {
                let id = self.sym(s);
                self.events.push(Event::Sym(id));
            }
        }
        let mut funs: Vec<FunId> = tc.methods.iter().chain(&tc.ctors).copied().collect();
        funs.sort_unstable_by_key(|&f| (prog.funs[f.idx()].sym, tc.ctors.contains(&f)));
        for f in funs {
            self.events.push(Event::Part(if tc.ctors.contains(&f) { "constructor" } else { "method" }));
            self.fun(f);
        }
    }

    fn prelude(&mut self, stmts: ListRef) {
        self.events.push(Event::Part("prelude"));
        for stmt in &self.cx.prog.stmts[stmts.range()] {
            if let TStmt::Val(_, e) = *stmt {
                let cx = self.cx;
                visit(self, cx, e);
            }
        }
    }

    fn fun(&mut self, f: FunId) {
        self.walk.funs.push(f);
        let tf = &self.cx.prog.funs[f.idx()];
        let id = self.sym(tf.sym);
        self.events.push(Event::Sym(id));
        for &d in &tf.defaults {
            self.opt_expr(d);
        }
        self.opt_expr(tf.body);
    }
}

fn kind_code(kind: SymKind) -> u64 {
    match kind {
        SymKind::Val => 1,
        SymKind::Var => 2,
        SymKind::Def => 3,
        SymKind::Param => 4,
        SymKind::Given => 5,
        SymKind::Object(_) | SymKind::EnumValue(_) | SymKind::Overloaded(_) => 6,
    }
}

fn class_kind_code(kind: ClassKind) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = crate::intern::FxHasher::default();
    std::mem::discriminant(&kind).hash(&mut h);
    h.finish()
}

fn js_code(js: JsKind) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = crate::intern::FxHasher::default();
    std::mem::discriminant(&js).hash(&mut h);
    h.finish() & 0xff
}

impl<'c> Meet for Recorder<'c> {
    fn expansion(&mut self, e: TExprId) {
        self.walk.roots.push(e);
        self.events.push(Event::Expansion);
    }

    fn js_global(&mut self, name: Name) {
        self.events.push(Event::JsGlobal(name));
    }

    fn local(&mut self, s: SymId) {
        let id = self.sym(s);
        self.events.push(Event::Local(id));
    }

    fn class(&mut self, _cx: Cx, c: ClassId) {
        let id = Recorder::class(self, c);
        self.events.push(Event::Class(id));
    }

    fn static_ref(&mut self, _cx: Cx, s: SymId) {
        let id = self.sym(s);
        self.events.push(Event::Static(id));
    }

    fn class_of(&mut self, _cx: Cx, c: ClassId) {
        let id = Recorder::class(self, c);
        self.events.push(Event::ClassOf(id));
    }

    fn field(&mut self, _cx: Cx, of_this: bool, s: SymId) {
        let id = self.sym(s);
        self.events.push(Event::Field(of_this, id));
    }

    fn call_static(&mut self, _cx: Cx, s: SymId) {
        let id = self.sym(s);
        self.events.push(Event::CallStatic(id));
    }

    fn call_method(&mut self, _cx: Cx, through_super: bool, s: SymId) {
        let id = self.sym(s);
        self.events.push(Event::CallMethod(through_super, id));
    }

    fn call_js(&mut self, _cx: Cx, name: Name) {
        self.events.push(Event::CallJs(name));
    }

    fn js_select(&mut self, cx: Cx, on: TExprId, name: Name) {
        let module = match cx.prog.expr(on) {
            TExpr::Module(c) => Some(Recorder::class(self, c)),
            _ => None,
        };
        self.events.push(Event::JsSelect(name, module));
    }

    fn new(&mut self, _cx: Cx, c: ClassId) {
        let id = Recorder::class(self, c);
        self.events.push(Event::New(id));
    }

    fn new_via(&mut self, _cx: Cx, s: SymId) {
        let id = self.sym(s);
        self.events.push(Event::NewVia(id));
    }

    fn lambda(&mut self, _cx: Cx, forwarded: Option<SymId>) {
        let id = forwarded.map(|s| self.sym(s));
        self.events.push(Event::Lambda(id));
    }

    /// Noted, and visited as the walk visits it where it is not left out: whether it is left out is
    /// the class's `runs_body`, which its record holds.
    fn object_left_out(&mut self, c: ClassId) -> bool {
        let id = Recorder::class(self, c);
        self.events.push(Event::Object(id));
        false
    }

    fn local_fun(&mut self, _cx: Cx, f: FunId) {
        let k = match self.made_funs.get(&f) {
            Some(&k) => k,
            None => {
                let k = self.made_funs.len() as u32;
                self.made_funs.insert(f, k);
                self.to_describe.push(Thing::Fun(f));
                k
            }
        };
        self.events.push(Event::LocalFun(k));
    }

    fn assign_field(&mut self, s: SymId) {
        let id = self.sym(s);
        self.events.push(Event::AssignField(id));
    }

    fn seq_lit(&mut self, _cx: Cx) {
        self.events.push(Event::SeqLit);
    }

    /// The template's text, and the strings it takes where the walk reads them: as the names
    /// JavaScript sees, and as the member a reflective call names.
    fn template(&mut self, cx: Cx, template: StrRef, args: ListRef) {
        let prog = cx.prog;
        let text = &prog.strings[template.idx()];
        let read = self.js_visible || text.starts_with("$memberByName") || text.starts_with("$callByName");
        let strings: Vec<Option<String>> = match read {
            true => prog
                .expr_list(args)
                .iter()
                .map(|&a| match prog.expr(a) {
                    TExpr::Str(k) => Some(prog.strings[k.idx()].clone()),
                    _ => None,
                })
                .collect(),
            false => Vec::new(),
        };
        self.events.push(Event::Template(text.clone(), strings));
    }

    fn test_array(&mut self, _cx: Cx) {
        self.events.push(Event::TestArray);
    }

    fn js_import(&mut self, i: u32) {
        self.events.push(Event::JsImport(i));
    }

    fn throw_unwrap(&mut self, _cx: Cx) {
        self.events.push(Event::ThrowUnwrap);
    }

    fn try_end(&mut self, _cx: Cx, wraps: bool) {
        self.events.push(Event::TryEnd(wraps));
    }

    fn pat_seq(&mut self, _cx: Cx) {
        self.events.push(Event::PatSeq);
    }

    fn pat_rest(&mut self, _cx: Cx) {
        self.events.push(Event::PatRest);
    }
}

/// What the walk starts from besides the units' definitions, which a retype must leave as it was.
#[derive(PartialEq)]
struct Roots {
    main: Option<SymId>,
    main_object: Option<ClassId>,
    js_exports: Vec<(SymId, Name)>,
    partial_function: Option<ClassId>,
    throwable: Option<ClassId>,
    js_exception: Option<ClassId>,
}

impl Roots {
    fn of(typer: &Worker) -> Roots {
        let prog = &typer.prog;
        Roots {
            main: prog.main,
            main_object: prog.main_object,
            js_exports: prog.js_exports.clone(),
            partial_function: prog.partial_function,
            throwable: prog.throwable,
            js_exception: prog.js_exception,
        }
    }
}

/// Whether a session keeps its reach: `TEQ_KEPT_REACH=off` walks every build's from the roots.
pub fn wanted() -> bool {
    static OFF: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    !*OFF.get_or_init(|| std::env::var_os("TEQ_KEPT_REACH").is_some_and(|v| v == "off"))
}

/// What the typer holds that a walk can add to: equal before and after a walk when the walk
/// typed, entered, checked and named nothing.
#[derive(PartialEq, Eq)]
/// (The reflective registrations are made anew at every walk that reaches a lookup: their
/// expressions are not counted, their symbols taken off.)
pub struct Holdings([usize; 9]);

impl Holdings {
    pub fn of(typer: &Worker) -> Holdings {
        let prog = &typer.prog;
        Holdings([
            typer.syms.syms.len(),
            typer.syms.classes.len(),
            prog.funs.len(),
            prog.classes.len(),
            prog.top_funs.len(),
            prog.top_vals.len(),
            typer.val_init.local.len(),
            typer.class_done.local.len(),
            typer.renamed_roots.len() + typer.dispatch_pending.len(),
        ])
    }
}

/// How a settling ended (`Kept::settle`).
pub enum Settling {
    Settled,
    /// A request came before the walks were done: the reach is settled after its answer.
    GaveWay,
    /// Still typing after that many walks: nothing is kept.
    Dropped(usize),
}

/// A session's reach between its builds, with what its walk took from the typer.
pub struct Kept {
    pub reach: Reach,
    setup: Setup,
    roots: Roots,
    /// Whether the walk that made it typed nothing: a walk that types library bodies as it goes
    /// meets some of them before and some after their typing, which one from the roots over
    /// the typer it leaves does not, so only a walk that typed nothing is kept for a retype.
    settled: bool,
    /// The walks a settling took and their time, for the next answer's parts.
    pub settling: Option<(usize, std::time::Duration)>,
    /// The units retyped since the reach was made or last moved, each with its walk then: what
    /// the reach rests on of them, which a retype's or a later one's walks are compared with, a
    /// retype that failed in between included.
    baseline: Vec<UnitWalk>,
    /// What the typer held when the baseline began, of what is not the baseline units'.
    mark: Mark,
}

/// The typer's holdings when a baseline began, for what retypes added outside their units: the
/// symbols, classes and functions by how many there were, the classes with a record and the vals
/// with an initialiser by which they were.
#[derive(Default)]
struct Mark {
    syms: usize,
    classes: usize,
    funs: usize,
    records: Vec<bool>,
    vals: FxMap<SymId, ()>,
    /// The template calls (`Program::template_calls`), which the walk calls whether or not it
    /// reaches the body they stand in.
    template_calls: FxMap<(SymId, FileId), usize>,
    /// The top-level vals with an initialiser, which a reference reaches without asking.
    top_vals: FxMap<SymId, ()>,
    /// The targets of every record's super accessors, which the walk reaches whatever it reaches
    /// of the record's class.
    accessor_targets: FxMap<SymId, ()>,
}

fn accessor_targets(typer: &Worker, units: &[FileId]) -> FxMap<SymId, ()> {
    let syms = &typer.syms;
    typer.prog.classes.iter().filter(|tc| !syms.class_of_files(tc.id, units)).flat_map(|tc| tc.super_accessors.iter().filter_map(|a| a.target)).map(|t| (t, ())).collect()
}

impl Mark {
    fn of(typer: &Worker) -> Mark {
        let mut records = vec![false; typer.syms.classes.len()];
        for tc in typer.prog.classes.iter() {
            records[tc.id.idx()] = true;
        }
        Mark {
            syms: typer.syms.syms.len(),
            classes: typer.syms.classes.len(),
            funs: typer.prog.funs.len(),
            records,
            vals: typer.val_init.local.keys().map(|&s| (s, ())).collect(),
            template_calls: template_calls(typer),
            top_vals: typer.prog.top_vals.iter().map(|&(s, _)| (s, ())).collect(),
            accessor_targets: accessor_targets(typer, &[]),
        }
    }
}

fn template_calls(typer: &Worker) -> FxMap<(SymId, FileId), usize> {
    let mut calls: FxMap<(SymId, FileId), usize> = FxMap::default();
    for &call in typer.prog.template_calls.iter() {
        *calls.entry(call).or_default() += 1;
    }
    calls
}

impl Kept {
    /// The reach of a walk, `before` what the typer held when it began.
    pub fn new(typer: &Worker, reach: Reach, setup: Setup, before: &Holdings) -> Kept {
        let mut after = Holdings::of(typer);
        after.0[0] -= setup.registered_syms();
        Kept { settled: after == *before, reach, setup, roots: Roots::of(typer), settling: None, baseline: Vec::new(), mark: Mark::default() }
    }

    /// Walks from the roots again until a walk types nothing, keeping that walk's reach; a
    /// session does it between its answers, and a walk is begun and goes on only while no request
    /// is `waiting`: one that arrives is answered first, the settling taken up again after it.
    /// Not settled after `WALKS` walks, the reach is dropped.
    pub fn settle(kept: &mut Option<Kept>, typer: &mut Worker, waiting: &dyn Fn() -> bool) -> Settling {
        const WALKS: usize = 4;
        let Some(k) = kept.as_mut() else { return Settling::Settled };
        if k.settled {
            return Settling::Settled;
        }
        let start = std::time::Instant::now();
        let (array_seq, js_visible) = (k.setup.array_seq(), k.setup.js_visible());
        for walks in 1..=WALKS {
            if waiting() {
                return Settling::GaveWay;
            }
            let before = Holdings::of(typer);
            let Some((reach, setup)) = super::reach::compute_unless(typer, array_seq, js_visible, &|| waiting()) else { return Settling::GaveWay };
            *k = Kept::new(typer, reach, setup, &before);
            if k.settled {
                k.settling = Some((walks, start.elapsed()));
                return Settling::Settled;
            }
        }
        *kept = None;
        Settling::Dropped(WALKS)
    }

    /// Whether it is the reach of the program as the typer holds it: settled, and no unit
    /// retyped since it was made or moved.
    pub fn current(&self) -> bool {
        self.settled && self.baseline.is_empty()
    }

    /// Before a retype of `units`: the walks of those the reach holds none of yet.
    pub fn retyping(&mut self, typer: &Worker, units: &[FileId]) {
        if self.baseline.is_empty() {
            self.mark = Mark::of(typer);
        }
        for &u in units {
            if !self.baseline.iter().any(|b| b.unit == u) {
                let walk = unit_walk(typer, u, self.setup.js_visible());
                self.baseline.push(walk);
            }
        }
    }

    /// What the retypes since the baseline began added outside its units that the kept walk would
    /// meet, if anything: a body typed for a member it asked for and found none of or of a class
    /// it reached, a record for a class it reached, a member of a class it reached, a super
    /// accessor's target of any record (a local class's a macro's run made among them), a
    /// template call, a top-level val typed, a class or a top-level definition entered (a std
    /// file).
    ///
    /// A body that a macro's run or an inline fold typed and that nothing asked for stays out, and
    /// cannot be met by the next walk unless a unit's own walk changes: the walk takes a std or
    /// library body only through a symbol it meets (`reach_fun_of`, `reach_library_val`), meets a
    /// symbol only through an event of a walked body, a member of a class it reached (dispatch,
    /// the runtime's calls, a jar class's linked members), any record's super accessors or a
    /// template call, and asks for the body of each deferred symbol it meets without one. The
    /// kept walk met all of those over the typer as it was: the units' events are compared, the
    /// members and the records of reached classes, the accessors' targets and the template calls
    /// are checked here, and every symbol it met without a body is in its `asked`. A body typed since for a symbol outside all of that is
    /// one no walk over the units as they walk meets. (inline_fold_std's fold adds template calls
    /// and bodies of classes the walk links, which are checked; a Tailwind-like macro types std
    /// bodies nobody asked for, which are not.)
    fn added_outside(&self, typer: &Worker) -> Option<String> {
        let units: Vec<FileId> = self.baseline.iter().map(|b| b.unit).collect();
        let (syms, prog, m, r) = (&typer.syms, &typer.prog, &self.mark, &self.reach);
        let outside = |s: SymId| !made_sym(syms, s) && !units.contains(&syms.sym(s).file);
        let reached = |c: ClassId| r.classes.get(c.idx()).copied().unwrap_or(false);
        let asked: FxMap<SymId, ()> = r.asked.iter().map(|&s| (s, ())).collect();
        let trace = |what: String| {
            if std::env::var_os("TEQ_KEPT_TRACE").is_some() {
                eprintln!("kept reach: {}", what);
            }
            Some(what)
        };
        for i in m.syms..syms.syms.len() {
            let s = SymId(i as u32);
            let meets = match syms.sym(s).owner {
                Owner::Class(c) => reached(c),
                Owner::Package(_) => true,
                Owner::Local => false,
            };
            if outside(s) && meets {
                return trace(format!("the retypes made {} outside the units", typer.sym_path(s)));
            }
        }
        if let Some(i) = (m.classes..syms.classes.len()).find(|&i| !made_class(syms, ClassId(i as u32)) && !syms.class_of_files(ClassId(i as u32), &units)) {
            return trace(format!("the retypes entered {} outside the units", typer.class_path(ClassId(i as u32))));
        }
        // A body is met where the walk asked for it, and where it reaches the member's class: a
        // jar's member the walk links (`link_member`) is walked instead where a body is there,
        // one the interpreter's demand typed among them.
        let mut unmet = 0;
        for i in m.funs..prog.funs.len() {
            let s = prog.funs[i].sym;
            let met = asked.contains_key(&s) || matches!(syms.sym(s).owner, Owner::Class(c) if reached(c));
            if outside(s) && met {
                return trace(format!("the retypes typed the body of {}, which the kept walk meets", typer.sym_path(s)));
            }
            unmet += outside(s) as usize;
        }
        if unmet > 0 && std::env::var_os("TEQ_KEPT_TRACE").is_some() {
            eprintln!("kept reach: the retypes typed {} bodies outside the units that the kept walk does not meet", unmet);
        }
        for tc in prog.classes.iter() {
            let new = !m.records.get(tc.id.idx()).copied().unwrap_or(false);
            if new && !made_class(syms, tc.id) && !syms.class_of_files(tc.id, &units) && reached(tc.id) {
                return trace(format!("the retypes checked {}, which the kept walk reached", typer.class_path(tc.id)));
            }
        }
        if let Some((&t, _)) = accessor_targets(typer, &units).iter().find(|(t, _)| !m.accessor_targets.contains_key(t)) {
            return trace(format!("the retypes made a super accessor of {} outside the units", typer.sym_path(t)));
        }
        if let Some(&(s, _)) = prog.top_vals.iter().find(|&&(s, _)| outside(s) && !m.top_vals.contains_key(&s)) {
            return trace(format!("the retypes typed the top-level val {} outside the units", typer.sym_path(s)));
        }
        for (&(s, unit), &n) in template_calls(typer).iter() {
            if !units.contains(&unit) && m.template_calls.get(&(s, unit)).copied().unwrap_or(0) < n {
                return trace(format!("the retypes made a template call of {} outside the units", typer.sym_path(s)));
            }
        }
        for &s in typer.val_init.local.keys() {
            if !m.vals.contains_key(&s) && outside(s) && asked.contains_key(&s) && !r.vals.get(s.idx()).copied().unwrap_or(false) {
                return trace(format!("the retypes typed the initialiser of {}, which the kept walk asked for", typer.sym_path(s)));
            }
        }
        None
    }

    /// After a retype: the kept reach for the program as the typer holds it, the ids of the units
    /// retyped since it was made moved to the ones they have now, when each of those walks as it
    /// did then; or why the walk from the roots is needed.
    pub fn retyped(&mut self, typer: &Worker) -> Result<(), String> {
        if let Some(why) = self.added_outside(typer) {
            return Err(why);
        }
        let js_visible = self.setup.js_visible();
        let after: Vec<UnitWalk> = self.baseline.iter().map(|b| unit_walk(typer, b.unit, js_visible)).collect();
        let before = std::mem::take(&mut self.baseline);
        let kept = self.moved(typer, &before, &after);
        if kept.is_err() {
            self.baseline = before;
        }
        kept
    }

    fn moved(&mut self, typer: &Worker, before: &[UnitWalk], after: &[UnitWalk]) -> Result<(), String> {
        if !self.settled {
            return Err("the kept reach is not settled".to_string());
        }

        if Roots::of(typer) != self.roots {
            return Err("the entry point or the exports changed".to_string());
        }
        if !typer.renamed_roots.is_empty() || !typer.dispatch_pending.is_empty() {
            return Err("names of the output changed".to_string());
        }
        if before.len() != after.len() {
            return Err("the units retyped changed".to_string());
        }
        for (b, a) in before.iter().zip(after) {
            if std::env::var_os("TEQ_KEPT_TRACE").is_some() {
                if let Some(i) = (0..b.events.len().max(a.events.len())).find(|&i| b.events.get(i) != a.events.get(i)) {
                    eprintln!("kept reach: {} differs at event {}: {:?} before, {:?} after", typer.source(a.unit).path, i, &b.events[i.saturating_sub(3)..(i + 2).min(b.events.len())], &a.events[i.saturating_sub(3)..(i + 2).min(a.events.len())]);
                }
            }
            if b.unit != a.unit || b.events != a.events || b.syms.len() != a.syms.len() || b.classes.len() != a.classes.len() || b.funs.len() != a.funs.len() || b.roots.len() != a.roots.len() {
                return Err(format!("{}: what its bodies reach changed", typer.source(a.unit).path));
            }
        }
        let r = &mut self.reach;
        typer.extend_abstract_members(&mut r.abstract_syms);
        r.cover(typer.syms.classes.len(), typer.syms.syms.len());
        if r.funs.len() < typer.prog.funs.len() {
            r.funs.resize(typer.prog.funs.len(), false);
        }
        if r.imports.len() < typer.prog.js_imports.len() {
            r.imports.resize(typer.prog.js_imports.len(), false);
        }
        let mut funs: FxMap<FunId, FunId> = FxMap::default();
        let mut classes: FxMap<ClassId, ClassId> = FxMap::default();
        let mut roots: FxMap<TExprId, TExprId> = FxMap::default();
        for (b, a) in before.iter().zip(after) {
            for (&o, &n) in b.funs.iter().zip(&a.funs) {
                funs.insert(o, n);
                let marked = std::mem::replace(&mut r.funs[o.idx()], false);
                r.funs[n.idx()] |= marked;
            }
            for (&o, &n) in b.classes.iter().zip(&a.classes) {
                classes.insert(o, n);
                for v in [&mut r.classes, &mut r.trigger_classes] {
                    let marked = std::mem::replace(&mut v[o.idx()], false);
                    v[n.idx()] |= marked;
                }
            }
            for (&o, &n) in b.syms.iter().zip(&a.syms) {
                for v in [&mut r.bridged, &mut r.vals, &mut r.triggers, &mut r.forwarded, &mut r.fields_read, &mut r.init_read, &mut r.declared] {
                    let marked = std::mem::replace(&mut v[o.idx()], false);
                    v[n.idx()] |= marked;
                }
            }
            roots.extend(b.roots.iter().copied().zip(a.roots.iter().copied()));
        }
        for (e, place) in &mut r.roots {
            if let Some(&n) = roots.get(e) {
                *e = n;
            }
            match place {
                Place::Fun(f) => *f = funs.get(f).copied().unwrap_or(*f),
                Place::Class(c) => *c = classes.get(c).copied().unwrap_or(*c),
                Place::Val => {}
            }
        }
        Ok(())
    }

    /// The differences between the kept reach and the walk from the roots over the typer as it
    /// stands, which changes nothing of it (`reach::check_kept`): the assertion-enabled build's
    /// check of a reused reach.
    #[cfg(debug_assertions)]
    pub fn check(&self, typer: &Worker) -> Vec<String> {
        check_kept(typer, &self.reach, &self.setup)
    }
}
