//! The standard library entered on demand. Every std file of the build has a file slot from the
//! start (`frontend::lay_out` reserves them in the order of `main.rs`'s lists, so the ids of the
//! program's files and the layout of the output depend on the index alone), but a file is parsed
//! and entered only when a lookup first asks for a name it defines: a qualified name, an import's
//! prefix, an extension method's name, a binding the loader makes for a jar's body, or one of
//! the typer's own asks for the language's types (`seq_class` and the other accessors). The
//! index of names comes from `build.rs` (`crate::stdindex`). Files the program's text names
//! are parsed with the program (the pre-pass in `main.rs`); a file first asked for while typing
//! enters then, with its classes completed and checked as `check_all` would have done, which
//! is the fallback path the parallel typer keeps behind a lock.
//!
//! A definition of the program shadows the std's of the same qualified name (`namer.rs`): the
//! std's is skipped, or gives way when the program's enters after it, so that an application's
//! own `java.math` or `scala.scalajs` stand-ins take the std's place definition by definition.

use super::profile::Phase;
use super::Worker;
use std::sync::atomic::{AtomicBool, Ordering};
use crate::intern::{FxMap, Name};
use crate::source::FileId;
use crate::stdindex::StdFileIndex;
use crate::symbols::*;
use crate::types::*;

/// A layer of std files that stays out until a package of it is asked for, so that a program
/// naming none of it pays nothing: the Scala.js facades. Everything else is `Base`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Layer {
    Base,
    ScalaJs,
}

/// One std file of the build and where it stands in the file table.
pub struct StdSlot {
    pub index: &'static StdFileIndex,
    pub text: &'static str,
    pub file: FileId,
    /// The file ids of its `package p:` blocks, in the parser's order.
    pub blocks: Vec<FileId>,
    pub layer: Layer,
    /// Whether the AST is in place at `file` and `blocks`: parsed with the program, or here.
    pub parsed: AtomicBool,
    pub entered: AtomicBool,
    /// Entered and published: what a worker without the loader's lock may rely on. Set when
    /// the entering worker's lock hold ends (`Worker::with_loader`), at once with one worker.
    pub visible: AtomicBool,
}

impl StdSlot {
    #[inline]
    pub fn entered(&self) -> bool {
        self.entered.load(Ordering::Acquire)
    }

    #[inline]
    pub fn visible(&self) -> bool {
        self.visible.load(Ordering::Acquire)
    }
    #[inline]
    pub fn parsed(&self) -> bool {
        self.parsed.load(Ordering::Acquire)
    }
}

/// The index over the std files' definitions, replaced as a whole when a layer unlocks
/// (`StdFiles::index`), so that a reader never sees it half built.
#[derive(Default, Clone)]
pub struct StdIndex {
    /// (package, name) → the slots that define the name in the package and as what (`TYPE`,
    /// `TERM` or both), layers unlocked only.
    pub by_name: FxMap<(PkgId, Name), Vec<(u16, u8)>>,
    /// package → the slots with givens or implicit definitions in it that no lookup names.
    pub givens_in: FxMap<PkgId, Vec<u16>>,
    /// (package, name) → the slots with a class extending that class, and name → the slots
    /// with a class extending some class of that name the index could not place.
    pub by_parent: FxMap<(PkgId, Name), Vec<u16>>,
    pub by_parent_name: FxMap<Name, Vec<u16>>,
}

/// The std files as the parallel typer's workers share them: the slots' flags are atomics,
/// the index is swapped as a whole, and the rest is written before the arenas fork or under
/// the loader's lock (`Worker::std_mut`).
pub struct StdCell(pub std::cell::UnsafeCell<StdFiles>);

unsafe impl Sync for StdCell {}
unsafe impl Send for StdCell {}

impl StdCell {
    pub fn new(files: StdFiles) -> StdCell {
        StdCell(std::cell::UnsafeCell::new(files))
    }
}

impl std::ops::Deref for StdCell {
    type Target = StdFiles;
    #[inline]
    fn deref(&self) -> &StdFiles {
        unsafe { &*self.0.get() }
    }
}

pub struct StdFiles {
    pub slots: Vec<StdSlot>,
    index: std::sync::atomic::AtomicPtr<StdIndex>,
    /// The indexes replaced, kept for the readers that still hold one.
    retired: Vec<Box<StdIndex>>,
    /// file → its slot, for the main file and the blocks of every std file.
    slot_of_file: FxMap<FileId, u16>,
    /// The segments of the package paths of the locked layers: a package ask for any other
    /// name unlocks nothing.
    locked_segments: FxMap<Name, ()>,
    /// The std classes a program definition shadowed: completed by nothing.
    pub shadowed_classes: FxMap<ClassId, ()>,
    scalajs_unlocked: AtomicBool,
    /// The definitions the program shadowed, as `path.name (file)`.
    pub shadowed: Vec<String>,
    /// How many files entered after `enter_all`, for the profile.
    pub late_entries: u32,
    /// The packages the files entered late touched, in order: every worker reads the log from
    /// where it left it (`Worker::refresh_std_if_stale`) and drops the memos of those packages
    /// and finds the std classes again, as the worker that entered the files did.
    pub late_pkgs: crate::shared::SlabVec<PkgId>,
    /// Under `--profile`, the name each file entered while typing was asked as.
    pub asked_as: Vec<(u16, String)>,
}

impl Default for StdFiles {
    fn default() -> StdFiles {
        StdFiles {
            slots: Vec::new(),
            index: std::sync::atomic::AtomicPtr::new(Box::into_raw(Box::new(StdIndex::default()))),
            retired: Vec::new(),
            slot_of_file: FxMap::default(),
            locked_segments: FxMap::default(),
            shadowed_classes: FxMap::default(),
            scalajs_unlocked: AtomicBool::new(false),
            shadowed: Vec::new(),
            late_entries: 0,
            late_pkgs: crate::shared::SlabVec::with_capacity(16),
            asked_as: Vec::new(),
        }
    }
}

impl Drop for StdFiles {
    fn drop(&mut self) {
        drop(unsafe { Box::from_raw(*self.index.get_mut()) });
    }
}

impl StdFiles {
    pub fn slot_of(&self, file: FileId) -> Option<usize> {
        self.slot_of_file.get(&file).map(|&i| i as usize)
    }

    /// The index over the definitions, as it stands.
    #[inline]
    pub fn index(&self) -> &StdIndex {
        unsafe { &*self.index.load(Ordering::Acquire) }
    }

    /// Puts a new index in place; the old one stays for whoever reads it.
    fn set_index(&mut self, index: StdIndex) {
        let old = self.index.swap(Box::into_raw(Box::new(index)), Ordering::AcqRel);
        self.retired.push(unsafe { Box::from_raw(old) });
    }

    pub fn scalajs_unlocked(&self) -> bool {
        self.scalajs_unlocked.load(Ordering::Acquire)
    }
}

impl<'a> Worker<'a> {
    /// Installs the std files of the build. Packages of the unlocked layers are created here,
    /// so that a path through them resolves before any of their files is entered, and the
    /// names are indexed by (package, name).
    pub fn set_std(&mut self, slots: Vec<StdSlot>, scalajs_unlocked: bool) {
        let std = self.std_mut();
        std.slots = slots;
        std.scalajs_unlocked.store(scalajs_unlocked, Ordering::Release);
        for (i, slot) in std.slots.iter().enumerate() {
            std.slot_of_file.insert(slot.file, i as u16);
            for &b in &slot.blocks {
                std.slot_of_file.insert(b, i as u16);
            }
        }
        let mut index = StdIndex::default();
        for i in 0..self.std.slots.len() {
            if self.std.slots[i].layer == Layer::Base || scalajs_unlocked {
                self.index_std_slot(i, &mut index);
            } else {
                for path in self.std.slots[i].index.packages {
                    for seg in path.split('.') {
                        let n = self.interner.intern(seg);
                        self.std_mut().locked_segments.insert(n, ());
                    }
                }
            }
        }
        self.std_mut().set_index(index);
    }

    fn index_std_slot(&mut self, i: usize, into: &mut StdIndex) {
        let index = self.std.slots[i].index;
        for path in index.packages {
            self.std_package(path);
        }
        for &(pkg, name, kind) in index.defines {
            let p = self.std_package(pkg);
            let n = self.interner.intern(name);
            into.by_name.entry((p, n)).or_default().push((i as u16, kind));
        }
        for pkg in index.unnamed_givens {
            let p = self.std_package(pkg);
            into.givens_in.entry(p).or_default().push(i as u16);
        }
        for &(pkg, name) in index.extends {
            let n = self.interner.intern(name);
            match self.existing_std_package(pkg) {
                Some(p) => into.by_parent.entry((p, n)).or_default().push(i as u16),
                None => into.by_parent_name.entry(n).or_default().push(i as u16),
            }
        }
    }

    /// A package of the index that exists already; a path through a class (`scala.deriving.Mirror`)
    /// or an empty one is none.
    fn existing_std_package(&mut self, path: &str) -> Option<PkgId> {
        if path.is_empty() {
            return None;
        }
        let mut p = ROOT_PKG;
        for seg in path.split('.') {
            let n = self.interner.lookup(seg)?;
            p = self.syms.pkg(p).entries.get(&n).and_then(|e| e.pkg)?;
        }
        Some(p)
    }

    /// Enters the std files with classes below the std class `c`, transitively, so that its
    /// subclasses are what they are with the whole std entered: what the emitters and the
    /// match checker read of a class's hierarchy.
    pub fn demand_extenders_transitive(&mut self, c: ClassId) {
        if self.std.slot_of(self.syms.class(c).file).is_none() || self.syms.class(c).kind == ClassKind::Trait {
            return;
        }
        let mut todo = vec![c];
        let mut seen: Vec<ClassId> = Vec::new();
        while let Some(k) = todo.pop() {
            if seen.contains(&k) {
                continue;
            }
            seen.push(k);
            self.demand_extenders(k);
            todo.extend(self.syms.class(k).subclasses.iter().copied());
        }
    }

    /// Enters the std files of the build's layers whose text names `word`, whether or not the
    /// program named what they define.
    pub(super) fn enter_std_naming(&mut self, word: &str) {
        let open = |s: &StdSlot| s.layer == Layer::Base || self.std.scalajs_unlocked();
        let slots: Vec<usize> = (0..self.std.slots.len()).filter(|&i| !self.std.slots[i].entered() && open(&self.std.slots[i]) && self.std.slots[i].text.contains(word)).collect();
        for i in slots {
            self.enter_std_slot(i);
        }
    }

    /// Enters the std files with a class extending the std class `c`.
    fn demand_extenders(&mut self, c: ClassId) {
        let (name, owner) = {
            let info = self.syms.class(c);
            (info.name, info.owner)
        };
        let mut slots: Vec<u16> = Vec::new();
        if let Owner::Package(p) = owner {
            if let Some(found) = self.std.index().by_parent.get(&(p, name)) {
                slots.extend(found.iter().copied());
            }
        }
        if let Some(found) = self.std.index().by_parent_name.get(&name) {
            slots.extend(found.iter().copied());
        }
        slots.sort_unstable();
        slots.dedup();
        for i in slots {
            if !self.std.slots[i as usize].entered() {
                if self.profile.on && self.entering_done {
                    let asked = format!("a subclass of {}", self.name_str(name));
                    self.std_mut().asked_as.push((i, asked));
                }
                self.enter_std_slot(i as usize);
            }
            self.complete_slot_classes(i as usize);
        }
    }

    /// Completes every class of a std file, which registers each with its superclass and, for
    /// a sealed parent, among its children.
    fn complete_slot_classes(&mut self, i: usize) {
        let files: Vec<FileId> = std::iter::once(self.std.slots[i].file).chain(self.std.slots[i].blocks.iter().copied()).collect();
        for f in files {
            self.complete_file_classes(f);
        }
    }

    /// Completes the classes a file defines at any depth, in the order they were entered.
    pub fn complete_file_classes(&mut self, f: FileId) {
        let mut classes: Vec<ClassId> = self.def_classes.values_in(f.0 as usize);
        classes.sort_unstable();
        for c in classes {
            if !self.std.shadowed_classes.contains_key(&c) {
                self.complete_class(c);
            }
        }
    }

    fn std_package(&mut self, path: &str) -> PkgId {
        let mut p = ROOT_PKG;
        for seg in path.split('.') {
            let n = self.interner.intern(seg);
            p = self.syms.sub_pkg(p, n);
        }
        p
    }

    /// Enters the std files defining `name` in package `p` as a `kind` (`stdindex::TYPE`,
    /// `TERM`) that are not entered yet; whether any did, in which case the package's entries
    /// answer the lookup that missed. With several workers, also whether another worker entered
    /// one since the caller read the entries: they are read again.
    #[inline]
    pub fn demand_std(&mut self, p: PkgId, name: Name, kind: u8) -> bool {
        let Some(slots) = self.std.index().by_name.get(&(p, name)) else { return false };
        // A slot another worker is entering counts as not entered: the lock waits for it.
        if !slots.iter().any(|&(i, k)| k & kind != 0 && !self.std.slots[i as usize].visible()) {
            if self.forked {
                crate::measure::looked_up_unlocked(crate::measure::Lookup::StdName);
                return slots.iter().any(|&(_, k)| k & kind != 0);
            }
            return false;
        }
        self.enter_std_for(p, name, kind)
    }

    #[cold]
    #[inline(never)]
    fn enter_std_for(&mut self, p: PkgId, name: Name, kind: u8) -> bool {
        self.with_loader_for(crate::measure::Hold::StdLookup, |w| w.enter_std_for_unlocked(p, name, kind))
    }

    fn enter_std_for_unlocked(&mut self, p: PkgId, name: Name, kind: u8) -> bool {
        let slots: Vec<u16> = self.std.index().by_name[&(p, name)].iter().filter(|&&(i, k)| k & kind != 0 && !self.std.slots[i as usize].entered()).map(|&(i, _)| i).collect();
        crate::measure::looked_up(crate::measure::Lookup::StdName, !slots.is_empty());
        for i in slots {
            if self.profile.on && self.entering_done {
                let asked = format!("{}.{}", self.pkg_path(p), self.name_str(name));
                self.std_mut().asked_as.push((i, asked));
            }
            self.enter_std_slot(i as usize);
        }
        true
    }

    /// Enters the std files with package-level givens or implicit definitions of `p` that no
    /// name asks for, before the package's given or conversion index is built.
    pub fn demand_std_givens(&mut self, p: PkgId) {
        self.with_loader_for(crate::measure::Hold::StdEntry, |w| w.demand_std_givens_unlocked(p))
    }

    pub fn demand_std_givens_unlocked(&mut self, p: PkgId) {
        let Some(slots) = self.std.index().givens_in.get(&p) else { return };
        let slots: Vec<u16> = slots.iter().copied().filter(|&i| !self.std.slots[i as usize].entered()).collect();
        for i in slots {
            self.enter_std_slot(i as usize);
        }
    }

    /// A package asked for under `parent` that a locked layer defines: the layer unlocks and
    /// the package exists from then on. `None` for any other name.
    #[inline]
    pub fn demand_std_package(&mut self, parent: PkgId, name: Name) -> Option<PkgId> {
        if self.std.scalajs_unlocked() || !self.std.locked_segments.contains_key(&name) {
            return None;
        }
        self.unlock_std_package(parent, name)
    }

    #[cold]
    #[inline(never)]
    fn unlock_std_package(&mut self, parent: PkgId, name: Name) -> Option<PkgId> {
        self.with_loader_for(crate::measure::Hold::StdEntry, |w| w.unlock_std_package_unlocked(parent, name))
    }

    fn unlock_std_package_unlocked(&mut self, parent: PkgId, name: Name) -> Option<PkgId> {
        let mut path = self.pkg_path(parent);
        if !path.is_empty() {
            path.push('.');
        }
        path.push_str(self.interner.get(name));
        let opens = self.std.slots.iter().any(|s| {
            s.layer == Layer::ScalaJs && s.index.packages.iter().any(|p| *p == path || p.starts_with(&format!("{}.", path)))
        });
        if !opens {
            return None;
        }
        self.unlock_scalajs();
        self.syms.pkg(parent).entries.get(&name).and_then(|e| e.pkg)
    }

    fn unlock_scalajs(&mut self) {
        self.std.scalajs_unlocked.store(true, Ordering::Release);
        let mut index = self.std.index().clone();
        for i in 0..self.std.slots.len() {
            if self.std.slots[i].layer == Layer::ScalaJs {
                self.index_std_slot(i, &mut index);
            }
        }
        self.std_mut().set_index(index);
    }

    pub fn pkg_path(&self, p: PkgId) -> String {
        let mut segs = Vec::new();
        let mut at = Some(p);
        while let Some(k) = at.filter(|&k| k != ROOT_PKG) {
            segs.push(self.interner.get(self.syms.pkg(k).name).to_string());
            at = self.syms.pkg(k).parent;
        }
        segs.reverse();
        segs.join(".")
    }

    /// The std files to enter before the program's: those parsed with it. Called by
    /// `enter_all` in slot order, so that their symbols come before the program's.
    pub(super) fn enter_parsed_std(&mut self) {
        debug_assert!(!self.forked, "the std files parsed with the program are entered before the fork");
        for i in 0..self.std.slots.len() {
            if self.std.slots[i].parsed() && !self.std.slots[i].entered() {
                self.std.slots[i].entered.store(true, Ordering::Release);
                // Entered before the fork, and so every worker's: a name one of them defines is
                // answered from the index without the loader's lock (`demand_std`).
                self.std.slots[i].visible.store(true, Ordering::Release);
                let files: Vec<FileId> = std::iter::once(self.std.slots[i].file).chain(self.std.slots[i].blocks.iter().copied()).collect();
                for f in files {
                    self.enter_file_defs(f);
                }
            }
        }
    }

    /// Enters one std file now: parses it if the pre-pass did not, enters its definitions, and
    /// past `enter_all` completes and checks them as the eager passes would have.
    #[cold]
    #[inline(never)]
    /// Enters every std file, the Scala.js layer's too: what lists the std's definitions whole
    /// (`TEQ_STD_SHAPES`).
    pub(crate) fn enter_all_std(&mut self) {
        if !self.std.scalajs_unlocked() {
            self.with_loader_for(crate::measure::Hold::StdEntry, |w| w.unlock_scalajs());
        }
        for i in 0..self.std.slots.len() {
            self.enter_std_slot(i);
        }
    }

    pub(super) fn enter_std_slot(&mut self, i: usize) {
        if self.std.slots[i].entered() {
            return;
        }
        self.with_loader_for(crate::measure::Hold::StdEntry, |w| w.enter_std_slot_unlocked(i))
    }

    fn enter_std_slot_unlocked(&mut self, i: usize) {
        if self.std.slots[i].entered() {
            return;
        }
        if super::prep::capturing() {
            self.prep_note_slot(i);
        }
        self.std.slots[i].entered.store(true, Ordering::Release);
        if self.forked {
            self.std_entered_pending.push(i as u16);
        } else {
            self.std.slots[i].visible.store(true, Ordering::Release);
        }
        let p = self.phase(Phase::StdEnter);
        let (file, blocks) = (self.std.slots[i].file, self.std.slots[i].blocks.clone());
        if !self.std.slots[i].parsed() {
            self.parse_std_slot(i);
        }
        let files: Vec<FileId> = std::iter::once(file).chain(blocks.iter().copied()).collect();
        let saved_env_file = self.env.file;
        let first_overload = self.syms.overloads.len();
        for &f in &files {
            self.enter_file_defs(f);
        }
        self.env.file = saved_env_file;
        self.note_singleton_class();
        for &f in &files {
            if self.ast(f).tys.iter().any(|t| matches!(t, crate::ast::TyExpr::Wildcard | crate::ast::TyExpr::BoundedWildcard(..))) {
                self.wildcards_used.store(true, std::sync::atomic::Ordering::Release);
            }
        }
        if self.entering_done {
            self.std_mut().late_entries += 1;
            let touched: Vec<PkgId> = files.iter().map(|&f| self.file_pkgs[f.0 as usize]).collect();
            for p in touched {
                self.std.late_pkgs.push(p);
            }
            self.refresh_std_if_stale();
            // The names of overloaded members are settled by the final pass; a file entering after
            // it has its overload sets named before a body calls one of them, once no completion
            // is under way, since naming completes signatures.
            if self.alternatives_named {
                self.late_naming_from = Some(self.late_naming_from.unwrap_or(first_overload));
                self.outermost_completion_done();
            }
            // The walk over the files checks what is still ahead of it; the rest is checked here.
            if self.walk.started {
                let passed: Vec<FileId> = files.iter().copied().filter(|&f| self.walk.passed(f)).collect();
                if !passed.is_empty() {
                    self.check_std_files_now(&passed);
                }
            }
        }
        self.phase_end(p);
    }

    /// Checks the files of a std slot entered after the walk over the files passed their
    /// slots, in the state the walk would have had.
    fn check_std_files_now(&mut self, files: &[FileId]) {
        let env = std::mem::replace(&mut self.env, super::Env { file: files[0], frames: Vec::new(), imports: Vec::new() });
        let transparent = std::mem::take(&mut self.transparent);
        let ext_scope = self.ext_scope.take();
        let defining = self.defining.take();
        let walk_done = std::mem::replace(&mut self.walk.done, true);
        self.outside_inline(|t| {
            for &f in files {
                t.check_file(f);
            }
        });
        self.walk.done = walk_done;
        self.defining = defining;
        self.ext_scope = ext_scope;
        self.transparent = transparent;
        self.env = env;
    }

    fn parse_std_slot(&mut self, i: usize) {
        let slot = &self.std.slots[i];
        let (file, text) = (slot.file, slot.text);
        let blocks = slot.blocks.clone();
        // A language server's session records the std's names (`index::navigable`), whose spans
        // the parser keeps for it.
        let syntax = crate::parser::Syntax { index: self.index.is_some(), ..Default::default() };
        let (mut asts, errors) = crate::frontend::parse_one(text, self.interner, syntax);
        if syntax.index {
            asts.iter_mut().for_each(crate::ast::Ast::shrink_index_spans);
        }
        assert_eq!(asts.len(), 1 + blocks.len(), "the std index counts the package blocks of {}", slot.index.path);
        // The file's text, for the diagnostics that point into it.
        for &f in std::iter::once(&file).chain(&blocks) {
            self.files.set_text(f.0 as usize, text.to_string());
        }
        for (span, msg) in errors {
            self.diags.error(file, span, msg);
        }
        for (ast, f) in asts.into_iter().zip(std::iter::once(file).chain(blocks)) {
            self.asts.replace(f.0 as usize, ast);
        }
        self.std.slots[i].parsed.store(true, Ordering::Release);
    }

    /// The std files entered and the definitions the program shadowed, for `--profile`.
    pub fn std_report(&self) -> String {
        use std::fmt::Write as _;
        let mut out = String::new();
        let entered: Vec<String> = self
            .std
            .slots
            .iter()
            .enumerate()
            .filter(|(_, s)| s.entered())
            .map(|(i, s)| {
                let path = s.index.path.trim_start_matches("<std>/");
                match self.std.asked_as.iter().find(|(k, _)| *k as usize == i) {
                    Some((_, asked)) => format!("{} (as {})", path, asked),
                    None => path.to_string(),
                }
            })
            .collect();
        let _ = writeln!(
            out,
            "std: {} of {} files entered ({} while typing): {}",
            entered.len(),
            self.std.slots.len(),
            self.std.late_entries,
            entered.join(", ")
        );
        if !self.std.shadowed.is_empty() {
            let _ = writeln!(out, "std definitions shadowed by the program: {}", self.std.shadowed.join(", "));
        }
        out
    }

    /// The std's definitions the program shadowed, for the profile; a shadowed std class is
    /// completed by nothing.
    pub fn note_shadowed(&mut self, owner: Owner, name: Name, file: FileId) {
        let owner = match owner {
            Owner::Package(p) => self.pkg_path(p),
            Owner::Class(c) => self.class_path(c),
            Owner::Local => String::new(),
        };
        let path = if owner.is_empty() { self.name_str(name) } else { format!("{}.{}", owner, self.name_str(name)) };
        let note = format!("{} ({})", path, self.source(file).path);
        self.std_mut().shadowed.push(note);
    }
}
