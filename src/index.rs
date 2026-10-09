//! The queries a `teq compiler watch --check --index` session answers for the language server
//! (`src/lsp`), and the positions and locations of their answers. Queries name files by path
//! and positions by byte offset; answers are LSP values, one JSON line `{"result":...}` each,
//! with lines and columns counted as the session's `--positions` says.

use crate::lsp::json::{obj, Json};
use crate::source::Span;
use crate::watch::Positions;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub enum Query {
    Definition(PathBuf, u32),
    References(PathBuf, u32, bool),
    Hover(PathBuf, u32),
    Symbols(PathBuf),
    WorkspaceSymbols(String),
    Implementation(PathBuf, u32),
    PrepareCall(PathBuf, u32),
    Incoming(PathBuf, u32),
    Outgoing(PathBuf, u32),
    /// `complete <path> <offset> <flags>`: the completion list at the offset (`typer::complete`),
    /// the flags' bit 0 saying the client takes insert-and-replace edits, bit 1 snippets.
    Complete(PathBuf, u32, u32),
    /// `complete-resolve <path> <offset> <generation> <path of the name>`: the import an item
    /// of a name out of scope brings, answered while the program is the one of `generation`.
    Resolve(PathBuf, u32, String, String),
    /// `signature <path> <offset>`: the signature help at the offset (`typer::signature`).
    Signature(PathBuf, u32),
    /// `index-stats`: the sizes of the index's tables, which a session's edits keep flat.
    Stats,
}

impl Query {
    /// A command line of the session: `definition <path> <offset>`, `references <path>
    /// <offset> <0|1>`, `hover`, `implementation`, `prepare-call`, `incoming` and `outgoing`
    /// like `definition`, `symbols <path>`, `workspace-symbols <query>`. A path may hold spaces:
    /// the numbers are read from the end.
    pub fn parse(line: &str) -> Option<Query> {
        let (verb, rest) = line.split_once(' ').unwrap_or((line, ""));
        let at = |rest: &str| -> Option<(PathBuf, u32)> {
            let (path, offset) = rest.rsplit_once(' ')?;
            Some((PathBuf::from(path), offset.trim().parse().ok()?))
        };
        Some(match verb {
            "definition" => at(rest).map(|(p, o)| Query::Definition(p, o))?,
            "hover" => at(rest).map(|(p, o)| Query::Hover(p, o))?,
            "implementation" => at(rest).map(|(p, o)| Query::Implementation(p, o))?,
            "prepare-call" => at(rest).map(|(p, o)| Query::PrepareCall(p, o))?,
            "incoming" => at(rest).map(|(p, o)| Query::Incoming(p, o))?,
            "outgoing" => at(rest).map(|(p, o)| Query::Outgoing(p, o))?,
            "references" => {
                let (rest, declaration) = rest.rsplit_once(' ')?;
                let (path, offset) = at(rest)?;
                Query::References(path, offset, declaration.trim() == "1")
            }
            "complete" => {
                let (rest, flags) = rest.rsplit_once(' ')?;
                let (path, offset) = at(rest)?;
                Query::Complete(path, offset, flags.trim().parse().ok()?)
            }
            "complete-resolve" => {
                let (rest, name) = rest.rsplit_once(' ')?;
                let (rest, generation) = rest.rsplit_once(' ')?;
                let (path, offset) = at(rest)?;
                Query::Resolve(path, offset, generation.to_string(), name.trim().to_string())
            }
            "signature" => at(rest).map(|(p, o)| Query::Signature(p, o))?,
            "symbols" if !rest.is_empty() => Query::Symbols(PathBuf::from(rest)),
            "workspace-symbols" => Query::WorkspaceSymbols(rest.to_string()),
            "index-stats" => Query::Stats,
            _ => return None,
        })
    }

    pub fn file(&self) -> Option<&Path> {
        match self {
            Query::Definition(p, _)
            | Query::References(p, _, _)
            | Query::Hover(p, _)
            | Query::Symbols(p)
            | Query::Implementation(p, _)
            | Query::PrepareCall(p, _)
            | Query::Incoming(p, _)
            | Query::Outgoing(p, _)
            | Query::Complete(p, _, _)
            | Query::Resolve(p, _, _, _)
            | Query::Signature(p, _) => Some(p),
            Query::WorkspaceSymbols(_) | Query::Stats => None,
        }
    }
}

/// The starts of a text's lines, for turning offsets into positions.
pub struct Lines<'t> {
    text: &'t str,
    starts: Vec<u32>,
}

impl<'t> Lines<'t> {
    pub fn new(text: &'t str) -> Lines<'t> {
        let mut starts = vec![0];
        starts.extend(text.bytes().enumerate().filter(|&(_, b)| b == b'\n').map(|(i, _)| i as u32 + 1));
        Lines { text, starts }
    }

    /// The LSP position of a byte offset: its line from 0, and its column in UTF-16 units or
    /// bytes. An offset inside a character counts from the character's start.
    pub fn position(&self, offset: u32, positions: Positions) -> Json {
        let offset = offset.min(self.text.len() as u32);
        let line = self.starts.partition_point(|&s| s <= offset) - 1;
        let start = self.starts[line] as usize;
        let mut end = offset as usize;
        while !self.text.is_char_boundary(end) {
            end -= 1;
        }
        let column = match positions {
            Positions::Utf16 => self.text[start..end].encode_utf16().count(),
            Positions::Utf8 => end - start,
        };
        obj([("line", (line as u32).into()), ("character", (column as u32).into())])
    }

    pub fn range(&self, span: Span, positions: Positions) -> Json {
        obj([("start", self.position(span.start, positions)), ("end", self.position(span.end.max(span.start), positions))])
    }
}

/// The byte offset of an LSP position in a text: past the end of its line, the line's end
/// (before its `\r\n`); past the last line, the text's end.
pub fn offset_of(text: &str, line: u32, character: u32, positions: Positions) -> u32 {
    let mut start = 0usize;
    for _ in 0..line {
        match text[start..].find('\n') {
            Some(i) => start += i + 1,
            None => return text.len() as u32,
        }
    }
    let line_end = text[start..].find('\n').map_or(text.len(), |i| start + i);
    let line_end = if line_end > start && text.as_bytes()[line_end - 1] == b'\r' { line_end - 1 } else { line_end };
    let line_text = &text[start..line_end];
    let within = match positions {
        Positions::Utf8 => {
            let mut b = (character as usize).min(line_text.len());
            while !line_text.is_char_boundary(b) {
                b -= 1;
            }
            b
        }
        Positions::Utf16 => {
            let mut units = 0u32;
            let mut at = line_text.len();
            for (i, c) in line_text.char_indices() {
                if units >= character {
                    at = i;
                    break;
                }
                units += c.len_utf16() as u32;
                if units > character {
                    // Inside a surrogate pair: the character's start.
                    at = i;
                    break;
                }
            }
            at
        }
    };
    (start + within) as u32
}

pub fn location(path: &Path, lines: &Lines, span: Span, positions: Positions) -> Json {
    obj([("uri", crate::lsp::uri::from_path(path).into()), ("range", lines.range(span, positions))])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positions_round_trip() {
        let text = "a\r\nx😀y = 1\nz";
        let lines = Lines::new(text);
        let y = text.find('y').unwrap() as u32;
        assert_eq!(lines.position(y, Positions::Utf16).to_text(), r#"{"line":1,"character":3}"#);
        assert_eq!(lines.position(y, Positions::Utf8).to_text(), r#"{"line":1,"character":5}"#);
        assert_eq!(offset_of(text, 1, 3, Positions::Utf16), y);
        assert_eq!(offset_of(text, 1, 5, Positions::Utf8), y);
        assert_eq!(offset_of(text, 0, 9, Positions::Utf16), 1);
        assert_eq!(offset_of(text, 9, 0, Positions::Utf16), text.len() as u32);
    }

    #[test]
    fn parses_queries() {
        assert_eq!(Query::parse("references /a b.scala 12 1"), Some(Query::References(PathBuf::from("/a b.scala"), 12, true)));
        assert_eq!(Query::parse("workspace-symbols Foo bar"), Some(Query::WorkspaceSymbols("Foo bar".to_string())));
        assert_eq!(Query::parse("definition /a.scala x"), None);
        assert_eq!(Query::parse("complete /a b.scala 12 1"), Some(Query::Complete(PathBuf::from("/a b.scala"), 12, 1)));
        assert_eq!(Query::parse("complete-resolve /a.scala 3 7:2 java.util.UUID"), Some(Query::Resolve(PathBuf::from("/a.scala"), 3, "7:2".into(), "java.util.UUID".into())));
        assert_eq!(Query::parse("signature /a b.scala 12"), Some(Query::Signature(PathBuf::from("/a b.scala"), 12)));
    }
}
