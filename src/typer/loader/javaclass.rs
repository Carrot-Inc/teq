//! Java classes, read from the class files of the classpath's jars and directories and from
//! the JDK's `lib/ct.sym`, entered as the TASTy loader enters Scala classes: a class on the
//! first lookup that names it, its members when it completes, a member's signature when it
//! is first asked for. A class that a library signature names becomes a placeholder, a trait
//! without members, and is read when a member is selected from it, when it is constructed or
//! extended, or when its nested classes are looked up. The JDK is opened on the first `java.*`
//! lookup of the program that no jar and no std file answers.

use super::Worker;
use crate::arena::SharedMap;
use crate::ast::mods;
use crate::classfile::jdk::{self, CtSym};
use crate::classfile::sig::{self, ClassType, JType, TypeArg, TypeParam};
use crate::classfile::{ClassFile, ACC_ABSTRACT, ACC_BRIDGE, ACC_ENUM, ACC_FINAL, ACC_INTERFACE, ACC_PRIVATE, ACC_PROTECTED, ACC_STATIC, ACC_SYNTHETIC, ACC_VARARGS};
use crate::classpath::CpFile;
use crate::intern::{FxMap, Name};
use crate::names;
use crate::source::{FileId, Span};
use crate::symbols::*;
use crate::types::*;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// The packages `ct.sym` holds classes under: the lookups worth opening it for.
const JDK_ROOTS: &[&str] = &["java", "javax", "jdk", "sun", "com.sun", "org.w3c", "org.xml", "org.ietf", "netscape.javascript"];

const MAX_NESTING: u32 = 8;

pub struct Java {
    jdk: Option<Jdk>,
    pub release: Option<u32>,
    /// The pseudo file that stands for the JDK in diagnostics.
    pub jdk_file: FileId,
    jdk_missing: bool,
    /// The packages of the JDK by their id, once it is open, with their index in `Jdk::packages`.
    jdk_pkgs: FxMap<PkgId, u32>,
    /// The Java classes entered, read outside the loader's lock (`Worker::is_java_class`).
    pub classes: SharedMap<ClassId, JClass>,
    /// The members entered: the lock holder's.
    pub syms: FxMap<SymId, JSym>,
    /// The classes a signature named, each with its place until something reads it, `None` from
    /// then: read outside the loader's lock (`Java::placeholder`).
    placeholders: SharedMap<ClassId, Option<(PkgId, Name)>>,
    /// Every class that was a placeholder, marked as its entry is made: the subtype check asks
    /// `placeholder` of nearly every class, and a class without its mark is refused at one bit.
    placeholder_ids: crate::arena::SharedBits,
    builtin_members: [bool; 2],
    /// The std classes whose JDK constructors `absorb_java_ctors` has looked at, and the
    /// constructors it added.
    ctors_absorbed: FxMap<ClassId, ()>,
    absorbed_ctors: SharedMap<SymId, ()>,
    /// `jdk_rooted` per package, which the subtype check asks of every unread JDK class.
    rooted: crate::arena::SharedMap<PkgId, bool>,
    /// The classes whose JDK members are absorbed, the class and its companion, or known to
    /// have none with the JDK open; the classes whose JDK constructors were looked at; the
    /// builtins whose Java members are entered: each published by the hold that did it
    /// (`Worker::loader_done`), for the lookups outside the loader's lock.
    pub absorbed_done: crate::arena::SharedMap<ClassId, ()>,
    pub ctors_done: crate::arena::SharedMap<ClassId, ()>,
    pub builtin_done: [std::sync::atomic::AtomicBool; 2],
    pub stats: JavaStats,
}

#[derive(Default)]
pub struct JavaStats {
    pub ct_open: Option<Duration>,
    pub ct_index: Duration,
    pub release: u32,
    pub jdk_files_parsed: usize,
    pub jdk_bytes: usize,
    pub jdk_parse_time: Duration,
    pub classes_entered: usize,
    pub signatures_decoded: usize,
}

struct Jdk {
    ct: CtSym,
    /// Per package, its path and the entries of its classes.
    packages: Vec<(String, Vec<u32>)>,
    /// Per package index, its classes by simple name, built on the first lookup.
    names: FxMap<u32, FxMap<Box<str>, u32>>,
    parsed: FxMap<u32, Arc<ClassFile>>,
}

#[derive(Clone, Copy)]
pub enum JSource {
    Cp(CpFile),
    Jdk(u32),
    /// A class file compiled into teq: the JDK's `String.sig` and `Object.sig`, whose members
    /// the builtins get without `ct.sym`.
    Embedded,
}

const STRING_SIG: &[u8] = include_bytes!("../../../std/java/String.sig");
const OBJECT_SIG: &[u8] = include_bytes!("../../../std/java/Object.sig");

pub struct JClass {
    pub source: JSource,
    pub cf: Arc<ClassFile>,
    /// For the object that holds the static members: the class.
    pub companion_of: Option<ClassId>,
    /// A non-static inner class, whose instances need an outer one.
    pub inner: bool,
}

#[derive(Clone, Copy)]
pub enum JMember {
    Method(u32),
    Field(u32),
}

#[derive(Clone, Copy)]
pub struct JSym {
    pub class: ClassId,
    pub member: JMember,
}

/// The type variables in scope while a signature is mapped.
struct JCx {
    class: Vec<(String, TParamId)>,
    method: Vec<(String, TParamId)>,
}

impl Java {
    pub fn new(jdk_file: FileId, release: Option<u32>) -> Java {
        Java {
            jdk: None,
            release,
            jdk_file,
            jdk_missing: false,
            jdk_pkgs: FxMap::default(),
            classes: SharedMap::new(),
            syms: FxMap::default(),
            placeholders: SharedMap::new(),
            placeholder_ids: crate::arena::SharedBits::new(),
            builtin_members: [false; 2],
            ctors_absorbed: FxMap::default(),
            absorbed_ctors: SharedMap::new(),
            rooted: Default::default(),
            absorbed_done: Default::default(),
            ctors_done: Default::default(),
            builtin_done: Default::default(),
            stats: JavaStats::default(),
        }
    }

    pub fn jdk_open(&self) -> bool {
        self.jdk.is_some()
    }

    /// Whether the JDK was opened or a Java class of the classpath entered or named.
    pub fn read_anything(&self) -> bool {
        self.stats.ct_open.is_some() || self.stats.classes_entered > 0 || self.unread().next().is_some()
    }

    /// The package and name of the class `c` while it is a placeholder: named by a signature, not
    /// read.
    #[inline]
    pub fn placeholder(&self, c: ClassId) -> Option<(PkgId, Name)> {
        if !self.placeholder_ids.has(c.0) {
            return None;
        }
        self.placeholders.get(&c).copied().flatten()
    }

    /// Makes `c` a placeholder for the class `name` of package `p`: the loader's lock holder's.
    pub(super) fn add_placeholder(&self, c: ClassId, p: PkgId, name: Name) {
        self.placeholders.insert(c, Some((p, name)));
        self.placeholder_ids.set(c.0);
    }

    /// `c`, a placeholder, has been read: the loader's lock holder's.
    fn placeholder_read(&self, c: ClassId) {
        self.placeholders.replace(c, None);
    }

    /// The placeholders nothing has read, each once: a placeholder's first entry is its only one
    /// until it is read.
    pub fn unread(&self) -> impl Iterator<Item = ClassId> + '_ {
        self.placeholders.keys().copied().filter(|&c| self.placeholder(c).is_some())
    }

    pub fn report(&self, section: &mut crate::report::Section) {
        let s = &self.stats;
        if let Some(open) = s.ct_open {
            section.row("JDK ct.sym opened").time(open).note(format!("release {}, package index {:.2} ms", s.release, s.ct_index.as_secs_f64() * 1000.0));
            section.row("JDK class files parsed").count(s.jdk_files_parsed).time(s.jdk_parse_time).note(crate::report::bytes(s.jdk_bytes as u64));
        }
        section.row("classes entered").count(s.classes_entered);
        section.row("signatures decoded").count(s.signatures_decoded);
        let unread = self.unread().count();
        if unread > 0 {
            section.row("classes named but not read").count(unread);
        }
    }
}

/// The members a program may name: package-private ones too, which scalac takes inside the
/// package and which `ct.sym` leaves out anyway.
fn visible(access: u16) -> bool {
    access & (ACC_PRIVATE | ACC_SYNTHETIC | ACC_BRIDGE) == 0
}

/// The package path and the simple name of a binary class name: `java/util/Map$Entry` is
/// `java/util` and `Map$Entry`.
fn split_binary(name: &str) -> (&str, &str) {
    match name.rfind('/') {
        Some(at) => (&name[..at], &name[at + 1..]),
        None => ("", name),
    }
}

impl<'a> Worker<'a> {
    fn java(&self) -> &Java {
        &self.loaded.as_ref().unwrap().java
    }

    fn java_mut(&mut self) -> &mut Java {
        &mut self.loaded_mut().java
    }

    pub fn is_java_placeholder(&self, c: ClassId) -> bool {
        self.loaded.as_ref().map_or(false, |l| l.java.placeholder(c).is_some())
    }

    pub fn is_java_class(&self, c: ClassId) -> bool {
        self.loaded.as_ref().map_or(false, |l| l.java.classes.contains_key(&c))
    }

    /// Whether `c` is a JDK class nothing has read yet and `target` no Java class: a class of
    /// the JDK has no Scala ancestor, so the subtype check needs no reading. The std's stand-ins
    /// for JDK classes (`java.lang.AutoCloseable`, which `FileLock` implements) are ancestors.
    pub fn jdk_class_unrelated(&self, c: ClassId, target: ClassId) -> bool {
        let loaded = self.loaded.as_ref().unwrap();
        let Some((p, _)) = loaded.java.placeholder(c) else { return false };
        if loaded.slot(p).is_some() || !self.jdk_rooted(p) {
            return false;
        }
        target != self.b.any_ref && !self.is_java_class(target) && !self.is_java_placeholder(target) && !self.jdk_rooted(self.package_of_class(target))
    }

    /// Reports the construction of a Java inner class, whose instances need an outer one.
    pub fn report_java_inner(&mut self, c: ClassId, span: Span) -> bool {
        let inner = self.loaded.as_ref().and_then(|l| l.java.classes.get(&c)).map_or(false, |j| j.inner && j.companion_of.is_none());
        if inner {
            let msg = format!("not supported yet: inner class {} needs an outer instance (Outer.Inner)", self.class_path(c));
            self.error(span, msg);
        }
        inner
    }

    fn java_file(&self, source: JSource) -> FileId {
        let loaded = self.loaded.as_ref().unwrap();
        match source {
            JSource::Cp(f) => loaded.jar_files[f.jar as usize],
            JSource::Jdk(_) | JSource::Embedded => loaded.java.jdk_file,
        }
    }

    /// The package's path as a Java name, `java.util`, with `member` appended.
    fn dotted(&self, p: PkgId, member: Option<&str>) -> String {
        let mut segments = Vec::new();
        let mut p = p;
        while p != ROOT_PKG {
            let info = self.syms.pkg(p);
            segments.push(self.interner.get(info.name).to_string());
            match info.parent {
                Some(parent) => p = parent,
                None => break,
            }
        }
        segments.reverse();
        if let Some(m) = member {
            segments.push(m.to_string());
        }
        segments.join(".")
    }

    /// Whether `p` is a package the JDK may hold classes of.
    /// A package above a root (`com`) is one, as the path to the root goes through it.
    pub(in crate::typer) fn jdk_rooted(&self, p: PkgId) -> bool {
        let Some(loaded) = self.loaded.as_ref() else { return self.jdk_rooted_uncached(p) };
        if let Some(&rooted) = loaded.java.rooted.get(&p) {
            return rooted;
        }
        let rooted = self.jdk_rooted_uncached(p);
        loaded.java.rooted.insert(p, rooted);
        rooted
    }

    fn jdk_rooted_uncached(&self, p: PkgId) -> bool {
        let (mut top, mut second) = (None, None);
        let mut p = p;
        while p != ROOT_PKG {
            let info = self.syms.pkg(p);
            second = top;
            top = Some(info.name);
            match info.parent {
                Some(parent) => p = parent,
                None => return false,
            }
        }
        let Some(top) = top else { return false };
        let (top, second) = (self.interner.get(top), second.map(|n| self.interner.get(n)));
        JDK_ROOTS.iter().any(|root| match root.split_once('.') {
            None => *root == top,
            Some((a, b)) => a == top && second.map_or(true, |s| s == b),
        })
    }

    // ---- the JDK ----

    /// Opens the JDK's `ct.sym` for a lookup of `wanted` and enters its packages; false, with
    /// the error reported once, when no JDK is found.
    fn open_jdk(&mut self, wanted: &str) -> bool {
        if self.java().jdk.is_some() {
            return true;
        }
        if self.java().jdk_missing {
            return false;
        }
        let start = Instant::now();
        let file = self.env.file;
        let Some(home) = jdk::find_home() else {
            self.java_mut().jdk_missing = true;
            let msg = format!("no JDK found (JAVA_HOME, /usr/libexec/java_home or java on the PATH) while looking for {}", wanted);
            self.diags.error(file, Span::default(), msg);
            return false;
        };
        let path = jdk::ct_sym_path(&home);
        let ct = match CtSym::open(&path.to_string_lossy()) {
            Ok(ct) => ct,
            Err(e) => {
                self.java_mut().jdk_missing = true;
                self.diags.error(file, Span::default(), e);
                return false;
            }
        };
        let release = match self.java().release {
            Some(r) if ct.releases.contains(&r) => r,
            Some(r) => {
                self.java_mut().jdk_missing = true;
                let msg = format!("{}: release {} is not in it (it holds {} to {})", path.display(), r, ct.releases[0], ct.latest_release());
                self.diags.error(file, Span::default(), msg);
                return false;
            }
            None => ct.latest_release(),
        };
        let opened = start.elapsed();
        let start = Instant::now();
        let mut packages: Vec<(String, Vec<u32>)> = ct.index(release).into_iter().map(|(k, v)| (k.to_string(), v)).collect();
        packages.sort();
        for (i, (path, _)) in packages.iter().enumerate() {
            let mut p = ROOT_PKG;
            for seg in path.split('/').filter(|s| !s.is_empty()) {
                let n = self.interner.intern(seg);
                p = self.syms.sub_pkg(p, n);
            }
            self.java_mut().jdk_pkgs.insert(p, i as u32);
        }
        let j = self.java_mut();
        j.stats.ct_open = Some(opened);
        j.stats.ct_index = start.elapsed();
        j.stats.release = release;
        j.jdk = Some(Jdk { ct, packages, names: FxMap::default(), parsed: FxMap::default() });
        true
    }

    /// The class file of the JDK's runtime image that holds the class of the `ct.sym` entry, as
    /// scalac's class path names it: `/modules/java.base/java/lang/String.class`.
    pub(crate) fn jdk_runtime_path(&self, entry: u32) -> Option<String> {
        let jdk = self.loaded.as_ref()?.java.jdk.as_ref()?;
        let name = jdk.ct.zip.name(jdk.ct.zip.entries.get(entry as usize)?);
        let stem = name.strip_suffix(".sig")?;
        let (_, rest) = stem.split_once('/')?;
        Some(format!("/modules/{}.class", rest))
    }

    /// The runtime image's class file of the JDK class `simple` of the package `p` where the
    /// typing opened the JDK; `None` where it did not or the JDK has no such class.
    pub(crate) fn jdk_runtime_path_of(&mut self, p: PkgId, simple: &str) -> Option<String> {
        if !self.loaded.as_ref()?.java.jdk_open() {
            return None;
        }
        let entry = self.jdk_entry(p, simple)?;
        self.jdk_runtime_path(entry)
    }

    /// Every top-level class of the JDK by package (dotted) and simple name, once it is open.
    pub(crate) fn jdk_names(&self) -> Vec<(String, String)> {
        let Some(jdk) = self.loaded.as_ref().and_then(|l| l.java.jdk.as_ref()) else { return Vec::new() };
        let mut out = Vec::new();
        for (path, entries) in &jdk.packages {
            let owner = path.trim_matches('/').replace('/', ".");
            if owner.is_empty() {
                continue;
            }
            for &e in entries {
                let name = jdk.ct.zip.name(&jdk.ct.zip.entries[e as usize]);
                let stem = name.strip_suffix(".sig").unwrap_or(name);
                let simple = stem.rfind('/').map_or(stem, |at| &stem[at + 1..]);
                if !simple.contains('$') && simple != "package-info" && simple != "module-info" {
                    out.push((owner.clone(), simple.to_string()));
                }
            }
        }
        out
    }

    /// The simple names of the JDK's top-level classes in package `p`, once the JDK is open.
    pub(crate) fn jdk_class_names(&self, p: PkgId) -> Vec<String> {
        let Some(java) = self.loaded.as_ref().map(|l| &l.java) else { return Vec::new() };
        let (Some(&idx), Some(jdk)) = (java.jdk_pkgs.get(&p), java.jdk.as_ref()) else { return Vec::new() };
        jdk.packages[idx as usize]
            .1
            .iter()
            .filter_map(|&e| {
                let name = jdk.ct.zip.name(&jdk.ct.zip.entries[e as usize]);
                let stem = name.strip_suffix(".sig").unwrap_or(name);
                let simple = stem.rfind('/').map_or(stem, |at| &stem[at + 1..]);
                (!simple.contains('$') && simple != "package-info" && simple != "module-info").then(|| simple.to_string())
            })
            .collect()
    }

    /// The entry of the class `name` in the JDK package `p`, once the JDK is open.
    fn jdk_entry(&mut self, p: PkgId, name: &str) -> Option<u32> {
        let idx = *self.java().jdk_pkgs.get(&p)?;
        let jdk = self.java_mut().jdk.as_mut()?;
        if !jdk.names.contains_key(&idx) {
            let mut names: FxMap<Box<str>, u32> = FxMap::default();
            for &e in &jdk.packages[idx as usize].1 {
                let entry_name = jdk.ct.zip.name(&jdk.ct.zip.entries[e as usize]);
                let stem = entry_name.strip_suffix(".sig").unwrap_or(entry_name);
                let simple = stem.rfind('/').map_or(stem, |at| &stem[at + 1..]);
                names.insert(simple.into(), e);
            }
            jdk.names.insert(idx, names);
        }
        jdk.names[&idx].get(name).copied()
    }

    fn jdk_class_file(&mut self, entry: u32) -> Option<Arc<ClassFile>> {
        let start = Instant::now();
        let file = self.env.file;
        let jdk = self.java_mut().jdk.as_mut()?;
        if let Some(cf) = jdk.parsed.get(&entry) {
            return Some(cf.clone());
        }
        let name = jdk.ct.zip.name(&jdk.ct.zip.entries[entry as usize]).to_string();
        let mut bytes_read = 0;
        let parsed = jdk.ct.zip.read(entry as usize).and_then(|bytes| {
            bytes_read = bytes.len();
            ClassFile::parse(&bytes)
        });
        match parsed {
            Ok(cf) => {
                let rc = Arc::new(cf);
                jdk.parsed.insert(entry, rc.clone());
                let j = self.java_mut();
                j.stats.jdk_files_parsed += 1;
                j.stats.jdk_bytes += bytes_read;
                j.stats.jdk_parse_time += start.elapsed();
                Some(rc)
            }
            Err(e) => {
                self.diags.error(file, Span::default(), format!("{}: {}", name, e));
                None
            }
        }
    }

    /// Whether the class a placeholder `name` of package `p` stands for is a static member of the
    /// Java class `p` names: a pickle reaches `fix.Outer.Nested` through the class's companion,
    /// which the loader enters as a package `fix.Outer`. `None` where no class file `Outer$Nested`
    /// of the package around `p` tells.
    pub(super) fn placeholder_static_nested(&mut self, p: PkgId, name: Name) -> Option<bool> {
        let info = self.syms.pkg(p);
        let parent = info.parent?;
        let stem = format!("{}${}", self.interner.get(info.name), self.interner.get(name));
        let (_, cf) = self.locate_java_class(parent, &stem, false)?;
        Some(cf.inner_outer().is_none())
    }

    /// The class file `name` of the classpath package `slot`, or of the JDK package `p`.
    fn locate_java_class(&mut self, p: PkgId, name: &str, open_jdk: bool) -> Option<(JSource, Arc<ClassFile>)> {
        if let Some(slot) = self.loaded.as_ref().unwrap().slot(p) {
            if let Some(f) = self.java_class_file_named(slot, name) {
                let file = self.loaded.as_ref().unwrap().jar_files[f.jar as usize];
                return match self.loaded_mut().cp.class_file(f) {
                    // A class of a Scala source that is not the file's own stem (one nested in a
                    // package object) carries the compiler's attribute; its TASTy is the truth.
                    Ok(cf) if cf.scala => None,
                    Ok(cf) => Some((JSource::Cp(f), cf)),
                    Err(e) => {
                        self.diags.error(file, Span::default(), e);
                        None
                    }
                };
            }
        }
        if !self.jdk_rooted(p) {
            return None;
        }
        if !self.java().jdk_open() && (!open_jdk || !self.open_jdk(&self.dotted(p, Some(name)))) {
            return None;
        }
        let entry = self.jdk_entry(p, name)?;
        let cf = self.jdk_class_file(entry)?;
        Some((JSource::Jdk(entry), cf))
    }

    // ---- entering ----

    /// Enters the Java class `name` of package `p` from the jars or the JDK, or a sub-package
    /// of the JDK; whether the package's entries answer the lookup now. A placeholder that
    /// stands for the class already is read when it is completed, and the lookup of its term
    /// side (the companion) completes it here.
    pub(super) fn load_java_pkg_member(&mut self, p: PkgId, name: Name) -> bool {
        // The std is written against itself; `java.lang.String` and `Object` are the builtins.
        // On the JVM a class of the JavaScript layer alone is the JDK's for the std too.
        let file = self.env.file;
        if self.source(file).is_std && !self.in_jar(file) && !(self.jvm && crate::js_only_class(self.interner.get(name), || self.pkg_path(p))) {
            return false;
        }
        if p == self.java_lang_pkg_id() && (name == names::STRING || self.interner.get(name) == "Object") {
            return false;
        }
        // An object the std defines under the name (`scala.runtime.Statics`) stands for the
        // Java class.
        self.demand_std(p, name, crate::stdindex::TYPE | crate::stdindex::TERM);
        if self.syms.pkg(p).entries.get(&name).map_or(false, |e| e.term.is_some() && e.class.is_none()) {
            return false;
        }
        let existing = self.syms.pkg(p).entries.get(&name).and_then(|e| e.class);
        if let Some(c) = existing {
            if self.is_java_placeholder(c) {
                self.complete_class(c);
                return self.syms.class(c).companion.is_some();
            }
            // With the std present its extension methods are the String API on JavaScript.
            if c == self.b.string || c == self.b.any_ref {
                return self.loaded.as_ref().unwrap().scala_library && self.enter_java_builtin_members(c);
            }
            return false;
        }
        // A class a signature names becomes a placeholder (`type_in_pkg`, `java_top_level`),
        // whichever source holds it, and is read when the program needs it.
        if self.mapping_signature {
            return false;
        }
        let text = self.interner.get(name).to_string();
        if !self.java().jdk_open() && self.jdk_rooted(p) && std::env::var_os("TEQ_CLASSPATH_DETAIL").is_some() {
            eprintln!("open jdk for {}", self.dotted(p, Some(&text)));
        }
        if !self.java().jdk_open() && self.jdk_rooted(p) && self.open_jdk(&self.dotted(p, Some(&text))) {
            if self.syms.pkg(p).entries.get(&name).map_or(false, |e| e.pkg.is_some()) {
                return true;
            }
        }
        let Some((source, cf)) = self.locate_java_class(p, &text, false) else { return false };
        self.enter_java_class(Owner::Package(p), Owner::Package(p), name, source, cf, None, 0);
        true
    }

    /// Whether `load_java_pkg_member` would enter nothing for `name` in `p`, read outside the
    /// loader's lock (`pkg_member_settled`): the std's definition of the name, if any, entered
    /// first as there; nothing of a class of that name in the package's entries, whose placeholder
    /// or builtin is the lock's to complete; no Java class of the name in the package's jars; and
    /// no JDK package, whose index is the lock holder's.
    pub(super) fn java_pkg_member_settled(&mut self, p: PkgId, name: Name, slot: Option<usize>) -> bool {
        let file = self.env.file;
        if self.source(file).is_std && !self.in_jar(file) {
            return true;
        }
        let java_lang = self.syms.pkg(ROOT_PKG).entries.get(&names::JAVA).and_then(|e| e.pkg).and_then(|j| self.syms.pkg(j).entries.get(&names::LANG).and_then(|e| e.pkg));
        if Some(p) == java_lang && (name == names::STRING || self.interner.get(name) == "Object") {
            return true;
        }
        if self.jdk_rooted(p) {
            return false;
        }
        self.demand_std(p, name, crate::stdindex::TYPE | crate::stdindex::TERM);
        if let Some(e) = self.syms.pkg(p).entries.get(&name) {
            if e.term.is_some() && e.class.is_none() {
                return true;
            }
            if e.class.is_some() {
                return false;
            }
        }
        match slot {
            Some(slot) => match self.loaded.as_ref().unwrap().java_names(slot) {
                Some(java) => !java.contains_key(self.interner.get(name)),
                None => false,
            },
            None => true,
        }
    }

    /// Reads the class a placeholder stands for and completes it; a class no source holds
    /// stays the empty trait it was.
    pub(super) fn complete_java_placeholder(&mut self, c: ClassId) {
        let Some((p, name)) = self.java().placeholder(c) else { return };
        let text = self.interner.get(name).to_string();
        match self.locate_java_class(p, &text, true) {
            Some((source, cf)) => {
                self.enter_java_class(Owner::Package(p), Owner::Package(p), name, source, cf, Some(c), 0);
                self.complete_java_class(c);
            }
            None => {
                self.syms.class_mut(c).state().set(Completion::Done);
            }
        }
    }

    fn enter_java_class(
        &mut self,
        owner: Owner,
        term_owner: Owner,
        name: Name,
        source: JSource,
        cf: Arc<ClassFile>,
        placeholder: Option<ClassId>,
        depth: u32,
    ) -> ClassId {
        let file = self.java_file(source);
        let is_interface = cf.access & ACC_INTERFACE != 0;
        let kind = if is_interface { ClassKind::Trait } else { ClassKind::Class };
        // The constructors are alternatives of `<init>`; the primary teq gives every class
        // stays out of reach.
        let mut m = if is_interface { 0 } else { mods::PRIVATE_CTOR };
        if cf.access & ACC_FINAL != 0 {
            m |= mods::FINAL;
        }
        if cf.access & ACC_ABSTRACT != 0 && !is_interface {
            m |= mods::ABSTRACT;
        }
        if !cf.permitted.is_empty() {
            m |= mods::SEALED;
        }
        if cf.access & crate::classfile::ACC_ANNOTATION != 0 {
            m |= mods::JAVA_ANNOTATION;
        }
        let c = match placeholder {
            Some(c) => {
                let mut info = self.syms.class_mut(c);
                info.kind = kind;
                info.mods = m;
                info.file = file;
                info.base_types.clear();
                c
            }
            None => self.syms.new_class(name, kind, m, owner, file, None, Span::default()),
        };
        let tparam_names: Vec<String> = match &cf.signature {
            Some(s) => sig::parse_class(s).map(|cs| cs.tparams.into_iter().map(|p| p.name).collect()).unwrap_or_default(),
            None => Vec::new(),
        };
        let existing = self.syms.class(c).tparams.clone();
        let tparams: Vec<TParamId> = if existing.len() == tparam_names.len() {
            for (&id, n) in existing.iter().zip(&tparam_names) {
                self.syms.tparams[id.idx()].name = self.interner.intern(n);
            }
            existing
        } else {
            tparam_names
                .iter()
                .map(|n| {
                    let n = self.interner.intern(n);
                    self.syms.new_tparam(n, 0)
                })
                .collect()
        };
        self.syms.class_mut(c).tparams = tparams;
        if placeholder.is_some() {
            self.java_mut().placeholder_read(c);
        } else {
            match owner {
                Owner::Package(p) => self.syms.pkgs[p.idx()].entries.entry(name).or_default().class = Some(c),
                Owner::Class(o) => {
                    self.syms.class_mut(o).nested.insert(name, c);
                    if let Some(oc) = self.syms.class(o).companion {
                        self.syms.class_mut(oc).nested.insert(name, c);
                    }
                }
                Owner::Local => {}
            }
        }
        let inner = cf.inner_outer().is_some();
        self.java_mut().classes.replace(c, JClass { source, cf: cf.clone(), companion_of: None, inner });
        if std::env::var_os("TEQ_CLASSPATH_DETAIL").is_some() {
            eprintln!("enter java {} from {}", self.class_path(c), cf.name);
        }
        // The static members go to an object of the class's name.
        let obj = self.syms.new_class(name, ClassKind::Object, mods::FINAL, term_owner, file, None, Span::default());
        let sym = self.syms.new_sym(name, SymKind::Object(obj), 0, term_owner, file, None, Span::default());
        let ty = self.types.class(obj, &[]);
        {
            let mut s = self.syms.sym_mut(sym);
            s.sig = Some(Arc::new(MethodSig::value(ty)));
            s.state().set(Completion::Done);
        }
        self.syms.class_mut(obj).module_sym = Some(sym);
        self.syms.class_mut(obj).companion = Some(c);
        self.syms.class_mut(c).companion = Some(obj);
        // A std object of the name (`java.util.AbstractMap`'s entries) stands for the statics.
        let std_object = match term_owner {
            Owner::Package(p) => self.syms.pkg(p).entries.get(&name).and_then(|e| e.term).map_or(false, |t| self.source(self.syms.sym(t).file).is_std && !self.in_jar(self.syms.sym(t).file)),
            _ => false,
        };
        if !std_object {
            self.register_term(term_owner, sym);
        }
        self.java_mut().classes.replace(obj, JClass { source, cf: cf.clone(), companion_of: Some(c), inner: false });
        self.java_mut().stats.classes_entered += 1;
        if depth < MAX_NESTING {
            self.enter_java_nested(c, obj, &cf, depth + 1);
        }
        c
    }

    /// The nested classes of `c`, each its own class file next to the outer's.
    fn enter_java_nested(&mut self, c: ClassId, obj: ClassId, cf: &ClassFile, depth: u32) {
        let mut p = self.syms.class(c).owner;
        while let Owner::Class(o) = p {
            p = self.syms.class(o).owner;
        }
        let Owner::Package(pkg) = p else { return };
        let nested: Vec<(String, String)> = cf
            .member_classes()
            .filter_map(|i| i.name.as_ref().map(|n| (i.inner.clone(), n.clone())))
            .collect();
        for (binary, simple) in nested {
            let (_, stem) = split_binary(&binary);
            let name = self.interner.intern(&simple);
            if self.syms.class(c).nested.contains_key(&name) {
                continue;
            }
            let Some((source, ncf)) = self.locate_java_class(pkg, stem, false) else { continue };
            self.enter_java_class(Owner::Class(c), Owner::Class(obj), name, source, ncf, None, depth);
        }
    }

    // ---- completing ----

    /// Completes a Java class: its parents, constructors and members, and its companion's
    /// static members. True when `c` is one.
    pub(super) fn complete_java_class(&mut self, c: ClassId) -> bool {
        let Some(jc) = self.java().classes.get(&c) else { return false };
        let (cf, companion_of, inner) = (jc.cf.clone(), jc.companion_of, jc.inner);
        if let Some(class) = companion_of {
            if self.syms.class(class).state() == Completion::NotStarted {
                self.complete_class(class);
            }
            let self_ty = self.types.class(c, &[]);
            let mut info = self.syms.class_mut(c);
            if info.base_types.is_empty() {
                info.base_types.push((c, self_ty));
            }
            info.state().set(Completion::Done);
            return true;
        }
        self.syms.class_mut(c).state().set(Completion::InProgress);
        self.loaded_mut().classes_completed += 1;
        if std::env::var_os("TEQ_CLASSPATH_DETAIL").is_some() {
            eprintln!("complete {}", self.class_path(c));
        }
        let file = self.syms.class(c).file;
        let obj = self.syms.class(c).companion.unwrap_or(c);
        let csig = sig::class_sig(&cf);
        let mut cx = self.java_cx(c);
        let was_mapping = self.mapping_signature;
        self.mapping_signature = true;
        let mut parents = Vec::new();
        if let Ok(csig) = &csig {
            let ids = self.syms.class(c).tparams.clone();
            for (tp, &id) in csig.tparams.iter().zip(&ids) {
                let upper = self.java_bound(&mut cx, tp);
                self.syms.tparams[id.idx()].upper = upper;
            }
            let heads: Vec<&JType> = std::iter::once(&csig.superclass).chain(&csig.interfaces).collect();
            for t in heads {
                if matches!(t, JType::Void) || t.is_class("java/lang/Object") {
                    continue;
                }
                let JType::Class(ct) = t else { continue };
                let mapped = self.map_jclass(&mut cx, ct, false, true);
                if matches!(self.types.get(mapped), Type::Class(..)) {
                    parents.push(mapped);
                }
            }
        }
        self.mapping_signature = was_mapping;
        self.set_loaded_parents(c, parents);
        for permitted in cf.permitted.clone() {
            if let Some(child) = self.java_class_by_binary(&permitted, false, 0) {
                if child != c && self.syms.class(child).kind != ClassKind::Builtin && !self.syms.class(c).children.contains(&child) {
                    self.syms.class_mut(c).children.push(child);
                }
            }
        }
        for (i, m) in cf.methods.iter().enumerate() {
            if !visible(m.access) || m.name == "<clinit>" {
                continue;
            }
            if m.name == "<init>" {
                let sym = self.syms.new_sym(names::INIT, SymKind::Def, 0, Owner::Class(c), file, None, Span::default());
                self.java_mut().syms.insert(sym, JSym { class: c, member: JMember::Method(i as u32) });
                self.syms.class_mut(c).ctors.push(sym);
                continue;
            }
            let is_static = m.access & ACC_STATIC != 0;
            let owner = if is_static { obj } else { c };
            let name = self.interner.intern(&m.name);
            let mut mm = 0;
            if m.access & ACC_ABSTRACT != 0 {
                mm |= mods::ABSTRACT;
            }
            if m.access & ACC_FINAL != 0 {
                mm |= mods::FINAL;
            }
            if m.access & ACC_PROTECTED != 0 {
                mm |= mods::PROTECTED;
            }
            let sym = self.syms.new_sym(name, SymKind::Def, mm, Owner::Class(owner), file, None, Span::default());
            self.syms.sym_mut(sym).java_defined = true;
            self.java_mut().syms.insert(sym, JSym { class: c, member: JMember::Method(i as u32) });
            self.register_term(Owner::Class(owner), sym);
        }
        for (i, f) in cf.fields.iter().enumerate() {
            if !visible(f.access) {
                continue;
            }
            let is_static = f.access & ACC_STATIC != 0;
            let owner = if is_static { obj } else { c };
            let name = self.interner.intern(&f.name);
            let (kind, mut fm) = if f.access & ACC_FINAL != 0 || f.access & ACC_ENUM != 0 {
                (SymKind::Val, mods::FIELD)
            } else {
                (SymKind::Var, mods::FIELD | mods::MUTABLE)
            };
            if f.access & ACC_PROTECTED != 0 {
                fm |= mods::PROTECTED;
            }
            if self.syms.class(owner).members.contains_key(&name) {
                continue;
            }
            let sym = self.syms.new_sym(name, kind, fm, Owner::Class(owner), file, None, Span::default());
            self.syms.sym_mut(sym).java_defined = true;
            self.java_mut().syms.insert(sym, JSym { class: c, member: JMember::Field(i as u32) });
            self.register_term(Owner::Class(owner), sym);
        }
        // Java overloads a name across the classes that declare it: `DecimalFormat.format(x,
        // buffer, position)` beside `NumberFormat.format(double)` and `Format.format(Object)`.
        if self.syms.class(c).base_types.len() > 1 {
            self.mark_members_meeting_inherited(c);
            self.join_unrelated_inherited(c);
        }
        if inner {
            let msg = format!("inner class {} needs an outer instance (Outer.Inner)", self.class_path(c));
            self.loaded_mut().note_blocked(&msg);
        }
        self.syms.class_mut(c).state().set(Completion::Done);
        if obj != c {
            let self_ty = self.types.class(obj, &[]);
            let mut info = self.syms.class_mut(obj);
            if info.base_types.is_empty() {
                info.base_types.push((obj, self_ty));
            }
            info.state().set(Completion::Done);
        }
        true
    }

    /// The JDK class that a class of the std's platform layer stands for, named by its
    /// `@jvmClass` annotation.
    fn std_jvm_class(&self, c: ClassId) -> Option<String> {
        let info = self.syms.class(c);
        if !self.source(info.file).is_std || self.in_jar(info.file) {
            return None;
        }
        let ast = self.ast(info.file);
        let def = ast.def(info.def?);
        let a = def.annots.iter().find(|a| a.name == names::JVM_CLASS)?;
        ast.annot_args(a).first().map(|&s| ast.str(s).to_string())
    }

    /// A member lookup missed on a class of the std's platform layer: the JDK class it stands
    /// for gives it the members the std left out, so that they keep the JDK's signatures on
    /// both targets and the JavaScript backend names what it has no implementation of. The
    /// class and the object that holds the statics are read together.
    fn std_member_named(&self, c: ClassId, name: Name) -> bool {
        let info = self.syms.class(c);
        info.members.contains_key(&name) || info.base_types.iter().any(|&(b, _)| self.syms.class(b).members.contains_key(&name))
    }

    /// The constructors of the JDK class a class of the std's platform layer stands for join
    /// the std's own where they take other parameters: `new PrintWriter(file)` is the JDK's
    /// `PrintWriter(File)`, a type the std, written for JavaScript too, cannot name. On the JVM
    /// with a class path only, where the class is the JDK's.
    pub(in crate::typer) fn absorb_java_ctors(&mut self, c: ClassId) {
        if self.forked && crate::shared::lock_depth() == 0 && (!self.jvm || !self.sees_classpath_outside_lock() || self.java().ctors_done.contains_key(&c)) {
            crate::measure::looked_up_unlocked(crate::measure::Lookup::JavaClassEntry);
            return;
        }
        self.with_loader_for(crate::measure::Hold::JarEntry, |w| {
            let first = w.jvm && w.sees_classpath() && !w.java().ctors_absorbed.contains_key(&c);
            w.absorb_java_ctors_unlocked(c);
            crate::measure::looked_up(crate::measure::Lookup::JavaClassEntry, first);
        })
    }

    fn absorb_java_ctors_unlocked(&mut self, c: ClassId) {
        if !self.jvm || !self.sees_classpath() || self.java().ctors_absorbed.contains_key(&c) {
            return;
        }
        self.java_mut().ctors_absorbed.insert(c, ());
        self.absorb_java_ctors_now(c);
        self.loader_done(crate::typer::LoaderDone::JavaCtors(c));
    }

    fn absorb_java_ctors_now(&mut self, c: ClassId) {
        if self.std_jvm_class(c).is_none() || self.syms.class(c).kind != ClassKind::Class {
            return;
        }
        if !self.java().classes.contains_key(&c) {
            self.absorb_java_class(c);
        }
        let Some(cf) = self.java().classes.get(&c).filter(|j| j.companion_of.is_none()).map(|j| j.cf.clone()) else { return };
        let explicit = |sig: &MethodSig| -> Vec<TypeId> { sig.clauses.iter().find(|cl| !cl.is_using).map_or(Vec::new(), |cl| cl.params.iter().map(|p| p.ty).collect()) };
        let mut own: Vec<Vec<TypeId>> = Vec::new();
        let primary = self.syms.class(c).ctor.clone();
        own.push(primary.iter().find(|cl| !cl.is_using).map_or(Vec::new(), |cl| cl.params.iter().map(|p| p.ty).collect()));
        for s in self.syms.class(c).ctors.clone() {
            own.push(explicit(self.sig_of(s)));
        }
        let file = self.syms.class(c).file;
        for (i, m) in cf.methods.iter().enumerate() {
            if m.name != "<init>" || !visible(m.access) || m.access & ACC_PROTECTED != 0 {
                continue;
            }
            let sym = self.syms.new_sym(names::INIT, SymKind::Def, 0, Owner::Class(c), file, None, Span::default());
            self.java_mut().syms.insert(sym, JSym { class: c, member: JMember::Method(i as u32) });
            let params = explicit(self.sig_of(sym));
            let known = own.iter().any(|o| o.len() == params.len() && o.iter().zip(&params).all(|(&a, &b)| self.is_same(a, b)));
            if known {
                self.java_mut().syms.remove(&sym);
                continue;
            }
            self.syms.class_mut(c).ctors.push(sym);
            self.java_mut().absorbed_ctors.insert(sym, ());
            self.ensure_primary_ctor(c);
            // Beside alternatives that each take a list, `new StringWriter()` means the primary.
            if !self.syms.class(c).ctor.iter().any(|cl| !cl.is_using) {
                self.syms.class_mut(c).ctor.insert(0, ClauseSig { params: Vec::new(), is_using: false, is_implicit: false });
            }
        }
    }

    /// Whether a constructor may be called here: a JDK one `absorb_java_ctors` added is the
    /// program's alone, since a library body the interpreter runs has no body to run for it.
    pub(in crate::typer) fn ctor_callable_here(&self, s: SymId) -> bool {
        let Some(loaded) = self.loaded.as_ref() else { return true };
        if !loaded.java.absorbed_ctors.contains_key(&s) {
            return true;
        }
        let file = self.env.file;
        !(self.in_jar(file) || self.is_body_file(file) || self.source(file).is_std)
    }

    fn absorb_java_class(&mut self, c: ClassId) -> bool {
        if self.forked && crate::shared::lock_depth() == 0 && (self.std_jvm_class(c).is_none() || self.java().absorbed_done.contains_key(&c)) {
            crate::measure::looked_up_unlocked(crate::measure::Lookup::JavaClassEntry);
            return false;
        }
        self.with_loader_for(crate::measure::Hold::JarEntry, |w| {
            let absorbed = w.absorb_java_class_unlocked(c);
            crate::measure::looked_up(crate::measure::Lookup::JavaClassEntry, absorbed);
            absorbed
        })
    }

    fn absorb_java_class_unlocked(&mut self, c: ClassId) -> bool {
        if self.java().classes.contains_key(&c) {
            return false;
        }
        let absorbed = self.absorb_java_class_now(c);
        // A class read, or none to read with the JDK open, is settled for good; one whose JDK
        // was not open yet may be read once it is.
        if absorbed || self.java().jdk_open() {
            self.loader_done(crate::typer::LoaderDone::JavaClass(c));
            let companion = self.syms.class(c).companion;
            if let Some(o) = companion.filter(|_| absorbed) {
                self.loader_done(crate::typer::LoaderDone::JavaClass(o));
            }
        }
        absorbed
    }

    fn absorb_java_class_now(&mut self, c: ClassId) -> bool {
        let Some(binary) = self.std_jvm_class(c) else { return false };
        let (pkg_path, stem) = split_binary(&binary);
        let mut pkg = ROOT_PKG;
        for seg in pkg_path.split('/').filter(|s| !s.is_empty()) {
            let n = self.interner.intern(seg);
            match self.syms.pkg(pkg).entries.get(&n).and_then(|e| e.pkg) {
                Some(p) => pkg = p,
                None => return false,
            }
        }
        let Some((source, cf)) = self.locate_java_class(pkg, stem, true) else { return false };
        let is_object = self.syms.class(c).kind == ClassKind::Object;
        let companion = self.syms.class(c).companion;
        let class = if is_object { companion.unwrap_or(c) } else { c };
        let obj = if is_object { Some(c) } else { companion.filter(|&o| self.syms.class(o).kind == ClassKind::Object) };
        let file = self.syms.class(c).file;
        self.java_mut().classes.replace(class, JClass { source, cf: cf.clone(), companion_of: None, inner: false });
        if let Some(o) = obj.filter(|&o| o != class) {
            self.java_mut().classes.replace(o, JClass { source, cf: cf.clone(), companion_of: Some(class), inner: false });
        }
        for (i, m) in cf.methods.iter().enumerate() {
            let universal = matches!(m.name.as_str(), "equals" | "hashCode" | "toString" | "getClass" | "clone" | "<init>" | "<clinit>");
            if !visible(m.access) || universal {
                continue;
            }
            let is_static = m.access & ACC_STATIC != 0;
            let owner = match (is_static, obj) {
                (true, Some(o)) => o,
                (true, None) => continue,
                (false, _) => class,
            };
            let name = self.interner.intern(&m.name);
            // The std's own members come first: a name it defines or inherits takes no Java
            // overloads.
            if self.std_member_named(owner, name) {
                continue;
            }
            let sym = self.syms.new_sym(name, SymKind::Def, 0, Owner::Class(owner), file, None, Span::default());
            self.syms.sym_mut(sym).java_defined = true;
            self.java_mut().syms.insert(sym, JSym { class, member: JMember::Method(i as u32) });
            self.register_term(Owner::Class(owner), sym);
        }
        for (i, f) in cf.fields.iter().enumerate() {
            if !visible(f.access) {
                continue;
            }
            let is_static = f.access & ACC_STATIC != 0;
            let owner = match (is_static, obj) {
                (true, Some(o)) => o,
                (true, None) => continue,
                (false, _) => class,
            };
            let name = self.interner.intern(&f.name);
            if self.std_member_named(owner, name) {
                continue;
            }
            let (kind, fm) = if f.access & ACC_FINAL != 0 { (SymKind::Val, mods::FIELD) } else { (SymKind::Var, mods::FIELD | mods::MUTABLE) };
            let sym = self.syms.new_sym(name, kind, fm, Owner::Class(owner), file, None, Span::default());
            self.syms.sym_mut(sym).java_defined = true;
            self.java_mut().syms.insert(sym, JSym { class, member: JMember::Field(i as u32) });
            self.register_term(Owner::Class(owner), sym);
        }
        true
    }

    fn java_cx(&mut self, c: ClassId) -> JCx {
        let class = self.syms.class(c).tparams.iter().map(|&id| (self.interner.get(self.syms.tparam(id).name).to_string(), id)).collect();
        JCx { class, method: Vec::new() }
    }

    /// The bounds of a type parameter joined: `T extends A & B` is `T <: A & B`.
    fn java_bound(&mut self, cx: &mut JCx, tp: &TypeParam) -> TypeId {
        let mut upper = ANY;
        for b in tp.bounds() {
            let t = self.map_jtype(cx, b, false);
            upper = self.types.inter(upper, t);
        }
        upper
    }

    /// Gives the builtin `String` or `AnyRef` the members of its Java class that it lacks, and
    /// the builtin `String` a companion with the static ones where the std supplies none. The
    /// class files are the JDK's, compiled into teq, so that no program pays for `ct.sym` to
    /// call a method of `String`.
    pub(super) fn enter_java_builtin_members(&mut self, c: ClassId) -> bool {
        let slot = if c == self.b.string { 0 } else { 1 };
        if self.forked && crate::shared::lock_depth() == 0 && self.java().builtin_done[slot].load(std::sync::atomic::Ordering::Acquire) {
            crate::measure::looked_up_unlocked(crate::measure::Lookup::JavaClassEntry);
            return false;
        }
        self.with_loader_for(crate::measure::Hold::JarEntry, |w| {
            let entered = w.enter_java_builtin_members_unlocked(c);
            crate::measure::looked_up(crate::measure::Lookup::JavaClassEntry, entered);
            entered
        })
    }

    pub(super) fn enter_java_builtin_members_unlocked(&mut self, c: ClassId) -> bool {
        let (slot, bytes) = if c == self.b.string {
            (0, STRING_SIG)
        } else if c == self.b.any_ref {
            (1, OBJECT_SIG)
        } else {
            return false;
        };
        if self.java().builtin_members[slot] {
            return false;
        }
        self.java_mut().builtin_members[slot] = true;
        let entered = self.enter_java_builtin_members_now(c, bytes);
        self.loader_done(crate::typer::LoaderDone::JavaBuiltin(slot));
        entered
    }

    fn enter_java_builtin_members_now(&mut self, c: ClassId, bytes: &[u8]) -> bool {
        let source = JSource::Embedded;
        let cf = match ClassFile::parse(bytes) {
            Ok(cf) => Arc::new(cf),
            Err(_) => return false,
        };
        let file = self.java_file(source);
        self.java_mut().classes.replace(c, JClass { source, cf: cf.clone(), companion_of: None, inner: false });
        // The std's own members come first: a name it defines takes no Java overloads. On the
        // builtin `String` the std's members are its extension methods in package `scala`, the
        // intrinsics the JavaScript target implements: the std files defining one under a
        // Java method's name enter first.
        let mut own: Vec<Name> = self.syms.class(c).members.keys().copied().collect();
        if c == self.b.string {
            let scala = self.b.scala_pkg;
            for m in &cf.methods {
                let name = self.interner.intern(&m.name);
                self.demand_std(scala, name, crate::stdindex::TERM);
            }
            own.extend(self.syms.pkg(scala).entries.iter().filter(|(_, e)| !e.extensions.is_empty()).map(|(&n, _)| n));
        }
        let statics = c == self.b.string && self.syms.class(c).companion.is_none();
        for (i, m) in cf.methods.iter().enumerate() {
            let universal = matches!(m.name.as_str(), "equals" | "hashCode" | "toString" | "getClass" | "clone" | "<init>" | "<clinit>");
            if !visible(m.access) || universal || m.access & ACC_STATIC != 0 {
                continue;
            }
            let name = self.interner.intern(&m.name);
            if own.contains(&name) {
                continue;
            }
            let sym = self.syms.new_sym(name, SymKind::Def, 0, Owner::Class(c), file, None, Span::default());
            self.syms.sym_mut(sym).java_defined = true;
            self.java_mut().syms.insert(sym, JSym { class: c, member: JMember::Method(i as u32) });
            self.register_term(Owner::Class(c), sym);
        }
        if statics {
            let name = self.syms.class(c).name;
            let scala = self.b.scala_pkg;
            let obj = self.syms.new_class(name, ClassKind::Object, mods::FINAL, Owner::Package(scala), file, None, Span::default());
            let sym = self.syms.new_sym(name, SymKind::Object(obj), 0, Owner::Package(scala), file, None, Span::default());
            let ty = self.types.class(obj, &[]);
            {
                let mut s = self.syms.sym_mut(sym);
                s.sig = Some(Arc::new(MethodSig::value(ty)));
                s.state().set(Completion::Done);
            }
            let self_ty = self.types.class(obj, &[]);
            {
                let mut info = self.syms.class_mut(obj);
                info.module_sym = Some(sym);
                info.companion = Some(c);
                info.base_types.push((obj, self_ty));
                info.state().set(Completion::Done);
            }
            self.syms.class_mut(c).companion = Some(obj);
            self.syms.pkgs[scala.idx()].entries.entry(name).or_default().term = Some(sym);
            self.java_mut().classes.replace(obj, JClass { source, cf: cf.clone(), companion_of: Some(c), inner: false });
            for (i, m) in cf.methods.iter().enumerate() {
                if !visible(m.access) || m.access & ACC_STATIC == 0 {
                    continue;
                }
                let name = self.interner.intern(&m.name);
                let msym = self.syms.new_sym(name, SymKind::Def, 0, Owner::Class(obj), file, None, Span::default());
                self.syms.sym_mut(msym).java_defined = true;
                self.java_mut().syms.insert(msym, JSym { class: c, member: JMember::Method(i as u32) });
                self.register_term(Owner::Class(obj), msym);
            }
        }
        true
    }

    fn java_lang_pkg_id(&mut self) -> PkgId {
        let java = self.syms.sub_pkg(ROOT_PKG, names::JAVA);
        self.syms.sub_pkg(java, names::LANG)
    }

    /// What a member lookup on `c` that found nothing may still get from Java: the members
    /// of the builtin's Java class, or those of placeholder ancestors read now. True when
    /// the lookup is worth repeating.
    pub fn java_member_miss(&mut self, c: ClassId) -> bool {
        if c == self.b.string || c == self.b.any_ref {
            return self.loaded.as_ref().unwrap().scala_library && self.enter_java_builtin_members(c);
        }
        if self.absorb_java_class(c) {
            return true;
        }
        let pending: Vec<ClassId> = self
            .syms
            .class(c)
            .base_types
            .iter()
            .skip(1)
            .map(|&(b, _)| b)
            .filter(|&b| self.is_java_placeholder(b))
            .collect();
        if pending.is_empty() {
            return false;
        }
        for &b in &pending {
            self.complete_class(b);
        }
        // A base that stays a placeholder has nothing to add: answering true again would
        // have the lookup ask forever.
        if pending.iter().all(|&b| self.is_java_placeholder(b)) {
            return false;
        }
        let parents = self.syms.class(c).parents.clone();
        self.set_loaded_parents(c, parents);
        true
    }

    // ---- signatures ----

    pub(super) fn java_sig(&mut self, sym: SymId) -> Option<Arc<MethodSig>> {
        let js = *self.java().syms.get(&sym)?;
        let jc = self.java().classes.get(&js.class)?;
        let (cf, inner) = (jc.cf.clone(), jc.inner);
        let file = self.syms.sym(sym).file;
        self.java_mut().stats.signatures_decoded += 1;
        let was_mapping = self.mapping_signature;
        self.mapping_signature = true;
        let mut cx = self.java_cx(js.class);
        let out = match js.member {
            JMember::Field(i) => {
                let f = &cf.fields[i as usize];
                let ty = match sig::field_type(f) {
                    Ok(t) => self.map_jtype(&mut cx, &t, false),
                    Err(e) => self.java_blocked(&e),
                };
                Arc::new(MethodSig::value(ty))
            }
            JMember::Method(i) => {
                let m = &cf.methods[i as usize];
                let msig = match sig::member_sig(&cf, m) {
                    Ok(s) => s,
                    Err(e) => {
                        let ty = self.java_blocked(&e);
                        self.mapping_signature = was_mapping;
                        return Some(Arc::new(MethodSig::value(ty)));
                    }
                };
                let is_ctor = m.name == "<init>";
                // A constructor takes the class's type parameters as its own, as a pickled one does.
                let mut tparams = if is_ctor { self.syms.class(js.class).tparams.clone() } else { Vec::new() };
                for tp in &msig.tparams {
                    let n = self.interner.intern(&tp.name);
                    let id = self.syms.new_tparam(n, 0);
                    cx.method.push((tp.name.clone(), id));
                    tparams.push(id);
                }
                let own = tparams.len() - msig.tparams.len();
                for (tp, &id) in msig.tparams.iter().zip(&tparams[own..]) {
                    let upper = self.java_bound(&mut cx, tp);
                    self.syms.tparams[id.idx()].upper = upper;
                }
                let names: Vec<Option<String>> = match &m.param_names {
                    Some(ns) if ns.len() >= msig.params.len() => ns[ns.len() - msig.params.len()..].to_vec(),
                    _ => Vec::new(),
                };
                let mut params = Vec::with_capacity(msig.params.len() + 1);
                if is_ctor && inner {
                    let ty = self.java_blocked(&format!("inner class {} needs an outer instance (Outer.Inner)", self.class_path(js.class)));
                    let pname = self.interner.intern("outer");
                    let psym = self.new_java_param(pname, ty, file);
                    params.push(ParamSig { name: pname, ty, by_name: false, repeated: false, has_default: false, sym: psym });
                }
                for (k, p) in msig.params.iter().enumerate() {
                    let varargs = m.access & ACC_VARARGS != 0 && k + 1 == msig.params.len();
                    let (ty, repeated) = match (varargs, p) {
                        (true, JType::Array(elem)) => (self.map_jtype(&mut cx, elem, true), true),
                        _ => (self.map_jtype(&mut cx, p, true), false),
                    };
                    let pname = match names.get(k).and_then(|n| n.as_deref()) {
                        Some(n) => self.interner.intern(n),
                        None => self.interner.intern(&format!("x${}", k + 1)),
                    };
                    let local_ty = if repeated {
                        match self.seq_class() {
                            Some(seq) => self.types.class(seq, &[ty]),
                            None => ty,
                        }
                    } else {
                        ty
                    };
                    let psym = self.new_java_param(pname, local_ty, file);
                    params.push(ParamSig { name: pname, ty, by_name: false, repeated, has_default: false, sym: psym });
                }
                let ret = if is_ctor {
                    let args: Vec<TypeId> = self.syms.class(js.class).tparams.iter().map(|&p| self.types.param(p)).collect();
                    self.types.class(js.class, &args)
                } else {
                    self.map_jtype(&mut cx, &msig.ret, false)
                };
                Arc::new(MethodSig { tparams, clauses: vec![ClauseSig { params, is_using: false, is_implicit: false }], ret })
            }
        };
        self.mapping_signature = was_mapping;
        Some(out)
    }

    fn new_java_param(&mut self, name: Name, ty: TypeId, file: FileId) -> SymId {
        let psym = self.syms.new_sym(name, SymKind::Param, 0, Owner::Local, file, None, Span::default());
        let mut s = self.syms.sym_mut(psym);
        s.sig = Some(Arc::new(MethodSig::value(ty)));
        s.state().set(Completion::Done);
        psym
    }

    fn java_blocked(&mut self, description: &str) -> TypeId {
        self.loaded_mut().note_blocked(description);
        self.types.blocked(description)
    }

    /// A Java type as Scala reads it. `java.lang.Object` is `Any` where a parameter takes it
    /// (scalac's `FromJavaObject`) and `AnyRef` elsewhere.
    fn map_jtype(&mut self, cx: &mut JCx, t: &JType, param: bool) -> TypeId {
        match t {
            JType::Prim(p) => {
                use crate::classfile::sig::Prim;
                let b = &self.b;
                match p {
                    Prim::Byte => b.t_byte,
                    Prim::Char => b.t_char,
                    Prim::Double => b.t_double,
                    Prim::Float => b.t_float,
                    Prim::Int => b.t_int,
                    Prim::Long => b.t_long,
                    Prim::Short => b.t_short,
                    Prim::Boolean => b.t_boolean,
                }
            }
            JType::Void => self.b.t_unit,
            JType::Var(v) => {
                let found = cx.method.iter().rev().chain(cx.class.iter()).find(|(n, _)| n == v).map(|&(_, id)| id);
                match found {
                    Some(id) => self.types.param(id),
                    None => self.java_blocked(&format!("type variable {} out of scope", v)),
                }
            }
            JType::Array(elem) => {
                let e = self.map_jtype(cx, elem, false);
                self.types.class(self.b.array, &[e])
            }
            JType::Class(ct) => self.map_jclass(cx, ct, param, false),
        }
    }

    /// A class type; `read` makes the head class real rather than a placeholder (parents).
    fn map_jclass(&mut self, cx: &mut JCx, ct: &ClassType, param: bool, read: bool) -> TypeId {
        if ct.outer.is_some() {
            return self.java_blocked(&format!("inner class of a parameterized outer (Outer<A>.Inner): {}", ct.name.replace('/', ".")));
        }
        match ct.name.as_str() {
            "java/lang/Object" if ct.args.is_empty() => return if param { ANY } else { self.b.t_any_ref },
            "java/lang/String" if ct.args.is_empty() => return self.b.t_string,
            _ => {}
        }
        let Some(c) = self.java_class_by_binary(&ct.name, read, ct.args.len()) else {
            return self.java_blocked(&format!("unknown class {}", ct.name.replace('/', ".")));
        };
        let arity = self.class_arity(c);
        let mut args = Vec::with_capacity(arity);
        if ct.args.is_empty() && arity > 0 {
            self.loaded_mut().note_blocked("raw type (List for List<E>): approximated");
            args.resize(arity, WILD);
        } else {
            for a in &ct.args {
                let mapped = match a {
                    TypeArg::Exact(t) => self.map_jtype(cx, t, false),
                    TypeArg::Wild => WILD,
                    TypeArg::Extends(t) => {
                        let hi = self.map_jtype(cx, t, false);
                        self.types.bounded_wild(NOTHING, hi)
                    }
                    TypeArg::Super(t) => {
                        let lo = self.map_jtype(cx, t, false);
                        self.types.bounded_wild(lo, ANY)
                    }
                };
                args.push(mapped);
            }
        }
        if args.len() != arity {
            return self.java_blocked(&format!("{} applied to {} type arguments, takes {}", ct.name.replace('/', "."), args.len(), arity));
        }
        self.types.class(c, &args)
    }

    /// The class of a binary name, `java/util/Map$Entry`: the top-level class through its
    /// package, which enters or stands in for it, and the nested ones through the outer's
    /// nested classes.
    fn java_class_by_binary(&mut self, binary: &str, read: bool, arity: usize) -> Option<ClassId> {
        match binary {
            "java/lang/String" => return Some(self.b.string),
            "java/lang/Object" => return Some(self.b.any_ref),
            _ => {}
        }
        let (pkg_path, simple) = split_binary(binary);
        let mut p = ROOT_PKG;
        for seg in pkg_path.split('/').filter(|s| !s.is_empty()) {
            let n = self.interner.intern(seg);
            p = self.syms.sub_pkg(p, n);
        }
        let name = self.interner.intern(simple);
        if let Some(c) = self.java_top_level(p, name, simple, read, arity) {
            return Some(c);
        }
        // A `$` in the simple name is the nesting of one class in another.
        let mut parts = simple.split('$').filter(|s| !s.is_empty());
        let first = parts.next()?;
        let first_name = self.interner.intern(first);
        let mut c = self.java_top_level(p, first_name, first, true, 0)?;
        for part in parts {
            if self.syms.class(c).state() == Completion::NotStarted {
                self.complete_class(c);
            }
            let n = self.interner.intern(part);
            c = *self.syms.class(c).nested.get(&n)?;
        }
        Some(c)
    }

    fn java_top_level(&mut self, p: PkgId, name: Name, text: &str, read: bool, arity: usize) -> Option<ClassId> {
        if let Some(c) = self.demand_class(p, name) {
            if read && self.is_java_placeholder(c) {
                self.complete_class(c);
            }
            return Some(c);
        }
        if self.load_java_pkg_member(p, name) {
            return self.syms.pkg(p).entries.get(&name).and_then(|e| e.class);
        }
        if read {
            if let Some((source, cf)) = self.locate_java_class(p, text, true) {
                return Some(self.enter_java_class(Owner::Package(p), Owner::Package(p), name, source, cf, None, 0));
            }
        }
        if !text.contains('$') {
            return self.placeholder_class(p, name, arity);
        }
        None
    }
}
