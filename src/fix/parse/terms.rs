//! ScalametaParser.scala's expressions, `condExpr` through `generator`, with the term infix context
//! (`TermInfixContext`: its stack and `reduceStack` over trees' `precedence`) and trees' `PlaceholderChecks`, which
//! `maybeAnonymousFunction` asks of a parsed term.

use std::collections::VecDeque;

use super::chars::{is_left_assoc, precedence};
use super::stats::Location;
use super::{Owner, Parser, Stop, P};
use crate::fix::nodes::{F, K};
use crate::fix::scanner::C;

/// `TermInfixContext.UnfinishedInfix`: `[lhs op]` and the operator's type arguments.
#[derive(Clone, Copy)]
pub(super) struct Unfinished {
    lhs: u32,
    op: u32,
    targs: u32,
}

/// `Unary`'s operators.
pub(super) fn is_unary(text: &str) -> bool {
    matches!(text, "+" | "-" | "~" | "!")
}

/// `TreeSyntacticGroup.opNeedsParens(outerPrecedence, innerPrecedence, ifSamePrecedence)`.
pub(super) fn op_needs_parens(outer: i32, inner: i32, if_same: bool) -> bool {
    let diff = outer - inner;
    diff > 0 || diff == 0 && if_same
}

/// A class that extends `Term` (a `Lit` among them), for `blockMaybeRaw`'s single term.
pub(super) fn is_term(k: K) -> bool {
    let name = k.name();
    (name.starts_with("Term.") || name.starts_with("Lit.")) && !matches!(k, K::ParamClause | K::Param | K::TermArgClause | K::TermCasesBlock | K::TermEnumeratorsBlock)
}

/// `Term.Ref`'s classes.
fn is_term_ref(k: K) -> bool {
    matches!(k, K::TermThis | K::TermSuper | K::TermName | K::TermAnonymous | K::TermApplyUnary | K::TermSelect | K::TermSelectPostfix)
}

impl<'a> Parser<'a> {
    pub(super) fn value_of(&self, n: u32) -> String {
        self.nodes[n as usize].value.clone().unwrap_or_default()
    }

    pub(super) fn fields_of(&self, n: u32, f: F) -> Vec<u32> {
        self.nodes[n as usize].fields.iter().filter(|&&(g, _)| g == f).map(|&(_, c)| c).collect()
    }

    /// `condExpr()`.
    fn cond_expr(&mut self) -> P<u32> {
        self.accept(C::LeftParen)?;
        let e = self.expr()?;
        self.in_parens_after_open(e)
    }

    /// `expr()`.
    pub(super) fn expr(&mut self) -> P<u32> {
        self.expr_at(Location::NoStat, false)
    }

    /// `inlineMatchClause(inlineMods)`: the match a copy with the modifiers.
    pub(super) fn inline_match_clause(&mut self, mods: Vec<u32>) -> P<u32> {
        let start = self.beg_index(mods[0]);
        let t = self.auto_end_pos(start, |p| p.postfix_expr(false))?;
        if self.kind(t) != K::TermMatch {
            let at = self.s.tokens[self.beg_index(t).max(0) as usize].start;
            return Err(Stop::Refused(at, "`inline` must be followed by an `if` or a `match`".to_string()));
        }
        let mut fields: Vec<(F, u32)> = self.nodes[t as usize].fields.iter().copied().filter(|&(f, _)| f != F::Mods).collect();
        fields.extend(mods.into_iter().map(|m| (F::Mods, m)));
        let copy = self.mk(K::TermMatch, fields);
        Ok(self.copy_pos(t, copy))
    }

    /// `matchClause(t, startPos, isSelect)`.
    fn match_clause(&mut self, t: u32, start: i32, is_select: bool) -> P<u32> {
        let cases = self.auto_pos(|p| {
            if p.at(C::Indent) {
                p.next();
                let c = p.cases_block()?;
                p.indented_after_open(c)
            } else {
                p.in_braces(|p| p.cases_block())
            }
        })?;
        self.auto_end_pos(start, |p| Ok(p.mk(if is_select { K::TermSelectMatch } else { K::TermMatch }, vec![(F::Expr, t), (F::CasesBlock, cases)])))
    }

    /// `inBraces(body)`: past a line break, the braces' body, a refusal where no `{` opens it.
    fn in_braces(&mut self, body: impl FnOnce(&mut Self) -> P<u32>) -> P<u32> {
        self.new_line_opt();
        if !self.accept_opt(C::LeftBrace) {
            return self.expected_c(C::LeftBrace);
        }
        let b = body(self)?;
        self.in_braces_after_open(b)
    }

    /// `ifClause(mods)`.
    pub(super) fn if_clause(&mut self, mods: Vec<u32>) -> P<u32> {
        let start = mods.first().map_or_else(|| self.curr_index(), |&m| self.beg_index(m));
        self.auto_end_pos(start, |p| {
            p.accept(C::KwIf)?;
            if p.at(C::Indent) {
                p.if_maybe_indented_cond(&mods)
            } else {
                let (cond, thenp) = p.cond_expr_with_optional_body(C::KwThen)?;
                match thenp {
                    Some(t) => p.if_with_cond_and_thenp(cond, t, &mods),
                    None => p.if_with_cond(cond, &mods),
                }
            }
        })
    }

    /// `ifClause`'s `getMaybeIndented` of the condition: an indented condition, then `then`.
    fn if_maybe_indented_cond(&mut self, mods: &[u32]) -> P<u32> {
        let cond = self.maybe_indented_init()?;
        let (term, outdented) = cond;
        let r = (|| {
            self.accept_after_opt_nl(C::KwThen)?;
            self.if_with_cond(term, mods)
        })();
        self.after_maybe_indented(r, outdented)
    }

    /// `getMaybeIndented`'s indented part: its first term (a partial function or a block), what follows it.
    fn maybe_indented_init(&mut self) -> P<(u32, bool)> {
        let beg = self.curr_index();
        let raw = match self.block_expr_maybe_partial_raw()? {
            Some(t) => t,
            None => {
                self.next();
                self.block_maybe_raw(false, false)?
            }
        };
        let outdented = self.accept_opt(C::Outdent);
        let init = self.auto_end_pos_opt(beg, |_| Ok(raw))?;
        let term = self.expr_after_simple_init(init, beg, Location::NoStat, false, false)?;
        Ok((term, outdented))
    }

    /// `getMaybeIndented`'s `finally`: the `Outdent` its region ends with where none was read.
    fn after_maybe_indented(&mut self, r: P<u32>, outdented: bool) -> P<u32> {
        if outdented {
            return r;
        }
        match self.accept(C::Outdent) {
            Ok(()) => r,
            Err(e) => Err(e),
        }
    }

    /// `ifClause`'s `getWithCond(cond)`.
    fn if_with_cond(&mut self, cond: u32, mods: &[u32]) -> P<u32> {
        if self.at(C::Indent) {
            let (term, outdented) = self.maybe_indented_init()?;
            let r = self.if_with_cond_and_thenp(cond, term, mods);
            self.after_maybe_indented(r, outdented)
        } else {
            let thenp = self.expr()?;
            self.if_with_cond_and_thenp(cond, thenp, mods)
        }
    }

    /// `ifClause`'s `getWithCondAndThenp(cond, thenp)`: no `else` a `Lit.Unit` of no width.
    fn if_with_cond_and_thenp(&mut self, cond: u32, thenp: u32, mods: &[u32]) -> P<u32> {
        let elsep = if self.accept_if_after_opt(|c| c == C::KwElse, |c| c == C::Semicolon || c.at_eol()) {
            self.expr()?
        } else {
            let u = self.mk_value(K::LitUnit, Vec::new(), String::new());
            self.at_cur_pos_empty(u)
        };
        let mut fields = vec![(F::Cond, cond), (F::Thenp, thenp), (F::Elsep, elsep)];
        fields.extend(mods.iter().map(|&m| (F::Mods, m)));
        Ok(self.mk(K::TermIf, fields))
    }

    /// `condExprWithOptionalBody[T]`: the condition, and the body where the parenthesized condition's tail began it.
    fn cond_expr_with_optional_body(&mut self, t: C) -> P<(u32, Option<u32>)> {
        if !self.dialect.allow_quiet_syntax {
            let cond = self.cond_expr()?;
            self.new_lines_opt();
            return Ok((cond, None));
        }
        if !self.at(C::LeftParen) {
            let cond = self.expr()?;
            self.accept_after_opt_nl(t)?;
            return Ok((cond, None));
        }
        let start = self.curr_index();
        let simple = self.in_parens_or_tuple_or_unit_expr(false)?;
        if self.accept_if_after_opt_nl(t) {
            return Ok((simple, None));
        }
        let arg_pos = self.curr_index();
        let args_or_init_body = match self.curr_c() {
            C::LeftParen => Some(self.in_parens_or_tuple_or_unit_expr(false)?),
            C::LeftBrace => Some(self.block_expr_on_brace(false, false)?),
            _ => None,
        };
        let complex = self.try_parse(|p| {
            let with_args = match args_or_init_body {
                None => simple,
                Some(a) => {
                    let args = p.to_arg_clause(a);
                    let args = p.copy_pos(a, args);
                    p.auto_end_pos(start, |p| Ok(p.mk(K::TermApply, vec![(F::Fun, simple), (F::ArgClause, args)])))?
                }
            };
            let x = p.try_refusal(|p| p.expr_after_simple_init(with_args, start, Location::NoStat, true, false))?;
            Ok(x.filter(|_| p.accept_if_after_opt_nl(t)))
        })?;
        if let Some(x) = complex {
            return Ok((x, None));
        }
        if args_or_init_body.is_none() {
            self.new_lines_opt();
        }
        let body = match args_or_init_body {
            Some(a) => Some(self.expr_after_simple_init(a, arg_pos, Location::NoStat, true, false)?),
            None => None,
        };
        Ok((simple, body))
    }

    /// `condExprWithBody[T]`.
    fn cond_expr_with_body(&mut self, t: C) -> P<(u32, u32)> {
        let (cond, body) = self.cond_expr_with_optional_body(t)?;
        let body = match body {
            Some(b) => b,
            None => self.expr()?,
        };
        Ok((cond, body))
    }

    /// `expr(location, allowRepeated)`.
    pub(super) fn expr_at(&mut self, location: Location, allow_repeated: bool) -> P<u32> {
        let res = self.auto_pos_opt(|p| p.expr_body(location, allow_repeated))?;
        self.maybe_anonymous_function(res, location, false)
    }

    /// `inlineMod()`: `inline` before an `if` or a `match`.
    fn inline_mod(&mut self) -> P<u32> {
        self.auto_pos(|p| {
            p.accept(C::Ident)?;
            Ok(p.mk(K::ModInline, Vec::new()))
        })
    }

    fn expr_body(&mut self, location: Location, allow_repeated: bool) -> P<u32> {
        let index = self.curr_index();
        match self.curr_c() {
            C::Ident if self.at_soft(self.dialect.allow_inline_mods, "inline") && self.peek(C::KwIf) => {
                let m = self.inline_mod()?;
                self.if_clause(vec![m])
            }
            _ if self.s.is_inline_match_mod(index) => {
                let m = self.inline_mod()?;
                self.inline_match_clause(vec![m])
            }
            C::KwIf => self.if_clause(Vec::new()),
            C::KwTry => self.try_expr(),
            C::KwWhile => {
                self.next();
                let (cond, body) = self.cond_expr_with_body(C::KwDo)?;
                // The schema's name of `Term.While.expr`, which the rules' API reads (`While.expr`).
                Ok(self.mk(K::TermWhile, vec![(F::Cond, cond), (F::Body, body)]))
            }
            C::KwDo => {
                self.next();
                let body = self.expr()?;
                self.skip_all_stat_sep();
                self.accept(C::KwWhile)?;
                let cond = self.cond_expr()?;
                Ok(self.mk(K::TermDo, vec![(F::Body, body), (F::Expr, cond)]))
            }
            C::KwFor => self.for_expr(),
            C::KwReturn => {
                self.next();
                let e = if self.s.is_expr_intro_of(self.curr_c(), self.curr_index()) {
                    self.expr()?
                } else {
                    let u = self.mk_value(K::LitUnit, Vec::new(), String::new());
                    self.at_cur_pos_empty(u)
                };
                Ok(self.mk(K::TermReturn, vec![(F::Expr, e)]))
            }
            C::KwThrow => {
                self.next();
                let e = self.expr()?;
                Ok(self.mk(K::TermThrow, vec![(F::Expr, e)]))
            }
            C::KwImplicit => {
                self.next();
                self.implicit_closure(location)
            }
            C::LeftBracket if self.dialect.allow_polymorphic_functions => {
                let quants = self.type_param_clause_opt()?;
                self.accept(C::RightArrow)?;
                let body = self.expr_at(location, allow_repeated)?;
                Ok(self.mk(K::TermPolyFunction, vec![(F::TparamClause, quants), (F::Body, body)]))
            }
            _ => {
                let start = self.curr_index();
                let t = self.postfix_expr_at(start, allow_repeated)?;
                self.expr_other_rest(t, start, location, allow_repeated)
            }
        }
    }

    /// `expr`'s `try`: its handler a cases block or an expression, its `finally`.
    fn try_expr(&mut self) -> P<u32> {
        self.next();
        let body = if self.dialect.allow_try_with_any_expr {
            self.expr()?
        } else {
            match self.curr_c() {
                C::LeftParen => {
                    self.next();
                    let e = self.expr()?;
                    self.in_parens_after_open(e)?
                }
                C::LeftBrace => self.block_on_brace(false)?,
                C::Indent => self.block_on_indent(false)?,
                _ => self.expr()?,
            }
        };
        let finally_opt = |p: &mut Self| -> P<Option<u32>> { if p.accept_if_after_opt_nl(C::KwFinally) { Ok(Some(p.expr()?)) } else { Ok(None) } };
        let with_cases = |p: &mut Self, cases: Option<u32>| -> P<u32> {
            let mut fields = vec![(F::Expr, body)];
            fields.extend(cases.map(|c| (F::CatchClause, c)));
            fields.extend(finally_opt(p)?.map(|f| (F::Finallyp, f)));
            Ok(p.mk(K::TermTry, fields))
        };
        let with_handler = |p: &mut Self, handler: u32| -> P<u32> {
            let mut fields = vec![(F::Expr, body), (F::Catchp, handler)];
            fields.extend(finally_opt(p)?.map(|f| (F::Finallyp, f)));
            Ok(p.mk(K::TermTryWithHandler, fields))
        };
        if !self.accept_if_after_opt_nl(C::KwCatch) {
            return with_cases(self, None);
        }
        match self.curr_c() {
            C::KwCase => {
                let cases = self.auto_pos(|p| {
                    p.next();
                    let c = p.case_clause(true)?;
                    Ok(p.mk(K::TermCasesBlock, vec![(F::Cases, c)]))
                })?;
                with_cases(self, Some(cases))
            }
            C::Indent | C::LeftBrace => {
                let (close, catch_pos) = (if self.at(C::Indent) { C::Outdent } else { C::RightBrace }, self.curr_index());
                self.next();
                let handler = match self.cases_block_if_any()? {
                    Some(cb) => Ok(cb),
                    None => Err(self.block_raw(false)?),
                };
                self.accept_after_opt_nl(close)?;
                match handler {
                    Ok(cb) => {
                        let cb = self.auto_end_pos(catch_pos, |_| Ok(cb))?;
                        with_cases(self, Some(cb))
                    }
                    Err(h) => {
                        let h = self.auto_end_pos(catch_pos, |_| Ok(h))?;
                        with_handler(self, h)
                    }
                }
            }
            _ => {
                let h = self.expr()?;
                with_handler(self, h)
            }
        }
    }

    /// `expr`'s `for`: its enumerators in braces, in parentheses (retried without them) or as they stand.
    fn for_expr(&mut self) -> P<u32> {
        self.next();
        let enums = self.auto_pos(|p| {
            let list = if p.accept_opt(C::LeftBrace) {
                let e = p.enumerators()?;
                p.in_braces_after_open(e)?
            } else if p.at(C::LeftParen) {
                let in_parens = |p: &mut Self| -> P<Vec<u32>> {
                    p.next();
                    let e = p.enumerators()?;
                    p.in_parens_after_open(e)
                };
                if p.dialect.allow_quiet_syntax {
                    match p.try_parse(|p| p.try_refusal(in_parens))? {
                        Some(e) => e,
                        None => p.enumerators()?,
                    }
                } else {
                    in_parens(p)?
                }
            } else if p.accept_opt(C::Indent) {
                let e = p.enumerators()?;
                p.indented_after_open(e)?
            } else {
                p.enumerators()?
            };
            Ok(p.mk(K::TermEnumeratorsBlock, list.into_iter().map(|e| (F::Enums, e)).collect()))
        })?;
        self.new_lines_opt();
        let kind = if self.accept_opt(C::KwDo) {
            K::TermFor
        } else if self.accept_opt(C::KwYield) {
            K::TermForYield
        } else {
            K::TermFor
        };
        let body = self.expr()?;
        Ok(self.mk(kind, vec![(F::EnumsBlock, enums), (F::Body, body)]))
    }

    /// `exprOtherRest(prefix, startPos, location, allowRepeated)`: assignments, ascriptions, annotations, a repeated
    /// argument, a match, and a lambda where `=>` follows what was read.
    fn expr_other_rest(&mut self, prefix: u32, start: i32, location: Location, allow_repeated: bool) -> P<u32> {
        let mut t = prefix;
        let res = loop {
            match self.curr_c() {
                C::Equals => {
                    if is_term_ref(self.kind(t)) || self.kind(t) == K::TermApply {
                        self.next();
                        break self.auto_end_pos(start, |p| {
                            let rhs = p.expr_at(Location::NoStat, true)?;
                            Ok(p.mk(K::TermAssign, vec![(F::Lhs, t), (F::Rhs, rhs)]))
                        })?;
                    }
                    break t;
                }
                C::Colon => {
                    if let Some(x) = self.get_fewer_braces_apply_on_colon(t, start, false)? {
                        break x;
                    }
                    self.next();
                    if self.at(C::At) {
                        t = self.auto_end_pos(start, |p| {
                            let annots = p.annots(false, true)?;
                            let mut fields = vec![(F::Expr, t)];
                            fields.extend(annots.into_iter().map(|a| (F::Annots, a)));
                            Ok(p.mk(K::TermAnnotate, fields))
                        })?;
                    } else if self.at(C::Underscore) && self.peek(C::Ident) && self.peek_text() == "*" {
                        break self.repeated_term(t, start, location, 2)?;
                    } else {
                        t = self.auto_end_pos(start, |p| {
                            let tpe = p.type_or_infix_type(location.full_type_ok())?;
                            Ok(p.mk(K::TermAscribe, vec![(F::Expr, t), (F::Tpe, tpe)]))
                        })?;
                    }
                }
                C::Ident if allow_repeated && self.at_soft(self.dialect.allow_postfix_star_vararg_splices, "*") && matches!(self.peek_c(), C::RightParen | C::Comma) => {
                    break self.repeated_term(t, start, location, 1)?;
                }
                C::KwMatch => {
                    self.next();
                    break self.match_clause(t, start, false)?;
                }
                _ => break t,
            }
        };
        if !matches!(self.curr_c(), C::RightArrow | C::ContextArrow) {
            return Ok(res);
        }
        let allow_name = location != Location::TemplateStat;
        let allow_param = |p: &Self, has_type: bool| location.func_param_ok() && !(has_type && p.dialect.allow_fewer_braces) || p.tok_c(start) == C::LeftParen && p.prev(C::RightParen);
        let params: Option<Vec<u32>> = match self.kind(res) {
            K::LitUnit => Some(Vec::new()),
            K::TermTuple => {
                let mut params = Vec::new();
                let mut ok = true;
                for a in self.fields_of(res, F::Args) {
                    match self.convert_to_param(a) {
                        Some(p) => params.push(p),
                        None => {
                            ok = false;
                            break;
                        }
                    }
                }
                ok.then_some(params)
            }
            _ => self.convert_to_param(res).filter(|&p| {
                if self.field(p, F::Decltpe).is_some() {
                    allow_param(self, true)
                } else if self.field(p, F::Mods).is_some() {
                    allow_param(self, false)
                } else {
                    allow_name
                }
            }).map(|p| vec![p]),
        };
        let Some(params) = params else { return Ok(res) };
        let context = self.at(C::ContextArrow);
        let pc = self.auto_end_pos(start, |p| Ok(p.param_clause_of(params, None)))?;
        self.next();
        let body = self.term_function_body(location)?;
        self.auto_end_pos(start, |p| Ok(p.mk(if context { K::TermContextFunction } else { K::TermFunction }, vec![(F::ParamClause, pc), (F::Body, body)])))
    }

    /// `exprOtherRest`'s `repeatedTerm(t, nextTokens)`: past `nextTokens` tokens, `t*`.
    fn repeated_term(&mut self, t: u32, start: i32, location: Location, next_tokens: usize) -> P<u32> {
        self.auto_end_pos(start, |p| {
            for _ in 0..next_tokens {
                p.next();
            }
            let e = p.maybe_anonymous_function(t, location, true)?;
            Ok(p.mk(K::TermRepeated, vec![(F::Expr, e)]))
        })
    }

    /// `typeOrInfixType(fullTypeOK)`.
    fn type_or_infix_type(&mut self, full_type_ok: bool) -> P<u32> {
        if full_type_ok { self.typ() } else { self.start_infix_type(false) }
    }

    /// `getDeclTpeOpt(fullTypeOK)`.
    fn get_decl_tpe_opt(&mut self, full_type_ok: bool) -> P<Option<u32>> {
        if self.accept_opt(C::Colon) { Ok(Some(self.type_or_infix_type(full_type_ok)?)) } else { Ok(None) }
    }

    /// `toParamClause(mod)(values)`: the clause's `mod` the one given, else `getMod`'s.
    pub(super) fn param_clause_of(&mut self, params: Vec<u32>, mod_: Option<u32>) -> u32 {
        let clause_mod = mod_.or_else(|| self.get_mod(&params));
        let mut fields: Vec<(F, u32)> = params.into_iter().map(|x| (F::Values, x)).collect();
        fields.extend(clause_mod.map(|m| (F::Mod, m)));
        self.mk(K::ParamClause, fields)
    }

    /// `termFunctionBody(location)`.
    fn term_function_body(&mut self, location: Location) -> P<u32> {
        if location != Location::BlockStat {
            return self.expr();
        }
        let t = match self.curr_c() {
            C::LeftBrace => self.block_expr_on_brace(false, true)?,
            C::Indent => self.block_expr_on_indent(false)?,
            _ => self.block_on_other(false)?,
        };
        if self.kind(t) != K::TermPartialFunction {
            return Ok(t);
        }
        match self.get_decl_tpe_opt(false)? {
            None => Ok(t),
            Some(tpe) => {
                let start = self.beg_index(t);
                self.auto_end_pos(start, |p| Ok(p.mk(K::TermAscribe, vec![(F::Expr, t), (F::Tpe, tpe)])))
            }
        }
    }

    /// `convertToParam(tree)`: a lambda's parameter of what was read before its `=>`.
    fn convert_to_param(&mut self, tree: u32) -> Option<u32> {
        match self.kind(tree) {
            K::LitUnit => None,
            K::TermAscribe => {
                let (e, tpe) = (self.field(tree, F::Expr)?, self.field(tree, F::Tpe)?);
                let (name, mods) = self.get_name_and_mod(e)?;
                let mut fields: Vec<(F, u32)> = mods.into_iter().map(|m| (F::Mods, m)).collect();
                fields.extend([(F::Name, name), (F::Decltpe, tpe)]);
                let p = self.mk(K::Param, fields);
                Some(self.copy_pos(tree, p))
            }
            _ => {
                let (name, mods) = self.get_name_and_mod(tree)?;
                let mut fields: Vec<(F, u32)> = mods.into_iter().map(|m| (F::Mods, m)).collect();
                fields.push((F::Name, name));
                let p = self.mk(K::Param, fields);
                Some(self.copy_pos(tree, p))
            }
        }
    }

    /// `convertToParam`'s `getModFromName(name)`: `using` and `erased` a parameter's modifiers.
    fn get_mod_from_name(&mut self, name: u32) -> Option<u32> {
        let kind = match self.nodes[name as usize].value.as_deref() {
            Some("using") if self.dialect.allow_given_using => K::ModUsing,
            Some("erased") if self.dialect.allow_erased_defs => K::ModErased,
            _ => return None,
        };
        let m = self.mk(kind, Vec::new());
        Some(self.copy_pos(name, m))
    }

    /// `convertToParam`'s `getMod(t, mods)`.
    fn get_mod_of(&mut self, mut t: u32, mut mods: Vec<u32>) -> Option<Vec<u32>> {
        loop {
            match self.kind(t) {
                K::TermName => {
                    let m = self.get_mod_from_name(t)?;
                    mods.insert(0, m);
                    return Some(mods);
                }
                K::TermSelectPostfix => {
                    let m = self.get_mod_from_name(self.field(t, F::Name)?)?;
                    mods.insert(0, m);
                    t = self.field(t, F::Qual)?;
                }
                K::TermApplyInfix => {
                    let args = self.fields_of(self.field(t, F::ArgClause)?, F::Values);
                    let targs_empty = self.field(t, F::TargClause).is_none_or(|c| self.field(c, F::Values).is_none());
                    let [n] = args[..] else { return None };
                    if !targs_empty || !matches!(self.kind(n), K::TermName | K::TypeName | K::NameAnonymous | K::NamePlaceholder | K::NameIndeterminate | K::NameThis) {
                        return None;
                    }
                    let m1 = self.get_mod_from_name(self.field(t, F::Op)?)?;
                    let m2 = self.get_mod_from_name(n)?;
                    mods.splice(0..0, [m1, m2]);
                    t = self.field(t, F::Lhs)?;
                }
                _ => return None,
            }
        }
    }

    /// `convertToParam`'s `getNameAndMod(t)`.
    fn get_name_and_mod(&mut self, t: u32) -> Option<(u32, Vec<u32>)> {
        match self.kind(t) {
            K::TermName | K::TypeName | K::NameAnonymous | K::NamePlaceholder | K::NameIndeterminate | K::NameThis => Some((t, Vec::new())),
            K::TermSelectPostfix => {
                let (qual, name) = (self.field(t, F::Qual)?, self.field(t, F::Name)?);
                Some((name, self.get_mod_of(qual, Vec::new())?))
            }
            K::TermApplyInfix => {
                let args = self.fields_of(self.field(t, F::ArgClause)?, F::Values);
                let targs_empty = self.field(t, F::TargClause).is_none_or(|c| self.field(c, F::Values).is_none());
                let [arg] = args[..] else { return None };
                if !targs_empty {
                    return None;
                }
                let m = self.get_mod_from_name(self.field(t, F::Op)?)?;
                let name = match self.kind(arg) {
                    K::TermPlaceholder => {
                        let n = self.mk_value(K::NamePlaceholder, Vec::new(), "_".to_string());
                        self.copy_pos(arg, n)
                    }
                    K::TermName | K::TypeName | K::NameAnonymous | K::NamePlaceholder | K::NameIndeterminate | K::NameThis => arg,
                    _ => return None,
                };
                let mods = self.get_mod_of(self.field(t, F::Lhs)?, vec![m])?;
                Some((name, mods))
            }
            K::TermPlaceholder => {
                let n = self.mk_value(K::NamePlaceholder, Vec::new(), "_".to_string());
                Some((self.copy_pos(t, n), Vec::new()))
            }
            K::TermEta => {
                let mods = self.get_mod_of(self.field(t, F::Expr)?, Vec::new())?;
                let n = self.mk_value(K::NamePlaceholder, Vec::new(), "_".to_string());
                let end = self.end_index(t);
                Some((self.at_pos1(end, n), mods))
            }
            _ => None,
        }
    }

    /// `exprAfterSimple(term, startPos, location, allowRepeated)`.
    fn expr_after_simple(&mut self, term: u32, start: i32, location: Location, allow_repeated: bool) -> P<u32> {
        let postfix = self.postfix_expr_rest(term, start, allow_repeated)?;
        self.expr_other_rest(postfix, start, location, allow_repeated)
    }

    /// `exprAfterSimpleInit(term, startPos, location, canApply, allowRepeated)`.
    fn expr_after_simple_init(&mut self, term: u32, start: i32, location: Location, can_apply: bool, allow_repeated: bool) -> P<u32> {
        let simple = self.simple_expr_rest(term, can_apply, start)?;
        self.expr_after_simple(simple, start, location, allow_repeated)
    }

    /// `implicitClosure(location)`: `implicit x => body`, after `implicit`.
    pub(super) fn implicit_closure(&mut self, location: Location) -> P<u32> {
        let implicit_pos = self.prev_index();
        let name = self.term_name()?;
        let tpt = self.get_decl_tpe_opt(false)?;
        let m = self.mk(K::ModImplicit, Vec::new());
        let m = self.at_pos1(implicit_pos, m);
        let param = self.auto_end_pos(implicit_pos, |p| {
            let mut fields = vec![(F::Mods, m), (F::Name, name)];
            fields.extend(tpt.map(|t| (F::Decltpe, t)));
            Ok(p.mk(K::Param, fields))
        })?;
        let params = self.mk(K::ParamClause, vec![(F::Values, param), (F::Mod, m)]);
        let params = self.copy_pos(param, params);
        self.accept(C::RightArrow)?;
        self.auto_end_pos(implicit_pos, |p| {
            let body = p.term_function_body(location)?;
            Ok(p.mk(K::TermFunction, vec![(F::ParamClause, params), (F::Body, body)]))
        })
    }

    /* -------- TermInfixContext -------- */

    /// `TermInfixContext.toArgClause(rhs)`: a tuple's elements the arguments, but a named one.
    fn to_arg_clause(&mut self, rhs: u32) -> u32 {
        let values = match self.kind(rhs) {
            K::LitUnit if self.dialect.allow_empty_infix_args => Vec::new(),
            K::TermTuple => {
                let args = self.fields_of(rhs, F::Args);
                let named = matches!(args[..], [a] if self.kind(a) == K::TermAssign);
                if !named || !self.dialect.allow_named_tuples { args } else { vec![rhs] }
            }
            _ => vec![rhs],
        };
        let c = self.mk(K::TermArgClause, values.into_iter().map(|v| (F::Values, v)).collect());
        self.copy_pos(rhs, c)
    }

    /// `TermInfixContext.finishInfixExpr(unf, rhs, rhsEnd)`.
    fn finish_infix_expr(&mut self, unf: Unfinished, rhs: u32, rhs_end: i32) -> u32 {
        let lhs = match self.kind(unf.lhs) {
            K::TermTuple => match self.fields_of(unf.lhs, F::Args)[..] {
                [arg] if !self.dialect.allow_named_tuples || self.kind(arg) != K::TermAssign => arg,
                _ => unf.lhs,
            },
            _ => unf.lhs,
        };
        let args = self.to_arg_clause(rhs);
        let n = self.mk(K::TermApplyInfix, vec![(F::Lhs, lhs), (F::Op, unf.op), (F::TargClause, unf.targs), (F::ArgClause, args)]);
        let start = self.beg_index(unf.lhs);
        self.at_pos(start, rhs_end, n)
    }

    /// `InfixContext.reduceStack(base, curr, currEnd, op)`.
    fn reduce_stack(&mut self, base: usize, curr: u32, curr_end: i32, op: u32) -> u32 {
        if self.term_infix.len() == base {
            return curr;
        }
        let opv = self.value_of(op);
        let (op_precedence, if_same) = (precedence(&opv), !is_left_assoc(&opv));
        let mut rhs = curr;
        loop {
            let lhs = *self.term_infix.last().unwrap();
            if op_needs_parens(op_precedence, precedence(&self.value_of(lhs.op)), if_same) {
                return rhs;
            }
            self.term_infix.pop();
            let fin = self.finish_infix_expr(lhs, rhs, curr_end);
            if self.term_infix.len() == base {
                return fin;
            }
            rhs = fin;
        }
    }

    /// `InfixContext.drainStack(base, curr, currEnd)`.
    fn drain_stack(&mut self, base: usize, curr: u32, curr_end: i32) -> u32 {
        let mut rhs = curr;
        while self.term_infix.len() > base {
            let lhs = self.term_infix.pop().unwrap();
            rhs = self.finish_infix_expr(lhs, rhs, curr_end);
        }
        rhs
    }

    /// `getLeadingInfix(lf)(Term.Name.apply)`.
    fn get_leading_infix(&mut self) -> P<Option<u32>> {
        self.get_leading_infix_name(K::TermName)
    }

    /// `getLeadingInfix(lf)(f)`: at an `InfixLF`, the operator that begins the next line, a name of `kind`, past it and
    /// a line break; none where the token after it is an indentation.
    pub(super) fn get_leading_infix_name(&mut self, kind: K) -> P<Option<u32>> {
        if !self.peek(C::Ident) {
            return Ok(None);
        }
        let saved = self.save();
        let invalid = self.s.invalid_infix.contains(&self.curr_ref());
        self.next();
        if self.peek_c().indentation() {
            self.restore(saved);
            return Ok(None);
        }
        if invalid {
            return self.refused("Invalid indented leading infix operator found".to_string());
        }
        let at = self.curr_index();
        let value = super::names::ident_value(self.curr_text());
        self.next();
        self.new_line_opt();
        let n = self.mk_value(kind, Vec::new(), value);
        Ok(Some(self.at_pos1(at, n)))
    }

    /// `postfixExpr(allowRepeated)`.
    pub(super) fn postfix_expr(&mut self, allow_repeated: bool) -> P<u32> {
        let start = self.curr_index();
        self.postfix_expr_at(start, allow_repeated)
    }

    /// `postfixExpr(startPos, allowRepeated)`.
    fn postfix_expr_at(&mut self, start: i32, allow_repeated: bool) -> P<u32> {
        let rhs0 = self.prefix_expr(allow_repeated)?;
        self.postfix_expr_rest(rhs0, start, allow_repeated)
    }

    /// `postfixExpr(rhs0, startPos, allowRepeated)`: the infix chain from `rhs0`, each operator's left side reduced by
    /// precedence on the stack, a postfix operator its end.
    fn postfix_expr_rest(&mut self, rhs0: u32, start: i32, allow_repeated: bool) -> P<u32> {
        let base = self.term_infix.len();
        let mut rhs_k = rhs0;
        let rhs_n = loop {
            let rhs_end_k = self.prev_index();
            let k = Chain { base, rhs0, start, rhs_k, rhs_end_k };
            let res = match self.curr_c() {
                C::InfixLf => match self.get_leading_infix()? {
                    Some(op) => {
                        let targs = self.empty_type_args();
                        let rhs = self.argument_exprs_or_prefix_expr(Location::PostfixStat)?;
                        Some(Err(self.next_rhs_with(&k, op, targs, rhs)))
                    }
                    None => None,
                },
                C::KwMatch if self.dialect.allow_match_as_operator => {
                    let op = self.term_name_value("match".to_string());
                    let lhs = self.reduce_stack(base, rhs_k, rhs_end_k, op);
                    let lhs_start = if lhs == rhs0 { start } else { self.beg_index(lhs) };
                    Some(Err(self.match_clause(lhs, lhs_start, false)?))
                }
                _ if self.prev(C::Outdent) && !self.in_region_paren() => None,
                C::Ident if !(allow_repeated && self.at_soft(self.dialect.allow_postfix_star_vararg_splices, "*") && matches!(self.peek_c(), C::RightParen | C::Comma)) => {
                    let op = self.term_name()?;
                    Some(self.postfix_or_next_rhs(&k, op)?)
                }
                _ => None,
            };
            match res {
                Some(Ok(postfix)) => break postfix,
                Some(Err(next)) => rhs_k = next,
                None => break rhs_k,
            }
        };
        if rhs_n == rhs0 && self.term_infix.len() == base {
            return Ok(rhs0);
        }
        let end = self.prev_index();
        let t = self.drain_stack(base, rhs_n, end);
        Ok(self.at_pos_with_body(start, t, end))
    }

    /// `getNextRhsWith(op, targs, rhs)`: the left side reduced and pushed with `op`, `rhs` the next right side.
    fn next_rhs_with(&mut self, k: &Chain, op: u32, targs: u32, rhs: u32) -> u32 {
        let lhs = self.reduce_stack(k.base, k.rhs_k, k.rhs_end_k, op);
        let wrap = lhs == k.rhs0 && self.beg_index(lhs) != k.start;
        let lhs_ext = if wrap {
            let t = self.mk(K::TermTuple, vec![(F::Args, lhs)]);
            self.at_pos_with_body(k.start, t, k.rhs_end_k)
        } else {
            lhs
        };
        self.term_infix.push(Unfinished { lhs: lhs_ext, op, targs });
        rhs
    }

    /// `getPostfixOrNextRhs(op)`: the next right side where an expression follows the operator, else a postfix
    /// selection (`Ok`) or a colon argument.
    fn postfix_or_next_rhs(&mut self, k: &Chain, op: u32) -> P<Result<u32, u32>> {
        let has_type_args = self.at(C::LeftBracket) || self.is_indenting_or_eol(true) && self.try_ahead(C::LeftBracket);
        let targs = if has_type_args { self.type_args_in_brackets()? } else { self.empty_type_args() };
        let continues = if self.curr_c().eol() {
            let ok = self.s.is_expr_intro_of(self.peek_c(), self.peek_index());
            self.next_if(ok)
        } else {
            self.is_ident_or_expr_intro()
        };
        if continues {
            let rhs = self.argument_exprs_or_prefix_expr(Location::PostfixStat)?;
            return Ok(Err(self.next_rhs_with(k, op, targs, rhs)));
        }
        let arg_pos = self.curr_index();
        let colon = if self.at(C::Colon) { self.get_fewer_braces_arg_on_colon(false)? } else { None };
        match colon {
            None => {
                if self.field(targs, F::Values).is_some() {
                    let at = self.s.tokens[self.beg_index(targs).max(0) as usize].start;
                    return Err(Stop::Refused(at, "type application is not allowed for postfix operators".to_string()));
                }
                let fin_qual = self.reduce_stack(k.base, k.rhs_k, k.rhs_end_k, op);
                let lhs_start = if fin_qual == k.rhs0 { k.start } else { self.beg_index(fin_qual) };
                let t = self.mk(K::TermSelectPostfix, vec![(F::Qual, fin_qual), (F::Name, op)]);
                let end = self.end_index(op);
                Ok(Ok(self.at_pos(lhs_start, end, t)))
            }
            Some(x) => {
                let t = self.auto_end_pos(arg_pos, |p| Ok(p.mk(K::TermTuple, vec![(F::Args, x)])))?;
                Ok(Err(self.next_rhs_with(k, op, targs, t)))
            }
        }
    }

    /// `emptyTypeArgs`: an empty `Type.ArgClause` at the current token.
    fn empty_type_args(&mut self) -> u32 {
        let n = self.mk(K::TypeArgClause, Vec::new());
        self.at_cur_pos_empty(n)
    }

    /// `prefixExpr(allowRepeated)`: a unary operator's application, a literal folded with it.
    fn prefix_expr(&mut self, allow_repeated: bool) -> P<u32> {
        if !(self.at(C::Ident) && is_unary(self.curr_text())) {
            return self.simple_expr(allow_repeated);
        }
        let start = self.curr_index();
        let unary = self.curr_text().to_string();
        self.next();
        let op = |p: &mut Self| {
            let n = p.mk_value(K::TermName, Vec::new(), unary.clone());
            p.at_pos1(start, n)
        };
        if self.curr_c().literal() {
            let lit = self.raw_literal(Some(&unary))?;
            let t = self.auto_end_pos(start, |_| Ok(lit))?;
            return self.simple_expr_rest(t, true, start);
        }
        match self.simple_expr0(true)? {
            Some(x) => self.auto_end_pos(start, |p| {
                let o = op(p);
                Ok(p.mk(K::TermApplyUnary, vec![(F::Op, o), (F::Arg, x)]))
            }),
            None => {
                let o = op(self);
                self.simple_expr_rest(o, true, start)
            }
        }
    }

    /// `simpleExpr(allowRepeated)`.
    fn simple_expr(&mut self, allow_repeated: bool) -> P<u32> {
        match self.simple_expr0(allow_repeated)? {
            Some(t) => Ok(t),
            None => self.refused("illegal start of simple expression".to_string()),
        }
    }

    /// `simpleExpr0(allowRepeated)`: none where no simple expression starts.
    fn simple_expr0(&mut self, allow_repeated: bool) -> P<Option<u32>> {
        let start = self.curr_index();
        let mut can_apply = true;
        let t = match self.curr_c() {
            C::MacroQuote => self.macro_quoted()?,
            C::Ident if self.macro_spliced_ident_here() => self.macro_spliced_ident()?,
            c if c.literal() => self.literal()?,
            C::InterpId => self.interpolate_term()?,
            C::Ident | C::KwThis | C::KwSuper => self.path(true)?,
            C::Underscore => {
                let n = self.mk(K::TermPlaceholder, Vec::new());
                self.at_cur_pos_next(n)
            }
            C::LeftParen => self.in_parens_or_tuple_or_unit_expr(allow_repeated)?,
            C::LeftBrace => {
                can_apply = false;
                self.block_expr_on_brace(false, false)?
            }
            C::Indent => {
                can_apply = false;
                self.block_expr_on_indent(false)?
            }
            C::KwNew => {
                can_apply = false;
                self.auto_pos(|p| {
                    p.next();
                    let tpl = p.within_owner(Owner::Trait, |p| p.template(false))?;
                    let inits = p.fields_of(tpl, F::Inits);
                    let body = p.field(tpl, F::Body).unwrap();
                    let body_empty = p.nodes[body as usize].fields.is_empty();
                    match inits[..] {
                        [init] if !p.prev(C::RightBrace) && p.field(tpl, F::EarlyClause).is_none() && body_empty => Ok(p.mk(K::TermNew, vec![(F::Init, init)])),
                        _ => Ok(p.mk(K::TermNewAnonymous, vec![(F::Templ, tpl)])),
                    }
                })?
            }
            _ => return Ok(None),
        };
        Ok(Some(self.simple_expr_rest(t, can_apply, start)?))
    }

    /// `simpleExprRest(t, canApply, startPos)`: selections, type applications, argument clauses, a colon argument and
    /// an eta expansion after `t`.
    fn simple_expr_rest(&mut self, mut t: u32, mut can_apply: bool, start: i32) -> P<u32> {
        loop {
            match self.curr_c() {
                c if c.at_eol()
                    && match self.peek_c() {
                        C::Dot => true,
                        C::LeftParen | C::LeftBracket | C::LeftBrace if can_apply => self.is_indenting_or_eol(self.peek(C::LeftBrace)),
                        _ => false,
                    } =>
                {
                    self.next();
                }
                C::Dot => {
                    self.next();
                    let is_match = self.dialect.allow_match_as_operator && self.accept_opt(C::KwMatch);
                    t = if is_match { self.match_clause(t, start, true)? } else { self.selector(t, start)? };
                    can_apply = !is_match;
                }
                C::LeftBracket => {
                    if !self.is_ok_for_type_args(t) {
                        return self.auto_end_pos(start, |_| Ok(t));
                    }
                    loop {
                        t = self.auto_end_pos(start, |p| {
                            let targs = p.type_args_in_brackets()?;
                            Ok(p.mk(K::TermApplyType, vec![(F::Fun, t), (F::TargClause, targs)]))
                        })?;
                        if !self.at(C::LeftBracket) {
                            break;
                        }
                    }
                    can_apply = true;
                }
                C::LeftParen | C::LeftBrace if can_apply => {
                    let brace = self.at(C::LeftBrace);
                    t = self.auto_end_pos(start, |p| {
                        let args = if brace { p.get_arg_clause_on_brace()? } else { p.get_arg_clause_on_paren(Location::NoStat)? };
                        Ok(p.mk(K::TermApply, vec![(F::Fun, t), (F::ArgClause, args)]))
                    })?;
                    can_apply = true;
                }
                C::Colon if can_apply => return Ok(self.get_fewer_braces_apply_on_colon(t, start, true)?.unwrap_or(t)),
                C::Underscore if can_apply => {
                    self.next();
                    return self.auto_end_pos(start, |p| Ok(p.mk(K::TermEta, vec![(F::Expr, t)])));
                }
                _ => return Ok(t),
            }
        }
    }

    /// `simpleExprRest`'s `isOk(tree)`: a term type arguments may follow.
    fn is_ok_for_type_args(&self, mut t: u32) -> bool {
        loop {
            match self.kind(t) {
                K::TermName | K::TermSelect | K::TermApply | K::TermApplyInfix | K::TermApplyUnary | K::TermNew | K::TermPlaceholder | K::TermInterpolate | K::TermSplice | K::TermPolyFunction => return true,
                K::TermBlock => match self.fields_of(t, F::Stats)[..] {
                    [s] => t = s,
                    _ => return false,
                },
                _ => return false,
            }
        }
    }

    /// `getFewerBracesArgOnColon(okSingleLineLambda)`: at `:`, its argument: a lambda block, else the indented block.
    fn get_fewer_braces_arg_on_colon(&mut self, ok_single_line: bool) -> P<Option<u32>> {
        if !self.dialect.allow_fewer_braces {
            return Ok(None);
        }
        let colon_pos = self.curr_index();
        let lambda_block = |p: &mut Self, post_check: &dyn Fn(&mut Self) -> bool| -> P<Option<u32>> {
            let Some(arg) = p.try_get_arg_as_lambda(ok_single_line)? else { return Ok(None) };
            if !post_check(p) {
                return Ok(None);
            }
            let b = p.mk(K::TermBlock, vec![(F::Stats, arg)]);
            Ok(Some(p.auto_end_pos(colon_pos, |_| Ok(b))?))
        };
        match self.peek_c() {
            C::Indent => {
                self.next();
                let lambda = self.try_parse(|p| {
                    p.next();
                    lambda_block(p, &|p: &mut Self| p.accept_if_after_opt_nl(C::Outdent))
                })?;
                match lambda {
                    Some(x) => Ok(Some(x)),
                    None => {
                        let b = self.block_expr_on_indent(true)?;
                        Ok(Some(self.auto_end_pos(colon_pos, |_| Ok(b))?))
                    }
                }
            }
            C::Outdent => self.refused("expected fewer-braces method body".to_string()),
            c if c.at_eol() => {
                let colon_at = self.curr_offset();
                self.next_twice();
                match lambda_block(self, &|_: &mut Self| true)? {
                    Some(x) => Ok(Some(x)),
                    None => Err(Stop::Refused(colon_at, "expected fewer-braces method body".to_string())),
                }
            }
            _ => self.try_parse(|p| {
                p.next();
                lambda_block(p, &|_: &mut Self| true)
            }),
        }
    }

    /// `tryGetArgAsLambda(okSingleLine)`: a colon argument's lambda, none where what follows is not one (a refusal
    /// none too).
    fn try_get_arg_as_lambda(&mut self, ok_single_line: bool) -> P<Option<u32>> {
        Ok(self.try_refusal(|p| p.arg_as_lambda(ok_single_line))?.flatten())
    }

    fn arg_as_lambda(&mut self, ok_single_line: bool) -> P<Option<u32>> {
        let param_pos = self.curr_index();
        match self.curr_c() {
            C::LeftParen => {
                let params = self.auto_pos(|p| {
                    p.next();
                    if p.accept_opt(C::RightParen) {
                        return Ok(p.mk(K::ParamClause, Vec::new()));
                    }
                    let pt = if p.at(C::KwImplicit) { Some(p.mod_at_cur(K::ModImplicit)) } else { None };
                    let mods: Vec<u32> = pt.into_iter().collect();
                    let mut params = vec![p.lambda_param_with_pos(mods.clone(), true)?];
                    while p.accept_opt(C::Comma) {
                        params.push(p.lambda_param_with_pos(mods.clone(), true)?);
                    }
                    let c = p.param_clause_of(params, pt);
                    p.in_parens_after_open(c)
                })?;
                self.lambda_function_term(param_pos, params, ok_single_line)
            }
            C::LeftBracket => {
                let tparams = self.type_param_clause_on_bracket()?;
                if self.at(C::RightArrow) && self.try_ahead(C::Indent) {
                    let body = self.block_expr_on_indent(false)?;
                    Ok(Some(self.auto_end_pos(param_pos, |p| Ok(p.mk(K::TermPolyFunction, vec![(F::TparamClause, tparams), (F::Body, body)])))?))
                } else {
                    Ok(None)
                }
            }
            C::Ident | C::Underscore => self.lambda_param_as_function(param_pos, None, ok_single_line),
            C::KwImplicit => {
                let m = self.mod_at_cur(K::ModImplicit);
                self.lambda_param_as_function(param_pos, Some(m), ok_single_line)
            }
            _ => Ok(None),
        }
    }

    /// `tryGetArgAsLambda`'s `getParamAsFunction(pt)`.
    fn lambda_param_as_function(&mut self, param_pos: i32, pt: Option<u32>, ok_single_line: bool) -> P<Option<u32>> {
        let param = self.lambda_param_with_pos(pt.into_iter().collect(), false)?;
        let c = self.param_clause_of(vec![param], pt);
        let c = self.copy_pos(param, c);
        self.lambda_function_term(param_pos, c, ok_single_line)
    }

    /// `tryGetArgAsLambda`'s `getFunctionTerm(params)`: a function where an arrow follows, its body indented, else
    /// on its line where that is allowed.
    fn lambda_function_term(&mut self, param_pos: i32, params: u32, ok_single_line: bool) -> P<Option<u32>> {
        let kind = match self.curr_c() {
            C::RightArrow => K::TermFunction,
            C::ContextArrow => K::TermContextFunction,
            _ => return Ok(None),
        };
        let body = if self.try_ahead(C::Indent) {
            self.block_expr_on_indent(false)?
        } else if !ok_single_line || self.in_region_paren() {
            return Ok(None);
        } else {
            self.next();
            self.expr()?
        };
        Ok(Some(self.auto_end_pos(param_pos, |p| Ok(p.mk(kind, vec![(F::ParamClause, params), (F::Body, body)])))?))
    }

    /// `tryGetArgAsLambda`'s `getParamWithPos(mods, fullTypeOK)`.
    fn lambda_param_with_pos(&mut self, mods: Vec<u32>, full_type_ok: bool) -> P<u32> {
        self.auto_pos(|p| p.lambda_param(mods, full_type_ok))
    }

    /// `tryGetArgAsLambda`'s `getParam(mods, fullTypeOK)`: `erased` and `using` before a name its modifiers.
    fn lambda_param(&mut self, mut mods: Vec<u32>, full_type_ok: bool) -> P<u32> {
        loop {
            let name = match self.curr_c() {
                C::Ident => {
                    let d = self.dialect;
                    let m = if self.peek(C::Ident) {
                        if self.at_soft(d.allow_erased_defs, "erased") {
                            Some(K::ModErased)
                        } else if self.at_soft(d.allow_given_using, "using") {
                            Some(K::ModUsing)
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                    if let Some(kind) = m {
                        let m = self.mod_at_cur(kind);
                        mods.insert(0, m);
                        continue;
                    }
                    self.term_name()?
                }
                C::Underscore => self.name_placeholder(),
                _ => return self.expected_c(C::Ident),
            };
            let tpe = if full_type_ok || !mods.is_empty() { self.get_decl_tpe_opt(full_type_ok)? } else { None };
            let mut fields: Vec<(F, u32)> = mods.into_iter().map(|m| (F::Mods, m)).collect();
            fields.push((F::Name, name));
            fields.extend(tpe.map(|t| (F::Decltpe, t)));
            return Ok(self.mk(K::Param, fields));
        }
    }

    /// `getFewerBracesApplyOnColon(fun, startPos, okSingleLineLambda)`.
    fn get_fewer_braces_apply_on_colon(&mut self, fun: u32, start: i32, ok_single_line: bool) -> P<Option<u32>> {
        let colon_pos = self.curr_index();
        let Some(arg) = self.get_fewer_braces_arg_on_colon(ok_single_line)? else { return Ok(None) };
        let end = self.prev_index();
        let clause = self.mk(K::TermArgClause, vec![(F::Values, arg)]);
        let clause = self.at_pos(colon_pos, end, clause);
        let app = self.mk(K::TermApply, vec![(F::Fun, fun), (F::ArgClause, clause)]);
        let app = self.at_pos(start, end, app);
        Ok(Some(self.simple_expr_rest(app, true, start)?))
    }

    /// `argumentExprsOrPrefixExpr(location)`: an infix operator's right side.
    fn argument_exprs_or_prefix_expr(&mut self, location: Location) -> P<u32> {
        let brace = self.at(C::LeftBrace);
        if !brace && !self.at(C::LeftParen) {
            return self.prefix_expr(false);
        }
        let lp_pos = self.curr_index();
        let args = if brace {
            vec![self.block_expr_on_brace(true, false)?]
        } else {
            self.next();
            if self.accept_opt(C::RightParen) {
                Vec::new()
            } else {
                let a = self.argument_exprs_in_parens(location)?;
                self.in_parens_after_open(a)?
            }
        };
        let rest = match self.curr_c() {
            C::Dot | C::LeftParen | C::LeftBracket | C::LeftBrace | C::Underscore => true,
            c if c.eol() && !brace && !self.dialect.allow_significant_indentation => self.try_ahead(C::LeftBrace),
            _ => false,
        };
        if rest {
            let t = self.make_tuple_term(lp_pos, args, |p, x| p.tuple_single_term(x))?;
            return self.simple_expr_rest(t, true, lp_pos);
        }
        self.make_tuple_term(lp_pos, args, |p, arg| {
            let res = p.maybe_anonymous_function(arg, Location::NoStat, false)?;
            Ok(if brace { p.tuple_single_term(res)? } else { Err(vec![res]) })
        })
    }

    /// `getArgClauseOnBrace()`.
    pub(super) fn get_arg_clause_on_brace(&mut self) -> P<u32> {
        self.auto_pos(|p| {
            let arg = p.block_expr_on_brace(true, false)?;
            Ok(p.mk(K::TermArgClause, vec![(F::Values, arg)]))
        })
    }

    /// `getArgClauseOnParen(location)`: `using` before the arguments the clause's `mod`.
    pub(super) fn get_arg_clause_on_paren(&mut self, location: Location) -> P<u32> {
        self.auto_pos(|p| {
            p.next();
            if p.accept_opt(C::RightParen) {
                return Ok(p.mk(K::TermArgClause, Vec::new()));
            }
            let using = p.curr_text() == "using" && p.s.might_start_stat(p.peek_c(), false);
            let mod_ = using.then(|| p.mod_at_cur(K::ModUsing));
            let mut fields: Vec<(F, u32)> = p.argument_exprs_in_parens(location)?.into_iter().map(|a| (F::Values, a)).collect();
            fields.extend(mod_.map(|m| (F::Mod, m)));
            let clause = p.mk(K::TermArgClause, fields);
            p.in_parens_after_open(clause)
        })
    }

    /// `argumentExprsInParens(location)`.
    fn argument_exprs_in_parens(&mut self, location: Location) -> P<Vec<u32>> {
        let mut args = vec![self.expr_at(location, true)?];
        while self.accept_opt(C::Comma) {
            args.push(self.expr_at(location, true)?);
        }
        Ok(args)
    }

    /// `makeTuple(lpPos, body, Lit.Unit(), Term.Tuple.apply)(single)`: none a `Lit.Unit`, one what `single` keeps
    /// (`Ok`) or a tuple of what it gives (`Err`).
    fn make_tuple_term(&mut self, lp_pos: i32, body: Vec<u32>, single: impl FnOnce(&mut Self, u32) -> P<Result<u32, Vec<u32>>>) -> P<u32> {
        let tuple = |p: &mut Self, args: Vec<u32>| p.auto_end_pos(lp_pos, |p| Ok(p.mk(K::TermTuple, args.into_iter().map(|a| (F::Args, a)).collect())));
        match body[..] {
            [] => self.auto_end_pos(lp_pos, |p| Ok(p.mk_value(K::LitUnit, Vec::new(), String::new()))),
            [t] => match single(self, t)? {
                Ok(x) => Ok(x),
                Err(xs) => tuple(self, xs),
            },
            _ => tuple(self, body),
        }
    }

    /// `getTupleSingleTerm(single)`: a named element (`Term.Assign`) a tuple of one.
    fn tuple_single_term(&mut self, single: u32) -> P<Result<u32, Vec<u32>>> {
        Ok(if self.kind(single) == K::TermAssign && self.dialect.allow_named_tuples { Err(vec![single]) } else { Ok(single) })
    }

    /// `inParensOrTupleOrUnitExpr(allowRepeated)`.
    fn in_parens_or_tuple_or_unit_expr(&mut self, allow_repeated: bool) -> P<u32> {
        let lp_pos = self.curr_index();
        self.next();
        let args = if self.accept_opt(C::RightParen) {
            Vec::new()
        } else {
            let mut args = vec![self.expr_at(Location::PostfixStat, allow_repeated)?];
            while self.accept_opt(C::Comma) {
                args.push(self.expr_at(Location::PostfixStat, allow_repeated)?);
            }
            self.in_parens_after_open(args)?
        };
        self.make_tuple_term(lp_pos, args, |p, x| {
            let x = p.maybe_anonymous_function(x, Location::NoStat, false)?;
            p.tuple_single_term(x)
        })
    }

    /* -------- blocks -------- */

    /// `blockExprPartial[T](orElse)`: a partial function where `case` begins the block, else `or_else`.
    fn block_expr_partial(&mut self, close: C, or_else: impl FnOnce(&mut Self) -> P<u32>) -> P<u32> {
        let start = self.curr_index();
        match self.block_expr_maybe_partial_raw()? {
            Some(term) => {
                self.accept_after_opt_nl(close)?;
                self.auto_end_pos(start, |_| Ok(term))
            }
            None => or_else(self),
        }
    }

    /// `blockExprMaybePartialRaw()`: past the block's opening, its cases' partial function.
    fn block_expr_maybe_partial_raw(&mut self) -> P<Option<u32>> {
        let partial = self.peek(C::KwCase) && self.ahead(|p| p.is_case_intro_on_kw_case());
        if !partial {
            return Ok(None);
        }
        self.next();
        let cases = self.case_clauses()?;
        Ok(Some(self.mk(K::TermPartialFunction, cases.into_iter().map(|c| (F::Cases, c)).collect())))
    }

    /// `blockRaw(allowRepeated)`.
    fn block_raw(&mut self, allow_repeated: bool) -> P<u32> {
        let stats = self.block_stat_seq(allow_repeated)?;
        Ok(self.mk(K::TermBlock, stats.into_iter().map(|s| (F::Stats, s)).collect()))
    }

    /// `blockMaybeRaw(allowRepeated, keepBlock)`: a single term its own, but one a detached comment precedes.
    fn block_maybe_raw(&mut self, allow_repeated: bool, keep_block: bool) -> P<u32> {
        let stats = self.block_stat_seq(allow_repeated)?;
        if let [t] = stats[..] {
            if is_term(self.kind(t)) && !(keep_block || self.is_preceded_by_detached_comment(self.curr_index(), self.end_index(t))) {
                return Ok(t);
            }
        }
        Ok(self.mk(K::TermBlock, stats.into_iter().map(|s| (F::Stats, s)).collect()))
    }

    /// `blockOnIndent(keepBlock)`.
    fn block_on_indent(&mut self, keep_block: bool) -> P<u32> {
        self.auto_pos_opt(|p| {
            p.next();
            let b = p.block_maybe_raw(false, keep_block)?;
            p.indented_after_open(b)
        })
    }

    /// `blockExprOnIndent(keepBlock)`.
    fn block_expr_on_indent(&mut self, keep_block: bool) -> P<u32> {
        self.block_expr_partial(C::Outdent, |p| p.block_on_indent(keep_block))
    }

    /// `blockOnBrace(allowRepeated)`.
    fn block_on_brace(&mut self, allow_repeated: bool) -> P<u32> {
        self.auto_pos(|p| {
            p.next();
            let stats = p.block_stat_seq(allow_repeated)?;
            p.in_braces_after_open(())?;
            Ok(p.mk(K::TermBlock, stats.into_iter().map(|s| (F::Stats, s)).collect()))
        })
    }

    /// `blockExprOnBrace(allowRepeated, isOptional)`.
    fn block_expr_on_brace(&mut self, allow_repeated: bool, is_optional: bool) -> P<u32> {
        self.block_expr_partial(C::RightBrace, |p| if is_optional { p.block_on_other(allow_repeated) } else { p.block_on_brace(allow_repeated) })
    }

    /// `blockOnOther(allowRepeated)`: a single term its own, else a block from where its statements began.
    fn block_on_other(&mut self, allow_repeated: bool) -> P<u32> {
        let start = self.curr_index();
        let stats = self.block_stat_seq(allow_repeated)?;
        if let [t] = stats[..] {
            if is_term(self.kind(t)) {
                return Ok(t);
            }
        }
        self.auto_end_pos(start, |p| Ok(p.mk(K::TermBlock, stats.into_iter().map(|s| (F::Stats, s)).collect())))
    }

    /* -------- cases and enumerators -------- */

    /// `caseClause(forceSingleExpr)`: after `case`, its pattern, guard and body; a `match` in a guard a sub-match.
    fn case_clause(&mut self, force_single_expr: bool) -> P<u32> {
        if self.at(C::KwCase) {
            return self.refused("not expected `case`".to_string());
        }
        let start = self.prev_index();
        self.auto_end_pos(start, |p| {
            let pat = p.pattern()?;
            if !p.at(C::KwIf) {
                let body = p.case_body(force_single_expr)?;
                return Ok(p.mk(K::Case, vec![(F::Pat, pat), (F::Body, body)]));
            }
            let first_guard = p.guard_on_if()?;
            if p.at(C::KwIf) {
                let t = p.guard_on_if()?;
                if p.kind(t) != K::TermMatch {
                    let at = p.s.tokens[p.beg_index(t).max(0) as usize].start;
                    return Err(Stop::Refused(at, "`match` expected".to_string()));
                }
                let sub = p.as_sub_match(t);
                return Ok(p.mk(K::Case, vec![(F::Pat, pat), (F::Cond, first_guard), (F::Body, sub)]));
            }
            if p.kind(first_guard) == K::TermMatch && !p.at(C::RightArrow) {
                let sub = p.as_sub_match(first_guard);
                return Ok(p.mk(K::Case, vec![(F::Pat, pat), (F::Body, sub)]));
            }
            let body = p.case_body(force_single_expr)?;
            Ok(p.mk(K::Case, vec![(F::Pat, pat), (F::Cond, first_guard), (F::Body, body)]))
        })
    }

    /// `caseClause`'s `caseBody()`.
    fn case_body(&mut self, force_single_expr: bool) -> P<u32> {
        self.accept(C::RightArrow)?;
        if self.at(C::Indent) {
            self.block_expr_on_indent(false)
        } else if force_single_expr {
            self.expr_at(Location::BlockStat, false)
        } else {
            self.block_on_other(false)
        }
    }

    /// `caseClause`'s `asSubMatch(t)`.
    fn as_sub_match(&mut self, t: u32) -> u32 {
        let fields = self.nodes[t as usize].fields.clone();
        let s = self.mk(K::TermSubMatch, fields);
        self.copy_pos(t, s)
    }

    /// `casesBlock()`.
    fn cases_block(&mut self) -> P<u32> {
        match self.cases_block_if_any()? {
            Some(b) => Ok(b),
            None => self.expected_c(C::KwCase),
        }
    }

    /// `casesBlockIfAny()`.
    fn cases_block_if_any(&mut self) -> P<Option<u32>> {
        let cases = self.case_clauses_if_any()?;
        Ok(cases.map(|cs| self.mk(K::TermCasesBlock, cs.into_iter().map(|c| (F::Cases, c)).collect())))
    }

    /// `caseClauses()`.
    fn case_clauses(&mut self) -> P<Vec<u32>> {
        match self.case_clauses_if_any()? {
            Some(cs) => Ok(cs),
            None => self.expected_c(C::KwCase),
        }
    }

    /// `caseClausesIfAny()`: each `case` and the separator before the next.
    fn case_clauses_if_any(&mut self) -> P<Option<Vec<u32>>> {
        let mut cases = Vec::new();
        while self.at(C::KwCase) && self.is_case_intro_on_kw_case() {
            self.next();
            cases.push(self.case_clause(false)?);
            if self.at_stat_sep() {
                self.try_ahead_if(|p| Ok(p.at(C::KwCase) && p.is_case_intro_on_kw_case()))?;
            }
        }
        Ok((!cases.is_empty()).then_some(cases))
    }

    /// `guardOnIf()`.
    fn guard_on_if(&mut self) -> P<u32> {
        self.next();
        self.postfix_expr(false)
    }

    /// `enumeratorGuardOnIf()`.
    fn enumerator_guard_on_if(&mut self) -> P<u32> {
        self.auto_pos(|p| {
            let g = p.guard_on_if()?;
            Ok(p.mk(K::EnumGuard, vec![(F::Cond, g)]))
        })
    }

    /// `enumerators()`: generators and guards to the end of their braces, parentheses or region, or `do`, `yield`.
    fn enumerators(&mut self) -> P<Vec<u32>> {
        let not_enums_end = |c: C| !(matches!(c, C::Outdent | C::KwDo | C::KwYield) || c.close_delim());
        let mut enums = Vec::new();
        loop {
            let first = enums.is_empty();
            enums.push(self.enumerator(first)?);
            while self.at(C::KwIf) {
                enums.push(self.enumerator_guard_on_if()?);
            }
            let more = if self.at_stat_sep() {
                let ok = not_enums_end(self.peek_c());
                self.next_if(ok)
            } else {
                self.is_implicit_stat_sep() && not_enums_end(self.curr_c())
            };
            if !more {
                return Ok(enums);
            }
        }
    }

    /// `enumerator(isFirst)`.
    fn enumerator(&mut self, is_first: bool) -> P<u32> {
        if self.at(C::KwIf) && !is_first { self.enumerator_guard_on_if() } else { self.generator() }
    }

    /// `generator()`: `p <- e`, `case p <- e`, `p = e`.
    fn generator(&mut self) -> P<u32> {
        self.auto_pos(|p| {
            p.accept_opt(C::KwVal);
            let is_case = p.accept_opt(C::KwCase);
            let pat = p.pattern1(true)?;
            let has_eq = p.accept_opt(C::Equals);
            if !has_eq {
                p.accept(C::LeftArrow)?;
            }
            let rhs = if p.at(C::Indent) { p.block_on_indent(false)? } else { p.expr()? };
            let kind = if has_eq { K::EnumVal } else if is_case { K::EnumCaseGenerator } else { K::EnumGenerator };
            Ok(p.mk(kind, vec![(F::Pat, pat), (F::Rhs, rhs)]))
        })
    }

    /* -------- literals, interpolations and quotes -------- */

    /// `rawLiteral(unary)`: past it, the current literal, a unary operator folded into a number's value (`-1`, `!true`)
    /// or kept beside its literal (`Lit.WithUnary`).
    pub(super) fn raw_literal(&mut self, unary: Option<&str>) -> P<u32> {
        let at = self.curr_offset();
        let text = self.curr_text().to_string();
        let (kind, value, folded) = match self.curr_c() {
            C::KwTrue | C::KwFalse => {
                let v = self.at(C::KwTrue);
                match unary {
                    Some("!") => (K::LitBoolean, (!v).to_string(), true),
                    _ => (K::LitBoolean, v.to_string(), false),
                }
            }
            C::KwNull => (K::LitNull, "null".to_string(), false),
            C::Constant => {
                let numeric = unary.filter(|u| *u != "!");
                let minus_before = self.s.is_ident(self.curr_index() - 1, "-");
                super::names::constant_with_unary(&text, numeric, minus_before).map_err(|m| Stop::Refused(at, m))?
            }
            _ => return Err(Stop::NotYet("literal")),
        };
        let lit = self.mk_value(kind, Vec::new(), value);
        let res = match unary {
            Some(op) if !folded => {
                let prev = self.prev_index();
                let o = self.mk_value(K::TermName, Vec::new(), op.to_string());
                let o = self.at_pos1(prev, o);
                let arg = self.at_cur_pos(lit);
                self.mk(K::LitWithUnary, vec![(F::Op, o), (F::Arg, arg)])
            }
            _ => lit,
        };
        self.next();
        Ok(res)
    }

    /// `interpolateTerm()`: its id, parts and splices.
    fn interpolate_term(&mut self) -> P<u32> {
        self.interpolate_with(K::TermInterpolate, |p| p.unquote_expr())
    }

    /// `interpolateWith(arg, result)`: its id, parts and splices, each splice's argument `arg`'s.
    pub(super) fn interpolate_with(&mut self, kind: K, arg: impl Fn(&mut Self) -> P<u32>) -> P<u32> {
        self.auto_pos(|p| {
            let prefix_value = super::names::ident_value(p.curr_text());
            let unicode = matches!(prefix_value.as_str(), "s" | "f");
            let prefix = p.mk_value(K::TermName, Vec::new(), prefix_value);
            let prefix = p.at_cur_pos(prefix);
            p.next();
            p.accept(C::InterpStart)?;
            let (mut parts, mut args) = (Vec::new(), Vec::new());
            loop {
                match p.curr_c() {
                    C::InterpPart => {
                        let value = crate::fix::nodes::part_value(p.curr_text(), unicode);
                        let part = p.mk_value(K::LitString, Vec::new(), value);
                        parts.push(p.at_cur_pos(part));
                        p.next();
                    }
                    C::InterpSpliceStart => {
                        p.next();
                        args.push(arg(p)?);
                        p.accept(C::InterpSpliceEnd)?;
                    }
                    _ => break,
                }
            }
            p.accept(C::InterpEnd)?;
            let mut fields = vec![(F::Prefix, prefix)];
            fields.extend(parts.into_iter().map(|x| (F::Parts, x)));
            fields.extend(args.into_iter().map(|x| (F::Args, x)));
            Ok(p.mk(kind, fields))
        })
    }

    /// `unquoteExpr()`: a splice's identifier, block or `this`.
    fn unquote_expr(&mut self) -> P<u32> {
        match self.curr_c() {
            C::Ident => self.term_name(),
            C::LeftBrace => self.expr_at(Location::UnquoteStat, true),
            C::KwThis => Ok(self.anon_this()),
            _ => self.refused("error in interpolated string: identifier, `this' or block expected".to_string()),
        }
    }

    /// `MacroQuoted`: `'{ ... }`, `'[ T ]`, `'x`, `'this`.
    pub(super) fn macro_quoted(&mut self) -> P<u32> {
        self.auto_pos(|p| {
            p.ctx.quoted_splice += 1;
            let r = (|| {
                p.next();
                match p.curr_c() {
                    C::LeftBrace => {
                        let b = p.auto_pos(|p| {
                            p.next();
                            let b = p.block_raw(false)?;
                            p.in_braces_after_open(b)
                        })?;
                        Ok(p.mk(K::TermQuote, vec![(F::Body, b)]))
                    }
                    C::LeftBracket => {
                        p.next();
                        let t = p.type_block()?;
                        p.in_brackets_after_open(())?;
                        Ok(p.mk(K::TermQuotedType, vec![(F::Tpe, t)]))
                    }
                    C::Ident => {
                        let n = p.term_name()?;
                        Ok(p.mk(K::TermQuote, vec![(F::Body, n)]))
                    }
                    C::KwThis => {
                        let t = p.anon_this();
                        Ok(p.mk(K::TermQuote, vec![(F::Body, t)]))
                    }
                    _ => p.refused("Macro quote must be followed by id, brace or bracket".to_string()),
                }
            })();
            p.ctx.quoted_splice -= 1;
            r
        })
    }

    /// `MacroSplicedIdent`'s test: `$x` and `$this` in a quote, `${` anywhere, under `allowSpliceAndQuote`.
    pub(super) fn macro_spliced_ident_here(&self) -> bool {
        let text = self.curr_text();
        if !self.dialect.allow_splice_and_quote || !text.starts_with('$') {
            return false;
        }
        if text.len() > 1 {
            return self.ctx.quoted_splice > 0;
        }
        match self.peek_c() {
            C::LeftBrace => true,
            C::Ident => !self.peek_text().is_empty() && self.ctx.quoted_splice > 0,
            C::KwThis => self.ctx.quoted_splice > 0,
            _ => false,
        }
    }

    /// `MacroSplicedIdent`: the splice `macro_spliced_ident_here` found.
    pub(super) fn macro_spliced_ident(&mut self) -> P<u32> {
        let text = self.curr_text().to_string();
        if text.len() > 1 {
            return self.auto_pos(|p| {
                let ident = &text[1..];
                let body = if ident == "this" {
                    p.anon_this()
                } else {
                    let n = p.mk_value(K::TermName, Vec::new(), ident.to_string());
                    p.at_cur_pos_next(n)
                };
                Ok(p.mk(K::TermSplice, vec![(F::Body, body)]))
            });
        }
        self.auto_pos(|p| {
            p.next();
            match p.curr_c() {
                C::LeftBrace if p.ctx.pattern > 0 => {
                    let pat = p.auto_pos(|p| {
                        p.next();
                        let x = p.pattern()?;
                        p.in_braces_after_open(x)
                    })?;
                    Ok(p.mk(K::TermSplicePat, vec![(F::Pat, pat)]))
                }
                C::LeftBrace => {
                    let b = p.auto_pos(|p| {
                        p.next();
                        let b = p.block_raw(false)?;
                        p.in_braces_after_open(b)
                    })?;
                    Ok(p.mk(K::TermSplice, vec![(F::Body, b)]))
                }
                C::Ident => {
                    let n = p.term_name()?;
                    Ok(p.mk(K::TermSplice, vec![(F::Body, n)]))
                }
                _ => {
                    let t = p.anon_this();
                    Ok(p.mk(K::TermSplice, vec![(F::Body, t)]))
                }
            }
        })
    }

    /* -------- maybeAnonymousFunction -------- */

    /// `maybeAnonymousFunction(t, location, includeArg)`: a term with a placeholder of its own a
    /// `Term.AnonymousFunction` of it.
    pub(super) fn maybe_anonymous_function(&mut self, t: u32, location: Location, include_arg: bool) -> P<u32> {
        if location.anon_func_ok() && self.has_placeholder(t, include_arg) {
            let f = self.mk(K::TermAnonymousFunction, vec![(F::Body, t)]);
            return Ok(self.copy_pos(t, f));
        }
        Ok(t)
    }

    /// `PlaceholderChecks.isPlaceholder(tree)`.
    fn is_placeholder(&self, t: u32) -> bool {
        match self.kind(t) {
            K::TermPlaceholder => true,
            K::TermAscribe | K::TermRepeated => self.field(t, F::Expr).is_some_and(|e| self.kind(e) == K::TermPlaceholder),
            _ => false,
        }
    }

    /// `PlaceholderChecks.isBlockPlaceholder(args)`.
    fn is_block_placeholder(&self, args: &[u32]) -> bool {
        match args {
            [a] if self.kind(*a) == K::TermBlock => matches!(self.fields_of(*a, F::Stats)[..], [s] if self.is_placeholder(s)),
            _ => false,
        }
    }

    /// `PlaceholderChecks.hasPlaceholder(tree, includeArg)`: the tree's own placeholder, breadth first through the
    /// trees a placeholder function's body is made of; an argument clause's parent's kind with it.
    fn has_placeholder(&self, tree: u32, include_arg: bool) -> bool {
        let mut queue: VecDeque<(u32, Option<K>)> = VecDeque::from([(tree, None)]);
        while let Some((t, parent)) = queue.pop_front() {
            if self.is_placeholder(t) {
                return t != tree || include_arg;
            }
            let found = match self.kind(t) {
                K::TermSelect => {
                    queue.extend(self.field(t, F::Qual).map(|q| (q, Some(K::TermSelect))));
                    false
                }
                K::TermTuple => self.fields_of(t, F::Args).iter().any(|&a| self.is_placeholder(a)),
                K::Init => self.fields_of(t, F::ArgClauses).iter().any(|&c| self.fields_of(c, F::Values).iter().any(|&v| self.is_placeholder(v))),
                K::TermApply => {
                    queue.extend(self.field(t, F::Fun).map(|f| (f, Some(K::TermApply))));
                    queue.extend(self.field(t, F::ArgClause).map(|c| (c, Some(K::TermApply))));
                    false
                }
                K::TermArgClause => {
                    let values = self.fields_of(t, F::Values);
                    if self.is_block_placeholder(&values) {
                        true
                    } else if parent == Some(K::TermApplyInfix) {
                        queue.extend(values.into_iter().map(|v| (v, Some(K::TermArgClause))));
                        false
                    } else {
                        values.iter().any(|&v| self.is_placeholder(v))
                    }
                }
                K::TermApplyInfix => {
                    queue.extend(self.field(t, F::Lhs).map(|l| (l, Some(K::TermApplyInfix))));
                    queue.extend(self.field(t, F::ArgClause).map(|c| (c, Some(K::TermApplyInfix))));
                    false
                }
                K::TermApplyUnary => {
                    queue.extend(self.field(t, F::Arg).map(|a| (a, Some(K::TermApplyUnary))));
                    false
                }
                K::TermApplyType => {
                    queue.extend(self.field(t, F::Fun).map(|f| (f, Some(K::TermApplyType))));
                    false
                }
                K::TermNew => {
                    queue.extend(self.field(t, F::Init).map(|i| (i, Some(K::TermNew))));
                    false
                }
                K::TermAnonymousFunction if t == tree => return false,
                K::TermAnonymousFunction => false,
                _ => self.children(t).any(|c| self.is_placeholder(c)),
            };
            if found {
                return true;
            }
        }
        false
    }

    /// `inRegionParen`: the current token in a parenthesis's region.
    fn in_region_paren(&self) -> bool {
        matches!(self.s.head(self.curr_regions()), Some(crate::fix::scanner::R::Paren(_)))
    }
}

/// The state of `postfixExpr`'s loop an operator's productions read: the stack's base, the chain's first term and
/// start, the right side so far and where it ended.
struct Chain {
    base: usize,
    rhs0: u32,
    start: i32,
    rhs_k: u32,
    rhs_end_k: i32,
}
