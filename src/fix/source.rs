//! What a rule reads of a file: its editable token stream, every byte of the file in one token or another, made
//! from what the lexer kept under `Syntax::keep` (`ast::SourceKept`), never from a second reading of the text.

use crate::ast::SourceKept;
use crate::source::Span;
use crate::token::Tok;

/// A token of the editable stream: one the lexer made, or trivia the lexer skipped (scalameta's `Token.Space`,
/// `Tab`, `LF`, `CR`, `FF`, `Comment`), or the two words of an end marker, which the lexer records apart.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Tok(Tok),
    /// A run of spaces.
    Space,
    Tab,
    LF,
    CRLF,
    CR,
    FF,
    Comment,
    /// The `end` of an end marker (its name is the name's own token: an `Ident`, or the keyword's of `end while`).
    End,
    /// A script's header, its `#!` line through its `!#` line where it has one, which the lexer skips
    /// (`directives::script_header`).
    Shebang,
    /// A byte order mark.
    Bom,
    /// What delimits an interpolated string's parts and splices but its id and its parts (the lexer's tokens):
    /// its quotes and each `$` or `${` and `}` (scalameta's `Interpolation.Start`, `SpliceStart`, `SpliceEnd`,
    /// `End`).
    InterpDelim,
}

macro_rules! token_kinds {
    ($($code:ident = $name:literal),* $(,)?) => {
        #[allow(clippy::upper_case_acronyms)]
        #[derive(Clone, Copy)]
        #[repr(u16)]
        enum Code {
            $($code),*
        }
        /// The token kinds the API names (`Token.kind`), by code: scalameta's token classes as the bridge names them
        /// (docs/FIX.md, "Token"), each once. A document's codes are handed to the rule program once
        /// (`Native.tokenCodes`), the names once a run (`Native.tokenKindNames`).
        pub const TOKEN_KINDS: &[&str] = &[$($name),*];
    };
}

token_kinds!(
    Space = "Space",
    Tab = "Tab",
    LF = "LF",
    CRLF = "CRLF",
    CR = "CR",
    FF = "FF",
    Comment = "Comment",
    Ident = "Ident",
    Shebang = "Shebang",
    Bom = "BOM",
    InterpDelim = "Interpolation.Delimiter",
    Semicolon = "Semicolon",
    Comma = "Comma",
    Dot = "Dot",
    LeftParen = "LeftParen",
    RightParen = "RightParen",
    LeftBracket = "LeftBracket",
    RightBracket = "RightBracket",
    LeftBrace = "LeftBrace",
    RightBrace = "RightBrace",
    Colon = "Colon",
    Equals = "Equals",
    RightArrow = "RightArrow",
    ContextArrow = "ContextArrow",
    LeftArrow = "LeftArrow",
    Subtype = "Subtype",
    Supertype = "Supertype",
    At = "At",
    Underscore = "Underscore",
    ConstantInt = "Constant.Int",
    ConstantLong = "Constant.Long",
    ConstantDouble = "Constant.Double",
    ConstantFloat = "Constant.Float",
    ConstantChar = "Constant.Char",
    ConstantString = "Constant.String",
    InterpolationId = "Interpolation.Id",
    InterpolationPart = "Interpolation.Part",
    InterpolationEnd = "Interpolation.End",
    Quote = "Quote",
    QuoteId = "QuoteId",
    Splice = "Splice",
    TypeLambdaArrow = "TypeLambdaArrow",
    Eof = "Eof",
    EndLambda = "EndLambda",
    Newline = "Newline",
    Indent = "Indent",
    Outdent = "Outdent",
    KwAbstract = "KwAbstract",
    KwCase = "KwCase",
    KwClass = "KwClass",
    KwDef = "KwDef",
    KwDo = "KwDo",
    KwElse = "KwElse",
    KwEnum = "KwEnum",
    KwExtends = "KwExtends",
    KwFalse = "KwFalse",
    KwFinal = "KwFinal",
    KwFor = "KwFor",
    KwGiven = "KwGiven",
    KwIf = "KwIf",
    KwImport = "KwImport",
    KwLazy = "KwLazy",
    KwMatch = "KwMatch",
    KwNew = "KwNew",
    KwNull = "KwNull",
    KwObject = "KwObject",
    KwOverride = "KwOverride",
    KwPackage = "KwPackage",
    KwPrivate = "KwPrivate",
    KwProtected = "KwProtected",
    KwReturn = "KwReturn",
    KwSealed = "KwSealed",
    KwSuper = "KwSuper",
    KwThen = "KwThen",
    KwThis = "KwThis",
    KwThrow = "KwThrow",
    KwTrait = "KwTrait",
    KwTrue = "KwTrue",
    KwTry = "KwTry",
    KwCatch = "KwCatch",
    KwFinally = "KwFinally",
    KwType = "KwType",
    KwVal = "KwVal",
    KwVar = "KwVar",
    KwWhile = "KwWhile",
    KwWith = "KwWith",
    KwYield = "KwYield",
    KwExport = "KwExport",
    KwImplicit = "KwImplicit",
);

/// A token's kind's code, its index in `TOKEN_KINDS`: every `Tok` named, those that never reach the stream (`Eof`, the
/// layout) by their own names.
pub fn kind_code(k: Kind) -> u16 {
    use Code as C;
    (match k {
        Kind::Space => C::Space,
        Kind::Tab => C::Tab,
        Kind::LF => C::LF,
        Kind::CRLF => C::CRLF,
        Kind::CR => C::CR,
        Kind::FF => C::FF,
        Kind::Comment => C::Comment,
        Kind::End => C::Ident,
        Kind::Shebang => C::Shebang,
        Kind::Bom => C::Bom,
        Kind::InterpDelim => C::InterpDelim,
        Kind::Tok(t) => match t {
            Tok::Ident | Tok::OpIdent => C::Ident,
            Tok::IntLit => C::ConstantInt,
            Tok::LongLit => C::ConstantLong,
            Tok::DoubleLit => C::ConstantDouble,
            Tok::FloatLit => C::ConstantFloat,
            Tok::CharLit => C::ConstantChar,
            Tok::StringLit => C::ConstantString,
            Tok::InterpStart => C::InterpolationId,
            Tok::StrPart => C::InterpolationPart,
            Tok::InterpEnd => C::InterpolationEnd,
            Tok::Quote => C::Quote,
            Tok::QuoteId => C::QuoteId,
            Tok::Splice => C::Splice,
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
            Tok::CtxArrow => C::ContextArrow,
            Tok::LArrow => C::LeftArrow,
            Tok::Subtype => C::Subtype,
            Tok::Supertype => C::Supertype,
            Tok::At => C::At,
            Tok::Underscore => C::Underscore,
            Tok::TypeLambdaArrow => C::TypeLambdaArrow,
            Tok::Eof => C::Eof,
            Tok::EndLambda => C::EndLambda,
            Tok::Newline => C::Newline,
            Tok::Indent => C::Indent,
            Tok::Outdent => C::Outdent,
            Tok::KwAbstract => C::KwAbstract,
            Tok::KwCase => C::KwCase,
            Tok::KwClass => C::KwClass,
            Tok::KwDef => C::KwDef,
            Tok::KwDo => C::KwDo,
            Tok::KwElse => C::KwElse,
            Tok::KwEnum => C::KwEnum,
            Tok::KwExtends => C::KwExtends,
            Tok::KwFalse => C::KwFalse,
            Tok::KwFinal => C::KwFinal,
            Tok::KwFor => C::KwFor,
            Tok::KwGiven => C::KwGiven,
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
            Tok::KwThen => C::KwThen,
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
            Tok::KwExport => C::KwExport,
            Tok::KwImplicit => C::KwImplicit,
        },
    }) as u16
}

/// A token's kind as the API names it, scalameta's token class (`KwVar`, `Ident`, `Semicolon`, `Tab`, `Comment`).
pub fn kind_name(k: Kind) -> &'static str {
    TOKEN_KINDS[kind_code(k) as usize]
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Piece {
    pub kind: Kind,
    pub span: Span,
}

/// A kept token as the editable stream has it. The lexer reads braces as regions: the `{` of a template body is a
/// `ColonEol` followed by an `Indent` of the same span, a block's `{` and `}` an `Indent` and an `Outdent` at the
/// brace; these are the brace they stand at. The layout's other tokens hold no text (`None`), and a layout token
/// at the span of the token before it is that token's (`editable` keeps the first).
pub(super) fn editable_kind(text: &str, t: &crate::token::Token) -> Option<Tok> {
    if t.span.end <= t.span.start || matches!(t.kind, Tok::Eof | Tok::EndLambda) {
        return None;
    }
    match (t.kind, text.as_bytes()[t.span.start as usize]) {
        (Tok::Indent | Tok::Outdent | Tok::Newline | Tok::ColonEol, b'{') => Some(Tok::LBrace),
        (Tok::Indent | Tok::Outdent | Tok::Newline, b'}') => Some(Tok::RBrace),
        (Tok::Indent | Tok::Outdent | Tok::Newline, _) => None,
        (k, _) => Some(k),
    }
}

/// The kind scalameta's tokenizer gives a word it scans as a token of its own: its keyword's (`while`, `this`, `null`),
/// else `Ident`. An end marker's name and a quoted identifier's are such words (scalameta's `LegacyScanner` lexes the
/// name after `end` and after the `'` of `'x` as any token), where teq's lexer makes one token of `'x` and its parser
/// reads the end marker's name as a name.
fn word_kind(word: &str) -> Kind {
    Kind::Tok(crate::token::keyword(word).unwrap_or(Tok::Ident))
}

/// Every byte of `text` in order: the kept tokens and end markers, and the comments and trivia between them. A gap
/// holds only what the lexer skips; anything else there is a defect of the record, `Err` with its offset.
pub fn editable(text: &str, kept: &SourceKept) -> Result<Vec<Piece>, u32> {
    let mut marks: Vec<Piece> = Vec::with_capacity(kept.tokens.len() + kept.end_markers.len() * 2);
    for t in &kept.tokens {
        let Some(k) = editable_kind(text, t) else { continue };
        if k == Tok::QuoteId {
            // `'x`: the quote, then the name's own token, as scalameta's tokenizer scans them.
            let quote = Span::new(t.span.start, t.span.start + 1);
            marks.push(Piece { kind: Kind::Tok(Tok::Quote), span: quote });
            marks.push(Piece { kind: word_kind(&text[quote.end as usize..t.span.end as usize]), span: Span::new(quote.end, t.span.end) });
        } else {
            marks.push(Piece { kind: Kind::Tok(k), span: t.span });
        }
    }
    for &m in &kept.end_markers {
        let word = Span::new(m.start, m.start + 3);
        let name_start = text[word.end as usize..m.end as usize].find(|c: char| c != ' ').map_or(m.end, |i| word.end + i as u32);
        marks.push(Piece { kind: Kind::End, span: word });
        marks.push(Piece { kind: word_kind(&text[name_start as usize..m.end as usize]), span: Span::new(name_start, m.end) });
    }
    marks.sort_by_key(|p| p.span.start);
    marks.dedup_by_key(|p| p.span);
    let mut out = Vec::with_capacity(marks.len() * 2);
    let mut at = 0u32;
    let mut interpolating = 0u32;
    for p in marks {
        if p.span.start < at {
            return Err(p.span.start);
        }
        at = suffix(text, at, &mut out);
        gap(text, at, p.span.start, interpolating > 0, &mut out)?;
        match p.kind {
            Kind::Tok(Tok::InterpStart) => interpolating += 1,
            Kind::Tok(Tok::InterpEnd) => interpolating = interpolating.saturating_sub(1),
            _ => {}
        }
        out.push(p);
        at = p.span.end;
    }
    at = suffix(text, at, &mut out);
    gap(text, at, text.len() as u32, false, &mut out)?;
    Ok(out)
}

/// A number literal's span leaves its suffix out (`3L`, `1.5f`: `Lexer::number`, for the parser to read the
/// digits); the suffix the gap after one starts with is the literal's.
fn suffix(text: &str, at: u32, out: &mut [Piece]) -> u32 {
    let Some(last) = out.last_mut() else { return at };
    let Kind::Tok(tok) = last.kind else { return at };
    if last.span.end == at && suffixed(text, tok, at) {
        last.span.end += 1;
        return at + 1;
    }
    at
}

/// Whether a number literal of kind `tok` whose digits end at `end` is written with its suffix there.
pub fn suffixed(text: &str, tok: Tok, end: u32) -> bool {
    let suffixes = match tok {
        Tok::LongLit => b"Ll".as_slice(),
        Tok::DoubleLit => b"Dd",
        Tok::FloatLit => b"Ff",
        _ => return false,
    };
    text.as_bytes().get(end as usize).is_some_and(|c| suffixes.contains(c))
}

/// The trivia of `text[from..to]`; inside an interpolated string (`interpolating`), what is no trivia is its
/// delimiters.
fn gap(text: &str, from: u32, to: u32, interpolating: bool, out: &mut Vec<Piece>) -> Result<(), u32> {
    let b = text.as_bytes();
    let mut i = from as usize;
    let to = to as usize;
    while i < to {
        let start = i;
        let kind = match b[i] {
            b' ' => {
                while i < to && b[i] == b' ' {
                    i += 1;
                }
                Kind::Space
            }
            b'\t' => {
                i += 1;
                Kind::Tab
            }
            b'\n' => {
                i += 1;
                Kind::LF
            }
            b'\r' if b.get(i + 1) == Some(&b'\n') && i + 1 < to => {
                i += 2;
                Kind::CRLF
            }
            b'\r' => {
                i += 1;
                Kind::CR
            }
            0x0c => {
                i += 1;
                Kind::FF
            }
            b'#' | b':' if i == 0 && crate::directives::script_header(text) > 0 => {
                i = crate::directives::script_header(text).min(to);
                Kind::Shebang
            }
            0xef if i == 0 && text.starts_with('\u{feff}') => {
                i = 3;
                Kind::Bom
            }
            // A comment, as the lexer skips it (dotty's `skipComment`, the comments `Scanner.keepComments` keeps): a
            // line comment to the end of its line (`skipLine`), a block comment through the close of the comments
            // nested in it. A gap of an interpolated string's holds one in a splice.
            b'/' if b.get(i + 1) == Some(&b'/') => {
                i = crate::lexer::line_comment_end(b, i).min(to);
                Kind::Comment
            }
            b'/' if b.get(i + 1) == Some(&b'*') => {
                i = crate::lexer::block_comment_end(b, i).unwrap_or(b.len()).min(to);
                Kind::Comment
            }
            // A trailing comma, which the lexer drops (`Lexer::at_trailing_comma`): the only comma a gap holds.
            b',' if !interpolating => {
                i += 1;
                Kind::Tok(Tok::Comma)
            }
            _ if interpolating => {
                while i < to && !matches!(b[i], b' ' | b'\t' | b'\n' | b'\r' | 0x0c) {
                    i += 1;
                }
                Kind::InterpDelim
            }
            _ => return Err(i as u32),
        };
        out.push(Piece { kind, span: Span::new(start as u32, i as u32) });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pieces(text: &str) -> Result<Vec<Piece>, u32> {
        let interner = crate::intern::Interner::new();
        let (asts, _) = crate::frontend::parse_one(text, &interner, crate::parser::Syntax { keep: true, ..Default::default() });
        let kept = asts[0].notes.source.as_deref().expect("the source kept");
        editable(text, kept)
    }

    #[test]
    fn kind_names_by_code() {
        // Each name once; the kinds named as the API named them before the codes (the bridge's names, docs/FIX.md).
        let mut names: Vec<&str> = TOKEN_KINDS.to_vec();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), TOKEN_KINDS.len());
        for w in ["while", "if", "this", "null", "true", "implicit", "export", "given", "then"] {
            let t = crate::token::keyword(w).expect(w);
            assert_eq!(kind_name(Kind::Tok(t)), format!("{:?}", t));
        }
        assert_eq!(kind_name(Kind::Tok(Tok::OpIdent)), "Ident");
        assert_eq!(kind_name(Kind::End), "Ident");
        assert_eq!(kind_name(Kind::Tok(Tok::ColonEol)), "Colon");
        assert_eq!(kind_name(Kind::Tok(Tok::QuoteId)), "QuoteId");
        assert_eq!(kind_name(Kind::InterpDelim), "Interpolation.Delimiter");
        assert_eq!(kind_name(Kind::Bom), "BOM");
    }

    #[test]
    fn comments_end_markers_and_line_ends() {
        let text = "// head\r\nobject O:\r\n\t/* a /* nested */ b */ val x = 1 // tail\r\n  def f = 2\r\nend O\r\n";
        let ps = pieces(text).unwrap();
        let joined: String = ps.iter().map(|p| &text[p.span.start as usize..p.span.end as usize]).collect();
        assert_eq!(joined, text);
        let kinds: Vec<Kind> = ps.iter().map(|p| p.kind).collect();
        assert_eq!(kinds[..2], [Kind::Comment, Kind::CRLF]);
        assert!(ps.iter().any(|p| p.kind == Kind::Comment && &text[p.span.start as usize..p.span.end as usize] == "/* a /* nested */ b */"));
        assert!(ps.iter().any(|p| p.kind == Kind::Comment && &text[p.span.start as usize..p.span.end as usize] == "// tail"));
        assert!(kinds.contains(&Kind::Tab) && kinds.contains(&Kind::End));
        let name = ps.iter().position(|p| p.kind == Kind::End).map(|i| ps[i + 2]);
        assert_eq!(name.map(|p| (p.kind, &text[p.span.start as usize..p.span.end as usize])), Some((Kind::Tok(Tok::Ident), "O")));
    }

    #[test]
    fn end_marker_keywords_and_quoted_identifiers() {
        // As scalameta's tokenizer scans them: `end while`'s name a `KwWhile`, `end O`'s an `Ident`, the quote of `'x`
        // and then its name.
        let text = "object O:\n  def f(x: Int): Unit =\n    while x < 0 do ()\n    end while\n  def q(using scala.quoted.Quotes) = '{ ${ 'x } }\nend O\n";
        let ps = pieces(text).unwrap();
        let shown: Vec<(Kind, &str)> = ps.iter().map(|p| (p.kind, &text[p.span.start as usize..p.span.end as usize])).filter(|(k, _)| !matches!(k, Kind::Space | Kind::LF)).collect();
        assert_eq!(shown.iter().filter(|&&(k, t)| k == Kind::End && t == "end").count(), 2);
        assert!(shown.windows(2).any(|w| w[0] == (Kind::End, "end") && w[1] == (Kind::Tok(Tok::KwWhile), "while")));
        assert!(shown.windows(2).any(|w| w[0] == (Kind::End, "end") && w[1] == (Kind::Tok(Tok::Ident), "O")));
        assert!(shown.windows(2).any(|w| w[0] == (Kind::Tok(Tok::Quote), "'") && w[1] == (Kind::Tok(Tok::Ident), "x")));
    }

    #[test]
    fn modifiers_and_variances_where_written() {
        // The tree finds them over the kept tokens; the parser records none of them.
        use crate::fix::nodes::K;
        let text = "object O {\n  final implicit val a: Int = 1\n  class C[+A, -B]\n  @deprecated(\"x\") private[O] def f = 2\n}\n";
        let interner = crate::intern::Interner::new();
        let (asts, _) = crate::frontend::parse_one(text, &interner, crate::parser::Syntax { keep: true, index: true, ..Default::default() });
        let kept = asts[0].notes.source.as_deref().expect("the source kept");
        let refs: Vec<&crate::ast::Ast> = asts.iter().collect();
        let tree = crate::fix::nodes::build(text, &refs, kept, &interner);
        let mut written: Vec<(u32, &str)> = tree
            .nodes
            .iter()
            .filter(|n| matches!(n.kind, K::ModFinal | K::ModImplicit | K::ModPrivate | K::ModCovariant | K::ModContravariant))
            .map(|n| (n.span.start, &text[n.span.start as usize..n.span.end as usize]))
            .collect();
        written.sort();
        assert_eq!(written.iter().map(|w| w.1).collect::<Vec<_>>(), ["final", "implicit", "+", "-", "private"]);
        let (plain, _) = crate::frontend::parse_one(text, &interner, crate::parser::Syntax::default());
        assert!(plain[0].notes.source.is_none());
    }

    /// Every case of the suites, the warnings' edit cases (CRLF, Unicode, comments) and `teq fix`'s fixtures: the
    /// stream is the file, byte for byte.
    #[test]
    fn lossless_over_the_cases() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut files = Vec::new();
        for dir in ["tests/cases", "tests/warnings/edits", "tests/fix", "tests/errors"] {
            collect(&root.join(dir), &mut files);
        }
        assert!(files.len() > 1000, "{} files", files.len());
        let mut checked = 0;
        for f in files {
            let Ok(text) = std::fs::read_to_string(&f) else { continue };
            let interner = crate::intern::Interner::new();
            let (asts, found) = crate::frontend::parse_one(&text, &interner, crate::parser::Syntax { keep: true, ..Default::default() });
            if !found.errors.is_empty() {
                continue;
            }
            let kept = asts[0].notes.source.as_deref().expect("the source kept");
            let ps = editable(&text, kept).unwrap_or_else(|at| {
                let near: Vec<_> = kept.tokens.iter().filter(|t| t.span.end + 8 >= at && t.span.start <= at + 8).map(|t| (t.kind, t.span)).collect();
                panic!("{}: a byte the stream misses at {}: {:?}", f.display(), at, near)
            });
            let joined: String = ps.iter().map(|p| &text[p.span.start as usize..p.span.end as usize]).collect();
            assert!(joined == text, "{}: the stream differs from the text", f.display());
            checked += 1;
        }
        assert!(checked > 1000, "{} files checked", checked);
    }

    fn collect(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                collect(&p, out);
            } else if p.extension().is_some_and(|x| x == "scala") {
                out.push(p);
            }
        }
    }
}
