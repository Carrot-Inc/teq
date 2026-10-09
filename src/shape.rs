//! Whether an edit changed only bodies. The new AST of a file is compared with the old one at
//! signature level: the package and imports, and every definition outside a body with its kind,
//! name, modifiers, annotations, type parameters, parameter lists, declared types, parents,
//! exports and, for objects, classes and givens, its members in turn. The right-hand sides of
//! vals and defs and the statements of a template body may differ. When nothing else does, the
//! definitions of the two ASTs pair up, and the pairing tells the typer where each of its symbols
//! now stands (`typer::incremental`).

use crate::ast::*;
use crate::intern::{FxMap, Interner, Name};
use crate::source::Span;

#[derive(Default)]
pub struct Remap {
    /// Old definition → new definition, for every definition outside a body.
    pub defs: FxMap<DefId, DefId>,
    /// Old offset → new span, for those definitions and their parameters.
    pub spans: FxMap<u32, Span>,
    /// With `compare_moving`: old span → new span of what the two ASTs pair outside bodies
    /// (the definitions and parameters, the type expressions and their names, the import and
    /// export clauses and their names), which the language server's index moves its records
    /// by and a retype the diagnostics it keeps. By the whole span: a type application and
    /// its head start at one offset, as an import and its first name do, and what the
    /// comparison lets through inside one (spaces, a line break, a comment, backticks) moves
    /// its end.
    pub moved: FxMap<Span, Span>,
    /// Old → new of the `T @uncheckedVariance` outside bodies, which the typer keeps what each
    /// resolved to by.
    pub unchecked: FxMap<TyExprId, TyExprId>,
}

/// The pairing of the definitions of `old` and `new`, or what differs beyond bodies.
pub fn compare(old: &Ast, new: &Ast, interner: &Interner) -> Result<Remap, String> {
    let mut cmp = Cmp { old, new, interner, remap: Remap::default(), moving: false };
    cmp.top()?;
    Ok(cmp.remap)
}

/// `compare` with `Remap::moved` filled in.
pub fn compare_moving(old: &Ast, new: &Ast, interner: &Interner) -> Result<Remap, String> {
    let mut cmp = Cmp { old, new, interner, remap: Remap::default(), moving: true };
    cmp.top()?;
    Ok(cmp.remap)
}

struct Cmp<'a> {
    old: &'a Ast,
    new: &'a Ast,
    interner: &'a Interner,
    remap: Remap,
    moving: bool,
}

type Check = Result<(), String>;

fn differ<T>(what: &str) -> Result<T, String> {
    Err(what.to_string())
}

impl<'a> Cmp<'a> {
    fn name(&self, n: Name) -> &'a str {
        self.interner.get(n)
    }

    fn top(&mut self) -> Check {
        let (o, n) = (self.old, self.new);
        if o.package != n.package || o.package_clauses != n.package_clauses {
            return differ("package clause");
        }
        if !self.imports_eq(&o.imports, &n.imports) {
            return differ("imports");
        }
        if !self.imports_eq(&o.top_exports, &n.top_exports) {
            return differ("export clauses");
        }
        if o.top_level.len() != n.top_level.len() {
            return differ("top-level definitions added or removed");
        }
        for (&a, &b) in o.top_level.iter().zip(&n.top_level) {
            self.def_sig(a, b, true)?;
        }
        Ok(())
    }

    /// The two definitions agree outside their bodies. With `record` the pairing is kept, which
    /// is the case for the definitions the typer holds symbols for; the definitions inside an
    /// expression compared as a whole are typed afresh with it.
    fn def_sig(&mut self, a: DefId, b: DefId, record: bool) -> Check {
        let (old_ast, new_ast) = (self.old, self.new);
        let (da, db) = (old_ast.def(a), new_ast.def(b));
        let what = |t: &Self, detail: &str| Err(format!("{}: {}", t.name(da.name), detail));
        if da.name != db.name {
            return Err(format!("{} renamed to {}", self.name(da.name), self.name(db.name)));
        }
        if da.mods != db.mods || self.scope_of(self.old, da.span.start) != self.scope_of(self.new, db.span.start) {
            return what(self, "modifiers changed");
        }
        if !self.annots_eq(&da.annots, &db.annots) {
            return what(self, "annotations changed");
        }
        match (&da.kind, &db.kind) {
            (DefKind::Val { pat: pa, ty: ta, rhs: ra }, DefKind::Val { pat: pb, ty: tb, rhs: rb }) => {
                if !self.opt(pa, pb, |t, x, y| t.pat_eq(x, y)) || !self.opt(ta, tb, |t, x, y| t.ty_eq(x, y)) {
                    return what(self, "declared type changed");
                }
                if ra.is_some() != rb.is_some() {
                    return what(self, "initialiser added or removed");
                }
            }
            (DefKind::Fun(fa), DefKind::Fun(fb)) => {
                if !self.tparams_eq(&fa.tparams, &fb.tparams) || !self.clauses_eq(&fa.clauses, &fb.clauses, record) {
                    return what(self, "parameters changed");
                }
                if !self.opt(&fa.ret, &fb.ret, |t, x, y| t.ty_eq(x, y)) {
                    return what(self, "result type changed");
                }
                if fa.body.is_some() != fb.body.is_some() {
                    return what(self, "body added or removed");
                }
                if (fa.ext_tparams, fa.ext_clauses, fa.is_extension, fa.ext_group)
                    != (fb.ext_tparams, fb.ext_clauses, fb.is_extension, fb.ext_group)
                {
                    return what(self, "extension clause changed");
                }
            }
            (DefKind::Class(ca), DefKind::Class(cb)) => {
                if ca.kind != cb.kind {
                    return what(self, "kind changed");
                }
                if !self.tparams_eq(&ca.tparams, &cb.tparams) || !self.clauses_eq(&ca.clauses, &cb.clauses, record) {
                    return what(self, "parameters changed");
                }
                if !self.parents_eq(&ca.parents, &cb.parents) {
                    return what(self, "parents changed");
                }
                if !self.opt(&ca.self_type, &cb.self_type, |t, x, y| t.ty_eq(x, y)) {
                    return what(self, "self type changed");
                }
                if ca.self_alias != cb.self_alias {
                    return what(self, "self alias changed");
                }
                if !self.imports_eq(&old_ast.exports[ca.exports.range()], &new_ast.exports[cb.exports.range()]) {
                    return what(self, "export clauses changed");
                }
                self.body_sig(&ca.body, &cb.body, record).map_err(|e| format!("{}.{}", self.name(da.name), e))?;
            }
            (
                DefKind::TypeAlias { tparams: pa, rhs: ra, lower: la, upper: ua },
                DefKind::TypeAlias { tparams: pb, rhs: rb, lower: lb, upper: ub },
            ) => {
                let same = self.tparams_eq(pa, pb)
                    && self.opt(ra, rb, |t, x, y| t.ty_eq(x, y))
                    && self.opt(la, lb, |t, x, y| t.ty_eq(x, y))
                    && self.opt(ua, ub, |t, x, y| t.ty_eq(x, y));
                if !same {
                    return what(self, "type changed");
                }
            }
            (DefKind::Given(ga), DefKind::Given(gb)) => {
                if !self.tparams_eq(&ga.tparams, &gb.tparams) || !self.clauses_eq(&ga.clauses, &gb.clauses, record) {
                    return what(self, "parameters changed");
                }
                if !self.ty_eq(ga.ty, gb.ty) {
                    return what(self, "type changed");
                }
                if ga.alias.is_some() != gb.alias.is_some() {
                    return what(self, "alias changed to a structural given or back");
                }
                self.body_sig(&ga.body, &gb.body, record).map_err(|e| format!("{}.{}", self.name(da.name), e))?;
            }
            _ => return what(self, "kind changed"),
        }
        if record {
            self.remap.defs.insert(a, b);
            self.remap.spans.insert(da.span.start, db.span);
            if self.moving {
                self.remap.moved.insert(da.span, db.span);
            }
        }
        Ok(())
    }

    /// The statements of a template body: the same kinds in the same order, the definitions
    /// agreeing outside their bodies; an expression statement may differ.
    fn body_sig(&mut self, a: &[Stmt], b: &[Stmt], record: bool) -> Check {
        if a.len() != b.len() {
            return differ("members added or removed");
        }
        for (x, y) in a.iter().zip(b) {
            match (x, y) {
                (Stmt::Def(p), Stmt::Def(q)) => self.def_sig(*p, *q, record)?,
                (Stmt::Import(p), Stmt::Import(q)) => {
                    let (old_ast, new_ast) = (self.old, self.new);
                    if !self.imports_eq(old_ast.import_stmt(*p), new_ast.import_stmt(*q)) {
                        return differ("imports changed");
                    }
                }
                (Stmt::Expr(_), Stmt::Expr(_)) => {}
                _ => return differ("members reordered"),
            }
        }
        Ok(())
    }

    /// The `private[scope]` of a definition or parameter starting at `at`.
    fn scope_of(&self, ast: &Ast, at: u32) -> Option<Name> {
        ast.access_scopes.iter().find(|(start, _)| *start == at).map(|&(_, n)| n)
    }

    fn opt<T: Copy>(&mut self, a: &Option<T>, b: &Option<T>, eq: impl FnOnce(&mut Self, T, T) -> bool) -> bool {
        match (a, b) {
            (None, None) => true,
            (Some(x), Some(y)) => eq(self, *x, *y),
            _ => false,
        }
    }

    fn annots_eq(&mut self, a: &[Annot], b: &[Annot]) -> bool {
        a.len() == b.len()
            && a.iter().zip(b).all(|(x, y)| {
                x.name == y.name
                    && self.strs_eq(x.args, y.args)
                    && self.expr_eq(x.instance, y.instance)
            })
    }

    fn strs_eq(&self, a: ListRef, b: ListRef) -> bool {
        let (sa, sb) = (&self.old.str_lists[a.range()], &self.new.str_lists[b.range()]);
        sa.len() == sb.len() && sa.iter().zip(sb).all(|(&x, &y)| self.old.str(x) == self.new.str(y))
    }

    fn imports_eq(&mut self, a: &[Import], b: &[Import]) -> bool {
        let same = a.len() == b.len()
            && a.iter().zip(b).all(|(x, y)| {
                x.path == y.path
                    && match (&x.sel, &y.sel) {
                        (ImportSel::Wildcard, ImportSel::Wildcard) => true,
                        (ImportSel::Given, ImportSel::Given) => match (x.bound, y.bound) {
                            (None, None) => true,
                            (Some(a), Some(b)) => self.ty_eq(a, b),
                            _ => false,
                        },
                        (ImportSel::Name(n, an), ImportSel::Name(m, am)) => n == m && an == am,
                        _ => false,
                    }
            });
        if same && self.moving {
            let names = |ast: &'a Ast, span: Span| ast.import_names.iter().find(|(s, _)| *s == span).map(|(_, n)| n.as_slice()).unwrap_or(&[]);
            for (x, y) in a.iter().zip(b) {
                self.remap.moved.insert(x.span, y.span);
                for (p, q) in names(self.old, x.span).iter().zip(names(self.new, y.span)) {
                    self.remap.moved.insert(*p, *q);
                }
            }
        }
        same
    }

    fn tparams_eq(&mut self, a: &[TypeParam], b: &[TypeParam]) -> bool {
        let same = self.tparams_eq_now(a, b);
        if same && self.moving {
            for (x, y) in a.iter().zip(b) {
                self.remap.moved.insert(x.span, y.span);
            }
        }
        same
    }

    fn tparams_eq_now(&mut self, a: &[TypeParam], b: &[TypeParam]) -> bool {
        a.len() == b.len()
            && a.iter().zip(b).all(|(x, y)| {
                x.name == y.name
                    && x.variance == y.variance
                    && x.arity == y.arity
                    && x.hk_variances == y.hk_variances
                    && self.opt(&x.upper, &y.upper, |t, p, q| t.ty_eq(p, q))
                    && self.opt(&x.lower, &y.lower, |t, p, q| t.ty_eq(p, q))
                    && x.context_bounds.len() == y.context_bounds.len()
                    && x.context_bounds.iter().zip(&y.context_bounds).all(|(&p, &q)| self.ty_eq(p, q))
                    && x.evidence_names == y.evidence_names
                    && self.annots_eq(&x.annots, &y.annots)
            })
    }

    /// Parameter lists: names, types, modifiers, annotations and whether a default exists; the
    /// default's expression is a body.
    fn clauses_eq(&mut self, a: &[ParamClause], b: &[ParamClause], record: bool) -> bool {
        if a.len() != b.len() {
            return false;
        }
        for (x, y) in a.iter().zip(b) {
            if x.is_using != y.is_using || x.is_implicit != y.is_implicit || x.params.len() != y.params.len() {
                return false;
            }
            for (p, q) in x.params.iter().zip(&y.params) {
                let same = p.name == q.name
                    && p.mods == q.mods
                    && p.default.is_some() == q.default.is_some()
                    && self.ty_eq(p.ty, q.ty)
                    && self.scope_of(self.old, p.span.start) == self.scope_of(self.new, q.span.start)
                    && self.annots_eq(self.old.param_annots(p), self.new.param_annots(q));
                if !same {
                    return false;
                }
                if record {
                    self.remap.spans.insert(p.span.start, q.span);
                    if self.moving {
                        self.remap.moved.insert(p.span, q.span);
                    }
                }
            }
        }
        true
    }

    fn parents_eq(&mut self, a: &[Parent], b: &[Parent]) -> bool {
        a.len() == b.len()
            && a.iter().zip(b).all(|(x, y)| {
                self.ty_eq(x.ty, y.ty)
                    && x.args.len() == y.args.len()
                    && x.args.iter().zip(&y.args).all(|(&(la, ua), &(lb, ub))| ua == ub && self.exprs_eq(la, lb))
            })
    }

    fn exprs_eq(&mut self, a: ListRef, b: ListRef) -> bool {
        let (la, lb) = (self.old.expr_list(a), self.new.expr_list(b));
        la.len() == lb.len() && la.iter().zip(lb).all(|(&x, &y)| self.expr_eq(x, y))
    }

    fn tys_eq(&mut self, a: ListRef, b: ListRef) -> bool {
        let (la, lb) = (self.old.ty_list(a), self.new.ty_list(b));
        la.len() == lb.len() && la.iter().zip(lb).all(|(&x, &y)| self.ty_eq(x, y))
    }

    fn pats_eq(&mut self, a: ListRef, b: ListRef) -> bool {
        let (la, lb) = (self.old.pat_list(a), self.new.pat_list(b));
        la.len() == lb.len() && la.iter().zip(lb).all(|(&x, &y)| self.pat_eq(x, y))
    }

    fn names_eq(&self, a: ListRef, b: ListRef) -> bool {
        self.old.name_lists[a.range()] == self.new.name_lists[b.range()]
    }

    fn opt_expr_eq(&mut self, a: Option<ExprId>, b: Option<ExprId>) -> bool {
        self.opt(&a, &b, |t, x, y| t.expr_eq(x, y))
    }

    fn expr_eq(&mut self, a: ExprId, b: ExprId) -> bool {
        use Expr::*;
        match (self.old.expr(a), self.new.expr(b)) {
            (IntLit(x), IntLit(y)) | (LongLit(x), LongLit(y)) => x == y,
            (DoubleLit(x), DoubleLit(y)) => x.to_bits() == y.to_bits(),
            (DecimalLit(x), DecimalLit(y)) => self.old.str(x) == self.new.str(y),
            (FloatLit(x), FloatLit(y)) => x.to_bits() == y.to_bits(),
            (BoolLit(x), BoolLit(y)) => x == y,
            (CharLit(x), CharLit(y)) => x == y,
            (StringLit(x), StringLit(y)) => self.old.str(x) == self.new.str(y),
            (UnitLit, UnitLit) | (This, This) | (Derived, Derived) | (Error, Error) => true,
            (Ident(x), Ident(y)) | (Super(x), Super(y)) => x == y,
            (Select(e, n), Select(f, m)) => n == m && self.expr_eq(e, f),
            (Apply(e, l), Apply(f, k)) | (UsingApply(e, l), UsingApply(f, k)) => self.expr_eq(e, f) && self.exprs_eq(l, k),
            (NamedArg(n, e), NamedArg(m, f)) => n == m && self.expr_eq(e, f),
            (TypeApply(e, l), TypeApply(f, k)) => self.expr_eq(e, f) && self.tys_eq(l, k),
            (Infix(e1, n, e2), Infix(f1, m, f2)) => n == m && self.expr_eq(e1, f1) && self.expr_eq(e2, f2),
            (Prefix(n, e), Prefix(m, f)) => n == m && self.expr_eq(e, f),
            (PolyLambda(l, e), PolyLambda(k, f)) => self.names_eq(l, k) && self.expr_eq(e, f),
            (Lambda(pl, e), Lambda(ql, f)) => {
                let (pa, pb) = (&self.old.lambda_params[pl.range()], &self.new.lambda_params[ql.range()]);
                pa.len() == pb.len()
                    && pa.iter().zip(pb).all(|(p, q)| p.name == q.name && self.opt(&p.ty, &q.ty, |t, x, y| t.ty_eq(x, y)))
                    && self.expr_eq(e, f)
            }
            (If(c1, t1, e1), If(c2, t2, e2)) => self.expr_eq(c1, c2) && self.expr_eq(t1, t2) && self.opt_expr_eq(e1, e2),
            (Match(s1, l), Match(s2, k)) => self.expr_eq(s1, s2) && self.cases_eq(l, k),
            (Block(l), Block(k)) => {
                let (sa, sb) = (self.old.stmt_list(l), self.new.stmt_list(k));
                sa.len() == sb.len() && sa.iter().zip(sb).all(|(&x, &y)| self.stmt_eq(x, y))
            }
            (While(c1, b1), While(c2, b2)) => self.expr_eq(c1, c2) && self.expr_eq(b1, b2),
            (For(l, e, y1), For(k, f, y2)) => {
                let (ea, eb) = (&self.old.enumerators[l.range()], &self.new.enumerators[k.range()]);
                y1 == y2
                    && ea.len() == eb.len()
                    && ea.iter().zip(eb).all(|(x, y)| match (x, y) {
                        (Enumerator::Gen(p, e), Enumerator::Gen(q, f))
                        | (Enumerator::CaseGen(p, e), Enumerator::CaseGen(q, f))
                        | (Enumerator::Val(p, e), Enumerator::Val(q, f)) => self.pat_eq(*p, *q) && self.expr_eq(*e, *f),
                        (Enumerator::Guard(e), Enumerator::Guard(f)) => self.expr_eq(*e, *f),
                        _ => false,
                    })
                    && self.expr_eq(e, f)
            }
            (Assign(t1, v1), Assign(t2, v2)) => self.expr_eq(t1, t2) && self.expr_eq(v1, v2),
            (Tuple(l), Tuple(k)) => self.exprs_eq(l, k),
            (Parens(e), Parens(f)) | (Unchecked(e), Unchecked(f)) => self.expr_eq(e, f),
            (Typed(e, t), Typed(f, u)) => self.expr_eq(e, f) && self.ty_eq(t, u),
            (New(t, l), New(u, k)) => self.ty_eq(t, u) && self.exprs_eq(l, k),
            (NewAnon(d), NewAnon(e)) => self.def_full(d, e),
            (Interp(n, s, l), Interp(m, t, k)) => n == m && self.strs_eq(s, t) && self.exprs_eq(l, k),
            (NullLit, NullLit) => true,
            (Throw(e), Throw(f)) => self.expr_eq(e, f),
            (Try(i), Try(j)) => {
                let (x, y) = (self.old.try_expr(i), self.new.try_expr(j));
                self.expr_eq(x.body, y.body)
                    && self.cases_eq(x.cases, y.cases)
                    && self.opt_expr_eq(x.handler, y.handler)
                    && self.opt_expr_eq(x.finalizer, y.finalizer)
            }
            _ => false,
        }
    }

    fn cases_eq(&mut self, l: ListRef, k: ListRef) -> bool {
        let (ca, cb) = (self.old.case_list(l), self.new.case_list(k));
        ca.len() == cb.len()
            && ca.iter().zip(cb).all(|(x, y)| {
                self.pat_eq(x.pat, y.pat) && self.opt_expr_eq(x.guard, y.guard) && self.expr_eq(x.body, y.body)
            })
    }

    fn stmt_eq(&mut self, a: Stmt, b: Stmt) -> bool {
        match (a, b) {
            (Stmt::Expr(e), Stmt::Expr(f)) => self.expr_eq(e, f),
            (Stmt::Def(d), Stmt::Def(e)) => self.def_full(d, e),
            (Stmt::Import(i), Stmt::Import(j)) => {
                let (old_ast, new_ast) = (self.old, self.new);
                self.imports_eq(old_ast.import_stmt(i), new_ast.import_stmt(j))
            }
            _ => false,
        }
    }

    /// Two definitions inside an expression: the same in every respect, bodies included.
    fn def_full(&mut self, a: DefId, b: DefId) -> bool {
        if self.def_sig(a, b, false).is_err() {
            return false;
        }
        let (da, db) = (self.old.def(a), self.new.def(b));
        match (&da.kind, &db.kind) {
            (DefKind::Val { rhs: ra, .. }, DefKind::Val { rhs: rb, .. }) => self.opt_expr_eq(*ra, *rb),
            (DefKind::Fun(fa), DefKind::Fun(fb)) => {
                self.opt_expr_eq(fa.body, fb.body) && self.defaults_eq(&fa.clauses, &fb.clauses)
            }
            (DefKind::Class(ca), DefKind::Class(cb)) => {
                self.defaults_eq(&ca.clauses, &cb.clauses) && self.stmts_full(&ca.body, &cb.body)
            }
            (DefKind::Given(ga), DefKind::Given(gb)) => {
                self.opt_expr_eq(ga.alias, gb.alias)
                    && self.defaults_eq(&ga.clauses, &gb.clauses)
                    && self.stmts_full(&ga.body, &gb.body)
            }
            (DefKind::TypeAlias { .. }, DefKind::TypeAlias { .. }) => true,
            _ => false,
        }
    }

    fn defaults_eq(&mut self, a: &[ParamClause], b: &[ParamClause]) -> bool {
        let da = a.iter().flat_map(|c| c.params.iter().map(|p| p.default));
        let db = b.iter().flat_map(|c| c.params.iter().map(|p| p.default));
        let (da, db): (Vec<_>, Vec<_>) = (da.collect(), db.collect());
        da.len() == db.len() && da.into_iter().zip(db).all(|(x, y)| self.opt_expr_eq(x, y))
    }

    fn stmts_full(&mut self, a: &[Stmt], b: &[Stmt]) -> bool {
        a.len() == b.len() && a.iter().zip(b).all(|(&x, &y)| self.stmt_eq(x, y))
    }

    fn pat_eq(&mut self, a: PatId, b: PatId) -> bool {
        use Pat::*;
        match (self.old.pat(a), self.new.pat(b)) {
            (Wildcard, Wildcard) | (Error, Error) => true,
            (Bind(n, p), Bind(m, q)) => n == m && self.opt(&p, &q, |t, x, y| t.pat_eq(x, y)),
            (Typed(p, t), Typed(q, u)) => self.pat_eq(p, q) && self.ty_eq(t, u),
            (Lit(e), Lit(f)) | (StableId(e), StableId(f)) => self.expr_eq(e, f),
            (Ctor(e, l), Ctor(f, k)) => self.expr_eq(e, f) && self.pats_eq(l, k),
            (Tuple(l), Tuple(k)) | (Alt(l), Alt(k)) => self.pats_eq(l, k),
            (Rest(p), Rest(q)) => self.pat_eq(p, q),
            _ => false,
        }
    }

    fn ty_eq(&mut self, a: TyExprId, b: TyExprId) -> bool {
        let same = self.ty_eq_now(a, b);
        if same && matches!(self.old.ty(a), TyExpr::UncheckedVariance(_)) {
            self.remap.unchecked.insert(a, b);
        }
        if same && self.moving {
            let (sa, sb) = (self.old.ty_spans[a.idx()], self.new.ty_spans[b.idx()]);
            self.remap.moved.insert(sa, sb);
            if let (Some(na), Some(nb)) = (self.old.ty_name_span(a), self.new.ty_name_span(b)) {
                self.remap.moved.insert(na, nb);
            }
        }
        same
    }

    fn ty_eq_now(&mut self, a: TyExprId, b: TyExprId) -> bool {
        use TyExpr::*;
        match (self.old.ty(a), self.new.ty(b)) {
            (Name(n), Name(m)) => n == m,
            (Select(t, n), Select(u, m)) => n == m && self.ty_eq(t, u),
            (Apply(t, l), Apply(u, k)) => self.ty_eq(t, u) && self.tys_eq(l, k),
            (Fun(l, t), Fun(k, u)) | (CtxFun(l, t), CtxFun(k, u)) => {
                let names_same = match (self.old.fun_param_names(a), self.new.fun_param_names(b)) {
                    (None, None) => true,
                    (Some(n), Some(m)) => self.names_eq(n, m),
                    _ => false,
                };
                names_same && self.tys_eq(l, k) && self.ty_eq(t, u)
            }
            (Wildcard, Wildcard) | (Error, Error) => true,
            (Resolved(t), Resolved(u)) => t == u,
            (Tuple(l), Tuple(k)) => self.tys_eq(l, k),
            (NamedTuple(n, l), NamedTuple(m, k)) => self.names_eq(n, m) && self.tys_eq(l, k),
            (Union(a1, b1), Union(a2, b2)) | (Inter(a1, b1), Inter(a2, b2)) | (BoundedWildcard(a1, b1), BoundedWildcard(a2, b2)) => {
                self.ty_eq(a1, a2) && self.ty_eq(b1, b2)
            }
            (ByName(t), ByName(u))
            | (Repeated(t), Repeated(u))
            | (Singleton(t), Singleton(u))
            | (Unchecked(t), Unchecked(u))
            | (UncheckedVariance(t), UncheckedVariance(u)) => self.ty_eq(t, u),
            (Lambda(l, t), Lambda(k, u)) | (PolyFun(l, t), PolyFun(k, u)) => self.names_eq(l, k) && self.ty_eq(t, u),
            (Match(t, l), Match(u, k)) => self.ty_eq(t, u) && self.tys_eq(l, k),
            (MatchCase(p, t), MatchCase(q, u)) => self.ty_eq(p, q) && self.ty_eq(t, u),
            (TypeVar(n), TypeVar(m)) => n == m,
            (Lit(e), Lit(f)) => self.expr_eq(e, f),
            (Project(t, n), Project(u, m)) => n == m && self.ty_eq(t, u),
            (Refined(t, l), Refined(u, k)) => {
                let (ds, es) = (self.old.def_list(l).to_vec(), self.new.def_list(k).to_vec());
                self.ty_eq(t, u) && ds.len() == es.len() && ds.iter().zip(&es).all(|(&d, &e)| self.def_sig(d, e, false).is_ok())
            }
            _ => false,
        }
    }
}
