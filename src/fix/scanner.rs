//! scalameta 4.17.4's scanner over a document's pieces (`ScannerTokens.nextToken`, read in order as
//! `LazyTokenIterator` reads it): the tokens its parser reads, the source's less trivia and the `Indent`, `Outdent` and
//! statement-separating `LF`, `LFLF` and `InfixLF` tokens the scanner makes, each at its `pointPos`, under the regions
//! (`SepRegion`) it keeps. Ported in scalameta's structure and names, each of its dialect's branches reading the
//! dialect it is given (`parse::Dialect`). The pieces hold a run of spaces as one token, which every count over spaces reads by its width, as
//! scalameta's `HSpace.len`.

use super::parse::chars::is_ident_symbolic_infix_operator;
use super::parse::Dialect;
use super::source::{Kind, Piece};
use crate::source::Span;
use crate::token::Tok;

/// scalameta's token classes as the scanner tells them apart; the last four only the scanner makes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum C {
    Bof,
    Eof,
    Space,
    Tab,
    Lf,
    Cr,
    Crlf,
    Ff,
    Comment,
    Shebang,
    Ident,
    KwAbstract,
    KwCase,
    KwCatch,
    KwClass,
    KwDef,
    KwDo,
    KwElse,
    KwEnum,
    KwExport,
    KwExtends,
    KwFalse,
    KwFinal,
    KwFinally,
    KwFor,
    KwForsome,
    KwGiven,
    KwIf,
    KwImplicit,
    KwImport,
    KwLazy,
    KwMatch,
    KwMacro,
    KwNew,
    KwNull,
    KwObject,
    KwOverride,
    KwPackage,
    KwPrivate,
    KwProtected,
    KwReturn,
    KwSealed,
    KwSuper,
    KwThen,
    KwThis,
    KwThrow,
    KwTrait,
    KwTrue,
    KwTry,
    KwType,
    KwVal,
    KwVar,
    KwWhile,
    KwWith,
    KwYield,
    Hash,
    Colon,
    Viewbound,
    LeftArrow,
    Subtype,
    Equals,
    RightArrow,
    Supertype,
    At,
    Underscore,
    TypeLambdaArrow,
    ContextArrow,
    MacroQuote,
    LeftParen,
    RightParen,
    Comma,
    Dot,
    Semicolon,
    LeftBracket,
    RightBracket,
    LeftBrace,
    RightBrace,
    Constant,
    InterpId,
    InterpStart,
    InterpPart,
    InterpSpliceStart,
    InterpSpliceEnd,
    InterpEnd,
    Indent,
    Outdent,
    LfLf,
    InfixLf,
}

impl C {
    pub(crate) fn hspace(self) -> bool {
        matches!(self, C::Space | C::Tab)
    }
    pub(crate) fn eol(self) -> bool {
        matches!(self, C::Lf | C::Cr | C::Crlf | C::Ff | C::InfixLf)
    }
    pub(crate) fn at_eol(self) -> bool {
        self.eol() || self == C::LfLf
    }
    pub(crate) fn at_eol_or_f(self) -> bool {
        self.at_eol() || matches!(self, C::Bof | C::Eof)
    }
    pub(crate) fn indentation(self) -> bool {
        matches!(self, C::Indent | C::Outdent)
    }
    pub(crate) fn whitespace(self) -> bool {
        self.hspace() || self.at_eol() || self.indentation()
    }
    pub(crate) fn trivia(self) -> bool {
        self.whitespace() || self == C::Comment
    }
    pub(crate) fn htrivia(self) -> bool {
        self.hspace() || self == C::Comment
    }
    pub(crate) fn close_delim(self) -> bool {
        matches!(self, C::RightParen | C::RightBracket | C::RightBrace)
    }
    pub(crate) fn literal(self) -> bool {
        matches!(self, C::Constant | C::KwTrue | C::KwFalse | C::KwNull)
    }
    pub(crate) fn modifier_keyword(self) -> bool {
        matches!(self, C::KwAbstract | C::KwFinal | C::KwImplicit | C::KwLazy | C::KwOverride | C::KwPrivate | C::KwProtected | C::KwSealed)
    }
    pub(crate) fn is_class_or_object(self) -> bool {
        matches!(self, C::KwClass | C::KwObject)
    }
}

/// A token of scalameta's token array: its class and offsets.
#[derive(Clone, Copy)]
pub(crate) struct T {
    pub(crate) c: C,
    pub(crate) start: u32,
    pub(crate) end: u32,
}

/// A token a `TokenRef` holds: the array's token at an index, or one the scanner makes (at the start of its
/// `pointPos` token). Two are the same token only where both are the array's at one index.
#[derive(Clone, Copy)]
pub(crate) struct RefTok {
    pub(crate) c: C,
    pub(crate) real: Option<usize>,
}

/// scalameta's `SepRegion`s. An indentation's and a brace's region carries the number it was made with, so that it is
/// the same region where the scanner conses it again (`x :: RegionCaseMark :: rs => expr() :: x :: rs`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum R {
    Indent(i32, u32),
    Line(i32),
    Paren(i32),
    Bracket(i32),
    Brace(i32, u32),
    CaseMark,
    CasePat(i32),
    CaseGuard(i32),
    CaseBody(i32, usize),
    ColonMaybeFewerBraces,
    DefMark,
    DefType,
    TemplateMark,
    NewMark,
    TemplateInherit,
    GivenDecl(usize),
    TemplateBody,
    ExtensionMark,
    WhileCond,
    WhileMaybeCond,
    WhileMaybeBody,
    IfCond,
    IfMaybeCond,
    IfMaybeBody,
    Then,
    ForMaybeParens,
    ForParens,
    ForBraces,
    ForOther,
    Try,
}

impl R {
    pub(crate) fn indent(self) -> i32 {
        match self {
            R::Indent(i, _) | R::Line(i) | R::Paren(i) | R::Bracket(i) | R::Brace(i, _) | R::CasePat(i) | R::CaseGuard(i) | R::CaseBody(i, _) => i,
            _ => -1,
        }
    }
    pub(crate) fn is_indented(self) -> bool {
        matches!(self, R::Indent(..))
    }
    pub(crate) fn can_produce_lf(self) -> bool {
        matches!(self, R::Indent(..) | R::Line(_) | R::Brace(..) | R::CaseBody(..) | R::ColonMaybeFewerBraces | R::TemplateMark | R::NewMark | R::GivenDecl(_) | R::Try) || self.control()
    }
    pub(crate) fn delim(self) -> bool {
        matches!(self, R::Indent(..) | R::Paren(_) | R::Bracket(_) | R::Brace(..))
    }
    pub(crate) fn delim_non_indented(self) -> bool {
        matches!(self, R::Paren(_) | R::Bracket(_) | R::Brace(..))
    }
    pub(crate) fn non_delim_non_indented(self) -> bool {
        matches!(
            self,
            R::Line(_) | R::CaseMark | R::CasePat(_) | R::CaseGuard(_) | R::ColonMaybeFewerBraces | R::DefMark | R::DefType | R::TemplateMark | R::NewMark | R::TemplateInherit | R::GivenDecl(_) | R::TemplateBody | R::ExtensionMark
        ) || self.control()
    }
    pub(crate) fn case_expr(self) -> bool {
        matches!(self, R::CasePat(_) | R::CaseGuard(_))
    }
    pub(crate) fn has_param_mark(self) -> bool {
        matches!(self, R::DefMark | R::ExtensionMark | R::TemplateMark | R::GivenDecl(_))
    }
    pub(crate) fn def_decl(self) -> bool {
        matches!(self, R::DefMark | R::DefType)
    }
    pub(crate) fn template_decl(self) -> bool {
        matches!(self, R::TemplateMark | R::NewMark | R::TemplateInherit)
    }
    pub(crate) fn template_mark(self) -> bool {
        matches!(self, R::TemplateMark | R::NewMark)
    }
    pub(crate) fn region_while(self) -> bool {
        matches!(self, R::WhileCond | R::WhileMaybeCond | R::WhileMaybeBody)
    }
    pub(crate) fn region_if(self) -> bool {
        matches!(self, R::IfCond | R::IfMaybeCond | R::IfMaybeBody)
    }
    pub(crate) fn region_for(self) -> bool {
        matches!(self, R::ForMaybeParens | R::ForParens | R::ForBraces | R::ForOther)
    }
    pub(crate) fn control(self) -> bool {
        self.region_while() || self.region_if() || self.region_for() || self == R::Then
    }
    pub(crate) fn maybe_cond(self) -> bool {
        matches!(self, R::WhileMaybeCond | R::IfMaybeCond)
    }
    pub(crate) fn is_control_keyword(self, t: C) -> bool {
        if self.region_while() {
            t == C::KwWhile
        } else if self.region_if() {
            t == C::KwIf
        } else if self.region_for() {
            t == C::KwFor
        } else {
            self == R::Then && t == C::KwThen
        }
    }
    pub(crate) fn is_terminating_token(self, t: C) -> bool {
        match self {
            R::IfCond => t == C::KwThen,
            R::IfMaybeCond | R::IfMaybeBody => matches!(t, C::KwThen | C::KwElse),
            R::Then => t == C::KwElse,
            r if r.region_while() => t == C::KwDo,
            r if r.region_for() => matches!(t, C::KwDo | C::KwYield),
            _ => false,
        }
    }
    pub(crate) fn is_terminating_token_required(self) -> bool {
        matches!(self, R::WhileCond | R::IfCond | R::ForOther)
    }
    pub(crate) fn is_not_terminating_token_if_optional(self, t: C) -> bool {
        !self.is_terminating_token_required() && !self.is_terminating_token(t)
    }
    pub(crate) fn is_closing_condition_token(self, t: C) -> bool {
        match self {
            R::ForParens => t == C::RightParen,
            R::ForBraces => t == C::RightBrace,
            _ => false,
        }
    }
    pub(crate) fn as_cond(self) -> R {
        if self == R::WhileMaybeCond { R::WhileCond } else { R::IfCond }
    }
    pub(crate) fn as_cond_or_body(self) -> R {
        if self == R::WhileMaybeCond { R::WhileMaybeBody } else { R::IfMaybeBody }
    }
    pub(crate) fn as_body(self) -> Option<R> {
        (self == R::IfMaybeCond).then_some(R::Then)
    }
    pub(crate) fn region_while_for(next: C) -> R {
        if next == C::LeftParen { R::WhileMaybeCond } else { R::WhileCond }
    }
    pub(crate) fn region_if_for(next: C) -> R {
        if next == C::LeftParen { R::IfMaybeCond } else { R::IfCond }
    }
    pub(crate) fn region_for_for(next: C) -> R {
        match next {
            C::LeftParen => R::ForMaybeParens,
            C::LeftBrace => R::ForBraces,
            _ => R::ForOther,
        }
    }
}

/// A list of regions, a cell of `Scanner::lists`; 0 is `Nil`.
pub(crate) type L = u32;
pub(crate) const NIL: L = 0;

#[derive(Clone, Copy)]
pub(crate) struct TokenRef {
    pub(crate) regions: L,
    pub(crate) token: RefTok,
    pos: i32,
    next_pos: i32,
    pub(crate) point_pos: i32,
    next: Option<usize>,
}

#[derive(Clone, Copy)]
struct OutdentInfo {
    outdent: Option<i32>,
    regions: L,
    done: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LeadingInfix {
    No,
    Yes,
    InvalidArg,
}

/// `nextToken`'s locals over one token: `prev`, `curr`, `next` and the next token's indentation.
#[derive(Clone, Copy)]
struct At {
    prev_token: RefTok,
    prev_pos: i32,
    curr_pos: i32,
    curr_non_trivial: bool,
    next_pos: i32,
    next_indent: i32,
    indent_pos: i32,
}

pub(crate) struct Scanner<'a> {
    pub(crate) text: &'a str,
    pub(crate) tokens: Vec<T>,
    pub(crate) lists: Vec<(R, L)>,
    pub(crate) refs: Vec<TokenRef>,
    regions_made: u32,
    pub(crate) dialect: Dialect,
    /// The `InfixLF`s of an invalid leading infix operator (`InfixLF.invalid`), by their `TokenRef`.
    pub(crate) invalid_infix: Vec<usize>,
}

/// The corpus's line of a document's stream under `dialects.Scala3` (`#!regions`, tests/fix/support/RegionStream.scala):
/// each token the scanner makes, by a letter (`I` an `Indent`, `O` an `Outdent`, `L` a line break, `F` an `LFLF`, `X` an
/// `InfixLF`), its start in `units` and the widths of the indented regions it stands in, the innermost first.
#[cfg(test)]
pub(crate) fn region_line(text: &str, pieces: &[Piece], units: impl Fn(u32) -> u32) -> String {
    let (s, order) = scanner(text, pieces, Dialect::SCALA3);
    let shown: Vec<String> = order
        .iter()
        .filter_map(|&r| {
            let tr = s.refs[r];
            let letter = match tr.token.c {
                C::Indent => 'I',
                C::Outdent => 'O',
                C::LfLf => 'F',
                C::InfixLf => 'X',
                c if c.at_eol() => 'L',
                _ => return None,
            };
            let at = s.tokens.get(tr.point_pos.max(0) as usize).map_or(text.len() as u32, |t| t.start);
            let widths: Vec<String> = s.widths(tr.regions).iter().map(|w| w.to_string()).collect();
            Some(format!("{}{}/{}", letter, units(at), widths.join(".")))
        })
        .collect();
    shown.join(" ")
}

/// The scanner over a document's `pieces` under `dialect` and the tokens its parser reads in order (each a
/// `TokenRef` of it).
pub(crate) fn scanner<'a>(text: &'a str, pieces: &[Piece], dialect: Dialect) -> (Scanner<'a>, Vec<usize>) {
    let mut tokens = vec![T { c: C::Bof, start: 0, end: 0 }];
    for p in pieces {
        for (c, span) in classes(text, *p, &dialect) {
            tokens.push(T { c, start: span.start, end: span.end });
        }
    }
    let len = text.len() as u32;
    tokens.push(T { c: C::Eof, start: len, end: len });
    let tokens = with_splice_ends(tokens);
    let mut s = Scanner { text, tokens, lists: vec![(R::CaseMark, NIL)], refs: Vec::new(), regions_made: 0, dialect, invalid_infix: Vec::new() };
    let order = s.read();
    (s, order)
}

/// The interpolations' tokens scalameta's tokenizer makes and the pieces do not hold: an empty `SpliceEnd` after a
/// splice's identifier or its block's `}`, and an empty `Interpolation.Part` where none is written after the start or
/// a splice's end.
fn with_splice_ends(tokens: Vec<T>) -> Vec<T> {
    let empty = |c, at: u32| T { c, start: at, end: at };
    let mut out = Vec::with_capacity(tokens.len());
    let (mut depth, mut blocks) = (0i32, Vec::new());
    let mut i = 0;
    while i < tokens.len() {
        let t = tokens[i];
        out.push(t);
        let mut end_at = None;
        match t.c {
            C::InterpStart if tokens.get(i + 1).map(|n| n.c) != Some(C::InterpPart) => out.push(empty(C::InterpPart, t.end)),
            C::InterpSpliceStart if tokens.get(i + 1).map(|n| n.c) == Some(C::LeftBrace) => blocks.push(depth),
            C::InterpSpliceStart => {
                i += 1;
                out.push(tokens[i]);
                end_at = Some(tokens[i].end);
            }
            C::LeftBrace => depth += 1,
            C::RightBrace => {
                depth -= 1;
                if blocks.last() == Some(&depth) {
                    blocks.pop();
                    end_at = Some(t.end);
                }
            }
            _ => {}
        }
        if let Some(at) = end_at {
            out.push(empty(C::InterpSpliceEnd, at));
            if tokens.get(i + 1).map(|n| n.c) != Some(C::InterpPart) {
                out.push(empty(C::InterpPart, at));
            }
        }
        i += 1;
    }
    out
}

/// A piece's scalameta tokens: its class by kind and text (a symbolic keyword the lexer reads as an operator), a
/// keyword `dialect` does not have an identifier (`LegacyTokenData.setIdentifier`).
fn classes(text: &str, p: Piece, dialect: &Dialect) -> Vec<(C, Span)> {
    let kw = |enabled: bool, c: C| if enabled { c } else { C::Ident };
    let written = &text[p.span.start as usize..p.span.end as usize];
    let c = match p.kind {
        Kind::Space => C::Space,
        Kind::Tab => C::Tab,
        Kind::LF => C::Lf,
        Kind::CRLF => C::Crlf,
        Kind::CR => C::Cr,
        Kind::FF => C::Ff,
        Kind::Comment => C::Comment,
        Kind::End => C::Ident,
        Kind::Shebang => C::Shebang,
        Kind::Bom => return Vec::new(),
        // Its quotes `Interpolation.Start`, its `$` a `SpliceStart` (`"$` both).
        Kind::InterpDelim => {
            let quote = written.trim_end_matches('$').len() as u32;
            let mut out = Vec::new();
            if quote > 0 {
                out.push((C::InterpStart, Span::new(p.span.start, p.span.start + quote)));
            }
            if written.ends_with('$') {
                out.push((C::InterpSpliceStart, Span::new(p.span.end - 1, p.span.end)));
            }
            return out;
        }
        Kind::Tok(t) => match t {
            // Words and symbols the tokenizer reads as keywords whatever the dialect.
            Tok::Ident | Tok::OpIdent | Tok::QuoteId => match written {
                "_" => C::Underscore,
                "#" => C::Hash,
                "<%" => C::Viewbound,
                "macro" => C::KwMacro,
                "forSome" => C::KwForsome,
                _ => C::Ident,
            },
            Tok::IntLit | Tok::LongLit | Tok::DoubleLit | Tok::FloatLit | Tok::CharLit | Tok::StringLit => C::Constant,
            Tok::InterpStart => C::InterpId,
            Tok::StrPart => C::InterpPart,
            Tok::InterpEnd => C::InterpEnd,
            Tok::Quote => C::MacroQuote,
            Tok::Splice => C::Ident,
            Tok::LParen => C::LeftParen,
            Tok::RParen => C::RightParen,
            Tok::LBracket => C::LeftBracket,
            Tok::RBracket => C::RightBracket,
            Tok::LBrace => C::LeftBrace,
            Tok::RBrace => C::RightBrace,
            Tok::Comma => C::Comma,
            Tok::Semi => C::Semicolon,
            Tok::Dot => C::Dot,
            Tok::Colon | Tok::ColonEol => C::Colon,
            Tok::Eq => C::Equals,
            Tok::Arrow => C::RightArrow,
            Tok::CtxArrow => kw(dialect.allow_given_using, C::ContextArrow),
            Tok::LArrow => C::LeftArrow,
            Tok::Subtype => C::Subtype,
            Tok::Supertype => C::Supertype,
            Tok::At => C::At,
            Tok::Underscore => C::Underscore,
            Tok::TypeLambdaArrow => kw(dialect.allow_type_lambdas, C::TypeLambdaArrow),
            Tok::Eof | Tok::EndLambda | Tok::Newline | Tok::Indent | Tok::Outdent => return Vec::new(),
            Tok::KwAbstract => C::KwAbstract,
            Tok::KwCase => C::KwCase,
            Tok::KwClass => C::KwClass,
            Tok::KwDef => C::KwDef,
            Tok::KwDo => C::KwDo,
            Tok::KwElse => C::KwElse,
            Tok::KwEnum => kw(dialect.allow_enums, C::KwEnum),
            Tok::KwExtends => C::KwExtends,
            Tok::KwFalse => C::KwFalse,
            Tok::KwFinal => C::KwFinal,
            Tok::KwFor => C::KwFor,
            Tok::KwGiven => kw(dialect.allow_given_using, C::KwGiven),
            Tok::KwIf => C::KwIf,
            Tok::KwImport => C::KwImport,
            Tok::KwLazy => C::KwLazy,
            Tok::KwMatch => C::KwMatch,
            Tok::KwNew => C::KwNew,
            Tok::KwNull => C::KwNull,
            Tok::KwObject => C::KwObject,
            Tok::KwOverride => C::KwOverride,
            Tok::KwPackage => C::KwPackage,
            Tok::KwPrivate => C::KwPrivate,
            Tok::KwProtected => C::KwProtected,
            Tok::KwReturn => C::KwReturn,
            Tok::KwSealed => C::KwSealed,
            Tok::KwSuper => C::KwSuper,
            Tok::KwThen => kw(dialect.allow_quiet_syntax, C::KwThen),
            Tok::KwThis => C::KwThis,
            Tok::KwThrow => C::KwThrow,
            Tok::KwTrait => C::KwTrait,
            Tok::KwTrue => C::KwTrue,
            Tok::KwTry => C::KwTry,
            Tok::KwCatch => C::KwCatch,
            Tok::KwFinally => C::KwFinally,
            Tok::KwType => C::KwType,
            Tok::KwVal => C::KwVal,
            Tok::KwVar => C::KwVar,
            Tok::KwWhile => C::KwWhile,
            Tok::KwWith => C::KwWith,
            Tok::KwYield => C::KwYield,
            Tok::KwExport => kw(dialect.allow_export_clause, C::KwExport),
            Tok::KwImplicit => C::KwImplicit,
        },
    };
    vec![(c, p.span)]
}

impl<'a> Scanner<'a> {
    pub(crate) fn text_of(&self, i: i32) -> &'a str {
        let t = self.tokens[i as usize];
        &self.text[t.start as usize..t.end as usize]
    }
    pub(crate) fn c(&self, i: i32) -> C {
        self.tokens[i as usize].c
    }
    pub(crate) fn is_ident(&self, i: i32, name: &str) -> bool {
        self.c(i) == C::Ident && self.text_of(i) == name
    }
    fn real(&self, i: i32) -> RefTok {
        RefTok { c: self.c(i), real: Some(i as usize) }
    }

    fn cons(&mut self, r: R, tail: L) -> L {
        self.lists.push((r, tail));
        (self.lists.len() - 1) as L
    }
    pub(crate) fn head(&self, l: L) -> Option<R> {
        (l != NIL).then(|| self.lists[l as usize].0)
    }
    pub(crate) fn tail(&self, l: L) -> L {
        self.lists[l as usize].1
    }
    pub(crate) fn second(&self, l: L) -> Option<R> {
        if l == NIL { None } else { self.head(self.tail(l)) }
    }
    fn new_indent(&mut self, indent: i32) -> R {
        self.regions_made += 1;
        R::Indent(indent, self.regions_made)
    }
    fn new_brace(&mut self, indent: i32) -> R {
        self.regions_made += 1;
        R::Brace(indent, self.regions_made)
    }

    fn skip_before(&self, index: i32, p: impl Fn(C) -> bool) -> i32 {
        if index <= 0 {
            return 0;
        }
        let mut i = index - 1;
        while i > 0 && p(self.c(i)) {
            i -= 1;
        }
        i
    }
    fn skip_after(&self, index: i32, p: impl Fn(C) -> bool) -> i32 {
        let max = self.tokens.len() as i32 - 1;
        if index >= max {
            return max;
        }
        let mut i = index + 1;
        while i < max && p(self.c(i)) {
            i += 1;
        }
        i
    }
    pub(crate) fn get_prev_index(&self, index: i32) -> i32 {
        self.skip_before(index, C::trivia)
    }
    pub(crate) fn get_next_index(&self, index: i32) -> i32 {
        self.skip_after(index, C::trivia)
    }
    pub(crate) fn get_strict_prev(&self, index: i32) -> i32 {
        self.skip_before(index, C::htrivia)
    }
    pub(crate) fn get_strict_next(&self, index: i32) -> i32 {
        self.skip_after(index, C::htrivia)
    }
    pub(crate) fn is_preceded_by_nl(&self, index: i32) -> bool {
        self.c(self.get_strict_prev(index)).at_eol_or_f()
    }
    pub(crate) fn is_followed_by_nl(&self, index: i32) -> bool {
        self.c(self.get_strict_next(index)).at_eol_or_f()
    }

    /// `isClassOrObjectOrEnum`: an enum's case's name an identifier under `allowEnums`.
    pub(crate) fn is_class_or_object_or_enum(&self, c: C) -> bool {
        c.is_class_or_object() || c == C::Ident && self.dialect.allow_enums
    }
    pub(crate) fn is_end_marker_identifier(&self, i: i32) -> bool {
        self.dialect.allow_end_marker && self.is_ident(i, "end")
    }
    pub(crate) fn is_end_marker_specifier(&self, i: i32) -> bool {
        matches!(self.c(i), C::Ident | C::KwIf | C::KwWhile | C::KwFor | C::KwMatch | C::KwTry | C::KwNew | C::KwThis | C::KwGiven | C::KwVal)
    }
    pub(crate) fn is_end_marker_intro(&self, index: i32) -> bool {
        self.is_end_marker_identifier(index) && {
            let next = self.get_strict_next(index);
            self.is_end_marker_specifier(next) && self.is_followed_by_nl(next)
        }
    }
    pub(crate) fn is_expr_intro(&self, index: i32) -> bool {
        self.is_expr_intro_of(self.c(index), index)
    }
    /// `isExprIntro(token, index)`: the token's class `c` (the parser's current token, which may be one the scanner
    /// made), its soft modifier and end marker read at `index`.
    pub(crate) fn is_expr_intro_of(&self, c: C, index: i32) -> bool {
        self.is_expr_intro_impl(c, index, || !self.is_soft_modifier(index) && !self.is_end_marker_intro(index))
    }
    /// `isIdentOrExprIntro(token)`.
    pub(crate) fn is_ident_or_expr_intro(&self, c: C, index: i32) -> bool {
        self.is_expr_intro_impl(c, index, || true)
    }
    fn is_expr_intro_impl(&self, c: C, index: i32, ident_ok: impl Fn() -> bool) -> bool {
        match c {
            C::Ident => ident_ok() || self.dialect.allow_splice_and_quote && ident_value(self.text_of(index)).starts_with('$'),
            C::LeftBracket => self.dialect.allow_polymorphic_functions,
            c => c.literal() || matches!(c, C::InterpId | C::KwDo | C::KwFor | C::KwIf | C::KwNew | C::KwReturn | C::KwSuper | C::KwThis | C::KwThrow | C::KwTry | C::KwWhile | C::LeftParen | C::LeftBrace | C::Underscore | C::MacroQuote | C::Indent),
        }
    }
    pub(crate) fn is_soft_modifier(&self, index: i32) -> bool {
        let next_is_dcl_intro_or_modifier_or = |f: &dyn Fn(C) -> bool| {
            let next = self.get_next_index(index);
            self.is_dcl_intro(next) || self.is_modifier(next) || f(self.c(next))
        };
        let next_is_class_or_object_or_opaque_type = || {
            let next = self.get_next_index(index);
            matches!(self.c(next), C::KwClass | C::KwType | C::KwTrait | C::KwEnum) || self.is_modifier(next)
        };
        if self.c(index) != C::Ident {
            return false;
        }
        let d = &self.dialect;
        match self.text_of(index) {
            "transparent" if d.allow_inline_mods => next_is_dcl_intro_or_modifier_or(&|c| matches!(c, C::KwTrait | C::KwClass)),
            "opaque" if d.allow_opaque_types => next_is_dcl_intro_or_modifier_or(&|_| false),
            "into" if d.allow_parameter_type_conversions => next_is_class_or_object_or_opaque_type(),
            "inline" if d.allow_inline_mods => next_is_dcl_intro_or_modifier_or(&matches_after_inline_match_mod),
            "open" if d.allow_open_class => self.is_def_intro(self.get_next_index(index)),
            "infix" if d.allow_infix_mods => self.is_def_intro(self.get_next_index(index)),
            "erased" if d.allow_erased_defs => self.is_def_intro(self.get_next_index(index)),
            "tracked" if d.allow_tracked_parameters => self.is_def_intro(self.get_next_index(index)),
            _ => false,
        }
    }
    /// `isInlineMatchMod(index)`: `inline` before what a match's scrutinee begins with.
    pub(crate) fn is_inline_match_mod(&self, index: i32) -> bool {
        self.dialect.allow_inline_mods && self.is_ident(index, "inline") && matches_after_inline_match_mod(self.c(self.get_next_index(index)))
    }
    pub(crate) fn is_def_intro(&self, index: i32) -> bool {
        match self.c(index) {
            C::At => true,
            C::KwCase => self.is_class_or_object_or_enum(self.c(self.get_next_index(index))),
            _ => self.is_dcl_intro(index) || self.is_modifier(index) || self.is_template_intro(index),
        }
    }
    pub(crate) fn is_template_intro(&self, index: i32) -> bool {
        match self.c(index) {
            C::At | C::KwClass | C::KwObject | C::KwTrait => true,
            C::KwCase => self.is_class_or_object_or_enum(self.c(self.get_next_index(index))),
            _ => self.is_modifier(index),
        }
    }
    pub(crate) fn is_dcl_intro(&self, index: i32) -> bool {
        match self.c(index) {
            C::KwDef | C::KwType | C::KwEnum | C::KwVal | C::KwVar | C::KwGiven => true,
            _ => self.is_kw_extension(index),
        }
    }
    pub(crate) fn is_kw_extension(&self, index: i32) -> bool {
        self.dialect.allow_extension_methods && self.is_ident(index, "extension") && matches!(self.c(self.get_next_index(index)), C::LeftParen | C::LeftBracket)
    }
    pub(crate) fn is_modifier(&self, index: i32) -> bool {
        self.c(index).modifier_keyword() || self.is_soft_modifier(index)
    }
    pub(crate) fn might_start_stat(&self, c: C, close_delim_ok: bool) -> bool {
        match c {
            C::KwCatch | C::KwElse | C::KwExtends | C::KwFinally | C::KwForsome | C::KwMatch | C::KwWith | C::KwYield | C::Comma | C::Colon | C::Dot | C::Equals | C::Semicolon | C::Hash | C::RightArrow | C::LeftArrow | C::Subtype | C::Supertype | C::Viewbound => false,
            c if c.at_eol_or_f() => false,
            c if c.close_delim() => close_delim_ok,
            _ => true,
        }
    }
    pub(crate) fn can_end_stat(&self, c: C) -> bool {
        c.literal() || matches!(c, C::Ident | C::KwGiven | C::InterpEnd | C::KwReturn | C::KwThis | C::KwType | C::RightParen | C::RightBracket | C::RightBrace | C::Underscore)
    }
    fn can_be_leading_infix_arg(&self, index: i32) -> bool {
        self.is_expr_intro(index)
            && match self.c(index) {
                C::Ident => {
                    let text = self.text_of(index);
                    matches!(ident_value(text), "-" | "+" | "~" | "!") || !is_ident_symbolic_infix_operator(text)
                }
                _ => true,
            }
    }
    pub(crate) fn is_symbolic_infix_operator(&self, index: i32) -> bool {
        self.c(index) == C::Ident && is_ident_symbolic_infix_operator(self.text_of(index))
    }

    /// `countIndentAndNewlineIndex`: the token's line's indentation and its line break's index, or -1 and -1.
    fn count_indent_and_newline_index(&self, token_position: i32) -> (i32, i32) {
        if token_position < 0 || self.c(token_position).whitespace() {
            return (-1, -1);
        }
        let (mut pos, mut acc) = (token_position - 1, 0);
        loop {
            if pos < 0 {
                return (acc, pos);
            }
            let t = self.tokens[pos as usize];
            match t.c {
                c if c.at_eol() || c == C::Bof => return (acc, pos),
                C::Comment => {
                    if self.is_multiline(pos) {
                        return (self.multiline_comment_indent(pos), pos);
                    }
                    acc = 0;
                }
                c if c.hspace() => acc += (t.end - t.start) as i32,
                _ => return (-1, -1),
            }
            pos -= 1;
        }
    }
    pub(crate) fn count_indent(&self, token_position: i32) -> i32 {
        self.count_indent_and_newline_index(token_position).0
    }
    fn is_multiline(&self, i: i32) -> bool {
        self.text_of(i).contains('\n')
    }
    /// `multilineCommentIndent`: the spaces before the first character of the last line of the comment's value.
    fn multiline_comment_indent(&self, i: i32) -> i32 {
        let written = self.text_of(i);
        let value = if let Some(v) = written.strip_prefix("//") { v } else { written.strip_prefix("/*").and_then(|v| v.strip_suffix("*/")).unwrap_or(written) };
        let bytes = value.as_bytes();
        let (mut idx, mut indent) = (bytes.len() as i32 - 1, 0);
        loop {
            if idx <= 0 {
                return -1;
            }
            match bytes[idx as usize] {
                b'\n' | b'\r' => return indent,
                b' ' | b'\t' => indent += 1,
                _ => indent = 0,
            }
            idx -= 1;
        }
    }

    /// `findOutdentPos`.
    fn find_outdent_pos(&self, prev_pos: i32, curr_pos: i32, outdent: i32, ok_blank: bool) -> i32 {
        let iter_pos = 1 + prev_pos;
        if iter_pos < curr_pos {
            let (mut i, mut pos, mut indent, mut num_eol) = (iter_pos, prev_pos, if self.c(prev_pos).at_eol() { 0 } else { -1 }, 0);
            loop {
                if i >= curr_pos {
                    return if pos < curr_pos { pos } else { curr_pos - 1 };
                }
                let t = self.tokens[i as usize];
                match t.c {
                    c if c.at_eol() => {
                        pos = i;
                        indent = 0;
                        num_eol += 1;
                    }
                    c if c.hspace() && indent >= 0 => indent += (t.end - t.start) as i32,
                    c if c.whitespace() => {}
                    C::Comment if indent < 0 || outdent < indent || outdent == indent && (ok_blank || num_eol < 2) => {
                        pos = i + 1;
                        indent = -1;
                        num_eol = 0;
                    }
                    _ => return pos,
                }
                i += 1;
            }
        } else if self.c(curr_pos) == C::Eof {
            curr_pos
        } else {
            prev_pos
        }
    }

    fn find_indent(&self, mut l: L) -> i32 {
        while let Some(r) = self.head(l) {
            if r.indent() >= 0 {
                return r.indent();
            }
            l = self.tail(l);
        }
        0
    }
    fn is_indented(&self, l: L, cur_indent: i32) -> bool {
        self.find_indent(l) < cur_indent
    }
    fn in_parens(&self, mut l: L) -> bool {
        while let Some(r) = self.head(l) {
            if r.delim_non_indented() {
                return matches!(r, R::Paren(_));
            }
            l = self.tail(l);
        }
        false
    }
    fn drop_region_line(&self, mut l: L) -> L {
        while let Some(R::Line(_)) = self.head(l) {
            l = self.tail(l);
        }
        l
    }
    fn drop_region_line_at(&self, indent: i32, mut l: L) -> L {
        while let Some(R::Line(r)) = self.head(l) {
            if r < indent {
                break;
            }
            l = self.tail(l);
        }
        l
    }
    fn drop_while_not_try(&self, mut l: L) -> L {
        while let Some(r) = self.head(l) {
            if r == R::Try {
                break;
            }
            l = self.tail(l);
        }
        l
    }
    fn drop_until_try(&self, mut l: L) -> L {
        while let Some(r) = self.head(l) {
            l = self.tail(l);
            if r == R::Try {
                return l;
            }
        }
        NIL
    }

    fn push(&mut self, r: TokenRef) -> usize {
        self.refs.push(r);
        self.refs.len() - 1
    }
    /// `TokenRef(regions, token, pos, next)`.
    fn ref_at(&mut self, regions: L, token: RefTok, pos: i32, next: Option<usize>) -> usize {
        self.push(TokenRef { regions, token, pos, next_pos: pos + 1, point_pos: pos, next })
    }

    /// `nextToken(ref)`.
    fn next_ref(&mut self, r: usize) -> usize {
        if let Some(n) = self.refs[r].next {
            return n;
        }
        let pos = self.refs[r].next_pos;
        let next = if (pos as usize) < self.tokens.len() {
            let TokenRef { token, pos: prev_pos, regions, .. } = self.refs[r];
            self.next_token(token, prev_pos, pos, regions)
        } else {
            let n = self.push(TokenRef { regions: NIL, token: RefTok { c: C::Eof, real: None }, pos, next_pos: pos, point_pos: pos, next: None });
            self.refs[n].next = Some(n);
            n
        };
        self.refs[r].next = Some(next);
        next
    }

    /// The tokens the parser reads, from `BOF` (`LazyTokenIterator`) to `EOF`.
    fn read(&mut self) -> Vec<usize> {
        let bof = self.real(0);
        let mut curr = self.ref_at(NIL, bof, 0, None);
        let mut order = vec![curr];
        while self.refs[curr].token.c != C::Eof {
            curr = self.next_ref(curr);
            order.push(curr);
        }
        order
    }

    /// `nextToken(prevToken, prevPos, currPos, sepRegionsOrig)`, its tail calls a loop.
    fn next_token(&mut self, mut prev_token: RefTok, mut prev_pos: i32, mut curr_pos: i32, mut sep_regions_orig: L) -> usize {
        loop {
            let curr = self.c(curr_pos);
            let curr_non_trivial = !curr.trivia();
            let next_pos = (curr_pos as usize + 1..self.tokens.len()).find(|&i| !self.tokens[i].c.trivia()).map_or(-1, |i| i as i32);
            let (next_indent, indent_pos) = self.count_indent_and_newline_index(next_pos);
            let at = At { prev_token, prev_pos, curr_pos, curr_non_trivial, next_pos, next_indent, indent_pos };
            if next_pos < 0 {
                let rs = self.get_non_trivial_regions(&at, sep_regions_orig);
                return self.get_at_eof(&at, rs);
            }
            if curr_non_trivial {
                // `isTrailingComma`: relaxed, the closing delimiter on any line.
                if self.dialect.allow_trailing_commas && curr == C::Comma && self.c(next_pos).close_delim() {
                    prev_token = self.real(curr_pos);
                    prev_pos = curr_pos;
                    curr_pos += 1;
                    continue;
                }
                let rs = self.get_non_trivial_regions(&at, sep_regions_orig);
                return self.non_trivial(&at, rs);
            }
            match self.on_whitespace(&at, sep_regions_orig) {
                Some(Ok(r)) => return r,
                Some(Err(rs)) => sep_regions_orig = rs,
                None => {}
            }
            curr_pos = next_pos;
        }
    }

    fn mk_indent(&mut self, at: &At, pos: i32, point_pos: i32, rs: L, next: Option<usize>) -> usize {
        let token = RefTok { c: C::Indent, real: None };
        self.push(TokenRef { regions: rs, token, pos, next_pos: at.next_pos, point_pos, next })
    }
    fn mk_outdent_at(&mut self, at: &At, outdent: i32, regions: L) -> usize {
        let max_point_pos = if at.next_pos < 0 || at.curr_non_trivial { at.curr_pos } else { at.next_pos };
        let point_pos = self.find_outdent_pos(at.prev_pos, max_point_pos, outdent, self.is_indented(regions, outdent));
        let (next_prev_pos, next_curr_pos) = if point_pos > at.curr_pos { (at.curr_pos, point_pos) } else { (at.prev_pos, at.curr_pos) };
        let token = RefTok { c: C::Outdent, real: None };
        self.push(TokenRef { regions, token, pos: next_prev_pos, next_pos: next_curr_pos, point_pos, next: None })
    }
    fn curr_ref(&mut self, at: &At, regions: L) -> usize {
        self.curr_and_next_ref(at, regions, None)
    }
    fn curr_and_next_ref(&mut self, at: &At, regions: L, next: Option<usize>) -> usize {
        let token = self.real(at.curr_pos);
        self.ref_at(regions, token, at.curr_pos, next)
    }
    fn eol_ref_for(&mut self, at: &At, rs: L, out: RefTok, eol_pos: i32) -> usize {
        self.push(TokenRef { regions: rs, token: out, pos: eol_pos, next_pos: at.next_pos, point_pos: eol_pos, next: None })
    }
    fn eof_ref_at(&mut self, at: &At, rs: L, eol_pos: i32, multi_eol: bool) -> usize {
        let token = self.c(eol_pos);
        let out = if !multi_eol || token == C::LfLf { self.real(eol_pos) } else { RefTok { c: C::LfLf, real: None } };
        self.eol_ref_for(at, rs, out, eol_pos)
    }
    fn outdent_then_curr_ref(&mut self, at: &At, ri: i32, rs: L, rn: Option<R>) -> usize {
        let r = self.mk_outdent_at(at, ri, rs);
        let regions = match rn {
            Some(x) => self.cons(x, rs),
            None => rs,
        };
        let c = self.curr_ref(at, regions);
        self.refs[r].next = Some(c);
        r
    }

    /// `mkOutdentsOpt`: an `Outdent` for each indented region `f` closes, chained, the current token after them where
    /// it is a token, else the line break where a blank line separates what may start a statement.
    fn mk_outdents_opt(&mut self, at: &At, xs: L, multi_eol: bool, f: &dyn Fn(&Scanner, &At, L) -> Option<OutdentInfo>) -> Result<(usize, usize), L> {
        let (mut xs, mut was_done, mut head, mut last) = (xs, false, None::<usize>, None::<usize>);
        loop {
            if !was_done {
                match f(self, at, xs) {
                    None => was_done = true,
                    Some(OutdentInfo { outdent, regions: rs, done }) => {
                        let is_done = done || rs == xs;
                        if let Some(o) = outdent {
                            let tr = self.mk_outdent_at(at, o, rs);
                            match last {
                                None => head = Some(tr),
                                Some(l) => self.refs[l].next = Some(tr),
                            }
                            last = Some(tr);
                        }
                        xs = rs;
                        was_done = is_done;
                    }
                }
                continue;
            }
            let (Some(head), Some(last)) = (head, last) else { return Err(xs) };
            if at.curr_non_trivial {
                let c = self.curr_ref(at, xs);
                self.refs[last].next = Some(c);
            } else if multi_eol && self.might_start_stat(self.c(at.next_pos), false) {
                let point = self.refs[last].point_pos;
                let e = self.eof_ref_at(at, xs, point, true);
                self.refs[last].next = Some(e);
            }
            return Ok((head, last));
        }
    }
    fn mk_outdents(&mut self, at: &At, xs: L, f: &dyn Fn(&Scanner, &At, L) -> Option<OutdentInfo>) -> usize {
        match self.mk_outdents_opt(at, xs, false, f) {
            Ok((head, _)) => head,
            Err(xs) => self.curr_ref(at, xs),
        }
    }

    fn get_case_intro(&mut self, at: &At, sep_regions: L) -> usize {
        let next = self.c(at.next_pos);
        match next {
            // `case` after `catch` or `match` on its line: an indent before it, relative to the region around.
            C::KwCase if self.dialect.allow_significant_indentation && !self.text[self.tokens[at.curr_pos as usize].end as usize..self.tokens[at.next_pos as usize].start as usize].contains('\n') => {
                let regions = self.cons(R::CaseMark, sep_regions);
                let indent = self.find_indent(sep_regions) + self.get_extra_indent(sep_regions);
                let ri = self.new_indent(indent);
                let next_regions = self.cons(ri, regions);
                let indent_ref = self.mk_indent(at, at.curr_pos, at.next_pos, next_regions, None);
                self.curr_and_next_ref(at, regions, Some(indent_ref))
            }
            C::KwCase | C::LeftBrace => {
                let marked = self.cons(R::CaseMark, sep_regions);
                self.curr_ref(at, marked)
            }
            _ => self.curr_ref(at, sep_regions),
        }
    }
    fn get_extra_indent(&self, mut rs: L) -> i32 {
        loop {
            match self.head(rs) {
                Some(R::CaseGuard(_)) => return 1,
                Some(r) if r.region_if() || matches!(r, R::Line(_)) => rs = self.tail(rs),
                _ => return 0,
            }
        }
    }
    fn get_template_inherit(&mut self, at: &At, sep_regions: L) -> usize {
        let regions = match self.head(sep_regions) {
            Some(r) if r.template_mark() => {
                let rs = self.tail(sep_regions);
                self.cons(R::TemplateInherit, rs)
            }
            _ => sep_regions,
        };
        self.curr_ref(at, regions)
    }
    fn is_prev_end_marker(&self, at: &At) -> bool {
        at.prev_pos > 0 && self.is_end_marker_identifier(at.prev_pos) && self.is_preceded_by_nl(at.prev_pos)
    }
    fn get_at_eof(&mut self, at: &At, sep_regions: L) -> usize {
        self.mk_outdents(at, sep_regions, &|s, _, xs| match s.head(xs) {
            Some(R::Indent(i, _)) => Some(OutdentInfo { outdent: Some(i), regions: s.tail(xs), done: false }),
            Some(_) => Some(OutdentInfo { outdent: None, regions: s.tail(xs), done: false }),
            None => None,
        })
    }

    /// `swapWithOutdents`: the `Outdent`s after the comma at `token_ref` before it.
    fn swap_with_outdents(&mut self, token_ref: usize) -> usize {
        let mut oref = token_ref;
        loop {
            let nref = self.next_ref(oref);
            if self.refs[nref].token.c == C::Outdent {
                oref = nref;
            } else if oref == token_ref {
                return token_ref;
            } else {
                self.refs[oref].next = Some(token_ref);
                let first = self.refs[token_ref].next.expect("a comma's next token is read before it is swapped");
                self.refs[token_ref].next = Some(nref);
                return first;
            }
        }
    }

    fn non_trivial(&mut self, at: &At, sep_regions: L) -> usize {
        let (prev, curr, next) = (if at.prev_pos >= 0 { Some(self.c(at.prev_pos)) } else { None }, self.c(at.curr_pos), self.c(at.next_pos));
        let prev_end_marker = self.is_prev_end_marker(at);
        match curr {
            C::Eof => self.get_at_eof(at, sep_regions),
            C::Comma => {
                if !self.in_parens(sep_regions) {
                    return self.curr_ref(at, sep_regions);
                }
                let f = |s: &Scanner, _: &At, xs: L| match s.head(xs) {
                    Some(R::Indent(i, _)) => Some(OutdentInfo { outdent: Some(i), regions: s.tail(xs), done: false }),
                    Some(r) if r.non_delim_non_indented() => Some(OutdentInfo { outdent: None, regions: s.tail(xs), done: false }),
                    _ => None,
                };
                match self.mk_outdents_opt(at, sep_regions, false, &f) {
                    Err(xs) => {
                        let c = self.curr_ref(at, xs);
                        self.swap_with_outdents(c)
                    }
                    Ok((head, last)) => {
                        let after = self.refs[last].next.expect("the comma after its outdents");
                        let swapped = self.swap_with_outdents(after);
                        self.refs[last].next = Some(swapped);
                        head
                    }
                }
            }
            C::KwEnum => {
                let rs = self.cons(R::TemplateMark, sep_regions);
                self.curr_ref(at, rs)
            }
            C::KwGiven if !matches!(at.prev_token.c, C::Dot | C::KwCase) => {
                let rs = self.drop_region_line(sep_regions);
                let regions = match self.head(rs) {
                    None | Some(R::Brace(..)) | Some(R::Indent(..)) => self.cons(R::GivenDecl(at.curr_pos as usize), rs),
                    _ => sep_regions,
                };
                self.curr_ref(at, regions)
            }
            C::KwWith => {
                let rs = self.drop_region_line(sep_regions);
                let regions = match self.head(rs) {
                    Some(R::GivenDecl(_)) => self.tail(rs),
                    _ => sep_regions,
                };
                self.curr_ref(at, regions)
            }
            C::KwPackage if self.dialect.allow_significant_indentation && next != C::KwObject => {
                let rs = self.cons(R::TemplateMark, sep_regions);
                self.curr_ref(at, rs)
            }
            C::KwObject | C::KwClass | C::KwTrait if self.dialect.allow_significant_indentation => {
                let rs = self.cons(R::TemplateMark, sep_regions);
                self.curr_ref(at, rs)
            }
            C::KwNew if self.dialect.allow_significant_indentation => {
                let rs = self.cons(R::NewMark, sep_regions);
                self.curr_ref(at, rs)
            }
            C::KwTry if !prev_end_marker => {
                let rs = self.drop_region_line(sep_regions);
                let rs = self.cons(R::Try, rs);
                self.curr_ref(at, rs)
            }
            C::KwMatch if !prev_end_marker => self.get_case_intro(at, sep_regions),
            C::KwCatch => {
                let rs = self.drop_while_not_try(sep_regions);
                self.get_case_intro(at, rs)
            }
            C::KwCase if !next.is_class_or_object() => {
                let expr = R::CasePat(self.count_indent(at.curr_pos));
                let d = self.drop_region_line(sep_regions);
                let regions = match (self.head(d), self.second(d)) {
                    // `case` after the body of a previous case
                    (Some(R::CaseBody(..)), _) => {
                        let rs = self.tail(d);
                        self.cons(expr, rs)
                    }
                    // the region of the cases, an indentation's or a brace's
                    (Some(x), Some(R::CaseMark)) => {
                        let rs = self.tail(self.tail(d));
                        let rs = self.cons(x, rs);
                        self.cons(expr, rs)
                    }
                    (Some(x), Some(y)) if x.delim() && (y.region_for() || y == R::TemplateBody) => sep_regions,
                    // a partial function
                    (Some(R::Brace(..)), _) => self.cons(expr, sep_regions),
                    // a partial function in an assignment or as fewer braces
                    (Some(R::Indent(..)), _)
                        if at.prev_token.c == C::Indent
                            && match prev {
                                Some(C::Equals | C::Colon) => true,
                                Some(C::RightArrow) => matches!(self.second(d), Some(R::CaseBody(..))),
                                _ => false,
                            } =>
                    {
                        self.cons(expr, sep_regions)
                    }
                    (None, _) if at.prev_pos == 0 => self.cons(expr, sep_regions),
                    _ => sep_regions,
                };
                self.curr_ref(at, regions)
            }
            C::KwFinally => {
                let d = self.drop_region_line(sep_regions);
                match (self.head(d), self.second(d)) {
                    // `finally` after a case of `catch` on its line
                    (Some(R::CaseBody(..)), Some(R::Indent(i, _))) => {
                        let rs = self.tail(self.tail(d));
                        let r = self.mk_outdent_at(at, i, rs);
                        let regions = self.drop_until_try(rs);
                        let c = self.curr_ref(at, regions);
                        self.refs[r].next = Some(c);
                        r
                    }
                    _ => {
                        let regions = self.drop_until_try(d);
                        self.curr_ref(at, regions)
                    }
                }
            }
            C::LeftBrace => {
                let lb = match self.head(sep_regions) {
                    Some(R::GivenDecl(kw)) if at.prev_token.real != Some(kw) => {
                        let rs = self.tail(sep_regions);
                        self.cons(R::TemplateBody, rs)
                    }
                    Some(r) if r.template_mark() => {
                        let rs = self.tail(sep_regions);
                        self.cons(R::TemplateBody, rs)
                    }
                    Some(R::TemplateInherit) if prev != Some(C::KwExtends) => {
                        let rs = self.tail(sep_regions);
                        self.cons(R::TemplateBody, rs)
                    }
                    _ => sep_regions,
                };
                let brace = self.new_brace(at.next_indent);
                let rs = self.cons(brace, lb);
                self.curr_ref(at, rs)
            }
            C::RightBrace => self.mk_outdents(at, sep_regions, &|s, _, xs| match (s.head(xs), s.second(xs)) {
                (Some(R::Indent(i, _)), _) => Some(OutdentInfo { outdent: Some(i), regions: s.tail(xs), done: false }),
                (Some(R::Brace(..)), Some(R::TemplateBody | R::CaseMark)) => Some(OutdentInfo { outdent: None, regions: s.tail(s.tail(xs)), done: true }),
                (Some(R::Brace(..)), _) => Some(OutdentInfo { outdent: None, regions: s.tail(xs), done: true }),
                (Some(_), _) => Some(OutdentInfo { outdent: None, regions: s.tail(xs), done: false }),
                (None, _) => None,
            }),
            C::LeftBracket => {
                let rs = self.cons(R::Bracket(at.next_indent), sep_regions);
                self.curr_ref(at, rs)
            }
            C::RightBracket => self.mk_outdents(at, sep_regions, &|s, _, xs| match s.head(xs) {
                Some(R::Indent(i, _)) => Some(OutdentInfo { outdent: Some(i), regions: s.tail(xs), done: false }),
                Some(R::Bracket(_)) => Some(OutdentInfo { outdent: None, regions: s.tail(xs), done: true }),
                Some(_) => Some(OutdentInfo { outdent: None, regions: s.tail(xs), done: false }),
                None => None,
            }),
            C::LeftParen => {
                let rs = self.cons(R::Paren(at.next_indent), sep_regions);
                self.curr_ref(at, rs)
            }
            C::RightParen => self.mk_outdents(at, sep_regions, &|s, _, xs| match s.head(xs) {
                Some(R::Indent(i, _)) => Some(OutdentInfo { outdent: Some(i), regions: s.tail(xs), done: false }),
                Some(R::Paren(_)) => Some(OutdentInfo { outdent: None, regions: s.tail(xs), done: true }),
                Some(_) => Some(OutdentInfo { outdent: None, regions: s.tail(xs), done: false }),
                None => None,
            }),
            C::RightArrow => {
                let regions = match self.head(sep_regions) {
                    // a case region for `match {`, for the indentation of its statements
                    Some(r) if r.case_expr() => {
                        let rs = self.tail(sep_regions);
                        let body_indent = match self.head(rs) {
                            Some(R::Indent(i, _)) => i,
                            _ => at.next_indent,
                        };
                        self.cons(R::CaseBody(body_indent, at.curr_pos as usize), rs)
                    }
                    Some(R::ColonMaybeFewerBraces) => self.tail(sep_regions),
                    _ => sep_regions,
                };
                self.curr_ref(at, regions)
            }
            C::KwFor if !prev_end_marker => {
                let rs = self.cons(R::region_for_for(next), sep_regions);
                self.curr_ref(at, rs)
            }
            C::KwWhile if self.dialect.allow_quiet_syntax && !prev_end_marker => {
                let rs = self.cons(R::region_while_for(next), sep_regions);
                self.curr_ref(at, rs)
            }
            C::KwIf if self.dialect.allow_quiet_syntax && !prev_end_marker => {
                let d = self.drop_region_line(sep_regions);
                let regions = match (self.head(d), self.second(d)) {
                    (Some(R::CasePat(i)), _) => {
                        let rs = self.tail(d);
                        self.cons(R::CaseGuard(i), rs)
                    }
                    (Some(r), _) if r.region_for() => d,
                    (Some(x), Some(y)) if x.delim() && y.region_for() => d,
                    _ => self.cons(R::region_if_for(next), sep_regions),
                };
                self.curr_ref(at, regions)
            }
            C::KwThen => {
                let d = self.drop_region_line(sep_regions);
                match (self.head(d), self.second(d)) {
                    (Some(R::Indent(i, _)), Some(y)) if y.region_if() => {
                        let rs = self.tail(self.tail(d));
                        self.outdent_then_curr_ref(at, i, rs, Some(R::Then))
                    }
                    (Some(r), _) if r.region_if() => {
                        let rs = self.tail(d);
                        let rs = self.cons(R::Then, rs);
                        self.curr_ref(at, rs)
                    }
                    _ => {
                        let rs = self.cons(R::Then, sep_regions);
                        self.curr_ref(at, rs)
                    }
                }
            }
            C::KwElse if self.dialect.allow_quiet_syntax => {
                let d = self.drop_region_line(sep_regions);
                match self.head(d) {
                    Some(r) if r.region_if() || r == R::Then => {
                        let rs = self.tail(d);
                        self.curr_ref(at, rs)
                    }
                    _ => self.curr_ref(at, sep_regions),
                }
            }
            C::KwDo | C::KwYield => {
                let d = self.drop_region_line(sep_regions);
                match (self.head(d), self.second(d)) {
                    (Some(R::Indent(i, _)), Some(y)) if y.region_while() || y.region_for() => {
                        let rs = self.tail(self.tail(d));
                        self.outdent_then_curr_ref(at, i, rs, None)
                    }
                    (Some(r), _) if r.region_while() || r.region_for() => {
                        let rs = self.tail(d);
                        self.curr_ref(at, rs)
                    }
                    _ => self.curr_ref(at, sep_regions),
                }
            }
            C::KwDef | C::KwVal | C::KwVar if self.dialect.allow_significant_indentation && !prev_end_marker => {
                let rs = self.cons(R::DefMark, sep_regions);
                self.curr_ref(at, rs)
            }
            C::Colon => {
                let regions = match self.head(sep_regions) {
                    Some(R::DefMark) => {
                        let rs = self.tail(sep_regions);
                        self.cons(R::DefType, rs)
                    }
                    // to tell `=>` of a lambda from the end of a case's expression
                    Some(R::CaseGuard(_)) if self.dialect.allow_fewer_braces => self.cons(R::ColonMaybeFewerBraces, sep_regions),
                    _ => sep_regions,
                };
                self.curr_ref(at, regions)
            }
            C::Equals => {
                let regions = match self.head(sep_regions) {
                    Some(r) if r.def_decl() || matches!(r, R::GivenDecl(_)) => self.tail(sep_regions),
                    _ => sep_regions,
                };
                self.curr_ref(at, regions)
            }
            C::KwExtends => self.get_template_inherit(at, sep_regions),
            C::Ident => match self.text_of(at.curr_pos) {
                "derives" if self.dialect.allow_derives => self.get_template_inherit(at, sep_regions),
                "extension"
                    if self.dialect.allow_extension_methods && matches!(at.prev_token.c, C::Bof | C::Indent | C::Outdent | C::LeftBrace | C::RightArrow | C::Semicolon) || at.prev_token.c.at_eol() =>
                {
                    if matches!(next, C::LeftParen | C::LeftBracket) {
                        let rs = self.cons(R::ExtensionMark, sep_regions);
                        self.curr_ref(at, rs)
                    } else {
                        self.curr_ref(at, sep_regions)
                    }
                }
                _ => self.curr_ref(at, sep_regions),
            },
            _ => self.curr_ref(at, sep_regions),
        }
    }

    fn get_non_trivial_regions(&mut self, at: &At, regions: L) -> L {
        let curr = self.c(at.curr_pos);
        let prev = if at.prev_pos >= 0 { Some(self.c(at.prev_pos)) } else { None };
        let d = self.drop_region_line(regions);
        match (self.head(d), self.second(d)) {
            (Some(R::ExtensionMark), _) => match curr {
                C::LeftBrace => {
                    let rs = self.tail(d);
                    self.cons(R::TemplateBody, rs)
                }
                C::LeftParen | C::LeftBracket => regions,
                _ => self.tail(d),
            },
            (Some(r), _) if r.control() && r.is_not_terminating_token_if_optional(curr) => {
                let rs = self.tail(d);
                if r.maybe_cond() && prev == Some(C::RightParen) {
                    match curr {
                        C::Dot | C::KwMatch => self.cons(r.as_cond(), rs),
                        // might continue the condition or start the body
                        C::Ident | C::LeftBrace | C::LeftBracket | C::LeftParen | C::Underscore if self.dialect.allow_quiet_syntax => self.cons(r.as_cond_or_body(), rs),
                        _ => match r.as_body() {
                            Some(b) => self.cons(b, rs),
                            None => rs,
                        },
                    }
                } else if r == R::ForMaybeParens && prev == Some(C::RightParen) {
                    if curr == C::LeftArrow { self.cons(R::ForOther, rs) } else { rs }
                } else if r.region_for() && prev.is_some_and(|p| r.is_closing_condition_token(p)) {
                    rs
                } else if at.prev_token.c.at_eol() || curr.close_delim() {
                    rs
                } else {
                    regions
                }
            }
            (Some(p @ R::Paren(_)), Some(R::ForMaybeParens)) if curr == C::LeftArrow => {
                let rs = self.tail(self.tail(d));
                let rs = self.cons(R::ForParens, rs);
                self.cons(p, rs)
            }
            _ => regions,
        }
    }

    /// `nextToken` at whitespace: the token it hands the parser (`Ok`), the regions it reads the next token under
    /// (`Err`), or none.
    fn on_whitespace(&mut self, at: &At, sep_regions_orig: L) -> Option<Result<usize, L>> {
        let prev = self.c(at.prev_pos);
        let next = self.c(at.next_pos);
        let find_first_eol = |s: &Scanner, mut pos: i32| loop {
            if pos > at.indent_pos {
                return -1;
            }
            if s.c(pos).at_eol() {
                return pos;
            }
            pos += 1;
        };
        let has_blank = |s: &Scanner, mut pos: i32| {
            let mut had_eol = false;
            loop {
                if pos > at.indent_pos {
                    return false;
                }
                match s.c(pos) {
                    C::LfLf => return true,
                    c if c.at_eol() => {
                        if had_eol {
                            return true;
                        }
                        had_eol = true;
                    }
                    c if c.whitespace() => {}
                    _ => had_eol = false,
                }
                pos += 1;
            }
        };
        let has_lf = at.indent_pos > at.prev_pos;
        let eol_pos = if has_lf { find_first_eol(self, at.prev_pos + 1) } else { -1 };
        let multi_eol = eol_pos >= 0 && has_blank(self, eol_pos);
        let w = Ws { prev, next, eol_pos, multi_eol, sep_regions_orig, can_be_leading_infix: self.can_be_leading_infix(at, eol_pos, multi_eol) };
        if !has_lf {
            return None;
        }
        if !self.dialect.allow_significant_indentation {
            return match self.get_infix_lf_if_needed(at, &w, sep_regions_orig) {
                Some(r) => Some(r),
                None => self.get_if_can_produce_lf(at, &w, sep_regions_orig, -1),
            };
        }
        self.iter(at, &w, sep_regions_orig)
    }

    fn can_be_leading_infix(&self, at: &At, eol_pos: i32, multi_eol: bool) -> LeadingInfix {
        if !multi_eol && eol_pos >= 0 && self.dialect.allow_infix_operator_after_nl && at.next_pos >= 0 && self.is_symbolic_infix_operator(at.next_pos) {
            self.is_leading_infix_arg(at.next_pos + 1, at.next_indent)
        } else {
            LeadingInfix::No
        }
    }

    fn last_whitespace_token(&mut self, at: &At, w: &Ws, rs: L, line_indent: i32) -> usize {
        let add_region_line = line_indent >= 0 && self.is_indented(rs, line_indent);
        let regions = if add_region_line { self.cons(R::Line(line_indent), rs) } else { rs };
        self.eof_ref_at(at, regions, w.eol_pos, w.multi_eol)
    }

    fn strip_if_can_produce_lf(&self, at: &At, w: &Ws, regions: L) -> Option<L> {
        let derives = |i: i32| self.dialect.allow_derives && self.is_ident(i, "derives");
        let blank_brace_or = |ok: bool| if w.next == C::LeftBrace { w.multi_eol } else { ok };
        let is_end_marker = || {
            self.is_end_marker_specifier(at.prev_pos) && {
                let prev_prev_pos = self.get_strict_prev(at.prev_pos);
                self.is_end_marker_identifier(prev_prev_pos) && self.is_preceded_by_nl(prev_prev_pos)
            }
        };
        let ok = w.eol_pos >= 0 && self.might_start_stat(w.next, true) && (self.can_end_stat(w.prev) || is_end_marker());
        if !ok {
            return None;
        }
        let mut rs = regions;
        loop {
            match self.head(rs) {
                Some(R::DefType) => {
                    if w.next == C::LeftParen {
                        return None;
                    }
                }
                Some(r) if r.template_mark() => {
                    if !blank_brace_or(!derives(at.next_pos)) {
                        return None;
                    }
                }
                Some(R::TemplateInherit) => {
                    if !blank_brace_or(!derives(at.next_pos) && !derives(at.prev_pos)) {
                        return None;
                    }
                }
                Some(R::Try) if !matches!(w.next, C::KwCatch | C::KwFinally) && w.can_be_leading_infix != LeadingInfix::Yes && !self.is_indented(self.tail(rs), at.next_indent) => {}
                Some(R::GivenDecl(_)) => {}
                None => return Some(rs),
                Some(r) if r.can_produce_lf() => return Some(rs),
                _ => return None,
            }
            rs = self.tail(rs);
        }
    }

    fn get_if_can_produce_lf(&mut self, at: &At, w: &Ws, regions: L, line_indent: i32) -> Option<Result<usize, L>> {
        let rs = self.strip_if_can_produce_lf(at, w, regions)?;
        Some(Ok(self.last_whitespace_token(at, w, rs, line_indent)))
    }

    fn get_infix_lf_if_needed(&mut self, at: &At, w: &Ws, regions: L) -> Option<Result<usize, L>> {
        let leading = match self.head(regions) {
            None => w.can_be_leading_infix,
            Some(r) if r.can_produce_lf() => w.can_be_leading_infix,
            _ => LeadingInfix::No,
        };
        let infix_lf = |s: &mut Scanner, invalid: bool| {
            let out = RefTok { c: C::InfixLf, real: None };
            let r = s.eol_ref_for(at, w.sep_regions_orig, out, at.indent_pos);
            if invalid {
                s.invalid_infix.push(r);
            }
            Some(Ok(r))
        };
        match leading {
            LeadingInfix::Yes => infix_lf(self, false),
            LeadingInfix::InvalidArg if self.head(w.sep_regions_orig).is_some_and(|x| x.indent() >= 0 && x.indent() < at.next_indent) => infix_lf(self, true),
            _ => None,
        }
    }

    fn no_outdent(&self, at: &At, mut l: L) -> bool {
        while let Some(r) = self.head(l) {
            if r.is_indented() {
                return r.indent() <= at.next_indent;
            }
            l = self.tail(l);
        }
        true
    }

    fn get_outdent_on_indent(&self, at: &At, w: &Ws, r: i32, rs: L, ctrl_only: bool) -> Option<OutdentInfo> {
        let d = self.drop_region_line_at(at.next_indent + 1, rs);
        let oi = |outdent, regions, done| Some(OutdentInfo { outdent: Some(outdent), regions, done });
        match self.head(d) {
            Some(R::Try) => {
                let xs = self.tail(d);
                if at.next_indent < r || at.next_indent == r && matches!(w.next, C::KwCatch | C::KwFinally) {
                    let done = self.no_outdent(at, xs);
                    oi(r, if done { rs } else { xs }, done)
                } else {
                    None
                }
            }
            _ if at.next_indent >= r => None,
            Some(rc) if rc.control() => oi(r, if rc.is_terminating_token(w.next) { rs } else { self.tail(d) }, false),
            _ if ctrl_only => None,
            Some(R::TemplateBody) => oi(r, self.tail(d), false),
            _ if match w.prev {
                C::KwElse | C::KwDo | C::KwFinally | C::KwYield => false,
                // a leading infix operator's
                _ => !self.is_indented(d, at.next_indent) || w.can_be_leading_infix != LeadingInfix::Yes,
            } =>
            {
                oi(r, d, false)
            }
            _ => None,
        }
    }

    fn get_outdent_info(&self, at: &At, w: &Ws, sep_regions: L) -> Option<OutdentInfo> {
        match (self.head(sep_regions), self.second(sep_regions)) {
            (Some(R::CaseBody(_, arrow)), Some(R::Indent(r, _))) => {
                let rs = self.tail(self.tail(sep_regions));
                if at.next_indent > r {
                    None
                } else if w.next == C::KwFinally {
                    Some(OutdentInfo { outdent: Some(r), regions: rs, done: self.no_outdent(at, rs) })
                } else if at.next_indent < r
                    // only where the region around is at the same indentation; the special treatment of case clauses
                    || self.find_indent(rs) == at.next_indent && (arrow as i32 != at.prev_pos && w.next != C::KwCase || self.c(self.get_next_index(at.next_pos)).is_class_or_object())
                {
                    Some(OutdentInfo { outdent: Some(r), regions: self.drop_region_line_at(at.next_indent, rs), done: false })
                } else {
                    self.get_outdent_on_indent(at, w, r, rs, true)
                }
            }
            (Some(R::Try), _) => {
                let rs = self.tail(sep_regions);
                if self.no_outdent(at, rs) { None } else { Some(OutdentInfo { outdent: None, regions: rs, done: false }) }
            }
            (Some(x), _)
                if x.non_delim_non_indented()
                    && match w.prev {
                        C::KwThen | C::KwCatch | C::KwMatch => false,
                        _ => !self.no_outdent(at, self.tail(sep_regions)),
                    } =>
            {
                Some(OutdentInfo { outdent: None, regions: self.tail(sep_regions), done: false })
            }
            (Some(R::Indent(r, _)), _) => self.get_outdent_on_indent(at, w, r, self.tail(sep_regions), false),
            _ => None,
        }
    }

    fn get_indent_if_needed(&mut self, at: &At, w: &Ws, sep_regions: L) -> Option<Result<usize, L>> {
        let exceeds_indent = self.is_indented(sep_regions, at.next_indent);
        let next = w.next;
        if at.next_indent < 0 || next == C::RightBrace {
            return None;
        }
        match w.prev {
            // `if` does not work with indentation in pattern matches
            C::KwIf if !self.head(sep_regions).is_some_and(R::region_if) => None,
            C::KwCatch | C::KwMatch => {
                // always an indent for an indented `match` block
                let ok = next == C::KwCase && self.head(sep_regions) == Some(R::CaseMark);
                if ok { self.emit_indent(at, sep_regions, None) } else { None }
            }
            C::Colon => {
                let could_be_fewer_braces = || {
                    let regions_ok = self.dialect.allow_fewer_braces
                        && match (self.head(sep_regions), self.second(sep_regions)) {
                            (Some(R::Bracket(_)), _) => false,
                            (Some(r), _) if r.has_param_mark() => false,
                            (Some(R::Paren(_)), Some(r)) if r.has_param_mark() => false,
                            _ => true,
                        };
                    let before = self.c(self.get_prev_index(at.prev_pos));
                    regions_ok && (matches!(before, C::Ident | C::KwThis) || before.close_delim())
                };
                match self.head(sep_regions) {
                    Some(r) if r.template_decl() => {
                        let rs = self.tail(sep_regions);
                        if exceeds_indent {
                            let rs = self.cons(R::TemplateBody, rs);
                            self.emit_indent(at, rs, None)
                        } else if self.is_end_marker_identifier(at.next_pos) {
                            let outdent = self.mk_outdent_at(at, at.next_indent, rs);
                            self.emit_indent(at, rs, Some(outdent))
                        } else {
                            Some(Ok(self.last_whitespace_token(at, w, rs, at.next_indent)))
                        }
                    }
                    Some(r) if r.def_decl() => None,
                    _ if !exceeds_indent => {
                        if !could_be_fewer_braces() {
                            None
                        } else {
                            Some(Ok(self.last_whitespace_token(at, w, sep_regions, at.next_indent)))
                        }
                    }
                    Some(R::GivenDecl(_)) => {
                        let rs = self.tail(sep_regions);
                        let rs = self.cons(R::TemplateBody, rs);
                        self.emit_indent(at, rs, None)
                    }
                    Some(R::ColonMaybeFewerBraces) => {
                        let rs = self.tail(sep_regions);
                        self.emit_indent(at, rs, None)
                    }
                    _ => match next {
                        // a refinement's declaration
                        C::KwVal | C::KwDef | C::KwType | C::Semicolon => self.emit_indent(at, sep_regions, None),
                        // a fewer-braces function (though it could be a self-type)
                        _ if could_be_fewer_braces() => self.emit_indent(at, sep_regions, None),
                        _ => None,
                    },
                }
            }
            _ if !exceeds_indent => None,
            C::RightArrow => {
                let d = self.drop_region_line(sep_regions);
                match (self.head(d), self.second(d)) {
                    (Some(R::GivenDecl(_)), _) => None,
                    (Some(R::CaseBody(_, arrow)), Some(R::Brace(..))) if arrow as i32 == at.prev_pos => None,
                    _ if self.is_end_marker_intro(at.next_pos) => None,
                    _ => self.emit_indent(at, sep_regions, None),
                }
            }
            C::KwTry => self.emit_indent(at, sep_regions, None),
            prev => {
                let d = self.drop_region_line(sep_regions);
                match self.head(d) {
                    Some(r) if r.region_for() && r.is_closing_condition_token(prev) => {
                        let ko = r.is_terminating_token(next);
                        if ko {
                            None
                        } else {
                            let rs = self.tail(d);
                            self.emit_indent(at, rs, None)
                        }
                    }
                    Some(R::ForMaybeParens) if prev == C::RightParen => {
                        let ko = next == C::LeftArrow || R::ForMaybeParens.is_terminating_token(next);
                        if ko {
                            None
                        } else {
                            let rs = self.tail(d);
                            self.emit_indent(at, rs, None)
                        }
                    }
                    Some(x) if x.maybe_cond() && prev == C::RightParen => {
                        let ko = next == C::Dot || !x.is_not_terminating_token_if_optional(next) || w.can_be_leading_infix == LeadingInfix::Yes;
                        if ko {
                            None
                        } else {
                            let rs = self.tail(d);
                            let rs = match x.as_body() {
                                Some(b) => self.cons(b, rs),
                                None => rs,
                            };
                            self.emit_indent(at, rs, None)
                        }
                    }
                    Some(x) if x.maybe_cond() && x.is_control_keyword(prev) => {
                        let rs = self.tail(d);
                        let rs = self.cons(x.as_cond(), rs);
                        self.emit_indent(at, rs, None)
                    }
                    // skip this line break
                    Some(r) if r.template_decl() && next == C::LeftParen => Some(Err(sep_regions)),
                    Some(R::ExtensionMark) if prev == C::RightParen && next != C::LeftParen => {
                        let rs = self.tail(d);
                        let rs = self.cons(R::TemplateBody, rs);
                        self.emit_indent(at, rs, None)
                    }
                    _ => {
                        let ok = match prev {
                            C::KwYield | C::KwFinally | C::KwDo | C::KwFor | C::KwThen | C::KwElse | C::KwWhile | C::KwIf | C::KwReturn | C::LeftArrow | C::ContextArrow => true,
                            C::Equals => next != C::KwMacro,
                            C::KwWith => self.is_def_intro(at.next_pos) || matches!(next, C::KwImport | C::KwExport),
                            _ => false,
                        };
                        if ok { self.emit_indent(at, sep_regions, None) } else { None }
                    }
                }
            }
        }
    }

    fn emit_indent(&mut self, at: &At, regions: L, next: Option<usize>) -> Option<Result<usize, L>> {
        let ri = self.new_indent(at.next_indent);
        let rs = self.cons(ri, regions);
        Some(Ok(self.mk_indent(at, at.prev_pos, at.indent_pos, rs, next)))
    }

    fn regions_to_res(&mut self, at: &At, w: &Ws, regions: L) -> Option<Result<usize, L>> {
        if at.prev_token.c.indentation() {
            return None;
        }
        let ws = *w;
        let f = move |s: &Scanner, a: &At, xs: L| s.get_outdent_info(a, &ws, xs);
        match self.mk_outdents_opt(at, regions, w.multi_eol, &f) {
            Ok((head, _)) => Some(Ok(head)),
            Err(xs) => self.get_indent_if_needed(at, w, xs),
        }
    }

    fn only_without_lf(&mut self, at: &At, w: &Ws) -> Option<Result<usize, L>> {
        match self.head(w.sep_regions_orig) {
            Some(ri) if ri.is_indented() && ri.indent() < at.next_indent => Some(Err(self.cons(R::Line(at.next_indent), w.sep_regions_orig))),
            _ => None,
        }
    }

    fn iter(&mut self, at: &At, w: &Ws, mut regions: L) -> Option<Result<usize, L>> {
        loop {
            let res = match self.regions_to_res(at, w, regions) {
                Some(r) => Some(r),
                None => self.get_infix_lf_if_needed(at, w, regions),
            };
            if res.is_some() {
                return res;
            }
            match self.head(regions) {
                None if w.prev == C::Bof => return Some(Err(self.cons(R::Line(at.next_indent), NIL))),
                Some(R::Line(r)) if r >= at.next_indent => regions = self.tail(regions),
                Some(r) if r.control() && !r.is_control_keyword(w.prev) && r.is_not_terminating_token_if_optional(w.next) => {
                    let rs = self.tail(regions);
                    if r.maybe_cond() && w.prev == C::RightParen {
                        if w.next == C::Dot {
                            return None;
                        }
                        match r.as_body() {
                            Some(x) => {
                                let xs = self.cons(x, rs);
                                return match self.get_if_can_produce_lf(at, w, xs, at.next_indent) {
                                    Some(r) => Some(r),
                                    None => self.only_without_lf(at, w),
                                };
                            }
                            None => regions = rs,
                        }
                    } else {
                        regions = rs;
                    }
                }
                _ => match self.strip_if_can_produce_lf(at, w, regions) {
                    Some(xs) => {
                        if xs == regions {
                            return Some(Ok(self.last_whitespace_token(at, w, xs, at.next_indent)));
                        }
                        regions = xs;
                    }
                    None => return self.only_without_lf(at, w),
                },
            }
        }
    }

    /// `isLeadingInfixArg`.
    fn is_leading_infix_arg(&self, after_op_pos: i32, next_indent: i32) -> LeadingInfix {
        let (mut pos, mut indent, mut prev_no_nl) = match self.c(after_op_pos) {
            c if c.eol() => (after_op_pos + 1, 0, false),
            c if c.hspace() => (after_op_pos + 1, -1, true),
            C::Comment => return LeadingInfix::InvalidArg,
            _ => return LeadingInfix::No,
        };
        loop {
            let t = self.tokens[pos as usize];
            match t.c {
                c if c.eol() => {
                    if !prev_no_nl {
                        return LeadingInfix::No;
                    }
                    indent = 0;
                    prev_no_nl = false;
                }
                c if c.hspace() => {
                    if !prev_no_nl {
                        indent += (t.end - t.start) as i32;
                    }
                }
                C::Comment => {
                    let comment_indent = self.multiline_comment_indent(pos);
                    if comment_indent >= 0 {
                        indent = comment_indent;
                    }
                    prev_no_nl = true;
                }
                _ => {
                    return if !self.can_be_leading_infix_arg(pos) {
                        LeadingInfix::No
                    } else if indent >= 0 && indent < next_indent {
                        LeadingInfix::InvalidArg
                    } else {
                        LeadingInfix::Yes
                    };
                }
            }
            pos += 1;
        }
    }
    #[cfg(test)]
    fn widths(&self, mut l: L) -> Vec<i32> {
        let mut widths = Vec::new();
        while let Some(x) = self.head(l) {
            if let R::Indent(w, _) = x {
                widths.push(w);
            }
            l = self.tail(l);
        }
        widths
    }


}

/// `nextToken`'s locals over whitespace, as its closures read them.
#[derive(Clone, Copy)]
struct Ws {
    prev: C,
    next: C,
    eol_pos: i32,
    multi_eol: bool,
    sep_regions_orig: L,
    can_be_leading_infix: LeadingInfix,
}

fn matches_after_inline_match_mod(c: C) -> bool {
    c.literal() || matches!(c, C::LeftParen | C::LeftBrace | C::KwNew | C::Ident | C::InterpId | C::KwSuper | C::KwThis | C::MacroQuote)
}

/// An identifier's value: its text, a backquoted one's without the backquotes.
fn ident_value(text: &str) -> &str {
    text.strip_prefix('`').and_then(|t| t.strip_suffix('`')).unwrap_or(text)
}
