//! The documents a language server's client is shown a library's source in (docs/TARGETS.md,
//! "The language server"). A jar's source is an entry
//! of the sources jar beside it, written once, read-only, under the cache at a path named by
//! the sources jar's identity (a hash of its entries' names, CRCs and sizes) and the entry, so
//! that every session reading one sources jar names one document, and two versions of a jar two;
//! a product's source is the producer's own file, which the product's manifest names. A library
//! declaration is located by its definition's explicit position in the pickle (its name's
//! point), converted through that text; a query on such a document is answered from the
//! occurrence index of the TASTy files behind it (`occurrences.rs`), those files found through
//! the jar's source map and entered, the program's reach notwithstanding.

use super::super::index::Target;
use super::super::Worker;
use super::places::{Match, SourcesJar};
use crate::source::Span;
use crate::symbols::{ClassKind, Owner, SymKind};
use crate::types::ClassId;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// A library source as the client is shown it.
pub struct Document {
    pub path: PathBuf,
    pub text: Arc<str>,
}

/// Where the documents of the sources jars are written, canonical.
pub fn documents_dir() -> Option<PathBuf> {
    let dir = crate::jarcache::dir()?.join("attached");
    std::fs::create_dir_all(&dir).ok()?;
    crate::source::canonicalize(&dir).ok()
}

/// Whether a path names a document of a sources jar or of the std.
pub fn is_document(path: &Path) -> bool {
    documents_dir().map_or(false, |d| path.starts_with(&d))
}

/// Whether a path names a document of the std (`std_document_path`), this binary's or another's.
pub fn is_std_document(path: &Path) -> bool {
    documents_dir().and_then(|d| Some(path.strip_prefix(&d).ok()?.components().next()?.as_os_str().to_str()?.starts_with("std-"))).unwrap_or(false)
}

/// The document of an embedded std file (`<std>/core.scala`), `<cache>/attached/std-<identity>/
/// core.scala`, the identity a hash of the binary's std sources (`build.rs`): the sessions of
/// one binary's std name one document per file, whatever their program, and another std's text
/// is another document. Written by the session that first answers a location into it
/// (`write_document`).
pub fn std_document_path(std_path: &str) -> Option<PathBuf> {
    static DIR: std::sync::OnceLock<Option<PathBuf>> = std::sync::OnceLock::new();
    let dir = DIR.get_or_init(|| documents_dir().map(|d| d.join(format!("std-{}", crate::stdindex::STD_IDENTITY))));
    Some(dir.as_ref()?.join(std_path.strip_prefix("<std>/")?))
}

/// The paths `write_document` refused since the last answer, each a note of the answer that
/// follows (`watch.rs`), which the language server logs.
static REFUSED: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());

/// The refusals since the last call.
pub fn take_refusals() -> Vec<String> {
    std::mem::take(&mut *REFUSED.lock().unwrap_or_else(|e| e.into_inner()))
}

/// Records that what stands at a document's path is no regular file, left as it is.
fn refuse(path: &Path) {
    let note = format!("not a regular file, so no document is written there: {}", path.display());
    eprintln!("teq: {}", note);
    REFUSED.lock().unwrap_or_else(|e| e.into_inner()).push(note);
}

/// The bytes of the document at `path` as they are, a regular file's alone: what is no regular
/// file is never opened (a pipe would block) and refused; `None` for that and for no file.
pub fn document_bytes(path: &Path) -> Option<Vec<u8>> {
    match std::fs::symlink_metadata(path) {
        Ok(m) if m.file_type().is_file() => std::fs::read(path).ok(),
        Ok(_) => {
            refuse(path);
            None
        }
        Err(_) => None,
    }
}

/// Makes the document at `path` hold `text`, read-only, the one its locations are answered on:
/// a file already there is taken only when its bytes are the text, and one that is missing,
/// truncated or altered is replaced through a file of its own renamed into place, so that a reader
/// never sees it half written. What is not a regular file at the path (a directory, a link, a
/// pipe) is left as it is, never read, the path told with the answer (`take_refusals`). Whether
/// the document holds the text: a location is declined where it does not.
pub fn write_document(path: &Path, text: &str) -> bool {
    if std::fs::symlink_metadata(path).map_or(false, |m| !m.file_type().is_file()) {
        refuse(path);
        return false;
    }
    let holds = || std::fs::read(path).map_or(false, |bytes| bytes == text.as_bytes());
    if holds() {
        return true;
    }
    let write = || -> Option<()> {
        use std::io::Write;
        std::fs::create_dir_all(path.parent()?).ok()?;
        // Made here and nowhere else: whatever had the name (a file a session of the same
        // process id left, a link) goes first, itself and not what it names.
        let partial = path.with_extension(format!("part{}", std::process::id()));
        let create = || std::fs::OpenOptions::new().write(true).create_new(true).open(&partial);
        let mut file = match create() {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                std::fs::remove_file(&partial).ok()?;
                create().ok()?
            }
            Err(_) => return None,
        };
        let written = file.write_all(text.as_bytes()).and_then(|_| {
            let mut perms = file.metadata()?.permissions();
            perms.set_readonly(true);
            file.set_permissions(perms)
        });
        drop(file);
        if written.is_err() {
            let _ = std::fs::remove_file(&partial);
            return None;
        }
        if std::fs::rename(&partial, path).is_err() {
            // A damaged document left read-only, which Windows does not replace: writable first.
            if let Some(m) = std::fs::symlink_metadata(path).ok().filter(|m| m.file_type().is_file()) {
                let mut perms = m.permissions();
                perms.set_readonly(false);
                let _ = std::fs::set_permissions(path, perms);
            }
            if std::fs::rename(&partial, path).is_err() {
                let _ = std::fs::remove_file(&partial);
                return None;
            }
        }
        Some(())
    };
    let _ = write();
    // What another session renamed into place meanwhile is the same text.
    holds()
}

impl SourcesJar {
    /// What names the jar's documents: a hash of its entries' names, CRCs and sizes.
    pub fn identity(&self) -> String {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        let mut eat = |bytes: &[u8]| {
            for &b in bytes {
                h = (h ^ b as u64).wrapping_mul(0x0100_0000_01b3);
            }
        };
        for i in 0..self.len() {
            let (name, crc, size) = self.entry_fields(i);
            eat(name.as_bytes());
            eat(&crc.to_le_bytes());
            eat(&size.to_le_bytes());
        }
        format!("{:016x}", h)
    }
}

/// The byte offset in `text` of the UTF-16 offset `units`.
fn byte_offset(text: &str, units: u32) -> usize {
    let mut n = 0u32;
    for (i, c) in text.char_indices() {
        if n >= units {
            return i;
        }
        n += c.len_utf16() as u32;
    }
    text.len()
}

/// The span of `name` at `at` in `text`, inside backquotes or not: the whole token, so that
/// `twice` is not found at `twiceMore`.
pub(super) fn name_span(text: &str, at: usize, name: &str) -> Option<Span> {
    let end = at.checked_add(name.len())?;
    if text.get(at..end) == Some(name) && token_ends(name, text.get(end..).unwrap_or("")) {
        return Some(Span::new(at as u32, end as u32));
    }
    if text.get(at..at + 1) == Some("`") && text.get(at + 1..end + 1) == Some(name) && text.get(end + 1..end + 2) == Some("`") {
        return Some(Span::new(at as u32 + 1, (end + 1) as u32));
    }
    None
}

/// Whether the token `name` ends where `rest` begins: an identifier at a character no
/// identifier holds, an operator at one no operator holds, `foo_+` (an identifier ending in an
/// operator after `_`) at one neither holds.
fn token_ends(name: &str, rest: &str) -> bool {
    let Some(next) = rest.chars().next() else { return true };
    let ident = |c: char| c.is_alphanumeric() || c == '_' || c == '$';
    let op = |c: char| !c.is_alphanumeric() && !c.is_whitespace() && !"()[]{}`'\".,;".contains(c) && c != '_' && c != '$';
    match name.chars().last() {
        Some(last) if op(last) => !op(next),
        _ => !ident(next),
    }
}

impl<'a> Worker<'a> {
    /// The path of the document of the source `recorded` of the loaded file `file`, with the
    /// sources jar's entry it shows where it is a jar's: `None` without a sources jar or a
    /// product's source on disk, and for a recorded path the sources jar holds no entry or
    /// several entries for.
    fn document_path(&mut self, file: u32, recorded: &str) -> Option<(PathBuf, Option<(String, usize)>)> {
        let jar = {
            let loaded = self.loaded.as_ref()?;
            loaded.cp.paths[loaded.file(file).cp.jar as usize].clone()
        };
        if !jar.ends_with(".jar") {
            let (source, _) = self.product_source_of(file)?;
            return Some((crate::source::canonicalize(&source).ok()?, None));
        }
        let sources = super::places::sources_jar_of(&jar)?;
        let dir = documents_dir()?;
        self.with_loader(|w| {
            let loaded = w.loaded_mut();
            let sj = loaded.sources_jars.entry(sources.clone()).or_insert_with(|| SourcesJar::open(&sources));
            let sj = sj.as_mut()?;
            let Match::Found(i) = sj.find(recorded) else { return None };
            let jar_name = sources.rsplit(['/', '\\']).next().unwrap_or(&sources).to_string();
            Some((dir.join(sj.identity()).join(jar_name).join(sj.entry_name(i)), Some((sources.clone(), i))))
        })
    }

    /// The document of the source `recorded` of the loaded file `file`, a jar's written on the
    /// first call and made whole again where it was truncated or altered since (`write_document`).
    pub(in crate::typer) fn source_document(&mut self, file: u32, recorded: &str) -> Option<Document> {
        let (path, entry) = self.document_path(file, recorded)?;
        let Some((sources, i)) = entry else {
            let (_, text) = self.product_text(file)?;
            return Some(Document { path, text });
        };
        let text = self.with_loader(|w| w.loaded_mut().sources_jars.get_mut(&sources)?.as_mut()?.text(i))?;
        if !write_document(&path, &text) {
            return None;
        }
        Some(Document { path, text })
    }

    /// The loaded file, the address and the name of a library target's definition.
    pub(in crate::typer) fn library_definition(&mut self, t: Target) -> Option<(u32, crate::tasty::tree::Addr, crate::intern::Name)> {
        let loaded = self.loaded.as_ref()?;
        match t {
            Target::Sym(s) => match loaded.syms.get(&s).copied() {
                Some(ls) => Some((ls.file, ls.addr, self.syms.sym(s).name)),
                // A parameter, which the signature decoding entered forward alone, or a
                // constructor parameter's accessor, entered from the constructor's clause: found
                // among the tables' entries, once.
                None if matches!(self.syms.sym(s).kind, SymKind::Param | SymKind::Val | SymKind::Var) => {
                    let found = (0..loaded.tables.len()).find_map(|i| loaded.tables[i].terms.iter().find(|(_, &x)| x == s).map(|(&a, _)| (i as u32, a)))?;
                    self.with_loader(|w| w.loaded_mut().syms.insert(s, super::LSym { file: found.0, addr: found.1, conversion: false }));
                    Some((found.0, found.1, self.syms.sym(s).name))
                }
                // The `apply` of a function class the typer makes (`Function1`, `ContextFunction1`):
                // the jars' class's member of its name.
                None => match self.syms.sym(s).owner {
                    Owner::Class(c) if self.is_function_class(c) || self.is_context_function_class(c) => self.builtin_jar_member(c, s),
                    _ => None,
                },
            },
            Target::Class(c) => match loaded.classes.get(&c).copied() {
                Some(lc) => Some((lc.file, lc.addr, self.syms.class(c).name)),
                // A builtin class (`Int`, `AnyRef`) stands for the jar's class of its name,
                // whose file is opened for the place alone, nothing of it entered.
                None if self.syms.class(c).kind == ClassKind::Builtin || self.is_function_class(c) || self.is_context_function_class(c) => self.builtin_jar_definition(c),
                None => None,
            },
            Target::Alias(a) => {
                let la = loaded.aliases.get(&a).copied()?;
                Some((la.file, la.addr, self.syms.aliases[a.idx()].name))
            }
            // A type parameter a document's index met (`occurrences.rs`).
            Target::TParam(p) => {
                let &(file, addr) = loaded.tparam_defs.get(&p)?;
                Some((file, addr, self.syms.tparam(p).name))
            }
            Target::Node(_) => None,
        }
    }

    /// The TASTy file of the jars that defines a class of the builtin layer's name in its
    /// package, and the address of that definition: opened, not entered.
    fn builtin_jar_definition(&mut self, c: ClassId) -> Option<(u32, crate::tasty::tree::Addr, crate::intern::Name)> {
        let (name, pkg) = match self.syms.class(c).owner {
            Owner::Package(p) => (self.syms.class(c).name, p),
            _ => return None,
        };
        let slot = self.loaded.as_ref()?.slot(pkg)?;
        let stem = crate::classpath::encode_name(self.name_ref(name)).into_owned();
        let f = *self.loaded.as_ref()?.pkg_names(slot).get(stem.as_str())?;
        let file = self.open_file(f)?;
        let tasty = self.tasty(file);
        let wanted = self.name_ref(name).to_string();
        let addr = crate::tasty::tree::index_top_level(&tasty)
            .into_iter()
            .map(|t| t.entry)
            .find(|e| e.tag == crate::tasty::tags::TYPEDEF && e.is_class && !tasty.is_object_class(e.name) && tasty.name(e.name) == wanted)
            .map(|e| e.addr)?;
        Some((file, addr, name))
    }

    /// The member `m` of the class `c` of the builtin layer, in the jars' class of its name
    /// (`builtin_jar_definition`): the definition of the name its template indexes, where one
    /// alone has it.
    fn builtin_jar_member(&mut self, c: ClassId, m: crate::types::SymId) -> Option<(u32, crate::tasty::tree::Addr, crate::intern::Name)> {
        let (file, class, _) = self.builtin_jar_definition(c)?;
        let tasty = self.tasty(file);
        let wanted = self.name_ref(self.syms.sym(m).name).to_string();
        let members = crate::tasty::tree::Decoder::new(&tasty).class_sig(class).index.members;
        let mut named = members.iter().filter(|e| e.tag != crate::tasty::tags::TYPEDEF && tasty.name(e.name) == wanted);
        match (named.next(), named.next()) {
            (Some(e), None) => Some((file, e.addr, self.syms.sym(m).name)),
            _ => None,
        }
    }

    /// Where a library target is declared: its source's document and the span of its name, from
    /// the definition's position in the pickle; `None` without a document or a position.
    pub(in crate::typer) fn library_declaration(&mut self, t: Target) -> Option<(Document, Span)> {
        let (file, addr, name) = self.library_definition(t)?;
        let positions = self.file_positions(file);
        let pos = positions.section.as_ref()?.entry(addr)?;
        let recorded = positions.source_path(addr, None)?;
        let doc = self.source_document(file, &recorded)?;
        let at = byte_offset(&doc.text, pos.point.unwrap_or(pos.start));
        // A text that does not spell the name at its point is not the one pickled.
        let span = name_span(&doc.text, at, &self.name_ref(name).to_string())?;
        Some((doc, span))
    }

    /// Per jar of the class path, its TASTy entries by the source path their pickles record
    /// (the primary source of the `Positions` section, else `SOURCEFILEattr`), read from the
    /// jar once per session on the first document of the jar asked about, nothing of the files
    /// kept; measured on stderr under `TEQ_CLASSPATH_DETAIL`.
    fn source_map(&mut self, jar: u16) -> Arc<crate::intern::FxMap<String, Vec<crate::classpath::CpFile>>> {
        if let Some(m) = self.loaded.as_ref().and_then(|l| l.source_maps.get(&jar)) {
            return m.clone();
        }
        let start = std::time::Instant::now();
        let mut map: crate::intern::FxMap<String, Vec<crate::classpath::CpFile>> = crate::intern::FxMap::default();
        let (mut files, mut bytes) = (0usize, 0usize);
        if let Some(loaded) = self.loaded.as_ref() {
            for f in loaded.cp.files_of(jar) {
                let Ok(raw) = loaded.cp.raw_bytes(f) else { continue };
                files += 1;
                bytes += raw.len();
                let Ok(tasty) = crate::tasty::TastyFile::parse(raw) else { continue };
                let recorded = crate::tasty::positions::Section::read(&tasty)
                    .and_then(|s| s.primary())
                    .map(|n| tasty.name(n))
                    .or_else(|| super::places::source_file_attr(&tasty));
                if let Some(r) = recorded {
                    map.entry(r).or_default().push(f);
                }
            }
        }
        let map = Arc::new(map);
        let ms = start.elapsed().as_secs_f64() * 1000.0;
        if self.loaded.as_ref().map_or(false, |l| l.detail) {
            eprintln!("source map of {}: {} files, {} bytes, {} sources, {:.2} ms", self.loaded.as_ref().map_or("?".to_string(), |l| l.cp.paths[jar as usize].clone()), files, bytes, map.len(), ms);
        }
        self.with_loader(|w| {
            let loaded = w.loaded_mut();
            loaded.source_maps.insert(jar, map.clone());
            loaded.source_map_stats.push((jar, files, bytes, ms));
        });
        map
    }

    /// The jar of the class path a document's path names: the one whose sources jar has the
    /// name and the identity the path's first two components give.
    fn jar_of_document(&mut self, rel: &Path) -> Option<u16> {
        let mut parts = rel.components();
        let identity = parts.next()?.as_os_str().to_str()?.to_string();
        let name = parts.next()?.as_os_str().to_str()?.to_string();
        let paths: Vec<String> = self.loaded.as_ref()?.cp.paths.clone();
        for (j, jar) in paths.iter().enumerate() {
            if !jar.ends_with(".jar") {
                continue;
            }
            let Some(sources) = super::places::sources_jar_of(jar) else { continue };
            if sources.rsplit(['/', '\\']).next() != Some(name.as_str()) {
                continue;
            }
            let same = self.with_loader(|w| {
                let loaded = w.loaded_mut();
                let sj = loaded.sources_jars.entry(sources.clone()).or_insert_with(|| SourcesJar::open(&sources));
                sj.as_ref().map_or(false, |sj| sj.identity() == identity)
            });
            if same {
                return Some(j as u16);
            }
        }
        None
    }

    /// Opens and enters the TASTy files of the jar a document names whose recorded source is
    /// the document's entry, those the program never reached included, so that the document
    /// stands for every class of its source (`Nil` of `List.scala`).
    fn enter_document_files(&mut self, rel: &Path) {
        let Some(jar) = self.jar_of_document(rel) else { return };
        let entry: Vec<&str> = rel.components().skip(2).filter_map(|c| c.as_os_str().to_str()).collect();
        let entry = entry.join("/");
        let Some(file_name) = entry.rsplit('/').next().map(str::to_string) else { return };
        let map = self.source_map(jar);
        let jar_path = self.loaded.as_ref().map(|l| l.cp.paths[jar as usize].clone());
        let Some(sources) = jar_path.as_deref().and_then(super::places::sources_jar_of) else { return };
        let mut wanted: Vec<crate::classpath::CpFile> = Vec::new();
        for (recorded, files) in map.iter() {
            if recorded.rsplit('/').next() != Some(file_name.as_str()) {
                continue;
            }
            let found = self.with_loader(|w| {
                let loaded = w.loaded_mut();
                let sj = loaded.sources_jars.get(&sources).and_then(|s| s.as_ref())?;
                match sj.find(recorded) {
                    Match::Found(i) if sj.entry_name(i) == entry => Some(()),
                    _ => None,
                }
            });
            if found.is_some() {
                wanted.extend(files.iter().copied());
            }
        }
        wanted.sort_by_key(|f| (f.jar, f.entry));
        for f in wanted {
            if !self.file_entered(f) {
                self.enter_found_file(f);
            }
        }
    }

    /// The loaded files a document stands for: those of the jar beside the sources jar the
    /// path names (`<identity>/<sources jar>/<entry>`) whose recorded source is the entry,
    /// opened and entered where the program never reached them, or of the product directories
    /// for another path, whose source's document is the path. Kept until the loader reads
    /// another file.
    pub(in crate::typer) fn document_files(&mut self, path: &Path) -> Vec<u32> {
        let Some(loaded) = self.loaded.as_ref() else { return Vec::new() };
        let read = loaded.files.len();
        if let Some(state) = loaded.documents.get(path) {
            if state.read == read {
                return state.files.clone();
            }
        }
        let jar_document = documents_dir().and_then(|d| path.strip_prefix(&d).ok().map(Path::to_path_buf));
        if let Some(rel) = &jar_document {
            if self.loaded.as_ref().map_or(false, |l| !l.documents.contains_key(path)) {
                self.enter_document_files(rel);
            }
        }
        let Some(loaded) = self.loaded.as_ref() else { return Vec::new() };
        let read = loaded.files.len();
        let mut candidates: Vec<u32> = Vec::new();
        for (i, f) in loaded.files.as_slice().iter().enumerate() {
            let jar = &loaded.cp.paths[f.cp.jar as usize];
            let fits = match &jar_document {
                Some(rel) => {
                    let sources = jar.strip_suffix(".jar").map(|s| format!("{}-sources.jar", s));
                    let named = sources.as_deref().and_then(|s| s.rsplit(['/', '\\']).next());
                    rel.components().nth(1).and_then(|c| c.as_os_str().to_str()) == named
                }
                None => !jar.ends_with(".jar"),
            };
            if fits {
                candidates.push(i as u32);
            }
        }
        let mut found = Vec::new();
        for file in candidates {
            let positions = self.file_positions(file);
            let Some(recorded) = positions.source_path(0, None) else { continue };
            if self.document_path(file, &recorded).map_or(false, |(p, _)| p == path) {
                found.push(file);
            }
        }
        self.with_loader(|w| {
            let state = w.loaded_mut().documents.entry(path.to_path_buf()).or_default();
            state.read = read;
            if state.files != found {
                state.index = None;
            }
            state.files = found.clone();
        });
        found
    }

    /// The text of the document the loaded files `files` show, as `source_document` checks it.
    pub(in crate::typer) fn document_text(&mut self, files: &[u32]) -> Option<Arc<str>> {
        let &first = files.first()?;
        let recorded = self.file_positions(first).source_path(0, None)?;
        Some(self.source_document(first, &recorded)?.text)
    }
}

#[cfg(test)]
mod tests {
    use super::name_span;

    #[test]
    fn a_name_is_its_whole_token() {
        let text = "  def twiceMore(x: Int) = x\n  def twice(x: Int) = x\n  def `type`(x: Int) = x\n  def `typed` = 1\n  def ++(i: Int) = i\n  def +++(i: Int) = i";
        let at = |s: &str| text.find(s).unwrap();
        assert_eq!(name_span(text, at("twiceMore"), "twice"), None);
        assert!(name_span(text, at("twice("), "twice").is_some());
        assert!(name_span(text, at("`type`"), "type").is_some());
        assert_eq!(name_span(text, at("`typed`"), "type"), None);
        assert!(name_span(text, at("++("), "++").is_some());
        assert_eq!(name_span(text, at("+++"), "++"), None);
    }
}
