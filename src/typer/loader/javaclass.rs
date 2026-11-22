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
use crate::classfile::{ClassFile, ACC_ABSTRACT, ACC_BRIDGE, ACC_ENUM, ACC_FINAL, ACC_INTERFACE, ACC_PRIVATE, ACC_PROTECTED, ACC_PUBLIC, ACC_STATIC, ACC_SYNTHETIC, ACC_VARARGS};
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

/// The root of a package's slash path, as the JDK's packages give theirs: its first segment, its first two where the
/// first is `com` or `org` (`java`, `jdk`, `com.sun`, `org.w3c`), dotted.
pub(crate) fn jdk_root(path: &str) -> String {
    let mut segs = path.split('/').filter(|s| !s.is_empty());
    let first = segs.next().unwrap_or("");
    match (first, segs.next()) {
        ("com" | "org", Some(second)) => format!("{}.{}", first, second),
        _ => first.to_string(),
    }
}

pub struct Java {
    /// `ct.sym` for the questions of what the JDK declares, opened on the first one (`None` inside
    /// where no JDK is found).
    pub jdk_probe: Option<Option<Box<JdkProbe>>>,
    jdk: Option<Jdk>,
    pub release: Option<u32>,
    /// The pseudo file that stands for the JDK in diagnostics.
    pub jdk_file: FileId,
    jdk_missing: bool,
    /// The packages of the JDK by their id, once it is open, with their index in `Jdk::packages`.
    jdk_pkgs: FxMap<PkgId, u32>,
    /// The JDK's roots, once it is open, derived from its packages (`jdk_root`): `java`, `jdk`, `com.sun`, `org.w3c`.
    jdk_roots: Vec<String>,
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
    /// The program's Java sources' classes by their package and simple name, every top-level
    /// declaration of every selected `.java` (`OutlineJavaParser`): asked before the class path, a
    /// source standing for its class over a class file of the same name.
    pub sources: FxMap<(PkgId, Name), SourceClass>,
    /// Whether any is: the one test the loader's lookups make on a program without Java sources.
    pub has_sources: bool,
}

/// A top-level class of one of the program's Java sources (`crate::java`): its file, its unit's declarations, and
/// its record once something asked for the class.
pub struct SourceClass {
    pub file: FileId,
    pub unit: Arc<crate::java::parser::Unit>,
    pub decl: usize,
    /// For a member type, its place among the members of the declarations around it, from the top-level one
    /// (`unit.types[decl]`) in; empty for a top-level class.
    pub path: Vec<usize>,
    pub record: Option<Arc<ClassFile>>,
    /// The record's refusals owed once a Scala reader asks for the class (`Problem::when_read`).
    pub when_read: Vec<crate::java::record::Problem>,
    /// Whether its record is being built (a walk of parents that comes back to it stops).
    pub building: bool,
    /// The written paths of the names its record did not resolve (`crate::java::record::build`), which bind nothing: a
    /// check of parents skips them.
    pub unresolved: Vec<String>,
}

/// The declaration at `path` among the members of the top-level declaration `k` of `unit`, and those around it, the
/// top-level one first.
pub fn decl_at<'u>(unit: &'u crate::java::parser::Unit, k: usize, path: &[usize]) -> (&'u crate::java::parser::TypeDecl, Vec<&'u crate::java::parser::TypeDecl>) {
    let mut d = &unit.types[k];
    let mut outers = Vec::new();
    for &i in path {
        if let Some(crate::java::parser::Member::Type(t)) = d.members.get(i) {
            outers.push(d);
            d = t;
        }
    }
    (d, outers)
}

impl SourceClass {
    /// Its declaration.
    pub fn decl(&self) -> &crate::java::parser::TypeDecl {
        decl_at(&self.unit, self.decl, &self.path).0
    }
}

/// Java's name resolution for a source's record (`crate::java::record::Resolve`), answered from what the loader
/// knows without entering a class: the program's Java sources, the program's and the std's classes, the class
/// path's and the JDK's class files.
struct JavaNames<'w, 'a> {
    w: &'w mut Worker<'a>,
}

impl crate::java::record::Resolve for JavaNames<'_, '_> {
    fn class_in(&mut self, pkg: &str, simple: &str) -> Option<crate::java::record::Found> {
        let p = self.w.java_source_pkg(pkg)?;
        self.w.java_source_class_in(p, pkg, simple)
    }

    fn member_of(&mut self, outer: &str, simple: &str) -> Option<crate::java::record::Found> {
        let (pkg, top) = outer.rsplit_once('/').unwrap_or(("", outer));
        let p = self.w.java_source_pkg(pkg)?;
        let stem = format!("{}${}", top, simple);
        self.w.java_source_class_in(p, pkg, &stem)
    }

    fn package(&mut self, pkg: &str) -> bool {
        self.w.java_source_pkg(pkg).is_some()
    }

    fn parents_of(&mut self, binary: &str) -> Vec<String> {
        self.w.java_parents(binary)
    }

    fn member_types(&mut self, binary: &str) -> Vec<(String, String, u16)> {
        self.w.java_member_types(binary)
    }

    fn jdk_rooted(&mut self, pkg: &str) -> bool {
        if !self.w.java().jdk_open() && !self.w.open_jdk_for(None) {
            return false;
        }
        let root = jdk_root(pkg);
        self.w.java().jdk_roots.iter().any(|r| *r == root)
    }

    fn member_unseen(&mut self, outer: &str, _simple: &str) -> bool {
        let (pkg, stem) = outer.rsplit_once('/').unwrap_or(("", outer));
        let Some(p) = self.w.java_source_pkg(pkg) else { return false };
        let name = self.w.interner.intern(stem);
        // a program's source: its member types are read
        if self.w.java().sources.contains_key(&(p, name)) {
            return false;
        }
        // a JDK class: its index holds the API alone
        self.w.java().jdk_pkgs.contains_key(&p)
    }

    fn is_interface(&mut self, binary: &str) -> bool {
        // a program's Java source by its declaration, without its record (two interfaces bounding each other's type
        // parameters ask it of each other while their records are being built)
        let (pkg, stem) = binary.rsplit_once('/').unwrap_or(("", binary));
        let p = self.w.java_source_pkg(pkg);
        if let Some(p) = p {
            let name = self.w.interner.intern(stem);
            if let Some(s) = self.w.java().sources.get(&(p, name)) {
                return matches!(s.decl().kind, crate::java::parser::Kind::Interface | crate::java::parser::Kind::Annotation);
            }
        }
        if let Some(cf) = self.w.java_class_of_binary(binary) {
            return cf.access & ACC_INTERFACE != 0;
        }
        // a class of the program's or the std's Scala sources: a trait is an interface
        let Some(p) = p else { return false };
        let name = self.w.interner.intern(stem);
        self.w.syms.entry_at(p, name).and_then(|e| e.class).is_some_and(|c| self.w.syms.class(c).kind == ClassKind::Trait)
    }
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

/// `ct.sym` opened apart from the loader's `Jdk` to answer what the JDK declares, which enters no
/// package: `Worker::jdk_declares_nullary_method`.
pub struct JdkProbe {
    ct: CtSym,
    release: u32,
    answers: FxMap<(Box<str>, Box<str>), bool>,
}

struct Jdk {
    /// The JDK's home `jdk::find_home` gave the typing: its runtime image holds the class files `ct.sym` strips.
    home: std::path::PathBuf,
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
    /// A Java source of the program, read for its declarations (`crate::java`): the file it is.
    Source(FileId),
    /// A class file compiled into teq: the JDK's `String.sig` and `Object.sig`, whose members
    /// the builtins get without `ct.sym`.
    Embedded,
}

const STRING_SIG: &[u8] = include_bytes!("../../../std/java/String.sig");
const OBJECT_SIG: &[u8] = include_bytes!("../../../std/java/Object.sig");

/// The JDK's `java.lang.String` as std/java/String.sig records it, parsed once: the members a
/// selection on a `String` tests (`Worker::jdk_string_alternatives`) in either std, where the
/// lean std's methods of std/javalib/string.scala are its implementations.
pub fn jdk_string_class() -> Option<&'static ClassFile> {
    static PARSED: std::sync::OnceLock<Option<ClassFile>> = std::sync::OnceLock::new();
    PARSED.get_or_init(|| ClassFile::parse(STRING_SIG).ok()).as_ref()
}

pub struct JClass {
    pub source: JSource,
    pub cf: Arc<ClassFile>,
    /// For the object that holds the static members: the class.
    pub companion_of: Option<ClassId>,
    /// A non-static inner class, whose instances need an outer one.
    pub inner: bool,
    /// For a static member class: the class it is written in, whose companion owns it (none where the
    /// class it is written in is that object itself: a stand-in the std declares as an object alone).
    pub outer: Option<ClassId>,
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
            jdk_roots: Vec::new(),
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
            jdk_probe: None,
            sources: FxMap::default(),
            has_sources: false,
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
/// The method of `class` that a std member's `@jvm` template calls as its last instruction
/// (`invokevirtual java/io/InputStream.read()I`), by name and descriptor.
fn template_method<'t>(template: &'t str, class: &str) -> Option<(&'t str, &'t str)> {
    let mut words = template.split_whitespace().rev();
    let target = words.next()?;
    if !matches!(words.next()?, "invokevirtual" | "invokeinterface") {
        return None;
    }
    let (owner_name, desc) = target.split_at(target.find('(')?);
    let (owner, name) = owner_name.rsplit_once('.')?;
    (owner == class).then_some((name, desc))
}

fn split_binary(name: &str) -> (&str, &str) {
    match name.rfind('/') {
        Some(at) => (&name[..at], &name[at + 1..]),
        None => ("", name),
    }
}

/// What asking Java for the class a std class stands for gave: read by this call, read before
/// it, or nothing to read.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Absorbed {
    Now,
    Before,
    Nothing,
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

    /// The class path's source that a Java class (or the object of its statics) was read from.
    pub fn java_class_source(&self, c: ClassId) -> Option<u16> {
        let l = self.loaded.as_ref()?;
        let class = l.java.classes.get(&c)?;
        let k = class.companion_of.unwrap_or(c);
        let source = if k == c { class.source } else { l.java.classes.get(&k).map_or(class.source, |j| j.source) };
        match source {
            JSource::Cp(f) => Some(f.jar),
            _ => None,
        }
    }

    /// Whether the Java class `c` is a member class its class file's `InnerClasses` entry marks
    /// private.
    pub fn java_private_member_class(&self, c: ClassId) -> bool {
        self.java().classes.get(&c).is_some_and(|j| j.cf.own_inner_entry().is_some_and(|e| e.access & ACC_PRIVATE != 0))
    }

    /// The class of the program's Java source a class (or the object of its statics) was read from, and the
    /// source's path.
    /// Whether the build reads any Java source.
    pub fn has_java_sources(&self) -> bool {
        self.loaded.as_ref().is_some_and(|l| !l.java.sources.is_empty())
    }

    pub fn java_source_of(&self, c: ClassId) -> Option<(ClassId, &str)> {
        let java = &self.loaded.as_ref()?.java;
        let class = java.classes.get(&c)?;
        let k = class.companion_of.unwrap_or(c);
        let source = if k == c { class.source } else { java.classes.get(&k).map_or(class.source, |j| j.source) };
        match source {
            JSource::Source(file) => Some((k, self.source(file).path.as_str())),
            _ => None,
        }
    }

    pub fn is_java_class(&self, c: ClassId) -> bool {
        self.loaded.as_ref().map_or(false, |l| l.java.classes.contains_key(&c))
    }

    /// The case of the Java enum constant `s` (a static field of a Java enum's statics object), which its enum's
    /// children hold (`complete_java_class`): what a match on the enum projects the constant to.
    pub fn java_enum_case(&self, s: SymId) -> Option<ClassId> {
        let Owner::Class(obj) = self.syms.sym(s).owner else { return None };
        let c = self.syms.class(obj).companion?;
        if !self.is_java_class(c) {
            return None;
        }
        self.syms.class(c).children.iter().copied().find(|&k| self.syms.class(k).singleton == Some(s))
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
            self.error_by(crate::source::Group::Own, span, msg);
        }
        inner
    }

    fn java_file(&self, source: JSource) -> FileId {
        let loaded = self.loaded.as_ref().unwrap();
        match source {
            JSource::Cp(f) => loaded.jar_files[f.jar as usize],
            JSource::Jdk(_) | JSource::Embedded => loaded.java.jdk_file,
            JSource::Source(file) => file,
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
        self.open_jdk_for(Some(wanted))
    }

    /// `open_jdk` for a read nothing asked for by name (`None`: a loaded class's parent, read
    /// with the class): where no JDK is found it says nothing and leaves the report to a lookup
    /// that names what it wants, so that a build that needs no JDK goes on without one.
    pub(super) fn open_jdk_for(&mut self, wanted: Option<&str>) -> bool {
        if self.java().jdk.is_some() {
            return true;
        }
        if self.java().jdk_missing {
            return false;
        }
        let start = Instant::now();
        let file = self.env.file;
        let Some(home) = jdk::find_home() else {
            if let Some(wanted) = wanted {
                self.java_mut().jdk_missing = true;
                let msg = format!("no JDK found (JAVA_HOME, /usr/libexec/java_home or java on the PATH) while looking for {}", wanted);
                self.diags.error_by(crate::source::Group::Own, file, Span::default(), msg);
            }
            return false;
        };
        let path = jdk::ct_sym_path(&home);
        let ct = match CtSym::open(&path.to_string_lossy()) {
            Ok(ct) => ct,
            Err(e) => {
                if wanted.is_some() {
                    self.java_mut().jdk_missing = true;
                    self.diags.error_by(crate::source::Group::Own, file, Span::default(), e);
                }
                return false;
            }
        };
        let release = match self.java().release {
            Some(r) if ct.releases.contains(&r) => r,
            Some(r) => {
                if wanted.is_some() {
                    self.java_mut().jdk_missing = true;
                    let msg = format!("{}: release {} is not in it (it holds {} to {})", path.display(), r, ct.releases[0], ct.latest_release());
                    self.diags.error_by(crate::source::Group::Own, file, Span::default(), msg);
                }
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
                p = self.syms.sub_pkg(self.interner, p, n);
            }
            self.java_mut().jdk_pkgs.insert(p, i as u32);
        }
        let mut roots: Vec<String> = packages.iter().map(|(path, _)| jdk_root(path)).collect();
        roots.sort();
        roots.dedup();
        let j = self.java_mut();
        j.jdk_roots = roots;
        j.stats.ct_open = Some(opened);
        j.stats.ct_index = start.elapsed();
        j.stats.release = release;
        j.jdk = Some(Jdk { home, ct, packages, names: FxMap::default(), parsed: FxMap::default() });
        true
    }

    /// The class file of the Java class `c` with its code, from where the loader selected it: its class path entry
    /// (the class path's order, a multi-release jar's versioned entry), or the resource of its `ct.sym` entry's class
    /// in the runtime image of the JDK home the typing opened. `ct.sym`'s own files keep no code.
    pub(crate) fn java_class_bytes(&self, c: ClassId) -> Result<Vec<u8>, String> {
        let l = self.loaded.as_ref().ok_or("no class path is loaded")?;
        let class = l.java.classes.get(&c).ok_or("the loader read no class file for it")?;
        match class.source {
            JSource::Cp(f) => l.cp.raw_bytes(f),
            JSource::Jdk(entry) => {
                let jdk = l.java.jdk.as_ref().ok_or("the JDK is not open")?;
                let name = self.jdk_runtime_path(entry).ok_or("its ct.sym entry names no class")?;
                let name = name.strip_prefix("/modules").unwrap_or(&name);
                crate::classfile::jimage::with_shared(&jdk.home, |img| img.resource(name).map_err(|e| e.to_string()))?
            }
            JSource::Source(_) => Err("it is one of the program's Java sources, read for its declarations alone".to_string()),
            JSource::Embedded => Err("its class file is the one compiled into teq, without code".to_string()),
        }
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

    /// Whether the JDK's class `binary` (`java/time/Duration`) declares a method `name` without
    /// parameters, as its class file in `ct.sym` has it, read through a handle of its own: the
    /// loader's tables stay as they are, whatever the order of the questions. `None` without a
    /// JDK.
    pub(in crate::typer) fn jdk_declares_nullary_method(&mut self, binary: &str, name: &str) -> Option<bool> {
        self.loaded.as_ref()?;
        self.with_loader_for(crate::measure::Hold::JarEntry, |w| w.jdk_declares_nullary_method_unlocked(binary, name))
    }

    fn jdk_declares_nullary_method_unlocked(&mut self, binary: &str, name: &str) -> Option<bool> {
        let java = &mut self.loaded_mut().java;
        if java.jdk_probe.is_none() {
            let release = java.release;
            let opened = jdk::find_home().and_then(|home| CtSym::open(&jdk::ct_sym_path(&home).to_string_lossy()).ok()).and_then(|ct| {
                let release = match release {
                    Some(r) if ct.releases.contains(&r) => r,
                    Some(_) => return None,
                    None => ct.latest_release(),
                };
                Some(Box::new(JdkProbe { ct, release, answers: FxMap::default() }))
            });
            java.jdk_probe = Some(opened);
        }
        let probe = java.jdk_probe.as_mut()?.as_mut()?;
        let key: (Box<str>, Box<str>) = (binary.into(), name.into());
        if let Some(&known) = probe.answers.get(&key) {
            return Some(known);
        }
        let package = binary.rfind('/').map_or("", |at| &binary[..at]);
        let entry = probe.ct.package(probe.release, package).into_iter().find(|e| e.class == binary).map(|e| e.entry);
        let declared = entry.and_then(|e| probe.ct.zip.read(e).ok()).and_then(|bytes| ClassFile::parse(&bytes).ok()).is_some_and(|cf| {
            cf.methods.iter().any(|m| m.name == name && m.descriptor.starts_with("()"))
        });
        probe.answers.insert(key, declared);
        Some(declared)
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

    /// Enters the program's Java sources (`crate::java`): every top-level declaration of every `.java` under its
    /// package and simple name (`OutlineJavaParser`), the package made where nothing else made it, for
    /// `locate_java_class` to find before the class path. What the reader refuses is reported at its place: the
    /// scanner's and the parser's errors in the reference's words, and a form past this reader ("not supported yet").
    pub fn enter_java_sources(&mut self) {
        let mut found = Vec::new();
        for i in 0..self.files.len() {
            let f = &self.files[i];
            if f.is_std || !f.path.ends_with(".java") {
                continue;
            }
            found.push((FileId(i as u32), crate::java::parser::parse(&f.text)));
        }
        // TEQ_JAVA_RECORDS=raw prints each source's top-level declarations as they are entered, by binary name with
        // where each starts and where its name stands (`DECL`), and a compact source's first member (`COMPACT`): what
        // a fence places a refusal by.
        let raw = std::env::var("TEQ_JAVA_RECORDS").as_deref() == Ok("raw");
        for (file, unit) in found {
            if raw {
                let path = &self.source(file).path;
                let pkg = unit.package.join("/");
                let mut out = String::new();
                if let Some(at) = unit.compact {
                    out.push_str(&format!("COMPACT\t{}\t{}\n", path, at));
                }
                for d in &unit.types {
                    out.push_str(&format!("DECL\t{}\t{}\t{}\t{}\n", path, if pkg.is_empty() { d.name.clone() } else { format!("{}/{}", pkg, d.name) }, d.start, d.offset));
                }
                eprint!("{}", out);
            }
            for (at, msg) in &unit.errors {
                self.diags.error_by(crate::source::Group::Parser, file, Span { start: *at, end: *at + 1 }, msg.clone());
            }
            // A member outside any type, a compact source file: the reference drops the whole unit (`compilationUnit`'s
            // EmptyTree), without a word; its classes no Scala source then finds (E006 where one names them).
            if unit.compact.is_some() {
                continue;
            }
            let unit = Arc::new(unit);
            let mut p = ROOT_PKG;
            for seg in &unit.package {
                let name = self.interner.intern(seg);
                p = self.syms.sub_pkg(self.interner, p, name);
            }
            for (k, decl) in unit.types.iter().enumerate() {
                for (at, msg) in crate::java::conflicts(decl) {
                    self.diags.error_by(crate::source::Group::Typer, file, Span { start: at, end: at + 1 }, msg);
                }
                let key = self.interner.intern(&decl.name);
                // A second declaration of one class (`Namer.checkNoConflict`, E161): refused where it stands, the first
                // one's statics object already defined; the first is the class.
                if let Some(first) = self.java().sources.get(&(p, key)).map(|s| s.file) {
                    let path = self.source(first).path.clone();
                    let msg = format!("{} is already defined as object {} in {}", decl.name, decl.name, path);
                    self.diags.error_by(crate::source::Group::Typer, file, Span { start: decl.start, end: decl.start + 1 }, msg);
                    continue;
                }
                let java = &mut self.loaded_mut().java;
                java.sources.insert((p, key), SourceClass { file, unit: unit.clone(), decl: k, path: Vec::new(), record: None, when_read: Vec::new(), building: false, unresolved: Vec::new() });
                java.has_sources = true;
                // Its member types, at any depth, each a class of its own by its binary name's stem (`Outer$Inner`),
                // which the loader locates as it locates a class file's member class.
                let mut stack: Vec<(String, Vec<usize>)> = vec![(decl.name.clone(), Vec::new())];
                while let Some((stem, path)) = stack.pop() {
                    let (d, _) = decl_at(&unit, k, &path);
                    for (i, m) in d.members.iter().enumerate() {
                        let crate::java::parser::Member::Type(t) = m else { continue };
                        for (at, msg) in crate::java::conflicts(t) {
                            self.diags.error_by(crate::source::Group::Typer, file, Span { start: at, end: at + 1 }, msg);
                        }
                        let member = format!("{}${}", stem, t.name);
                        let mut mpath = path.clone();
                        mpath.push(i);
                        let mkey = self.interner.intern(&member);
                        self.loaded_mut().java.sources.insert((p, mkey), SourceClass { file, unit: unit.clone(), decl: k, path: mpath.clone(), record: None, when_read: Vec::new(), building: false, unresolved: Vec::new() });
                        stack.push((member, mpath));
                    }
                }
            }
        }
    }

    /// Every Java source's record built and its problems reported, as scalac types every Java unit's signatures
    /// whether a Scala source names its classes or not: once the Scala sources' classes are entered (a Java
    /// declaration naming one is told so), in the sources' order. Nothing to do without Java sources.
    pub fn build_java_source_records(&mut self) {
        let Some(loaded) = self.loaded.as_ref() else { return };
        if loaded.java.sources.is_empty() {
            return;
        }
        let mut keys: Vec<(FileId, u32, PkgId, Name)> = loaded.java.sources.iter().filter(|(_, s)| s.record.is_none()).map(|(&(p, n), s)| (s.file, s.decl().start, p, n)).collect();
        keys.sort_by_key(|&(f, at, _, _)| (f.0, at));
        for (_, _, p, n) in keys {
            self.java_source_record(p, n, false);
        }
    }

    /// The run's Java sources whose class extends or implements the class `binary` (a sealed Java source's children,
    /// `Namer.registerIfChildInCreationContext` as each is entered), by binary name, in the sources' order (file, then
    /// place), whatever was read before: every source's record is built for it.
    fn java_source_children(&mut self, binary: &str) -> Vec<String> {
        let Some(loaded) = self.loaded.as_ref() else { return Vec::new() };
        let mut keys: Vec<(FileId, u32, PkgId, Name)> = loaded.java.sources.iter().map(|(&(p, n), s)| (s.file, s.decl().start, p, n)).collect();
        keys.sort_by_key(|&(f, at, _, _)| (f.0, at));
        let mut out = Vec::new();
        for (_, _, p, n) in keys {
            let (_, r) = self.java_source_record(p, n, false);
            if r.name != binary && (r.superclass.as_deref() == Some(binary) || r.interfaces.iter().any(|i| i == binary)) {
                out.push(r.name.clone());
            }
        }
        out
    }

    /// A Java source's parents as the reference checks them (`Checking.checkParents`, `checkNonCyclic`), once every
    /// record is built: a final class extended (E093, at the class's name), a class implemented as an interface
    /// (E104, at its name), a chain of parents through the sources or the class path back to the class (E110, at its
    /// parent; once a cycle). An interface extended as a class and an interface extending `Object` are accepted, as
    /// scalac accepts them.
    pub fn check_java_source_parents(&mut self) {
        let Some(loaded) = self.loaded.as_ref() else { return };
        if loaded.java.sources.is_empty() {
            return;
        }
        #[allow(clippy::type_complexity)]
        let mut entries: Vec<(FileId, u32, Arc<crate::java::parser::Unit>, (usize, Vec<usize>), Arc<ClassFile>, Vec<String>)> = loaded
            .java
            .sources
            .values()
            .filter_map(|s| s.record.clone().map(|r| (s.file, s.decl().start, s.unit.clone(), (s.decl, s.path.clone()), r, s.unresolved.clone())))
            .collect();
        entries.sort_by_key(|e| (e.0 .0, e.1));
        let at_of = |t: Option<&crate::java::parser::Ty>| match t {
            Some(crate::java::parser::Ty::Named(_, at)) => Some(*at),
            _ => None,
        };
        let simple = |b: &str| b.rsplit(['/', '$']).next().unwrap_or(b).to_string();
        let mut cyclic: std::collections::HashSet<String> = std::collections::HashSet::new();
        for (file, _, unit, (k, path), cf, unresolved) in entries {
            let decl = decl_at(&unit, k, &path).0;
            let mut errors: Vec<(u32, String)> = Vec::new();
            // a parent its record did not resolve names no class: nothing to check
            let resolved = |b: &String| !unresolved.contains(b);
            if let Some(sup) = cf.superclass.as_ref().filter(|b| resolved(b)) {
                if self.java_class_of_binary(sup).is_some_and(|p| p.access & ACC_FINAL != 0) {
                    errors.push((decl.offset, format!("class {} cannot extend final class {}", decl.name, simple(sup))));
                }
            }
            if decl.kind != crate::java::parser::Kind::Interface {
                for (i, iface) in cf.interfaces.iter().enumerate().filter(|(_, b)| resolved(b)) {
                    if self.java_class_of_binary(iface).is_some_and(|p| p.access & ACC_INTERFACE == 0) {
                        if let Some(at) = at_of(decl.interfaces.get(i)) {
                            errors.push((at, format!("class {} is not a trait", simple(iface))));
                        }
                    }
                }
            }
            if !cyclic.contains(&cf.name) {
                let mut stack: Vec<String> = cf.superclass.iter().chain(&cf.interfaces).filter(|b| resolved(b)).cloned().collect();
                let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
                let mut back = false;
                while let Some(b) = stack.pop() {
                    if b == cf.name {
                        back = true;
                        break;
                    }
                    if !seen.insert(b.clone()) {
                        continue;
                    }
                    stack.extend(self.java_parents(&b));
                }
                if back {
                    let parent = decl.superclass.as_ref().or(decl.interfaces.first());
                    if let Some(at) = at_of(parent) {
                        let kind = if decl.kind == crate::java::parser::Kind::Interface { "trait" } else { "class" };
                        errors.push((at, format!("Cyclic inheritance: {} {} extends itself", kind, decl.name)));
                    }
                    cyclic.extend(seen);
                    cyclic.insert(cf.name.clone());
                }
            }
            for (at, msg) in errors {
                self.diags.error_by(crate::source::Group::Typer, file, Span { start: at, end: at + 1 }, msg);
            }
        }
    }

    /// The record or class file of a Java class by its binary name (`java/lang/String`, `p/Outer$Inner`), as the loader
    /// locates it: a program's Java source first.
    fn java_class_of_binary(&mut self, binary: &str) -> Option<Arc<ClassFile>> {
        let (pkg, stem) = binary.rsplit_once('/').unwrap_or(("", binary));
        let p = self.java_source_pkg(pkg)?;
        // a source's record read for its parents, not asked for by a Scala reader
        let key = self.interner.intern(stem);
        if let Some(s) = self.java().sources.get(&(p, key)) {
            // none while it is being built
            if s.building {
                return s.record.clone();
            }
            return Some(self.java_source_record(p, key, false).1);
        }
        self.locate_java_class(p, stem, true).map(|(_, cf)| cf)
    }

    /// The superclass and the interfaces of the Java class `binary` that a check reads: its record's or its class
    /// file's, but a name a source's record did not resolve, which names no class.
    fn java_parents(&mut self, binary: &str) -> Vec<String> {
        let Some(cf) = self.java_class_of_binary(binary) else { return Vec::new() };
        let unresolved = self.java_unresolved(binary);
        cf.superclass.iter().chain(&cf.interfaces).filter(|b| !unresolved.contains(b)).cloned().collect()
    }

    /// The names the record of the program's Java class `binary` did not resolve; none for a class file.
    fn java_unresolved(&mut self, binary: &str) -> Vec<String> {
        let (pkg, stem) = binary.rsplit_once('/').unwrap_or(("", binary));
        let Some(p) = self.java_source_pkg(pkg) else { return Vec::new() };
        let key = self.interner.intern(stem);
        self.java().sources.get(&(p, key)).map(|s| s.unresolved.clone()).unwrap_or_default()
    }

    /// The member types the Java class `binary` declares, whatever their access, each by its simple and its binary
    /// name: a program's Java source's member declarations, a class file's `InnerClasses` entries of which it is the
    /// outer class.
    fn java_member_types(&mut self, binary: &str) -> Vec<(String, String, u16)> {
        let (pkg, stem) = binary.rsplit_once('/').unwrap_or(("", binary));
        let Some(p) = self.java_source_pkg(pkg) else { return Vec::new() };
        let key = self.interner.intern(stem);
        if let Some(s) = self.java().sources.get(&(p, key)) {
            let decl = s.decl();
            return decl
                .members
                .iter()
                .filter_map(|m| match m {
                    crate::java::parser::Member::Type(t) => Some((t.name.clone(), format!("{}${}", binary, t.name), crate::java::record::member_access(t, decl))),
                    _ => None,
                })
                .collect();
        }
        let Some((_, cf)) = self.locate_java_class(p, stem, true) else { return Vec::new() };
        cf.inner_classes.iter().filter(|i| i.outer.as_deref() == Some(binary)).filter_map(|i| i.name.clone().map(|n| (n, i.inner.clone(), i.access))).collect()
    }

    /// The package of a Java name's path (`java/util`), `None` where none is: the JDK opened for a JDK-rooted path
    /// that is not entered yet.
    fn java_source_pkg(&mut self, path: &str) -> Option<PkgId> {
        let lookup = |w: &mut Self| {
            let mut p = ROOT_PKG;
            for seg in path.split('/').filter(|s| !s.is_empty()) {
                let n = w.interner.intern(seg);
                p = w.syms.entry_at(p, n).and_then(|e| e.pkg)?;
            }
            Some(p)
        };
        if let Some(p) = lookup(self) {
            return Some(p);
        }
        // The JDK is opened at the first path the program's names do not hold (every package of its index entered),
        // whatever was resolved before, so that a package's answer does not depend on the order the names come in.
        if !self.java().jdk_open() && self.open_jdk_for(None) {
            return lookup(self);
        }
        None
    }

    /// The class `stem` of package `p` (path `pkg`) as a Java name finds it, without entering it.
    fn java_source_class_in(&mut self, p: PkgId, pkg: &str, stem: &str) -> Option<crate::java::record::Found> {
        let binary = if pkg.is_empty() { stem.to_string() } else { format!("{}/{}", pkg, stem) };
        let found = |run_scala| Some(crate::java::record::Found { binary: binary.clone(), run_scala });
        let name = self.interner.intern(stem);
        if self.java().sources.contains_key(&(p, name)) {
            return found(false);
        }
        // A class or an object of the std (entered for the lookup) or of the program, where the JVM holds a class of
        // this name: no type alias, no opaque type, no std stand-in of a JDK class (`scala.String`, whose class is
        // `java.lang.String`), which javac does not see; the lookup goes on past one, as javac's does.
        if self.std.index().by_name.contains_key(&(p, name)) {
            self.demand_std(p, name, crate::stdindex::TYPE | crate::stdindex::TERM);
        }
        let entered = self.syms.entry_at(p, name).and_then(|e| e.class.or_else(|| e.term.and_then(|t| match self.syms.sym(t).kind {
            SymKind::Object(c) => Some(c),
            _ => None,
        })));
        if let Some(c) = entered.filter(|&c| self.jvm_class_named(c, &binary)) {
            let f = self.syms.class(c).file;
            let src = self.source(f);
            let program_scala = !src.is_std && !self.in_jar(f) && !src.path.ends_with(".java") && !self.is_java_class(c);
            return found(program_scala);
        }
        // `Foo$`, the class of the program's Scala object `Foo` (its JVM name), is that object's.
        if let Some(object) = stem.strip_suffix('$').filter(|o| !o.is_empty()) {
            let oname = self.interner.intern(object);
            let entry = self.syms.entry_at(p, oname);
            let file = entry.as_ref().and_then(|e| e.term.map(|t| self.syms.sym(t).file).or(e.class.filter(|&c| !self.is_java_class(c)).map(|c| self.syms.class(c).file)));
            if let Some(f) = file {
                let src = self.source(f);
                if !src.is_std && !self.in_jar(f) && !src.path.ends_with(".java") {
                    return found(true);
                }
            }
        }
        if let Some(slot) = self.loaded.as_ref().unwrap().slot(p) {
            if self.java_class_file_named(slot, stem).is_some() || self.loaded.as_ref().unwrap().pkg_names(slot).contains_key(stem) {
                return found(false);
            }
        }
        // the JDK's class, its package one of the JDK's (`jdk_entry`); the JDK missing is told where the name looks the
        // JDK's
        let open = self.java().jdk_open() || if self.jdk_rooted(p) { self.open_jdk(&binary.replace('/', ".")) } else { self.open_jdk_for(None) };
        if open && self.jdk_entry(p, stem).is_some() {
            return found(false);
        }
        None
    }

    /// Whether the JVM holds the class `c` by the binary name `binary`: not an opaque type, nor a builtin (teq's own
    /// `scala.Int`, `scala.String`, `scala.Array`: javac sees the class path's class files, `scala/Int.class` of
    /// scala-library, which the lookup reaches after this), nor a std stand-in of a JDK class (one whose `@jvmClass`
    /// names another class, or a `@javaDefined` one outside the JDK's packages).
    fn jvm_class_named(&self, c: ClassId, binary: &str) -> bool {
        if matches!(self.syms.class(c).kind, ClassKind::Opaque | ClassKind::Builtin) {
            return false;
        }
        match self.std_jvm_class(c) {
            Some(j) => j == binary,
            None => !(self.java_stand_in(c) && !self.jdk_rooted(self.package_of_class(c))),
        }
    }

    /// The record of the program's Java class `name` of `p`, built the first time it is asked for, its problems
    /// reported at their places in the source; a refusal owed only to a Scala reader (`Problem::when_read`) is held
    /// until one asks (`asked`).
    fn java_source_record(&mut self, p: PkgId, name: Name, asked: bool) -> (JSource, Arc<ClassFile>) {
        let (file, unit, decl, path, record) = {
            let s = &self.java().sources[&(p, name)];
            (s.file, s.unit.clone(), s.decl, s.path.clone(), s.record.clone())
        };
        if let Some(r) = record {
            if asked {
                let owed = self.loaded_mut().java.sources.get_mut(&(p, name)).map(|s| std::mem::take(&mut s.when_read)).unwrap_or_default();
                for pr in owed {
                    self.diags.error_by(crate::source::Group::Own, file, Span { start: pr.offset, end: pr.offset + 1 }, pr.message);
                }
            }
            return (JSource::Source(file), r);
        }
        let text = self.source(file).text.clone();
        if let Some(s) = self.loaded_mut().java.sources.get_mut(&(p, name)) {
            s.building = true;
        }
        let (d, outers) = decl_at(&unit, decl, &path);
        let (cf, problems, unresolved) = crate::java::record::build(&unit, &outers, d, &text, &mut JavaNames { w: self });
        let mut owed = Vec::new();
        for pr in problems {
            if pr.when_read && !asked {
                owed.push(pr);
                continue;
            }
            self.diags.error_by(crate::source::Group::Own, file, Span { start: pr.offset, end: pr.offset + 1 }, pr.message);
        }
        // TEQ_JAVA_RECORDS=raw prints each record a member a line with the problems held for a Scala reader
        // (`record::raw`), to set beside javac's class files; any other value prints it as `teq classfile --full`
        // prints a class file.
        match std::env::var("TEQ_JAVA_RECORDS").as_deref() {
            Ok("raw") => eprint!("{}", crate::java::record::raw(&cf, &owed)),
            Ok(_) => {
                let mut printer = crate::classfile::show::Printer::new(&cf, true);
                let pkg = printer.package();
                printer.class();
                eprint!("// {} (Java source record)\n{}{}", self.source(file).path, if pkg.is_empty() { String::new() } else { format!("package {}\n", pkg) }, printer.out);
            }
            Err(_) => {}
        }
        let cf = Arc::new(cf);
        if let Some(s) = self.loaded_mut().java.sources.get_mut(&(p, name)) {
            s.record = Some(cf.clone());
            s.when_read = owed;
            s.building = false;
            s.unresolved = unresolved;
        }
        (JSource::Source(file), cf)
    }

    /// The class file `name` of the classpath package `slot`, or of the JDK package `p`; a Java source of the
    /// program's first (`java_source_record`).
    fn locate_java_class(&mut self, p: PkgId, name: &str, open_jdk: bool) -> Option<(JSource, Arc<ClassFile>)> {
        // A Java source of the program stands for its class over a class file of the same name.
        if self.java().has_sources {
            let key = self.interner.intern(name);
            if self.java().sources.contains_key(&(p, key)) {
                return Some(self.java_source_record(p, key, true));
            }
        }
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
        // A Java class or package has an identifier's name, its own key (`Symbols::entry_at`): a name encoding
        // changes names none of them.
        let key = name;
        // The std is written against itself; `java.lang.String` and `Object` are the builtins.
        // On the JVM a class of the JavaScript layer alone, and a JDK class no std file defines
        // (`java.time.Instant`, which JavaScript takes from scala-java-time's jar), are the JDK's
        // for the std too.
        let file = self.env.file;
        if self.source(file).is_std && !self.in_jar(file) {
            let jdk_only = || self.pkg_path(p).starts_with("java.") && !self.std.index().by_name.contains_key(&(p, name));
            if !(self.jvm && (crate::js_only_class(self.interner.get(name), || self.pkg_path(p)) || jdk_only())) {
                return false;
            }
        }
        if p == self.java_lang_pkg_id() && (name == names::STRING || self.interner.get(name) == "Object") {
            return false;
        }
        // An object the std defines under the name (`scala.runtime.Statics`) stands for the
        // Java class.
        self.demand_std(p, name, crate::stdindex::TYPE | crate::stdindex::TERM);
        if self.syms.entry_at(p, key).map_or(false, |e| e.term.is_some() && e.class.is_none()) {
            return false;
        }
        let existing = self.syms.entry_at(p, key).and_then(|e| e.class);
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
            if self.syms.entry_at(p, key).map_or(false, |e| e.pkg.is_some()) {
                return true;
            }
        }
        let Some((source, cf)) = self.locate_java_class(p, &text, false) else { return false };
        self.enter_java_class(Owner::Package(p), Owner::Package(p), name, source, cf, None);
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
        let java_lang = self.syms.entry_at(ROOT_PKG, names::JAVA).and_then(|e| e.pkg).and_then(|j| self.syms.entry_at(j, names::LANG).and_then(|e| e.pkg));
        if Some(p) == java_lang && (name == names::STRING || self.interner.get(name) == "Object") {
            return true;
        }
        if self.jdk_rooted(p) {
            return false;
        }
        self.demand_std(p, name, crate::stdindex::TYPE | crate::stdindex::TERM);
        if let Some(e) = self.syms.entry_at(p, name) {
            if e.term.is_some() && e.class.is_none() {
                return true;
            }
            if e.class.is_some() {
                return false;
            }
        }
        // A class of the program's Java sources not entered yet is the lock's to enter.
        let java = &self.loaded.as_ref().unwrap().java;
        if java.has_sources && java.sources.contains_key(&(p, name)) {
            return false;
        }
        match slot {
            Some(slot) => match self.loaded.as_ref().unwrap().java_names(slot) {
                Some(java) => !java.contains_key(self.interner.get(name)),
                None => false,
            },
            None => true,
        }
    }

    /// The Java class that the package `p` stands for, a pickle having reached its statics through its companion
    /// (`placeholder_static_nested`): the class of `p`'s name in `p`'s parent, entered.
    fn java_class_of_pkg(&self, p: PkgId) -> Option<ClassId> {
        let info = self.syms.pkg(p);
        let parent = info.parent?;
        let c = self.syms.entry_at(parent, info.name).and_then(|e| e.class)?;
        (self.is_java_class(c) && !self.is_java_placeholder(c)).then_some(c)
    }

    /// The static member class `name` of the Java class the package `pkg` stands for, where that class is entered:
    /// what a pickle's `Outer.Nested` through the companion names.
    pub(super) fn entered_java_static_member(&self, pkg: PkgId, name: Name) -> Option<ClassId> {
        let outer = self.java_class_of_pkg(pkg)?;
        let obj = self.syms.class(outer).companion?;
        self.syms.class(obj).nested.get(&name).copied()
    }

    /// Reads the class a placeholder stands for and completes it; a class no source holds
    /// stays the empty trait it was. A placeholder of a static member class a pickle reached through its outer
    /// class's companion is filled where that outer class is entered (`enter_java_nested`), the outer entered here.
    pub(super) fn complete_java_placeholder(&mut self, c: ClassId) {
        let Some((p, name)) = self.java().placeholder(c) else { return };
        let text = self.interner.get(name).to_string();
        match self.locate_java_class(p, &text, true) {
            Some((source, cf)) => {
                self.enter_java_class(Owner::Package(p), Owner::Package(p), name, source, cf, Some(c));
                self.complete_java_class(c);
            }
            None => {
                if self.placeholder_static_nested(p, name) == Some(true) {
                    let info = self.syms.pkg(p);
                    if let Some(parent) = info.parent {
                        let outer = info.name;
                        self.load_java_pkg_member(parent, outer);
                        if !self.is_java_placeholder(c) {
                            self.complete_java_class(c);
                            return;
                        }
                    }
                }
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
        // A class file's `PermittedSubclasses` is not read (dotty's ClassfileParser: a sealed Java class of the class
        // path is open to a Scala subclass, and its match is not checked by its children); a Java source's `sealed` is
        // (`JavaParsers.modifiers`), its children the run's sources that extend it.
        if cf.sealed {
            m |= mods::SEALED;
        }
        if cf.access & crate::classfile::ACC_ANNOTATION != 0 {
            m |= mods::JAVA_ANNOTATION;
        }
        // A member class records whether it is static in its own `InnerClasses` entry.
        if cf.inner_classes.iter().any(|i| i.inner == cf.name && i.outer.is_some() && i.access & ACC_STATIC != 0) {
            m |= mods::JAVA_STATIC;
        }
        // Universal traits as dotty's `ClassfileParser.parseParents` reads them; `Comparable`, of
        // that list too, has its `Object` back from its generic signature.
        if matches!(cf.name.as_str(), "java/io/Serializable" | "java/lang/Cloneable") {
            m |= mods::UNIVERSAL;
        }
        // A static member class is a declaration of the outer class's companion, where its own
        // companion is; an inner class one of the class (`ClassfileParser.getOwner` and `getScope`:
        // the module class and the static scope for the one, the class and the instance scope for
        // the other).
        let (owner, outer) = match (owner, term_owner) {
            (Owner::Class(written_in), Owner::Class(obj)) if m & mods::JAVA_STATIC != 0 => (Owner::Class(obj), (written_in != obj).then_some(written_in)),
            _ => (owner, None),
        };
        // A placeholder settled as an empty trait before its class file was entered (`complete_java_placeholder`, its
        // outer a std class then, whose JDK member classes are entered later): read from the class file as it is filled.
        let settled = placeholder.filter(|&k| self.syms.class(k).state() == Completion::Done);
        let c = match placeholder {
            Some(c) => {
                let mut info = self.syms.class_mut(c);
                info.kind = kind;
                info.mods = m;
                info.file = file;
                info.base_types.clear();
                // a member class's placeholder (`enter_java_nested`) is the member of its outer class
                if let Owner::Class(_) = owner {
                    info.owner = owner;
                }
                c
            }
            None => self.syms.new_class(name, kind, m, owner, file, None, Span::default()),
        };
        self.mark_inner_class(c);
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
        }
        if placeholder.is_none() || matches!(owner, Owner::Class(_)) {
            match owner {
                Owner::Package(p) => self.syms.enter_scoped(self.interner, p, name).class = Some(c),
                Owner::Class(o) => {
                    let mut info = self.syms.class_mut(o);
                    let old = info.nested.insert(name, c);
                    info.order_nested(old, c);
                    crate::symbols::type_member_written(name);
                }
                Owner::Local => {}
            }
        }
        let inner = cf.inner_outer().is_some();
        self.java_mut().classes.replace(c, JClass { source, cf: cf.clone(), companion_of: None, inner, outer });
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
            Owner::Package(p) => self.syms.entry_named(self.interner, p, name).and_then(|e| e.term).map_or(false, |t| self.source(self.syms.sym(t).file).is_std && !self.in_jar(self.syms.sym(t).file)),
            _ => false,
        };
        if !std_object {
            self.register_term(term_owner, sym);
        }
        self.java_mut().classes.replace(obj, JClass { source, cf: cf.clone(), companion_of: Some(c), inner: false, outer: None });
        self.java_mut().stats.classes_entered += 1;
        // A top-level class enters its member types with itself, for reads that complete no class; a member type
        // enters its own as it is completed (`complete_java_class`), so that every depth is reached on request.
        if matches!(owner, Owner::Package(_)) {
            self.enter_java_nested(c, obj, &cf);
        }
        if settled.is_some() {
            self.complete_java_class(c);
        }
        c
    }

    /// The nested classes of `c`, each its own class file next to the outer's.
    fn enter_java_nested(&mut self, c: ClassId, obj: ClassId, cf: &ClassFile) {
        let mut p = self.syms.class(c).owner;
        while let Owner::Class(o) = p {
            p = self.syms.class(o).owner;
        }
        let Owner::Package(pkg) = p else { return };
        let nested: Vec<(String, String)> = cf
            .member_classes()
            .filter_map(|i| i.name.as_ref().map(|n| (i.inner.clone(), n.clone())))
            .collect();
        // A pickle that reached a static member class through the outer class's companion (entered as a package of the
        // class's name) left a placeholder there: the member class fills it, one class whichever came first.
        let class_name = self.syms.class(c).name;
        let as_pkg = match self.syms.class(c).owner {
            Owner::Package(op) => self.syms.entry_at(op, class_name).and_then(|e| e.pkg),
            _ => None,
        };
        for (binary, simple) in nested {
            let (_, stem) = split_binary(&binary);
            let name = self.interner.intern(&simple);
            if self.syms.class(c).nested.contains_key(&name) || self.syms.class(obj).nested.contains_key(&name) {
                continue;
            }
            let Some((source, ncf)) = self.locate_java_class(pkg, stem, false) else { continue };
            let placeholder = as_pkg.and_then(|ap| self.syms.entry_at(ap, name).and_then(|e| e.class)).filter(|&k| self.is_java_placeholder(k));
            self.enter_java_class(Owner::Class(c), Owner::Class(obj), name, source, ncf, placeholder);
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
        // Its member types, entered as it is completed, as dotty's `ClassfileParser.enterOwnInnerClasses` enters a class
        // file's own inner classes as it reads it: one level, each of them entering the next as it is completed.
        if obj != c {
            self.enter_java_nested(c, obj, &cf);
        }
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
        if cf.sealed {
            for binary in self.java_source_children(&cf.name) {
                if let Some(child) = self.java_class_by_binary(&binary, false, 0) {
                    if child != c && self.syms.class(child).kind != ClassKind::Builtin && !self.syms.class(c).children.contains(&child) {
                        self.syms.class_mut(c).children.push(child);
                    }
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
            // An annotation interface's element with a default (`AnnotationDefault`): an annotation may leave it out.
            if m.annotation_default {
                mm |= mods::JAVA_ANNOTATION;
            }
            let sym = self.syms.new_sym(name, SymKind::Def, mm, Owner::Class(owner), file, None, Span::default());
            self.syms.sym_mut(sym).java_defined = true;
            self.java_mut().syms.insert(sym, JSym { class: c, member: JMember::Method(i as u32) });
            self.register_term(Owner::Class(owner), sym);
        }
        let mut enum_constants = Vec::new();
        for (i, f) in cf.fields.iter().enumerate() {
            if !visible(f.access) {
                continue;
            }
            let is_static = f.access & ACC_STATIC != 0;
            let owner = if is_static { obj } else { c };
            let name = self.interner.intern(&f.name);
            let (kind, mut fm) = if f.access & ACC_FINAL != 0 || f.access & ACC_ENUM != 0 {
                (SymKind::Val, mods::FIELD | constant_final(f))
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
            if is_static && f.access & ACC_ENUM != 0 && f.descriptor.strip_prefix('L').and_then(|d| d.strip_suffix(';')) == Some(cf.name.as_str()) {
                enum_constants.push(sym);
            }
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
        // Once the enum is complete (a parent under way gives its case no base types): a Java enum is sealed, abstract,
        // its constants its children (ClassfileParser's `ACC_ENUM` fields, JavaParsers'
        // `JavaEnumValue`s through `Namer.addEnumConstants`): each a singleton case whose value is the constant's
        // field, which a match projects (`space::java_enum_case`); the field stays the static field it is.
        if !enum_constants.is_empty() {
            if self.syms.class(c).mods & mods::SEALED == 0 {
                self.syms.class_mut(c).mods |= mods::SEALED | mods::ABSTRACT;
            }
            let enum_ty = self.types.class(c, &[]);
            for f in enum_constants {
                let name = self.syms.sym(f).name;
                let case = self.syms.new_class(name, ClassKind::EnumCase, mods::FINAL, Owner::Class(obj), file, None, Span::default());
                self.syms.class_mut(case).singleton = Some(f);
                self.set_loaded_parents(case, vec![enum_ty]);
                self.syms.class_mut(case).state().set(Completion::Done);
                if !self.syms.class(c).children.contains(&case) {
                    self.syms.class_mut(c).children.push(case);
                }
            }
        }
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
    pub(crate) fn std_jvm_class(&self, c: ClassId) -> Option<String> {
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

    /// The superclass the loader's class file of the Java class `c` names (`java/lang/Object`), `Some(None)` for none;
    /// `None` where the loader read no class file for it.
    pub(crate) fn java_class_superclass(&self, c: ClassId) -> Option<Option<String>> {
        let l = self.loaded.as_ref()?;
        Some(l.java.classes.get(&c)?.cf.superclass.clone())
    }

    /// Whether the loader read the Java class `c` from the JDK (its `ct.sym` entry, its class file the runtime image's),
    /// not from the class path.
    pub(crate) fn java_class_from_jdk(&self, c: ClassId) -> bool {
        self.loaded.as_ref().and_then(|l| l.java.classes.get(&c)).is_some_and(|k| matches!(k.source, JSource::Jdk(_)))
    }

    /// The JVM descriptor of the Java constructor `s` (`(II)V`), as its class file declares it; `None` for a symbol
    /// no class file declares.
    pub(crate) fn java_ctor_descriptor(&self, s: SymId) -> Option<String> {
        let java = &self.loaded.as_ref()?.java;
        let js = *java.syms.get(&s)?;
        let JMember::Method(i) = js.member else { return None };
        let m = java.classes.get(&js.class)?.cf.methods.get(i as usize)?;
        (m.name == "<init>").then(|| m.descriptor.clone())
    }

    /// A Java constructor's access where it is not public: whether it is protected, a package-private one
    /// otherwise (a private one is not entered); `None` for a public one and for a constructor no class file
    /// declares. Both are their package's (dotty's `ClassfileParser` sets `privateWithin` to the enclosing
    /// package), which `Worker::creation_accessible` asks.
    pub(in crate::typer) fn java_ctor_restricted(&self, s: SymId) -> Option<bool> {
        let java = &self.loaded.as_ref()?.java;
        let js = *java.syms.get(&s)?;
        let JMember::Method(i) = js.member else { return None };
        let access = java.classes.get(&js.class)?.cf.methods.get(i as usize)?.access;
        (access & ACC_PUBLIC == 0).then_some(access & ACC_PROTECTED != 0)
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

    /// Reads the JDK classes that the std's stand-ins among `c`'s superclasses stand for, so that
    /// their members carry the JDK's `ACC_ABSTRACT` (`absorb_java_class_now`) before `c`'s
    /// overrides and implementations are checked, whichever worker reads them first. On the JVM
    /// with a class path only.
    pub(in crate::typer) fn read_jdk_superclasses(&mut self, c: ClassId) {
        if !self.jvm || !self.sees_classpath() || self.source(self.syms.class(c).file).is_std {
            return;
        }
        let stand_ins: Vec<ClassId> =
            self.syms.class(c).base_types.iter().skip(1).map(|&(b, _)| b).filter(|&b| self.syms.class(b).kind == ClassKind::Class && self.std_jvm_class(b).is_some()).collect();
        for b in stand_ins {
            self.absorb_java_class(b);
        }
    }

    fn absorb_java_class(&mut self, c: ClassId) -> bool {
        self.java_class_absorbed(c) == Absorbed::Now
    }

    /// `absorb_java_class`, telling a class read before this call from one with nothing to read:
    /// with several workers, the one that read it may be a peer, whose release published the
    /// JDK's members after the asking worker last looked at the class.
    fn java_class_absorbed(&mut self, c: ClassId) -> Absorbed {
        if self.forked && crate::shared::lock_depth() == 0 {
            let stand_in = self.std_jvm_class(c).is_some();
            if !stand_in || self.java().absorbed_done.contains_key(&c) {
                crate::measure::looked_up_unlocked(crate::measure::Lookup::JavaClassEntry);
                return if stand_in { Absorbed::Before } else { Absorbed::Nothing };
            }
        }
        self.with_loader_for(crate::measure::Hold::JarEntry, |w| {
            // Read already where this worker waited for the lock while a peer read it.
            let before = w.forked && w.java().classes.contains_key(&c) && w.std_jvm_class(c).is_some();
            let absorbed = w.absorb_java_class_unlocked(c);
            crate::measure::looked_up(crate::measure::Lookup::JavaClassEntry, absorbed);
            match (absorbed, before) {
                (true, _) => Absorbed::Now,
                (false, true) => Absorbed::Before,
                (false, false) => Absorbed::Nothing,
            }
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
            match self.syms.entry_at(pkg, n).and_then(|e| e.pkg) {
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
        self.java_mut().classes.replace(class, JClass { source, cf: cf.clone(), companion_of: None, inner: false, outer: None });
        if let Some(o) = obj.filter(|&o| o != class) {
            self.java_mut().classes.replace(o, JClass { source, cf: cf.clone(), companion_of: Some(class), inner: false, outer: None });
        }
        // The std's members take the JDK's `ACC_ABSTRACT`, as dotty's `ClassfileParser` gives a
        // Java method `Deferred`: one without a body whose template calls a method the JDK
        // declares abstract (`InputStream.read()`) is implemented, not overridden, by a subclass.
        let own: Vec<SymId> = self.syms.class(class).members.values().flat_map(|&m| self.syms.alternatives(m).map_or(vec![m], |a| a.to_vec())).collect();
        for m in own {
            let info = self.syms.sym(m);
            let Some((name, desc)) = info.intrinsic.as_deref().and_then(|t| template_method(t, &binary)) else { continue };
            if info.owner == Owner::Class(class) && cf.methods.iter().any(|jm| jm.name == name && jm.descriptor == desc && jm.access & ACC_ABSTRACT != 0) {
                self.syms.sym_mut(m).mods |= mods::ABSTRACT;
            }
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
            let (kind, fm) = if f.access & ACC_FINAL != 0 { (SymKind::Val, mods::FIELD | constant_final(f)) } else { (SymKind::Var, mods::FIELD | mods::MUTABLE) };
            let sym = self.syms.new_sym(name, kind, fm, Owner::Class(owner), file, None, Span::default());
            self.syms.sym_mut(sym).java_defined = true;
            self.java_mut().syms.insert(sym, JSym { class, member: JMember::Field(i as u32) });
            self.register_term(Owner::Class(owner), sym);
        }
        // The class's own nested classes the std leaves out (`AbstractMap.SimpleEntry`), entered
        // as a class file's are (dotty's `ClassfileParser.enterOwnInnerClasses`).
        if let Some(o) = obj {
            self.enter_java_nested(class, o, &cf);
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
        self.java_mut().classes.replace(c, JClass { source, cf: cf.clone(), companion_of: None, inner: false, outer: None });
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
            own.extend(self.syms.extension_names(scala));
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
            self.syms.enter_scoped(self.interner, scala, name).term = Some(sym);
            self.java_mut().classes.replace(obj, JClass { source, cf: cf.clone(), companion_of: Some(c), inner: false, outer: None });
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
        // Identifiers, their own scope keys (`Symbols::sub_pkg_at`).
        let java = self.syms.sub_pkg_at(ROOT_PKG, names::JAVA, names::JAVA);
        self.syms.sub_pkg_at(java, names::LANG, names::LANG)
    }

    /// What a member lookup on `c` that found nothing may still get from Java: the members
    /// of the builtin's Java class, or those of placeholder ancestors read now. True when
    /// the lookup is worth repeating.
    pub fn java_member_miss(&mut self, c: ClassId) -> bool {
        if c == self.b.string || c == self.b.any_ref {
            return self.loaded.as_ref().unwrap().scala_library && self.enter_java_builtin_members(c);
        }
        // A class Java has given all it has is asked no more: every later miss would read its
        // ancestors again for nothing.
        if self.java_members_exhausted.get(c.0 as usize).copied().unwrap_or(false) {
            return false;
        }
        let got = self.java_member_miss_now(c);
        if got != Absorbed::Now {
            let i = c.0 as usize;
            if self.java_members_exhausted.len() <= i {
                self.java_members_exhausted.resize(i + 1, false);
            }
            self.java_members_exhausted[i] = true;
        }
        // What a peer read of the class since this worker's lookup found nothing is there now:
        // the lookup is repeated once more, and the class is marked, so the next miss ends here.
        got != Absorbed::Nothing
    }

    /// `Now` where this call gave the class more members, `Before` where it gave none and the
    /// JDK's class of `c` or of a base the std stands in for had been read before it.
    fn java_member_miss_now(&mut self, c: ClassId) -> Absorbed {
        let mut got = self.java_class_absorbed(c);
        if got == Absorbed::Now {
            return Absorbed::Now;
        }
        // A class below one the std stands in for inherits the JDK class's members the std leaves
        // out, its default methods (`Predicate.negate`) among them.
        let stand_ins: Vec<ClassId> = self.syms.class(c).base_types.iter().skip(1).map(|&(b, _)| b).filter(|&b| self.std_jvm_class(b).is_some()).collect();
        let mut absorbed = false;
        for b in stand_ins {
            match self.java_class_absorbed(b) {
                Absorbed::Now => absorbed = true,
                Absorbed::Before => got = Absorbed::Before,
                Absorbed::Nothing => {}
            }
        }
        if absorbed {
            return Absorbed::Now;
        }
        got
    }

    // ---- signatures ----

    /// The type of a final field with a `ConstantValue`: the constant converted to the declared
    /// type (dotty's `ClassfileParser.parseAttributes`, `ConstantType(convertTo(c, tp))`), a
    /// boolean's from its int. teq's literal types have no `Byte` or `Short`, which keep the
    /// declared type.
    fn java_constant_type(&mut self, c: &crate::classfile::ConstValue, declared: TypeId) -> Option<TypeId> {
        use crate::classfile::ConstValue;
        let b = &self.b;
        let lit = match *c {
            ConstValue::Int(v) if declared == b.t_int => LitVal::Int(v),
            ConstValue::Int(v) if declared == b.t_boolean => LitVal::Bool(v != 0),
            ConstValue::Int(v) if declared == b.t_char => LitVal::Char(v as u16),
            ConstValue::Long(v) if declared == b.t_long => LitVal::Long(v),
            ConstValue::Double(v) if declared == b.t_double => LitVal::Double(v.to_bits()),
            ConstValue::Float(v) if declared == b.t_float => LitVal::Float(v.to_bits()),
            ConstValue::Str(ref s) if declared == b.t_string => LitVal::Str(self.interner.intern(s)),
            _ => return None,
        };
        Some(self.types.lit(lit))
    }

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
                let ty = match &f.constant {
                    Some(c) if f.access & ACC_FINAL != 0 => self.java_constant_type(c, ty).unwrap_or(ty),
                    _ => ty,
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
                Arc::new(MethodSig { tparams, clauses: vec![ClauseSig { params, is_using: false, is_implicit: false }], ret, declared: crate::tir::NO_TYPE })
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
        // A binary name's segments are in the scope's key form already (`Symbols::sub_pkg_at`).
        for seg in pkg_path.split('/').filter(|s| !s.is_empty()) {
            let n = self.interner.intern(seg);
            p = self.syms.sub_pkg_at(p, n, n);
        }
        let name = self.interner.intern(simple);
        // A class nested in one the std stands in for is the std's (`Map$Entry`, nested in its
        // companion), or the JDK's the std's class takes in (`java_member_miss`), never read again
        // as a class of its own.
        let first = simple.split('$').find(|s| !s.is_empty())?;
        let first_name = self.interner.intern(first);
        let in_std = first.len() < simple.len()
            && self.syms.scoped(self.interner, p, first_name).and_then(|e| e.class).map_or(false, |o| self.std_jvm_class(o).is_some());
        if !in_std {
            if let Some(c) = self.java_top_level(p, name, simple, read, arity) {
                return Some(c);
            }
        }
        // A `$` in the simple name is the nesting of one class in another.
        let mut parts = simple.split('$').filter(|s| !s.is_empty());
        parts.next();
        let mut c = self.java_top_level(p, first_name, first, true, 0)?;
        for part in parts {
            if self.syms.class(c).state() == Completion::NotStarted {
                self.complete_class(c);
            }
            let n = self.interner.intern(part);
            let nested = |w: &Self, c: ClassId| {
                let own = w.syms.class(c).nested.get(&n).copied();
                own.or_else(|| w.syms.class(c).companion.and_then(|o| w.syms.class(o).nested.get(&n).copied()))
            };
            c = match nested(self, c) {
                Some(k) => k,
                None if in_std && self.java_member_miss(c) => nested(self, c)?,
                None => return None,
            };
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
            return self.syms.scoped(self.interner, p, name).and_then(|e| e.class);
        }
        if let Some(c) = self.scala_pkg_class(p, name) {
            return Some(c);
        }
        if read {
            if let Some((source, cf)) = self.locate_java_class(p, text, true) {
                return Some(self.enter_java_class(Owner::Package(p), Owner::Package(p), name, source, cf, None));
            }
        }
        if !text.contains('$') {
            return self.placeholder_class(p, name, arity);
        }
        None
    }

    /// The class `name` of the class path's package `p` that a Scala source of the jars defines,
    /// entered by its loader, the TASTy one or Scala 2's: one class per name of the package,
    /// whichever reference met it first, as dotty's `SymbolLoaders.initializeFromClassPath` reads
    /// a class file beside its TASTy from the TASTy. A Java class's parent or signature naming a
    /// Scala trait is that trait, never the empty placeholder the Java reader would leave of its
    /// class file, which `locate_java_class` refuses.
    fn scala_pkg_class(&mut self, p: PkgId, name: Name) -> Option<ClassId> {
        self.loaded.as_ref()?.slot(p)?;
        if !self.load_pkg_member(p, name) {
            return None;
        }
        self.syms.scoped(self.interner, p, name).and_then(|e| e.class)
    }
}

/// `FINAL` for a final field with a `ConstantValue`, whose read is its constant (`java_sig`), as
/// dotty's `ClassfileParser` flags a Java final field.
fn constant_final(f: &crate::classfile::Field) -> crate::ast::Mods {
    if f.constant.is_some() {
        mods::FINAL
    } else {
        0
    }
}
