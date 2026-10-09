use super::{unescape, Parser};
use crate::ast::*;
use crate::names;
use crate::source::Span;
use crate::token::Tok;

impl<'a> Parser<'a> {
    pub(super) fn parse_pattern(&mut self) -> PatId {
        let start = self.span();
        let first = self.parse_pattern1();
        if !self.at_op(names::BAR) {
            return first;
        }
        let mark = self.pat_scratch.len();
        self.pat_scratch.push(first);
        while self.eat_op(names::BAR) {
            let p = self.parse_pattern1();
            self.pat_scratch.push(p);
        }
        let l = self.pat_list(mark);
        self.ast.add_pat(Pat::Alt(l), start.to(self.prev_span()))
    }

    fn parse_pattern1(&mut self) -> PatId {
        let start = self.span();
        let p = self.parse_infix_pattern();
        if self.at(Tok::Colon) && matches!(self.ast.pat(p), Pat::Wildcard | Pat::Bind(_, None)) {
            // `rest: _*` is the Scala 2 spelling of `rest*`.
            if self.kind_at(1) == Tok::Underscore && self.spread_star_at(2) {
                self.bump();
                self.bump();
                self.bump();
                return self.ast.add_pat(Pat::Rest(p), start.to(self.prev_span()));
            }
            self.bump();
            let mut t = self.parse_inter_type();
            // The `=>` of the case follows `(Int | String)`; it does not make the type a parameter list.
            if let TyExpr::Tuple(l) = self.ast.ty(t) {
                if l.len == 1 {
                    t = self.ast.ty_list(l)[0];
                }
            }
            self.unbind_type_variables(t);
            return self.ast.add_pat(Pat::Typed(p, t), start.to(self.prev_span()));
        }
        p
    }

    /// A lowercase type argument of a pattern type (`case l: List[t]`) is a type variable the
    /// case binds; a backquoted one (`` List[`t`] ``) names the type `t` in scope.
    fn unbind_type_variables(&mut self, t: TyExprId) {
        let (TyExpr::Apply(_, args) | TyExpr::Tuple(args)) = self.ast.ty(t) else { return };
        for i in 0..args.len {
            let a = self.ast.ty_lists[(args.start + i) as usize];
            let backquoted = self.text.as_bytes().get(self.ast.ty_spans[a.idx()].start as usize) == Some(&b'`');
            match self.ast.ty(a) {
                TyExpr::Name(n) if !backquoted && self.interner.get(n).starts_with(|c: char| c.is_lowercase()) => {
                    self.ast.tys[a.idx()] = TyExpr::TypeVar(n);
                }
                TyExpr::Apply(..) | TyExpr::Tuple(..) => self.unbind_type_variables(a),
                _ => {}
            }
        }
    }

    /// A lowercase name of a quoted type pattern (`case '[t]`, `case '[List[t]]`) is a type
    /// variable the case binds.
    fn bind_quote_type_variables(&mut self, t: TyExprId) {
        match self.ast.ty(t) {
            TyExpr::Name(n) if self.interner.get(n).starts_with(|c: char| c.is_lowercase()) => {
                self.ast.tys[t.idx()] = TyExpr::TypeVar(n);
            }
            TyExpr::Apply(f, args) => {
                self.bind_quote_type_variables(f);
                for i in 0..args.len {
                    let a = self.ast.ty_lists[(args.start + i) as usize];
                    self.bind_quote_type_variables(a);
                }
            }
            TyExpr::Tuple(items) | TyExpr::Fun(items, _) => {
                for i in 0..items.len {
                    let a = self.ast.ty_lists[(items.start + i) as usize];
                    self.bind_quote_type_variables(a);
                }
                if let TyExpr::Fun(_, r) = self.ast.ty(t) {
                    self.bind_quote_type_variables(r);
                }
            }
            _ => {}
        }
    }

    fn parse_infix_pattern(&mut self) -> PatId {
        let start = self.span();
        let lhs = self.parse_simple_pattern();
        let seq_wildcard = self.at(Tok::OpIdent)
            && self.tok().name == names::STAR
            && matches!(self.kind_at(1), Tok::RParen | Tok::Comma);
        if seq_wildcard {
            let star = self.bump();
            if !matches!(self.ast.pat(lhs), Pat::Wildcard | Pat::Bind(_, None)) {
                self.error_at(star.span, "a sequence wildcard follows a variable or _");
            }
            return self.ast.add_pat(Pat::Rest(lhs), start.to(self.prev_span()));
        }
        if self.at(Tok::OpIdent) && self.tok().name != names::BAR {
            let op = self.bump();
            let rhs = self.parse_infix_pattern();
            // `1 +` with its operand missing extracts nothing.
            if matches!(self.ast.pat(rhs), Pat::Error) {
                return self.ast.add_pat(Pat::Error, start.to(self.prev_span()));
            }
            let path = self.ast.add_expr(Expr::Ident(op.name), op.span);
            let l = push_list(&mut self.ast.pat_lists, &[lhs, rhs]);
            return self.ast.add_pat(Pat::Ctor(path, l), start.to(self.prev_span()));
        }
        lhs
    }

    fn is_backquoted(&self) -> bool {
        self.text.as_bytes()[self.span().start as usize] == b'`'
    }


    fn parse_simple_pattern(&mut self) -> PatId {
        let start = self.span();
        match self.kind() {
            Tok::Underscore => {
                self.bump();
                self.ast.add_pat(Pat::Wildcard, start)
            }
            // `case given T =>` of a `summonFrom`: a typed wildcard.
            Tok::KwGiven => {
                self.bump();
                let t = self.parse_inter_type();
                let span = start.to(self.prev_span());
                let wildcard = self.ast.add_pat(Pat::Wildcard, start);
                let typed = self.ast.add_pat(Pat::Typed(wildcard, t), span);
                let name = self.given_name_from_type(t);
                let bind = self.ast.add_pat(Pat::Bind(name, Some(typed)), span);
                self.ast.given_binds.insert(bind);
                bind
            }
            Tok::Ident => {
                let t = self.tok();
                let text = self.interner.get(t.name);
                let is_var = text.starts_with(|c: char| c.is_lowercase() || c == '_')
                    && !self.is_backquoted()
                    && !matches!(self.kind_at(1), Tok::Dot | Tok::LParen);
                if is_var {
                    self.bump();
                    if self.eat(Tok::At) {
                        let inner = self.parse_infix_pattern();
                        let span = start.to(self.prev_span());
                        // `xs @ _*` is the Scala 2 spelling of `xs*`.
                        if let Pat::Rest(w) = self.ast.pat(inner) {
                            if matches!(self.ast.pat(w), Pat::Wildcard) {
                                let bind = self.ast.add_pat(Pat::Bind(t.name, None), span);
                                return self.ast.add_pat(Pat::Rest(bind), span);
                            }
                        }
                        return self.ast.add_pat(Pat::Bind(t.name, Some(inner)), span);
                    }
                    return self.ast.add_pat(Pat::Bind(t.name, None), start);
                }
                self.bump();
                let mut path = self.ast.add_expr(Expr::Ident(t.name), t.span);
                while self.at(Tok::Dot) && self.kind_at(1) == Tok::Ident {
                    self.bump();
                    let n = self.bump();
                    path = self.ast.add_expr(Expr::Select(path, n.name), start.to(n.span));
                    self.note_name(path, n.span);
                }
                if self.at(Tok::LParen) {
                    let l = self.parse_pattern_args();
                    return self.ast.add_pat(Pat::Ctor(path, l), start.to(self.prev_span()));
                }
                self.ast.add_pat(Pat::StableId(path), start.to(self.prev_span()))
            }
            Tok::Quote => {
                let e = self.parse_quote(true);
                let span = start.to(self.prev_span());
                match self.ast.expr(e) {
                    Expr::QuoteType(t) => {
                        self.bind_quote_type_variables(t);
                        self.ast.add_pat(Pat::QuoteType(t), span)
                    }
                    Expr::Quote(body) => self.ast.add_pat(Pat::Quote(body), span),
                    _ => self.ast.add_pat(Pat::Quote(e), span),
                }
            }
            Tok::LParen => {
                let l = self.parse_pattern_args();
                let span = start.to(self.prev_span());
                match l.len {
                    0 => {
                        let unit = self.ast.add_expr(Expr::UnitLit, span);
                        self.ast.add_pat(Pat::Lit(unit), span)
                    }
                    1 if !matches!(self.ast.pat(self.ast.pat_list(l)[0]), Pat::NamedField(..)) => self.ast.pat_list(l)[0],
                    _ => self.ast.add_pat(Pat::Tuple(l), span),
                }
            }
            Tok::IntLit | Tok::LongLit | Tok::DoubleLit | Tok::FloatLit | Tok::CharLit | Tok::StringLit
            | Tok::KwTrue | Tok::KwFalse | Tok::KwNull => {
                let e = self.parse_literal();
                self.ast.add_pat(Pat::Lit(e), start.to(self.prev_span()))
            }
            Tok::InterpStart => self.parse_interp_pattern(),
            Tok::OpIdent
                if self.tok().name == names::MINUS
                    && matches!(self.kind_at(1), Tok::IntLit | Tok::LongLit | Tok::DoubleLit | Tok::FloatLit) =>
            {
                let e = self.parse_prefix_expr();
                self.ast.add_pat(Pat::Lit(e), start.to(self.prev_span()))
            }
            _ => {
                let found = self.kind().describe();
                self.error_at(start, format!("expected a pattern, found {}", found));
                self.ast.add_pat(Pat::Error, start)
            }
        }
    }

    /// `s"a-$x-${Some(y)}"` in pattern position: the holes are patterns, `$x` a binder.
    fn parse_interp_pattern(&mut self) -> PatId {
        let head = self.bump();
        let raw = head.name != names::S_INTERP;
        let mut parts: Vec<StrId> = Vec::new();
        let mut spans: Vec<Span> = Vec::new();
        let mark = self.pat_scratch.len();
        loop {
            match self.kind() {
                Tok::StrPart => {
                    let t = self.bump();
                    let text = &self.text[t.span.start as usize..t.span.end as usize];
                    let s = if raw { text.replace("$$", "$") } else { unescape(text, true, &mut self.errors, t.span) };
                    parts.push(self.ast.add_str(s));
                    spans.push(t.span);
                }
                Tok::Ident => {
                    let t = self.bump();
                    let p = self.ast.add_pat(Pat::Bind(t.name, None), t.span);
                    self.pat_scratch.push(p);
                }
                Tok::Underscore => {
                    let t = self.bump();
                    let p = self.ast.add_pat(Pat::Wildcard, t.span);
                    self.pat_scratch.push(p);
                }
                Tok::LBrace => {
                    self.bump();
                    let p = self.parse_pattern();
                    self.pat_scratch.push(p);
                    self.expect(Tok::RBrace);
                }
                Tok::InterpEnd => {
                    self.bump();
                    break;
                }
                _ => {
                    self.error_at(self.span(), "malformed interpolated string pattern");
                    break;
                }
            }
        }
        let holes = self.pat_list(mark);
        let parts = push_list(&mut self.ast.str_lists, &parts);
        self.ast.set_part_spans(parts, &spans);
        self.ast.add_pat(Pat::Interp(head.name, parts, holes), head.span.to(self.prev_span()))
    }

    fn parse_pattern_args(&mut self) -> ListRef {
        self.expect(Tok::LParen);
        let mark = self.pat_scratch.len();
        let mut cut = false;
        while !self.at(Tok::RParen) && !self.at(Tok::Eof) {
            // `name = pattern`, a field of a named tuple or a case class.
            let p = if self.at(Tok::Ident) && self.kind_at(1) == Tok::Eq {
                let start = self.span();
                let name = self.bump().name;
                self.bump();
                let inner = self.parse_pattern();
                self.ast.add_pat(Pat::NamedField(name, inner), start.to(self.prev_span()))
            } else {
                self.parse_pattern()
            };
            self.pat_scratch.push(p);
            if !self.list_continues(Tok::RParen, &mut cut) {
                break;
            }
        }
        self.expect(Tok::RParen);
        self.pat_list(mark)
    }
}
