//! ScalametaParser.scala's types outside patterns (`outPattern`'s productions of `PatternContextSensitive`), the
//! refinements and `typedOpt`; the branches of capture checking and of pure functions, which neither dialect the port
//! reads enables, stop (`Stop::NotYet`).

use super::chars::{is_left_assoc, precedence};
use super::terms::op_needs_parens;
use super::{Parser, Stop, P};
use crate::fix::nodes::{F, K};
use crate::fix::scanner::C;
use std::collections::VecDeque;

/// `TypeInfixContext.UnfinishedInfix`: `[lhs op]`.
#[derive(Clone, Copy)]
pub(super) struct UnfinishedType {
    lhs: u32,
    op: u32,
}

fn not_yet<T>(production: &'static str) -> P<T> {
    Err(Stop::NotYet(production))
}

/// `Type.FunctionType`'s classes.
fn is_function_type(k: K) -> bool {
    matches!(k, K::TypeFunction | K::TypeContextFunction)
}

impl<'a> Parser<'a> {
    /// `tupleInfixType(allowFunctionType)`: in parentheses types or a dependent function's typed parameters, then a
    /// function past `=>`, else their tuple and what a simple, an annotated, a compound and an infix type read past it.
    pub(super) fn tuple_infix_type(&mut self, allow_function_type: bool) -> P<u32> {
        self.auto_pos_opt(|p| {
            let mut has_implicits = false;
            let open_paren_pos = p.curr_index();
            p.accept(C::LeftParen)?;
            let ts = if p.accept_opt(C::RightParen) {
                Vec::new()
            } else {
                let mut ts = vec![p.param_or_type(&mut has_implicits, allow_function_type)?];
                while p.accept_opt(C::Comma) {
                    ts.push(p.param_or_type(&mut has_implicits, allow_function_type)?);
                }
                p.in_parens_after_open(ts)?
            };
            let has_params = ts.iter().any(|&t| p.kind(t) == K::TypeTypedParam);
            let has_types = ts.iter().any(|&t| p.kind(t) != K::TypeTypedParam);
            if has_types && has_params {
                return p.refused("can't mix function type and dependent function type syntaxes".to_string());
            }
            if has_params && !p.dialect.allow_dependent_function_types {
                return p.refused("dependent function types are not supported".to_string());
            }
            if !has_types && !has_implicits && p.at(C::LeftParen) {
                return p.refused("can't have multiple parameter lists in function types".to_string());
            }
            p.type_captures(true)?;
            if allow_function_type {
                if let Some(f) = p.get_after_opt_new_line(|p| p.type_func_on_arrow_opt(open_paren_pos, &ts, false))? {
                    return Ok(f);
                }
            }
            let tuple = p.make_tuple_type(open_paren_pos, ts)?;
            let simple = p.simple_type_rest(tuple, open_paren_pos)?;
            let annotated = p.annot_type_rest(simple, open_paren_pos)?;
            let compound = p.compound_type_rest(annotated, open_paren_pos)?;
            let t = p.infix_type_rest(compound, false, false)?;
            if t == compound {
                Ok(compound)
            } else {
                p.auto_end_pos(open_paren_pos, |_| Ok(t))
            }
        })
    }

    /// `tupleInfixType`'s `paramOrType`: a typed parameter, else past the modifiers (`implicit`, once in the list, and
    /// `erased`) the type, a `Type.FunctionArg` of them where there are any.
    fn param_or_type(&mut self, has_implicits: &mut bool, allow_function_type: bool) -> P<u32> {
        let mut mods: Vec<u32> = Vec::new();
        loop {
            if let Some(x) = self.named_type_opt(&mods, allow_function_type)? {
                return Ok(x);
            }
            if self.at(C::KwImplicit) && !*has_implicits && allow_function_type {
                self.next();
                *has_implicits = true;
            } else if allow_function_type && self.at_soft(self.dialect.allow_erased_defs, "erased") {
                let m = self.mk(K::ModErased, Vec::new());
                mods.push(self.at_cur_pos_next(m));
            } else {
                let tpe = self.maybe_param_type(allow_function_type)?;
                let Some(&first) = mods.first() else { return Ok(tpe) };
                let mut fields: Vec<(F, u32)> = mods.iter().map(|&m| (F::Mods, m)).collect();
                fields.push((F::Tpe, tpe));
                let start = self.beg_index(first);
                return self.auto_end_pos(start, |p| Ok(p.mk(K::TypeFunctionArg, fields)));
            }
        }
    }

    /// `typeLambdaOrPoly()`: after type parameters `=>>` a type lambda, `=>` a polymorphic function's type.
    fn type_lambda_or_poly(&mut self) -> P<u32> {
        let quants = self.type_param_clause_opt()?;
        self.new_line_opt();
        match self.curr_c() {
            C::TypeLambdaArrow => {
                self.next();
                let t = self.type_indented_opt()?;
                Ok(self.mk(K::TypeLambda, vec![(F::TparamClause, quants), (F::Tpe, t)]))
            }
            C::RightArrow => {
                let at = self.curr_offset();
                self.next();
                let t = self.type_indented_opt()?;
                if !is_function_type(self.kind(t)) {
                    return Err(Stop::Refused(at, "polymorphic function types must have a value parameter".to_string()));
                }
                Ok(self.mk(K::TypePolyFunction, vec![(F::TparamClause, quants), (F::Tpe, t)]))
            }
            _ => self.refused("expected =>> or =>".to_string()),
        }
    }

    /// `typeIndentedOpt()`: `maybeIndented(typ())`.
    pub(super) fn type_indented_opt(&mut self) -> P<u32> {
        if self.accept_opt(C::Indent) {
            let t = self.typ()?;
            self.indented_after_open(t)
        } else {
            self.typ()
        }
    }

    /// `typ()`.
    pub(super) fn typ(&mut self) -> P<u32> {
        self.typ_in(false)
    }

    /// `typ(inParam)`: a type lambda or a polymorphic function's type, else an infix type or a tuple's; then, past a
    /// line break, `forSome`, a match type's cases or a function's arrow, a parameter's value type its result where
    /// `in_param`.
    fn typ_in(&mut self, in_param: bool) -> P<u32> {
        self.auto_pos_opt(|p| {
            let start = p.curr_index();
            let t = if p.at(C::LeftBracket) && p.dialect.allow_type_lambdas { p.type_lambda_or_poly()? } else { p.infix_type_or_tuple(false)? };
            let allow_type_match = p.dialect.allow_type_match;
            let res = p.get_after_opt_new_line(|p| match p.curr_c() {
                C::KwForsome => Ok(Some(p.existential_type_on_for_some(t)?)),
                C::KwMatch if allow_type_match => {
                    p.next();
                    let cases = p.type_case_clauses()?;
                    Ok(Some(p.mk(K::TypeMatch, vec![(F::Tpe, t), (F::CasesBlock, cases)])))
                }
                _ => p.type_func_on_arrow_opt(start, &[t], in_param),
            })?;
            Ok(res.unwrap_or(t))
        })
    }

    /// `getAfterOptNewLine(body)`: past a line break `body`, the stream as it was where it gives none.
    fn get_after_opt_new_line<T>(&mut self, body: impl FnOnce(&mut Self) -> P<Option<T>>) -> P<Option<T>> {
        if self.curr_c().eol() {
            self.try_parse(|p| {
                p.next();
                body(p)
            })
        } else {
            body(self)
        }
    }

    /// `paramValueType(allowRepeated)`: `T*` a `Type.Repeated`.
    fn param_value_type(&mut self, allow_repeated: bool) -> P<u32> {
        let start = self.curr_index();
        let t = self.typ_in(true)?;
        if !allow_repeated || !self.at_ident("*") {
            return Ok(t);
        }
        self.next();
        self.auto_end_pos(start, |p| Ok(p.mk(K::TypeRepeated, vec![(F::Tpe, t)])))
    }

    /// `paramType()`.
    pub(super) fn param_type(&mut self) -> P<u32> {
        if self.at(C::RightArrow) {
            return self.auto_pos(|p| {
                p.next();
                let t = p.param_value_type(p.dialect.allow_by_name_repeated_parameters)?;
                Ok(p.mk(K::TypeByName, vec![(F::Tpe, t)]))
            });
        }
        if self.at_soft(self.dialect.allow_pure_functions, "->") {
            return not_yet("Type.PureByName");
        }
        self.param_value_type(true)
    }

    /// `maybeParamType(allowFunctionType)`.
    fn maybe_param_type(&mut self, allow_function_type: bool) -> P<u32> {
        if allow_function_type { self.param_type() } else { self.typ() }
    }

    /// `namedTypeOpt(mods, allowFunctionType)`: `name: T`, a dependent function's typed parameter.
    fn named_type_opt(&mut self, mods: &[u32], allow_function_type: bool) -> P<Option<u32>> {
        if !(self.at(C::Ident) && self.peek(C::Colon)) {
            return Ok(None);
        }
        let start = self.curr_index();
        let value = super::names::ident_value(self.curr_text());
        self.next_twice();
        let mod_pos = mods.first().map_or(start, |&m| self.beg_index(m));
        let name = self.mk_value(K::TypeName, Vec::new(), value);
        let name = self.at_pos1(start, name);
        let tpe = self.maybe_param_type(allow_function_type)?;
        let mut fields = vec![(F::Name, name), (F::Typ, tpe)];
        fields.extend(mods.iter().map(|&m| (F::Mods, m)));
        Ok(Some(self.auto_end_pos(mod_pos, |p| Ok(p.mk(K::TypeTypedParam, fields)))?))
    }

    /// `typeBlock()`: a quoted type's type definitions before it under `allowQuotedTypeVariables`, else the type.
    pub(super) fn type_block(&mut self) -> P<u32> {
        if !(self.dialect.allow_quoted_type_variables && self.at(C::KwType)) {
            return self.typ();
        }
        self.auto_pos(|p| {
            let mut fields = vec![(F::TypeDefs, p.type_def_or_dcl(Vec::new())?)];
            while p.accept_if_stat_sep() && p.at(C::KwType) {
                fields.push((F::TypeDefs, p.type_def_or_dcl(Vec::new())?));
            }
            fields.push((F::Tpe, p.typ()?));
            Ok(p.mk(K::TypeBlock, fields))
        })
    }

    /// `typeCaptures(needCaret, allowCaptures)` under `allowCaptureChecking`, not ported: none else.
    fn type_captures(&self, need_caret: bool) -> P<()> {
        let at = if need_caret { self.at_ident("^") } else { self.at(C::LeftBrace) };
        if self.dialect.allow_capture_checking && at {
            return not_yet("Type.Capturing");
        }
        Ok(())
    }

    /// `typeFuncOnArrowOpt(paramPos, params, inParam)`: at `=>` or `?=>` a function of `params`, each a type variable
    /// where it reads as one (`typeVar`).
    fn type_func_on_arrow_opt(&mut self, param_pos: i32, params: &[u32], in_param: bool) -> P<Option<u32>> {
        let kind = match self.curr_c() {
            C::RightArrow => K::TypeFunction,
            C::ContextArrow => K::TypeContextFunction,
            C::Ident if self.dialect.allow_pure_functions && matches!(self.curr_text(), "->" | "?->") => return not_yet("Type.PureFunction"),
            _ => return Ok(None),
        };
        let mut values = Vec::new();
        for &t in params {
            let text = self.written_text(t);
            values.push((F::Values, self.type_var(t, &text)));
        }
        let clause = self.auto_end_pos(param_pos, |p| Ok(p.mk(K::TypeFuncParamClause, values)))?;
        self.next();
        let f = self.auto_end_pos(param_pos, |p| {
            let res = if p.accept_opt(C::Indent) {
                let r = if in_param { p.param_value_type(false)? } else { p.typ()? };
                p.indented_after_open(r)?
            } else if in_param {
                p.param_value_type(false)?
            } else {
                p.typ()?
            };
            Ok(p.mk(kind, vec![(F::ParamClause, clause), (F::Res, res)]))
        })?;
        Ok(Some(f))
    }

    /// The source text of a positioned tree (`Tree.text`).
    fn written_text(&self, n: u32) -> String {
        let (b, e) = (self.beg_index(n), self.end_index(n));
        self.s.text[self.s.tokens[b as usize].start as usize..self.s.tokens[e as usize].end as usize].to_string()
    }

    /// `typeCaseClauses()`: a match type's cases, indented or in braces.
    fn type_case_clauses(&mut self) -> P<u32> {
        self.auto_pos(|p| {
            let cases = |p: &mut Self| -> P<Vec<u32>> {
                let mut all = Vec::new();
                while p.at(C::KwCase) {
                    all.push(p.auto_pos(|p| {
                        p.next();
                        let pat = p.infix_type_or_tuple(true)?;
                        p.accept(C::RightArrow)?;
                        let body = p.type_indented_opt()?;
                        Ok(p.mk(K::TypeCase, vec![(F::Pat, pat), (F::Body, body)]))
                    })?);
                    p.new_lines_opt();
                }
                Ok(all)
            };
            let all = if p.accept_opt(C::Indent) {
                let all = cases(p)?;
                p.indented_after_open(all)?
            } else {
                p.new_line_opt();
                if !p.accept_opt(C::LeftBrace) {
                    return p.expected_c(C::LeftBrace);
                }
                let all = cases(p)?;
                p.in_braces_after_open(all)?
            };
            Ok(p.mk(K::TypeCasesBlock, all.into_iter().map(|c| (F::Cases, c)).collect()))
        })
    }

    /// `typeArg()`: `name = T` a named argument, else `typeVar` of the type and the text of the token it starts at.
    fn type_arg(&mut self) -> P<u32> {
        if self.at(C::Ident) && self.peek(C::Equals) {
            let start = self.curr_index();
            let value = super::names::ident_value(self.curr_text());
            self.next_twice();
            let name = self.mk_value(K::TypeName, Vec::new(), value);
            let name = self.at_pos1(start, name);
            return self.auto_end_pos(start, |p| {
                let rhs = p.typ()?;
                Ok(p.mk(K::TypeAssign, vec![(F::Name, name), (F::Rhs, rhs)]))
            });
        }
        let text = self.curr_text();
        let t = self.typ()?;
        Ok(self.type_var(t, text))
    }

    /// `typeArgsInBrackets()`.
    pub(super) fn type_args_in_brackets(&mut self) -> P<u32> {
        self.type_clause_in_brackets(K::TypeArgClause, |p| p.type_arg())
    }

    /// `infixTypeOrTuple(inMatchType)`.
    fn infix_type_or_tuple(&mut self, in_match_type: bool) -> P<u32> {
        if self.at(C::LeftParen) { self.tuple_infix_type(!in_match_type) } else { self.infix_type(in_match_type, false) }
    }

    /// `infixType(inMatchType, inGivenSig)`: a type lambda where a placeholder stands in the infix type.
    fn infix_type(&mut self, in_match_type: bool, in_given_sig: bool) -> P<u32> {
        let t = self.compound_type_in(in_match_type, in_given_sig)?;
        let t = self.infix_type_rest(t, in_match_type, in_given_sig)?;
        Ok(self.maybe_anonymous_lambda(t))
    }

    /// `startInfixType(inGivenSig)`.
    pub(super) fn start_infix_type(&mut self, in_given_sig: bool) -> P<u32> {
        self.infix_type(false, in_given_sig)
    }

    /// `startModType()`: `annotType()`.
    pub(super) fn start_mod_type(&mut self) -> P<u32> {
        self.annot_type(false)
    }

    /// `infixTypeRest(t, inMatchType, inGivenSig)`: the operators after `t`, by precedence under
    /// `useInfixTypePrecedence`.
    pub(super) fn infix_type_rest(&mut self, t: u32, in_match_type: bool, in_given_sig: bool) -> P<u32> {
        if self.dialect.use_infix_type_precedence {
            self.infix_type_rest_with_precedence(t, in_match_type, in_given_sig)
        } else {
            self.infix_type_rest_impl(t, in_match_type, in_given_sig)
        }
    }

    /// `InfixTypeIdent`: an identifier other than a pure arrow (`allowPureFunctions`) and a vararg's `*`.
    fn is_infix_type_ident(&self) -> bool {
        match self.curr_text() {
            "->" | "?->" if self.dialect.allow_pure_functions => false,
            "*" => !matches!(self.peek_c(), C::RightParen | C::Comma | C::Equals | C::RightBrace | C::Eof),
            _ => true,
        }
    }

    /// `newLineOptWhenFollowedBy(TypeIntro)`: past a line break a type's first token follows.
    fn new_line_opt_when_type_intro(&mut self) {
        let c = self.peek_c();
        let intro = matches!(c, C::Ident | C::KwSuper | C::KwThis | C::LeftParen | C::At | C::Underscore) || c.literal() && self.dialect.allow_literal_types;
        if self.curr_c().eol() && intro {
            self.next();
        }
    }

    /// `Type.ApplyInfix(lhs, op, rhs)` from `lhs`'s start to `rhs_end`.
    fn mk_infix_type(&mut self, lhs: u32, op: u32, rhs: u32, rhs_end: i32) -> u32 {
        let n = self.mk(K::TypeApplyInfix, vec![(F::Lhs, lhs), (F::Op, op), (F::Rhs, rhs)]);
        let start = self.beg_index(lhs);
        self.at_pos(start, rhs_end, n)
    }

    /// `infixTypeRestImpl(t, f)`: a left-associative operator applied at once, a right-associative one's application
    /// kept for what follows.
    fn infix_type_rest_impl(&mut self, t: u32, in_match_type: bool, in_given_sig: bool) -> P<u32> {
        let mut t = t;
        let mut kept: Vec<(u32, u32)> = Vec::new();
        while self.at(C::Ident) && self.is_infix_type_ident() {
            let op = self.type_name()?;
            self.new_line_opt_when_type_intro();
            let rhs = self.compound_type_in(in_match_type, in_given_sig)?;
            if is_left_assoc(&self.value_of(op)) {
                let end = self.end_index(rhs);
                t = self.mk_infix_type(t, op, rhs, end);
            } else {
                kept.push((t, op));
                t = rhs;
            }
        }
        while let Some((lhs, op)) = kept.pop() {
            let end = self.end_index(t);
            t = self.mk_infix_type(lhs, op, t, end);
        }
        Ok(t)
    }

    /// `infixTypeRestWithPrecedence(t, inMatchType, inGivenSig)`: each operator, a line's leading one among them, its
    /// left side reduced by precedence on `TypeInfixContext`'s stack.
    fn infix_type_rest_with_precedence(&mut self, t: u32, in_match_type: bool, in_given_sig: bool) -> P<u32> {
        let base = self.type_infix.len();
        let mut rhs = t;
        loop {
            let op = if self.at(C::InfixLf) {
                match self.get_leading_infix_name(K::TypeName)? {
                    Some(op) => op,
                    None => break,
                }
            } else if self.at(C::Ident) && self.is_infix_type_ident() {
                self.type_name()?
            } else {
                break;
            };
            self.new_line_opt_when_type_intro();
            let end = self.end_index(rhs);
            let lhs = self.reduce_type_stack(base, rhs, end, op);
            self.type_infix.push(UnfinishedType { lhs, op });
            rhs = self.compound_type_in(in_match_type, in_given_sig)?;
        }
        let end = self.end_index(rhs);
        Ok(self.drain_type_stack(base, rhs, end))
    }

    /// `InfixContext.reduceStack(base, curr, currEnd, op)` over `TypeInfixContext`'s stack.
    fn reduce_type_stack(&mut self, base: usize, curr: u32, curr_end: i32, op: u32) -> u32 {
        if self.type_infix.len() == base {
            return curr;
        }
        let opv = self.value_of(op);
        let (op_precedence, if_same) = (precedence(&opv), !is_left_assoc(&opv));
        let mut rhs = curr;
        loop {
            let lhs = *self.type_infix.last().unwrap();
            if op_needs_parens(op_precedence, precedence(&self.value_of(lhs.op)), if_same) {
                return rhs;
            }
            self.type_infix.pop();
            let fin = self.mk_infix_type(lhs.lhs, lhs.op, rhs, curr_end);
            if self.type_infix.len() == base {
                return fin;
            }
            rhs = fin;
        }
    }

    /// `InfixContext.drainStack(base, curr, currEnd)` over `TypeInfixContext`'s stack.
    fn drain_type_stack(&mut self, base: usize, curr: u32, curr_end: i32) -> u32 {
        let mut rhs = curr;
        while self.type_infix.len() > base {
            let lhs = self.type_infix.pop().unwrap();
            rhs = self.mk_infix_type(lhs.lhs, lhs.op, rhs, curr_end);
        }
        rhs
    }

    /// `compoundType()`.
    pub(super) fn compound_type(&mut self) -> P<u32> {
        self.compound_type_in(false, false)
    }

    /// `compoundType(inMatchType, inGivenSig)`: a refinement alone, else an annotated type and, outside a given's
    /// signature, its `with` types and refinements.
    fn compound_type_in(&mut self, in_match_type: bool, in_given_sig: bool) -> P<u32> {
        if let Some(t) = self.refinement(None)? {
            return Ok(t);
        }
        let start = self.curr_index();
        let annot = self.annot_type_at(start, in_match_type)?;
        if in_given_sig { Ok(annot) } else { self.compound_type_rest(annot, start) }
    }

    /// `compoundTypeRest(typ, startPos)`: `with` types from `start` until a refinement ends them.
    fn compound_type_rest(&mut self, typ: u32, start: i32) -> P<u32> {
        let mut previous = typ;
        loop {
            match self.refinement(Some(previous))? {
                Some(t) if t != previous => return Ok(t),
                _ => {
                    if !self.accept_opt(C::KwWith) {
                        return Ok(previous);
                    }
                    let rhs = self.annot_type(false)?;
                    previous = self.auto_end_pos(start, |p| Ok(p.mk(K::TypeWith, vec![(F::Lhs, previous), (F::Rhs, rhs)])))?;
                }
            }
        }
    }

    /// `annotType(inMatchType)`.
    fn annot_type(&mut self, in_match_type: bool) -> P<u32> {
        let start = self.curr_index();
        self.annot_type_at(start, in_match_type)
    }

    /// `annotType(startPos, inMatchType)`.
    fn annot_type_at(&mut self, start: i32, in_match_type: bool) -> P<u32> {
        let t = self.simple_type_in(in_match_type)?;
        self.annot_type_rest(t, start)
    }

    /// `annotTypeRest(t, startPos)`: the annotations after a type, an annotated type from `start`.
    fn annot_type_rest(&mut self, t: u32, start: i32) -> P<u32> {
        let annots = self.annots(false, true)?;
        if annots.is_empty() {
            return Ok(t);
        }
        self.auto_end_pos(start, |p| {
            let mut fields = vec![(F::Tpe, t)];
            fields.extend(annots.into_iter().map(|a| (F::Annots, a)));
            Ok(p.mk(K::TypeAnnotate, fields))
        })
    }

    /// `simpleType()`.
    pub(super) fn simple_type(&mut self) -> P<u32> {
        self.simple_type_in(false)
    }

    /// `simpleType(inMatchType)`: a path's type, a tuple's, a spliced one, a wildcard or a placeholder, a literal; then
    /// projections and applications.
    fn simple_type_in(&mut self, in_match_type: bool) -> P<u32> {
        let start = self.curr_index();
        let d = self.dialect;
        let res = match self.curr_c() {
            C::Ident if self.peek(C::Dot) => self.path_simple_type()?,
            C::LeftParen => {
                let ts = self.types_in_parens()?;
                self.make_tuple_type(start, ts)?
            }
            C::Ident if self.macro_spliced_ident_here() => {
                let term = self.macro_spliced_ident()?;
                self.mk(K::TypeMacro, vec![(F::Body, term)])
            }
            C::Underscore if in_match_type => {
                self.next();
                self.mk(K::TypePatWildcard, Vec::new())
            }
            C::Underscore if d.allow_underscore_as_type_placeholder || d.allow_type_lambdas && self.ctx.pattern_type == 0 && self.ctx.type_brackets > 1 && !matches!(self.peek_c(), C::Supertype | C::Subtype) => {
                self.next();
                self.mk(K::TypeAnonymousParam, Vec::new())
            }
            C::Underscore => self.wildcard_type()?,
            c if c.literal() => self.literal()?,
            C::Ident if super::terms::is_unary(self.curr_text()) && self.peek_c().literal() => {
                let unary = self.curr_text();
                self.next();
                let pos = self.prev_index();
                let lit = self.raw_literal(Some(unary))?;
                self.auto_end_pos(pos, |_| Ok(lit))?
            }
            C::Ident if !in_match_type => {
                let text = self.curr_text();
                let variant = |text: &str| if text.starts_with('+') { K::ModCovariant } else { K::ModContravariant };
                if text == "?" && d.allow_question_mark_as_type_wildcard {
                    self.wildcard_type()?
                } else if d.allow_star_as_type_placeholder && matches!(text, "*" | "+*" | "-*") {
                    self.next();
                    let v = (text != "*").then(|| variant(text));
                    self.anonymous_param_with_variant(start, v)
                } else if matches!(text, "+" | "-") && (d.allow_plus_minus_underscore_as_ident || d.allow_underscore_as_type_placeholder) && self.try_ahead(C::Underscore) {
                    self.next();
                    if d.allow_underscore_as_type_placeholder {
                        self.anonymous_param_with_variant(start, Some(variant(text)))
                    } else {
                        self.mk_value(K::TypeName, Vec::new(), format!("{}_", text))
                    }
                } else {
                    self.path_simple_type()?
                }
            }
            _ => self.path_simple_type()?,
        };
        let res = self.auto_end_pos_opt(start, |_| Ok(res))?;
        let rest = self.simple_type_rest(res, start)?;
        self.type_captures(true)?;
        Ok(rest)
    }

    /// `simpleType`'s `wildcardType()`: `_` or `?` and its bounds.
    fn wildcard_type(&mut self) -> P<u32> {
        self.next();
        let bounds = self.type_bounds()?;
        Ok(self.mk(K::TypeWildcard, vec![(F::Bounds, bounds)]))
    }

    /// `simpleType`'s `anonymousParamWithVariant(modOpt)`: a placeholder, its variance at `start`.
    fn anonymous_param_with_variant(&mut self, start: i32, variant: Option<K>) -> u32 {
        let fields = match variant {
            Some(k) => {
                let v = self.mk(k, Vec::new());
                vec![(F::Variant, self.at_pos1(start, v))]
            }
            None => Vec::new(),
        };
        self.mk(K::TypeAnonymousParam, fields)
    }

    /// `pathSimpleType()`: a path's type, `p.type` a singleton.
    fn path_simple_type(&mut self) -> P<u32> {
        let r = self.path(true)?;
        if self.accept_opt(C::Dot) {
            self.accept(C::KwType)?;
            return Ok(self.mk(K::TypeSingleton, vec![(F::Ref, r)]));
        }
        match self.kind(r) {
            K::TermSelect => {
                let (qual, name) = (self.field(r, F::Qual).unwrap(), self.field(r, F::Name).unwrap());
                let tn = self.name_as_type(name);
                Ok(self.mk(K::TypeSelect, vec![(F::Qual, qual), (F::Name, tn)]))
            }
            K::TermName => {
                let value = self.nodes[r as usize].value.clone().unwrap_or_default();
                Ok(self.mk_value(K::TypeName, Vec::new(), value))
            }
            _ => {
                let at = self.s.tokens[self.beg_index(r).max(0) as usize].start;
                Err(Stop::Refused(at, "identifier expected".to_string()))
            }
        }
    }

    /// `simpleTypeRest(t, startPos)`: projections and applications, an application past a line break that indents.
    fn simple_type_rest(&mut self, mut t: u32, start: i32) -> P<u32> {
        loop {
            match self.curr_c() {
                C::Hash => {
                    self.next();
                    t = self.auto_end_pos(start, |p| {
                        let n = p.type_name()?;
                        Ok(p.mk(K::TypeProject, vec![(F::Qual, t), (F::Name, n)]))
                    })?;
                }
                C::LeftBracket => t = self.type_apply(t, start)?,
                c if c.at_eol() && self.is_indenting_or_eol(true) && self.try_ahead(C::LeftBracket) => t = self.type_apply(t, start)?,
                _ => return Ok(t),
            }
        }
    }

    fn type_apply(&mut self, t: u32, start: i32) -> P<u32> {
        self.auto_end_pos(start, |p| {
            let args = p.type_args_in_brackets()?;
            Ok(p.mk(K::TypeApply, vec![(F::Tpe, t), (F::ArgClause, args)]))
        })
    }

    /// `typesInParens()`: in parentheses, each a typed parameter or a type.
    fn types_in_parens(&mut self) -> P<Vec<u32>> {
        self.next();
        let mut ts = Vec::new();
        loop {
            let t = match self.named_type_opt(&[], false)? {
                Some(t) => t,
                None => self.typ()?,
            };
            ts.push(t);
            if !self.accept_opt(C::Comma) {
                break;
            }
        }
        self.in_parens_after_open(ts)
    }

    /// `makeTupleType(lpPos, body)`: none refused; one the type itself, a type lambda where it holds a placeholder,
    /// but a typed parameter's under named tuples; more a tuple.
    fn make_tuple_type(&mut self, lp_pos: i32, body: Vec<u32>) -> P<u32> {
        match body[..] {
            [] => self.refused("illegal literal type (), use Unit instead".to_string()),
            [t] => {
                let single = self.maybe_anonymous_lambda(t);
                if self.kind(single) == K::TypeTypedParam && self.dialect.allow_named_tuples {
                    self.auto_end_pos(lp_pos, |p| Ok(p.mk(K::TypeTuple, vec![(F::Args, single)])))
                } else {
                    Ok(single)
                }
            }
            _ => self.auto_end_pos(lp_pos, |p| Ok(p.mk(K::TypeTuple, body.into_iter().map(|a| (F::Args, a)).collect()))),
        }
    }

    /// `maybeAnonymousLambda(t)`: a `Type.AnonymousLambda` of `t` where a placeholder stands in it.
    fn maybe_anonymous_lambda(&mut self, t: u32) -> u32 {
        if !(self.dialect.allow_type_lambdas && self.has_anonymous_param(t, false)) {
            return t;
        }
        let n = self.mk(K::TypeAnonymousLambda, vec![(F::Tpe, t)]);
        self.copy_pos(t, n)
    }

    /// `PlaceholderChecks.hasAnonymousParam(tree, includeArg)`: the tree's own placeholder, breadth first through the
    /// types a placeholder's lambda is made of.
    fn has_anonymous_param(&self, tree: u32, include_arg: bool) -> bool {
        let is_param = |p: &Self, n: u32| p.kind(n) == K::TypeAnonymousParam;
        let mut queue = VecDeque::from([tree]);
        while let Some(t) = queue.pop_front() {
            let found = match self.kind(t) {
                K::TypeAnonymousParam => return t != tree || include_arg,
                K::TypeTuple => self.fields_of(t, F::Args).iter().any(|&a| is_param(self, a)),
                K::TypeArgClause => self.fields_of(t, F::Values).iter().any(|&a| is_param(self, a)),
                K::TypeApply => {
                    queue.extend(self.field(t, F::Tpe));
                    queue.extend(self.field(t, F::ArgClause));
                    false
                }
                K::TypeApplyInfix | K::TypeWith => {
                    queue.extend(self.field(t, F::Lhs));
                    queue.extend(self.field(t, F::Rhs));
                    false
                }
                K::TypeRepeated => {
                    queue.extend(self.field(t, F::Tpe));
                    false
                }
                _ => self.children(t).any(|c| is_param(self, c)),
            };
            if found {
                return true;
            }
        }
        false
    }

    /// `typeClauseInBrackets(part, f)`: the parts in brackets, a clause of `kind` over them.
    pub(super) fn type_clause_in_brackets(&mut self, kind: K, part: impl Fn(&mut Self) -> P<u32>) -> P<u32> {
        self.ctx.type_brackets += 1;
        let out = self.auto_pos(|p| {
            p.accept(C::LeftBracket)?;
            let mut values = vec![(F::Values, part(p)?)];
            while p.accept_opt(C::Comma) {
                values.push((F::Values, part(p)?));
            }
            let n = p.mk(kind, values);
            p.in_brackets_after_open(n)
        });
        self.ctx.type_brackets -= 1;
        out
    }

    /// `typedOpt()`: past `:` a type, `@` an anonymous name's annotations.
    pub(super) fn typed_opt(&mut self) -> P<Option<u32>> {
        if !self.accept_opt(C::Colon) {
            return Ok(None);
        }
        if self.at(C::At) && self.peek(C::Ident) {
            let start = self.curr_index();
            let n = self.mk(K::TypeAnonymousName, Vec::new());
            let n = self.auto_end_pos(start, |_| Ok(n))?;
            return Ok(Some(self.annot_type_rest(n, start)?));
        }
        Ok(Some(self.typ()?))
    }

    /* -------- refinements -------- */

    /// `refinement(innerType)`: an indented refinement after `:` or `with` under `allowSignificantIndentation`, else
    /// the braced ones that follow; `inner` where none does.
    pub(super) fn refinement(&mut self, inner: Option<u32>) -> P<Option<u32>> {
        if !self.dialect.allow_significant_indentation {
            return self.refinement_in_braces(inner, -1);
        }
        match self.curr_c() {
            c @ (C::Colon | C::KwWith) => {
                if !self.try_ahead(C::Indent) {
                    return Ok(inner);
                }
                let stats_pos = if c == C::Colon { self.prev_index() } else { self.curr_index() };
                let t = self.refine_with(inner, stats_pos, |p| {
                    p.accept(C::Indent)?;
                    let stats = p.refine_stat_seq()?;
                    p.indented_after_open(stats)
                })?;
                Ok(Some(t))
            }
            _ => {
                let min_indent = self.previous_indentation() + 1;
                self.refinement_in_braces(inner, min_indent)
            }
        }
    }

    /// `refinementInBraces(innerType, minIndent)`: each braced refinement that follows, but a brace that starts a line
    /// left of `min_indent`.
    fn refinement_in_braces(&mut self, inner: Option<u32>, min_indent: i32) -> P<Option<u32>> {
        let mut inner = inner;
        loop {
            let not_refined = match self.curr_c() {
                C::LeftBrace => min_indent > 0 && self.start_column(self.curr_index()) < min_indent && self.end_line(self.prev_index()) < self.end_line(self.curr_index()),
                c if c.eol() => min_indent > 0 && self.start_column(self.peek_index()) < min_indent || !self.try_ahead(C::LeftBrace),
                _ => true,
            };
            if not_refined {
                return Ok(inner);
            }
            let stats_pos = self.curr_index();
            inner = Some(self.refine_with(inner, stats_pos, |p| p.in_braces_refine_stats())?);
        }
    }

    /// `inBraces(refineStatSeq())`.
    fn in_braces_refine_stats(&mut self) -> P<Vec<u32>> {
        self.new_line_opt();
        if !self.accept_opt(C::LeftBrace) {
            return self.expected_c(C::LeftBrace);
        }
        let stats = self.refine_stat_seq()?;
        self.in_braces_after_open(stats)
    }

    /// `refineWith(innerType, statsPos, stats)`: a `Type.Refine` of `inner` and the statements' block from
    /// `stats_pos`.
    fn refine_with(&mut self, inner: Option<u32>, stats_pos: i32, stats: impl FnOnce(&mut Self) -> P<Vec<u32>>) -> P<u32> {
        let start = match inner {
            Some(t) => self.beg_index(t),
            None => self.curr_index(),
        };
        self.auto_end_pos(start, |p| {
            let block = p.stats_block(stats_pos, stats)?;
            let mut fields: Vec<(F, u32)> = inner.into_iter().map(|t| (F::Tpe, t)).collect();
            fields.push((F::Body, block));
            Ok(p.mk(K::TypeRefine, fields))
        })
    }

    /// `toStatsBlock(startPos)(stats)`.
    fn stats_block(&mut self, start: i32, stats: impl FnOnce(&mut Self) -> P<Vec<u32>>) -> P<u32> {
        let stats = stats(self)?;
        self.auto_end_pos(start, |p| Ok(p.mk(K::StatBlock, stats.into_iter().map(|s| (F::Stats, s)).collect())))
    }

    /// `refineStatSeq()`: a refinement's declarations, each past the separators before it.
    fn refine_stat_seq(&mut self) -> P<Vec<u32>> {
        let mut stats = Vec::new();
        self.skip_all_stat_sep();
        while !self.at_stat_seq_end() {
            if self.s.is_dcl_intro(self.curr_index()) {
                stats.push(self.def_or_dcl_or_secondary_ctor(Vec::new())?);
            } else if self.ctx.return_type > 0 {
                return self.refused("illegal start of declaration (possible cause: missing `=' in front of current method body)".to_string());
            } else {
                return self.refused("illegal start of declaration".to_string());
            }
            if !self.accept_extended_stat_sep_opt()? {
                break;
            }
            self.skip_all_stat_sep();
        }
        Ok(stats)
    }

    /// `existentialTypeOnForSome(t)`: past `forSome` the braced declarations.
    pub(super) fn existential_type_on_for_some(&mut self, t: u32) -> P<u32> {
        self.next();
        let pos = self.curr_index();
        let block = self.stats_block(pos, |p| p.in_braces_refine_stats())?;
        let n = self.mk(K::TypeExistential, vec![(F::Tpe, t), (F::Body, block)]);
        let (start, end) = (self.beg_index(t), self.end_index(block));
        Ok(self.at_pos(start, end, n))
    }

    /// The column the token at the scanner's index `k` starts at (`pos.startColumn`), in UTF-16 units.
    fn start_column(&self, k: i32) -> i32 {
        let start = self.s.tokens[k.max(0) as usize].start as usize;
        let line_start = self.s.text[..start].rfind('\n').map_or(0, |i| i + 1);
        self.s.text[line_start..start].encode_utf16().count() as i32
    }

    /// The line the token at the scanner's index `k` ends on (`pos.endLine`).
    fn end_line(&self, k: i32) -> usize {
        let end = self.s.tokens[k.max(0) as usize].end as usize;
        self.s.text.as_bytes()[..end].iter().filter(|&&b| b == b'\n').count()
    }
}
