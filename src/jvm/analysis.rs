//! The analysis of a JVM build for a build tool (`teq compiler watch --target jvm`, sbt-teq): per source
//! file, the classes it defines with what zinc's discovery reads of a class, the class files
//! each stands behind, and the class files of its local classes. A check session under `--own`
//! (sbt-teq's `compile` of a Scala.js project) answers the same of the typed program, without
//! class files: `files` and `local` are empty.
//!
//! One JSON object per file: `{"file":"src/A.scala","classes":[{"name":"p.A","kind":"class",
//! "top":true,"public":true,"abstract":false,"final":false,"bases":["p.Base","zio.test.
//! ZIOSpecDefault",...],"annotations":["p.Marker"],"methodAnnotations":["org.junit.Test"],
//! "main":false,"files":["p/A.class"]}],"local":["p/A$$anon$1.class"]}`. `kind` is `class`,
//! `trait` or `object` (a companion and its class are two entries of one name); `bases` is the
//! whole linearisation by qualified name, so that a build tool reading a class's parents finds a
//! framework's suite through a base class of the program; `annotations` names the class's
//! annotations as they resolve, `methodAnnotations` those of its public methods, declared and
//! inherited (a JUnit suite is found by its `@Test` methods); `main` marks the class the `java`
//! launcher runs (an object with `main`, or the class of a `@main` method).

use super::Output;
use crate::ast::mods;
use crate::intern::FxMap;
use crate::source::FileId;
use crate::symbols::*;
use crate::types::{ClassId, PkgId, SymId};

use crate::typer::{TypeRef, Worker};
use crate::watch::json_string;

/// The analysis of each of `files`, in their order, as JSON objects; `unit_file` gives the
/// file a class is reported under from the unit that defines it (a `package p:` block's file).
/// A file's package blocks are files of their own, all answered under the file's path: one entry
/// per file, as `render_typed` answers them, its main classes those recorded under the file or
/// its blocks (a package object's `@main` is recorded under the block).
pub fn render(t: &mut Worker, out: &Output, files: &[FileId], unit_file: &[FileId]) -> Vec<String> {
    let unit = |f: FileId| unit_file.get(f.0 as usize).copied().unwrap_or(f);
    let mut by_file: FxMap<FileId, Vec<usize>> = FxMap::default();
    for (i, c) in out.classes.iter().enumerate() {
        if let Some(f) = c.source {
            by_file.entry(unit(f)).or_default().push(i);
        }
    }
    let mut units: Vec<FileId> = Vec::with_capacity(files.len());
    for &f in files {
        let u = unit(f);
        if !units.contains(&u) {
            units.push(u);
        }
    }
    units
        .iter()
        .map(|&f| {
            let mains: Vec<&str> = out.main_classes.iter().filter(|(mf, _)| unit(*mf) == f).map(|(_, n)| n.as_str()).collect();
            render_file(t, out, f, by_file.get(&f).map_or(&[][..], Vec::as_slice), &mains)
        })
        .collect()
}

/// The analysis of each of `files` as a check session types them: the classes of the program
/// that each defines, top-level and nested ones, with the entry points' classes marked `main`
/// (an object's `main`, and the class `@main` makes, named after its method).
pub fn render_typed(t: &mut Worker, files: &[FileId], unit_file: &[FileId]) -> Vec<String> {
    let unit = |f: FileId| unit_file.get(f.0 as usize).copied().unwrap_or(f);
    let mut by_file: FxMap<FileId, Vec<ClassId>> = FxMap::default();
    for i in 0..t.syms.classes.len() {
        let c = ClassId(i as u32);
        let info = t.syms.class(c);
        if info.def.is_none() || matches!(info.kind, ClassKind::Builtin | ClassKind::Opaque) || in_body(t, c) {
            continue;
        }
        by_file.entry(unit(info.file)).or_default().push(c);
    }
    let mut object_mains: Vec<ClassId> = Vec::new();
    let mut method_mains: Vec<(FileId, String)> = Vec::new();
    let mut holder_mains: Vec<(FileId, String)> = Vec::new();
    for &(main, object) in t.entry_points.clone().iter() {
        let is_main = t.syms.sym(main).is_main;
        match (object, t.syms.sym(main).owner) {
            (Some(c), _) => object_mains.push(c),
            (None, Owner::Class(c)) if !is_main => object_mains.push(c),
            // A package's own `main`: its file's `<stem>$package`, which holds the forwarder.
            (None, Owner::Package(_)) if !is_main => holder_mains.push((unit(t.syms.sym(main).file), t.entry_point_class(main, None))),
            (None, _) => method_mains.push((unit(t.syms.sym(main).file), t.entry_point_class(main, None))),
        }
    }
    // A file's package blocks are files of their own, all answered under the file's path: one
    // entry per file.
    let mut units: Vec<FileId> = Vec::with_capacity(files.len());
    for &f in files {
        let u = unit(f);
        if !units.contains(&u) {
            units.push(u);
        }
    }
    units
        .iter()
        .map(|&f| {
            let mut classes: Vec<String> = Vec::new();
            for &c in by_file.get(&f).map_or(&[][..], Vec::as_slice) {
                let mains: Vec<String> = if object_mains.contains(&c) { vec![api_name(t, c)] } else { Vec::new() };
                let mains: Vec<&str> = mains.iter().map(String::as_str).collect();
                let binaries = binaries(t, c);
                classes.push(render_class(t, c, &binaries, &mains));
            }
            let mut own_mains: Vec<&String> = method_mains.iter().filter(|(mf, _)| *mf == f).map(|(_, n)| n).collect();
            own_mains.sort();
            for name in own_mains {
                classes.push(class_object(name, "class", true, true, false, true, &[], &[], &[], true, &[name.replace('.', "/")]));
            }
            let mut own_holders: Vec<&String> = holder_mains.iter().filter(|(mf, _)| *mf == f).map(|(_, n)| n).collect();
            own_holders.sort();
            own_holders.dedup();
            for name in own_holders {
                let path = name.replace('.', "/");
                classes.push(class_object(name, "object", true, true, false, true, &[], &[], &[], true, &[path.clone(), format!("{}$", path)]));
            }
            let mut out = String::from("{\"file\":");
            json_string(&t.source(f).path, &mut out);
            out.push_str(",\"classes\":[");
            out.push_str(&classes.join(","));
            out.push_str("],\"local\":[]}");
            out
        })
        .collect()
}

/// Whether `c` is defined in a body: local or anonymous, or nested in a class that is. Such a
/// class is no product of discovery, and the arena keeps the ones a retype of the body replaced.
fn in_body(t: &Worker, c: ClassId) -> bool {
    let mut c = c;
    loop {
        let info = t.syms.class(c);
        if info.kind == ClassKind::Anon {
            return true;
        }
        match info.owner {
            Owner::Local => return true,
            Owner::Class(o) => c = o,
            Owner::Package(_) => return false,
        }
    }
}

/// The binary names the JVM gives the class, as paths without an
/// extension (`p/Outer$Inner`; `p/Obj$` and its mirror `p/Obj` for a top-level object without a
/// companion class): what a check's analysis names as the products sbt-teq stamps. A singleton
/// enum case is a value of its companion and has none.
fn binaries(t: &Worker, c: ClassId) -> Vec<String> {
    let info = t.syms.class(c);
    if info.kind == ClassKind::EnumCase && info.singleton.is_some() {
        return Vec::new();
    }
    let binary = match package_object_member(t, c) {
        Some(p) => through_package_object(&t.pkg_path(p), &t.binary_name(c), "$"),
        None => t.binary_name(c),
    }
    .replace('.', "/");
    let mirror = info.kind == ClassKind::Object && is_top_level(t, c) && info.companion.map_or(true, |k| t.syms.class(k).def.is_none());
    if mirror {
        vec![binary.strip_suffix('$').unwrap_or(&binary).to_string(), binary]
    } else {
        vec![binary]
    }
}

/// The package whose `package object` declares the class, or the class it is nested in: scalac
/// makes such a class a member of the object `p.package`. The object itself, which the package
/// object's parents make (`object package`), is the package's own.
fn package_object_member(t: &Worker, c: ClassId) -> Option<PkgId> {
    let mut k = c;
    loop {
        let info = t.syms.class(k);
        match info.owner {
            Owner::Package(_) if info.name == crate::names::PACKAGE => return None,
            Owner::Package(p) => return t.ast(info.file).package_object.then_some(p),
            Owner::Class(o) => k = o,
            Owner::Local => return None,
        }
    }
}

/// The name zinc knows a module class of a `package object p`'s block by
/// (`Worker::package_object_holder`): `p.package` for the object of its members,
/// `p.<stem>$package` by its binary name; any other keeps its own.
fn package_object_holder(t: &Worker, block: FileId, dotted: &str) -> String {
    let stem = crate::tasty::write::file_stem(&t.source(block).path);
    let holder = format!("{}$package", stem);
    match dotted.rsplit_once('.') {
        Some((pkg, own)) if own == holder => format!("{}.package", pkg),
        None if dotted == holder => "package".to_string(),
        _ => dotted.to_string(),
    }
}

/// `p.Suite` as a member of `p.package`: `p.package$.Suite` for zinc's name (`sep` "."), and
/// `p.package$Suite` for the binary one (`sep` "$").
fn through_package_object(pkg: &str, name: &str, sep: &str) -> String {
    let (head, rest) = if pkg.is_empty() { (String::new(), name) } else { (format!("{}.", pkg), name.strip_prefix(&format!("{}.", pkg)).unwrap_or(name)) };
    let joined = if sep == "." { format!("package$.{}", rest) } else { format!("package${}", rest) };
    format!("{}{}", head, joined)
}

/// The class's name in zinc's API, as scalac's callback gives it.
fn api_name(t: &Worker, c: ClassId) -> String {
    match package_object_member(t, c) {
        Some(p) => through_package_object(&t.pkg_path(p), &t.class_path(c), "."),
        None => t.class_path(c),
    }
}

/// Whether the class is a top-level one for discovery: a package's own, not a member of its
/// `package object`.
fn is_top_level(t: &Worker, c: ClassId) -> bool {
    matches!(t.syms.class(c).owner, Owner::Package(_)) && package_object_member(t, c).is_none()
}

fn render_file(t: &mut Worker, out: &Output, file: FileId, emitted: &[usize], mains: &[&str]) -> String {
    let syms = &t.syms;
    let mut of_class: Vec<(ClassId, Vec<String>)> = Vec::new();
    let mut local: Vec<String> = Vec::new();
    let mut standalone: Vec<(String, Option<FileId>)> = Vec::new();
    for &i in emitted {
        let c = &out.classes[i];
        let path = format!("{}.class", c.name);
        match c.of {
            Some(id) if syms.class(id).owner == Owner::Local || syms.class(id).kind == ClassKind::Anon => local.push(path),
            Some(id) => match of_class.iter_mut().find(|(k, _)| *k == id) {
                Some((_, paths)) => paths.push(path),
                None => of_class.push((id, vec![path])),
            },
            None => standalone.push((c.name.clone(), c.source)),
        }
    }
    let mut classes: Vec<String> = Vec::new();
    for (c, paths) in of_class {
        classes.push(render_class(t, c, &paths, mains));
    }
    // A class without a class of the program behind it is the module class of the file's
    // top-level definitions (with its mirror class), or the class of a `@main` method.
    let mut modules: FxMap<String, Vec<String>> = FxMap::default();
    for (name, source) in standalone {
        let dotted = name.trim_end_matches('$').replace('/', ".");
        let dotted = match source {
            Some(block) if t.package_object_holder(block) => package_object_holder(t, block, &dotted),
            _ => dotted,
        };
        modules.entry(dotted).or_default().push(format!("{}.class", name));
    }
    let mut module_names: Vec<String> = modules.keys().cloned().collect();
    module_names.sort();
    for name in module_names {
        let mut paths = modules.remove(&name).unwrap();
        paths.sort();
        let main = mains.contains(&name.as_str());
        classes.push(class_object(&name, if main { "class" } else { "object" }, true, true, false, true, &[], &[], &[], main, &paths));
    }
    local.sort();
    let mut out = String::from("{\"file\":");
    json_string(&t.source(file).path, &mut out);
    out.push_str(",\"classes\":[");
    out.push_str(&classes.join(","));
    out.push_str("],\"local\":[");
    for (i, p) in local.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        json_string(p, &mut out);
    }
    out.push_str("]}");
    out
}

fn render_class(t: &mut Worker, c: ClassId, paths: &[String], mains: &[&str]) -> String {
    let name = api_name(t, c);
    let info = t.syms.class(c);
    let kind = match info.kind {
        ClassKind::Object => "object",
        ClassKind::EnumCase if info.singleton.is_some() => "object",
        ClassKind::Trait => "trait",
        _ => "class",
    };
    let top = is_top_level(t, c);
    let public = info.mods & (mods::PRIVATE | mods::PROTECTED) == 0;
    let abstract_ = info.kind == ClassKind::Trait || info.mods & mods::ABSTRACT != 0;
    let final_ = kind == "object" || info.mods & mods::FINAL != 0;
    let bases: Vec<String> = info.base_types.iter().skip(1).map(|&(b, _)| b).collect::<Vec<_>>().into_iter().map(|b| t.class_path(b)).collect();
    let annotations = annotation_names(t, c);
    let method_annotations = method_annotation_names(t, c);
    let main = mains.contains(&name.as_str());
    let mut paths = paths.to_vec();
    paths.sort();
    class_object(&name, kind, top, public, abstract_, final_, &bases, &annotations, &method_annotations, main, &paths)
}

/// The qualified names of the class's annotations, as they resolve at the class; one that does
/// not resolve keeps the name as written.
pub(super) fn annotation_names(t: &mut Worker, c: ClassId) -> Vec<String> {
    let info = t.syms.class(c);
    let (file, owner, span, def) = (info.file, info.owner, info.span, info.def);
    let Some(d) = def else { return Vec::new() };
    let written = written_annotations(t, file, d);
    resolve_annotations(t, file, owner, span, written)
}

/// The qualified names of the annotations on the public methods the class declares or
/// inherits from the program's classes, each once.
fn method_annotation_names(t: &mut Worker, c: ClassId) -> Vec<String> {
    let classes: Vec<ClassId> = std::iter::once(c).chain(t.syms.class(c).base_types.iter().skip(1).map(|&(b, _)| b)).collect();
    let mut out: Vec<String> = Vec::new();
    for k in classes {
        let members: Vec<SymId> = t.syms.class(k).member_order.clone();
        for m in members {
            let s = t.syms.sym(m);
            if s.kind != SymKind::Def || s.mods & (mods::PRIVATE | mods::PROTECTED) != 0 {
                continue;
            }
            let (file, owner, span, def) = (s.file, s.owner, s.span, s.def);
            let Some(d) = def else { continue };
            let written = written_annotations(t, file, d);
            for name in resolve_annotations(t, file, owner, span, written) {
                if !out.contains(&name) {
                    out.push(name);
                }
            }
        }
    }
    out
}

pub(super) fn resolve_annotations(t: &mut Worker, file: FileId, owner: Owner, span: crate::source::Span, written: Vec<(crate::intern::Name, Option<crate::ast::TyExprId>)>) -> Vec<String> {
    if written.is_empty() {
        return Vec::new();
    }
    let env = t.env_at(file, owner, span.start);
    t.with_env(env, |t| {
        let mark = t.diags.items.len();
        let out = written
            .into_iter()
            .map(|(n, path)| {
                let class = match path.map(|p| t.resolve_type_ctor(p)).map(|r| t.types.get(r)) {
                    Some(crate::types::Type::Class(k, _) | crate::types::Type::Ctor(k)) => Some(k),
                    _ => match t.lookup_type(n) {
                        Some(TypeRef::Class(a)) => Some(a),
                        _ => None,
                    },
                };
                class.map_or_else(|| t.name_str(n), |k| t.class_path(k))
            })
            .collect();
        t.diags.items.truncate(mark);
        out
    })
}

/// The annotations a definition's source writes: each one's name, and the path its instance
/// `new <path>(...)` names, which a qualified annotation (`@ann.Test`) needs.
pub(super) fn written_annotations(t: &Worker, file: FileId, def: crate::ast::DefId) -> Vec<(crate::intern::Name, Option<crate::ast::TyExprId>)> {
    let ast = t.ast(file);
    ast.def(def)
        .annots
        .iter()
        .map(|a| (a.name, ast.annot_new(a).map(|(path, _)| path)))
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn class_object(name: &str, kind: &str, top: bool, public: bool, abstract_: bool, final_: bool, bases: &[String], annotations: &[String], method_annotations: &[String], main: bool, files: &[String]) -> String {
    let mut out = String::from("{\"name\":");
    json_string(name, &mut out);
    out.push_str(",\"kind\":\"");
    out.push_str(kind);
    out.push_str(&format!("\",\"top\":{},\"public\":{},\"abstract\":{},\"final\":{},\"bases\":", top, public, abstract_, final_));
    strings(bases, &mut out);
    out.push_str(",\"annotations\":");
    strings(annotations, &mut out);
    out.push_str(",\"methodAnnotations\":");
    strings(method_annotations, &mut out);
    out.push_str(&format!(",\"main\":{},\"files\":", main));
    strings(files, &mut out);
    out.push('}');
    out
}

fn strings(items: &[String], out: &mut String) {
    out.push('[');
    for (i, s) in items.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        json_string(s, out);
    }
    out.push(']');
}
