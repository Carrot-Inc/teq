//! What a piece of the typed IR refers to from outside itself: the captures of a lambda or a
//! local def, and the locals of a method that have to live in a cell because a closure shares
//! them with the method.

use super::Cx;
use crate::ast::{mods, ListRef};
use crate::intern::FxMap;
use crate::symbols::*;
use crate::tir::*;
use crate::types::*;
use std::rc::Rc;

#[derive(Default, Clone)]
pub struct Free {
    /// In the order of the first reference.
    pub syms: Vec<SymId>,
    pub this: bool,
}

#[derive(Default)]
pub struct Analysis {
    /// The captures of the local defs and local lazy vals looked at so far.
    memo: FxMap<SymId, Rc<Free>>,
    in_progress: Vec<SymId>,
    /// The initialiser of every local lazy val met so far.
    pub lazy_inits: FxMap<SymId, TExprId>,
}

struct Collector<'a, 'c> {
    cx: &'a Cx<'c>,
    an: &'a mut Analysis,
    referenced: Vec<SymId>,
    declared: Vec<SymId>,
    this: bool,
}

impl Analysis {
    /// The captures of the body of a lambda with the given parameters.
    pub fn of_lambda(&mut self, cx: &Cx, params: &[SymId], body: TExprId) -> Free {
        let mut c = Collector { cx, an: self, referenced: Vec::new(), declared: params.to_vec(), this: false };
        c.expr(body);
        c.finish()
    }

    /// The captures of a local def: what its body and its defaults refer to.
    pub fn of_fun(&mut self, cx: &Cx, sym: SymId) -> Rc<Free> {
        if let Some(f) = self.memo.get(&sym) {
            return f.clone();
        }
        let f = cx.fun_of_sym[sym.idx()];
        if f == u32::MAX || self.in_progress.contains(&sym) {
            return Rc::new(Free::default());
        }
        self.in_progress.push(sym);
        let fun = &cx.input.prog.funs[f as usize];
        let mut c = Collector { cx, an: self, referenced: Vec::new(), declared: fun.params.clone(), this: false };
        for &d in fun.defaults.iter().flatten() {
            c.expr(d);
        }
        if let Some(body) = fun.body {
            c.expr(body);
        }
        let free = Rc::new(c.finish());
        self.in_progress.pop();
        // A def that was cut short by a cycle is looked at again from the top.
        if self.in_progress.is_empty() {
            self.memo.insert(sym, free.clone());
        }
        free
    }

    /// The captures of the initialiser of a local lazy val, the val's own cell left out.
    pub fn of_lazy(&mut self, cx: &Cx, sym: SymId) -> Rc<Free> {
        if let Some(f) = self.memo.get(&sym) {
            return f.clone();
        }
        let Some(&init) = self.lazy_inits.get(&sym) else { return Rc::new(Free::default()) };
        if self.in_progress.contains(&sym) {
            return Rc::new(Free::default());
        }
        self.in_progress.push(sym);
        let mut c = Collector { cx, an: self, referenced: Vec::new(), declared: vec![sym], this: false };
        c.expr(init);
        let free = Rc::new(c.finish());
        self.in_progress.pop();
        if self.in_progress.is_empty() {
            self.memo.insert(sym, free.clone());
        }
        free
    }
}

impl Collector<'_, '_> {
    fn finish(self) -> Free {
        let mut syms: Vec<SymId> = Vec::new();
        for s in self.referenced {
            if !self.declared.contains(&s) && !syms.contains(&s) {
                syms.push(s);
            }
        }
        Free { syms, this: self.this }
    }

    fn merge(&mut self, free: &Free) {
        self.referenced.extend_from_slice(&free.syms);
        self.this |= free.this;
    }

    fn list(&mut self, l: ListRef) {
        let prog = self.cx.input.prog;
        for &e in prog.expr_list(l) {
            self.expr(e);
        }
    }

    fn local(&mut self, s: SymId) {
        let info = self.cx.input.syms.sym(s);
        if info.owner != Owner::Local {
            return;
        }
        self.referenced.push(s);
        if info.mods & mods::LAZY != 0 && info.kind != SymKind::Def {
            let free = self.an.of_lazy(self.cx, s);
            self.merge(&free);
        }
    }

    fn expr(&mut self, e: TExprId) {
        let prog = self.cx.input.prog;
        match prog.expr(e) {
            TExpr::Int(_) | TExpr::Long(_) | TExpr::Double(_) | TExpr::Bool(_) | TExpr::Char(_) | TExpr::Str(_)
            | TExpr::Unit | TExpr::Static(_) | TExpr::Module(_) | TExpr::ClassOf(_) | TExpr::JsImport(_) | TExpr::JsGlobal(..) => {}
            TExpr::Local(s) => self.local(s),
            TExpr::This | TExpr::Super(_) => self.this = true,
            TExpr::Field(r, _) | TExpr::Unary(_, r) | TExpr::ToStr(r, _) | TExpr::TypeTest(r, _) | TExpr::Cast(r, ..) | TExpr::Index(r, _)
            | TExpr::Spread(r) | TExpr::JsSelect(r, _) | TExpr::Return(r) => self.expr(r),
            TExpr::CallStatic(s, args) => {
                let info = self.cx.input.syms.sym(s);
                if info.owner == Owner::Local {
                    if info.kind == SymKind::Def {
                        let free = self.an.of_fun(self.cx, s);
                        self.merge(&free);
                    } else {
                        self.local(s);
                    }
                }
                self.list(args);
            }
            TExpr::CallMethod(r, _, args) | TExpr::CallClosure(r, args) => {
                self.expr(r);
                self.list(args);
            }
            TExpr::New(_, args) | TExpr::NewVia(_, args) | TExpr::StrConcat(args) | TExpr::Js(_, args) | TExpr::SeqLit(args) | TExpr::ArrayLit(args)
            | TExpr::ObjLit(args) => self.list(args),
            TExpr::Lambda(params, body) => {
                self.declared.extend_from_slice(prog.sym_list(params));
                self.expr(body);
            }
            TExpr::If(c, t, els) => {
                self.expr(c);
                self.expr(t);
                if let Some(x) = els {
                    self.expr(x);
                }
            }
            TExpr::While(a, b) | TExpr::Assign(a, b) | TExpr::Prim(_, a, b) => {
                self.expr(a);
                self.expr(b);
            }
            TExpr::Block(stmts, res) => {
                for s in &prog.stmts[stmts.range()] {
                    match *s {
                        TStmt::Expr(x) => self.expr(x),
                        TStmt::Val(sym, init) => {
                            self.declared.push(sym);
                            if self.cx.input.syms.sym(sym).mods & mods::LAZY != 0 {
                                self.an.lazy_inits.insert(sym, init);
                            } else {
                                self.expr(init);
                            }
                        }
                        TStmt::Fun(f) => self.declared.push(prog.funs[f.idx()].sym),
                        TStmt::Pat(p, init) => {
                            self.expr(init);
                            self.pat(p);
                        }
                    }
                }
                self.expr(res);
            }
            TExpr::Match(scrut, cases) => {
                self.expr(scrut);
                self.cases(cases);
            }
            TExpr::Null => {}
            TExpr::Throw(inner, _) => self.expr(inner),
            TExpr::Try(i) => {
                let t = &prog.tries[i as usize];
                self.expr(t.body);
                self.cases(t.cases);
                if let Some(f) = t.finalizer {
                    self.expr(f);
                }
            }
            TExpr::Splice(_) => unreachable!("a stored inline body is emitted nowhere"),
        }
    }

    fn cases(&mut self, cases: ListRef) {
        let prog = self.cx.input.prog;
        for c in &prog.cases[cases.range()] {
            self.pat(c.pat);
            if let Some(g) = c.guard {
                self.expr(g);
            }
            self.expr(c.body);
        }
    }

    /// The values a test compares with (a path's, an outer's), which may read locals.
    fn test(&mut self, t: crate::tir::TestId) {
        let prog = self.cx.input.prog;
        match prog.tests[t.idx()] {
            crate::tir::TypeTest::Value(e) => self.expr(e),
            crate::tir::TypeTest::Or(a, b) | crate::tir::TypeTest::And(a, b) => {
                self.test(a);
                self.test(b);
            }
            crate::tir::TypeTest::Outer(_, inner) => self.test(inner),
            _ => {}
        }
    }

    fn pat(&mut self, p: TPatId) {
        let prog = self.cx.input.prog;
        match prog.pats[p.idx()] {
            TPat::Wildcard => {}
            TPat::Bind(s, inner) => {
                self.declared.push(s);
                if let Some(i) = inner {
                    self.pat(i);
                }
            }
            TPat::Test(test, _, inner) => {
                self.test(test);
                self.pat(inner);
            }
            TPat::Equals(e, _) => self.expr(e),
            TPat::Unapply(s, call, inner) => {
                self.declared.push(s);
                self.expr(call);
                self.pat(inner);
            }
            TPat::Class(_, _, _, subs) | TPat::Alt(subs) => {
                for &s in &prog.pat_lists[subs.range()] {
                    self.pat(s);
                }
            }
            TPat::Seq(items, rest) => {
                for &s in prog.pat_lists[items.range()].iter().chain(rest.iter()) {
                    self.pat(s);
                }
            }
        }
    }
}

/// The locals declared by a method body that live in a cell: a `var` that a closure, a local
/// def or an anonymous class refers to, and a `val` that a closure refers to before it is set
/// (`val f: Int => Int = n => f(n - 1)`). The initialisers of the lazy vals are noted on the way.
pub fn cells(cx: &Cx, an: &mut Analysis, params: &[SymId], roots: &[TExprId]) -> Vec<SymId> {
    let mut w = CellWalk { cx, an, declared: params.to_vec(), nested: Vec::new(), early: Vec::new(), depth: 0 };
    for &r in roots {
        w.expr(r);
    }
    let syms = cx.input.syms;
    let mut out = Vec::new();
    for &s in &w.declared {
        let is_var = syms.sym(s).kind == SymKind::Var;
        if (is_var && w.nested.contains(&s) || w.early.contains(&s)) && !out.contains(&s) {
            out.push(s);
        }
    }
    out
}

struct CellWalk<'a, 'c> {
    cx: &'a Cx<'c>,
    an: &'a mut Analysis,
    /// Declared by the method itself, outside every closure.
    declared: Vec<SymId>,
    nested: Vec<SymId>,
    early: Vec<SymId>,
    depth: u32,
}

impl CellWalk<'_, '_> {
    fn capture_count(&self, c: ClassId) -> usize {
        let cx = self.cx;
        let idx = cx.tclass_of[c.idx()];
        if idx == u32::MAX {
            return 0;
        }
        let tc = &cx.input.prog.classes[idx as usize];
        if cx.input.syms.class(c).kind == ClassKind::Anon { tc.ctor_params.len() } else { tc.captures }
    }

    fn reference(&mut self, s: SymId) {
        if self.depth > 0 && self.cx.input.syms.sym(s).owner == Owner::Local {
            self.nested.push(s);
            if !self.declared.contains(&s) {
                self.early.push(s);
            }
        }
    }

    fn list(&mut self, l: ListRef) {
        let prog = self.cx.input.prog;
        for &e in prog.expr_list(l) {
            self.expr(e);
        }
    }

    fn nested(&mut self, e: TExprId) {
        self.depth += 1;
        self.expr(e);
        self.depth -= 1;
    }

    fn expr(&mut self, e: TExprId) {
        let prog = self.cx.input.prog;
        match prog.expr(e) {
            TExpr::Int(_) | TExpr::Long(_) | TExpr::Double(_) | TExpr::Bool(_) | TExpr::Char(_) | TExpr::Str(_)
            | TExpr::Unit | TExpr::Static(_) | TExpr::Module(_) | TExpr::ClassOf(_) | TExpr::JsImport(_) | TExpr::JsGlobal(..) | TExpr::This
            | TExpr::Super(_) => {}
            TExpr::Local(s) => self.reference(s),
            TExpr::Field(r, _) | TExpr::Unary(_, r) | TExpr::ToStr(r, _) | TExpr::TypeTest(r, _) | TExpr::Cast(r, ..) | TExpr::Index(r, _)
            | TExpr::Spread(r) | TExpr::JsSelect(r, _) | TExpr::Return(r) => self.expr(r),
            TExpr::CallStatic(s, args) => {
                self.reference(s);
                self.list(args);
            }
            TExpr::CallMethod(r, _, args) | TExpr::CallClosure(r, args) => {
                self.expr(r);
                self.list(args);
            }
            TExpr::New(c, args) => {
                // What a lifted class captures it shares with the scope it stands in.
                let items = self.cx.input.prog.expr_list(args).to_vec();
                let captures = self.capture_count(c);
                self.depth += 1;
                for &a in &items[..captures] {
                    self.expr(a);
                }
                self.depth -= 1;
                for &a in &items[captures..] {
                    self.expr(a);
                }
            }
            TExpr::NewVia(_, args) | TExpr::StrConcat(args) | TExpr::Js(_, args) | TExpr::SeqLit(args) | TExpr::ArrayLit(args) | TExpr::ObjLit(args) => {
                self.list(args)
            }
            TExpr::Lambda(_, body) => self.nested(body),
            TExpr::If(c, t, els) => {
                self.expr(c);
                self.expr(t);
                if let Some(x) = els {
                    self.expr(x);
                }
            }
            TExpr::While(a, b) | TExpr::Assign(a, b) | TExpr::Prim(_, a, b) => {
                self.expr(a);
                self.expr(b);
            }
            TExpr::Block(stmts, res) => {
                for s in &prog.stmts[stmts.range()] {
                    match *s {
                        TStmt::Expr(x) => self.expr(x),
                        TStmt::Val(sym, init) => {
                            if self.cx.input.syms.sym(sym).mods & mods::LAZY != 0 {
                                self.an.lazy_inits.insert(sym, init);
                                self.nested(init);
                            } else {
                                self.expr(init);
                            }
                            if self.depth == 0 {
                                self.declared.push(sym);
                            }
                        }
                        TStmt::Fun(f) => {
                            let fun = &prog.funs[f.idx()];
                            for &d in fun.defaults.iter().flatten() {
                                self.nested(d);
                            }
                            if let Some(body) = fun.body {
                                self.nested(body);
                            }
                        }
                        TStmt::Pat(p, init) => {
                            self.expr(init);
                            self.pat(p);
                        }
                    }
                }
                self.expr(res);
            }
            TExpr::Match(scrut, cases) => {
                self.expr(scrut);
                self.cases(cases);
            }
            TExpr::Null => {}
            TExpr::Throw(inner, _) => self.expr(inner),
            TExpr::Try(i) => {
                let t = &prog.tries[i as usize];
                self.expr(t.body);
                self.cases(t.cases);
                if let Some(f) = t.finalizer {
                    self.expr(f);
                }
            }
            TExpr::Splice(_) => unreachable!("a stored inline body is emitted nowhere"),
        }
    }

    fn cases(&mut self, cases: ListRef) {
        let prog = self.cx.input.prog;
        for c in &prog.cases[cases.range()] {
            self.pat(c.pat);
            if let Some(g) = c.guard {
                self.expr(g);
            }
            self.expr(c.body);
        }
    }

    /// The values a test compares with (a path's, an outer's), which may read locals.
    fn test(&mut self, t: crate::tir::TestId) {
        let prog = self.cx.input.prog;
        match prog.tests[t.idx()] {
            crate::tir::TypeTest::Value(e) => self.expr(e),
            crate::tir::TypeTest::Or(a, b) | crate::tir::TypeTest::And(a, b) => {
                self.test(a);
                self.test(b);
            }
            crate::tir::TypeTest::Outer(_, inner) => self.test(inner),
            _ => {}
        }
    }

    fn pat(&mut self, p: TPatId) {
        let prog = self.cx.input.prog;
        match prog.pats[p.idx()] {
            TPat::Wildcard => {}
            TPat::Bind(s, inner) => {
                if self.depth == 0 {
                    self.declared.push(s);
                }
                if let Some(i) = inner {
                    self.pat(i);
                }
            }
            TPat::Test(test, _, inner) => {
                self.test(test);
                self.pat(inner);
            }
            TPat::Equals(e, _) => self.expr(e),
            TPat::Unapply(s, call, inner) => {
                if self.depth == 0 {
                    self.declared.push(s);
                }
                self.expr(call);
                self.pat(inner);
            }
            TPat::Class(_, _, _, subs) | TPat::Alt(subs) => {
                for &s in &prog.pat_lists[subs.range()] {
                    self.pat(s);
                }
            }
            TPat::Seq(items, rest) => {
                for &s in prog.pat_lists[items.range()].iter().chain(rest.iter()) {
                    self.pat(s);
                }
            }
        }
    }
}
