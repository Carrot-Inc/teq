//! Local traits whose members read the values of their block, lifted as dotty's `LambdaLift`
//! lifts them (`transform/Dependencies.scala`, `transform/LambdaLift.scala`, the forwarders of
//! `transform/Mixin.scala`). A trait has no constructor to take what it captures, so a member of
//! it whose body reads such a value takes the values it reads as leading parameters, its proxies
//! (`TFun::captures`, `addFreeParams`), and the trait's initialiser takes those its statements
//! read (the trait's `TClass::captures`). Each class that mixes the trait in holds the values as
//! it holds its own captures (a `var` in its cell, shared), and implements the member by a
//! forwarder that passes them (`Mixin.mixinForwarders`, `addFreeArgs`), as its own `super` calls
//! of the member and its call of the initialiser pass them.
//!
//! A local trait, owned by a term or by such a trait (dotty's `Dependencies.isLocal`), is lifted
//! as dotty lifts it after `Mixin`: what a member needs is what its body reads, what the members
//! it calls need and what the classes it makes need (`markFree`, `markCalled`,
//! `computeFreeVars`), a class nested in the trait at any depth holding what its body needs; and
//! every call of the member passes them (`addFreeArgs`, `Worker::adapt_lifted_calls`), so that the
//! lifted method is the member's one slot, which a class's member of the source signature does
//! not override, and a lazy val's initialiser runs at each read. A `super` call inside a trait
//! goes through the accessor of the class that mixes the trait in (`ResolveSuper`), which passes
//! the class's own values when it binds to a lifted member.
//!
//! A trait a class of the block owns takes none: dotty's `markFree` makes a value free in the
//! largest real class between a read and the block, and a trait's members, as the classes nested
//! in it at any depth, read it through their enclosing instances (`LambdaLift.proxyRef`,
//! `memberRef`). That class holds the value as it holds its own captures, and its proxy member
//! stands for the value's read, a `var`'s write, a local def's call and a block class's creation
//! (`HeldOf`), each typed as the class's own body would type it; a reference typed across such a
//! trait goes through it (`held_holder`). A trait of a class or package is left as it is.

use super::{Frame, Worker};
use crate::ast::mods;
use crate::symbols::*;
use crate::tir::*;
use crate::types::*;
use std::sync::Arc;

/// What a proxy member of a class of a block stands for (`Worker::held_proxies`): the class holds
/// a value of the block, which dotty's `markFree` makes free in the largest real class between
/// a read and the block, for the traits of classes nested in it, whose members read it through
/// their enclosing instances (`LambdaLift.proxyRef`, `memberRef`).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum HeldOf {
    /// A value of the block (a `val`, a `var`, a parameter, a lazy val, a local object's val, a
    /// given), or the instance of a class around the block that the local `$this` stands for:
    /// its read.
    Get(SymId),
    /// A `var` of the block: its write.
    Set(SymId),
    /// A local def of the block: its call.
    Call(SymId),
    /// A class of the block: its creation, with what it captures passed where the class of the
    /// block makes it (dotty passes the proxies, a `var`'s cell among them).
    New(ClassId),
}

impl<'a> Worker<'a> {
    /// The class of a block that holds the value `s` of the block for a read from here that
    /// crosses a trait a class owns: dotty's `markFree`, whose storage is the largest real class
    /// between the read and the block, a trait taking none (its members reach the value through
    /// their enclosing instances). The outermost class that is no trait after the scope that
    /// binds `s`, where a trait not local in dotty's sense (`dotty_local_trait`) follows it on the
    /// way here; none where the read crosses no such trait, a class's own capture then serving.
    pub(super) fn held_holder(&self, s: SymId) -> Option<ClassId> {
        if !self.prog.has_block_class_traits() || self.quote.level > 0 {
            return None;
        }
        let at = self.env.frames.iter().rposition(|f| matches!(f, Frame::Locals { names, givens, .. } if names.iter().any(|&(_, x)| x == s) || givens.contains(&s)))?;
        self.held_holder_after(at, true)
    }

    /// `held_holder` for the creation of the class `k` of the block.
    pub(super) fn held_holder_of_class(&self, k: ClassId) -> Option<ClassId> {
        if !self.prog.has_block_class_traits() || self.quote.level > 0 {
            return None;
        }
        let at = self.env.frames.iter().rposition(|f| matches!(f, Frame::Locals { classes, .. } if classes.iter().any(|&(_, x)| x == k)))?;
        self.held_holder_after(at, true)
    }

    /// The creation of the class `k` of the block with its own arguments `args`, from here,
    /// where the class `r` holds what `k` captures: its proxy factory, which makes the instance
    /// as `r`'s own body does. None where the arguments are not those of `k`'s constructor.
    pub(super) fn held_new(&mut self, r: ClassId, k: ClassId, args: crate::ast::ListRef) -> Option<TExprId> {
        let own: usize = self.syms.class(k).ctor.iter().map(|c| c.params.len()).sum();
        if own != args.len as usize {
            return None;
        }
        let m = self.held_proxy(r, HeldOf::New(k));
        let recv = self.this_ref(r);
        Some(self.prog.add(TExpr::CallMethod(recv, m, args)))
    }

    /// `held_holder` for the instance of the class `x` around a block, which a class of the block
    /// holds in the local `$this` that stands for it.
    pub(super) fn held_holder_of_this(&self, x: ClassId) -> Option<ClassId> {
        if !self.prog.has_block_class_traits() || self.quote.level > 0 {
            return None;
        }
        let at = self.env.frames.iter().rposition(|f| matches!(f, Frame::Class(k) if *k == x))?;
        self.held_holder_after(at, false)
    }

    fn held_holder_after(&self, at: usize, in_block: bool) -> Option<ClassId> {
        let mut in_block = in_block;
        let mut holder = None;
        for f in &self.env.frames[at + 1..] {
            match f {
                Frame::Locals { .. } => in_block = true,
                Frame::Class(k) => {
                    if self.syms.class(*k).kind == ClassKind::Trait {
                        if !self.dotty_local_trait(*k) {
                            return holder;
                        }
                    } else if holder.is_none() && in_block {
                        holder = Some(*k);
                    }
                }
            }
        }
        None
    }

    /// The read of the value `s` the class `r` holds, from here: its proxy member on `r`'s
    /// instance as this place reaches it (`this_ref`), dotty's `proxyRef` through `memberRef`.
    pub(super) fn held_read(&mut self, r: ClassId, s: SymId) -> TExprId {
        let m = self.held_proxy(r, HeldOf::Get(s));
        let ret = self.sig_of(m).ret;
        let recv = self.this_ref(r);
        let te = self.prog.add(TExpr::CallMethod(recv, m, crate::ast::ListRef::EMPTY));
        self.prog.set_type(te, ret);
        te
    }

    /// The write of the `var` `s` the class `r` holds, from here: its proxy setter.
    pub(super) fn held_write(&mut self, r: ClassId, s: SymId, recv: TExprId, value: TExprId) -> TExprId {
        let m = self.held_proxy(r, HeldOf::Set(s));
        let l = self.prog.list(&[value]);
        let te = self.prog.add(TExpr::CallMethod(recv, m, l));
        self.prog.set_type(te, self.b.t_unit);
        te
    }

    /// The call of the local def `s` the class `r` holds, from here, with `args`.
    pub(super) fn held_call(&mut self, r: ClassId, s: SymId, args: crate::ast::ListRef) -> TExprId {
        let m = self.held_proxy(r, HeldOf::Call(s));
        // A default the call leaves to the local def's getter is the forwarder's, selected on the
        // class that holds it: dotty's getter of a local def is a local def of the block, which
        // the trait reaches through the same proxy.
        self.retarget_defaults(args, DefaultOf::Method(s), DefaultOf::Method(m), |w| w.this_ref(r));
        let recv = self.this_ref(r);
        self.prog.add(TExpr::CallMethod(recv, m, args))
    }

    /// The proxy member of the class `r` that stands for `of`, made once: a method typed as in
    /// `r`'s body, whose own read of the value is what `r` captures, as dotty's proxy field is
    /// `r`'s own (`LambdaLift.generateProxies`), a `var` in its cell, a local def `r`'s copy.
    fn held_proxy(&mut self, r: ClassId, of: HeldOf) -> SymId {
        if let Some(&m) = self.held_proxies.get(&(r, of)) {
            return m;
        }
        let (name, span, by_name) = match of {
            HeldOf::Get(s) | HeldOf::Set(s) | HeldOf::Call(s) => {
                let info = self.syms.sym(s);
                (info.name, info.span, info.by_name)
            }
            HeldOf::New(k) => {
                let info = self.syms.class(k);
                (info.name, info.span, false)
            }
        };
        let base = self.name_str(name);
        let text = match of {
            HeldOf::Get(_) | HeldOf::Call(_) => format!("{}$proxy", base),
            HeldOf::Set(_) => format!("{}$proxy_=", base),
            HeldOf::New(_) => format!("{}$new$proxy", base),
        };
        let proxy_name = self.interner.intern(&text);
        // At the end of `r`'s definition, whichever file holds it (the source or a product's
        // pseudo file), after its own members alike in a build and in one over its products.
        let (r_file, r_def, r_span) = {
            let info = self.syms.class(r);
            (info.file, info.def, info.span)
        };
        let m = self.syms.new_sym(proxy_name, SymKind::Def, 0, Owner::Class(r), r_file, r_def, crate::source::Span { start: r_span.end, end: r_span.end });
        let mut defaults: Vec<Option<TExprId>> = Vec::new();
        let (sig, params, body) = match of {
            HeldOf::Get(s) => {
                let ret = self.sig_of(s).ret;
                let local = self.prog.add(TExpr::Local(s));
                let body = if by_name { self.prog.add(TExpr::CallClosure(local, crate::ast::ListRef::EMPTY)) } else { local };
                self.prog.set_type(body, ret);
                (MethodSig::value(ret), Vec::new(), body)
            }
            HeldOf::Set(s) => {
                let ty = self.sig_of(s).ret;
                let v = self.new_local(name, SymKind::Val, ty, span);
                let target = self.prog.add(TExpr::Local(s));
                let value = self.prog.add(TExpr::Local(v));
                let body = self.assignment(target, value);
                let unit = self.b.t_unit;
                let clause = ClauseSig { params: vec![ParamSig { name, ty, by_name: false, repeated: false, has_default: false, sym: v }], is_using: false, is_implicit: false };
                (MethodSig { tparams: Vec::new(), clauses: vec![clause], ret: unit, declared: crate::tir::NO_TYPE }, vec![v], body)
            }
            HeldOf::Call(s) => {
                let sig = self.sig_of(s).clone();
                let total: usize = sig.clauses.iter().map(|c| c.params.len()).sum();
                // The def's own parameters, which its defaults name, and the defaults copied here,
                // in `r`'s body, whose captures take what they read: a call that leaves an argument
                // out gets the def's default, as a call of the def does.
                let own = self.fun_of_sym.get(&s).copied().map(|f| (self.prog.funs[f.idx()].params.clone(), self.prog.funs[f.idx()].defaults.clone(), self.prog.funs[f.idx()].captures));
                let mut params = Vec::new();
                let mut clauses = Vec::new();
                match own {
                    Some((ps, ds, 0)) if ps.len() == total => {
                        for c in &sig.clauses {
                            clauses.push(ClauseSig { params: c.params.clone(), is_using: c.is_using, is_implicit: c.is_implicit });
                        }
                        params = ps;
                        defaults = ds.into_iter().map(|d| d.map(|e| self.copy_expr(e))).collect();
                    }
                    _ => {
                        for c in &sig.clauses {
                            let mut ps = Vec::new();
                            for p in &c.params {
                                let v = self.new_local(p.name, SymKind::Val, p.ty, span);
                                if p.by_name {
                                    self.syms.sym_mut(v).by_name = true;
                                }
                                params.push(v);
                                ps.push(ParamSig { sym: v, has_default: false, ..p.clone() });
                            }
                            clauses.push(ClauseSig { params: ps, is_using: c.is_using, is_implicit: c.is_implicit });
                        }
                    }
                }
                let args: Vec<TExprId> = params.iter().map(|&v| self.prog.add(TExpr::Local(v))).collect();
                let l = self.prog.list(&args);
                let body = self.prog.add(TExpr::CallStatic(s, l));
                self.prog.set_type(body, sig.ret);
                (MethodSig { tparams: sig.tparams.clone(), clauses, ret: sig.ret, declared: crate::tir::NO_TYPE }, params, body)
            }
            HeldOf::New(k) => {
                let ctor = self.syms.class(k).ctor.clone();
                let mut params = Vec::new();
                let mut clauses = Vec::new();
                for c in &ctor {
                    let mut ps = Vec::new();
                    for p in &c.params {
                        let v = self.new_local(p.name, SymKind::Val, p.ty, span);
                        if p.by_name {
                            self.syms.sym_mut(v).by_name = true;
                        }
                        params.push(v);
                        ps.push(ParamSig { sym: v, has_default: false, ..p.clone() });
                    }
                    clauses.push(ClauseSig { params: ps, is_using: c.is_using, is_implicit: c.is_implicit });
                }
                let args: Vec<TExprId> = params.iter().map(|&v| self.prog.add(TExpr::Local(v))).collect();
                let l = self.prog.list(&args);
                // Made as in `r`'s body: the frames up to `r`'s, where what `k` captures is `r`'s.
                let at = self.env.frames.iter().rposition(|f| matches!(f, Frame::Class(x) if *x == r)).expect("the holder encloses the creation");
                let inner = self.env.frames.split_off(at + 1);
                let body = self.new_instance(k, l, span);
                self.env.frames.extend(inner);
                let ty = self.syms.this_type(k);
                self.prog.set_type(body, ty);
                (MethodSig { tparams: Vec::new(), clauses, ret: ty, declared: crate::tir::NO_TYPE }, params, body)
            }
        };
        {
            let mut info = self.syms.sym_mut(m);
            info.sig = Some(Arc::new(sig));
            info.state().set(Completion::Done);
        }
        self.syms.add_member(r, m);
        let n = params.len();
        if defaults.len() != n {
            defaults = vec![None; n];
        }
        let f = self.prog.add_fun(TFun { sym: m, params, defaults, body: Some(body), body_unconsuming: false, captures: 0, lifted: false });
        self.fun_of_sym.insert(m, f);
        match self.checked_class_mut(r) {
            Some(tc) => tc.methods.push(f),
            None => self.pending_held.entry(r).or_default().push(f),
        }
        self.held_proxies.insert((r, of), m);
        self.held_proxy_of.insert(m, of);
        m
    }

    /// The locals of the block that the members of the trait `t` may read: those the local
    /// trait's definition sees, or a trait nested in a local class's at any depth. None for a
    /// trait of a class or package alone.
    fn trait_block_locals(&mut self, t: ClassId) -> Option<Vec<SymId>> {
        let mut k = t;
        loop {
            if self.anon_envs.contains_key(&k) {
                return Some(self.visible_to_class(k));
            }
            match self.syms.class(k).owner {
                Owner::Class(o) => k = o,
                _ => return None,
            }
        }
    }

    /// The values of `visible` the trees under `roots` read, in the order they are first met.
    fn block_reads(&self, roots: &[TExprId], visible: &[SymId]) -> Vec<SymId> {
        let mut out: Vec<SymId> = Vec::new();
        for &root in roots {
            for e in self.prog.descendants(root) {
                let s = match self.prog.exprs[e.idx()] {
                    TExpr::Local(s) | TExpr::CallStatic(s, _) => s,
                    _ => continue,
                };
                if visible.contains(&s) && !out.contains(&s) {
                    out.push(s);
                }
            }
        }
        out
    }

    /// The trait `t` just checked. A local trait (`dotty_local_trait`) is lifted: its members take
    /// the block's values they need as proxies (`lift_local_trait`). A trait a class of the block
    /// owns takes none: its members and the classes nested in it read them through the
    /// enclosing instances, from the proxies the class of the block holds (`held_holder`,
    /// dotty's `markFree` and `proxyRef`); its calls of the lifted members of the local traits
    /// around it pass their proxies.
    pub(super) fn lift_trait_proxies(&mut self, t: ClassId) {
        // A stored body's trait is lifted where each expansion copies it, over the copy's body
        // (`Worker::lift_class_copy`), as dotty lifts after inlining.
        if self.is_stored_class(t) {
            return;
        }
        let Some(visible) = self.trait_block_locals(t) else { return };
        if visible.is_empty() {
            return;
        }
        if self.dotty_local_trait(t) {
            return self.lift_local_trait(t, &visible);
        }
        if let Some(tc) = self.checked_class(t).cloned() {
            let roots = self.lifted_roots(std::slice::from_ref(&tc));
            self.adapt_lifted_calls(&roots);
        }
    }

    /// Whether the trait `t` is local as dotty's `Dependencies.isLocal` has a trait whose members
    /// take their free values from their callers: owned by a term, or by such a trait.
    pub(super) fn dotty_local_trait(&self, t: ClassId) -> bool {
        let info = self.syms.class(t);
        info.kind == ClassKind::Trait
            && match info.owner {
                Owner::Local => true,
                Owner::Class(o) => self.dotty_local_trait(o),
                Owner::Package(_) => false,
            }
    }

    /// The expressions of the classes `classes`: their members' bodies and defaults, their
    /// initialisers and the arguments of their parents' constructors.
    fn lifted_roots(&self, classes: &[TClass]) -> Vec<TExprId> {
        let mut out = self.class_roots(classes);
        for tc in classes {
            for i in &tc.init {
                if let TInit::Parent(_, pc) = i {
                    out.extend(self.prog.expr_list(pc.args).iter().copied());
                }
            }
        }
        out
    }

    /// The classes nested in `t` at any depth that are checked, objects and their companions
    /// included: what `capture_through_nested` gives captures.
    fn nested_closure(&mut self, t: ClassId) -> Vec<ClassId> {
        let mut out: Vec<ClassId> = Vec::new();
        let mut todo = vec![t];
        while let Some(k) = todo.pop() {
            let mut inner: Vec<ClassId> = self.syms.class(k).nested.values().copied().collect();
            inner.extend(inner.clone().into_iter().filter_map(|n| self.syms.class(n).companion));
            let members: Vec<SymId> = self.syms.class(k).members.values().copied().collect();
            inner.extend(members.into_iter().filter_map(|s| self.inner_object_of_sym(s)));
            inner.sort();
            inner.dedup();
            for n in inner {
                if n != t && !out.contains(&n) && self.checked_class(n).is_some() {
                    out.push(n);
                    todo.push(n);
                }
            }
        }
        out
    }

    /// The local trait `t` just checked, lifted as dotty's `LambdaLift` lifts it: the free values
    /// of each member, of each lazy val's initialiser, of the trait's initialiser and of each class
    /// nested in it at any depth, closed over the calls and the creations between them
    /// (`Dependencies.computeFreeVars`); the members take theirs as leading parameters, a lazy
    /// val's initialiser becomes such a member, the classes nested in it hold theirs, and every
    /// call in the trait passes the proxies (`addFreeArgs`).
    fn lift_local_trait(&mut self, t: ClassId, visible: &[SymId]) {
        let Some(mut tclass) = self.checked_class(t).cloned() else { return };
        let nested = self.nested_closure(t);
        // The units: each member, each lazy val, the initialiser, each nested class.
        #[derive(Clone, Copy, PartialEq)]
        enum Unit {
            Fun(FunId),
            Lazy(usize),
            Init,
            Class(ClassId),
        }
        let mut units: Vec<(Unit, Vec<TExprId>)> = Vec::new();
        for &f in &tclass.methods {
            let fun = &self.prog.funs[f.idx()];
            let mut roots: Vec<TExprId> = fun.defaults.iter().flatten().copied().collect();
            roots.extend(fun.body);
            units.push((Unit::Fun(f), roots));
        }
        let mut init_roots: Vec<TExprId> = Vec::new();
        for (i, x) in tclass.init.iter().enumerate() {
            match *x {
                TInit::Field(s, e) if self.syms.sym(s).mods & mods::LAZY != 0 => units.push((Unit::Lazy(i), vec![e])),
                TInit::Field(_, e) | TInit::Stmt(e) => init_roots.push(e),
                TInit::Parent(_, pc) => init_roots.extend(self.prog.expr_list(pc.args).iter().copied()),
            }
        }
        units.push((Unit::Init, init_roots));
        for &n in &nested {
            if let Some(tc) = self.checked_class(n).cloned() {
                let roots = self.lifted_roots(std::slice::from_ref(&tc));
                units.push((Unit::Class(n), roots));
            }
        }
        let own_fun = |w: &Self, s: SymId| units.iter().position(|(u, _)| matches!(*u, Unit::Fun(f) if w.prog.funs[f.idx()].sym == s));
        let own_lazy = |s: SymId| units.iter().position(|(u, _)| matches!(*u, Unit::Lazy(i) if matches!(tclass.init[i], TInit::Field(y, _) if y == s)));
        // What each reads directly, the members it calls, the nested classes it makes.
        enum Edge {
            Unit(usize),
            Known(Vec<SymId>),
        }
        let mut free: Vec<Vec<SymId>> = Vec::with_capacity(units.len());
        let mut edges: Vec<Vec<Edge>> = Vec::with_capacity(units.len());
        for (_, roots) in &units {
            let mut reads: Vec<SymId> = Vec::new();
            let mut out: Vec<Edge> = Vec::new();
            for &root in roots {
                for e in self.prog.descendants(root) {
                    match self.prog.exprs[e.idx()] {
                        TExpr::Local(s) | TExpr::CallStatic(s, _) if visible.contains(&s) => {
                            if !reads.contains(&s) {
                                reads.push(s);
                            }
                        }
                        TExpr::CallMethod(r, s, _) if !matches!(self.prog.exprs[r.idx()], TExpr::Super(SuperTarget::Mixin(_))) => {
                            if let Some(u) = own_fun(self, s).or_else(|| own_lazy(s)) {
                                out.push(Edge::Unit(u));
                            } else if let Some(p) = self.lifted_members.get(&s).or_else(|| self.lifted_lazy.get(&s).and_then(|m| self.lifted_members.get(m))) {
                                out.push(Edge::Known(p.clone()));
                            }
                        }
                        TExpr::Field(_, s) => {
                            if let Some(u) = own_lazy(s) {
                                out.push(Edge::Unit(u));
                            } else if let Some(p) = self.lifted_lazy.get(&s).and_then(|m| self.lifted_members.get(m)) {
                                out.push(Edge::Known(p.clone()));
                            }
                        }
                        TExpr::New(k, _) => {
                            if let Some(u) = units.iter().position(|(u, _)| *u == Unit::Class(k)) {
                                out.push(Edge::Unit(u));
                            }
                        }
                        _ => {}
                    }
                }
            }
            free.push(reads);
            edges.push(out);
        }
        // Closed over the calls and creations (`computeFreeVars`).
        loop {
            let mut grew = false;
            for u in 0..units.len() {
                let mut add: Vec<SymId> = Vec::new();
                for edge in &edges[u] {
                    match edge {
                        Edge::Unit(v) => add.extend(free[*v].iter().copied()),
                        Edge::Known(p) => add.extend(p.iter().copied()),
                    }
                }
                for s in add {
                    if !free[u].contains(&s) {
                        free[u].push(s);
                        grew = true;
                    }
                }
            }
            if !grew {
                break;
            }
        }
        // A lazy val that needs values: its initialiser a member taking them, which a read calls
        // (the trait's own field reaches it through the source signature, a class's forwarder).
        for (u, (unit, _)) in units.iter().enumerate() {
            let Unit::Lazy(i) = *unit else { continue };
            if free[u].is_empty() {
                continue;
            }
            let TInit::Field(s, e) = tclass.init[i] else { continue };
            let call = self.lazy_initialiser(t, s, e, free[u].clone(), &mut tclass);
            let m = match self.prog.exprs[call.idx()] {
                TExpr::CallMethod(_, m, _) => m,
                _ => continue,
            };
            tclass.init[i] = TInit::Field(s, call);
            self.lifted_members.insert(m, free[u].clone());
            self.lifted_lazy.insert(s, m);
        }
        for (u, (unit, _)) in units.iter().enumerate() {
            if let Unit::Fun(f) = *unit {
                if !free[u].is_empty() {
                    let sym = self.prog.funs[f.idx()].sym;
                    self.lifted_members.insert(sym, free[u].clone());
                }
            }
        }
        if let Some(tc) = self.checked_class_mut(t) {
            *tc = tclass.clone();
        }
        // Every call in the trait and the classes nested in it passes the proxies; not the
        // lazy fields' own reads of their initialisers, which go through the source signature.
        let mut roots: Vec<TExprId> = Vec::new();
        for (unit, r) in &units {
            match unit {
                Unit::Lazy(i) => {
                    if let TInit::Field(_, e) = tclass.init[*i] {
                        if !matches!(self.prog.exprs[e.idx()], TExpr::CallMethod(_, m, _) if self.lifted_lazy.values().any(|&x| x == m)) {
                            roots.push(e);
                        }
                    }
                    roots.extend(r.iter().copied());
                }
                _ => roots.extend(r.iter().copied()),
            }
        }
        self.adapt_lifted_calls(&roots);
        // What the nested classes need, held by them and passed where they are made.
        let mut needed: Vec<SymId> = Vec::new();
        for (u, (unit, _)) in units.iter().enumerate() {
            if let Unit::Class(_) = unit {
                for &s in &free[u] {
                    if !needed.contains(&s) {
                        needed.push(s);
                    }
                }
            }
        }
        if !needed.is_empty() {
            let ordered: Vec<SymId> = visible.iter().copied().filter(|s| needed.contains(s)).collect();
            self.capture_through_nested(t, &ordered);
        }
        // The members take their proxies first, the initialiser its own.
        for (u, (unit, _)) in units.iter().enumerate() {
            if let Unit::Fun(f) = *unit {
                let n = free[u].len();
                if n == 0 {
                    continue;
                }
                let fun = &mut self.prog.funs[f.idx()];
                fun.params.splice(0..0, free[u].iter().copied());
                fun.defaults.splice(0..0, std::iter::repeat(None).take(n));
                fun.captures = n as u32;
                fun.lifted = true;
                self.prog.note_lifted_fun();
            }
        }
        let init = units.iter().position(|(u, _)| *u == Unit::Init).map(|u| free[u].clone()).unwrap_or_default();
        if !init.is_empty() {
            if let Some(tc) = self.checked_class_mut(t) {
                let n = init.len();
                tc.ctor_params.splice(0..0, init);
                tc.ctor_defaults.splice(0..0, std::iter::repeat(None).take(n));
                tc.captures = n;
            }
        }
    }

    /// dotty's `LambdaLift.addFreeArgs`: each call under `roots` of a lifted member of a local
    /// trait passes the member's proxies before its own arguments, and a read of a lazy val whose
    /// initialiser takes some calls the initialiser with them. A call through a trait's `super`
    /// goes through the mixing class's accessor, which passes them itself.
    pub(super) fn adapt_lifted_calls(&mut self, roots: &[TExprId]) {
        self.adapt_lifted_calls_in(roots, false);
    }

    /// `adapt_lifted_calls` over the statements of a block being finished, where a proxy that
    /// stands for an enclosing instance is that instance as the block reads it (`this` in its
    /// class's own method), not the local a lifted class holds it in.
    pub(super) fn adapt_block_calls(&mut self, block: TExprId) {
        self.adapt_lifted_calls_in(&[block], true);
    }

    fn adapt_lifted_calls_in(&mut self, roots: &[TExprId], at_block: bool) {
        if self.lifted_members.is_empty() {
            return;
        }
        let mut found: Vec<TExprId> = Vec::new();
        for &root in roots {
            for e in self.prog.descendants(root) {
                match self.prog.exprs[e.idx()] {
                    TExpr::CallMethod(r, s, _) if (self.lifted_members.contains_key(&s) || self.lifted_lazy.contains_key(&s)) && !matches!(self.prog.exprs[r.idx()], TExpr::Super(SuperTarget::Mixin(_))) => found.push(e),
                    TExpr::Field(_, s) if self.lifted_lazy.contains_key(&s) => found.push(e),
                    _ => {}
                }
            }
        }
        for e in found {
            let (r, s, args) = match self.prog.exprs[e.idx()] {
                TExpr::CallMethod(r, s, args) => (r, s, Some(args)),
                TExpr::Field(r, s) => (r, s, None),
                _ => continue,
            };
            let (target, args) = match self.lifted_lazy.get(&s) {
                Some(&m) if args.map_or(true, |a| a.len == 0) => (m, crate::ast::ListRef::EMPTY),
                _ => match args {
                    Some(a) => (s, a),
                    None => continue,
                },
            };
            let Some(proxies) = self.lifted_members.get(&target).cloned() else { continue };
            let Some(&f) = self.fun_of_sym.get(&target) else { continue };
            let fun = &self.prog.funs[f.idx()];
            let declared = fun.params.len() - fun.captures as usize;
            if args.len as usize != declared {
                continue;
            }
            let mut all: Vec<TExprId> = Vec::with_capacity(proxies.len() + args.len as usize);
            for &p in &proxies {
                let instance = match self.syms.sym(p).sig.as_ref().map(|sig| self.types.get(sig.ret)) {
                    Some(Type::This(o)) if at_block && self.outer_this.get(&o) == Some(&p) => Some(o),
                    _ => None,
                };
                all.push(match instance {
                    Some(o) => self.this_ref(o),
                    None => self.proxy_read(p),
                });
            }
            all.extend_from_slice(self.prog.expr_list(args));
            let l = self.prog.list(&all);
            self.prog.exprs[e.idx()] = TExpr::CallMethod(r, target, l);
        }
    }

    /// What the class `k` holds for its super accessors: the proxies of each lifted member a
    /// `super` call of a trait it mixes in binds to in its linearisation (dotty's `ResolveSuper`
    /// accessor, which reads them from the class).
    fn super_accessor_proxies(&mut self, k: ClassId) -> Vec<SymId> {
        let mut out: Vec<SymId> = Vec::new();
        let bases: Vec<ClassId> = self.syms.class(k).base_types.iter().map(|&(b, _)| b).collect();
        let superclass = self.syms.class(k).superclass;
        for (pos, &tr) in bases.iter().enumerate().skip(1) {
            if self.syms.class(tr).kind != ClassKind::Trait || !self.in_block(tr) {
                continue;
            }
            // A superclass that mixes the trait in binds its calls already.
            if superclass.is_some_and(|s| self.syms.class(s).base_types.iter().any(|&(b, _)| b == tr)) {
                continue;
            }
            let Some(members) = self.mixin_supers.get(&tr).cloned() else { continue };
            for m in members {
                let Some(target) = self.mixin_super_target_in(&bases, pos, m.member) else { continue };
                let Some(&f) = self.fun_of_sym.get(&target) else { continue };
                let fun = &self.prog.funs[f.idx()];
                for &s in &fun.params[..fun.captures as usize] {
                    if !out.contains(&s) {
                        out.push(s);
                    }
                }
            }
        }
        out
    }

    /// The initialiser `e` of the lazy member `s` of the trait `t`, which reads `proxies`, as a
    /// member of the trait taking them (dotty's `y$lzyINIT`), and the call of it that initialises
    /// `s` in its place.
    fn lazy_initialiser(&mut self, t: ClassId, s: SymId, e: TExprId, proxies: Vec<SymId>, tclass: &mut TClass) -> TExprId {
        let (name, file, def, span) = {
            let info = self.syms.sym(s);
            (info.name, info.file, info.def, info.span)
        };
        let ret = self.sig_of(s).ret;
        let text = format!("{}$lzyINIT", self.name_str(name));
        let init_name = self.interner.intern(&text);
        // At the lazy val's place, whichever file holds the trait's definition (the source or
        // a product's pseudo file), the outputs ordering it alike.
        let m = self.syms.new_sym(init_name, SymKind::Def, 0, Owner::Class(t), file, def, span);
        {
            let mut info = self.syms.sym_mut(m);
            info.sig = Some(Arc::new(MethodSig { tparams: Vec::new(), clauses: Vec::new(), ret, declared: crate::tir::NO_TYPE }));
            info.state().set(Completion::Done);
        }
        self.syms.add_member(t, m);
        let n = proxies.len();
        let f = self.prog.add_fun(TFun { sym: m, params: proxies, defaults: vec![None; n], body: Some(e), body_unconsuming: false, captures: n as u32, lifted: false });
        self.prog.note_lifted_fun();
        self.fun_of_sym.insert(m, f);
        tclass.methods.push(f);
        let this = self.prog.add(TExpr::This);
        let call = self.prog.add(TExpr::CallMethod(this, m, crate::ast::ListRef::EMPTY));
        self.prog.set_type(call, ret);
        call
    }

    /// The copy `c` of a stored body's class, walked: a trait takes its proxies from what the
    /// copy's members read, a class the forwarders of the traits it mixes in and captures what
    /// they pass after `captures`.
    pub(super) fn lift_class_copy(&mut self, c: ClassId, captures: &mut Vec<SymId>) {
        if self.syms.class(c).kind == ClassKind::Trait {
            self.lift_trait_proxies(c);
            return;
        }
        self.add_trait_forwarders(c);
        let roots = self.checked_class(c).cloned().map(|tc| self.lifted_roots(std::slice::from_ref(&tc))).unwrap_or_default();
        let visible = self.visible_to_class(c);
        for s in self.block_reads(&roots, &visible) {
            if !captures.contains(&s) {
                captures.push(s);
            }
        }
        self.add_accessor_captures(c, captures);
    }

    /// The proxies the local class `c`'s super accessors pass (`super_accessor_proxies`), held
    /// after what its body reads.
    pub(super) fn add_accessor_captures(&mut self, c: ClassId, captures: &mut Vec<SymId>) {
        if let Some(held) = self.accessor_captures.get(&c) {
            for &s in held {
                if !captures.contains(&s) {
                    captures.push(s);
                }
            }
        }
    }

    /// Whether the class `c` mixes in a local trait not checked yet: one of the block defined
    /// after it, whose members' proxies are not known.
    pub(super) fn waits_for_local_trait(&mut self, c: ClassId) -> bool {
        let traits: Vec<ClassId> = self.syms.class(c).base_types.iter().skip(1).map(|&(b, _)| b).collect();
        traits.into_iter().any(|b| self.syms.class(b).kind == ClassKind::Trait && self.syms.class(b).owner == Owner::Local && self.checked_class(b).is_none())
    }

    /// The local classes of the block that waited for a local trait checked since: each takes the
    /// forwarders of the traits it mixes in, and captures what they pass after what its own body
    /// does. `all` at the end of the block finishes every one left from `from` on, its own.
    pub(super) fn finish_deferred_classes(&mut self, from: usize, all: bool) {
        let mut i = from;
        while i < self.deferred_classes.len() {
            let c = self.deferred_classes[i].0;
            if !all && self.waits_for_local_trait(c) {
                i += 1;
                continue;
            }
            let (_, mut captures) = self.deferred_classes.remove(i);
            self.add_trait_forwarders(c);
            // What its forwarders and its adapted calls read besides.
            let roots = self.checked_class(c).cloned().map(|tc| self.lifted_roots(std::slice::from_ref(&tc))).unwrap_or_default();
            let visible = self.visible_to_class(c);
            for s in self.block_reads(&roots, &visible) {
                if !captures.contains(&s) {
                    captures.push(s);
                }
            }
            self.add_accessor_captures(c, &mut captures);
            self.settle_local_class(c, captures);
        }
    }

    /// Whether the trait `t` may have proxies: a local trait, or one nested in a local class.
    fn lifted_trait(&self, t: ClassId) -> bool {
        self.syms.class(t).kind == ClassKind::Trait && self.in_block(t)
    }

    /// Whether the class `c` is defined in a block, at any depth of classes: only such a class
    /// can mix in a local trait.
    pub(super) fn in_block(&self, c: ClassId) -> bool {
        let mut k = c;
        loop {
            match self.syms.class(k).owner {
                Owner::Local => return true,
                Owner::Class(o) => k = o,
                _ => return false,
            }
        }
    }

    /// The class `tclass` just checked mixes in local traits whose members take proxies: it
    /// implements each such member that wins in its linearisation and that no class above it
    /// implements already by a forwarder passing the proxies (`Mixin.mixinForwarders`), its
    /// own `super` calls of such members pass them, and so does its call of a trait's
    /// initialiser that takes some. Its captures then take what it passes.
    pub(super) fn add_trait_forwarders(&mut self, k: ClassId) {
        if !self.in_block(k) || self.is_stored_class(k) {
            return;
        }
        // Its calls of local traits' lifted members pass their proxies, and its accessors of
        // `super` calls bound to such members pass the ones it holds.
        if !self.lifted_members.is_empty() {
            if let Some(tc) = self.checked_class(k).cloned() {
                let roots = self.lifted_roots(std::slice::from_ref(&tc));
                self.adapt_lifted_calls(&roots);
            }
        }
        let held = self.super_accessor_proxies(k);
        if !held.is_empty() {
            self.accessor_captures.insert(k, held);
        }
        let traits: Vec<ClassId> = self.syms.class(k).base_types.iter().skip(1).map(|&(b, _)| b).filter(|&b| self.lifted_trait(b)).collect();
        if traits.is_empty() {
            return;
        }
        // The members with proxies of those traits, and the initialisers that take some.
        let mut proxied: Vec<(SymId, FunId)> = Vec::new();
        let mut inits: Vec<(ClassId, Vec<SymId>)> = Vec::new();
        for &t in &traits {
            if self.checked_class(t).is_none() && self.syms.class(t).owner != Owner::Local {
                self.check_class(t);
            }
            let Some((methods, captures, ctor_params)) = self.checked_class(t).map(|tc| (tc.methods.clone(), tc.captures, tc.ctor_params.clone())) else { continue };
            for &f in &methods {
                let fun = &self.prog.funs[f.idx()];
                if fun.captures > 0 {
                    proxied.push((fun.sym, f));
                }
            }
            if captures > 0 {
                inits.push((t, ctor_params[..captures].to_vec()));
            }
        }
        if proxied.is_empty() && inits.is_empty() {
            return;
        }
        let Some(mut tclass) = self.checked_class_mut(k).map(|tc| tc.clone()) else { return };
        let superclass = self.syms.class(k).superclass;
        for &(m, f) in &proxied {
            if !self.wins_in(k, m) {
                continue;
            }
            // A class above that mixes the trait in has the forwarder already.
            if superclass.is_some_and(|s| self.wins_in(s, m)) {
                continue;
            }
            let fwd = self.trait_forwarder(k, m, f);
            tclass.methods.push(fwd);
        }
        // `super.m(args)` of a member with proxies, in the class's own body.
        let roots = self.class_roots(std::slice::from_ref(&tclass));
        let mut supers: Vec<TExprId> = Vec::new();
        for root in roots {
            for e in self.prog.descendants(root) {
                if let TExpr::CallMethod(r, s, _) = self.prog.exprs[e.idx()] {
                    if matches!(self.prog.exprs[r.idx()], TExpr::Super(_)) && proxied.iter().any(|&(m, _)| m == s) {
                        supers.push(e);
                    }
                }
            }
        }
        for e in supers {
            let TExpr::CallMethod(r, s, args) = self.prog.exprs[e.idx()] else { continue };
            let Some(&(_, f)) = proxied.iter().find(|&&(m, _)| m == s) else { continue };
            let n = self.prog.funs[f.idx()].captures as usize;
            let proxies: Vec<SymId> = self.prog.funs[f.idx()].params[..n].to_vec();
            let items = self.prog.expr_list(args).to_vec();
            if items.len() + n != self.prog.funs[f.idx()].params.len() {
                continue;
            }
            let mut all: Vec<TExprId> = proxies.iter().map(|&p| self.proxy_read(p)).collect();
            all.extend(items);
            let l = self.prog.list(&all);
            self.prog.exprs[e.idx()] = TExpr::CallMethod(r, s, l);
        }
        for (t, proxies) in inits {
            for init in tclass.init.iter_mut() {
                let TInit::Parent(b, pc) = init else { continue };
                if *b != t {
                    continue;
                }
                let given = self.prog.expr_list(pc.args);
                if given.len() >= proxies.len() && proxies.iter().zip(given).all(|(&p, &a)| matches!(self.prog.exprs[a.idx()], TExpr::Local(s) if s == p)) {
                    continue;
                }
                let mut all: Vec<TExprId> = proxies.iter().map(|&p| self.prog.add(TExpr::Local(p))).collect();
                all.extend_from_slice(self.prog.expr_list(pc.args));
                pc.args = self.prog.list(&all);
            }
        }
        if let Some(tc) = self.checked_class_mut(k) {
            *tc = tclass;
        }
    }

    /// A read of the proxy `p` where a class passes it on.
    fn proxy_read(&mut self, p: SymId) -> TExprId {
        let e = self.prog.add(TExpr::Local(p));
        if let Some(sig) = self.syms.sym(p).sig.as_ref().map(|s| s.ret) {
            self.prog.set_type(e, sig);
        }
        e
    }

    /// Whether the member `m` of a trait `k` derives from is the definition that runs on an
    /// instance of `k`: no class or trait before the trait in `k`'s linearisation defines a member
    /// with a body that overrides it.
    fn wins_in(&mut self, k: ClassId, m: SymId) -> bool {
        let Owner::Class(t) = self.syms.sym(m).owner else { return false };
        let name = self.syms.sym(m).name;
        let bases: Vec<(ClassId, TypeId)> = self.syms.class(k).base_types.clone();
        let Some(&(_, t_ty)) = bases.iter().find(|&&(b, _)| b == t) else { return false };
        for (i, &(b, bt)) in bases.iter().enumerate() {
            if b == t {
                return true;
            }
            let mut j = 0;
            while let Some(s) = self.inherited_member(b, name, false, j) {
                j += 1;
                if self.syms.sym(s).mods & mods::ABSTRACT != 0 {
                    continue;
                }
                let ssig = self.sig_arc(s);
                if self.compare_sigs(k, ssig, (i > 0).then_some(bt), m, t_ty) != super::check::Agreement::Params {
                    return false;
                }
            }
        }
        false
    }

    /// The forwarder `s` among the members of `k`, an alternative beside a forwarder of its name
    /// made before it (the trait's alternatives).
    fn add_forwarder_member(&mut self, k: ClassId, s: SymId) {
        let name = self.syms.sym(s).name;
        let own = self.syms.class(k).members.get(&name).copied().filter(|&o| self.syms.sym(o).owner == Owner::Class(k));
        match own {
            None => self.syms.add_member(k, s),
            Some(o) => {
                self.syms.class_mut(k).member_order.push(s);
                match self.syms.sym(o).kind {
                    SymKind::Overloaded(i) => {
                        self.syms.sym_mut(s).alternative = true;
                        self.syms.overloads.get_mut(i).1.push(s);
                    }
                    _ => {
                        let set = self.syms.new_overloaded(o, Owner::Class(k), vec![o, s]);
                        self.syms.class_mut(k).members.insert(name, set);
                    }
                }
            }
        }
    }

    /// The forwarder of the class `k` for the member `m` of a local trait, whose body is `f`:
    /// `def m(params) = super[T].m(proxies, params)`, the proxies read from what `k` holds.
    fn trait_forwarder(&mut self, k: ClassId, m: SymId, f: FunId) -> FunId {
        let Owner::Class(t) = self.syms.sym(m).owner else { unreachable!("a member") };
        let (name, file, span) = {
            let info = self.syms.sym(m);
            (info.name, self.syms.class(k).file, info.span)
        };
        let msig = self.sig_of(m).clone();
        let s = self.syms.new_sym(name, SymKind::Def, mods::OVERRIDE, Owner::Class(k), file, None, span);
        let n = self.prog.funs[f.idx()].captures as usize;
        let proxies: Vec<SymId> = self.prog.funs[f.idx()].params[..n].to_vec();
        let mut args: Vec<TExprId> = proxies.iter().map(|&p| self.proxy_read(p)).collect();
        let mut clauses = Vec::with_capacity(msig.clauses.len());
        let mut params = Vec::new();
        for cl in &msig.clauses {
            let mut ps = Vec::with_capacity(cl.params.len());
            for p in &cl.params {
                let q = self.accessor_param(p, file, span);
                params.push(q);
                let r = self.prog.add(TExpr::Local(q));
                self.prog.set_type(r, p.ty);
                args.push(r);
                ps.push(ParamSig { sym: q, ..*p });
            }
            clauses.push(ClauseSig { params: ps, ..*cl });
        }
        let sig = MethodSig { tparams: msig.tparams.clone(), clauses, ret: msig.ret, declared: crate::tir::NO_TYPE };
        let recv = self.prog.add(TExpr::Super(SuperTarget::Class(t)));
        let list = self.prog.list(&args);
        let call = self.prog.add(TExpr::CallMethod(recv, m, list));
        self.prog.set_type(call, msig.ret);
        {
            let mut info = self.syms.sym_mut(s);
            info.sig = Some(Arc::new(sig));
            info.state().set(Completion::Done);
        }
        self.add_forwarder_member(k, s);
        // A default reads the block's values where the class holds them: the trait's own.
        let defaults = self.prog.funs[f.idx()].defaults[n..].to_vec();
        let fwd = self.prog.add_fun(TFun { sym: s, params, defaults, body: Some(call), body_unconsuming: false, captures: 0, lifted: false });
        self.fun_of_sym.insert(s, fwd);
        fwd
    }
}
