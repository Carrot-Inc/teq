use super::{unescape, Parser};
use crate::ast::*;
use crate::intern::Name;
use crate::names;
use crate::source::Span;
use crate::token::Tok;

pub(super) fn op_precedence(op: &str) -> (u8, bool) {
    let right = op.ends_with(':');
    let first = op.as_bytes()[0];
    let is_assign = op.ends_with('=')
        && first != b'='
        && !matches!(op, "<=" | ">=" | "!=");
    if is_assign {
        return (1, false);
    }
    let prec = match first {
        b'|' => 3,
        b'^' => 4,
        b'&' => 5,
        b'=' | b'!' => 6,
        b'<' | b'>' => 7,
        b':' => 8,
        b'+' | b'-' => 9,
        b'*' | b'/' | b'%' => 10,
        c if c.is_ascii_alphabetic() || c == b'_' || c == b'$' || c >= 0x80 => 2,
        _ => 11,
    };
    (prec, right)
}

impl<'a> Parser<'a> {
    pub(super) fn parse_expr(&mut self) -> ExprId {
        let mark = self.placeholders.len();
        let start = self.span();
        let e = self.parse_expr_inner();
        self.placeholder_lambda(mark, start, e)
    }

    /// The expression `e` as the function its placeholders from `mark` on make, where it is more
    /// than one of them alone.
    fn placeholder_lambda(&mut self, mark: usize, start: Span, e: ExprId) -> ExprId {
        if self.placeholders.len() > mark {
            let bare = self.placeholders.len() == mark + 1
                && matches!(self.ast.expr(e), Expr::Ident(n) if n == self.placeholders[mark].name);
            if !bare {
                let params = push_list(&mut self.ast.lambda_params, &self.placeholders[mark..]);
                self.placeholders.truncate(mark);
                return self.ast.add_expr(Expr::Lambda(params, e), start.to(self.prev_span()));
            }
        }
        e
    }

    pub(super) fn parse_block_or_expr(&mut self) -> ExprId {
        if !self.at(Tok::Indent) {
            return self.parse_expr();
        }
        self.parse_block_operand(0)
    }

    /// A block at the `Indent` and what continues it. A block is a simple expression, as
    /// scalac's `blockExpr` is. After a brace block, selections, operators, `match` and an
    /// ascription continue it (`{ .. }.map(f)`, `{ 2 } * 3`, `{ .. } : @unchecked`); an argument
    /// list does not apply it. After an indented one, the lexer leaves an operator on the same
    /// logical line only where it leads the next line as an infix operator. Above a `min_prec`
    /// of zero the block is an operator's right operand, which operators of lower precedence
    /// do not continue.
    fn parse_block_operand(&mut self, min_prec: u8) -> ExprId {
        let mark = self.placeholders.len();
        let start = self.span();
        let braced = start.end > start.start;
        let block = self.parse_block();
        if braced || self.at_operator() {
            let e = if braced && matches!(self.kind(), Tok::Dot | Tok::LBracket) { self.parse_postfix(block, start) } else { block };
            let e = self.parse_infix_rest(e, start, min_prec);
            if min_prec > 0 {
                return e;
            }
            let e = self.parse_expr1_rest(e, start);
            return self.placeholder_lambda(mark, start, e);
        }
        // `{ case ... }: PartialFunction[A, B]`
        if self.at(Tok::Colon) {
            return self.parse_expr1_rest(block, start);
        }
        block
    }

    /// scalac's `isOperator`: a symbolic identifier, a backquoted one, or one whose name ends
    /// in an operator character (`a_+`).
    fn at_operator(&self) -> bool {
        match self.kind() {
            Tok::OpIdent => true,
            Tok::Ident => {
                let span = self.span();
                let text = &self.text.as_bytes()[span.start as usize..span.end as usize];
                text.first() == Some(&b'`') || text.last().is_some_and(|&b| crate::lexer::is_op_char(b))
            }
            _ => false,
        }
    }

    /// An indented block starting with `case` is a pattern-matching anonymous function.
    fn parse_case_lambda(&mut self) -> ExprId {
        let start = self.span();
        let cases = self.parse_cases();
        self.case_lambda(cases, start)
    }

    /// The cases that fill an open region up to its end, as a function of them.
    pub(super) fn parse_case_lambda_in_region(&mut self) -> ExprId {
        let start = self.span();
        let cases = self.parse_cases_from(true);
        self.case_lambda(cases, start)
    }

    fn case_lambda(&mut self, cases: ListRef, start: Span) -> ExprId {
        let span = start.to(self.prev_span());
        let scrutinee = self.ast.add_expr(Expr::Ident(names::CASE_PARAM), start);
        let body = self.ast.add_expr(Expr::Match(scrutinee, cases), span);
        let param = LambdaParam { name: names::CASE_PARAM, span: start, ty: None, implicit: false, contextual: false };
        let params = push_list(&mut self.ast.lambda_params, &[param]);
        self.ast.add_expr(Expr::Lambda(params, body), span)
    }

    pub(super) fn parse_block(&mut self) -> ExprId {
        let start = self.span();
        if self.kind_at(1) == Tok::KwCase && !matches!(self.kind_at(2), Tok::KwClass | Tok::KwObject) {
            return self.parse_case_lambda();
        }
        self.expect(Tok::Indent);
        self.parse_block_rest(start)
    }

    /// A block's statements after its `Indent`, up to and with the end of its region.
    pub(super) fn parse_block_rest(&mut self, start: Span) -> ExprId {
        let derives_mark = self.pending_derives.len();
        let placeholders_mark = self.placeholders.len();
        // `{ x => a; b }`: the lambda's body is the rest of the block, as under scalac.
        let lambda = self.at_lambda_start().then(|| (self.span(), self.parse_lambda_params()));
        let mut stmts: Vec<Stmt> = Vec::new();
        self.statements(RecoverySite::Block, |p| p.parse_block_stmt(&mut stmts));
        self.end_region();
        // Local classes are rejected by the typer; their clauses must not reach an outer body.
        self.pending_derives.truncate(derives_mark);
        // A statement that is a placeholder alone binds it to no function: scalac's
        // `checkNoEscapingPlaceholders` around a block's statements (`{ _ } + 10`).
        if let Some(p) = self.placeholders.get(placeholders_mark) {
            let span = p.span;
            self.error_at(span, "unbound placeholder parameter; incorrect use of `_`");
            self.placeholders.truncate(placeholders_mark);
        }
        let body = match stmts[..] {
            [Stmt::Expr(e)] => {
                self.ast.braced.push(e);
                e
            }
            _ => {
                let l = push_list(&mut self.ast.stmts, &stmts);
                self.ast.add_expr(Expr::Block(l), start.to(self.prev_span()))
            }
        };
        match lambda {
            Some((lambda_start, params)) => self.ast.add_expr(Expr::Lambda(params, body), lambda_start.to(self.prev_span())),
            None => body,
        }
    }

    pub(super) fn parse_block_stmt(&mut self, out: &mut Vec<Stmt>) {
        if self.eat(Tok::KwImport) {
            let start = self.ast.local_imports.len() as u32;
            let mut imports = std::mem::take(&mut self.ast.local_imports);
            self.parse_import_exprs(&mut imports);
            self.ast.local_imports = imports;
            let len = self.ast.local_imports.len() as u32 - start;
            out.push(Stmt::Import(self.ast.import_stmts.len() as u32));
            self.ast.import_stmts.push(ListRef { start, len });
        } else if self.at(Tok::KwExport) {
            self.error_at(self.span(), "exports are only supported in the body of an object, trait or class");
            let mark = self.export_scratch.len();
            self.parse_export();
            self.export_scratch.truncate(mark);
        } else if self.at_def_start() {
            let mut defs = Vec::new();
            self.parse_def(&mut defs);
            out.extend(defs.into_iter().map(Stmt::Def));
        } else {
            let e = self.parse_expr();
            out.push(Stmt::Expr(e));
        }
    }

    /// True if the parenthesized group at the current token is followed by a lambda arrow.
    fn parens_followed_by_arrow(&self) -> bool {
        self.parens_followed_by_arrow_at(0)
    }

    fn parens_followed_by_arrow_at(&self, from: usize) -> bool {
        let mut depth = 0usize;
        let mut i = from;
        loop {
            match self.kind_at(i) {
                Tok::LParen | Tok::LBracket | Tok::LBrace => depth += 1,
                Tok::RParen | Tok::RBracket | Tok::RBrace => {
                    depth -= 1;
                    if depth == 0 {
                        return matches!(self.kind_at(i + 1), Tok::Arrow | Tok::CtxArrow);
                    }
                }
                Tok::Eof | Tok::Indent | Tok::Outdent | Tok::Newline => return false,
                _ => {}
            }
            i += 1;
        }
    }

    /// dotc's `isExprIntro`: the token can begin an expression on the same line.
    fn at_expr_start(&self) -> bool {
        self.expr_start_at(0)
    }

    #[inline]
    pub(super) fn expr_start_at(&self, n: usize) -> bool {
        match self.kind_at(n) {
            Tok::Ident | Tok::OpIdent | Tok::IntLit | Tok::LongLit | Tok::DoubleLit | Tok::FloatLit | Tok::CharLit
            | Tok::StringLit | Tok::InterpStart | Tok::LParen | Tok::Underscore | Tok::KwThis
            | Tok::KwSuper | Tok::KwTrue | Tok::KwFalse | Tok::KwNew | Tok::KwIf | Tok::KwWhile
            | Tok::KwFor | Tok::KwReturn | Tok::KwThrow | Tok::KwTry | Tok::KwNull | Tok::Indent
            | Tok::Quote | Tok::QuoteId | Tok::Splice => true,
            Tok::ColonEol => self.kind_at(n + 1) == Tok::Indent,
            _ => false,
        }
    }

    fn at_lambda_start(&self) -> bool {
        match self.kind() {
            Tok::Ident | Tok::Underscore => matches!(self.kind_at(1), Tok::Arrow | Tok::CtxArrow),
            Tok::LParen => self.parens_followed_by_arrow(),
            Tok::KwImplicit => match self.kind_at(1) {
                Tok::Ident | Tok::Underscore => matches!(self.kind_at(2), Tok::Arrow | Tok::CtxArrow),
                Tok::LParen => self.parens_followed_by_arrow_at(1),
                _ => false,
            },
            _ => false,
        }
    }

    fn parse_lambda(&mut self) -> ExprId {
        let start = self.span();
        let params = self.parse_lambda_params();
        let body = self.parse_block_or_expr();
        self.ast.add_expr(Expr::Lambda(params, body), start.to(self.prev_span()))
    }

    /// The lambda of a colon argument (`xs.exists: x =>`): an indented body is a block that
    /// ends where it does, and an operator leading the line after it continues the call, as
    /// scalac's `closureRest` reads a body in `InColonArg`.
    fn parse_colon_lambda(&mut self) -> ExprId {
        let start = self.span();
        let params = self.parse_lambda_params();
        let indented = self.at(Tok::Indent) && self.span().start == self.span().end;
        let body = if indented { self.parse_block() } else { self.parse_block_or_expr() };
        self.ast.add_expr(Expr::Lambda(params, body), start.to(self.prev_span()))
    }

    /// The parameters of a lambda up to and including its arrow.
    fn parse_lambda_params(&mut self) -> ListRef {
        let mut params: Vec<LambdaParam> = Vec::new();
        // `implicit x => ...`, still legal in Scala 3.
        let implicit = self.eat(Tok::KwImplicit);
        if self.eat(Tok::LParen) {
            let mut cut = false;
            while !self.at(Tok::RParen) && !self.at(Tok::Eof) {
                let p = self.parse_lambda_param(params.len());
                params.push(p);
                if !self.list_continues(Tok::RParen, &mut cut) {
                    break;
                }
            }
            self.expect(Tok::RParen);
        } else {
            let p = self.parse_lambda_param(0);
            params.push(p);
        }
        let contextual = self.eat(Tok::CtxArrow);
        if !contextual {
            self.expect(Tok::Arrow);
        }
        for p in &mut params {
            p.implicit = implicit || contextual;
            p.contextual = contextual;
        }
        push_list(&mut self.ast.lambda_params, &params)
    }

    /// `_` parameters are named apart from each other and from the placeholders of the body.
    fn parse_lambda_param(&mut self, index: usize) -> LambdaParam {
        let span = self.span();
        let name = if self.eat(Tok::Underscore) {
            self.placeholder_name(8 + index)
        } else {
            self.expect_ident().0
        };
        let ty = if self.eat(Tok::Colon) { Some(self.parse_type()) } else { None };
        LambdaParam { name, span, ty, implicit: false, contextual: false }
    }

    fn parse_expr_inner(&mut self) -> ExprId {
        let start = self.span();
        match self.kind() {
            Tok::KwIf => return self.parse_if(),
            Tok::KwWhile => {
                self.bump();
                let cond = self.parse_cond(Tok::KwDo);
                let body = self.parse_block_or_expr();
                return self.ast.add_expr(Expr::While(cond, body), start.to(self.prev_span()));
            }
            Tok::KwFor => return self.parse_for(),
            Tok::KwDo => {
                self.error_at(
                    start,
                    "`do <body> while <cond>` is no longer supported, use `while <body> ; <cond> do ()` instead",
                );
                self.bump();
                self.parse_block_or_expr();
                self.eat(Tok::Semi);
                if self.eat(Tok::KwWhile) {
                    self.parse_block_or_expr();
                }
                return self.ast.add_expr(Expr::Error, start);
            }
            Tok::KwReturn => {
                self.bump();
                let value = if self.at_expr_start() { Some(self.parse_expr()) } else { None };
                return self.ast.add_expr(Expr::Return(value), start.to(self.prev_span()));
            }
            Tok::KwThrow => return self.parse_throw(),
            Tok::KwTry => return self.parse_try(),
            _ => {}
        }
        if self.at_lambda_start() {
            return self.parse_lambda();
        }
        let e = self.parse_infix_expr();
        self.parse_expr1_rest(e, start)
    }

    /// What may follow an infix expression: `match`, an assignment, an ascription.
    fn parse_expr1_rest(&mut self, mut e: ExprId, start: Span) -> ExprId {
        loop {
            match self.kind() {
                Tok::KwMatch => {
                    self.bump();
                    let cases = self.parse_cases();
                    e = self.ast.add_expr(Expr::Match(e, cases), start.to(self.prev_span()));
                }
                Tok::Eq => {
                    self.bump();
                    let rhs = self.parse_block_or_expr();
                    return self.ast.add_expr(Expr::Assign(e, rhs), start.to(self.prev_span()));
                }
                Tok::Colon if self.kind_at(1) == Tok::At => {
                    self.bump();
                    let annots = self.parse_inline_annots();
                    self.ast.inline_annots.extend_from_slice(&annots);
                    if annots.iter().any(|a| a.name == names::UNCHECKED) {
                        e = self.ast.add_expr(Expr::Unchecked(e), start.to(self.prev_span()));
                    }
                }
                // `xs: _*` is the Scala 2 spelling of `xs*`, which the argument list reads.
                Tok::Colon if self.kind_at(1) == Tok::Underscore && self.spread_star_at(2) => {
                    self.bump();
                    self.bump();
                    return e;
                }
                Tok::Colon => {
                    self.bump();
                    let t = self.parse_type();
                    e = self.ast.add_expr(Expr::Typed(e, t), start.to(self.prev_span()));
                }
                // `(1:` with the type on the next line, which `colon_takes_argument` leaves.
                Tok::ColonEol if self.at_colon_before_indent() && !self.colon_takes_argument() => {
                    let region = self.eat_annotation_colon().flatten();
                    let t = self.parse_type();
                    e = self.ast.add_expr(Expr::Typed(e, t), start.to(self.prev_span()));
                    self.end_annotation_region(region);
                }
                _ => return e,
            }
        }
    }

    /// The prefix of `new p.C(..)` as an expression: the enclosing instance of `C` when `C` is a
    /// class nested in a class, which the typer tells.
    fn prefix_value(&mut self, ty: crate::ast::TyExprId, span: crate::source::Span) -> Option<ExprId> {
        let ty = match self.ast.ty(ty) {
            TyExpr::Apply(f, _) => f,
            _ => ty,
        };
        let TyExpr::Select(prefix, _) = self.ast.ty(ty) else { return None };
        self.path_value(prefix, span)
    }

    fn path_value(&mut self, t: crate::ast::TyExprId, span: crate::source::Span) -> Option<ExprId> {
        match self.ast.ty(t) {
            TyExpr::Name(names::THIS) => Some(self.ast.add_expr(Expr::This, span)),
            TyExpr::Name(n) => Some(self.ast.add_expr(Expr::Ident(n), span)),
            TyExpr::Select(q, names::THIS) => match self.ast.ty(q) {
                TyExpr::Name(n) => Some(self.ast.add_expr(Expr::QualThis(n), span)),
                _ => None,
            },
            TyExpr::Select(q, n) => {
                let q = self.path_value(q, span)?;
                Some(self.ast.add_expr(Expr::Select(q, n), span))
            }
            _ => None,
        }
    }

    fn parse_if(&mut self) -> ExprId {
        let start = self.span();
        self.expect(Tok::KwIf);
        let cond = self.parse_cond(Tok::KwThen);
        let then = self.parse_block_or_expr();
        if self.at(Tok::Semi) && self.kind_at(1) == Tok::KwElse {
            self.bump();
        }
        let els = if self.eat(Tok::KwElse) { Some(self.parse_block_or_expr()) } else { None };
        self.ast.add_expr(Expr::If(cond, then, els), start.to(self.prev_span()))
    }

    /// The condition of an `if` or `while`, as dotc's `condExpr` reads it: `(c)` alone is the
    /// Scala 2 form and needs no `then` or `do`; `(c)` that continues as an expression
    /// (`if (a) && b then`) and every other form end with `alt`.
    fn parse_cond(&mut self, alt: Tok) -> ExprId {
        let start = self.span();
        if self.at(Tok::LParen) {
            let t = self.parse_parens();
            if self.at(alt) {
                self.reject_end_marker_before(alt.describe());
                self.bump();
                return t;
            }
            if self.to_be_continued(alt) {
                let t = self.parse_postfix(t, start);
                let t = self.parse_infix_rest(t, start, 0);
                let t = self.parse_expr1_rest(t, start);
                self.expect(alt);
                return t;
            }
            self.eat(Tok::Newline);
            return t;
        }
        let cond = self.parse_block_or_expr();
        if self.at(alt) {
            self.reject_end_marker_before(alt.describe());
        }
        self.expect(alt);
        cond
    }

    /// Whether `(c)` continues as the operand of a larger condition: no line break follows,
    /// and what follows cannot start a statement or leads to `alt` before one could start.
    fn to_be_continued(&self, alt: Tok) -> bool {
        match self.kind() {
            Tok::Newline | Tok::Indent | Tok::Outdent | Tok::Eof => false,
            Tok::ColonEol => self.followed_by_token(alt),
            k => !k.can_start_statement() || self.followed_by_token(alt),
        }
    }

    /// dotc's `followedByToken`: `alt` comes before a token that has to start a statement,
    /// nested regions skipped.
    fn followed_by_token(&self, alt: Tok) -> bool {
        let mut depth = 0usize;
        let mut i = 0usize;
        loop {
            let k = self.kind_at(i);
            if depth == 0 {
                if k == alt {
                    return true;
                }
                if k.stops_cond_scan() || k == Tok::Outdent {
                    return false;
                }
            } else if k == Tok::Eof {
                return false;
            } else if k == Tok::Outdent {
                depth -= 1;
            }
            if k == Tok::Indent {
                depth += 1;
            }
            i += 1;
        }
    }

    fn parse_throw(&mut self) -> ExprId {
        let start = self.span();
        self.expect(Tok::KwThrow);
        let e = self.parse_expr();
        self.ast.add_expr(Expr::Throw(e), start.to(self.prev_span()))
    }

    /// `try body catch { case ... } finally f`; the handler may also be an expression, a
    /// `PartialFunction` applied to what was thrown.
    fn parse_try(&mut self) -> ExprId {
        let start = self.span();
        self.expect(Tok::KwTry);
        let body = self.parse_block_or_expr();
        let mut cases = ListRef::EMPTY;
        let mut handler = None;
        if self.eat(Tok::KwCatch) {
            let at_case = self.at(Tok::KwCase) || (self.at(Tok::Indent) && self.kind_at(1) == Tok::KwCase);
            if at_case {
                cases = self.parse_cases();
            } else if matches!(self.kind(), Tok::KwFinally | Tok::Newline | Tok::Outdent | Tok::Eof) {
                let msg = "The catch block does not contain a valid expression, try adding a case like - case e: Exception => to the block";
                self.error_at(self.prev_span(), msg);
            } else {
                handler = Some(self.parse_block_or_expr());
            }
        }
        let finalizer = if self.eat(Tok::KwFinally) { Some(self.parse_block_or_expr()) } else { None };
        let index = self.ast.tries.len() as u32;
        self.ast.tries.push(TryExpr { body, cases, handler, finalizer });
        self.ast.add_expr(Expr::Try(index), start.to(self.prev_span()))
    }

    pub(super) fn parse_cases(&mut self) -> ListRef {
        let indented = self.eat(Tok::Indent);
        self.parse_cases_from(indented)
    }

    /// The cases, `indented` when they fill a region whose `Indent` is behind, up to and with
    /// its end.
    fn parse_cases_from(&mut self, indented: bool) -> ListRef {
        let mut cases: Vec<CaseClause> = Vec::new();
        let mut broken: Vec<u32> = Vec::new();
        loop {
            if indented {
                self.skip_separators();
            } else if self.at(Tok::Newline) && self.kind_at(1) == Tok::KwCase && !cases.is_empty() {
                self.bump();
            }
            if !self.at(Tok::KwCase) {
                // A token between the cases of an indented match: skipped, its nested regions
                // with it, up to the next case or the region's end.
                if indented && !matches!(self.kind(), Tok::Outdent | Tok::Eof) {
                    let (at, found) = self.found();
                    self.error_at(at, format!("expected 'case', found {}", found));
                    self.skip_one(RecoverySite::Cases);
                    continue;
                }
                break;
            }
            self.bump();
            let mark = self.error_events;
            let pat = self.parse_pattern();
            if self.at(Tok::Newline) && self.kind_at(1) == Tok::KwIf {
                self.bump();
            }
            let guard = if self.eat(Tok::KwIf) { Some(self.parse_infix_expr()) } else { None };
            if self.error_events != mark {
                broken.push(cases.len() as u32);
            }
            let arrow = self.expect(Tok::Arrow);
            // Without its `=>`, the lines after the case up to the next one are its body: the
            // lexer opened no region for them.
            let body = if !arrow && self.at(Tok::Newline) && !matches!(self.kind_at(1), Tok::KwCase | Tok::Outdent | Tok::Eof) {
                self.parse_case_lines()
            } else {
                self.parse_case_body()
            };
            cases.push(CaseClause { pat, guard, body });
        }
        if indented {
            self.end_region();
        }
        if cases.is_empty() {
            self.error_at(self.span(), "expected 'case'");
        }
        let l = push_list(&mut self.ast.cases, &cases);
        self.ast.broken_cases.extend(broken.into_iter().map(|i| l.start + i));
        l
    }

    /// The statements up to the next `case` or the end of the cases' region, as a block.
    fn parse_case_lines(&mut self) -> ExprId {
        let start = self.span();
        let mut stmts: Vec<Stmt> = Vec::new();
        loop {
            self.skip_separators();
            if matches!(self.kind(), Tok::KwCase | Tok::Outdent | Tok::Eof) {
                break;
            }
            let before = self.pos;
            self.parse_block_stmt(&mut stmts);
            if self.pos == before {
                self.skip_one(RecoverySite::Cases);
            } else {
                self.statement_end();
            }
        }
        let l = push_list(&mut self.ast.stmts, &stmts);
        self.ast.add_expr(Expr::Block(l), start.to(self.prev_span()))
    }

    fn parse_case_body(&mut self) -> ExprId {
        if self.at(Tok::Indent) {
            return self.parse_block();
        }
        if matches!(self.kind(), Tok::Newline | Tok::Outdent) {
            return self.ast.add_expr(Expr::UnitLit, self.span());
        }
        let start = self.span();
        let mut stmts: Vec<Stmt> = Vec::new();
        loop {
            self.parse_block_stmt(&mut stmts);
            if !self.eat(Tok::Semi) || matches!(self.kind(), Tok::Newline | Tok::Outdent | Tok::Eof | Tok::KwCase) {
                break;
            }
        }
        if let [Stmt::Expr(e)] = stmts[..] {
            return e;
        }
        let l = push_list(&mut self.ast.stmts, &stmts);
        self.ast.add_expr(Expr::Block(l), start.to(self.prev_span()))
    }

    fn parse_for(&mut self) -> ExprId {
        let start = self.span();
        self.expect(Tok::KwFor);
        let mut enums: Vec<Enumerator> = Vec::new();
        let indented = self.eat(Tok::Indent);
        let parens = !indented && self.at(Tok::LParen) && !self.parens_then_generator();
        if parens {
            self.bump();
        }
        loop {
            if indented {
                self.skip_separators();
                if matches!(self.kind(), Tok::Outdent | Tok::Eof) {
                    break;
                }
            }
            let before = self.pos;
            if self.eat(Tok::KwIf) {
                let g = self.parse_infix_expr();
                enums.push(Enumerator::Guard(g));
            } else {
                let filtering = self.eat(Tok::KwCase);
                let pat = self.parse_pattern();
                if self.eat(Tok::LArrow) {
                    let e = self.parse_expr();
                    enums.push(if filtering { Enumerator::CaseGen(pat, e) } else { Enumerator::Gen(pat, e) });
                } else {
                    if filtering {
                        self.error_at(self.span(), "expected '<-' after a case pattern");
                    }
                    self.expect(Tok::Eq);
                    let e = self.parse_expr();
                    enums.push(Enumerator::Val(pat, e));
                }
            }
            if self.at(Tok::KwIf) {
                continue;
            }
            if indented {
                if self.pos == before {
                    self.skip_one(RecoverySite::ForEnumerators);
                } else if !matches!(self.kind(), Tok::Newline | Tok::Semi | Tok::Outdent | Tok::Eof) {
                    let (at, found) = self.found();
                    self.error_at(at, format!("expected end of enumerator, found {}", found));
                    self.quiet = true;
                }
            } else if !self.eat(Tok::Semi) {
                break;
            }
            if self.at(Tok::Eof) {
                break;
            }
        }
        if indented {
            self.expect(Tok::Outdent);
            self.quiet = false;
        }
        if parens {
            self.expect(Tok::RParen);
        }
        if matches!(self.kind(), Tok::KwYield | Tok::KwDo) {
            self.reject_end_marker_before("'yield' or 'do'");
        }
        let is_yield = self.at(Tok::KwYield);
        if !self.eat(Tok::KwYield) && !self.eat(Tok::KwDo) {
            // Enclosed enumerators take their body without `do`, as in Scala 2.
            if !indented && !parens {
                let at = if self.at(Tok::Newline) { self.prev_span() } else { self.span() };
                self.error_at(at, "`yield` or `do` expected");
            }
            self.eat(Tok::Newline);
        }
        let body = self.parse_block_or_expr();
        // Aliases in front of the first generator are vals of a block around the expression.
        let leading = enums.iter().take_while(|e| matches!(e, Enumerator::Val(..))).count();
        if !matches!(enums.get(leading), Some(Enumerator::Gen(..) | Enumerator::CaseGen(..))) {
            self.error_at(start, "a for expression needs a generator");
            return self.ast.add_expr(Expr::Error, start.to(self.prev_span()));
        }
        let l = push_list(&mut self.ast.enumerators, &enums[leading..]);
        let span = start.to(self.prev_span());
        let for_expr = self.ast.add_expr(Expr::For(l, body, is_yield), span);
        if leading == 0 {
            return for_expr;
        }
        let mut stmts: Vec<Stmt> = Vec::new();
        for e in &enums[..leading] {
            if let Enumerator::Val(pat_id, rhs) = *e {
                let (name, pat) = match self.ast.pat(pat_id) {
                    Pat::Bind(n, None) => (n, None),
                    _ => (names::EMPTY, Some(pat_id)),
                };
                let kind = DefKind::Val { pat, ty: None, rhs: Some(rhs) };
                let def = Def { name, span: self.ast.pat_spans[pat_id.idx()], mods: 0, annots: Vec::new(), kind };
                stmts.push(Stmt::Def(self.ast.add_def(def)));
            }
        }
        stmts.push(Stmt::Expr(for_expr));
        let l = push_list(&mut self.ast.stmts, &stmts);
        self.ast.add_expr(Expr::Block(l), span)
    }

    /// Distinguishes `for (a, b) <- pairs` from `for (x <- xs) do`, as dotc's
    /// `followingIsEnclosedGenerators` does: a `<-` right after the parens makes them a
    /// pattern, an identifier makes them the enumerators, and otherwise the enumerators
    /// hold a `<-` before anything that has to start a statement.
    fn parens_then_generator(&self) -> bool {
        let mut depth = 0usize;
        let mut i = 0usize;
        loop {
            match self.kind_at(i) {
                Tok::LParen => depth += 1,
                Tok::RParen => {
                    depth -= 1;
                    if depth == 0 {
                        return match self.kind_at(i + 1) {
                            Tok::LArrow => true,
                            Tok::Ident | Tok::OpIdent => false,
                            _ => !self.followed_by_token(Tok::LArrow),
                        };
                    }
                }
                Tok::Eof => return false,
                _ => {}
            }
            i += 1;
        }
    }

    pub(super) fn parse_infix_expr(&mut self) -> ExprId {
        self.parse_infix_prec(0)
    }

    /// `*` closing an argument: the token at `n` is the star of `xs: _*` or `xs*`.
    pub(super) fn spread_star_at(&self, n: usize) -> bool {
        self.kind_at(n) == Tok::OpIdent
            && self.toks[self.pos + n].name == names::STAR
            && matches!(self.kind_at(n + 1), Tok::RParen | Tok::Comma)
    }

    /// A brace block after an alphanumeric operator is its argument: `xs map { x => x }`.
    fn can_start_operand(&self, n: usize) -> bool {
        match self.kind_at(n) {
            Tok::Ident | Tok::OpIdent | Tok::IntLit | Tok::LongLit | Tok::DoubleLit | Tok::FloatLit | Tok::CharLit
            | Tok::StringLit | Tok::InterpStart | Tok::LParen | Tok::Underscore | Tok::KwThis | Tok::KwSuper
            | Tok::KwTrue | Tok::KwFalse | Tok::KwNull | Tok::KwNew | Tok::Quote | Tok::QuoteId | Tok::Splice => true,
            Tok::ColonEol => self.kind_at(n + 1) == Tok::Indent,
            _ => false,
        }
    }

    /// An alphanumeric operator ending its line continues on the next one when that line starts
    /// an operand of dotc's `canStartInfixExprTokens`: `x shouldBe` then the value. An `if`, a
    /// `while` or a `throw` there ends the statement instead, as under dotc, whose lookahead
    /// scans the next token without inserting the newline.
    fn ident_is_operator(&self) -> bool {
        match self.kind_at(1) {
            Tok::Newline => matches!(
                self.kind_at(2),
                Tok::Ident | Tok::OpIdent | Tok::IntLit | Tok::LongLit | Tok::DoubleLit | Tok::FloatLit | Tok::CharLit
                    | Tok::StringLit | Tok::InterpStart | Tok::LParen | Tok::LBracket | Tok::LBrace | Tok::Indent
                    | Tok::Underscore | Tok::KwThis | Tok::KwSuper | Tok::KwTrue | Tok::KwFalse | Tok::KwNull
                    | Tok::KwReturn | Tok::KwNew | Tok::Quote | Tok::QuoteId | Tok::Splice
            ),
            _ => self.can_start_operand(1),
        }
    }

    /// dotc's `nextCanFollowOperator` for a symbolic operator: an `if`, a `while`, a `for`, a
    /// `try` or a `throw` after it, on its line or the next, cannot be its operand, and the
    /// statement ends before the operator (`true` then `&& try ...` on the lines after it); nor
    /// can a token that starts no expression, a definition's keyword among them.
    fn symbolic_is_operator(&self) -> bool {
        let n = if self.kind_at(1) == Tok::Newline { 2 } else { 1 };
        !matches!(self.kind_at(n), Tok::KwIf | Tok::KwWhile | Tok::KwFor | Tok::KwTry | Tok::KwThrow)
            && (self.expr_start_at(n) || matches!(self.kind_at(n), Tok::LBracket | Tok::Eof))
    }

    fn parse_infix_prec(&mut self, min_prec: u8) -> ExprId {
        let start = self.span();
        let lhs = self.parse_prefix_expr();
        self.parse_infix_rest(lhs, start, min_prec)
    }

    fn parse_infix_rest(&mut self, mut lhs: ExprId, start: Span, min_prec: u8) -> ExprId {
        loop {
            let t = self.tok();
            let is_op = match t.kind {
                Tok::OpIdent => {
                    !(t.name == names::STAR && matches!(self.kind_at(1), Tok::RParen | Tok::Comma)) && self.symbolic_is_operator()
                }
                Tok::Ident => t.name != names::EMPTY && self.ident_is_operator(),
                _ => false,
            };
            if !is_op {
                return lhs;
            }
            let (prec, right) = op_precedence(self.interner.get(t.name));
            if prec < min_prec {
                return lhs;
            }
            self.bump();
            // The operand may follow on the next line, not past a blank one (dotc's `NEWLINES`).
            if self.at(Tok::Newline) {
                if self.blank_line_before(0) {
                    self.error_at(Span::new(t.span.end, t.span.end), "expected an expression, found a blank line");
                    return lhs;
                }
                self.bump();
            }
            let rhs = if prec == 1 && self.at(Tok::Indent) {
                self.parse_block_operand(if right { prec } else { prec + 1 })
            } else {
                self.parse_infix_prec(if right { prec } else { prec + 1 })
            };
            lhs = self.ast.add_expr(Expr::Infix(lhs, t.name, rhs), start.to(self.prev_span()));
            self.note_name(lhs, t.span);
        }
    }

    pub(super) fn parse_prefix_expr(&mut self) -> ExprId {
        let t = self.tok();
        if t.kind == Tok::OpIdent
            && matches!(t.name, names::MINUS | names::PLUS | names::BANG | names::TILDE)
        {
            self.bump();
            // A minus before a number token is the number's sign, whatever lies between them on
            // the line (dotc's `prefixExpr`): `- 1.5` is the literal `-1.5`, a `Float` where one
            // is expected; `-(1.5)` stays the operation.
            if t.name == names::MINUS && matches!(self.kind(), Tok::IntLit | Tok::LongLit | Tok::DoubleLit | Tok::FloatLit) {
                let lit = self.parse_literal();
                let negated = match self.ast.expr(lit) {
                    Expr::IntLit(v) => Expr::IntLit(-v),
                    Expr::LongLit(v) => Expr::LongLit(v.wrapping_neg()),
                    Expr::DoubleLit(v) => Expr::DoubleLit(-v),
                    Expr::DecimalLit(s) => {
                        self.ast.strings[s.idx()].insert(0, '-');
                        Expr::DecimalLit(s)
                    }
                    Expr::FloatLit(v) => Expr::FloatLit(-v),
                    other => other,
                };
                self.ast.exprs[lit.idx()] = negated;
                self.ast.expr_spans[lit.idx()] = t.span.to(self.prev_span());
                return self.parse_postfix(lit, t.span);
            }
            let operand = self.parse_simple_expr();
            let e = self.ast.add_expr(Expr::Prefix(t.name, operand), t.span.to(self.prev_span()));
            self.note_name(e, t.span);
            return e;
        }
        self.parse_simple_expr()
    }

    pub(super) fn parse_literal(&mut self) -> ExprId {
        let t = self.bump();
        let text = &self.text[t.span.start as usize..t.span.end as usize];
        let e = match t.kind {
            Tok::IntLit | Tok::LongLit => {
                let clean: String = text.chars().filter(|&c| c != '_').collect();
                let parsed = if let Some(hex) = clean.strip_prefix("0x").or(clean.strip_prefix("0X")) {
                    u64::from_str_radix(hex, 16).ok().map(|v| {
                        if t.kind == Tok::IntLit && v <= u32::MAX as u64 {
                            v as u32 as i32 as i64
                        } else {
                            v as i64
                        }
                    })
                } else {
                    clean.parse::<u64>().ok().map(|v| v as i64)
                };
                let v = parsed.unwrap_or_else(|| {
                    self.error_at(t.span, "number too large");
                    0
                });
                if t.kind == Tok::IntLit { Expr::IntLit(v) } else { Expr::LongLit(v) }
            }
            Tok::DoubleLit => {
                let clean: String = text.chars().filter(|&c| c != '_').collect();
                // The token's span leaves its suffix out (`lexer.rs`): a `d` or `D` the lexer took
                // as one follows the digits.
                let rest = &self.text.as_bytes()[t.span.end as usize..];
                let suffixed = matches!(rest.first(), Some(b'd' | b'D')) && !rest.get(1).is_some_and(|&c| crate::lexer::is_ident_part(c));
                if suffixed {
                    let v = clean.parse::<f64>().unwrap_or(0.0);
                    self.number_range(v.is_infinite(), v == 0.0 && !zero_digits(&clean), t.span);
                    Expr::DoubleLit(v)
                } else {
                    Expr::DecimalLit(self.ast.add_str(clean))
                }
            }
            Tok::FloatLit => {
                let clean: String = text.chars().filter(|&c| c != '_').collect();
                let v = clean.parse::<f32>().unwrap_or(0.0);
                self.number_range(v.is_infinite(), v == 0.0 && !zero_digits(&clean), t.span);
                Expr::FloatLit(v)
            }
            Tok::CharLit => {
                let inner = &text[1..text.len().saturating_sub(1).max(1)];
                let s = unescape(inner, false, &mut self.errors, t.span);
                let mut units = crate::text::utf16_units(&s);
                let c = units.next().unwrap_or(0) as u32;
                if units.next().is_some() {
                    self.error_at(t.span, "character literal does not fit in a Char");
                }
                Expr::CharLit(c)
            }
            Tok::StringLit => {
                let s = if text.starts_with("\"\"\"") && text.len() >= 6 {
                    text[3..text.len() - 3].to_string()
                } else {
                    let inner = &text[1..text.len().saturating_sub(1).max(1)];
                    unescape(inner, false, &mut self.errors, t.span)
                };
                Expr::StringLit(self.ast.add_str(s))
            }
            Tok::KwTrue => Expr::BoolLit(true),
            Tok::KwFalse => Expr::BoolLit(false),
            Tok::KwNull => Expr::NullLit,
            _ => Expr::Error,
        };
        self.ast.add_expr(e, t.span)
    }

    /// A suffixed number out of its type's range, as dotc's parser reports it
    /// (`FromDigits.floatFromDigits`, `doubleFromDigits`): infinite, or zero from nonzero digits.
    fn number_range(&mut self, large: bool, small: bool, span: Span) {
        if large {
            self.error_at(span, "number too large");
        } else if small {
            self.error_at(span, "number too small");
        }
    }

    fn parse_interp(&mut self) -> ExprId {
        let head = self.bump();
        // Only `s` gets its escapes processed here; StringContext.parts holds the text as written.
        let raw = head.name != names::S_INTERP;
        let mut parts: Vec<StrId> = Vec::new();
        let mut spans: Vec<Span> = Vec::new();
        let mark = self.expr_scratch.len();
        loop {
            match self.kind() {
                Tok::StrPart => {
                    let t = self.bump();
                    let text = &self.text[t.span.start as usize..t.span.end as usize];
                    let s = if raw {
                        text.replace("$$", "$")
                    } else {
                        unescape(text, true, &mut self.errors, t.span)
                    };
                    parts.push(self.ast.add_str(s));
                    spans.push(t.span);
                }
                Tok::Ident => {
                    let t = self.bump();
                    let e = self.ast.add_expr(Expr::Ident(t.name), t.span);
                    self.expr_scratch.push(e);
                }
                Tok::KwThis => {
                    let t = self.bump();
                    let e = self.ast.add_expr(Expr::This, t.span);
                    self.expr_scratch.push(e);
                }
                Tok::LBrace => {
                    self.bump();
                    let e = self.parse_expr();
                    self.expr_scratch.push(e);
                    self.expect(Tok::RBrace);
                }
                Tok::InterpEnd => {
                    self.bump();
                    break;
                }
                _ => {
                    self.error_at(self.span(), "malformed interpolated string");
                    break;
                }
            }
        }
        let args = self.expr_list(mark);
        let parts = push_list(&mut self.ast.str_lists, &parts);
        self.ast.set_part_spans(parts, &spans);
        self.ast.add_expr(Expr::Interp(head.name, parts, args), head.span.to(self.prev_span()))
    }

    fn placeholder(&mut self, span: Span) -> (Name, ExprId) {
        let name = self.placeholder_name(self.placeholders.len());
        self.placeholders.push(LambdaParam { name, span, ty: None, implicit: false, contextual: false });
        (name, self.ast.add_expr(Expr::Ident(name), span))
    }

    fn parse_simple_expr(&mut self) -> ExprId {
        let start = self.span();
        let e = match self.kind() {
            Tok::IntLit | Tok::LongLit | Tok::DoubleLit | Tok::FloatLit | Tok::CharLit | Tok::StringLit
            | Tok::KwTrue | Tok::KwFalse | Tok::KwNull => self.parse_literal(),
            Tok::InterpStart => self.parse_interp(),
            // `inline` is a soft keyword: an expression start after it makes it one, as scalac reads it.
            Tok::Ident if self.tok().name == names::INLINE && self.kind_at(1) != Tok::Arrow && self.expr_start_at(1) => {
                self.bump();
                let e = self.parse_expr();
                match self.ast.expr(e) {
                    Expr::If(c, t, els) => self.ast.exprs[e.idx()] = Expr::InlineIf(c, t, els),
                    Expr::Match(s, cases) => self.ast.exprs[e.idx()] = Expr::InlineMatch(s, cases),
                    _ => self.error_at(start, "`inline` must be followed by an `if` or a `match`"),
                }
                return e;
            }
            // Inside a quote an identifier `$x` splices `x`; in a quote pattern it binds `x`.
            Tok::Ident if self.quote_level > 0 && self.spliced_name().is_some() => {
                let t = self.bump();
                let name = self.spliced_name_of(t.name);
                let inner_span = Span::new(t.span.start + 1, t.span.end);
                if self.quote_pattern {
                    let p = self.ast.add_pat(Pat::Bind(name, None), inner_span);
                    self.ast.add_expr(Expr::SplicePat(p), t.span)
                } else {
                    let inner = self.ast.add_expr(Expr::Ident(name), inner_span);
                    self.ast.add_expr(Expr::Splice(inner), t.span)
                }
            }
            Tok::Ident | Tok::OpIdent => {
                let t = self.bump();
                self.ast.add_expr(Expr::Ident(t.name), t.span)
            }
            Tok::Quote => self.parse_quote(false),
            Tok::QuoteId => {
                let t = self.bump();
                self.ast.has_quotes = true;
                let inner = self.ast.add_expr(Expr::Ident(t.name), Span::new(t.span.start + 1, t.span.end));
                self.ast.add_expr(Expr::Quote(inner), t.span)
            }
            Tok::Splice if self.quote_pattern => {
                self.bump();
                self.expect(Tok::Indent);
                let outer = (self.quote_level, self.quote_pattern);
                self.quote_level = 0;
                self.quote_pattern = false;
                let p = self.parse_pattern();
                (self.quote_level, self.quote_pattern) = outer;
                self.skip_separators();
                self.eat(Tok::Outdent);
                self.ast.add_expr(Expr::SplicePat(p), start.to(self.prev_span()))
            }
            Tok::Splice => {
                self.bump();
                self.ast.has_quotes = true;
                let outer = (self.quote_level, self.quote_pattern);
                self.quote_level = 0;
                self.quote_pattern = false;
                let body = self.parse_block();
                (self.quote_level, self.quote_pattern) = outer;
                self.ast.add_expr(Expr::Splice(body), start.to(self.prev_span()))
            }
            Tok::KwThis => {
                self.bump();
                self.ast.add_expr(Expr::This, start)
            }
            Tok::KwSuper => {
                self.bump();
                let mut parent = names::EMPTY;
                if self.at(Tok::LBracket) && self.kind_at(1) == Tok::Ident && self.kind_at(2) == Tok::RBracket {
                    self.bump();
                    parent = self.bump().name;
                    self.bump();
                }
                if !self.at(Tok::Dot) {
                    self.error_at(self.span(), "expected '.' after super");
                }
                self.ast.add_expr(Expr::Super(parent), start.to(self.prev_span()))
            }
            Tok::Underscore => {
                self.bump();
                self.placeholder(start).1
            }
            Tok::LParen => self.parse_parens(),
            Tok::LBracket if self.bracket_followed_by(Tok::Arrow) => {
                let (params, bounds) = self.parse_type_params_of_lambda();
                self.expect(Tok::Arrow);
                let lambda = self.parse_expr();
                if !matches!(self.ast.expr(lambda), Expr::Lambda(..)) {
                    self.error_at(start, "a polymorphic function literal takes a function literal after `=>`");
                }
                let l = push_list(&mut self.ast.name_lists, &params);
                let id = self.ast.add_expr(Expr::PolyLambda(l, lambda), start.to(self.prev_span()));
                if let Some(bl) = self.lambda_bounds_list(&bounds) {
                    self.ast.poly_lambda_bounds.push((id, bl));
                }
                id
            }
            Tok::Indent => self.parse_block(),
            Tok::ColonEol if self.kind_at(1) == Tok::Indent => {
                self.bump();
                self.parse_block()
            }
            Tok::KwIf => self.parse_if(),
            Tok::KwNew => {
                self.bump();
                self.template_type = true;
                let ty = self.parse_simple_type();
                self.template_type = false;
                let mut args = Vec::new();
                let mut cut = false;
                while self.at(Tok::LParen) {
                    args.push(self.parse_args());
                    cut |= self.args_cut;
                }
                let has_body = self.at(Tok::ColonEol) && self.kind_at(1) == Tok::Indent;
                if self.at(Tok::KwWith) || has_body {
                    self.parse_anon_class(ty, args, start)
                } else {
                    // `new C(a)(using b)`: the lists after the first apply to the instance.
                    // A first list that is a using list stays an application of it.
                    let first_using = args.first().map_or(false, |&(_, using)| using);
                    let first = if first_using { ListRef::EMPTY } else { args.first().map_or(ListRef::EMPTY, |&(l, _)| l) };
                    let mut e = self.ast.add_expr(Expr::New(ty, first), start.to(self.prev_span()));
                    self.note_cut_args(e, cut);
                    if let Some(outer) = self.prefix_value(ty, start.to(self.prev_span())) {
                        self.ast.new_outers.insert(e, outer);
                    }
                    if args.len() > 1 && !first_using && first.is_empty() {
                        self.ast.new_empty_first.push(e);
                    }
                    for &(list, using) in args.iter().skip(if first_using { 0 } else { 1 }) {
                        let node = if using { Expr::UsingApply(e, list) } else { Expr::Apply(e, list) };
                        e = self.ast.add_expr(node, start.to(self.prev_span()));
                    }
                    e
                }
            }
            _ => {
                let found = self.kind().describe();
                self.error_at(start, format!("expected an expression, found {}", found));
                return self.ast.add_expr(Expr::Error, start);
            }
        };
        self.parse_postfix(e, start)
    }

    /// The name behind the `$` of the identifier at the cursor, when it is a splice.
    fn spliced_name(&self) -> Option<Name> {
        let text = self.interner.get(self.tok().name);
        (text.len() > 1 && text.starts_with('$') && !text.starts_with("$$")).then_some(self.tok().name)
    }

    fn spliced_name_of(&mut self, name: Name) -> Name {
        let text = self.interner.get(name)[1..].to_string();
        self.interner.intern(&text)
    }

    /// `'{ ... }` or `'[T]`; in a pattern the splices of the body hold patterns.
    pub(super) fn parse_quote(&mut self, pattern: bool) -> ExprId {
        let start = self.span();
        self.ast.has_quotes = true;
        self.expect(Tok::Quote);
        if self.at(Tok::LBracket) {
            self.bump();
            let t = self.parse_type();
            self.expect(Tok::RBracket);
            return self.ast.add_expr(Expr::QuoteType(t), start.to(self.prev_span()));
        }
        let outer = (self.quote_level, self.quote_pattern);
        self.quote_level += 1;
        self.quote_pattern = pattern;
        let body = self.parse_block();
        (self.quote_level, self.quote_pattern) = outer;
        self.ast.add_expr(Expr::Quote(body), start.to(self.prev_span()))
    }

    fn parse_parens(&mut self) -> ExprId {
        let start = self.span();
        self.expect(Tok::LParen);
        let mark = self.expr_scratch.len();
        let mut cut = false;
        // `(` at the end of the file opens an expression that is missing, as scalac reads it,
        // and no `()`.
        while !self.at(Tok::RParen) && !(self.at(Tok::Eof) && self.expr_scratch.len() > mark) {
            let e = self.parse_block_or_expr();
            self.expr_scratch.push(e);
            if !self.list_continues(Tok::RParen, &mut cut) {
                break;
            }
        }
        self.expect(Tok::RParen);
        let span = start.to(self.prev_span());
        match self.expr_scratch.len() - mark {
            0 => self.ast.add_expr(Expr::UnitLit, span),
            1 => {
                let only = self.expr_scratch[mark];
                self.expr_scratch.truncate(mark);
                // `(_: Int)` carries its ascription over to the placeholder parameter.
                if let Expr::Typed(inner, ty) = self.ast.expr(only) {
                    if let (Expr::Ident(n), Some(last)) =
                        (self.ast.expr(inner), self.placeholders.last_mut())
                    {
                        if last.name == n {
                            last.ty = Some(ty);
                            return inner;
                        }
                    }
                }
                if matches!(self.ast.expr(only), Expr::Tuple(_)) {
                    return self.ast.add_expr(Expr::Parens(only), span);
                }
                only
            }
            _ => {
                let l = self.expr_list(mark);
                self.tuple_or_named(l, span)
            }
        }
    }

    /// `(a = x, b = y)`, every element an assignment to a name, is a named tuple.
    fn tuple_or_named(&mut self, l: ListRef, span: Span) -> ExprId {
        let items = self.ast.expr_list(l).to_vec();
        let mut names = Vec::with_capacity(items.len());
        let mut values = Vec::with_capacity(items.len());
        for &e in &items {
            if let Expr::Assign(lhs, rhs) = self.ast.expr(e) {
                if let Expr::Ident(n) = self.ast.expr(lhs) {
                    names.push(n);
                    values.push(rhs);
                }
            }
        }
        if names.is_empty() {
            return self.ast.add_expr(Expr::Tuple(l), span);
        }
        if names.len() != items.len() {
            self.error_at(span, "Illegal combination of named and unnamed tuple elements");
            return self.ast.add_expr(Expr::Tuple(l), span);
        }
        let nl = push_list(&mut self.ast.name_lists, &names);
        let vl = push_list(&mut self.ast.expr_lists, &values);
        self.ast.add_expr(Expr::NamedTuple(nl, vl), span)
    }

    /// Whether the `ColonEol` at the current token passes the indented block after it as an
    /// argument: always one the lexer put before a brace block, and a written `:` after what
    /// scalac's scanner makes a `COLONfollow` of, an alphanumeric or backquoted identifier, `)`,
    /// `]`, `this`, `super` or `new`; after a literal or `_` (`(1:`) the type of an ascription
    /// follows. A `:` after a closed region (`}`, an outdent) stays an argument: scalac rejects
    /// it there either way.
    fn colon_takes_argument(&self) -> bool {
        if self.text.as_bytes()[self.span().start as usize] != b':' {
            return true;
        }
        let Some(prev) = self.pos.checked_sub(1).map(|i| self.toks[i]) else { return true };
        matches!(prev.kind, Tok::Ident | Tok::RParen | Tok::RBracket | Tok::KwThis | Tok::KwSuper | Tok::KwNew | Tok::Outdent)
    }

    /// Detects `: x =>` / `: (x, y) =>` followed by an indented body.
    fn at_colon_lambda(&self) -> bool {
        if !self.at(Tok::Colon) {
            return false;
        }
        let mut i = 1usize;
        match self.kind_at(1) {
            Tok::Ident | Tok::Underscore => i += 1,
            Tok::LParen => {
                let mut depth = 0usize;
                loop {
                    match self.kind_at(i) {
                        Tok::LParen | Tok::LBracket => depth += 1,
                        Tok::RParen | Tok::RBracket => {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                        Tok::Eof | Tok::Newline | Tok::Indent | Tok::Outdent => return false,
                        _ => {}
                    }
                    i += 1;
                }
                i += 1;
            }
            _ => return false,
        }
        matches!(self.kind_at(i), Tok::Arrow | Tok::CtxArrow) && self.kind_at(i + 1) == Tok::Indent
    }

    pub(super) fn parse_postfix(&mut self, mut e: ExprId, start: Span) -> ExprId {
        loop {
            match self.kind() {
                Tok::Dot => {
                    self.bump();
                    if self.at(Tok::KwMatch) {
                        self.bump();
                        let cases = self.parse_cases();
                        e = self.ast.add_expr(Expr::Match(e, cases), start.to(self.prev_span()));
                        continue;
                    }
                    if self.at(Tok::KwThis) {
                        if let Expr::Ident(q) | Expr::Select(_, q) = self.ast.expr(e) {
                            self.bump();
                            e = self.ast.add_expr(Expr::QualThis(q), start.to(self.prev_span()));
                            continue;
                        }
                    }
                    let (n, name_span) = self.expect_ident();
                    e = self.ast.add_expr(Expr::Select(e, n), start.to(self.prev_span()));
                    self.note_name(e, name_span);
                }
                Tok::LBracket => {
                    let targs = self.parse_type_args();
                    e = self.ast.add_expr(Expr::TypeApply(e, targs), start.to(self.prev_span()));
                }
                Tok::LParen => {
                    let (args, using) = self.parse_args();
                    let cut = self.args_cut;
                    let node = if using { Expr::UsingApply(e, args) } else { Expr::Apply(e, args) };
                    e = self.ast.add_expr(node, start.to(self.prev_span()));
                    self.note_cut_args(e, cut);
                }
                Tok::ColonEol if self.kind_at(1) == Tok::Indent && self.colon_takes_argument() => {
                    self.bump();
                    let body = self.parse_block();
                    let args = push_list(&mut self.ast.expr_lists, &[body]);
                    e = self.ast.add_expr(Expr::Apply(e, args), start.to(self.prev_span()));
                }
                Tok::Colon if self.at_colon_lambda() => {
                    self.bump();
                    let lambda = self.parse_colon_lambda();
                    let args = push_list(&mut self.ast.expr_lists, &[lambda]);
                    e = self.ast.add_expr(Expr::Apply(e, args), start.to(self.prev_span()));
                }
                // `f _` of Scala 2: the method is a function value on its own.
                Tok::Underscore
                    if matches!(
                        self.kind_at(1),
                        Tok::RParen | Tok::RBracket | Tok::Comma | Tok::Newline | Tok::Semi | Tok::Outdent | Tok::Eof
                    ) =>
                {
                    self.bump();
                }
                _ => return e,
            }
        }
    }

    /// An argument list, whether it is a `using` one. One whose recovery skipped tokens or
    /// stopped short of its `)` is cut (`Ast::cut_args`): noted by its first argument, or, when
    /// it holds none, in `args_cut` for the application the caller makes.
    pub(super) fn parse_args(&mut self) -> (ListRef, bool) {
        self.expect(Tok::LParen);
        let mut cut = false;
        let using = self.at_soft(names::USING) && self.kind_at(1) != Tok::RParen
            && !matches!(self.kind_at(1), Tok::Comma | Tok::Dot | Tok::LParen);
        if using {
            self.bump();
        }
        let mark = self.expr_scratch.len();
        while !self.at(Tok::RParen) && !self.at(Tok::Eof) {
            let arg_start = self.span();
            let arg = if self.at(Tok::Ident) && self.kind_at(1) == Tok::Eq {
                let name = self.bump().name;
                self.bump();
                let value = self.parse_block_or_expr();
                self.ast.add_expr(Expr::NamedArg(name, value), arg_start.to(self.prev_span()))
            } else {
                let value = self.parse_block_or_expr();
                if self.at_op(names::STAR) {
                    let star = self.bump().span;
                    let wild = self.ast.add_ty(TyExpr::Wildcard, star);
                    let rep = self.ast.add_ty(TyExpr::Repeated(wild), star);
                    self.ast.add_expr(Expr::Typed(value, rep), arg_start.to(star))
                } else {
                    value
                }
            };
            self.expr_scratch.push(arg);
            if !self.list_continues(Tok::RParen, &mut cut) {
                break;
            }
        }
        if !self.expect(Tok::RParen) {
            cut = true;
        }
        let first = self.expr_scratch.get(mark).copied();
        match first {
            Some(head) if cut => self.ast.cut_args.push(head),
            _ => {}
        }
        self.args_cut = cut && first.is_none();
        (self.expr_list(mark), using)
    }
}

/// Whether a number's digits before its exponent are all zeros (`FromDigits`' `zeroFloat`).
fn zero_digits(digits: &str) -> bool {
    digits.bytes().take_while(|&c| c != b'e' && c != b'E').all(|c| matches!(c, b'0' | b'.'))
}
