//! Where a library's definitions and the nodes of its converted bodies stand in its sources:
//! the span scalac's
//! unpickler gives the tree a node stands for, in its source as the pickle names it, and that
//! source matched to a text where one is found: an entry of the sources jar beside a jar, the
//! owning source a product's manifest names.

use super::super::Worker;
use crate::ast::{ExprId, NodePlaces};
use crate::intern::FxMap;
use crate::source::{FileId, Span};
use crate::tasty::positions::{Pos, Section, Spans};
use crate::tasty::tree::Addr;
use std::sync::Arc;

/// The positions of one TASTy file, read when a place in it is first asked for: the section
/// at once, the spans of every tree on the first place of a node (a definition's is its entry).
pub struct FilePositions {
    pub section: Option<Section>,
    spans: std::sync::OnceLock<Spans>,
    tasty: Arc<crate::tasty::TastyFile>,
    /// `SOURCEFILEattr`'s path, the source where the file has no `Positions` section.
    pub source_file: Option<String>,
}

impl FilePositions {
    pub fn spans(&self) -> &Spans {
        self.spans.get_or_init(|| self.section.as_ref().map_or_else(Spans::default, |s| Spans::of(&self.tasty, s)))
    }

    /// The path of the source of the tree at `addr`: its switch's, else the primary source's or
    /// `SOURCEFILEattr`'s.
    pub fn source_path(&self, addr: Addr, placed_source: Option<crate::tasty::NameRef>) -> Option<String> {
        match (placed_source.or_else(|| self.section.as_ref().and_then(|s| s.source_at(addr))), &self.section) {
            (Some(n), _) => Some(self.tasty.name(n)),
            (None, None) => self.source_file.clone(),
            (None, Some(s)) => s.primary().map(|n| self.tasty.name(n)).or_else(|| self.source_file.clone()),
        }
    }
}

/// A source matched to the text a jar or product directory holds for it.
pub struct Attached {
    pub text: Arc<str>,
}

/// A library node's place: the artifact, the source's path as the pickle records it, the span
/// in UTF-16 units, the line and column where they are known.
#[derive(Clone, Debug)]
pub struct LibPlace {
    /// The jar's file name, or the product directory's path.
    pub artifact: String,
    pub path: Option<String>,
    pub pos: Option<Pos>,
    /// The line from 1 and the column from 1: from the attached text in bytes, else from the
    /// primary source's line table in UTF-16 units; `None` for a switched source without text.
    pub line_col: Option<(u32, u32)>,
    /// The line of the attached text the place is on.
    pub line_text: Option<String>,
}

impl LibPlace {
    /// The listing's form: `<path>:<line>:<column>`, the path
    /// alone where the coordinates are not known, `-` without a position.
    pub fn listed(&self) -> String {
        match (&self.path, self.pos, self.line_col) {
            (_, None, _) => "-".to_string(),
            (Some(p), Some(_), Some((l, c))) => format!("{}:{}:{}", p, l, c),
            (Some(p), Some(_), None) => p.clone(),
            (None, Some(_), _) => "-".to_string(),
        }
    }

    /// `<artifact>!<path>`, the artifact alone where the pickle names no source.
    pub fn source(&self) -> String {
        match &self.path {
            Some(p) => format!("{}!{}", self.artifact, p),
            None => self.artifact.clone(),
        }
    }
}

/// What the loader keeps of a sources jar: its `.scala` and `.java` entries by file name.
pub struct SourcesJar {
    zip: crate::zip::Zip,
    by_name: FxMap<String, Vec<usize>>,
    texts: FxMap<usize, Arc<str>>,
}

/// The outcome of matching a recorded path to the entries of a sources jar.
#[derive(Debug, PartialEq, Eq)]
pub enum Match {
    Found(usize),
    /// Several entries share the longest suffix of components with the path.
    Ambiguous(Vec<String>),
    None,
}

impl SourcesJar {
    pub fn open(path: &str) -> Option<SourcesJar> {
        let zip = crate::zip::Zip::open(path).ok()?;
        let mut by_name: FxMap<String, Vec<usize>> = FxMap::default();
        for (i, e) in zip.entries.iter().enumerate() {
            let name = zip.name(e);
            if name.ends_with(".scala") || name.ends_with(".java") {
                let file = name.rsplit('/').next().unwrap_or(name).to_string();
                by_name.entry(file).or_default().push(i);
            }
        }
        Some(SourcesJar { zip, by_name, texts: FxMap::default() })
    }

    /// The entry whose path shares the longest suffix of components with `recorded` among those
    /// of the same file name.
    pub fn find(&self, recorded: &str) -> Match {
        let parts: Vec<&str> = recorded.split('/').filter(|p| !p.is_empty()).collect();
        let Some(file) = parts.last() else { return Match::None };
        let Some(candidates) = self.by_name.get(*file) else { return Match::None };
        let shared = |entry: &str| entry.split('/').rev().zip(parts.iter().rev()).take_while(|(a, b)| a == *b).count();
        let best = candidates.iter().map(|&i| shared(self.zip.name(&self.zip.entries[i]))).max().unwrap_or(0);
        let top: Vec<usize> = candidates.iter().copied().filter(|&i| shared(self.zip.name(&self.zip.entries[i])) == best).collect();
        match top.as_slice() {
            [] => Match::None,
            [i] => Match::Found(*i),
            many => {
                let mut names: Vec<String> = many.iter().map(|&i| self.zip.name(&self.zip.entries[i]).to_string()).collect();
                names.sort();
                Match::Ambiguous(names)
            }
        }
    }

    pub fn entry_name(&self, i: usize) -> &str {
        self.zip.name(&self.zip.entries[i])
    }

    pub fn len(&self) -> usize {
        self.zip.entries.len()
    }

    /// An entry's name, CRC-32 and size, from the central directory.
    pub fn entry_fields(&self, i: usize) -> (&str, u32, u32) {
        let e = &self.zip.entries[i];
        (self.zip.name(e), e.crc32, e.size)
    }

    pub fn text(&mut self, i: usize) -> Option<Arc<str>> {
        if let Some(t) = self.texts.get(&i) {
            return Some(t.clone());
        }
        let bytes = self.zip.read(i).ok()?;
        let text: Arc<str> = Arc::from(String::from_utf8_lossy(&bytes).as_ref());
        self.texts.insert(i, text.clone());
        Some(text)
    }
}

/// The sources jar beside a jar: `<stem>-sources.jar` in its directory.
pub fn sources_jar_of(jar: &str) -> Option<String> {
    let stem = jar.strip_suffix(".jar")?;
    let path = format!("{}-sources.jar", stem);
    std::path::Path::new(&path).is_file().then_some(path)
}

/// The byte offset in `text` of the UTF-16 offset `units`, its line from 1, its column from 1 in
/// bytes and the line's text.
pub fn byte_place(text: &str, units: u32) -> (usize, u32, u32, String) {
    let mut n = 0u32;
    let mut at = text.len();
    for (i, c) in text.char_indices() {
        if n >= units {
            at = i;
            break;
        }
        n += c.len_utf16() as u32;
    }
    let (line, col, line_text) = crate::source::locate(text, at);
    (at, line as u32, col as u32, line_text.to_string())
}

impl<'a> Worker<'a> {
    /// The positions of the loaded TASTy file `file`, read on the first call.
    pub(in crate::typer) fn file_positions(&mut self, file: u32) -> Arc<FilePositions> {
        if let Some(p) = self.loaded.as_ref().unwrap().positions.get(&file) {
            return p.clone();
        }
        self.with_loader(|w| {
            if let Some(p) = w.loaded.as_ref().unwrap().positions.get(&file) {
                return p.clone();
            }
            let tasty = w.tasty(file);
            let section = Section::read(&tasty);
            let source_file = source_file_attr(&tasty);
            let p = Arc::new(FilePositions { section, spans: std::sync::OnceLock::new(), tasty, source_file });
            w.loaded_mut().positions.insert(file, p.clone());
            p
        })
    }

    /// The source path and the span (UTF-16 units of that source) the pickle `file` gives its
    /// tree at `addr`, which a pickle holding an expansion of the tree gives it again.
    pub(crate) fn pickled_place(&mut self, file: u32, addr: Addr) -> Option<(String, crate::tasty::positions::Pos)> {
        let positions = self.file_positions(file);
        let placed = positions.spans().get(addr)?;
        let pos = placed.pos?;
        let source = placed.source.or_else(|| positions.section.as_ref().and_then(|s| s.source_at(addr)));
        let path = positions.source_path(addr, source)?;
        Some((path, pos))
    }

    /// The places the pickle `file` gives the term parameters of its method at `addr`, in order.
    pub(crate) fn pickled_param_places(&mut self, file: u32, addr: Addr) -> Vec<Option<(String, crate::tasty::positions::Pos)>> {
        let positions = self.file_positions(file);
        let sig = crate::tasty::tree::Decoder::new(&positions.tasty).def_sig(addr);
        let params: Vec<Addr> = sig
            .clauses
            .iter()
            .flat_map(|c| match c {
                crate::tasty::tree::Clause::Terms(ps) => ps.iter().map(|p| p.addr).collect(),
                crate::tasty::tree::Clause::Types(_) => Vec::new(),
            })
            .collect();
        params.into_iter().map(|p| self.pickled_place(file, p)).collect()
    }

    /// Whether the application at `addr` of the pickle `file` gives its function a position of
    /// its own: an auto-applied selection's, whose span is the call's where the source wrote no
    /// parentheses.
    pub(crate) fn pickled_function_placed(&mut self, file: u32, addr: Addr) -> bool {
        let positions = self.file_positions(file);
        let mut decoder = crate::tasty::tree::Decoder::new(&positions.tasty);
        let call = crate::tasty::terms::TermDecoder::new(&mut decoder).term_at(addr);
        match call.kind {
            crate::tasty::terms::TermKind::Apply(f, _) => positions.section.as_ref().map_or(false, |s| s.entry(f.at).is_some()),
            _ => false,
        }
    }

    /// The place of the tree at `addr` of the loaded TASTy file `file`.
    pub(in crate::typer) fn tasty_place(&mut self, file: u32, addr: Addr) -> LibPlace {
        let positions = self.file_positions(file);
        let placed = positions.spans().get(addr);
        let pos = placed.and_then(|p| p.pos);
        let source = placed.and_then(|p| p.source).or_else(|| positions.section.as_ref().and_then(|s| s.source_at(addr)));
        let path = positions.source_path(addr, source);
        let primary = match &positions.section {
            Some(s) => source.map_or(true, |n| s.primary() == Some(n)),
            None => false,
        };
        let (artifact, jar) = self.artifact_of(file);
        let mut place = LibPlace { artifact, path: path.clone(), pos, line_col: None, line_text: None };
        let attached = path.as_deref().and_then(|p| self.attached(file, &jar, p));
        match (attached, pos) {
            (Some(text), Some(pos)) => {
                let (_, line, col, line_text) = byte_place(&text.text, pos.point.unwrap_or(pos.start));
                place.line_col = Some((line, col));
                place.line_text = Some(line_text);
            }
            (None, Some(pos)) if primary => {
                place.line_col = positions.section.as_ref().and_then(|s| s.line_col(pos.point.unwrap_or(pos.start)));
            }
            _ => {}
        }
        place
    }

    /// The jar's file name (a product directory's path) and the jar's path.
    fn artifact_of(&self, file: u32) -> (String, String) {
        let loaded = self.loaded.as_ref().unwrap();
        let cp = loaded.file(file).cp;
        let jar = loaded.cp.paths[cp.jar as usize].clone();
        let name = if jar.ends_with(".jar") { jar.rsplit(['/', '\\']).next().unwrap_or(&jar).to_string() } else { jar.clone() };
        (name, jar)
    }

    /// The text attached to the source `recorded` of the loaded file `file` from the jar `jar`:
    /// the sources jar's entry, or the product's owning source; `None` where there is none.
    pub(in crate::typer) fn attached(&mut self, file: u32, jar: &str, recorded: &str) -> Option<Attached> {
        if jar.ends_with(".jar") {
            let sources = sources_jar_of(jar)?;
            return self.with_loader(|w| {
                let loaded = w.loaded_mut();
                let sj = loaded.sources_jars.entry(sources.clone()).or_insert_with(|| SourcesJar::open(&sources));
                let sj = sj.as_mut()?;
                match sj.find(recorded) {
                    Match::Found(i) => Some(Attached { text: sj.text(i)? }),
                    _ => None,
                }
            });
        }
        let (_, text) = self.product_text(file)?;
        Some(Attached { text })
    }

    /// The owning source of a product's TASTy file and its text, where the text is the one the
    /// producer typed: a source whose fingerprint differs from the manifest's has changed since
    /// the build, and the pickle's positions describe another text.
    pub(in crate::typer) fn product_text(&mut self, file: u32) -> Option<(String, Arc<str>)> {
        let (path, fingerprint) = self.product_source_of(file)?;
        let text = std::fs::read_to_string(&path).ok()?;
        if fingerprint.map_or(false, |f| f != crate::products::fingerprint(&text)) {
            return None;
        }
        Some((path, Arc::from(text.as_str())))
    }

    /// The owning source of a product's TASTy file, from its directory's manifest, with the
    /// fingerprint of the text the producer typed.
    pub(in crate::typer) fn product_source_of(&mut self, file: u32) -> Option<(String, Option<String>)> {
        let loaded = self.loaded.as_ref().unwrap();
        let cp = loaded.file(file).cp;
        let dir = loaded.cp.paths[cp.jar as usize].clone();
        let entry = loaded.cp.entry_name(cp).to_string();
        let manifest = match loaded.manifests.get(&cp.jar) {
            Some(m) => m.clone(),
            None => {
                // Read outside the lock; two workers that both missed read the same file.
                let read = crate::products::Manifest::read(std::path::Path::new(&dir)).and_then(Result::ok).map(Arc::new);
                self.with_loader(|w| w.loaded_mut().manifests.insert(cp.jar, read.clone()));
                read
            }
        }?;
        let e = manifest.entries.iter().find(|e| e.tasty.as_deref() == Some(entry.as_str()))?;
        let p = std::path::Path::new(&e.source);
        let path = if p.is_absolute() { p.to_path_buf() } else { std::path::Path::new(&manifest.root).join(p) };
        path.is_file().then(|| (path.to_string_lossy().to_string(), e.fingerprint.clone()))
    }

    /// The place of a node of a library body's pseudo file: the tree it stands for.
    pub(in crate::typer) fn expr_place(&mut self, file: FileId, e: ExprId) -> Option<LibPlace> {
        let ast = self.ast(file);
        let places: &NodePlaces = &ast.reader.as_ref()?.places;
        let addr = *places.exprs.get(e.idx())?;
        let tasty_file = places.file;
        Some(self.tasty_place(tasty_file, addr))
    }

    /// The place of a pattern of a library body's pseudo file: the tree it stands for.
    pub(in crate::typer) fn pat_place(&mut self, file: FileId, p: crate::ast::PatId) -> Option<LibPlace> {
        let ast = self.ast(file);
        let places: &NodePlaces = &ast.reader.as_ref()?.places;
        let addr = *places.pats.get(p.idx())?;
        let tasty_file = places.file;
        Some(self.tasty_place(tasty_file, addr))
    }

    /// The place of the definition of a pseudo file whose span is `span`: the member a node of
    /// that span stands in.
    pub(in crate::typer) fn member_place(&mut self, file: FileId, span: Span) -> Option<LibPlace> {
        let (_, ls) = self.library_member_at(file, span)?;
        Some(self.tasty_place(ls.file, ls.addr))
    }

    /// The member of the library a node of the pseudo file `file` spanning `span` stands in:
    /// the member's line holds the definitions its body makes as well (an anonymous class's
    /// members), so of the definitions of that span the one the pickle declares, the first in
    /// the file's order, whatever order the table of definitions holds them in (one the worker
    /// that converted the file gives).
    pub(in crate::typer) fn library_member_at(&self, file: FileId, span: Span) -> Option<(crate::types::SymId, super::LSym)> {
        let loaded = self.loaded.as_ref()?;
        let mut defs: Vec<(crate::ast::DefId, crate::types::SymId)> = self.def_syms.entries_in(file.0 as usize).into_iter().filter(|(d, _)| self.ast(file).def(*d).span == span).collect();
        defs.sort_unstable_by_key(|&(d, _)| d);
        defs.into_iter().find_map(|(_, s)| loaded.syms.get(&s).copied().map(|ls| (s, ls)))
    }
}

/// The `SOURCEFILEattr` of a file, the path it names.
pub(super) fn source_file_attr(tasty: &crate::tasty::TastyFile) -> Option<String> {
    let bytes = &tasty.bytes[tasty.attributes.clone()];
    let mut r = crate::tasty::Reader::new(bytes);
    while !r.at_end() {
        let tag = r.byte();
        match tag {
            1..=32 => {}
            129..=160 => {
                let n = r.nat();
                if tag == 129 {
                    return Some(tasty.name(n));
                }
            }
            _ => return None,
        }
    }
    None
}
