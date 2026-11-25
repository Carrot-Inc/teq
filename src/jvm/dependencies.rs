//! The free variables dotty's `Dependencies` finds in a unit (compiler/src/dotty/tools/dotc/transform/
//! Dependencies.scala at 3.8.4), for the proxies `LambdaLift.generateProxies` names before it names a lifted method
//! (LambdaLift.scala 73-85, 224-227): one name per free variable of each method and class that has some, the
//! variable's name freshened, drawn from the counters the lifted methods' names draw from. The walk of the unit's
//! lifted names (`lifted.rs`) records where it is (each method and class around its code), what each of them declares,
//! the locals the code reads and what it calls; `proxies` then closes the free variables over the calls and counts
//! the names the proxies draw.

use super::lazy::sanitize;
use crate::intern::FxMap;
use crate::types::{ClassId, SymId};

/// A definition whose code `Dependencies` treats as one: a method or a class (`Symbol.enclosure`).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    /// The unit's package: what no method or class holds.
    Root,
    /// A class, `real` where it is no trait (`isRealClass`).
    Class { real: bool },
    /// A class's constructor, whose free variables are looked for from the class's owner (`markFree`'s
    /// `enclosure.owner.enclosure`).
    Ctor,
    Method,
}

/// What a call goes to (`Dependencies.called`).
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum Callee {
    /// A local def or a secondary constructor, by its symbol; a local lazy val's accessor, by the val's.
    Fun(SymId),
    /// A local lazy val's initialiser (`LazyVals.transformLocalDef`).
    LazyInit(SymId),
    /// The getter of the default `i` of a local def or of a local trait's member.
    Default(SymId, usize),
    /// A class's primary constructor.
    Ctor(ClassId),
    /// A local class, which its primary constructor calls (`Dependencies.process`, the `DefDef` case).
    Class(ClassId),
    /// A local trait's member with a body, which the class that mixes the trait in forwards to (`Mixin`).
    TraitMember(SymId),
    /// A local trait's initialiser, which the constructor of a class mixing the trait in calls (`Mixin`).
    TraitInit(ClassId),
    /// The accessor of the companion module holding a local class's constructor's default getters.
    Companion(ClassId),
    /// A method `HoistSuperArgs` made, by its number in the unit.
    SuperArg(u32),
}

struct Encl {
    parent: u32,
    kind: Kind,
    declared: Vec<SymId>,
}

/// What the walk of a unit recorded (`LiftWalk::deps`).
pub(super) struct Deps {
    encls: Vec<Encl>,
    cur: u32,
    stack: Vec<u32>,
    refs: Vec<(u32, SymId)>,
    calls: Vec<(u32, Callee)>,
    callees: FxMap<Callee, u32>,
    /// Calls between definitions with no symbol of their own (a companion the namer makes and its parts).
    links: Vec<(u32, u32)>,
    /// Each class's enclosure and its primary constructor's, by the class.
    pub(super) classes: FxMap<ClassId, (u32, u32)>,
    /// The companion module holding each local class's constructors' default getters, by the class: the module's
    /// class where it has one, and its enclosure.
    pub(super) companions: FxMap<ClassId, (Option<ClassId>, u32)>,
    /// The lazy val of the block that holds each module class (its right-hand side `new C$`), by the class.
    pub(super) module_vals: FxMap<ClassId, SymId>,
    super_args: u32,
    /// The references to a local that no definition around them declares.
    pub(super) unresolved: Vec<SymId>,
}

impl Deps {
    pub(super) fn new() -> Deps {
        Deps {
            encls: vec![Encl { parent: 0, kind: Kind::Root, declared: Vec::new() }],
            cur: 0,
            stack: Vec::new(),
            refs: Vec::new(),
            calls: Vec::new(),
            callees: FxMap::default(),
            links: Vec::new(),
            classes: FxMap::default(),
            companions: FxMap::default(),
            module_vals: FxMap::default(),
            super_args: 0,
            unresolved: Vec::new(),
        }
    }

    /// Enters a definition of the kind `kind` that the current one holds, `callee` where calls name it, declaring
    /// `declares` (its parameters).
    pub(super) fn push(&mut self, kind: Kind, callee: Option<Callee>, declares: &[SymId]) -> u32 {
        let id = self.encls.len() as u32;
        self.encls.push(Encl { parent: self.cur, kind, declared: declares.to_vec() });
        if let Some(c) = callee {
            self.callees.entry(c).or_insert(id);
        }
        self.reenter(id);
        id
    }

    /// Enters a definition `parent` holds, whatever the current one.
    pub(super) fn push_at(&mut self, parent: u32, kind: Kind, callee: Option<Callee>, declares: &[SymId]) -> u32 {
        let outer = self.cur;
        self.cur = parent;
        let id = self.push(kind, callee, declares);
        self.stack.pop();
        self.stack.push(outer);
        id
    }

    pub(super) fn parent_of(&self, e: u32) -> u32 {
        self.parent(e)
    }

    /// The nearest enclosure around the current one that is no class: where a local class's block stands.
    pub(super) fn block(&self) -> u32 {
        let mut e = self.cur;
        while matches!(self.kind(e), Kind::Class { .. }) {
            e = self.parent(e);
        }
        e
    }

    /// Enters a definition the current one's owner holds: a member's default getter, which stands beside the member.
    pub(super) fn push_beside(&mut self, kind: Kind, callee: Option<Callee>, declares: &[SymId]) -> u32 {
        let id = self.encls.len() as u32;
        self.encls.push(Encl { parent: self.parent(self.cur), kind, declared: declares.to_vec() });
        if let Some(c) = callee {
            self.callees.entry(c).or_insert(id);
        }
        self.reenter(id);
        id
    }

    pub(super) fn link(&mut self, caller: u32, callee: u32) {
        self.links.push((caller, callee));
    }

    /// Enters a definition entered before, whose code the walk meets apart from the rest.
    pub(super) fn reenter(&mut self, id: u32) {
        self.stack.push(self.cur);
        self.cur = id;
    }

    pub(super) fn pop(&mut self) {
        self.cur = self.stack.pop().unwrap_or(0);
    }

    pub(super) fn callee(&self, c: Callee) -> Option<u32> {
        self.callees.get(&c).copied()
    }

    pub(super) fn current(&self) -> u32 {
        self.cur
    }

    pub(super) fn declare(&mut self, s: SymId) {
        self.encls[self.cur as usize].declared.push(s);
    }

    pub(super) fn refer(&mut self, s: SymId) {
        self.refs.push((self.cur, s));
    }

    pub(super) fn call(&mut self, callee: Callee) {
        self.calls.push((self.cur, callee));
    }

    pub(super) fn call_from(&mut self, caller: u32, callee: Callee) {
        self.calls.push((caller, callee));
    }

    /// The number of the next method `HoistSuperArgs` makes.
    pub(super) fn next_super_arg(&mut self) -> u32 {
        self.super_args += 1;
        self.super_args
    }

    /// A call of a local class's constructor's default getter reads its companion module's val: the accessor of the
    /// val that holds it, else one made for it beside the module, with the initialiser it calls (`new C$`).
    fn close_companions(&mut self) {
        let mut companions: Vec<(ClassId, (Option<ClassId>, u32))> = self.companions.iter().map(|(&c, &v)| (c, v)).collect();
        companions.sort_by_key(|&(c, _)| c.0);
        for (c, (module, class)) in companions {
            let held = module.and_then(|k| self.module_vals.get(&k)).and_then(|&v| self.callees.get(&Callee::Fun(v)).copied());
            let accessor = match held {
                Some(a) => a,
                None => {
                    let parent = self.parent(class);
                    let ctor = module.and_then(|k| self.callees.get(&Callee::Ctor(k)).copied()).unwrap_or(class);
                    let init = self.add(parent, Kind::Method);
                    self.links.push((init, ctor));
                    let accessor = self.add(parent, Kind::Method);
                    self.links.push((accessor, init));
                    accessor
                }
            };
            self.callees.insert(Callee::Companion(c), accessor);
        }
    }

    fn add(&mut self, parent: u32, kind: Kind) -> u32 {
        self.encls.push(Encl { parent, kind, declared: Vec::new() });
        self.encls.len() as u32 - 1
    }

    fn kind(&self, e: u32) -> Kind {
        self.encls[e as usize].kind
    }

    fn parent(&self, e: u32) -> u32 {
        self.encls[e as usize].parent
    }

    fn declares_around(&self, mut e: u32, s: SymId) -> bool {
        loop {
            if self.encls[e as usize].declared.contains(&s) {
                return true;
            }
            if e == 0 {
                return false;
            }
            e = self.parent(e);
        }
    }

    /// `nestedInConstructor`: a constructor, or a method a constructor holds.
    fn nested_in_ctor(&self, e: u32) -> bool {
        match self.kind(e) {
            Kind::Ctor => true,
            Kind::Method => self.nested_in_ctor(self.parent(e)),
            _ => false,
        }
    }

    /// `Dependencies.markFree` (131-170): `s` free in `e` and in what lies between `e` and the definition declaring
    /// `s`, but where a class between holds it for them; the class that holds it, if any.
    fn mark_free(&self, free: &mut [Vec<SymId>], changed: &mut bool, s: SymId, e: u32) -> Option<u32> {
        if e == 0 || self.encls[e as usize].declared.contains(&s) {
            return None;
        }
        let up = match self.kind(e) {
            Kind::Ctor => self.parent(self.parent(e)),
            _ => self.parent(e),
        };
        let intermediate = self.mark_free(free, changed, s, up);
        let real = |x: u32| self.kind(x) == Kind::Class { real: true };
        let intermediate_real = intermediate.is_some_and(real);
        if (!intermediate_real || self.nested_in_ctor(e)) && self.kind(e) != (Kind::Class { real: false }) && !free[e as usize].contains(&s) {
            free[e as usize].push(s);
            *changed = true;
        }
        if intermediate_real {
            intermediate
        } else if real(e) {
            Some(e)
        } else if intermediate.is_some() {
            intermediate
        } else if matches!(self.kind(e), Kind::Class { .. }) {
            Some(e)
        } else {
            None
        }
    }

    /// The proxies' names the unit draws, by their counters' keys (`sanitize(name) + "$"`): each free variable of
    /// each method and class, the free variables closed over the calls (`computeFreeVars`, 260-272).
    pub(super) fn proxies(&mut self, name: impl Fn(SymId) -> String) -> FxMap<String, u32> {
        self.close_companions();
        let mut free: Vec<Vec<SymId>> = vec![Vec::new(); self.encls.len()];
        let mut changed = false;
        for i in 0..self.refs.len() {
            let (e, s) = self.refs[i];
            if self.declares_around(e, s) {
                self.mark_free(&mut free, &mut changed, s, e);
            } else {
                self.unresolved.push(s);
            }
        }
        let mut edges: Vec<(u32, u32)> = self.calls.iter().filter_map(|&(caller, c)| self.callees.get(&c).map(|&to| (caller, to))).collect();
        edges.extend_from_slice(&self.links);
        loop {
            changed = false;
            for &(caller, callee) in &edges {
                for i in 0..free[callee as usize].len() {
                    let s = free[callee as usize][i];
                    if self.declares_around(caller, s) {
                        self.mark_free(&mut free, &mut changed, s, caller);
                    }
                }
            }
            if !changed {
                break;
            }
        }
        let mut counts: FxMap<String, u32> = FxMap::default();
        for s in free.into_iter().flatten() {
            *counts.entry(format!("{}$", sanitize(&name(s)))).or_insert(0) += 1;
        }
        counts
    }
}
