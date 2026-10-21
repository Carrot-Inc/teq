//! What a rule worker holds of the run (docs/FIX.md, "The bridge"): the document the rules read, its tokens and
//! tree, its offsets in the interpreter's UTF-16 units, the configuration, and what the rules return. The
//! interpreter's `teq.fix.Native` builtins (`interp::fix`) read and fill it through `with_host` on its worker;
//! a handle is a document's generation and an index, so a rule that keeps a tree past its document is told so.

use super::config::Conf;
use super::nodes::Tree;
use super::source::Piece;
use crate::source::Span;
use std::cell::RefCell;

/// A document as the rules read it, built on the typing thread and handed over whole.
pub struct DocView {
    pub gen: u32,
    /// The path the run prints.
    pub path: String,
    /// The path as the program has it, from where the run works: what is written.
    pub file: String,
    pub text: String,
    pub tokens: Vec<Piece>,
    pub tree: Tree,
    pub utf16: Utf16,
    /// The importees the typer's unused-import analysis reports (`typer::unused`), as nodes of `tree`; `Err` where the
    /// analysis is unavailable (withheld after an error, or not run), with the reason.
    pub unused: Result<Vec<u32>, String>,
    /// A file where an expansion was made, a macro's or an inline method's: NoUnusedImports removes nothing there
    /// (docs/FIX.md, "NoUnusedImports").
    pub expanded: bool,
    /// The file's import selectors as the parser recorded them, in source order, which the planner's import edits read
    /// (`typer::unused_edits`), and each importee node's selector.
    pub imports: Vec<crate::ast::Import>,
    pub importee_of: Vec<(u32, usize)>,
}

/// Offsets in UTF-8 bytes and in UTF-16 units, the interpreter's (`text.rs`): the pairs where they part, after each
/// character that is not ASCII, between which both advance together.
#[derive(Default)]
pub struct Utf16 {
    marks: Vec<(u32, u32)>,
}

impl Utf16 {
    pub fn of(text: &str) -> Utf16 {
        let mut marks = vec![(0, 0)];
        let mut units = 0u32;
        for (i, c) in text.char_indices() {
            if !c.is_ascii() {
                units += c.len_utf16() as u32;
                marks.push(((i + c.len_utf8()) as u32, units));
            } else {
                units += 1;
            }
        }
        Utf16 { marks }
    }

    pub fn units(&self, byte: u32) -> u32 {
        let i = self.marks.partition_point(|m| m.0 <= byte) - 1;
        let (b, u) = self.marks[i];
        u + (byte - b)
    }

    pub fn bytes(&self, units: u32) -> u32 {
        let i = self.marks.partition_point(|m| m.1 <= units) - 1;
        let (b, u) = self.marks[i];
        b + (units - u)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Op {
    /// Text added left of a token, right of it, or the token removed.
    Left,
    Right,
    Remove,
}

#[derive(Clone, Debug)]
pub struct Edit {
    pub rule: u32,
    /// The rule's atomic patch the edit belongs to, 0 for none (`Patch.atomic`).
    pub group: u32,
    pub op: Op,
    pub token: u32,
    pub text: String,
}

#[derive(Clone, Debug)]
pub enum ImportOp {
    /// `Patch.removeImportee`: the importee's node.
    Remove { rule: u32, group: u32, importee: u32 },
    /// `Patch.addGlobalImport`: the importer as written, `a.b.c`.
    Add { rule: u32, group: u32, importer: String },
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, PartialOrd, Ord)]
pub enum Severity {
    Info,
    Warning,
    Error,
}

#[derive(Clone, Debug)]
pub struct Lint {
    pub rule: u32,
    pub category: String,
    pub message: String,
    pub severity: Severity,
    /// In bytes of the document.
    pub span: Span,
    /// The rule's atomic patch the lint is part of, 0 for none.
    pub group: u32,
}

#[derive(Default)]
pub struct Results {
    pub edits: Vec<Edit>,
    pub imports: Vec<ImportOp>,
    pub lints: Vec<Lint>,
    pub failures: Vec<(u32, String)>,
}

/// Who reads a document: its job, its round, its slot (its place among the round's documents), its generation and the
/// worker. The coordinator assigns it when it hands the document out; the worker's answer carries it back from the host,
/// and a query to the target typer would carry it (fix-speed's table, §4: `fix::query`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Asker {
    pub job: u32,
    pub round: u32,
    pub slot: u32,
    pub gen: u32,
    pub worker: u32,
}

/// A rule worker's state.
pub struct Host {
    pub doc: Option<DocView>,
    pub results: Results,
    pub conf: Conf,
    /// Per node of `conf`, whether a rule's decoding read it (`Native.confField`): a configured rule's settings no
    /// decoding read are unknown ones.
    pub read: Vec<bool>,
    /// The atomic groups the current document's rules made (`Native.newGroup`), numbered from 1 there.
    pub groups: u32,
    /// Who reads the current document.
    pub asker: Option<Asker>,
}

thread_local! {
    static HOST: RefCell<Option<Host>> = const { RefCell::new(None) };
}

pub fn install(host: Host) {
    HOST.with(|h| *h.borrow_mut() = Some(host));
}

/// `f` over the rule thread's host; `None` outside `teq fix`.
pub fn with_host<T>(f: impl FnOnce(&mut Host) -> T) -> Option<T> {
    HOST.with(|h| h.borrow_mut().as_mut().map(f))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf16_offsets() {
        // The design pass's probe: `null` at byte 16, UTF-16 unit 14.
        let text = "val x = \"😀\"; null";
        let u = Utf16::of(text);
        assert_eq!(text.find("null"), Some(16));
        assert_eq!(u.units(16), 14);
        assert_eq!(u.bytes(14), 16);
        assert_eq!(u.units(0), 0);
        assert_eq!(u.units(9), 9);
        let t = "é1€2";
        let v = Utf16::of(t);
        assert_eq!((v.units(2), v.units(3), v.units(6), v.units(7)), (1, 2, 3, 4));
        assert_eq!((v.bytes(1), v.bytes(2), v.bytes(3), v.bytes(4)), (2, 3, 6, 7));
    }
}
