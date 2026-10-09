//! Library bodies on the JavaScript target. A class read from a jar that the program reaches
//! gets the AST of its template (`bodies.rs`) in a pseudo file of its own, every member joined
//! to the symbol the loader entered for it, and is then checked as a source class: the vals and
//! statements with the class, the methods when the reach pass asks for them (`std_body`), so
//! that the cost is proportional to what the program reaches.
//!
//! With teq's lean std present, a library's reference to a scala-library definition that the
//! std defines binds to the std's: by qualified name for the names the std defines in the same
//! package, and through `STD_BINDINGS` for the names it defines in another one
//! (`scala.collection.immutable.List` is the std's `scala.List`). Only what the std lacks is
//! compiled from its TASTy body. Under `--std=scala-library` nothing binds: the jar is the std.

use super::super::profile::{About, Kind, Outcome, Phase};
use super::super::resolve::{TermRef, TypeRef};
use super::super::Worker;
use crate::intern::Name;
use crate::source::Span;
use crate::symbols::*;
use super::super::check::DeferredBody;
use crate::tir::{TExprId, TInit};
use crate::types::*;

/// Library definitions the std defines under another package: (the library's package, the
/// name, the std's package).
pub(crate) const STD_BINDINGS: &[(&str, &str, &str)] = &[
    ("scala.collection.immutable", "List", "scala"),
    ("scala.collection.immutable", "::", "scala"),
    ("scala.collection.immutable", "Nil", "scala"),
    ("scala.collection.immutable", "Vector", "scala"),
    ("scala.collection.immutable", "Map", "scala"),
    ("scala.collection.immutable", "Set", "scala"),
    ("scala.collection.immutable", "Seq", "scala"),
    ("scala.collection.immutable", "IndexedSeq", "scala"),
    ("scala.collection.immutable", "Iterable", "scala"),
    ("scala.collection.immutable", "LazyList", "scala"),
    ("scala.collection.immutable", "Range", "scala"),
    ("scala.collection.immutable", "NumericRange", "scala"),
    ("scala.collection.immutable", "ArraySeq", "scala"),
    ("scala.collection.immutable", "SeqOps", "scala"),
    ("scala.collection.immutable", "SetOps", "scala"),
    ("scala.collection", "Seq", "scala"),
    ("scala.collection", "IndexedSeq", "scala"),
    ("scala.collection", "Iterable", "scala"),
    ("scala.collection", "IterableOnce", "scala"),
    ("scala.collection", "IterableOps", "scala"),
    ("scala.collection", "SeqOps", "scala"),
    ("scala.collection", "IndexedSeqOps", "scala"),
    ("scala.collection", "StrictOptimizedIterableOps", "scala"),
    ("scala.collection", "StrictOptimizedSeqOps", "scala"),
    ("scala.collection", "IterableFactoryDefaults", "scala"),
    ("scala.collection", "IterableFactory", "scala"),
    ("scala.collection", "SeqFactory", "scala"),
    ("scala.collection", "StrictOptimizedSeqFactory", "scala"),
    ("scala.collection.immutable", "IndexedSeqOps", "scala"),
    ("scala.collection.immutable", "StrictOptimizedSeqOps", "scala"),
    ("scala.collection", "SortedMap", "scala.collection.immutable"),
    ("scala.collection", "SortedSet", "scala.collection.immutable"),
    ("scala.collection", "SetOps", "scala"),
    ("scala.collection", "Iterator", "scala"),
    ("scala.collection", "View", "scala"),
    ("scala.collection", "Set", "scala"),
    ("scala.collection", "MapView", "scala"),
    ("scala.collection", "Factory", "scala"),
    ("scala.collection", "MapFactory", "scala"),
    ("scala.collection.mutable", "StringBuilder", "scala"),
    ("scala.collection.mutable", "ArraySeq", "scala"),
    ("scala.collection.mutable", "Seq", "scala"),
    ("scala.collection.mutable", "IndexedSeq", "scala"),
    ("scala.collection.mutable", "Iterable", "scala"),
    ("scala.math", "Ordering", "scala"),
    ("scala.math", "Ordered", "scala"),
    ("scala.math", "Numeric", "scala"),
    ("scala.math", "Integral", "scala"),
    ("scala.math", "Fractional", "scala"),
    ("scala.math", "PartialOrdering", "scala"),
    ("scala.util", "Either", "scala"),
    ("scala.util", "Left", "scala"),
    ("scala.util", "Right", "scala"),
    ("scala.runtime", "Tuples", "scala"),
    ("java.lang", "StringBuilder", "scala"),
];

/// What the bodies compiled from a jar cost and where they stopped, for `--time` and the
/// census of the misses.
#[derive(Default)]
pub struct BodyStats {
    pub classes_converted: usize,
    pub members_typed: usize,
    pub time: std::time::Duration,
    /// Classes the converter refused, with the reason.
    pub refused: Vec<(String, String)>,
    /// Members that library bodies asked of the std's classes and did not find, by the
    /// description `member of Class`, with how many bodies asked.
    pub misses: Vec<(String, u32)>,
    /// Methods a library class inherits under one name from two unrelated ancestors, which
    /// the output cannot tell apart.
    pub name_clashes: Vec<String>,
    /// The shape census (`shapes.rs`), taken under `TEQ_CLASSPATH_DETAIL`.
    pub shapes: Vec<super::shapes::ShapeGap>,
}

impl<'a> Worker<'a> {
    /// Whether teq's std is compiled in next to the classpath, which makes it the binding for
    /// what it covers.
    pub(super) fn std_binds(&self) -> bool {
        self.loaded.as_ref().map_or(false, |l| !l.scala_library)
    }

    /// Whether the std is scala-library itself (`--std=scala-library`).
    pub fn scala_library_std(&self) -> bool {
        self.loaded.as_ref().map_or(false, |l| l.scala_library)
    }

    /// Link mode, the one mode of the JVM target, whose std is scala-library: the jars' classes
    /// are never compiled or emitted, the output calls their bytecode and runs beside them. The
    /// name stays for what it describes; it is `--target jvm` itself (`StdMode`).
    pub fn link_mode(&self) -> bool {
        self.jvm
    }

    /// The std definition a library name in package `p` stands for, by `STD_BINDINGS`.
    fn std_binding(&mut self, p: PkgId, name: Name) -> Option<(PkgId, Name)> {
        if !self.std_binds() || p == ROOT_PKG {
            return None;
        }
        let text = self.interner.get(name);
        let matches: Vec<&str> = STD_BINDINGS.iter().filter(|(_, n, _)| *n == text).map(|(from, _, to)| (*from, *to)).filter(|(from, _)| self.pkg_is(p, from)).map(|(_, to)| to).collect();
        let to = *matches.first()?;
        let mut pkg = ROOT_PKG;
        for seg in to.split('.') {
            let n = self.interner.intern(seg);
            pkg = self.syms.pkg(pkg).entries.get(&n).and_then(|e| e.pkg)?;
        }
        Some((pkg, name))
    }

    pub(in crate::typer) fn pkg_is(&self, p: PkgId, path: &str) -> bool {
        let mut segs = path.rsplit('.');
        let mut at = Some(p);
        loop {
            match (segs.next(), at) {
                (None, Some(k)) => return k == ROOT_PKG,
                (None, None) => return true,
                (Some(seg), Some(k)) if k != ROOT_PKG => {
                    if self.interner.get(self.syms.pkg(k).name) != seg {
                        return false;
                    }
                    at = self.syms.pkg(k).parent;
                }
                _ => return false,
            }
        }
    }

    /// A type of package `p` that the std defines elsewhere, when its entries lack `name`. An
    /// alias the jar spilled into the std's package (`scala.PartialOrdering`) is no binding.
    pub(in crate::typer) fn std_bound_type(&mut self, p: PkgId, name: Name) -> Option<TypeRef> {
        let (pkg, name) = self.std_binding(p, name)?;
        self.demand_std(pkg, name, crate::stdindex::TYPE);
        let e = self.syms.pkg(pkg).entries.get(&name)?;
        if let Some(c) = e.class.filter(|&c| !self.in_jar(self.syms.class(c).file)) {
            return Some(TypeRef::Class(c));
        }
        e.alias.filter(|&a| !self.in_jar(self.syms.aliases[a.idx()].file)).map(TypeRef::Alias)
    }

    /// A term of package `p` that the std defines elsewhere, when its entries lack `name`.
    pub(in crate::typer) fn std_bound_term(&mut self, p: PkgId, name: Name) -> Option<TermRef> {
        let (pkg, name) = self.std_binding(p, name)?;
        self.demand_std(pkg, name, crate::stdindex::TERM);
        let e = self.syms.pkg(pkg).entries.get(&name)?;
        if let Some(s) = e.term.filter(|&s| !self.in_jar(self.syms.sym(s).file)) {
            return Some(TermRef::Global(s));
        }
        e.class.filter(|&c| !self.in_jar(self.syms.class(c).file)).map(TermRef::Class)
    }

    /// A member of scala-library's `Predef` that the std defines at the level of package
    /// `scala` (`println`, `assert`, `identity`, ...).
    pub(in crate::typer) fn predef_std_term(&mut self, c: ClassId, name: Name) -> Option<TermRef> {
        if !self.std_binds() || self.loaded.as_ref().unwrap().predef != Some(c) {
            return None;
        }
        let scala = self.b.scala_pkg;
        let s = self.demand_term(scala, name)?;
        let file = self.syms.sym(s).file;
        (!self.in_jar(file)).then_some(TermRef::Global(s))
    }

    pub fn is_library_class(&self, c: ClassId) -> bool {
        self.loaded.as_ref().map_or(false, |l| l.classes.contains_key(&c))
    }

    /// Whether `c` was read from a directory of teq's products: another module's program.
    pub fn is_product_class(&self, c: ClassId) -> bool {
        self.loaded.as_ref().map_or(false, |l| l.is_product_class(c))
    }

    /// Whether `c` was read from a Scala 2 library (`Loaded::is_scala2_class`).
    pub fn is_scala2_class(&self, c: ClassId) -> bool {
        self.loaded.as_ref().map_or(false, |l| l.is_scala2_class(c))
    }

    /// Whether `c` is a class of the lean std standing for one of scala-library's compiled from
    /// Scala 2's sources, which dotty flags `Scala2x` (`TreeUnpickler.readNewDef`, for a TASTy
    /// file with the `SCALA2STANDARDLIBRARY` attribute), or for one of Scala.js's 2.13 library:
    /// any class of a `scala` package but those of the Scala 3 library (`SCALA3_LIBRARY`).
    pub fn is_std_scala2_class(&self, c: ClassId) -> bool {
        let info = self.syms.class(c);
        if !self.source(info.file).is_std || self.in_jar(info.file) {
            return false;
        }
        let mut owner = info.owner;
        let mut top = c;
        while let Owner::Class(o) = owner {
            top = o;
            owner = self.syms.class(o).owner;
        }
        let Owner::Package(p) = owner else { return false };
        let mut path = Vec::new();
        let mut at = Some(p);
        while let Some(k) = at.filter(|&k| k != ROOT_PKG) {
            path.push(self.interner.get(self.syms.pkg(k).name));
            at = self.syms.pkg(k).parent;
        }
        path.reverse();
        if path.first() != Some(&"scala") {
            return false;
        }
        let pkg = path.join(".");
        let name = self.interner.get(self.syms.class(top).name);
        let name = name.strip_suffix('$').unwrap_or(name);
        !SCALA3_LIBRARY.iter().any(|&(k, n)| (n == "*" && (pkg == k || pkg.strip_prefix(k).map_or(false, |r| r.starts_with('.')))) || (pkg == k && n == name))
    }

    pub fn is_library_member(&self, s: SymId) -> bool {
        self.loaded.as_ref().map_or(false, |l| l.syms.contains_key(&s))
    }

    /// The class of a jar that holds `c`: `c` itself, or the outermost entered class it is
    /// nested in, whose conversion covers it.
    pub(super) fn conversion_root(&self, c: ClassId) -> ClassId {
        let mut root = c;
        let mut owner = self.syms.class(c).owner;
        while let Owner::Class(o) = owner {
            if !self.is_library_class(o) {
                break;
            }
            root = o;
            owner = self.syms.class(o).owner;
        }
        root
    }

    /// Gives the library class `c` its AST, converting the class it stands in if that has not
    /// happened yet; `false` when the converter refused it, which was reported.
    pub fn convert_library_class(&mut self, c: ClassId) -> bool {
        self.predecode_conversion(c);
        self.with_loader_for(crate::measure::Hold::LibraryConversion, |w| w.convert_library_class_unlocked(c))
    }

    /// Decodes, before the loader's lock is taken for the conversion of the library class `c`, the
    /// templates its root's conversion starts from: the root's and its companion's in the same file.
    /// The decode is a pure function of the file's TASTy, which the
    /// loaded files' table hands out without the lock, and the class records it reads are their
    /// entries'; the conversion enters what it decoded under the lock once its root is found still
    /// unconverted there, and drops it otherwise (`convert_library_class_now`). Only a forked
    /// build's worker outside the lock decodes ahead; `TEQ_PREDECODE=off`, a diagnostic, none.
    fn predecode_conversion(&mut self, c: ClassId) {
        static OFF: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        if !self.forked || crate::shared::lock_depth() > 0 || *OFF.get_or_init(|| std::env::var_os("TEQ_PREDECODE").is_some_and(|v| v == "off")) {
            return;
        }
        let Some(loaded) = self.loaded.as_ref() else { return };
        let Some(lc) = loaded.classes.get(&c).copied() else { return };
        if lc.value_case || self.syms.class(c).def.is_some() || self.syms.class(c).js == JsKind::Native {
            return;
        }
        let root = self.conversion_root(c);
        let (def, companion) = (self.syms.class(root).def, self.syms.class(root).companion);
        let Some(lr) = loaded.classes.get(&root).copied() else { return };
        if def.is_some() || lr.file as usize >= loaded.files.len() {
            return;
        }
        let tasty = loaded.file(lr.file).tasty.clone();
        let mut addrs = vec![lr.addr];
        addrs.extend(companion.and_then(|k| loaded.classes.get(&k).copied()).filter(|lc| lc.file == lr.file).map(|lc| lc.addr));
        let p = self.phase(Phase::TermDecode);
        let decoded = addrs.into_iter().map(|addr| ((lr.file, addr), std::sync::Arc::new(crate::tasty::terms::TermDecoder::new(&mut crate::tasty::tree::Decoder::new(&tasty)).class_def(addr)))).collect();
        self.predecoded = Some((root, decoded));
        self.phase_end(p);
    }

    pub fn convert_library_class_unlocked(&mut self, c: ClassId) -> bool {
        let predecoded = std::mem::take(&mut self.predecoded);
        if self.syms.class(c).def.is_some() {
            return true;
        }
        if super::super::prep::capturing() {
            self.prep_note_class('V', c);
        }
        self.completing += 1;
        let converted = self.convert_library_class_now(c, predecoded);
        self.leave_completion();
        converted
    }

    fn convert_library_class_now(&mut self, c: ClassId, predecoded: Option<(ClassId, Vec<((u32, crate::tasty::tree::Addr), std::sync::Arc<crate::tasty::terms::ClassDef>)>)>) -> bool {
        let info = self.syms.class(c);
        if info.def.is_some() {
            return true;
        }
        if !self.is_library_class(c) || matches!(info.kind, ClassKind::Opaque | ClassKind::Builtin) {
            return false;
        }
        // An enum value's definition comes with its enum's conversion.
        if self.loaded.as_ref().unwrap().classes[&c].value_case {
            let Owner::Class(companion) = info.owner else { return false };
            let Some(e) = self.syms.class(companion).companion else { return false };
            return self.convert_library_class(e) && self.syms.class(c).def.is_some();
        }
        let root = self.conversion_root(c);
        if self.syms.class(root).def.is_some() {
            return self.syms.class(c).def.is_some();
        }
        let start = std::time::Instant::now();
        let path = self.class_path(root);
        if self.loaded.as_ref().unwrap().bodies.refused.iter().any(|(p, _)| *p == path) {
            return false;
        }
        for (at, decoded) in predecoded.filter(|&(r, _)| r == root).map_or_else(Vec::new, |(_, d)| d) {
            self.loaded_mut().decoded_classes.entry(at).or_insert(decoded);
        }
        let p = self.phase(Phase::Convert);
        let converted = self.convert_class(root);
        self.phase_end(p);
        let converted = match converted {
            Ok(cc) => cc,
            Err(why) => {
                let jar = self.source(self.syms.class(root).file).path.clone();
                self.diags.error(self.syms.class(root).file, Span::default(), format!("not supported yet: {} (in the body of {} of {})", why, path, jar));
                self.loaded_mut().bodies.refused.push((path, why));
                return false;
            }
        };
        let pkg = self.package_of_class(root);
        let jar = self.source(self.syms.class(root).file).path.clone();
        let jar_name = jar.rsplit(std::path::is_separator).next().unwrap_or(&jar).to_string();
        let super::LClass { file: tasty_file, addr, .. } = self.loaded.as_ref().unwrap().classes[&root];
        let key = self.body_key(tasty_file, addr);
        let origin = self.syms.class(root).file;
        let file = self.new_body_file(origin, format!("{}!{}", jar_name, path), key, converted.ast, converted.text, pkg);
        if self.is_product_class(root) {
            self.loaded_mut().place_product_defs(file, tasty_file, &converted.def_at);
        }
        self.note_tasty_opaques(file, tasty_file);
        for o in converted.holder_objects {
            let imp = super::super::ResolvedImport { name: None, target: super::super::ImportTarget::ClassAll(o), hidden: crate::ast::ListRef::EMPTY, depth: 0, unimports_predef: None, sel: super::super::unused::SelRef::NONE };
            std::sync::Arc::make_mut(self.file_imports[file.0 as usize].get_or_insert_with(Default::default)).push(imp);
        }
        for &(d, s) in &converted.def_syms {
            self.def_syms.insert(file.0 as usize, d, s);
            let span = self.ast(file).def(d).span;
            let mut info = self.syms.sym_mut(s);
            info.def = Some(d);
            info.file = file;
            info.span = span;
        }
        for &(s, e) in &converted.retained_bodies {
            self.retained_bodies.insert(s, e);
        }
        for &(d, k) in &converted.def_classes {
            self.def_classes.insert(file.0 as usize, d, k);
            let span = self.ast(file).def(d).span;
            let mut info = self.syms.class_mut(k);
            info.def = Some(d);
            info.file = file;
            info.span = span;
            self.note_class_body(k, d, file);
            // An enum's companion stands where its enum does, as the whole program's does
            // whether the source writes it or not.
            let enum_of = self.syms.class(k).companion.filter(|&e| self.syms.class(k).kind == ClassKind::Object && self.syms.class(e).kind == ClassKind::Enum);
            if let Some(lc) = enum_of.filter(|_| self.is_product_class(k)).and_then(|e| self.loaded.as_ref().unwrap().classes.get(&e).copied()) {
                if lc.file == tasty_file {
                    self.loaded_mut().place_def_at(file, d, tasty_file, lc.addr);
                }
            }
        }
        let stats = &mut self.loaded_mut().bodies;
        stats.classes_converted += converted.def_classes.len();
        stats.time += start.elapsed();
        self.syms.class(c).def.is_some()
    }

    /// A product's `<file>$package` object converted as the top-level definitions of its
    /// package: its members, in a pseudo file of their own whose tag is the
    /// source's recorded token, so that its defs are top-level functions and its vals a file
    /// group with the source's `$file` initialiser, as the whole program has them. Once per
    /// object; a definition is checked when the build needs it (`check_product_package_def`).
    pub fn check_product_package(&mut self, obj: ClassId) {
        self.with_loader_for(crate::measure::Hold::LibraryClassCheck, |w| w.check_product_package_unlocked(obj))
    }

    fn check_product_package_unlocked(&mut self, obj: ClassId) {
        if self.loaded.as_ref().unwrap().product_packages.get(&obj).copied() != Some(false) {
            return;
        }
        self.loaded_mut().product_packages.insert(obj, true);
        let start = std::time::Instant::now();
        let converted = match self.convert_class(obj) {
            Ok(cc) => cc,
            Err(why) => {
                let path = self.class_path(obj);
                self.diags.error(self.syms.class(obj).file, Span::default(), format!("not supported yet: {} (in the top-level definitions of {})", why, path));
                return;
            }
        };
        let crate::ast::DefKind::Class(cls) = &converted.ast.def(converted.def).kind else { return };
        let top: Vec<crate::ast::DefId> = cls.body.iter().filter_map(|s| match s {
            crate::ast::Stmt::Def(d) => Some(*d),
            _ => None,
        }).collect();
        let mut ast = converted.ast;
        ast.top_level = top.clone();
        let Owner::Package(pkg) = self.syms.class(obj).owner else { return };
        let super::LClass { file: tasty_file, addr, .. } = self.loaded.as_ref().unwrap().classes[&obj];
        let token = match &self.loaded.as_ref().unwrap().file(tasty_file).provenance {
            crate::tasty::origins::Found::Read(o) => Some(o.token),
            _ => None,
        };
        let jar = self.source(self.syms.class(obj).file).path.clone();
        let jar_name = jar.rsplit(std::path::is_separator).next().unwrap_or(&jar).to_string();
        let key = self.body_key(tasty_file, addr);
        let origin = self.syms.class(obj).file;
        let path = self.class_path(obj);
        let file = self.new_body_file_tagged(origin, format!("{}!{}", jar_name, path), key, ast, converted.text, pkg, token);
        self.loaded_mut().place_product_defs(file, tasty_file, &converted.def_at);
        self.note_tasty_opaques(file, tasty_file);
        for &(d, s) in &converted.def_syms {
            self.def_syms.insert(file.0 as usize, d, s);
            let span = self.ast(file).def(d).span;
            let mut info = self.syms.sym_mut(s);
            info.def = Some(d);
            info.file = file;
            info.span = span;
        }
        for &(d, k) in &converted.def_classes {
            if k == obj {
                continue;
            }
            self.def_classes.insert(file.0 as usize, d, k);
            let span = self.ast(file).def(d).span;
            let mut info = self.syms.class_mut(k);
            info.def = Some(d);
            info.file = file;
            info.span = span;
            self.note_class_body(k, d, file);
        }
        let _ = top;
        let stats = &mut self.loaded_mut().bodies;
        stats.classes_converted += 1;
        stats.time += start.elapsed();
    }

    /// Checks a top-level definition of a product as a source file's (`check_top_def`), its
    /// file converted first, with every eager val of its file, which its file's initialiser
    /// runs together and an access of the definition runs first (`layout::initialises_file`).
    fn check_product_package_def(&mut self, sym: SymId, obj: ClassId) {
        self.check_product_package(obj);
        let info = self.syms.sym(sym);
        let (file, Owner::Package(pkg)) = (info.file, info.owner) else { return };
        let Some(d) = info.def else { return };
        // The vals the file's initialiser runs together, lazy ones apart (`is_eager_top_val`).
        let eager = |t: &Worker, d: crate::ast::DefId| {
            let def = t.ast(file).def(d);
            matches!(def.kind, crate::ast::DefKind::Val { .. }) && def.mods & (crate::ast::mods::LAZY | crate::ast::mods::GIVEN) == 0
        };
        let mut defs: Vec<crate::ast::DefId> = self.ast(file).top_level.iter().copied().filter(|&t| eager(self, t)).collect();
        if !defs.contains(&d) {
            defs.push(d);
        }
        for d in defs {
            if self.loaded_mut().product_checked.insert((file, d), ()).is_none() {
                // A method is typed as a std method the walk reaches is, registered at the top
                // level; the rest as a source file's top-level definitions.
                let method = matches!(self.ast(file).def(d).kind, crate::ast::DefKind::Fun(_));
                match self.def_syms.get(file.0 as usize, &d).copied().filter(|_| method) {
                    Some(s) => {
                        self.outside_inline(|t| t.std_body(s));
                    }
                    None => self.outside_inline(|t| t.check_top_def(file, Owner::Package(pkg), d)),
                }
            }
        }
    }

    /// The names the bundle would declare twice because the products' recorded tokens and
    /// offsets give two definitions one: two named local classes of one name,
    /// token and offset, two anonymous classes of one name, two file initialisers of one token,
    /// at least one of each pair a product's. Each pair is named with the definitions and their
    /// artifacts, in the order of the names, so that the build fails the same way every time.
    pub fn product_name_collisions(&self, reached: &[bool]) -> Vec<String> {
        let Some(loaded) = self.loaded.as_ref() else { return Vec::new() };
        if loaded.product_files.is_empty() {
            return Vec::new();
        }
        let artifact = |f: crate::source::FileId| -> Option<String> {
            let n = self.files.len();
            let i = (f.0 as usize).checked_sub(n)?;
            let jar = *loaded.body_jars.get(i);
            loaded.product_files.contains_key(&f).then(|| self.source(jar).path.clone())
        };
        let where_of = |c: ClassId| -> String {
            let info = self.syms.class(c);
            match (artifact(info.file), self.syms.product_file_key(info.file)) {
                (Some(a), Some(key)) => format!("of {} in the products of {}", key, a),
                (Some(a), None) => format!("in the products of {}", a),
                _ => format!("of {}", self.source(info.file).path),
            }
        };
        let mut named: crate::intern::FxMap<String, Vec<ClassId>> = Default::default();
        for (i, info) in self.syms.classes.iter().enumerate() {
            let c = ClassId(i as u32);
            if !reached.get(i).copied().unwrap_or(false) || info.owner != Owner::Local {
                continue;
            }
            let name = match info.kind {
                ClassKind::Anon => self.name_str(info.name),
                _ => match self.syms.product_position(info.file, info.def) {
                    Some((token, offset)) => format!("{}${}_{}", self.name_str(info.name), crate::source::tag_text(token), offset),
                    None => format!("{}${}", self.name_str(info.name), self.prog.position(info.file, info.span.start)),
                },
            };
            named.entry(name).or_default().push(c);
        }
        let mut out = Vec::new();
        let mut names: Vec<(&String, &Vec<ClassId>)> = named.iter().filter(|(_, cs)| cs.len() > 1 && cs.iter().any(|&c| artifact(self.syms.class(c).file).is_some())).collect();
        names.sort_by(|a, b| a.0.cmp(b.0));
        for (name, cs) in names {
            let mut places: Vec<String> = cs.iter().map(|&c| format!("{} {}", self.class_path(c), where_of(c))).collect();
            places.sort();
            out.push(format!("the bundle would declare {} twice: {}; their sources' keys give one token, which the products record as their producers assigned it", name, places.join(" and ")));
        }
        out
    }

    /// The opaque types of the TASTy file a pseudo file's bodies come from are the pseudo
    /// file's, as a source file's are that file's: each is transparent where its scope
    /// encloses the body, its object or class, or for a top-level one the package object and
    /// its companion, which scalac pickles into the package object's file.
    pub(super) fn note_tasty_opaques(&mut self, file: crate::source::FileId, tasty_file: u32) {
        let mut opaques: Vec<ClassId> =
            self.file_tables(tasty_file).classes.values().copied().filter(|&k| self.syms.class(k).kind == ClassKind::Opaque).collect();
        opaques.sort();
        self.file_opaques[file.0 as usize].extend(opaques);
    }

    /// What the namer notes about a body when it enters one: whether it runs statements. The
    /// vals of a jar's class are lazy fields on JavaScript (`check_member`), so only its vars
    /// and its expression statements run when a class that mixes it in is constructed there; a
    /// product's vals are fields, as the whole program's.
    fn note_class_body(&mut self, c: ClassId, d: crate::ast::DefId, file: crate::source::FileId) {
        use crate::ast::{mods, DefKind, Stmt};
        let eager = self.jvm || self.is_product_class(c);
        let ast = self.ast(file);
        let DefKind::Class(cls) = &ast.def(d).kind else { return };
        let in_trait = self.syms.class(c).kind == ClassKind::Trait;
        let mut has_statements = false;
        for stmt in &cls.body {
            match stmt {
                Stmt::Def(m) => {
                    let def = ast.def(*m);
                    let module_val = self.nested_object_of(c, def.name).is_some();
                    let runs = def.mods & mods::MUTABLE != 0 || (eager && def.mods & mods::LAZY == 0 && !module_val);
                    if in_trait && runs && matches!(def.kind, DefKind::Val { rhs: Some(_), .. }) {
                        has_statements = true;
                    }
                }
                Stmt::Expr(_) => has_statements = true,
                Stmt::Import(_) => {}
            }
        }
        self.syms.class_mut(c).has_statements = has_statements;
    }

    pub(in crate::typer) fn package_of_class(&self, c: ClassId) -> PkgId {
        let mut owner = self.syms.class(c).owner;
        loop {
            match owner {
                Owner::Package(p) => return p,
                Owner::Class(k) => owner = self.syms.class(k).owner,
                Owner::Local => return ROOT_PKG,
            }
        }
    }

    /// The reach pass met the library class `c`: it is converted and checked, so that its
    /// constructor, fields and statements are in the program and its methods can be asked for.
    pub fn check_library_class(&mut self, c: ClassId) {
        self.predecode_conversion(c);
        self.with_loader_for(crate::measure::Hold::LibraryClassCheck, |w| w.check_library_class_unlocked(c))
    }

    pub fn check_library_class_unlocked(&mut self, c: ClassId) {
        // JavaScript defines a native type, and scalac checked its declarations.
        if self.syms.class(c).js == JsKind::Native {
            return;
        }
        if super::super::prep::capturing() && !self.class_done.contains_key(&c) {
            self.prep_note_class('L', c);
        }
        let start = std::time::Instant::now();
        if std::env::var_os("TEQ_CLASSPATH_DETAIL").is_some() {
            eprintln!("check library class {}", self.class_path(c));
        }
        let phase = self.phase(Phase::Deferred);
        let prof = self.prof(Kind::LibraryBody, Span::default(), About::Class(c));
        if self.convert_library_class(c) {
            self.outside_inline(|t| t.check_class(c));
            self.name_library_members(c);
            self.bridge_library_class(c);
        }
        if let Some(p) = prof {
            self.profile.exit(p, Outcome::Found);
        }
        self.phase_end(phase);
        self.loaded_mut().bodies.time += start.elapsed();
    }

    /// Types what is a definition's own with the expansions under way set aside: the body of a
    /// member or a class checked because an expansion asked for it (a result type inferred, a
    /// library body a macro's run reached) is typed as its class's walk types it. `return` is
    /// allowed in it, `this` and the class's type parameters are its own, and a search runs
    /// where it stands.
    pub(in crate::typer) fn outside_inline<T>(&mut self, f: impl FnOnce(&mut Self) -> T) -> T {
        let outer = self.inline.set_aside();
        // Read as differences around a test or a condition of the expansion under way, which
        // what the body reads must not move.
        let reads = (self.inline.tparam_reads.get(), self.inline.leaf_reads);
        let result = f(self);
        if let Some(outer) = outer {
            self.inline.restore(outer);
        }
        self.inline.tparam_reads.set(reads.0);
        self.inline.leaf_reads = reads.1;
        result
    }

    /// The names the methods of `c` go by in the output, which `name_all_alternatives` settled
    /// for the program's classes before the jars' classes were compiled: the alternatives of an
    /// overloaded name and what meets an inherited method.
    pub(in crate::typer) fn name_library_members(&mut self, c: ClassId) {
        if self.syms.class(c).named_for_output {
            return;
        }
        self.with_loader_for(crate::measure::Hold::LibraryClassCheck, |w| w.name_library_members_unlocked(c))
    }

    fn name_library_members_unlocked(&mut self, c: ClassId) {
        if self.syms.class(c).named_for_output {
            return;
        }
        self.syms.class_mut(c).named_for_output = true;
        self.naming_library += 1;
        // What no lookup has settled is settled first, the most distant ancestor's members
        // before those of the classes below, since the name of a method is that of the
        // declaration it overrides.
        let classes: Vec<ClassId> = self.syms.class(c).base_types.iter().rev().map(|&(b, _)| b).collect();
        for k in classes {
            let entries: Vec<SymId> = self.syms.class(k).members.values().copied().collect();
            for entry in entries {
                if self.syms.sym(entry).merge_pending {
                    self.merge_inherited(k, entry);
                }
            }
        }
        let members: Vec<SymId> = self.syms.class(c).member_order.clone();
        let sets: Vec<SymId> = self.syms.overloads.entries().map(|(_, (set, _))| *set).filter(|&set| self.syms.sym(set).owner == Owner::Class(c) && !self.syms.sym(set).superseded).collect();
        for set in sets {
            self.name_alternatives(set);
        }
        for m in members {
            if self.syms.sym(m).kind == SymKind::Def {
                self.dispatch_name(m);
            }
        }
        self.naming_library -= 1;
        self.settle_pending_names();
    }

    /// A method of an overloaded name that a value of the class implements (tapir's
    /// `Endpoint.info` for `EndpointInfoOps.info`) is answered through a bridge, settled when
    /// the class is checked so that the reach pass sees it before it walks the class.
    fn bridge_library_class(&mut self, c: ClassId) {
        if self.loaded_mut().bridged.insert(c, ()).is_none() {
            self.bridge_class(c);
        }
    }

    /// The local and anonymous classes a library body made since `first` are named as the
    /// jar's classes are, so that a method overloading an inherited one goes by a name of its
    /// own (`get(implicit u: Unsafe)` beside `AtomicReference.get()`).
    fn name_local_library_classes(&mut self, first: usize) {
        for i in first..self.syms.classes.len() {
            let k = ClassId(i as u32);
            let info = self.syms.class(k);
            if info.owner == Owner::Local && self.in_jar(info.file) {
                self.name_library_members(k);
            }
        }
    }

    /// A method of a jar that overrides a method of the std whose `ClassTag` evidence the std
    /// erases on this target (`toArray[B: ClassTag]` of zio's `Chunk` over the std's
    /// `IterableOps.toArray`): the std's own calls pass no evidence, so the jar's evidence gets
    /// the tag of `AnyRef` as its default, the array a call of the std's makes without one;
    /// a call that passes the tag keeps it.
    fn default_erased_evidence(&mut self, sym: SymId, f: crate::typer::FunId) {
        let Some(tag_class) = self.sites.class_tag else { return };
        let mut roots = Vec::new();
        self.root_declarations(sym, 0, &mut roots);
        let erased = roots.iter().any(|&r| r != sym && self.syms.sym(r).erased_tags != 0 && !self.is_library_member(r));
        if !erased {
            return;
        }
        let clauses = self.sig_of(sym).clauses.clone();
        let mut index = 0;
        let mut evidence = Vec::new();
        for clause in &clauses {
            for p in &clause.params {
                let ty = self.dealias(p.ty);
                let tagged = (clause.is_using || clause.is_implicit) && matches!(self.types.get(ty), Type::Class(k, _) if k == tag_class);
                if tagged && !p.has_default {
                    evidence.push(index);
                }
                index += 1;
            }
        }
        for i in evidence {
            let Some(tag) = self.any_ref_class_tag(tag_class) else { return };
            let fun = &mut self.prog.funs[f.idx()];
            if i < fun.defaults.len() && fun.defaults[i].is_none() {
                fun.defaults[i] = Some(tag);
            }
        }
    }

    /// The body of the library method or val `sym` the reach pass asked for: its class
    /// converted and checked first, then the method typed as a std method is, or the val's
    /// initialiser typed and joined to the class as a lazy field.
    pub fn library_body(&mut self, sym: SymId) -> Option<DeferredBody> {
        if let Owner::Class(c) = self.syms.sym(sym).owner {
            if !self.body_is_native(sym) {
                self.predecode_conversion(c);
            }
        }
        self.with_loader_for(crate::measure::Hold::Body, |w| w.library_body_unlocked(sym))
    }

    pub fn library_body_unlocked(&mut self, sym: SymId) -> Option<DeferredBody> {
        let start = std::time::Instant::now();
        if let Some(&obj) = self.loaded.as_ref().unwrap().product_package_members.get(&sym) {
            self.check_product_package_def(sym, obj);
            return match self.val_init.get(&sym) {
                Some(&init) => Some(DeferredBody::Val(init)),
                None => self.fun_of_sym.get(&sym).copied().map(DeferredBody::Fun),
            };
        }
        let Owner::Class(c) = self.syms.sym(sym).owner else { return None };
        if self.body_is_native(sym) {
            return None;
        }
        if !self.convert_library_class(c) {
            return None;
        }
        self.outside_inline(|t| t.check_class(c));
        self.name_library_members(c);
        self.bridge_library_class(c);
        if std::env::var_os("TEQ_CLASSPATH_DETAIL").is_some() {
            eprintln!("type body {}", self.sym_path(sym));
        }
        let prof = self.prof(Kind::LibraryBody, Span::default(), About::Sym(sym));
        let first_class = self.syms.classes.len();
        let body = match self.syms.sym(sym).kind {
            SymKind::Val => self.outside_inline(|t| t.library_val(sym, c)).map(DeferredBody::Val),
            _ => self.outside_inline(|t| t.std_body(sym)).map(DeferredBody::Fun),
        };
        if let Some(DeferredBody::Fun(f)) = &body {
            self.default_erased_evidence(sym, *f);
        }
        self.name_local_library_classes(first_class);
        if let Some(p) = prof {
            self.profile.exit(p, if body.is_some() { Outcome::Found } else { Outcome::NotFound });
        }
        let stats = &mut self.loaded_mut().bodies;
        stats.members_typed += body.is_some() as usize;
        stats.time += start.elapsed();
        body
    }

    fn library_val(&mut self, sym: SymId, c: ClassId) -> Option<TExprId> {
        if let Some(&init) = self.val_init.get(&sym) {
            return Some(init);
        }
        self.ensure_body(sym);
        let init = *self.val_init.get(&sym)?;
        let i = self.prog_index.class(&self.prog, c)?;
        // The field joins the class where its definition stands in the template (the loaded
        // class's members, entered in its pickle's order), not after the fields the program read
        // first, so that the class is written alike whatever reads it: scalac's `Memoize` makes
        // a field and its accessor where the definition stands.
        let rank = |order: &[SymId], s: SymId| order.iter().position(|&m| m == s).unwrap_or(usize::MAX);
        let order = self.syms.class(c).member_order.clone();
        let own = rank(&order, sym);
        let fields = &mut self.prog.classes[i].init;
        let at = fields.iter().position(|x| matches!(*x, TInit::Field(s, _) if rank(&order, s) > own)).unwrap_or(fields.len());
        fields.insert(at, TInit::Field(sym, init));
        Some(init)
    }

    /// `classOf[T]`: the class value of a class type, a builtin or an array included.
    pub fn type_class_of(&mut self, ty: TypeId, span: Span) -> (crate::tir::TExprId, TypeId) {
        let ty = self.dealias(ty);
        let c = match self.types.get(ty) {
            Type::Any => Some(self.b.any_ref),
            _ => self.class_of(ty),
        };
        let c = c.map(|k| self.runtime_tuple_class(k));
        let (Some(c), Some(class)) = (c, self.java_lang_class("Class")) else {
            let shown = self.show(ty);
            self.error(span, format!("classOf[{}] needs a class type", shown));
            return (self.prog.add(crate::tir::TExpr::Unit), ERROR);
        };
        let result = self.types.class(class, &[ty]);
        (self.prog.add(crate::tir::TExpr::ClassOf(c)), result)
    }

    /// Whether `f` is the pseudo file of a converted library class or body.
    pub fn is_body_file(&self, f: crate::source::FileId) -> bool {
        f.0 as usize >= self.files.len()
    }
}

/// The assertion-enabled builds' count of the converted library bodies whose declared types a
/// worker read that another worker converted, which `TEQ_BODY_CROSSINGS=1` prints at the merge:
/// the case where a worker once read the converter's flag in the place of the body's own
/// inferred types.
#[cfg(debug_assertions)]
pub(in crate::typer) mod body_crossings {
    use crate::intern::FxMap;
    use crate::source::FileId;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;

    static CONVERTERS: Mutex<Option<FxMap<u32, usize>>> = Mutex::new(None);
    static READ: AtomicUsize = AtomicUsize::new(0);
    static CROSSED: AtomicUsize = AtomicUsize::new(0);

    pub fn converted(file: FileId, worker: usize) {
        CONVERTERS.lock().unwrap_or_else(|e| e.into_inner()).get_or_insert_with(FxMap::default).insert(file.0, worker);
    }

    pub fn read(file: FileId, worker: usize) {
        let converter = CONVERTERS.lock().unwrap_or_else(|e| e.into_inner()).as_ref().and_then(|m| m.get(&file.0).copied());
        READ.fetch_add(1, Ordering::Relaxed);
        if converter.is_some_and(|w| w != worker) {
            CROSSED.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn report() {
        if std::env::var_os("TEQ_BODY_CROSSINGS").is_some() {
            eprintln!("body crossings: {} of {} declared types of converted bodies read by a worker other than their converter", CROSSED.load(Ordering::Relaxed), READ.load(Ordering::Relaxed));
        }
    }
}

/// The classes of scala-library 3.8.4 whose TASTy lacks the `SCALA2STANDARDLIBRARY` attribute,
/// the Scala 3 library's own, by package (`*` for every class of the package and those below);
/// annotations, which have no methods to call, are left out.
const SCALA3_LIBRARY: &[(&str, &str)] = &[
    ("scala", "$times$colon"), ("scala", "*:"), ("scala", "CanEqual"), ("scala", "CanThrow"), ("scala", "Conversion"),
    ("scala", "IArray$package"), ("scala", "IArray"), ("scala", "NamedTuple"), ("scala", "NamedTupleDecomposition"),
    ("scala", "NonEmptyTuple"), ("scala", "PolyFunction"), ("scala", "Precise"), ("scala", "Selectable"), ("scala", "Tuple"),
    ("scala", "Tuple$package"), ("scala", "main"), ("scala", "unsafeExceptions"),
    ("scala.annotation.internal", "*"), ("scala.caps", "*"), ("scala.compiletime", "*"), ("scala.deriving", "*"), ("scala.quoted", "*"),
    ("scala.reflect", "Enum"), ("scala.reflect", "Selectable"), ("scala.reflect", "TypeTest"), ("scala.reflect", "Typeable$package"),
    ("scala.runtime", "$throws$package"), ("scala.runtime", "Arrays"), ("scala.runtime", "EnumValue"), ("scala.runtime", "FunctionXXL"),
    ("scala.runtime", "LazyVals"), ("scala.runtime", "MatchCase"), ("scala.runtime", "Scala3RunTime"), ("scala.runtime", "TupleMirror"),
    ("scala.runtime", "TupleXXL"), ("scala.runtime", "TupledFunctions"), ("scala.runtime", "Tuples"), ("scala.runtime", "TypeBox"),
    ("scala.runtime", "VarArgsBuilder"), ("scala.runtime.coverage", "*"), ("scala.runtime.stdLibPatches", "*"),
    ("scala.util", "CommandLineParser"), ("scala.util", "FromDigits"), ("scala.util", "LowPriorityNotGiven"), ("scala.util", "NotGiven"),
    ("scala.util", "TupledFunction"), ("scala.util", "boundary"), ("scala.util.control", "NonLocalReturns"),
];
