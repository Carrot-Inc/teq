//! `TEQ_INIT_VIEW=<source root> teq tasty <dir>` (a test's switch): the initialization view of TASTy files printed as the reference's oracle prints it
//! (tests/init-view/InitDump.scala, whose header is the format): per source, the classes the checker collects, then
//! the tree, a node a line, with its position and what the checker reads of it. tests/init-view.sh compares the two.
//!
//! A unit is a source: the files the products hold for it (one per top-level class with its companion, the
//! `$package` file, a package object) are read and their package clauses' statements put in the source's order, as
//! the reference's unit holds them in one `PackageDef` per clause.

use super::view::{classes_post_order, DefFlags, Kind, Node, ViewReader};
use crate::tasty::tree::{Const, TType};
use crate::tasty::{TName, TastyFile};

/// One source's trees and the file each was read from.
pub struct Unit<'f> {
    pub path: String,
    pub parts: Vec<(&'f TastyFile, Vec<Node>, ViewReader<'f>)>,
}

/// The sources the dump has named, `s1` the first.
#[derive(Default)]
pub struct Sources {
    names: Vec<String>,
}

pub struct Printer<'a> {
    pub out: String,
    pub sources: &'a mut Sources,
    /// How a pickle's source path is found on disk (its root).
    pub root: std::path::PathBuf,
}

/// The text of a symbol: its owners' names from the package down, joined by `.`.
#[derive(Clone)]
struct Owners {
    chain: Vec<String>,
}

fn sha256_hex(bytes: &[u8]) -> String {
    crate::task::sha256::hex(bytes)
}

impl<'a> Printer<'a> {
    fn source_id(&mut self, path: &str) -> usize {
        if let Some(i) = self.sources.names.iter().position(|p| p == path) {
            return i + 1;
        }
        self.sources.names.push(path.to_string());
        let n = self.sources.names.len();
        let on_disk = self.root.join(path);
        let name = std::path::Path::new(path).file_name().map_or(path.to_string(), |f| f.to_string_lossy().to_string());
        let (content, sha) = match std::fs::read(&on_disk) {
            Ok(b) if !b.is_empty() => ("yes", sha256_hex(&b)),
            _ => ("no", "-".to_string()),
        };
        self.out.push_str(&format!(
            "source s{} name={} path={} content={} sha256={}\n",
            n,
            name,
            on_disk.display(),
            content,
            sha
        ));
        n
    }

    fn pos(&mut self, v: &ViewReader, n: &Node) -> String {
        let Some(p) = v.pos(n.addr) else { return "[nopos]".to_string() };
        let src = v.source(n.addr).map(|s| v.file.name(s)).unwrap_or_default();
        let id = self.source_id(&src);
        let point = p.point.unwrap_or(p.start);
        let lc = |o: u32| -> String {
            match v.section.as_ref().and_then(|s| s.line_col(o)) {
                Some((l, c)) => format!("{}:{}", l, c - 1),
                None => "?:?".to_string(),
            }
        };
        format!("[s{} {}..{}@{} {}-{}]", id, p.start, p.end, point, lc(p.start), lc(p.end))
    }
}

/// The name of a definition or a reference as the reference prints `Name.toString`.
fn name_text(file: &TastyFile, n: u32) -> String {
    match file.names.get(n as usize) {
        Some(TName::Signed { original, .. }) => file.name(*original),
        Some(TName::ObjectClass(u)) => format!("{}$", file.name(*u)),
        _ => file.name(n),
    }
}

/// A signed name's signature as the reference's dump prints it: `#(params)result`.
fn signature(file: &TastyFile, n: u32) -> Option<String> {
    match file.names.get(n as usize) {
        Some(TName::Signed { params, result, .. }) => {
            let ps: Vec<String> = params
                .iter()
                .map(|&p| if p < 0 { (-p).to_string() } else { file.name(p as u32) })
                .collect();
            Some(format!("#({}){}", ps.join(","), file.name(*result)))
        }
        _ => None,
    }
}

/// The text of a type's symbol, as far as the type names it: a package's full name, a reference's prefix and name.
fn type_symbol(file: &TastyFile, ty: &TType, defs: &std::collections::HashMap<u32, String>) -> String {
    match ty {
        TType::Package(n) => file.name(*n),
        TType::TypeRef(pre, n) | TType::TermRef(pre, n) => {
            let p = type_symbol(file, pre, defs);
            let sig = signature(file, *n).unwrap_or_default();
            format!("{}.{}{}", p, name_text(file, *n), sig)
        }
        TType::LocalType(a, _) | TType::LocalTerm(a, _) => defs.get(a).cloned().unwrap_or_else(|| format!("@{}", a)),
        TType::This(c) => type_symbol(file, c, defs),
        TType::Applied(t, _) => type_symbol(file, t, defs),
        TType::Annotated(t, _) => type_symbol(file, t, defs),
        TType::Const(_) => "-".to_string(),
        _ => "?".to_string(),
    }
}

fn flags_text(f: DefFlags) -> String {
    let names: Vec<&str> = DefFlags::PRINTED.iter().filter(|(b, _)| f.has(*b)).map(|(_, n)| *n).collect();
    if names.is_empty() {
        String::new()
    } else {
        format!(" {{{}}}", names.join(","))
    }
}

fn const_text(file: &TastyFile, c: &Const) -> String {
    match c {
        Const::Unit => "1:()".to_string(),
        Const::Bool(b) => format!("2:{}", b),
        Const::Byte(v) => format!("3:{}", v),
        Const::Short(v) => format!("4:{}", v),
        Const::Char(v) => format!("5:{}", char::from_u32(*v).unwrap_or('?')),
        Const::Int(v) => format!("6:{}", v),
        Const::Long(v) => format!("7:{}", v),
        Const::Float(v) => format!("8:{}", f32::from_bits(*v)),
        Const::Double(v) => format!("9:{}", f64::from_bits(*v)),
        Const::Str(n) => format!("10:{}", file.name(*n)),
        Const::Null => "11:null".to_string(),
        Const::Class(_) => "12:?".to_string(),
    }
}

/// The definitions of a file by address, each with its owners' chain (`symId`).
fn index_defs(file: &TastyFile, nodes: &[Node], owners: &Owners, out: &mut std::collections::HashMap<u32, String>) {
    for n in nodes {
        let mut chain = owners.clone();
        match n.kind {
            Kind::PackageDef => {
                if let Some(TType::Package(p)) = n.children.first().and_then(|c| c.ty.as_ref()) {
                    chain = Owners { chain: vec![file.name(*p)] };
                }
            }
            Kind::ValDef | Kind::DefDef | Kind::TypeDef | Kind::Bind => {
                if let Some(name) = n.name {
                    let mut text = name_text(file, name);
                    if n.is_class_def() && n.flags.has(DefFlags::MODULE) && !text.ends_with('$') {
                        text.push('$');
                    }
                    chain.chain.push(text);
                    out.insert(n.at, chain.chain.join("."));
                }
            }
            _ => {}
        }
        index_defs(file, &n.children, &chain, out);
    }
}

impl<'a> Printer<'a> {
    /// One unit: `unit <path>`, its classes, its trees.
    pub fn unit(&mut self, unit: &mut Unit) {
        let mut body = std::mem::take(&mut self.out);
        let mut classes: Vec<String> = Vec::new();
        let mut defs_all = Vec::new();
        for (file, nodes, _) in unit.parts.iter() {
            let mut defs = std::collections::HashMap::new();
            index_defs(file, nodes, &Owners { chain: Vec::new() }, &mut defs);
            let mut cs = Vec::new();
            classes_post_order(nodes, &mut cs);
            for c in cs {
                let id = defs.get(&c.at).cloned().unwrap_or_default();
                let concrete = c.flags.has(DefFlags::MODULE) || !(c.flags.has(DefFlags::ABSTRACT) || c.flags.has(DefFlags::TRAIT));
                let static_object = c.flags.has(DefFlags::MODULE);
                classes.push(format!("class {} concrete={} staticObject={}", id, concrete, static_object));
            }
            defs_all.push(defs);
        }
        let mut tree = String::new();
        std::mem::swap(&mut self.out, &mut tree);
        // One `PackageDef` a package clause, as the reference's unit holds it: the clause of the first part that has
        // one, its statements those of every part's clause of the same package, in the order of their spans.
        let mut stats: Vec<(u32, usize, &Node)> = Vec::new();
        // The clause of the part whose statements the source has first, which holds the unit's package id as the
        // unit's own `PackageDef` does: each part's clause has it before its own first statement.
        let mut first: Option<(u32, usize, &Node)> = None;
        for (i, (_, nodes, v)) in unit.parts.iter().enumerate() {
            for n in nodes {
                if n.kind == Kind::PackageDef {
                    let lead = n.children.iter().skip(1).filter_map(|s| v.pos(s.addr)).map(|p| p.start).min().unwrap_or(u32::MAX);
                    if first.map_or(true, |(l, _, _)| lead < l) {
                        first = Some((lead, i, n));
                    }
                    for s in n.children.iter().skip(1) {
                        stats.push((v.pos(s.addr).map_or(u32::MAX, |p| p.start), i, s));
                    }
                } else {
                    stats.push((v.pos(n.addr).map_or(u32::MAX, |p| p.start), i, n));
                }
            }
        }
        stats.sort_by_key(|&(start, i, _)| (start, i));
        let mut depth = 0;
        if let Some((_, i, pkg)) = first {
            let (file, _, v) = &unit.parts[i];
            let pos = self.pos(v, pkg);
            self.out.push_str(&format!("PackageDef {}\n", pos));
            if let Some(pid) = pkg.children.first() {
                self.node(file, v, pid, 1, &defs_all[i]);
            }
            depth = 1;
        }
        for (_, i, s) in stats {
            let (file, _, v) = &unit.parts[i];
            self.node(file, v, s, depth, &defs_all[i]);
        }
        std::mem::swap(&mut self.out, &mut tree);
        body.push_str(&format!("unit {}\n", self.root.join(&unit.path).display()));
        for c in classes {
            body.push_str(&c);
            body.push('\n');
        }
        body.push_str(&tree);
        self.out = body;
    }

    fn node(&mut self, file: &TastyFile, v: &ViewReader, n: &Node, depth: usize, defs: &std::collections::HashMap<u32, String>) {
        let indent = "  ".repeat(depth);
        if n.kind == Kind::Import || n.kind == Kind::Export {
            return;
        }
        let pos = self.pos(v, n);
        if n.kind == Kind::Tpt {
            self.out.push_str(&format!("{}Tpt {} paths=[]\n", indent, pos));
            return;
        }
        let sym_of = |ty: &Option<TType>| ty.as_ref().map_or("?".to_string(), |t| type_symbol(file, t, defs));
        let facts = match n.kind {
            Kind::ValDef | Kind::DefDef | Kind::TypeDef => {
                format!(" sym={}{}", defs.get(&n.at).cloned().unwrap_or_else(|| "?".to_string()), flags_text(n.flags))
            }
            Kind::Bind => format!(" sym={}{}", defs.get(&n.at).cloned().unwrap_or_else(|| "?".to_string()), flags_text(n.flags)),
            Kind::Ident => format!(" sym={} pre=?", sym_of(&n.ty)),
            Kind::Select => {
                let name = n.name.map(|nm| format!("{}{}", name_text(file, nm), signature(file, nm).unwrap_or_default())).unwrap_or_default();
                format!(" sym=?.{} qual=?", name)
            }
            Kind::This => format!(" this={}", sym_of(&n.ty)),
            Kind::New => format!(" new={}", sym_of(&n.ty)),
            Kind::Literal => match &n.ty {
                Some(TType::Const(c)) => format!(" const={}", const_text(file, c)),
                _ => String::new(),
            },
            Kind::Apply => " fn=? margs=?".to_string(),
            Kind::TypeApply => " fn=?".to_string(),
            Kind::Return => format!(" from={}", n.target.and_then(|a| defs.get(&a).cloned()).unwrap_or_else(|| "?".to_string())),
            Kind::Inlined => match &n.call {
                Some(c) => {
                    let cp = self.pos(v, c);
                    format!(" call=? callpos={}", cp)
                }
                None => " call=-".to_string(),
            },
            Kind::Closure => " meth=?".to_string(),
            Kind::UnApply => " fun=?".to_string(),
            Kind::Elided => " sym=_ pre=-".to_string(),
            _ => String::new(),
        };
        self.out.push_str(&format!("{}{} {}{}\n", indent, n.kind.name(), pos, facts));
        for c in &n.children {
            self.node(file, v, c, depth + 1, defs);
        }
    }
}

/// The `.tasty` files under `dir`, by path.
fn tasty_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let mut paths: Vec<std::path::PathBuf> = entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
    paths.sort();
    for p in paths {
        if p.is_dir() {
            tasty_files(&p, out);
        } else if p.extension().is_some_and(|e| e == "tasty") {
            out.push(p);
        }
    }
}

/// `TEQ_INIT_VIEW=<root> teq tasty <dir>`: the view of every TASTy file under `dir`, a unit per source, the sources
/// found under `root` (the `-sourceroot` the files were written with).
pub fn run(dir: &str, root: &str) -> Result<String, String> {
    let mut paths = Vec::new();
    tasty_files(std::path::Path::new(dir), &mut paths);
    let mut files = Vec::new();
    for p in &paths {
        let bytes = std::fs::read(p).map_err(|e| format!("{}: {}", p.display(), e))?;
        files.push(TastyFile::parse(bytes).map_err(|e| format!("{}: {}", p.display(), e))?);
    }
    let mut units: Vec<Unit> = Vec::new();
    for file in &files {
        let mut v = ViewReader::new(file);
        let nodes = v.top_level();
        let path = v.section.as_ref().and_then(|s| s.primary()).map(|n| file.name(n)).unwrap_or_default();
        match units.iter_mut().find(|u| u.path == path) {
            Some(u) => u.parts.push((file, nodes, v)),
            None => units.push(Unit { path, parts: vec![(file, nodes, v)] }),
        }
    }
    units.sort_by(|a, b| a.path.cmp(&b.path));
    let mut sources = Sources::default();
    let mut out = String::new();
    for u in &mut units {
        let mut p = Printer { out: String::new(), sources: &mut sources, root: std::path::PathBuf::from(root) };
        p.unit(u);
        out.push_str(&p.out);
    }
    Ok(out)
}
