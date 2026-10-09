use crate::intern::Interner;
use crate::names;
use crate::source::Span;
use crate::token::{keyword, Tok, Token};

#[derive(Clone, Copy, PartialEq, Debug)]
enum Region {
    Indent(u32),
    /// Parens, brackets and import selectors disable newlines. The width an indented region inside
    /// has to exceed is fixed at the first line break: that line's own when the bracket ends the
    /// line before it, else the enclosing region's (dotc's `proposeKnownWidth`).
    Enclosed(u8, Option<u32>),
    /// A brace block holds statements like an indented region. Its width is that of the first
    /// statement line, unknown until then.
    Braces(Option<u32>),
    InterpBrace { triple: bool },
}

pub struct LexResult {
    pub tokens: Vec<Token>,
    pub errors: Vec<(Span, String)>,
    /// The `end name` markers, in source order; they leave no token behind.
    pub end_markers: Vec<Span>,
    /// The lines that start inside an unclosed parenthesis or bracket indented no further than
    /// the region around it, by the offset of their first token, ascending: where the parser's
    /// grammar for the parenthesised construct fails at such a token, the line is a statement
    /// of that region and the file is lexed again with the delimiters closed before it
    /// (`lex_closing`).
    pub boundaries: Vec<u32>,
}

/// How many times the lexer reads a file again to close the delimiters an error left open.
pub const MAX_RELEXES: usize = 16;

struct Lexer<'a> {
    src: &'a [u8],
    text: &'a str,
    pos: usize,
    tokens: Vec<Token>,
    regions: Vec<Region>,
    interner: &'a Interner,
    errors: Vec<(Span, String)>,
    end_markers: Vec<Span>,
    extension_line: bool,
    /// The trivia last skipped held a line with nothing on it.
    blank_line: bool,
    /// Region depths of the parentheses that follow `if`, `while` and `for`; their `)` opens
    /// an indented body on the next line, as scalac's `observeIndented` does.
    cond_parens: Vec<usize>,
    cond_closed: bool,
    /// The lines, by their first token's offset, before which the parentheses and brackets
    /// open around them are closed.
    close_parens: &'a [u32],
    /// The lines before which the innermost brace block open around them is closed.
    close_braces: &'a [u32],
    boundaries: Vec<u32>,
    /// The `}` that are dropped as closing nothing: a `}` the diagnosis found closing a block
    /// whose `{` stands on a line indented less than the `}`'s, where another `}` closes nothing.
    drop_closers: &'a [u32],
    /// The file is lexed again to close delimiters and braces (`close_parens`, `close_braces`).
    relexing: bool,
    /// A brace block was left open at the end of the file.
    brace_left_open: bool,
    /// A `}` closed nothing.
    closer_unmatched: bool,
    /// While diagnosing: the `}` that start a line indented further than the line of the `{`
    /// of the block they close.
    suspect_closers: Vec<u32>,
    /// Set when the file is read again to find where a `}` is missing: the brace blocks in the
    /// order they open, and the indices of the open ones.
    diagnose: bool,
    brace_blocks: Vec<BraceBlock>,
    open_blocks: Vec<usize>,
    /// Per indentation region, by its depth in `regions`, the index of the token it opened at;
    /// an entry is set when a region is pushed at its depth.
    indent_starts: Vec<u32>,
}

/// A brace block, while the lexer looks for a missing `}`.
struct BraceBlock {
    /// The indentation of the line its `{` stands on.
    open_width: u32,
    /// Its first line indented less than its statements (scalac's "Line is indented too far to
    /// the left, or a `}` is missing"), a closer or a token that continues a statement aside.
    left_line: Option<u32>,
    /// It was left open at the end of the file, or closed by a `}` that starts a line
    /// indented less than the line of its `{`: the `}` it lacks belongs before `left_line`.
    suspect: bool,
}

pub fn lex(text: &str, interner: &Interner) -> LexResult {
    lex_closing(text, interner, &[])
}

/// `lex`, with the parentheses and brackets open around the lines at `close_parens` closed
/// before them. When a brace block is left open at the end of the file, the file is read again
/// to find the blocks that lack their `}` (`BraceBlock::suspect`), and again with the earliest
/// such block closed before its first line indented less than it, as long as one is found.
pub fn lex_closing(text: &str, interner: &Interner, close_parens: &[u32]) -> LexResult {
    let mut close_braces: Vec<u32> = Vec::new();
    let mut drop_closers: Vec<u32> = Vec::new();
    let mut diagnose = false;
    loop {
        let mut lx = Lexer {
            src: text.as_bytes(),
            text,
            // A script header (`#!/usr/bin/env -S teq interp`) is skipped as scalac skips it.
            pos: crate::directives::script_header(text),
            tokens: Vec::with_capacity(text.len() / 4 + 16),
            regions: vec![Region::Indent(0)],
            interner,
            errors: Vec::new(),
            end_markers: Vec::new(),
            extension_line: false,
            blank_line: false,
            cond_parens: Vec::new(),
            cond_closed: false,
            close_parens,
            close_braces: &close_braces,
            drop_closers: &drop_closers,
            relexing: !close_braces.is_empty() || !close_parens.is_empty(),
            boundaries: Vec::new(),
            brace_left_open: false,
            closer_unmatched: false,
            suspect_closers: Vec::new(),
            diagnose,
            brace_blocks: Vec::new(),
            open_blocks: Vec::new(),
            indent_starts: vec![0],
        };
        lx.run();
        let unbalanced = lx.brace_left_open || lx.closer_unmatched;
        if unbalanced && !diagnose {
            diagnose = true;
            continue;
        }
        let relexes = close_braces.len() + drop_closers.len();
        let stray = lx.suspect_closers.iter().copied().filter(|c| !drop_closers.contains(c)).min();
        let lines = lx.brace_blocks.iter().filter(|b| b.suspect).filter_map(|b| b.left_line);
        let missing = lines.filter(|l| !close_braces.contains(l)).min();
        match (stray, missing) {
            (Some(c), _) if lx.closer_unmatched && relexes < MAX_RELEXES => {
                drop_closers.push(c);
                drop_closers.sort_unstable();
            }
            (_, Some(line)) if lx.brace_left_open && relexes < MAX_RELEXES => {
                close_braces.push(line);
                close_braces.sort_unstable();
            }
            _ => return LexResult { tokens: lx.tokens, errors: lx.errors, end_markers: lx.end_markers, boundaries: lx.boundaries },
        }
    }
}

/// Where the block comment opening at `start` (its `/*`) ends, past its `*/`: comments nest, each
/// `/*` inside one opening another that its own `*/` closes, as scalac reads them. None when the
/// text ends first. The lexer and the header directives' scan (`crate::directives`) read comments
/// by this one rule.
pub(crate) fn block_comment_end(src: &[u8], start: usize) -> Option<usize> {
    block_comment(src, start).0
}

/// `block_comment_end`, and where the comment's last line starts (past its last newline) when it
/// holds one, found in the one pass.
fn block_comment(src: &[u8], start: usize) -> (Option<usize>, Option<usize>) {
    let mut pos = start + 2;
    let mut depth = 1;
    let mut line = None;
    while depth > 0 {
        let Some(&b) = src.get(pos) else { return (None, line) };
        match b {
            b'\n' => {
                line = Some(pos + 1);
                pos += 1;
            }
            b'/' if src.get(pos + 1) == Some(&b'*') => {
                depth += 1;
                pos += 2;
            }
            b'*' if src.get(pos + 1) == Some(&b'/') => {
                depth -= 1;
                pos += 2;
            }
            _ => pos += 1,
        }
    }
    (Some(pos), line)
}

#[inline]
fn is_ident_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_' || b == b'$' || b >= 0x80
}

#[inline]
pub(crate) fn is_ident_part(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'$' || b >= 0x80
}

#[inline]
pub(crate) fn is_op_char(b: u8) -> bool {
    matches!(
        b,
        b'!' | b'#' | b'%' | b'&' | b'*' | b'+' | b'-' | b'/' | b':' | b'<' | b'=' | b'>' | b'?'
            | b'@' | b'\\' | b'^' | b'|' | b'~'
    )
}

impl<'a> Lexer<'a> {
    #[inline]
    fn peek(&self, off: usize) -> u8 {
        *self.src.get(self.pos + off).unwrap_or(&0)
    }

    fn error(&mut self, start: usize, end: usize, msg: &str) {
        self.errors.push((Span::new(start as u32, end as u32), msg.to_string()));
    }

    #[inline]
    fn push(&mut self, kind: Tok, start: usize, end: usize) {
        self.tokens.push(Token {
            kind,
            span: Span::new(start as u32, end as u32),
            name: names::EMPTY,
        });
    }

    /// Skips whitespace and comments. Returns the indentation width of the new line if a line
    /// break was crossed. A line break inside a block comment is one, as dotc's scanner has it
    /// (`val a = 1 /*⏎*/ val b = 2` is two statements); the line it ends in is then indented by
    /// its leading whitespace, before the comment's end.
    #[inline(always)]
    fn skip_trivia(&mut self) -> Option<u32> {
        let mut new_line: Option<usize> = None;
        let mut in_comment = false;
        let mut comment_on_line = false;
        self.blank_line = false;
        loop {
            match self.peek(0) {
                b' ' | b'\t' | b'\r' => self.pos += 1,
                b'\n' => {
                    self.pos += 1;
                    self.blank_line |= new_line.is_some() && !comment_on_line;
                    new_line = Some(self.pos);
                    in_comment = false;
                    comment_on_line = false;
                }
                b'/' if self.peek(1) == b'/' => {
                    comment_on_line = true;
                    while self.pos < self.src.len() && self.src[self.pos] != b'\n' {
                        self.pos += 1;
                    }
                }
                b'/' if self.peek(1) == b'*' => {
                    comment_on_line = true;
                    let start = self.pos;
                    let (end, line) = block_comment(self.src, start);
                    if line.is_some() {
                        new_line = line;
                        in_comment = true;
                    }
                    match end {
                        Some(end) => self.pos = end,
                        None => {
                            self.pos = self.src.len();
                            self.error(start, start + 2, "unterminated comment");
                        }
                    }
                }
                _ => break,
            }
        }
        new_line.map(|ls| match in_comment {
            true => self.src[ls..].iter().take_while(|&&c| c == b' ' || c == b'\t').count() as u32,
            false => (self.pos - ls) as u32,
        })
    }

    fn at_line_end(&self, mut p: usize) -> bool {
        while p < self.src.len() {
            match self.src[p] {
                b' ' | b'\t' | b'\r' => p += 1,
                b'\n' => return true,
                b'/' if self.src.get(p + 1) == Some(&b'/') => return true,
                b'/' if self.src.get(p + 1) == Some(&b'*') => {
                    match self.text[p + 2..].find("*/") {
                        Some(end) if !self.text[p + 2..p + 2 + end].contains('\n') => p += end + 4,
                        _ => return true,
                    }
                }
                _ => return false,
            }
        }
        true
    }

    /// A comma is a trailing one when the closing bracket follows on a later line.
    #[inline]
    fn at_trailing_comma(&mut self) -> bool {
        let mut p = self.pos + 1;
        while matches!(self.src.get(p), Some(b' ' | b'\t' | b'\r')) {
            p += 1;
        }
        matches!(self.src.get(p), Some(b'\n' | b'/')) && self.closing_bracket_follows()
    }

    #[cold]
    fn closing_bracket_follows(&mut self) -> bool {
        let (pos, errors) = (self.pos, self.errors.len());
        self.pos += 1;
        let crossed_line = self.skip_trivia().is_some();
        let closes = matches!(self.peek(0), b')' | b']' | b'}');
        self.pos = pos;
        self.errors.truncate(errors);
        crossed_line && closes
    }

    fn try_skip_end_marker(&mut self) -> bool {
        if !self.text[self.pos..].starts_with("end ") {
            return false;
        }
        let mut p = self.pos + 4;
        while p < self.src.len() && self.src[p] == b' ' {
            p += 1;
        }
        let word_start = p;
        while p < self.src.len() && is_ident_part(self.src[p]) {
            p += 1;
        }
        if p == word_start || !self.at_line_end(p) {
            return false;
        }
        self.end_markers.push(Span::new(self.pos as u32, p as u32));
        self.pos = p;
        true
    }

    fn run(&mut self) {
        let mut first = true;
        // An end marker ends the statement before it: the line after it is a statement of its
        // own, as scalac's `statSepOrEnd` has it, whatever it starts with.
        let mut after_end_marker = false;
        loop {
            let mut line_break = self.skip_trivia();
            if first {
                line_break = line_break.or(Some(self.pos as u32));
                first = false;
            }
            if self.pos >= self.src.len() {
                break;
            }
            if line_break.is_some() && self.try_skip_end_marker() {
                after_end_marker = true;
                continue;
            }
            if let Some(width) = line_break {
                let start = self.pos;
                if self.relexing {
                    self.close_at_line(width, start);
                }
                let (next, next_end) = self.peek_kind();
                let leading_op = !std::mem::take(&mut after_end_marker) && self.leading_operator(next, start, next_end, width);
                let starts_extension = next == Tok::Ident && &self.text[start..next_end] == "extension";
                let insert_at = self.tokens.len();
                let cond_body = self.cond_closed && next.can_start_statement() && !(leading_op && self.infix_fits_region(width));
                if !(next == Tok::LBrace && (cond_body || self.template_header_open())) {
                    self.line_break(width, next, leading_op, cond_body, insert_at, start);
                }
                if matches!(self.regions.last(), Some(Region::Enclosed(..))) {
                    self.note_boundary(width, start);
                }
                // The header of an extension may continue with a parameter clause on the next line.
                self.extension_line = starts_extension || (self.extension_line && next == Tok::LParen);
            }
            self.lex_token();
        }
        self.finish();
    }

    /// The regions left open at the end of the file, and the end.
    #[inline(never)]
    fn finish(&mut self) {
        let end = self.src.len();
        let last_end = self.tokens.last().map_or(0, |t| t.span.end as usize);
        while let Some(r) = self.regions.pop() {
            match r {
                Region::Indent(w) => {
                    if w > 0 || !self.regions.is_empty() {
                        self.push(Tok::Outdent, end, end);
                    }
                }
                Region::Braces(_) => {
                    if !self.brace_left_open {
                        self.error(last_end, last_end, "expected '}', found end of file");
                    }
                    self.brace_left_open = true;
                    if let Some(b) = self.open_blocks.pop() {
                        self.brace_blocks[b].suspect = true;
                    }
                }
                _ => {}
            }
        }
        self.push(Tok::Eof, end, end);
    }

    /// At the line starting at `start`, in a file lexed again: the brace block or the delimiters
    /// the parser's or the lexer's diagnosis closes before it.
    #[cold]
    #[inline(never)]
    fn close_at_line(&mut self, width: u32, start: usize) {
        if self.close_braces.binary_search(&(start as u32)).is_ok() {
            self.close_brace_block(start);
        }
        if self.close_parens.binary_search(&(start as u32)).is_ok() {
            self.close_enclosed(width, start);
        }
    }

    /// The width of the innermost indentation or brace region below `regions[..below]` whose
    /// width is known.
    fn owning_width(&self, below: usize) -> Option<u32> {
        self.regions[..below].iter().rev().find_map(|r| match *r {
            Region::Indent(w) | Region::Braces(Some(w)) => Some(w),
            _ => None,
        })
    }

    /// A line starting inside a parenthesis or bracket, indented no further than the region
    /// that owns the statement around them, is a boundary the parser may close them at.
    #[cold]
    #[inline(never)]
    fn note_boundary(&mut self, width: u32, start: usize) {
        let first = self.regions.iter().rposition(|r| !matches!(r, Region::Enclosed(..))).map_or(0, |i| i + 1);
        if self.owning_width(first).is_some_and(|w| width <= w) {
            self.boundaries.push(start as u32);
        }
    }

    /// Closes the parentheses and brackets open around the line at `start`, with the regions
    /// opened inside them, as long as the line is indented no further than the region that owns
    /// them. Their closing tokens are left for the parser to report missing.
    #[cold]
    #[inline(never)]
    fn close_enclosed(&mut self, width: u32, start: usize) {
        loop {
            let Some(e) = self.regions.iter().rposition(|r| matches!(r, Region::Enclosed(..))) else { return };
            if self.regions[e + 1..].iter().any(|r| !matches!(r, Region::Indent(_))) {
                return;
            }
            if !self.owning_width(e).is_some_and(|w| width <= w) {
                return;
            }
            while self.regions.len() > e + 1 {
                self.regions.pop();
                self.push(Tok::Outdent, start, start);
            }
            self.regions.pop();
            self.cond_parens.retain(|&d| d < e);
        }
    }

    /// Closes the innermost brace block open around the line at `start`, which is indented less
    /// than the block: its `}` is missing there.
    #[cold]
    #[inline(never)]
    fn close_brace_block(&mut self, start: usize) {
        let Some(b) = self.regions.iter().rposition(|r| matches!(r, Region::Braces(_))) else { return };
        while self.regions.len() > b {
            if let Some(r) = self.regions.pop() {
                if matches!(r, Region::Indent(_) | Region::Braces(_)) {
                    self.push(Tok::Outdent, start, start);
                }
            }
        }
        self.open_blocks.pop();
        self.cond_parens.retain(|&d| d < b);
        let (next, next_end) = self.peek_kind();
        let found = if next == Tok::Ident { "identifier" } else { next.describe() };
        self.error(start, next_end.min(self.src.len()), &format!("expected '}}', found {}", found));
    }

    /// scalac's `isLeadingInfixOperator` short of its test of the regions: the token at `start`
    /// is an operator (symbolic, backquoted, or a name ending in an operator character), white
    /// space or the line's end follows it, no blank line precedes it, and an operand follows.
    fn leading_operator(&mut self, next: Tok, start: usize, end: usize, width: u32) -> bool {
        let operator = match next {
            Tok::OpIdent => true,
            Tok::Ident => self.src[start] == b'`' || is_op_char(self.src[end - 1]),
            _ => false,
        };
        operator
            && !self.blank_line
            && matches!(self.src.get(end), Some(b' ' | b'\t' | b'\r' | b'\n'))
            && self.operand_follows(end, width)
    }

    /// The lookahead of `isLeadingInfixOperator`: the token after the operator ending at `end`
    /// starts an expression and is no operator but a unary one; on a later line, past no blank
    /// one, it stands at least as far in as the operator at `width`.
    #[cold]
    fn operand_follows(&mut self, end: usize, width: u32) -> bool {
        let (pos, errors, blank) = (self.pos, self.errors.len(), self.blank_line);
        self.pos = end;
        let line = self.skip_trivia();
        let past_blank = self.blank_line;
        let operand = self.pos;
        let (kind, kind_end) = self.peek_kind();
        (self.pos, self.blank_line) = (pos, blank);
        self.errors.truncate(errors);
        let text = &self.src[operand..kind_end.min(self.src.len())];
        let unary = |name: &[u8]| matches!(name, b"-" | b"+" | b"~" | b"!");
        let starts_expr = match kind {
            Tok::OpIdent => unary(text),
            // An interpolation starts an expression whatever its name ends in.
            Tok::Ident if self.src.get(kind_end) == Some(&b'"') => true,
            Tok::Ident if text[0] == b'`' => unary(&text[1..text.len().saturating_sub(1).max(1)]),
            Tok::Ident => !is_op_char(text[text.len() - 1]),
            Tok::IntLit | Tok::CharLit | Tok::StringLit | Tok::Quote | Tok::Underscore | Tok::Splice => true,
            Tok::LParen | Tok::LBracket | Tok::LBrace => true,
            Tok::KwNull | Tok::KwThis | Tok::KwSuper | Tok::KwTrue | Tok::KwFalse | Tok::KwReturn | Tok::KwNew => true,
            Tok::KwIf | Tok::KwWhile | Tok::KwFor | Tok::KwTry | Tok::KwThrow => true,
            _ => false,
        };
        match line {
            None => starts_expr,
            Some(w) => starts_expr && !past_blank && width <= w,
        }
    }

    /// The test of the regions of `isLeadingInfixOperator` for an operator at `width`: an
    /// indented region no wider than it, or one whose enclosing region is narrower.
    fn infix_fits_region(&self, width: u32) -> bool {
        let n = self.regions.len();
        match self.regions[n - 1] {
            Region::Indent(w) if w > width && n > 1 => match self.regions[n - 2] {
                Region::Indent(outer) => outer < width,
                _ => self.outer_width() < width,
            },
            _ => true,
        }
    }

    /// Classifies the upcoming token without consuming it, returning its kind and end offset.
    fn peek_kind(&self) -> (Tok, usize) {
        let b = self.peek(0);
        let p = self.pos;
        match b {
            b'(' => (Tok::LParen, p + 1),
            b'[' => (Tok::LBracket, p + 1),
            b'{' => (Tok::LBrace, p + 1),
            b')' => (Tok::RParen, p + 1),
            b']' => (Tok::RBracket, p + 1),
            b'}' => (Tok::RBrace, p + 1),
            b',' => (Tok::Comma, p + 1),
            b';' => (Tok::Semi, p + 1),
            b'.' if !self.peek(1).is_ascii_digit() => (Tok::Dot, p + 1),
            b'"' => (Tok::StringLit, p + 1),
            b'\'' if self.quote_kind(p).is_some() => (Tok::Quote, p + 1),
            b'\'' => (Tok::CharLit, p + 1),
            b'`' => {
                let close = self.src[p + 1..].iter().position(|&c| matches!(c, b'`' | b'\n'));
                let end = close.map_or(self.src.len(), |i| p + 1 + i);
                (Tok::Ident, if self.src.get(end) == Some(&b'`') { end + 1 } else { p + 1 })
            }
            b'$' if self.peek(1) == b'{' => (Tok::Splice, p + 1),
            b'0'..=b'9' | b'.' => (Tok::IntLit, p + 1),
            _ if is_ident_start(b) => {
                let mut e = p;
                while e < self.src.len() && is_ident_part(self.src[e]) {
                    e += 1;
                }
                if self.src[e - 1] == b'_' && e - p > 1 {
                    while e < self.src.len() && is_op_char(self.src[e]) {
                        e += 1;
                    }
                }
                let s = &self.text[p..e];
                if s == "_" {
                    (Tok::Underscore, e)
                } else {
                    (keyword(s).unwrap_or(Tok::Ident), e)
                }
            }
            _ if is_op_char(b) => {
                let mut e = p;
                while e < self.src.len() && is_op_char(self.src[e]) {
                    if self.src[e] == b'/' && matches!(self.src.get(e + 1), Some(b'/' | b'*')) {
                        break;
                    }
                    e += 1;
                }
                let kind = match &self.text[p..e] {
                    "=" => Tok::Eq,
                    "=>" => Tok::Arrow,
                    "?=>" => Tok::CtxArrow,
                    "=>>" => Tok::TypeLambdaArrow,
                    "<-" => Tok::LArrow,
                    "<:" => Tok::Subtype,
                    ">:" => Tok::Supertype,
                    "@" => Tok::At,
                    ":" => Tok::Colon,
                    _ => Tok::OpIdent,
                };
                (kind, e)
            }
            _ => (Tok::Eof, p + 1),
        }
    }

    /// Whether the statement so far is the header of a class, trait, object, enum or given
    /// without its body: a `{` on the next line continues it, as the language spec accepts one
    /// newline in front of an opening brace that continues the statement.
    fn template_header_open(&self) -> bool {
        for t in self.tokens.iter().rev() {
            match t.kind {
                Tok::KwClass | Tok::KwTrait | Tok::KwObject | Tok::KwEnum | Tok::KwGiven => return true,
                Tok::Eq | Tok::Newline | Tok::Semi | Tok::Indent | Tok::Outdent | Tok::ColonEol | Tok::LBrace | Tok::RBrace => {
                    return false
                }
                _ => {}
            }
        }
        false
    }

    /// The layout before the line at `pos`, whose first token is the next one pushed: its
    /// tokens go at the end of `tokens`, which `at` is.
    fn line_break(&mut self, width: u32, next: Tok, leading_op: bool, cond_body: bool, at: usize, pos: usize) {
        debug_assert_eq!(at, self.tokens.len());
        let last = if at == 0 { Tok::Eof } else { self.tokens[at - 1].kind };
        let top = *self.regions.last().unwrap_or(&Region::Indent(0));
        let opens_indent = last.can_start_indent()
            || cond_body
            || (self.extension_line && matches!(last, Tok::RParen | Tok::RBracket) && next != Tok::LParen);
        match top {
            Region::Enclosed(close, known) => {
                let w = known.unwrap_or_else(|| {
                    let w = if matches!(last, Tok::LParen | Tok::LBracket) { width } else { self.outer_width() };
                    *self.regions.last_mut().unwrap() = Region::Enclosed(close, Some(w));
                    w
                });
                if width > w && opens_indent {
                    self.push_indent(width);
                    self.push(Tok::Indent, pos, pos);
                }
            }
            Region::InterpBrace { .. } => {}
            Region::Braces(None) if !opens_indent => {
                *self.regions.last_mut().unwrap() = Region::Braces(Some(width));
                if self.diagnose {
                    self.note_first_brace_line(width, pos);
                }
                // dotc's `handleNewLine`: the first line takes the width of the braces
                // (`proposeKnownWidth`), and its break separates statements as every later one
                // does, from one begun on the line of the `{`.
                if at > 0 && last.can_end_statement() && next.can_start_statement() && !leading_op {
                    self.push(Tok::Newline, pos, pos);
                }
            }
            Region::Indent(_) | Region::Braces(_) => {
                let cur = match top {
                    Region::Indent(w) | Region::Braces(Some(w)) => w,
                    _ => 0,
                };
                if (width > cur || top == Region::Braces(None)) && opens_indent {
                    if top == Region::Braces(None) && matches!(last, Tok::Arrow | Tok::CtxArrow) {
                        *self.regions.last_mut().unwrap() = Region::Braces(Some(width));
                        if self.diagnose {
                            self.note_first_brace_line(width, pos);
                        }
                    }
                    self.push_indent(width);
                    self.push(Tok::Indent, pos, pos);
                } else {
                    let mut outdented = false;
                    // dotc's `canDedent`: a line after `yield`, `then` or `else` continues it.
                    let continued = statement_continues(last);
                    while let Some(&Region::Indent(w)) = self.regions.last().filter(|_| !continued) {
                        if width < w
                            && self.regions.len() > 1
                            && !(self.is_brace_lambda_body(w) && statement_continues(next))
                            && !(leading_op && self.infix_fits_region(width))
                        {
                            let n = self.regions.len();
                            if matches!(self.regions[n - 2], Region::Indent(outer) if outer < width) && self.misaligned(width, next, w, pos) {
                                break;
                            }
                            self.regions.pop();
                            self.push(Tok::Outdent, pos, pos);
                            outdented = true;
                        } else {
                            break;
                        }
                    }
                    if self.diagnose {
                        self.note_left_line(width, next, pos);
                    }
                    let in_statements =
                        matches!(self.regions.last(), Some(Region::Indent(_) | Region::Braces(_)));
                    let prev = if outdented { Tok::Outdent } else { last };
                    if in_statements
                        && at > 0
                        && prev.can_end_statement()
                        && next.can_start_statement()
                        && !(leading_op && self.infix_fits_region(width))
                    {
                        self.push(Tok::Newline, pos, pos);
                    }
                }
            }
        }
    }

    /// While diagnosing, a line of the top brace block indented less than its statements.
    #[cold]
    #[inline(never)]
    fn note_left_line(&mut self, width: u32, next: Tok, pos: usize) {
        if !matches!(self.regions.last(), Some(Region::Braces(Some(_)))) {
            return;
        }
        let Some(&Region::Braces(Some(w))) = self.regions.last() else { return };
        if width < w && !(closes_region(next) && !self.case_definition_at(pos)) {
            if let Some(&b) = self.open_blocks.last() {
                self.brace_blocks[b].left_line.get_or_insert(pos as u32);
            }
        }
    }

    #[inline(always)]
    fn push_indent(&mut self, width: u32) {
        let depth = self.regions.len();
        self.regions.push(Region::Indent(width));
        let at = self.tokens.len() as u32;
        if self.indent_starts.len() <= depth {
            self.indent_starts.resize(depth + 1, at);
        } else {
            self.indent_starts[depth] = at;
        }
    }

    /// Whether a line of the indentation region at `depth` stood at `width` without opening a
    /// region since the region opened: scalac's `otherIndentWidths`, read again from the tokens
    /// when it is needed. A line inside a nested region or a delimiter does not count.
    fn width_seen_in(&self, depth: usize, width: u32) -> bool {
        let Some(&from) = self.indent_starts.get(depth) else { return false };
        // Past the region's own `Indent`, which its first line put in front of its tokens; the
        // outermost region has none.
        let from = if depth == 0 { 0 } else { (from as usize + 1).min(self.tokens.len()) };
        let mut nested = 0u32;
        for t in &self.tokens[from..] {
            match t.kind {
                Tok::Indent | Tok::LParen | Tok::LBracket => nested += 1,
                Tok::Outdent | Tok::RParen | Tok::RBracket => nested = nested.saturating_sub(1),
                _ => {}
            }
            let start = t.span.start as usize;
            if nested == 0 && t.span.start < t.span.end && self.starts_line(start) && self.line_indent(start) == width {
                return true;
            }
        }
        false
    }

    /// Whether a line at `width`, starting with `next`, that leaves the top region without
    /// reaching the indentation region around it, at a width no line of that region stood at
    /// before, stays in the top region: scalac's "does not match any of the previous indentation
    /// widths" (a line that starts with a `.` passes unless it stands one column off either
    /// width). A line that cannot start a statement (`else`) leaves the region as scalac's does
    /// and continues the statement; another stays. The body of a lambda that starts on the line
    /// of its `{` is the brace block's region for scalac, which checks no width there.
    #[cold]
    #[inline(never)]
    fn misaligned(&mut self, width: u32, next: Tok, inner: u32, pos: usize) -> bool {
        let n = self.regions.len();
        let Region::Indent(outer) = self.regions[n - 2] else { return false };
        if n >= 3 && self.regions[n - 3] == Region::Braces(Some(outer)) {
            return false;
        }
        let close = |w: u32| width.abs_diff(w) <= 1;
        let passes = next == Tok::Dot && !close(inner) && !close(outer);
        if passes || self.width_seen_in(n - 2, width) {
            return false;
        }
        let msg = format!(
            "The start of this line does not match any of the previous indentation widths.\nIndentation width of current line : {} spaces\nThis falls between previous widths: {} spaces and {} spaces",
            width, outer, inner
        );
        self.error(pos, pos, &msg);
        next.can_start_statement()
    }

    /// Whether the top region is the body of a lambda or case clause that starts on the line of
    /// its `{`. dotc gives the brace block that body's width and opens no region for it, so a
    /// `yield` or `else` further left closes the regions inside the body and not the body.
    fn is_brace_lambda_body(&self, width: u32) -> bool {
        let n = self.regions.len();
        n >= 2 && self.regions[n - 2] == Region::Braces(Some(width))
    }

    /// The width of the innermost region below the top one that knows its width.
    fn outer_width(&self) -> u32 {
        let below = &self.regions[..self.regions.len().saturating_sub(1)];
        below
            .iter()
            .rev()
            .find_map(|r| match *r {
                Region::Indent(w) | Region::Braces(Some(w)) | Region::Enclosed(_, Some(w)) => Some(w),
                _ => None,
            })
            .unwrap_or(0)
    }

    fn open(&mut self, kind: Tok, close: u8) {
        let last = self.tokens.last().map_or(Tok::Eof, |t| t.kind);
        if kind == Tok::LParen && matches!(last, Tok::KwIf | Tok::KwWhile | Tok::KwFor) {
            self.cond_parens.push(self.regions.len());
        }
        self.push(kind, self.pos, self.pos + 1);
        self.pos += 1;
        self.regions.push(Region::Enclosed(close, None));
    }

    /// Whether the text from `at` starts, after white space, with `def`, `val`, `var` or `type`.
    fn declaration_follows(&self, at: usize) -> bool {
        let rest = &self.text.as_bytes()[at.min(self.text.len())..];
        let start = rest.iter().position(|b| !b.is_ascii_whitespace()).unwrap_or(rest.len());
        let word: Vec<u8> = rest[start..].iter().take_while(|b| b.is_ascii_alphanumeric() || **b == b'_').copied().collect();
        matches!(word.as_slice(), b"def" | b"val" | b"var" | b"type")
    }

    /// A brace block is handed to the parser as an indented region: `f { .. }` reads like `f:`
    /// followed by an indented block, and `= { .. }` like `=` followed by one.
    fn open_brace(&mut self) {
        let p = self.pos;
        let last = self.tokens.last().map_or(Tok::Eof, |t| t.kind);
        // Import selectors and sets of context bounds (`T: {A, B}`) are not blocks; a refinement
        // written alone as a type (`x: { def m: Int }`) is a block of declarations.
        if matches!(last, Tok::Dot | Tok::Colon) && !(last == Tok::Colon && self.declaration_follows(p + 1)) {
            return self.open(Tok::LBrace, b'}');
        }
        // After an operator the brace is a block argument (`a + { .. }`), unless the operator
        // names the class or object whose header this is (`object *: { .. }`).
        let header_name = last == Tok::OpIdent && self.template_header_open();
        if last.can_end_statement() && (!matches!(last, Tok::KwReturn | Tok::OpIdent) || header_name) {
            self.push(Tok::ColonEol, p, p + 1);
        }
        self.push(Tok::Indent, p, p + 1);
        self.pos += 1;
        self.regions.push(Region::Braces(None));
        if self.diagnose {
            let open_width = self.line_indent(p);
            self.open_blocks.push(self.brace_blocks.len());
            self.brace_blocks.push(BraceBlock { open_width, left_line: None, suspect: false });
        }
    }

    /// The first line of the innermost brace block, at `width`, stands no further in than the
    /// line of its `{`: where the block lacks its `}`, the `}` belongs before that line.
    #[cold]
    #[inline(never)]
    fn note_first_brace_line(&mut self, width: u32, pos: usize) {
        if let Some(&b) = self.open_blocks.last() {
            if width <= self.brace_blocks[b].open_width {
                self.brace_blocks[b].left_line.get_or_insert(pos as u32);
            }
        }
    }

    /// Whether the `case` at `p` starts a case class or a case object.
    fn case_definition_at(&self, p: usize) -> bool {
        let rest = self.text[p..].strip_prefix("case").unwrap_or("");
        let word = rest.trim_start_matches([' ', '\t']);
        rest.len() > word.len() && (word.starts_with("class") || word.starts_with("object"))
    }

    /// A character outside ASCII at the cursor that starts no token is reported and passed over.
    #[cold]
    #[inline(never)]
    fn illegal_here(&mut self) -> bool {
        let start = self.pos;
        let Some(c) = self.text[start..].chars().next().filter(|&c| illegal_start(c)) else { return false };
        self.error(start, start + c.len_utf8(), "unexpected character");
        self.pos += c.len_utf8();
        true
    }

    /// Whether only white space stands before the offset `p` on its line.
    fn starts_line(&self, p: usize) -> bool {
        self.src[..p].iter().rev().take_while(|&&b| b != b'\n').all(|&b| b == b' ' || b == b'\t')
    }

    /// The indentation of the line holding the offset `p`.
    #[cold]
    fn line_indent(&self, p: usize) -> u32 {
        let start = self.src[..p].iter().rposition(|&b| b == b'\n').map_or(0, |i| i + 1);
        self.src[start..p].iter().take_while(|&&b| b == b' ' || b == b'\t').count() as u32
    }

    /// The `}` at `p` closed the innermost open brace block. When it starts a line indented less
    /// than the line of the block's `{`, the block is a suspect of lacking its own `}`; further,
    /// the `}` is a suspect of having lost its `{`.
    #[cold]
    #[inline(never)]
    fn note_brace_closed(&mut self, p: usize) {
        let Some(b) = self.open_blocks.pop() else { return };
        let start = self.src[..p].iter().rposition(|&c| c == b'\n').map_or(0, |i| i + 1);
        if !self.src[start..p].iter().all(|&c| c == b' ' || c == b'\t') {
            return;
        }
        let width = (p - start) as u32;
        if width < self.brace_blocks[b].open_width {
            self.brace_blocks[b].suspect = true;
        } else if width > self.brace_blocks[b].open_width {
            self.suspect_closers.push(p as u32);
        }
    }

    fn close(&mut self, kind: Tok, close: u8) {
        let p = self.pos;
        // The common closers close the top region; the rest takes `close_past`.
        match self.regions[self.regions.len() - 1] {
            Region::Enclosed(c, _) if c == close => {
                self.regions.pop();
                while self.cond_parens.last().is_some_and(|&d| d > self.regions.len()) {
                    self.cond_parens.pop();
                }
                if self.cond_parens.last() == Some(&self.regions.len()) {
                    self.cond_parens.pop();
                    self.cond_closed = true;
                }
                self.push(kind, p, p + 1);
                self.pos += 1;
            }
            Region::Braces(_) if close == b'}' && !self.diagnose && self.drop_closers.is_empty() => {
                self.regions.pop();
                self.push(Tok::Outdent, p, p + 1);
                self.pos += 1;
            }
            _ => self.close_past(kind, close),
        }
    }

    /// A closer that does not close the region under the indented ones: the innermost region of
    /// its kind, past the delimiters left open inside it but never past a brace block for a `)`
    /// or a `]`. One that closes nothing is reported and dropped, every region left as it was.
    #[inline(never)]
    fn close_past(&mut self, kind: Tok, close: u8) {
        let p = self.pos;
        let target = self.regions.iter().rposition(|r| match *r {
            Region::Indent(_) => false,
            Region::Enclosed(c, _) => c == close,
            Region::Braces(_) | Region::InterpBrace { .. } => true,
        });
        let closes = target.is_some_and(|t| match self.regions[t] {
            Region::Enclosed(c, _) => c == close,
            Region::Braces(_) | Region::InterpBrace { .. } => close == b'}',
            Region::Indent(_) => false,
        });
        if !closes || (close == b'}' && self.drop_closers.binary_search(&(p as u32)).is_ok()) {
            self.closer_unmatched |= close == b'}' && !closes;
            self.error(p, p + 1, "unmatched closing bracket");
            self.pos += 1;
            return;
        }
        let target = target.unwrap_or(0);
        while self.regions.len() > target + 1 {
            if let Some(r) = self.regions.pop() {
                if matches!(r, Region::Indent(_) | Region::Braces(_)) {
                    self.push(Tok::Outdent, p, p);
                }
            }
        }
        self.close_top(kind, close);
    }

    /// Closes the top region, which the closer `close` closes.
    #[inline]
    fn close_top(&mut self, kind: Tok, close: u8) {
        let p = self.pos;
        if close == b'}' && matches!(self.regions.last(), Some(Region::Braces(_))) {
            self.regions.pop();
            if self.diagnose {
                self.note_brace_closed(p);
            }
            self.push(Tok::Outdent, p, p + 1);
            self.pos += 1;
            return;
        }
        match self.regions.last() {
            Some(&Region::Enclosed(c, _)) if c == close => {
                self.regions.pop();
                while self.cond_parens.last().is_some_and(|&d| d > self.regions.len()) {
                    self.cond_parens.pop();
                }
                if self.cond_parens.last() == Some(&self.regions.len()) {
                    self.cond_parens.pop();
                    self.cond_closed = true;
                }
            }
            Some(&Region::InterpBrace { triple }) if close == b'}' => {
                self.regions.pop();
                self.push(Tok::RBrace, p, p + 1);
                self.pos += 1;
                self.lex_interp_parts(triple);
                return;
            }
            _ => self.error(p, p + 1, "unmatched closing bracket"),
        }
        self.push(kind, p, p + 1);
        self.pos += 1;
    }

    fn lex_token(&mut self) {
        let b = self.peek(0);
        let start = self.pos;
        self.cond_closed = false;
        match b {
            b'(' => self.open(Tok::LParen, b')'),
            b'[' => self.open(Tok::LBracket, b']'),
            b'{' => self.open_brace(),
            b')' => self.close(Tok::RParen, b')'),
            b']' => self.close(Tok::RBracket, b']'),
            b'}' => self.close(Tok::RBrace, b'}'),
            b',' => {
                if self.at_trailing_comma() {
                    self.pos += 1;
                    return;
                }
                let indents = self.regions.iter().rev().take_while(|r| matches!(r, Region::Indent(_))).count();
                let below = self.regions.len().checked_sub(indents + 1).map(|i| self.regions[i]);
                if matches!(below, Some(Region::Enclosed(..))) {
                    for _ in 0..indents {
                        self.regions.pop();
                        self.push(Tok::Outdent, start, start);
                    }
                }
                self.push(Tok::Comma, start, start + 1);
                self.pos += 1;
            }
            b';' => {
                self.push(Tok::Semi, start, start + 1);
                self.pos += 1;
            }
            b'.' if !self.peek(1).is_ascii_digit() => {
                self.push(Tok::Dot, start, start + 1);
                self.pos += 1;
            }
            b'"' => self.lex_string(),
            b'\'' => self.lex_char(),
            b'`' => {
                self.pos += 1;
                while self.pos < self.src.len() && !matches!(self.src[self.pos], b'`' | b'\n') {
                    self.pos += 1;
                }
                // A backquote that closes nothing on its line is reported and passed over, the
                // rest of the line read as it stands.
                if self.peek(0) != b'`' {
                    self.error(start, start + 1, "unterminated backquoted identifier");
                    self.pos = start + 1;
                    return;
                }
                let name = self.interner.intern(&self.text[start + 1..self.pos]);
                self.pos += 1;
                self.tokens.push(Token {
                    kind: Tok::Ident,
                    span: Span::new(start as u32, self.pos as u32),
                    name,
                });
            }
            b'0'..=b'9' => self.lex_number(),
            b'.' => self.lex_number(),
            _ if is_ident_start(b) => self.lex_ident(),
            _ if is_op_char(b) => self.lex_op(),
            _ => {
                let len = self.text[start..].chars().next().map_or(1, |c| c.len_utf8());
                self.error(start, start + len, "unexpected character");
                self.pos += len;
            }
        }
    }

    fn lex_ident(&mut self) {
        let start = self.pos;
        if self.src[start] >= 0x80 && self.illegal_here() {
            return;
        }
        while is_ident_part(self.peek(0)) {
            self.pos += 1;
        }
        if self.src[self.pos - 1] == b'_' && is_op_char(self.peek(0)) && self.pos - start > 1 {
            while is_op_char(self.peek(0)) {
                self.pos += 1;
            }
        }
        let s = &self.text[start..self.pos];
        if self.peek(0) == b'"' && self.src[self.pos - 1] != b'_' {
            let name = self.interner.intern(s);
            self.tokens.push(Token {
                kind: Tok::InterpStart,
                span: Span::new(start as u32, self.pos as u32),
                name,
            });
            let triple = self.peek(1) == b'"' && self.peek(2) == b'"';
            self.pos += if triple { 3 } else { 1 };
            self.lex_interp_parts(triple);
            return;
        }
        if s == "_" {
            self.push(Tok::Underscore, start, self.pos);
            return;
        }
        if s == "$" && self.peek(0) == b'{' {
            self.push(Tok::Splice, start, self.pos);
            return;
        }
        if let Some(kw) = keyword(s) {
            if kw == Tok::KwDef {
                self.extension_line = false;
            }
            self.push(kw, start, self.pos);
            return;
        }
        let name = self.interner.intern(s);
        self.tokens.push(Token {
            kind: Tok::Ident,
            span: Span::new(start as u32, self.pos as u32),
            name,
        });
    }

    fn lex_op(&mut self) {
        let start = self.pos;
        while is_op_char(self.peek(0)) {
            if self.peek(0) == b'/' && matches!(self.peek(1), b'/' | b'*') {
                break;
            }
            self.pos += 1;
        }
        let s = &self.text[start..self.pos];
        let kind = match s {
            "=" => Tok::Eq,
            "=>" => Tok::Arrow,
            "?=>" => Tok::CtxArrow,
            "=>>" => Tok::TypeLambdaArrow,
            "<-" => Tok::LArrow,
            "<:" => Tok::Subtype,
            ">:" => Tok::Supertype,
            "@" => Tok::At,
            ":" => {
                if self.at_line_end(self.pos) {
                    Tok::ColonEol
                } else {
                    Tok::Colon
                }
            }
            _ => Tok::OpIdent,
        };
        if kind == Tok::OpIdent {
            let name = self.interner.intern(s);
            self.tokens.push(Token {
                kind,
                span: Span::new(start as u32, self.pos as u32),
                name,
            });
        } else {
            self.push(kind, start, self.pos);
        }
    }

    fn lex_number(&mut self) {
        let start = self.pos;
        let mut kind = Tok::IntLit;
        if self.peek(0) == b'0' && matches!(self.peek(1), b'x' | b'X') {
            self.pos += 2;
            while self.peek(0).is_ascii_hexdigit() || self.peek(0) == b'_' {
                self.pos += 1;
            }
        } else {
            while self.peek(0).is_ascii_digit() || self.peek(0) == b'_' {
                self.pos += 1;
            }
            if self.peek(0) == b'.' && self.peek(1).is_ascii_digit() {
                kind = Tok::DoubleLit;
                self.pos += 1;
                while self.peek(0).is_ascii_digit() || self.peek(0) == b'_' {
                    self.pos += 1;
                }
            }
            if matches!(self.peek(0), b'e' | b'E')
                && (self.peek(1).is_ascii_digit()
                    || (matches!(self.peek(1), b'+' | b'-') && self.peek(2).is_ascii_digit()))
            {
                kind = Tok::DoubleLit;
                self.pos += 2;
                while self.peek(0).is_ascii_digit() {
                    self.pos += 1;
                }
            }
        }
        let end = self.pos;
        match self.peek(0) {
            b'L' | b'l' if kind == Tok::IntLit => {
                kind = Tok::LongLit;
                self.pos += 1;
            }
            b'd' | b'D' if !is_ident_part(self.peek(1)) => {
                kind = Tok::DoubleLit;
                self.pos += 1;
            }
            b'f' | b'F' if !is_ident_part(self.peek(1)) => {
                kind = Tok::FloatLit;
                self.pos += 1;
            }
            _ => {}
        }
        // The span excludes the suffix so the parser can read the digits directly.
        self.push(kind, start, end);
    }

    /// What a `'` opens when it is no character literal: a quoted block or type (`'{`, `'[`,
    /// whose next character is not the closing `'` of a literal), or a quoted identifier
    /// (`'x`, `'xs`), with the length of the identifier.
    fn quote_kind(&self, p: usize) -> Option<Tok> {
        let next = *self.src.get(p + 1).unwrap_or(&0);
        if matches!(next, b'{' | b'[') {
            return (self.src.get(p + 2) != Some(&b'\'')).then_some(Tok::Quote);
        }
        if is_ident_start(next) && next != b'$' {
            let mut e = p + 2;
            while e < self.src.len() && is_ident_part(self.src[e]) {
                e += 1;
            }
            return (self.src.get(e) != Some(&b'\'')).then_some(Tok::QuoteId);
        }
        None
    }

    fn lex_char(&mut self) {
        let start = self.pos;
        match self.quote_kind(start) {
            Some(Tok::Quote) => {
                self.pos += 1;
                self.push(Tok::Quote, start, self.pos);
                return;
            }
            Some(Tok::QuoteId) => {
                self.pos += 1;
                let id_start = self.pos;
                while is_ident_part(self.peek(0)) {
                    self.pos += 1;
                }
                let name = self.interner.intern(&self.text[id_start..self.pos]);
                self.tokens.push(Token { kind: Tok::QuoteId, span: Span::new(start as u32, self.pos as u32), name });
                return;
            }
            _ => {}
        }
        self.pos += 1;
        if self.peek(0) == b'\\' {
            self.pos += 2;
            if self.src[self.pos - 1] == b'u' {
                while self.peek(0).is_ascii_hexdigit() {
                    self.pos += 1;
                }
            }
        } else {
            let len = self.text[self.pos..].chars().next().map_or(1, |c| c.len_utf8());
            self.pos += len;
        }
        if self.peek(0) == b'\'' {
            self.pos += 1;
        } else {
            self.error(start, self.pos, "unterminated character literal");
        }
        self.push(Tok::CharLit, start, self.pos);
    }

    fn lex_string(&mut self) {
        let start = self.pos;
        if self.peek(1) == b'"' && self.peek(2) == b'"' {
            self.pos += 3;
            loop {
                if self.pos >= self.src.len() {
                    self.error(start, start + 3, "unclosed multi-line string literal");
                    break;
                }
                if self.peek(0) == b'"' && self.peek(1) == b'"' && self.peek(2) == b'"' {
                    self.pos += 3;
                    while self.peek(0) == b'"' {
                        self.pos += 1;
                    }
                    break;
                }
                self.pos += 1;
            }
        } else {
            self.pos += 1;
            loop {
                match self.peek(0) {
                    b'"' => {
                        self.pos += 1;
                        break;
                    }
                    b'\\' => self.pos += 2,
                    b'\n' | 0 => {
                        self.error(start, self.pos, "unclosed string literal");
                        break;
                    }
                    _ => self.pos += 1,
                }
            }
        }
        self.push(Tok::StringLit, start, self.pos);
    }

    /// Lexes string parts up to the closing quote or the next `${`, where lexing resumes through
    /// the InterpBrace region once the matching `}` is reached.
    fn lex_interp_parts(&mut self, triple: bool) {
        let mut part_start = self.pos;
        loop {
            if self.pos >= self.src.len() {
                self.error(part_start, self.pos, if triple { "unclosed multi-line string literal" } else { "unclosed string literal" });
                self.push(Tok::StrPart, part_start, self.pos);
                self.push(Tok::InterpEnd, self.pos, self.pos);
                return;
            }
            let b = self.peek(0);
            if b == b'"' {
                let closes = if triple {
                    self.peek(1) == b'"' && self.peek(2) == b'"'
                } else {
                    true
                };
                if closes {
                    let mut end = self.pos;
                    if triple {
                        while self.src.get(end + 3) == Some(&b'"') {
                            end += 1;
                        }
                    }
                    self.push(Tok::StrPart, part_start, end);
                    self.pos = end + if triple { 3 } else { 1 };
                    self.push(Tok::InterpEnd, end, self.pos);
                    return;
                }
                self.pos += 1;
            } else if b == b'\\' && !triple {
                // Only `\"` and `\\` are taken as one: `\$k` is a backslash and an interpolation.
                self.pos += if matches!(self.peek(1), b'"' | b'\\') { 2 } else { 1 };
            } else if b == b'\n' && !triple {
                self.error(part_start, self.pos, "unclosed string literal");
                self.push(Tok::StrPart, part_start, self.pos);
                self.push(Tok::InterpEnd, self.pos, self.pos);
                return;
            } else if b == b'$' {
                let n = self.peek(1);
                if n == b'$' {
                    self.pos += 2;
                } else if n == b'{' {
                    self.push(Tok::StrPart, part_start, self.pos);
                    self.push(Tok::LBrace, self.pos + 1, self.pos + 2);
                    self.pos += 2;
                    self.regions.push(Region::InterpBrace { triple });
                    return;
                } else if n.is_ascii_alphabetic() || n == b'_' {
                    self.push(Tok::StrPart, part_start, self.pos);
                    self.pos += 1;
                    let id_start = self.pos;
                    while self.peek(0).is_ascii_alphanumeric() || self.peek(0) == b'_' {
                        self.pos += 1;
                    }
                    let s = &self.text[id_start..self.pos];
                    if s == "this" {
                        self.push(Tok::KwThis, id_start, self.pos);
                    } else {
                        let name = self.interner.intern(s);
                        self.tokens.push(Token {
                            kind: Tok::Ident,
                            span: Span::new(id_start as u32, self.pos as u32),
                            name,
                        });
                    }
                    part_start = self.pos;
                } else {
                    self.pos += 1;
                }
            } else {
                self.pos += 1;
            }
        }
    }
}

/// A character outside ASCII that starts no token: white space, or punctuation of the Latin-1
/// supplement and of the General Punctuation block (`§`, `¶`, `«`, a dash or a quote), which
/// scalac reports as an illegal character; letters, digits and symbols start identifiers.
fn illegal_start(c: char) -> bool {
    c.is_whitespace() || matches!(c as u32, 0xA1 | 0xA7 | 0xAB | 0xB6 | 0xB7 | 0xBB | 0xBF | 0x2000..=0x206F)
}

/// dotc's `closingRegionTokens`: a line starting with one is not too far left in a brace block.
fn closes_region(tok: Tok) -> bool {
    matches!(tok, Tok::RBrace | Tok::RParen | Tok::RBracket | Tok::KwCase) || statement_continues(tok)
}

/// dotc's `statCtdTokens`: a line starting with one continues the statement before it.
fn statement_continues(tok: Tok) -> bool {
    matches!(tok, Tok::KwThen | Tok::KwElse | Tok::KwDo | Tok::KwCatch | Tok::KwFinally | Tok::KwYield | Tok::KwMatch)
}
