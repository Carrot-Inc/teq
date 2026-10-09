//! The occurrence index of a library document: what
//! every written identifier of a sources jar's entry names, read from the pickled trees of the
//! TASTy files behind the document, never from its text. The index is built the first time a
//! document is asked about and kept with the document (`Loaded::documents`) while the files
//! behind it are the same; the check and build paths never build one.
//!
//! The traversal is the one `tasty::positions::Spans` makes of every tree of a file, by address:
//! it gives each tree its tag, the span scalac's unpickler reconstructs for it and its effective
//! source. An occurrence is a tree whose tag refers to something (an identifier, a selection, a
//! type or term reference by symbol or by name, `this`, `super`, an import's selector) and a
//! definition's name; it is decoded at its address and resolved through the loader's tables
//! (`types.rs`: packages, classes, members, aliases, parameters), entering what is not yet. An
//! occurrence whose source is not the document's (an inline expansion from another file, a
//! source switch), one inside the expansion of an inline call (a copy of trees that stand
//! elsewhere), and one whose span does not spell the written name in the document's text (a
//! synthetic tree, an inferred type) make no record. Where trees share a position the innermost
//! written identifier wins, an explicit point before a reconstructed start, a use before a
//! declaration.
//!
//! A definition of the document that the tables know (a class, a member, an alias, a parameter,
//! a type parameter) is its `Target`; one they do not (a local val or def, a pattern's binder, a
//! type lambda's parameter, a refinement's member, a local class) is `DocTarget::Local`, the
//! address of its definition in its file, located in the document by its definition's position.
//!
//! `index-stats` lists the documents indexed with their records, bytes and build times;
//! `TEQ_DOCUMENT_INDEX_DUMP=1` in the child's environment prints every record and every
//! occurrence left unresolved on its stderr as an index is built.

use super::super::index::{Kind, Target};
use super::super::Worker;
use super::attach::name_span;
use super::types::{MapCx, Scope};
use super::LSym;
use crate::intern::{FxMap, Name};
use crate::source::Span;
use crate::symbols::*;
use crate::tasty::tags::*;
use crate::tasty::terms::{Scope as TermScope, Stat, Term, TermDecoder, TermKind, Walk};
use crate::tasty::tree::{Addr, Const, Decoder, TType};
use crate::tasty::{NameRef, TastyFile};
use crate::types::*;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

/// What an occurrence names: a symbol of the session's tables, or a definition of the document
/// that has none, by the loaded file and the address of its definition.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub enum DocTarget {
    Shared(Target),
    Local(u32, Addr),
}

#[derive(Clone, Copy, Debug)]
pub struct DocRecord {
    /// The written name's span, in bytes of the document's text.
    pub span: Span,
    pub target: DocTarget,
    pub kind: Kind,
    /// The innermost definition the occurrence stands in: a member, a class (for its parents
    /// and its template's statements), a local definition.
    pub owner: Option<DocTarget>,
}

pub struct DocIndex {
    pub path: PathBuf,
    pub text: Arc<str>,
    /// The loaded files the index was built from, in the order `document_files` gave them.
    pub files: Vec<u32>,
    /// By the span's start and end; one record per span.
    pub records: Vec<DocRecord>,
    /// The records that use a target (every kind but `Def`), by target.
    pub uses: FxMap<DocTarget, Vec<u32>>,
    /// The record of each definition's name in this document.
    pub defs: FxMap<DocTarget, u32>,
    pub stats: DocStats,
}

#[derive(Default, Clone, Copy, Debug)]
pub struct DocStats {
    pub build_ms: f64,
    /// The trees of the files behind the document.
    pub trees: usize,
    /// The trees that refer to something or declare it, in the document's source.
    pub candidates: usize,
    /// Of those, the ones no target was found for.
    pub unresolved: usize,
    /// Of those, the ones whose span does not spell the written name.
    pub unspelled: usize,
}

impl DocIndex {
    /// The bytes the index holds.
    pub fn held(&self) -> usize {
        let table = |n: usize, k: usize| n * k * 2;
        std::mem::size_of::<DocIndex>()
            + self.records.capacity() * std::mem::size_of::<DocRecord>()
            + table(self.uses.len(), std::mem::size_of::<(DocTarget, Vec<u32>)>())
            + self.uses.values().map(|v| v.capacity() * 4).sum::<usize>()
            + table(self.defs.len(), std::mem::size_of::<(DocTarget, u32)>())
            + self.files.capacity() * 4
    }

    /// The innermost record whose span holds `offset`.
    pub fn at(&self, offset: u32) -> Option<DocRecord> {
        let upto = self.records.partition_point(|r| r.span.start <= offset);
        let mut best: Option<DocRecord> = None;
        for r in self.records[..upto].iter().rev().take(64) {
            if r.span.end < offset {
                continue;
            }
            if best.map_or(true, |b| r.span.end - r.span.start < b.span.end - b.span.start) {
                best = Some(*r);
            }
        }
        best
    }
}

/// What the index of a document is while it is built.
struct Candidate {
    file: u32,
    addr: Addr,
    span: Span,
    target: DocTarget,
    kind: Kind,
    /// 0 a use at an explicit point, 1 a use at a reconstructed start, 2 a declaration.
    prio: u8,
}

/// A definition's tree as a range of addresses, for the owners of the occurrences in it and
/// the methods whose parameters a reference names: `owner` where the calls inside it are its
/// (a member, a class, a local method), not a local val's.
#[derive(Clone, Copy)]
struct DefRange {
    file: u32,
    start: Addr,
    end: Addr,
    target: DocTarget,
    owner: bool,
}

/// The range of a class's tree inside which an opaque type it defines is transparent: its
/// right-hand side is seen through there, its bound outside (`Worker::compute_transparent` for
/// a program's sources).
#[derive(Clone, Copy)]
struct OpaqueScope {
    start: Addr,
    end: Addr,
    opaque: ClassId,
}

/// What resolves a candidate tree once its span is known to spell its name: a type or type
/// tree at an address (its kind a type's where the tree is one), a `SELECTin`, a reference
/// by the address of a definition, a definition, or what the term pass found.
enum Pending {
    Type(Addr, bool),
    SelectIn(NameRef, Addr, Addr),
    Local(Addr, bool),
    Def(Addr),
    Resolved(Option<DocTarget>, bool),
}

/// The byte offset of each UTF-16 offset of `text`, one entry past its end.
fn utf16_table(text: &str) -> Vec<u32> {
    let mut table = Vec::with_capacity(text.len() + 1);
    for (i, c) in text.char_indices() {
        for _ in 0..c.len_utf16() {
            table.push(i as u32);
        }
    }
    table.push(text.len() as u32);
    table
}

/// Whether the tag is one of a definition whose name is written.
fn is_definition(tag: u8) -> bool {
    matches!(tag, VALDEF | DEFDEF | TYPEDEF | PARAM | TYPEPARAM | BIND)
}

impl<'a> Worker<'a> {
    /// The occurrence index of the document at `path`, built on the first call and kept while
    /// the files behind the document are the same and its text is the one the index was built
    /// from, the document made whole again first (`document_text`, as a declaration's document
    /// is); `None` for a path no session's file maps to and a document that cannot be made whole.
    pub(in crate::typer) fn document_index(&mut self, path: &Path) -> Option<Arc<DocIndex>> {
        let files = self.document_files(path);
        if files.is_empty() {
            return None;
        }
        let text = self.document_text(&files)?;
        if let Some(ix) = self.loaded.as_ref()?.documents.get(path).and_then(|s| s.index.clone()) {
            if ix.files == files && ix.text == text {
                return Some(ix);
            }
        }
        let ix = Arc::new(self.build_document_index(path, &files, text));
        self.with_loader(|w| {
            if let Some(s) = w.loaded_mut().documents.get_mut(path) {
                s.index = Some(ix.clone());
            }
        });
        Some(ix)
    }

    /// The indexes built so far.
    pub(in crate::typer) fn document_indexes(&self) -> Vec<Arc<DocIndex>> {
        let Some(loaded) = self.loaded.as_ref() else { return Vec::new() };
        let mut out: Vec<Arc<DocIndex>> = loaded.documents.values().filter_map(|s| s.index.clone()).collect();
        out.sort_by(|a, b| a.path.cmp(&b.path));
        out
    }

    /// The document the loaded file `file` is shown in, indexed.
    pub(in crate::typer) fn document_index_of_file(&mut self, file: u32) -> Option<Arc<DocIndex>> {
        let recorded = self.file_positions(file).source_path(0, None)?;
        let doc = self.source_document(file, &recorded)?;
        self.document_index(&doc.path)
    }

    fn build_document_index(&mut self, path: &Path, files: &[u32], text: Arc<str>) -> DocIndex {
        let start = Instant::now();
        let units = utf16_table(&text);
        let mut stats = DocStats::default();
        let mut candidates: Vec<Candidate> = Vec::new();
        // Per file the definitions by their tree's range, for the owners (a local val's
        // initializer belongs to the method or class it stands in: it is no owner itself).
        let mut defs: Vec<DefRange> = Vec::new();
        for &f in files {
            self.complete_document_file(f);
            self.index_file(f, &text, &units, &mut candidates, &mut defs, &mut stats);
        }
        defs.sort_by_key(|d| (d.file, d.start, std::cmp::Reverse(d.end)));
        let owner_of = |file: u32, addr: Addr| -> Option<DocTarget> {
            let upto = defs.partition_point(|d| (d.file, d.start) < (file, addr));
            defs[..upto].iter().rev().take_while(|d| d.file == file).find(|d| d.owner && d.end > addr).map(|d| d.target)
        };
        let mut records: Vec<(DocRecord, u8)> = candidates
            .iter()
            .map(|c| (DocRecord { span: c.span, target: c.target, kind: c.kind, owner: owner_of(c.file, c.addr) }, c.prio))
            .collect();
        records.sort_by_key(|(r, prio)| (r.span.start, r.span.end, *prio));
        records.dedup_by(|later, kept| later.0.span == kept.0.span);
        let records: Vec<DocRecord> = records.into_iter().map(|(r, _)| r).collect();
        let mut uses: FxMap<DocTarget, Vec<u32>> = FxMap::default();
        let mut defs_at: FxMap<DocTarget, u32> = FxMap::default();
        for (i, r) in records.iter().enumerate() {
            if r.kind == Kind::Def {
                defs_at.entry(r.target).or_insert(i as u32);
            } else {
                uses.entry(r.target).or_default().push(i as u32);
            }
        }
        stats.build_ms = start.elapsed().as_secs_f64() * 1000.0;
        if std::env::var_os("TEQ_DOCUMENT_INDEX_DUMP").is_some() {
            let lines = crate::index::Lines::new(&text);
            for r in &records {
                let shown = match r.target {
                    DocTarget::Shared(t) => self.describe(t),
                    DocTarget::Local(f, a) => format!("local {}@{}", f, a),
                };
                let at = lines.position(r.span.start, crate::watch::Positions::Utf16).to_text();
                eprintln!("  {} {:?} {} -> {}", at, r.kind, &text[r.span.start as usize..r.span.end as usize], shown);
            }
        }
        if self.loaded.as_ref().map_or(false, |l| l.detail) || std::env::var_os("TEQ_DOCUMENT_INDEX_DUMP").is_some() {
            eprintln!(
                "document index {}: {} files, {} trees, {} candidates, {} unresolved, {} unspelled, {} records, {:.2} ms",
                path.display(),
                files.len(),
                stats.trees,
                stats.candidates,
                stats.unresolved,
                stats.unspelled,
                records.len(),
                stats.build_ms
            );
        }
        DocIndex { path: path.to_path_buf(), text, files: files.to_vec(), records, uses, defs: defs_at, stats }
    }

    /// The opaque types of the file's classes, each with the range of the class that defines
    /// it, inside which it is transparent.
    fn opaque_scopes(&mut self, file: u32, defs: &[DefRange]) -> Vec<OpaqueScope> {
        let mut out = Vec::new();
        for d in defs.iter().filter(|d| d.file == file) {
            let DocTarget::Shared(Target::Class(c)) = d.target else { continue };
            let opaques: Vec<ClassId> = self.syms.class(c).nested.values().copied().filter(|&k| self.syms.class(k).kind == ClassKind::Opaque).collect();
            out.extend(opaques.into_iter().map(|opaque| OpaqueScope { start: d.start, end: d.end, opaque }));
        }
        out
    }

    /// Runs `f` with the opaque types whose defining class holds `addr` transparent, as they
    /// are to the class's own definitions, and restores what was transparent before.
    fn with_transparent<R>(&mut self, scopes: &[OpaqueScope], addr: Addr, f: impl FnOnce(&mut Self) -> R) -> R {
        if scopes.is_empty() {
            return f(self);
        }
        let saved = std::mem::take(&mut self.transparent);
        self.transparent = scopes.iter().filter(|s| s.start <= addr && addr < s.end).map(|s| s.opaque).collect();
        let r = f(self);
        self.transparent = saved;
        r
    }

    /// Completes every class of a loaded file, the nested ones included, so that its members,
    /// aliases and type parameters are in the file's tables.
    fn complete_document_file(&mut self, file: u32) {
        if self.loaded.is_none() {
            return;
        }
        let mut done: Vec<ClassId> = Vec::new();
        loop {
            let mut next: Vec<ClassId> = self.file_tables(file).classes.values().copied().filter(|c| !done.contains(c)).collect();
            if next.is_empty() {
                break;
            }
            next.sort_unstable();
            for c in next {
                done.push(c);
                self.complete_class(c);
            }
        }
    }

    fn index_file(&mut self, f: u32, text: &str, units: &[u32], out: &mut Vec<Candidate>, defs: &mut Vec<DefRange>, stats: &mut DocStats) {
        let positions = self.file_positions(f);
        let Some(section) = positions.section.as_ref() else { return };
        let tasty = self.tasty(f);
        let primary = section.primary();
        let primary_path = primary.map(|n| tasty.name(n));
        let nodes = positions.spans().sorted();
        stats.trees += nodes.len();
        let decoder = Decoder::new(&tasty);
        // The expansions of inline calls: copies of trees that stand elsewhere.
        let mut skip: Vec<(Addr, Addr)> = Vec::new();
        for &(addr, p) in &nodes {
            if p.tag == INLINED && p.target.is_none() {
                let mut r = tasty.trees().at(addr as usize);
                r.byte();
                r.end();
                let expansion = r.pos as Addr;
                let end = Decoder::new(&tasty).skip_at(expansion);
                skip.push((expansion, end));
            }
        }
        skip.sort_unstable();
        let skipped = |addr: Addr| -> bool {
            let upto = skip.partition_point(|&(s, _)| s <= addr);
            skip[..upto].iter().rev().any(|&(_, e)| addr < e)
        };
        // The definitions' ranges first: the owners, and the methods whose parameters a
        // reference may name before their signature is decoded.
        for &(addr, p) in &nodes {
            if p.target.is_some() || !matches!(p.tag, VALDEF | DEFDEF | TYPEDEF) {
                continue;
            }
            let end = decoder.tree_end(addr);
            let Some(target) = self.definition_target(f, addr, &[]) else { continue };
            // A constructor's body and the parents belong to the class.
            if matches!(target, DocTarget::Local(..)) && decoder.name_at(addr).map_or(false, |n| tasty.name(n) == "<init>") {
                continue;
            }
            let owner = !matches!(target, DocTarget::Local(..)) || p.tag != VALDEF;
            defs.push(DefRange { file: f, start: addr, end, target, owner });
        }
        let mut cx = MapCx::new(f);
        // The selections and named arguments of the bodies, whose qualifier or method the
        // decoded terms give: resolved first, by address.
        // The opaque types transparent inside the classes of this file that define them.
        let scopes = self.opaque_scopes(f, defs);
        let resolved = self.term_pass(&mut cx, f, &tasty, defs, &scopes, &skipped);
        let mut last_import: Option<DocTarget> = None;
        for &(addr, p) in &nodes {
            let Some(pos) = p.pos else { continue };
            if skipped(addr) {
                continue;
            }
            // An occurrence of another source makes no record of this document.
            if p.source != primary {
                let same = match (p.source, &primary_path) {
                    (Some(n), Some(path)) => tasty.name(n) == *path,
                    (None, _) => true,
                    _ => false,
                };
                if !same {
                    continue;
                }
            }
            // A shared term's target the walk gives; a shared type (a reference pickled once and
            // named again by its address) is a leaf of the walk, its target in its bytes.
            let shared_type = (p.tag == SHAREDTYPE).then(|| {
                let mut r = tasty.trees().at(addr as usize);
                r.byte();
                r.nat() as Addr
            });
            let decode_at = p.target.or(shared_type).unwrap_or(addr);
            let tag = if decode_at != addr { decoder.tag_at(decode_at) } else { p.tag };
            let mut r = tasty.trees().at(decode_at as usize);
            // The written name and what resolves the tree, the kind told once it is resolved.
            let found: Option<(String, Pending)> = match tag {
                SELECT | SELECTIN | NAMEDARG if resolved.contains_key(&decode_at) => {
                    r.byte();
                    if tag == SELECTIN {
                        r.end();
                    }
                    let n = r.nat();
                    Some((tasty.name(tasty.source_name(n)), Pending::Resolved(resolved[&decode_at], tag == NAMEDARG)))
                }
                IDENT | IDENTTPT | SELECT | SELECTTPT | TERMREF | TYPEREF | TERMREFIN | TYPEREFIN => {
                    r.byte();
                    if matches!(tag, TERMREFIN | TYPEREFIN) {
                        r.end();
                    }
                    let n = r.nat();
                    let is_type = matches!(tag, IDENTTPT | SELECTTPT | TYPEREF | TYPEREFIN);
                    Some((tasty.name(tasty.source_name(n)), Pending::Type(decode_at, is_type)))
                }
                SELECTIN => {
                    r.byte();
                    r.end();
                    let n = r.nat();
                    let qual = r.pos as Addr;
                    let after_qual = Decoder::new(&tasty).skip_at(qual);
                    let owner_at = decoder.shared_target(after_qual);
                    Some((tasty.name(tasty.source_name(n)), Pending::SelectIn(n, owner_at, qual)))
                }
                TERMREFSYMBOL | TERMREFDIRECT | TYPEREFSYMBOL | TYPEREFDIRECT => {
                    r.byte();
                    let def = r.nat() as Addr;
                    let Some(n) = decoder.name_at(def) else { continue };
                    let is_type = matches!(tag, TYPEREFSYMBOL | TYPEREFDIRECT);
                    Some((tasty.name(tasty.source_name(n)), Pending::Local(def, is_type)))
                }
                THIS | QUALTHIS => Some(("this".to_string(), Pending::Type(decode_at, false))),
                // `super.m` and `super[M].m`: the enclosing class, whose `this` the qualifier is.
                SUPER => {
                    r.byte();
                    r.end();
                    Some(("super".to_string(), Pending::Type(r.pos as Addr, false)))
                }
                IMPORT | EXPORT => {
                    self.index_import_clause(&mut cx, f, &tasty, decode_at, &positions, text, units, out, stats, &mut last_import);
                    continue;
                }
                _ if is_definition(tag) => {
                    let Some(n) = decoder.name_at(decode_at) else { continue };
                    Some((tasty.name(tasty.source_name(n)), Pending::Def(decode_at)))
                }
                _ => continue,
            };
            let Some((written, pending)) = found else { continue };
            if written == "_" || written.is_empty() || written.starts_with('<') {
                continue;
            }
            // A prefix operator is pickled as `unary_!`, written `!`.
            let written = written.strip_prefix("unary_").filter(|op| !op.is_empty()).map_or(written.clone(), str::to_string);
            stats.candidates += 1;
            // The span before the target: a synthetic tree is not resolved.
            let Some((span, at_start)) = placed_span(text, units, pos.point, pos.start, &written) else {
                stats.unspelled += 1;
                continue;
            };
            let (target, kind) = match pending {
                Pending::Resolved(t, named) => (t, if named { Kind::NamedArg } else { self.term_kind(t) }),
                Pending::Type(at, is_type) => {
                    let ty = Decoder::new(&tasty).type_at(at);
                    let t = self.with_transparent(&scopes, addr, |w| w.resolve_type_target(&mut cx, f, &ty, defs));
                    (t, if is_type { Kind::Type } else { self.term_kind(t) })
                }
                Pending::SelectIn(n, owner_at, qual) => {
                    let t = self.with_transparent(&scopes, addr, |w| w.select_in_target(&mut cx, f, &tasty, n, owner_at, qual));
                    (t, self.term_kind(t))
                }
                Pending::Local(def, is_type) => {
                    let t = if is_type { self.local_type_target(f, def, defs) } else { self.local_term_target(f, def, defs) };
                    (t, if is_type { Kind::Type } else { self.term_kind(t) })
                }
                Pending::Def(at) => (self.definition_target(f, at, defs), Kind::Def),
            };
            let Some(target) = target else {
                stats.unresolved += 1;
                if std::env::var_os("TEQ_DOCUMENT_INDEX_DUMP").is_some() {
                    let shape: String = format!("{:?}", Decoder::new(&tasty).type_at(decode_at)).chars().take(160).collect();
                    eprintln!("  unresolved {} `{}` at {}..{} (tree {}): {}", crate::tasty::dump::tag_name(tag), written, pos.start, pos.end, addr, shape);
                }
                continue;
            };
            let prio = if kind == Kind::Def { 2 } else { at_start as u8 };
            out.push(Candidate { file: f, addr, span, target, kind, prio });
        }
    }

    /// The bodies' selections and named arguments, resolved from the decoded terms: a
    /// selection's qualifier typed from the declared types the pickle holds (`term_type`), a
    /// named argument through the method applied. By the address of the tree; `None` for one
    /// found but not resolved, so that the address pass does not try again.
    fn term_pass(&mut self, cx: &mut MapCx, f: u32, tasty: &TastyFile, defs: &[DefRange], scopes: &[OpaqueScope], skipped: &dyn Fn(Addr) -> bool) -> FxMap<Addr, Option<DocTarget>> {
        let mut out: FxMap<Addr, Option<DocTarget>> = FxMap::default();
        let tops = crate::tasty::tree::index_top_level(tasty);
        let stats: Vec<Stat> = {
            let mut decoder = Decoder::new(tasty);
            let mut td = TermDecoder::new(&mut decoder);
            tops.iter().map(|t| td.stat_at(t.entry.addr)).collect()
        };
        let mut found: Vec<(Addr, Term)> = Vec::new();
        {
            let mut visit = |t: &Term, _: &TermScope| {
                if skipped(t.at) {
                    return;
                }
                match &t.kind {
                    TermKind::Select(..) | TermKind::SelectIn(..) => found.push((t.at, t.clone())),
                    TermKind::Apply(fun, args) | TermKind::ApplySigPoly(fun, _, args) if args.iter().any(|a| matches!(a.kind, TermKind::NamedArg(..))) => {
                        found.push((t.at, Term::new(t.at, TermKind::Apply(fun.clone(), args.clone()))))
                    }
                    _ => {}
                }
            };
            let mut walk = Walk { f: &mut visit };
            for s in &stats {
                walk.stat(s, TermScope::default());
            }
        }
        for (at, t) in found {
            match &t.kind {
                TermKind::Select(q, n) => {
                    let target = self.with_transparent(scopes, at, |w| w.selection_target(cx, f, tasty, q, *n, None, defs)).map(|(t, _)| t);
                    out.insert(at, target);
                }
                TermKind::SelectIn(q, n, owner, _) => {
                    let target = self.with_transparent(scopes, at, |w| w.selection_target(cx, f, tasty, q, *n, Some(owner), defs)).map(|(t, _)| t);
                    out.insert(at, target);
                }
                TermKind::Apply(fun, args) => {
                    let callee = self.with_transparent(scopes, at, |w| w.callee_of(cx, f, tasty, fun, defs));
                    for a in args {
                        let TermKind::NamedArg(n, _) = &a.kind else { continue };
                        let name = self.lname(tasty, *n);
                        let target = match callee {
                            Some(DocTarget::Shared(Target::Sym(m))) => {
                                let sig = self.sig_arc(m);
                                let param = sig.clauses.iter().flat_map(|c| c.params.iter()).find(|p| p.name == name).map(|p| p.sym);
                                param.map(|p| {
                                    self.place_param(f, p);
                                    self.shared(Target::Sym(p))
                                })
                            }
                            // A local method's parameter, from its pickled clauses.
                            Some(DocTarget::Local(file, addr)) => {
                                let t = self.tasty(file);
                                let sig = Decoder::new(&t).def_sig(addr);
                                let wanted = tasty.name(*n);
                                sig.clauses
                                    .iter()
                                    .filter_map(|c| if let crate::tasty::tree::Clause::Terms(ps) = c { Some(ps) } else { None })
                                    .flatten()
                                    .find(|p| t.name(p.name) == wanted)
                                    .map(|p| DocTarget::Local(file, p.addr))
                            }
                            _ => None,
                        };
                        out.insert(a.at, target);
                    }
                }
                _ => {}
            }
        }
        out
    }

    /// A parameter named by a named argument gets its place in the tables, found among the
    /// forward entries of its method's file.
    fn place_param(&mut self, file: u32, p: SymId) {
        if self.loaded.as_ref().map_or(true, |l| l.syms.get(&p).is_some()) {
            return;
        }
        let owner_file = self.loaded.as_ref().and_then(|l| {
            // The method's file: the parameter's forward entry is in the tables of the file its
            // method was read from, this document's or another.
            (0..l.tables.len()).find(|&i| l.tables[i].terms.values().any(|&s| s == p))
        });
        let Some(owner_file) = owner_file else { return };
        let addr = self.file_tables(owner_file as u32).terms.iter().find(|(_, &s)| s == p).map(|(&a, _)| a);
        if let Some(addr) = addr {
            self.note_param(owner_file as u32, addr, p);
        }
        let _ = file;
    }

    /// The method a selection or application applies, if it names one: a member, or a local
    /// method of the document.
    fn callee_of(&mut self, cx: &mut MapCx, f: u32, tasty: &TastyFile, fun: &Term, defs: &[DefRange]) -> Option<DocTarget> {
        let method = |w: &Self, t: DocTarget| match t {
            DocTarget::Shared(Target::Sym(s)) if w.syms.sym(s).kind == SymKind::Def => Some(t),
            DocTarget::Local(file, addr) if w.local_is_method(file, addr) => Some(t),
            _ => None,
        };
        match &fun.kind {
            TermKind::Apply(inner, _) | TermKind::ApplySigPoly(inner, _, _) | TermKind::TypeApply(inner, _, _) => self.callee_of(cx, f, tasty, inner, defs),
            TermKind::Select(q, n) => {
                let (t, _) = self.selection_target(cx, f, tasty, q, *n, None, defs)?;
                method(self, t)
            }
            TermKind::SelectIn(q, n, owner, _) => {
                let (t, _) = self.selection_target(cx, f, tasty, q, *n, Some(owner), defs)?;
                method(self, t)
            }
            TermKind::Path(ty) | TermKind::Ident(_, ty) => {
                let t = self.resolve_type_target(cx, f, ty, defs)?;
                method(self, t)
            }
            TermKind::Block(_, e) | TermKind::Inlined { expansion: e, .. } => self.callee_of(cx, f, tasty, e, defs),
            _ => None,
        }
    }

    /// The member a selection names, with the type of the selection where the qualifier's type
    /// is known: in the class its `SELECTin` owner states, else as `select_member` finds it in
    /// the qualifier's type.
    fn selection_target(&mut self, cx: &mut MapCx, f: u32, tasty: &TastyFile, q: &Term, n: NameRef, owner: Option<&TType>, defs: &[DefRange]) -> Option<(DocTarget, Option<TypeId>)> {
        let written = tasty.name(tasty.source_name(n));
        if written.starts_with('<') {
            return None;
        }
        let name = self.lname(tasty, n);
        let signed = matches!(tasty.names.get(n as usize), Some(crate::tasty::TName::Signed { .. })).then_some(n);
        let owner_class = owner.and_then(|o| match o {
            TType::LocalType(addr, _) => self.file_tables(f).classes.get(addr).copied(),
            other => {
                let ctor = self.map_type_ctor(cx, other);
                match self.types.get(ctor) {
                    Type::Class(k, _) | Type::Ctor(k) => Some(k),
                    _ => None,
                }
            }
        });
        if let Some(k) = owner_class {
            if let Some(t) = self.member_target(k, name, signed, tasty) {
                // The selection's type from the qualifier's, where it is known and names the
                // same member; the owner's declared result otherwise.
                let qt = self.term_type(cx, f, tasty, q, defs);
                let ty = match (qt, t) {
                    (Some(qt), DocTarget::Shared(Target::Sym(s))) => match self.select_member(cx, qt, qt, name, signed, tasty, 0) {
                        Some((found, ty)) if found == t => Some(ty),
                        _ => Some(self.member_result_type(qt, qt, s)),
                    },
                    (_, DocTarget::Shared(Target::Sym(s))) => Some(self.sig_arc(s).ret),
                    (_, DocTarget::Shared(other)) => self.target_value_type(other),
                    (_, DocTarget::Local(file, addr)) => self.local_declared_type(cx, file, addr),
                };
                return Some((t, ty));
            }
        }
        // `super.m`: the member of a parent, the mixin's where one is named.
        if let TermKind::Super(this, mixin) = &q.kind {
            let parents: Vec<ClassId> = match mixin {
                Some(m) => {
                    let t = self.map_type(cx, m);
                    self.class_of_prefix_type(t, 0).into_iter().collect()
                }
                None => {
                    let t = self.term_type(cx, f, tasty, this, defs)?;
                    let c = self.class_of_prefix_type(t, 0)?;
                    self.complete_class(c);
                    self.syms.class(c).base_types.iter().skip(1).map(|&(b, _)| b).collect()
                }
            };
            for k in parents {
                if let Some(t) = self.class_member_named(k, name, signed, Some(tasty)) {
                    let ty = match t {
                        Target::Sym(s) => Some(self.sig_arc(s).ret),
                        other => self.target_value_type(other),
                    };
                    return Some((self.shared(t), ty));
                }
            }
            return None;
        }
        let qt = self.term_type(cx, f, tasty, q, defs)?;
        self.select_member(cx, qt, qt, name, signed, tasty, 0).map(|(t, ty)| (t, Some(ty)))
    }

    /// The member `name` selected from a receiver of type `pre`, as scalac's `findMember` finds
    /// it (`Types.scala` 821 to 1000 at the 3.8.4 tag), with the type of the selection; `t` is
    /// the part of `pre` being read, `pre` itself at first. A path's underlying type, an alias's
    /// expansion (an opaque type's inside its scope, `Worker::transparent`) and a type
    /// parameter's upper bound (`goParam`, 979) are read through; a refinement of the member
    /// itself narrows the type wherever the reading meets it, a parameter's bound included
    /// (`goRefined`, 900: the refinement's info meets the parent's member), `narrow` carrying
    /// the narrowest met; a this-type reads its class and its self type as dotty's
    /// `ClassInfo.selfType` has them, `givenSelf & cls` (5597 to 5610); an intersection's two
    /// members are merged (`goAnd`, 996) by `merge_members`, or decline where the merge's score
    /// is not certain; a class gives the member and its result seen from `pre`
    /// (`member_result_type`).
    fn select_member(&mut self, cx: &mut MapCx, pre: TypeId, t: TypeId, name: Name, signed: Option<NameRef>, tasty: &TastyFile, depth: u32) -> Option<(DocTarget, TypeId)> {
        self.select_narrowed(cx, pre, t, None, name, signed, tasty, depth)
    }

    #[allow(clippy::too_many_arguments)]
    fn select_narrowed(&mut self, cx: &mut MapCx, pre: TypeId, t: TypeId, narrow: Option<TypeId>, name: Name, signed: Option<NameRef>, tasty: &TastyFile, depth: u32) -> Option<(DocTarget, TypeId)> {
        if depth > 8 {
            return None;
        }
        // A this-type before its underlying type: `givenSelf & cls`, the class's own members
        // merged with the self type's, the class's base classes first.
        if let Type::This(c) = self.types.get(t) {
            if let Some(given) = self.syms.class(c).declared_self {
                let cls = self.types.class(c, &[]);
                let inter = self.types.mk(Type::Inter(given, cls));
                return self.select_narrowed(cx, pre, inter, narrow, name, signed, tasty, depth + 1);
            }
        }
        let t = self.widen_path(t);
        let t = self.dealias(t);
        match self.types.get(t) {
            Type::Param(p) | Type::AppParam(p, _) => {
                let upper = self.syms.tparam(p).upper;
                self.select_narrowed(cx, pre, upper, narrow, name, signed, tasty, depth + 1)
            }
            Type::Refined(parent, r) => {
                let own = match self.types.refinement(r) {
                    Refinement::Val(n, _, ty) if n == name => Some(ty),
                    Refinement::Term(n, _, types) if n == name => self.types.items(types).last().copied(),
                    _ => None,
                };
                let narrow = match (narrow, own) {
                    (Some(a), Some(b)) => Some(self.meet_types(a, b)),
                    (a, b) => a.or(b),
                };
                self.select_narrowed(cx, pre, parent, narrow, name, signed, tasty, depth + 1)
            }
            Type::Inter(a, b) => {
                let left = self.select_narrowed(cx, pre, a, narrow, name, signed, tasty, depth + 1);
                let right = self.select_narrowed(cx, pre, b, narrow, name, signed, tasty, depth + 1);
                match (left, right) {
                    (Some(l), Some(r)) => {
                        // Scored against the whole receiver's base classes at every merge, as
                        // `goAnd` passes `pre` down (996) and `linearScore` searches it (427).
                        let bases = self.base_classes(pre, 0);
                        self.merge_members(&bases, l, r)
                    }
                    (l, r) => l.or(r),
                }
            }
            Type::Class(c, _) | Type::Ctor(c) | Type::This(c) => {
                let target = self.member_target(c, name, signed, tasty)?;
                let ty = match target {
                    DocTarget::Shared(Target::Sym(s)) => self.member_result_type(pre, t, s),
                    DocTarget::Shared(other) => self.target_value_type(other)?,
                    DocTarget::Local(file, addr) => self.local_declared_type(cx, file, addr)?,
                };
                let ty = match narrow {
                    Some(n) => self.meet_types(n, ty),
                    None => ty,
                };
                Some((target, ty))
            }
            Type::Any => {
                let target = self.member_target(self.b.any_ref, name, signed, tasty)?;
                let ty = match target {
                    DocTarget::Shared(Target::Sym(s)) => self.sig_arc(s).ret,
                    DocTarget::Shared(other) => self.target_value_type(other)?,
                    DocTarget::Local(file, addr) => self.local_declared_type(cx, file, addr)?,
                };
                Some((target, ty))
            }
            _ => None,
        }
    }

    /// The meet of two types as `infoMeet` takes it (`Denotations.scala` 541): the one that
    /// conforms to the other, else their intersection.
    fn meet_types(&mut self, a: TypeId, b: TypeId) -> TypeId {
        if self.is_sub(a, b) {
            a
        } else if self.is_sub(b, a) {
            b
        } else {
            self.types.mk(Type::Inter(a, b))
        }
    }

    /// The base classes of a receiver's type in the order scalac's `baseClasses` gives them
    /// (`Types.scala` 728 to 741 at 3.8.4: a class's own linearisation; a proxy its super
    /// type's, here a parameter's bound and a refinement's parent; an intersection's
    /// (`AndType.baseClasses`, 3544 to 3560) the second component's classes not among the
    /// first's, up to their first common class, then the first's; a this-type's its
    /// `givenSelf & cls`), `Nil` for a type with none.
    fn base_classes(&mut self, t: TypeId, depth: u32) -> Vec<ClassId> {
        if depth > 8 {
            return Vec::new();
        }
        if let Type::This(c) = self.types.get(t) {
            if let Some(given) = self.syms.class(c).declared_self {
                let cls = self.types.class(c, &[]);
                let inter = self.types.mk(Type::Inter(given, cls));
                return self.base_classes(inter, depth + 1);
            }
        }
        let t = self.widen_path(t);
        let t = self.dealias(t);
        match self.types.get(t) {
            Type::Class(c, _) | Type::Ctor(c) | Type::This(c) => {
                self.complete_class(c);
                self.syms.class(c).base_types.iter().map(|&(b, _)| b).collect()
            }
            Type::Param(p) | Type::AppParam(p, _) => {
                let upper = self.syms.tparam(p).upper;
                self.base_classes(upper, depth + 1)
            }
            Type::Refined(parent, _) => self.base_classes(parent, depth + 1),
            Type::Inter(a, b) => {
                let bcs1 = self.base_classes(a, depth + 1);
                let bcs2 = self.base_classes(b, depth + 1);
                let mut out = Vec::new();
                for bc2 in bcs2 {
                    if bcs1.contains(&bc2) {
                        if self.syms.class(bc2).kind == ClassKind::Trait {
                            continue;
                        }
                        break;
                    }
                    out.push(bc2);
                }
                out.extend(bcs1);
                out
            }
            _ => Vec::new(),
        }
    }

    /// The members an intersection's two components give one name, merged as scalac's
    /// `mergeSingleDenot` merges their denotations (`Denotations.scala` 420 to 500 at 3.8.4):
    /// the symbol score prefers a member as concrete as the other (`isAsConcrete`,
    /// `SymDenotations.scala` 997: not deferred, or the other deferred too; 462 to 469), else
    /// the owner that comes first in the receiver's base classes (`linearScore`, 427 to 440: an
    /// owner deriving from the other, else the first found in `bases`); the member whose type
    /// overrides the other's wins where the score allows (488 to 491); else the joint member is
    /// the scoring symbol with the meet of the types (493 to 496). `None` where the score is
    /// not certain: a member whose deferredness the tables do not tell, or owners found in
    /// neither order among the base classes.
    fn merge_members(&mut self, bases: &[ClassId], left: (DocTarget, TypeId), right: (DocTarget, TypeId)) -> Option<(DocTarget, TypeId)> {
        let ((sa, ta), (sb, tb)) = (left, right);
        // One symbol through both components, with the information each gives it: the meet
        // (`infoMeet`, 541), as for two denotations of one symbol.
        if sa == sb {
            let meet = self.meet_types(ta, tb);
            return Some((sa, meet));
        }
        let owner_of = |w: &Self, t: DocTarget| match t {
            DocTarget::Shared(Target::Sym(s)) => match w.syms.sym(s).owner {
                Owner::Class(c) => Some(c),
                _ => None,
            },
            _ => None,
        };
        // Deferred: a member declared without a body (the loader marks it abstract, a
        // constructor parameter's accessor is not), a program's by its definition.
        let deferred = |w: &Self, t: DocTarget| -> Option<bool> {
            match t {
                DocTarget::Shared(Target::Sym(s)) => Some(w.is_abstract_member(s)),
                _ => None,
            }
        };
        let (da, db) = (deferred(self, sa)?, deferred(self, sb)?);
        let (oa, ob) = (owner_of(self, sa)?, owner_of(self, sb)?);
        let score: i32 = if da && !db {
            -1
        } else if db && !da {
            1
        } else if oa == ob {
            0
        } else {
            self.complete_class(oa);
            self.complete_class(ob);
            if self.syms.class(oa).base_types.iter().any(|&(c, _)| c == ob) {
                1
            } else if self.syms.class(ob).base_types.iter().any(|&(c, _)| c == oa) {
                -1
            } else {
                match bases.iter().find(|&&c| c == oa || c == ob) {
                    Some(&c) if c == oa => 1,
                    Some(_) => -1,
                    None => return None,
                }
            }
        };
        if score <= 0 && self.is_sub(tb, ta) {
            return Some((sb, tb));
        }
        if score >= 0 && self.is_sub(ta, tb) {
            return Some((sa, ta));
        }
        if score == 0 {
            return None;
        }
        let meet = self.meet_types(ta, tb);
        Some((if score > 0 { sa } else { sb }, meet))
    }

    /// The type of a term of a body, from the declared types the pickle holds: a path's as a
    /// prefix has it, a selection's its member's result seen from the qualifier, an
    /// application's its method's result (its type arguments where written), a `new` its type,
    /// an ascription's its type, a block's its expression's, a constant's its class; `None`
    /// where the declared types do not tell (a lambda, a match).
    fn term_type(&mut self, cx: &mut MapCx, f: u32, tasty: &TastyFile, t: &Term, defs: &[DefRange]) -> Option<TypeId> {
        match &t.kind {
            TermKind::Path(ty) | TermKind::Ident(_, ty) => self.path_prefix_type(cx, f, ty, defs),
            TermKind::QualThis(ty) => self.path_prefix_type(cx, f, &TType::This(Box::new(ty.clone())), defs),
            TermKind::Const(c) => Some(match c {
                Const::Unit => self.b.t_unit,
                Const::Bool(_) => self.b.t_boolean,
                Const::Byte(_) => self.b.t_byte,
                Const::Short(_) => self.b.t_short,
                Const::Char(_) => self.b.t_char,
                Const::Int(_) => self.b.t_int,
                Const::Long(_) => self.b.t_long,
                Const::Float(_) => self.b.t_float,
                Const::Double(_) => self.b.t_double,
                Const::Str(_) => self.b.t_string,
                Const::Null => self.b.t_null,
                Const::Class(_) => return None,
            }),
            TermKind::Select(q, n) | TermKind::SelectIn(q, n, _, _) => {
                let owner = if let TermKind::SelectIn(_, _, o, _) = &t.kind { Some(o) } else { None };
                let (_, ty) = self.selection_target(cx, f, tasty, q, *n, owner, defs)?;
                ty
            }
            TermKind::Apply(fun, _) | TermKind::ApplySigPoly(fun, _, _) => self.term_type(cx, f, tasty, fun, defs),
            TermKind::TypeApply(fun, targs, _) => {
                let callee = self.callee_of(cx, f, tasty, fun, defs);
                // `x.asInstanceOf[T]`, the builtin of `Any` that no declaration stands behind: `T`.
                if callee.is_none() {
                    if let TermKind::Select(_, n) | TermKind::SelectIn(_, n, _, _) = &fun.kind {
                        if tasty.name(tasty.source_name(*n)) == "asInstanceOf" {
                            return targs.first().map(|t| self.map_type(cx, t));
                        }
                    }
                }
                let ret = self.term_type(cx, f, tasty, fun, defs)?;
                let Some(DocTarget::Shared(Target::Sym(m))) = callee else { return Some(ret) };
                let sig = self.sig_arc(m);
                if sig.tparams.len() != targs.len() {
                    return Some(ret);
                }
                let args: Vec<TypeId> = targs.iter().map(|a| self.map_type(cx, a)).collect();
                let subst: Subst = sig.tparams.iter().copied().zip(args).collect();
                Some(self.types.subst(ret, &subst))
            }
            TermKind::New(ty) | TermKind::Elided(ty) => Some(self.map_type(cx, ty)),
            TermKind::Typed(_, ty, _) => Some(self.map_type(cx, ty)),
            TermKind::Block(_, e) | TermKind::Inlined { expansion: e, .. } => self.term_type(cx, f, tasty, e, defs),
            // The branches' join, as scalac types the expression: the first class every branch
            // conforms to, in the first branch's linearisation; none where a branch tells nothing.
            TermKind::If { then, els, .. } => {
                let a = self.term_type(cx, f, tasty, then, defs)?;
                let b = self.term_type(cx, f, tasty, els, defs)?;
                self.join_types(a, b)
            }
            TermKind::Match { cases, .. } => {
                let mut joined: Option<TypeId> = None;
                for c in cases {
                    let t = self.term_type(cx, f, tasty, &c.body, defs)?;
                    joined = Some(match joined {
                        Some(j) => self.join_types(j, t)?,
                        None => t,
                    });
                }
                joined
            }
            TermKind::Try { body, cases, .. } => {
                let mut joined = self.term_type(cx, f, tasty, body, defs)?;
                for c in cases {
                    let t = self.term_type(cx, f, tasty, &c.body, defs)?;
                    joined = self.join_types(joined, t)?;
                }
                Some(joined)
            }
            TermKind::Assign(..) | TermKind::While(..) => Some(self.b.t_unit),
            TermKind::Repeated(ty, _) => {
                let elem = self.map_type(cx, ty);
                let seq = self.b.seq?;
                Some(self.types.class(seq, &[elem]))
            }
            _ => None,
        }
    }

    /// The join of two types for the members selected from a branching expression: the type
    /// itself where both are the same type, `Nothing` yielding the other; else the one common
    /// base class every other common one derives from, where it takes no type arguments (the
    /// join of two instances of a generic class, `GA` and `GB` both `GIndirect[Left]`, needs the
    /// arguments' join, which is not built: no record then); two dominant common parents
    /// (`Mix1 extends Wide, Narrow` and `Mix2 extends Narrow, Wide`) or a meet at `AnyRef` or
    /// `AnyVal`, whose members have no symbols, give none.
    fn join_types(&mut self, a: TypeId, b: TypeId) -> Option<TypeId> {
        let wa = self.widen_path(a);
        let da = self.dealias(wa);
        let wb = self.widen_path(b);
        let db = self.dealias(wb);
        if da == db {
            return Some(a);
        }
        if matches!(self.types.get(da), Type::Nothing) {
            return Some(b);
        }
        if matches!(self.types.get(db), Type::Nothing) {
            return Some(a);
        }
        let ka = self.class_of_prefix_type(a, 0)?;
        let kb = self.class_of_prefix_type(b, 0)?;
        if ka == kb {
            // One class applied to different arguments: neither branch's arguments stand for
            // the join's (`Box[Left]` and `Box[Right]` join at `Box[? <: Base]`).
            return None;
        }
        self.complete_class(ka);
        self.complete_class(kb);
        let bases_b: Vec<ClassId> = self.syms.class(kb).base_types.iter().map(|&(c, _)| c).collect();
        let common: Vec<ClassId> = self.syms.class(ka).base_types.iter().map(|&(c, _)| c).filter(|c| bases_b.contains(c)).collect();
        let dominant: Vec<ClassId> = common
            .iter()
            .copied()
            .filter(|&c| !common.iter().any(|&o| o != c && self.syms.class(o).base_types.iter().any(|&(d, _)| d == c)))
            .collect();
        let [d] = dominant[..] else { return None };
        if d == self.b.any_ref || d == self.b.any_val || !self.syms.class(d).tparams.is_empty() {
            return None;
        }
        Some(self.types.class(d, &[]))
    }

    /// The selectors of an import or export clause at `addr`: each a record of what the
    /// qualifier's scope holds under its name, a renamed selector's that of the name it renames.
    #[allow(clippy::too_many_arguments)]
    fn index_import_clause(
        &mut self,
        cx: &mut MapCx,
        f: u32,
        tasty: &TastyFile,
        addr: Addr,
        positions: &super::places::FilePositions,
        text: &str,
        units: &[u32],
        out: &mut Vec<Candidate>,
        stats: &mut DocStats,
        last: &mut Option<DocTarget>,
    ) {
        let mut r = tasty.trees().at(addr as usize);
        r.byte();
        let end = r.end();
        let qual_at = r.pos as Addr;
        let mut decoder = Decoder::new(tasty);
        let after = decoder.skip_at(qual_at);
        let qual = decoder.type_at(qual_at);
        r.pos = after as usize;
        *last = None;
        while r.pos < end {
            let at = r.pos as Addr;
            match r.byte() {
                tag @ (IMPORTED | RENAMED) => {
                    let n = r.nat();
                    let written = tasty.name(tasty.source_name(n));
                    if written == "_" || written.is_empty() {
                        *last = None;
                        continue;
                    }
                    let target = if tag == IMPORTED {
                        let name = self.lname(tasty, n);
                        let t = self.import_selector_target(cx, &qual, name);
                        *last = t;
                        t
                    } else {
                        *last
                    };
                    stats.candidates += 1;
                    let Some(target) = target else {
                        stats.unresolved += 1;
                        continue;
                    };
                    let Some(pos) = positions.spans().get(at).and_then(|p| p.pos) else { continue };
                    match placed_span(text, units, pos.point, pos.start, &written) {
                        Some((span, at_start)) => out.push(Candidate { file: f, addr: at, span, target, kind: Kind::Import, prio: at_start as u8 }),
                        None => stats.unspelled += 1,
                    }
                }
                BOUNDED => {
                    r.pos = decoder.skip_at(r.pos as Addr) as usize;
                }
                _ => break,
            }
        }
    }

    /// What an import's selector `name` names in the scope of `qual`: a class or alias first,
    /// then a term (an object, a val, a def).
    fn import_selector_target(&mut self, cx: &mut MapCx, qual: &TType, name: Name) -> Option<DocTarget> {
        match self.scope_of(cx, qual)? {
            Scope::Pkg(p) => {
                if let Some(t) = self.pkg_type(p, name).and_then(type_ref_target) {
                    return Some(t);
                }
                match self.pkg_term(p, name)? {
                    super::super::resolve::TermRef::Class(c) => Some(DocTarget::Shared(Target::Class(c))),
                    super::super::resolve::TermRef::Package(_) => None,
                    r => r.sym().map(|s| self.shared(Target::Sym(s))),
                }
            }
            Scope::Class(c) => {
                if let Some(t) = self.member_type_of(c, name).and_then(type_ref_target) {
                    return Some(t);
                }
                self.class_member_named(c, name, None, None).map(|t| self.shared(t))
            }
        }
    }

    /// A shared target as the index records it: an object its class, an extension group's
    /// parameter its first method's.
    fn shared(&self, t: Target) -> DocTarget {
        DocTarget::Shared(self.normal_target(t))
    }

    fn term_kind(&self, t: Option<DocTarget>) -> Kind {
        match t {
            Some(DocTarget::Shared(Target::Sym(s))) if self.syms.sym(s).kind == SymKind::Def => Kind::Call,
            Some(DocTarget::Local(file, addr)) if self.local_is_method(file, addr) => Kind::Call,
            _ => Kind::Ref,
        }
    }

    /// The target of a definition at `addr` of the loaded file `file`: what the tables entered
    /// of it, else a local of the document.
    fn definition_target(&mut self, file: u32, addr: Addr, defs: &[DefRange]) -> Option<DocTarget> {
        let tables = self.file_tables(file);
        if let Some(&c) = tables.classes.get(&addr) {
            return Some(DocTarget::Shared(Target::Class(c)));
        }
        if let Some(&s) = tables.terms.get(&addr) {
            self.note_param(file, addr, s);
            return Some(self.shared(Target::Sym(s)));
        }
        if let Some(&a) = tables.aliases.get(&addr) {
            return Some(DocTarget::Shared(Target::Alias(a)));
        }
        if let Some(&p) = tables.tparams.get(&addr) {
            self.note_tparam(file, addr, p);
            return Some(DocTarget::Shared(Target::TParam(p)));
        }
        let tasty = self.tasty(file);
        match Decoder::new(&tasty).tag_at(addr) {
            PARAM => self.local_term_target(file, addr, defs),
            TYPEPARAM => self.local_type_target(file, addr, defs),
            _ => Some(DocTarget::Local(file, addr)),
        }
    }

    /// A parameter the signature decoding entered forward alone gets its place in the tables
    /// the attach path locates by, once.
    fn note_param(&mut self, file: u32, addr: Addr, s: SymId) {
        if self.syms.sym(s).kind == SymKind::Param && self.loaded.as_ref().map_or(false, |l| l.syms.get(&s).is_none()) {
            self.with_loader(|w| w.loaded_mut().syms.insert(s, LSym { file, addr, conversion: false }));
        }
    }

    fn note_tparam(&mut self, file: u32, addr: Addr, p: TParamId) {
        if self.loaded.as_ref().map_or(false, |l| !l.tparam_defs.contains_key(&p)) {
            self.with_loader(|w| w.loaded_mut().tparam_defs.insert(p, (file, addr)));
        }
    }

    /// The innermost method or class whose tree holds `addr`, from the definitions' ranges.
    fn enclosing_definition(&self, file: u32, addr: Addr, defs: &[DefRange]) -> Option<(Addr, DocTarget)> {
        defs.iter().filter(|d| d.file == file && d.start < addr && addr < d.end).max_by_key(|d| d.start).map(|d| (d.start, d.target))
    }

    /// The target of a term defined at `addr` of `file`: a member or a parameter the tables
    /// hold, a parameter of a method whose signature is decoded for it, else a local.
    fn local_term_target(&mut self, file: u32, addr: Addr, defs: &[DefRange]) -> Option<DocTarget> {
        if let Some(&s) = self.file_tables(file).terms.get(&addr) {
            self.note_param(file, addr, s);
            return Some(self.shared(Target::Sym(s)));
        }
        let tasty = self.tasty(file);
        let tag = Decoder::new(&tasty).tag_at(addr);
        if tag == PARAM {
            if let Some((_, DocTarget::Shared(Target::Sym(m)))) = self.enclosing_definition(file, addr, defs) {
                let _ = self.sig_arc(m);
                if let Some(&s) = self.file_tables(file).terms.get(&addr) {
                    self.note_param(file, addr, s);
                    return Some(self.shared(Target::Sym(s)));
                }
            }
        }
        if matches!(tag, VALDEF | DEFDEF | PARAM | BIND) {
            return Some(DocTarget::Local(file, addr));
        }
        None
    }

    /// The target of a type defined at `addr` of `file`: a class, an alias or a type parameter
    /// the tables hold, a method's type parameter entered with its signature, else a local.
    fn local_type_target(&mut self, file: u32, addr: Addr, defs: &[DefRange]) -> Option<DocTarget> {
        let found = |w: &mut Self| -> Option<DocTarget> {
            let tables = w.file_tables(file);
            if let Some(&c) = tables.classes.get(&addr) {
                return Some(DocTarget::Shared(Target::Class(c)));
            }
            if let Some(&a) = tables.aliases.get(&addr) {
                return Some(DocTarget::Shared(Target::Alias(a)));
            }
            if let Some(&p) = tables.tparams.get(&addr) {
                w.note_tparam(file, addr, p);
                return Some(DocTarget::Shared(Target::TParam(p)));
            }
            None
        };
        if let Some(t) = found(self) {
            return Some(t);
        }
        let tasty = self.tasty(file);
        let tag = Decoder::new(&tasty).tag_at(addr);
        if tag == TYPEPARAM {
            if let Some((_, DocTarget::Shared(Target::Sym(m)))) = self.enclosing_definition(file, addr, defs) {
                let _ = self.sig_arc(m);
                if let Some(t) = found(self) {
                    return Some(t);
                }
            }
        }
        if matches!(tag, TYPEDEF | TYPEPARAM | BIND) {
            return Some(DocTarget::Local(file, addr));
        }
        None
    }

    /// What a type as TASTy states it names, as a target: a class, an alias, a type parameter,
    /// a term (a val, a def, an object), a package nothing; `defs` the definitions' ranges of
    /// the file being indexed, for a parameter named before its method's signature is decoded.
    fn resolve_type_target(&mut self, cx: &mut MapCx, file: u32, t: &TType, defs: &[DefRange]) -> Option<DocTarget> {
        let tasty = self.tasty(file);
        match t {
            TType::LocalType(addr, _) => self.local_type_target(file, *addr, defs),
            TType::LocalTerm(addr, _) => self.local_term_target(file, *addr, defs),
            TType::TypeRef(prefix, n) => {
                if tasty.is_object_class(*n) {
                    let name = self.lname(&tasty, tasty.source_name(*n));
                    return self.object_in(cx, prefix, name).map(|c| DocTarget::Shared(Target::Class(c)));
                }
                let name = self.lname(&tasty, *n);
                // `x.Out` for a parameter or local whose declared type is a refinement that
                // defines `Out`: that member, a definition of the document.
                if let TType::LocalTerm(addr, _) = &**prefix {
                    let end = Decoder::new(&tasty).tree_end(*addr);
                    let wanted = tasty.name(*n);
                    let member = defs
                        .iter()
                        .filter(|d| d.file == file && *addr < d.start && d.start < end && matches!(d.target, DocTarget::Local(..)))
                        .find(|d| Decoder::new(&tasty).tag_at(d.start) == TYPEDEF && Decoder::new(&tasty).name_at(d.start).map_or(false, |m| tasty.name(m) == wanted))
                        .map(|d| d.target);
                    if member.is_some() {
                        return member;
                    }
                }
                match self.scope_of(cx, prefix) {
                    Some(Scope::Pkg(p)) => self.pkg_type(p, name).and_then(type_ref_target),
                    Some(Scope::Class(c)) => self.type_member_target(c, name),
                    None => {
                        let pt = self.path_prefix_type(cx, file, prefix, defs)?;
                        let k = self.class_of_prefix_type(pt, 0)?;
                        self.type_member_target(k, name)
                    }
                }
            }
            TType::TermRef(prefix, n) => {
                if let Some(s) = self.path_term(cx, t) {
                    return Some(self.shared(Target::Sym(s)));
                }
                let name = self.lname(&tasty, *n);
                let signed = matches!(tasty.names.get(*n as usize), Some(crate::tasty::TName::Signed { .. })).then_some(*n);
                match self.scope_of(cx, prefix) {
                    Some(Scope::Pkg(p)) => match self.pkg_term(p, name)? {
                        super::super::resolve::TermRef::Class(c) => Some(DocTarget::Shared(Target::Class(c))),
                        super::super::resolve::TermRef::Package(_) => None,
                        r => r.sym().map(|s| self.shared(Target::Sym(s))),
                    },
                    Some(Scope::Class(c)) => self.member_target(c, name, signed, &tasty),
                    None => {
                        let pt = self.path_prefix_type(cx, file, prefix, defs)?;
                        self.select_member(cx, pt, pt, name, signed, &tasty, 0).map(|(t, _)| t)
                    }
                }
            }
            // A type constructor argument eta-expanded to a lambda: the constructor.
            TType::Lambda { result, .. } => self.resolve_type_target(cx, file, result, defs),
            TType::This(inner) => match &**inner {
                TType::LocalType(addr, _) => self.file_tables(file).classes.get(addr).copied().map(|c| DocTarget::Shared(Target::Class(c))),
                TType::Package(_) => None,
                other => match self.scope_of(cx, other) {
                    Some(Scope::Class(c)) => Some(DocTarget::Shared(Target::Class(c))),
                    _ => self.class_of_prefix(cx, other).map(|c| DocTarget::Shared(Target::Class(c))),
                },
            },
            TType::Super(this, _) => self.resolve_type_target(cx, file, this, defs),
            TType::Applied(ctor, _) => self.resolve_type_target(cx, file, ctor, defs),
            TType::Annotated(u, _) | TType::ByName(u) | TType::Flexible(u) | TType::Alias(u) => self.resolve_type_target(cx, file, u, defs),
            _ => None,
        }
    }

    /// The type member `name` of the class `c`: a nested class, an alias, an abstract member.
    fn type_member_target(&mut self, c: ClassId, name: Name) -> Option<DocTarget> {
        if let Some(t) = self.member_type_of(c, name).and_then(type_ref_target) {
            return Some(t);
        }
        self.complete_class(c);
        let mut owners = vec![c];
        owners.extend(self.syms.class(c).base_types.iter().skip(1).map(|&(b, _)| b));
        for k in owners {
            if let Some(&n) = self.syms.class(k).nested.get(&name) {
                return Some(DocTarget::Shared(Target::Class(n)));
            }
        }
        None
    }

    /// The class a prefix type names an instance of, for the members selected from it.
    fn class_of_prefix(&mut self, cx: &mut MapCx, prefix: &TType) -> Option<ClassId> {
        let t = self.map_type(cx, prefix);
        self.class_of_prefix_type(t, 0)
    }

    /// The type of a path as the members selected from it see it: a parameter's or a local's
    /// declared type where the tables hold no symbol for it, a member's result type seen from
    /// the path it is selected from, an object's class type, the type as mapped otherwise.
    fn path_prefix_type(&mut self, cx: &mut MapCx, file: u32, prefix: &TType, defs: &[DefRange]) -> Option<TypeId> {
        match prefix {
            TType::LocalTerm(addr, _) => match self.local_term_target(file, *addr, defs)? {
                DocTarget::Shared(t) => self.target_value_type(t),
                DocTarget::Local(f, a) => self.local_declared_type(cx, f, a),
            },
            TType::TermRef(p, n) => {
                if let Some(s) = self.path_term(cx, prefix) {
                    return self.target_value_type(self.normal_target(Target::Sym(s)));
                }
                let tasty = self.tasty(file);
                let name = self.lname(&tasty, *n);
                let signed = matches!(tasty.names.get(*n as usize), Some(crate::tasty::TName::Signed { .. })).then_some(*n);
                let pt = match self.scope_of(cx, p) {
                    Some(Scope::Class(c)) => self.types.class(c, &[]),
                    Some(Scope::Pkg(_)) => return None,
                    None => self.path_prefix_type(cx, file, p, defs)?,
                };
                self.select_member(cx, pt, pt, name, signed, &tasty, 0).map(|(_, ty)| ty)
            }
            TType::This(inner) => {
                let c = match self.resolve_type_target(cx, file, &TType::This(inner.clone()), defs)? {
                    DocTarget::Shared(Target::Class(c)) => c,
                    _ => return None,
                };
                Some(self.this_prefix(c))
            }
            other => Some(self.map_type(cx, other)),
        }
    }

    /// The type a target has as a value: a val's or parameter's type, a parameterless def's
    /// result, an object's class type.
    fn target_value_type(&mut self, t: Target) -> Option<TypeId> {
        match t {
            Target::Sym(s) => match self.syms.sym(s).kind {
                SymKind::Object(c) => Some(self.types.class(c, &[])),
                _ => Some(self.sig_arc(s).ret),
            },
            Target::Class(c) => Some(self.types.class(c, &[])),
            _ => None,
        }
    }

    /// The declared type of a local definition of the document: its type tree, inferred or
    /// written, as the pickle states it.
    fn local_declared_type(&mut self, cx: &mut MapCx, file: u32, addr: Addr) -> Option<TypeId> {
        let tasty = self.tasty(file);
        let mut decoder = Decoder::new(&tasty);
        let ty = match decoder.tag_at(addr) {
            VALDEF | DEFDEF | PARAM => decoder.def_sig(addr).ret,
            BIND => decoder.bind_type(addr)?,
            _ => return None,
        };
        let mut local_cx = MapCx::new(file);
        let cx = if cx.file == file { cx } else { &mut local_cx };
        Some(self.map_type(cx, &ty))
    }

    /// A member's result type seen from the type `pre` it is selected from, `leaf` the class
    /// type of `pre` the member was found in: the owner's type parameters replaced by the
    /// arguments the leaf's base type gives them, the owner's `this` by the prefix, so that a
    /// type member the prefix fixes (`x.Item` for `x: Carrier { type Item = Left }`) is looked
    /// up anew (`as_seen_from`); a refinement of the member itself is `select_narrowed`'s.
    pub(in crate::typer) fn member_result_type(&mut self, pre: TypeId, leaf: TypeId, s: SymId) -> TypeId {
        let ret = self.sig_arc(s).ret;
        let Owner::Class(owner) = self.syms.sym(s).owner else { return ret };
        let ret = (|w: &mut Self| {
            let Some(t) = w.receiver_type(leaf, 0) else { return ret };
            let Type::Class(c, args) = w.types.get(t) else { return ret };
            w.complete_class(c);
            let base = w.syms.class(c).base_types.iter().find(|&&(b, _)| b == owner).map(|&(_, bt)| bt);
            let Some(base) = base else { return ret };
            let outer: Subst = w.syms.class(c).tparams.iter().copied().zip(w.types.items(args).iter().copied()).collect();
            let base = if outer.is_empty() { base } else { w.types.subst(base, &outer) };
            let Type::Class(_, kargs) = w.types.get(base) else { return ret };
            let inner: Subst = w.syms.class(owner).tparams.iter().copied().zip(w.types.items(kargs).iter().copied()).collect();
            if inner.is_empty() {
                ret
            } else {
                w.types.subst(ret, &inner)
            }
        })(self);
        self.seen_from_receiver(ret, pre, owner, 0).unwrap_or(ERROR)
    }

    /// `ret`, a member's declared result in class `owner`, seen from the receiver `pre`:
    /// `as_seen_from`, except through an intersection, whose components each give a type
    /// member its own information, met as `goAnd` meets the denotations (`infoMeet`,
    /// `Denotations.scala` 541; two bounds meet as `TypeBounds.&`, `Types.scala` 5750, the
    /// upper bound the one a selection on the result reads, `TypeBounds.underlying`, 5730): the
    /// result the components agree on, the narrower where one conforms to the other, the one
    /// resolved where the other leaves the member open; `None` where two resolved results are
    /// unrelated, or a refinement over an intersection disagrees with it, which is not built.
    fn seen_from_receiver(&mut self, ret: TypeId, pre: TypeId, owner: ClassId, depth: u32) -> Option<TypeId> {
        if !self.types.has_paths(ret) || depth > 8 {
            return Some(ret);
        }
        let t = self.widen_path(pre);
        let t = self.dealias(t);
        match self.types.get(t) {
            Type::Inter(a, b) => {
                let ra = self.seen_from_receiver(ret, a, owner, depth + 1);
                let rb = self.seen_from_receiver(ret, b, owner, depth + 1);
                let resolved = |w: &Self, r: Option<TypeId>| r.filter(|&x| !w.types.has_paths(x));
                match (resolved(self, ra), resolved(self, rb)) {
                    (Some(x), Some(y)) if x == y => Some(x),
                    (Some(x), Some(y)) => {
                        if self.is_sub(x, y) {
                            Some(x)
                        } else if self.is_sub(y, x) {
                            Some(y)
                        } else {
                            None
                        }
                    }
                    (Some(x), None) | (None, Some(x)) => Some(x),
                    (None, None) => None,
                }
            }
            Type::Param(p) | Type::AppParam(p, _) => {
                let upper = self.syms.tparam(p).upper;
                self.seen_from_receiver(ret, upper, owner, depth + 1)
            }
            Type::Refined(parent, _) if self.contains_intersection(parent, 0) => {
                let own = self.as_seen_from(ret, t, owner);
                let through = self.seen_from_receiver(ret, parent, owner, depth + 1)?;
                (own == through).then_some(own)
            }
            _ => Some(self.as_seen_from(ret, t, owner)),
        }
    }

    /// Whether a receiver's type is or holds an intersection, through parameters' bounds and
    /// refinements' parents.
    fn contains_intersection(&mut self, t: TypeId, depth: u32) -> bool {
        if depth > 8 {
            return false;
        }
        let t = self.widen_path(t);
        let t = self.dealias(t);
        match self.types.get(t) {
            Type::Inter(..) => true,
            Type::Param(p) | Type::AppParam(p, _) => {
                let upper = self.syms.tparam(p).upper;
                self.contains_intersection(upper, depth + 1)
            }
            Type::Refined(parent, _) => self.contains_intersection(parent, depth + 1),
            _ => false,
        }
    }

    /// The class of a type: of a path its underlying type's, of a parameter its upper bound's,
    /// of a refinement its parent's.
    pub(in crate::typer) fn class_of_prefix_type(&mut self, t: TypeId, depth: u32) -> Option<ClassId> {
        let t = self.receiver_type(t, depth)?;
        match self.types.get(t) {
            Type::Class(c, _) | Type::Ctor(c) | Type::This(c) => Some(c),
            Type::Any => Some(self.b.any_ref),
            _ => None,
        }
    }

    /// The type the members selected from `t` are looked up in: a path's underlying type, an
    /// alias's expansion, a type parameter's upper bound, a refinement's parent, of an
    /// intersection the component whose class is the more specific (`Base & Left` selects in
    /// `Left`), each step repeated until a class type stands.
    fn receiver_type(&mut self, t: TypeId, depth: u32) -> Option<TypeId> {
        if depth > 8 {
            return None;
        }
        let t = self.widen_path(t);
        let t = self.dealias(t);
        match self.types.get(t) {
            Type::Param(p) | Type::AppParam(p, _) => {
                let upper = self.syms.tparam(p).upper;
                self.receiver_type(upper, depth + 1)
            }
            Type::Refined(parent, _) => self.receiver_type(parent, depth + 1),
            Type::Inter(a, b) => {
                let (ra, rb) = (self.receiver_type(a, depth + 1), self.receiver_type(b, depth + 1));
                let (Some(ra), Some(rb)) = (ra, rb) else { return ra.or(rb) };
                let (Some(ka), Some(kb)) = (self.class_of_prefix_type(ra, depth + 1), self.class_of_prefix_type(rb, depth + 1)) else { return Some(ra) };
                self.complete_class(kb);
                let b_derives = self.syms.class(kb).base_types.iter().any(|&(c, _)| c == ka);
                Some(if b_derives && ka != kb { rb } else { ra })
            }
            _ => Some(t),
        }
    }

    /// A member of the class `c` as a target: the tables' symbol, else for a class of the
    /// builtin layer, whose members teq models without symbols (`Int.+`, `Boolean.unary_!`),
    /// the declaration in the jar's TASTy of the class, a definition of the jar's document.
    fn member_target(&mut self, c: ClassId, name: Name, signed: Option<NameRef>, tasty: &TastyFile) -> Option<DocTarget> {
        if let Some(t) = self.class_member_named(c, name, signed, Some(tasty)) {
            return Some(self.shared(t));
        }
        if self.syms.class(c).kind != ClassKind::Builtin {
            return None;
        }
        self.jar_builtin_member(c, name, signed, tasty)
    }

    /// The `DEFDEF` or `VALDEF` named `name` of the jar's class of the builtin `c`'s name in its
    /// package, among several the one whose parameters are those the signed name erases to; the
    /// file opened for it, nothing of it entered.
    fn jar_builtin_member(&mut self, c: ClassId, name: Name, signed: Option<NameRef>, tasty: &TastyFile) -> Option<DocTarget> {
        let (class_name, pkg) = match self.syms.class(c).owner {
            Owner::Package(p) => (self.syms.class(c).name, p),
            _ => return None,
        };
        let slot = self.loaded.as_ref()?.slot(pkg)?;
        let stem = crate::classpath::encode_name(self.name_ref(class_name)).into_owned();
        let f = *self.loaded.as_ref()?.pkg_names(slot).get(stem.as_str())?;
        let file = self.open_file(f)?;
        let jar_tasty = self.tasty(file);
        let wanted_class = self.name_ref(class_name).to_string();
        let class_addr = crate::tasty::tree::index_top_level(&jar_tasty)
            .into_iter()
            .map(|t| t.entry)
            .find(|e| e.tag == TYPEDEF && e.is_class && !jar_tasty.is_object_class(e.name) && jar_tasty.name(e.name) == wanted_class)
            .map(|e| e.addr)?;
        let mut decoder = Decoder::new(&jar_tasty);
        let sig = decoder.class_sig(class_addr);
        let wanted = self.name_ref(name).to_string();
        let candidates: Vec<Addr> = sig
            .index
            .members
            .iter()
            .filter(|e| matches!(e.tag, DEFDEF | VALDEF) && jar_tasty.name(jar_tasty.source_name(e.name)) == wanted)
            .map(|e| e.addr)
            .collect();
        if let [one] = candidates[..] {
            return Some(DocTarget::Local(file, one));
        }
        // The parameters the signed name erases to, against each candidate's declared ones.
        let Some(crate::tasty::TName::Signed { params, .. }) = signed.and_then(|n| tasty.names.get(n as usize)) else { return None };
        let erased: Vec<String> = params.iter().filter(|&&p| p >= 0).map(|&p| tasty.name(p as u32)).collect();
        for a in candidates {
            let ds = decoder.def_sig(a);
            let declared: Vec<String> = ds
                .clauses
                .iter()
                .filter_map(|cl| if let crate::tasty::tree::Clause::Terms(ps) = cl { Some(ps) } else { None })
                .flatten()
                .map(|p| crate::tasty::terms::qualified_name(&jar_tasty, &decoder, &p.ty))
                .collect();
            if declared == erased {
                return Some(DocTarget::Local(file, a));
            }
        }
        None
    }

    /// The member `name` of the class `c` or of its ancestors: among several alternatives the
    /// one the signed name `signed` states, else none where the name alone does not tell.
    pub(in crate::typer) fn class_member_named(&mut self, c: ClassId, name: Name, signed: Option<NameRef>, tasty: Option<&TastyFile>) -> Option<Target> {
        self.complete_class(c);
        if self.syms.class(c).kind == ClassKind::Object {
            if let Some(r) = self.module_term(c, name) {
                let found = match r {
                    super::super::resolve::TermRef::Class(k) => Some(Target::Class(k)),
                    other => other.sym().map(Target::Sym),
                };
                if let Some(t) = found {
                    if let Target::Sym(m) = t {
                        if self.syms.alternatives(m).is_some() {
                            return self.alternative_of(c, m, name, signed, tasty).map(Target::Sym);
                        }
                    }
                    return Some(t);
                }
            }
        }
        let mut owners = vec![c];
        owners.extend(self.syms.class(c).base_types.iter().skip(1).map(|&(b, _)| b));
        for k in owners {
            self.complete_class(k);
            let Some(&m) = self.syms.class(k).members.get(&name) else { continue };
            if self.syms.alternatives(m).is_some() {
                return self.alternative_of(c, m, name, signed, tasty).map(Target::Sym);
            }
            return Some(Target::Sym(m));
        }
        None
    }

    /// Of an overloaded member's alternatives, the one a signed name states.
    fn alternative_of(&mut self, c: ClassId, m: SymId, name: Name, signed: Option<NameRef>, tasty: Option<&TastyFile>) -> Option<SymId> {
        let alts: Vec<SymId> = self.syms.alternatives(m)?.to_vec();
        if let [one] = alts[..] {
            return Some(one);
        }
        let (Some(n), Some(tasty)) = (signed, tasty) else { return None };
        let mut owners = vec![c];
        owners.extend(self.syms.class(c).base_types.iter().skip(1).map(|&(b, _)| b));
        for k in owners {
            let found = self.signed_alternatives(k, tasty, n)?;
            match found[..] {
                [] => continue,
                [s] => return Some(s),
                _ => return None,
            }
        }
        let _ = name;
        None
    }

    /// The member a `SELECTin` names: the declaration its owner type and signed name state.
    fn select_in_target(&mut self, cx: &mut MapCx, file: u32, tasty: &TastyFile, n: NameRef, owner_at: Addr, qual: Addr) -> Option<DocTarget> {
        let owner = Decoder::new(tasty).type_at(owner_at);
        let name = self.lname(tasty, n);
        let signed = matches!(tasty.names.get(n as usize), Some(crate::tasty::TName::Signed { .. })).then_some(n);
        let k = match &owner {
            TType::LocalType(addr, _) => self.file_tables(file).classes.get(addr).copied(),
            other => {
                let ctor = self.map_type_ctor(cx, other);
                match self.types.get(ctor) {
                    Type::Class(k, _) | Type::Ctor(k) => Some(k),
                    _ => None,
                }
            }
        };
        let k = match k {
            Some(k) => k,
            None => {
                // The owner's type as the qualifier has it.
                let q = Decoder::new(tasty).type_at(qual);
                self.class_of_prefix(cx, &q)?
            }
        };
        self.member_target(k, name, signed, tasty)
    }

    // ---- what the queries read of the index ----

    /// The document and the span of the name of a local definition.
    pub(in crate::typer) fn local_declaration(&mut self, file: u32, addr: Addr) -> Option<(super::attach::Document, Span)> {
        let positions = self.file_positions(file);
        let pos = positions.section.as_ref()?.entry(addr)?;
        let recorded = positions.source_path(addr, None)?;
        let doc = self.source_document(file, &recorded)?;
        let tasty = self.tasty(file);
        let n = Decoder::new(&tasty).name_at(addr)?;
        let written = tasty.name(tasty.source_name(n));
        let units = utf16_table(&doc.text);
        let (span, _) = placed_span(&doc.text, &units, pos.point, pos.start, &written)?;
        Some((doc, span))
    }

    /// What hover shows of a local definition: its kind, name and type, read from its tree.
    pub(in crate::typer) fn describe_local(&mut self, file: u32, addr: Addr) -> Option<String> {
        let tasty = self.tasty(file);
        let mut decoder = Decoder::new(&tasty);
        let tag = decoder.tag_at(addr);
        let mut cx = MapCx::new(file);
        let n = decoder.name_at(addr)?;
        let name = tasty.name(tasty.source_name(n));
        let name = if name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '$') || name.chars().all(|c| !c.is_alphanumeric() && c != '_') { name } else { format!("`{}`", name) };
        Some(match tag {
            VALDEF | PARAM => {
                let sig = decoder.def_sig(addr);
                let ty = self.map_type(&mut cx, &sig.ret);
                let keyword = if tag == PARAM { "" } else if sig.mods.flags.has(MUTABLE) { "var " } else if sig.mods.flags.has(LAZY) { "lazy val " } else { "val " };
                format!("{}{}: {}", keyword, name, self.show(ty))
            }
            DEFDEF => {
                let sig = decoder.def_sig(addr);
                let mut out = format!("def {}", name);
                for clause in &sig.clauses {
                    match clause {
                        crate::tasty::tree::Clause::Types(ps) => {
                            let names: Vec<String> = ps.iter().map(|p| tasty.name(p.name)).collect();
                            out.push_str(&format!("[{}]", names.join(", ")));
                        }
                        crate::tasty::tree::Clause::Terms(ps) => {
                            let using = super::is_using_clause(ps);
                            let shown: Vec<String> = ps
                                .iter()
                                .map(|p| {
                                    let ty = self.map_type(&mut cx, &p.ty);
                                    format!("{}: {}", tasty.name(p.name), self.show(ty))
                                })
                                .collect();
                            out.push_str(&format!("({}{})", if using { "using " } else { "" }, shown.join(", ")));
                        }
                    }
                }
                let ret = self.map_type(&mut cx, &sig.ret);
                out.push_str(&format!(": {}", self.show(ret)));
                out
            }
            TYPEPARAM => format!("type {}", name),
            TYPEDEF => {
                let mut d = Decoder::new(&tasty);
                if d.local_type_kind(addr) == crate::tasty::tree::LocalKind::Class {
                    format!("class {}", name)
                } else {
                    let sig = d.type_def_sig(addr);
                    match &sig.rhs {
                        TType::Bounds(lo, hi) => {
                            let (lo, hi) = (self.map_type(&mut cx, lo), self.map_type(&mut cx, hi));
                            format!("type {} >: {} <: {}", name, self.show(lo), self.show(hi))
                        }
                        rhs => {
                            let t = self.map_type(&mut cx, rhs);
                            format!("type {} = {}", name, self.show(t))
                        }
                    }
                }
            }
            BIND => match decoder.bind_type(addr) {
                Some(ty) => {
                    let t = self.map_type(&mut cx, &ty);
                    format!("{}: {}", name, self.show(t))
                }
                None => name,
            },
            _ => return None,
        })
    }

    /// Whether a local definition is a method.
    pub(in crate::typer) fn local_is_method(&self, file: u32, addr: Addr) -> bool {
        let tasty = self.tasty(file);
        Decoder::new(&tasty).tag_at(addr) == DEFDEF
    }
}

fn type_ref_target(r: super::super::resolve::TypeRef) -> Option<DocTarget> {
    use super::super::resolve::TypeRef;
    match r {
        TypeRef::Class(c) => Some(DocTarget::Shared(Target::Class(c))),
        TypeRef::Alias(a) => Some(DocTarget::Shared(Target::Alias(a))),
        TypeRef::Param(p) => Some(DocTarget::Shared(Target::TParam(p))),
        TypeRef::Member(..) | TypeRef::ValueMember(..) => None,
    }
}

/// The span of the written name `name` at a tree's position: at its point where the text spells
/// it there, else at its start (a reconstructed span, a `NAMEDARG` whose point is at `=`), with
/// whether the start was taken.
fn placed_span(text: &str, units: &[u32], point: Option<u32>, start: u32, name: &str) -> Option<(Span, bool)> {
    let unit = |u: u32| -> usize { units.get(u as usize).copied().unwrap_or(text.len() as u32) as usize };
    if let Some(p) = point {
        if let Some(span) = name_span(text, unit(p), name) {
            return Some((span, false));
        }
    }
    if point != Some(start) {
        if let Some(span) = name_span(text, unit(start), name) {
            return Some((span, true));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{placed_span, utf16_table};
    use crate::source::Span;

    #[test]
    fn utf16_offsets_map_to_bytes() {
        let text = "a😀b";
        let table = utf16_table(text);
        assert_eq!(table, vec![0, 1, 1, 5, 6]);
    }

    #[test]
    fn a_name_is_taken_at_the_point_else_at_the_start() {
        let text = "😀 helper(x = 2) + x";
        let units = utf16_table(text);
        let unit = |s: &str| text[..text.find(s).unwrap()].encode_utf16().count() as u32;
        // A selection's point is at its name.
        assert_eq!(placed_span(text, &units, Some(unit("helper")), unit("helper"), "helper"), Some((Span::new(5, 11), false)));
        // A named argument's point is at `=`: the name at its start.
        assert_eq!(placed_span(text, &units, Some(unit("= 2")), unit("x = 2"), "x"), Some((Span::new(12, 13), true)));
        // A synthetic tree spells nothing.
        assert_eq!(placed_span(text, &units, None, unit("+ x"), "plus"), None);
        // A span that holds a longer token is not the name.
        assert_eq!(placed_span(text, &units, Some(unit("helper")), unit("helper"), "help"), None);
    }
}
