mod defs;
pub mod dump;
mod expr;
mod pat;
mod types;

use crate::ast::*;
use crate::intern::{Interner, Name};
use crate::lexer::LexResult;
use crate::names;
use crate::source::Span;
use crate::token::{Tok, Token};

/// Syntax that a scalac option turns on.
#[derive(Clone, Copy, Default)]
pub struct Syntax {
    /// `-Xkind-projector`: a `*` among the arguments of a type application stands for a
    /// parameter of a type lambda around it (`Either[String, *]`).
    pub kind_projector: bool,
    /// The language server's index is kept: the parser records the name spans of selections,
    /// operators and import paths (`Ast::name_spans`).
    pub index: bool,
}

pub struct Parser<'a> {
    toks: &'a [Token],
    syntax: Syntax,
    pos: usize,
    text: &'a str,
    end_markers: &'a [Span],
    pub ast: Ast,
    interner: &'a Interner,
    pub errors: Vec<(Span, String)>,
    placeholders: Vec<LambdaParam>,
    placeholder_names: Vec<Name>,
    fresh_counter: u32,
    ext_groups: u32,
    expr_scratch: Vec<ExprId>,
    pat_scratch: Vec<PatId>,
    ty_scratch: Vec<TyExprId>,
    export_scratch: Vec<Import>,
    /// The `scala.language` imports of the clause parsed last.
    language_scratch: Vec<Import>,
    /// Classes with a `derives` clause whose enclosing body is still being parsed.
    pending_derives: Vec<PendingDerives>,
    /// The `self =>` alias of the template body being parsed, EMPTY without one; a nested body
    /// hides it, as `Outer.this` is not in the subset either.
    /// One AST per `package p:` block, each read by the typer as a file of its own in that
    /// package, with the imports in effect where the block starts.
    package_blocks: Vec<Ast>,
    /// Set while a parent or the type after `new` is parsed, where braces start a class body.
    template_type: bool,
    /// How many quotes enclose the expression being parsed, less the splices inside them; an
    /// identifier `$x` is a splice where this is positive.
    quote_level: u32,
    /// Set while the body of a quote pattern is parsed, where a splice holds a pattern.
    quote_pattern: bool,
    /// The lexer's boundaries (`LexResult::boundaries`) and the first of them a syntax error
    /// was found at, where the file is lexed again with the delimiters around it closed.
    boundaries: &'a [u32],
    pub boundary: Option<u32>,
    /// The boundaries the delimiters were closed at, where the lexer's layout stands in front
    /// of a line's first token that scalac's scanner reads with no layout before it.
    closed: &'a [u32],
    /// How many syntax errors were found, the ones not reported among them: whether a
    /// definition's header is complete is read from it.
    error_events: u32,
    /// Set from a recovery of a statement sequence until the sequence passes a separator: the
    /// errors found meanwhile are not reported, as scalac's skip moves its `lastErrorOffset`.
    quiet: bool,
    /// Set by `parse_args` when the list's recovery skipped tokens or stopped short of its `)`.
    args_cut: bool,
    /// The token at which a header's skip found the template body, a brace block after a token
    /// that left the lexer's `ColonEol` out (`Parser::skip_header_to`).
    body_at: usize,
    /// The recovery last recorded and the token after it, which a skip right there extends.
    last_skip: Option<(usize, usize)>,
    /// The outdents of the regions the lexer opened after a type annotation's `:` at the end of
    /// a line, innermost last: scalac opens none there, so each is dropped where it comes
    /// (`Parser::eat_annotation_colon`).
    dissolved: Vec<usize>,
}

struct PendingDerives {
    class: Name,
    span: Span,
    tparams: Vec<TypeParam>,
    derives: Vec<(TyExprId, Name)>,
    /// `case object O derives TC`: the given is `TC[O.type]`, in the object itself.
    is_object: bool,
}

/// What a file parses to: the file's AST first, then one per package block in source order;
/// the syntax errors; the boundary of the lexer's a syntax error was found at, if any.
pub struct Parsed {
    pub asts: Vec<Ast>,
    pub errors: Vec<(Span, String)>,
    pub boundary: Option<u32>,
}

pub fn parse(text: &str, lexed: &LexResult, closed: &[u32], interner: &Interner, syntax: Syntax) -> Parsed {
    let mut p = Parser {
        toks: &lexed.tokens,
        syntax,
        pos: 0,
        text,
        end_markers: &lexed.end_markers,
        ast: Ast::new(text.len()),
        interner,
        errors: Vec::new(),
        placeholders: Vec::new(),
        placeholder_names: Vec::new(),
        fresh_counter: 0,
        ext_groups: 0,
        expr_scratch: Vec::new(),
        pat_scratch: Vec::new(),
        ty_scratch: Vec::new(),
        export_scratch: Vec::new(),
        language_scratch: Vec::new(),
        pending_derives: Vec::new(),
        package_blocks: Vec::new(),
        template_type: false,
        quote_level: 0,
        quote_pattern: false,
        boundaries: &lexed.boundaries,
        boundary: None,
        closed,
        error_events: 0,
        quiet: false,
        args_cut: false,
        body_at: usize::MAX,
        last_skip: None,
        dissolved: Vec::new(),
    };
    p.parse_file();
    let mut asts = vec![p.ast];
    asts.extend(p.package_blocks);
    Parsed { asts, errors: p.errors, boundary: p.boundary }
}

impl<'a> Parser<'a> {
    #[inline]
    fn tok(&self) -> Token {
        self.toks[self.pos]
    }
    #[inline]
    fn kind(&self) -> Tok {
        self.toks[self.pos].kind
    }
    #[inline]
    fn kind_at(&self, n: usize) -> Tok {
        self.toks.get(self.pos + n).map_or(Tok::Eof, |t| t.kind)
    }
    #[inline]
    fn span(&self) -> Span {
        self.toks[self.pos].span
    }
    /// Span of the last token that is not a layout token, so that node spans end with their text.
    #[inline]
    /// The span of the last token consumed, past the layout tokens: a closing brace, which the
    /// lexer hands over as an `Outdent` with the brace's own span, ends what it closes.
    fn prev_span(&self) -> Span {
        let mut i = self.pos.saturating_sub(1);
        while i > 0 {
            let t = &self.toks[i];
            let layout = match t.kind {
                Tok::Indent | Tok::Newline => true,
                Tok::Outdent => t.span.start == t.span.end,
                _ => false,
            };
            if !layout {
                break;
            }
            i -= 1;
        }
        self.toks[i].span
    }
    #[inline]
    fn at(&self, k: Tok) -> bool {
        self.kind() == k
    }
    #[inline]
    fn bump(&mut self) -> Token {
        let t = self.toks[self.pos];
        if t.kind != Tok::Eof {
            self.pos += 1;
        }
        t
    }
    #[inline]
    fn eat(&mut self, k: Tok) -> bool {
        if self.at(k) {
            self.bump();
            true
        } else {
            false
        }
    }

    /// dotc's `pastBlankLine`: whether a line of white space alone lies between the token before
    /// the one at `n` and that token. A comment makes its line count as text.
    fn blank_line_before(&self, n: usize) -> bool {
        let i = self.pos + n;
        let (Some(prev), Some(tok)) = (i.checked_sub(1).and_then(|j| self.toks.get(j)), self.toks.get(i)) else {
            return false;
        };
        let mut blank = false;
        for &b in &self.text.as_bytes()[prev.span.end as usize..tok.span.start as usize] {
            if matches!(b, b'\n' | 0x0c) {
                if blank {
                    return true;
                }
                blank = true;
            } else {
                blank = blank && b <= b' ';
            }
        }
        false
    }

    fn error_at(&mut self, span: Span, msg: impl Into<String>) {
        self.error_events += 1;
        if self.boundary.is_none() && self.boundaries.binary_search(&span.start).is_ok() {
            self.boundary = Some(span.start);
        }
        // A single syntax error tends to cascade; only the first one per position is useful.
        if !self.quiet && self.errors.last().map_or(true, |(s, _)| s.start != span.start) {
            self.errors.push((span, msg.into()));
        }
    }

    /// What a message calls the current token. At a boundary the delimiters were closed at, the
    /// layout the lexer put in front of the line's first token is passed over: scalac's scanner,
    /// still inside the delimiters there, reads the token itself.
    pub(super) fn found(&self) -> (Span, &'static str) {
        let mut i = self.pos;
        let at = self.toks[i].span.start;
        if self.closed.binary_search(&at).is_ok() {
            while matches!(self.toks[i].kind, Tok::Newline | Tok::Indent | Tok::Outdent) && self.toks[i].span.end == at && i + 1 < self.toks.len() {
                i += 1;
            }
        }
        let t = self.toks[i];
        let braced = t.span.end > t.span.start;
        let name = match t.kind {
            Tok::Outdent if braced => "'}'",
            Tok::Indent if braced => "'{'",
            Tok::ColonEol if self.text.as_bytes()[t.span.start as usize] == b'{' => "'{'",
            k => k.describe(),
        };
        (t.span, name)
    }

    /// Whether a definition's header may end at the current token: the statement ends, or a body
    /// or an initializer follows.
    #[inline]
    pub(super) fn at_header_end(&self) -> bool {
        matches!(self.kind(), Tok::Newline | Tok::Semi | Tok::Outdent | Tok::Eof | Tok::Eq)
            || (self.at(Tok::ColonEol) && self.kind_at(1) == Tok::Indent)
    }

    /// At a token of a broken header on the header's own line, the tokens up to the first one
    /// `stop` accepts at the header's level are skipped, if one comes before the line ends, a
    /// closer of an enclosing list, a definition keyword or modifier, or the end of the file at
    /// any depth.
    pub(super) fn skip_header_to(&mut self, stop: impl Fn(&Self, usize) -> bool) -> bool {
        let mut depth = 0u32;
        let mut i = self.pos;
        loop {
            let k = self.toks[i].kind;
            if depth == 0 && stop(self, i) {
                break;
            }
            match k {
                Tok::Eof => return false,
                Tok::Newline | Tok::Semi | Tok::Outdent if depth == 0 => return false,
                Tok::LParen | Tok::LBracket | Tok::Indent => depth += 1,
                Tok::RParen | Tok::RBracket | Tok::Outdent => {
                    if depth == 0 {
                        return false;
                    }
                    depth -= 1;
                }
                _ if depth == 0 && starts_definition(k) => return false,
                _ => {}
            }
            i += 1;
        }
        let from = self.pos;
        self.pos = i;
        self.note_skipped(RecoverySite::Header, from);
        if self.at(Tok::Indent) {
            self.body_at = i;
        }
        true
    }

    /// What is left of a broken header's line: the tokens up to the end of the line at the
    /// header's level, an indented block, a closer of an enclosing list, a definition keyword or
    /// modifier, or the end of the file at any depth.
    pub(super) fn skip_header_line(&mut self) {
        let mut depth = 0u32;
        let mut i = self.pos;
        loop {
            let k = self.toks[i].kind;
            match k {
                Tok::Eof => break,
                Tok::Newline | Tok::Semi | Tok::Indent | Tok::Outdent if depth == 0 => break,
                Tok::LParen | Tok::LBracket | Tok::Indent => depth += 1,
                Tok::RParen | Tok::RBracket | Tok::Outdent => {
                    if depth == 0 {
                        break;
                    }
                    depth -= 1;
                }
                _ if depth == 0 && starts_definition(k) => break,
                _ => {}
            }
            i += 1;
        }
        let from = self.pos;
        self.pos = i;
        self.note_skipped(RecoverySite::Header, from);
    }

    /// The statements of a sequence up to the end of its region, an `Outdent` or the end of the
    /// file, which is left to the caller (IntelliJ's `parseRuleInBlockOrIndentationRegion`).
    /// After a statement comes a separator, or the region's end, or the missing separator is
    /// reported without advancing and the next statement starts at that token. A statement that
    /// consumed nothing has reported its token, which is skipped alone: a brace block with all
    /// it holds, so that the braces stay balanced, and an indented block's own layout, its
    /// statements becoming this sequence's.
    #[inline]
    fn statements(&mut self, site: RecoverySite, mut stmt: impl FnMut(&mut Self)) {
        let mut entered = 0u32;
        loop {
            self.skip_separators();
            match self.kind() {
                Tok::Eof => return,
                Tok::Outdent if self.take_dissolved_outdent() => continue,
                Tok::Outdent if entered > 0 => {
                    entered -= 1;
                    self.skip_token(site);
                    continue;
                }
                Tok::Outdent => return,
                _ => {}
            }
            let before = self.pos;
            stmt(self);
            if self.pos != before {
                self.statement_end();
            } else if self.at(Tok::Indent) && self.span().start == self.span().end {
                entered += 1;
                self.skip_token(site);
            } else {
                self.skip_one(site);
            }
        }
    }

    /// The `:` of a type annotation, or a `:` that ends its line before the type on the next,
    /// indented one (`val v:` then `Int = 1`), where scalac's scanner opens no region: the
    /// lexer's is dissolved, its outdent taken where the definition ends at it
    /// (`end_annotation_region`) or else by the statement sequence that reaches it, the
    /// statements after the definition in the region being that sequence's. Some(the dissolved
    /// region's outdent, if one was), None without a colon.
    fn eat_annotation_colon(&mut self) -> Option<Option<usize>> {
        if self.eat(Tok::Colon) {
            return Some(None);
        }
        if !self.written_colon_at(0) {
            return None;
        }
        // The type on a line indented no further: scalac reads it as the line's continuation.
        if !matches!(self.kind_at(1), Tok::Indent | Tok::Outdent | Tok::Newline | Tok::Semi | Tok::Eof) {
            self.bump();
            return Some(None);
        }
        if self.kind_at(1) != Tok::Indent {
            return None;
        }
        self.bump();
        let outdent = self.matching_outdent(self.pos);
        self.bump();
        self.dissolve(outdent);
        Some(Some(outdent))
    }

    /// Records the outdent of a dissolved region, the innermost, which comes first, last.
    fn dissolve(&mut self, outdent: usize) {
        let at = self.dissolved.partition_point(|&i| i > outdent);
        self.dissolved.insert(at, outdent);
    }

    /// A `:` written at the end of its line with an indented line after it, as against the
    /// `ColonEol` the lexer puts before a brace block.
    fn at_colon_before_indent(&self) -> bool {
        self.written_colon_at(0) && self.kind_at(1) == Tok::Indent
    }

    /// Whether the token `n` ahead is a `:`, at the end of its line or not.
    fn written_colon_at(&self, n: usize) -> bool {
        match self.kind_at(n) {
            Tok::Colon => true,
            Tok::ColonEol => self.text.as_bytes()[self.toks[self.pos + n].span.start as usize] == b':',
            _ => false,
        }
    }

    /// The `Outdent` that closes the region the `Indent` at `i` opens.
    fn matching_outdent(&self, i: usize) -> usize {
        let mut depth = 0u32;
        for (j, t) in self.toks.iter().enumerate().skip(i) {
            match t.kind {
                Tok::Indent => depth += 1,
                Tok::Outdent => {
                    depth -= 1;
                    if depth == 0 {
                        return j;
                    }
                }
                Tok::Eof => return j,
                _ => {}
            }
        }
        self.toks.len() - 1
    }

    /// After the type of an annotation that dissolved a region holding the type alone: the
    /// region's end where an `=` follows it (`val x:` / `Int` / `= 1`), which scalac reads as the
    /// definition's continuation.
    fn annotation_before_eq(&mut self, region: Option<usize>) {
        if region == Some(self.pos) && self.kind_at(1) == Tok::Eq {
            self.take_dissolved_outdent();
        }
    }

    /// A definition's right-hand side. After an annotation that dissolved a region, an `=` that
    /// ends its line before a line indented as far as the type (`def f:` / `Unit =` /
    /// `println("f")`) has the region scalac opens there, which holds what is left of the
    /// dissolved one.
    fn parse_rhs(&mut self, region: Option<usize>) -> ExprId {
        match self.rhs_fills_region(region) {
            Some(outdent) => self.parse_rest_of_region(outdent),
            None => self.parse_block_or_expr(),
        }
    }

    /// A parameter's default, as `parse_rhs` reads a right-hand side.
    fn parse_default(&mut self, region: Option<usize>) -> ExprId {
        match self.rhs_fills_region(region) {
            Some(outdent) => self.parse_rest_of_region(outdent),
            None => self.parse_expr(),
        }
    }

    fn rhs_fills_region(&self, region: Option<usize>) -> Option<usize> {
        // A brace block may start what is left of the region (`{ println("f") }` then more
        // lines); a layout `Indent` there is the region scalac opens itself.
        let layout_indent = self.at(Tok::Indent) && self.span().start == self.span().end;
        region.filter(|&outdent| self.pos < outdent && !layout_indent && self.line_starts_here())
    }

    /// What is left of a dissolved region, as the block of a right-hand side, or its cases.
    fn parse_rest_of_region(&mut self, outdent: usize) -> ExprId {
        self.dissolved.retain(|&i| i != outdent);
        if self.at(Tok::KwCase) && !matches!(self.kind_at(1), Tok::KwClass | Tok::KwObject) {
            return self.parse_case_lambda_in_region();
        }
        let start = self.span();
        self.parse_block_rest(start)
    }

    /// After a definition whose annotation dissolved a region: its outdent, where the definition
    /// ends at it.
    fn end_annotation_region(&mut self, region: Option<usize>) {
        if region == Some(self.pos) {
            self.take_dissolved_outdent();
        }
    }

    /// Takes the `Outdent` at the current token if it closes a dissolved region.
    fn take_dissolved_outdent(&mut self) -> bool {
        while self.dissolved.last().is_some_and(|&i| i < self.pos) {
            self.dissolved.pop();
        }
        if self.dissolved.last() != Some(&self.pos) {
            return false;
        }
        self.dissolved.pop();
        self.bump();
        true
    }

    /// The `Outdent` that ends a statement sequence's region, after which errors are reported
    /// again.
    fn end_region(&mut self) {
        self.eat(Tok::Outdent);
        self.quiet = false;
    }

    /// After a statement of a sequence: a separator or the end of the region, or a missing
    /// separator reported without advancing, the next statement starting at the token.
    #[inline]
    fn statement_end(&mut self) {
        if !matches!(self.kind(), Tok::Newline | Tok::Semi | Tok::Outdent | Tok::Eof) {
            self.separator_missing();
        }
    }

    #[cold]
    #[inline(never)]
    fn separator_missing(&mut self) {
        let (at, found) = self.found();
        self.error_at(at, format!("expected end of statement, found {}", found));
        self.quiet = true;
    }

    /// Skips the token no statement of a sequence starts with, a brace block with all it holds
    /// so that the braces stay balanced.
    fn skip_one(&mut self, site: RecoverySite) {
        let from = self.pos;
        if self.at(Tok::Indent) {
            let mut depth = 0u32;
            loop {
                match self.kind() {
                    Tok::Eof => break,
                    Tok::Indent => depth += 1,
                    Tok::Outdent => {
                        depth -= 1;
                        if depth == 0 {
                            self.bump();
                            break;
                        }
                    }
                    _ => {}
                }
                self.bump();
            }
        } else {
            self.bump();
        }
        self.note_skipped(site, from);
        self.quiet = true;
    }

    fn expect(&mut self, k: Tok) -> bool {
        if self.eat(k) {
            return true;
        }
        let (at, found) = self.found();
        self.error_at(at, format!("expected {}, found {}", k.describe(), found));
        false
    }

    /// After an element of a list that `close` ends: whether another element follows a comma.
    /// At a token that neither separates elements nor closes the list, the missing closer is
    /// reported, and the tokens up to the next comma at the list's level or up to the list's
    /// own closer are skipped where one follows; without one the list ends there with nothing
    /// consumed.
    #[inline]
    pub(super) fn list_continues(&mut self, close: Tok, cut: &mut bool) -> bool {
        if self.eat(Tok::Comma) {
            return true;
        }
        if self.at(close) || self.at(Tok::Eof) {
            return false;
        }
        *cut = true;
        self.list_recovers(close)
    }

    #[cold]
    #[inline(never)]
    fn list_recovers(&mut self, close: Tok) -> bool {
        // scalac's `enclosedWithCommas`: the comma is named where the token could have followed one.
        let (at, found) = self.found();
        let comma = if self.expr_start_at(0) { "',' or " } else { "" };
        self.error_at(at, format!("expected {}{}, found {}", comma, close.describe(), found));
        let (n, comma) = match self.list_resume(close) {
            Ok(resume) => resume,
            // A line of the owning region inside the list, which the lexer's layout leaves open
            // there: the list is closed before it when the file is lexed again.
            Err(Some(line)) => {
                if self.boundary.is_none() && self.boundaries.binary_search(&line).is_ok() {
                    self.boundary = Some(line);
                }
                return false;
            }
            Err(None) => return false,
        };
        let from = self.pos;
        for _ in 0..n {
            self.bump();
        }
        self.note_skipped(RecoverySite::List, from);
        comma && self.eat(Tok::Comma)
    }

    /// How many tokens lie before the next comma at the current level or the closer `close` of
    /// the list, and whether a comma comes first; without either, the first token of the line
    /// indented no further than the line of the list's opener that came first, if one did (and
    /// not another closer or the end of the file).
    fn list_resume(&self, close: Tok) -> Result<(usize, bool), Option<u32>> {
        let bound = self.list_opener(close).map(|o| self.line_indent(self.toks[o].span.start));
        let mut depth = 0u32;
        let mut i = self.pos;
        let mut prev_end = self.toks[..i].iter().rev().find(|t| t.span.start < t.span.end).map_or(0, |t| t.span.end);
        loop {
            let t = self.toks[i];
            let k = t.kind;
            if t.span.start < t.span.end {
                let new_line = self.text.as_bytes()[prev_end.min(t.span.start) as usize..t.span.start as usize].contains(&b'\n');
                if new_line && k != close && bound.is_some_and(|b| self.line_indent(t.span.start) <= b) {
                    return Err(Some(t.span.start));
                }
                prev_end = t.span.end;
            }
            match k {
                Tok::Eof => return Err(None),
                Tok::Comma if depth == 0 => return Ok((i - self.pos, true)),
                _ if k == close && depth == 0 => return Ok((i - self.pos, false)),
                Tok::LParen | Tok::LBracket | Tok::Indent | Tok::LBrace | Tok::InterpStart => depth += 1,
                Tok::RParen | Tok::RBracket | Tok::Outdent | Tok::RBrace | Tok::InterpEnd => {
                    if depth == 0 {
                        return Err(None);
                    }
                    depth -= 1;
                }
                _ => {}
            }
            i += 1;
        }
    }

    /// An `end` marker between the last token and the current one closes a construct too
    /// early, as `end while` before the `do` of the loop does.
    fn reject_end_marker_before(&mut self, what: &str) {
        let from = self.prev_span().end;
        let to = self.span().start;
        let i = self.end_markers.partition_point(|m| m.start < from);
        if let Some(&m) = self.end_markers.get(i).filter(|m| m.end <= to) {
            self.error_at(m, format!("expected {} before the end marker", what));
        }
    }

    fn at_soft(&self, n: Name) -> bool {
        self.at(Tok::Ident) && self.tok().name == n
    }

    fn at_op(&self, n: Name) -> bool {
        self.at(Tok::OpIdent) && self.tok().name == n
    }

    fn eat_op(&mut self, n: Name) -> bool {
        if self.at_op(n) {
            self.bump();
            true
        } else {
            false
        }
    }

    /// Records the span of the name of the selection or operator `e` when the index is kept.
    #[inline]
    fn note_name(&mut self, e: ExprId, span: Span) {
        if self.syntax.index {
            self.ast.name_spans.push((e, span));
        }
    }

    #[inline]
    fn note_ty_name(&mut self, t: TyExprId, span: Span) {
        if self.syntax.index {
            self.ast.ty_name_spans.push((t, span));
        }
    }

    /// A name of an import selector, kept as the last entry of `import_names` until the
    /// selector is complete (`note_import`).
    fn note_selector_name(&mut self, span: Span) {
        if self.syntax.index {
            self.ast.import_names.push((Span::default(), vec![span]));
        }
    }

    /// The selector whose names from `sel_start` on were noted is complete: one entry of
    /// `import_names` with the path's spans before them.
    fn note_import(&mut self, sel_start: usize, path: &[Span], span: Span) {
        if !self.syntax.index {
            return;
        }
        let mut names = path.to_vec();
        for (_, n) in self.ast.import_names.drain(sel_start..) {
            names.extend(n);
        }
        self.ast.import_names.push((span, names));
    }

    /// Gives the definitions from `first` on that have no range yet (the ones just parsed, not
    /// those nested in them) the range from `start` to the last token.
    fn close_def_ranges(&mut self, first: usize, start: u32) {
        let end = self.prev_span().end;
        for r in &mut self.ast.def_ranges[first..] {
            if *r == crate::ast::NO_RANGE {
                *r = Span { start, end: end.max(start) };
            }
        }
    }

    fn skip_separators(&mut self) {
        while matches!(self.kind(), Tok::Newline | Tok::Semi) {
            self.bump();
            self.quiet = false;
        }
    }

    fn expect_ident(&mut self) -> (Name, Span) {
        let t = self.tok();
        if matches!(t.kind, Tok::Ident | Tok::OpIdent) {
            self.bump();
            (t.name, t.span)
        } else {
            self.error_at(t.span, format!("expected an identifier, found {}", t.kind.describe()));
            (names::EMPTY, t.span)
        }
    }

    /// Placeholder parameters only need distinct names within one lambda.
    fn placeholder_name(&mut self, index: usize) -> Name {
        if self.placeholder_names.len() <= index {
            for i in self.placeholder_names.len()..=index {
                let n = self.interner.intern(&format!("_${}", i));
                self.placeholder_names.push(n);
            }
        }
        self.placeholder_names[index]
    }

    fn fresh_name(&mut self, prefix: &str) -> Name {
        self.fresh_counter += 1;
        let s = format!("{}${}", prefix, self.fresh_counter);
        self.interner.intern(&s)
    }

    /// Skips the current token, which nothing could start with.
    fn skip_token(&mut self, site: RecoverySite) {
        let from = self.pos;
        self.bump();
        self.note_skipped(site, from);
    }

    /// Records the tokens from `from` up to the current one as skipped by `site`, as part of the
    /// skip that ended right before them.
    fn note_skipped(&mut self, site: RecoverySite, from: usize) {
        if self.pos <= from {
            return;
        }
        let end = self.toks[self.pos - 1].span;
        match self.last_skip {
            Some((i, after)) if after == from && i < self.ast.recoveries.len() && self.ast.recoveries[i].site == site => {
                let r = &mut self.ast.recoveries[i];
                r.span = r.span.to(end);
            }
            _ => {
                let span = self.toks[from].span.to(end);
                self.ast.recoveries.push(Recovery { site, span });
            }
        }
        self.last_skip = Some((self.ast.recoveries.len() - 1, self.pos));
    }

    /// The index of the token that opens the list `close` ends, around the current token.
    fn list_opener(&self, close: Tok) -> Option<usize> {
        let open = match close {
            Tok::RParen => Tok::LParen,
            Tok::RBracket => Tok::LBracket,
            _ => Tok::LBrace,
        };
        let mut depth = 0u32;
        for i in (0..self.pos).rev() {
            match self.toks[i].kind {
                Tok::LParen | Tok::LBracket | Tok::LBrace | Tok::InterpStart => {
                    if depth == 0 {
                        return (self.toks[i].kind == open).then_some(i);
                    }
                    depth -= 1;
                }
                Tok::RParen | Tok::RBracket | Tok::RBrace | Tok::InterpEnd => depth += 1,
                _ => {}
            }
        }
        None
    }

    /// Whether the current token is the first of its line.
    fn line_starts_here(&self) -> bool {
        let start = self.span().start as usize;
        let prev = self.toks[..self.pos].iter().rev().find(|t| t.span.start < t.span.end).map_or(0, |t| t.span.end as usize);
        self.text.as_bytes()[prev.min(start)..start].contains(&b'\n')
    }

    /// The indentation of the line holding the offset `at`.
    fn line_indent(&self, at: u32) -> u32 {
        let bytes = self.text.as_bytes();
        let start = bytes[..at as usize].iter().rposition(|&b| b == b'\n').map_or(0, |i| i + 1);
        bytes[start..].iter().take_while(|&&b| b == b' ' || b == b'\t').count() as u32
    }

    /// The application or instantiation `e` of the argument lists parsed last is noted when a
    /// list was cut (`Ast::cut_args`).
    #[inline]
    pub(super) fn note_cut_args(&mut self, e: ExprId, cut: bool) {
        if cut {
            self.ast.cut_args.push(e);
        }
    }

    fn expr_list(&mut self, mark: usize) -> ListRef {
        let l = push_list(&mut self.ast.expr_lists, &self.expr_scratch[mark..]);
        self.expr_scratch.truncate(mark);
        l
    }

    fn pat_list(&mut self, mark: usize) -> ListRef {
        let l = push_list(&mut self.ast.pat_lists, &self.pat_scratch[mark..]);
        self.pat_scratch.truncate(mark);
        l
    }

    fn ty_list(&mut self, mark: usize) -> ListRef {
        let l = push_list(&mut self.ast.ty_lists, &self.ty_scratch[mark..]);
        self.ty_scratch.truncate(mark);
        l
    }

    /// True at `package a.b:` with an indented body, as opposed to a clause for the whole file.
    fn package_block_follows(&self) -> bool {
        let mut i = 1;
        while self.kind_at(i) == Tok::Ident && self.kind_at(i + 1) == Tok::Dot {
            i += 2;
        }
        self.kind_at(i) == Tok::Ident && self.kind_at(i + 1) == Tok::ColonEol && self.kind_at(i + 2) == Tok::Indent
    }

    fn parse_file(&mut self) {
        self.skip_separators();
        while self.at(Tok::KwPackage) && self.kind_at(1) != Tok::KwObject && !self.package_block_follows() {
            let start = self.bump().span;
            loop {
                let (n, _) = self.expect_ident();
                self.ast.package.push(n);
                if !self.eat(Tok::Dot) {
                    break;
                }
            }
            self.ast.package_clauses.push(self.ast.package.len() as u32);
            if self.syntax.index {
                self.ast.package_ranges.push(start.to(self.prev_span()));
            }
            self.skip_separators();
        }
        loop {
            self.statements(RecoverySite::TopLevel, |p| p.parse_top_stmt());
            if self.at(Tok::Eof) {
                break;
            }
            self.error_at(self.span(), "unexpected indentation");
            self.skip_token(RecoverySite::Indentation);
        }
        self.add_top_level_derived_givens();
    }

    fn add_top_level_derived_givens(&mut self) {
        if !self.pending_derives.is_empty() {
            let defs = self.ast.top_level.clone();
            let companions = self.add_derived_givens(0, &defs);
            self.ast.top_level.extend(companions);
        }
    }

    /// `package p:` or `package p { ... }` among the definitions: its statements go to an AST of
    /// their own whose package clauses are the file's plus `p`.
    fn parse_package_block(&mut self) {
        let start = self.bump().span;
        let outer = self.open_package_block();
        loop {
            let (n, _) = self.expect_ident();
            self.ast.package.push(n);
            if !self.eat(Tok::Dot) {
                break;
            }
        }
        self.ast.package_clauses.push(self.ast.package.len() as u32);
        if self.syntax.index {
            self.ast.package_ranges.push(start.to(self.prev_span()));
        }
        if !self.parse_package_body() {
            self.error_at(start.to(self.prev_span()), "a package clause after other definitions needs a body: `package p:` or `package p { ... }`");
        }
        self.close_package_block(outer);
    }

    fn open_package_block(&mut self) -> Ast {
        let outer = std::mem::replace(&mut self.ast, Ast::new(0));
        self.ast.package = outer.package.clone();
        self.ast.package_clauses = outer.package_clauses.clone();
        self.ast.imports = outer.imports.clone();
        outer
    }

    fn close_package_block(&mut self, outer: Ast) {
        self.add_top_level_derived_givens();
        let block = std::mem::replace(&mut self.ast, outer);
        self.package_blocks.push(block);
    }

    /// The indented statements of a package block, if there are any.
    fn parse_package_body(&mut self) -> bool {
        if !(self.at(Tok::ColonEol) && self.kind_at(1) == Tok::Indent) {
            return false;
        }
        self.bump();
        self.bump();
        self.statements(RecoverySite::TopLevel, |p| p.parse_top_stmt());
        self.end_region();
        true
    }

    fn parse_top_stmt(&mut self) {
        match self.kind() {
            Tok::KwPackage if self.kind_at(1) == Tok::KwObject => self.parse_package_object(),
            Tok::KwPackage => self.parse_package_block(),
            Tok::KwImport => self.parse_import(),
            Tok::KwExport => {
                self.parse_export();
                let clauses = self.export_scratch.drain(..);
                self.ast.top_exports.extend(clauses);
            }
            _ if self.at_def_start() => {
                let mut out = Vec::new();
                self.parse_def(&mut out);
                self.ast.top_level.extend(out);
            }
            _ => {
                let (at, found) = self.found();
                self.error_at(at, format!("expected a definition, found {}", found));
            }
        }
    }

    /// The members of `package object p` are the top-level definitions of the package `p`, read
    /// as a package block. Its parents are those of an `object package` in that block, whose
    /// members the lookups of the package fall back to.
    fn parse_package_object(&mut self) {
        self.bump();
        self.bump();
        let (name, span) = self.expect_ident();
        let outer = self.open_package_block();
        self.ast.package.push(name);
        self.ast.package_clauses.push(self.ast.package.len() as u32);
        let parents = self.parse_parents();
        if !parents.is_empty() {
            let class = ClassDef {
                kind: ClassKind::Object,
                tparams: Vec::new(),
                clauses: Vec::new(),
                parents,
                body: Vec::new(),
                exports: ListRef::EMPTY,
                self_type: None,
                self_alias: names::EMPTY,
            };
            let def = self.ast.add_def(Def {
                name: names::PACKAGE,
                span,
                mods: 0,
                annots: Vec::new(),
                kind: DefKind::Class(Box::new(class)),
            });
            self.ast.top_level.push(def);
        }
        self.parse_package_body();
        self.ast.package_object = true;
        self.close_package_block(outer);
    }

    fn parse_import(&mut self) {
        let start = self.span();
        self.expect(Tok::KwImport);
        let mut imports = std::mem::take(&mut self.ast.imports);
        self.parse_import_exprs(&mut imports);
        self.ast.imports = imports;
        if self.syntax.index {
            self.ast.import_ranges.push(start.to(self.prev_span()));
        }
        let language = std::mem::take(&mut self.language_scratch);
        self.ast.language_imports.extend(language);
    }

    /// The clauses are collected until the enclosing body ends, see `export_list`.
    pub(super) fn parse_export(&mut self) {
        self.expect(Tok::KwExport);
        let mut exports = std::mem::take(&mut self.export_scratch);
        self.parse_import_exprs(&mut exports);
        self.export_scratch = exports;
    }

    pub(super) fn export_list(&mut self, mark: usize) -> ListRef {
        let start = self.ast.exports.len() as u32;
        self.ast.exports.extend(self.export_scratch.drain(mark..));
        ListRef { start, len: self.ast.exports.len() as u32 - start }
    }

    /// The part shared by `import` and `export`: one entry per selector is appended to `out`.
    /// `import scala.language.*` and its selectors switch on features of scalac and are dropped.
    fn parse_import_exprs(&mut self, out: &mut Vec<Import>) {
        let mark = out.len();
        loop {
            let start = self.span();
            let mut path = Vec::new();
            let mut path_spans = Vec::new();
            loop {
                if self.at(Tok::LBrace) {
                    self.bump();
                    let mut cut = false;
                    loop {
                        let sel_start = self.ast.import_names.len();
                        let from = self.span();
                        let (sel, bound) = self.parse_import_selector();
                        let span = start.to(self.prev_span());
                        let selector_span = from.to(self.prev_span());
                        self.note_import(sel_start, &path_spans, span);
                        out.push(Import { path: path.clone(), sel, span, selector_span, bound });
                        if !self.list_continues(Tok::RBrace, &mut cut) {
                            break;
                        }
                    }
                    self.expect(Tok::RBrace);
                    break;
                }
                if self.at_op(names::STAR) || self.at(Tok::Underscore) || self.at(Tok::KwGiven) {
                    let sel_start = self.ast.import_names.len();
                    let from = self.span();
                    let (sel, bound) = self.parse_import_selector();
                    let span = start.to(self.prev_span());
                    let selector_span = from.to(self.prev_span());
                    self.note_import(sel_start, &path_spans, span);
                    out.push(Import { path, sel, span, selector_span, bound });
                    break;
                }
                // `import a.b.` cut after its last `.` imports what `a.b` holds, which scalac's
                // erroneous selector resolves names through; a definition on the next line is no
                // selector, whatever soft modifier it starts with.
                let cut = !matches!(self.kind(), Tok::Ident | Tok::OpIdent) || (self.at_def_start() && self.line_starts_here());
                if !path.is_empty() && cut {
                    let (at, found) = self.found();
                    self.error_at(at, format!("expected an identifier, found {}", found));
                    let span = start.to(self.prev_span());
                    let selector_span = self.prev_span();
                    out.push(Import { path, sel: ImportSel::Wildcard, span, selector_span, bound: None });
                    break;
                }
                let (n, name_span) = self.expect_ident();
                if self.eat(Tok::Dot) {
                    path.push(n);
                    path_spans.push(name_span);
                    continue;
                }
                let sel_start = self.ast.import_names.len();
                self.note_selector_name(name_span);
                let rename = if self.at_soft(names::AS) {
                    self.bump();
                    // `import a.B as _` hides `B`, as a selector in braces does.
                    if self.eat(Tok::Underscore) {
                        Some(names::WILDCARD)
                    } else {
                        let (r, r_span) = self.expect_ident();
                        self.note_selector_name(r_span);
                        Some(r)
                    }
                } else {
                    None
                };
                let span = start.to(self.prev_span());
                let selector_span = name_span.to(self.prev_span());
                self.note_import(sel_start, &path_spans, span);
                out.push(Import { path, sel: ImportSel::Name(n, rename), span, selector_span, bound: None });
                break;
            }
            if !self.eat(Tok::Comma) {
                break;
            }
        }
        let mut kept = mark;
        self.language_scratch.clear();
        for i in mark..out.len() {
            if !is_language_import(&out[i]) {
                out.swap(kept, i);
                kept += 1;
                continue;
            }
            if matches!(out[i].sel, ImportSel::Name(names::STRICT_EQUALITY, _)) {
                self.ast.strict_equality = true;
            } else if matches!(out[i].sel, ImportSel::Name(names::FUTURE, _)) {
                self.ast.source_future = true;
            }
            self.language_scratch.push(out[i].clone());
        }
        out.truncate(kept);
    }

    /// Also takes the Scala 2 forms `_`, `a => b` and `a => _`; `given T` imports the givens
    /// like `given` does, without the type as a filter, which comes back with it.
    fn parse_import_selector(&mut self) -> (ImportSel, Option<crate::ast::TyExprId>) {
        if self.eat_op(names::STAR) || self.eat(Tok::Underscore) {
            return (ImportSel::Wildcard, None);
        }
        if self.eat(Tok::KwGiven) {
            let mut bound = None;
            if !matches!(self.kind(), Tok::Comma | Tok::RBrace | Tok::Newline | Tok::Semi | Tok::Outdent | Tok::Eof) {
                bound = Some(self.parse_type());
            }
            return (ImportSel::Given, bound);
        }
        let (n, name_span) = self.expect_ident();
        self.note_selector_name(name_span);
        if self.at_soft(names::AS) || self.at(Tok::Arrow) {
            self.bump();
            if self.eat(Tok::Underscore) {
                return (ImportSel::Name(n, Some(names::WILDCARD)), None);
            }
            let (r, r_span) = self.expect_ident();
            self.note_selector_name(r_span);
            return (ImportSel::Name(n, Some(r)), None);
        }
        (ImportSel::Name(n, None), None)
    }
}

/// A definition keyword or a hard modifier, which a broken header's skip stops before.
fn starts_definition(k: Tok) -> bool {
    matches!(
        k,
        Tok::KwVal | Tok::KwVar | Tok::KwDef | Tok::KwClass | Tok::KwTrait | Tok::KwObject | Tok::KwEnum | Tok::KwType
            | Tok::KwGiven | Tok::KwPrivate | Tok::KwProtected | Tok::KwSealed | Tok::KwAbstract | Tok::KwFinal
            | Tok::KwLazy | Tok::KwOverride | Tok::KwImplicit | Tok::KwImport | Tok::KwExport | Tok::KwPackage
    )
}

fn is_language_import(imp: &Import) -> bool {
    let path = match imp.path.split_first() {
        Some((&names::SCALA, rest)) => rest,
        _ => &imp.path[..],
    };
    match path.first() {
        Some(&names::LANGUAGE) => true,
        None => matches!(imp.sel, ImportSel::Name(names::LANGUAGE, _)),
        _ => false,
    }
}

/// The low surrogate of a `\uXXXX` escape next in `chars`, consumed only when it is one.
fn low_surrogate_escape(chars: &mut std::iter::Peekable<std::str::Chars>) -> Option<u32> {
    let mut ahead = chars.clone();
    if ahead.next()? != '\\' || ahead.next()? != 'u' {
        return None;
    }
    let mut skip = 2;
    let mut c = ahead.next()?;
    while c == 'u' {
        skip += 1;
        c = ahead.next()?;
    }
    let mut low = c.to_digit(16)?;
    for _ in 0..3 {
        low = low * 16 + ahead.next()?.to_digit(16)?;
    }
    if !(0xDC00..=0xDFFF).contains(&low) {
        return None;
    }
    for _ in 0..skip + 4 {
        chars.next();
    }
    Some(low)
}

pub fn unescape(s: &str, interpolated: bool, errors: &mut Vec<(Span, String)>, at: Span) -> String {
    if !s.contains('\\') && !(interpolated && s.contains("$$")) {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '$' && interpolated && chars.peek() == Some(&'$') {
            chars.next();
            out.push('$');
        } else if c == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('r') => out.push('\r'),
                Some('b') => out.push('\u{8}'),
                Some('f') => out.push('\u{c}'),
                Some('0') => out.push('\0'),
                Some('\\') => out.push('\\'),
                Some('"') => out.push('"'),
                Some('\'') => out.push('\''),
                Some('u') => {
                    while chars.peek() == Some(&'u') {
                        chars.next();
                    }
                    let hex: String = (0..4).filter_map(|_| chars.next()).collect();
                    let unit = u32::from_str_radix(&hex, 16).ok();
                    // A high surrogate escaped next to a low one spells one code point, one below
                    // the stand-ins of `crate::text`.
                    let code = match unit {
                        Some(high) if crate::text::pair_joins(high) => low_surrogate_escape(&mut chars).map(|low| 0x10000 + ((high - 0xD800) << 10) + (low - 0xDC00)),
                        _ => unit,
                    };
                    match (code.and_then(char::from_u32), unit) {
                        (Some(ch), _) => out.push(ch),
                        (None, Some(lone @ 0xD800..=0xDFFF)) => out.push(crate::text::lone_surrogate(lone)),
                        (None, _) => errors.push((at, "invalid unicode escape".to_string())),
                    }
                }
                _ => errors.push((at, "invalid escape sequence".to_string())),
            }
        } else {
            out.push(c);
        }
    }
    out
}
