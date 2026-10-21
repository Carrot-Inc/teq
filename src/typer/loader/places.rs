//! Where a library's definitions and the nodes of its converted bodies stand in its sources:
//! the span scalac's
//! unpickler gives the tree a node stands for, in its source as the pickle names it, and that
//! source matched to a text where one is found: an entry of a jar's sources jar (`SourcesLookup`),
//! the owning source a product's manifest names.

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

/// What the loader keeps of a sources jar: its `.scala` and `.java` entries by file name, and the
/// size and modification time of the file it was opened from (read through its handle).
pub struct SourcesJar {
    zip: crate::zip::Zip,
    by_name: FxMap<String, Vec<usize>>,
    texts: FxMap<usize, Arc<str>>,
    stamp: Stamp,
}

/// A file's size and modification time.
type Stamp = (u64, Option<std::time::SystemTime>);

/// The stamp of what stands at `path` now; none where no file is.
fn path_stamp(path: &str) -> Option<Stamp> {
    std::fs::metadata(path).ok().filter(|m| m.is_file()).map(|m| (m.len(), m.modified().ok()))
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
    /// The sources jar at `path` when it is a whole archive (`zip::Zip::open_valid`, the language
    /// server's validation too).
    pub fn open(path: &str) -> Option<SourcesJar> {
        let zip = crate::zip::Zip::open_valid(path).ok()?;
        let stamp = zip.stamp()?;
        let mut by_name: FxMap<String, Vec<usize>> = FxMap::default();
        for (i, e) in zip.entries.iter().enumerate() {
            let name = zip.name(e);
            if name.ends_with(".scala") || name.ends_with(".java") {
                let file = name.rsplit('/').next().unwrap_or(name).to_string();
                by_name.entry(file).or_default().push(i);
            }
        }
        Some(SourcesJar { zip, by_name, texts: FxMap::default(), stamp })
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

    /// The text of entry `i`, read once; why it does not read (its CRC or size not the directory's,
    /// its data cut), which says the archive is damaged.
    pub fn text(&mut self, i: usize) -> Result<Arc<str>, String> {
        if let Some(t) = self.texts.get(&i) {
            return Ok(t.clone());
        }
        let bytes = self.zip.read(i)?;
        let text: Arc<str> = Arc::from(String::from_utf8_lossy(&bytes).as_ref());
        self.texts.insert(i, text.clone());
        Ok(text)
    }
}

/// The sources jars of the class path's jars: per jar the files its sources are looked for in, in
/// order (the language server's `--sources-of`: beside the jar, in coursier's cache, in teq's sources
/// store; else `<stem>-sources.jar` beside it), the one that opened, kept while its entries read, and
/// each that failed to open or to read with its size and modification time then, passed over until
/// either changes.
/// A jar none of whose files opens is looked for again at each call, so that a sources jar placed
/// later (fetched by the language server, written by a build tool, a damaged one replaced) is found
/// at the next navigation without a restart. Read under the loader's lock.
#[derive(Default)]
pub struct SourcesLookup {
    /// The language server's candidates, by the jar's path as the class path names it.
    pub candidates: FxMap<String, Vec<String>>,
    /// Per jar, the candidate that opened.
    chosen: FxMap<String, String>,
    /// The sources jars opened, by path.
    pub opened: FxMap<String, SourcesJar>,
    failed: FxMap<String, Stamp>,
    /// The sha1s of the archives rejected at a read: a candidate opened with those bytes, at any path
    /// or stamp (a copy of them, a file changed back to them), is rejected as they were, unread.
    rejected_hashes: std::collections::BTreeSet<String>,
}

impl SourcesLookup {
    /// The files `jar`'s sources are looked for in, in order.
    pub fn candidates_of(&self, jar: &str) -> Vec<String> {
        match self.candidates.get(jar) {
            Some(files) => files.clone(),
            None => jar.strip_suffix(".jar").map(|stem| vec![format!("{}-sources.jar", stem)]).unwrap_or_default(),
        }
    }

    /// Whether a candidate of `jar` has the file name `name`, which its document's path names.
    pub fn names(&self, jar: &str, name: &str) -> bool {
        self.candidates_of(jar).iter().any(|c| file_name(c) == name)
    }

    /// The sources jar of `jar`: the one chosen while its file is the one opened, else the first of
    /// its candidates that opens now; none where no candidate opens. A file replaced or changed since
    /// it was opened is opened again and judged anew, never by the reading of the one before.
    pub fn find(&mut self, jar: &str) -> Option<String> {
        if let Some(chosen) = self.chosen.get(jar).cloned() {
            if self.opened.get(&chosen).is_some_and(|sj| path_stamp(&chosen) == Some(sj.stamp)) {
                return Some(chosen);
            }
            self.chosen.retain(|_, c| *c != chosen);
            self.opened.remove(&chosen);
        }
        for c in self.candidates_of(jar) {
            // Has the file changed since it was last looked at: the only stamp read from the path.
            let Some(stamp) = path_stamp(&c) else { continue };
            if self.failed.get(&c) == Some(&stamp) {
                continue;
            }
            if self.opened.get(&c).is_some_and(|sj| sj.stamp != stamp) {
                self.opened.remove(&c);
            }
            if !self.opened.contains_key(&c) {
                match SourcesJar::open(&c) {
                    // Bytes rejected before, read through the handle of the archive opened: rejected as
                    // a read rejects, and told to the language server the same way.
                    Some(opened) if !self.rejected_hashes.is_empty() && opened.zip.sha1().is_ok_and(|h| self.rejected_hashes.contains(&h)) => {
                        self.failed.insert(c.clone(), opened.stamp);
                        REJECTED.lock().unwrap_or_else(|e| e.into_inner()).push((c, opened.stamp));
                        continue;
                    }
                    Some(opened) => {
                        self.failed.remove(&c);
                        self.opened.insert(c.clone(), opened);
                    }
                    None => {
                        self.failed.insert(c, stamp);
                        continue;
                    }
                }
            }
            self.chosen.insert(jar.to_string(), c.clone());
            return Some(c);
        }
        // A navigation into a library the language server named sources for, finding none: told to
        // the server with the session's next message, whose fetcher reads its store again.
        if self.candidates.contains_key(jar) {
            MISSED.lock().unwrap_or_else(|e| e.into_inner()).insert(jar.to_string());
        }
        None
    }

    /// The sources jar `sources` found damaged by a read (an entry's CRC or size not the directory's): no
    /// longer chosen for any jar, and a failed open from now on, as one that did not open is, until its
    /// size or modification time changes; the next candidate is the next `find`'s. The rejection is
    /// told to the language server with the session's next answer (`take_rejections`), whose fetcher's
    /// validation reads no entry's data and learns of a damaged one from it.
    pub fn reject(&mut self, sources: &str) {
        // Every jar that chose it.
        self.chosen.retain(|_, c| c != sources);
        // The archive read is the one opened: its stamp, never what stands at the path now (a
        // replacement there is judged anew).
        let Some(read) = self.opened.remove(sources) else { return };
        self.failed.insert(sources.to_string(), read.stamp);
        if let Ok(hash) = read.zip.sha1() {
            self.rejected_hashes.insert(hash);
        }
        REJECTED.lock().unwrap_or_else(|e| e.into_inner()).push((sources.to_string(), read.stamp));
    }

    /// The entry of `jar`'s sources jar that `recorded` names, with that jar's path, the entry and its
    /// text: the chosen candidate's while its entry reads, one whose entry fails to read rejected
    /// (`reject`) and the next candidate tried. None where no candidate holds a readable entry for the
    /// path; a path a sources jar holds no entry for (or several) rejects nothing.
    pub fn entry_text(&mut self, jar: &str, recorded: &str) -> Option<(String, usize, Arc<str>)> {
        loop {
            let sources = self.find(jar)?;
            let sj = self.opened.get_mut(&sources)?;
            let Match::Found(i) = sj.find(recorded) else { return None };
            match sj.text(i) {
                Ok(text) => return Some((sources, i, text)),
                // Each rejection passes over one more file until it changes: the loop ends.
                Err(_) => self.reject(&sources),
            }
        }
    }
}

/// The sources jars rejected at a read since the session's last answer, with the size and modification
/// time they had (`SourcesLookup::reject`).
static REJECTED: std::sync::Mutex<Vec<(String, Stamp)>> = std::sync::Mutex::new(Vec::new());

/// The rejections since the last call, as the answer's `rejected` field gives them to the language
/// server: `[{"path":..,"size":..,"secs":..,"nanos":..}]`, the modification time in seconds and
/// nanoseconds since the epoch (none where the file system keeps none); none when there are none.
pub fn take_rejections() -> Option<String> {
    let taken = std::mem::take(&mut *REJECTED.lock().unwrap_or_else(|e| e.into_inner()));
    if taken.is_empty() {
        return None;
    }
    use crate::lsp::json::Json;
    let items = taken
        .into_iter()
        .map(|(path, (size, modified))| {
            let mut fields = vec![("path".to_string(), Json::from(path)), ("size".to_string(), Json::Num(size as f64))];
            if let Some(d) = modified.and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()) {
                fields.push(("secs".to_string(), Json::Num(d.as_secs() as f64)));
                fields.push(("nanos".to_string(), Json::Num(d.subsec_nanos() as f64)));
            }
            Json::Obj(fields)
        })
        .collect();
    Some(Json::Arr(items).to_text())
}

/// The jars whose sources a lookup found nowhere since the session's last message, of those the
/// language server named candidates for (`SourcesLookup::find`).
static MISSED: std::sync::Mutex<std::collections::BTreeSet<String>> = std::sync::Mutex::new(std::collections::BTreeSet::new());

/// The misses since the last call, as a message's `missed` field gives them to the language server:
/// the jars' paths; none when there are none.
pub fn take_misses() -> Option<String> {
    let taken = std::mem::take(&mut *MISSED.lock().unwrap_or_else(|e| e.into_inner()));
    if taken.is_empty() {
        return None;
    }
    Some(crate::lsp::json::Json::Arr(taken.into_iter().map(Into::into).collect()).to_text())
}

/// The file name of a path, either separator.
pub fn file_name(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
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
            return self.with_loader(|w| w.loaded_mut().sources.entry_text(jar, recorded)).map(|(_, _, text)| Attached { text });
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

#[cfg(test)]
mod tests {
    use super::SourcesLookup;
    use std::fs;
    use std::path::Path;
    use std::time::{Duration, SystemTime};

    const SOURCES: &[u8] = include_bytes!("../../../tests/lsp/libs/lib/liba-1.0-sources.jar");

    fn place(path: &Path, bytes: &[u8], modified: SystemTime) {
        fs::write(path, bytes).unwrap();
        fs::File::options().write(true).open(path).unwrap().set_modified(modified).unwrap();
    }

    #[test]
    fn looks_for_a_jars_sources_in_order_and_again_after_a_miss() {
        let dir = std::env::temp_dir().join(format!("teq-sources-lookup-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let (beside, coursier, store) = (dir.join("beside.jar"), dir.join("coursier.jar"), dir.join("store.jar"));
        let jar = dir.join("x-1.0.jar").to_string_lossy().into_owned();
        let named = |p: &Path| p.to_string_lossy().into_owned();
        let mut lookup = SourcesLookup::default();
        lookup.candidates.insert(jar.clone(), vec![named(&beside), named(&coursier), named(&store)]);
        let t = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);
        // None there: a miss, looked for again at the next call, the store's found once placed.
        let missed = lookup.find(&jar);
        place(&store, SOURCES, t);
        let stored = lookup.find(&jar);
        // Another session's lookup: the first candidate that opens, a truncated one passed over.
        let mut later = SourcesLookup { candidates: lookup.candidates.clone(), ..Default::default() };
        place(&beside, &SOURCES[..SOURCES.len() - 30], t);
        place(&coursier, SOURCES, t);
        let past_truncated = later.find(&jar);
        let mut first = SourcesLookup { candidates: lookup.candidates.clone(), ..Default::default() };
        place(&beside, SOURCES, t);
        let in_order = first.find(&jar);
        fs::remove_dir_all(&dir).unwrap();
        assert_eq!(missed, None);
        assert_eq!(stored, Some(named(&store)));
        assert_eq!(past_truncated, Some(named(&coursier)));
        assert_eq!(in_order, Some(named(&beside)));
    }

    /// A candidate whose directory reads but whose entry lies past the file (its local header's
    /// offset): a failed open, the next candidate taken; alone, it is found once a whole archive
    /// replaces it by a rename, as a cache's publisher replaces a file.
    #[test]
    fn passes_over_a_damaged_archive_until_it_is_replaced() {
        let dir = std::env::temp_dir().join(format!("teq-sources-damaged-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let entry = SOURCES.windows(4).position(|w| w == [0x50, 0x4b, 0x01, 0x02]).unwrap();
        let mut damaged = SOURCES.to_vec();
        damaged[entry + 42..entry + 46].copy_from_slice(&(SOURCES.len() as u32 + 100).to_le_bytes());
        let (sibling, store) = (dir.join("sibling.jar"), dir.join("store.jar"));
        let named = |p: &Path| p.to_string_lossy().into_owned();
        let (one, two) = (dir.join("x-1.0.jar").to_string_lossy().into_owned(), dir.join("y-1.0.jar").to_string_lossy().into_owned());
        let mut lookup = SourcesLookup::default();
        lookup.candidates.insert(one.clone(), vec![named(&sibling), named(&store)]);
        lookup.candidates.insert(two.clone(), vec![named(&sibling)]);
        let t = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);
        place(&sibling, &damaged, t);
        place(&store, SOURCES, t);
        let past = lookup.find(&one);
        let alone = lookup.find(&two);
        let replacement = dir.join("replacement.jar");
        place(&replacement, SOURCES, t + Duration::from_secs(1));
        fs::rename(&replacement, &sibling).unwrap();
        let replaced = lookup.find(&two);
        fs::remove_dir_all(&dir).unwrap();
        assert_eq!(past, Some(named(&store)));
        assert_eq!(alone, None);
        assert_eq!(replaced, Some(named(&sibling)));
    }

    /// A chosen sources jar holds while its entry reads: one whose entry's CRC is not the directory's
    /// opens and is chosen, fails at the read, is rejected and passed over from then on, the next
    /// candidate's entry read in its place; alone, it is read again once replaced.
    #[test]
    fn rejects_a_chosen_archive_whose_entry_does_not_read() {
        let dir = std::env::temp_dir().join(format!("teq-sources-unread-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let entry = SOURCES.windows(4).position(|w| w == [0x50, 0x4b, 0x01, 0x02]).unwrap();
        let mut damaged = SOURCES.to_vec();
        damaged[entry + 16] ^= 0xff;
        let (sibling, store) = (dir.join("sibling.jar"), dir.join("store.jar"));
        let named = |p: &Path| p.to_string_lossy().into_owned();
        let (one, two) = (dir.join("x-1.0.jar").to_string_lossy().into_owned(), dir.join("y-1.0.jar").to_string_lossy().into_owned());
        let mut lookup = SourcesLookup::default();
        lookup.candidates.insert(one.clone(), vec![named(&sibling), named(&store)]);
        lookup.candidates.insert(two.clone(), vec![named(&sibling)]);
        let t = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);
        place(&sibling, &damaged, t);
        place(&store, SOURCES, t);
        let chosen = lookup.find(&one);
        let read = lookup.entry_text(&one, "liba/A.scala").map(|(path, _, text)| (path, text.contains("def make")));
        let after = (lookup.find(&one), lookup.find(&two));
        let absent = lookup.entry_text(&one, "liba/Absent.scala");
        let kept = lookup.find(&one);
        place(&sibling, SOURCES, t + Duration::from_secs(1));
        let replaced = lookup.entry_text(&two, "liba/A.scala").map(|(path, _, _)| path);
        fs::remove_dir_all(&dir).unwrap();
        assert_eq!(chosen, Some(named(&sibling)));
        assert_eq!(read, Some((named(&store), true)));
        assert_eq!(after, (Some(named(&store)), None));
        // A path the jar holds no entry for rejects nothing.
        assert_eq!((absent, kept), (None, Some(named(&store))));
        assert_eq!(replaced, Some(named(&sibling)));
    }

    /// A rejection belongs to the archive read: an archive whose one entry's CRC is damaged, opened and
    /// read for its other entry, then replaced whole by a rename, is opened again before the damaged
    /// entry is read and answers, rejecting nothing; damaged again, its rejection stamps the archive
    /// read, so the whole one renamed over it afterwards is judged anew.
    #[test]
    fn a_rejection_belongs_to_the_archive_read() {
        let dir = std::env::temp_dir().join(format!("teq-sources-read-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let whole: &[u8] = include_bytes!("../../../tests/lsp/libs/lib/libb-1.0-sources.jar");
        let mut damaged = whole.to_vec();
        let mut at = 0;
        while let Some(i) = damaged[at..].windows(4).position(|w| w == [0x50, 0x4b, 0x01, 0x02]) {
            let entry = at + i;
            let n = u16::from_le_bytes([damaged[entry + 28], damaged[entry + 29]]) as usize;
            if damaged[entry + 46..entry + 46 + n].ends_with(b"Cases.scala") {
                damaged[entry + 16] ^= 0xff;
            }
            at = entry + 46 + n;
        }
        let sibling = dir.join("libb-1.0-sources.jar");
        let jar = dir.join("libb-1.0.jar").to_string_lossy().into_owned();
        let mut lookup = SourcesLookup::default();
        let replace = |bytes: &[u8], seconds: u64| {
            let next = dir.join("next.jar");
            place(&next, bytes, SystemTime::UNIX_EPOCH + Duration::from_secs(seconds));
            fs::rename(&next, &sibling).unwrap();
        };
        replace(&damaged, 1_000_000);
        let other = lookup.entry_text(&jar, "libb/B.scala").is_some();
        replace(whole, 1_000_001);
        let repaired = lookup.entry_text(&jar, "libb/Cases.scala").is_some();
        let rejected_before = lookup.failed.clone();
        // Damaged again and read: rejected at the damaged archive's stamp (the one told to the server
        // too); the whole one renamed over it afterwards is not that archive.
        replace(&damaged, 1_000_002);
        let unread = lookup.entry_text(&jar, "libb/Cases.scala");
        let rejected = lookup.failed.get(&sibling.to_string_lossy().into_owned()).copied();
        replace(whole, 1_000_003);
        let judged_anew = lookup.entry_text(&jar, "libb/Cases.scala").is_some();
        fs::remove_dir_all(&dir).unwrap();
        assert!(other && repaired && rejected_before.is_empty());
        assert_eq!((unread.is_none(), rejected), (true, Some((damaged.len() as u64, Some(SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_002))))));
        assert!(judged_anew);
    }

    #[test]
    fn tries_a_file_that_failed_to_open_once_it_changed() {
        let dir = std::env::temp_dir().join(format!("teq-sources-failed-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let sources = dir.join("x-1.0-sources.jar");
        let jar = dir.join("x-1.0.jar").to_string_lossy().into_owned();
        let mut lookup = SourcesLookup::default();
        let t = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);
        // Beside the jar by default: a damaged one fails to open.
        place(&sources, &[b'x'; 221], t);
        let damaged = lookup.find(&jar);
        // Replaced by a whole one of the same size and modification time: passed over as it was.
        assert_eq!(SOURCES.len(), 221);
        place(&sources, SOURCES, t);
        let unchanged = lookup.find(&jar);
        // Its modification time changed: opened.
        place(&sources, SOURCES, t + Duration::from_secs(1));
        let changed = lookup.find(&jar);
        // Its size changed: opened too.
        let mut sized = SourcesLookup::default();
        place(&sources, &SOURCES[..SOURCES.len() / 2], t);
        let short = sized.find(&jar);
        place(&sources, SOURCES, t);
        let whole = sized.find(&jar);
        fs::remove_dir_all(&dir).unwrap();
        assert_eq!((damaged, unchanged), (None, None));
        assert_eq!(changed, Some(sources.to_string_lossy().into_owned()));
        assert_eq!((short, whole), (None, Some(sources.to_string_lossy().into_owned())));
        assert!(lookup.names(&jar, "x-1.0-sources.jar") && !lookup.names(&jar, "x-2.0-sources.jar"));
    }
}
