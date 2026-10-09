//! Classes and members read from the TASTy files of `--classpath`. A package lists its classes
//! from the jars' central directories and enters one when a lookup names it; a class decodes
//! its header and enters its members when it is completed; a member decodes its signature when
//! `sig_of` asks for it. Nothing is read for what the program does not touch.

pub mod attach;
pub mod bodies;
pub mod compile;
pub mod declared;
pub mod javaclass;
pub mod occurrences;
pub mod places;
mod provenance;
pub mod shapes;
pub(super) mod types;

use super::profile::{About, Kind, Outcome};
use super::Worker;
use crate::arena::SharedMap;
use crate::ast::{mods, Mods};
use crate::classpath::{encode_name, Classpath, CpFile};
use crate::shared::SlabVec;
use crate::intern::{FxMap, Name};
use crate::names;
use crate::source::{FileId, Span};
use crate::symbols::*;
use crate::tasty::tags;
use crate::tasty::tree::{Addr, AnnotArg, Clause, Decoder, DefSig, Entry, Flags, TType};
use crate::tasty::TastyFile;
use crate::types::*;
use std::sync::Arc;

pub use javaclass::Java;
pub use types::MapCx;

/// The classpath's tables as the parallel typer's workers share them: every change is the
/// loader's lock holder's (`Worker::loaded_mut`), and what a worker
/// reads outside the lock is a structure whose readers take none (`SharedMap`, `SlabVec`): the
/// files opened, the classes, members and aliases entered, the access and target names, the inner
/// objects, the pseudo files' sources, Java's classes and placeholders, the products' places,
/// members and manifests. The rest, a `Vec` or an `FxMap` the holder changes, is read under the lock or after
/// the bodies.
pub struct Loaded {
    pub cp: Classpath,
    /// The pseudo file standing for each jar in diagnostics and `SymInfo::file`.
    pub jar_files: Vec<FileId>,
    /// The type aliases of a jar declared `private[p]`, which a wildcard import from outside
    /// `p` does not see (zio's `private[zio] type Callback` in its package object).
    pub qualified_private_aliases: SharedMap<AliasId, ()>,
    /// The members of a jar declared `private[p]` or `protected[p]`, with the simple name of `p`
    /// and whether they are `protected`.
    pub access_within: SharedMap<SymId, (Name, bool)>,
    /// The library classes whose bridges were computed once their members were named.
    pub bridged: FxMap<ClassId, ()>,
    /// The member returning the enclosing instance of a class or trait nested in a class.
    /// The names `@targetName` gives members in the class files, read with their signatures.
    pub target_names: SharedMap<SymId, Name>,
    /// The `@targetName` of a product's member, which its class files do not carry but a pickle
    /// naming the member signs it with.
    pub product_target_names: SharedMap<SymId, Name>,
    /// The vars and vals `@volatile` marks, read with their signatures: a trait's, whose field
    /// a class of the program mixing it in holds (`ACC_VOLATILE`).
    pub volatile: SharedMap<SymId, ()>,
    /// The first file id that is a jar's: the jars' pseudo files follow the program's, and the
    /// pseudo files of library bodies follow them.
    jar_first: u32,
    /// Per package id, the index of its classpath package, `u32::MAX` for a package the jars
    /// hold nothing of.
    pkg_slots: Vec<u32>,
    pkg_states: Vec<PkgState>,
    /// The TASTy files opened, by index, each pushed whole by the lock holder: the entries never
    /// move, so a worker outside the lock reads the file whose index reached it (an `LClass`, an
    /// `LSym`, a `DeclRef`) while the holder pushes more (`Loaded::file`).
    pub files: SlabVec<LoadedFile>,
    /// Per loaded file, what the loader entered of it (`FileTables`): the lock holder's.
    pub tables: Vec<FileTables>,
    /// The files opened, by the lock holder, before their definitions are entered: its marker
    /// against entering one twice, read under the lock alone.
    file_index: FxMap<CpFile, u32>,
    /// The class files whose top-level definitions are entered into their package and
    /// published, marked at the end of the outermost hold that entered them
    /// (`Worker::loader_done`): what a lookup outside the lock takes for "nothing to enter".
    files_done: SharedMap<CpFile, ()>,
    pub classes: SharedMap<ClassId, LClass>,
    pub syms: SharedMap<SymId, LSym>,
    pub aliases: SharedMap<AliasId, LAlias>,
    /// `scala.Predef`, whose members every file sees.
    pub predef: Option<ClassId>,
    /// Under `--std=scala-library` the builtin layer is all the std there is, and it meets the
    /// classpath: nothing of the jars binds to teq's std.
    pub scala_library: bool,
    /// `TEQ_CLASSPATH_DETAIL` is set: every class read, body typed and Java class absorbed is
    /// traced on stderr.
    pub detail: bool,
    /// The Java classes of the jars and the JDK.
    pub java: Java,
    /// The library's own classes behind a builtin of the same name (`scala.Tuple2`), entered
    /// without a place in their package, and the builtin each stands behind.
    shadowed: FxMap<(PkgId, Name), ClassId>,
    pub builtin_of: SharedMap<ClassId, ClassId>,
    entering_shadowed: bool,
    pub classes_completed: usize,
    pub signatures_decoded: usize,
    pub blocked_met: Vec<(String, u32)>,
    /// The pseudo files of the library bodies converted so far, after the program's files.
    pub body_sources: SlabVec<crate::source::SourceFile>,
    /// Per library body's pseudo file, the file of the jar its definition was read from.
    pub body_jars: SlabVec<FileId>,
    /// What the JavaScript backend compiled from the jars' bodies.
    pub bodies: compile::BodyStats,
    pub decoded_classes: FxMap<(u32, crate::tasty::tree::Addr), Arc<crate::tasty::terms::ClassDef>>,
    /// The declarations library bodies' selections name, as the typer resolved them
    /// (`declared.rs`): symbols of this shared region, the same for every worker.
    pub declared: SharedMap<crate::ast::DeclRef, declared::Declared>,
    /// Per loaded file asked for a place, its positions.
    pub positions: SharedMap<u32, Arc<places::FilePositions>>,
    /// The sources jars opened for the places they attach, by path; `None` for one that cannot
    /// be read. Read under the loader's lock.
    pub sources_jars: FxMap<String, Option<places::SourcesJar>>,
    /// The tokens the products' files record, composed with the build's own for the listing.
    pub tokens: provenance::Tokens,
    /// The library documents the language server was asked about, with the loaded files whose
    /// source each shows, how many files the loader had read then (`attach.rs`) and the
    /// occurrence index built over them (`occurrences.rs`).
    pub documents: FxMap<std::path::PathBuf, DocState>,
    /// Where the type parameters a document's index met are declared: their file and the
    /// address of their `TYPEPARAM`, which the attach path locates them by. Filled by the
    /// index alone.
    pub tparam_defs: FxMap<TParamId, (u32, Addr)>,
    /// Per jar of the class path, the TASTy entries by the source path their pickles record,
    /// built once per jar on the first document of the jar asked about (`attach.rs`), with the
    /// files and bytes read for it and its time in milliseconds.
    pub source_maps: FxMap<u16, Arc<FxMap<String, Vec<CpFile>>>>,
    pub source_map_stats: Vec<(u16, usize, usize, f64)>,
    /// The manifests of the product directories, by their entry of the class path, read once each:
    /// read outside the lock where a diagnostic is placed in a product's source (`product_source_of`).
    pub manifests: SharedMap<u16, Option<Arc<crate::products::Manifest>>>,
    /// Where the definitions converted from products stand in their sources,
    /// by pseudo file and definition: what the canonical order and the names made from
    /// positions take in the place of the pseudo file's. Read outside the lock (`anon.rs`).
    pub product_places: SharedMap<(FileId, crate::ast::DefId), ProductPlace>,
    /// The sources those places are in, each by its key and the token its producer recorded:
    /// read outside the lock by a place's index.
    pub product_sources: SlabVec<(String, u64)>,
    product_sources_by_token: FxMap<u64, Vec<u32>>,
    /// Per pseudo file of a product, its owning source.
    pub product_files: FxMap<FileId, u32>,
    /// The members of the products' `<file>$package` objects, which a converted target reads as
    /// the top-level definitions of their package, each with its object.
    /// Read outside the lock (`Worker::is_product_top_def`).
    pub product_package_members: SharedMap<SymId, ClassId>,
    /// Those objects, and whether their definitions were converted.
    pub product_packages: FxMap<ClassId, bool>,
    /// The top-level definitions of those objects' pseudo files checked so far.
    pub product_checked: FxMap<(FileId, crate::ast::DefId), ()>,
    /// The pseudo files of products whose expansions the typing replays (`ReaderTables::replay`):
    /// read outside the lock by every typing of an expression.
    pub replay_files: SharedMap<FileId, ()>,
    /// The methods those expansions expand, each with its source among `product_sources`:
    /// what puts an outlined function in its module.
    pub product_callee_sources: FxMap<SymId, u32>,
    /// The tree of a product's pickle each definition of a converted body stands for (its
    /// TASTy file and address), whose place a pickle holding an expansion of it writes.
    pub product_def_trees: FxMap<(FileId, crate::ast::DefId), (u32, Addr)>,
    /// The members of products' classes that scalac adds and teq's model of the class has not:
    /// the default getters and the vars' setters, which the release renaming leaves out.
    pub product_synthetics: FxMap<SymId, ()>,
    /// The vals holding the mirrors of products' classes the program derived, each with its
    /// class: the val stands in the class's file once the class is converted.
    pub product_mirror_vals: Vec<(SymId, ClassId)>,
    /// The classes of products that no conversion places, each with its place in its source,
    /// which orders it as the whole program's class: a SAM conversion's
    /// class (the lambda's place) and an enum's value case (its definition's point).
    pub product_class_places: FxMap<ClassId, (u32, u32)>,
}

/// A library document as the loader knows it: the loaded files whose source it shows, how many
/// files the loader had read when they were found (more read may add to them), and the
/// occurrence index built over those files.
#[derive(Default)]
pub struct DocState {
    pub read: usize,
    pub files: Vec<u32>,
    pub index: Option<Arc<occurrences::DocIndex>>,
}

/// A definition converted from a product, placed in its source: the source
/// (`Loaded::product_sources`), the byte offset of its start (an anonymous class's `new`), and
/// for an anonymous class the name its producer gave it.
#[derive(Clone, Debug)]
pub struct ProductPlace {
    pub source: u32,
    pub offset: u32,
    pub name: Option<String>,
}

/// `Loaded` as the workers share it: one instance, changed by the loader's lock holder through
/// `Worker::loaded_mut` while the others read the structures that take no lock.
pub struct LoadedCell(pub std::cell::UnsafeCell<Loaded>);

unsafe impl Sync for LoadedCell {}
unsafe impl Send for LoadedCell {}

impl std::ops::Deref for LoadedCell {
    type Target = Loaded;
    #[inline]
    fn deref(&self) -> &Loaded {
        unsafe { &*self.0.get() }
    }
}

impl LoadedCell {
    /// The tables for a change once the typer is done and nothing else reads them.
    pub fn get_mut(&self) -> &mut Loaded {
        unsafe { &mut *self.0.get() }
    }
}

/// What the loader knows of one package of the jars. The two name indexes are the classpath's
/// alone, built whole under the loader's lock on the first lookup that needs each and never
/// changed after, so a lookup outside the lock reads the one it finds; what the package's
/// entries hold is told by the completion marks (`Loaded::files_done`, `objects_done`).
struct PkgState {
    /// The TASTy files of the package by their encoded stem.
    names: std::sync::OnceLock<FxMap<Box<str>, CpFile>>,
    /// The Java class files of the package by their stem, Scala's left out.
    java_names: std::sync::OnceLock<FxMap<Box<str>, CpFile>>,
    /// `package.tasty` and `<file>$package.tasty`, whose members belong to the package.
    package_objects: Vec<CpFile>,
    /// The lock holder's marker, set before the objects are entered.
    objects_entered: bool,
    /// The objects entered and published (`Worker::loader_done`).
    objects_done: std::sync::atomic::AtomicBool,
}

/// A TASTy file the loader opened, as every worker reads it: what is fixed when it is opened,
/// pushed whole into `Loaded::files` and never changed.
pub struct LoadedFile {
    pub tasty: Arc<TastyFile>,
    pub cp: CpFile,
    pub file_id: FileId,
    /// teq's `TeqOrigins` section as read: what a product's generated names
    /// are made of.
    pub provenance: crate::tasty::origins::Found,
}

/// What the loader entered of a loaded file, by the address of each definition: the loader's
/// lock holder's, read and written under the lock, or with one worker (`Worker::file_tables`).
#[derive(Default)]
pub struct FileTables {
    /// The classes of the file by the address of their definition.
    pub classes: FxMap<Addr, ClassId>,
    /// Type parameters by the address of their `TYPEPARAM`: those of classes from when the
    /// class is entered, those of methods from when the signature is decoded.
    pub tparams: FxMap<Addr, TParamId>,
    pub terms: FxMap<Addr, SymId>,
    pub aliases: FxMap<Addr, AliasId>,
    pub packages: FxMap<u32, PkgId>,
}

#[derive(Clone, Copy)]
pub struct LClass {
    pub file: u32,
    pub addr: Addr,
    /// The value case of an enum stands for a class of its own, which has the val's type as
    /// its parent.
    pub value_case: bool,
}

#[derive(Clone, Copy)]
pub struct LSym {
    pub file: u32,
    pub addr: Addr,
    /// An `implicit def` with a leading explicit parameter: a conversion, which provides no
    /// instance on its own.
    pub conversion: bool,
}

#[derive(Clone, Copy)]
pub struct LAlias {
    pub file: u32,
    pub addr: Addr,
}

const NONE: u32 = u32::MAX;

impl Loaded {
    pub fn new(cp: Classpath, jar_files: Vec<FileId>, jdk_file: FileId, release: Option<u32>) -> Loaded {
        let pkg_states = cp.packages.iter().map(|_| PkgState { names: Default::default(), java_names: Default::default(), package_objects: Vec::new(), objects_entered: false, objects_done: Default::default() }).collect();
        let jar_first = jar_files.iter().chain(std::iter::once(&jdk_file)).map(|f| f.0).min().unwrap_or(u32::MAX);
        Loaded {
            cp,
            jar_files,
            qualified_private_aliases: SharedMap::new(),
            access_within: SharedMap::new(),
            bridged: FxMap::default(),
            target_names: SharedMap::new(),
            product_target_names: SharedMap::new(),
            volatile: SharedMap::new(),
            jar_first,
            pkg_slots: Vec::new(),
            pkg_states,
            files: SlabVec::with_capacity(256),
            tables: Vec::new(),
            file_index: FxMap::default(),
            files_done: SharedMap::new(),
            classes: SharedMap::new(),
            syms: SharedMap::new(),
            aliases: SharedMap::new(),
            predef: None,
            scala_library: false,
            detail: std::env::var_os("TEQ_CLASSPATH_DETAIL").is_some(),
            java: Java::new(jdk_file, release),
            shadowed: FxMap::default(),
            builtin_of: SharedMap::new(),
            entering_shadowed: false,
            classes_completed: 0,
            signatures_decoded: 0,
            blocked_met: Vec::new(),
            body_sources: SlabVec::with_capacity(64),
            body_jars: SlabVec::with_capacity(64),
            bodies: compile::BodyStats::default(),
            decoded_classes: FxMap::default(),
            declared: SharedMap::new(),
            positions: SharedMap::new(),
            sources_jars: FxMap::default(),
            tokens: Default::default(),
            documents: FxMap::default(),
            tparam_defs: FxMap::default(),
            source_maps: FxMap::default(),
            source_map_stats: Vec::new(),
            manifests: SharedMap::new(),
            product_places: SharedMap::new(),
            product_sources: SlabVec::with_capacity(16),
            product_sources_by_token: FxMap::default(),
            product_files: FxMap::default(),
            product_package_members: SharedMap::new(),
            product_packages: FxMap::default(),
            product_checked: FxMap::default(),
            replay_files: SharedMap::new(),
            product_callee_sources: FxMap::default(),
            product_def_trees: FxMap::default(),
            product_synthetics: FxMap::default(),
            product_mirror_vals: Vec::new(),
            product_class_places: FxMap::default(),
        }
    }

    /// A library body's pseudo file counts as a jar file: its names see the classpath. Its text
    /// is one line per member, which diagnostics point into.
    pub fn add_body_file(&mut self, id: FileId, jar: FileId, path: String, key: String, text: String) {
        debug_assert!(id.0 >= self.jar_first);
        self.body_sources.push(crate::source::SourceFile { path, text, is_std: true, key });
        self.body_jars.push(jar);
    }

    /// The TASTy files of the package at `slot` by their encoded stem, built on the first
    /// lookup, by the loader's lock holder once the arenas are forked.
    fn pkg_names(&self, slot: usize) -> &FxMap<Box<str>, CpFile> {
        self.pkg_states[slot].names.get_or_init(|| {
            let mut names = FxMap::default();
            for &f in &self.cp.packages[slot].files {
                let stem = self.cp.stem(f);
                if stem != "package" && !stem.ends_with("$package") {
                    names.entry(stem.into()).or_insert(f);
                }
            }
            names
        })
    }

    /// The Java class files of the package at `slot` by their stem, once built.
    pub(super) fn java_names(&self, slot: usize) -> Option<&FxMap<Box<str>, CpFile>> {
        self.pkg_states[slot].java_names.get()
    }

    /// Marks what a hold finished as published, once the outermost hold has published it
    /// (`Worker::loader_done`).
    pub fn mark_done(&self, done: super::LoaderDone) {
        match done {
            super::LoaderDone::File(f) => {
                self.files_done.insert(f, ());
            }
            super::LoaderDone::Objects(slot) => self.pkg_states[slot].objects_done.store(true, std::sync::atomic::Ordering::Release),
            super::LoaderDone::JavaClass(c) => {
                self.java.absorbed_done.insert(c, ());
            }
            super::LoaderDone::JavaCtors(c) => {
                self.java.ctors_done.insert(c, ());
            }
            super::LoaderDone::JavaBuiltin(slot) => self.java.builtin_done[slot].store(true, std::sync::atomic::Ordering::Release),
        }
    }

    #[inline]
    pub fn slot(&self, p: PkgId) -> Option<usize> {
        match self.pkg_slots.get(p.idx()) {
            Some(&s) if s != NONE => Some(s as usize),
            _ => None,
        }
    }

    /// Whether a class was read from a directory of teq's products: another module's, whose
    /// class files are there already.
    pub fn is_product_class(&self, c: ClassId) -> bool {
        self.classes.get(&c).map_or(false, |lc| self.cp.is_products(self.file(lc.file).cp))
    }

    /// Whether a class was read from a Scala 2 library, which scalac marks `Scala2x`:
    /// scala-library's TASTy, whose attribute says so, or a Scala 2 jar's pickles.
    pub fn is_scala2_class(&self, c: ClassId) -> bool {
        self.classes.get(&c).map_or(false, |lc| {
            let f = self.file(lc.file);
            self.cp.is_scala2(f.cp) || f.tasty.has_attribute(crate::tasty::tags::attr::SCALA2_STANDARD_LIBRARY)
        })
    }

    /// Whether another module's class is one scalac made (an enum's or a case class's
    /// companion): teq's model of the program has no body for it, often no class at all.
    pub fn is_synthetic_product_class(&self, c: ClassId) -> bool {
        let Some(lc) = self.classes.get(&c) else { return false };
        if !self.cp.is_products(self.file(lc.file).cp) {
            return false;
        }
        let tasty = &self.file(lc.file).tasty;
        Decoder::new(tasty).class_sig(lc.addr).mods.flags.has(tags::SYNTHETIC)
    }

    /// Whether a member read from TASTy has a body: an `abstract override` is marked abstract
    /// and has one.
    pub fn has_body(&self, s: SymId) -> bool {
        let Some(ls) = self.syms.get(&s) else { return false };
        Decoder::new(&self.file(ls.file).tasty).def_sig(ls.addr).body.is_some()
    }

    pub fn is_loaded_class(&self, c: ClassId) -> bool {
        self.classes.contains_key(&c)
    }

    pub fn is_conversion(&self, s: SymId) -> bool {
        self.syms.get(&s).map_or(false, |l| l.conversion)
    }

    /// The loaded file `i`, read from any thread: its index reached the reader through what the
    /// lock holder published after the push.
    #[inline]
    pub fn file(&self, i: u32) -> &LoadedFile {
        self.files.get(i as usize)
    }

    /// Whether `f` stands for a jar: what is defined there is complete and checked already,
    /// and nothing of it is emitted.
    #[inline]
    pub fn is_jar(&self, f: FileId) -> bool {
        f.0 >= self.jar_first
    }

    pub fn note_blocked(&mut self, description: &str) {
        match self.blocked_met.iter_mut().find(|(d, _)| d == description) {
            Some((_, n)) => *n += 1,
            None => self.blocked_met.push((description.to_string(), 1)),
        }
    }

    /// What a check read from the jars, for `--time`.
    pub fn report(&self, report: &mut crate::report::Report, placeholder_name: impl Fn(ClassId) -> String) {
        let detail = std::env::var_os("TEQ_CLASSPATH_DETAIL").is_some();
        let s = &self.cp.stats;
        if s.jars_opened > 0 || self.classes_completed > 0 || !self.blocked_met.is_empty() {
            self.report_classpath(report.section("classpath"), detail);
        }
        if self.java.read_anything() {
            let section = report.section("Java");
            self.java.report(section);
            if detail {
                let mut named: Vec<String> = self.java.unread().map(placeholder_name).collect();
                named.sort();
                for n in named {
                    section.detail(format!("named but not read: {}", n));
                }
            }
        }
        self.report_bodies(report, detail);
    }

    fn report_classpath(&self, section: &mut crate::report::Section, detail: bool) {
        use crate::report::{bytes, grouped};
        let s = &self.cp.stats;
        section.row("jars opened").count(s.jars_opened).time(s.open_time);
        let cached = if s.files_cached > 0 { format!(", {} from the jar cache", grouped(s.files_cached as u64)) } else { String::new() };
        section.row("files read").count(s.files_inflated).time(s.inflate_time).note(format!("{}{}", bytes(s.bytes_inflated as u64), cached));
        if s.caches_written > 0 {
            section.row("jar caches written").count(s.caches_written).time(s.cache_write_time).note(bytes(s.cache_bytes_written as u64));
        }
        section.row("classes completed").count(self.classes_completed);
        section.row("signatures decoded").count(self.signatures_decoded);
        if s.scan_time > std::time::Duration::ZERO {
            section.row("files read for reflection").count(s.files_scanned).time(s.scan_time);
        }
        if s.class_files_parsed > 0 {
            section.row("class files parsed").count(s.class_files_parsed).time(s.class_parse_time).note(bytes(s.class_bytes as u64));
        }
        if !self.blocked_met.is_empty() {
            let total: u32 = self.blocked_met.iter().map(|(_, n)| n).sum();
            section.row("blocked shapes met").count(total as usize).note(format!("{} distinct", self.blocked_met.len()));
        }
        if detail {
            for f in self.files.as_slice() {
                section.detail(format!("{} ({} bytes)", self.cp.entry_name(f.cp), f.tasty.bytes.len()));
            }
            let mut blocked = self.blocked_met.clone();
            blocked.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
            for (d, n) in blocked {
                section.detail(format!("{:>4}  {}", n, d));
            }
        }
    }

    fn report_bodies(&self, report: &mut crate::report::Report, detail: bool) {
        if self.bodies.classes_converted > 0 || !self.bodies.refused.is_empty() {
            let section = report.section("library bodies");
            section.row("classes converted").count(self.bodies.classes_converted);
            section.row("methods typed").count(self.bodies.members_typed).time(self.bodies.time);
            section.row("refused").count(self.bodies.refused.len());
            section.row("members missing").count(self.bodies.misses.len());
            if detail {
                for (what, why) in &self.bodies.refused {
                    section.detail(format!("refused {}: {}", what, why));
                }
                for what in &self.bodies.name_clashes {
                    section.detail(format!("name clash {}", what));
                }
                let mut misses = self.bodies.misses.clone();
                misses.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
                for (d, n) in misses {
                    section.detail(format!("{:>4}  {}", n, d));
                }
            }
        }
        if !self.bodies.shapes.is_empty() {
            let (lacking, other): (usize, usize) = self.bodies.shapes.iter().fold((0, 0), |(l, o), g| (l + g.lacking.len(), o + g.other_shape.len()));
            let section = report.section("shape census");
            section.row("std classes standing for jar classes").count(self.bodies.shapes.len());
            section.row("members lacking").count(lacking);
            section.row("members with another shape").count(other);
            for g in &self.bodies.shapes {
                section.detail(format!("{} <- {}: {} members, {} lacking, {} with another shape", g.std, g.jar, g.members, g.lacking.len(), g.other_shape.len()));
                if !g.lacking.is_empty() {
                    section.detail(format!("  lacking: {}", g.lacking.join(", ")));
                }
                if !g.other_shape.is_empty() {
                    section.detail(format!("  other shape: {}", g.other_shape.join(", ")));
                }
            }
        }
    }
}

/// A synthetic val `$N` of a class: what holds the value of a tuple pattern in the body.
fn is_pattern_field(tasty: &TastyFile, e: &Entry) -> bool {
    e.tag == tags::VALDEF && is_pattern_field_name(Some(&tasty.name(e.name)))
}

pub(super) fn is_pattern_field_name(name: Option<&str>) -> bool {
    name.map_or(false, |n| n.len() > 1 && n.starts_with('$') && n[1..].bytes().all(|b| b.is_ascii_digit()))
}

pub(super) fn flag_mods(f: Flags) -> Mods {
    let mut m = 0;
    for (tag, bit) in [
        (tags::PRIVATE, mods::PRIVATE),
        (tags::PROTECTED, mods::PROTECTED),
        (tags::SEALED, mods::SEALED),
        (tags::ABSTRACT, mods::ABSTRACT),
        (tags::FINAL, mods::FINAL),
        (tags::CASE, mods::CASE),
        (tags::LAZY, mods::LAZY),
        (tags::OVERRIDE, mods::OVERRIDE),
        (tags::OPAQUE, mods::OPAQUE),
        (tags::INLINE, mods::INLINE),
        (tags::TRANSPARENT, mods::TRANSPARENT),
        (tags::OPEN, mods::OPEN),
        (tags::INFIX, mods::INFIX),
        (tags::MUTABLE, mods::MUTABLE),
        (tags::IMPLICIT, mods::IMPLICIT),
        (tags::GIVEN, mods::GIVEN),
        (tags::ENUM, mods::ENUM),
        (tags::EXTENSION, mods::EXTENSION),
    ] {
        if f.has(tag) {
            m |= bit;
        }
    }
    m
}

pub(super) fn is_using_clause(ps: &[crate::tasty::tree::Param]) -> bool {
    ps.first().map_or(false, |p| p.flags.has(tags::GIVEN) || p.flags.has(tags::IMPLICIT))
}

/// A Scala 2 `(implicit ...)` clause, whose arguments may also be passed without `using`.
fn is_implicit_clause(ps: &[crate::tasty::tree::Param]) -> bool {
    ps.first().map_or(false, |p| p.flags.has(tags::IMPLICIT) && !p.flags.has(tags::GIVEN))
}

/// The class an object entry is the companion of, if its outer declares one.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Companion {
    OfClass,
    OfEnum,
}

impl<'a> Worker<'a> {
    /// Sets the classpath up: every package of the jars gets its id, so that imports and
    /// qualified names resolve before anything is read.
    pub fn set_classpath(&mut self, cp: Classpath, jar_files: Vec<FileId>, jdk_file: FileId, release: Option<u32>, std: crate::StdMode) {
        let mut loaded = Loaded::new(cp, jar_files, jdk_file, release);
        let scala_library = std == crate::StdMode::ScalaLibrary;
        loaded.scala_library = scala_library;
        let n = loaded.cp.packages.len();
        for i in 0..n {
            let path = loaded.cp.packages[i].path.clone();
            let mut p = ROOT_PKG;
            for seg in path.split('/').filter(|s| !s.is_empty()) {
                let name = self.interner.intern(&package_segment(seg));
                p = self.syms.sub_pkg(p, name);
            }
            if loaded.pkg_slots.len() <= p.idx() {
                loaded.pkg_slots.resize(p.idx() + 1, NONE);
            }
            loaded.pkg_slots[p.idx()] = i as u32;
            let mut objects: Vec<CpFile> = Vec::new();
            for &f in &loaded.cp.packages[i].files {
                let stem = loaded.cp.stem(f);
                if (stem == "package" || stem.ends_with("$package")) && !objects.iter().any(|&o| loaded.cp.stem(o) == stem) {
                    objects.push(f);
                }
            }
            if objects.is_empty() {
                objects.extend(loaded.cp.packages[i].scala2_objects.first().copied());
            }
            loaded.pkg_states[i].package_objects = objects;
        }
        self.loaded = Some(std::sync::Arc::new(LoadedCell(std::cell::UnsafeCell::new(loaded))));
        let predef = self.interner.intern("Predef");
        let scala = self.b.scala_pkg;
        if let Some(super::resolve::TermRef::Global(s)) = self.pkg_term(scala, predef) {
            if let SymKind::Object(c) = self.syms.sym(s).kind {
                self.loaded_mut().predef = Some(c);
            }
        }
    }

    // ---- packages ----

    /// Every top-level class and object the class path holds, by package (dotted), name and
    /// whether it is an object, the JDK's once it is open: completion's catalogue.
    pub(crate) fn class_path_names(&self) -> Vec<(String, String, bool)> {
        let Some(loaded) = self.loaded.as_ref() else { return Vec::new() };
        let mut out = Vec::new();
        for pkg in &loaded.cp.packages {
            if pkg.path.is_empty() {
                continue;
            }
            let owner = pkg.path.replace('/', ".");
            for &f in pkg.files.iter().chain(&pkg.classes) {
                let stem = loaded.cp.stem(f);
                let object = stem.ends_with('$');
                let stem = stem.strip_suffix('$').unwrap_or(stem);
                if stem.is_empty() || stem.contains('$') || stem == "package" || stem == "module-info" || stem == "package-info" {
                    continue;
                }
                out.push((owner.clone(), stem.to_string(), object));
            }
        }
        out.extend(self.jdk_names().into_iter().map(|(owner, name)| (owner, name, false)));
        out
    }

    /// The top-level classes and objects the class path holds in package `p`, by the names of
    /// its jars' and directories' entries, the JDK's once it is open: what a completion offers of
    /// a package before a lookup enters it (`typer::complete`). Nested classes, package objects
    /// and names the compiler encoded are left out.
    pub(crate) fn catalogue_names(&self, p: PkgId) -> Vec<String> {
        let Some(loaded) = self.loaded.as_ref() else { return Vec::new() };
        let mut out: Vec<String> = Vec::new();
        if let Some(slot) = loaded.slot(p) {
            let pkg = &loaded.cp.packages[slot];
            for &f in pkg.files.iter().chain(&pkg.classes) {
                let stem = loaded.cp.stem(f);
                let stem = stem.strip_suffix('$').unwrap_or(stem);
                if stem.is_empty() || stem.contains('$') || stem == "package" || stem == "module-info" || stem == "package-info" {
                    continue;
                }
                out.push(stem.to_string());
            }
        }
        out.extend(self.jdk_class_names(p));
        out.sort();
        out.dedup();
        out
    }

    /// Enters what the jars hold under `name` in package `p`, if anything, so that the
    /// package's entries answer the lookup that missed.
    #[cold]
    #[inline(never)]
    pub(super) fn load_pkg_member(&mut self, p: PkgId, name: Name) -> bool {
        if self.forked && self.pkg_member_settled(p, name) {
            crate::measure::looked_up_unlocked(crate::measure::Lookup::JarMember);
            return false;
        }
        self.with_loader_for(crate::measure::Hold::JarLookup, |w| {
            let entered = w.load_pkg_member_unlocked(p, name);
            crate::measure::looked_up(crate::measure::Lookup::JarMember, entered);
            entered
        })
    }

    /// Whether `load_pkg_member` would enter nothing for `name` in `p`, read outside the
    /// loader's lock: the package's name indexes are built and the
    /// class file of the name, if any, and the package's objects are entered and published, so
    /// that the package's entries a caller reads again are the loader's; a Scala 2 or a Java
    /// class of the name in the jars is entered, or there is none, and the package is no JDK
    /// one, whose index is the lock holder's. Any other answer is the lock's to give, with the
    /// same checks made again under it.
    fn pkg_member_settled(&mut self, p: PkgId, name: Name) -> bool {
        if crate::shared::lock_depth() > 0 {
            return false;
        }
        let Some(loaded) = self.loaded.as_ref() else { return true };
        let slot = loaded.slot(p);
        if slot.is_some_and(|slot| !self.jar_member_settled(slot, name)) {
            return false;
        }
        self.java_pkg_member_settled(p, name, slot)
    }

    /// The jars' part of `pkg_member_settled`, for the package at `slot`.
    fn jar_member_settled(&self, slot: usize, name: Name) -> bool {
        let loaded = self.loaded.as_ref().unwrap();
        let state = &loaded.pkg_states[slot];
        let encoded = encode_name(self.interner.get(name));
        let Some(names) = state.names.get() else { return false };
        if names.get(&*encoded).is_some_and(|f| !loaded.files_done.contains_key(f)) {
            return false;
        }
        if !state.objects_done.load(std::sync::atomic::Ordering::Acquire) {
            return false;
        }
        let std_file = self.source(self.env.file).is_std && !loaded.is_jar(self.env.file);
        if self.jvm && loaded.cp.holds_scala2() && !std_file {
            let Some(java) = state.java_names.get() else { return false };
            if java.get(&*encoded).is_some_and(|&f| loaded.cp.is_scala2(f) && !loaded.files_done.contains_key(&f)) {
                return false;
            }
        }
        true
    }

    pub(super) fn load_pkg_member_unlocked(&mut self, p: PkgId, name: Name) -> bool {
        if let Some(slot) = self.loaded.as_ref().and_then(|l| l.slot(p)) {
            if let Some(f) = self.class_file_named(slot, name) {
                if !self.file_entered(f) {
                    if super::prep::capturing() {
                        self.prep_note_pkg_member(p, name);
                    }
                    self.enter_file(f, p);
                    // A package object's alias may share the name of a class file's object
                    // (`zio.Trace`); both are the package's members from then on.
                    self.enter_pkg_objects(p);
                    return true;
                }
            }
        }
        let objects = self.enter_pkg_objects(p);
        let defined = self.syms.pkg(p).entries.get(&name).map_or(false, |e| e.term.is_some() || e.class.is_some() || e.alias.is_some() || e.pkg.is_some());
        if objects && defined {
            return true;
        }
        if self.loaded.is_some() && (self.load_scala2_pkg_member(p, name) || self.load_java_pkg_member(p, name)) {
            return true;
        }
        objects
    }

    /// Enters the class file of a Scala 2 jar named `name` in package `p` when it holds a
    /// pickle: its top-level class and object.
    fn load_scala2_pkg_member(&mut self, p: PkgId, name: Name) -> bool {
        let loaded = self.loaded.as_ref().unwrap();
        if !self.jvm || !loaded.cp.holds_scala2() || (self.source(self.env.file).is_std && !loaded.is_jar(self.env.file)) {
            return false;
        }
        let Some(slot) = loaded.slot(p) else { return false };
        let encoded = encode_name(self.interner.get(name)).into_owned();
        let Some(f) = self.java_class_file_named(slot, &encoded) else { return false };
        if !self.loaded.as_ref().unwrap().cp.is_scala2(f) || self.file_entered(f) {
            return false;
        }
        let pickled = self.loaded_mut().cp.class_file(f).map_or(false, |cf| cf.scala_sig.is_some());
        if !pickled {
            return false;
        }
        self.enter_file(f, p);
        self.enter_pkg_objects(p);
        true
    }

    /// The Java class file `stem` of the package, when the jars hold one that no TASTy file
    /// accounts for.
    pub(super) fn java_class_file_named(&mut self, slot: usize, stem: &str) -> Option<CpFile> {
        self.with_loader_for(crate::measure::Hold::JarEntry, |w| {
            let found = w.java_class_file_named_unlocked(slot, stem);
            crate::measure::looked_up(crate::measure::Lookup::JavaClassFile, found.is_some());
            found
        })
    }

    pub(super) fn java_class_file_named_unlocked(&mut self, slot: usize, stem: &str) -> Option<CpFile> {
        let loaded = self.loaded.as_ref().unwrap();
        let tasty = loaded.pkg_names(slot);
        let java = loaded.pkg_states[slot].java_names.get_or_init(|| {
            let mut names = FxMap::default();
            for &f in &loaded.cp.packages[slot].classes {
                if !loaded.cp.is_scala(tasty, f) {
                    names.entry(loaded.cp.stem(f).into()).or_insert(f);
                }
            }
            names
        });
        java.get(stem).copied()
    }

    /// Enters the package objects of `p` from the jars, once; whether this was the first time.
    pub(super) fn enter_pkg_objects(&mut self, p: PkgId) -> bool {
        self.with_loader_for(crate::measure::Hold::JarEntry, |w| {
            let entered = w.enter_pkg_objects_unlocked(p);
            crate::measure::looked_up(crate::measure::Lookup::PackageObjects, entered);
            entered
        })
    }

    pub(super) fn enter_pkg_objects_unlocked(&mut self, p: PkgId) -> bool {
        let Some(slot) = self.loaded.as_ref().and_then(|l| l.slot(p)) else { return false };
        let loaded = self.loaded_mut();
        if loaded.pkg_states[slot].objects_entered {
            return false;
        }
        loaded.pkg_states[slot].objects_entered = true;
        let objects = loaded.pkg_states[slot].package_objects.clone();
        for f in objects {
            if self.jvm || !self.loaded.as_ref().unwrap().cp.is_scala2(f) {
                self.enter_package_object(f, p);
            }
        }
        self.loader_done(super::LoaderDone::Objects(slot));
        true
    }

    fn class_file_named(&mut self, slot: usize, name: Name) -> Option<CpFile> {
        let encoded = encode_name(self.interner.get(name));
        self.loaded.as_ref().unwrap().pkg_names(slot).get(&*encoded).copied()
    }

    fn file_entered(&self, f: CpFile) -> bool {
        self.loaded.as_ref().unwrap().file_index.contains_key(&f)
    }

    pub(super) fn open_file(&mut self, f: CpFile) -> Option<u32> {
        if let Some(&i) = self.loaded.as_ref().unwrap().file_index.get(&f) {
            return Some(i);
        }
        let p = self.phase(super::profile::Phase::TastyRead);
        let read = self.loaded_mut().cp.file(f);
        self.phase_end(p);
        let file_id = self.loaded.as_ref().unwrap().jar_files[f.jar as usize];
        let tasty = match read {
            Ok(t) => t,
            Err(e) => {
                self.diags.error(file_id, Span::default(), e);
                return None;
            }
        };
        // The record is whole before it is pushed: a worker outside the lock may read it as soon
        // as an index of it reaches that worker.
        let provenance = self.read_provenance(&tasty, f, file_id);
        let loaded = self.loaded_mut();
        let i = loaded.files.len() as u32;
        loaded.files.push(LoadedFile { tasty, cp: f, file_id, provenance });
        loaded.tables.push(FileTables::default());
        loaded.file_index.insert(f, i);
        Some(i)
    }

    /// Enters the top-level definitions of a class file into its package.
    fn enter_file(&mut self, f: CpFile, pkg: PkgId) -> Option<u32> {
        let file = self.open_file(f)?;
        let tasty = self.tasty(file);
        let tops = crate::tasty::tree::index_top_level(&tasty);
        let entries: Vec<Entry> = tops.into_iter().map(|t| t.entry).collect();
        self.enter_entries(file, &entries, Owner::Package(pkg));
        self.loader_done(super::LoaderDone::File(f));
        Some(file)
    }

    /// Enters a package object and spills its members into the package: what stands in
    /// `object package` or `object file$package` is what the package defines.
    fn enter_package_object(&mut self, f: CpFile, pkg: PkgId) {
        let Some(file) = self.enter_file(f, pkg) else { return };
        let stem = self.loaded.as_ref().unwrap().cp.stem(f).to_string();
        let object_name = self.interner.intern(&stem);
        let Some(obj) = self.file_tables(file).classes.values().copied().find(|&c| {
            let info = self.syms.class(c);
            info.kind == ClassKind::Object && info.name == object_name && info.owner == Owner::Package(pkg)
        }) else {
            return;
        };
        self.complete_class(obj);
        if stem == "package" {
            self.syms.pkgs[pkg.idx()].package_object = Some(obj);
        }
        // Another module's top-level export clauses: their forwarders are the object's table,
        // which the package's holds.
        let exporting = self.syms.class(obj).has_exports && self.loaded.as_ref().unwrap().cp.is_products(f);
        if exporting && !self.syms.pkg(pkg).export_objects.contains(&obj) {
            self.syms.pkgs[pkg.idx()].export_objects.push(obj);
        }
        let info = self.syms.class(obj);
        let members: Vec<SymId> = info.member_order.clone();
        let nested: Vec<(Name, ClassId)> = info.nested.iter().map(|(&n, &c)| (n, c)).collect();
        let aliases: Vec<(Name, AliasId)> = info.type_aliases.iter().map(|(&n, &a)| (n, a)).collect();
        let extensions = info.extensions.clone();
        let givens = info.givens.clone();
        // What the std defines under a name the object spills is the binding: its file enters
        // first, so that the package's entry is the std's.
        for &(n, _) in &nested {
            self.demand_std(pkg, n, crate::stdindex::TYPE);
        }
        for &(n, _) in &aliases {
            self.demand_std(pkg, n, crate::stdindex::TYPE);
        }
        for &s in &members {
            let n = self.syms.sym(s).name;
            self.demand_std(pkg, n, crate::stdindex::TERM | crate::stdindex::TYPE);
        }
        let entries = &mut self.syms.pkgs[pkg.idx()].entries;
        for (n, c) in nested {
            let e = entries.entry(n).or_default();
            if e.class.is_none() {
                e.class = Some(c);
            }
        }
        for (n, a) in aliases {
            let e = entries.entry(n).or_default();
            if e.class.is_none() && e.alias.is_none() {
                e.alias = Some(a);
            }
        }
        for s in members {
            let name = self.syms.sym(s).name;
            let entry = self.syms.class(obj).members[&name];
            // `val Right = scala.util.Right` of `scala.package` names the companion of a class
            // the std defines itself, whose constructor the name means there.
            let std_class = self.syms.pkg(pkg).entries.get(&name).and_then(|e| e.class).map_or(false, |c| !self.in_jar(self.syms.class(c).file));
            let e = self.syms.pkgs[pkg.idx()].entries.entry(name).or_default();
            if e.term.is_none() && !std_class {
                e.term = Some(entry);
            }
        }
        for s in &extensions {
            let name = self.syms.sym(*s).name;
            self.syms.pkgs[pkg.idx()].entries.entry(name).or_default().extensions.push(*s);
        }
        self.syms.pkgs[pkg.idx()].givens.extend(givens);
        // Where a product's bodies are converted, its file's top-level definitions are the
        // package's, as the whole program has them: called by their paths, typed and emitted as
        // the top level of a pseudo file of their own (`Worker::check_product_package`).
        let product = self.loaded.as_ref().unwrap().cp.is_products(f);
        if product && !self.jvm && !self.writes_products && stem.ends_with("$package") {
            let mut defs: Vec<SymId> = Vec::new();
            for &m in self.syms.class(obj).members.values() {
                match self.syms.alternatives(m) {
                    Some(alts) => defs.extend(alts.iter().copied()),
                    None => defs.push(m),
                }
            }
            defs.extend(extensions);
            defs.extend(self.syms.class(obj).givens.iter().copied());
            let mut classes: Vec<ClassId> = Vec::new();
            for s in defs {
                if self.syms.sym(s).owner == Owner::Class(obj) {
                    self.syms.sym_mut(s).owner = Owner::Package(pkg);
                    self.loaded_mut().product_package_members.insert(s, obj);
                    classes.extend(self.syms.sym(s).impl_class);
                }
            }
            // The classes the object holds, a given's class among them, are the package's too.
            classes.extend(self.syms.class(obj).nested.values().copied());
            classes.retain(|&k| self.syms.class(k).owner == Owner::Class(obj));
            classes.sort();
            classes.dedup();
            for k in classes {
                self.syms.class_mut(k).owner = Owner::Package(pkg);
            }
            self.loaded_mut().product_packages.insert(obj, false);
        }
    }

    /// Gives a builtin class of package `scala` (a tuple or function class) the members and
    /// base types of the library's class of that name: the builtin stays the type, and meets
    /// what the library declares for it.
    pub(super) fn link_library_class(&mut self, c: ClassId) {
        let (name, pkg) = match self.syms.class(c).owner {
            Owner::Package(p) => (self.syms.class(c).name, p),
            _ => return,
        };
        let Some(slot) = self.loaded.as_ref().and_then(|l| l.slot(pkg)) else { return };
        let Some(f) = self.class_file_named(slot, name) else { return };
        if self.file_entered(f) {
            return;
        }
        self.loaded_mut().entering_shadowed = true;
        self.enter_file(f, pkg);
        self.loaded_mut().entering_shadowed = false;
        let Some(lib) = self.loaded.as_ref().unwrap().shadowed.get(&(pkg, name)).copied() else { return };
        self.loaded_mut().builtin_of.insert(lib, c);
        self.complete_class(lib);
        // A member the std defines as an extension on the builtin (`andThen`, `swap`,
        // `length`) is the std's: the builtin's value has no such method.
        let scala = self.b.scala_pkg;
        let shadowed: Vec<Name> = self.syms.pkg(scala).entries.iter().filter(|(_, e)| !e.extensions.is_empty()).map(|(&n, _)| n).collect();
        for n in shadowed {
            self.syms.class_mut(lib).members.remove(&n);
        }
        let own: Vec<TParamId> = self.syms.class(c).tparams.clone();
        let theirs: Vec<TParamId> = self.syms.class(lib).tparams.clone();
        if own.len() != theirs.len() {
            return;
        }
        let subst: Subst = theirs.iter().zip(&own).map(|(&t, &o)| (t, self.types.param(o))).collect();
        let inherited: Vec<(ClassId, TypeId)> = self.syms.class(lib).base_types.clone();
        let mut bases = self.syms.class(c).base_types.clone();
        for (b, bt) in inherited {
            let t = self.types.subst(bt, &subst);
            if !bases.iter().any(|&(x, _)| x == b) {
                bases.push((b, t));
            }
        }
        self.syms.class_mut(c).base_types = bases;
        if self.syms.class(c).base_types.len() > 1 {
            self.mark_members_meeting_inherited(c);
        }
    }

    /// The key of the pseudo file a definition of a jar is converted into: its TASTy file and
    /// the address of the definition there (`zio-json.jar!zio/json/JsonDecoder.tasty@<address>`).
    pub(super) fn body_key(&self, file: u32, addr: Addr) -> String {
        let loaded = self.loaded.as_ref().unwrap();
        let f = loaded.file(file);
        format!("{}!{}@{}", self.source(f.file_id).key, loaded.cp.entry_name(f.cp), addr)
    }

    pub(super) fn tasty(&self, file: u32) -> Arc<TastyFile> {
        self.loaded.as_ref().unwrap().file(file).tasty.clone()
    }

    /// What the loader entered of the loaded file `file` (`FileTables`): the lock holder's, read
    /// under the loader's lock or with one worker.
    pub(super) fn file_tables(&self, file: u32) -> &FileTables {
        debug_assert!(!self.forked || crate::shared::lock_depth() > 0, "the tables of loaded file {} read outside the loader's lock", file);
        &self.loaded.as_ref().unwrap().tables[file as usize]
    }

    pub(super) fn lname(&mut self, tasty: &TastyFile, n: u32) -> Name {
        if let Some(&id) = tasty.interned.borrow().get(n as usize).filter(|&&id| id != u32::MAX) {
            return Name(id);
        }
        let name = self.interner.intern(&tasty.name(tasty.source_name(n)));
        if let Some(slot) = tasty.interned.borrow_mut().get_mut(n as usize) {
            *slot = name.0;
        }
        name
    }

    // ---- entering classes ----

    /// Enters classes, objects, aliases and opaque types among `entries` under `owner`; vals
    /// and defs are entered when their class completes. Module vals come after the classes so
    /// that their modifiers find the object.
    fn enter_entries(&mut self, file: u32, entries: &[Entry], owner: Owner) {
        let tasty = self.tasty(file);
        let class_names: Vec<(Name, bool)> = entries
            .iter()
            .filter(|e| e.tag == tags::TYPEDEF && e.is_class && !tasty.is_object_class(e.name))
            .map(|e| (self.lname(&tasty, e.name), e.flags.has(tags::ENUM)))
            .collect();
        // Another module's enum case classes have no companion in teq's model of them, as the
        // program's own have none: the case is made by its constructor.
        let product = self.loaded.as_ref().map_or(false, |l| l.cp.is_products(l.file(file).cp));
        let enum_cases: Vec<Name> = if product {
            entries
                .iter()
                .filter(|e| e.tag == tags::TYPEDEF && e.is_class && e.flags.has(tags::ENUM) && e.flags.has(tags::CASE))
                .map(|e| self.lname(&tasty, e.name))
                .collect()
        } else {
            Vec::new()
        };
        // A given with a body and no parameters is a given object in scalac's pickle; in teq's
        // model, which another module's classes are read as, a lazy given of a class of its own.
        let given_objects: Vec<Name> = if product {
            entries
                .iter()
                .filter(|e| e.tag == tags::VALDEF && e.flags.has(tags::OBJECT) && e.flags.has(tags::GIVEN))
                .map(|e| self.lname(&tasty, e.name))
                .collect()
        } else {
            Vec::new()
        };
        for e in entries {
            if e.tag == tags::TYPEDEF {
                if e.is_class && tasty.is_object_class(e.name) && e.flags.has(tags::SYNTHETIC) && !enum_cases.is_empty() {
                    let name = self.lname(&tasty, e.name);
                    if enum_cases.contains(&name) {
                        continue;
                    }
                }
                if e.is_class && tasty.is_object_class(e.name) && !given_objects.is_empty() {
                    let name = self.lname(&tasty, e.name);
                    if given_objects.contains(&name) {
                        self.enter_product_given_object(file, e, owner, name);
                        continue;
                    }
                }
                if e.is_class {
                    let companion = if tasty.is_object_class(e.name) {
                        let name = self.lname(&tasty, e.name);
                        class_names.iter().find(|(n, _)| *n == name).map(|&(_, is_enum)| if is_enum { Companion::OfEnum } else { Companion::OfClass })
                    } else {
                        None
                    };
                    self.enter_loaded_class(file, e, owner, companion);
                } else {
                    self.enter_loaded_type(file, e, owner);
                }
            }
        }
        for e in entries {
            if e.tag == tags::VALDEF && e.flags.has(tags::OBJECT) {
                let name = self.lname(&tasty, e.name);
                if given_objects.contains(&name) {
                    let sym = match owner {
                        Owner::Package(p) => self.syms.pkg(p).entries.get(&name).and_then(|x| x.term),
                        Owner::Class(c) => self.syms.class(c).members.get(&name).copied(),
                        Owner::Local => None,
                    };
                    if let Some(sym) = sym {
                        self.loaded_mut().tables[file as usize].terms.insert(e.addr, sym);
                    }
                    continue;
                }
                self.note_module_val(file, e, owner);
            }
        }
    }

    /// A given object of another module's products as teq models the program's own: a lazy
    /// given whose value is the one instance of a class of its own (`SymInfo::impl_class`).
    fn enter_product_given_object(&mut self, file: u32, e: &Entry, owner: Owner, name: Name) {
        let file_id = self.loaded.as_ref().unwrap().file(file).file_id;
        let cid = self.syms.new_class(name, ClassKind::GivenImpl, mods::FINAL, owner, file_id, None, Span::default());
        self.loaded_mut().classes.insert(cid, LClass { file, addr: e.addr, value_case: false });
        self.loaded_mut().tables[file as usize].classes.insert(e.addr, cid);
        let sym = self.syms.new_sym(name, SymKind::Given, mods::FINAL | mods::LAZY | mods::GIVEN, owner, file_id, None, Span::default());
        // Its type is the type it was declared with, the class's last parent, as the
        // program's own given has.
        let tasty = self.tasty(file);
        let sig = Decoder::new(&tasty).class_sig(e.addr);
        let ty = match sig.parents.last() {
            Some(p) => {
                let mut cx = MapCx::new(file);
                self.map_type(&mut cx, p)
            }
            None => self.types.class(cid, &[]),
        };
        {
            let mut s = self.syms.sym_mut(sym);
            s.sig = Some(Arc::new(MethodSig::value(ty)));
            s.state().set(Completion::Done);
            s.impl_class = Some(cid);
        }
        self.register_term(owner, sym);
        match owner {
            Owner::Package(p) => self.syms.pkgs[p.idx()].givens.push(sym),
            Owner::Class(c) => self.syms.class_mut(c).givens.push(sym),
            Owner::Local => {}
        }
        let tasty = self.tasty(file);
        let mut decoder = Decoder::new(&tasty);
        let nested = decoder.nested_types(e.addr);
        self.enter_entries(file, &nested, Owner::Class(cid));
    }

    fn enter_loaded_class(&mut self, file: u32, e: &Entry, owner: Owner, companion: Option<Companion>) -> Option<ClassId> {
        let companion_class = companion.is_some();
        let tasty = self.tasty(file);
        let is_object = tasty.is_object_class(e.name);
        let name = self.lname(&tasty, e.name);
        let file_id = self.loaded.as_ref().unwrap().file(file).file_id;
        let mut m = flag_mods(e.flags);
        // An object nested in a trait or class, a companion too, is an inner class whose
        // instance the outer holds in a lazy val of the object's name (`ClassInfo::inner_object`);
        // one with a companion class is not among the outer's nested classes, where the
        // companion stands under that name. An enum's companion stays a single object, where
        // its cases are made.
        let inner_object = is_object
            && companion != Some(Companion::OfEnum)
            && matches!(owner, Owner::Class(o) if !matches!(self.syms.class(o).kind, ClassKind::Object | ClassKind::Builtin));
        let is_object = is_object && !inner_object;
        let product = self.loaded.as_ref().map_or(false, |l| l.cp.is_products(l.file(file).cp));
        let kind = if is_object {
            ClassKind::Object
        } else if product && e.flags.has(tags::SYNTHETIC) && e.flags.has(tags::GIVEN) {
            // The class of a given with a body and parameters, as teq models the program's.
            ClassKind::GivenImpl
        } else if e.flags.has(tags::TRAIT) {
            ClassKind::Trait
        } else if e.flags.has(tags::ENUM) && !e.flags.has(tags::CASE) {
            ClassKind::Enum
        } else if e.flags.has(tags::ENUM) && e.flags.has(tags::CASE) {
            ClassKind::EnumCase
        } else {
            ClassKind::Class
        };
        // An object a package object spills (`scala.Tuple$package.EmptyTuple`) is the std's
        // top-level object of that name where the std defines one, in the file's own
        // references too.
        if is_object {
            if let Some(obj) = self.std_object_spilled(owner, name) {
                self.loaded_mut().tables[file as usize].classes.insert(e.addr, obj);
                return None;
            }
        }
        // A name the program or the std defines itself shadows the library's.
        let taken = match owner {
            Owner::Package(p) => {
                self.demand_std(p, name, crate::stdindex::TYPE | crate::stdindex::TERM);
                let entry = self.syms.pkg(p).entries.get(&name);
                if is_object {
                    entry.and_then(|e| e.term).is_some()
                } else {
                    entry.map_or(false, |e| e.class.is_some() || e.alias.is_some())
                }
            }
            Owner::Class(c) => {
                if is_object || inner_object {
                    self.syms.class(c).members.contains_key(&name)
                } else {
                    self.syms.class(c).nested.contains_key(&name)
                }
            }
            Owner::Local => true,
        };
        let shadowed = taken && !is_object && self.loaded.as_ref().unwrap().entering_shadowed && matches!(owner, Owner::Package(_));
        if taken && !shadowed {
            // The file's own references to a class a builtin stands for mean the builtin.
            if let Owner::Package(p) = owner {
                if let Some(b) = self.syms.pkg(p).entries.get(&name).and_then(|e| e.class).filter(|&b| self.syms.class(b).kind == ClassKind::Builtin) {
                    self.loaded_mut().tables[file as usize].classes.insert(e.addr, b);
                }
            }
            return None;
        }
        if is_object || inner_object {
            m |= mods::FINAL;
        }
        // The class of an inner companion object goes by scalac's module class name, apart
        // from its companion's.
        let class_name = if inner_object && companion_class { self.interner.intern(&format!("{}$", self.name_ref(name))) } else { name };
        let cid = self.syms.new_class(class_name, kind, m, owner, file_id, None, Span::default());
        // A class nested in a generic class takes the enclosing type parameters first.
        let outer: Vec<TParamId> = match owner {
            Owner::Class(o) if !matches!(self.syms.class(o).kind, ClassKind::Object | ClassKind::Builtin) => self.syms.class(o).tparams.clone(),
            _ => Vec::new(),
        };
        self.syms.class_mut(cid).outer_tparams = outer.len() as u8;
        if inner_object {
            let sym = self.syms.new_sym(name, SymKind::Val, mods::FINAL | mods::LAZY, owner, file_id, None, Span::default());
            let outer_args: Vec<TypeId> = outer.iter().map(|&p| self.types.param(p)).collect();
            let ty = self.types.class(cid, &outer_args);
            let mut s = self.syms.sym_mut(sym);
            s.sig = Some(Arc::new(MethodSig::value(ty)));
            s.state().set(Completion::Done);
            self.register_term(owner, sym);
            self.syms.class_mut(cid).inner_object = Some(sym);
        }
        // The library's tuple cons is the one the tuple rules relate to `TupleN` where the
        // standard library does not supply it.
        if self.b.cons_tuple.is_none() && matches!(owner, Owner::Package(p) if p == self.b.scala_pkg) && name == crate::names::CONS_TUPLE {
            self.b.cons_tuple = Some(cid);
            self.types.mark_op_class(cid);
        }
        if shadowed {
            if let Owner::Package(p) = owner {
                self.loaded_mut().shadowed.insert((p, name), cid);
            }
        }
        self.loaded_mut().classes.insert(cid, LClass { file, addr: e.addr, value_case: false });
        self.loaded_mut().tables[file as usize].classes.insert(e.addr, cid);
        if e.js_annotated {
            self.mark_loaded_js_class(file, e, cid, name);
        }
        if kind == ClassKind::Class && extends_any_val(&tasty, e.addr) {
            self.syms.class_mut(cid).value_class = true;
        }
        if is_object {
            let sym = self.syms.new_sym(name, SymKind::Object(cid), m & !mods::FINAL, owner, file_id, None, Span::default());
            let ty = self.types.class(cid, &[]);
            let mut s = self.syms.sym_mut(sym);
            s.sig = Some(Arc::new(MethodSig::value(ty)));
            s.state().set(Completion::Done);
            self.syms.class_mut(cid).module_sym = Some(sym);
            self.register_term(owner, sym);
            if let Some(class) = self.existing_class(owner, name) {
                self.syms.class_mut(class).companion = Some(cid);
                self.syms.class_mut(cid).companion = Some(class);
            }
        } else if shadowed {
        } else {
            match owner {
                Owner::Package(p) => self.syms.pkgs[p.idx()].entries.entry(name).or_default().class = Some(cid),
                Owner::Class(_) if inner_object && companion_class => {}
                Owner::Class(c) => {
                    self.syms.class_mut(c).nested.insert(name, cid);
                }
                Owner::Local => {}
            }
            if let Some(obj) = self.existing_object(owner, name).filter(|_| !inner_object) {
                self.syms.class_mut(cid).companion = Some(obj);
                self.syms.class_mut(obj).companion = Some(cid);
            }
            if kind == ClassKind::EnumCase {
                if let Owner::Class(companion) = owner {
                    if let Some(en) = self.syms.class(companion).companion.filter(|&en| self.syms.class(en).kind == ClassKind::Enum) {
                        self.syms.class_mut(en).children.push(cid);
                        if let Some(ordinal) = self.product_case_ordinal(companion, e.addr) {
                            self.syms.class_mut(cid).ordinal = ordinal;
                        }
                    }
                }
            }
        }
        let mut decoder = Decoder::new(&tasty);
        let tparams = decoder.class_tparams(e.addr);
        let ids: Vec<TParamId> = tparams
            .iter()
            .map(|tp| {
                let n = self.lname(&tasty, tp.name);
                let id = self.syms.new_tparam(n, variance_of(tp.flags));
                self.syms.tparams[id.idx()].arity = types::hk_arity(&tp.info);
                self.loaded_mut().tables[file as usize].tparams.insert(tp.addr, id);
                id
            })
            .collect();
        self.syms.class_mut(cid).tparams = outer.into_iter().chain(ids).collect();
        let nested = decoder.nested_types(e.addr);
        self.enter_entries(file, &nested, Owner::Class(cid));
        Some(cid)
    }

    /// A type alias or an opaque type of a class or package object.
    fn enter_loaded_type(&mut self, file: u32, e: &Entry, owner: Owner) {
        // An export's type forwarder of another module's class is the name its clause gives the
        // original, as a term forwarder is (`enter_loaded_member`): no type member of the class.
        // Its alias stays, by its address, for an original the export table cannot name.
        let product = self.loaded.as_ref().map_or(false, |l| l.cp.is_products(l.file(file).cp));
        let forwarder = product && e.flags.has(tags::EXPORTED) && matches!(owner, Owner::Class(_));
        if let (true, Owner::Class(c)) = (forwarder, owner) {
            self.syms.class_mut(c).has_exports = true;
        }
        let tasty = self.tasty(file);
        let name = self.lname(&tasty, e.name);
        let file_id = self.loaded.as_ref().unwrap().file(file).file_id;
        let taken = match owner {
            Owner::Package(p) => {
                self.demand_std(p, name, crate::stdindex::TYPE);
                self.syms.pkg(p).entries.get(&name).map_or(false, |e| e.class.is_some() || e.alias.is_some())
            }
            Owner::Class(c) => {
                let info = self.syms.class(c);
                info.nested.contains_key(&name) || info.type_aliases.contains_key(&name)
            }
            Owner::Local => true,
        };
        if taken {
            return;
        }
        let mut decoder = Decoder::new(&tasty);
        let sig = decoder.type_def_sig(e.addr);
        let abstract_member = !e.flags.has(tags::OPAQUE) && matches!(strip_lambda(&sig.rhs), TType::Bounds(..));
        // The operations of `scala.compiletime.ops` are abstract type members the compiler
        // evaluates; they load as the opaque types teq's own std declares them as.
        if e.flags.has(tags::OPAQUE) || (abstract_member && self.is_compiletime_ops_object(owner)) {
            let cid = self.syms.new_class(name, ClassKind::Opaque, flag_mods(e.flags), owner, file_id, None, Span::default());
            if abstract_member {
                self.types.mark_op_class(cid);
            } else {
                self.mark_opaque_in_class(cid);
            }
            self.loaded_mut().classes.insert(cid, LClass { file, addr: e.addr, value_case: false });
            self.loaded_mut().tables[file as usize].classes.insert(e.addr, cid);
            if let TType::Lambda { params, .. } = &sig.rhs {
                let ids: Vec<TParamId> = params
                    .iter()
                    .map(|p| {
                        let pn = self.lname(&tasty, p.name);
                        let id = self.syms.new_tparam(pn, variance_of(p.flags));
                        self.loaded_mut().tables[file as usize].tparams.insert(p.addr, id);
                        id
                    })
                    .collect();
                self.syms.class_mut(cid).tparams = ids;
            }
            match owner {
                Owner::Package(p) => self.syms.pkgs[p.idx()].entries.entry(name).or_default().class = Some(cid),
                Owner::Class(c) => {
                    self.syms.class_mut(c).nested.insert(name, cid);
                }
                Owner::Local => {}
            }
            return;
        }
        let aid = AliasId(self.syms.aliases.len() as u32);
        self.syms.aliases.push(AliasInfo {
            name,
            owner,
            file: file_id,
            def: None,
            tparams: Vec::new(),
            rhs: ERROR,
            bounds: abstract_member.then_some((NOTHING, ANY)),
        });
        self.loaded_mut().aliases.insert(aid, LAlias { file, addr: e.addr });
        self.loaded_mut().tables[file as usize].aliases.insert(e.addr, aid);
        if e.qualified_private {
            self.loaded_mut().qualified_private_aliases.insert(aid, ());
        }
        match owner {
            Owner::Package(p) => self.syms.pkgs[p.idx()].entries.entry(name).or_default().alias = Some(aid),
            Owner::Class(_) if forwarder => {}
            Owner::Class(c) => {
                self.syms.class_mut(c).type_aliases.insert(name, aid);
            }
            Owner::Local => {}
        }
    }

    fn is_compiletime_ops_object(&self, owner: Owner) -> bool {
        let Owner::Class(c) = owner else { return false };
        let Owner::Package(ops) = self.syms.class(c).owner else { return false };
        let Some(compiletime) = self.syms.pkg(ops).parent else { return false };
        let Some(scala) = self.syms.pkg(compiletime).parent else { return false };
        scala == self.b.scala_pkg
            && self.name_ref(self.syms.pkg(compiletime).name) == "compiletime"
            && self.name_ref(self.syms.pkg(ops).name) == "ops"
            && matches!(self.name_ref(self.syms.class(c).name), "int" | "string" | "boolean" | "any")
    }

    /// The std's top-level object `name` of package `p` where `owner` is a package object of
    /// `p`, which spills its members into `p`.
    fn std_object_spilled(&mut self, owner: Owner, name: Name) -> Option<ClassId> {
        let Owner::Class(o) = owner else { return None };
        let Owner::Package(p) = self.syms.class(o).owner else { return None };
        if self.syms.class(o).kind != ClassKind::Object || !types::is_package_object_name(Some(self.name_ref(self.syms.class(o).name))) {
            return None;
        }
        self.demand_std(p, name, crate::stdindex::TERM);
        let sym = self.syms.pkg(p).entries.get(&name)?.term?;
        match self.syms.sym(sym).kind {
            SymKind::Object(c) if !self.in_jar(self.syms.class(c).file) => Some(c),
            _ => None,
        }
    }

    /// The `implicit`, `given` and `lazy` of an object stand on its module val.
    fn note_module_val(&mut self, file: u32, e: &Entry, owner: Owner) {
        let tasty = self.tasty(file);
        let name = self.lname(&tasty, e.name);
        if let Owner::Class(o) = owner {
            let inner = self.nested_object_of(o, name).and_then(|c| self.syms.class(c).inner_object);
            if let Some(sym) = inner {
                self.loaded_mut().syms.insert(sym, LSym { file, addr: e.addr, conversion: false });
                self.loaded_mut().tables[file as usize].terms.insert(e.addr, sym);
                // An `implicit object` of a trait is a given of the trait (scalatest's
                // `UseDefaultAssertions`), found by its singleton type as the path itself.
                let given = flag_mods(e.flags) & (mods::IMPLICIT | mods::GIVEN);
                if given != 0 {
                    self.syms.sym_mut(sym).mods |= given;
                    self.syms.class_mut(o).givens.push(sym);
                }
                return;
            }
        }
        if let Some(std) = self.std_object_spilled(owner, name).and_then(|c| self.syms.class(c).module_sym) {
            self.loaded_mut().tables[file as usize].terms.insert(e.addr, std);
            return;
        }
        let Some(obj) = self.existing_object(owner, name) else { return };
        let Some(sym) = self.syms.class(obj).module_sym else { return };
        if !self.loaded.as_ref().unwrap().is_loaded_class(obj) {
            return;
        }
        self.loaded_mut().tables[file as usize].terms.insert(e.addr, sym);
        let m = flag_mods(e.flags) & (mods::IMPLICIT | mods::GIVEN | mods::LAZY);
        if m == 0 {
            return;
        }
        self.syms.sym_mut(sym).mods |= m;
        if m & (mods::IMPLICIT | mods::GIVEN) != 0 {
            match owner {
                Owner::Package(p) => self.syms.pkgs[p.idx()].givens.push(sym),
                Owner::Class(c) => self.syms.class_mut(c).givens.push(sym),
                Owner::Local => {}
            }
        }
    }

    // ---- completing classes ----

    /// Completes the jar classes whose erasure their completion decides and that nothing
    /// completed: the value classes, which erase to the type of their parameter, the opaque
    /// types, which erase to what they stand for, and the classes of a union's parts, whose
    /// erasure is their common base class. A class that only a signature names is entered and
    /// never completed, and the JVM's backend erases every type its descriptors name.
    pub fn complete_erased_jar_classes(&mut self) {
        // A completion may enter classes that are such classes in turn.
        let mut tried: FxMap<ClassId, ()> = FxMap::default();
        loop {
            let Some(loaded) = self.loaded.as_ref() else { return };
            // The classes a union's part erases through: an array's elements, a bound, a path's
            // type, a refinement's parent, an intersection's parts.
            let mut in_unions: FxMap<ClassId, ()> = FxMap::default();
            let mut work = self.types.union_parts();
            let mut seen: FxMap<TypeId, ()> = FxMap::default();
            while let Some(t) = work.pop() {
                if seen.insert(t, ()).is_some() {
                    continue;
                }
                match self.types.get(t) {
                    Type::Class(c, args) => {
                        in_unions.insert(c, ());
                        if c == self.b.array {
                            work.extend(self.types.items(args).iter().copied());
                        }
                    }
                    Type::Param(p) | Type::AppParam(p, _) => work.push(self.syms.tparam(p).upper),
                    Type::Lambda(_, body) => work.push(body),
                    Type::Term(s) | Type::Select(_, s) => {
                        if let Some(ret) = self.syms.sym(s).sig.as_ref().map(|sig| sig.ret) {
                            work.push(ret);
                        }
                    }
                    Type::Refined(parent, _) => work.push(parent),
                    Type::Inter(a, b) | Type::Union(a, b) => work.extend([a, b]),
                    _ => {}
                }
            }
            let mut open: Vec<ClassId> = loaded
                .classes
                .keys()
                .copied()
                .filter(|&c| {
                    let info = self.syms.class(c);
                    let opaque = info.kind == ClassKind::Opaque && !self.types.is_op_class(c);
                    info.state() == Completion::NotStarted && (info.value_class || opaque || in_unions.contains_key(&c)) && !tried.contains_key(&c)
                })
                .collect();
            if open.is_empty() {
                return;
            }
            open.sort_unstable();
            for c in open {
                tried.insert(c, ());
                self.complete_class(c);
            }
        }
    }

    pub(super) fn complete_loaded_class(&mut self, c: ClassId) {
        if self.is_java_placeholder(c) {
            self.complete_java_placeholder(c);
            return;
        }
        if self.complete_java_class(c) {
            return;
        }
        let Some(lc) = self.loaded.as_ref().unwrap().classes.get(&c).copied() else {
            let targs: Vec<TypeId> = self.syms.class(c).tparams.clone().iter().map(|&p| self.types.param(p)).collect();
            let self_ty = self.types.class(c, &targs);
            let mut info = self.syms.class_mut(c);
            info.base_types.push((c, self_ty));
            info.state().set(Completion::Done);
            return;
        };
        self.loaded_mut().classes_completed += 1;
        if std::env::var_os("TEQ_CLASSPATH_DETAIL").is_some() {
            eprintln!("complete {}", self.class_path(c));
        }
        let prof = self.prof(Kind::Completion, Span::default(), About::Class(c));
        self.complete_tasty_class(c, lc);
        if let Some(p) = prof {
            self.profile.exit(p, Outcome::Found);
        }
    }

    fn complete_tasty_class(&mut self, c: ClassId, lc: LClass) {
        let tasty = self.tasty(lc.file);
        let mut decoder = Decoder::new(&tasty);
        let kind = self.syms.class(c).kind;
        if kind == ClassKind::Opaque {
            let sig = decoder.type_def_sig(lc.addr);
            let mut cx = MapCx::new(lc.file);
            let under = self.map_type(&mut cx, strip_lambda(&sig.rhs));
            self.syms.class_mut(c).underlying = Some(under);
            let targs: Vec<TypeId> = self.syms.class(c).tparams.clone().iter().map(|&p| self.types.param(p)).collect();
            let self_ty = self.types.class(c, &targs);
            self.syms.class_mut(c).base_types = vec![(c, self_ty)];
            // An opaque type conforms to its declared bound; an abstract type read so, whose
            // definition is its bounds (`type +[X <: Int, Y <: Int] <: Int` of
            // `scala.compiletime.ops.int`), to its upper one.
            let upper = match (&sig.opaque_bounds, strip_lambda(&sig.rhs)) {
                (Some(TType::Bounds(_, hi)), _) | (None, TType::Bounds(_, hi)) => Some(&**hi),
                _ => None,
            };
            if let Some(hi) = upper {
                let bound = self.map_type(&mut cx, hi);
                if let Type::Class(bc, bargs) = self.types.get(bound) {
                    self.syms.class_mut(c).parents = vec![bound];
                    self.complete_class(bc);
                    let subst: Subst = self.syms.class(bc).tparams.iter().copied().zip(self.types.items(bargs).iter().copied()).collect();
                    let inherited: Vec<(ClassId, TypeId)> = self.syms.class(bc).base_types.clone();
                    for (x, xt) in inherited {
                        let t = self.types.subst(xt, &subst);
                        self.syms.class_mut(c).base_types.push((x, t));
                    }
                }
            }
            self.syms.class_mut(c).state().set(Completion::Done);
            return;
        }
        if lc.value_case {
            let sig = decoder.def_sig(lc.addr);
            let mut cx = MapCx::new(lc.file);
            let parent = self.map_type(&mut cx, &sig.ret);
            self.set_loaded_parents(c, vec![parent]);
            self.syms.class_mut(c).state().set(Completion::Done);
            return;
        }
        let sig = decoder.class_sig(lc.addr);
        let mut cx = MapCx::new(lc.file);
        let ids = self.syms.class(c).own_tparams().to_vec();
        for (tp, &id) in sig.tparams.iter().zip(&ids) {
            self.set_tparam_bounds(&mut cx, id, &tp.info);
        }
        let mut parents: Vec<TypeId> = sig.parents.iter().map(|p| self.map_type(&mut cx, p)).collect();
        // An enum of another module is read as the program's own: that it is a
        // `scala.reflect.Enum` is the typer's rule (`Typer::is_enum_value`), not a parent whose
        // members it would take.
        if matches!(kind, ClassKind::Enum | ClassKind::EnumCase) && self.loaded.as_ref().map_or(false, |l| l.is_product_class(c)) {
            let reflect_enum = self.b.reflect_enum;
            parents.retain(|&p| !matches!(self.types.get(p), Type::Class(pc, _) if Some(pc) == reflect_enum));
        }
        // A case class of another module is read as the program's own: its pickle extends
        // `Product` and `Serializable` as scalac's `Desugar` makes it, which teq's typer gives a
        // case class of source by its rules, not as parents (an inferred type over the classes
        // of two modules is then the one it is within one program).
        if sig.mods.flags.has(tags::CASE) && matches!(kind, ClassKind::Class | ClassKind::EnumCase) && self.loaded.as_ref().map_or(false, |l| l.is_product_class(c)) {
            let product = self.b.product;
            parents.retain(|&p| match self.types.get(p) {
                Type::Class(pc, _) => Some(pc) != product && !(self.name_str(self.syms.class(pc).name) == "Serializable" && matches!(self.syms.class(pc).owner, Owner::Package(k) if self.pkg_description(k) == "java.io")),
                _ => true,
            });
        }
        // So are a case object and a companion: the parents teq's writer appends after the
        // declared ones, as scalac's `Desugar` and `SyntheticMembers` do (a case object's
        // `Product`, `Serializable` and `Mirror.Singleton`, a companion's mirror), which teq's
        // typer gives by its rules, are taken off the end, as many as the pickle's origins
        // record (kind 11); a parent the source declares stays.
        if kind == ClassKind::Object && self.loaded.as_ref().map_or(false, |l| l.is_product_class(c)) {
            let appended = match &self.loaded.as_ref().unwrap().file(lc.file).provenance {
                crate::tasty::origins::Found::Read(o) => o.appended.binary_search_by_key(&lc.addr, |&(a, _)| a).ok().map_or(0, |i| o.appended[i].1),
                _ => 0,
            };
            let keep = parents.len().saturating_sub(appended as usize);
            parents.truncate(keep);
        }
        if let Some((_, t)) = sig.self_type.as_ref().filter(|_| !sig.mods.flags.has(tags::OBJECT)) {
            let declared = self.map_type(&mut cx, t);
            self.syms.class_mut(c).declared_self = Some(declared);
        }
        self.set_loaded_parents(c, parents);
        if sig.mods.flags.has(tags::SEALED) {
            self.enter_sealed_children(c, &mut cx, &sig.mods.annots);
        }
        if let Some(ctor) = &sig.ctor {
            self.enter_loaded_ctor(c, &mut cx, ctor, &sig.accessors);
            self.enter_loaded_ctor_access(c, &ctor.mods);
        }
        // As in source: the values of an enum with parameters or a superclass are created by
        // its companion, with the arguments their cases pass.
        if kind == ClassKind::Enum {
            let info = self.syms.class(c);
            if info.superclass.is_some() || info.ctor.iter().any(|cl| !cl.params.is_empty()) {
                self.syms.class_mut(c).stateful = true;
            }
        }
        // A companion that declares an `apply` of its own next to the synthetic one keeps both:
        // they are alternatives of one name.
        let explicit_apply = sig.index.members.iter().any(|m| {
            m.tag == tags::DEFDEF && !m.flags.has(tags::SYNTHETIC) && self.tasty(lc.file).simple(m.name) == Some("apply")
        });
        // A Scala 2 macro definition kept in a cross-built library beside its `inline` twin
        // (munit's `Location.generate`) has nothing teq could run; one on its own
        // (scala-library's `StringContext.s`) is a member like any other.
        let inline_names: Vec<_> = sig.index.members.iter().filter(|m| m.tag == tags::DEFDEF && m.flags.has(tags::INLINE)).map(|m| m.name).collect();
        for e in &sig.index.members {
            if e.tag == tags::DEFDEF && e.flags.has(tags::MACRO) && !e.flags.has(tags::INLINE) && inline_names.contains(&e.name) {
                continue;
            }
            self.enter_loaded_member(c, lc.file, e, explicit_apply);
        }
        // A trait of another module that calls members through `super` has their accessors
        // in its pickle: the classes of this build that mix it in bind them, as for the
        // program's own traits (`Worker::bind_mixin_supers`).
        if kind == ClassKind::Trait && self.loaded.as_ref().map_or(false, |l| l.is_product_class(c)) {
            let tasty = self.tasty(lc.file);
            let called: Vec<Name> = sig
                .index
                .members
                .iter()
                .filter(|e| e.tag == tags::DEFDEF && e.flags.has(tags::ARTIFACT))
                .filter_map(|e| super_accessor_member(&tasty, e.name))
                .map(|n| self.interner.intern(&n))
                .collect();
            for name in called {
                let parents: Vec<TypeId> = self.syms.class(c).parents.clone();
                let found = parents.iter().rev().find_map(|&p| self.find_member(p, name).map(|(m, _)| m));
                if let Some(m) = found {
                    let calls = self.mixin_supers.entry(c).or_default();
                    if !calls.contains(&m) {
                        calls.push(m);
                    }
                }
            }
        }
        if self.syms.class(c).base_types.len() > 1 {
            self.mark_members_meeting_inherited(c);
            self.join_unrelated_inherited(c);
        }
        // A trait whose vars have initialisers runs them in the constructor of each class that
        // mixes it in, which the class has to know before the trait's body is compiled; its
        // vals are lazy fields on JavaScript and run there too on the JVM.
        // A val that implements an ancestor's def stays eager too (`ZIOAppDefault.bootstrap`).
        // A product's trait is another module's program: its vals and its statements run as the
        // whole program's do. On the JVM a jar trait's statements are its
        // `$init$`'s, which every class mixing it in calls, as scalac's `Mixin` does.
        if kind == ClassKind::Trait {
            let tasty = self.tasty(lc.file);
            let product = self.loaded.as_ref().map_or(false, |l| l.is_product_class(c));
            let eager = self.jvm || product;
            let has_statements = (eager && sig.index.statements)
                || sig.index.members.iter().any(|e| {
                    e.tag == tags::VALDEF && e.has_body && !e.flags.has(tags::OBJECT) && !e.flags.has(tags::LAZY) && {
                        let name = self.interner.intern(&tasty.name(tasty.source_name(e.name)));
                        eager || e.flags.has(tags::MUTABLE) || self.syms.class(c).members.get(&name).map_or(false, |&m| self.implements_def(c, m))
                    }
                });
            self.syms.class_mut(c).has_statements |= has_statements;
        }
        self.syms.class_mut(c).state().set(Completion::Done);
        // The names the members go by in JavaScript are settled before any call to them is
        // walked: the reach dispatches by name, and a watch session's later builds meet the
        // names as settled. They are settled once the outermost completion is done, since
        // naming decodes signatures, which may name the alias or class being completed.
        if !self.jvm {
            self.pending_library_names.push(c);
        }
    }

    /// The children of a sealed class are its `@Child[C]` annotations, so that a match over it
    /// knows them before any of them is completed.
    fn enter_sealed_children(&mut self, c: ClassId, cx: &mut MapCx, annots: &[TType]) {
        let tasty = self.tasty(cx.file);
        for a in annots {
            let TType::Applied(class, args) = a else { continue };
            let is_child = matches!(&**class, TType::TypeRef(_, n) if tasty.simple(*n) == Some("Child"));
            if !is_child || args.len() != 1 {
                continue;
            }
            // teq's tuple classes stand in for the library's, which `Tuple` lists.
            let is_tuple = matches!(&args[0], TType::TypeRef(_, n) if tasty.simple(*n).map_or(false, |s| s.starts_with("Tuple")));
            if is_tuple {
                continue;
            }
            // `Red.type` names a value case through its val, whose type is the enum itself.
            let k = match self.sealed_child_term(cx, &args[0]) {
                Some(k) => k,
                None => {
                    let child = self.map_type(cx, &args[0]);
                    match self.types.get(child) {
                        Type::Class(k, _) => k,
                        _ => continue,
                    }
                }
            };
            if k != c && !self.syms.class(c).children.contains(&k) {
                self.syms.class_mut(c).children.push(k);
            }
        }
    }

    /// The ordinal of a product's enum case, the case at `addr` of the enum's companion: its
    /// place among the companion's cases in the source, value and class cases alike, as the
    /// compiler numbers them (the definitions' points of the origins, kind 1). `None` for a jar's
    /// or a pickle without the points.
    fn product_case_ordinal(&mut self, companion: ClassId, addr: Addr) -> Option<u32> {
        let loaded = self.loaded.as_ref()?;
        let lc = *loaded.classes.get(&companion)?;
        if !loaded.is_product_class(companion) {
            return None;
        }
        let crate::tasty::origins::Found::Read(o) = &loaded.file(lc.file).provenance else { return None };
        let point = |a: Addr| o.definitions.binary_search_by_key(&a, |&(d, _)| d).ok().map(|i| o.definitions[i].1);
        let tasty = self.tasty(lc.file);
        let sig = Decoder::new(&tasty).class_sig(lc.addr);
        let mut cases: Vec<(u32, Addr)> = sig.index.members.iter().filter(|m| m.flags.has(tags::ENUM) && m.flags.has(tags::CASE)).map(|m| point(m.addr).map(|p| (p, m.addr))).collect::<Option<_>>()?;
        cases.sort_unstable();
        cases.iter().position(|&(_, a)| a == addr).map(|i| i as u32)
    }

    fn sealed_child_term(&mut self, cx: &mut MapCx, t: &TType) -> Option<ClassId> {
        let sym = match t {
            TType::TermRef(..) | TType::LocalTerm(..) => self.path_term(cx, t)?,
            _ => return None,
        };
        match self.syms.sym(sym).kind {
            SymKind::EnumValue(k) | SymKind::Object(k) => Some(k),
            _ => None,
        }
    }

    pub(super) fn set_tparam_bounds(&mut self, cx: &mut MapCx, id: TParamId, info: &TType) {
        let (lower, upper) = match info {
            TType::Bounds(lo, hi) => (self.map_bound(cx, lo, false), self.map_bound(cx, hi, true)),
            // `F2[x] >: F[x]`: a lambda over the bounds, each bound a lambda of its own.
            TType::Lambda { result, .. } if matches!(**result, TType::Bounds(..)) => (self.map_bound(cx, info, false), self.map_bound(cx, info, true)),
            TType::Lambda { .. } => (NOTHING, self.map_bound(cx, info, true)),
            other => (NOTHING, self.map_bound(cx, other, true)),
        };
        let lambda = match info {
            TType::Bounds(_, hi) => &**hi,
            other => other,
        };
        let hk_variances: Vec<i8> = match lambda {
            TType::Lambda { params, .. } => params
                .iter()
                .map(|p| variance_of(p.flags))
                .collect(),
            _ => Vec::new(),
        };
        let p = &mut self.syms.tparams[id.idx()];
        p.lower = lower;
        p.upper = upper;
        if hk_variances.iter().any(|&v| v != 0) {
            p.hk_variances = hk_variances;
        }
    }

    /// A bound of a type parameter: `Any` and `Nothing` stay the absent bounds, a type lambda
    /// over bounds keeps its result as the bound of a higher-kinded parameter.
    fn map_bound(&mut self, cx: &mut MapCx, t: &TType, upper: bool) -> TypeId {
        match t {
            TType::Lambda { kind: crate::tasty::tree::LambdaKind::Type, params, result, binder } => {
                let inner = match &**result {
                    TType::Bounds(lo, hi) => if upper { &**hi } else { &**lo },
                    other => other,
                };
                if self.is_open_bound(cx, inner, upper) {
                    return if upper { ANY } else { NOTHING };
                }
                let lambda = TType::Lambda { kind: crate::tasty::tree::LambdaKind::Type, params: params.clone(), result: Box::new(inner.clone()), binder: *binder };
                self.map_type_ctor(cx, &lambda)
            }
            other => {
                if self.is_open_bound(cx, other, upper) {
                    return if upper { ANY } else { NOTHING };
                }
                self.map_type(cx, other)
            }
        }
    }

    pub(super) fn is_open_bound(&self, cx: &MapCx, t: &TType, upper: bool) -> bool {
        let tasty = self.tasty(cx.file);
        match t {
            TType::TypeRef(prefix, n) => {
                // `AnyKind` bounds a parameter of any kind (shapeless3's `Kind[Up <: AnyKind, ...]`).
                let open = tasty.simple(*n) == Some(if upper { "Any" } else { "Nothing" }) || (upper && tasty.simple(*n) == Some("AnyKind"));
                open && matches!(&**prefix, TType::Package(p) if tasty.simple(*p) == Some("scala"))
            }
            _ => false,
        }
    }

    /// Base types through the linearisation, without the checks a source class gets: the
    /// library was checked by scalac.
    fn set_loaded_parents(&mut self, c: ClassId, parents: Vec<TypeId>) {
        let targs: Vec<TypeId> = self.syms.class(c).tparams.clone().iter().map(|&p| self.types.param(p)).collect();
        let self_ty = self.types.class(c, &targs);
        let mut linearisations: Vec<Vec<(ClassId, TypeId)>> = Vec::with_capacity(parents.len());
        let mut parent_types = Vec::new();
        let mut superclass: Option<ClassId> = None;
        let kind = self.syms.class(c).kind;
        for pt in parents {
            let pt = self.deref_alias(pt);
            let Type::Class(pc, pargs) = self.types.get(pt) else { continue };
            // A Java class that only a signature named stays unread until a member is asked of it.
            if !self.is_java_placeholder(pc) {
                self.complete_class(pc);
            }
            let pinfo = self.syms.class(pc);
            if pinfo.state() == Completion::InProgress || pinfo.kind == ClassKind::Builtin {
                if pc == self.b.any_val && kind == ClassKind::Class {
                    self.syms.class_mut(c).value_class = true;
                }
                continue;
            }
            // Nothing is emitted for a native JS class, so extending one costs no inheritance.
            if pinfo.kind == ClassKind::Class && pinfo.js == JsKind::Native {
            } else if pinfo.kind == ClassKind::Class && (kind != ClassKind::Trait || self.in_jar(pinfo.file)) {
                // A trait's class parent is the superclass of what mixes it in, unless a
                // stand-in of the std plays the library's trait (`ClassTag`).
                superclass = Some(pc);
            } else if pinfo.kind == ClassKind::Enum && kind == ClassKind::EnumCase && pinfo.superclass.is_some() {
                // As in source: the cases of an enum that extends a class are subclasses of
                // the enum, which passes its arguments on to that class.
                superclass = Some(pc);
            } else if let Some(sc) = pinfo.superclass {
                if superclass.map_or(true, |s| self.syms.class(sc).base_types.iter().any(|&(b, _)| b == s)) {
                    superclass = Some(sc);
                }
            }
            if pinfo.mods & mods::SEALED != 0 && !self.syms.class(pc).children.contains(&c) {
                self.syms.class_mut(pc).children.push(c);
            }
            parent_types.push(pt);
            let subst: Subst = self.syms.class(pc).tparams.iter().copied().zip(self.types.items(pargs).iter().copied()).collect();
            let inherited: Vec<(ClassId, TypeId)> = self.syms.class(pc).base_types.clone();
            linearisations.push(inherited.into_iter().map(|(bc, bt)| (bc, self.types.subst(bt, &subst))).collect());
        }
        let (mut base, _) = self.linearise(c, self_ty, linearisations.clone(), false);
        self.narrow_base_types(c, &mut base, &linearisations);
        if let Some(parent) = superclass {
            let subclasses = &mut self.syms.class_mut(parent).subclasses;
            if !subclasses.contains(&c) {
                subclasses.push(c);
            }
        }
        let inherits_types = base.iter().skip(1).any(|&(b, _)| {
            let bi = self.syms.class(b);
            !bi.type_aliases.is_empty()
                || bi.inherits_types
                || bi.nested.values().any(|&n| self.syms.class(n).outer_tparams > 0 || self.types.is_path_class(n))
        });
        let has_native_base = base.iter().skip(1).any(|&(b, _)| self.syms.class(b).js == JsKind::Native);
        let mut info = self.syms.class_mut(c);
        info.parents = parent_types;
        info.base_types = base;
        info.superclass = superclass;
        info.inherits_types = inherits_types;
        if has_native_base && info.js == JsKind::Scala {
            info.js = JsKind::Object;
        }
        if let Some(declared) = info.declared_self {
            info.this_type = Some(self.types.inter(self_ty, declared));
        }
    }

    /// A library class inherits one ancestor at several instantiations that differ in variant
    /// parameters (`List` has `IterableOps[A, Iterable, Iterable[A]]` through `AbstractSeq` and
    /// `IterableOps[A, List, List[A]]` through `LinearSeqOps`), and scalac's base type is the
    /// most specific of them. The linearisation keeps the first parent's; each covariant
    /// argument is replaced by another instantiation's when that one's head class is nearer to
    /// `c` in the linearisation, which is what the collections' `CC` and `C` parameters are.
    fn narrow_base_types(&mut self, c: ClassId, base: &mut [(ClassId, TypeId)], linearisations: &[Vec<(ClassId, TypeId)>]) {
        let order: Vec<ClassId> = base.iter().map(|&(b, _)| b).collect();
        let rank = |t: &Self, ty: TypeId| -> Option<usize> {
            let head = match t.types.get(ty) {
                Type::Class(k, _) | Type::Ctor(k) => k,
                Type::Lambda(_, body) => match t.types.get(body) {
                    Type::Class(k, _) => k,
                    _ => return None,
                },
                _ => return None,
            };
            if head == c {
                return Some(0);
            }
            order.iter().position(|&b| b == head)
        };
        let parts = |t: &Self, ty: TypeId| {
            let mut out = Vec::new();
            let mut stack = vec![ty];
            while let Some(x) = stack.pop() {
                match t.types.get(x) {
                    Type::Inter(a, b) => stack.extend([a, b]),
                    _ if x == crate::types::ANY => {}
                    _ => out.push(x),
                }
            }
            out
        };
        let parts_within = |t: &Self, upper: TypeId, lower: TypeId| {
            let within = parts(t, lower);
            parts(t, upper).iter().all(|p| within.contains(p))
        };
        for i in 1..base.len() {
            let (bc, bt) = base[i];
            let from = |ty: TypeId| linearisations.iter().position(|l| l.iter().any(|&(x, t)| x == bc && t == ty));
            let others: Vec<(Option<usize>, TypeId)> = linearisations
                .iter()
                .enumerate()
                .flat_map(|(j, l)| l.iter().filter(|&&(x, t)| x == bc && t != bt).map(move |&(_, t)| (Some(j), t)))
                .collect();
            if others.is_empty() {
                continue;
            }
            let mine_from = from(bt);
            let Type::Class(_, args) = self.types.get(bt) else { continue };
            let mut args: Vec<TypeId> = self.types.items(args).to_vec();
            let tparams = self.syms.class(bc).tparams.clone();
            let mut changed = false;
            let mut decided_by: Vec<Option<usize>> = vec![mine_from; args.len()];
            for (other_from, other) in others {
                let Type::Class(_, oargs) = self.types.get(other) else { continue };
                let oargs: Vec<TypeId> = self.types.items(oargs).to_vec();
                for (k, &p) in tparams.iter().enumerate() {
                    if self.syms.tparam(p).variance != 1 || k >= args.len() || k >= oargs.len() || args[k] == oargs[k] {
                        continue;
                    }
                    // An intersection is below each of its parts, `Any` above everything:
                    // the meet of the two is the narrower one (`GenericBackend[F, S &
                    // WebSockets]` through the parent, `[F, WebSockets]` and `[F, Any]` above).
                    match (parts_within(self, oargs[k], args[k]), parts_within(self, args[k], oargs[k])) {
                        (true, _) => continue,
                        (false, true) => {
                            args[k] = oargs[k];
                            decided_by[k] = other_from;
                            changed = true;
                            continue;
                        }
                        _ => {}
                    }
                    match (rank(self, args[k]), rank(self, oargs[k])) {
                        (Some(mine), Some(theirs)) => {
                            if theirs < mine {
                                args[k] = oargs[k];
                                decided_by[k] = other_from;
                                changed = true;
                            }
                        }
                        // What the ranks do not order, the linearisation does: the base
                        // through the parent written last comes first in it.
                        _ => {
                            if other_from > decided_by[k] {
                                args[k] = oargs[k];
                                decided_by[k] = other_from;
                                changed = true;
                            }
                        }
                    }
                }
            }
            if changed {
                base[i].1 = self.types.class(bc, &args);
            }
        }
    }

    /// A jar class's private constructor is one for the program as a source class's is. A
    /// qualified or protected one stays public.
    fn enter_loaded_ctor_access(&mut self, c: ClassId, m: &crate::tasty::tree::Mods) {
        if m.within.is_none() && m.flags.has(tags::PRIVATE) {
            self.syms.class_mut(c).mods |= mods::PRIVATE_CTOR;
        }
    }

    fn enter_loaded_ctor(&mut self, c: ClassId, cx: &mut MapCx, ctor: &DefSig, accessors: &[crate::tasty::tree::Param]) {
        let tasty = self.tasty(cx.file);
        let file_id = self.syms.class(c).file;
        let mut clauses = Vec::new();
        let mut syms_per_clause = Vec::new();
        // The constructor's type parameters are the class's under addresses of their own.
        let class_tparams = self.syms.class(c).own_tparams().to_vec();
        let mut i = 0;
        for clause in &ctor.clauses {
            if let Clause::Types(ps) = clause {
                for p in ps {
                    if let Some(&id) = class_tparams.get(i) {
                        self.loaded_mut().tables[cx.file as usize].tparams.insert(p.addr, id);
                    }
                    i += 1;
                }
            }
        }
        for clause in &ctor.clauses {
            let Clause::Terms(ps) = clause else { continue };
            let is_using = is_using_clause(ps);
            let mut sig = ClauseSig { params: Vec::with_capacity(ps.len()), is_using, is_implicit: is_implicit_clause(ps) };
            let mut syms = Vec::with_capacity(ps.len());
            for p in ps {
                let name = self.lname(&tasty, p.name);
                let accessor = accessors.iter().find(|a| a.name == p.name);
                let flags = accessor.map_or(p.flags, |a| a.flags);
                let mut m = flag_mods(flags) & !mods::FINAL;
                let is_field = !flags.has(tags::PRIVATE) || flags.has(tags::CASEACCESSOR);
                if is_field {
                    m |= mods::FIELD;
                    m &= !mods::PRIVATE;
                } else {
                    m |= mods::PRIVATE;
                }
                let kind = if flags.has(tags::MUTABLE) { SymKind::Var } else { SymKind::Val };
                let (ty, by_name, repeated) = self.map_param_type(cx, &p.ty);
                let field_ty = match (repeated, self.seq_class()) {
                    (true, Some(seq)) => self.types.class(seq, &[ty]),
                    _ => ty,
                };
                let sym = self.syms.new_sym(name, kind, m, Owner::Class(c), file_id, None, Span::default());
                {
                    let mut s = self.syms.sym_mut(sym);
                    s.sig = Some(Arc::new(MethodSig::value(field_ty)));
                    s.state().set(Completion::Done);
                    s.by_name = by_name;
                }
                if !self.syms.class(c).members.contains_key(&name) {
                    self.register_term(Owner::Class(c), sym);
                }
                if is_using {
                    self.syms.class_mut(c).givens.push(sym);
                }
                // A body names the field by the constructor parameter or its accessor
                // (`quote[From](source)` in `TransformerInto.transform`), whichever it read first.
                let terms = &mut self.loaded_mut().tables[cx.file as usize].terms;
                for addr in std::iter::once(p.addr).chain(accessor.map(|a| a.addr)) {
                    terms.entry(addr).or_insert(sym);
                }
                sig.params.push(ParamSig { name, ty, by_name, repeated, has_default: p.flags.has(tags::HASDEFAULT), sym });
                syms.push(sym);
            }
            clauses.push(sig);
            syms_per_clause.push(syms);
        }
        // `class C` is pickled with the empty clause of `class C()`.
        if clauses.len() == 1 && clauses[0].params.is_empty() && !clauses[0].is_using {
            clauses.clear();
            syms_per_clause.clear();
        }
        let mut info = self.syms.class_mut(c);
        info.ctor = clauses;
        info.ctor_syms = syms_per_clause;
    }

    fn enter_loaded_member(&mut self, c: ClassId, file: u32, e: &Entry, explicit_apply: bool) {
        let tasty = self.tasty(file);
        let f = e.flags;
        if f.has(tags::ARTIFACT) || f.has(tags::OBJECT) {
            return;
        }
        // An export forwarder of another module's class is the name its clause gives the
        // original, as the program's own export is (`Worker::product_exports`).
        let product = self.loaded.as_ref().map_or(false, |l| l.cp.is_products(l.file(file).cp));
        if product && f.has(tags::EXPORTED) {
            self.syms.class_mut(c).has_exports = true;
            return;
        }
        if tasty.simple(e.name) == Some("<init>") {
            self.enter_loaded_secondary_ctor(c, file, e);
            return;
        }
        // Of what scalac synthesizes for a case class, `apply` is kept for the companion of an
        // enum case: the patterns go by the case accessors, `copy` teq derives itself, and a
        // plain case class's companion is applied through the constructor, as the program's
        // own are. The field of a tuple pattern in a class body (`val (a, b) = ..` is a
        // synthetic `$1` the accessors read) and an inline accessor are members like any other.
        // Another module's classes are the program's own as teq compiles them: a case class
        // or an enum case is made by its constructor there too, whatever `apply` the companion
        // writes itself.
        if f.has(tags::SYNTHETIC)
            && !f.has(tags::CASEACCESSOR)
            && !is_pattern_field(&tasty, e)
            && !tasty.is_inline_accessor(e.name)
            && (tasty.simple(e.name) != Some("apply") || (self.is_plain_case_companion(c) && !explicit_apply) || product)
        {
            return;
        }
        if e.tag == tags::TYPEDEF {
            return;
        }
        let name = self.lname(&tasty, e.name);
        let file_id = self.syms.class(c).file;
        let mut m = flag_mods(f);
        // A given without a body is abstract too: an old-style abstract given, a def, or with
        // `HASDEFAULT` a deferred given (dotty's `Namer` flags it `Deferred | HasDefault`).
        if !e.has_body {
            m |= mods::ABSTRACT;
            if f.has(tags::GIVEN) && f.has(tags::HASDEFAULT) {
                m |= mods::DEFERRED;
            }
        }
        // An abstract var's setter, as scalac and teq pickle it (`typer::setters`); a var
        // parameter's has no body either.
        if e.tag == tags::DEFDEF && !e.has_body && f.has(tags::FIELDACCESSOR) && f.has(tags::MUTABLE) && !f.has(tags::PARAMSETTER) {
            m |= mods::SETTER;
        }
        let is_enum_case = e.tag == tags::VALDEF && f.has(tags::ENUM) && f.has(tags::CASE);
        // An old-style abstract given (`given x: T`) is a def in the program, as in its pickle.
        let abstract_given_def = e.tag == tags::DEFDEF && !e.has_body && !f.has(tags::HASDEFAULT);
        let product_given = product && f.has(tags::GIVEN) && !is_enum_case && !abstract_given_def;
        let kind = if product_given {
            SymKind::Given
        } else if is_enum_case {
            let case = self.syms.new_class(name, ClassKind::EnumCase, mods::FINAL | mods::CASE, Owner::Class(c), file_id, None, Span::default());
            self.loaded_mut().classes.insert(case, LClass { file, addr: e.addr, value_case: true });
            if product {
                if let Some(place) = self.loaded_mut().definition_place(file, e.addr) {
                    self.loaded_mut().product_class_places.insert(case, place);
                }
            }
            if let Some(en) = self.syms.class(c).companion.filter(|&en| self.syms.class(en).kind == ClassKind::Enum) {
                let ordinal = self.product_case_ordinal(c, e.addr).unwrap_or(self.syms.class(en).children.len() as u32);
                self.syms.class_mut(en).children.push(case);
                self.syms.class_mut(case).ordinal = ordinal;
            }
            SymKind::EnumValue(case)
        } else if e.tag == tags::DEFDEF {
            SymKind::Def
        } else if f.has(tags::MUTABLE) {
            SymKind::Var
        } else {
            SymKind::Val
        };
        if f.has(tags::CASEACCESSOR) && self.syms.class(c).members.contains_key(&name) {
            return;
        }
        // An implicit class brings a conversion method and an object of its name: the method
        // is what the name means.
        let shadows_object = kind == SymKind::Def
            && self.syms.class(c).members.get(&name).map_or(false, |&o| matches!(self.syms.sym(o).kind, SymKind::Object(_)));
        if shadows_object {
            let mut info = self.syms.class_mut(c);
            let object = info.members.remove(&name);
            info.member_order.retain(|&m| Some(m) != object);
        }
        let sym = self.syms.new_sym(name, kind, m, Owner::Class(c), file_id, None, Span::default());
        // An abstract var is read through its getter (`typer::setters`).
        if kind == SymKind::Var && m & mods::ABSTRACT != 0 {
            self.syms.sym_mut(sym).needs_accessor = true;
        }
        if let SymKind::EnumValue(case) = kind {
            self.syms.class_mut(case).singleton = Some(sym);
        }
        // What scalac adds to a class that teq's model of the class has not as members: the
        // default getters, a concrete var's setter; an
        // abstract var's (`mods::SETTER`) is a member of the model too.
        if product && e.tag == tags::DEFDEF {
            let default_getter = matches!(tasty.names.get(tasty.source_name(e.name) as usize), Some(crate::tasty::TName::DefaultGetter(..)));
            let setter = m & mods::SETTER == 0 && tasty.simple(e.name).and_then(|n| n.strip_suffix("_=")).map_or(false, |var| {
                let var = self.interner.intern(var);
                self.syms.class(c).members.get(&var).map_or(false, |&v| self.syms.sym(v).kind == SymKind::Var)
            });
            // And the accessors teq's writer gives a class for its inline bodies.
            let accessor = tasty.simple(e.name).map_or(false, |n| n.starts_with("inline$"));
            if default_getter || setter || accessor {
                self.loaded_mut().product_synthetics.insert(sym, ());
            }
        }
        if product_given && e.tag == tags::DEFDEF {
            let impl_class = self.syms.class(c).nested.get(&name).copied().filter(|&k| self.syms.class(k).kind == ClassKind::GivenImpl);
            self.syms.sym_mut(sym).impl_class = impl_class;
        }
        let conversion = f.has(tags::IMPLICIT) && e.tag == tags::DEFDEF && {
            let sig = Decoder::new(&tasty).def_sig(e.addr);
            matches!(sig.clauses.iter().find(|c| matches!(c, Clause::Terms(_))), Some(Clause::Terms(ps)) if !is_using_clause(ps))
        };
        self.loaded_mut().syms.insert(sym, LSym { file, addr: e.addr, conversion });
        if let Some(q) = e.access_within {
            let full = tasty.name(tasty.source_name(q));
            let within = self.interner.intern(full.rsplit('.').next().unwrap_or(&full));
            self.loaded_mut().access_within.insert(sym, (within, !e.qualified_private));
            self.syms.sym_mut(sym).mods |= crate::ast::mods::QUALIFIED;
        }
        self.loaded_mut().tables[file as usize].terms.insert(e.addr, sym);
        if e.js_annotated {
            let annots = self.loaded_js_annots(file, e);
            self.mark_js_member(sym, &annots);
        }
        if f.has(tags::EXTENSION) && e.tag == tags::DEFDEF {
            let mut decoder = Decoder::new(&tasty);
            let sig = decoder.def_sig(e.addr);
            let (ext_tparams, ext_clauses) = extension_shape(&tasty, &sig);
            let mut s = self.syms.sym_mut(sym);
            s.is_extension = true;
            s.ext_tparams = ext_tparams as u8;
            s.ext_clauses = ext_clauses as u8;
            self.syms.class_mut(c).extensions.push(sym);
        } else if !self.join_value_and_method(c, sym) {
            self.register_term(Owner::Class(c), sym);
        }
        if m & (mods::IMPLICIT | mods::GIVEN) != 0 {
            self.syms.class_mut(c).givens.push(sym);
        }
    }

    /// The Scala.js annotations of a jar's definition, as `js_annots` reads them from source.
    fn loaded_js_annots(&mut self, file: u32, e: &Entry) -> super::interop::JsAnnots {
        let tasty = self.tasty(file);
        let annots = Decoder::new(&tasty).annotations(e);
        let mut out = super::interop::JsAnnots::default();
        for a in annots {
            let TType::TypeRef(prefix, n) = &a.class else { continue };
            if !is_scalajs_path(&tasty, prefix) {
                continue;
            }
            let string = |this: &mut Self, arg: Option<&AnnotArg>| match arg {
                Some(AnnotArg::Str(s)) => Some(this.lname(&tasty, *s)),
                _ => None,
            };
            match tasty.simple(*n) {
                Some("native") => out.native = true,
                Some("JSType") => out.js_type = true,
                Some("JSGlobalScope") => out.global_scope = true,
                Some("JSBracketAccess") => out.bracket = true,
                Some("JSGlobal") => out.global = Some(string(self, a.args.first())),
                Some("JSName") => match a.args.first() {
                    Some(AnnotArg::Path(TType::TermRef(prefix, m))) if matches!(&**prefix, TType::TermRef(_, s) if tasty.simple(*s) == Some("Symbol")) => {
                        out.js_name = Some(self.lname(&tasty, *m));
                        out.js_symbol = true;
                    }
                    arg => out.js_name = string(self, arg),
                },
                Some("JSExportTopLevel") => out.export = string(self, a.args.first()),
                Some("JSImport") => {
                    let Some(module) = string(self, a.args.first()) else { continue };
                    let name = match a.args.get(1) {
                        Some(AnnotArg::Str(s)) => self.lname(&tasty, *s),
                        Some(AnnotArg::Path(TType::TermRef(_, m))) => match tasty.simple(*m) {
                            Some("Default") => self.interner.intern("default"),
                            Some("Namespace") => names::STAR,
                            _ => continue,
                        },
                        _ => continue,
                    };
                    out.import = Some((module, name));
                }
                _ => {}
            }
        }
        out
    }

    /// A jar's class, trait or object annotated `@js.native` is a native JS type bound where its
    /// annotations say, one annotated `@JSType` alone a JS-object class, as the namer marks
    /// them in source.
    fn mark_loaded_js_class(&mut self, file: u32, e: &Entry, c: ClassId, name: Name) {
        let annots = self.loaded_js_annots(file, e);
        if annots.native {
            let binding = self.js_class_binding(&annots, name);
            let mut info = self.syms.class_mut(c);
            info.js = JsKind::Native;
            info.js_binding = binding;
        } else if annots.js_type {
            self.syms.class_mut(c).js = JsKind::Object;
        }
    }

    fn is_plain_case_companion(&self, module: ClassId) -> bool {
        self.syms.class(module).companion.is_some_and(|c| {
            let info = self.syms.class(c);
            info.kind == ClassKind::Class && info.mods & mods::CASE != 0 && info.mods & mods::ABSTRACT == 0
        })
    }

    /// Enters the classes of a TASTy file that a search of the class path found, as a lookup
    /// of the file's name would, and completes them and the classes nested in them: the
    /// classes of the file.
    pub(in crate::typer) fn enter_found_file(&mut self, f: CpFile) -> Vec<ClassId> {
        let loaded = self.loaded.as_ref().unwrap();
        let entry = loaded.cp.entry_name(f);
        let dir = entry.rfind('/').map_or("", |at| &entry[..at]).to_string();
        let stem = loaded.cp.stem(f).to_string();
        let mut p = ROOT_PKG;
        for seg in dir.split('/').filter(|s| !s.is_empty()) {
            let name = self.interner.intern(&package_segment(seg));
            p = self.syms.sub_pkg(p, name);
        }
        let Some(slot) = self.loaded.as_ref().and_then(|l| l.slot(p)) else { return Vec::new() };
        if stem == "package" || stem.ends_with("$package") {
            self.enter_pkg_objects(p);
        } else if !self.file_entered(f) {
            // A class that an earlier jar defines as well is that jar's.
            let first = self.loaded.as_ref().unwrap().pkg_names(slot).get(stem.as_str()).copied();
            if first != Some(f) {
                return Vec::new();
            }
            self.enter_file(f, p);
            self.enter_pkg_objects(p);
        }
        let Some(&file) = self.loaded.as_ref().unwrap().file_index.get(&f) else { return Vec::new() };
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
        done.sort_unstable();
        done
    }

    /// Of a class read from a jar: whether it carries `@EnableReflectiveInstantiation`, and
    /// whether its primary constructor is hidden from a reflective lookup.
    pub(in crate::typer) fn loaded_reflect_info(&mut self, c: ClassId) -> (bool, bool) {
        let Some(lc) = self.loaded.as_ref().and_then(|l| l.classes.get(&c).copied()) else { return (false, false) };
        if lc.value_case {
            return (false, false);
        }
        let tasty = self.tasty(lc.file);
        let sig = Decoder::new(&tasty).class_sig(lc.addr);
        let annotated = sig.mods.annots.iter().any(|a| is_reflect_annotation(&tasty, a));
        let hidden = sig.ctor.as_ref().map_or(false, |k| ctor_hidden(&k.mods));
        (annotated, hidden)
    }

    /// Of a class read from TASTy: the meta annotations of `scala.annotation.meta` it carries
    /// (`meta_annotation`), which say the symbols an annotation of the class goes on.
    pub(crate) fn loaded_meta_annotations(&mut self, c: ClassId) -> Option<u8> {
        let lc = self.loaded.as_ref().and_then(|l| l.classes.get(&c).copied())?;
        if lc.value_case {
            return Some(0);
        }
        let tasty = self.tasty(lc.file);
        let sig = Decoder::new(&tasty).class_sig(lc.addr);
        let mut bits = 0;
        for a in &sig.mods.annots {
            let TType::TypeRef(prefix, n) = a else { continue };
            let mut pkg = String::new();
            if write_path(&tasty, prefix, &mut pkg) && pkg == "scala.annotation.meta" {
                bits |= tasty.simple(*n).map_or(0, meta_annotation);
            }
        }
        Some(bits)
    }

    /// Whether a secondary constructor of a jar's class is hidden from a reflective lookup.
    pub(in crate::typer) fn loaded_ctor_hidden(&mut self, s: SymId) -> bool {
        let Some(ls) = self.loaded.as_ref().and_then(|l| l.syms.get(&s).copied()) else { return false };
        let tasty = self.tasty(ls.file);
        let sig = Decoder::new(&tasty).def_sig(ls.addr);
        ctor_hidden(&sig.mods)
    }

    /// A secondary constructor of a loaded class: an alternative of the constructor, whose
    /// signature is read when it is first tried.
    fn enter_loaded_secondary_ctor(&mut self, c: ClassId, file: u32, e: &Entry) {
        let file_id = self.syms.class(c).file;
        let m = flag_mods(e.flags);
        let sym = self.syms.new_sym(names::INIT, SymKind::Def, m, Owner::Class(c), file_id, None, Span::default());
        self.loaded_mut().syms.insert(sym, LSym { file, addr: e.addr, conversion: false });
        self.loaded_mut().tables[file as usize].terms.insert(e.addr, sym);
        self.syms.class_mut(c).ctors.push(sym);
        self.ensure_primary_ctor(c);
    }

    /// A val next to a method of its name (`val start: Int` with `def start(i: Int)` in
    /// `Regex.Match`), which the library may have: they become alternatives of one name.
    fn join_value_and_method(&mut self, c: ClassId, sym: SymId) -> bool {
        let name = self.syms.sym(sym).name;
        let Some(&old) = self.syms.class(c).members.get(&name) else { return false };
        let is_value = |t: &Self, s: SymId| matches!(t.syms.sym(s).kind, SymKind::Val | SymKind::Var);
        let is_method = |t: &Self, s: SymId| matches!(t.syms.sym(s).kind, SymKind::Def | SymKind::Overloaded(_));
        if !((is_value(self, old) && is_method(self, sym)) || (is_method(self, old) && is_value(self, sym))) {
            return false;
        }
        if let SymKind::Overloaded(i) = self.syms.sym(old).kind {
            self.syms.overloads[i as usize].1.push(sym);
            self.syms.sym_mut(sym).alternative = true;
        } else {
            let set = self.syms.new_overloaded(old, Owner::Class(c), vec![old, sym]);
            self.syms.class_mut(c).members.insert(name, set);
        }
        self.syms.class_mut(c).member_order.push(sym);
        true
    }

    // ---- signatures ----

    pub(super) fn loaded_sig(&mut self, sym: SymId) -> Option<Arc<MethodSig>> {
        let Some(ls) = self.loaded.as_ref()?.syms.get(&sym).copied() else { return self.java_sig(sym) };
        self.loaded_mut().signatures_decoded += 1;
        let tasty = self.tasty(ls.file);
        let mut decoder = Decoder::new(&tasty);
        let sig = decoder.def_sig(ls.addr);
        // Another module's class was compiled by teq, whose class files name a method by its
        // Scala name whatever its `@targetName` says.
        let product = self.loaded.as_ref().map_or(false, |l| l.cp.is_products(l.file(ls.file).cp));
        if sig.mods.volatile {
            self.loaded_mut().volatile.insert(sym, ());
        }
        if let Some(n) = sig.mods.target_name.filter(|_| product) {
            let name = self.lname(&tasty, n);
            self.loaded_mut().product_target_names.insert(sym, name);
        }
        if let Some(n) = sig.mods.target_name.filter(|_| !product) {
            let name = self.lname(&tasty, n);
            self.loaded_mut().target_names.insert(sym, name);
        } else if tasty.is_inline_accessor(sig.name) {
            // The accessor of a private member with an expanded name is `inline$q$$m` in TASTy
            // and `q$$inline$m` in the class file, the qualifier mangled first.
            let text = tasty.name(tasty.source_name(sig.name));
            if let Some((qualifier, member)) = text.strip_prefix("inline$").and_then(|rest| rest.split_once("$$")) {
                let binary = self.interner.intern(&format!("{}$$inline${}", qualifier, member));
                self.loaded_mut().target_names.insert(sym, binary);
            }
        }
        let mut cx = MapCx::new(ls.file);
        let file_id = self.syms.sym(sym).file;
        if sig.tag == tags::VALDEF {
            let ty = self.map_type(&mut cx, &sig.ret);
            return Some(Arc::new(MethodSig::value(ty)));
        }
        let is_extension = self.syms.sym(sym).is_extension;
        let name_text = tasty.name(tasty.source_name(sig.name));
        let mut clauses = sig.clauses.clone();
        if is_extension && name_text.ends_with(':') {
            let explicit: Vec<usize> = clauses
                .iter()
                .enumerate()
                .filter(|(_, c)| matches!(c, Clause::Terms(ps) if !is_using_clause(ps)))
                .map(|(i, _)| i)
                .collect();
            // dotty swaps the receiver's clause with the method's first alone, which then
            // stand next to each other (`rightAssocParams`); a using clause between them is the
            // method's first, which kept the order.
            if let [first, second, ..] = explicit[..] {
                if second == first + 1 {
                    clauses.swap(first, second);
                }
            }
        }
        let mut tparams: Vec<TParamId> = Vec::new();
        let mut pending_bounds: Vec<(TParamId, TType)> = Vec::new();
        // A secondary constructor is pickled with a copy of the class's type parameters, which
        // it means: its parameters and body are typed under the class's.
        let class_tparams: Vec<TParamId> = match self.syms.sym(sym).owner {
            Owner::Class(c) if self.syms.sym(sym).name == names::INIT => self.syms.class(c).own_tparams().to_vec(),
            _ => Vec::new(),
        };
        for clause in &clauses {
            if let Clause::Types(ps) = clause {
                for (i, tp) in ps.iter().enumerate() {
                    if let Some(&own) = class_tparams.get(i) {
                        self.loaded_mut().tables[ls.file as usize].tparams.insert(tp.addr, own);
                        continue;
                    }
                    let n = self.lname(&tasty, tp.name);
                    let id = self.syms.new_tparam(n, 0);
                    self.syms.tparams[id.idx()].arity = types::hk_arity(&tp.info);
                    self.loaded_mut().tables[ls.file as usize].tparams.insert(tp.addr, id);
                    tparams.push(id);
                    pending_bounds.push((id, tp.info.clone()));
                }
            }
        }
        for (id, info) in pending_bounds {
            self.set_tparam_bounds(&mut cx, id, &info);
        }
        if !class_tparams.is_empty() {
            tparams = class_tparams.clone();
        }
        let mut out = Vec::with_capacity(clauses.len());
        for clause in &clauses {
            let Clause::Terms(ps) = clause else { continue };
            let is_using = is_using_clause(ps);
            let mut cs = ClauseSig { params: Vec::with_capacity(ps.len()), is_using, is_implicit: is_implicit_clause(ps) };
            for p in ps {
                let name = self.lname(&tasty, p.name);
                let (ty, by_name, repeated) = self.map_param_type(&mut cx, &p.ty);
                let local_ty = if repeated {
                    match self.seq_class() {
                        Some(seq) => self.types.class(seq, &[ty]),
                        None => ERROR,
                    }
                } else {
                    ty
                };
                // A parameter of a using clause is a given inside the body, and what a body
                // passes to another using clause.
                let pm = if is_using { mods::IMPLICIT } else { 0 };
                let psym = self.syms.new_sym(name, SymKind::Param, pm, Owner::Local, file_id, None, Span::default());
                {
                    let mut s = self.syms.sym_mut(psym);
                    s.sig = Some(Arc::new(MethodSig::value(local_ty)));
                    s.state().set(Completion::Done);
                    s.by_name = by_name;
                }
                self.loaded_mut().tables[ls.file as usize].terms.insert(p.addr, psym);
                cs.params.push(ParamSig { name, ty, by_name, repeated, has_default: p.flags.has(tags::HASDEFAULT), sym: psym });
            }
            out.push(cs);
        }
        let ret = match self.syms.sym(sym).owner {
            // A constructor is pickled with the class's type parameters and a `Unit` result.
            Owner::Class(c) if self.syms.sym(sym).name == names::INIT => {
                let args: Vec<TypeId> = self.syms.class(c).tparams.clone().iter().map(|&p| self.types.param(p)).collect();
                self.types.class(c, &args)
            }
            _ => self.map_type(&mut cx, &sig.ret),
        };
        // A given class's def of another module's products as teq types the program's (the
        // namer's `DefKind::Given`): its result is the type the given was declared with, the
        // class's last parent, of the def's type parameters, where the pickle's is the class.
        let ret = match self.syms.sym(sym).impl_class.filter(|_| product && self.syms.sym(sym).kind == SymKind::Given) {
            Some(k) => self.given_class_declared(k, &tparams).unwrap_or(ret),
            None => ret,
        };
        Some(Arc::new(MethodSig { tparams, clauses: out, ret }))
    }

    /// The type a product's given class `k` was declared with, its last parent, its own type
    /// parameters those of its def, `tparams`.
    fn given_class_declared(&mut self, k: ClassId, tparams: &[TParamId]) -> Option<TypeId> {
        self.complete_class(k);
        let info = self.syms.class(k);
        let declared = *info.parents.last()?;
        let own = info.own_tparams().to_vec();
        if own.len() != tparams.len() {
            return None;
        }
        let subst: Subst = own.into_iter().zip(tparams.iter().map(|&p| self.types.param(p))).collect();
        Some(self.types.subst(declared, &subst))
    }

    /// The type of a parameter with its by-name and repeated markers taken off; `=> T*` is
    /// both, a sequence of the arguments passed by name.
    fn map_param_type(&mut self, cx: &mut MapCx, t: &TType) -> (TypeId, bool, bool) {
        match t {
            TType::ByName(inner) => match types::repeated_element(&self.tasty(cx.file), inner) {
                Some(elem) => (self.map_type(cx, elem), true, true),
                None => (self.map_type(cx, inner), true, false),
            },
            other => match types::repeated_element(&self.tasty(cx.file), other) {
                Some(elem) => (self.map_type(cx, elem), false, true),
                None => (self.map_type(cx, other), false, false),
            },
        }
    }

    /// Whether a library alias's right-hand side is a match type.
    pub(super) fn loaded_alias_is_match(&self, a: AliasId) -> bool {
        let Some(la) = self.loaded.as_ref().and_then(|l| l.aliases.get(&a).copied()) else { return false };
        let tasty = self.tasty(la.file);
        let sig = Decoder::new(&tasty).type_def_sig(la.addr);
        match &sig.rhs {
            TType::Match { .. } => true,
            TType::Lambda { result, .. } => matches!(&**result, TType::Match { .. } | TType::Lambda { kind: crate::tasty::tree::LambdaKind::Type, .. }),
            _ => false,
        }
    }

    pub(super) fn complete_loaded_alias(&mut self, a: AliasId) {
        let Some(la) = self.loaded.as_ref().and_then(|l| l.aliases.get(&a).copied()) else {
            self.syms.alias(a).state().set(Completion::Done);
            return;
        };
        let tasty = self.tasty(la.file);
        let mut decoder = Decoder::new(&tasty);
        let sig = decoder.type_def_sig(la.addr);
        let mut cx = MapCx::new(la.file);
        let (ids, rhs, bounds) = match &sig.rhs {
            TType::Lambda { kind: crate::tasty::tree::LambdaKind::Type, params, result, .. } => {
                let ids: Vec<TParamId> = params
                    .iter()
                    .map(|p| {
                        let n = self.lname(&tasty, p.name);
                        let id = self.syms.new_tparam(n, variance_of(p.flags));
                        self.syms.tparams[id.idx()].arity = types::hk_arity(&p.info);
                        self.loaded_mut().tables[la.file as usize].tparams.insert(p.addr, id);
                        id
                    })
                    .collect();
                for (p, &id) in params.iter().zip(&ids) {
                    self.set_tparam_bounds(&mut cx, id, &p.info);
                }
                // A match type names the alias itself in its cases, by these parameters.
                self.syms.aliases[a.idx()].tparams = ids.clone();
                match &**result {
                    TType::Bounds(..) => {
                        let bounds = (self.map_bound(&mut cx, &sig.rhs, false), self.map_bound(&mut cx, &sig.rhs, true));
                        (ids, ERROR, Some(bounds))
                    }
                    _ => (ids, self.map_type_ctor(&mut cx, result), None),
                }
            }
            TType::Bounds(lo, hi) => {
                let bounds = (self.map_bound(&mut cx, lo, false), self.map_bound(&mut cx, hi, true));
                (Vec::new(), ERROR, Some(bounds))
            }
            other => (Vec::new(), self.map_type_ctor(&mut cx, other), None),
        };
        let mut info = self.syms.alias_mut(a);
        info.tparams = ids;
        info.rhs = rhs;
        info.bounds = bounds;
        info.state().set(Completion::Done);
    }

    /// The Scala classes of the jars are compiled from their TASTy (`compile.rs`); a Java class
    /// that the std's platform layer does not define has no implementation on JavaScript.
    /// The export table of another module's class, from its pickle: each forwarder's name,
    /// which the `export` clause makes of a member of its qualifier, refers to that member,
    /// as the program's own export clause's names do. Types are the qualifier's classes.
    pub fn product_exports(&mut self, c: ClassId) -> Option<super::exports::ExportScope> {
        use super::{TermRef, TypeRef};
        self.complete_class(c);
        let lc = self.loaded.as_ref()?.classes.get(&c).copied()?;
        let tasty = self.tasty(lc.file);
        let mut decoder = Decoder::new(&tasty);
        let sig = decoder.class_sig(lc.addr);
        let parents: Vec<ClassId> = self.syms.class(c).parents.clone().iter().filter_map(|&p| self.class_of(p)).collect();
        // Each clause's forwarders follow it in the pickle.
        let forwarders: Vec<(Addr, Name, bool, bool, bool)> = sig
            .index
            .members
            .iter()
            .filter(|m| m.flags.has(tags::EXPORTED))
            .map(|m| (m.addr, self.lname(&tasty, m.name), m.tag == tags::TYPEDEF, m.flags.has(tags::EXTENSION), m.flags.has(tags::GIVEN)))
            .collect();
        let mut scope = super::exports::ExportScope::default();
        // A class inherits its parents' exports, whose forwarders are their members.
        for p in parents {
            if let Some(inherited) = self.exports_of(p) {
                scope.inherit(&inherited, &self.syms);
            }
        }
        // The type forwarders' own aliases, by name, which the loader entered aside.
        let forwarder_aliases: FxMap<Name, AliasId> = forwarders
            .iter()
            .filter(|f| f.2)
            .filter_map(|&(at, n, ..)| self.file_tables(lc.file).aliases.get(&at).map(|&a| (n, a)))
            .collect();
        let clauses = sig.index.exports.clone();
        for (i, &addr) in clauses.iter().enumerate() {
            let next = clauses.get(i + 1).copied().unwrap_or(Addr::MAX);
            let clause = Decoder::new(&tasty).export_sig(addr);
            let selectors: Vec<(Name, Option<Name>)> = clause
                .selectors
                .iter()
                .map(|&(n, r)| (self.lname(&tasty, n), r.map(|r| self.lname(&tasty, r))))
                .collect();
            let wildcard = self.interner.intern("_");
            // A `given` selector is the empty name, which takes the clause's given forwarders.
            let given_selector = self.interner.intern("");
            let original_of = |fwd: Name, is_given: bool| {
                selectors
                    .iter()
                    .find_map(|&(n, r)| match r {
                        Some(to) if to == fwd => Some(n),
                        None if n == fwd => Some(n),
                        _ => None,
                    })
                    .or_else(|| selectors.iter().any(|&(n, _)| n == wildcard || is_given && n == given_selector).then_some(fwd))
            };
            let own: Vec<(Name, Name, bool, bool)> = forwarders
                .iter()
                .filter(|&&(at, ..)| at > addr && at < next)
                .filter_map(|&(_, fwd, is_type, is_ext, is_given)| original_of(fwd, is_given).map(|o| (fwd, o, is_type, is_ext)))
                .collect();
            // An export from a package: its members are the package's entries.
            if let Some(p) = self.product_package_of(&tasty, &clause.path) {
                for (fwd, original, is_type, is_ext) in own {
                    self.load_pkg_member(p, original);
                    let Some(entry) = self.syms.pkg(p).entries.get(&original).cloned() else {
                        // A type of the package's package object, of a jar's Scala 2 one.
                        if let Some(&a) = forwarder_aliases.get(&fwd).filter(|_| is_type) {
                            scope.types.insert(fwd, TypeRef::Alias(a));
                        }
                        continue;
                    };
                    if is_type {
                        if let Some(k) = entry.class {
                            scope.types.insert(fwd, TypeRef::Class(k));
                        } else if let Some(a) = entry.alias {
                            scope.types.insert(fwd, TypeRef::Alias(a));
                        } else if let Some(&a) = forwarder_aliases.get(&fwd) {
                            scope.types.insert(fwd, TypeRef::Alias(a));
                        }
                    } else if is_ext {
                        scope.extensions.insert(fwd, entry.extensions.clone());
                    } else if let Some(t) = entry.term {
                        // A top-level definition read from TASTy is a member of its file's
                        // package object, as a package lookup finds it.
                        let r = match self.syms.sym(t).owner {
                            Owner::Class(k) => TermRef::ModuleMember(k, t),
                            _ => TermRef::Global(t),
                        };
                        scope.terms.insert(fwd, r);
                        if self.syms.is_given(t) {
                            scope.givens.push(t);
                        }
                    } else if let Some(k) = entry.class {
                        scope.terms.insert(fwd, TermRef::Class(k));
                    }
                }
                continue;
            }
            let mut cx = MapCx::new(lc.file);
            let qualifier = self.map_type(&mut cx, &clause.path);
            let Type::Class(obj, _) = self.types.get(qualifier) else { continue };
            self.complete_class(obj);
            // What the qualifier itself exports, which its members do not hold: a forwarder of a
            // forwarder.
            let exported = self.exports_of(obj);
            for (fwd, original, is_type, is_ext) in own {
                let info = (*self.syms.class(obj)).clone();
                if is_type {
                    if let Some(&k) = info.nested.get(&original) {
                        scope.types.insert(fwd, TypeRef::Class(k));
                    } else if let Some(&a) = info.type_aliases.get(&original) {
                        scope.types.insert(fwd, TypeRef::Alias(a));
                    } else if let Some(&t) = exported.as_ref().and_then(|e| e.types.get(&original)) {
                        scope.types.insert(fwd, t);
                    } else if let Some(t) = self.class_scope_type(obj, original) {
                        scope.types.insert(fwd, t);
                    } else if let Some(&a) = forwarder_aliases.get(&fwd) {
                        scope.types.insert(fwd, TypeRef::Alias(a));
                    }
                    continue;
                }
                let found = if is_ext { None } else { info.members.get(&original).copied().or_else(|| self.find_member(qualifier, original).map(|(m, _)| m)) };
                if let Some(m) = found {
                    scope.terms.insert(fwd, TermRef::ModuleMember(obj, m));
                    if self.syms.is_given(m) {
                        scope.givens.push(m);
                    }
                    continue;
                }
                let ext: Vec<SymId> = info.extensions.iter().copied().filter(|&x| self.syms.sym(x).name == original).collect();
                if !ext.is_empty() {
                    scope.extensions.insert(fwd, ext);
                    continue;
                }
                let Some(e) = exported.as_ref() else { continue };
                if let Some(&r) = e.terms.get(&original) {
                    scope.terms.insert(fwd, r);
                    if let Some(g) = r.sym().filter(|&g| e.givens.contains(&g)) {
                        scope.givens.push(g);
                    }
                }
                if let Some(x) = e.extensions.get(&original) {
                    scope.extensions.insert(fwd, x.clone());
                }
            }
        }
        let empty = scope.terms.is_empty() && scope.types.is_empty() && scope.extensions.is_empty() && scope.givens.is_empty();
        (!empty).then_some(scope)
    }

    pub fn report_reached_library(&mut self, reach: &crate::emit::reach::Reach) {
        let Some(loaded) = &self.loaded else { return };
        let mut java_reached: Vec<(FileId, String)> = Vec::new();
        // The JVM target calls Java classes as the jars and the JDK hold them.
        let java = !self.jvm;
        for &s in &reach.asked {
            if java && loaded.java.syms.contains_key(&s) {
                let info = self.syms.sym(s);
                let owner = match info.owner {
                    Owner::Class(c) => format!(" of {}", self.class_path(c)),
                    _ => String::new(),
                };
                java_reached.push((info.file, format!("{}{}", self.member_description(s, None), owner)));
            }
        }
        // A class of the platform layer that absorbed the JDK's members is implemented; a class
        // that stands in for a Java one is not.
        for (&c, _) in loaded.java.classes.iter().filter(|_| java) {
            let info = self.syms.class(c);
            if reach.classes.get(c.idx()).copied().unwrap_or(false) && info.kind != ClassKind::Builtin && info.def.is_none() {
                java_reached.push((info.file, format!("{} {}", self.class_description(c), self.class_path(c))));
            }
        }
        java_reached.sort_by(|a, b| a.1.cmp(&b.1));
        java_reached.dedup();
        for (file, what) in java_reached {
            self.diags.error(file, Span::default(), format!("not supported on JavaScript: {} (a Java definition without an implementation in the std's platform layer)", what));
        }
    }

    /// Reports a type of an unsupported shape met at a use site.
    pub fn report_blocked_type(&mut self, t: TypeId, span: Span) {
        if let Some(b) = types::first_blocked(&self.types, t) {
            let msg = format!("not supported yet: {}", self.types.blocked_description(b));
            self.error(span, msg);
        }
    }

    /// Reports a member whose signature holds a type of an unsupported shape, at its use site.
    pub fn report_blocked_sig(&mut self, sym: SymId, span: Span) -> bool {
        let Some(b) = self.blocked_in_sig(sym) else { return false };
        let description = self.types.blocked_description(b).to_string();
        let msg = format!("not supported yet: {}", description);
        self.error(span, msg);
        true
    }

    /// The first type of an unsupported shape in the signature of `sym`.
    pub fn blocked_in_sig(&mut self, sym: SymId) -> Option<BlockedId> {
        let sig = self.syms.sym(sym).info.sig.as_deref()?;
        let mut found: Option<BlockedId> = None;
        let mut visit = |t: TypeId, types: &TypeStore| {
            if found.is_none() {
                found = types::first_blocked(types, t);
            }
        };
        visit(sig.ret, &self.types);
        for c in &sig.clauses {
            for p in &c.params {
                visit(p.ty, &self.types);
            }
        }
        for &tp in &sig.tparams {
            let info = self.syms.tparam(tp);
            visit(info.upper, &self.types);
            visit(info.lower, &self.types);
        }
        found
    }
}

/// `teq tasty --load`: enters every class of the jars, completes each and decodes every
/// member signature, so that a decoder or mapping defect shows on the whole jar rather than
/// on the program that happens to reach it.
pub fn load_everything(paths: &[String]) -> Result<String, String> {
    let start = std::time::Instant::now();
    with_everything_loaded(paths, |typer| {
        let elapsed = start.elapsed();
        let errors = typer.diags.error_count();
        let mut out = format!("{} classes, {} symbols, {} errors in {:?}\n", typer.syms.classes.len(), typer.syms.syms.len(), errors, elapsed);
        let mut report = crate::report::Report::default();
        typer.loaded.as_ref().unwrap().report(&mut report, |c| typer.class_path(c));
        out.push_str(&report.render());
        if errors > 0 {
            out.push('\n');
            out.push_str(&typer.diags.render(typer.files.as_slice()));
        }
        out
    })
}

impl<'a> Worker<'a> {
    /// The objects of the class path's product directories, each with its directory, every file
    /// of them entered first: what `TEQ_PRODUCT_INITS` derives the directories' `inits` over.
    pub fn product_objects(&mut self) -> Vec<(String, ClassId)> {
        let Some(loaded) = self.loaded.as_ref() else { return Vec::new() };
        let n = loaded.cp.packages.len();
        for slot in 0..n {
            let (path, files) = {
                let l = self.loaded.as_ref().unwrap();
                let files: Vec<CpFile> = l.cp.packages[slot].files.iter().copied().filter(|&f| l.cp.is_products(f)).collect();
                (l.cp.packages[slot].path.clone(), files)
            };
            if files.is_empty() {
                continue;
            }
            let mut p = ROOT_PKG;
            for seg in path.split('/').filter(|s| !s.is_empty()) {
                let name = self.interner.intern(&package_segment(seg));
                p = self.syms.sub_pkg(p, name);
            }
            for f in files {
                if self.file_entered(f) {
                    continue;
                }
                let stem = self.loaded.as_ref().unwrap().cp.stem(f).to_string();
                if stem == "package" || stem.ends_with("$package") {
                    self.enter_package_object(f, p);
                } else {
                    self.enter_file(f, p);
                }
            }
        }
        let loaded = self.loaded.as_ref().unwrap();
        let mut out: Vec<(String, ClassId)> = loaded
            .classes
            .iter()
            .filter(|(&c, _)| loaded.is_product_class(c) && !loaded.is_synthetic_product_class(c) && self.syms.class(c).kind == ClassKind::Object)
            .map(|(&c, lc)| (loaded.cp.paths[loaded.file(lc.file).cp.jar as usize].clone(), c))
            .collect();
        out.sort();
        out
    }
}

/// The index of the source of `key` and `token` among a product's sources, entered if new.
fn source_index(sources: &SlabVec<(String, u64)>, by_token: &mut FxMap<u64, Vec<u32>>, key: &str, token: u64) -> u32 {
    let same = by_token.entry(token).or_default();
    if let Some(&i) = same.iter().find(|&&i| sources[i as usize].0 == key) {
        return i;
    }
    sources.push((key.to_string(), token));
    same.push(sources.len() as u32 - 1);
    sources.len() as u32 - 1
}

impl Loaded {
    /// The index of a product's source of `key` and `token`, entered if new.
    pub fn product_source_index(&mut self, key: &str, token: u64) -> u32 {
        source_index(&self.product_sources, &mut self.product_sources_by_token, key, token)
    }

    /// Records the source of a product's method an expansion replayed expands, from the place
    /// its record gives (kind 8): the source of `tasty_file`'s origins or one they name.
    pub fn note_callee_source(&mut self, callee: SymId, tasty_file: u32, at: crate::tasty::origins::Place) {
        let Loaded { files, product_sources, product_sources_by_token, product_callee_sources, .. } = self;
        let crate::tasty::origins::Found::Read(o) = &files[tasty_file as usize].provenance else { return };
        let (key, token) = match at.0 {
            0 => (o.key.as_str(), o.token),
            r => match o.sources.get(r as usize - 1) {
                Some((key, token)) => (key.as_str(), *token),
                None => return,
            },
        };
        let s = source_index(product_sources, product_sources_by_token, key, token);
        product_callee_sources.insert(callee, s);
    }

    /// The source and offset of the definition at `addr` of the product file `tasty_file`, its
    /// point (kind 1).
    pub fn definition_place(&mut self, tasty_file: u32, addr: Addr) -> Option<(u32, u32)> {
        let Loaded { files, product_sources, product_sources_by_token, .. } = self;
        let crate::tasty::origins::Found::Read(o) = &files[tasty_file as usize].provenance else { return None };
        let i = o.definitions.binary_search_by_key(&addr, |&(a, _)| a).ok()?;
        let own = source_index(product_sources, product_sources_by_token, &o.key, o.token);
        Some((own, o.definitions[i].1))
    }

    /// Places the definition `d` of the pseudo file `file` where the tree at `addr` of the
    /// product file `tasty_file` stands, its point.
    pub fn place_def_at(&mut self, file: FileId, d: crate::ast::DefId, tasty_file: u32, addr: Addr) {
        let Loaded { files, product_sources, product_sources_by_token, product_places, .. } = self;
        let crate::tasty::origins::Found::Read(o) = &files[tasty_file as usize].provenance else { return };
        let Ok(i) = o.definitions.binary_search_by_key(&addr, |&(a, _)| a) else { return };
        let own = source_index(product_sources, product_sources_by_token, &o.key, o.token);
        product_places.replace((file, d), ProductPlace { source: own, offset: o.definitions[i].1, name: None });
    }

    /// Places the definitions of a class converted from the product file `tasty_file` into the
    /// pseudo file `file`, each by its tree's address in the file's origins: an anonymous
    /// class by its kind 2 record, a named local class by its kind 4 record, any other
    /// definition by its point (kind 1), the offset the compiler orders and names a definition
    /// by (`SymInfo::span`, `ClassInfo::span`). A pickle without the origins places nothing.
    pub fn place_product_defs(&mut self, file: FileId, tasty_file: u32, defs: &[(crate::ast::DefId, Addr)]) {
        use crate::tasty::origins::{self, ClassOrigin, Found};
        let Loaded { files, product_sources, product_sources_by_token, product_places, product_files, product_def_trees, .. } = self;
        let Found::Read(o) = &files[tasty_file as usize].provenance else { return };
        product_def_trees.extend(defs.iter().map(|&(d, addr)| ((file, d), (tasty_file, addr))));
        let mut source_of = |key: &str, token: u64| source_index(product_sources, product_sources_by_token, key, token);
        let own = source_of(&o.key, o.token);
        product_files.insert(file, own);
        for &(d, addr) in defs {
            let place = match o.classes.binary_search_by_key(&addr, |&(a, _)| a) {
                Ok(i) => {
                    let origin = &o.classes[i].1;
                    let (at, name) = match origin {
                        ClassOrigin::Anonymous { at, .. } => (*at, origins::class_name(o, origin, "")),
                        ClassOrigin::Local { at } => (*at, None),
                    };
                    let source = match at.0 {
                        0 => Some(own),
                        r => o.sources.get(r as usize - 1).map(|(key, token)| source_of(key, *token)),
                    };
                    source.map(|s| ProductPlace { source: s, offset: at.1, name })
                }
                Err(_) => o.definitions.binary_search_by_key(&addr, |&(a, _)| a).ok().map(|i| ProductPlace { source: own, offset: o.definitions[i].1, name: None }),
            };
            if let Some(p) = place {
                product_places.replace((file, d), p);
            }
        }
    }
}

/// A typer over the jars alone with every class entered and completed and every signature
/// decoded, handed to `f` for a report.
pub fn with_everything_loaded(paths: &[String], f: impl FnOnce(&mut Worker) -> String) -> Result<String, String> {
    use crate::ast::{Ast, Asts};
    use crate::source::{SourceFile, Sources};
    let mut interner = crate::intern::Interner::new();
    let mut files: Vec<SourceFile> =
        paths.iter().map(|p| SourceFile { path: p.clone(), text: String::new(), is_std: true, key: crate::file_name(p).to_string() }).collect();
    files.push(SourceFile { path: "<jdk>".to_string(), text: String::new(), is_std: true, key: "<jdk>".to_string() });
    let asts = Asts::new(files.iter().map(|_| Ast::new(0)).collect());
    let sources = Sources::new(files);
    let mut typer = Worker::new(&asts, &sources, &mut interner);
    let cp = Classpath::open(paths, None, None)?;
    let jar_files: Vec<FileId> = (0..paths.len()).map(|i| FileId(i as u32)).collect();
    typer.set_classpath(cp, jar_files, FileId(paths.len() as u32), None, crate::StdMode::ScalaLibrary);
    typer.env.file = FileId(0);
    let n = typer.loaded.as_ref().unwrap().cp.packages.len();
    for slot in 0..n {
        let (path, mut files) = {
            let l = typer.loaded.as_ref().unwrap();
            (l.cp.packages[slot].path.clone(), l.cp.packages[slot].files.clone())
        };
        let classes = typer.loaded.as_ref().unwrap().cp.packages[slot].classes.clone();
        for f in classes {
            let cp = &mut typer.loaded_mut().cp;
            if cp.is_scala2(f) && !cp.stem(f).contains('$') && cp.class_file(f).map_or(false, |cf| cf.scala_sig.is_some()) {
                files.push(f);
            }
        }
        let mut p = ROOT_PKG;
        for seg in path.split('/').filter(|s| !s.is_empty()) {
            let name = typer.interner.intern(&package_segment(seg));
            p = typer.syms.sub_pkg(p, name);
        }
        let mut seen = std::collections::HashSet::new();
        for f in files {
            let stem = typer.loaded.as_ref().unwrap().cp.stem(f).to_string();
            if !seen.insert(stem.clone()) {
                continue;
            }
            if stem == "package" || stem.ends_with("$package") {
                typer.enter_package_object(f, p);
            } else if !typer.file_entered(f) {
                typer.enter_file(f, p);
            }
        }
    }
    let mut done = 0;
    while done < typer.syms.classes.len() {
        let c = ClassId(done as u32);
        typer.complete_class(c);
        done += 1;
    }
    let mut sig_done = 0;
    while sig_done < typer.syms.syms.len() {
        let s = SymId(sig_done as u32);
        if typer.loaded.as_ref().unwrap().syms.contains_key(&s) {
            typer.sig_of(s);
        }
        sig_done += 1;
    }
    Ok(f(&mut typer))
}

/// How many leading type parameters and parameter clauses of an extension method belong to
/// the extension: the receiver's clause and the using clauses right after it.
fn extension_shape(tasty: &TastyFile, sig: &DefSig) -> (usize, usize) {
    let right_assoc = tasty.name(tasty.source_name(sig.name)).ends_with(':');
    let mut clauses: Vec<&Clause> = sig.clauses.iter().collect();
    if right_assoc {
        let explicit: Vec<usize> =
            clauses.iter().enumerate().filter(|(_, c)| matches!(c, Clause::Terms(ps) if !is_using_clause(ps))).map(|(i, _)| i).collect();
        if let [first, second, ..] = explicit[..] {
            if second == first + 1 {
                clauses.swap(first, second);
            }
        }
    }
    let mut tparams = 0;
    let mut term_clauses = 0;
    let mut seen_receiver = false;
    for c in clauses {
        match c {
            Clause::Types(ps) if !seen_receiver => tparams += ps.len(),
            Clause::Types(_) => break,
            // A using clause before the receiver (`extension (using q: Quotes)(self: q.reflect.Term)`)
            // is the extension's, and the receiver is the first clause that is not one.
            Clause::Terms(ps) => {
                let using = is_using_clause(ps);
                if seen_receiver && !using {
                    break;
                }
                term_clauses += 1;
                seen_receiver |= !using;
            }
        }
    }
    (tparams, term_clauses)
}

impl<'a> Worker<'a> {
    /// The package a product pickle's export clause names, `a.b` as a `TERMREFpkg`.
    fn product_package_of(&mut self, tasty: &TastyFile, path: &TType) -> Option<PkgId> {
        let TType::Package(n) = path else { return None };
        let text = tasty.name(*n);
        let mut pkg = ROOT_PKG;
        for seg in text.split('.').filter(|s| !s.is_empty() && !matches!(*s, "_root_" | "<root>" | "<empty>")) {
            let name = self.interner.intern(seg);
            pkg = self.syms.pkg(pkg).entries.get(&name).and_then(|e| e.pkg)?;
        }
        Some(pkg)
    }
}

/// A package's name from its directory's: a symbolic package is stored encoded (`$plus` for
/// `+`), as scalac writes it and reads it back.
fn package_segment(seg: &str) -> std::borrow::Cow<'_, str> {
    if seg.contains('$') {
        std::borrow::Cow::Owned(crate::jvm::names::decode(seg))
    } else {
        std::borrow::Cow::Borrowed(seg)
    }
}

/// The member a super accessor `super$a$b$C$$m` calls: `m`.
fn super_accessor_member(tasty: &TastyFile, n: crate::tasty::NameRef) -> Option<String> {
    let crate::tasty::TName::SuperAccessor(u) = tasty.names.get(n as usize)? else { return None };
    match tasty.names.get(*u as usize)? {
        crate::tasty::TName::Expanded(_, m) => Some(tasty.name(*m)),
        _ => Some(tasty.name(*u)),
    }
}

fn strip_lambda(t: &TType) -> &TType {
    match t {
        TType::Lambda { kind: crate::tasty::tree::LambdaKind::Type, result, .. } => strip_lambda(result),
        other => other,
    }
}

/// Whether the class at `addr` has `scala.AnyVal` among its parents: a value class, known when
/// the class is entered, since erasure asks it of every class a signature names, completed or
/// not.
fn extends_any_val(tasty: &TastyFile, addr: Addr) -> bool {
    Decoder::new(tasty).class_parents(addr).iter().any(|p| {
        let TType::TypeRef(prefix, n) = p else { return false };
        let mut s = String::new();
        tasty.simple(*n) == Some("AnyVal") && write_path(tasty, prefix, &mut s) && s == "scala"
    })
}

/// Whether a TASTy prefix is `scala.scalajs.js` or a package or object inside it.
pub(super) fn is_scalajs_path(tasty: &TastyFile, t: &TType) -> bool {
    let mut s = String::new();
    write_path(tasty, t, &mut s) && (s == "scala.scalajs.js" || s.starts_with("scala.scalajs.js."))
}

/// Scala.js registers the constructors that are neither protected nor private; a `private[p]` one
/// counts, a `protected[p]` one does not.
fn ctor_hidden(m: &crate::tasty::tree::Mods) -> bool {
    m.flags.has(tags::PROTECTED) || (m.flags.has(tags::PRIVATE) && m.within.is_none())
}

/// The bit of a meta annotation of `scala.annotation.meta` by its name: scalac's
/// `NonBeanMetaAnnots`, which `PostTyper` keeps an annotation on a field, an accessor, a
/// parameter or a setter by.
pub(crate) fn meta_annotation(name: &str) -> u8 {
    match name {
        "field" => META_FIELD,
        "getter" => META_GETTER,
        "setter" => META_SETTER,
        "param" => META_PARAM,
        "companionClass" => META_COMPANION_CLASS,
        "companionMethod" => META_COMPANION_METHOD,
        "beanGetter" | "beanSetter" => META_BEAN,
        _ => 0,
    }
}

pub(crate) const META_FIELD: u8 = 1;
pub(crate) const META_GETTER: u8 = 2;
pub(crate) const META_SETTER: u8 = 4;
pub(crate) const META_PARAM: u8 = 8;
pub(crate) const META_COMPANION_CLASS: u8 = 16;
pub(crate) const META_COMPANION_METHOD: u8 = 32;
pub(crate) const META_BEAN: u8 = 64;
/// scalac's `NonBeanMetaAnnots`.
pub(crate) const META_NON_BEAN: u8 = 63;
/// scalac's `MetaAnnots`, the bean getter's and setter's with them.
pub(crate) const META_ALL: u8 = 127;

fn is_reflect_annotation(tasty: &TastyFile, t: &TType) -> bool {
    let TType::TypeRef(prefix, n) = t else { return false };
    let mut s = String::new();
    tasty.simple(*n) == Some("EnableReflectiveInstantiation") && write_path(tasty, prefix, &mut s) && s == "scala.scalajs.reflect.annotation"
}

fn write_path(tasty: &TastyFile, t: &TType, out: &mut String) -> bool {
    fn path(tasty: &TastyFile, t: &TType, out: &mut String) -> bool {
        match t {
            TType::Package(n) => {
                tasty.write_name(*n, out);
                true
            }
            TType::This(inner) => path(tasty, inner, out),
            TType::TermRef(p, n) | TType::TypeRef(p, n) => {
                if !path(tasty, p, out) {
                    return false;
                }
                out.push('.');
                tasty.write_name(tasty.source_name(*n), out);
                true
            }
            _ => false,
        }
    }
    path(tasty, t, out)
}

/// The variance a type parameter's flags declare: 1 for `+`, -1 for `-`.
fn variance_of(flags: crate::tasty::tree::Flags) -> i8 {
    if flags.has(tags::COVARIANT) {
        1
    } else if flags.has(tags::CONTRAVARIANT) {
        -1
    } else {
        0
    }
}
