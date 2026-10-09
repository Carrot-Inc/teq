//! The API graph of an analysis build (`--analysis-version 2`; docs/TARGETS.md, "The analysis
//! graph"): per source file, the classes it defines as zinc's `xsbti.api` states them, for a
//! build tool to hand zinc, which hashes them (sbt-teq's `ApiGraph.scala` builds the objects
//! mechanically). It is scalac 3.8.4's `ExtractAPI` over what teq writes as TASTy: a class's
//! declarations as the pickler states them (`tasty::write`) and as the TASTy reader reads them
//! back, the classes it inherits from from the program's pickles, the library's TASTy and the
//! compiler's own root classes, its linearization computed from those parents as scalac
//! computes base classes. Rendered at the end of a build, after the output is written, from
//! the typer's records through the pickler, without typing anything again.
//!
//! One JSON object per file: `{"file":"src/A.scala","nodes":[...],"classes":[...],"local":[...]}`.
//! A node is an array whose first element names the `xsbti.api` class and whose other elements
//! are that class's fields in the order of its factory method, a node in a field standing by its
//! index: `["Projection",0,"Int"]` over `["Singleton",[["Id","scala"],["This"]]]`. An access
//! (`"Public"`, `["Private","Unqualified"]`, `["Protected",["Id","p"]]`, `["Private","This"]`),
//! a modifier set (the byte of zinc's `Modifiers.raw`), an annotation (`[base,[[name,value]]]`),
//! a type parameter (`[id,annotations,typeParameters,variance,lower,upper]`) and a parameter list
//! (`[[[name,type,hasDefault,modifier]],isImplicit]`) sit inline. `classes` lists the
//! `ClassLike`s zinc's callback receives, `local` the names of the file's local classes, which
//! have no API. Equal nodes are one node, as `ExtractAPI`'s caches make them one object within a
//! compilation unit (a type by its value, a refinement by its parent and member), except that a
//! class's structure is its own.

use std::sync::Arc;

use crate::intern::FxMap;
use crate::source::FileId;
use crate::symbols::*;
use crate::tasty::tags::*;
use crate::tasty::tree::{Addr, Clause, Const, Decoder, Entry, LambdaKind, Mods, Param, TParam, TType};
use crate::tasty::{NameRef, TName, TastyFile};
use crate::typer::Worker;
use crate::types::ClassId;
use crate::watch::json_string;

/// The versions of the analysis this teq answers: 2 the graph alone, 3 the graph and the
/// dependencies of its classes (`typer::deps`).
pub const VERSIONS: [u32; 2] = [2, 3];

/// The version that answers the dependencies too.
pub const WITH_DEPS: u32 = 3;

/// Where a class's definition is: a TASTy file the build pickled (`Unit`) or one of the class
/// path's (`Lib`), and the address of its `TYPEDEF`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum Src {
    Unit(u32),
    Lib(u32),
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
struct At {
    src: Src,
    addr: Addr,
}

/// A class as the renderer reads it: from TASTy, or one of the classes the compiler defines,
/// or a class read from class files.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum Cls {
    Tasty(At),
    Any,
    Matchable,
    Object,
    /// A class of the class files: a Java class, or one of a Scala 2 library.
    Binary(ClassId),
}

/// What the renderer keeps of a class read from TASTy.
struct ClassData {
    at: At,
    /// How many of `path`'s names are its package's.
    pkg_len: usize,
    /// The names of the class's owners, outermost first, and its own last: `apiThis`'s path.
    /// An object's class is named with its `$`.
    path: Vec<String>,
    module: bool,
    trait_: bool,
    top_level: bool,
    /// `path` as scalac names the class: the object of a `package object p`'s members, which is
    /// `<stem>$package` by its binary name, and what is nested in it are `p.package`'s.
    api_path: Vec<String>,
    /// Read from the Scala 2 standard library's TASTy: scalac marks the class `Scala2x`, and its
    /// members are not inherited into the API.
    scala2: bool,
    tparams: Vec<TParam>,
    accessors: Vec<Param>,
    parents: Vec<TType>,
    self_type: Option<TType>,
    ctor: Option<Addr>,
    members: Vec<Entry>,
    mods: Mods,
}

impl ClassData {
    /// scalac's `fullName`: `p.O$.C`.
    fn full(&self) -> String {
        self.api_path.join(".")
    }

    /// The name zinc knows the class by, `fullName` without an object's `$`.
    fn api_name(&self) -> String {
        let full = self.full();
        if self.module {
            full.strip_suffix('$').map(str::to_string).unwrap_or(full)
        } else {
            full
        }
    }
}

/// The graph of an answer as it is built: one table of nodes for every file it covers, each node
/// made once. Sharing across files is sharing within each class, which is what zinc's hashing
/// of a class sees: a node shared within a file is as it is in scalac's unit.
#[derive(Default)]
struct Graph {
    nodes: Vec<std::rc::Rc<str>>,
    by_value: FxMap<std::rc::Rc<str>, u32>,
    /// The `ClassLikeDef` of each class, and the classes whose `ClassLike` a file has made.
    class_defs: FxMap<Cls, u32>,
    class_likes: std::collections::HashSet<Cls, crate::intern::FxBuild>,
    /// The structure node of each recursive type's self-reference, filled once the type is.
    rec_this: FxMap<(Src, Addr), u32>,
    /// The node of each definition made, which every class that inherits it lists.
    defs: FxMap<DeclKey, u32>,
    /// The `this` of each path, and the types of the compiler's own classes, by name.
    paths: FxMap<(Vec<String>, usize), u32>,
    fixed: FxMap<(u8, &'static str), u32>,
    /// The structures of intersections, unions, match types and refinements, each one node for
    /// what scalac's caches make one object of.
    structs: FxMap<StructKey, u32>,
    /// What the file being rendered has.
    file: FileGraph,
    scratch: String,
    files: Vec<String>,
}

/// One file's part of the graph.
#[derive(Default)]
struct FileGraph {
    classes: Vec<u32>,
    /// Per class, its name and the binary name of its class file, as scalac's bridge reports
    /// them (`generatedNonLocalClass`).
    products: Vec<(String, String)>,
    /// The annotations of a top-level class of the file, its `@SourceFile` among them.
    top_annotations: Option<String>,
}

/// What makes two structures one object, as `ExtractAPI` makes them: an intersection, a union,
/// a match type or a `super` type is one per type, as its `typeCache` keeps them, a type being
/// told apart by its kind, the alias it was written as (`origin`), its parts and the parameters
/// it refers to, whose references render alike; a refinement is one per parent and member
/// (`refinedTypeCache`).
#[derive(PartialEq, Eq, Hash)]
struct StructKey {
    kind: StructKind,
    origin: Option<Origin>,
    parts: Vec<u32>,
    params: Vec<ParamId>,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum StructKind {
    And,
    Or,
    Match,
    Super,
    Refined,
}

/// The alias a structure's type was written as: the alias's definition, and the nodes and the
/// parameters of its arguments.
type Origin = (At, Vec<u32>, Vec<ParamId>);

/// A type parameter, or a term parameter a singleton type names: its definition's address, or
/// its lambda's and its position.
type ParamId = (Src, Addr, u32);

/// A declaration as the key of its node in a graph.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum DeclKey {
    Member(At, Addr),
    ClassTypeParam(At, Addr),
    Accessor(At, Addr),
    Root(u8),
    JavaInit(ClassId),
    ProxyModule(At),
    ProxyVal(At),
    ProxyApply(At, Addr),
    ProxyType(At, Addr),
}

impl Graph {
    /// The node of the text `args` writes, made unless an equal one was; the text is written
    /// into a buffer kept for it, so that a node met again costs no allocation.
    fn node_fmt(&mut self, args: std::fmt::Arguments) -> u32 {
        use std::fmt::Write as _;
        let mut buf = std::mem::take(&mut self.scratch);
        buf.clear();
        let _ = buf.write_fmt(args);
        let i = match self.by_value.get(buf.as_str()) {
            Some(&i) => i,
            None => {
                let i = self.nodes.len() as u32;
                let text: std::rc::Rc<str> = buf.as_str().into();
                self.nodes.push(text.clone());
                self.by_value.insert(text, i);
                i
            }
        };
        self.scratch = buf;
        i
    }

    fn node(&mut self, text: String) -> u32 {
        if let Some(&i) = self.by_value.get(text.as_str()) {
            return i;
        }
        let i = self.nodes.len() as u32;
        let text: std::rc::Rc<str> = text.into();
        self.nodes.push(text.clone());
        self.by_value.insert(text, i);
        i
    }

    /// A node that is itself and no other, as a class's structure and a class are.
    fn reserve(&mut self) -> u32 {
        self.nodes.push("".into());
        (self.nodes.len() - 1) as u32
    }

    fn fill(&mut self, i: u32, text: String) {
        self.nodes[i as usize] = text.into();
    }

    /// The structure `key` names, made with the text `args` writes unless it was.
    fn structure(&mut self, key: StructKey, args: std::fmt::Arguments) -> u32 {
        if let Some(&i) = self.structs.get(&key) {
            return i;
        }
        let i = self.reserve();
        self.fill(i, args.to_string());
        self.structs.insert(key, i);
        i
    }

    /// Ends the file being rendered: its classes, products and local classes as an entry of the
    /// answer's `files`, with the dependencies of its classes where the answer has them.
    fn end_file(&mut self, path: &str, local: &[String], deps: Option<&str>) {
        let f = std::mem::take(&mut self.file);
        let mut out = String::from("{\"file\":");
        json_string(path, &mut out);
        out.push_str(",\"classes\":[");
        let classes: Vec<String> = f.classes.iter().map(u32::to_string).collect();
        out.push_str(&classes.join(","));
        out.push_str("],\"products\":[");
        for (i, (name, binary)) in f.products.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push('[');
            json_string(name, &mut out);
            out.push(',');
            json_string(binary, &mut out);
            out.push(']');
        }
        out.push_str("],\"local\":[");
        for (i, l) in local.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            json_string(l, &mut out);
        }
        out.push(']');
        if let Some(d) = deps {
            out.push_str(",\"deps\":");
            out.push_str(d);
        }
        out.push('}');
        self.files.push(out);
    }

    /// The answer's `api`: `{"nodes":[...],"files":[...]}`, and with the dependencies the table
    /// of the binary entries they name, `"entries":[...]`.
    fn render(&self, entries: Option<&str>) -> String {
        let mut out = String::with_capacity(64 + self.nodes.iter().map(|n| n.len() + 1).sum::<usize>() + self.files.iter().map(|f| f.len() + 1).sum::<usize>());
        out.push_str("{\"nodes\":[");
        for (i, n) in self.nodes.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push_str(n);
        }
        out.push_str("],\"files\":[");
        out.push_str(&self.files.join(","));
        out.push(']');
        if let Some(e) = entries {
            out.push_str(",\"entries\":");
            out.push_str(e);
        }
        out.push('}');
        out
    }
}

fn jstr(s: &str) -> String {
    let mut out = String::new();
    json_string(s, &mut out);
    out
}

fn list<I: IntoIterator<Item = String>>(items: I) -> String {
    let mut out = String::from("[");
    for (i, s) in items.into_iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&s);
    }
    out.push(']');
    out
}

fn refs(items: &[u32]) -> String {
    list(items.iter().map(u32::to_string))
}

/// The modifier bits of zinc's `Modifiers.raw`.
mod bits {
    pub const ABSTRACT: u8 = 1;
    pub const OVERRIDE: u8 = 2;
    pub const FINAL: u8 = 4;
    pub const SEALED: u8 = 8;
    pub const IMPLICIT: u8 = 16;
    pub const LAZY: u8 = 32;
    pub const MACRO: u8 = 64;
    pub const SUPER_ACCESSOR: u8 = 128;
}

/// The substitution of a class's type parameters by the arguments a subclass gives them, while
/// the subclass's base types are rendered.
/// A parameter is keyed by the address of its `TYPEPARAM` (`NO_INDEX`), or a lambda type's by
/// the lambda's address and its position.
/// Each argument with the parameters it refers to.
type Subst = FxMap<(Src, Addr, u32), (u32, Vec<ParamId>)>;

const NO_INDEX: u32 = u32::MAX;

/// The type parameters a definition's signature binds, by address: what their references
/// render as, `ParameterRef`s of scalac's method and lambda types.
#[derive(Default, Clone)]
struct Scope {
    params: Vec<Addr>,
}

/// The renderer of one build: the build's pickles and the library's TASTy, read once.
struct Renderer<'r, 'a> {
    w: &'r mut Worker<'a>,
    sourceroot: std::path::PathBuf,
    /// The build's pickles not read yet, by their file, and the files read.
    available: FxMap<FileId, Vec<crate::tasty::write::Product>>,
    pickled: FxMap<FileId, ()>,
    units_of: FxMap<FileId, Vec<u32>>,
    /// The pickles of a `package object`'s members, by unit, with their object's name.
    package_objects: FxMap<u32, String>,
    /// What the pickler could not write, which leaves the graph out.
    errors: Vec<String>,
    units: Vec<Arc<TastyFile>>,
    libs: Vec<Arc<TastyFile>>,
    lib_index: FxMap<(u16, u32), u32>,
    /// The program's classes by their full names.
    program: FxMap<String, At>,
    /// The path of every class of a file, by address.
    paths: FxMap<Src, Arc<FxMap<Addr, ClassPath>>>,
    data: FxMap<At, Arc<ClassData>>,
    lin: FxMap<Cls, Arc<Vec<Cls>>>,
    /// The class a qualified name names.
    resolved: FxMap<String, Option<Cls>>,
    /// Whether a package of the program or of the class path has a path, by its segments.
    packages: FxMap<Vec<String>, bool>,
    /// Whether the class path has a directory, by its path.
    directories: FxMap<String, bool>,
    /// The pickles of the file being rendered.
    current: Vec<u32>,
    /// Each class's declarations, with an object's constructor or without.
    decls: FxMap<(At, bool), Arc<Vec<Decl>>>,
    /// The class a type of a pickle names, by the type's shape.
    class_of_cache: FxMap<(Src, Vec<u32>), Option<Cls>>,
    /// Names as scalac prints them, by their file (its address, which the renderer keeps
    /// alive) and reference.
    names: std::cell::RefCell<FxMap<(usize, NameRef), Box<str>>>,
    /// The alias the type rendered next was written as, which its structure is one per.
    origin: Option<Origin>,
    /// The class, and the member of it, being rendered, which an error names.
    context: Vec<(At, Option<Addr>)>,
    /// Each inline method's fingerprint, and each inline definition's body hash with the inline
    /// definitions it refers to.
    fingerprints: FxMap<(Src, Addr), u64>,
    body_hashes: FxMap<(Src, Addr), Arc<(u64, Vec<(Src, Addr)>)>>,
}

/// A body's hash as it is taken, the inline definitions its references resolve to, and whether
/// the writer withheld a tree of it.
struct BodyHash {
    h: Fp,
    refs: Vec<(Src, Addr)>,
    withheld: bool,
}

/// The modifiers of a definition in a body: every flag it carries, by its tag.
fn hash_flags(flags: crate::tasty::tree::Flags, w: &mut BodyHash) {
    for tag in 1..64u8 {
        if flags.has(tag) {
            w.h.num(tag as i64);
        }
    }
    w.h.tag("flags");
}

/// FNV-1a over a body's tokens, each tag ended by a zero byte.
struct Fp(u64);

impl Fp {
    fn new() -> Fp {
        Fp(0xcbf2_9ce4_8422_2325)
    }

    fn bytes(&mut self, b: &[u8]) {
        for &x in b {
            self.0 = (self.0 ^ x as u64).wrapping_mul(0x0100_0000_01b3);
        }
    }

    fn tag(&mut self, s: &str) {
        self.bytes(s.as_bytes());
        self.bytes(&[0]);
    }

    fn num(&mut self, n: i64) {
        self.bytes(&n.to_le_bytes());
    }
}

/// A class's owners' names, outermost first, its own last, of which the first `pkg_len` are
/// its package's.
#[derive(Clone)]
struct ClassPath {
    pkg_len: usize,
    path: Vec<String>,
}

/// The graph of each of `files` (in their order, the files a `package` block belongs to
/// counted as their file, `unit_file`), from the build's pickles of them where it has them
/// (`pickled`) and fresh ones otherwise; the errors of what could not be pickled when a file's
/// classes cannot be stated.
pub fn render(w: &mut Worker, files: &[FileId], unit_file: &[FileId], pickled: Option<&[crate::tasty::write::Product]>, sourceroot: &std::path::Path, version: u32) -> Result<String, Vec<String>> {
    let unit = |f: FileId| unit_file.get(f.0 as usize).copied().unwrap_or(f);
    let mut wanted: Vec<FileId> = Vec::new();
    // Each file with the `package` blocks it holds, whose pickles are the blocks' own.
    let mut members: FxMap<FileId, Vec<FileId>> = FxMap::default();
    for &f in files {
        let u = unit(f);
        if !members.contains_key(&u) {
            wanted.push(u);
            members.insert(u, vec![u]);
        }
    }
    for (i, &u) in unit_file.iter().enumerate() {
        let f = FileId(i as u32);
        if f != u {
            if let Some(m) = members.get_mut(&u) {
                m.push(f);
            }
        }
    }
    // The pickles of the files, the build's where it made them: a file whose class another
    // one's inherits from or whose alias it names is pickled when that is met (`ensure_file`).
    let mut available: FxMap<FileId, Vec<crate::tasty::write::Product>> = FxMap::default();
    if let Some(given) = pickled {
        for p in given {
            available.entry(p.source).or_default().push(crate::tasty::write::Product { package: p.package.clone(), name: p.name.clone(), source: p.source, bytes: p.bytes.clone(), uuid: p.uuid, complete: p.complete });
        }
    }
    let mut r = Renderer {
        w,
        sourceroot: sourceroot.to_path_buf(),
        available,
        pickled: FxMap::default(),
        units_of: FxMap::default(),
        package_objects: FxMap::default(),
        errors: Vec::new(),
        units: Vec::new(),
        libs: Vec::new(),
        lib_index: FxMap::default(),
        program: FxMap::default(),
        paths: FxMap::default(),
        data: FxMap::default(),
        lin: FxMap::default(),
        resolved: FxMap::default(),
        packages: FxMap::default(),
        directories: FxMap::default(),
        current: Vec::new(),
        decls: FxMap::default(),
        class_of_cache: FxMap::default(),
        names: Default::default(),
        origin: None,
        context: Vec::new(),
        fingerprints: FxMap::default(),
        body_hashes: FxMap::default(),
    };
    // The files asked for that the build did not pickle, pickled at once.
    let mut mask = vec![false; r.w.files.len()];
    let mut any = false;
    for &f in wanted.iter().flat_map(|u| &members[u]) {
        if !r.available.contains_key(&f) && program_file(r.w, f) {
            mask[f.0 as usize] = true;
            any = true;
        }
    }
    if any {
        let written = crate::tasty::write::write_products(r.w, Some(&mask), &r.sourceroot);
        r.errors.extend(written.errors);
        for p in written.products {
            r.available.entry(p.source).or_default().push(p);
        }
        // A file with nothing to pickle has an empty entry, and is not pickled again.
        for &f in wanted.iter().flat_map(|u| &members[u]) {
            if mask[f.0 as usize] {
                r.available.entry(f).or_default();
            }
        }
    }
    for &f in wanted.iter().flat_map(|u| &members[u]) {
        r.ensure_file(f);
    }
    if !r.errors.is_empty() {
        return Err(r.errors);
    }
    let mut deps = if version >= WITH_DEPS { Some(crate::typer::deps::render(r.w, &wanted, unit_file)?) } else { None };
    let mut g = Graph::default();
    for &f in &wanted {
        r.current = members[&f].iter().flat_map(|m| r.units_of.get(m).cloned().unwrap_or_default()).collect();
        let units = r.current.clone();
        for &u in &units {
            let src = Src::Unit(u);
            let tops: Vec<Addr> = r.top_classes(src);
            for addr in tops {
                r.api_class(&mut g, At { src, addr }, true);
            }
        }
        r.main_classes(&mut g, f, unit_file);
        let local = local_classes(r.w, f, unit_file);
        let file_deps = deps.as_mut().map(|d| d.files.remove(&f).unwrap_or_else(|| "{}".to_string()));
        g.end_file(&r.w.source(f).path, &local, file_deps.as_deref());
    }
    if !r.errors.is_empty() {
        return Err(r.errors);
    }
    Ok(g.render(deps.as_ref().map(|d| d.entries.as_str())))
}

/// Whether the file is one of the program's sources, which the build can pickle.
fn program_file(w: &Worker, f: FileId) -> bool {
    (f.0 as usize) < w.files.len() && !w.files.as_slice()[f.0 as usize].is_std && !w.in_jar(f)
}

/// The qualified names of the local classes a file defines (a class of a block, an anonymous
/// class), which zinc's analysis knows as products alone.
fn local_classes(w: &Worker, f: FileId, unit_file: &[FileId]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for i in 0..w.syms.classes.len() {
        let c = ClassId(i as u32);
        let info = w.syms.class(c);
        if info.def.is_none() || unit_file.get(info.file.0 as usize).copied().unwrap_or(info.file) != f {
            continue;
        }
        if info.owner == Owner::Local || info.kind == ClassKind::Anon {
            out.push(w.binary_name(c).replace('/', "."));
        }
    }
    out.sort();
    out.dedup();
    out
}

impl<'r, 'a> Renderer<'r, 'a> {
    /// The class scalac makes for each `@main` method of the file: `final class m` beside the
    /// method, with a constructor and a `main(args)`, annotated as the file's classes are.
    fn main_classes(&mut self, g: &mut Graph, f: FileId, unit_file: &[FileId]) {
        let entry_points = self.w.entry_points.clone();
        let mut names: Vec<(Vec<String>, String)> = Vec::new();
        for &(main, object) in entry_points.iter() {
            if object.is_some() {
                continue;
            }
            let info = self.w.syms.sym(main);
            if unit_file.get(info.file.0 as usize).copied().unwrap_or(info.file) != f {
                continue;
            }
            let Owner::Package(p) = info.owner else { continue };
            let pkg: Vec<String> = self.w.pkg_path(p).split('.').filter(|s| !s.is_empty()).map(str::to_string).collect();
            names.push((pkg, self.w.interner.get(info.name).to_string()));
        }
        names.sort();
        // The `@SourceFile` of the file's classes.
        let source_file = g.file.top_annotations.clone();
        for (pkg, name) in names {
            let mut full = pkg.clone();
            full.push(name.clone());
            let api_name = full.join(".");
            let owner = self.this_path(g, &pkg, pkg.len());
            let class_ref = g.node_fmt(format_args!("[\"Projection\",{},{}]", owner, jstr(&name)));
            let annots = source_file.clone().unwrap_or_else(|| "[]".to_string());
            let ctor_name = format!("{};init;", full.iter().map(|p| crate::jvm::names::encode(p)).collect::<Vec<_>>().join(";"));
            let ctor = g.node_fmt(format_args!("[\"Def\",{},\"Public\",0,[],[],[[[],false]],{}]", jstr(&ctor_name), class_ref));
            let string = java_lang_type(g, "String");
            let array = scala_type(g, "Array");
            let args = g.node_fmt(format_args!("[\"Parameterized\",{},[{}]]", array, string));
            let unit = scala_type(g, "Unit");
            let main = g.node_fmt(format_args!("[\"Def\",\"main\",\"Public\",0,[],[],[[[[\"args\",{},false,\"Plain\"]],false]],{}]", args, unit));
            let parents = vec![java_lang_type(g, "Object"), scala_type(g, "Matchable"), scala_type(g, "Any")];
            let mut inherited: Vec<u32> = ROOT_OBJECT.iter().map(|&r| root_member(g, r)).collect();
            inherited.extend(ROOT_ANY.iter().map(|&r| root_member(g, r)));
            let s = g.reserve();
            g.fill(s, format!("[\"Structure\",{},[{},{}],{}]", refs(&parents), ctor, main, refs(&inherited)));
            let empty = g_empty(g);
            let c = g.reserve();
            g.fill(c, format!("[\"ClassLike\",{},\"Public\",{},{},\"ClassDef\",{},{},[],[{}],true,[]]", jstr(&api_name), bits::FINAL, annots, empty, s, class_ref));
            g.file.classes.push(c);
            g.file.products.push((api_name.clone(), binary_name(&full, pkg.len())));
        }
    }

    fn file(&self, src: Src) -> Arc<TastyFile> {
        match src {
            Src::Unit(u) => self.units[u as usize].clone(),
            Src::Lib(l) => self.libs[l as usize].clone(),
        }
    }

    // ---- names ----

    /// The class, or the member, being rendered: `p.C` or `p.C.m`.
    fn place(&mut self) -> String {
        let Some(&(at, member)) = self.context.last() else { return "the build".to_string() };
        let d = self.class_data(at);
        let mut out = d.path.join(".");
        if let Some(addr) = member {
            let f = self.file(at.src);
            if let Some(n) = Decoder::new(&f).name_at(addr) {
                out.push('.');
                out.push_str(&self.name(&f, n));
            }
        }
        out
    }

    /// A name as scalac's `Name.toString` gives it (`$lessinit$greater$default$1` for a
    /// constructor's default getter), an object's class with its `$`.
    fn name(&self, f: &TastyFile, n: NameRef) -> String {
        let key = (f as *const TastyFile as usize, n);
        if let Some(s) = self.names.borrow().get(&key) {
            return s.to_string();
        }
        let mut out = String::new();
        write_name(f, n, &mut out);
        self.names.borrow_mut().insert(key, out.as_str().into());
        out
    }

    // ---- the classes of the files ----

    /// Reads a file's pickles, the build's or made now, and enters its classes by their full
    /// names; whether it was not read before.
    fn ensure_file(&mut self, f: FileId) -> bool {
        if self.pickled.contains_key(&f) {
            return false;
        }
        self.pickled.insert(f, ());
        let products = match self.available.remove(&f) {
            Some(ps) => ps,
            None => {
                let mut mask = vec![false; self.w.files.len()];
                mask[f.0 as usize] = true;
                let written = crate::tasty::write::write_products(self.w, Some(&mask), &self.sourceroot);
                self.errors.extend(written.errors);
                written.products
            }
        };
        for p in products {
            match TastyFile::parse(p.bytes) {
                Ok(t) => {
                    let u = self.units.len() as u32;
                    self.units.push(Arc::new(t));
                    self.units_of.entry(f).or_default().push(u);
                    if self.w.package_object_holder(f) && p.name.ends_with("$package") {
                        self.package_objects.insert(u, p.name.clone());
                    }
                    let src = Src::Unit(u);
                    let paths = self.paths_of(src);
                    for (&addr, cp) in paths.iter() {
                        self.program.insert(cp.path.join("."), At { src, addr });
                    }
                }
                Err(e) => self.errors.push(format!("{}: the pickle of {} does not read back: {}", self.w.source(f).path, p.name, e)),
            }
        }
        true
    }

    /// The path of every class of the file by its address.
    fn paths_of(&mut self, src: Src) -> Arc<FxMap<Addr, ClassPath>> {
        if let Some(p) = self.paths.get(&src) {
            return p.clone();
        }
        let f = self.file(src);
        let mut out: FxMap<Addr, ClassPath> = FxMap::default();
        let tops = crate::tasty::tree::index_top_level(&f);
        for t in tops {
            if t.entry.tag != TYPEDEF || !t.entry.is_class {
                continue;
            }
            let pkg = package_segments(&f, t.package);
            let pkg_len = pkg.len();
            self.collect_paths(&f, t.entry.addr, pkg, pkg_len, &mut out);
        }
        let out = Arc::new(out);
        self.paths.insert(src, out.clone());
        out
    }

    fn collect_paths(&self, f: &TastyFile, addr: Addr, mut prefix: Vec<String>, pkg_len: usize, out: &mut FxMap<Addr, ClassPath>) {
        let Some(name) = Decoder::new(f).name_at(addr) else { return };
        prefix.push(self.name(f, name));
        let nested = Decoder::new(f).nested_types(addr);
        for e in nested {
            if e.tag == TYPEDEF && e.is_class {
                self.collect_paths(f, e.addr, prefix.clone(), pkg_len, out);
            }
        }
        out.insert(addr, ClassPath { pkg_len, path: prefix });
    }

    /// The classes a pickle defines at its top, as scalac's `apiSource` walks a unit.
    fn top_classes(&mut self, src: Src) -> Vec<Addr> {
        let f = self.file(src);
        crate::tasty::tree::index_top_level(&f).into_iter().filter(|t| t.entry.tag == TYPEDEF && t.entry.is_class).map(|t| t.entry.addr).collect()
    }

    fn class_data(&mut self, at: At) -> Arc<ClassData> {
        if let Some(d) = self.data.get(&at) {
            return d.clone();
        }
        let f = self.file(at.src);
        let sig = Decoder::new(&f).class_sig(at.addr);
        let paths = self.paths_of(at.src);
        let cp = paths.get(&at.addr).cloned().unwrap_or_else(|| ClassPath { pkg_len: 0, path: vec![self.name(&f, sig.name)] });
        let flags = sig.mods.flags;
        let in_package_object = match at.src {
            Src::Unit(u) => self.package_objects.get(&u).is_some_and(|holder| cp.path.get(cp.pkg_len).is_some_and(|n| n.trim_end_matches('$') == holder)),
            Src::Lib(_) => false,
        };
        let mut api_path = cp.path.clone();
        if in_package_object {
            api_path[cp.pkg_len] = if api_path[cp.pkg_len].ends_with('$') { "package$" } else { "package" }.to_string();
        }
        let data = ClassData {
            at,
            pkg_len: cp.pkg_len,
            top_level: cp.path.len() == cp.pkg_len + 1,
            api_path,
            path: cp.path,
            module: flags.has(OBJECT),
            trait_: flags.has(TRAIT),
            scala2: matches!(at.src, Src::Lib(_)) && f.has_attribute(crate::tasty::tags::attr::SCALA2_STANDARD_LIBRARY),
            tparams: sig.tparams,
            accessors: sig.accessors,
            parents: sig.parents,
            self_type: sig.self_type.map(|(_, t)| t),
            ctor: sig.ctor.map(|c| c.addr),
            members: sig.index.members,
            mods: sig.mods,
        };
        let data = Arc::new(data);
        self.data.insert(at, data.clone());
        data
    }

    // ---- resolution ----

    /// The class a class reference in a TASTy type names.
    fn class_of(&mut self, src: Src, t: &TType) -> Option<Cls> {
        let key = (src, type_key(t));
        if let Some(c) = self.class_of_cache.get(&key) {
            return *c;
        }
        let c = self.class_of_now(src, t);
        self.class_of_cache.insert(key, c);
        c
    }

    fn class_of_now(&mut self, src: Src, t: &TType) -> Option<Cls> {
        match t {
            TType::LocalType(addr, _) => {
                let f = self.file(src);
                (Decoder::new(&f).local_type_kind(*addr) == crate::tasty::tree::LocalKind::Class).then_some(Cls::Tasty(At { src, addr: *addr }))
            }
            TType::Applied(tycon, _) => self.class_of(src, tycon),
            TType::Annotated(t, _) => self.class_of(src, t),
            _ => {
                let (pkg, classes) = self.ref_path(src, t)?;
                if classes.is_empty() {
                    return None;
                }
                self.resolve(&pkg, &classes)
            }
        }
    }

    /// The package a package reference names, and `None` without resolving anything for any
    /// reference that is not a package's.
    fn package_path(&mut self, src: Src, t: &TType) -> Option<Vec<String>> {
        match t {
            TType::Package(p) => {
                let f = self.file(src);
                Some(package_segments(&f, *p))
            }
            TType::This(inner) => match &**inner {
                TType::Package(_) => self.package_path(src, inner),
                _ => None,
            },
            TType::TermRef(prefix, n) => {
                let mut pkg = self.package_path(src, prefix)?;
                let f = self.file(src);
                pkg.push(self.name(&f, *n));
                self.is_package(&pkg).then_some(pkg)
            }
            _ => None,
        }
    }

    /// The package and the class names a reference goes through: `scala.deriving.Mirror.Product`
    /// is `scala.deriving` and `Mirror$`, `Product`.
    fn ref_path(&mut self, src: Src, t: &TType) -> Option<(Vec<String>, Vec<String>)> {
        match t {
            TType::Package(p) => {
                let f = self.file(src);
                Some((package_segments(&f, *p), Vec::new()))
            }
            TType::This(inner) => self.ref_path(src, inner),
            TType::Applied(inner, _) => self.ref_path(src, inner),
            TType::TypeRef(prefix, n) => {
                let f = self.file(src);
                let (pkg, mut classes) = self.ref_path(src, prefix)?;
                classes.push(self.name(&f, *n));
                Some((pkg, classes))
            }
            TType::TermRef(prefix, n) => {
                let f = self.file(src);
                let (mut pkg, mut classes) = self.ref_path(src, prefix)?;
                let name = self.name(&f, *n);
                if classes.is_empty() {
                    pkg.push(name.clone());
                    if self.is_package(&pkg) {
                        return Some((pkg, classes));
                    }
                    pkg.pop();
                }
                classes.push(format!("{}$", name));
                Some((pkg, classes))
            }
            TType::LocalType(addr, _) => {
                let paths = self.paths_of(src);
                let cp = paths.get(addr)?;
                Some((cp.path[..cp.pkg_len].to_vec(), cp.path[cp.pkg_len..].to_vec()))
            }
            TType::LocalTerm(addr, _) => {
                // An object's val: its class is the sibling named with the `$`.
                let f = self.file(src);
                let name = format!("{}$", self.name(&f, Decoder::new(&f).name_at(*addr)?));
                let paths = self.paths_of(src);
                let owner = paths.iter().filter(|(&a, _)| a < *addr && *addr < Decoder::new(&f).tree_end(a)).max_by_key(|(&a, _)| a).map(|(_, p)| p.clone());
                match owner {
                    Some(o) => {
                        let mut classes = o.path[o.pkg_len..].to_vec();
                        classes.push(name);
                        Some((o.path[..o.pkg_len].to_vec(), classes))
                    }
                    None => {
                        let top = paths.values().next()?;
                        Some((top.path[..top.pkg_len].to_vec(), vec![name]))
                    }
                }
            }
            _ => None,
        }
    }

    /// Whether a package of the program or of the class path has the path `segs`.
    fn is_package(&mut self, segs: &[String]) -> bool {
        if let Some(&found) = self.packages.get(segs) {
            return found;
        }
        let dir = segs.join("/");
        let mut found = self.w.loaded.as_ref().map_or(false, |l| l.cp.packages.iter().any(|p| p.path == dir));
        if !found {
            let mut p = Some(crate::symbols::ROOT_PKG);
            for s in segs {
                let n = self.w.interner.intern(s);
                p = p.and_then(|p| self.w.demand_pkg(p, n));
            }
            found = p.is_some();
        }
        self.packages.insert(segs.to_vec(), found);
        found
    }

    /// The class `classes` of package `pkg`: the program's, a library's read from its TASTy,
    /// a class the compiler defines, or a class of the class files.
    fn resolve(&mut self, pkg: &[String], classes: &[String]) -> Option<Cls> {
        let key = format!("{}#{}", pkg.join("."), classes.join("."));
        if let Some(c) = self.resolved.get(&key) {
            return *c;
        }
        let found = self.resolve_now(pkg, classes);
        self.resolved.insert(key, found);
        found
    }

    fn resolve_now(&mut self, pkg: &[String], classes: &[String]) -> Option<Cls> {
        let joined = pkg.join(".");
        if classes.len() == 1 {
            match (joined.as_str(), classes[0].as_str()) {
                ("scala", "Any") => return Some(Cls::Any),
                ("scala", "Matchable") => return Some(Cls::Matchable),
                ("scala", "AnyRef") | ("java.lang", "Object") => return Some(Cls::Object),
                _ => {}
            }
        }
        let mut full: Vec<String> = pkg.to_vec();
        full.extend(classes.iter().cloned());
        if let Some(&at) = self.program.get(&full.join(".")) {
            return Some(Cls::Tasty(at));
        }
        if let Some(src) = self.lib_file(pkg, &classes[0]) {
            let paths = self.paths_of(src);
            let want = full.join(".");
            if let Some((&addr, _)) = paths.iter().find(|(_, p)| p.path.join(".") == want) {
                return Some(Cls::Tasty(At { src, addr }));
            }
        }
        // A Java class, or one of a library without TASTy: the typer's. A class of a file of
        // the program not read yet is read from the file's pickle.
        let top = classes[0].strip_suffix('$').unwrap_or(&classes[0]);
        let mut c = self.class_in_package(pkg, top)?;
        let file = self.w.syms.class(c).file;
        if program_file(self.w, file) && self.ensure_file(file) {
            if let Some(&at) = self.program.get(&full.join(".")) {
                return Some(Cls::Tasty(at));
            }
        }
        for (i, n) in classes.iter().enumerate() {
            if i == 0 {
                if n.ends_with('$') {
                    c = self.object_of(c)?;
                }
                continue;
            }
            let bare = n.strip_suffix('$').unwrap_or(n);
            let name = self.w.interner.lookup(bare)?;
            c = self.w.syms.class(c).nested.get(&name).copied()?;
            if n.ends_with('$') {
                c = self.object_of(c)?;
            }
        }
        Some(Cls::Binary(c))
    }

    /// The TASTy file of the class path that holds the top-level class `class` of `pkg`.
    fn lib_file(&mut self, pkg: &[String], class: &str) -> Option<Src> {
        let loaded = self.w.loaded.as_ref()?.clone();
        let cp = &mut loaded.get_mut().cp;
        let dir = pkg.join("/");
        let stem = crate::classpath::encode_name(class.strip_suffix('$').unwrap_or(class)).into_owned();
        let p = cp.packages.iter().position(|p| p.path == dir)?;
        let f = cp.packages[p].files.iter().copied().find(|&f| cp.stem(f) == stem)?;
        if let Some(&l) = self.lib_index.get(&(f.jar, f.entry)) {
            return Some(Src::Lib(l));
        }
        let tasty = cp.file(f).ok()?;
        let l = self.libs.len() as u32;
        self.libs.push(tasty);
        self.lib_index.insert((f.jar, f.entry), l);
        Some(Src::Lib(l))
    }

    fn object_of(&mut self, c: ClassId) -> Option<ClassId> {
        let info = self.w.syms.class(c);
        if info.kind == ClassKind::Object {
            return Some(c);
        }
        info.companion.filter(|&k| self.w.syms.class(k).kind == ClassKind::Object)
    }

    fn class_in_package(&mut self, segs: &[String], name: &str) -> Option<ClassId> {
        let mut p = crate::symbols::ROOT_PKG;
        for s in segs {
            let n = self.w.interner.intern(s);
            p = self.w.demand_pkg(p, n)?;
        }
        let n = self.w.interner.intern(name);
        match self.w.pkg_type(p, n)? {
            crate::typer::TypeRef::Class(c) => Some(c),
            _ => None,
        }
    }

    /// The class the renderer reads a class of the typer's as: by its name.
    fn cls_of_class(&mut self, c: ClassId) -> Option<Cls> {
        let mut classes: Vec<String> = Vec::new();
        let mut k = c;
        loop {
            let info = self.w.syms.class(k);
            let own = self.w.interner.get(info.name).to_string();
            classes.insert(0, if info.kind == ClassKind::Object { format!("{}$", own) } else { own });
            match info.owner {
                Owner::Package(p) => {
                    let pkg: Vec<String> = self.w.pkg_path(p).split('.').filter(|s| !s.is_empty()).map(str::to_string).collect();
                    return self.resolve(&pkg, &classes);
                }
                Owner::Class(o) => k = o,
                Owner::Local => return None,
            }
        }
    }

    // ---- linearization ----

    /// scalac's `baseClasses`: the class, then the base classes of its parents, the last
    /// parent's first, each once.
    fn linearization(&mut self, c: Cls) -> Arc<Vec<Cls>> {
        if let Some(l) = self.lin.get(&c) {
            return l.clone();
        }
        // A cycle through a broken program ends at the class itself.
        self.lin.insert(c, Arc::new(vec![c]));
        let parents: Vec<Cls> = self.parent_classes(c);
        let mut to: Vec<Cls> = Vec::new();
        for p in parents {
            let bcs = self.linearization(p);
            let mut added: Vec<Cls> = bcs.iter().filter(|b| !to.contains(b)).copied().collect();
            added.extend(to);
            to = added;
        }
        let mut out = vec![c];
        out.extend(to.into_iter().filter(|&b| b != c));
        let out = Arc::new(out);
        self.lin.insert(c, out.clone());
        out
    }

    fn parent_classes(&mut self, c: Cls) -> Vec<Cls> {
        match c {
            Cls::Any => Vec::new(),
            Cls::Matchable => vec![Cls::Any],
            Cls::Object => vec![Cls::Any, Cls::Matchable],
            Cls::Binary(k) => {
                let parents: Vec<crate::types::TypeId> = self.w.syms.class(k).parents.clone();
                let mut out: Vec<Cls> = Vec::new();
                for p in parents {
                    if let crate::types::Type::Class(pc, _) = self.w.types.get(p) {
                        if let Some(c) = self.cls_of_class(pc) {
                            out.push(c);
                        }
                    }
                }
                if out.is_empty() {
                    out.push(Cls::Object);
                }
                out
            }
            Cls::Tasty(at) => {
                let d = self.class_data(at);
                let mut out = Vec::new();
                for p in d.parents.iter() {
                    if let Some(pc) = self.class_of(at.src, p) {
                        out.push(pc);
                    }
                }
                if out.is_empty() && !matches!(c, Cls::Any) {
                    out.push(Cls::Object);
                }
                out
            }
        }
    }

    // ---- classes ----

    /// scalac's `apiClass`: the class's `ClassLikeDef`, and its `ClassLike` among the file's
    /// classes when the file defines it (`own`).
    fn api_class(&mut self, g: &mut Graph, at: At, own: bool) -> u32 {
        self.context.push((at, None));
        let n = self.api_class_now(g, at, own);
        self.context.pop();
        n
    }

    fn api_class_now(&mut self, g: &mut Graph, at: At, own: bool) -> u32 {
        let key = Cls::Tasty(at);
        if let Some(&d) = g.class_defs.get(&key) {
            if !own || g.class_likes.contains(&key) {
                return d;
            }
        }
        let d = self.class_data(at);
        let f = self.file(at.src);
        let def_type = if d.trait_ { "Trait" } else if d.module { "Module" } else { "ClassDef" };
        let name = d.api_name();
        let access = self.access(at.src, &d.mods);
        let mods = self.class_modifiers(&d);
        let annots = list(self.annotation_list(g, at.src, at.addr, &Scope::default()));
        let scope = Scope::default();
        let tparams: Vec<String> = d.tparams.iter().map(|p| self.type_parameter(g, at.src, p, &scope, true)).collect();
        let tparams = list(tparams);
        let class_def = g.node_fmt(format_args!("[\"ClassLikeDef\",{},{},{},{},{},\"{}\"]", jstr(&name), access, mods, annots, tparams, def_type));
        g.class_defs.insert(key, class_def);
        if own {
            g.class_likes.insert(key);
        }
        if own && d.top_level && g.file.top_annotations.is_none() {
            g.file.top_annotations = Some(annots.clone());
        }
        if own {
            let class_like = g.reserve();
            let renamed = d.api_path != d.path;
            let mut self_type = match (&d.self_type, d.module) {
                (Some(t), _) if !(d.module && renamed) => self.ty(g, at.src, t, &scope, &Subst::default()),
                // An object's class has its object's singleton type as its self type, a package
                // object's by scalac's name.
                (_, true) => {
                    let owner = self.this_path(g, &d.api_path[..d.api_path.len() - 1], d.pkg_len);
                    let own = d.api_path.last().map(|n| n.strip_suffix('$').unwrap_or(n).to_string()).unwrap_or_default();
                    g.node_fmt(format_args!("[\"Projection\",{},{}]", owner, jstr(&own)))
                }
                (_, false) => g.node("[\"EmptyType\"]".to_string()),
            };
            // An object's opaque types are aliases inside it, as refinements of its self type.
            for m in d.members.iter().filter(|m| m.tag == TYPEDEF && !m.is_class && m.flags.has(OPAQUE)) {
                let sig = Decoder::new(&f).type_def_sig(m.addr);
                let alias = match &sig.rhs {
                    TType::Alias(t) => (**t).clone(),
                    other => other.clone(),
                };
                let t = self.ty(g, at.src, &alias, &scope, &Subst::default());
                let decl = g.node_fmt(format_args!("[\"TypeAlias\",{},\"Public\",0,[],[],{}]", jstr(&self.name(&f, sig.name)), t));
                let key = StructKey { kind: StructKind::Refined, origin: None, parts: vec![self_type, decl], params: Vec::new() };
                self_type = g.structure(key, format_args!("[\"Structure\",[{}],[{}],[]]", self_type, decl));
            }
            let children = self.sealed_children(g, &d);
            let structure = self.class_structure(g, &d);
            g.fill(
                class_like,
                format!(
                    "[\"ClassLike\",{},{},{},{},\"{}\",{},{},[],{},{},{}]",
                    jstr(&name),
                    access,
                    mods,
                    annots,
                    def_type,
                    self_type,
                    structure,
                    refs(&children),
                    d.top_level,
                    tparams
                ),
            );
            g.file.classes.push(class_like);
            let binary = binary_name(&d.path, d.pkg_len);
            g.file.products.push((name.clone(), binary.clone()));
            // A top-level object without a companion class has a mirror class of its name.
            if d.module && d.top_level && self.linked_class(&d).is_none() {
                g.file.products.push((name.clone(), binary.strip_suffix('$').unwrap_or(&binary).to_string()));
            }
        }
        class_def
    }

    fn class_modifiers(&self, d: &ClassData) -> String {
        let fl = d.mods.flags;
        let mut m = 0u8;
        if fl.has(ABSTRACT) || fl.has(TRAIT) {
            m |= bits::ABSTRACT;
        }
        if fl.has(OVERRIDE) {
            m |= bits::OVERRIDE;
        }
        if fl.has(FINAL) || d.module {
            m |= bits::FINAL;
        }
        if fl.has(SEALED) {
            m |= bits::SEALED;
        }
        if fl.has(IMPLICIT) || fl.has(GIVEN) {
            m |= bits::IMPLICIT;
        }
        if fl.has(MACRO) {
            m |= bits::MACRO;
        }
        m.to_string()
    }

    /// scalac's `sealedDescendants`, sorted as `ExtractAPI` sorts them: the class itself and
    /// every class its `@Child` annotations name, breadth first, classes before objects' values.
    fn sealed_children(&mut self, g: &mut Graph, d: &ClassData) -> Vec<u32> {
        let mut found: Vec<(Cls, Option<(Src, TType)>)> = vec![(Cls::Tasty(d.at), None)];
        let mut explore: std::collections::VecDeque<Cls> = std::collections::VecDeque::from([Cls::Tasty(d.at)]);
        while let Some(c) = explore.pop_front() {
            let Cls::Tasty(at) = c else { continue };
            let cd = self.class_data(at);
            // `children` reads the `@Child` annotations last first.
            for a in cd.mods.annots.iter().rev() {
                let TType::Applied(tycon, args) = a else { continue };
                let is_child = matches!(&**tycon, TType::TypeRef(_, n) if self.file(at.src).simple(*n) == Some("Child"));
                let Some(arg) = args.first().filter(|_| is_child) else { continue };
                // A child that is a class is named by its type, an object's value by its
                // singleton type.
                let child: Option<Cls> = match arg {
                    TType::TermRef(..) | TType::LocalTerm(..) => None,
                    other => self.class_of(at.src, other),
                };
                match child {
                    Some(k) => {
                        if !found.iter().any(|(f, _)| *f == k) {
                            found.push((k, None));
                            explore.push_back(k);
                        }
                    }
                    None => found.push((Cls::Any, Some((at.src, arg.clone())))),
                }
            }
        }
        // Classes first, objects' classes before the others, by their full names; then the
        // values, in the order found.
        let mut classes: Vec<(bool, String, Cls)> = Vec::new();
        let mut values: Vec<(Src, TType)> = Vec::new();
        for (c, value) in found {
            match value {
                Some(v) => values.push(v),
                None => {
                    if let Cls::Tasty(at) = c {
                        let cd = self.class_data(at);
                        classes.push((!cd.module, cd.full(), c));
                    }
                }
            }
        }
        classes.sort_by(|a, b| (a.0, &a.1).cmp(&(b.0, &b.1)));
        let mut out = Vec::new();
        for (_, _, c) in classes {
            if let Cls::Tasty(at) = c {
                let cd = self.class_data(at);
                out.push(self.class_type_ref(g, &cd));
            }
        }
        for (src, v) in values {
            out.push(self.ty(g, src, &v, &Scope::default(), &Subst::default()));
        }
        out
    }

    /// `c.typeRef`: the class seen from its owner's `this`.
    fn class_type_ref(&mut self, g: &mut Graph, d: &ClassData) -> u32 {
        let owner = self.this_path(g, &d.api_path[..d.api_path.len() - 1], d.pkg_len);
        let own = d.api_path.last().cloned().unwrap_or_default();
        g.node_fmt(format_args!("[\"Projection\",{},{}]", owner, jstr(&own)))
    }

    /// `apiThis`: the singleton of the owners' path.
    fn this_path(&mut self, g: &mut Graph, path: &[String], pkg_len: usize) -> u32 {
        let key = (path.to_vec(), pkg_len);
        if let Some(&n) = g.paths.get(&key) {
            return n;
        }
        let names = self.package_names(path, pkg_len);
        let mut comps: Vec<String> = names.iter().map(|p| format!("[\"Id\",{}]", jstr(p))).collect();
        comps.push("[\"This\"]".to_string());
        let n = g.node_fmt(format_args!("[\"Singleton\",{}]", list(comps)));
        g.paths.insert(key, n);
        n
    }

    /// scalac's `apiClassStructure`: the linearization's base types as the parents (a value
    /// class's underlying type first), the declarations, and what the base classes declare
    /// that is neither private nor declared here.
    fn class_structure(&mut self, g: &mut Graph, d: &ClassData) -> u32 {
        let s = g.reserve();
        let me = Cls::Tasty(d.at);
        let lin = self.linearization(me);
        let mut parents: Vec<u32> = Vec::new();
        if let Some(u) = self.value_class_underlying(g, d) {
            parents.push(u);
        }
        for &bc in lin.iter().skip(1) {
            parents.push(self.base_type(g, me, &Subst::default(), bc).unwrap_or_else(|| self.plain_class_ref(g, bc)));
        }
        let declared_items = self.declarations(d, true);
        let declared: Vec<u32> = self.definitions_mixed(g, &declared_items);
        let own_addrs: Vec<Addr> = declared_items.iter().map(|m| m.addr()).collect();
        let mut inherited: Vec<Decl> = Vec::new();
        for &bc in lin.iter() {
            match bc {
                Cls::Tasty(at) => {
                    let bd = self.class_data(at);
                    if bd.scala2 {
                        continue;
                    }
                    for m in self.declarations(&bd, false).iter().cloned() {
                        let own_decl = at == d.at && (own_addrs.contains(&m.addr()) && m.addr() != 0 || matches!(m, Decl::ProxyModule(_) | Decl::ProxyVal(_) | Decl::ProxyApply(..) | Decl::ProxyType(..)));
                        if m.private() || own_decl {
                            continue;
                        }
                        inherited.push(m);
                    }
                }
                Cls::Object => inherited.extend(ROOT_OBJECT.iter().map(|&m| Decl::Root(m))),
                Cls::Any => inherited.extend(ROOT_ANY.iter().map(|&m| Decl::Root(m))),
                Cls::Matchable => {}
                Cls::Binary(k) => {
                    // A Java class's constructor; its methods are outside this stage
                    // (docs/TARGETS.md, "The analysis graph"). A class of a Scala 2 library
                    // declares nothing into the API, as for scalac.
                    if self.w.loaded.as_ref().map_or(true, |l| l.classes.get(&k).is_none()) {
                        inherited.push(Decl::JavaInit(k));
                    }
                }
            }
        }
        let inherited = self.definitions_mixed(g, &inherited);
        g.fill(s, format!("[\"Structure\",{},{},{}]", refs(&parents), refs(&declared), refs(&inherited)));
        s
    }

    /// A class of the linearization as a bare reference, where its base type cannot be read.
    fn plain_class_ref(&mut self, g: &mut Graph, c: Cls) -> u32 {
        match c {
            Cls::Any => scala_type(g, "Any"),
            Cls::Matchable => scala_type(g, "Matchable"),
            Cls::Object => java_lang_type(g, "Object"),
            Cls::Tasty(at) => {
                let d = self.class_data(at);
                self.class_type_ref(g, &d)
            }
            Cls::Binary(k) => self.binary_class_ref(g, k),
        }
    }

    fn binary_class_ref(&mut self, g: &mut Graph, k: ClassId) -> u32 {
        let info = self.w.syms.class(k);
        let own = self.w.interner.get(info.name).to_string();
        let owner: Vec<String> = match info.owner {
            Owner::Package(p) => self.w.pkg_path(p).split('.').filter(|s| !s.is_empty()).map(str::to_string).collect(),
            _ => Vec::new(),
        };
        let prefix = self.this_path(g, &owner, owner.len());
        g.node_fmt(format_args!("[\"Projection\",{},{}]", prefix, jstr(&own)))
    }

    /// `ref.baseType(bc)`: the parent type through which `c` reaches `bc`, its type parameters
    /// replaced by the arguments `c`'s own parents give them.
    fn base_type(&mut self, g: &mut Graph, c: Cls, subst: &Subst, bc: Cls) -> Option<u32> {
        match c {
            Cls::Tasty(at) => {
                let d = self.class_data(at);
                for p in d.parents.clone().iter() {
                    let Some(pc) = self.class_of(at.src, p) else { continue };
                    if pc == bc {
                        return Some(self.ty(g, at.src, p, &Scope::default(), subst));
                    }
                    if self.linearization(pc).contains(&bc) {
                        let mut inner = Subst::default();
                        if let (Cls::Tasty(pat), TType::Applied(_, args)) = (pc, p) {
                            let pd = self.class_data(pat);
                            for (tp, arg) in pd.tparams.iter().zip(args.iter()) {
                                let node = self.ty(g, at.src, arg, &Scope::default(), subst);
                                let ids = self.param_ids(at.src, arg, subst);
                                inner.insert((pat.src, tp.addr, NO_INDEX), (node, ids));
                            }
                        }
                        return self.base_type(g, pc, &inner, bc);
                    }
                }
                None
            }
            Cls::Object => match bc {
                Cls::Any => Some(scala_type(g, "Any")),
                Cls::Matchable => Some(scala_type(g, "Matchable")),
                _ => None,
            },
            Cls::Matchable => (bc == Cls::Any).then(|| scala_type(g, "Any")),
            Cls::Any => None,
            Cls::Binary(_) => Some(self.plain_class_ref(g, bc)),
        }
    }

    /// The underlying type of a value class, which scalac puts first among the parents.
    fn value_class_underlying(&mut self, g: &mut Graph, d: &ClassData) -> Option<u32> {
        let extends_any_val = d.parents.iter().any(|p| matches!(p, TType::TypeRef(pre, n) if self.file(d.at.src).simple(*n) == Some("AnyVal") && matches!(&**pre, TType::Package(_))));
        if !extends_any_val || d.accessors.len() != 1 {
            return None;
        }
        let a = d.accessors[0].clone();
        Some(self.ty(g, d.at.src, &a.ty, &Scope::default(), &Subst::default()))
    }

    // ---- definitions ----

    /// The class's declarations in the order scalac enters them: its type parameters, the
    /// parameter accessors, the constructor, the template's definitions, then the constructor
    /// proxies scalac's namer adds (`NamerOps.addConstructorProxies`). What the class declares
    /// for its own structure leaves an object's constructor out, which is always there.
    fn declarations(&mut self, d: &ClassData, own: bool) -> Arc<Vec<Decl>> {
        if let Some(ds) = self.decls.get(&(d.at, own)) {
            return ds.clone();
        }
        let ds = Arc::new(self.declarations_now(d, own));
        self.decls.insert((d.at, own), ds.clone());
        ds
    }

    fn declarations_now(&mut self, d: &ClassData, own: bool) -> Vec<Decl> {
        let mut out: Vec<Decl> = Vec::new();
        for p in &d.tparams {
            out.push(Decl::ClassTypeParam(d.at, p.clone()));
        }
        for p in &d.accessors {
            out.push(Decl::Accessor(d.at, p.clone()));
        }
        if let Some(c) = d.ctor {
            if !(own && d.module) {
                out.push(Decl::Member(d.at, c, false));
            }
        }
        for m in &d.members {
            out.push(Decl::Member(d.at, m.addr, m.flags.has(PRIVATE) && !m.qualified_private));
        }
        out.extend(self.constructor_proxies(d));
        out
    }

    /// scalac's constructor proxies in a class: for a nested class that may be constructed
    /// and has no companion, a companion object of the class's name with an `apply` per
    /// constructor; in an object without `apply` whose companion class may be constructed,
    /// those `apply`s; for an exported type naming such a class, a method of the type's name
    /// returning the proxy.
    fn constructor_proxies(&mut self, d: &ClassData) -> Vec<Decl> {
        let f = self.file(d.at.src);
        let mut out = Vec::new();
        let term_names: Vec<String> = d.members.iter().filter(|m| m.tag != TYPEDEF).map(|m| self.name(&f, m.name)).collect();
        let class_names: Vec<String> = d.members.iter().filter(|m| m.tag == TYPEDEF && m.is_class).map(|m| self.name(&f, m.name)).collect();
        for m in d.members.iter() {
            if m.tag != TYPEDEF || !m.is_class {
                continue;
            }
            let name = self.name(&f, m.name);
            if name.ends_with('$') || class_names.contains(&format!("{}$", name)) || term_names.contains(&name) {
                continue;
            }
            if !self.needs_constructor_proxies(At { src: d.at.src, addr: m.addr }) || self.term_member_exists(d, &name) {
                continue;
            }
            out.push(Decl::ProxyModule(At { src: d.at.src, addr: m.addr }));
            out.push(Decl::ProxyVal(At { src: d.at.src, addr: m.addr }));
        }
        if d.module {
            if let Some(linked) = self.linked_class(d) {
                if self.needs_constructor_proxies(linked) && !self.term_member_exists(d, "apply") {
                    let ld = self.class_data(linked);
                    for c in self.constructors(&ld) {
                        out.push(Decl::ProxyApply(linked, c));
                    }
                }
            }
        }
        for m in d.members.iter() {
            if m.tag != TYPEDEF || m.is_class || !m.flags.has(EXPORTED) {
                continue;
            }
            let name = self.name(&f, m.name);
            if term_names.contains(&name) || self.term_member_exists(d, &name) {
                continue;
            }
            let sig = Decoder::new(&f).type_def_sig(m.addr);
            let target = match &sig.rhs {
                TType::Alias(t) => (**t).clone(),
                TType::Bounds(lo, _) => (**lo).clone(),
                other => other.clone(),
            };
            if let Some(Cls::Tasty(at)) = self.class_of(d.at.src, &target) {
                if self.needs_constructor_proxies(at) && !self.has_companion(at) {
                    out.push(Decl::ProxyType(d.at, m.addr, target));
                }
            }
        }
        out
    }

    /// `needsConstructorProxies`: a class that is not abstract, a trait, a case class, an
    /// object's, synthetic or anonymous.
    fn needs_constructor_proxies(&mut self, at: At) -> bool {
        let d = self.class_data(at);
        let fl = d.mods.flags;
        let name = d.path.last().cloned().unwrap_or_default();
        !(fl.has(ABSTRACT) || fl.has(TRAIT) || fl.has(CASE) || fl.has(SYNTHETIC) || fl.has(OBJECT) || fl.has(INVISIBLE) || name.starts_with("$anon"))
    }

    /// Whether a class of the pickle has an object of its name beside it.
    fn has_companion(&mut self, at: At) -> bool {
        let d = self.class_data(at);
        let mut path = d.path.clone();
        if let Some(last) = path.last_mut() {
            last.push('$');
        }
        let paths = self.paths_of(at.src);
        paths.values().any(|p| p.path == path)
    }

    /// The class an object is the companion of, in the same pickle.
    fn linked_class(&mut self, d: &ClassData) -> Option<At> {
        let mut path = d.path.clone();
        let last = path.last_mut()?;
        *last = last.strip_suffix('$')?.to_string();
        let paths = self.paths_of(d.at.src);
        paths.iter().find(|(_, p)| p.path == path).map(|(&addr, _)| At { src: d.at.src, addr })
    }

    /// The constructors of a class: the primary, then the secondary ones.
    fn constructors(&mut self, d: &ClassData) -> Vec<Addr> {
        let f = self.file(d.at.src);
        let mut out: Vec<Addr> = d.ctor.into_iter().collect();
        out.extend(d.members.iter().filter(|m| m.tag == DEFDEF && f.simple(m.name) == Some("<init>")).map(|m| m.addr));
        out
    }

    /// Whether the class or one of its base classes declares a term `name`.
    fn term_member_exists(&mut self, d: &ClassData, name: &str) -> bool {
        let lin = self.linearization(Cls::Tasty(d.at));
        for &bc in lin.iter() {
            match bc {
                Cls::Tasty(at) => {
                    let bd = self.class_data(at);
                    let f = self.file(at.src);
                    if bd.members.iter().any(|m| m.tag != TYPEDEF && self.name(&f, m.name) == name) || bd.accessors.iter().any(|p| self.name(&f, p.name) == name) {
                        return true;
                    }
                }
                Cls::Any => {
                    if ["==", "!=", "equals", "hashCode", "toString", "##", "getClass", "isInstanceOf", "asInstanceOf", "$isInstanceOf", "$asInstanceOf"].contains(&name) {
                        return true;
                    }
                }
                Cls::Object => {
                    if ["eq", "ne", "synchronized", "clone", "finalize", "notify", "notifyAll", "wait"].contains(&name) {
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }

    /// scalac's `apiDefinitions`: the classes first, objects' before the others and each by
    /// its full name, then the rest in their order.
    fn definitions_mixed(&mut self, g: &mut Graph, decls: &[Decl]) -> Vec<u32> {
        let mut classes: Vec<(bool, String, Decl)> = Vec::new();
        let mut rest: Vec<Decl> = Vec::new();
        for m in decls {
            match self.class_decl(m) {
                Some((module, full)) => classes.push((!module, full, m.clone())),
                None => rest.push(m.clone()),
            }
        }
        classes.sort_by(|a, b| (a.0, &a.1).cmp(&(b.0, &b.1)));
        let mut out = Vec::new();
        for (_, _, m) in classes {
            if let Some(n) = self.definition(g, &m) {
                out.push(n);
            }
        }
        for m in rest {
            if let Some(n) = self.definition(g, &m) {
                out.push(n);
            }
        }
        out
    }

    /// Whether the declaration is a class: whether it is an object's, and its full name.
    fn class_decl(&mut self, m: &Decl) -> Option<(bool, String)> {
        if let Decl::ProxyModule(at) = m {
            let d = self.class_data(*at);
            return Some((true, format!("{}$", d.full())));
        }
        let Decl::Member(at, addr, _) = m else { return None };
        let f = self.file(at.src);
        if Decoder::new(&f).tag_at(*addr) != TYPEDEF || Decoder::new(&f).local_type_kind(*addr) != crate::tasty::tree::LocalKind::Class {
            return None;
        }
        let d = self.class_data(At { src: at.src, addr: *addr });
        Some((d.module, d.full()))
    }

    fn definition(&mut self, g: &mut Graph, m: &Decl) -> Option<u32> {
        // A class's own memo makes its `ClassLike` once the file that defines it is rendered.
        match m {
            Decl::ProxyModule(at) => return Some(self.proxy_module(g, *at)),
            Decl::Member(at, addr, _) if self.class_decl(m).is_some() => return self.member(g, *at, *addr),
            _ => {}
        }
        let key = m.key();
        if let Some(&n) = g.defs.get(&key) {
            return Some(n);
        }
        let place = m.place();
        if let Some(p) = place {
            self.context.push(p);
        }
        let n = self.definition_now(g, m);
        if place.is_some() {
            self.context.pop();
        }
        let n = n?;
        g.defs.insert(key, n);
        Some(n)
    }

    fn definition_now(&mut self, g: &mut Graph, m: &Decl) -> Option<u32> {
        match m {
            Decl::Root(r) => Some(root_member(g, *r)),
            Decl::JavaInit(k) => {
                let info = self.w.syms.class(*k);
                let owner: Vec<String> = match info.owner {
                    Owner::Package(p) => self.w.pkg_path(p).split('.').filter(|s| !s.is_empty()).map(str::to_string).collect(),
                    _ => Vec::new(),
                };
                let own = self.w.interner.get(info.name).to_string();
                let mut full = owner.clone();
                full.push(own);
                let name = format!("{};init;", full.join(";"));
                let ret = self.binary_class_ref(g, *k);
                Some(g.node_fmt(format_args!("[\"Def\",{},\"Public\",0,[],[],[[[],false]],{}]", jstr(&name), ret)))
            }
            Decl::ClassTypeParam(at, p) => {
                let f = self.file(at.src);
                let name = self.name(&f, p.name);
                let (lo, hi) = self.bounds(g, at.src, &p.info, &Scope::default());
                // A class's type parameter is `private[this]` in scalac's symbols.
                Some(g.node_fmt(format_args!("[\"TypeDeclaration\",{},[\"Private\",\"This\"],0,[],[],{},{}]", jstr(&name), lo, hi)))
            }
            Decl::Accessor(at, p) => {
                let f = self.file(at.src);
                let name = self.name(&f, p.name);
                let mods = Mods { flags: p.flags, ..Mods::default() };
                let access = self.access(at.src, &mods);
                let mut m = 0u8;
                if p.flags.has(OVERRIDE) {
                    m |= bits::OVERRIDE;
                }
                if p.flags.has(FINAL) {
                    m |= bits::FINAL;
                }
                if p.flags.has(IMPLICIT) || p.flags.has(GIVEN) {
                    m |= bits::IMPLICIT;
                }
                let annots = self.param_annotations(g, at.src, p.addr);
                let t = self.ty(g, at.src, &p.ty, &Scope::default(), &Subst::default());
                let kind = if p.flags.has(MUTABLE) { "Var" } else { "Val" };
                Some(g.node_fmt(format_args!("[\"{}\",{},{},{},{},{}]", kind, jstr(&name), access, m, annots, t)))
            }
            Decl::Member(at, addr, _) => self.member(g, *at, *addr),
            Decl::ProxyModule(at) => Some(self.proxy_module(g, *at)),
            Decl::ProxyVal(at) => {
                let d = self.class_data(*at);
                let owner = self.this_path(g, &d.api_path[..d.api_path.len() - 1], d.pkg_len);
                let own = d.api_path.last().cloned().unwrap_or_default();
                let t = g.node_fmt(format_args!("[\"Projection\",{},{}]", owner, jstr(&format!("{}$", own))));
                let access = self.proxy_access(&d);
                Some(g.node_fmt(format_args!("[\"Val\",{},{},{},[],{}]", jstr(&own), access, bits::FINAL | bits::LAZY, t)))
            }
            Decl::ProxyApply(class, ctor) => Some(self.proxy_apply(g, *class, *ctor)),
            Decl::ProxyType(owner, addr, target) => {
                let f = self.file(owner.src);
                let name = Decoder::new(&f).name_at(*addr).map(|n| self.name(&f, n)).unwrap_or_default();
                let (pre, own) = match target {
                    TType::TypeRef(prefix, n) => (self.prefix(g, owner.src, prefix, &Scope::default(), &Subst::default()), self.name(&f, *n)),
                    other => {
                        let cls = self.class_of(owner.src, other);
                        match cls {
                            Some(Cls::Tasty(at)) => {
                                let d = self.class_data(at);
                                (self.this_path(g, &d.api_path[..d.api_path.len() - 1], d.pkg_len), d.api_path.last().cloned().unwrap_or_default())
                            }
                            _ => (g_empty(g), name.clone()),
                        }
                    }
                };
                let t = g.node_fmt(format_args!("[\"Projection\",{},{}]", pre, jstr(&format!("{}$", own))));
                Some(g.node_fmt(format_args!("[\"Def\",{},\"Public\",0,[],[],[],{}]", jstr(&name), t)))
            }
        }
    }

    fn member(&mut self, g: &mut Graph, owner: At, addr: Addr) -> Option<u32> {
        let f = self.file(owner.src);
        let tag = Decoder::new(&f).tag_at(addr);
        match tag {
            TYPEDEF => {
                if Decoder::new(&f).local_type_kind(addr) == crate::tasty::tree::LocalKind::Class {
                    let own = self.defines(owner.src);
                    return Some(self.api_class(g, At { src: owner.src, addr }, own));
                }
                Some(self.type_member(g, owner, addr))
            }
            VALDEF | DEFDEF => Some(self.value_or_def(g, owner, addr)),
            _ => None,
        }
    }

    /// Whether the classes of the pickle are the file's own, whose `ClassLike`s the file's
    /// graph holds. scalac's `apiClass` makes a `ClassLike` of a class another file defines
    /// that a class of this one inherits, which zinc files under the class's own source.
    fn defines(&self, src: Src) -> bool {
        matches!(src, Src::Unit(u) if self.current.contains(&u))
    }

    /// A constructor proxy object: `final`, of the class's access when it is protected or
    /// qualified, with `Any` as its one parent and an `apply` per constructor.
    fn proxy_module(&mut self, g: &mut Graph, at: At) -> u32 {
        let key = Cls::Tasty(At { src: at.src, addr: at.addr | PROXY_BIT });
        let own = self.defines(at.src);
        if let Some(&d) = g.class_defs.get(&key) {
            if !own || g.class_likes.contains(&key) {
                return d;
            }
        }
        let d = self.class_data(at);
        let name = d.api_name();
        let access = self.proxy_access(&d);
        let class_def = g.node_fmt(format_args!("[\"ClassLikeDef\",{},{},{},[],[],\"Module\"]", jstr(&name), access, bits::FINAL));
        g.class_defs.insert(key, class_def);
        if own {
            g.class_likes.insert(key);
            let class_like = g.reserve();
            let owner = self.this_path(g, &d.api_path[..d.api_path.len() - 1], d.pkg_len);
            let own = d.api_path.last().cloned().unwrap_or_default();
            let self_type = g.node_fmt(format_args!("[\"Projection\",{},{}]", owner, jstr(&own)));
            let module_ref = g.node_fmt(format_args!("[\"Projection\",{},{}]", owner, jstr(&format!("{}$", own))));
            let any = scala_type(g, "Any");
            let applies: Vec<u32> = self.constructors(&d).into_iter().map(|c| self.proxy_apply(g, at, c)).collect();
            let inherited: Vec<u32> = ROOT_ANY.iter().map(|&r| root_member(g, r)).collect();
            let s = g.reserve();
            g.fill(s, format!("[\"Structure\",[{}],{},{}]", any, refs(&applies), refs(&inherited)));
            g.fill(class_like, format!("[\"ClassLike\",{},{},{},[],\"Module\",{},{},[],[{}],false,[]]", jstr(&name), access, bits::FINAL, self_type, s, module_ref));
            // A proxy object has no class file, and scalac reports no product for it.
            g.file.classes.push(class_like);
        }
        class_def
    }

    fn proxy_access(&mut self, d: &ClassData) -> String {
        if let Some(within) = &d.mods.within {
            let q = format!("[\"Id\",{}]", jstr(&self.qualifier_name(d.at.src, within)));
            return if d.mods.flags.has(PROTECTED) { format!("[\"Protected\",{}]", q) } else { format!("[\"Private\",{}]", q) };
        }
        if d.mods.flags.has(PROTECTED) {
            return "[\"Protected\",\"Unqualified\"]".to_string();
        }
        "\"Public\"".to_string()
    }

    /// A proxy's `apply`: the constructor's signature under the name `apply`, of the
    /// constructor's access, or the class's where that is protected or qualified.
    fn proxy_apply(&mut self, g: &mut Graph, class: At, ctor: Addr) -> u32 {
        let d = self.class_data(class);
        let f = self.file(class.src);
        let sig = Decoder::new(&f).def_sig(ctor);
        // The constructor's access flags and the class's qualifier (`ApplyProxyFlags`).
        let access = match &d.mods.within {
            Some(within) => {
                let q = format!("[\"Id\",{}]", jstr(&self.qualifier_name(class.src, within)));
                if sig.mods.flags.has(PROTECTED) {
                    format!("[\"Protected\",{}]", q)
                } else {
                    format!("[\"Private\",{}]", q)
                }
            }
            None => self.access(class.src, &sig.mods),
        };
        self.def_node(g, class, ctor, Some(access))
    }

    /// scalac's `apiTypeMember`: an alias or an abstract type, without type parameters.
    fn type_member(&mut self, g: &mut Graph, owner: At, addr: Addr) -> u32 {
        let f = self.file(owner.src);
        let sig = Decoder::new(&f).type_def_sig(addr);
        let name = self.name(&f, sig.name);
        let access = self.access(owner.src, &sig.mods);
        let opaque = sig.mods.flags.has(OPAQUE);
        let abstract_ = matches!(sig.rhs, TType::Bounds(..)) || matches!(&sig.rhs, TType::Lambda { result, .. } if matches!(**result, TType::Bounds(..))) || opaque;
        let mods = self.member_modifiers(&f, &sig.mods, sig.name, abstract_);
        let annots = self.annotations(g, owner.src, addr, &Scope::default());
        let scope = Scope::default();
        if opaque {
            // An opaque type's symbol is abstract outside its scope, as scalac states it: the
            // bounds, `Nothing` and `Any` unless declared.
            let (lo, hi) = match &sig.opaque_bounds {
                Some(b) => self.bounds(g, owner.src, b, &scope),
                None => (scala_type(g, "Nothing"), scala_type(g, "Any")),
            };
            return g.node_fmt(format_args!("[\"TypeDeclaration\",{},{},{},{},[],{},{}]", jstr(&name), access, mods, annots, lo, hi));
        }
        match &sig.rhs {
            TType::Bounds(lo, hi) => {
                let lo = self.ty(g, owner.src, lo, &scope, &Subst::default());
                let hi = self.ty(g, owner.src, hi, &scope, &Subst::default());
                g.node_fmt(format_args!("[\"TypeDeclaration\",{},{},{},{},[],{},{}]", jstr(&name), access, mods, annots, lo, hi))
            }
            TType::Lambda { kind: LambdaKind::Type, params, result, binder } if matches!(**result, TType::Bounds(..)) => {
                // `type F[X] <: U`: the bounds as lambdas over the parameters.
                let TType::Bounds(lo, hi) = &**result else { unreachable!() };
                let lo_l = TType::Lambda { kind: LambdaKind::Type, binder: *binder, params: params.clone(), result: lo.clone() };
                let hi_l = TType::Lambda { kind: LambdaKind::Type, binder: *binder, params: params.clone(), result: hi.clone() };
                let lo = if is_scala_nothing(&f, lo) { self.ty(g, owner.src, lo, &scope, &Subst::default()) } else { self.ty(g, owner.src, &lo_l, &scope, &Subst::default()) };
                let hi = self.ty(g, owner.src, &hi_l, &scope, &Subst::default());
                g.node_fmt(format_args!("[\"TypeDeclaration\",{},{},{},{},[],{},{}]", jstr(&name), access, mods, annots, lo, hi))
            }
            rhs => {
                let alias = match rhs {
                    TType::Alias(t) => (**t).clone(),
                    other => other.clone(),
                };
                let t = self.alias_rhs(g, owner.src, &alias, &scope);
                g.node_fmt(format_args!("[\"TypeAlias\",{},{},{},{},[],{}]", jstr(&name), access, mods, annots, t))
            }
        }
    }

    /// scalac's `apiDefinition` for a term: a `var` is a `Var`, a stable member that is no
    /// method a `Val`, anything else a `Def`.
    fn value_or_def(&mut self, g: &mut Graph, owner: At, addr: Addr) -> u32 {
        self.def_node(g, owner, addr, None)
    }

    /// A val, var or def; with `proxy`, a constructor as its proxy's `apply` of that access.
    fn def_node(&mut self, g: &mut Graph, owner: At, addr: Addr, proxy: Option<String>) -> u32 {
        let f = self.file(owner.src);
        let sig = Decoder::new(&f).def_sig(addr);
        let fl = sig.mods.flags;
        let is_ctor = f.simple(sig.name) == Some("<init>");
        let access = self.access(owner.src, &sig.mods);
        // A term without a body is `Deferred`, which scalac's modifiers leave out: a term is
        // `abstract` for `abstract override` alone.
        let mods = self.member_modifiers(&f, &sig.mods, sig.name, false);
        let mods = if sig.tag == VALDEF && fl.has(OBJECT) { (mods.parse::<u8>().unwrap_or(0) | bits::FINAL | bits::LAZY).to_string() } else { mods };
        if sig.tag == VALDEF {
            let scope = Scope::default();
            let annots = self.annotations(g, owner.src, addr, &scope);
            let name = self.name(&f, sig.name);
            let t = self.ty(g, owner.src, &sig.ret, &scope, &Subst::default());
            let kind = if fl.has(MUTABLE) { "Var" } else { "Val" };
            return g.node_fmt(format_args!("[\"{}\",{},{},{},{},{}]", kind, jstr(&name), access, mods, annots, t));
        }
        // The type parameters bound by the signature, and the lists of each clause.
        let mut scope = Scope::default();
        for c in &sig.clauses {
            if let Clause::Types(ps) = c {
                scope.params.extend(ps.iter().map(|p| p.addr));
            }
        }
        let mut tparams: Vec<String> = Vec::new();
        // The type parameter lists after the first, rendered into a graph of their own, which
        // holds what they refer to and nothing else.
        let mut extras = Graph::default();
        let mut extra_lists: Vec<String> = Vec::new();
        let mut plists: Vec<String> = Vec::new();
        for (i, c) in sig.clauses.iter().enumerate() {
            match c {
                Clause::Types(ps) => {
                    if i == 0 {
                        tparams = ps.iter().map(|p| self.type_parameter(g, owner.src, p, &scope, false)).collect();
                    } else {
                        let rendered: Vec<String> = ps.iter().map(|p| self.type_parameter(&mut extras, owner.src, p, &scope, false)).collect();
                        extra_lists.push(format!("[{},{}]", i, list(rendered)));
                    }
                }
                Clause::Terms(ps) => {
                    let implicit = ps.iter().any(|p| p.flags.has(GIVEN) || p.flags.has(IMPLICIT));
                    let params: Vec<String> = ps
                        .iter()
                        .map(|p| {
                            let mut t = self.param_type(g, owner.src, &p.ty, &scope);
                            if p.flags.has(INLINE) {
                                // scalac's namer marks an inline parameter's type
                                // `@InlineParam`.
                                let internal = g.node("[\"Singleton\",[[\"Id\",\"scala\"],[\"Id\",\"annotation\"],[\"Id\",\"internal\"],[\"This\"]]]".to_string());
                                let class = g.node_fmt(format_args!("[\"Projection\",{},\"InlineParam\"]", internal));
                                t = g.node_fmt(format_args!("[\"Annotated\",{},[[{},[[\"TREE_HASH\",{}]]]]]", t, class, jstr(&EMPTY_ANNOTATION_HASH.to_string())));
                            }
                            // A proxy's parameters are its method type's, without defaults.
                            let has_default = p.flags.has(HASDEFAULT) && proxy.is_none();
                            format!("[{},{},{},\"Plain\"]", jstr(&self.name(&f, p.name)), t, has_default)
                        })
                        .collect();
                    plists.push(format!("[{},{}]", list(params), implicit));
                }
            }
        }
        let name = if proxy.is_some() {
            "apply".to_string()
        } else if is_ctor {
            let d = self.class_data(owner);
            format!("{};init;", d.api_path.iter().map(|p| crate::jvm::names::encode(p)).collect::<Vec<_>>().join(";"))
        } else {
            self.name(&f, sig.name)
        };
        let ret = if is_ctor {
            let d = self.class_data(owner);
            let r = self.class_type_ref(g, &d);
            let first_types: Vec<String> = match sig.clauses.first() {
                Some(Clause::Types(ps)) => ps.iter().map(|p| self.name(&f, p.name)).collect(),
                _ => Vec::new(),
            };
            if first_types.is_empty() {
                r
            } else {
                let args: Vec<u32> = first_types.iter().map(|n| g.node_fmt(format_args!("[\"ParameterRef\",{}]", jstr(n)))).collect();
                g.node_fmt(format_args!("[\"Parameterized\",{},{}]", r, refs(&args)))
            }
        } else {
            self.ty(g, owner.src, &sig.ret, &scope, &Subst::default())
        };
        let mut annots: Vec<String> = Vec::new();
        if !extra_lists.is_empty() {
            // scalac's marker hashes the Java hash codes of its type parameters, whose variance
            // is an enum hashed by identity: its value differs from one JVM to the next. teq's
            // hashes the lists with everything they refer to, the same from one build to the next.
            let mut text = list(extra_lists);
            for n in &extras.nodes {
                text.push('\n');
                text.push_str(n);
            }
            let empty = g_empty(g);
            let marker = g.node_fmt(format_args!("[\"Constant\",{},{}]", empty, jstr(&format!("tparamsExtra:{:016x}", fnv(text.as_bytes())))));
            annots.push(format!("[{},[]]", marker));
        }
        if proxy.is_none() && fl.has(INLINE) && sig.body.is_some() {
            let fingerprint = self.inline_fingerprint(owner.src, addr);
            let empty = g_empty(g);
            let marker = g.node_fmt(format_args!("[\"Constant\",{},{}]", empty, jstr(&format!("inline:{:016x}", fingerprint))));
            annots.push(format!("[{},[]]", marker));
        }
        if proxy.is_none() {
            annots.extend(self.annotation_list(g, owner.src, addr, &scope));
        }
        let access = proxy.unwrap_or(access);
        g.node_fmt(format_args!("[\"Def\",{},{},{},{},{},{},{}]", jstr(&name), access, mods, list(annots), list(tparams), list(plists), ret))
    }

    fn member_modifiers(&self, f: &TastyFile, mods: &Mods, name: NameRef, deferred: bool) -> String {
        let fl = mods.flags;
        let mut m = 0u8;
        if fl.has(ABSTRACT) || deferred {
            m |= bits::ABSTRACT;
        }
        if fl.has(OVERRIDE) {
            m |= bits::OVERRIDE;
        }
        if fl.has(FINAL) {
            m |= bits::FINAL;
        }
        if fl.has(SEALED) {
            m |= bits::SEALED;
        }
        if fl.has(IMPLICIT) || fl.has(GIVEN) {
            m |= bits::IMPLICIT;
        }
        if fl.has(LAZY) {
            m |= bits::LAZY;
        }
        if fl.has(MACRO) {
            m |= bits::MACRO;
        }
        if matches!(f.names.get(f.source_name(name) as usize), Some(TName::SuperAccessor(_))) {
            m |= bits::SUPER_ACCESSOR;
        }
        m.to_string()
    }

    /// scalac's `apiAccess`.
    fn access(&mut self, src: Src, mods: &Mods) -> String {
        let fl = mods.flags;
        if let Some(within) = &mods.within {
            let q = format!("[\"Id\",{}]", jstr(&self.qualifier_name(src, within)));
            return if fl.has(PROTECTED) { format!("[\"Protected\",{}]", q) } else { format!("[\"Private\",{}]", q) };
        }
        if !fl.has(PRIVATE) && !fl.has(PROTECTED) {
            return "\"Public\"".to_string();
        }
        let q = if fl.has(LOCAL) { "\"This\"" } else { "\"Unqualified\"" };
        if fl.has(PROTECTED) {
            format!("[\"Protected\",{}]", q)
        } else {
            format!("[\"Private\",{}]", q)
        }
    }

    /// The full name of the class or package a `private[p]` names, as scalac's `fullName`
    /// gives it (`p.Outer`, `p.Obj$.Inner`).
    fn qualifier_name(&mut self, src: Src, t: &TType) -> String {
        let f = self.file(src);
        match t {
            TType::Package(p) => package_segments(&f, *p).join("."),
            TType::TypeRef(prefix, n) => {
                let pre = self.qualifier_name(src, prefix);
                let n = self.name(&f, *n);
                if pre.is_empty() {
                    n
                } else {
                    format!("{}.{}", pre, n)
                }
            }
            TType::TermRef(prefix, n) => {
                let pre = self.qualifier_name(src, prefix);
                let n = format!("{}$", self.name(&f, *n));
                if pre.is_empty() {
                    n
                } else {
                    format!("{}.{}", pre, n)
                }
            }
            TType::This(inner) => self.qualifier_name(src, inner),
            TType::LocalType(addr, _) => match self.paths_of(src).get(addr) {
                Some(cp) => cp.path.join("."),
                None => Decoder::new(&f).name_at(*addr).map(|n| self.name(&f, n)).unwrap_or_default(),
            },
            _ => String::new(),
        }
    }

    fn type_parameter(&mut self, g: &mut Graph, src: Src, p: &TParam, scope: &Scope, class: bool) -> String {
        let f = self.file(src);
        let variance = if !class && !p.flags.has(COVARIANT) && !p.flags.has(CONTRAVARIANT) {
            "Invariant"
        } else if p.flags.has(COVARIANT) {
            "Covariant"
        } else if p.flags.has(CONTRAVARIANT) {
            "Contravariant"
        } else {
            "Invariant"
        };
        let (lo, hi) = self.bounds(g, src, &p.info, scope);
        format!("[{},[],[],\"{}\",{},{}]", jstr(&self.name(&f, p.name)), variance, lo, hi)
    }

    /// The lower and upper bounds of a type parameter's info; a higher-kinded parameter's upper
    /// bound is a lambda over its own parameters.
    fn bounds(&mut self, g: &mut Graph, src: Src, info: &TType, scope: &Scope) -> (u32, u32) {
        match info {
            TType::Bounds(lo, hi) => (self.ty(g, src, lo, scope, &Subst::default()), self.ty(g, src, hi, scope, &Subst::default())),
            TType::Lambda { kind: LambdaKind::Type, binder, params, result } => match &**result {
                TType::Bounds(lo, hi) => {
                    let f = self.file(src);
                    let lo_t = if is_scala_nothing(&f, lo) { (**lo).clone() } else { TType::Lambda { kind: LambdaKind::Type, binder: *binder, params: params.clone(), result: lo.clone() } };
                    let hi_t = TType::Lambda { kind: LambdaKind::Type, binder: *binder, params: params.clone(), result: hi.clone() };
                    (self.ty(g, src, &lo_t, scope, &Subst::default()), self.ty(g, src, &hi_t, scope, &Subst::default()))
                }
                _ => (scala_type(g, "Nothing"), self.ty(g, src, info, scope, &Subst::default())),
            },
            TType::Alias(t) => {
                let n = self.ty(g, src, t, scope, &Subst::default());
                (n, n)
            }
            other => (scala_type(g, "Nothing"), self.ty(g, src, other, scope, &Subst::default())),
        }
    }

    /// A method parameter's type: `T*` is scalac's `<repeated>[T]`, which TASTy states as the
    /// annotated `Seq[T] @Repeated`.
    fn param_type(&mut self, g: &mut Graph, src: Src, t: &TType, scope: &Scope) -> u32 {
        if let TType::Annotated(inner, class) = t {
            let f = self.file(src);
            if names_class(&f, class, "Repeated") {
                if let TType::Applied(_, args) = &**inner {
                    if let Some(a) = args.first() {
                        let arg = self.ty(g, src, a, scope, &Subst::default());
                        let rep = scala_type(g, "<repeated>");
                        return g.node_fmt(format_args!("[\"Parameterized\",{},[{}]]", rep, arg));
                    }
                }
            }
        }
        self.ty(g, src, t, scope, &Subst::default())
    }

    // ---- annotations ----

    fn annotations(&mut self, g: &mut Graph, src: Src, addr: Addr, scope: &Scope) -> String {
        list(self.annotation_list(g, src, addr, scope))
    }

    /// scalac's `apiAnnotations`: every annotation of the definition but `@Child` and `@Body`,
    /// its class and the hash of its tree (`TREE_HASH`).
    fn annotation_list(&mut self, g: &mut Graph, src: Src, addr: Addr, scope: &Scope) -> Vec<String> {
        let f = self.file(src);
        let trees = Decoder::new(&f).annotation_addrs(addr);
        let mut out = Vec::new();
        for (class_at, tree_at) in trees {
            let class = Decoder::new(&f).type_at(class_at);
            if names_class(&f, &class, "Child") || names_class(&f, &class, "Body") {
                continue;
            }
            // The annotation's type: its class applied to the arguments its constructor call
            // gives explicitly.
            let base = match annotation_type(&f, tree_at) {
                Some(t) => t,
                None => class,
            };
            let base = self.ty(g, src, &base, scope, &Subst::default());
            let hash = tree_hash(&f, tree_at);
            out.push(format!("[{},[[\"TREE_HASH\",{}]]]", base, jstr(&hash.to_string())));
        }
        out
    }

    /// The annotations of a class parameter, which its accessor carries.
    fn param_annotations(&mut self, g: &mut Graph, src: Src, addr: Addr) -> String {
        list(self.annotation_list(g, src, addr, &Scope::default()))
    }

    // ---- inline bodies ----

    /// The fingerprint of an inline method, scalac's marker of its body (`apiAnnotations`): its
    /// right-hand side's hash and those of the inline methods and inline vals the body reaches,
    /// directly or through each other, in the order a walk from it first meets them. It is
    /// scalac's rule over content alone: scalac mixes in a reached method's `Def` by its Java
    /// hash code, which moves with the positions of a method with parameters.
    fn inline_fingerprint(&mut self, src: Src, addr: Addr) -> u64 {
        if let Some(&h) = self.fingerprints.get(&(src, addr)) {
            return h;
        }
        let mut h = Fp::new();
        let mut seen: Vec<(Src, Addr)> = vec![(src, addr)];
        let mut i = 0;
        while i < seen.len() {
            let (s, a) = seen[i];
            let body = self.inline_body_hash(s, a);
            h.num(body.0 as i64);
            for r in &body.1 {
                if !seen.contains(r) {
                    seen.push(*r);
                }
            }
            i += 1;
        }
        let v = h.0;
        self.fingerprints.insert((src, addr), v);
        v
    }

    /// The hash of a definition's right-hand side, per node its kind, names by their text and
    /// constants by their value, a type by the names it is made of (an inferred one as a written
    /// one: the pickle does not always tell them apart, and a type argument changes an expansion),
    /// a local definition with its modifiers, positions nowhere; with the inline definitions its
    /// references resolve to.
    /// A tree the writer withheld leaves the source's own text in its place, all of it, which
    /// a library's definition has none of.
    fn inline_body_hash(&mut self, src: Src, addr: Addr) -> Arc<(u64, Vec<(Src, Addr)>)> {
        if let Some(b) = self.body_hashes.get(&(src, addr)) {
            return b.clone();
        }
        let f = self.file(src);
        let mut d = Decoder::new(&f);
        let (_, body) = crate::tasty::terms::TermDecoder::new(&mut d).def_with_body(addr);
        let mut w = BodyHash { h: Fp::new(), refs: Vec::new(), withheld: false };
        match &body {
            Some(t) => self.hash_term(src, &f, t, &mut w),
            None => w.h.tag("<none>"),
        }
        if w.withheld {
            let text = match src {
                Src::Unit(u) => self.units_of.iter().find(|(_, us)| us.contains(&u)).map(|(&file, _)| self.w.source(file).text.clone()),
                Src::Lib(_) => None,
            };
            w.h.tag(&text.unwrap_or_default());
        }
        let out = Arc::new((w.h.0, w.refs));
        self.body_hashes.insert((src, addr), out.clone());
        out
    }

    fn hash_term(&mut self, src: Src, f: &TastyFile, t: &crate::tasty::terms::Term, w: &mut BodyHash) {
        use crate::tasty::terms::{MatchKind, TermKind};
        match &t.kind {
            TermKind::Path(ty) => {
                w.h.tag("Path");
                self.hash_path(src, f, ty, w);
                self.reference(src, ty, w);
            }
            TermKind::Const(c) => self.hash_const(src, f, c, w),
            TermKind::Ident(n, ty) => {
                w.h.tag("Ident");
                w.h.tag(&self.name(f, *n));
                self.reference(src, ty, w);
            }
            TermKind::Select(q, n) => {
                w.h.tag("Select");
                self.hash_term(src, f, q, w);
                let name = self.name(f, *n);
                w.h.tag(&name);
                for c in self.term_classes(src, q) {
                    self.member_reference(c, &name, w);
                }
            }
            TermKind::SelectIn(q, n, owner, _) => {
                w.h.tag("Select");
                self.hash_term(src, f, q, w);
                let name = self.name(f, *n);
                w.h.tag(&name);
                if let Some(c) = self.class_of(src, owner) {
                    self.member_reference(c, &name, w);
                }
            }
            TermKind::QualThis(ty) => {
                w.h.tag("This");
                self.hash_path(src, f, ty, w);
            }
            TermKind::New(ty) => {
                w.h.tag("New");
                self.hash_path(src, f, ty, w);
            }
            TermKind::Throw(e) => {
                w.h.tag("Throw");
                self.hash_term(src, f, e, w);
            }
            TermKind::NamedArg(n, e) => {
                w.h.tag("NamedArg");
                w.h.tag(&self.name(f, *n));
                self.hash_term(src, f, e, w);
            }
            TermKind::Apply(fun, args) | TermKind::ApplySigPoly(fun, _, args) => {
                w.h.tag("Apply");
                self.hash_term(src, f, fun, w);
                self.hash_terms(src, f, args, w);
            }
            TermKind::TypeApply(fun, targs, _) => {
                w.h.tag("TypeApply");
                self.hash_term(src, f, fun, w);
                w.h.num(targs.len() as i64);
                for a in targs {
                    self.hash_path(src, f, a, w);
                }
            }
            TermKind::Super(q, mix) => {
                w.h.tag("Super");
                self.hash_term(src, f, q, w);
                if let Some(m) = mix {
                    self.hash_path(src, f, m, w);
                }
            }
            TermKind::Typed(e, ty, _) => {
                w.h.tag("Typed");
                self.hash_term(src, f, e, w);
                self.hash_path(src, f, ty, w);
            }
            TermKind::Assign(l, r) | TermKind::While(l, r) => {
                w.h.tag(if matches!(t.kind, TermKind::Assign(..)) { "Assign" } else { "WhileDo" });
                self.hash_term(src, f, l, w);
                self.hash_term(src, f, r, w);
            }
            TermKind::Block(stats, e) => {
                w.h.tag("Block");
                w.h.num(stats.len() as i64);
                for s in stats {
                    self.hash_stat(src, f, s, w);
                }
                self.hash_term(src, f, e, w);
            }
            TermKind::If { inline, cond, then, els } => {
                w.h.tag(if *inline { "InlineIf" } else { "If" });
                self.hash_term(src, f, cond, w);
                self.hash_term(src, f, then, w);
                self.hash_term(src, f, els, w);
            }
            TermKind::Lambda(meth, ty) => {
                w.h.tag("Closure");
                self.hash_term(src, f, meth, w);
                if let Some(t) = ty {
                    self.hash_path(src, f, t, w);
                }
            }
            TermKind::Match { kind, selector, cases } => {
                w.h.tag(match kind {
                    MatchKind::Plain => "Match",
                    MatchKind::Inline => "InlineMatch",
                    MatchKind::Implicit => "SummonFrom",
                    MatchKind::Sub => "SubMatch",
                });
                if let Some(s) = selector {
                    self.hash_term(src, f, s, w);
                }
                self.hash_cases(src, f, cases, w);
            }
            TermKind::Try { body, cases, finalizer } => {
                w.h.tag("Try");
                self.hash_term(src, f, body, w);
                self.hash_cases(src, f, cases, w);
                if let Some(e) = finalizer {
                    self.hash_term(src, f, e, w);
                }
            }
            TermKind::Return { expr, .. } => {
                w.h.tag("Return");
                if let Some(e) = expr {
                    self.hash_term(src, f, e, w);
                }
            }
            TermKind::Inlined { expansion, call, bindings, .. } => {
                w.h.tag("Inlined");
                if let Some(c) = call {
                    self.hash_term(src, f, c, w);
                }
                w.h.num(bindings.len() as i64);
                for s in bindings {
                    self.hash_stat(src, f, s, w);
                }
                self.hash_term(src, f, expansion, w);
            }
            TermKind::Repeated(elem, elems) => {
                w.h.tag("SeqLiteral");
                self.hash_path(src, f, elem, w);
                self.hash_terms(src, f, elems, w);
            }
            TermKind::SelectOuter { levels, qual, .. } => {
                w.h.tag("SelectOuter");
                w.h.num(*levels as i64);
                self.hash_term(src, f, qual, w);
            }
            TermKind::Quote { body, .. } => {
                w.h.tag("Quote");
                self.hash_term(src, f, body, w);
            }
            TermKind::Splice { expr, .. } => {
                w.h.tag("Splice");
                self.hash_term(src, f, expr, w);
            }
            TermKind::QuotePattern { body, quotes, bindings, .. } => {
                w.h.tag("QuotePattern");
                for s in bindings {
                    self.hash_stat(src, f, s, w);
                }
                self.hash_term(src, f, body, w);
                self.hash_term(src, f, quotes, w);
            }
            TermKind::SplicePattern { pat, args, .. } => {
                w.h.tag("SplicePattern");
                self.hash_term(src, f, pat, w);
                self.hash_terms(src, f, args, w);
            }
            TermKind::Bind { name, body, ty, .. } => {
                w.h.tag("Bind");
                w.h.tag(&self.name(f, *name));
                self.hash_path(src, f, ty, w);
                self.hash_term(src, f, body, w);
            }
            TermKind::Alternative(ts) => {
                w.h.tag("Alternative");
                self.hash_terms(src, f, ts, w);
            }
            TermKind::Unapply { fun, implicits, pats, ty } => {
                w.h.tag("UnApply");
                self.hash_path(src, f, ty, w);
                self.hash_term(src, f, fun, w);
                self.hash_terms(src, f, implicits, w);
                self.hash_terms(src, f, pats, w);
            }
            TermKind::Hole { idx, args, .. } => {
                w.h.tag("Hole");
                w.h.num(*idx as i64);
                self.hash_terms(src, f, args, w);
            }
            TermKind::Elided(_) | TermKind::Unknown(..) | TermKind::TooDeep(_) => w.withheld = true,
            TermKind::Cycle(_) => w.h.tag("Cycle"),
        }
    }

    fn hash_terms(&mut self, src: Src, f: &TastyFile, ts: &[crate::tasty::terms::Term], w: &mut BodyHash) {
        w.h.num(ts.len() as i64);
        for t in ts {
            self.hash_term(src, f, t, w);
        }
    }

    fn hash_cases(&mut self, src: Src, f: &TastyFile, cases: &[crate::tasty::terms::Case], w: &mut BodyHash) {
        w.h.num(cases.len() as i64);
        for c in cases {
            w.h.tag("CaseDef");
            self.hash_term(src, f, &c.pat, w);
            match &c.guard {
                Some(g) => self.hash_term(src, f, g, w),
                None => w.h.tag("EmptyTree"),
            }
            self.hash_term(src, f, &c.body, w);
        }
    }

    fn hash_stat(&mut self, src: Src, f: &TastyFile, s: &crate::tasty::terms::Stat, w: &mut BodyHash) {
        use crate::tasty::terms::Stat;
        match s {
            Stat::Val(sig, body) | Stat::Def(sig, body) => {
                w.h.tag(if matches!(s, Stat::Val(..)) { "ValDef" } else { "DefDef" });
                w.h.tag(&self.name(f, sig.name));
                hash_flags(sig.mods.flags, w);
                for c in &sig.clauses {
                    match c {
                        Clause::Types(ps) => {
                            for p in ps {
                                w.h.tag(&self.name(f, p.name));
                                self.hash_path(src, f, &p.info, w);
                            }
                        }
                        Clause::Terms(ps) => {
                            for p in ps {
                                w.h.tag(&self.name(f, p.name));
                                hash_flags(p.flags, w);
                                self.hash_path(src, f, &p.ty, w);
                            }
                        }
                    }
                    w.h.tag(";");
                }
                self.hash_path(src, f, &sig.ret, w);
                match body {
                    Some(b) => self.hash_term(src, f, b, w),
                    None => w.h.tag("EmptyTree"),
                }
            }
            Stat::Type(sig) => {
                w.h.tag("TypeDef");
                w.h.tag(&self.name(f, sig.name));
                self.hash_path(src, f, &sig.rhs, w);
            }
            Stat::Class(c) => {
                w.h.tag("ClassDef");
                w.h.tag(&self.name(f, c.name));
                hash_flags(c.mods.flags, w);
                self.hash_terms(src, f, &c.template.parents, w);
                if let Some((sig, body)) = &c.template.ctor {
                    self.hash_stat(src, f, &Stat::Def(sig.clone(), body.clone()), w);
                }
                w.h.num(c.template.stats.len() as i64);
                for s in &c.template.stats {
                    self.hash_stat(src, f, s, w);
                }
            }
            Stat::Import { path, selectors } => {
                w.h.tag("Import");
                self.hash_path(src, f, path, w);
                for (n, rename) in selectors {
                    w.h.tag(&self.name(f, *n));
                    if let Some(r) = rename {
                        w.h.tag(&self.name(f, *r));
                    }
                }
            }
            Stat::Expr(t) => self.hash_term(src, f, t, w),
        }
    }

    fn hash_const(&mut self, src: Src, f: &TastyFile, c: &Const, w: &mut BodyHash) {
        w.h.tag("Literal");
        match c {
            Const::Unit => w.h.tag("()"),
            Const::Bool(b) => w.h.tag(if *b { "true" } else { "false" }),
            Const::Byte(v) | Const::Short(v) | Const::Int(v) => {
                w.h.tag(match c {
                    Const::Byte(_) => "B",
                    Const::Short(_) => "S",
                    _ => "I",
                });
                w.h.num(*v as i64);
            }
            Const::Char(v) => {
                w.h.tag("C");
                w.h.num(*v as i64);
            }
            Const::Long(v) => {
                w.h.tag("J");
                w.h.num(*v);
            }
            Const::Float(v) => {
                w.h.tag("F");
                w.h.num(*v as i64);
            }
            Const::Double(v) => {
                w.h.tag("D");
                w.h.num(*v as i64);
            }
            Const::Str(n) => {
                w.h.tag("String");
                w.h.tag(&f.name(*n));
            }
            Const::Null => w.h.tag("null"),
            Const::Class(t) => {
                w.h.tag("Class");
                self.hash_path(src, f, t, w);
            }
        }
    }

    /// A type by the names it is made of: what a reference or a written type says, the same
    /// whatever the addresses of the pickle (a definition of the pickle by its full path), and
    /// its structure.
    fn hash_path(&mut self, src: Src, f: &TastyFile, t: &TType, w: &mut BodyHash) {
        let pair = |this: &mut Self, tag: &str, a: &TType, b: &TType, w: &mut BodyHash| {
            w.h.tag(tag);
            this.hash_path(src, f, a, w);
            this.hash_path(src, f, b, w);
        };
        match t {
            TType::Package(n) => w.h.tag(&self.name(f, *n)),
            TType::TypeRef(p, n) | TType::TermRef(p, n) => {
                self.hash_path(src, f, p, w);
                w.h.tag(if matches!(t, TType::TypeRef(..)) { "#" } else { "." });
                w.h.tag(&self.name(f, *n));
            }
            TType::LocalType(a, _) | TType::LocalTerm(a, _) => match self.ref_path(src, t) {
                Some((pkg, classes)) => {
                    w.h.tag(&pkg.join("."));
                    w.h.tag(&classes.join("."));
                }
                None => match Decoder::new(f).name_at(*a) {
                    Some(n) => w.h.tag(&self.name(f, n)),
                    None => w.h.tag("TypeTree"),
                },
            },
            TType::This(inner) => {
                w.h.tag("this");
                self.hash_path(src, f, inner, w);
            }
            TType::Applied(c, args) => {
                self.hash_path(src, f, c, w);
                w.h.num(args.len() as i64);
                for a in args {
                    self.hash_path(src, f, a, w);
                }
            }
            TType::Const(c) => self.hash_const(src, f, c, w),
            TType::And(a, b) => pair(self, "&", a, b, w),
            TType::Or(a, b) => pair(self, "|", a, b, w),
            TType::Bounds(a, b) => pair(self, "<:", a, b, w),
            TType::BoundedAlias(a, b) => pair(self, "=<:", a, b, w),
            TType::Annotated(a, b) => pair(self, "@", a, b, w),
            TType::Super(a, b) => pair(self, "super", a, b, w),
            TType::MatchCase(a, b) => pair(self, "=>", a, b, w),
            TType::Alias(a) | TType::ByName(a) | TType::Flexible(a) | TType::Rec(_, a) => {
                w.h.tag(match t {
                    TType::Alias(_) => "=",
                    TType::ByName(_) => "=>",
                    TType::Flexible(_) => "?",
                    _ => "rec",
                });
                self.hash_path(src, f, a, w);
            }
            TType::Refined(parent, members) => {
                w.h.tag("{}");
                self.hash_path(src, f, parent, w);
                for (n, info) in members {
                    w.h.tag(&self.name(f, *n));
                    match info {
                        Some(i) => self.hash_path(src, f, i, w),
                        None => w.h.tag("TypeTree"),
                    }
                }
            }
            TType::Lambda { kind, params, result, .. } => {
                w.h.tag(&format!("{:?}", kind));
                for p in params {
                    w.h.tag(&self.name(f, p.name));
                    self.hash_path(src, f, &p.info, w);
                }
                self.hash_path(src, f, result, w);
            }
            TType::ParamRef(_, i) => {
                w.h.tag("param");
                w.h.num(*i as i64);
            }
            TType::Match { scrutinee, bound, cases } => {
                w.h.tag("match");
                self.hash_path(src, f, scrutinee, w);
                if let Some(b) = bound {
                    self.hash_path(src, f, b, w);
                }
                for c in cases {
                    self.hash_path(src, f, &c.pattern, w);
                    self.hash_path(src, f, &c.body, w);
                }
            }
            TType::RecThis(_) => w.h.tag("recThis"),
            TType::Unknown(tag) => {
                w.h.tag("unknown");
                w.h.num(*tag as i64);
            }
        }
    }

    /// A reference written as a type: to a definition of this pickle by its address, or to a
    /// member by its prefix and name.
    fn reference(&mut self, src: Src, ty: &TType, w: &mut BodyHash) {
        match ty {
            TType::LocalTerm(a, _) => {
                let f = self.file(src);
                let tag = Decoder::new(&f).tag_at(*a);
                if (tag == DEFDEF || tag == VALDEF) && Decoder::new(&f).def_sig(*a).mods.flags.has(INLINE) && !w.refs.contains(&(src, *a)) {
                    w.refs.push((src, *a));
                }
            }
            TType::TermRef(prefix, n) => {
                let f = self.file(src);
                let name = self.name(&f, *n);
                if let Some(c) = self.class_of(src, prefix) {
                    self.member_reference(c, &name, w);
                }
            }
            _ => {}
        }
    }

    /// The inline definitions named `name` of the first class of `c`'s linearization that has a
    /// member of that name, every overload of it.
    fn member_reference(&mut self, c: Cls, name: &str, w: &mut BodyHash) {
        let lin = self.linearization(c);
        for &b in lin.iter() {
            let Cls::Tasty(at) = b else { continue };
            let d = self.class_data(at);
            let f = self.file(at.src);
            let named: Vec<&Entry> = d.members.iter().filter(|e| (e.tag == DEFDEF || e.tag == VALDEF) && self.name(&f, e.name) == name).collect();
            if named.is_empty() {
                continue;
            }
            for e in named {
                if e.flags.has(INLINE) && !w.refs.contains(&(at.src, e.addr)) {
                    w.refs.push((at.src, e.addr));
                }
            }
            return;
        }
    }

    /// The classes of the value a qualifier stands for: an object, a class's `this`, a
    /// constructed or ascribed type, and otherwise the declared type of what it names (a
    /// parameter, a val, a method's result, a member of a typed prefix), each overload's.
    fn term_classes(&mut self, src: Src, t: &crate::tasty::terms::Term) -> Vec<Cls> {
        use crate::tasty::terms::TermKind;
        match &t.kind {
            TermKind::Path(ty) | TermKind::Ident(_, ty) => match self.class_of(src, ty) {
                Some(c) => vec![c],
                None => self.declared_classes(src, ty),
            },
            TermKind::QualThis(ty) | TermKind::New(ty) | TermKind::Typed(_, ty, _) => self.class_of(src, ty).into_iter().collect(),
            TermKind::Select(q, n) => {
                let name = self.name(&self.file(src), *n);
                if name == "<init>" {
                    return self.term_classes(src, q);
                }
                let mut out = Vec::new();
                for c in self.term_classes(src, q) {
                    for r in self.member_result_classes(c, &name) {
                        if !out.contains(&r) {
                            out.push(r);
                        }
                    }
                }
                out
            }
            TermKind::SelectIn(q, n, owner, _) => {
                let name = self.name(&self.file(src), *n);
                if name == "<init>" {
                    return self.term_classes(src, q);
                }
                match self.class_of(src, owner) {
                    Some(c) => self.member_result_classes(c, &name),
                    None => Vec::new(),
                }
            }
            TermKind::Apply(fun, _) | TermKind::TypeApply(fun, _, _) => self.term_classes(src, fun),
            TermKind::Block(_, e) => self.term_classes(src, e),
            TermKind::Inlined { expansion, .. } => self.term_classes(src, expansion),
            _ => Vec::new(),
        }
    }

    /// The classes of the declared type of the term a reference names: a parameter, a val or a
    /// method of this pickle, or a member of a prefix's class.
    fn declared_classes(&mut self, src: Src, ty: &TType) -> Vec<Cls> {
        match ty {
            TType::LocalTerm(a, _) => {
                let f = self.file(src);
                if !matches!(Decoder::new(&f).tag_at(*a), PARAM | VALDEF | DEFDEF) {
                    return Vec::new();
                }
                let ret = Decoder::new(&f).def_sig(*a).ret;
                self.class_of(src, &ret).into_iter().collect()
            }
            TType::TermRef(prefix, n) => {
                let name = self.name(&self.file(src), *n);
                match self.class_of(src, prefix) {
                    Some(c) => self.member_result_classes(c, &name),
                    None => Vec::new(),
                }
            }
            _ => Vec::new(),
        }
    }

    /// The classes of the declared result of every overload named `name` in the first class of
    /// `c`'s linearization that has a member of that name.
    fn member_result_classes(&mut self, c: Cls, name: &str) -> Vec<Cls> {
        let lin = self.linearization(c);
        for &b in lin.iter() {
            let Cls::Tasty(at) = b else { continue };
            let d = self.class_data(at);
            let f = self.file(at.src);
            let named: Vec<Addr> = d.members.iter().filter(|e| (e.tag == DEFDEF || e.tag == VALDEF) && self.name(&f, e.name) == name).map(|e| e.addr).collect();
            if named.is_empty() {
                continue;
            }
            let mut out = Vec::new();
            for a in named {
                let ret = Decoder::new(&f).def_sig(a).ret;
                if let Some(r) = self.class_of(at.src, &ret) {
                    if !out.contains(&r) {
                        out.push(r);
                    }
                }
            }
            return out;
        }
        Vec::new()
    }

    // ---- types ----

    /// scalac's `apiType` of a type read from the pickle of `src`: aliases followed, a
    /// package's members seen from the package, `this` as the path of its class's owners.
    fn ty(&mut self, g: &mut Graph, src: Src, t: &TType, scope: &Scope, subst: &Subst) -> u32 {
        let origin = self.origin.take();
        match t {
            TType::TypeRef(prefix, n) if self.file(src).simple(*n) == Some("AnyRef") && self.package_path(src, prefix).is_some_and(|p| p == ["scala"]) => {
                return java_lang_type(g, "Object");
            }
            // A package named by a term reference: its `this`.
            TType::TermRef(..) => {
                if let Some(pkg) = self.package_path(src, t) {
                    return self.this_path(g, &pkg, pkg.len());
                }
            }
            _ => {}
        }
        if let Some(n) = self.dealiased(g, src, t, scope, subst, origin.clone()) {
            return n;
        }
        match t {
            TType::Package(p) => {
                let f = self.file(src);
                let segs = package_segments(&f, *p);
                self.this_path(g, &segs, segs.len())
            }
            TType::TypeRef(prefix, n) => {
                let f = self.file(src);
                let name = self.name(&f, *n);
                let pre = self.prefix(g, src, prefix, scope, subst);
                g.node_fmt(format_args!("[\"Projection\",{},{}]", pre, jstr(&name)))
            }
            TType::TermRef(prefix, n) => {
                let f = self.file(src);
                let name = self.name(&f, *n);
                let pre = self.term_prefix(g, src, prefix, &name, scope, subst);
                g.node_fmt(format_args!("[\"Projection\",{},{}]", pre, jstr(&name)))
            }
            TType::LocalType(addr, prefix) => {
                if let Some((n, _)) = subst.get(&(src, *addr, NO_INDEX)) {
                    return *n;
                }
                let f = self.file(src);
                let name = Decoder::new(&f).name_at(*addr).map(|n| self.name(&f, n)).unwrap_or_default();
                let tag = Decoder::new(&f).tag_at(*addr);
                if (tag == TYPEPARAM || tag == BIND) && (prefix.is_none() || scope.params.contains(addr)) {
                    return g.node_fmt(format_args!("[\"ParameterRef\",{}]", jstr(&name)));
                }
                let package_prefix = matches!(prefix.as_deref(), Some(TType::Package(_)) | Some(TType::This(_))) && {
                    let p = prefix.as_deref().unwrap();
                    matches!(p, TType::Package(_)) || matches!(p, TType::This(inner) if matches!(**inner, TType::Package(_)))
                };
                let nested = self.paths_of(src).get(addr).map_or(false, |p| p.path.len() > p.pkg_len + 1);
                let pre = match prefix {
                    Some(_) if package_prefix && nested => {
                        let cp = self.paths_of(src).get(addr).cloned().unwrap_or(ClassPath { pkg_len: 0, path: Vec::new() });
                        self.this_path(g, &cp.path[..cp.path.len().saturating_sub(1)], cp.pkg_len)
                    }
                    Some(p) => self.prefix(g, src, p, scope, subst),
                    None => self.owner_this(g, src, *addr),
                };
                g.node_fmt(format_args!("[\"Projection\",{},{}]", pre, jstr(&name)))
            }
            TType::LocalTerm(addr, prefix) => {
                let f = self.file(src);
                let name = Decoder::new(&f).name_at(*addr).map(|n| self.name(&f, n)).unwrap_or_default();
                if prefix.is_none() && Decoder::new(&f).tag_at(*addr) == PARAM {
                    return g.node_fmt(format_args!("[\"ParameterRef\",{}]", jstr(&name)));
                }
                let package_prefix = match prefix.as_deref() {
                    Some(TType::Package(_)) => true,
                    Some(TType::This(inner)) => matches!(**inner, TType::Package(_)),
                    _ => false,
                };
                if package_prefix && self.inside_a_class(src, *addr) {
                    let pre = self.owner_this(g, src, *addr);
                    return g.node_fmt(format_args!("[\"Projection\",{},{}]", pre, jstr(&name)));
                }
                let pre = match prefix {
                    Some(p) => self.prefix(g, src, p, scope, subst),
                    None => self.owner_this(g, src, *addr),
                };
                g.node_fmt(format_args!("[\"Projection\",{},{}]", pre, jstr(&name)))
            }
            TType::This(inner) => {
                let (pkg, classes) = self.ref_path(src, inner).unwrap_or_default();
                let pkg_len = pkg.len();
                let mut path = pkg;
                path.extend(classes);
                self.this_path(g, &path, pkg_len)
            }
            TType::Applied(tycon, args) => {
                let base = self.ty(g, src, tycon, scope, subst);
                let args: Vec<u32> = args.iter().map(|a| self.type_arg(g, src, a, scope, subst)).collect();
                g.node_fmt(format_args!("[\"Parameterized\",{},{}]", base, refs(&args)))
            }
            TType::Bounds(..) => self.type_arg(g, src, t, scope, subst),
            TType::Alias(inner) => self.ty(g, src, inner, scope, subst),
            TType::BoundedAlias(_, alias) => self.ty(g, src, alias, scope, subst),
            TType::And(a, b) => {
                let (a, b) = (self.ty(g, src, a, scope, subst), self.ty(g, src, b, scope, subst));
                let key = self.struct_key(StructKind::And, origin, vec![a, b], src, t, subst);
                g.structure(key, format_args!("[\"Structure\",[{},{}],[],[]]", a, b))
            }
            TType::Or(a, b) => {
                let (a, b) = (self.ty(g, src, a, scope, subst), self.ty(g, src, b, scope, subst));
                let key = self.struct_key(StructKind::Or, origin, vec![a, b], src, t, subst);
                let s = g.structure(key, format_args!("[\"Structure\",[{},{}],[],[]]", a, b));
                let m = marker(g, "Or");
                g.node_fmt(format_args!("[\"Annotated\",{},[[{},[]]]]", s, m))
            }
            TType::ByName(inner) => {
                let i = self.ty(g, src, inner, scope, subst);
                let m = marker(g, "ByName");
                g.node_fmt(format_args!("[\"Annotated\",{},[[{},[]]]]", i, m))
            }
            TType::Flexible(inner) => self.ty(g, src, inner, scope, subst),
            TType::Annotated(inner, class) => {
                let i = self.ty(g, src, inner, scope, subst);
                let c = self.ty(g, src, class, scope, subst);
                // A type's annotation is read by its class: its tree is taken to be the call
                // of a constructor without arguments.
                g.node_fmt(format_args!("[\"Annotated\",{},[[{},[[\"TREE_HASH\",{}]]]]]", i, c, jstr(&EMPTY_ANNOTATION_HASH.to_string())))
            }
            TType::Refined(parent, members) => {
                let mut acc = self.ty(g, src, parent, scope, subst);
                let f = self.file(src);
                for (n, info) in members {
                    let name = self.name(&f, *n);
                    let decl = match info {
                        Some(TType::Alias(t)) => {
                            let t = self.ty(g, src, t, scope, subst);
                            Some(g.node_fmt(format_args!("[\"TypeAlias\",{},\"Public\",0,[],[],{}]", jstr(&name), t)))
                        }
                        Some(TType::Bounds(lo, hi)) => {
                            let (lo, hi) = (self.ty(g, src, lo, scope, subst), self.ty(g, src, hi, scope, subst));
                            Some(g.node_fmt(format_args!("[\"TypeDeclaration\",{},\"Public\",0,[],[],{},{}]", jstr(&name), lo, hi)))
                        }
                        _ => None,
                    };
                    let decls = decl.map(|d| format!("[{}]", d)).unwrap_or_else(|| "[]".to_string());
                    let key = StructKey { kind: StructKind::Refined, origin: None, parts: vec![acc, decl.unwrap_or(NO_INDEX)], params: Vec::new() };
                    acc = g.structure(key, format_args!("[\"Structure\",[{}],{},[]]", acc, decls));
                }
                acc
            }
            TType::Rec(addr, inner) => {
                let n = self.ty(g, src, inner, scope, subst);
                if let Some(&r) = g.rec_this.get(&(src, *addr)) {
                    g.fill(r, format!("[\"Structure\",[{}],[],[]]", n));
                }
                n
            }
            TType::RecThis(addr) => {
                if let Some(&r) = g.rec_this.get(&(src, *addr)) {
                    return r;
                }
                let r = g.reserve();
                g.fill(r, "[\"Structure\",[],[],[]]".to_string());
                g.rec_this.insert((src, *addr), r);
                r
            }
            TType::Super(this, sup) => {
                let (a, b) = (self.ty(g, src, this, scope, subst), self.ty(g, src, sup, scope, subst));
                let key = self.struct_key(StructKind::Super, origin, vec![a, b], src, t, subst);
                let s = g.structure(key, format_args!("[\"Structure\",[{},{}],[],[]]", a, b));
                let m = marker(g, "Super");
                g.node_fmt(format_args!("[\"Annotated\",{},[[{},[]]]]", s, m))
            }
            TType::Lambda { params, result, kind, .. } => self.lambda(g, src, params, result, *kind == LambdaKind::Type, None, scope, subst),
            TType::ParamRef(binder, i) => {
                if let Some((n, _)) = subst.get(&(src, *binder, *i)) {
                    return *n;
                }
                let name = self.lambda_param_name(src, *binder, *i);
                g.node_fmt(format_args!("[\"ParameterRef\",{}]", jstr(&name)))
            }
            TType::Match { scrutinee, bound, cases } => {
                let b = match bound {
                    Some(b) => self.ty(g, src, b, scope, subst),
                    None => scala_type(g, "Any"),
                };
                let s = self.ty(g, src, scrutinee, scope, subst);
                let mut parts = vec![b, s];
                for c in cases {
                    let mut inner = scope.clone();
                    inner.params.extend(c.binders.iter().map(|b| b.0));
                    if let Some((_, ps)) = &c.lambda {
                        inner.params.extend(ps.iter().map(|p| p.addr).filter(|&a| a != 0));
                    }
                    let p = self.ty(g, src, &c.pattern, &inner, subst);
                    let body = self.ty(g, src, &c.body, &inner, subst);
                    let mc = runtime_type(g, "MatchCase");
                    let case = g.node_fmt(format_args!("[\"Parameterized\",{},[{},{}]]", mc, p, body));
                    let case = match &c.lambda {
                        Some((_, ps)) if !ps.is_empty() => {
                            let tps: Vec<String> = ps.iter().map(|q| self.type_parameter(g, src, q, &inner, true)).collect();
                            g.node_fmt(format_args!("[\"Polymorphic\",{},{}]", case, list(tps)))
                        }
                        _ => case,
                    };
                    parts.push(case);
                }
                let key = self.struct_key(StructKind::Match, origin, parts.clone(), src, t, subst);
                let st = g.structure(key, format_args!("[\"Structure\",{},[],[]]", refs(&parts)));
                let m = marker(g, "Match");
                g.node_fmt(format_args!("[\"Annotated\",{},[[{},[]]]]", st, m))
            }
            TType::MatchCase(p, b) => {
                let (p, b) = (self.ty(g, src, p, scope, subst), self.ty(g, src, b, scope, subst));
                let mc = runtime_type(g, "MatchCase");
                g.node_fmt(format_args!("[\"Parameterized\",{},[{},{}]]", mc, p, b))
            }
            TType::Const(c) => {
                let f = self.file(src);
                self.constant(g, src, &f, c)
            }
            TType::Unknown(tag) => {
                let what = if *tag == 0 { "a type nested deeper than the TASTy reader reads".to_string() } else { format!("a type of TASTy tag {} the TASTy reader does not read", tag) };
                let error = format!("{}: {}, which the analysis graph cannot state", self.place(), what);
                if !self.errors.contains(&error) {
                    self.errors.push(error);
                }
                g_empty(g)
            }
        }
    }

    /// A lambda as `Polymorphic`, its parameters' variances the declared ones or `variances`.
    #[allow(clippy::too_many_arguments)]
    fn lambda(&mut self, g: &mut Graph, src: Src, params: &[TParam], result: &TType, variant: bool, variances: Option<&[i8]>, scope: &Scope, subst: &Subst) -> u32 {
        let mut inner = scope.clone();
        inner.params.extend(params.iter().map(|p| p.addr).filter(|&a| a != 0));
        let res = self.ty(g, src, result, &inner, subst);
        let tps: Vec<String> = params
            .iter()
            .enumerate()
            .map(|(i, p)| match variances.and_then(|v| v.get(i)) {
                Some(&v) => {
                    let mut q = p.clone();
                    q.flags.clear(COVARIANT);
                    q.flags.clear(CONTRAVARIANT);
                    if v > 0 {
                        q.flags.set(COVARIANT);
                    } else if v < 0 {
                        q.flags.set(CONTRAVARIANT);
                    }
                    self.type_parameter(g, src, &q, &inner, true)
                }
                None => self.type_parameter(g, src, p, &inner, variant),
            })
            .collect();
        g.node_fmt(format_args!("[\"Polymorphic\",{},{}]", res, list(tps)))
    }

    /// An alias's right-hand side: a lambda's parameters as variant as their uses in its body
    /// allow, scalac's structural variances of an alias's lambda.
    fn alias_rhs(&mut self, g: &mut Graph, src: Src, t: &TType, scope: &Scope) -> u32 {
        if let TType::Lambda { kind: LambdaKind::Type, params, result, binder } = t {
            let v = self.structural_variances(src, *binder, params, result);
            return self.lambda(g, src, params, result, true, Some(&v), scope, &Subst::default());
        }
        self.ty(g, src, t, scope, &Subst::default())
    }

    /// scalac's `setStructuralVariances`: each parameter narrowed by every occurrence in the
    /// body, `+` where it does not occur.
    fn structural_variances(&mut self, src: Src, binder: Addr, params: &[TParam], body: &TType) -> Vec<i8> {
        // A lambda written as a tree states its body as an alias's bounds.
        let body = match body {
            TType::Alias(inner) => &**inner,
            other => other,
        };
        let mut v: Vec<u8> = vec![3; params.len()];
        self.narrow(src, binder, params, body, 1, &mut v, 0);
        v.iter().map(|&b| if b == 2 { -1 } else if b == 0 { 0 } else { 1 }).collect()
    }

    #[allow(clippy::too_many_arguments)]
    fn narrow(&mut self, src: Src, binder: Addr, params: &[TParam], t: &TType, variance: i8, v: &mut Vec<u8>, depth: u32) {
        if depth > 64 {
            return;
        }
        let bits = |variance: i8| -> u8 {
            if variance > 0 {
                1
            } else if variance < 0 {
                2
            } else {
                0
            }
        };
        let d = depth + 1;
        match t {
            TType::ParamRef(b, i) if *b == binder => {
                if let Some(x) = v.get_mut(*i as usize) {
                    *x &= bits(variance);
                }
            }
            TType::LocalType(addr, prefix) => {
                if let Some(i) = params.iter().position(|p| p.addr != 0 && p.addr == *addr) {
                    v[i] &= bits(variance);
                } else if let Some(p) = prefix {
                    self.narrow(src, binder, params, p, variance.max(0), v, d);
                }
            }
            TType::TypeRef(prefix, _) | TType::TermRef(prefix, _) => self.narrow(src, binder, params, prefix, variance.max(0), v, d),
            TType::LocalTerm(_, Some(prefix)) => self.narrow(src, binder, params, prefix, variance.max(0), v, d),
            TType::Applied(tycon, args) => {
                self.narrow(src, binder, params, tycon, variance, v, d);
                let signs = self.param_variances(src, tycon);
                for (i, a) in args.iter().enumerate() {
                    let sign = signs.get(i).copied().unwrap_or(0);
                    self.narrow(src, binder, params, a, variance * sign, v, d);
                }
            }
            TType::Bounds(lo, hi) => {
                self.narrow(src, binder, params, lo, -variance, v, d);
                self.narrow(src, binder, params, hi, variance, v, d);
            }
            TType::Alias(t) => self.narrow(src, binder, params, t, 0, v, d),
            TType::And(a, b) | TType::Or(a, b) => {
                self.narrow(src, binder, params, a, variance, v, d);
                self.narrow(src, binder, params, b, variance, v, d);
            }
            TType::Annotated(t, _) | TType::ByName(t) | TType::Flexible(t) | TType::Rec(_, t) => self.narrow(src, binder, params, t, variance, v, d),
            TType::Refined(parent, members) => {
                self.narrow(src, binder, params, parent, variance, v, d);
                for (_, info) in members.iter() {
                    if let Some(i) = info {
                        self.narrow(src, binder, params, i, variance, v, d);
                    }
                }
            }
            TType::Lambda { params: inner, result, .. } => {
                for p in inner {
                    self.narrow(src, binder, params, &p.info, -variance, v, d);
                }
                self.narrow(src, binder, params, result, variance, v, d);
            }
            TType::Match { scrutinee, bound, cases } => {
                if let Some(b) = bound {
                    self.narrow(src, binder, params, b, variance, v, d);
                }
                self.narrow(src, binder, params, scrutinee, 0, v, d);
                for c in cases {
                    self.narrow(src, binder, params, &c.pattern, 0, v, d);
                    self.narrow(src, binder, params, &c.body, variance, v, d);
                }
            }
            _ => {}
        }
    }

    /// The variances of the parameters of a type constructor: a class's declared ones, an
    /// alias's structural ones.
    fn param_variances(&mut self, src: Src, tycon: &TType) -> Vec<i8> {
        if let Some((at, TType::Lambda { kind: LambdaKind::Type, params, result, binder })) = self.alias_of(src, tycon) {
            return self.structural_variances(at.src, binder, &params, &result);
        }
        match self.class_of(src, tycon) {
            Some(Cls::Tasty(at)) => {
                let d = self.class_data(at);
                d.tparams.iter().map(|p| if p.flags.has(COVARIANT) { 1 } else if p.flags.has(CONTRAVARIANT) { -1 } else { 0 }).collect()
            }
            _ => Vec::new(),
        }
    }

    /// A type argument: a wildcard is `EmptyType` when it has no bounds, otherwise an
    /// existential over a parameter `_` with them.
    fn type_arg(&mut self, g: &mut Graph, src: Src, t: &TType, scope: &Scope, subst: &Subst) -> u32 {
        if let Some(n) = self.eta_expanded(g, src, t, scope, subst) {
            return n;
        }
        if let TType::Bounds(lo, hi) = t {
            let f = self.file(src);
            if is_scala_nothing(&f, lo) && is_scala_any(&f, hi) {
                return g_empty(g);
            }
            let (lo, hi) = (self.ty(g, src, lo, scope, subst), self.ty(g, src, hi, scope, subst));
            let r = g.node("[\"ParameterRef\",\"_\"]".to_string());
            return g.node_fmt(format_args!("[\"Existential\",{},[[\"_\",[],[],\"Invariant\",{},{}]]]", r, lo, hi));
        }
        self.ty(g, src, t, scope, subst)
    }

    /// A class that takes type parameters named without arguments where a type constructor
    /// is expected, as scalac's typer eta-expands it: `[+A] =>> Option[A]`.
    fn eta_expanded(&mut self, g: &mut Graph, src: Src, t: &TType, scope: &Scope, subst: &Subst) -> Option<u32> {
        if !matches!(t, TType::TypeRef(..) | TType::LocalType(..)) || self.alias_of(src, t).is_some() {
            return None;
        }
        let Some(Cls::Tasty(at)) = self.class_of(src, t) else { return None };
        let d = self.class_data(at);
        if d.tparams.is_empty() {
            return None;
        }
        let f = self.file(at.src);
        let base = self.ty(g, src, t, scope, subst);
        let names: Vec<String> = d.tparams.iter().map(|p| self.name(&f, p.name)).collect();
        let args: Vec<u32> = names.iter().map(|n| g.node_fmt(format_args!("[\"ParameterRef\",{}]", jstr(n)))).collect();
        let body = g.node_fmt(format_args!("[\"Parameterized\",{},{}]", base, refs(&args)));
        let tparams: Vec<String> = d.tparams.clone().iter().map(|p| self.type_parameter(g, at.src, p, &Scope::default(), true)).collect();
        Some(g.node_fmt(format_args!("[\"Polymorphic\",{},{}]", body, list(tparams))))
    }

    /// A prefix: a package's member is seen from the package's `this`.
    fn prefix(&mut self, g: &mut Graph, src: Src, p: &TType, scope: &Scope, subst: &Subst) -> u32 {
        match p {
            TType::Package(_) => self.ty(g, src, p, scope, subst),
            TType::This(inner) if matches!(**inner, TType::Package(_)) => self.ty(g, src, inner, scope, subst),
            other => self.ty(g, src, other, scope, subst),
        }
    }

    /// The prefix of the term `name` read through `prefix`: a package's own object is its
    /// member, seen from the package's `this`; any other term of a package is a member of a
    /// file's `$package` object, and the package stays as the path it was named by.
    fn term_prefix(&mut self, g: &mut Graph, src: Src, prefix: &TType, name: &str, scope: &Scope, subst: &Subst) -> u32 {
        if let Some((pkg, classes)) = self.ref_path(src, prefix) {
            if classes.is_empty() && !pkg.is_empty() {
                let object = self.resolve(&pkg, &[format!("{}$", name)]).is_some();
                if !object {
                    let (parent, last) = pkg.split_at(pkg.len() - 1);
                    let parent = self.this_path(g, parent, parent.len());
                    let last = self.package_names(&pkg, pkg.len()).pop().unwrap_or_else(|| last[0].clone());
                    return g.node_fmt(format_args!("[\"Projection\",{},{}]", parent, jstr(&last)));
                }
                return self.this_path(g, &pkg, pkg.len());
            }
        }
        self.prefix(g, src, prefix, scope, subst)
    }

    /// Whether the definition at `addr` is inside a class of its pickle.
    fn inside_a_class(&mut self, src: Src, addr: Addr) -> bool {
        let paths = self.paths_of(src);
        let f = self.file(src);
        paths.keys().any(|&a| a < addr && addr < Decoder::new(&f).tree_end(a))
    }

    /// The `this` of the owner of the definition at `addr`: a class of the pickle.
    fn owner_this(&mut self, g: &mut Graph, src: Src, addr: Addr) -> u32 {
        let paths = self.paths_of(src);
        // The innermost class whose template holds the address.
        let f = self.file(src);
        let mut best: Option<(Addr, &ClassPath)> = None;
        for (&a, p) in paths.iter() {
            let end = Decoder::new(&f).tree_end(a);
            if a < addr && addr < end && best.map_or(true, |(b, _)| a > b) {
                best = Some((a, p));
            }
        }
        let (path, pkg_len) = best.map(|(_, p)| (p.path.clone(), p.pkg_len)).unwrap_or_default();
        self.this_path(g, &path, pkg_len)
    }

    /// A path's names as scalac's symbols carry them: a package the class path holds was
    /// entered by its directory's name, operators encoded (`$plus` for `+`); a package only the
    /// sources declare and a class keep their names. The first `pkg_len` names are packages.
    fn package_names(&mut self, path: &[String], pkg_len: usize) -> Vec<String> {
        let mut out = Vec::with_capacity(path.len());
        let mut dir = String::new();
        for (i, p) in path.iter().enumerate() {
            if i >= pkg_len {
                out.push(p.clone());
                continue;
            }
            let encoded = encode_package(p);
            if !dir.is_empty() {
                dir.push('/');
            }
            dir.push_str(&encoded);
            if encoded == *p {
                out.push(encoded);
                continue;
            }
            let on_classpath = match self.directories.get(&dir) {
                Some(&found) => found,
                None => {
                    let found = self.w.loaded.as_ref().map_or(false, |l| l.cp.packages.iter().any(|q| q.path == dir || q.path.starts_with(&format!("{}/", dir))));
                    self.directories.insert(dir.clone(), found);
                    found
                }
            };
            out.push(if on_classpath { encoded } else { p.clone() });
        }
        out
    }

    fn lambda_param_name(&mut self, src: Src, binder: Addr, i: u32) -> String {
        let f = self.file(src);
        match Decoder::new(&f).type_at(binder) {
            TType::Lambda { params, .. } => params.get(i as usize).map(|p| self.name(&f, p.name)).unwrap_or_default(),
            _ => String::new(),
        }
    }

    /// A reference to an alias rendered as what the alias stands for, scalac's `dealias`: the
    /// right-hand side read in the alias's pickle, an applied alias's lambda applied to the
    /// arguments rendered where they stand. A reference to an alias of a lambda that is not
    /// applied stays (`isLambdaSub`), and so does an opaque type outside its scope.
    fn struct_key(&mut self, kind: StructKind, origin: Option<Origin>, parts: Vec<u32>, src: Src, t: &TType, subst: &Subst) -> StructKey {
        let params = self.param_ids(src, t, subst);
        StructKey { kind, origin, parts, params }
    }

    /// The parameters `t` refers to, those an argument of `subst` stands for as the argument's.
    fn param_ids(&mut self, src: Src, t: &TType, subst: &Subst) -> Vec<ParamId> {
        let mut out = Vec::new();
        self.collect_param_ids(src, t, subst, &mut out);
        out
    }

    fn collect_param_ids(&mut self, src: Src, t: &TType, subst: &Subst, out: &mut Vec<ParamId>) {
        macro_rules! each {
            ($t:expr) => {
                self.collect_param_ids(src, $t, subst, out)
            };
        }
        match t {
            TType::ParamRef(binder, i) => match subst.get(&(src, *binder, *i)) {
                Some((_, ids)) => out.extend(ids.iter().copied()),
                None => out.push((src, *binder, *i)),
            },
            TType::LocalType(addr, prefix) | TType::LocalTerm(addr, prefix) => {
                if let Some((_, ids)) = subst.get(&(src, *addr, NO_INDEX)) {
                    out.extend(ids.iter().copied());
                    return;
                }
                let tag = Decoder::new(&self.file(src)).tag_at(*addr);
                if tag == TYPEPARAM || tag == BIND || tag == PARAM {
                    out.push((src, *addr, NO_INDEX));
                }
                if let Some(p) = prefix {
                    each!(p);
                }
            }
            TType::TypeRef(p, _) | TType::TermRef(p, _) | TType::This(p) | TType::Alias(p) | TType::ByName(p) | TType::Flexible(p) => each!(p),
            TType::Applied(a, args) => {
                each!(a);
                for x in args {
                    each!(x);
                }
            }
            TType::Bounds(a, b) | TType::BoundedAlias(a, b) | TType::And(a, b) | TType::Or(a, b) | TType::Annotated(a, b) | TType::Super(a, b) | TType::MatchCase(a, b) => {
                each!(a);
                each!(b);
            }
            TType::Refined(p, members) => {
                each!(p);
                for (_, m) in members {
                    if let Some(m) = m {
                        each!(m);
                    }
                }
            }
            TType::Rec(_, p) => each!(p),
            TType::Lambda { params, result, .. } => {
                for p in params {
                    each!(&p.info);
                }
                each!(result);
            }
            TType::Match { scrutinee, bound, cases } => {
                each!(scrutinee);
                if let Some(b) = bound {
                    each!(b);
                }
                for c in cases {
                    each!(&c.pattern);
                    each!(&c.body);
                }
            }
            TType::Package(_) | TType::RecThis(_) | TType::Const(_) | TType::Unknown(_) => {}
        }
    }

    /// The type an alias `t` names, followed; `origin` the alias an outer one was written as,
    /// which the type's structure is one per.
    fn dealiased(&mut self, g: &mut Graph, src: Src, t: &TType, scope: &Scope, subst: &Subst, origin: Option<Origin>) -> Option<u32> {
        match t {
            TType::LocalType(..) | TType::TypeRef(..) => {
                let (at, rhs) = self.alias_of(src, t)?;
                let rhs = match rhs {
                    TType::Lambda { .. } | TType::Bounds(..) => return None,
                    TType::Alias(inner) => *inner,
                    other => other,
                };
                self.origin = Some(origin.unwrap_or((at, Vec::new(), Vec::new())));
                Some(self.ty(g, at.src, &rhs, &Scope::default(), &Subst::default()))
            }
            TType::Applied(tycon, args) => {
                let (at, rhs) = self.alias_of(src, tycon)?;
                let TType::Lambda { kind: LambdaKind::Type, params, result, binder } = rhs else { return None };
                if matches!(*result, TType::Bounds(..)) {
                    return None;
                }
                let mut inner = Subst::default();
                let mut nodes = Vec::new();
                let mut all_ids = Vec::new();
                for (i, (p, a)) in params.iter().zip(args.iter()).enumerate() {
                    let node = self.type_arg(g, src, a, scope, subst);
                    let ids = self.param_ids(src, a, subst);
                    nodes.push(node);
                    all_ids.extend(ids.iter().copied());
                    if p.addr != 0 {
                        inner.insert((at.src, p.addr, NO_INDEX), (node, ids.clone()));
                    }
                    inner.insert((at.src, binder, i as u32), (node, ids));
                }
                self.origin = Some(origin.unwrap_or((at, nodes, all_ids)));
                Some(self.ty(g, at.src, &result, &Scope::default(), &inner))
            }
            _ => None,
        }
    }

    /// The alias a type reference names and its right-hand side: not an opaque type, unless
    /// named from inside the object that defines it (`this.T`), where scalac sees its alias.
    fn alias_of(&mut self, src: Src, t: &TType) -> Option<(At, TType)> {
        let (at, prefix) = match t {
            TType::LocalType(addr, prefix) => {
                let f = self.file(src);
                if Decoder::new(&f).tag_at(*addr) != TYPEDEF || Decoder::new(&f).local_type_kind(*addr) == crate::tasty::tree::LocalKind::Class {
                    return None;
                }
                (At { src, addr: *addr }, prefix.as_deref().cloned())
            }
            TType::TypeRef(prefix, n) => {
                let f = self.file(src);
                let name = self.name(&f, *n);
                (self.alias_target(src, prefix, &name)?, Some((**prefix).clone()))
            }
            _ => return None,
        };
        let tf = self.file(at.src);
        let sig = Decoder::new(&tf).type_def_sig(at.addr);
        // A match type's alias is scalac's `MatchAlias`, which `dealias` does not follow.
        let body = match &sig.rhs {
            TType::Lambda { result, .. } => &**result,
            other => other,
        };
        if matches!(body, TType::Match { .. }) {
            return None;
        }
        if sig.mods.flags.has(OPAQUE) {
            let inside = matches!(&prefix, Some(TType::This(_)));
            if !inside {
                return None;
            }
            return Some((at, sig.rhs));
        }
        Some((at, sig.rhs))
    }

    /// The `TYPEDEF` of the alias `prefix.name`, where one is found.
    fn alias_target(&mut self, src: Src, prefix: &TType, name: &str) -> Option<At> {
        let owner = match prefix {
            TType::Package(_) => None,
            TType::This(inner) => match &**inner {
                TType::Package(_) => None,
                other => self.class_of(src, other),
            },
            other => {
                let (pkg, classes) = self.ref_path(src, other)?;
                if classes.is_empty() {
                    None
                } else {
                    self.resolve(&pkg, &classes)
                }
            }
        };
        let Some(Cls::Tasty(at)) = owner else { return None };
        let f = self.file(at.src);
        let d = self.class_data(at);
        for m in d.members.iter() {
            if m.tag == TYPEDEF && !m.is_class && self.name(&f, m.name) == name {
                return Some(At { src: at.src, addr: m.addr });
            }
        }
        None
    }

    /// scalac's `Constant.of(apiType(constant.tpe), constant.stringValue)`.
    fn constant(&mut self, g: &mut Graph, src: Src, f: &TastyFile, c: &Const) -> u32 {
        let (tpe, value) = match c {
            Const::Unit => (scala_type(g, "Unit"), "()".to_string()),
            Const::Bool(b) => (scala_type(g, "Boolean"), b.to_string()),
            Const::Byte(v) => (scala_type(g, "Byte"), v.to_string()),
            Const::Short(v) => (scala_type(g, "Short"), v.to_string()),
            Const::Char(v) => (scala_type(g, "Char"), char::from_u32(*v).map(|c| c.to_string()).unwrap_or_default()),
            Const::Int(v) => (scala_type(g, "Int"), v.to_string()),
            Const::Long(v) => (scala_type(g, "Long"), v.to_string()),
            Const::Float(bits) => (scala_type(g, "Float"), java_float(f32::from_bits(*bits) as f64, true)),
            Const::Double(bits) => (scala_type(g, "Double"), java_float(f64::from_bits(*bits), false)),
            Const::Str(n) => (java_lang_type(g, "String"), f.name(*n)),
            Const::Null => (scala_type(g, "Null"), "null".to_string()),
            Const::Class(t) => {
                let arg = self.ty(g, src, t, &Scope::default(), &Subst::default());
                let class = java_lang_type(g, "Class");
                (g.node_fmt(format_args!("[\"Parameterized\",{},[{}]]", class, arg)), String::new())
            }
        };
        g.node_fmt(format_args!("[\"Constant\",{},{}]", tpe, jstr(&value)))
    }
}

/// A declaration of a class as the structure lists it.
#[derive(Clone)]
enum Decl {
    ClassTypeParam(At, TParam),
    Accessor(At, Param),
    /// A definition of the template, and whether it is private (not `private[p]`).
    Member(At, Addr, bool),
    Root(Root),
    JavaInit(ClassId),
    /// The constructor proxy object of the class at the address, and its val.
    ProxyModule(At),
    ProxyVal(At),
    /// An `apply` of a proxy, standing for the class's constructor at the address.
    ProxyApply(At, Addr),
    /// The proxy of an exported type: the exporting class, the type's definition, the class.
    ProxyType(At, Addr, TType),
}

impl Decl {
    /// The class a declaration belongs to and the definition's address, for an error to name.
    fn place(&self) -> Option<(At, Option<Addr>)> {
        match self {
            Decl::ClassTypeParam(at, p) => Some((*at, Some(p.addr))),
            Decl::Accessor(at, p) => Some((*at, Some(p.addr))),
            Decl::Member(at, a, _) | Decl::ProxyApply(at, a) | Decl::ProxyType(at, a, _) => Some((*at, Some(*a))),
            Decl::ProxyModule(at) | Decl::ProxyVal(at) => Some((*at, None)),
            Decl::Root(_) | Decl::JavaInit(_) => None,
        }
    }

    fn key(&self) -> DeclKey {
        match self {
            Decl::ClassTypeParam(at, p) => DeclKey::ClassTypeParam(*at, p.addr),
            Decl::Accessor(at, p) => DeclKey::Accessor(*at, p.addr),
            Decl::Member(at, a, _) => DeclKey::Member(*at, *a),
            Decl::Root(r) => DeclKey::Root(*r as u8),
            Decl::JavaInit(k) => DeclKey::JavaInit(*k),
            Decl::ProxyModule(at) => DeclKey::ProxyModule(*at),
            Decl::ProxyVal(at) => DeclKey::ProxyVal(*at),
            Decl::ProxyApply(at, a) => DeclKey::ProxyApply(*at, *a),
            Decl::ProxyType(at, a, _) => DeclKey::ProxyType(*at, *a),
        }
    }

    fn addr(&self) -> Addr {
        match self {
            Decl::ClassTypeParam(_, p) => p.addr,
            Decl::Accessor(_, p) => p.addr,
            Decl::Member(_, a, _) => *a,
            Decl::Root(_) | Decl::JavaInit(_) | Decl::ProxyModule(_) | Decl::ProxyVal(_) | Decl::ProxyApply(..) | Decl::ProxyType(..) => 0,
        }
    }

    /// Private members are not inherited: `private` and `private[this]`, which a class's type
    /// parameters are, but not `private[p]`.
    fn private(&self) -> bool {
        match self {
            Decl::ClassTypeParam(..) => true,
            Decl::Accessor(_, p) => p.flags.has(PRIVATE),
            Decl::Member(_, _, private) => *private,
            Decl::Root(_) | Decl::JavaInit(_) | Decl::ProxyModule(_) | Decl::ProxyVal(_) | Decl::ProxyApply(..) | Decl::ProxyType(..) => false,
        }
    }
}

/// Marks the address of a class whose constructor proxy object is meant, in the key of the
/// classes a graph has made: no pickle is that large.
const PROXY_BIT: Addr = 1 << 31;

/// The members of the classes the compiler defines, `scala.Any` and `java.lang.Object` as
/// scalac enters them.
#[derive(Clone, Copy)]
enum Root {
    ObjectInit,
    Eq,
    Ne,
    Synchronized,
    Clone,
    Finalize,
    Notify,
    NotifyAll,
    Wait0,
    Wait1,
    Wait2,
    EqEq,
    NotEq,
    Equals,
    HashCode,
    ToString,
    HashHash,
    GetClass,
    IsInstanceOf,
    AsInstanceOf,
    IsInstanceOfInternal,
    AsInstanceOfInternal,
}

const ROOT_OBJECT: [Root; 11] = [Root::ObjectInit, Root::Eq, Root::Ne, Root::Synchronized, Root::Clone, Root::Finalize, Root::Notify, Root::NotifyAll, Root::Wait0, Root::Wait1, Root::Wait2];
const ROOT_ANY: [Root; 11] = [
    Root::EqEq,
    Root::NotEq,
    Root::Equals,
    Root::HashCode,
    Root::ToString,
    Root::HashHash,
    Root::GetClass,
    Root::IsInstanceOf,
    Root::AsInstanceOf,
    Root::IsInstanceOfInternal,
    Root::AsInstanceOfInternal,
];

fn root_member(g: &mut Graph, r: Root) -> u32 {
    let object = java_lang_type(g, "Object");
    let boolean = scala_type(g, "Boolean");
    let unit = scala_type(g, "Unit");
    let int = scala_type(g, "Int");
    let long = scala_type(g, "Long");
    let any = scala_type(g, "Any");
    let nothing = scala_type(g, "Nothing");
    let string = java_lang_type(g, "String");
    let x0 = g.node("[\"ParameterRef\",\"X0\"]".to_string());
    let final_ = bits::FINAL;
    let param = |n: &str, t: u32| format!("[{},{},false,\"Plain\"]", jstr(n), t);
    let plist = |ps: Vec<String>| format!("[{},false]", list(ps));
    let tparam = |lo: u32, hi: u32| format!("[\"X0\",[],[],\"Invariant\",{},{}]", lo, hi);
    let def = |g: &mut Graph, name: &str, access: &str, mods: u8, tparams: Vec<String>, plists: Vec<String>, ret: u32| {
        g.node_fmt(format_args!("[\"Def\",{},{},{},[],{},{},{}]", jstr(name), access, mods, list(tparams), list(plists), ret))
    };
    let public = "\"Public\"";
    let protected = "[\"Protected\",\"Unqualified\"]";
    match r {
        Root::ObjectInit => def(g, "java;lang;Object;init;", public, 0, vec![], vec![plist(vec![])], object),
        Root::Eq => def(g, "eq", public, final_, vec![], vec![plist(vec![param("x$0", object)])], boolean),
        Root::Ne => def(g, "ne", public, final_, vec![], vec![plist(vec![param("x$0", object)])], boolean),
        Root::Synchronized => def(g, "synchronized", public, final_, vec![tparam(nothing, any)], vec![plist(vec![param("x$0", x0)])], x0),
        Root::Clone => def(g, "clone", protected, 0, vec![], vec![plist(vec![])], object),
        Root::Finalize => def(g, "finalize", protected, 0, vec![], vec![plist(vec![])], unit),
        Root::Notify => def(g, "notify", public, final_, vec![], vec![plist(vec![])], unit),
        Root::NotifyAll => def(g, "notifyAll", public, final_, vec![], vec![plist(vec![])], unit),
        Root::Wait0 => def(g, "wait", public, final_, vec![], vec![plist(vec![])], unit),
        Root::Wait1 => def(g, "wait", public, final_, vec![], vec![plist(vec![param("x$0", long)])], unit),
        Root::Wait2 => def(g, "wait", public, final_, vec![], vec![plist(vec![param("x$0", long), param("x$1", int)])], unit),
        Root::EqEq => def(g, "==", public, final_, vec![], vec![plist(vec![param("x$0", any)])], boolean),
        Root::NotEq => def(g, "!=", public, final_, vec![], vec![plist(vec![param("x$0", any)])], boolean),
        Root::Equals => def(g, "equals", public, 0, vec![], vec![plist(vec![param("x$0", any)])], boolean),
        Root::HashCode => def(g, "hashCode", public, 0, vec![], vec![plist(vec![])], int),
        Root::ToString => def(g, "toString", public, 0, vec![], vec![plist(vec![])], string),
        Root::HashHash => def(g, "##", public, final_, vec![], vec![], int),
        Root::GetClass => {
            let any_this = g.node("[\"Singleton\",[[\"Id\",\"scala\"],[\"Id\",\"Any\"],[\"This\"]]]".to_string());
            let r = g.node("[\"ParameterRef\",\"_\"]".to_string());
            let ex = g.node_fmt(format_args!("[\"Existential\",{},[[\"_\",[],[],\"Invariant\",{},{}]]]", r, nothing, x0));
            let class = java_lang_type(g, "Class");
            let ret = g.node_fmt(format_args!("[\"Parameterized\",{},[{}]]", class, ex));
            def(g, "getClass", public, final_, vec![tparam(any_this, any)], vec![plist(vec![])], ret)
        }
        Root::IsInstanceOf => def(g, "isInstanceOf", public, final_, vec![tparam(nothing, any)], vec![], boolean),
        Root::AsInstanceOf => def(g, "asInstanceOf", public, final_, vec![tparam(nothing, any)], vec![], x0),
        Root::IsInstanceOfInternal => def(g, "$isInstanceOf", public, final_, vec![tparam(nothing, any)], vec![], boolean),
        Root::AsInstanceOfInternal => def(g, "$asInstanceOf", public, final_, vec![tparam(nothing, any)], vec![], x0),
    }
}

fn g_empty(g: &mut Graph) -> u32 {
    g.node("[\"EmptyType\"]".to_string())
}

/// A class of one of the packages below, `scala.this#Int`, made once per graph.
fn package_type(g: &mut Graph, package: u8, name: &'static str) -> u32 {
    if let Some(&n) = g.fixed.get(&(package, name)) {
        return n;
    }
    let path = ["[[\"Id\",\"scala\"],[\"This\"]]", "[[\"Id\",\"java\"],[\"Id\",\"lang\"],[\"This\"]]", "[[\"Id\",\"scala\"],[\"Id\",\"runtime\"],[\"This\"]]"][package as usize];
    let p = g.node_fmt(format_args!("[\"Singleton\",{}]", path));
    let n = g.node_fmt(format_args!("[\"Projection\",{},{}]", p, jstr(name)));
    g.fixed.insert((package, name), n);
    n
}

/// A type of package `scala`, `scala.this#Int`.
fn scala_type(g: &mut Graph, name: &'static str) -> u32 {
    package_type(g, 0, name)
}

fn java_lang_type(g: &mut Graph, name: &'static str) -> u32 {
    package_type(g, 1, name)
}

fn runtime_type(g: &mut Graph, name: &'static str) -> u32 {
    package_type(g, 2, name)
}

/// `ExtractAPI`'s marker annotations for the types `xsbti.api` has no case for.
fn marker(g: &mut Graph, name: &str) -> u32 {
    let e = g_empty(g);
    g.node_fmt(format_args!("[\"Constant\",{},{}]", e, jstr(name)))
}

/// The binary name of a class of `path`, of which the first `pkg_len` names are its package's:
/// `p.O$Inner` for `Inner` in object `O`, `p.C$D` for `D` in class `C`, operators encoded.
fn binary_name(path: &[String], pkg_len: usize) -> String {
    let mut out: Vec<String> = path[..pkg_len.min(path.len())].iter().map(|p| crate::jvm::names::encode(p)).collect();
    let mut class = String::new();
    for c in path.iter().skip(pkg_len) {
        if !class.is_empty() && !class.ends_with('$') {
            class.push('$');
        }
        class.push_str(&crate::jvm::names::encode(c));
    }
    out.push(class);
    out.join(".")
}

/// A name in a path as scalac keeps a package's: an operator encoded (`$plus` for `+`). A class's
/// name has no operator characters to encode but in its `$`s, which stay.
fn encode_package(name: &str) -> String {
    crate::jvm::names::encode(name)
}

fn package_segments(f: &TastyFile, p: NameRef) -> Vec<String> {
    let mut segs = Vec::new();
    f.segments(p, &mut segs);
    segs.into_iter().map(|s| f.name(s)).filter(|s| !s.is_empty() && s != "<empty>" && s != "<root>" && s != "_root_").collect()
}

/// Whether `t` is `scala.annotation.internal.<name>`, applied or not.
fn names_class(f: &TastyFile, t: &TType, name: &str) -> bool {
    match t {
        TType::TypeRef(prefix, n) => {
            let internal = |t: &TType| matches!(t, TType::Package(p) if package_segments(f, *p) == ["scala", "annotation", "internal"]);
            f.simple(*n) == Some(name) && (internal(prefix) || matches!(&**prefix, TType::This(inner) if internal(inner)))
        }
        TType::Applied(tycon, _) => names_class(f, tycon, name),
        _ => false,
    }
}

fn is_scala_named(f: &TastyFile, t: &TType, name: &str) -> bool {
    match t {
        TType::TypeRef(prefix, n) => f.simple(*n) == Some(name) && matches!(&**prefix, TType::Package(p) if package_segments(f, *p) == ["scala"]) || matches!(&**prefix, TType::This(inner) if matches!(&**inner, TType::Package(p) if package_segments(f, *p) == ["scala"])) && f.simple(*n) == Some(name),
        _ => false,
    }
}

fn is_scala_nothing(f: &TastyFile, t: &TType) -> bool {
    is_scala_named(f, t, "Nothing")
}

fn is_scala_any(f: &TastyFile, t: &TType) -> bool {
    is_scala_named(f, t, "Any")
}

/// A name as scalac's `Name.toString` gives it.
fn write_name(f: &TastyFile, n: NameRef, out: &mut String) {
    match f.names.get(n as usize) {
        Some(TName::DefaultGetter(u, index)) => {
            let mut under = String::new();
            write_name(f, *u, &mut under);
            if under == "<init>" {
                out.push_str("$lessinit$greater");
            } else {
                out.push_str(&under);
            }
            out.push_str("$default$");
            out.push_str(&(index + 1).to_string());
        }
        Some(TName::Signed { original, .. }) => write_name(f, *original, out),
        // scalac names a trait's super accessor `T$$super$m`, the expansion outside.
        Some(TName::SuperAccessor(u)) => match f.names.get(*u as usize) {
            Some(TName::Expanded(prefix, name)) => {
                write_name(f, *prefix, out);
                out.push_str("$$super$");
                write_name(f, *name, out);
            }
            _ => {
                out.push_str("super$");
                write_name(f, *u, out);
            }
        },
        _ => f.write_name(n, out),
    }
}

/// A type's shape as far as it names a class: the key of a cache of what it resolves to.
fn type_key(t: &TType) -> Vec<u32> {
    fn walk(t: &TType, out: &mut Vec<u32>) {
        match t {
            TType::Package(n) => out.extend([1, *n]),
            TType::TypeRef(p, n) => {
                out.extend([2, *n]);
                walk(p, out);
            }
            TType::TermRef(p, n) => {
                out.extend([3, *n]);
                walk(p, out);
            }
            TType::LocalType(a, p) | TType::LocalTerm(a, p) => {
                out.extend([if matches!(t, TType::LocalType(..)) { 4 } else { 5 }, *a]);
                if let Some(p) = p {
                    walk(p, out);
                }
            }
            TType::This(p) => {
                out.push(6);
                walk(p, out);
            }
            TType::Annotated(p, _) => {
                out.push(7);
                walk(p, out);
            }
            TType::Applied(p, _) => {
                out.push(8);
                walk(p, out);
            }
            _ => out.push(0),
        }
    }
    let mut out = Vec::with_capacity(8);
    walk(t, &mut out);
    out
}

/// Java's `Float.toString` and `Double.toString`: the shortest digits that read back, in plain
/// notation between 10^-3 and 10^7 and in computerized scientific notation outside.
fn java_float(v: f64, single: bool) -> String {
    if v.is_nan() {
        return "NaN".to_string();
    }
    if v.is_infinite() {
        return if v > 0.0 { "Infinity" } else { "-Infinity" }.to_string();
    }
    if v == 0.0 {
        return if v.is_sign_negative() { "-0.0" } else { "0.0" }.to_string();
    }
    let text = if single { format!("{:e}", v as f32) } else { format!("{:e}", v) };
    let (mantissa, exp) = text.split_once('e').unwrap_or((&text, "0"));
    let exp: i32 = exp.parse().unwrap_or(0);
    let negative = mantissa.starts_with('-');
    let digits: String = mantissa.chars().filter(|c| c.is_ascii_digit()).collect();
    let sign = if negative { "-" } else { "" };
    let abs = v.abs();
    if (1e-3..1e7).contains(&abs) {
        let point = exp + 1;
        let (int_part, frac_part) = if point <= 0 {
            ("0".to_string(), format!("{}{}", "0".repeat((-point) as usize), digits))
        } else if (point as usize) >= digits.len() {
            (format!("{}{}", digits, "0".repeat(point as usize - digits.len())), String::new())
        } else {
            (digits[..point as usize].to_string(), digits[point as usize..].to_string())
        };
        let frac = if frac_part.is_empty() { "0".to_string() } else { frac_part };
        format!("{}{}.{}", sign, int_part, frac)
    } else {
        let frac = if digits.len() > 1 { digits[1..].to_string() } else { "0".to_string() };
        format!("{}{}.{}E{}", sign, &digits[..1], frac, exp)
    }
}

fn fnv(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

// ---- annotations and their trees ----

/// scalac's `treeHash` of `new C()` (`Apply(Select(New(TypeTree()), <init>), Nil)`), what an
/// annotation of a type is taken to be.
const EMPTY_ANNOTATION_HASH: i32 = 779890520;

/// The type of an annotation's constructor call, when it names type arguments (`@Child[C]`).
fn annotation_type(f: &TastyFile, tree_at: Addr) -> Option<TType> {
    let t = Decoder::new(f).call_type(tree_at);
    matches!(t, TType::Applied(..)).then_some(t)
}

/// scalac's `ExtractAPI.treeHash` over a tree as TASTy writes it: per node its class's name and
/// its fields in order, names by their kind and text, constants by their tag and value, a type
/// written in the place of a tree a `TypeTree`. `MurmurHash3` as Scala's.
fn tree_hash(f: &TastyFile, at: Addr) -> i32 {
    let mut h = TreeHasher { f, depth: 0, news: 0 };
    let mut r = f.trees();
    r.pos = at as usize;
    let v = h.term(&mut r, 4);
    murmur::finalize(v, 0)
}

struct TreeHasher<'f> {
    f: &'f TastyFile,
    depth: u32,
    /// How many `NEW`s were met: the first is the annotation's own, whose class its type names.
    news: u32,
}

const TERM_NAME_HASH: i32 = 1987;
const TYPE_NAME_HASH: i32 = 1993;

impl<'f> TreeHasher<'f> {
    fn node(&self, h: i32, prefix: &str) -> i32 {
        murmur::mix(h, java_hash(prefix))
    }

    fn name(&self, h: i32, n: NameRef, term: bool) -> i32 {
        let h = murmur::mix(h, if term { TERM_NAME_HASH } else { TYPE_NAME_HASH });
        let mut s = String::new();
        write_name(self.f, n, &mut s);
        murmur::mix(h, java_hash(&s))
    }

    /// A term at the reader, hashed onto `h`.
    fn term(&mut self, r: &mut crate::tasty::Reader, h: i32) -> i32 {
        self.depth += 1;
        if self.depth > 200 {
            // Past the depth hashed, the tree is passed over, so that a list of them ends.
            self.depth -= 1;
            r.pos = Decoder::new(self.f).skip_at(r.pos as Addr) as usize;
            return h;
        }
        let start = r.pos;
        let tag = r.byte();
        let out = match tag {
            SHAREDTERM | SHAREDTYPE => {
                let addr = r.nat() as usize;
                let mut s = r.at(addr);
                self.term(&mut s, h)
            }
            APPLY => {
                let end = r.end();
                let h = self.node(h, "Apply");
                let h = self.term(r, h);
                let mut h2 = h;
                while r.pos < end {
                    h2 = self.term(r, h2);
                }
                h2
            }
            TYPEAPPLY => {
                let end = r.end();
                let h = self.node(h, "TypeApply");
                let h = self.term(r, h);
                let mut h2 = h;
                while r.pos < end {
                    h2 = self.type_tree(r, h2);
                }
                h2
            }
            SELECT => {
                let n = r.nat();
                let h = self.node(h, "Select");
                let h = self.term(r, h);
                self.name(h, n, true)
            }
            SELECTIN => {
                let end = r.end();
                let n = r.nat();
                let h = self.node(h, "Select");
                let h = self.term(r, h);
                r.pos = end;
                self.name(h, n, true)
            }
            NEW => {
                let h = self.node(h, "New");
                self.news += 1;
                let tpt = matches!(self.f.trees().at(r.pos).byte(), IDENTTPT | SELECTTPT | APPLIEDTPT);
                if self.news == 1 || tpt {
                    self.type_tree(r, h)
                } else {
                    // An instance an argument makes (`@Ann(new X)`): its class and type arguments,
                    // which a type in the place of the tree would leave out.
                    let t = Decoder::new(self.f).type_at(r.pos as Addr);
                    r.pos = Decoder::new(self.f).skip_at(r.pos as Addr) as usize;
                    let text = crate::tasty::show::Printer::new(self.f, true).ty(&t);
                    murmur::mix(self.node(h, "TypeTree"), java_hash(&text))
                }
            }
            IDENT => {
                let n = r.nat();
                let h = self.node(h, "Ident");
                r.pos = Decoder::new(self.f).skip_at(r.pos as Addr) as usize;
                self.name(h, n, true)
            }
            NAMEDARG => {
                let n = r.nat();
                let h = self.node(h, "NamedArg");
                let h = self.name(h, n, true);
                self.term(r, h)
            }
            TYPED => {
                let end = r.end();
                let h = self.node(h, "Typed");
                let h = self.term(r, h);
                let h = self.type_tree(r, h);
                r.pos = end;
                h
            }
            REPEATED => {
                let end = r.end();
                let h = self.node(h, "SeqLiteral");
                r.pos = Decoder::new(self.f).skip_at(r.pos as Addr) as usize;
                let mut h2 = h;
                while r.pos < end {
                    h2 = self.term(r, h2);
                }
                // The element type, a tree of its own, follows the elements in scalac's order.
                murmur::mix(h2, java_hash("TypeTree"))
            }
            UNITCONST => self.constant(h, 1, 0),
            FALSECONST => self.constant(h, 2, 1237),
            TRUECONST => self.constant(h, 2, 1231),
            NULLCONST => {
                let h = self.node(h, "Literal");
                murmur::mix(h, 11)
            }
            BYTECONST => {
                let v = r.long_int() as i32;
                self.constant(h, 3, v)
            }
            SHORTCONST => {
                let v = r.long_int() as i32;
                self.constant(h, 4, v)
            }
            CHARCONST => {
                let v = r.nat() as i32;
                self.constant(h, 5, v)
            }
            INTCONST => {
                let v = r.long_int() as i32;
                self.constant(h, 6, v)
            }
            LONGCONST => {
                let v = r.long_int();
                self.constant(h, 7, (v ^ (v >> 32)) as i32)
            }
            FLOATCONST => {
                let v = r.long_int() as i32;
                self.constant(h, 8, v)
            }
            DOUBLECONST => {
                let v = r.long_int();
                self.constant(h, 9, (v ^ ((v as u64) >> 32) as i64) as i32)
            }
            STRINGCONST => {
                let n = r.nat();
                let s = self.f.name(n);
                self.constant(h, 10, java_hash(&s))
            }
            // scalac's `constantHash` of a class constant mixes the class's API type in: the
            // class's full name here.
            CLASSCONST => {
                let t = Decoder::new(self.f).type_at(r.pos as Addr);
                r.pos = Decoder::new(self.f).skip_at(r.pos as Addr) as usize;
                let text = crate::tasty::show::Printer::new(self.f, true).ty(&t);
                self.constant(h, 12, java_hash(&text))
            }
            // A path, a local, an executable tree: its text as the reader prints it, which names
            // what it refers to through its qualifiers and holds its values.
            TERMREF | TERMREFSYMBOL | TERMREFDIRECT | TERMREFIN | THIS | QUALTHIS | SUPER | SELECTOUTER | THROW | ASSIGN | BLOCK | IF | LAMBDA | MATCH | RETURN | WHILE | TRY | INLINED => {
                let mut d = Decoder::new(self.f);
                let term = crate::tasty::terms::TermDecoder::new(&mut d).term_at(start as Addr);
                r.pos = Decoder::new(self.f).skip_at(start as Addr) as usize;
                let text = crate::tasty::show::Printer::new(self.f, true).term(&term, 0);
                murmur::mix(self.node(h, "Tree"), java_hash(&text))
            }
            // A package's term, the tree the reader makes of it.
            TERMREFPKG => {
                let n = r.nat();
                let h = self.node(h, "Ident");
                self.name(h, n, true)
            }
            _ => {
                // A type or a tree the hash reads as a type: a `TypeTree`, which has no fields.
                r.pos = Decoder::new(self.f).skip_at(start as Addr) as usize;
                self.node(h, "TypeTree")
            }
        };
        self.depth -= 1;
        out
    }

    fn constant(&self, h: i32, tag: i32, value: i32) -> i32 {
        let h = self.node(h, "Literal");
        let h = murmur::mix(h, tag);
        if tag == 1 {
            // `()`'s hash code, a boxed unit's.
            return murmur::mix(h, 0);
        }
        murmur::mix(h, value)
    }

    /// A type tree at the reader: scalac hashes the tree the unpickler makes of it, a
    /// `TypeTree` for a type written where a tree stands.
    fn type_tree(&mut self, r: &mut crate::tasty::Reader, h: i32) -> i32 {
        let start = r.pos;
        let tag = r.byte();
        match tag {
            IDENTTPT => {
                let n = r.nat();
                let h = self.node(h, "Ident");
                r.pos = Decoder::new(self.f).skip_at(r.pos as Addr) as usize;
                self.name(h, n, false)
            }
            SELECTTPT => {
                let n = r.nat();
                let h = self.node(h, "Select");
                let h = self.term(r, h);
                self.name(h, n, false)
            }
            _ => {
                r.pos = Decoder::new(self.f).skip_at(start as Addr) as usize;
                self.node(h, "TypeTree")
            }
        }
    }
}

/// Java's `String.hashCode`.
fn java_hash(s: &str) -> i32 {
    let mut h: i32 = 0;
    for u in s.encode_utf16() {
        h = h.wrapping_mul(31).wrapping_add(u as i32);
    }
    h
}

/// Scala's `scala.util.hashing.MurmurHash3`, which `ExtractAPI` hashes trees with.
mod murmur {
    pub fn mix(h: i32, data: i32) -> i32 {
        let h = mix_last(h, data);
        let h = (h as u32).rotate_left(13) as i32;
        h.wrapping_mul(5).wrapping_add(0xe6546b64u32 as i32)
    }

    fn mix_last(h: i32, k: i32) -> i32 {
        let k = k.wrapping_mul(0xcc9e2d51u32 as i32);
        let k = (k as u32).rotate_left(15) as i32;
        let k = k.wrapping_mul(0x1b873593);
        h ^ k
    }

    pub fn finalize(h: i32, length: i32) -> i32 {
        avalanche(h ^ length)
    }

    fn avalanche(h: i32) -> i32 {
        let mut h = h as u32;
        h ^= h >> 16;
        h = h.wrapping_mul(0x85ebca6b);
        h ^= h >> 13;
        h = h.wrapping_mul(0xc2b2ae35);
        h ^= h >> 16;
        h as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tree_hash_of_an_annotation_without_arguments_is_scalacs() {
        let mut h = 4;
        for p in ["Apply", "Select", "New", "TypeTree"] {
            h = murmur::mix(h, java_hash(p));
        }
        h = murmur::mix(h, TERM_NAME_HASH);
        h = murmur::mix(h, java_hash("<init>"));
        assert_eq!(murmur::finalize(h, 0), EMPTY_ANNOTATION_HASH);
    }

    #[test]
    fn java_float_formats_as_java_does() {
        assert_eq!(java_float(1.0, false), "1.0");
        assert_eq!(java_float(0.5, false), "0.5");
        assert_eq!(java_float(1e7, false), "1.0E7");
        assert_eq!(java_float(123456.75, false), "123456.75");
        assert_eq!(java_float(0.001, false), "0.001");
        assert_eq!(java_float(0.0001, false), "1.0E-4");
        assert_eq!(java_float(-2.5, false), "-2.5");
        assert_eq!(java_float(0.1, true), "0.1");
    }
}
