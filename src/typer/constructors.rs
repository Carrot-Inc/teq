//! dotty's `Constructors.intoConstr` (Constructors.scala 152-180) over a class's constructor code, as the backends take
//! the class: an identifier naming one of the class's own immutable parameter accessors (`MARK_PARAM_IDENT`, which
//! `Worker::mark_if_param_ident` sets where it is typed and the expansions' and the quotes' copies keep) reads the
//! constructor's parameter wherever the constructor's code holds it, as a selection `this.s` does where the accessor is
//! effectively final (`isEffectivelyFinal`: final, private, or a member of a class nothing extends). The walk enters
//! what dotty's constructor keeps local: a closure, a local function and its parameters' defaults, a lambda's class for
//! a SAM type, whose body `ExpandSAMs` keeps a local function (`$anonfun`) and which captures the parameter here; it
//! stops at every class's body (`noDirectRefsFrom`): a local class, an anonymous class with its parent's arguments (one
//! with an empty body, which teq makes no class for, marked `MARK_ANON_PARENT`), a partial function literal (`$pf`),
//! which `ExpandSAMs.toPartialFunction` makes a class. A selection `this.s` of an accessor a subclass may override is
//! no identifier and stays the accessor's.
//!
//! The constructor's code is `Constructors.splitStats`': a non-lazy val's or var's right side and a statement of the
//! class's body. The walk runs once a class's expansions are done: over the program's classes after the inlining
//! phase, and over the files no suspended call waits on before a later round of it, whose macros may run their
//! constructors (dotty's later phases done with those units, `Run.compileSuspendedUnits`); at the end of the check of a
//! class whose calls expand as it is typed (a library's, the std's).

use super::*;
use crate::ast::mods;
use crate::tir::{TExpr, TInit, TStmt};

impl<'a> Worker<'a> {
    /// `into_constr` over the records of the classes of the program's files, those `waiting` names apart (the files of
    /// the phase's later round).
    pub fn into_constr_program(&mut self, waiting: Option<&crate::intern::FxMap<u32, ()>>) {
        for ti in 0..self.prog.classes.len() {
            if self.prog.classes[ti].init.is_empty() && self.prog.classes[ti].parent_args.is_none() {
                continue;
            }
            let file = self.syms.class(self.prog.classes[ti].id).file;
            if self.program_file(file) && !waiting.is_some_and(|w| w.contains_key(&file.0)) {
                self.into_constr(ti);
            }
        }
    }

    /// The reads of the parameters for their accessors in the constructor code of the class record `ti`.
    pub(super) fn into_constr(&mut self, ti: usize) {
        let c = self.prog.classes[ti].id;
        let no_code = self.prog.classes[ti].init.is_empty() && self.prog.classes[ti].parent_args.is_none();
        if no_code || self.syms.class(c).kind == ClassKind::Trait || self.syms.class(c).ctor_syms.is_empty() {
            return;
        }
        let roots: Vec<TExprId> = self.prog.classes[ti]
            .init
            .iter()
            .filter_map(|i| match *i {
                TInit::Field(s, e) if self.syms.sym(s).mods & mods::LAZY == 0 => Some(e),
                TInit::Stmt(e) => Some(e),
                _ => None,
            })
            .collect();
        self.into_constr_code(c, &roots, None, false);
        // The super calls, the superclass's and a trait's with what the class passes it (`inSuperCall`): a loaded
        // class's, whose pickle reads its parameters there as their accessors; the program's read them as the
        // constructor's parameters already (`parent_args_of`, dotty's `superCallContext`).
        if self.program_file(self.syms.class(c).file) {
            return;
        }
        let tc = &self.prog.classes[ti];
        let mut supers: Vec<TExprId> = tc.parent_args.map_or(Vec::new(), |l| self.prog.expr_list(l).to_vec());
        for i in &tc.init {
            if let TInit::Parent(_, call) = i {
                supers.extend_from_slice(self.prog.expr_list(call.prelude));
                supers.extend_from_slice(self.prog.expr_list(call.args));
            }
        }
        if !supers.is_empty() {
            self.into_constr_code(c, &supers, None, true);
        }
    }

    /// The marked reads of `c`'s parameters in the code `roots`: `c`'s constructor's code, whose receiver is `c`'s `this`,
    /// or with `outer` the body of a lambda's class for a SAM type made there, which reads `c`'s `this` through the
    /// local `outer` and captures what it reads, a lambda's class its body makes included. In a super call
    /// (`super_call`) every read of a parameter accessor is one, marked or not, a var's too: no getter may be called
    /// there. The parameters read.
    fn into_constr_code(&mut self, c: ClassId, roots: &[TExprId], outer: Option<SymId>, super_call: bool) -> Vec<SymId> {
        // One walk: the reads it switches, the lambdas' classes for SAM types, and the classes' bodies it stops at (an
        // anonymous class's arguments, its parent's call with the temporaries an empty body's creation evaluates them
        // into, and a partial function literal's cases), whose nodes are left out after.
        let mut reads: Vec<(TExprId, TExprId, SymId)> = Vec::new();
        let mut sams: Vec<(TExprId, ClassId)> = Vec::new();
        let mut bodies: Vec<TExprId> = Vec::new();
        for &root in roots {
            self.each_ctor_code_node(root, &mut |w, e| match w.prog.expr(e) {
                TExpr::Field(r, s) if w.is_this_of_c(r, outer) && (w.reads_own_param(c, e, s) || super_call && w.is_ctor_param_of(c, s)) => reads.push((e, r, s)),
                TExpr::New(k, _) if w.sam_classes.contains_key(&k) => sams.push((e, k)),
                TExpr::New(k, args) if w.syms.class(k).kind == ClassKind::Anon || w.anon_parent(e) => bodies.extend_from_slice(w.prog.expr_list(args)),
                TExpr::Block(..) if w.anon_parent(e) => bodies.push(e),
                TExpr::Js(t, args) if w.prog.strings[t.idx()].as_str() == "$pf($0, $1)" => bodies.extend_from_slice(w.prog.expr_list(args)),
                _ => {}
            });
        }
        if !bodies.is_empty() {
            let mut inside: crate::intern::FxMap<TExprId, ()> = Default::default();
            for b in bodies {
                self.each_ctor_code_node(b, &mut |_, d| {
                    inside.insert(d, ());
                });
            }
            reads.retain(|(e, _, _)| !inside.contains_key(e));
            sams.retain(|(e, _)| !inside.contains_key(e));
        }
        let mut read: Vec<SymId> = Vec::new();
        for (e, r, s) in reads {
            self.read_param_at(e, r, s);
            if !read.contains(&s) {
                read.push(s);
            }
        }
        // A lambda's class for a SAM type reads `c`'s `this` through the local standing for it.
        let Some(&o) = self.outer_this.get(&c) else { return read };
        for (new, k) in sams {
            let Some(tc) = self.checked_class(k).cloned() else { continue };
            let body = self.lifted_roots_of(&tc);
            for s in self.into_constr_code(c, &body, Some(o), super_call) {
                self.capture_at_creation(k, new, s);
                if !read.contains(&s) {
                    read.push(s);
                }
            }
        }
        read
    }

    /// Whether `s` is one of the parameters of `c`'s primary constructor, an accessor of `c` (`ParamAccessor`).
    fn is_ctor_param_of(&self, c: ClassId, s: SymId) -> bool {
        self.syms.sym(s).owner == Owner::Class(c) && self.syms.class(c).ctor_syms.iter().any(|cl| cl.contains(&s))
    }

    /// Whether `r` is `c`'s `this` in the code `into_constr_code` walks: `This` in `c`'s own code, the local `outer` in a
    /// lambda's class.
    fn is_this_of_c(&self, r: TExprId, outer: Option<SymId>) -> bool {
        match (self.prog.expr(r), outer) {
            (TExpr::This, None) => true,
            (TExpr::Local(l), Some(o)) => l == o,
            _ => false,
        }
    }

    /// `f` of every node of `root`, the defaults of its local functions' parameters with theirs: their default getters
    /// are local functions of the constructor's code too.
    fn each_ctor_code_node(&self, root: TExprId, f: &mut dyn FnMut(&Self, TExprId)) {
        let mut roots = vec![root];
        while let Some(r) = roots.pop() {
            for e in self.prog.descendants(r) {
                if let TExpr::Block(stmts, _) = self.prog.expr(e) {
                    for s in self.prog.stmt_list(stmts) {
                        if let TStmt::Fun(fun) = *s {
                            roots.extend(self.prog.funs[fun.idx()].defaults.iter().flatten().copied());
                        }
                    }
                }
                f(self, e);
            }
        }
    }

    /// Whether `e`, a `New` or the block around one, creates an anonymous class teq makes no class for
    /// (`MARK_ANON_PARENT`).
    fn anon_parent(&self, e: TExprId) -> bool {
        self.expr_marks.get(&e).is_some_and(|&m| m & MARK_ANON_PARENT != 0)
    }

    /// Whether `e`, a read of `s` on `c`'s `this`, is one `intoConstr` switches to the parameter (its case
    /// `Ident(_) | Select(This(_), _)`): `s` one of `c`'s own immutable parameter accessors (`isMutableVarOrAccessor`
    /// not), read by an identifier (`MARK_PARAM_IDENT`), or by a selection `this.s` where `s` is effectively final
    /// (`isOverridableSelect` not: `isEffectivelyFinal`).
    fn reads_own_param(&self, c: ClassId, e: TExprId, s: SymId) -> bool {
        let ident = self.expr_marks.get(&e).is_some_and(|&m| m & MARK_PARAM_IDENT != 0);
        self.syms.sym(s).owner == Owner::Class(c) && self.is_own_param_accessor(c, s) && (ident || self.syms.is_effectively_final(s))
    }

    /// The node `e`, a read of the accessor `s` on `r`, read as the parameter `s` of its class's constructor, its path
    /// the accessor's: an identifier's through a `this` that no tree holds, which `path_of` reads as the class's and no
    /// class captures; a selection's through its receiver, which the pickles write as the source does (dotty pickles
    /// before `Constructors`).
    fn read_param_at(&mut self, e: TExprId, r: TExprId, s: SymId) {
        let recv = if self.expr_marks.get(&e).is_some_and(|&m| m & MARK_PARAM_IDENT != 0) { self.prog.add(TExpr::This) } else { r };
        self.folded_paths.insert(e, (recv, s));
        self.prog.exprs[e.idx()] = TExpr::Local(s);
    }

    /// `s` added to the captures of the lambda's class `k` and passed at its creation `new`, whose leading arguments are
    /// the captures, as `settle_lifted_captures` adds what a class's trees read.
    fn capture_at_creation(&mut self, k: ClassId, new: TExprId, s: SymId) {
        let had: Vec<SymId> = match self.anon_captures.get(&k) {
            Some(v) => v.clone(),
            None => self.checked_class(k).map_or(Vec::new(), |tc| tc.ctor_params[..tc.captures.min(tc.ctor_params.len())].to_vec()),
        };
        if had.contains(&s) {
            return;
        }
        let n = had.len();
        if let Some(tc) = self.checked_class_mut(k) {
            let at = n.min(tc.ctor_params.len());
            tc.ctor_params.insert(at, s);
            tc.ctor_defaults.insert(at, None);
            if tc.captures == n {
                tc.captures += 1;
            }
        }
        self.anon_captures.entry(k).or_default().insert(n, s);
        let TExpr::New(_, args) = self.prog.expr(new) else { return };
        let mut items = self.prog.expr_list(args).to_vec();
        let arg = self.prog.add(TExpr::Local(s));
        let this = self.prog.add(TExpr::This);
        self.folded_paths.insert(arg, (this, s));
        self.mark_param_ident(arg);
        // A by-name parameter is passed as its thunk, which the class's capture holds.
        let ret = self.sig_of(s).ret;
        let ty = if self.syms.sym(s).by_name { self.by_name_type(ret) } else { ret };
        self.prog.set_type(arg, ty);
        items.insert(n.min(items.len()), arg);
        let l = self.prog.list(&items);
        self.prog.exprs[new.idx()] = TExpr::New(k, l);
    }
}
