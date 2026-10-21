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
    /// The `end` of an end marker, and its name or keyword.
    End,
    EndName,
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

/// A token's kind as the API names it, scalameta's token class (`KwVar`, `Ident`, `Semicolon`, `Tab`, `Comment`).
pub fn kind_name(k: Kind) -> String {
    let named = match k {
        Kind::Space => "Space",
        Kind::Tab => "Tab",
        Kind::LF => "LF",
        Kind::CRLF => "CRLF",
        Kind::CR => "CR",
        Kind::FF => "FF",
        Kind::Comment => "Comment",
        Kind::End | Kind::EndName => "Ident",
        Kind::Shebang => "Shebang",
        Kind::Bom => "BOM",
        Kind::InterpDelim => "Interpolation.Delimiter",
        Kind::Tok(t) => match t {
            Tok::Ident | Tok::OpIdent => "Ident",
            Tok::Semi => "Semicolon",
            Tok::Comma => "Comma",
            Tok::Dot => "Dot",
            Tok::LParen => "LeftParen",
            Tok::RParen => "RightParen",
            Tok::LBracket => "LeftBracket",
            Tok::RBracket => "RightBracket",
            Tok::LBrace => "LeftBrace",
            Tok::RBrace => "RightBrace",
            Tok::Colon | Tok::ColonEol => "Colon",
            Tok::Eq => "Equals",
            Tok::Arrow => "RightArrow",
            Tok::CtxArrow => "ContextArrow",
            Tok::LArrow => "LeftArrow",
            Tok::Subtype => "Subtype",
            Tok::Supertype => "Supertype",
            Tok::At => "At",
            Tok::Underscore => "Underscore",
            Tok::IntLit => "Constant.Int",
            Tok::LongLit => "Constant.Long",
            Tok::DoubleLit => "Constant.Double",
            Tok::FloatLit => "Constant.Float",
            Tok::CharLit => "Constant.Char",
            Tok::StringLit => "Constant.String",
            Tok::InterpStart => "Interpolation.Id",
            Tok::StrPart => "Interpolation.Part",
            Tok::InterpEnd => "Interpolation.End",
            _ => return format!("{:?}", t),
        },
    };
    named.to_string()
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

/// Every byte of `text` in order: the kept tokens and end markers, and the comments and trivia between them. A gap
/// holds only what the lexer skips; anything else there is a defect of the record, `Err` with its offset.
pub fn editable(text: &str, kept: &SourceKept) -> Result<Vec<Piece>, u32> {
    let mut marks: Vec<Piece> = Vec::with_capacity(kept.tokens.len() + kept.end_markers.len() * 2);
    marks.extend(kept.tokens.iter().filter_map(|t| editable_kind(text, t).map(|k| Piece { kind: Kind::Tok(k), span: t.span })));
    for &m in &kept.end_markers {
        let word = Span::new(m.start, m.start + 3);
        let name_start = text[word.end as usize..m.end as usize].find(|c: char| c != ' ').map_or(m.end, |i| word.end + i as u32);
        marks.push(Piece { kind: Kind::End, span: word });
        marks.push(Piece { kind: Kind::EndName, span: Span::new(name_start, m.end) });
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
    fn comments_end_markers_and_line_ends() {
        let text = "// head\r\nobject O:\r\n\t/* a /* nested */ b */ val x = 1 // tail\r\n  def f = 2\r\nend O\r\n";
        let ps = pieces(text).unwrap();
        let joined: String = ps.iter().map(|p| &text[p.span.start as usize..p.span.end as usize]).collect();
        assert_eq!(joined, text);
        let kinds: Vec<Kind> = ps.iter().map(|p| p.kind).collect();
        assert_eq!(kinds[..2], [Kind::Comment, Kind::CRLF]);
        assert!(ps.iter().any(|p| p.kind == Kind::Comment && &text[p.span.start as usize..p.span.end as usize] == "/* a /* nested */ b */"));
        assert!(ps.iter().any(|p| p.kind == Kind::Comment && &text[p.span.start as usize..p.span.end as usize] == "// tail"));
        assert!(kinds.contains(&Kind::Tab) && kinds.contains(&Kind::End) && kinds.contains(&Kind::EndName));
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
