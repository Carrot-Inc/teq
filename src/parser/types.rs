use super::Parser;
use crate::ast::*;
use crate::intern::Name;
use crate::names;
use crate::source::Span;
use crate::token::{Tok, Token};

/// A polymorphic function's context bound: its binder's index, the bound, its evidence's `as` name (EMPTY where it has
/// none) and the span of the bound and its name.
pub(super) type BinderBound = (usize, TyExprId, Name, Span);

/// A clause's binders' annotations before its node is made: each the binder's index, its span (its first annotation
/// to its last bound), the start of its name, the annotation (`BinderAnnot`); and the clause's binders' names, kept
/// where one of them is annotated (`SyntaxNotes::binder_names`).
#[derive(Default, Clone)]
pub(super) struct BinderAnnotations {
    annots: Vec<(u32, Span, u32, Annot)>,
    names: Vec<Span>,
}

impl BinderAnnotations {
    pub(super) fn is_empty(&self) -> bool {
        self.annots.is_empty()
    }
}

impl<'a> Parser<'a> {
    /// The type of a parameter: `=> T` is passed unevaluated, `T*` is a sequence, and `=> T*` a
    /// sequence built when the parameter is read.
    pub(super) fn parse_param_type(&mut self) -> TyExprId {
        let start = self.span();
        let by_name = self.eat(Tok::Arrow);
        let mut ty = self.parse_type();
        if by_name {
            ty = self.ast.add_ty(TyExpr::ByName(ty), start.to(self.prev_span()));
        }
        if self.at_op(names::STAR) {
            let star = self.bump().span;
            ty = self.ast.add_ty(TyExpr::Repeated(ty), star);
        }
        ty
    }

    pub(super) fn parse_type(&mut self) -> TyExprId {
        if self.at(Tok::Indent) && self.span().start == self.span().end {
            return self.parse_indented_type();
        }
        let start = self.span();
        if self.at(Tok::Arrow) {
            self.error_at(start, "a by-name type can only be the type of a parameter");
            self.bump();
            return self.parse_type();
        }
        if self.at(Tok::LBracket) && self.bracket_followed_by(Tok::TypeLambdaArrow) {
            return self.parse_type_lambda();
        }
        if self.at(Tok::LBracket) && self.bracket_followed_by(Tok::Arrow) {
            return self.parse_poly_fun_type(start);
        }
        let mut t = self.parse_union_type();
        if self.at(Tok::KwMatch) {
            let keyword = self.bump().span;
            let cases = self.parse_type_cases();
            t = self.ast.add_ty(TyExpr::Match(t, cases), start.to(self.prev_span()));
            self.ast.notes.match_keywords.push((t, keyword));
        }
        if matches!(self.kind(), Tok::Arrow | Tok::CtxArrow) {
            let ctx = self.bump().kind == Tok::CtxArrow;
            let (params, names) = match self.ast.ty(t) {
                TyExpr::Tuple(l) => (l, None),
                TyExpr::NamedTuple(names, l) => (l, Some(names)),
                _ => (push_list(&mut self.ast.ty_lists, &[t]), None),
            };
            let ret = self.parse_type();
            let span = start.to(self.prev_span());
            let node = if ctx { TyExpr::CtxFun(params, ret) } else { TyExpr::Fun(params, ret) };
            let id = self.ast.add_ty(node, span);
            if let Some(names) = names {
                self.ast.fun_param_names.push((id, names));
            }
            return id;
        }
        t
    }

    /// A type alone in the region the lexer opens after `=` or `=>` when the type starts on the
    /// next, indented line, as scalac's `typ()` takes `INDENT Type OUTDENT`. Anything after the
    /// type in the region is reported, scalac's "unindent expected", and the region dissolved,
    /// so that what follows is parsed as the enclosing sequence's statements, as scalac goes on.
    fn parse_indented_type(&mut self) -> TyExprId {
        let indent = self.pos;
        self.bump();
        let t = self.parse_type();
        if !self.eat(Tok::Outdent) {
            let (at, found) = self.found();
            self.error_at(at, format!("expected the end of the indented type, found {}", found));
            let outdent = self.matching_outdent(indent);
            self.dissolve(outdent);
        }
        t
    }

    /// The cases of a match type, `case P => T` each, in braces or indented.
    fn parse_type_cases(&mut self) -> ListRef {
        let indented = self.eat(Tok::Indent);
        let mark = self.ty_scratch.len();
        loop {
            if indented {
                self.skip_separators();
            }
            if !self.at(Tok::KwCase) {
                if indented && !matches!(self.kind(), Tok::Outdent | Tok::Eof) {
                    let (at, found) = self.found();
                    self.error_at(at, format!("expected 'case', found {}", found));
                    self.skip_one(RecoverySite::TypeCases);
                    continue;
                }
                break;
            }
            let start = self.bump().span;
            let pat = self.within_match_pattern(Self::parse_inter_type);
            self.bind_case_variables(pat);
            let at = self.span().start;
            if !self.expect(Tok::Arrow) {
                self.case_without_arrow(start.start, at);
            }
            let body = if self.eat(Tok::Indent) {
                let body = self.parse_type();
                self.skip_separators();
                self.eat(Tok::Outdent);
                body
            } else {
                self.parse_type()
            };
            let c = self.ast.add_ty(TyExpr::MatchCase(pat, body), start.to(self.prev_span()));
            self.ty_scratch.push(c);
            if !indented {
                break;
            }
        }
        if indented {
            self.end_region();
        }
        if self.ty_scratch.len() == mark {
            self.error_at(self.span(), "expected 'case'");
        }
        self.ty_list(mark)
    }

    /// The lowercase names among the arguments of a match type pattern are the type variables
    /// the case binds.
    fn bind_case_variables(&mut self, t: TyExprId) {
        let (TyExpr::Apply(_, args) | TyExpr::Tuple(args)) = self.ast.ty(t) else { return };
        for i in 0..args.len {
            let a = self.ast.ty_lists[(args.start + i) as usize];
            match self.ast.ty(a) {
                TyExpr::Name(n) if self.interner.get(n).starts_with(|c: char| c.is_lowercase()) => {
                    self.ast.tys[a.idx()] = TyExpr::TypeVar(n);
                }
                _ => self.bind_case_variables(a),
            }
        }
    }

    /// True if the bracket group starting at the current token is directly followed by `k`.
    pub(super) fn bracket_followed_by(&self, k: Tok) -> bool {
        let mut depth = 0usize;
        let mut i = 0usize;
        loop {
            match self.kind_at(i) {
                Tok::LBracket | Tok::LParen | Tok::LBrace => depth += 1,
                Tok::RBracket | Tok::RParen | Tok::RBrace => {
                    depth -= 1;
                    if depth == 0 {
                        return self.kind_at(i + 1) == k;
                    }
                }
                Tok::Eof => return false,
                _ => {}
            }
            i += 1;
        }
    }

    /// `[X] => X => R`, a polymorphic function type: its binders, its parameters named (`name_poly_fun_params`), its
    /// evidences (`poly_fun_type_evidence`), its binders' annotations with its node.
    #[cold]
    #[inline(never)]
    fn parse_poly_fun_type(&mut self, start: Span) -> TyExprId {
        let (params, bounds, ctx, annotated) = self.parse_type_params_of_lambda();
        self.expect(Tok::Arrow);
        let mut fun = self.parse_type();
        if !matches!(self.ast.ty(fun), TyExpr::Fun(..) | TyExpr::CtxFun(..)) {
            self.error_at(start, "a polymorphic function type takes a function type after `=>`");
        } else {
            self.name_poly_fun_params(fun);
            if !ctx.is_empty() {
                fun = self.poly_fun_type_evidence(&params, &ctx, fun);
            }
        }
        let l = push_list(&mut self.ast.name_lists, &params);
        let id = self.ast.add_ty(TyExpr::PolyFun(l, fun), start.to(self.prev_span()));
        self.record_lambda_bounds(id, &bounds);
        if !annotated.is_empty() {
            self.record_binder_annots(BinderClause::PolyFun(id), annotated);
        } else {
            self.record_binder_names(BinderClause::PolyFun(id), annotated, &bounds);
        }
        id
    }

    fn parse_type_lambda(&mut self) -> TyExprId {
        let start = self.span();
        let (params, bounds, ctx, annotated) = self.parse_type_params_of_lambda();
        // `stripContextBounds("type lambdas")`: each binder's bounds refused at their span, the binder kept.
        for i in 0..params.len() {
            let spans: Vec<Span> = ctx.iter().filter(|c| c.0 == i).map(|c| c.3).collect();
            if let (Some(first), Some(last)) = (spans.first(), spans.last()) {
                self.error_at(first.to(*last), "context bounds are not allowed in type lambdas");
            }
        }
        self.expect(Tok::TypeLambdaArrow);
        let body = self.parse_type();
        let id = self.type_lambda(&params, &bounds, body, start);
        if !annotated.is_empty() {
            self.record_binder_annots(BinderClause::Lambda(id), annotated);
        } else {
            self.record_binder_names(BinderClause::Lambda(id), annotated, &bounds);
        }
        if self.syntax.keep {
            self.ast.notes.written_lambdas.push(id);
        }
        id
    }

    /// `[X, Y >: L <: U, F[Z]]` of a type lambda or a polymorphic function type or literal. A
    /// higher-kinded binder takes a clause of its own, as dotty's `typeParamClause` parses it,
    /// and is bounded by a type lambda over that clause, `[Z] =>> Any` where it declares no upper
    /// bound (dotty's `F <: [Z] =>> Any`), which carries its kind to typing.
    pub(super) fn parse_type_params_of_lambda(&mut self) -> (Vec<Name>, Vec<LambdaBounds>, Vec<BinderBound>, BinderAnnotations) {
        // The clause's `[`, by which the source-preserving mode keeps its binders (`lambdaAbstract`'s `TypeDef`s).
        let clause = self.span().start;
        self.expect(Tok::LBracket);
        let mut params = Vec::new();
        let mut bounds = Vec::new();
        let mut ctx: Vec<BinderBound> = Vec::new();
        let mut annotated = BinderAnnotations::default();
        let mut cut = false;
        loop {
            // `typeParam` under `ParamOwner.Type`: `annotsAsMods` first; no variance (`acceptsVariance` fails); the name
            // or `_` (`acceptsWildcard`); its own clause and bounds; context bounds where the version enables the new
            // givens (`acceptsCtxBounds`), which a polymorphic function desugars and a type lambda refuses, else each
            // refused as `IllegalContextBounds`.
            let binder_start = self.span().start;
            let annots = self.parse_annots();
            if self.eat_op(names::PLUS) || self.eat_op(names::MINUS) {
                self.error_at(self.prev_span(), "no `+/-` variance annotation allowed here");
            }
            let name = self.span();
            // `WildcardParamName.fresh().toTypeName`: a derived name, numbered by its place in the clause.
            if self.eat(Tok::Underscore) {
                let n = self.interner.unique("_$", params.len() as u32);
                params.push(n);
            } else {
                params.push(self.expect_ident().0);
            }
            let (inner, inner_bounds, inner_variances, inner_annotated) = if self.at(Tok::LBracket) { self.parse_hk_clause() } else { Default::default() };
            bounds.push(self.parse_kind_bound(&inner, &inner_bounds, &inner_variances, &inner_annotated));
            if self.at(Tok::Colon) {
                if self.source.enables_new_givens() {
                    let i = params.len() - 1;
                    let found = self.parse_binder_context_bounds();
                    if self.syntax.keep {
                        self.ast.notes.binder_cbounds.extend(found.iter().map(|&(b, _, s)| (name.start, b, s)));
                    }
                    ctx.extend(found.into_iter().map(|(b, n, s)| (i, b, n, s)));
                } else {
                    self.illegal_context_bounds();
                }
            }
            let binder = Span::new(binder_start, self.prev_span().end);
            annotated.annots.extend(annots.iter().map(|&a| ((params.len() - 1) as u32, binder, name.start, a)));
            annotated.names.push(name);
            if self.syntax.keep {
                self.keep_hk_param(clause, name, None, *bounds.last().unwrap());
            }
            if !self.list_continues(Tok::RBracket, &mut cut) {
                break;
            }
        }
        self.expect(Tok::RBracket);
        (params, bounds, ctx, annotated)
    }

    /// The binders' names of a clause none of whose binders is annotated, where one has a bound: the place of a cycle
    /// through the bounds (`checkNonCyclic`'s E140 at the binder).
    pub(super) fn record_binder_names(&mut self, clause: BinderClause, annotated: BinderAnnotations, bounds: &[LambdaBounds]) {
        if bounds.iter().any(|b| b.0.is_some() || b.1.is_some()) {
            self.ast.notes.binder_names.extend(annotated.names.into_iter().enumerate().map(|(i, s)| (clause, i as u32, s)));
        }
    }

    /// The annotations of a clause's binders, recorded with the clause's node once it is made, and its binders' names.
    pub(super) fn record_binder_annots(&mut self, clause: BinderClause, annotated: BinderAnnotations) {
        if annotated.annots.is_empty() {
            return;
        }
        let records = annotated.annots.into_iter().map(|(index, span, name, annot)| BinderAnnot { clause, index, span, name, annot });
        self.ast.notes.binder_annots.extend(records);
        self.ast.notes.binder_names.extend(annotated.names.into_iter().enumerate().map(|(i, s)| (clause, i as u32, s)));
    }

    /// `Parsers.contextBounds` after a binder: `: B`, `: {B1, B2}`, each bound's evidence name after `as`
    /// (`contextBound`), with the span of the bound and its name; several colons are read as the type parameters' own
    /// clause reads them, the version's migration warning with them.
    fn parse_binder_context_bounds(&mut self) -> Vec<(TyExprId, Name, Span)> {
        let mut out = Vec::new();
        let mut after_bound = false;
        while self.at(Tok::Colon) {
            let colon = self.bump().span;
            if after_bound {
                self.migration(crate::source_version::Migration::GivenSyntax, colon, "Multiple context bounds should be enclosed in `{ ... }`", crate::warnings::NO_ID);
            }
            after_bound = !self.at(Tok::LBrace);
            if self.eat(Tok::LBrace) {
                loop {
                    out.push(self.context_bound(true));
                    if !self.eat(Tok::Comma) {
                        break;
                    }
                }
                self.expect(Tok::RBrace);
            } else {
                out.push(self.context_bound(false));
            }
        }
        out
    }

    /// `Parsers.contextBound`: a bound and its `as` name where the version enables the new givens, EMPTY where it has
    /// none.
    fn context_bound(&mut self, braced: bool) -> (TyExprId, Name, Span) {
        let start = self.span();
        // `toplevelTyp(inContextBound = true)`: a type lambda among them (`[X: [Y] =>> Ord[Y]]`).
        let b = if braced || self.at(Tok::LBracket) { self.parse_type() } else { self.parse_inter_type() };
        let name = if self.source.enables_new_givens() && self.at_soft(names::AS) {
            self.bump();
            self.expect_ident().0
        } else {
            names::EMPTY
        };
        (b, name, start.to(self.prev_span()))
    }

    /// The type a polymorphic function's context bound stands for, its bound applied to its binder, `B[X]` (dotty's
    /// `ContextBoundTypeTree`, which the typer applies).
    fn evidence_type(&mut self, binder: Name, bound: TyExprId, span: Span) -> TyExprId {
        let x = self.ast.add_ty(TyExpr::Name(binder), span);
        let l = push_list(&mut self.ast.ty_lists, &[x]);
        self.ast.add_ty(TyExpr::Apply(bound, l), span)
    }

    /// The evidences' names (`desugarContextBounds`): a bound's own `as` name, else a fresh `ContextBoundParamName`,
    /// `evidence$n` numbered from 1 in the function (`freshName`), a derived name no written identifier equals
    /// (`Interner::unique`). Every one of them is what `fitEvidenceParams` asks `referencesName` for (`getBoundNames`);
    /// the context bound proxies' (`WitnessNamesAnnot`) are `Feature.modularity`'s, which teq does not read.
    fn evidence_names(&self, ctx: &[BinderBound]) -> Vec<Name> {
        let mut fresh = 0;
        ctx.iter()
            .map(|c| {
                if c.2 != names::EMPTY {
                    c.2
                } else {
                    fresh += 1;
                    self.interner.unique("evidence$", fresh)
                }
            })
            .collect()
    }

    /// `elimContextBounds` of a polymorphic function literal (`fitEvidenceParams`, `functionsOf`): the evidences
    /// parameters of a context function literal (`ContextBoundParamName`'s `evidence$n` where a bound names none),
    /// before the value parameters where one's type names one (`referencesName`), else joined into them where they are
    /// contextual, else around the body: `[X: Ord] => (x: X) => b` is `[X] => (x: X) => (evidence$1: Ord[X]) ?=> b`.
    /// An empty clause goes (`.filter(_.nonEmpty)`): `[X: Ord] => () => b` is `[X] => (evidence$1: Ord[X]) ?=> b`.
    pub(super) fn poly_lambda_evidence(&mut self, params: &[Name], ctx: &[BinderBound], lambda: ExprId) {
        let Expr::Lambda(vparams, body) = self.ast.expr(lambda) else { return };
        let named = self.evidence_names(ctx);
        let mut evidence: Vec<LambdaParam> = Vec::new();
        for (&(i, b, _, s), &name) in ctx.iter().zip(&named) {
            let ty = self.evidence_type(params[i], b, s);
            evidence.push(LambdaParam { name, span: s, ty: Some(ty), implicit: false, contextual: true, cases: false });
        }
        let own: Vec<LambdaParam> = self.ast.lambda_params[vparams.range()].to_vec();
        let span = self.ast.expr_span(lambda);
        if own.iter().any(|p| p.ty.is_some_and(|t| references_name(&self.ast, t, &named))) {
            let inner = self.ast.add_expr(Expr::Lambda(vparams, body), span);
            let l = push_list(&mut self.ast.lambda_params, &evidence);
            self.ast.exprs[lambda.idx()] = Expr::Lambda(l, inner);
        } else if own.first().is_some_and(|p| p.contextual) {
            let joined: Vec<LambdaParam> = evidence.into_iter().chain(own).collect();
            let l = push_list(&mut self.ast.lambda_params, &joined);
            self.ast.exprs[lambda.idx()] = Expr::Lambda(l, body);
        } else if own.is_empty() {
            let l = push_list(&mut self.ast.lambda_params, &evidence);
            self.ast.exprs[lambda.idx()] = Expr::Lambda(l, body);
        } else {
            let l = push_list(&mut self.ast.lambda_params, &evidence);
            let body_span = self.ast.expr_span(body);
            let ctx_lambda = self.ast.add_expr(Expr::Lambda(l, body), body_span);
            self.ast.exprs[lambda.idx()] = Expr::Lambda(vparams, ctx_lambda);
        }
    }

    /// `makePolyFunctionType`'s parameters: each one the function type does not name is `x$n` by its place
    /// (`makeSyntheticParameter(n + 1, p)`), so that the result names it and the evidences' clause keeps it.
    fn name_poly_fun_params(&mut self, fun: TyExprId) {
        let (TyExpr::Fun(l, _) | TyExpr::CtxFun(l, _)) = self.ast.ty(fun) else { return };
        if l.len == 0 || self.ast.fun_param_names(fun).is_some() {
            return;
        }
        let synthetic: Vec<Name> = (1..=l.len).map(|n| self.interner.intern(&format!("x${}", n))).collect();
        let names = push_list(&mut self.ast.name_lists, &synthetic);
        self.ast.fun_param_names.push((fun, names));
    }

    /// `elimContextBounds` of a polymorphic function type (`makePolyFunctionType`'s `apply` under
    /// `PolyFunctionApply`, `fitEvidenceParams`, `functionsOf`): the evidences a context function before the
    /// parameters where a parameter's type names one, else joined into the parameters where they are contextual,
    /// else the result's: `[X: Ord] => X => R` is `[X] => (x$1: X) => Ord[X] ?=> R`. The function's parameters are
    /// named already (`name_poly_fun_params`), and keep their names wherever they go.
    fn poly_fun_type_evidence(&mut self, params: &[Name], ctx: &[BinderBound], fun: TyExprId) -> TyExprId {
        let span = self.ast.ty_spans[fun.idx()];
        let evidence: Vec<TyExprId> = ctx.iter().map(|&(i, b, _, s)| self.evidence_type(params[i], b, s)).collect();
        let named = self.evidence_names(ctx);
        let (vparams, res, contextual) = match self.ast.ty(fun) {
            TyExpr::Fun(l, r) => (l, r, false),
            TyExpr::CtxFun(l, r) => (l, r, true),
            _ => return fun,
        };
        let vparam_types: Vec<TyExprId> = self.ast.ty_list(vparams).to_vec();
        // The evidences' names, a parameter of a dependent function type each (`fun_param_names`), so that a later
        // type names one (`ord.T`).
        let evidence_names = named.clone();
        let named_fun = |p: &mut Self, id: TyExprId, names: &[Name]| {
            let l = push_list(&mut p.ast.name_lists, names);
            p.ast.fun_param_names.push((id, l));
        };
        if vparam_types.iter().any(|&t| references_name(&self.ast, t, &named)) {
            let l = push_list(&mut self.ast.ty_lists, &evidence);
            let id = self.ast.add_ty(TyExpr::CtxFun(l, fun), span);
            named_fun(self, id, &evidence_names);
            return id;
        }
        let own_names: Vec<Name> = self.ast.fun_param_names(fun).map(|l| self.ast.name_lists[l.range()].to_vec()).unwrap_or_default();
        if contextual {
            let joined: Vec<TyExprId> = evidence.into_iter().chain(vparam_types).collect();
            let l = push_list(&mut self.ast.ty_lists, &joined);
            let id = self.ast.add_ty(TyExpr::CtxFun(l, res), span);
            let all: Vec<Name> = evidence_names.into_iter().chain(own_names).collect();
            named_fun(self, id, &all);
            return id;
        }
        let l = push_list(&mut self.ast.ty_lists, &evidence);
        let res_span = self.ast.ty_spans[res.idx()];
        let ctx_res = self.ast.add_ty(TyExpr::CtxFun(l, res), res_span);
        named_fun(self, ctx_res, &evidence_names);
        let id = self.ast.add_ty(TyExpr::Fun(vparams, ctx_res), span);
        if !own_names.is_empty() {
            named_fun(self, id, &own_names);
        }
        id
    }

    /// The bounds of a parameter that may take a clause of its own, `inner` with its bounds:
    /// `>: L` and `<: U` written over that clause, or where it writes no upper one a type lambda
    /// of the clause with `Any` as its result, the kind dotty's `lambdaAbstract(hkparams, bounds)`
    /// keeps as the bound. The clause's binders' annotations are recorded with the upper one, the
    /// parameter's own clause (`BinderClause::Own`). Each lambda keeps the clause's variances (`clause_variances`).
    fn parse_kind_bound(&mut self, inner: &[Name], inner_bounds: &[LambdaBounds], variances: &[i8], annotated: &BinderAnnotations) -> LambdaBounds {
        let lower = if self.eat(Tok::Supertype) { Some(self.parse_hk_bound(inner, inner_bounds, variances)) } else { None };
        let upper = if self.eat(Tok::Subtype) {
            Some(self.parse_hk_bound(inner, inner_bounds, variances))
        } else if inner.is_empty() {
            None
        } else {
            let start = self.span();
            let any = self.ast.add_ty(TyExpr::Resolved(crate::types::ANY), start);
            let made = self.type_lambda(inner, inner_bounds, any, start);
            self.record_clause_variances(made, variances);
            Some(made)
        };
        if let (Some(clause), false) = (upper, inner.is_empty()) {
            self.ast.notes.own_clause_bounds.push(clause);
        }
        if let (Some(clause), false) = (upper, annotated.is_empty()) {
            self.record_binder_annots(BinderClause::Own(clause), annotated.clone());
        } else if let Some(clause) = upper {
            self.record_binder_names(BinderClause::Own(clause), annotated.clone(), inner_bounds);
        }
        (lower, upper)
    }

    /// The `[_]` of a higher-kinded parameter: its own parameters' names, bounds and variances,
    /// each of them parsed as dotty's `typeParamClauseOpt(ParamOwner.Hk)` parses it, so that one
    /// taking a clause of its own (`F[G[_]]`) has that kind as its bound; their names scope over
    /// the bounds (`S[T] <: Semigroup[T]`).
    fn parse_hk_clause(&mut self) -> (Vec<Name>, Vec<LambdaBounds>, Vec<i8>, BinderAnnotations) {
        let mut inner = Vec::new();
        let mut inner_bounds = Vec::new();
        let mut hk_variances = Vec::new();
        let mut annotated = BinderAnnotations::default();
        // The clause's owner, the name before its `[`, by which the source-preserving mode keeps its parameters.
        let owner = self.prev_span().start;
        self.expect(Tok::LBracket);
        loop {
            // dotty's `typeParam` under `ParamOwner.Hk`: `annotsAsMods`, a variance, the name or `_`, its own
            // clause, its bounds (`typeAndCtxBounds`, context bounds refused: `acceptsCtxBounds` is false for `Hk`).
            // `gobbleHat`'s `^` and `typeBounds`' capture set are capture checking's (`Feature.ccEnabled`), which teq
            // does not read, so a `^` or `{` there stays a parse error.
            let binder_start = self.span().start;
            let annots = self.parse_annots();
            let mark = self.span();
            hk_variances.push(if self.eat_op(names::PLUS) {
                1
            } else if self.eat_op(names::MINUS) {
                -1
            } else {
                0
            });
            let name = self.span();
            if self.eat(Tok::Underscore) {
                let n = self.interner.unique("_$", inner.len() as u32);
                inner.push(n);
            } else {
                inner.push(self.expect_ident().0);
            }
            let (own, own_bounds, own_variances, own_annotated) = if self.at(Tok::LBracket) { self.parse_hk_clause() } else { Default::default() };
            inner_bounds.push(self.parse_kind_bound(&own, &own_bounds, &own_variances, &own_annotated));
            if self.at(Tok::Colon) {
                self.illegal_context_bounds();
            }
            let binder = Span::new(binder_start, self.prev_span().end);
            annotated.annots.extend(annots.iter().map(|&a| ((inner.len() - 1) as u32, binder, name.start, a)));
            annotated.names.push(name);
            if self.syntax.keep {
                let variance = (*hk_variances.last().unwrap() != 0).then_some(mark);
                self.keep_hk_param(owner, name, variance, *inner_bounds.last().unwrap());
            }
            if !self.eat(Tok::Comma) {
                break;
            }
        }
        self.expect(Tok::RBracket);
        if self.syntax.keep {
            self.ast.notes.hk_clauses.push((owner, self.prev_span().end));
        }
        (inner, inner_bounds, hk_variances, annotated)
    }

    /// Under `Syntax::keep`: a parameter of a higher-kinded parameter's clause where it was written
    /// (`SyntaxNotes::hk_params`), as dotty's `typeParamClause` keeps the nested clause.
    #[cold]
    #[inline(never)]
    fn keep_hk_param(&mut self, owner: u32, name: Span, variance: Option<Span>, (lower, upper): LambdaBounds) {
        self.ast.notes.hk_params.push(crate::ast::HkParam { owner, name, variance, lower, upper });
    }

    /// A colon's context bounds where none is allowed (a higher-kinded parameter's, `Parsers.typeParam`): parsed as
    /// `contextBounds` parses them, each reported as `IllegalContextBounds` is.
    #[cold]
    #[inline(never)]
    fn illegal_context_bounds(&mut self) {
        for (_, _, at) in self.parse_binder_context_bounds() {
            self.error_at(at, "Context bounds are not allowed in this position");
        }
    }

    pub(super) fn record_lambda_bounds(&mut self, id: TyExprId, bounds: &[LambdaBounds]) {
        if let Some(bl) = self.lambda_bounds_list(bounds) {
            self.ast.lambda_bounds.push((id, bl));
        }
    }

    /// The bounds of a clause's parameters as `Ast::lambda_bounds` lists them, None where no
    /// parameter has one.
    pub(super) fn lambda_bounds_list(&mut self, bounds: &[LambdaBounds]) -> Option<ListRef> {
        if bounds.iter().all(|&(lo, hi)| lo.is_none() && hi.is_none()) {
            return None;
        }
        let ids: Vec<TyExprId> = bounds.iter().flat_map(|&(lo, hi)| [lo.unwrap_or(crate::ast::NO_BOUND), hi.unwrap_or(crate::ast::NO_BOUND)]).collect();
        Some(push_list(&mut self.ast.ty_lists, &ids))
    }

    /// Replaces each `*`, `+*` and `-*` among the type arguments from `mark` on with a fresh
    /// name, returned as the parameters of the type lambda the application becomes. As in
    /// dotc, a variance marker is dropped and the variance left to be inferred.
    fn kind_projector_params(&mut self, mark: usize) -> Vec<Name> {
        let mut params = Vec::new();
        for i in mark..self.ty_scratch.len() {
            let arg = self.ty_scratch[i];
            let TyExpr::Name(n) = self.ast.ty(arg) else { continue };
            if !matches!(self.interner.get(n), "*" | "+*" | "-*") {
                continue;
            }
            let fresh = self.fresh_name("_$kp");
            self.ast.tys[arg.idx()] = TyExpr::Name(fresh);
            params.push(fresh);
        }
        params
    }

    fn type_lambda(&mut self, params: &[Name], bounds: &[LambdaBounds], body: TyExprId, start: Span) -> TyExprId {
        let l = push_list(&mut self.ast.name_lists, params);
        let id = self.ast.add_ty(TyExpr::Lambda(l, body), start.to(self.prev_span()));
        self.record_lambda_bounds(id, bounds);
        id
    }

    pub(super) fn parse_union_type(&mut self) -> TyExprId {
        self.parse_infix_type(true)
    }

    /// An infix type where `|` is no operator: a parent, a typed pattern, a match type case.
    pub(super) fn parse_inter_type(&mut self) -> TyExprId {
        self.parse_infix_type(false)
    }

    /// `A Op B` for a type operator is `Op[A, B]`, alphanumeric or symbolic (`A <:< B`); `|` and
    /// `&` make unions and intersections. Precedence and associativity are those of terms,
    /// resolved on an operator stack as scalac does, which reports mixed associativity only
    /// against the operator on top of the stack.
    fn parse_infix_type(&mut self, bar: bool) -> TyExprId {
        let mut stack: Vec<(TyExprId, Token, Span)> = Vec::new();
        let mut top = self.parse_with_type();
        // An operator ending its line continues on the next when that line starts a type (dotc's
        // `newLineOptWhenFollowing(canStartInfixTypeTokens)`): `0 | 1 |` then `2 | 3`.
        let continued = |p: &Self| p.kind_at(1) == Tok::Newline && !p.blank_line_before(1) && p.can_start_type(2);
        while self.at_infix_type_op(bar) && (self.can_start_type(1) || self.at_refinement_block(1) || continued(self)) {
            let op = self.bump();
            if self.at(Tok::Newline) {
                self.bump();
            }
            let (prec, right) = super::expr::op_precedence(self.interner.get(op.name));
            top = self.reduce_type_ops(&mut stack, top, prec, !right, op.name);
            stack.push((top, op, self.span()));
            top = self.parse_with_type();
        }
        self.reduce_type_ops(&mut stack, top, 0, true, names::EMPTY)
    }

    /// `A with B`, the deprecated intersection, an infix type's operand (dotty's `withType`: `with`
    /// binds tighter than any type operator and associates to the right). A parent's `with` is
    /// the template's (`constrApps`), and `with` before a brace or an indented region opens a body.
    #[inline]
    fn parse_with_type(&mut self) -> TyExprId {
        let lhs = self.parse_simple_type();
        if self.at(Tok::KwWith) && !self.template_type && !matches!(self.kind_at(1), Tok::LBrace | Tok::Indent | Tok::ColonEol) {
            let with = self.bump().span;
            // `DeprecatedWithOperator`, by the version where the parser stands (`withTypeRest`).
            let msg = format!("with as a type operator has been deprecated; use & instead{}", crate::source_version::rewrite_notice("This construct", "3.4-migration"));
            self.migration(crate::source_version::Migration::WithOperator, with, msg, crate::warnings::id::DEPRECATED_WITH_OPERATOR);
            let rhs = self.parse_with_type();
            let span = self.ast.ty_spans[lhs.idx()].to(self.ast.ty_spans[rhs.idx()]);
            return self.ast.add_ty(TyExpr::Inter(lhs, rhs), span);
        }
        lhs
    }

    fn reduce_type_ops(
        &mut self,
        stack: &mut Vec<(TyExprId, Token, Span)>,
        mut top: TyExprId,
        prec: u8,
        left: bool,
        op2: Name,
    ) -> TyExprId {
        if let Some(&(_, op1, at)) = stack.last() {
            let (prec1, right1) = super::expr::op_precedence(self.interner.get(op1.name));
            if prec1 == prec && right1 == left {
                let (a1, a2) = if left { ("right", "left") } else { ("left", "right") };
                let msg = format!(
                    "{} (which is {a1}-associative) and {} (which is {a2}-associative) have same precedence and may not be mixed",
                    self.interner.get(op1.name),
                    self.interner.get(op2),
                );
                self.error_at(at, msg);
            }
        }
        while let Some(&(lhs, op, _)) = stack.last() {
            let (prec1, _) = super::expr::op_precedence(self.interner.get(op.name));
            if !(prec < prec1 || left && prec == prec1) {
                break;
            }
            stack.pop();
            let span = self.ast.ty_spans[lhs.idx()].to(self.ast.ty_spans[top.idx()]);
            let node = match (op.kind, op.name) {
                (Tok::OpIdent, names::BAR) => TyExpr::Union(lhs, top),
                (Tok::OpIdent, names::AMP) => TyExpr::Inter(lhs, top),
                _ => {
                    let op_ty = self.ast.add_ty(TyExpr::Name(op.name), op.span);
                    if op.kind == Tok::Ident {
                        self.ast.notes.infix_type_ops.push(op_ty);
                    }
                    TyExpr::Apply(op_ty, push_list(&mut self.ast.ty_lists, &[lhs, top]))
                }
            };
            top = self.ast.add_ty(node, span);
        }
        top
    }

    fn at_infix_type_op(&self, bar: bool) -> bool {
        match self.kind() {
            Tok::Ident => !matches!(self.tok().name, names::DERIVES | names::AS | names::USING),
            // `T*` is a repeated parameter; `2 * 3` is the type-level product.
            Tok::OpIdent => match self.tok().name {
                names::BAR => bar,
                names::QMARK => false,
                _ => true,
            },
            _ => false,
        }
    }

    /// A refinement without a parent at token `n`: the region the lexer opens for a brace
    /// (`A & { type T = Int }`, `A & {}`), whatever it holds, as dotty's `simpleType1` reads any
    /// `{` it starts at.
    fn at_refinement_block(&self, n: usize) -> bool {
        self.toks.get(self.pos + n).is_some_and(|t| t.kind == Tok::Indent && t.span.end > t.span.start)
    }

    /// `_root_.scala.AnyRef`, the parent of a refinement written without one, which no
    /// definition in scope shadows.
    fn root_any_ref(&mut self, at: Span) -> TyExprId {
        let root = self.ast.add_ty(TyExpr::Name(names::ROOT), at);
        let scala = self.ast.add_ty(TyExpr::Select(root, names::SCALA), at);
        self.ast.add_ty(TyExpr::Select(scala, names::ANY_REF), at)
    }

    /// scalac's `canStartInfixTypeTokens`: what may follow an infix type operator.
    fn can_start_type(&self, n: usize) -> bool {
        matches!(
            self.kind_at(n),
            Tok::Ident
                | Tok::OpIdent
                | Tok::StringLit
                | Tok::IntLit
                | Tok::LongLit
                | Tok::DoubleLit
                | Tok::FloatLit
                | Tok::CharLit
                | Tok::KwTrue
                | Tok::KwFalse
                | Tok::KwNull
                | Tok::KwThis
                | Tok::KwSuper
                | Tok::Underscore
                | Tok::LParen
                | Tok::LBrace
                | Tok::At
        )
    }

    pub(super) fn parse_simple_type(&mut self) -> TyExprId {
        let start = self.span();
        let mut t = match self.kind() {
            Tok::Ident => {
                let n = self.bump().name;
                self.ast.add_ty(TyExpr::Name(n), start)
            }
            Tok::KwThis => {
                self.bump();
                self.ast.add_ty(TyExpr::Name(names::THIS), start)
            }
            // `?` or the deprecated `_`, either with bounds (dotty's `simpleType`: `typeBounds()` after
            // either token, `[>: Type] [<: Type]`, each bound once and the lower first); `_` outside
            // a match pattern is reported by the version (`MigrationVersion.WildcardType`).
            Tok::Underscore | Tok::OpIdent if self.kind() == Tok::Underscore || self.tok().name == names::QMARK => {
                let underscore = self.bump().kind == Tok::Underscore;
                if underscore && !self.in_match_pattern {
                    self.migration_with(crate::source_version::Migration::WildcardType, start, crate::warnings::NO_ID, || {
                        format!("`_` is deprecated for wildcard arguments of types: use `?` instead{}", crate::source_version::rewrite_notice("This construct", "3.4-migration"))
                    });
                }
                let lo = self.eat(Tok::Supertype).then(|| self.parse_type());
                let hi = self.eat(Tok::Subtype).then(|| self.parse_type());
                if lo.is_none() && hi.is_none() {
                    self.ast.add_ty(TyExpr::Wildcard, start)
                } else {
                    let lo = lo.unwrap_or_else(|| self.ast.add_ty(TyExpr::Resolved(crate::types::NOTHING), start));
                    let hi = hi.unwrap_or_else(|| self.ast.add_ty(TyExpr::Resolved(crate::types::ANY), start));
                    self.ast.add_ty(TyExpr::BoundedWildcard(lo, hi), start.to(self.prev_span()))
                }
            }
            Tok::StringLit | Tok::IntLit | Tok::LongLit | Tok::DoubleLit | Tok::FloatLit | Tok::CharLit | Tok::KwTrue
            | Tok::KwFalse => {
                let e = self.parse_literal();
                self.ast.add_ty(TyExpr::Lit(e), start)
            }
            Tok::OpIdent
                if self.tok().name == names::MINUS
                    && matches!(self.kind_at(1), Tok::IntLit | Tok::LongLit | Tok::DoubleLit | Tok::FloatLit) =>
            {
                let e = self.parse_prefix_expr();
                self.ast.add_ty(TyExpr::Lit(e), start.to(self.prev_span()))
            }
            Tok::OpIdent => {
                let n = self.bump().name;
                self.ast.add_ty(TyExpr::Name(n), start)
            }
            // `{ def m: T }` or `{}` alone or as an operand, a refinement of `AnyRef` (dotty's
            // `simpleType1`: any `{` there starts a `RefinedTypeTree` over an empty parent, typed
            // as `Object`, its declarations possibly none), which the lexer hands over as a block.
            Tok::Indent if !self.template_type && self.at_refinement_block(0) => {
                let members = self.refinement_members(1);
                let any_ref = self.root_any_ref(start);
                self.ast.add_ty(TyExpr::Refined(any_ref, members), start.to(self.prev_span()))
            }
            // The same after a `:` (`val y: {} = ..`), where the lexer opens no region for the
            // brace.
            Tok::LBrace if !self.template_type => {
                let members = self.parse_brace_refinement();
                let any_ref = self.root_any_ref(start);
                self.ast.add_ty(TyExpr::Refined(any_ref, members), start.to(self.prev_span()))
            }
            Tok::LParen => {
                self.bump();
                // The types inside are types wherever the parentheses stand: a `{` refines, `with`
                // intersects, as in dotty's `argTypes` (a parent's restrictions are `constrApp`'s).
                let template_type = std::mem::replace(&mut self.template_type, false);
                let mark = self.ty_scratch.len();
                // `(a: A, b: B)` is a named tuple, or the parameters of a dependent function type.
                let mut names: Vec<Name> = Vec::new();
                // `(=> A) => B`: a function type's parameter may be by-name.
                let mut by_name: Option<Span> = None;
                let mut cut = false;
                // Each named element's type and the start of its name, recorded where they are a function's parameters.
                let mut starts: Vec<(TyExprId, u32)> = Vec::new();
                while !self.at(Tok::RParen) && !self.at(Tok::Eof) {
                    let mut named_at = None;
                    if self.at(Tok::Ident) && self.kind_at(1) == Tok::Colon {
                        let tok = self.bump();
                        names.push(tok.name);
                        named_at = Some(tok.span.start);
                        self.bump();
                    }
                    let e = if self.at(Tok::Arrow) {
                        let arrow = self.bump().span;
                        by_name.get_or_insert(arrow);
                        let inner = self.parse_type();
                        self.ast.add_ty(TyExpr::ByName(inner), arrow.to(self.prev_span()))
                    } else {
                        self.parse_type()
                    };
                    self.ty_scratch.push(e);
                    if let Some(at) = named_at {
                        starts.push((e, at));
                    }
                    if !self.list_continues(Tok::RParen, &mut cut) {
                        break;
                    }
                }
                self.expect(Tok::RParen);
                self.template_type = template_type;
                let is_fun_params = matches!(self.kind(), Tok::Arrow | Tok::CtxArrow);
                if is_fun_params && !starts.is_empty() {
                    self.ast.notes.fun_param_starts.extend(starts);
                }
                if let (Some(arrow), false) = (by_name, is_fun_params) {
                    self.error_at(arrow, "a by-name type can only be the type of a parameter");
                }
                let count = self.ty_scratch.len() - mark;
                if !names.is_empty() && (!is_fun_params || names.len() == count) {
                    if names.len() != count {
                        self.error_at(start.to(self.prev_span()), "Illegal combination of named and unnamed tuple elements");
                    }
                    let l = self.ty_list(mark);
                    let nl = push_list(&mut self.ast.name_lists, &names);
                    self.ast.add_ty(TyExpr::NamedTuple(nl, l), start.to(self.prev_span()))
                } else if count == 1 && !is_fun_params {
                    let only = self.ty_scratch[mark];
                    self.ty_scratch.truncate(mark);
                    only
                } else {
                    let l = self.ty_list(mark);
                    self.ast.add_ty(TyExpr::Tuple(l), start.to(self.prev_span()))
                }
            }
            _ => {
                let (at, found) = self.found();
                self.error_at(at, format!("expected an identifier, found {}", found));
                return self.ast.add_ty(TyExpr::Error, start);
            }
        };
        loop {
            match self.kind() {
                // After type arguments a selection is no longer part of the type: `new C[T].m`.
                Tok::Dot if matches!(self.kind_at(1), Tok::Ident | Tok::OpIdent) && !(self.template_type && matches!(self.ast.ty(t), TyExpr::Apply(..))) => {
                    self.bump();
                    let n = self.bump();
                    t = self.ast.add_ty(TyExpr::Select(t, n.name), start.to(self.prev_span()));
                    self.note_ty_name(t, n.span);
                }
                Tok::Dot if self.kind_at(1) == Tok::KwType => {
                    self.bump();
                    self.bump();
                    t = self.ast.add_ty(TyExpr::Singleton(t), start.to(self.prev_span()));
                }
                Tok::Dot if self.kind_at(1) == Tok::KwThis => {
                    self.bump();
                    self.bump();
                    t = self.ast.add_ty(TyExpr::Select(t, names::THIS), start.to(self.prev_span()));
                }
                Tok::OpIdent if self.tok().name == names::HASH && self.kind_at(1) == Tok::Ident => {
                    self.bump();
                    let n = self.bump().name;
                    t = self.ast.add_ty(TyExpr::Project(t, n), start.to(self.prev_span()));
                }
                Tok::At => {
                    let annots = self.parse_inline_annots();
                    self.ast.inline_annots.extend_from_slice(&annots);
                    // `T @nowarn` covers the type, not the expression it ascribes.
                    for a in annots.iter().filter(|a| a.name == names::NOWARN) {
                        self.ast.nowarn_ascriptions.push((start.to(self.prev_span()), a.clone()));
                    }
                    if annots.iter().any(|a| a.name == names::UNCHECKED) {
                        t = self.ast.add_ty(TyExpr::Unchecked(t), start.to(self.prev_span()));
                    }
                    if annots.iter().any(|a| a.name == names::UNCHECKED_VARIANCE) {
                        t = self.ast.add_ty(TyExpr::UncheckedVariance(t), start.to(self.prev_span()));
                    }
                    for a in annots.iter().filter(|a| !matches!(a.name, names::UNCHECKED | names::UNCHECKED_VARIANCE | names::NOWARN)) {
                        let class = self.ast.annot_new(a).map(|(path, _)| path);
                        t = self.ast.add_ty(TyExpr::Annotated(t, class), start.to(self.prev_span()));
                    }
                    // The annotations' trees under the type as written, which a val's declared
                    // type names (`Int @Ann(m)`, the pickle's `ANNOTATEDtpt`).
                    for a in annots.iter().filter(|a| ![names::UNCHECKED, names::UNCHECKED_VARIANCE, names::NOWARN].contains(&a.name)) {
                        self.ast.type_annots.push((t, a.instance));
                    }
                }
                Tok::LBracket => {
                    self.bump();
                    let template_type = std::mem::replace(&mut self.template_type, false);
                    let mark = self.ty_scratch.len();
                    let mut cut = false;
                    loop {
                        let a = self.parse_type();
                        self.ty_scratch.push(a);
                        if !self.list_continues(Tok::RBracket, &mut cut) {
                            break;
                        }
                    }
                    self.expect(Tok::RBracket);
                    self.template_type = template_type;
                    let params = if self.syntax.kind_projector { self.kind_projector_params(mark) } else { Vec::new() };
                    let l = self.ty_list(mark);
                    t = self.ast.add_ty(TyExpr::Apply(t, l), start.to(self.prev_span()));
                    if !params.is_empty() {
                        let bounds = vec![(None, None); params.len()];
                        t = self.type_lambda(&params, &bounds, t, start);
                    }
                }
                // `T { type X = ...; def m: Int }`; after `new` such braces are a class body.
                Tok::ColonEol
                    if !self.template_type
                        && self.kind_at(1) == Tok::Indent
                        && matches!(self.kind_at(2), Tok::KwType | Tok::KwDef | Tok::KwVal | Tok::KwVar) =>
                {
                    let members = self.parse_refinement();
                    t = self.ast.add_ty(TyExpr::Refined(t, members), start.to(self.prev_span()));
                }
                _ => return t,
            }
        }
    }

    /// The members of a refinement, declarations without bodies.
    fn parse_refinement(&mut self) -> ListRef {
        self.refinement_members(2)
    }

    /// The declarations of a refinement block, after its `opening` tokens.
    fn refinement_members(&mut self, opening: usize) -> ListRef {
        for _ in 0..opening {
            self.bump();
        }
        let mut defs = Vec::new();
        self.statements(RecoverySite::Refinement, |p| {
            if !p.at_def_start() {
                let (at, found) = p.found();
                p.error_at(at, format!("expected a declaration, found {}", found));
                return;
            }
            let first = defs.len();
            p.parse_def(&mut defs);
            p.check_refinement_defs(&defs[first..]);
        });
        self.end_region();
        push_list(&mut self.ast.def_lists, &defs)
    }

    /// The members of a refinement in braces the lexer opened no region for, up to and with
    /// its `}`, possibly none.
    fn parse_brace_refinement(&mut self) -> ListRef {
        self.expect(Tok::LBrace);
        let mut defs = Vec::new();
        loop {
            while matches!(self.kind(), Tok::Semi | Tok::Newline) {
                self.bump();
            }
            if matches!(self.kind(), Tok::RBrace | Tok::Eof) {
                break;
            }
            if !self.at_def_start() {
                let (at, found) = self.found();
                self.error_at(at, format!("expected a declaration, found {}", found));
                self.bump();
                continue;
            }
            let first = defs.len();
            self.parse_def(&mut defs);
            self.check_refinement_defs(&defs[first..]);
        }
        self.expect(Tok::RBrace);
        push_list(&mut self.ast.def_lists, &defs)
    }

    /// A refinement's members are declarations without modifiers (dotty's `refineStatSeq`,
    /// `checkLegal`): one with a modifier, or with a body, is an error.
    fn check_refinement_defs(&mut self, defs: &[DefId]) {
        const NAMED: [(Mods, &str); 14] = [
            (mods::PRIVATE, "private"),
            (mods::PROTECTED, "protected"),
            (mods::ABSTRACT, "abstract"),
            (mods::FINAL, "final"),
            (mods::SEALED, "sealed"),
            (mods::IMPLICIT, "implicit"),
            (mods::GIVEN, "given"),
            (mods::LAZY, "lazy"),
            (mods::OVERRIDE, "override"),
            (mods::INLINE, "inline"),
            (mods::TRANSPARENT, "transparent"),
            (mods::INFIX, "infix"),
            (mods::OPEN, "open"),
            (mods::OPAQUE, "opaque"),
        ];
        for &d in defs {
            let def = self.ast.def(d);
            let named: Vec<&str> = NAMED.iter().filter(|&&(m, _)| def.mods & m != 0).map(|&(_, n)| n).collect();
            if !named.is_empty() {
                let span = def.span;
                self.error_at(span, format!("refinement cannot be `{}`", named.join("`, `")));
                continue;
            }
            let has_body = match &def.kind {
                DefKind::Val { rhs, .. } => rhs.is_some(),
                DefKind::Fun(f) => f.body.is_some(),
                DefKind::TypeAlias { .. } => false,
                _ => true,
            };
            if has_body {
                self.error_at(def.span, "refinement cannot have a body");
            }
        }
    }

    pub(super) fn parse_type_args(&mut self) -> ListRef {
        self.expect(Tok::LBracket);
        let mark = self.ty_scratch.len();
        let mut cut = false;
        loop {
            let a = self.parse_type();
            self.ty_scratch.push(a);
            if !self.list_continues(Tok::RBracket, &mut cut) {
                break;
            }
        }
        self.expect(Tok::RBracket);
        self.ty_list(mark)
    }

    /// A context bound, named with `as x` or left to the compiler.
    fn push_context_bound(&mut self, tp: &mut TypeParam, bound: TyExprId) {
        let name = if self.at_soft(names::AS) {
            self.bump();
            self.expect_ident().0
        } else {
            names::EMPTY
        };
        tp.context_bounds.push(bound);
        tp.evidence_names.push(name);
    }

    /// A bound of a type parameter; for a higher-kinded one, a type lambda over its parameters.
    fn parse_hk_bound(&mut self, inner: &[Name], inner_bounds: &[LambdaBounds], variances: &[i8]) -> TyExprId {
        let start = self.span();
        let bound = self.parse_type();
        if inner.is_empty() {
            return bound;
        }
        let made = self.type_lambda(inner, inner_bounds, bound, start);
        self.record_clause_variances(made, variances);
        made
    }

    /// The variances of the clause the lambda `made` was made over (`SyntaxNotes::clause_variances`), where one is not
    /// invariant.
    fn record_clause_variances(&mut self, made: TyExprId, variances: &[i8]) {
        if variances.iter().any(|&v| v != 0) {
            self.ast.notes.clause_variances.push((made, variances.into()));
        }
    }

    /// A definition's type parameter's clause's annotations (`F[@a A]`, the parameter's name at `owner`) with its
    /// lambda: its upper bound, over the clause where there is one, else one made for the record
    /// (`SyntaxNotes::hk_param_clauses`), which the typer resolves with the parameter's bounds.
    #[cold]
    #[inline(never)]
    fn record_param_clause(&mut self, owner: u32, upper: Option<TyExprId>, inner: &[Name], inner_bounds: &[LambdaBounds], variances: &[i8], annotated: BinderAnnotations) {
        let clause = match upper {
            Some(upper) => upper,
            None => {
                let start = self.span();
                let any = self.ast.add_ty(TyExpr::Resolved(crate::types::ANY), start);
                let made = self.type_lambda(inner, inner_bounds, any, start);
                self.record_clause_variances(made, variances);
                self.ast.notes.hk_param_clauses.push((owner, made));
                made
            }
        };
        self.record_binder_annots(BinderClause::Own(clause), annotated);
    }

    pub(super) fn parse_type_params(&mut self) -> Vec<TypeParam> {
        let mut out = Vec::new();
        if !self.eat(Tok::LBracket) {
            return out;
        }
        let mut cut = false;
        loop {
            let annots = self.parse_annots();
            let mut variance = 0i8;
            if self.eat_op(names::PLUS) {
                variance = 1;
            } else if self.eat_op(names::MINUS) {
                variance = -1;
            }
            let (name, span) = if self.at(Tok::Underscore) {
                // `WildcardParamName.fresh().toTypeName`, numbered by its place in the clause.
                let n = self.interner.unique("_$", out.len() as u32);
                (n, self.bump().span)
            } else {
                let (n, s) = self.expect_ident();
                self.check_dollar_name(n, s);
                (n, s)
            };
            let (inner, inner_bounds, clause_variances, annotated) = if self.at(Tok::LBracket) { self.parse_hk_clause() } else { Default::default() };
            let arity = inner.len().min(u8::MAX as usize) as u8;
            let mut tp = TypeParam {
                name,
                span,
                variance,
                arity,
                hk_variances: if clause_variances.iter().any(|&v| v != 0) { clause_variances.clone() } else { Vec::new() },
                upper: None,
                lower: None,
                context_bounds: Vec::new(),
                evidence_names: Vec::new(),
                annots,
            };
            if self.eat(Tok::Supertype) {
                tp.lower = Some(self.parse_hk_bound(&inner, &inner_bounds, &clause_variances));
            }
            if self.eat(Tok::Subtype) {
                tp.upper = Some(self.parse_hk_bound(&inner, &inner_bounds, &clause_variances));
            } else if inner_bounds.iter().any(|&(lo, hi)| lo.is_some() || hi.is_some()) {
                // The clause's own parameters bounded or of a kind of their own (`G[H[_]]`):
                // the parameter's bound is the lambda of them, which keeps it.
                let start = self.span();
                let any = self.ast.add_ty(TyExpr::Resolved(crate::types::ANY), start);
                let made = self.type_lambda(&inner, &inner_bounds, any, start);
                self.record_clause_variances(made, &clause_variances);
                tp.upper = Some(made);
            }
            if !annotated.is_empty() {
                self.record_param_clause(span.start, tp.upper, &inner, &inner_bounds, &clause_variances, annotated);
            }
            // A colon after a bound that braces do not enclose is reported by the version
            // (`Parsers.contextBounds`, at the colon).
            let mut after_bound = false;
            while self.at(Tok::Colon) {
                let colon = self.bump().span;
                if after_bound {
                    self.migration(crate::source_version::Migration::GivenSyntax, colon, "Multiple context bounds should be enclosed in `{ ... }`", crate::warnings::NO_ID);
                }
                after_bound = !self.at(Tok::LBrace);
                if self.eat(Tok::LBrace) {
                    loop {
                        let b = self.parse_type();
                        self.push_context_bound(&mut tp, b);
                        if !self.eat(Tok::Comma) {
                            break;
                        }
                    }
                    self.expect(Tok::RBrace);
                } else {
                    let b = self.parse_inter_type();
                    self.push_context_bound(&mut tp, b);
                }
            }
            if name != names::EMPTY {
                out.push(tp);
            }
            if !self.list_continues(Tok::RBracket, &mut cut) {
                break;
            }
        }
        self.expect(Tok::RBracket);
        out
    }
}

/// `Desugar.referencesName`: whether the type `t` holds a term identifier among `names` anywhere in its tree
/// (`existsSubTree { case Ident(name: TermName) => names.contains(name) }`, over `TreeAccumulator.foldOver` and
/// `UntypedTreeAccumulator.foldMoreCases`, child by child as they fold): a path's root, an identifier or an infix,
/// prefix or postfix operator of an annotation's argument, a lone variable pattern there, an import's or an export's
/// qualifier (not its selectors), through every type and expression under it. A binder's annotations are its
/// `TypeDef`'s modifiers, which the fold does not read. The crate's one walk of it, a method's evidences' as a
/// polymorphic function's.
pub(crate) fn references_name(ast: &Ast, t: TyExprId, names: &[Name]) -> bool {
    Mentions { ast, names }.ty(t)
}

struct Mentions<'a> {
    ast: &'a Ast,
    names: &'a [Name],
}

impl Mentions<'_> {
    fn ty(&self, t: TyExprId) -> bool {
        let ast = self.ast;
        let tys = |l: ListRef| ast.ty_list(l).iter().any(|&x| self.ty(x));
        self.nowarn_on(t)
            || match ast.ty(t) {
                TyExpr::Name(_) | TyExpr::Wildcard | TyExpr::Resolved(_) | TyExpr::TypeVar(_) | TyExpr::MainParam(..) | TyExpr::Error => false,
                TyExpr::Select(q, _) | TyExpr::Singleton(q) => self.path(q),
                TyExpr::Apply(f, args) => self.ty(f) || tys(args),
                TyExpr::Fun(ps, r) | TyExpr::CtxFun(ps, r) => tys(ps) || self.ty(r),
                TyExpr::Tuple(l) | TyExpr::NamedTuple(_, l) => tys(l),
                TyExpr::Union(a, b) | TyExpr::Inter(a, b) | TyExpr::BoundedWildcard(a, b) | TyExpr::MatchCase(a, b) => self.ty(a) || self.ty(b),
                TyExpr::ByName(x) | TyExpr::Repeated(x) | TyExpr::Project(x, _) => self.ty(x),
                // A lambda's binders' bounds (`LambdaTypeTree`'s `TypeDef`s, their `rhs`), then its body.
                TyExpr::Lambda(_, body) | TyExpr::PolyFun(_, body) => {
                    let bounds = ast.lambda_bounds.iter().find(|(l, _)| *l == t).map_or(&[][..], |&(_, bl)| &ast.ty_lists[bl.range()]);
                    bounds.iter().any(|&b| b != NO_BOUND && self.ty(b)) || self.ty(body)
                }
                TyExpr::Refined(x, defs) => self.ty(x) || ast.def_list(defs).iter().any(|&d| self.def(d)),
                TyExpr::Lit(e) => self.expr(e),
                TyExpr::Match(s, cases) => self.ty(s) || tys(cases),
                // `Annotated(arg, annot)`: the annotations written after the type, each a `new` and its arguments.
                TyExpr::Annotated(x, _) | TyExpr::Unchecked(x) | TyExpr::UncheckedVariance(x) => {
                    let (after, end) = (ast.ty_spans[x.idx()].end, ast.ty_spans[t.idx()].end);
                    self.ty(x)
                        || ast.inline_annots.iter().any(|a| {
                            let s = ast.expr_span(a.instance);
                            after < s.start && s.end <= end && self.expr(a.instance)
                        })
                }
            }
    }

    /// A `@nowarn` written after the type `t`, which the parser keeps as no node of the type
    /// (`Ast::nowarn_ascriptions`, the type's range with the annotation's).
    fn nowarn_on(&self, t: TyExprId) -> bool {
        let span = self.ast.ty_spans[t.idx()];
        self.ast.nowarn_ascriptions.iter().any(|(range, a)| range.start == span.start && range.end > span.end && self.expr(a.instance))
    }

    /// A path's qualifier: its root a term identifier, `C.this` none (`This` of a type name).
    fn path(&self, q: TyExprId) -> bool {
        match self.ast.ty(q) {
            TyExpr::Name(n) => self.names.contains(&n),
            TyExpr::Select(_, names::THIS) => false,
            TyExpr::Select(inner, _) => self.path(inner),
            _ => self.ty(q),
        }
    }

    fn tparams(&self, tps: &[TypeParam]) -> bool {
        tps.iter().any(|tp| [tp.upper, tp.lower].into_iter().flatten().any(|b| self.ty(b)) || tp.context_bounds.iter().any(|&b| self.ty(b)))
    }

    fn clauses(&self, clauses: &[ParamClause]) -> bool {
        clauses.iter().flat_map(|c| c.params.iter()).any(|p| self.ty(p.ty) || p.default.is_some_and(|d| self.expr(d)))
    }

    fn parents(&self, parents: &[Parent]) -> bool {
        parents.iter().any(|p| self.ty(p.ty) || p.args.iter().any(|&(l, _)| self.ast.expr_list(l).iter().any(|&a| self.expr(a))))
    }

    fn stmts(&self, stmts: &[Stmt]) -> bool {
        stmts.iter().any(|s| match *s {
            Stmt::Expr(e) => self.expr(e),
            Stmt::Def(d) => self.def(d),
            // `Import(expr, selectors)`: the qualifier alone, its root the one identifier (`foldOver` reads no selector).
            Stmt::Import(i) => self.ast.import_stmt(i).iter().any(|imp| self.qualifier(imp)),
        })
    }

    /// An import's or an export's qualifier: its root, an identifier, its selections none.
    fn qualifier(&self, imp: &Import) -> bool {
        imp.path.first().is_some_and(|n| self.names.contains(n))
    }

    fn def(&self, d: DefId) -> bool {
        match &self.ast.def(d).kind {
            DefKind::Val { pat, ty, rhs } => pat.is_some_and(|p| self.pat(p)) || ty.is_some_and(|t| self.ty(t)) || rhs.is_some_and(|e| self.expr(e)),
            DefKind::Fun(f) => self.tparams(&f.tparams) || self.clauses(&f.clauses) || f.ret.is_some_and(|t| self.ty(t)) || f.body.is_some_and(|e| self.expr(e)),
            DefKind::TypeAlias { tparams, rhs, lower, upper } => self.tparams(tparams) || [*rhs, *lower, *upper].into_iter().flatten().any(|t| self.ty(t)),
            DefKind::Class(c) => {
                self.tparams(&c.tparams)
                    || self.clauses(&c.clauses)
                    || self.parents(&c.parents)
                    || c.self_type.is_some_and(|t| self.ty(t))
                    || self.stmts(&c.body)
                    || self.ast.exports[c.exports.range()].iter().any(|e| self.qualifier(e))
            }
            DefKind::Given(g) => {
                self.tparams(&g.tparams) || self.clauses(&g.clauses) || self.ty(g.ty) || g.alias.is_some_and(|e| self.expr(e)) || self.parents(&g.parents) || self.stmts(&g.body)
            }
        }
    }

    /// A pattern: a lone variable is an `Ident` before typing; a binder's name is no identifier.
    fn pat(&self, p: PatId) -> bool {
        let ast = self.ast;
        let pats = |l: ListRef| ast.pat_list(l).iter().any(|&x| self.pat(x));
        match ast.pat(p) {
            Pat::Wildcard | Pat::Error => false,
            Pat::Bind(n, None) => self.names.contains(&n),
            Pat::Bind(_, Some(x)) | Pat::NamedField(_, x) | Pat::Rest(x) => self.pat(x),
            Pat::Typed(x, t) => self.pat(x) || self.ty(t),
            Pat::Lit(e) | Pat::StableId(e) | Pat::Quote(e) => self.expr(e),
            Pat::Ctor(f, args) => self.expr(f) || pats(args),
            Pat::Tuple(l) | Pat::Alt(l) | Pat::Interp(_, _, l) => pats(l),
            Pat::QuoteType(t) => self.ty(t),
        }
    }

    fn case(&self, k: usize) -> bool {
        let c = self.ast.cases[k];
        self.pat(c.pat) || c.guard.is_some_and(|g| self.expr(g)) || self.expr(c.body)
    }

    fn expr(&self, e: ExprId) -> bool {
        let ast = self.ast;
        let exprs = |l: ListRef| ast.expr_list(l).iter().any(|&x| self.expr(x));
        match ast.expr(e) {
            Expr::Ident(n) => self.names.contains(&n),
            // `InfixOp(left, op, right)` and `PrefixOp(op, od)` keep the operator as an `Ident`.
            Expr::Infix(a, op, b, _) => self.names.contains(&op) || self.expr(a) || self.expr(b),
            Expr::Prefix(op, x) => self.names.contains(&op) || self.expr(x),
            // A postfix operator the parser made a selection of (`PostfixOp(od, Ident(op))`).
            Expr::Select(x, op) if ast.notes.postfix_ops.binary_search(&e).is_ok() => self.names.contains(&op) || self.expr(x),
            Expr::Select(x, _) | Expr::NamedArg(_, x) | Expr::Parens(x) | Expr::Unchecked(x) | Expr::Throw(x) | Expr::Quote(x) | Expr::Splice(x) | Expr::AsFunction(x) => self.expr(x),
            Expr::Apply(f, l) | Expr::UsingApply(f, l) => self.expr(f) || exprs(l),
            Expr::TypeApply(f, l) => self.expr(f) || ast.ty_list(l).iter().any(|&t| self.ty(t)),
            // `{ case .. }` is `Match(EmptyTree, cases)`: the parser's matching on its synthetic parameter is no tree of it.
            Expr::Lambda(ps, body) if ast.lambda_params[ps.range()].first().is_some_and(|p| p.cases) => match ast.expr(body) {
                Expr::Match(_, cases) => cases.range().any(|k| self.case(k)),
                _ => self.expr(body),
            },
            Expr::Lambda(ps, body) => ast.lambda_params[ps.range()].iter().any(|p| p.ty.is_some_and(|t| self.ty(t))) || self.expr(body),
            Expr::PolyLambda(_, body) => {
                let bounds = ast.poly_lambda_bounds.iter().find(|(l, _)| *l == e).map_or(&[][..], |&(_, bl)| &ast.ty_lists[bl.range()]);
                bounds.iter().any(|&b| b != NO_BOUND && self.ty(b)) || self.expr(body)
            }
            Expr::If(c, t, f) | Expr::InlineIf(c, t, f) => self.expr(c) || self.expr(t) || f.is_some_and(|f| self.expr(f)),
            Expr::Match(s, cases) | Expr::InlineMatch(s, cases) => self.expr(s) || cases.range().any(|k| self.case(k)),
            Expr::Block(stmts) => self.stmts(ast.stmt_list(stmts)),
            Expr::While(a, b) | Expr::Assign(a, b) => self.expr(a) || self.expr(b),
            Expr::For(enums, body, _) => {
                enums.range().any(|k| match ast.enumerators[k] {
                    Enumerator::Gen(p, x, _) | Enumerator::CaseGen(p, x) | Enumerator::Val(p, x) => self.pat(p) || self.expr(x),
                    Enumerator::Guard(x) => self.expr(x),
                }) || self.expr(body)
            }
            Expr::Tuple(l) | Expr::NamedTuple(_, l) | Expr::Interp(_, _, l) => exprs(l),
            Expr::SeqLit(l, t) => exprs(l) || self.ty(t),
            Expr::Typed(x, t) => self.expr(x) || self.ty(t),
            Expr::New(t, l) => self.ty(t) || exprs(l),
            Expr::NewAnon(d) => self.def(d),
            Expr::Return(x) => x.is_some_and(|x| self.expr(x)),
            Expr::Try(k) => {
                let t = ast.tries[k as usize];
                self.expr(t.body) || t.cases.range().any(|c| self.case(c)) || t.handler.is_some_and(|h| self.expr(h)) || t.finalizer.is_some_and(|f| self.expr(f))
            }
            Expr::QuoteType(t) | Expr::ClassOf(t) | Expr::ThisOf(t) => self.ty(t),
            Expr::SplicePat(p) => self.pat(p),
            Expr::IntLit(_)
            | Expr::LongLit(_)
            | Expr::DoubleLit(_)
            | Expr::DecimalLit(_)
            | Expr::FloatLit(_)
            | Expr::BoolLit(_)
            | Expr::CharLit(_)
            | Expr::StringLit(_)
            | Expr::UnitLit
            | Expr::NullLit
            | Expr::SymRef(_)
            | Expr::This
            | Expr::QualThis(_)
            | Expr::Super(_)
            | Expr::Unsupported(_)
            | Expr::Withheld(_)
            | Expr::Derived
            | Expr::Error => false,
        }
    }
}
