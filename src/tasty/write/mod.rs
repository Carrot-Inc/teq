//! Writes TASTy for the program's own definitions: one file per top-level class (with its
//! companion), per file's top-level definitions (`<file>$package`) and per package object, as
//! scalac 3.8.4's `Pickler` slices a compilation unit.
//!
//! This part writes the signatures: every definition with its type parameters, parameters,
//! result type, modifiers and annotations, the members scalac's `PostTyper` adds, and in the
//! place of a body the `ELIDED` tree of its type under the `OUTLINE` attribute, which is how
//! the format marks a definition whose body was not pickled (scalac writes the same for Java
//! sources). A shape that cannot be written fails the build with the construct named.

pub(crate) mod buf;
mod names;
mod pickle;

pub use buf::write_nat;
pub use pickle::{scala_name, std_shapes};
pub(crate) use pickle::widened_library_members;

use crate::intern::FxMap;
use crate::source::FileId;
use crate::symbols::*;
use crate::typer::Worker;
use crate::types::*;

/// One `.tasty` file of the products.
pub struct Product {
    /// The package as a path, `a/b`, empty for the root, encoded as the class files' paths are.
    pub package: String,
    /// The file's stem: the top-level class, `<file>$package` or `package`, encoded as its
    /// class file's name is (`$plus` for `+`).
    pub name: String,
    pub source: FileId,
    pub bytes: Vec<u8>,
    pub uuid: [u8; 16],
    /// Every body of the pickle written, none `ELIDED` (no `OUTLINEattr`).
    pub complete: bool,
}

#[derive(Default)]
pub struct Written {
    pub products: Vec<Product>,
    /// Definitions that could not be written, each naming the construct.
    pub errors: Vec<String>,
    /// Shapes written as teq's typer holds them where scalac's differ, by kind, with a count.
    pub approximated: FxMap<String, u32>,
    /// The right-hand sides by producer, and those left `ELIDED` by reason.
    pub bodies: FxMap<String, u32>,
}

/// The TASTy files of the program's files that `owned` selects (all program files when it is
/// `None`); a file's path in the pickles is taken relative to `sourceroot`, as scalac's
/// `-sourceroot` does.
pub fn write_products(w: &mut Worker, owned: Option<&[bool]>, sourceroot: &std::path::Path) -> Written {
    pickle::with_scala_names(|| write_products_now(w, owned, sourceroot))
}

fn write_products_now(w: &mut Worker, owned: Option<&[bool]>, sourceroot: &std::path::Path) -> Written {
    let mut out = Written::default();
    let index = pickle::Index::new(w, sourceroot);
    let n = w.files.len();
    for f in 0..n {
        let file = FileId(f as u32);
        let src = &w.files.as_slice()[f];
        if src.is_std || w.in_jar(file) || !owned.map_or(true, |o| o.get(f).copied().unwrap_or(false)) {
            continue;
        }
        let rel = relative_path(&src.path, sourceroot, index.cwd.as_deref());
        let lines = pickle::Lines::of(&src.text);
        for unit in units_of(w, file) {
            match pickle::pickle_unit(w, &index, &lines, &unit, &rel, &mut out.approximated, &mut out.bodies) {
                Ok(p) => out.products.push(p),
                Err(errors) => out.errors.extend(errors.errors),
            }
        }
    }
    out
}

/// A file's path as scalac's `SourceFile.relativePath` gives it against `-sourceroot`: under
/// the root relative to it, otherwise as it is.
pub(crate) fn relative_path(path: &str, root: &std::path::Path, cwd: Option<&std::path::Path>) -> String {
    path_under(path, root, cwd).unwrap_or_else(|| path.to_string())
}

/// The path of a file under `root`, `None` for a file outside it.
pub(crate) fn path_under(path: &str, root: &std::path::Path, cwd: Option<&std::path::Path>) -> Option<String> {
    let p = std::path::Path::new(path);
    let abs = if p.is_absolute() { p.to_path_buf() } else { cwd.map(|d| d.join(p)).unwrap_or_else(|| p.to_path_buf()) };
    let abs = normalize(&abs);
    let root = normalize(root);
    abs.strip_prefix(&root).ok().map(|rel| rel.to_string_lossy().replace('\\', "/"))
}

fn normalize(p: &std::path::Path) -> std::path::PathBuf {
    let mut out = std::path::PathBuf::new();
    for c in p.components() {
        match c {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    out
}

/// What one TASTy file holds.
pub(crate) enum UnitKind {
    /// A top-level class, trait or object with its companion.
    Class { class: Option<ClassId>, module: Option<ClassId> },
    /// The top-level definitions of a file in one package, the object `<file>$package` or,
    /// for a package object's file, `package`.
    Package { pkg: PkgId, name: String, defs: Vec<crate::ast::DefId> },
}

pub(crate) struct Unit {
    pub file: FileId,
    pub kind: UnitKind,
}

/// The TASTy files of a source file, in the order of its definitions: its classes', then its
/// `$package` objects', each named as the namer named its module class (`Worker::file_module`).
pub(crate) fn units_of(w: &Worker, file: FileId) -> Vec<Unit> {
    let (mut units, package_defs) = split_units(w, file);
    for (pkg, defs) in package_defs {
        let name = w.file_module_name(file, pkg);
        units.push(Unit { file, kind: UnitKind::Package { pkg, name, defs } });
    }
    units
}

/// The packages of a file's top-level definitions that are no class, each with them: the
/// `<file>$package` objects `Desugar.packageDef` makes (`Worker::enter_file_modules`), in one pass
/// over the file's definitions.
pub(crate) fn file_module_packages(w: &Worker, file: FileId) -> Vec<(PkgId, Vec<crate::ast::DefId>)> {
    let ast = w.ast(file);
    let f = file.0 as usize;
    let mut package_defs: Vec<(PkgId, Vec<crate::ast::DefId>)> = Vec::new();
    for &d in &ast.top_level {
        if unit_class(w, file, d).is_some() {
            continue;
        }
        let pkg = owner_pkg(w, file, d).unwrap_or(w.file_pkgs.own()[f]);
        match package_defs.iter_mut().find(|(p, _)| *p == pkg) {
            Some((_, ds)) => ds.push(d),
            None => package_defs.push((pkg, vec![d])),
        }
    }
    // A file of top-level `export` clauses alone has its `<file>$package` for them, as scalac's.
    if package_defs.is_empty() && !ast.top_exports.is_empty() {
        package_defs.push((w.file_pkgs.own()[f], Vec::new()));
    }
    package_defs
}

/// The class of the top-level definition `d` where it has a unit of its own: a class, trait or
/// object, but the companion object of a top-level opaque type, which goes with it into `$package`.
fn unit_class(w: &Worker, file: FileId, d: crate::ast::DefId) -> Option<ClassId> {
    if !matches!(&w.ast(file).def(d).kind, crate::ast::DefKind::Class(_)) {
        return None;
    }
    let c = w.def_classes.get(file.0 as usize, &d).copied()?;
    let opaque_companion = w.syms.class(c).companion.map_or(false, |k| w.syms.class(k).kind == ClassKind::Opaque);
    (!opaque_companion).then_some(c)
}

/// A file's class units, and its top-level definitions that are no class by package.
fn split_units(w: &Worker, file: FileId) -> (Vec<Unit>, Vec<(PkgId, Vec<crate::ast::DefId>)>) {
    let ast = w.ast(file);
    let mut units: Vec<Unit> = Vec::new();
    for &d in &ast.top_level {
        let Some(c) = unit_class(w, file, d) else { continue };
        let info = w.syms.class(c);
        let is_object = info.kind == ClassKind::Object;
        // A class and its companion object share one file, named after them.
        if let Some(existing) = units.iter_mut().find(|u| match u.kind {
            UnitKind::Class { class, module } => {
                let other = if is_object { class } else { module };
                other.map_or(false, |o| w.syms.class(o).companion == Some(c))
            }
            _ => false,
        }) {
            if let UnitKind::Class { class, module } = &mut existing.kind {
                if is_object {
                    *module = Some(c);
                } else {
                    *class = Some(c);
                }
            }
            continue;
        }
        // An enum's companion is made by the typer: it goes with the enum. A case class's the
        // typer made is written as scalac's synthesized one, with the class.
        let companion = info.companion.filter(|&k| w.syms.class(k).def.is_none() && w.syms.class(k).file == file && !w.syms.class(k).made_companion);
        let (class, module) = if is_object { (None, Some(c)) } else { (Some(c), companion) };
        units.push(Unit { file, kind: UnitKind::Class { class, module } });
    }
    (units, file_module_packages(w, file))
}

/// The package a top-level definition was entered in.
fn owner_pkg(w: &Worker, file: FileId, d: crate::ast::DefId) -> Option<PkgId> {
    let f = file.0 as usize;
    let owner = if let Some(&s) = w.def_syms.get(f, &d) {
        w.syms.sym(s).owner
    } else if let Some(&c) = w.def_classes.get(f, &d) {
        w.syms.class(c).owner
    } else if let Some(&a) = w.def_aliases.get(f, &d) {
        w.syms.alias(a).owner
    } else {
        return None;
    };
    match owner {
        Owner::Package(p) => Some(p),
        _ => None,
    }
}

pub(crate) fn file_stem(path: &str) -> String {
    let name = path.rsplit(std::path::is_separator).next().unwrap_or(path);
    name.strip_suffix(".scala").unwrap_or(name).to_string()
}
