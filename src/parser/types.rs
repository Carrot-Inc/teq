use super::Parser;
use crate::ast::*;
use crate::intern::Name;
use crate::names;
use crate::source::Span;
use crate::token::{Tok, Token};

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
            let (params, bounds) = self.parse_type_params_of_lambda();
            self.expect(Tok::Arrow);
            let fun = self.parse_type();
            if !matches!(self.ast.ty(fun), TyExpr::Fun(..) | TyExpr::CtxFun(..)) {
                self.error_at(start, "a polymorphic function type takes a function type after `=>`");
            }
            let l = push_list(&mut self.ast.name_lists, &params);
            let id = self.ast.add_ty(TyExpr::PolyFun(l, fun), start.to(self.prev_span()));
            self.record_lambda_bounds(id, &bounds);
            return id;
        }
        let mut t = self.parse_union_type();
        if self.at(Tok::KwMatch) {
            self.bump();
            let cases = self.parse_type_cases();
            t = self.ast.add_ty(TyExpr::Match(t, cases), start.to(self.prev_span()));
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
            let pat = self.parse_inter_type();
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

    fn parse_type_lambda(&mut self) -> TyExprId {
        let start = self.span();
        let (params, bounds) = self.parse_type_params_of_lambda();
        self.expect(Tok::TypeLambdaArrow);
        let body = self.parse_type();
        self.type_lambda(&params, &bounds, body, start)
    }

    /// `[X, Y >: L <: U, F[Z]]` of a type lambda or a polymorphic function type or literal. A
    /// higher-kinded binder takes a clause of its own, as dotty's `typeParamClause` parses it,
    /// and is bounded by a type lambda over that clause, `[Z] =>> Any` where it declares no upper
    /// bound (dotty's `F <: [Z] =>> Any`), which carries its kind to typing.
    pub(super) fn parse_type_params_of_lambda(&mut self) -> (Vec<Name>, Vec<LambdaBounds>) {
        self.expect(Tok::LBracket);
        let mut params = Vec::new();
        let mut bounds = Vec::new();
        let mut cut = false;
        loop {
            if self.eat_op(names::PLUS) || self.eat_op(names::MINUS) {
                self.error_at(self.prev_span(), "no `+/-` variance annotation allowed here");
            }
            params.push(self.expect_ident().0);
            let (inner, inner_bounds, _) = if self.at(Tok::LBracket) { self.parse_hk_clause() } else { Default::default() };
            bounds.push(self.parse_kind_bound(&inner, &inner_bounds));
            if !self.list_continues(Tok::RBracket, &mut cut) {
                break;
            }
        }
        self.expect(Tok::RBracket);
        (params, bounds)
    }

    /// The bounds of a parameter that may take a clause of its own, `inner` with its bounds:
    /// `>: L` and `<: U` written over that clause, or where it writes no upper one a type lambda
    /// of the clause with `Any` as its result, the kind dotty's `lambdaAbstract(hkparams, bounds)`
    /// keeps as the bound.
    fn parse_kind_bound(&mut self, inner: &[Name], inner_bounds: &[LambdaBounds]) -> LambdaBounds {
        let lower = if self.eat(Tok::Supertype) { Some(self.parse_hk_bound(inner, inner_bounds)) } else { None };
        let upper = if self.eat(Tok::Subtype) {
            Some(self.parse_hk_bound(inner, inner_bounds))
        } else if inner.is_empty() {
            None
        } else {
            let start = self.span();
            let any = self.ast.add_ty(TyExpr::Resolved(crate::types::ANY), start);
            Some(self.type_lambda(inner, inner_bounds, any, start))
        };
        (lower, upper)
    }

    /// The `[_]` of a higher-kinded parameter: its own parameters' names, bounds and variances,
    /// each of them parsed as dotty's `typeParamClauseOpt(ParamOwner.Hk)` parses it, so that one
    /// taking a clause of its own (`F[G[_]]`) has that kind as its bound; their names scope over
    /// the bounds (`S[T] <: Semigroup[T]`).
    fn parse_hk_clause(&mut self) -> (Vec<Name>, Vec<LambdaBounds>, Vec<i8>) {
        let mut inner = Vec::new();
        let mut inner_bounds = Vec::new();
        let mut hk_variances = Vec::new();
        self.expect(Tok::LBracket);
        loop {
            hk_variances.push(if self.eat_op(names::PLUS) {
                1
            } else if self.eat_op(names::MINUS) {
                -1
            } else {
                0
            });
            if self.eat(Tok::Underscore) {
                inner.push(names::WILDCARD);
            } else {
                inner.push(self.expect_ident().0);
            }
            let (own, own_bounds, _) = if self.at(Tok::LBracket) { self.parse_hk_clause() } else { Default::default() };
            inner_bounds.push(self.parse_kind_bound(&own, &own_bounds));
            if !self.eat(Tok::Comma) {
                break;
            }
        }
        self.expect(Tok::RBracket);
        (inner, inner_bounds, hk_variances)
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
            // either token, `[>: Type] [<: Type]`, each bound once and the lower first).
            Tok::Underscore | Tok::OpIdent if self.kind() == Tok::Underscore || self.tok().name == names::QMARK => {
                self.bump();
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
                while !self.at(Tok::RParen) && !self.at(Tok::Eof) {
                    if self.at(Tok::Ident) && self.kind_at(1) == Tok::Colon {
                        names.push(self.bump().name);
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
                    if !self.list_continues(Tok::RParen, &mut cut) {
                        break;
                    }
                }
                self.expect(Tok::RParen);
                self.template_type = template_type;
                let is_fun_params = matches!(self.kind(), Tok::Arrow | Tok::CtxArrow);
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
    fn parse_hk_bound(&mut self, inner: &[Name], inner_bounds: &[LambdaBounds]) -> TyExprId {
        let start = self.span();
        let bound = self.parse_type();
        if inner.is_empty() {
            return bound;
        }
        self.type_lambda(inner, inner_bounds, bound, start)
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
                (names::WILDCARD, self.bump().span)
            } else {
                self.expect_ident()
            };
            let (inner, inner_bounds, hk_variances) = if self.at(Tok::LBracket) { self.parse_hk_clause() } else { Default::default() };
            let arity = inner.len().min(u8::MAX as usize) as u8;
            let mut tp = TypeParam {
                name,
                span,
                variance,
                arity,
                hk_variances: if hk_variances.iter().any(|&v| v != 0) { hk_variances } else { Vec::new() },
                upper: None,
                lower: None,
                context_bounds: Vec::new(),
                evidence_names: Vec::new(),
                annots,
            };
            if self.eat(Tok::Supertype) {
                tp.lower = Some(self.parse_hk_bound(&inner, &inner_bounds));
            }
            if self.eat(Tok::Subtype) {
                tp.upper = Some(self.parse_hk_bound(&inner, &inner_bounds));
            } else if inner_bounds.iter().any(|&(lo, hi)| lo.is_some() || hi.is_some()) {
                // The clause's own parameters bounded or of a kind of their own (`G[H[_]]`):
                // the parameter's bound is the lambda of them, which keeps it.
                let start = self.span();
                let any = self.ast.add_ty(TyExpr::Resolved(crate::types::ANY), start);
                tp.upper = Some(self.type_lambda(&inner, &inner_bounds, any, start));
            }
            while self.eat(Tok::Colon) {
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
