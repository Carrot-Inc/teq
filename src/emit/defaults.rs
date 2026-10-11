//! The JavaScript lowering of an application's plan (`typer::elaborate`), whose default getters'
//! applications (`TExpr::Default`) and bound temporaries (`mods::LIFTED`) every target executes
//! as given: here written as compactly as the plan's order of evaluation allows. A getter's
//! application the callee's JavaScript parameter default computes at its entry (`f(a, b = ..)`)
//! is left out of the call, `undefined` where an argument follows, when nothing the call
//! evaluates after it in the plan has an effect at another time then; a temporary used once,
//! directly, in the call is written where it is used when the effects keep the plan's order.
//! A getter's application that stays is a call of the getter function (`Emitter::emit_default`).
//! The decision is a function of the tree, which the reach and the emission both make.

use crate::ast::{mods, ListRef};
use crate::intern::Interner;
use crate::symbols::*;
use crate::tir::*;
use crate::types::*;
use std::fmt::Write;

pub(crate) struct Defaults<'a> {
    pub prog: &'a Program,
    pub syms: &'a Symbols,
    pub types: &'a TypeStore,
    pub interner: &'a Interner,
}

/// What the call of a plan writes.
#[derive(Clone, Default, Debug)]
pub(crate) struct Lowering {
    /// The temporaries written where they are used, each with its value.
    pub sunk: Vec<(SymId, TExprId)>,
    /// The temporaries bound to a getter's application the call leaves out, which nothing writes.
    pub dropped: Vec<SymId>,
    /// Per argument of the call: left out for its parameter's default to compute.
    pub omitted: Vec<bool>,
}

impl Lowering {
    pub fn omits(&self, k: usize) -> bool {
        self.omitted.get(k).copied().unwrap_or(false)
    }

    pub fn moves(&self, s: SymId) -> bool {
        self.sunk.iter().any(|&(t, _)| t == s) || self.dropped.contains(&s)
    }
}

/// A call whose arguments may leave a parameter to its default.
pub(crate) struct Call {
    pub of: DefaultOf,
    pub recv: Option<TExprId>,
    pub args: ListRef,
    /// The leading arguments that are no parameter's: what a local class captures.
    pub skip: usize,
}

impl<'a> Defaults<'a> {
    /// The callee of a call node, the getters of whose defaults its parameters' JavaScript
    /// defaults compute; an anonymous class passes its superclass's arguments on.
    pub fn call(&self, e: TExprId) -> Option<Call> {
        match self.prog.expr(e) {
            TExpr::CallStatic(s, args) => Some(Call { of: DefaultOf::Method(s), recv: None, args, skip: 0 }),
            TExpr::CallMethod(r, s, args) => Some(Call { of: DefaultOf::Method(s), recv: Some(r), args, skip: 0 }),
            TExpr::NewVia(s, args) => Some(Call { of: DefaultOf::Method(s), recv: None, args, skip: 0 }),
            TExpr::New(c, args) => {
                let info = self.syms.class(c);
                let target = match (info.kind, info.superclass) {
                    (ClassKind::Anon, Some(sup)) => sup,
                    _ => c,
                };
                let declared: usize = self.syms.class(target).ctor.iter().map(|cl| cl.params.len()).sum();
                let skip = (args.len as usize).saturating_sub(declared);
                Some(Call { of: DefaultOf::Ctor(target), recv: None, args, skip })
            }
            _ => None,
        }
    }

    /// The parameter `i`, counted across the clauses, of what `of` names.
    pub fn param(&self, of: DefaultOf, i: usize) -> Option<ParamSig> {
        match of {
            DefaultOf::Method(s) => self.syms.sym(s).sig.as_ref()?.clauses.iter().flat_map(|c| c.params.iter()).nth(i).cloned(),
            DefaultOf::Ctor(c) => self.syms.class(c).ctor.iter().flat_map(|cl| cl.params.iter()).nth(i).cloned(),
        }
    }

    /// Whether the JavaScript function of `of` computes parameter `i`'s default where its
    /// argument is `undefined`: the parameter has a default, its own or, an override's, one a
    /// member it overrides declares (`js_declarer`), and no value of its type is `undefined`,
    /// which an argument passed as `()` would be.
    pub fn initializer(&self, of: DefaultOf, i: usize) -> bool {
        let Some(p) = self.param(of, i) else { return false };
        if p.repeated || !(p.by_name || self.undefined_free(p.ty)) {
            return false;
        }
        match of {
            DefaultOf::Ctor(_) => p.has_default,
            DefaultOf::Method(s) => p.has_default || self.js_declarer(s, i).is_some(),
        }
    }

    /// The method whose default of parameter `i` a call of `m` in JavaScript takes: `m` where its
    /// parameter has one, else the nearest member of its class's ancestors that the JavaScript
    /// output names alike (JavaScript dispatches by name) and that declares one.
    pub fn js_declarer(&self, m: SymId, i: usize) -> Option<SymId> {
        let declares = |s: SymId| self.syms.sym(s).sig.as_ref().and_then(|sig| sig.clauses.iter().flat_map(|c| c.params.iter()).nth(i).map(|p| p.has_default)).unwrap_or(false);
        if declares(m) {
            return Some(m);
        }
        let info = self.syms.sym(m);
        let Owner::Class(c) = info.owner else { return None };
        if info.kind != SymKind::Def || info.is_extension || info.name == crate::names::INIT {
            return None;
        }
        let name = self.syms.dispatch_name(m);
        for &(b, _) in self.syms.class(c).base_types.iter().skip(1) {
            let Some(&found) = self.syms.class(b).members.get(&info.name) else { continue };
            let alts: Vec<SymId> = match self.syms.alternatives(found) {
                Some(set) => set.to_vec(),
                None => vec![found],
            };
            for p in alts {
                if self.syms.sym(p).kind == SymKind::Def && self.syms.dispatch_name(p) == name && declares(p) {
                    return Some(p);
                }
            }
        }
        None
    }

    /// Whether no value of type `t` is JavaScript's `undefined`, which a parameter default of the
    /// JavaScript function takes for an omitted argument: a Scala class's, a primitive's, a
    /// string's; not `Unit`, whose value is `undefined`, nor `Any`, `AnyRef`, `AnyVal`, a type
    /// parameter or a JavaScript type, which can hold it.
    pub fn undefined_free(&self, t: TypeId) -> bool {
        let mut t = t;
        for _ in 0..8 {
            match self.types.get(t) {
                Type::Class(c, _) => {
                    let info = self.syms.class(c);
                    if info.js != JsKind::Scala {
                        return false;
                    }
                    if info.kind == ClassKind::Opaque {
                        match info.underlying {
                            Some(u) => {
                                t = u;
                                continue;
                            }
                            None => return false,
                        }
                    }
                    let name = self.interner.get(info.name);
                    let top = matches!(name, "Unit" | "Any" | "AnyVal" | "AnyRef" | "Matchable" | "Nothing" | "Null" | "Singleton")
                        || (name == "Object" && self.in_java_lang(c));
                    return !top;
                }
                Type::Lit(_) => return true,
                _ => return false,
            }
        }
        false
    }

    fn in_java_lang(&self, c: ClassId) -> bool {
        match self.syms.class(c).owner {
            Owner::Package(p) => self.interner.get(self.syms.pkg(p).name) == "lang",
            _ => false,
        }
    }

    /// Whether evaluating `e` can be told from nothing: a literal, `this`, a val's or a
    /// parameter's reference, a lambda, a class literal.
    pub fn effect_free(&self, e: TExprId) -> bool {
        match self.prog.expr(e) {
            TExpr::Int(_) | TExpr::Long(_) | TExpr::Double(_) | TExpr::Bool(_) | TExpr::Char(_) | TExpr::Str(_) | TExpr::Unit | TExpr::Null | TExpr::This => true,
            TExpr::Lambda(..) | TExpr::ClassOf(_) => true,
            TExpr::Local(s) => {
                let info = self.syms.sym(s);
                matches!(info.kind, SymKind::Val | SymKind::Param) && info.mods & (mods::MUTABLE | mods::LAZY) == 0
            }
            // An object without initialisation, whose first reference runs nothing.
            TExpr::Module(c) => crate::typer::elaborate::no_inits(self.syms, c),
            // A val of `this`.
            TExpr::Field(r, s) => {
                let info = self.syms.sym(s);
                matches!(self.prog.expr(r), TExpr::This) && info.kind == SymKind::Val && info.mods & (mods::MUTABLE | mods::LAZY) == 0 && !info.needs_accessor
            }
            _ => false,
        }
    }

    /// Whether `a`, a getter's receiver or argument, is the call's `b` evaluated once: the same
    /// local, `this`, object or literal.
    fn same_value(&self, a: TExprId, b: TExprId) -> bool {
        match (self.prog.expr(a), self.prog.expr(b)) {
            (TExpr::Local(x), TExpr::Local(y)) | (TExpr::Static(x), TExpr::Static(y)) => x == y,
            (TExpr::This, TExpr::This) => true,
            (TExpr::Module(x), TExpr::Module(y)) => x == y,
            (TExpr::Int(x), TExpr::Int(y)) => x == y,
            (TExpr::Long(x), TExpr::Long(y)) => x == y,
            (TExpr::Bool(x), TExpr::Bool(y)) => x == y,
            (TExpr::Char(x), TExpr::Char(y)) => x == y,
            (TExpr::Str(x), TExpr::Str(y)) => self.prog.strings[x.idx()] == self.prog.strings[y.idx()],
            (TExpr::Unit, TExpr::Unit) | (TExpr::Null, TExpr::Null) => true,
            _ => false,
        }
    }

    /// Whether `d` is a getter's application the call `c` leaves to the callee at argument `k`:
    /// of the callee's parameter `k`, on the call's receiver and arguments, which the callee's
    /// JavaScript parameter default computes.
    fn leaves_to_callee(&self, c: &Call, k: usize, d: TExprId) -> bool {
        let TExpr::Default(of, at, l) = self.prog.expr(d) else { return false };
        let index = (at & !DEFAULT_RECEIVER) as usize;
        if of != c.of || k < c.skip || index != k - c.skip || !self.initializer(c.of, index) {
            return false;
        }
        let items = self.prog.expr_list(l);
        let (recv, priors) = if at & DEFAULT_RECEIVER != 0 { (items.first().copied(), &items[1..]) } else { (None, items) };
        match (recv, c.recv) {
            (Some(a), Some(b)) if self.same_value(a, b) => {}
            (None, None) => {}
            _ => return false,
        }
        let args = self.prog.expr_list(c.args);
        priors.len() <= index && priors.iter().enumerate().all(|(j, &p)| args.get(c.skip + j).is_some_and(|&a| self.same_value(p, a)))
    }

    /// The lowering of the call `call`, `stmts` the statements of the plan's block around it (none
    /// where it stands alone): see the module's documentation.
    pub fn lower(&self, stmts: &[TStmt], call: TExprId) -> Option<Lowering> {
        let c = self.call(call)?;
        self.lower_call(stmts, c)
    }

    /// `lower` of the call `c`.
    pub fn lower_call(&self, stmts: &[TStmt], c: Call) -> Option<Lowering> {
        let args = self.prog.expr_list(c.args).to_vec();
        if !args.iter().any(|&a| matches!(self.prog.expr(a), TExpr::Default(..) | TExpr::Local(_))) && stmts.is_empty() {
            return None;
        }
        // The plan's temporaries, in order; a block of anything else is left as it is.
        let mut temps: Vec<(SymId, TExprId)> = Vec::with_capacity(stmts.len());
        for s in stmts {
            match *s {
                TStmt::Val(t, e) if self.syms.sym(t).mods & mods::LIFTED != 0 => temps.push((t, e)),
                _ => return self.lower_call(&[], c),
            }
        }
        let temp_at = |s: SymId| temps.iter().position(|&(t, _)| t == s);
        // Uses of each temporary: everywhere, and the direct ones, at the receiver (position 0)
        // or an argument (`k + 1`).
        let mut uses = vec![0usize; temps.len()];
        let mut direct: Vec<Option<usize>> = vec![None; temps.len()];
        let count = |root: TExprId, uses: &mut Vec<usize>| {
            for e in self.prog.descendants(root) {
                if let TExpr::Local(s) = self.prog.expr(e) {
                    if let Some(j) = temp_at(s) {
                        uses[j] += 1;
                    }
                }
            }
        };
        for &(_, e) in &temps {
            count(e, &mut uses);
        }
        if let Some(r) = c.recv {
            count(r, &mut uses);
            if let TExpr::Local(s) = self.prog.expr(r) {
                if let Some(j) = temp_at(s) {
                    direct[j] = Some(0);
                }
            }
        }
        for (k, &a) in args.iter().enumerate() {
            count(a, &mut uses);
            if let TExpr::Local(s) = self.prog.expr(a) {
                if let Some(j) = temp_at(s) {
                    direct[j] = Some(k + 1);
                }
            }
        }
        // The getters' applications the call may leave out: in place, or a temporary's value used
        // once, as that argument (once the omitted ones took their uses with them).
        let omittable = |k: usize, uses: &[usize]| match self.prog.expr(args[k]) {
            TExpr::Default(..) => self.leaves_to_callee(&c, k, args[k]),
            TExpr::Local(s) => temp_at(s).is_some_and(|j| uses[j] == 1 && direct[j] == Some(k + 1) && self.leaves_to_callee(&c, k, temps[j].1)),
            _ => false,
        };
        // The effects in the plan's order: the temporaries', then the receiver's and the
        // arguments' where they stand.
        #[derive(Clone, Copy, PartialEq)]
        enum Item {
            Temp(usize),
            Recv,
            Arg(usize),
        }
        let mut effects: Vec<Item> = Vec::new();
        for (j, &(_, e)) in temps.iter().enumerate() {
            if !self.effect_free(e) {
                effects.push(Item::Temp(j));
            }
        }
        if let Some(r) = c.recv {
            if !matches!(self.prog.expr(r), TExpr::Local(s) if temp_at(s).is_some()) && !self.effect_free(r) {
                effects.push(Item::Recv);
            }
        }
        for (k, &a) in args.iter().enumerate() {
            if !matches!(self.prog.expr(a), TExpr::Local(s) if temp_at(s).is_some()) && !self.effect_free(a) {
                effects.push(Item::Arg(k));
            }
        }
        // The last effects, getters' applications of increasing parameters, are left to the
        // callee, which computes them after every argument in that order.
        let mut omitted = vec![false; args.len()];
        let mut dropped: Vec<SymId> = Vec::new();
        let mut next_param = usize::MAX;
        // An object's reference as the receiver is idempotent: a getter's application on it,
        // evaluated before it, evaluated it first, so the callee may compute that getter after it;
        // it stays an effect for the temporaries written into the call below.
        let object_recv = c.recv.is_some_and(|r| matches!(self.prog.expr(r), TExpr::Module(_)));
        let mut recv_passed = None;
        while let Some(&item) = effects.last() {
            if item == Item::Recv && object_recv && recv_passed.is_none() {
                recv_passed = effects.pop();
                continue;
            }
            let k = match item {
                Item::Arg(k) if k < next_param && omittable(k, &uses) => k,
                Item::Temp(j) => match direct[j] {
                    Some(p) if p > 0 && p - 1 < next_param && matches!(self.prog.expr(args[p - 1]), TExpr::Local(_)) && omittable(p - 1, &uses) => p - 1,
                    _ => break,
                },
                _ => break,
            };
            omitted[k] = true;
            if let Item::Temp(j) = item {
                dropped.push(temps[j].0);
            }
            next_param = k;
            effects.pop();
            // The uses the omitted getter's application takes with it.
            let a = args[k];
            let tree = match self.prog.expr(a) {
                TExpr::Local(s) => temp_at(s).map(|j| temps[j].1),
                _ => Some(a),
            };
            if let Some(tree) = tree {
                for e in self.prog.descendants(tree) {
                    if let TExpr::Local(s) = self.prog.expr(e) {
                        if let Some(j) = temp_at(s) {
                            uses[j] = uses[j].saturating_sub(1);
                        }
                    }
                }
            }
            if let TExpr::Local(s) = self.prog.expr(a) {
                if let Some(j) = temp_at(s) {
                    uses[j] = 0;
                }
            }
        }
        effects.extend(recv_passed);
        // Then the temporaries written where they are used, from the last effect backwards, while
        // each one stands before the effects after it among the arguments.
        let mut sunk: Vec<(SymId, TExprId)> = Vec::new();
        let mut first = usize::MAX;
        for &item in effects.iter().rev() {
            match item {
                Item::Recv => first = first.min(0),
                Item::Arg(k) => first = first.min(k + 1),
                Item::Temp(j) => match direct[j] {
                    Some(p) if uses[j] == 1 && p < first => {
                        sunk.push(temps[j]);
                        first = p;
                    }
                    _ => break,
                },
            }
        }
        // A temporary without an effect moves wherever its one use is.
        for (j, &(t, e)) in temps.iter().enumerate() {
            if self.effect_free(e) && uses[j] == 1 && direct[j].is_some() && !sunk.iter().any(|&(s, _)| s == t) && !dropped.contains(&t) {
                sunk.push((t, e));
            }
        }
        if sunk.is_empty() && dropped.is_empty() && !omitted.iter().any(|&o| o) {
            return None;
        }
        Some(Lowering { sunk, dropped, omitted })
    }
}

impl<'a> super::Emitter<'a> {
    pub(super) fn defaults(&self) -> Defaults<'a> {
        Defaults { prog: self.prog, syms: self.syms, types: self.types, interner: self.interner }
    }

    /// The call of a plan's block `e` and what it writes (`Defaults::lower`): a block of the
    /// plan's temporaries in front of a call.
    pub(super) fn block_lowering(&self, e: TExprId) -> Option<(TExprId, Lowering)> {
        let TExpr::Block(stmts, res) = self.prog.expr(e) else { return None };
        let items = &self.prog.stmts[stmts.range()];
        if items.is_empty() || !items.iter().all(|s| matches!(*s, TStmt::Val(t, _) if self.syms.sym(t).mods & mods::LIFTED != 0)) {
            return None;
        }
        let lowering = self.defaults().lower(items, res)?;
        Some((res, lowering))
    }

    /// Whether every statement of the plan's block `e` is a temporary its call writes elsewhere
    /// or not at all, which leaves the call alone.
    pub(super) fn block_moves_all(&self, e: TExprId) -> bool {
        let TExpr::Block(stmts, _) = self.prog.expr(e) else { return false };
        match self.block_lowering(e) {
            Some((_, l)) => self.prog.stmts[stmts.range()].iter().all(|s| matches!(*s, TStmt::Val(t, _) if l.moves(t))),
            None => false,
        }
    }

    /// While the call of `lowering` is written: its moved temporaries and the arguments it omits.
    pub(super) fn install_lowering(&mut self, call: TExprId, lowering: Lowering) {
        for (t, v) in lowering.sunk {
            self.sunk.insert(t, v);
        }
        for t in lowering.dropped {
            self.dropped.insert(t, ());
        }
        self.lowered.insert(call, lowering.omitted);
    }

    pub(super) fn uninstall_lowering(&mut self, call: TExprId, block: TExprId) {
        self.lowered.remove(&call);
        if let TExpr::Block(stmts, _) = self.prog.expr(block) {
            for s in &self.prog.stmts[stmts.range()] {
                if let TStmt::Val(t, _) = *s {
                    self.sunk.remove(&t);
                    self.dropped.remove(&t);
                }
            }
        }
    }

    /// The arguments of the call `e` left to the callee's defaults.
    pub(super) fn omitted_args(&mut self, e: TExprId) -> Vec<bool> {
        if let Some(o) = self.lowered.remove(&e) {
            return o;
        }
        match self.defaults().lower(&[], e) {
            Some(l) => l.omitted,
            None => Vec::new(),
        }
    }

    /// The arguments a parent's constructor `c` is called with, a getter's application its
    /// JavaScript default computes left out.
    pub(super) fn emit_parent_args(&mut self, c: ClassId, args: ListRef) {
        let omitted = match self.defaults().lower_call(&[], Call { of: DefaultOf::Ctor(c), recv: None, args, skip: 0 }) {
            Some(l) => l.omitted,
            None => Vec::new(),
        };
        self.emit_args_omitting(args, &omitted);
    }

    /// The arguments of a call, those left to the callee's defaults written as `undefined`, or not
    /// at all after the last one written.
    pub(super) fn emit_args_omitting(&mut self, args: ListRef, omitted: &[bool]) {
        let items = self.prog.expr_list(args);
        let mut keep = items.len();
        while keep > 0 {
            match self.peek(items[keep - 1]) {
                TExpr::Spread(inner) if self.seq_literal_items(inner).map_or(false, |l| l.is_empty()) => keep -= 1,
                TExpr::Default(..) if omitted.get(keep - 1) == Some(&true) => keep -= 1,
                TExpr::Local(_) if omitted.get(keep - 1) == Some(&true) => keep -= 1,
                _ => break,
            }
        }
        self.out.push('(');
        let mut separator = "";
        for (k, &a) in items[..keep].iter().enumerate() {
            if omitted.get(k) == Some(&true) {
                self.out.push_str(separator);
                separator = ", ";
                self.out.push_str("undefined");
                continue;
            }
            let inlined = match self.peek(a) {
                TExpr::Spread(inner) => self.seq_literal_items(inner),
                _ => None,
            };
            for &item in inlined.unwrap_or(std::slice::from_ref(&a)) {
                self.out.push_str(separator);
                separator = ", ";
                self.emit_value(item);
            }
        }
        self.out.push(')');
    }
}

impl<'a> super::Emitter<'a> {
    /// The captures of the local class `c` its constructor takes before its parameters, which
    /// its getters take too.
    pub(super) fn ctor_captures(&self, c: ClassId) -> Vec<SymId> {
        match self.local_captures.get(&c) {
            Some(&n) => self.prog.classes.iter().find(|tc| tc.id == c).map_or(Vec::new(), |tc| tc.ctor_params[..n as usize].to_vec()),
            None => Vec::new(),
        }
    }

    /// The rest of a getter's `.call(this, ..)` on a `super` target: its arguments and the `)`.
    fn emit_super_priors(&mut self, priors: &[TExprId]) {
        for &p in priors {
            self.out.push_str(", ");
            self.emit_value(p);
        }
        self.out.push(')');
    }

    /// The name of the getter of parameter `i` of the method `m`, `m$default$N`: a member of its
    /// class, where an override names its own alike.
    pub(super) fn getter_name(&mut self, m: SymId, i: usize) -> String {
        format!("{}$default${}", self.sym_name(m), i + 1)
    }

    /// A getter's application the call does not leave to the callee (`TExpr::Default`): the
    /// getter called on the receiver, a def's getter (`f.$default$N`, a property of its
    /// function), a constructor's (`C.$default$N`, static); a JavaScript function's default is
    /// its own, nothing passed.
    pub(super) fn emit_default(&mut self, e: TExprId, of: DefaultOf, at: u32, l: ListRef) {
        let index = (at & !DEFAULT_RECEIVER) as usize;
        let items = self.prog.expr_list(l).to_vec();
        let (recv, priors) = if at & DEFAULT_RECEIVER != 0 { (items.first().copied(), items[1..].to_vec()) } else { (None, items) };
        let mut init = false;
        match of {
            DefaultOf::Method(s) if self.syms.js_member(s) || self.syms.sym(s).js_import.is_some() || self.syms.sym(s).js_global.is_some() => {
                self.out.push_str("undefined");
                return;
            }
            DefaultOf::Method(s) if self.syms.sym(s).name == crate::names::INIT => {
                let Owner::Class(c) = self.syms.sym(s).owner else {
                    self.out.push_str("undefined");
                    return;
                };
                let cn = self.class_name(c);
                let _ = write!(self.out, "{}.$default${}", cn, index + 1);
            }
            // A getter selected on `super` is the one the superclass or the trait has, as
            // `Applications.findDefaultGetter` selects it on the receiver: called as a `super`
            // call is.
            DefaultOf::Method(s) => match recv {
                Some(r) if matches!(self.prog.expr(r), TExpr::Super(_)) => {
                    let TExpr::Super(target) = self.prog.expr(r) else { unreachable!() };
                    let g = self.getter_name(s, index);
                    match target {
                        SuperTarget::Chain if self.this_name == "this" => {
                            let _ = write!(self.out, "super.{}", g);
                        }
                        SuperTarget::Chain => {
                            let _ = write!(self.out, "super.{}.call({}", g, self.this_name);
                            self.emit_super_priors(&priors);
                            return;
                        }
                        SuperTarget::Class(c) => {
                            let class = self.class_name(c);
                            let _ = write!(self.out, "{}.prototype.{}.call({}", class, g, self.this_name);
                            self.emit_super_priors(&priors);
                            return;
                        }
                        // Inside a trait: the accessor the class mixing it in defines.
                        SuperTarget::Mixin(tr) => {
                            let accessor = self.super_getter_accessor_name(tr, s, index);
                            let _ = write!(self.out, "{}.{}", self.this_name, accessor);
                        }
                    }
                }
                Some(r) => {
                    self.emit_receiver(r);
                    let g = self.getter_name(s, index);
                    let _ = write!(self.out, ".{}", g);
                }
                None => {
                    init = self.open_init(e);
                    let f = self.local_ref(s);
                    let _ = write!(self.out, "{}.$default${}", f, index + 1);
                }
            },
            DefaultOf::Ctor(c) => {
                let cn = self.class_name(c);
                let _ = write!(self.out, "{}.$default${}", cn, index + 1);
            }
        }
        self.out.push('(');
        // A local class's getter takes its captures first, as its constructor does.
        let captured = match of {
            DefaultOf::Ctor(c) => self.ctor_captures(c),
            DefaultOf::Method(_) => Vec::new(),
        };
        for (k, &s) in captured.iter().enumerate() {
            if k > 0 {
                self.out.push_str(", ");
            }
            self.emit_capture_ref(s);
        }
        for (k, &p) in priors.iter().enumerate() {
            if k > 0 || !captured.is_empty() {
                self.out.push_str(", ");
            }
            self.emit_value(p);
        }
        self.out.push(')');
        if init {
            self.out.push(')');
        }
    }

    /// The parameters before the clause of parameter `i` of `params`, `clauses` their clauses:
    /// what a getter takes.
    fn getter_params(clauses: &[ClauseSig], params: &[SymId], skip: usize, i: usize) -> Vec<SymId> {
        let before = crate::typer::elaborate::clause_start(&MethodSig { tparams: Vec::new(), clauses: clauses.to_vec(), ret: ANY }, i);
        params[skip..(skip + before).min(params.len())].to_vec()
    }

    /// The parameter list and body of a getter after its name: the default as the getter's value,
    /// a by-name parameter's evaluated, in the scope `root` of the function it belongs to.
    /// A local class's getter takes its captures first, `$c0, $c1, ...`, which its default reads
    /// as its constructor's parameters (`captures`).
    fn emit_getter_rest(&mut self, root: super::scope::Root, params: &[SymId], captures: usize, d: TExprId, by_name: bool, arrow: bool) {
        self.enter_scope(root);
        let mark = self.emit_params_after_captures(captures, params, &[]);
        let before_super = self.before_super;
        self.before_super = before_super || captures > 0;
        self.out.push_str(if arrow { " => " } else { " " });
        let outer_depth = std::mem::replace(&mut self.fn_depth, 0);
        self.open("{");
        self.line();
        self.out.push_str("return ");
        match self.prog.expr(d) {
            TExpr::Lambda(ps, body) if by_name && ps.len == 0 => self.emit_value(body),
            _ if by_name => {
                self.emit_expr(d);
                self.out.push_str("()");
            }
            _ => self.emit_value(d),
        }
        self.out.push(';');
        self.close("}");
        self.before_super = before_super;
        self.fn_depth = outer_depth;
        self.leave_params(mark);
        self.leave_scope();
    }

    /// The getters of a method's defaults its class's JavaScript output calls, as methods of the
    /// class (`Reach::getter_names`). A jar class's getter is a member of its pickle too, written
    /// as a method where a body of the jar calls it: one getter of the name, as
    /// `Desugar.addDefaultGetters` makes one (`members`, the class's methods).
    pub(super) fn emit_method_getters(&mut self, f: FunId, members: &[FunId]) {
        let fun = &self.prog.funs[f.idx()];
        let sym = fun.sym;
        let name = self.syms.sym(sym).name;
        let Some(sig) = self.syms.sym(sym).sig.clone() else { return };
        for i in 0..fun.defaults.len() {
            let Some(d) = fun.defaults[i] else { continue };
            if !self.reach.getter_names.contains_key(&(name, i as u32)) {
                continue;
            }
            let by_name = sig.clauses.iter().flat_map(|c| c.params.iter()).nth(i).is_some_and(|p| p.by_name);
            let params = Self::getter_params(&sig.clauses, &fun.params, 0, i);
            let g = self.getter_name(sym, i);
            if members.iter().any(|&m| m != f && self.reach.funs[m.idx()] && *self.sym_name(self.prog.funs[m.idx()].sym) == *g) {
                continue;
            }
            self.line();
            self.out.push_str(&g);
            self.emit_getter_rest(super::scope::Root::Fun(f), &params, 0, d, by_name, false);
        }
    }

    /// The getters of a def's defaults the output calls, properties of its function
    /// (`f.$default$N`), which its binding carries to every module: after a top-level def's
    /// declaration or a local one's.
    pub(super) fn emit_fun_getters(&mut self, f: FunId) {
        let fun = &self.prog.funs[f.idx()];
        let sym = fun.sym;
        let Some(sig) = self.syms.sym(sym).sig.clone() else { return };
        // An inline method's getters are functions of their own (`Desugar.addDefaultGetters`);
        // a top-level one's hang on a function of its name, the method having none, a local
        // one's on an object of its name.
        let reached = (0..fun.defaults.len()).any(|i| fun.defaults[i].is_some() && self.reach.getter_funs.contains_key(&(sym, i as u32)));
        if fun.body.is_none() && reached {
            match self.syms.sym(sym).owner {
                Owner::Package(_) => {
                    self.line();
                    self.bindings.defs.push(super::Def::Sym(sym));
                    let n = self.sym_name(sym);
                    let _ = write!(self.out, "function {}() {{}}", n);
                }
                Owner::Local => {
                    self.line();
                    let n = self.sym_name(sym);
                    let _ = write!(self.out, "const {} = {{}};", n);
                }
                Owner::Class(_) => {}
            }
        }
        for i in 0..fun.defaults.len() {
            let Some(d) = fun.defaults[i] else { continue };
            if !self.reach.getter_funs.contains_key(&(sym, i as u32)) {
                continue;
            }
            let by_name = sig.clauses.iter().flat_map(|c| c.params.iter()).nth(i).is_some_and(|p| p.by_name);
            let params = Self::getter_params(&sig.clauses, &fun.params, 0, i);
            self.line();
            let n = self.sym_name(sym);
            let _ = write!(self.out, "{}.$default${} = ", n, i + 1);
            self.emit_getter_rest(super::scope::Root::Fun(f), &params, 0, d, by_name, true);
            self.out.push(';');
        }
    }

    /// The getters of a constructor's defaults the output calls, static methods of its class
    /// (`C.$default$N`): the primary constructor's, and a secondary one's.
    pub(super) fn emit_ctor_getters(&mut self, idx: usize) {
        let tc = &self.prog.classes[idx];
        let c = tc.id;
        let clauses = self.syms.class(c).ctor.clone();
        let captures = tc.ctor_params.len().saturating_sub(clauses.iter().map(|cl| cl.params.len()).sum());
        for i in 0..tc.ctor_defaults.len().saturating_sub(captures) {
            let Some(d) = tc.ctor_defaults[captures + i] else { continue };
            if !self.reach.getter_ctors.contains_key(&(c, i as u32)) {
                continue;
            }
            let by_name = clauses.iter().flat_map(|cl| cl.params.iter()).nth(i).is_some_and(|p| p.by_name);
            // A local class's captures first, as its constructor takes them.
            let mut params = tc.ctor_params[..captures].to_vec();
            params.extend(Self::getter_params(&clauses, &tc.ctor_params, captures, i));
            self.line();
            let _ = write!(self.out, "static $default${}", i + 1);
            self.emit_getter_rest(super::scope::Root::Ctor(idx as u32), &params, captures, d, by_name, false);
        }
        for &f in &tc.ctors {
            let fun = &self.prog.funs[f.idx()];
            let sym = fun.sym;
            let Some(sig) = self.syms.sym(sym).sig.clone() else { continue };
            for i in 0..fun.defaults.len() {
                let Some(d) = fun.defaults[i] else { continue };
                if !self.reach.getter_funs.contains_key(&(sym, i as u32)) {
                    continue;
                }
                let by_name = sig.clauses.iter().flat_map(|c| c.params.iter()).nth(i).is_some_and(|p| p.by_name);
                let params = Self::getter_params(&sig.clauses, &fun.params, 0, i);
                self.line();
                let _ = write!(self.out, "static $default${}", i + 1);
                self.emit_getter_rest(super::scope::Root::Fun(f), &params, 0, d, by_name, false);
            }
        }
    }
}
