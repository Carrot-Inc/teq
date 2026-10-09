//! The shape census of `TEQ_CLASSPATH_DETAIL`: for every std class that stands for a class of
//! the jars, the members of the jar's class, declared and inherited, that the std's class lacks
//! or declares only with another shape. A library body calling one of them stops where the
//! program reaches it; the census says so before that.

use super::super::Worker;
use super::compile::STD_BINDINGS;
use super::types::MapCx;
use crate::intern::{FxMap, Name};
use crate::symbols::*;
use crate::tasty::tags;
use crate::tasty::tree::{index_top_level, Clause, Decoder, TType};
use crate::tasty::TastyFile;
use crate::types::*;

/// A method's shape: how many type parameters and the parameters of each clause; a value has
/// neither.
type Shape = (usize, Vec<usize>);

pub struct ShapeGap {
    pub std: String,
    pub jar: String,
    pub members: usize,
    pub lacking: Vec<String>,
    pub other_shape: Vec<String>,
}

fn describe(shape: &Shape) -> String {
    let tparams = if shape.0 > 0 { format!("[{}]", shape.0) } else { String::new() };
    let clauses: String = shape.1.iter().map(|n| format!("({})", n)).collect();
    format!("{}{}", tparams, clauses)
}

/// `def size(): Int` and `def size: Int` are one shape: a call fits both.
fn shape(tparams: usize, mut params: Vec<usize>) -> Shape {
    if params == [0] {
        params.clear();
    }
    (tparams, params)
}

fn shape_of_clauses(clauses: &[Clause]) -> Shape {
    let mut tparams = 0;
    let mut params = Vec::new();
    for c in clauses {
        match c {
            Clause::Types(ps) => tparams += ps.len(),
            Clause::Terms(ps) => params.push(ps.len()),
        }
    }
    shape(tparams, params)
}

/// Members every class has or that the typer supplies for a case class, which no std class
/// declares.
fn universal(name: &str) -> bool {
    matches!(
        name,
        "equals" | "hashCode" | "toString" | "getClass" | "##" | "==" | "!=" | "eq" | "ne" | "asInstanceOf" | "isInstanceOf" | "synchronized" | "clone" | "finalize" | "notify" | "notifyAll" | "wait"
            | "canEqual" | "productArity" | "productElement" | "productElementName" | "productElementNames" | "productIterator" | "productPrefix" | "copy" | "unapply" | "writeReplace"
    )
}

impl<'a> Worker<'a> {
    pub fn shape_census(&mut self) {
        if self.loaded.as_ref().map_or(true, |l| l.scala_library) {
            return;
        }
        let mut memo: FxMap<(PkgId, String), Vec<(String, Shape)>> = FxMap::default();
        let mut gaps = Vec::new();
        for (c, pkg, name) in self.std_bound_pairs() {
            let text = self.name_str(name);
            let Some(jar) = self.jar_members(pkg, &text, &mut memo) else { continue };
            let std = self.std_shapes(c);
            let mut by_name: FxMap<&str, Vec<&Shape>> = FxMap::default();
            for (n, shape) in &jar {
                by_name.entry(n).or_default().push(shape);
            }
            let mut names: Vec<&str> = by_name.keys().copied().collect();
            names.sort_unstable();
            let mut lacking = Vec::new();
            let mut other_shape = Vec::new();
            for n in names {
                match std.get(n) {
                    None => lacking.push(n.to_string()),
                    Some(shapes) if !by_name[n].iter().any(|s| shapes.contains(s)) => other_shape.push(format!("{}{}", n, describe(by_name[n][0]))),
                    Some(_) => {}
                }
            }
            let jar_path = format!("{}.{}", self.pkg_description(pkg), text);
            gaps.push(ShapeGap { std: self.class_path(c), jar: jar_path, members: by_name.len(), lacking, other_shape });
        }
        gaps.sort_by(|a, b| a.std.cmp(&b.std));
        self.loaded_mut().bodies.shapes = gaps;
    }

    /// The std classes that stand for a jar's class: those of `STD_BINDINGS`, and those whose
    /// package the jars hold a TASTy file of the same name in.
    fn std_bound_pairs(&mut self) -> Vec<(ClassId, PkgId, Name)> {
        let mut out: Vec<(ClassId, PkgId, Name)> = Vec::new();
        for &(from, name, to) in STD_BINDINGS {
            let (Some(from_pkg), Some(to_pkg)) = (self.pkg_by_path(from), self.pkg_by_path(to)) else { continue };
            let n = self.interner.intern(name);
            let Some(c) = self.syms.pkg(to_pkg).entries.get(&n).and_then(|e| e.class) else { continue };
            if !self.in_jar(self.syms.class(c).file) {
                out.push((c, from_pkg, n));
            }
        }
        for i in 0..self.syms.pkgs.len() {
            let p = PkgId(i as u32);
            let Some(slot) = self.loaded.as_ref().unwrap().slot(p) else { continue };
            let classes: Vec<(Name, ClassId)> = self.syms.pkg(p).entries.iter().filter_map(|(&n, e)| e.class.map(|c| (n, c))).collect();
            for (n, c) in classes {
                let info = self.syms.class(c);
                if self.in_jar(info.file) || matches!(info.kind, ClassKind::Builtin | ClassKind::Object) || out.iter().any(|&(k, _, _)| k == c) {
                    continue;
                }
                if self.class_file_named(slot, n).is_some() {
                    out.push((c, p, n));
                }
            }
        }
        out
    }

    fn pkg_by_path(&mut self, path: &str) -> Option<PkgId> {
        let mut pkg = ROOT_PKG;
        for seg in path.split('.') {
            let n = self.interner.intern(seg);
            pkg = self.syms.pkg(pkg).entries.get(&n).and_then(|e| e.pkg)?;
        }
        Some(pkg)
    }

    /// The public members of the jar's class `name` in `pkg` with their shapes, its parents'
    /// included; a Java parent has no TASTy file and contributes nothing.
    fn jar_members(&mut self, pkg: PkgId, name: &str, memo: &mut FxMap<(PkgId, String), Vec<(String, Shape)>>) -> Option<Vec<(String, Shape)>> {
        let key = (pkg, name.to_string());
        if let Some(m) = memo.get(&key) {
            return Some(m.clone());
        }
        memo.insert(key.clone(), Vec::new());
        let slot = self.loaded.as_ref().unwrap().slot(pkg)?;
        let n = self.interner.intern(name);
        let f = self.class_file_named(slot, n)?;
        let file = self.open_file(f)?;
        let tasty = self.tasty(file);
        let tops = index_top_level(&tasty);
        let entry = tops.iter().map(|t| &t.entry).find(|e| e.is_class && !e.flags.has(tags::OBJECT) && tasty.simple(e.name) == Some(name))?;
        let mut decoder = Decoder::new(&tasty);
        let sig = decoder.class_sig(entry.addr);
        let mut out: Vec<(String, Shape)> = Vec::new();
        for m in &sig.index.members {
            if !matches!(m.tag, tags::VALDEF | tags::DEFDEF) || m.flags.has(tags::PRIVATE) || m.flags.has(tags::SYNTHETIC) || m.flags.has(tags::ARTIFACT) {
                continue;
            }
            let Some(text) = tasty.simple(m.name) else { continue };
            if text == "<init>" || text.contains('$') || universal(text) {
                continue;
            }
            let shape = if m.tag == tags::DEFDEF { shape_of_clauses(&decoder.def_sig(m.addr).clauses) } else { (0, Vec::new()) };
            let member = (text.to_string(), shape);
            if !out.contains(&member) {
                out.push(member);
            }
        }
        let cx = MapCx::new(file);
        for p in &sig.parents {
            let Some((ppkg, pname)) = self.jar_parent_class(&cx, &tasty, p) else { continue };
            if let Some(inherited) = self.jar_members(ppkg, &pname, memo) {
                for m in inherited {
                    if !out.contains(&m) {
                        out.push(m);
                    }
                }
            }
        }
        memo.insert(key, out.clone());
        Some(out)
    }

    fn jar_parent_class(&mut self, cx: &MapCx, tasty: &TastyFile, t: &TType) -> Option<(PkgId, String)> {
        match t {
            TType::Applied(f, _) => self.jar_parent_class(cx, tasty, f),
            TType::Annotated(inner, _) => self.jar_parent_class(cx, tasty, inner),
            TType::TypeRef(prefix, n) => match &**prefix {
                TType::Package(p) => Some((self.resolve_package(cx, *p), tasty.simple(*n)?.to_string())),
                _ => None,
            },
            _ => None,
        }
    }

    /// The members of the std class `c` by name with their shapes: its own, its ancestors' and
    /// the extension methods of its package that take it as the receiver.
    fn std_shapes(&mut self, c: ClassId) -> FxMap<String, Vec<Shape>> {
        self.complete_class(c);
        let mut classes = vec![c];
        classes.extend(self.syms.class(c).base_types.iter().map(|&(b, _)| b));
        let mut out: FxMap<String, Vec<Shape>> = FxMap::default();
        for k in classes {
            self.complete_class(k);
            let members: Vec<SymId> = self.syms.class(k).members.values().copied().collect();
            for m in members {
                let alts: Vec<SymId> = self.syms.alternatives(m).map_or_else(|| vec![m], |a| a.to_vec());
                for alt in alts {
                    let name = self.name_str(self.syms.sym(alt).name);
                    let shape = self.sym_shape(alt, 0, 0);
                    out.entry(name).or_default().push(shape);
                }
            }
        }
        if let Owner::Package(p) = self.syms.class(c).owner {
            let extensions: Vec<SymId> = self.syms.pkg(p).entries.values().flat_map(|e| e.extensions.iter().copied()).collect();
            for e in extensions {
                if !self.extension_receives(e, c) {
                    continue;
                }
                let info = self.syms.sym(e);
                let (skip_tparams, skip_clauses) = (info.ext_tparams as usize, info.ext_clauses as usize);
                let name = self.name_str(info.name);
                let shape = self.sym_shape(e, skip_tparams, skip_clauses);
                out.entry(name).or_default().push(shape);
            }
        }
        out
    }

    fn sym_shape(&mut self, s: SymId, skip_tparams: usize, skip_clauses: usize) -> Shape {
        if self.syms.sym(s).kind != SymKind::Def {
            return (0, Vec::new());
        }
        let sig = self.sig_of(s);
        shape(sig.tparams.len().saturating_sub(skip_tparams), sig.clauses.iter().skip(skip_clauses).map(|c| c.params.len()).collect())
    }

    fn extension_receives(&mut self, e: SymId, c: ClassId) -> bool {
        let Some(first) = self.sig_of(e).clauses.first().and_then(|cl| cl.params.first()).map(|p| p.ty) else { return false };
        match self.class_of(first) {
            Some(k) => k == c || self.syms.class(c).base_types.iter().any(|&(b, _)| b == k),
            None => false,
        }
    }
}
