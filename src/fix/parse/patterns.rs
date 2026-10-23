//! ScalametaParser.scala's patterns (`SeqContextSensitive` and its four contexts) with the pattern infix context
//! (`PatInfixContext`) and the pattern types (`patternTyp`, `patternTypeArgs`, `typeVar`).

use super::chars::{get_type, is_left_assoc, is_lower_case, precedence, LOWERCASE_LETTER};
use super::{Parser, Stop, P};
use crate::fix::nodes::{F, K};
use crate::fix::scanner::C;

/// A pattern context (`noSeq`, `seqOKWithNamed`, `noSeqWithNamed`; `seqOK` the quasiquotes'): whether a sequence
/// wildcard `_*` and a named tuple's element are allowed.
#[derive(Clone, Copy)]
pub(super) struct Seq {
    sequence_ok: bool,
    named_tuple_ok: bool,
}

const NO_SEQ: Seq = Seq { sequence_ok: false, named_tuple_ok: false };
const SEQ_OK_WITH_NAMED: Seq = Seq { sequence_ok: true, named_tuple_ok: true };
const NO_SEQ_WITH_NAMED: Seq = Seq { sequence_ok: false, named_tuple_ok: true };

/// `PatInfixContext.UnfinishedInfix`: `[lhs op]`.
#[derive(Clone, Copy)]
pub(super) struct UnfinishedPat {
    lhs: u32,
    op: u32,
}

impl<'a> Parser<'a> {
    /// `noSeq.pattern()`.
    pub(super) fn pattern(&mut self) -> P<u32> {
        self.pattern_in(NO_SEQ)
    }

    /// `patterns()`.
    fn patterns(&mut self, ctx: Seq) -> P<Vec<u32>> {
        let mut pats = vec![self.pattern_in(ctx)?];
        while self.accept_opt(C::Comma) {
            pats.push(self.pattern_in(ctx)?);
        }
        Ok(pats)
    }

    /// `pattern()`: `patternAlternatives`, `a | b | c` right-nested, each alternative from its left side to the last
    /// one's end.
    fn pattern_in(&mut self, ctx: Seq) -> P<u32> {
        let mut pats = Vec::new();
        loop {
            let pat = self.pattern1_in(ctx, false)?;
            if self.at_ident("|") {
                self.next();
                pats.push(pat);
                continue;
            }
            let end = self.end_index(pat);
            let mut rt_all = pat;
            for &lt_one in pats.iter().rev() {
                let alt = self.mk(K::PatAlternative, vec![(F::Lhs, lt_one), (F::Rhs, rt_all)]);
                let start = self.beg_index(lt_one);
                rt_all = self.at_pos(start, end, alt);
            }
            return Ok(rt_all);
        }
    }

    /// `getSeqWildcard(isEnabled, elseF, mapF)`: at `_` a sequence wildcard where enabled, else `or_else`.
    fn get_seq_wildcard(&mut self, ctx: Seq, enabled: bool, or_else: impl FnOnce(&mut Self) -> P<u32>, map: impl FnOnce(&mut Self, u32) -> u32) -> P<u32> {
        if enabled && self.at(C::Underscore) {
            let start = self.curr_index();
            if let Some(w) = self.get_seq_wildcard_at_underscore(ctx)? {
                let end = self.prev_index();
                let w = self.at_pos(start, end, w);
                return Ok(map(self, w));
            }
        }
        or_else(self)
    }

    /// `getSeqWildcardAtUnderscore()`: `_*` before `)`, `}` or the end, where a sequence is allowed.
    fn get_seq_wildcard_at_underscore(&mut self, ctx: Seq) -> P<Option<u32>> {
        if !(ctx.sequence_ok && self.peek(C::Ident) && self.peek_text() == "*") {
            return Ok(None);
        }
        self.try_parse(|p| {
            p.next_twice();
            p.new_lines_opt();
            Ok(matches!(p.curr_c(), C::RightParen | C::RightBrace | C::Eof).then(|| p.mk(K::PatSeqWildcard, Vec::new())))
        })
    }

    /// `noSeq.pattern1(isForComprehension)`.
    pub(super) fn pattern1(&mut self, is_for_comprehension: bool) -> P<u32> {
        self.pattern1_in(NO_SEQ, is_for_comprehension)
    }

    /// `pattern1(isForComprehension)`: a typed pattern after `:`.
    fn pattern1_in(&mut self, ctx: Seq, is_for_comprehension: bool) -> P<u32> {
        let p = self.pattern2_in(ctx, is_for_comprehension)?;
        if !self.at(C::Colon) {
            return Ok(p);
        }
        let typed = |s: &mut Self| -> P<u32> {
            let t = s.pattern_typ(false)?;
            Ok(s.mk(K::PatTyped, vec![(F::Lhs, p), (F::Rhs, t)]))
        };
        let colon_varargs = self.dialect.allow_colon_for_extractor_varargs;
        let pat = match self.kind(p) {
            K::PatVar => {
                self.next();
                self.get_seq_wildcard(ctx, colon_varargs, typed, |s, w| s.mk(K::PatBind, vec![(F::Lhs, p), (F::Rhs, w)]))?
            }
            K::PatWildcard => {
                self.next();
                self.get_seq_wildcard(ctx, colon_varargs, typed, |_, w| w)?
            }
            _ if self.dialect.allow_all_typed_patterns => {
                self.next();
                typed(self)?
            }
            _ => p,
        };
        if pat == p {
            return Ok(p);
        }
        let start = self.beg_index(p);
        self.auto_end_pos(start, |_| Ok(pat))
    }

    /// `noSeq.pattern2(isForComprehension)`.
    pub(super) fn pattern2(&mut self, is_for_comprehension: bool) -> P<u32> {
        self.pattern2_in(NO_SEQ, is_for_comprehension)
    }

    /// `pattern2(isForComprehension)`: a binding after `@`.
    fn pattern2_in(&mut self, ctx: Seq, is_for_comprehension: bool) -> P<u32> {
        let p = self.pattern3(ctx, is_for_comprehension)?;
        if !self.at(C::At) {
            return Ok(p);
        }
        let at_varargs = self.dialect.allow_at_for_extractor_varargs;
        let pat = match self.kind(p) {
            K::TermName => {
                let at = self.s.tokens[self.beg_index(p).max(0) as usize].start;
                return Err(Stop::Refused(at, "Pattern variables must start with a lower-case letter. (SLS 8.1.1.)".to_string()));
            }
            K::PatVar => {
                self.next();
                let rhs = self.get_seq_wildcard(ctx, at_varargs, |s| s.pattern3(ctx, false), |_, w| w)?;
                self.mk(K::PatBind, vec![(F::Lhs, p), (F::Rhs, rhs)])
            }
            K::PatWildcard => {
                self.next();
                self.get_seq_wildcard(ctx, at_varargs, |s| s.pattern3(ctx, false), |_, w| w)?
            }
            _ => p,
        };
        if pat == p {
            return Ok(p);
        }
        let start = self.beg_index(p);
        self.auto_end_pos(start, |_| Ok(pat))
    }

    /// `pattern3(isForComprehension)`: the infix chain of simple patterns, each operator's left side reduced by
    /// precedence on `PatInfixContext`'s stack.
    fn pattern3(&mut self, ctx: Seq, is_for_comprehension: bool) -> P<u32> {
        let lhs = self.simple_pattern(ctx, false, is_for_comprehension)?;
        let base = self.pat_infix.len();
        let mut rhs = lhs;
        while self.at(C::Ident) && self.curr_text() != "|" {
            let op = self.term_name()?;
            if self.at(C::LeftBracket) {
                return self.refused("infix patterns cannot have type arguments: not expected `[`".to_string());
            }
            let end = self.end_index(rhs);
            let reduced = self.reduce_pat_stack(base, rhs, end, op);
            self.pat_infix.push(UnfinishedPat { lhs: reduced, op });
            rhs = self.simple_pattern(ctx, true, false)?;
        }
        let end = self.end_index(rhs);
        Ok(self.drain_pat_stack(base, rhs, end))
    }

    /// `PatInfixContext.finishInfixExpr(unf, rhs, rhsEnd)`.
    fn finish_pat_infix(&mut self, unf: UnfinishedPat, rhs: u32, rhs_end: i32) -> u32 {
        let named = |p: &Self, x: u32| p.kind(x) == K::PatAssign;
        let lhs = match self.kind(unf.lhs) {
            K::PatTuple => match self.fields_of(unf.lhs, F::Args)[..] {
                [arg] if !self.dialect.allow_named_tuples || !named(self, arg) => arg,
                _ => unf.lhs,
            },
            _ => unf.lhs,
        };
        let values = match self.kind(rhs) {
            K::LitUnit if self.dialect.allow_empty_infix_args => Vec::new(),
            K::PatTuple => {
                let args = self.fields_of(rhs, F::Args);
                let named_one = matches!(args[..], [a] if named(self, a));
                if !named_one || !self.dialect.allow_named_tuples { args } else { vec![rhs] }
            }
            _ => vec![rhs],
        };
        let args = self.mk(K::PatArgClause, values.into_iter().map(|v| (F::Values, v)).collect());
        let args = self.copy_pos(rhs, args);
        let n = self.mk(K::PatExtractInfix, vec![(F::Lhs, lhs), (F::Op, unf.op), (F::ArgClause, args)]);
        let start = self.beg_index(unf.lhs);
        self.at_pos(start, rhs_end, n)
    }

    /// `InfixContext.reduceStack` over `PatInfixContext`'s stack.
    fn reduce_pat_stack(&mut self, base: usize, curr: u32, curr_end: i32, op: u32) -> u32 {
        if self.pat_infix.len() == base {
            return curr;
        }
        let opv = self.nodes[op as usize].value.clone().unwrap_or_default();
        let (op_precedence, if_same) = (precedence(&opv), !is_left_assoc(&opv));
        let mut rhs = curr;
        loop {
            let lhs = *self.pat_infix.last().unwrap();
            let lhs_precedence = precedence(self.nodes[lhs.op as usize].value.as_deref().unwrap_or(""));
            let diff = op_precedence - lhs_precedence;
            if diff > 0 || diff == 0 && if_same {
                return rhs;
            }
            self.pat_infix.pop();
            let fin = self.finish_pat_infix(lhs, rhs, curr_end);
            if self.pat_infix.len() == base {
                return fin;
            }
            rhs = fin;
        }
    }

    /// `InfixContext.drainStack` over `PatInfixContext`'s stack.
    fn drain_pat_stack(&mut self, base: usize, curr: u32, curr_end: i32) -> u32 {
        let mut rhs = curr;
        while self.pat_infix.len() > base {
            let lhs = self.pat_infix.pop().unwrap();
            rhs = self.finish_pat_infix(lhs, rhs, curr_end);
        }
        rhs
    }

    /// `badPattern3(tok)`: the refusal where no simple pattern starts, a trailing `*` named.
    fn bad_pattern3<T>(&self, ctx: Seq) -> P<T> {
        let star = self.pat_infix.last().filter(|u| self.nodes[u.op as usize].value.as_deref() == Some("*"));
        let (is_underscore, is_star) = match star {
            Some(u) => (self.kind(u.lhs) == K::PatWildcard, true),
            None => (false, false),
        };
        let (is_comma, is_delimiter) = (self.at(C::Comma), matches!(self.curr_c(), C::RightParen | C::RightBrace));
        let subtext = match (is_underscore, is_star, ctx.sequence_ok) {
            (true, true, true) if is_comma => Some("bad use of _* (a sequence pattern must be the last pattern)"),
            (true, true, true) if is_delimiter => Some("bad brace or paren after _*"),
            (true, true, false) if is_delimiter => Some("bad use of _* (sequence pattern not allowed)"),
            (false, true, true) if is_delimiter => Some("use _* to match a sequence"),
            (false, true, _) if is_comma || is_delimiter => Some("trailing * is not a valid pattern"),
            _ => None,
        };
        match subtext {
            Some(t) => self.refused(format!("bad simple pattern: {}", t)),
            None => self.refused("illegal start of simple pattern".to_string()),
        }
    }

    /// `simplePattern(onError, isRhs, isForComprehension)`, in `PatternContext`.
    fn simple_pattern(&mut self, ctx: Seq, is_rhs: bool, is_for_comprehension: bool) -> P<u32> {
        let start = self.curr_index();
        self.ctx.pattern += 1;
        let pat = self.simple_pattern_body(ctx, is_rhs, is_for_comprehension);
        self.ctx.pattern -= 1;
        let pat = pat?;
        self.auto_end_pos(start, |_| Ok(pat))
    }

    fn simple_pattern_body(&mut self, ctx: Seq, is_rhs: bool, is_for_comprehension: bool) -> P<u32> {
        match self.curr_c() {
            C::Ident if matches!(self.curr_text(), "-" | "+" | "~" | "!") && self.peek_c().literal() => {
                let unary = self.curr_text().to_string();
                self.next();
                self.raw_literal(Some(&unary))
            }
            C::Ident | C::KwThis => self.pattern_stable_id(ctx, is_for_comprehension),
            C::Underscore => match self.get_seq_wildcard_at_underscore(ctx)? {
                Some(w) => Ok(w),
                None => {
                    self.next();
                    Ok(self.mk(K::PatWildcard, Vec::new()))
                }
            },
            C::MacroQuote => {
                let term = self.macro_quoted()?;
                Ok(self.mk(K::PatMacro, vec![(F::Body, term)]))
            }
            c if c.literal() => self.literal(),
            C::InterpId => self.interpolate_with(K::PatInterpolate, |p| p.unquote_pattern()),
            C::LeftParen => {
                let lp_pos = self.curr_index();
                self.next();
                let pats = if self.accept_opt(C::RightParen) {
                    Vec::new()
                } else {
                    let pats = self.patterns(NO_SEQ_WITH_NAMED)?;
                    self.in_parens_after_open(pats)?
                };
                self.make_tuple_pat(lp_pos, pats, is_rhs)
            }
            C::KwGiven => {
                self.next();
                let t = self.pattern_typ(false)?;
                Ok(self.mk(K::PatGiven, vec![(F::Tpe, t)]))
            }
            _ => self.bad_pattern3(ctx),
        }
    }

    /// `simplePattern`'s tuple (`makeTuple`): none a `Lit.Unit`, one the pattern itself but a named element or, on
    /// an operator's right side, any but a tuple of one.
    fn make_tuple_pat(&mut self, lp_pos: i32, pats: Vec<u32>, is_rhs: bool) -> P<u32> {
        let tuple = |p: &mut Self, args: Vec<u32>| p.auto_end_pos(lp_pos, |p| Ok(p.mk(K::PatTuple, args.into_iter().map(|a| (F::Args, a)).collect())));
        match pats[..] {
            [] => self.auto_end_pos(lp_pos, |p| Ok(p.mk_value(K::LitUnit, Vec::new(), String::new()))),
            [t] => {
                let keep = if self.kind(t) == K::PatAssign && self.dialect.allow_named_tuples {
                    false
                } else {
                    !is_rhs || self.kind(t) == K::PatTuple && self.fields_of(t, F::Args).len() == 1
                };
                if keep { Ok(t) } else { tuple(self, vec![t]) }
            }
            _ => tuple(self, pats),
        }
    }

    /// `simplePattern`'s stable identifier: an extractor's application where `(` follows, else a named element, a
    /// repeated one, a variable (`Pat.Var`), a name or a selection.
    fn pattern_stable_id(&mut self, ctx: Seq, is_for_comprehension: bool) -> P<u32> {
        let text = self.curr_text();
        let backquoted = !text.is_empty() && text.starts_with('`') && text.ends_with('`');
        let sid = self.stable_id()?;
        let targs = if self.at(C::LeftBracket) { Some(self.pattern_type_args()?) } else { None };
        if self.at(C::LeftParen) {
            let fun = match targs {
                Some(x) => {
                    let start = self.beg_index(sid);
                    self.auto_end_pos(start, |p| Ok(p.mk(K::TermApplyType, vec![(F::Fun, sid), (F::TargClause, x)])))?
                }
                None => sid,
            };
            let args = self.auto_pos(|p| {
                let pats = p.argument_patterns()?;
                Ok(p.mk(K::PatArgClause, pats.into_iter().map(|x| (F::Values, x)).collect()))
            })?;
            return Ok(self.mk(K::PatExtract, vec![(F::Fun, fun), (F::ArgClause, args)]));
        }
        if let Some(x) = targs {
            let written = |p: &Self, n: u32| {
                let (b, e) = (p.beg_index(n), p.end_index(n));
                p.s.text[p.s.tokens[b as usize].start as usize..p.s.tokens[e as usize].end as usize].to_string()
            };
            return self.refused(format!("pattern must be a value or have parens: {}{}", written(self, sid), written(self, x)));
        }
        match self.kind(sid) {
            K::TermName if ctx.named_tuple_ok && self.accept_opt(C::Equals) => {
                let rhs = self.pattern_in(NO_SEQ_WITH_NAMED)?;
                Ok(self.mk(K::PatAssign, vec![(F::Name, sid), (F::Rhs, rhs)]))
            }
            K::TermName => {
                if self.at_soft(self.dialect.allow_postfix_star_vararg_splices, "*") && matches!(self.peek_c(), C::RightParen | C::Comma) {
                    self.next();
                    return Ok(self.mk(K::PatRepeated, vec![(F::Name, sid)]));
                }
                let first = self.nodes[sid as usize].value.as_deref().and_then(|v| v.encode_utf16().next()).unwrap_or(0);
                let is_var = if !is_for_comprehension && backquoted {
                    self.at(C::Colon) || self.at(C::At)
                } else {
                    first == 0x5f || get_type(first) == LOWERCASE_LETTER || self.dialect.allow_upper_case_pattern_var_binding && self.at(C::At)
                };
                Ok(if is_var { self.mk(K::PatVar, vec![(F::Name, sid)]) } else { sid })
            }
            K::TermSelect => Ok(sid),
            _ => self.refused("unreachable".to_string()),
        }
    }

    /// `argumentPatterns()`: an extractor's patterns in parentheses, sequences and named elements allowed.
    fn argument_patterns(&mut self) -> P<Vec<u32>> {
        self.accept(C::LeftParen)?;
        let pats = if self.at(C::RightParen) { Vec::new() } else { self.patterns(SEQ_OK_WITH_NAMED)? };
        self.in_parens_after_open(pats)
    }

    /// `noSeq.unquotePattern()`: an interpolation's splice, braces dropped.
    fn unquote_pattern(&mut self) -> P<u32> {
        self.new_line_opt();
        if self.accept_opt(C::LeftBrace) {
            let p = self.pattern()?;
            self.in_braces_after_open(p)
        } else {
            self.pattern()
        }
    }

    /// `patternTyp(allowInfix)`, in `PatternTypeContext`.
    fn pattern_typ(&mut self, allow_infix: bool) -> P<u32> {
        self.ctx.pattern_type += 1;
        let t = if allow_infix { self.infix_pattern_type() } else { self.compound_type() };
        self.ctx.pattern_type -= 1;
        t
    }

    /// `infixPatternTypeImpl()`.
    fn infix_pattern_type(&mut self) -> P<u32> {
        let t = if self.at(C::LeftParen) { self.tuple_infix_type(true)? } else { self.compound_type()? };
        match self.curr_c() {
            C::KwForsome => self.existential_type_on_for_some(t),
            C::Ident if self.curr_text() != "|" => self.infix_type_rest(t, false, false),
            _ => Ok(t),
        }
    }

    /// `patternTypeArgs()`: an extractor's type arguments, in `PatternTypeContext`, a variable's name a `Type.Var`.
    fn pattern_type_args(&mut self) -> P<u32> {
        self.ctx.pattern_type += 1;
        let r = self.type_clause_in_brackets(K::TypeArgClause, |p| {
            let t = p.infix_pattern_type()?;
            let text = p.s.text_of(p.prev_index());
            Ok(p.type_var(t, text))
        });
        self.ctx.pattern_type -= 1;
        r
    }

    /// `typeVar(t, text)`: a type name in a pattern's type brackets, its `text` a variable's (`isPatVar`), a type
    /// variable.
    pub(super) fn type_var(&mut self, t: u32, text: &str) -> u32 {
        let units: Vec<u16> = text.encode_utf16().collect();
        let pat_var = units.first().is_some_and(|&u| is_lower_case(u)) && !(units[0] == 0x60 && units[units.len() - 1] == 0x60);
        if self.kind(t) == K::TypeName && self.ctx.pattern_type > 0 && self.ctx.type_brackets > 0 && pat_var {
            let v = self.mk(K::TypeVar, vec![(F::Name, t)]);
            return self.copy_pos(t, v);
        }
        t
    }
}
