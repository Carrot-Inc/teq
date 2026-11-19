//! `TEQ_INIT_VIEW=<source root> teq tasty <dir>` (a test's switch): the initialization view of TASTy files printed
//! as the reference's oracle prints it (tests/init-view/InitDump.scala, whose header is the format): per source, the
//! classes the checker collects, then the tree, a node a line, with its position and what the checker reads of it.
//! tests/init-view.sh compares the two.
//!
//! A unit is a source: the files the products hold for it (one per top-level class with its companion, the
//! `$package` file, a package object) are read into the graph and their package clauses' statements put in the
//! source's order, as the reference's unit holds them in one `PackageDef`; the classes are collected over that one
//! walk.

use super::graph::{find_node, name_text, signed, FileIx, Graph, SymId};
use super::view::{DefFlags, Kind, Node};
use crate::tasty::tree::{Const, TType};
use crate::tasty::TastyFile;

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
    pub g: &'a mut Graph,
    /// The classes the tree being printed is in, innermost last (`ctx.owner.enclosingClass`).
    pub enclosing: Vec<SymId>,
    /// The typed program's classes by source path (`TEQ_INIT_VIEW_INVENTORY`): their names and names' offsets.
    pub inventory: &'a std::collections::HashMap<String, Vec<(String, u32)>>,
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

/// Every class definition of the trees, children first, as `Checker.InitTreeTraverser` collects them.
fn classes_post_order(ix: FileIx, nodes: &[Node], out: &mut Vec<(FileIx, Node)>) {
    for n in nodes {
        classes_post_order(ix, &n.children, out);
        if n.is_class_def() {
            let mut shallow = n.clone();
            shallow.children.clear();
            out.push((ix, shallow));
        }
    }
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
            Ok(b) if !b.is_empty() => ("yes", crate::task::sha256::hex(&b)),
            _ => ("no", "-".to_string()),
        };
        self.out.push_str(&format!("source s{} name={} path={} content={} sha256={}\n", n, name, on_disk.display(), content, sha));
        n
    }

    fn pos(&mut self, ix: FileIx, n: &Node) -> String {
        let (p, src, lc) = {
            let l = &self.g.files[ix as usize];
            let Some(p) = l.spans.get(n.addr).and_then(|p| p.pos) else { return "[nopos]".to_string() };
            let src = l
                .spans
                .get(n.addr)
                .and_then(|p| p.source)
                .or_else(|| l.section.as_ref().and_then(|s| s.primary()))
                .map(|s| l.file.name(s))
                .unwrap_or_default();
            let lc = |o: u32| -> String {
                match l.section.as_ref().and_then(|s| s.line_col(o)) {
                    Some((line, c)) => format!("{}:{}", line, c - 1),
                    None => "?:?".to_string(),
                }
            };
            (p, src, (lc(p.start), lc(p.end)))
        };
        let id = self.source_id(&src);
        let point = p.point.unwrap_or(p.start);
        format!("[s{} {}..{}@{} {}-{}]", id, p.start, p.end, point, lc.0, lc.1)
    }

    fn file(&self, ix: FileIx) -> std::rc::Rc<TastyFile> {
        self.g.files[ix as usize].file.clone()
    }

    /// The text of the symbol a type names, `?` where the graph cannot name it.
    fn ref_text(&mut self, ix: FileIx, ty: &Option<TType>) -> String {
        let Some(ty) = ty else { return "?".to_string() };
        match self.g.resolve(ix, ty) {
            Some(s) => self.g.sym_text(s),
            None => "?".to_string(),
        }
    }

    /// The paths of a type `Semantic.checkTermUsage` evaluates (InitDump's `paths`): each `TermRef` whose prefix is a
    /// singleton (a `this`, a term, a package's `TermRef`), its own parts not visited; `NoPrefix` is none.
    fn paths(&mut self, ix: FileIx, t: &TType, out: &mut Vec<String>) {
        let singleton = |p: &TType| matches!(p, TType::This(_) | TType::TermRef(..) | TType::LocalTerm(..) | TType::Package(_) | TType::RecThis(_));
        match t {
            TType::TermRef(p, _) | TType::LocalTerm(_, Some(p)) if singleton(p) => {
                match self.g.resolve(ix, t) {
                    // A package is no path the checker evaluates (its prefix's `ThisType` in the typer's type).
                    Some(s) if self.g.sym(s).package => {}
                    Some(s) => out.push(self.g.sym_text(s)),
                    None => out.push("?".to_string()),
                }
            }
            // A static symbol's prefix is not visited (`TypeAccumulator.foldOver`'s `stopBecauseStaticOrLocal`).
            TType::TypeRef(p, _) | TType::LocalType(_, Some(p)) | TType::TermRef(p, _) | TType::LocalTerm(_, Some(p)) => {
                let stop = self.g.resolve(ix, t).is_some_and(|s| self.g.sym(s).package || self.g.is_static(s));
                if !stop {
                    self.paths(ix, p, out);
                }
            }
            TType::This(p) | TType::ByName(p) | TType::Alias(p) => self.paths(ix, p, out),
            TType::Applied(f, args) => {
                self.paths(ix, f, out);
                for a in args {
                    self.paths(ix, a, out);
                }
            }
            TType::Bounds(a, b) | TType::And(a, b) | TType::Or(a, b) | TType::BoundedAlias(a, b) | TType::Super(a, b) => {
                self.paths(ix, a, out);
                self.paths(ix, b, out);
            }
            TType::Annotated(a, _) => self.paths(ix, a, out),
            TType::Refined(p, members) => {
                self.paths(ix, p, out);
                for (_, m) in members {
                    if let Some(m) = m {
                        self.paths(ix, m, out);
                    }
                }
            }
            TType::Rec(_, a) => self.paths(ix, a, out),
            TType::Lambda { params, result, .. } => {
                for p in params {
                    self.paths(ix, &p.info, out);
                }
                self.paths(ix, result, out);
            }
            _ => {}
        }
    }

    /// A class constant's type as the reference prints a `Constant`'s value, dotty's `Type.toString`: a module's
    /// `TermRef(<this of its owner>,object <name>)`, a class's `TypeRef(<this of its owner>,class <name>)`, an owner's
    /// `ThisType(TypeRef(<its owner's>,module class <name>))`, a top-level package's owner `NoPrefix`.
    fn type_string(&mut self, ix: FileIx, t: &TType) -> String {
        let Some(s) = self.g.resolve(ix, t) else { return "?".to_string() };
        let sym = self.g.sym(s).clone();
        let prefix = sym.owner.map_or("NoPrefix".to_string(), |o| self.this_string(o));
        if sym.term {
            format!("TermRef({},object {})", prefix, sym.name)
        } else {
            format!("TypeRef({},class {})", prefix, sym.name)
        }
    }

    fn this_string(&mut self, c: SymId) -> String {
        let s = self.g.sym(c).clone();
        if c == self.g.root {
            return "NoPrefix".to_string();
        }
        let prefix = match s.owner {
            Some(o) if o != self.g.root => self.this_string(o),
            _ => "NoPrefix".to_string(),
        };
        let kind = if s.package || s.flags.has(DefFlags::MODULE) { "module class" } else { "class" };
        format!("ThisType(TypeRef({},{} {}))", prefix, kind, s.name)
    }

    /// `prefixIsElidable`'s third case: `C.this.x` of a parameter or parameter accessor `x` of `C`, not a trait, read
    /// in `C` itself (`ctx.owner.enclosingClass == pre.cls`).
    fn own_param_ref(&mut self, ix: FileIx, n: &Node) -> bool {
        let Some(q) = n.children.first().filter(|q| q.kind == Kind::This) else { return false };
        let Some(c) = q.ty.as_ref().and_then(|t| self.g.resolve(ix, t)) else { return false };
        if self.g.sym(c).flags.has(DefFlags::TRAIT) || self.enclosing.last() != Some(&c) {
            return false;
        }
        match self.g.tree_sym(ix, n) {
            Some(s) => {
                let f = self.g.sym(s).flags;
                f.has(DefFlags::PARAM) || f.has(DefFlags::PARAM_ACCESSOR)
            }
            None => false,
        }
    }

    /// A reference tree's symbol with its flags, `?` where the graph cannot name it.
    fn sym_flags_text(&mut self, ix: FileIx, n: &Node) -> String {
        match self.g.tree_sym(ix, n) {
            Some(s) => {
                let flags = self.g.sym(s).flags;
                format!("{}{}", self.g.sym_text(s), flags_text(flags))
            }
            None => "?".to_string(),
        }
    }

    /// An `Ident`'s prefix as the reference's dump classifies it: `none` (no prefix), `this(C)`, `term(p)`, `other`,
    /// `constant`, `-` (not a term reference).
    fn pre_text(&mut self, ix: FileIx, ty: &Option<TType>) -> String {
        let Some(ty) = ty else { return "?".to_string() };
        let file = self.file(ix);
        match ty {
            TType::LocalTerm(_, None) => "none".to_string(),
            TType::LocalTerm(_, Some(p)) | TType::TermRef(p, _) => match &**p {
                TType::This(c) => match self.g.resolve(ix, c) {
                    Some(s) => format!("this({})", self.g.sym_text(s)),
                    None => "this(?)".to_string(),
                },
                // `TERMREFpkg`: the package's own `TermRef`, a term prefix (`ThisType` of it is `THIS`).
                TType::Package(n) => {
                    let full = file.name(*n);
                    if full == "_root_" { "this(<root>)".to_string() } else { format!("term({})", full) }
                }
                TType::TermRef(..) | TType::LocalTerm(..) => match self.g.resolve(ix, p) {
                    Some(s) => format!("term({})", self.g.sym_text(s)),
                    None => "term(?)".to_string(),
                },
                _ => "other".to_string(),
            },
            TType::Package(n) => {
                let full = file.name(*n);
                match full.rsplit_once('.') {
                    Some((parent, _)) => format!("this({})", parent),
                    None => "this(<root>)".to_string(),
                }
            }
            TType::Const(_) => "constant".to_string(),
            _ => "-".to_string(),
        }
    }

    /// The symbol a definition at `n` enters.
    fn def_text(&mut self, ix: FileIx, n: &Node) -> String {
        // A self definition: its self symbol, with `SelfSymFlags` (private, local, deferred), `_` among the names
        // (`this: T =>`); a module class's `_` has none, its self type the module's (`Namer`'s `selfInfo`).
        if n.selfdef {
            let file = self.file(ix);
            let name = n.name.map(|nm| name_text(&file, nm));
            let class = self.enclosing.last().copied();
            let module = class.is_some_and(|c| self.g.sym(c).flags.has(DefFlags::MODULE));
            return match (name, class) {
                (Some(name), Some(c)) if !(module && name == "_") => format!(" sym={}.{} {{deferred,private,local}}", self.g.sym_text(c), name),
                _ => " sym=-".to_string(),
            };
        }
        match self.g.defined_at(ix, n.at) {
            Some(s) => {
                let flags = self.g.sym(s).flags;
                format!(" sym={}{}", self.g.sym_text(s), flags_text(flags))
            }
            None => " sym=?".to_string(),
        }
    }

    /// Whether a class named `name` is pickled with its name at `at` (its `TypeDef`'s point), and the innermost pickled
    /// definition whose span holds `at`.
    fn find_place(&mut self, ix: FileIx, n: &Node, at: u32, name: &str, pickled: &mut bool, owner: &mut Option<(FileIx, Node, u32)>) {
        let span = self.g.files[ix as usize].spans.get(n.addr).and_then(|p| p.pos);
        if let Some(p) = span {
            if n.is_class_def() && p.point.unwrap_or(p.start) == at {
                let file = self.file(ix);
                // An anonymous class: the typer's binary name (`C$$anon$..`), the pickle's `$anon`.
                if n.name.is_some_and(|nm| {
                    let pickled = name_text(&file, nm);
                    pickled.trim_end_matches('$') == name || (name.contains("$anon") && pickled == "$anon")
                }) {
                    *pickled = true;
                }
            }
            // An enum's singleton case (`case White`): a value of its companion in dotty (`Desugar.enumCase`), no
            // class, which the typed program holds as one: its value at the place is what there is.
            if n.kind == Kind::ValDef && n.flags.has(DefFlags::ENUM) && p.point.unwrap_or(p.start) == at {
                let file = self.file(ix);
                if n.name.is_some_and(|nm| name_text(&file, nm) == name) {
                    *pickled = true;
                }
            }
            let def = matches!(n.kind, Kind::DefDef | Kind::ValDef | Kind::TypeDef);
            if def && p.start <= at && at <= p.end {
                let width = p.end - p.start;
                if owner.as_ref().map_or(true, |(_, _, w)| width <= *w) {
                    *owner = Some((ix, n.clone(), width));
                }
            }
        }
        for c in &n.children {
            self.find_place(ix, c, at, name, pickled, owner);
        }
    }

    /// A class of the class path by its full name: its definition tree, under `deftree <name>`.
    fn deftree(&mut self, name: &str) {
        let (pkg, simple) = name.rsplit_once('.').unwrap_or(("", name));
        let found = self.g.library_class(pkg, simple, false).and_then(|c| self.g.sym(c).def);
        let Some((ix, addr)) = found else {
            self.out.push_str(&format!("deftree {} missing\n", name));
            return;
        };
        let node = find_node(&self.g.files[ix as usize].nodes, addr).cloned();
        match node {
            Some(n) => {
                self.out.push_str(&format!("deftree {}\n", name));
                // Printed in the class's own source (the printer names a source at its first position).
                let mut tree = std::mem::take(&mut self.out);
                self.node(ix, &n, 1);
                std::mem::swap(&mut self.out, &mut tree);
                self.out.push_str(&tree);
            }
            None => self.out.push_str(&format!("deftree {} empty\n", name)),
        }
    }

    /// One unit: `unit <path>`, its classes, its trees.
    fn unit(&mut self, path: &str, parts: &[FileIx]) {
        // One `PackageDef` a package clause, as the reference's unit holds it: the clause of the first part that has
        // one, its statements those of every part's clause, in the order of their spans. The clause is the one of
        // the part whose statements the source has first, which holds the unit's package id as the unit's own
        // `PackageDef` does: each part's clause has it before its own first statement.
        let mut stats: Vec<(u32, usize, FileIx, Node)> = Vec::new();
        let mut first: Option<(u32, FileIx, Node)> = None;
        for (k, &ix) in parts.iter().enumerate() {
            let l = &self.g.files[ix as usize];
            let start = |n: &Node| l.spans.get(n.addr).and_then(|p| p.pos).map_or(u32::MAX, |p| p.start);
            for n in &l.nodes {
                if n.kind == Kind::PackageDef {
                    let lead = n.children.iter().skip(1).map(|s| start(s)).min().unwrap_or(u32::MAX);
                    if first.as_ref().map_or(true, |(f, _, _)| lead < *f) {
                        first = Some((lead, ix, n.clone()));
                    }
                    for s in n.children.iter().skip(1) {
                        stats.push((start(s), k, ix, s.clone()));
                    }
                } else {
                    stats.push((start(n), k, ix, n.clone()));
                }
            }
        }
        // A clause's statements in the source's order, then what the compiler adds after them: the top-level
        // definitions' wrapper (`Desugar.packageDef`'s `topStats :+ grouped`, the synthetic `<file>$package` module),
        // then the `@main` proxies (`Typer.typedPackageDef`'s `stats1 ++ MainProxies.proxies(stats1)`, a class
        // `Final | Invisible`). No pickle carries that order: each top-level class is its own TASTy file.
        let added = |ix: FileIx, n: &Node| -> u8 {
            let file = &self.g.files[ix as usize].file;
            let module = n.flags.has(DefFlags::MODULE) && n.flags.has(DefFlags::SYNTHETIC);
            let name = n.name.map(|m| name_text(file, m)).unwrap_or_default();
            if module && (name.ends_with("$package") || name.ends_with("$package$")) {
                1
            } else if n.kind == Kind::TypeDef && n.flags.has(DefFlags::INVISIBLE) {
                2
            } else {
                0
            }
        };
        stats.sort_by_key(|(start, k, ix, n)| (added(*ix, n), *start, *k));
        // The classes, over the merged walk.
        let mut classes = Vec::new();
        for (_, _, ix, s) in &stats {
            classes_post_order(*ix, std::slice::from_ref(s), &mut classes);
        }
        let mut head = format!("unit {}\n", self.root.join(path).display());
        for (ix, c) in classes {
            let text = match self.g.defined_at(ix, c.at) {
                Some(s) => self.g.sym_text(s),
                None => "?".to_string(),
            };
            let concrete = c.flags.has(DefFlags::MODULE) || !(c.flags.has(DefFlags::ABSTRACT) || c.flags.has(DefFlags::TRAIT));
            // `Util.isStaticObject`: a module class that is static (owned by a package or a static module class).
            let static_object = c.flags.has(DefFlags::MODULE) && self.g.defined_at(ix, c.at).is_some_and(|s| self.g.is_static(s));
            head.push_str(&format!("class {} concrete={} staticObject={}\n", text, concrete, static_object));
        }
        // The typed classes with no pickled definition at their place: under the innermost pickled definition whose
        // span holds them, `unavailable=withheld` (F1).
        let typed = self.inventory.get(path).or_else(|| {
            let file = std::path::Path::new(path).file_name()?.to_str()?.to_string();
            self.inventory.iter().find(|(k, _)| std::path::Path::new(k).file_name().and_then(|f| f.to_str()) == Some(file.as_str())).map(|(_, v)| v)
        }).cloned().unwrap_or_default();
        // The typed program's offsets count the source's bytes, a pickle's its UTF-16 units: converted by the text.
        let text = std::fs::read_to_string(self.root.join(path)).unwrap_or_default();
        for (name, at) in typed {
            let at = text.get(..at as usize).map_or(at, |t| t.encode_utf16().count() as u32);
            let mut pickled = false;
            let mut owner: Option<(FileIx, Node, u32)> = None;
            for (_, _, ix, s) in &stats {
                self.find_place(*ix, s, at, &name, &mut pickled, &mut owner);
            }
            if pickled {
                continue;
            }
            let owner_text = match owner.and_then(|(ix, n, _)| self.g.defined_at(ix, n.at)) {
                Some(o) => self.g.sym_text(o),
                None => "?".to_string(),
            };
            head.push_str(&format!("class {}.{} unavailable=withheld\n", owner_text, name));
        }
        let mut tree = std::mem::take(&mut self.out);
        let mut depth = 0;
        if let Some((_, ix, pkg)) = first {
            let pos = self.pos(ix, &pkg);
            self.out.push_str(&format!("PackageDef {}\n", pos));
            if let Some(pid) = pkg.children.first() {
                self.node(ix, pid, 1);
            }
            depth = 1;
        }
        for (_, _, ix, s) in &stats {
            self.node(*ix, s, depth);
        }
        std::mem::swap(&mut self.out, &mut tree);
        self.out.push_str(&head);
        self.out.push_str(&tree);
    }

    fn node(&mut self, ix: FileIx, n: &Node, depth: usize) {
        if n.kind == Kind::Import || n.kind == Kind::Export {
            return;
        }
        let indent = "  ".repeat(depth);
        let pos = self.pos(ix, n);
        if n.kind == Kind::Tpt {
            let mut found = Vec::new();
            if let Some(t) = &n.ty {
                self.paths(ix, t, &mut found);
            }
            self.out.push_str(&format!("{}Tpt {} paths=[{}]\n", indent, pos, found.join(",")));
            return;
        }
        let file = self.file(ix);
        // A path `tpd.ref` makes an `Ident` of, its prefix elidable: printed as that `Ident`, its prefix in `pre`.
        if n.kind == Kind::Select && n.path && (self.g.prefix_elidable(ix, n) || self.own_param_ref(ix, n)) {
            let sym = self.sym_flags_text(ix, n);
            let pre = match n.children.first() {
                Some(q) if q.kind == Kind::This => match q.ty.as_ref().and_then(|t| self.g.resolve(ix, t)) {
                    Some(c) => format!("this({})", self.g.sym_text(c)),
                    None => "this(?)".to_string(),
                },
                Some(q) => match self.g.tree_sym(ix, q) {
                    Some(p) => format!("term({})", self.g.sym_text(p)),
                    None => "term(?)".to_string(),
                },
                None => "?".to_string(),
            };
            self.out.push_str(&format!("{}Ident {} sym={} pre={}\n", indent, pos, sym, pre));
            return;
        }
        let facts = match n.kind {
            Kind::ValDef | Kind::DefDef | Kind::TypeDef | Kind::Bind => self.def_text(ix, n),
            // `_root_` as the unpickler reads `TERMREFpkg _root_`: the root package's `TermRef`, no prefix.
            Kind::Ident if matches!(&n.ty, Some(TType::Package(p)) if file.name(*p) == "_root_") => {
                " sym=_root_ {package,module,final,java} pre=none".to_string()
            }
            Kind::Ident => {
                let sym = self.sym_flags_text(ix, n);
                format!(" sym={} pre={}", sym, self.pre_text(ix, &n.ty))
            }
            Kind::Select => {
                let sym = match self.g.tree_sym(ix, n) {
                    Some(_) => self.sym_flags_text(ix, n),
                    None => {
                        let name = n.name.map(|nm| format!("{}{}", name_text(&file, nm), signed(&file, nm).unwrap_or_default())).unwrap_or_default();
                        format!("?.{}", name)
                    }
                };
                // A `this` qualifier: its type's class, its self type's meet with the class (`-` for none).
                let this_class = n.children.first().filter(|q| q.kind == Kind::This).and_then(|q| self.g.tree_class(ix, q)).map(|c| self.g.this_class(c));
                let qual = match this_class {
                    Some(Some(c)) => self.g.sym_text(c),
                    Some(None) => "-".to_string(),
                    None => match n.children.first().and_then(|q| self.g.tree_class(ix, q)) {
                        Some(c) => self.g.sym_text(c),
                        None => "?".to_string(),
                    },
                };
                let methodic = match self.g.tree_sym(ix, n) {
                    Some(s) if self.g.sym(s).flags.has(DefFlags::METHOD) => " methodic",
                    _ => "",
                };
                format!(" sym={} qual={}{}", sym, qual, methodic)
            }
            Kind::This => format!(" this={}", self.ref_text(ix, &n.ty)),
            // `Util.typeRefOf(n.tpe).symbol`: the class, an alias dealiased (`scala.IndexOutOfBoundsException`).
            Kind::New => match self.g.tree_class(ix, n) {
                Some(c) => format!(" new={}", self.g.sym_text(c)),
                None => format!(" new={}", self.ref_text(ix, &n.ty)),
            },
            Kind::Literal => match &n.ty {
                Some(TType::Const(Const::Class(t))) => format!(" const=12:{}", self.type_string(ix, t)),
                Some(TType::Const(c)) => format!(" const={}", const_text(&file, c)),
                _ => String::new(),
            },
            Kind::Apply => {
                let fun = n.children.first();
                let s = fun.and_then(|f| self.g.tree_sym(ix, f));
                // Which clause this application passes: as many applications as the function holds below it.
                let mut k = 0;
                let mut f = fun;
                while let Some(x) = f {
                    match x.kind {
                        Kind::Apply => {
                            k += 1;
                            f = x.children.first();
                        }
                        Kind::TypeApply => f = x.children.first(),
                        _ => break,
                    }
                }
                let margs = s.and_then(|s| self.g.margs(s, k)).unwrap_or_else(|| "?".to_string());
                format!(" fn={} margs={}", s.map_or("?".to_string(), |s| self.g.sym_text(s)), margs)
            }
            Kind::TypeApply => {
                let s = n.children.first().and_then(|f| self.g.tree_sym(ix, f));
                format!(" fn={}", s.map_or("?".to_string(), |s| self.g.sym_text(s)))
            }
            Kind::Return => match n.target.and_then(|a| self.g.defined_at(ix, a)) {
                Some(s) => format!(" from={}", self.g.sym_text(s)),
                None => " from=?".to_string(),
            },
            Kind::Inlined => match &n.call {
                Some(c) => {
                    let cp = self.pos(ix, c);
                    format!(" call=? callpos={}", cp)
                }
                None => " call=-".to_string(),
            },
            // `e: @unchecked`: the type tree's type annotated with `scala.unchecked`.
            Kind::Typed => match n.children.get(1).and_then(|t| t.ty.clone()) {
                Some(TType::Annotated(_, a)) if self.g.resolve(ix, &a).is_some_and(|c| self.g.full_name(c) == "scala.unchecked") => " unchecked".to_string(),
                _ => String::new(),
            },
            Kind::Closure => {
                let s = n.children.first().and_then(|f| self.g.tree_sym(ix, f));
                format!(" meth={}", s.map_or("?".to_string(), |s| self.g.sym_text(s)))
            }
            Kind::UnApply => {
                let s = n.children.first().and_then(|f| self.g.tree_sym(ix, f));
                format!(" fun={}", s.map_or("?".to_string(), |s| self.g.sym_text(s)))
            }
            Kind::Elided => " sym=_ pre=-".to_string(),
            _ => String::new(),
        };
        // `throw e`: `TreeUnpickler`'s `ref(defn.throwMethod).appliedTo(e)`, the method's `Ident` at the tree's place.
        if n.kind == Kind::Throw {
            let throw = "<special-ops>.throw#(java.lang.Throwable)scala.Nothing";
            self.out.push_str(&format!("{}Apply {} fn={} margs=[v]\n", indent, pos, throw));
            // The `Ident`, unpositioned, takes the empty span at its next sibling's start (`Positioned.setChildSpans`):
            // the thrown expression's.
            let ident_pos = match n.children.first() {
                Some(c) => {
                    let cp = self.pos(ix, c);
                    let re_start = cp.split(' ').nth(1).and_then(|r| r.split("..").next()).map(|x| x.to_string());
                    let lc = cp.split(' ').nth(2).and_then(|x| x.split('-').next()).map(|x| x.to_string());
                    match (cp.split(' ').next(), re_start, lc) {
                        (Some(src), Some(st), Some(lc)) => format!("{} {}..{}@{} {}-{}]", src, st, st, st, lc, lc.trim_end_matches(']')),
                        _ => pos.clone(),
                    }
                }
                None => pos.clone(),
            };
            self.out.push_str(&format!("{}  Ident {} sym={} {{method}} pre=this(<special-ops>)\n", indent, ident_pos, throw));
            for c in &n.children {
                self.node(ix, c, depth + 1);
            }
            return;
        }
        self.out.push_str(&format!("{}{} {}{}\n", indent, n.kind.name(), pos, facts));
        let class = if n.is_class_def() { self.g.defined_at(ix, n.at) } else { None };
        if let Some(c) = class {
            self.enclosing.push(c);
        }
        for c in &n.children {
            self.node(ix, c, depth + 1);
        }
        if class.is_some() {
            self.enclosing.pop();
        }
        // `return e`: `TreeUnpickler`'s `Return(expr, Ident(from.termRef))`. The `Ident`, unpositioned and the last
        // child, takes the empty span at the `Return`'s end (`Positioned.setChildSpans`); its prefix is
        // `from.termRef`'s, the owner class's `this` or none for a local method.
        if n.kind == Kind::Return {
            if let Some(s) = n.target.and_then(|a| self.g.defined_at(ix, a)) {
                let parts: Vec<&str> = pos.trim_start_matches('[').trim_end_matches(']').split(' ').collect();
                let end_pos = match parts.as_slice() {
                    [src, range, lc] => {
                        let end = range.split('@').next().and_then(|r| r.split("..").nth(1)).unwrap_or("?");
                        let lc_end = lc.split('-').nth(1).unwrap_or("?");
                        format!("[{} {}..{}@{} {}-{}]", src, end, end, end, lc_end, lc_end)
                    }
                    _ => pos.clone(),
                };
                let owner = self.g.sym(s).owner;
                let pre = match owner {
                    Some(o) if self.g.sym(o).class => format!("this({})", self.g.sym_text(o)),
                    _ => "none".to_string(),
                };
                let flags = self.g.sym(s).flags;
                let text = self.g.sym_text(s);
                self.out.push_str(&format!("{}  Ident {} sym={}{} pre={}\n", indent, end_pos, text, flags_text(flags), pre));
            }
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
/// found under `root` (the `-sourceroot` the files were written with), the class path's classes read from the jars
/// `TEQ_INIT_VIEW_CLASSPATH` names (`:`-separated).
pub fn run(dir: &str, root: &str) -> Result<String, String> {
    let classpath: Vec<String> = std::env::var("TEQ_INIT_VIEW_CLASSPATH")
        .map(|c| c.split(':').filter(|s| !s.is_empty()).map(|s| s.to_string()).collect())
        .unwrap_or_default();
    let mut g = Graph::new(&classpath);
    let mut paths = Vec::new();
    tasty_files(std::path::Path::new(dir), &mut paths);
    let mut units: Vec<(String, Vec<FileIx>)> = Vec::new();
    for p in &paths {
        let bytes = std::fs::read(p).map_err(|e| format!("{}: {}", p.display(), e))?;
        let file = TastyFile::parse(bytes).map_err(|e| format!("{}: {}", p.display(), e))?;
        let ix = g.add_file(file);
        let l = &g.files[ix as usize];
        let path = l.section.as_ref().and_then(|s| s.primary()).map(|n| l.file.name(n)).unwrap_or_default();
        match units.iter_mut().find(|u| u.0 == path) {
            Some(u) => u.1.push(ix),
            None => units.push((path, vec![ix])),
        }
    }
    units.sort_by(|a, b| a.0.cmp(&b.0));
    // The typed program's classes (TEQ_INIT_VIEW_INVENTORY, written by the compile that made the products): a class
    // with no pickled definition at its place is in a body the writer withheld, printed `unavailable`, never absent.
    let mut inventory: std::collections::HashMap<String, Vec<(String, u32)>> = std::collections::HashMap::new();
    if let Some(file) = std::env::var_os("TEQ_INIT_VIEW_INVENTORY").filter(|f| !f.is_empty()) {
        if let Ok(text) = std::fs::read_to_string(&file) {
            for l in text.lines() {
                let f: Vec<&str> = l.split('\t').collect();
                if let [_, path, name, at] = f[..] {
                    if let Ok(at) = at.parse::<u32>() {
                        let rows = inventory.entry(path.to_string()).or_default();
                        if !rows.iter().any(|r| r.0 == name && r.1 == at) {
                            rows.push((name.to_string(), at));
                        }
                    }
                }
            }
        }
    }
    let mut sources = Sources::default();
    let mut out = String::new();
    for (path, parts) in &units {
        let mut p = Printer { out: String::new(), sources: &mut sources, root: std::path::PathBuf::from(root), g: &mut g, enclosing: Vec::new(), inventory: &inventory };
        p.unit(path, parts);
        out.push_str(&p.out);
    }
    // The library members the evaluation follows (TEQ_INIT_VIEW_MEMBERS, a file of class names, InitDump's
    // INITDUMP_MEMBERS): each class's definition tree, under `deftree <name>`, printed as a unit's tree is.
    if let Some(file) = std::env::var_os("TEQ_INIT_VIEW_MEMBERS").filter(|f| !f.is_empty()) {
        let list = std::fs::read_to_string(&file).map_err(|e| format!("{}: {}", std::path::Path::new(&file).display(), e))?;
        for name in list.lines().map(str::trim).filter(|n| !n.is_empty() && !n.starts_with('#')) {
            let mut p = Printer { out: String::new(), sources: &mut sources, root: std::path::PathBuf::from(root), g: &mut g, enclosing: Vec::new(), inventory: &inventory };
            p.deftree(name);
            out.push_str(&p.out);
        }
    }
    Ok(out)
}
