//! The parser's token stream (`TokenIterator`, `LazyTokenIterator`, the helpers of ScalametaParser.scala's "token
//! stream" and "parser common" sections): the current, previous and next token of the scanner's chain, a fork a saved
//! place in it.

use super::{Contexts, Parser, Stop, P};
use crate::fix::scanner::{C, TokenRef, L, R};

/// A place in the stream to come back to (`in.fork`) and the contexts as they were; the nodes made since stay (no
/// tree holds them), as the infix stack does.
#[derive(Clone, Copy)]
pub(super) struct Saved {
    i: usize,
    ctx: Contexts,
}

/// scalameta 4.17.4's `Dialect`: the flags its parser and its scanner read (`dialect.allow...`, the soft keywords'
/// own), each as dialects2's `Dialect.scala` names it. The configured dialect is `Scala3`'s (`Scala39`), a script's
/// it `withAllowToplevelTerms(true)` (scalafix's `ScalafixConfig.dialectForFile`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Dialect {
    pub(crate) allow_all_typed_patterns: bool,
    pub(crate) allow_as_for_import_rename: bool,
    pub(crate) allow_at_for_extractor_varargs: bool,
    pub(crate) allow_by_name_repeated_parameters: bool,
    pub(crate) allow_capture_checking: bool,
    pub(crate) allow_colon_for_extractor_varargs: bool,
    pub(crate) allow_comma_separated_extend: bool,
    pub(crate) allow_dependent_function_types: bool,
    pub(crate) allow_derives: bool,
    pub(crate) allow_empty_infix_args: bool,
    pub(crate) allow_end_marker: bool,
    pub(crate) allow_enums: bool,
    pub(crate) allow_erased_defs: bool,
    pub(crate) allow_export_clause: bool,
    pub(crate) allow_extension_methods: bool,
    pub(crate) allow_fewer_braces: bool,
    pub(crate) allow_given_imports: bool,
    pub(crate) allow_given_using: bool,
    pub(crate) allow_improved_type_classes_syntax: bool,
    pub(crate) allow_infix_mods: bool,
    pub(crate) allow_infix_operator_after_nl: bool,
    pub(crate) allow_inline_mods: bool,
    pub(crate) allow_literal_types: bool,
    pub(crate) allow_match_as_operator: bool,
    pub(crate) allow_named_tuples: bool,
    pub(crate) allow_opaque_types: bool,
    pub(crate) allow_open_class: bool,
    pub(crate) allow_param_clause_interleaving: bool,
    pub(crate) allow_parameter_type_conversions: bool,
    pub(crate) allow_plus_minus_underscore_as_ident: bool,
    pub(crate) allow_polymorphic_functions: bool,
    pub(crate) allow_postfix_star_vararg_splices: bool,
    pub(crate) allow_pure_functions: bool,
    pub(crate) allow_question_mark_as_type_wildcard: bool,
    pub(crate) allow_quiet_syntax: bool,
    pub(crate) allow_quoted_type_variables: bool,
    pub(crate) allow_significant_indentation: bool,
    pub(crate) allow_splice_and_quote: bool,
    pub(crate) allow_star_as_type_placeholder: bool,
    pub(crate) allow_star_wildcard_import: bool,
    pub(crate) allow_toplevel_statements: bool,
    pub(crate) allow_toplevel_terms: bool,
    pub(crate) allow_tracked_parameters: bool,
    pub(crate) allow_trailing_commas: bool,
    pub(crate) allow_trait_parameters: bool,
    pub(crate) allow_try_with_any_expr: bool,
    pub(crate) allow_type_lambdas: bool,
    pub(crate) allow_type_match: bool,
    pub(crate) allow_underscore_as_type_placeholder: bool,
    pub(crate) allow_upper_case_pattern_var_binding: bool,
    pub(crate) use_infix_type_precedence: bool,
}

impl Dialect {
    /// `dialects.Scala3`: `Scala39`, `Scala30` and its successors' flags over `Scala213`'s.
    pub(crate) const SCALA3: Dialect = Dialect {
        allow_all_typed_patterns: true,
        allow_as_for_import_rename: true,
        allow_at_for_extractor_varargs: true,
        allow_by_name_repeated_parameters: true,
        allow_capture_checking: false,
        allow_colon_for_extractor_varargs: true,
        allow_comma_separated_extend: true,
        allow_dependent_function_types: true,
        allow_derives: true,
        allow_empty_infix_args: false,
        allow_end_marker: true,
        allow_enums: true,
        allow_erased_defs: true,
        allow_export_clause: true,
        allow_extension_methods: true,
        allow_fewer_braces: true,
        allow_given_imports: false,
        allow_given_using: true,
        allow_improved_type_classes_syntax: true,
        allow_infix_mods: true,
        allow_infix_operator_after_nl: true,
        allow_inline_mods: true,
        allow_literal_types: true,
        allow_match_as_operator: true,
        allow_named_tuples: true,
        allow_opaque_types: true,
        allow_open_class: true,
        allow_param_clause_interleaving: true,
        allow_parameter_type_conversions: true,
        allow_plus_minus_underscore_as_ident: false,
        allow_polymorphic_functions: true,
        allow_postfix_star_vararg_splices: true,
        allow_pure_functions: false,
        allow_question_mark_as_type_wildcard: true,
        allow_quiet_syntax: true,
        allow_quoted_type_variables: true,
        allow_significant_indentation: true,
        allow_splice_and_quote: true,
        allow_star_as_type_placeholder: true,
        allow_star_wildcard_import: true,
        allow_toplevel_statements: true,
        allow_toplevel_terms: false,
        allow_tracked_parameters: false,
        allow_trailing_commas: true,
        allow_trait_parameters: true,
        allow_try_with_any_expr: true,
        allow_type_lambdas: true,
        allow_type_match: true,
        allow_underscore_as_type_placeholder: false,
        allow_upper_case_pattern_var_binding: true,
        use_infix_type_precedence: true,
    };

    /// `dialects.Scala213Source3`, `-Xsource:3`'s over `Scala213`, which the corpus's test reads the samples under.
    #[cfg(test)]
    pub(crate) const SCALA213_SOURCE3: Dialect = Dialect {
        allow_all_typed_patterns: false,
        allow_as_for_import_rename: true,
        allow_at_for_extractor_varargs: true,
        allow_by_name_repeated_parameters: false,
        allow_capture_checking: false,
        allow_colon_for_extractor_varargs: false,
        allow_comma_separated_extend: false,
        allow_dependent_function_types: false,
        allow_derives: false,
        allow_empty_infix_args: false,
        allow_end_marker: false,
        allow_enums: false,
        allow_erased_defs: false,
        allow_export_clause: false,
        allow_extension_methods: false,
        allow_fewer_braces: false,
        allow_given_imports: true,
        allow_given_using: false,
        allow_improved_type_classes_syntax: false,
        allow_infix_mods: true,
        allow_infix_operator_after_nl: true,
        allow_inline_mods: false,
        allow_literal_types: true,
        allow_match_as_operator: false,
        allow_named_tuples: false,
        allow_opaque_types: false,
        allow_open_class: true,
        allow_param_clause_interleaving: false,
        allow_parameter_type_conversions: false,
        allow_plus_minus_underscore_as_ident: true,
        allow_polymorphic_functions: false,
        allow_postfix_star_vararg_splices: true,
        allow_pure_functions: false,
        allow_question_mark_as_type_wildcard: true,
        allow_quiet_syntax: false,
        allow_quoted_type_variables: false,
        allow_significant_indentation: false,
        allow_splice_and_quote: false,
        allow_star_as_type_placeholder: false,
        allow_star_wildcard_import: true,
        allow_toplevel_statements: false,
        allow_toplevel_terms: false,
        allow_tracked_parameters: false,
        allow_trailing_commas: true,
        allow_trait_parameters: false,
        allow_try_with_any_expr: true,
        allow_type_lambdas: false,
        allow_type_match: false,
        allow_underscore_as_type_placeholder: false,
        allow_upper_case_pattern_var_binding: false,
        use_infix_type_precedence: false,
    };

    /// `withAllowToplevelTerms(true)`.
    pub(crate) fn with_allow_toplevel_terms(self) -> Dialect {
        Dialect { allow_toplevel_terms: true, ..self }
    }
}

impl<'a> Parser<'a> {
    fn r(&self, k: usize) -> &TokenRef {
        &self.s.refs[self.order[k.min(self.order.len() - 1)]]
    }

    pub(super) fn curr_c(&self) -> C {
        self.r(self.i).token.c
    }
    pub(super) fn peek_c(&self) -> C {
        self.r(self.i + 1).token.c
    }
    pub(super) fn prev_c(&self) -> Option<C> {
        (self.i > 0).then(|| self.r(self.i - 1).token.c)
    }
    /// `currIndex`: the current token's place among the scanner's tokens (`pointPos`).
    pub(super) fn curr_index(&self) -> i32 {
        self.r(self.i).point_pos
    }
    pub(super) fn prev_index(&self) -> i32 {
        if self.i == 0 { -1 } else { self.r(self.i - 1).point_pos }
    }
    pub(super) fn peek_index(&self) -> i32 {
        self.r(self.i + 1).point_pos
    }
    /// The regions the current token stands in (`currRegions`).
    pub(super) fn curr_regions(&self) -> L {
        self.r(self.i).regions
    }
    fn text_at(&self, k: usize) -> &'a str {
        match self.r(k).token.real {
            Some(t) => self.s.text_of(t as i32),
            None => "",
        }
    }
    /// The current token's text, a real token's (none for one the scanner makes).
    pub(super) fn curr_text(&self) -> &'a str {
        self.text_at(self.i)
    }
    pub(super) fn peek_text(&self) -> &'a str {
        self.text_at(self.i + 1)
    }
    /// The current token's start, where a refusal points.
    pub(super) fn curr_offset(&self) -> u32 {
        self.s.tokens[self.curr_index().max(0) as usize].start
    }

    pub(super) fn next(&mut self) {
        if self.i + 1 < self.order.len() {
            self.i += 1;
        }
    }
    pub(super) fn next_twice(&mut self) {
        self.next();
        self.next();
    }
    pub(super) fn next_if(&mut self, cond: bool) -> bool {
        if cond {
            self.next();
        }
        cond
    }

    pub(super) fn save(&self) -> Saved {
        Saved { i: self.i, ctx: self.ctx }
    }
    pub(super) fn restore(&mut self, s: Saved) {
        self.i = s.i;
        self.ctx = s.ctx;
    }
    /// The current token's `TokenRef` among the scanner's.
    pub(super) fn curr_ref(&self) -> usize {
        self.order[self.i.min(self.order.len() - 1)]
    }
    /// `ahead`: `body` after the next token, the stream and the nodes then as they were.
    pub(super) fn ahead<T>(&mut self, body: impl FnOnce(&mut Self) -> T) -> T {
        let saved = self.save();
        self.next();
        let out = body(self);
        self.restore(saved);
        out
    }
    /// `tryParse`: `body`, the stream as it was where it gives none.
    pub(super) fn try_parse<T>(&mut self, body: impl FnOnce(&mut Self) -> P<Option<T>>) -> P<Option<T>> {
        let saved = self.save();
        let out = body(self)?;
        if out.is_none() {
            self.restore(saved);
        }
        Ok(out)
    }
    /// `tryAhead(cond)`: past the next token where `cond` holds there.
    pub(super) fn try_ahead_if(&mut self, cond: impl FnOnce(&mut Self) -> P<bool>) -> P<bool> {
        let saved = self.save();
        self.next();
        let ok = cond(self)?;
        if !ok {
            self.restore(saved);
        }
        Ok(ok)
    }
    /// `tryAhead[T]`: past the current token where the next is `c`.
    pub(super) fn try_ahead(&mut self, c: C) -> bool {
        let ok = self.peek_c() == c;
        self.next_if(ok)
    }

    pub(super) fn at(&self, c: C) -> bool {
        self.curr_c() == c
    }
    pub(super) fn peek(&self, c: C) -> bool {
        self.peek_c() == c
    }
    pub(super) fn prev(&self, c: C) -> bool {
        self.prev_c() == Some(c)
    }
    /// An identifier written `name`.
    pub(super) fn at_ident(&self, name: &str) -> bool {
        self.at(C::Ident) && self.curr_text() == name
    }
    /// A soft keyword (`soft.Kw...`, `IsWithName`): an identifier written `name` under its dialect's flag.
    pub(super) fn at_soft(&self, enabled: bool, name: &str) -> bool {
        enabled && self.at_ident(name)
    }
    pub(super) fn peek_soft(&self, enabled: bool, name: &str) -> bool {
        enabled && self.peek(C::Ident) && self.peek_text() == name
    }
    /// `isIndentingOrEOL(nonOptBracesOK)`.
    pub(super) fn is_indenting_or_eol(&self, non_opt_braces_ok: bool) -> bool {
        if self.dialect.allow_significant_indentation { self.indenting() } else { non_opt_braces_ok && self.curr_c().eol() }
    }
    pub(super) fn at_eol(&self) -> bool {
        self.curr_c().at_eol()
    }
    /// `StatSep`: a `;` or a line break.
    pub(super) fn at_stat_sep(&self) -> bool {
        matches!(self.curr_c(), C::Semicolon) || self.curr_c().at_eol()
    }
    /// `StatSeqEnd`: `}`, the end of the file or an `Outdent`.
    pub(super) fn at_stat_seq_end(&self) -> bool {
        matches!(self.curr_c(), C::RightBrace | C::Eof | C::Outdent)
    }

    pub(super) fn refused<T>(&self, message: String) -> P<T> {
        Err(Stop::Refused(self.curr_offset(), message))
    }
    /// `expect[T]`.
    pub(super) fn expect(&self, c: C) -> P<()> {
        if self.at(c) { Ok(()) } else { self.expected_c(c) }
    }
    /// `accept[T]`: past the current token, which is `c`.
    pub(super) fn accept(&mut self, c: C) -> P<()> {
        self.expect(c)?;
        if !self.at(C::Eof) {
            self.next();
        }
        Ok(())
    }
    pub(super) fn accept_opt(&mut self, c: C) -> bool {
        let ok = self.at(c);
        self.next_if(ok)
    }
    /// `acceptIfAfterOpt(a, b)`: past an `a`, or a `b` and the `a` after it.
    pub(super) fn accept_if_after_opt(&mut self, a: impl Fn(C) -> bool, b: impl Fn(C) -> bool) -> bool {
        if a(self.curr_c()) {
            self.next();
            true
        } else if b(self.curr_c()) && a(self.peek_c()) {
            self.next_twice();
            true
        } else {
            false
        }
    }
    /// `acceptIfAfterOptNL[T]`.
    pub(super) fn accept_if_after_opt_nl(&mut self, c: C) -> bool {
        self.accept_if_after_opt(|x| x == c, C::at_eol)
    }
    /// `acceptAfterOptNL[T]`.
    pub(super) fn accept_after_opt_nl(&mut self, c: C) -> P<()> {
        if self.at_eol() {
            self.next();
        }
        self.accept(c)
    }
    /// `isAfterOptNewLine[T]`: at `c`, past a line break (`EOL`) before it.
    pub(super) fn is_after_opt_new_line(&mut self, c: C) -> bool {
        if self.curr_c().eol() { self.try_ahead(c) } else { self.at(c) }
    }
    pub(super) fn new_line_opt(&mut self) {
        if self.curr_c().eol() {
            self.next();
        }
    }
    pub(super) fn new_lines_opt(&mut self) {
        if self.at_eol() {
            self.next();
        }
    }
    /// `in.indenting`: at a line break in a line's region whose enclosing region, where there is one, indents less.
    pub(super) fn indenting(&self) -> bool {
        let rs = self.curr_regions();
        match self.s.head(rs) {
            Some(R::Line(indent)) if self.at_eol() => self.s.second(rs).map_or(true, |r| r.indent() >= 0 && r.indent() < indent),
            _ => false,
        }
    }

    /// `in.previousIndentation`.
    pub(super) fn previous_indentation(&self) -> i32 {
        let rs = self.curr_regions();
        match (self.s.head(rs), self.s.second(rs)) {
            (Some(R::Line(indent)), _) if !self.at_eol() => indent,
            (Some(_), Some(r)) => if matches!(r, R::Paren(_)) { r.indent() - 1 } else { r.indent() },
            _ => 0,
        }
    }

    /// `isCaseIntroOnKwCase()`: `case` not of a class or an object.
    pub(super) fn is_case_intro_on_kw_case(&self) -> bool {
        !self.peek_c().is_class_or_object()
    }

    /// `isAtEndMarker`.
    pub(super) fn is_at_end_marker(&self) -> bool {
        self.r(self.i).token.real.is_some_and(|t| self.s.is_end_marker_intro(t as i32))
    }
    pub(super) fn accept_if_stat_sep(&mut self) -> bool {
        let ok = self.at_stat_sep();
        self.next_if(ok)
    }
    /// `isImplicitStatSep`: after an `Outdent`.
    pub(super) fn is_implicit_stat_sep(&self) -> bool {
        self.prev(C::Outdent)
    }
    pub(super) fn accept_if_extended_stat_sep(&mut self) -> bool {
        self.accept_if_stat_sep() || self.is_at_end_marker() || self.is_implicit_stat_sep() && !self.at(C::Outdent)
    }
    pub(super) fn accept_extended_stat_sep(&mut self) -> P<()> {
        if self.accept_if_extended_stat_sep() { Ok(()) } else { self.expected_c(C::Semicolon) }
    }
    pub(super) fn accept_extended_stat_sep_opt(&mut self) -> P<bool> {
        let ok = !self.at_stat_seq_end();
        if ok {
            self.accept_extended_stat_sep()?;
        }
        Ok(ok)
    }
    pub(super) fn skip_all_stat_sep(&mut self) -> bool {
        let ok = self.accept_if_stat_sep();
        while self.accept_if_stat_sep() {}
        ok
    }

    /// `inParensOnOpen`'s and its kin's close: `)` past a line break.
    pub(super) fn in_parens_after_open<T>(&mut self, body: T) -> P<T> {
        self.accept_after_opt_nl(C::RightParen)?;
        Ok(body)
    }
    pub(super) fn in_braces_after_open<T>(&mut self, body: T) -> P<T> {
        self.accept_after_opt_nl(C::RightBrace)?;
        Ok(body)
    }
    pub(super) fn indented_after_open<T>(&mut self, body: T) -> P<T> {
        self.accept_after_opt_nl(C::Outdent)?;
        Ok(body)
    }
    pub(super) fn in_brackets_after_open<T>(&mut self, body: T) -> P<T> {
        self.accept(C::RightBracket)?;
        Ok(body)
    }

    /// The token at the scanner's index `k`.
    pub(super) fn tok_c(&self, k: i32) -> C {
        self.s.c(k)
    }
}

/// `getTokenName[T]`: a token class's name in "expected" (`syntaxExpectedMessage`).
pub(super) fn token_class_name(c: C) -> String {
    let fixed = match c {
        C::Semicolon => ";",
        C::Hash => "#",
        C::Colon => ":",
        C::Viewbound => "<%",
        C::LeftArrow => "<-",
        C::Subtype => "<:",
        C::Equals => "=",
        C::RightArrow => "=>",
        C::Supertype => ">:",
        C::At => "@",
        C::Underscore => "_",
        C::TypeLambdaArrow => "=>>",
        C::ContextArrow => "?=>",
        C::MacroQuote => "'",
        C::LeftParen => "(",
        C::RightParen => ")",
        C::Comma => ",",
        C::Dot => ".",
        C::LeftBracket => "[",
        C::RightBracket => "]",
        C::LeftBrace => "{",
        C::RightBrace => "}",
        C::Ident => "identifier",
        C::Eof => "end of file",
        C::Bof => "beginning of file",
        C::Indent => "indent",
        C::Outdent => "outdent",
        C::InterpId => "Interpolation.Id",
        C::InterpStart => "Interpolation.Start",
        C::InterpPart => "Interpolation.Part",
        C::InterpSpliceStart => "Interpolation.SpliceStart",
        C::InterpSpliceEnd => "Interpolation.SpliceEnd",
        C::InterpEnd => "Interpolation.End",
        _ => "",
    };
    if !fixed.is_empty() {
        return fixed.to_string();
    }
    let name = format!("{:?}", c);
    match name.strip_prefix("Kw") {
        Some(k) => k.to_lowercase(),
        None => name,
    }
}

/// `Token.name`: a token's fixed text, or its class's description, escaped as the token macro escapes it
/// (`Chars.escape`).
pub(super) fn found_name(c: C, text: &str) -> String {
    let provided = match c {
        C::Ident => "identifier".to_string(),
        C::Eof => "end of file".to_string(),
        C::Bof => "beginning of file".to_string(),
        C::Indent => "indent".to_string(),
        C::Outdent => "outdent".to_string(),
        C::Lf | C::InfixLf => "\n".to_string(),
        C::LfLf => "\n\n".to_string(),
        C::Cr => "\r".to_string(),
        C::Crlf => "\r\n".to_string(),
        C::Ff => "\u{c}".to_string(),
        C::Comment => "comment".to_string(),
        C::InterpId => "interpolation id".to_string(),
        C::InterpStart => "interpolation start".to_string(),
        C::InterpPart => "interpolation part".to_string(),
        C::InterpSpliceStart => "splice start".to_string(),
        C::InterpSpliceEnd => "splice end".to_string(),
        C::InterpEnd => "interpolation end".to_string(),
        C::Constant => match super::names::constant_value(text).map(|v| v.0) {
            Some(crate::fix::nodes::K::LitLong) => "long constant",
            Some(crate::fix::nodes::K::LitFloat) => "float constant",
            Some(crate::fix::nodes::K::LitDouble) => "double constant",
            Some(crate::fix::nodes::K::LitChar) => "character constant",
            Some(crate::fix::nodes::K::LitString) => "string constant",
            _ => "integer constant",
        }
        .to_string(),
        _ => text.to_string(),
    };
    super::chars::escape(&provided)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The line breaks' names as scalameta's token macro gives them, `Chars.escape` of their texts.
    #[test]
    fn found_names_of_line_breaks_escaped() {
        let names: Vec<String> = [C::Lf, C::InfixLf, C::LfLf, C::Cr, C::Crlf, C::Ff].iter().map(|&c| found_name(c, "")).collect();
        assert_eq!(names, ["\\n", "\\n", "\\n\\n", "\\r", "\\r\\n", "\\f"]);
    }
}
