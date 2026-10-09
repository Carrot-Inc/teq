//! `--size-report`: where the bytes of the JavaScript output go, per module, per package and
//! per definition, the size counterpart of `--time`.

use super::layout::Module;
use super::names::sanitize;
use super::{Def, Emitter, Sized};
use crate::intern::FxMap;
use crate::source::{FileId, SourceFile};
use crate::symbols::*;
use crate::types::*;
use std::fmt::Write;

const TOP: usize = 20;

impl<'a> Emitter<'a> {
    /// `files` are the modules of a split build, or the one file of a single-file build under
    /// an empty name; `paths` the source files, which name the groups of top-level vals.
    pub(crate) fn size_report(
        &self,
        sizes: &[Sized],
        modules: &[Module],
        files: &[(String, String)],
        runtime: usize,
        paths: &[SourceFile],
    ) -> String {
        // A definition left out of the output takes no bytes; the report does not count it.
        let sizes: Vec<&Sized> = sizes.iter().filter(|s| s.bytes > 0).collect();
        let total: usize = files.iter().map(|(_, text)| text.len()).sum();
        let mut out = String::new();
        let _ = writeln!(out, "size: {} bytes, runtime {}", group(total), group(runtime));
        if files.len() > 1 {
            let _ = writeln!(out, "{:>12}  {:>5}  module", "bytes", "defs");
            let mut rows: Vec<(usize, usize, &str)> = files
                .iter()
                .map(|(name, text)| {
                    let stem = name.strip_suffix(".mjs").unwrap_or(name);
                    let m = modules.iter().position(|m| m.name == stem);
                    let defs = m.map_or(0, |m| sizes.iter().filter(|s| s.module == m).map(|s| s.def).collect::<std::collections::BTreeSet<_>>().len());
                    (text.len(), defs, name.as_str())
                })
                .collect();
            rows.sort_by(|a, b| b.0.cmp(&a.0).then(a.2.cmp(b.2)));
            for (bytes, defs, name) in rows {
                let _ = writeln!(out, "{:>12}  {:>5}  {}", group(bytes), defs, name);
            }
        }
        let mut per_package: FxMap<String, (usize, usize)> = FxMap::default();
        let mut per_def: FxMap<Def, usize> = FxMap::default();
        for s in &sizes {
            *per_def.entry(s.def).or_insert(0) += s.bytes;
        }
        for (&def, &bytes) in &per_def {
            let entry = per_package.entry(self.package_of_def(def)).or_insert((0, 0));
            entry.0 += bytes;
            entry.1 += 1;
        }
        let mut packages: Vec<(String, (usize, usize))> = per_package.into_iter().collect();
        packages.sort_by(|a, b| b.1 .0.cmp(&a.1 .0).then(a.0.cmp(&b.0)));
        let _ = writeln!(out, "{:>12}  {:>5}  package", "bytes", "defs");
        for (name, (bytes, defs)) in packages {
            let _ = writeln!(out, "{:>12}  {:>5}  {}", group(bytes), defs, name);
        }
        // The definitions compiled from a jar's TASTy, by jar.
        let mut per_jar: FxMap<&str, (usize, usize)> = FxMap::default();
        for (&def, &bytes) in &per_def {
            let file = match def {
                Def::Class(c) => self.syms.class(c).file,
                Def::Sym(s) => self.syms.sym(s).file,
                Def::File(f) => f,
                Def::Outlined(f) => self.syms.sym(self.outline.funs[f as usize].callee).file,
            };
            let Some((jar, _)) = paths.get(file.0 as usize).and_then(|f| f.path.split_once('!')) else { continue };
            let entry = per_jar.entry(jar.rsplit(std::path::is_separator).next().unwrap_or(jar)).or_insert((0, 0));
            entry.0 += bytes;
            entry.1 += 1;
        }
        if !per_jar.is_empty() {
            let mut jars: Vec<(&str, (usize, usize))> = per_jar.into_iter().collect();
            jars.sort_by(|a, b| b.1 .0.cmp(&a.1 .0).then(a.0.cmp(b.0)));
            let _ = writeln!(out, "{:>12}  {:>5}  library", "bytes", "defs");
            for (jar, (bytes, defs)) in jars {
                let _ = writeln!(out, "{:>12}  {:>5}  {}", group(bytes), defs, jar);
            }
        }
        self.anonymous_classes(&sizes, paths, &mut out);
        self.outlined_expansions(&per_def, &mut out);
        // A class is measured twice, its body and its registration; the report adds them up.
        let mut defs: Vec<(Def, usize)> = per_def.into_iter().collect();
        defs.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        let _ = writeln!(out, "{:>12}  definition (largest {} of {})", "bytes", TOP.min(defs.len()), defs.len());
        for (def, bytes) in defs.into_iter().take(TOP) {
            let _ = writeln!(out, "{:>12}  {}", group(bytes), self.display_name(def, paths));
        }
        out
    }

    /// The anonymous classes the program reaches, grouped by the expression that makes them
    /// (one group per `Outer$$anon$<file>_<offset>`, whatever the call sites an inline
    /// expansion adds): how many are written, their bytes and those of their registrations,
    /// how many distinct bodies they have, and how many are closures in the output instead.
    fn anonymous_classes(&self, sizes: &[&Sized], paths: &[SourceFile], out: &mut String) {
        struct Group {
            name: String,
            count: usize,
            bytes: usize,
            registrations: usize,
            hashes: std::collections::BTreeSet<u64>,
            closures: usize,
            shared: usize,
        }
        let mut per_class: FxMap<ClassId, (usize, usize, u64)> = FxMap::default();
        for s in sizes {
            let Def::Class(c) = s.def else { continue };
            let entry = per_class.entry(c).or_insert((0, 0, 0));
            if s.registration {
                entry.1 += s.bytes;
            } else {
                entry.0 += s.bytes;
                entry.2 = s.hash;
            }
        }
        let mut groups: FxMap<(FileId, u32), Group> = FxMap::default();
        for (i, info) in self.syms.classes.iter().enumerate() {
            let c = ClassId(i as u32);
            if info.kind != ClassKind::Anon || !self.reach.classes.get(i).copied().unwrap_or(false) {
                continue;
            }
            let group = groups.entry((info.file, info.span.start)).or_insert_with(|| {
                let full = self.interner.get(info.name);
                let head = full.find("$$anon$").map_or(full.len(), |at| {
                    let rest = &full[at + "$$anon$".len()..];
                    at + "$$anon$".len() + rest.find('$').unwrap_or(rest.len())
                });
                let path = paths.get(info.file.0 as usize).map_or("", |f| f.path.as_str());
                let stem = path.rsplit(['/', '\\']).next().unwrap_or(path);
                Group { name: format!("{} ({})", sanitize(&full[..head]), stem), count: 0, bytes: 0, registrations: 0, hashes: Default::default(), closures: 0, shared: 0 }
            });
            if self.closure_anons.contains_key(&c) {
                group.closures += 1;
            } else if self.shared.stands_for_another(c) {
                group.shared += 1;
            } else if let Some(&(bytes, registration, hash)) = per_class.get(&c) {
                group.count += 1;
                group.bytes += bytes;
                group.registrations += registration;
                group.hashes.insert(hash);
            }
        }
        if groups.is_empty() {
            return;
        }
        let mut rows: Vec<Group> = groups.into_values().collect();
        rows.sort_by(|a, b| (b.count + b.closures + b.shared, b.bytes + b.registrations).cmp(&(a.count + a.closures + a.shared, a.bytes + a.registrations)).then(a.name.cmp(&b.name)));
        let _ = writeln!(
            out,
            "{:>12}  {:>13}  {:>6}  {:>8}  {:>8}  {:>6}  anonymous classes by expression (largest {} of {})",
            "bytes", "registrations", "count", "distinct", "closures", "shared", TOP.min(rows.len()), rows.len()
        );
        for g in rows.into_iter().take(TOP) {
            let _ = writeln!(out, "{:>12}  {:>13}  {:>6}  {:>8}  {:>8}  {:>6}  {}", group(g.bytes), group(g.registrations), g.count, g.hashes.len(), g.closures, g.shared, g.name);
        }
    }

    /// The functions of outlined expansions by the inline method expanded: how many shapes it
    /// has, how many sites call them and the bytes of the functions.
    fn outlined_expansions(&self, per_def: &FxMap<Def, usize>, out: &mut String) {
        let mut rows: FxMap<SymId, (usize, u32, usize)> = FxMap::default();
        for (&def, &bytes) in per_def {
            let Def::Outlined(f) = def else { continue };
            let fun = &self.outline.funs[f as usize];
            let row = rows.entry(fun.callee).or_insert((0, 0, 0));
            row.0 += 1;
            row.1 += fun.calls;
            row.2 += bytes;
        }
        if rows.is_empty() {
            return;
        }
        let mut rows: Vec<(String, (usize, u32, usize))> = rows.into_iter().map(|(s, r)| (sanitize(self.interner.get(self.syms.sym(s).name)), r)).collect();
        rows.sort_by(|a, b| b.1 .1.cmp(&a.1 .1).then(b.1 .2.cmp(&a.1 .2)).then(a.0.cmp(&b.0)));
        let (shapes, calls, bytes) = rows.iter().fold((0, 0, 0), |acc, (_, r)| (acc.0 + r.0, acc.1 + r.1, acc.2 + r.2));
        let _ = writeln!(
            out,
            "{:>12}  {:>6}  {:>6}  outlined expansions by inline method (largest {} of {}; {} bytes, {} shapes, {} calls)",
            "bytes", "shapes", "calls", TOP.min(rows.len()), rows.len(), group(bytes), shapes, calls
        );
        for (name, (shapes, calls, bytes)) in rows.into_iter().take(TOP) {
            let _ = writeln!(out, "{:>12}  {:>6}  {:>6}  {}", group(bytes), shapes, calls, name);
        }
    }

    /// The dotted package of a definition, `scala.collection.mutable`; `<root>` for the empty one.
    fn package_of_def(&self, def: Def) -> String {
        let pkg = match def {
            Def::Class(c) => self.package_of(self.syms.class(c).owner),
            Def::Sym(s) => self.package_of(self.syms.sym(s).owner),
            Def::File(f) => self.package_of_file(f),
            Def::Outlined(f) => self.package_of(self.syms.sym(self.outline.funs[f as usize].callee).owner),
        };
        let mut name = String::new();
        if let Some(p) = pkg {
            self.pkg_prefix(p, &mut name);
        }
        if name.is_empty() {
            return "<root>".to_string();
        }
        name.pop();
        name.replace('$', ".")
    }

    fn package_of_file(&self, f: FileId) -> Option<PkgId> {
        self.prog.top_vals.iter().map(|&(s, _)| self.syms.sym(s)).find(|s| s.file == f).and_then(|s| self.package_of(s.owner))
    }

    fn display_name(&self, def: Def, paths: &[SourceFile]) -> String {
        let mut name = self.package_of_def(def);
        if name == "<root>" {
            name.clear();
        } else {
            name.push('.');
        }
        match def {
            Def::Class(c) => {
                let info = self.syms.class(c);
                self.class_prefix(info.owner, &mut name);
                name.push_str(&sanitize(self.interner.get(info.name)));
                match info.kind {
                    ClassKind::Object => name.push_str(" (object)"),
                    ClassKind::GivenImpl => name.push_str(" (given)"),
                    ClassKind::EnumCase if info.singleton.is_some() => name.push_str(" (enum value)"),
                    ClassKind::Trait => name.push_str(" (trait)"),
                    ClassKind::Anon => name.push_str(" (anonymous)"),
                    _ => {}
                }
            }
            Def::Sym(s) => {
                let info = self.syms.sym(s);
                name.push_str(&sanitize(self.interner.get(info.name)));
                if info.is_extension {
                    name.push_str(" (extension)");
                }
            }
            Def::Outlined(f) => {
                let fun = &self.outline.funs[f as usize];
                let _ = write!(name, "{} (outlined, {} calls)", fun.name, fun.calls);
            }
            Def::File(f) => {
                let path = paths.get(f.0 as usize).map_or("", |file| file.path.as_str());
                let stem = path.rsplit(['/', '\\']).next().unwrap_or(path);
                let _ = write!(name, "<vals of {}>", stem);
            }
        }
        name
    }
}

/// `1,234,567`.
fn group(n: usize) -> String {
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}
