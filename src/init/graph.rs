//! The checker's symbols: entered from the definitions of every TASTy file the view opens, the program's pickles and
//! the class path's alike, each with its exact owner, its flags as the reference's unpickler gives them and the
//! address of its tree (`defTree`). A synthetic, artifact or local definition is entered like any other: the
//! reference's checker reads the synthesized members' bodies (a case class's `toString`), so nothing is filtered
//! (the loader's tables leave the supplied synthetics out on purpose, so they are not this graph's source).
//!
//! A class of the class path is entered when a reference first names it: its TASTy file is read from the jars
//! (`<package path>/<top-level name>.tasty`) and all its definitions are entered. A class with no TASTy (a Java class
//! of the JDK) is entered as it is referred to: its members as the references name them, `JavaDefined`, no tree.

use super::view::{DefFlags, Kind, Node, ViewReader};
use crate::tasty::positions::{Section, Spans};
use crate::tasty::tree::{Addr, Const, TType};
use crate::tasty::{TName, TastyFile};
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct SymId(pub u32);

/// A file the graph has read: its trees as the view reads them and their spans.
pub struct Loaded {
    pub file: Rc<TastyFile>,
    pub nodes: Vec<Node>,
    pub spans: Spans,
    pub section: Option<Section>,
}

pub type FileIx = u32;

#[derive(Clone, Debug)]
pub struct Sym {
    /// The name as the reference prints `Name.toString`: a module class's with its `$`.
    pub name: String,
    pub owner: Option<SymId>,
    pub flags: DefFlags,
    /// A term (a value, a method, a package, a module) or a type (a class, an alias, a type parameter).
    pub term: bool,
    pub class: bool,
    pub package: bool,
    /// Where the definition's tree is: the file and its address.
    pub def: Option<(FileIx, Addr)>,
    /// A class's or a package's members, in the order they are entered.
    pub decls: Vec<SymId>,
    /// A method's signature as the reference's dump prints it, `#(params)result`, once computed.
    pub sig: Option<String>,
}

pub struct Graph {
    pub files: Vec<Loaded>,
    pub syms: Vec<Sym>,
    by_def: HashMap<(FileIx, Addr), SymId>,
    packages: HashMap<String, SymId>,
    jars: Vec<crate::zip::Zip>,
    /// The class path's files read, by their entry name, so that a file is read once.
    read_entries: HashMap<String, Option<FileIx>>,
    pub root: SymId,
    /// The classes whose parents are being computed: a parent that leads back to one is not followed again (a
    /// member looked up through the class's own parents while they resolve).
    parents_in_progress: Vec<SymId>,
    /// The aliases being expanded (`ty_of`).
    expanding: Vec<SymId>,
    /// The classes a `base_args` query is walking (a path), and its results by class and type arguments: each class
    /// with given arguments is walked once a query, however many paths reach it.
    base_visiting: Vec<SymId>,
    base_memo: HashMap<(SymId, Vec<Ty>), Option<Vec<Ty>>>,
    /// How deep the current `base_args` query is (0: none open, its visited set cleared on entry).
    base_depth: u32,
    /// Each class's linearization, computed once (dotty's `baseData` cache, SymDenotations.scala 2091).
    lin_cache: HashMap<SymId, Vec<SymId>>,


}

/// The name of a definition's `NameRef` as the reference prints it.
pub fn name_text(file: &TastyFile, n: u32) -> String {
    let text = match file.names.get(n as usize) {
        Some(TName::Signed { original, .. }) => return name_text(file, *original),
        Some(TName::ObjectClass(u)) => format!("{}$", file.name(*u)),
        // A prefix kind of an expanded name (`super$` of `M$$foo`): the prefix on its last part, `M$$super$foo`
        // (`PrefixNameKind.mkString`'s `qualToString`).
        Some(TName::SuperAccessor(u)) | Some(TName::InlineAccessor(u)) => {
            let prefix = if matches!(file.names.get(n as usize), Some(TName::SuperAccessor(_))) { "super$" } else { "inline$" };
            match file.names.get(*u as usize) {
                Some(TName::Expanded(a, b)) => format!("{}$${}{}", file.name(*a), prefix, file.name(*b)),
                _ => file.name(n),
            }
        }
        _ => file.name(n),
    };
    // A constructor's default getter: `DefaultGetterName` of `<init>` is `$lessinit$greater$default$N`
    // (`nme.DEFAULT_GETTER_INIT`).
    match text.strip_prefix("<init>$default$") {
        Some(k) => format!("$lessinit$greater$default${}", k),
        None => text,
    }
}

/// A signed name's signature text, `#(params)result`.
pub fn signed(file: &TastyFile, n: u32) -> Option<String> {
    match file.names.get(n as usize) {
        Some(TName::Signed { params, result, .. }) => {
            let ps: Vec<String> = params.iter().map(|&p| if p < 0 { (-p).to_string() } else { file.name(p as u32) }).collect();
            Some(format!("#({}){}", ps.join(","), file.name(*result)))
        }
        _ => None,
    }
}

impl Graph {
    pub fn new(classpath: &[String]) -> Graph {
        let mut g = Graph {
            files: Vec::new(),
            syms: Vec::new(),
            by_def: HashMap::new(),
            packages: HashMap::new(),
            jars: Vec::new(),
            read_entries: HashMap::new(),
            parents_in_progress: Vec::new(),
            expanding: Vec::new(),
            base_visiting: Vec::new(),
            base_depth: 0,
            base_memo: HashMap::new(),
            lin_cache: HashMap::new(),
            root: SymId(0),
        };
        g.root = g.push(Sym { name: "<root>".into(), owner: None, flags: DefFlags(DefFlags::PACKAGE), term: true, class: false, package: true, def: None, decls: Vec::new(), sig: None });
        g.packages.insert(String::new(), g.root);
        for p in classpath {
            if let Ok(z) = crate::zip::Zip::open(p) {
                g.jars.push(z);
            }
        }
        g
    }

    fn push(&mut self, s: Sym) -> SymId {
        let id = SymId(self.syms.len() as u32);
        if let Some(o) = s.owner {
            self.syms[o.0 as usize].decls.push(id);
        }
        self.syms.push(s);
        id
    }

    pub fn sym(&self, id: SymId) -> &Sym {
        &self.syms[id.0 as usize]
    }

    /// The package of a full name (`a.b`), entered as needed; the empty name is the root, `<empty>` the empty package.
    pub fn package(&mut self, full: &str) -> SymId {
        if let Some(&p) = self.packages.get(full) {
            return p;
        }
        let (owner, last) = match full.rfind('.') {
            Some(i) => (self.package(&full[..i]), &full[i + 1..]),
            None => (self.root, full),
        };
        let id = self.push(Sym {
            name: last.to_string(),
            owner: Some(owner),
            flags: DefFlags(DefFlags::PACKAGE | DefFlags::MODULE | DefFlags::LAZY | DefFlags::FINAL | DefFlags::JAVA | DefFlags::STABLE),
            term: true,
            class: false,
            package: true,
            def: None,
            decls: Vec::new(),
            sig: None,
        });
        self.packages.insert(full.to_string(), id);
        id
    }

    /// A package's full name.
    pub fn full_name(&self, id: SymId) -> String {
        let s = self.sym(id);
        match s.owner {
            None => s.name.clone(),
            Some(o) if o == self.root => s.name.clone(),
            Some(o) => format!("{}.{}", self.full_name(o), s.name),
        }
    }

    /// Reads a file into the graph and enters its definitions.
    pub fn add_file(&mut self, file: TastyFile) -> FileIx {
        let file = Rc::new(file);
        let ix = self.files.len() as FileIx;
        let (nodes, spans, section) = {
            let mut v = ViewReader::new(&file);
            let nodes = v.top_level();
            let ViewReader { spans, section, .. } = v;
            (nodes, spans, section)
        };
        self.files.push(Loaded { file: file.clone(), nodes, spans, section });
        let nodes = std::mem::take(&mut self.files[ix as usize].nodes);
        for n in &nodes {
            self.enter(ix, &file, n, self.root);
        }
        self.files[ix as usize].nodes = nodes;
        ix
    }

    /// Enters the definitions of a tree, owned by `owner`.
    fn enter(&mut self, ix: FileIx, file: &TastyFile, n: &Node, owner: SymId) {
        let mut inner = owner;
        match n.kind {
            Kind::PackageDef => {
                if let Some(TType::Package(p)) = n.children.first().and_then(|c| c.ty.as_ref()) {
                    inner = self.package(&file.name(*p));
                }
            }
            Kind::ValDef | Kind::DefDef | Kind::TypeDef | Kind::Bind if n.name.is_some() => {
                let class = n.is_class_def();
                let mut name = name_text(file, n.name.unwrap());
                if class && n.flags.has(DefFlags::MODULE) && !name.ends_with('$') {
                    name.push('$');
                }
                let term = !matches!(n.kind, Kind::TypeDef) && !(n.kind == Kind::Bind && matches!(n.ty, Some(TType::Bounds(..))));
                let id = self.push(Sym { name, owner: Some(owner), flags: n.flags, term, class, package: false, def: Some((ix, n.at)), decls: Vec::new(), sig: None });
                self.by_def.insert((ix, n.at), id);
                inner = id;
            }
            _ => {}
        }
        if n.kind == Kind::Template {
            // `readTemplate`: the parents' trees, after the constructor, are the constructor's (the arguments a parent
            // call lifts, `i$1`); the body's statements are its local dummy's (`<local C>`), its definitions the
            // class's.
            let mut ctor: Option<SymId> = None;
            let mut dummy: Option<SymId> = None;
            for (i, c) in n.children.iter().enumerate() {
                let def = matches!(c.kind, Kind::DefDef | Kind::ValDef | Kind::TypeDef | Kind::Import | Kind::Export);
                let o = if def {
                    inner
                } else if n.parents.contains(&i) {
                    ctor.unwrap_or(inner)
                } else {
                    *dummy.get_or_insert_with(|| {
                        let name = format!("<local {}>", self.sym(inner).name);
                        self.push(Sym { name, owner: Some(inner), flags: DefFlags::default(), term: true, class: false, package: false, def: None, decls: Vec::new(), sig: None })
                    })
                };
                self.enter(ix, file, c, o);
                if c.kind == Kind::DefDef && ctor.is_none() && c.name.is_some_and(|nm| name_text(file, nm) == "<init>") {
                    ctor = self.defined_at(ix, c.at);
                }
            }
        } else {
            for c in &n.children {
                self.enter(ix, file, c, inner);
            }
        }
        if let Some(call) = &n.call {
            self.enter(ix, file, call, inner);
        }
    }

    /// The symbol defined at an address of a file.
    pub fn defined_at(&self, ix: FileIx, addr: Addr) -> Option<SymId> {
        self.by_def.get(&(ix, addr)).copied()
    }

    /// A member of a class or package by name, in the term or the type namespace, with a signature when given.
    pub fn decl(&mut self, owner: SymId, name: &str, term: bool, sig: Option<&str>) -> Option<SymId> {
        let decls = self.sym(owner).decls.clone();
        let mut named = Vec::new();
        for d in decls {
            let s = self.sym(d);
            if s.name == name && s.term == term {
                match sig {
                    Some(want) => {
                        if self.signature(d).as_deref() == Some(want) {
                            return Some(d);
                        }
                        named.push(d);
                    }
                    None => named.push(d),
                }
            }
        }
        // An unsigned name: a value, or a method with no parameter clause (`NotAMethod`), before any overload that
        // takes parameters.
        if sig.is_none() {
            for &d in &named {
                if !self.sym(d).flags.has(DefFlags::METHOD) || self.signature(d).as_deref() == Some("#()") {
                    return Some(d);
                }
            }
            return named.first().copied();
        }
        // No declaration of the signature: the one of the name where there is one, none among overloads (the
        // signature this graph computes may differ from the reference's; an overload is no guess).
        if named.len() == 1 {
            return named.pop();
        }
        None
    }

    /// A class of the class path by its package and name, read from the jars on first use.
    pub fn library_class(&mut self, pkg: &str, name: &str, term: bool) -> Option<SymId> {
        let p = self.package(pkg);
        if let Some(found) = self.decl(p, name, term, None) {
            return Some(found);
        }
        // The file of a top-level class or object: the class's name, without an object's `$`.
        let stem = encode_name(name.trim_end_matches('$'));
        let entry = if pkg.is_empty() || pkg == "<empty>" { format!("{}.tasty", stem) } else { format!("{}/{}.tasty", pkg.replace('.', "/"), stem) };
        if !self.read_entries.contains_key(&entry) {
            let mut loaded = None;
            for j in 0..self.jars.len() {
                if let Some(i) = self.jars[j].find(&entry) {
                    if let Ok(bytes) = self.jars[j].read(i) {
                        if let Ok(f) = TastyFile::parse(bytes) {
                            loaded = Some(self.add_file(f));
                        }
                    }
                    break;
                }
            }
            self.read_entries.insert(entry, loaded);
        }
        self.decl(p, name, term, None)
    }

    /// The symbol a type names, in the file it was read from; `None` where the graph cannot name it.
    pub fn resolve(&mut self, ix: FileIx, ty: &TType) -> Option<SymId> {
        let file = self.files[ix as usize].file.clone();
        match ty {
            TType::Package(n) => {
                let full = file.name(*n);
                Some(if full == "_root_" { self.root } else { self.package(&full) })
            }
            TType::LocalType(a, _) | TType::LocalTerm(a, _) => self.defined_at(ix, *a),
            TType::This(c) => self.resolve(ix, c),
            TType::TypeRef(pre, n) | TType::TermRef(pre, n) => {
                let term = matches!(ty, TType::TermRef(..));
                let name = name_text(&file, *n);
                let sig = signed(&file, *n);
                let owner = self.resolve(ix, pre)?;
                let id = self.member(owner, &name, term, sig.as_deref())?;
                // A Java class's module class, named as one (`ObjectClass`): `SymbolLoaders.enterClassAndModule`
                // makes it `ModuleClass | Final | JavaDefined`, a static owner whose `this` `tpd.ref` elides
                // (`scala.runtime.Statics$`).
                let s = &self.syms[id.0 as usize];
                if !term && s.def.is_none() && s.flags.has(DefFlags::JAVA) && matches!(file.names.get(*n as usize), Some(TName::ObjectClass(_))) {
                    self.syms[id.0 as usize].flags.0 |= DefFlags::MODULE | DefFlags::FINAL;
                }
                Some(id)
            }
            TType::Applied(t, _) | TType::Annotated(t, _) => self.resolve(ix, t),
            // A class's type constructor eta-expanded (`New` of `[T] =>> C[T]`, its arguments a `TypeApply`'s): the class.
            TType::Lambda { result, .. } => self.resolve(ix, result),
            _ => None,
        }
    }

    /// A member of a class (its own, then its parents'), of a package (reading the class path), or of a term's type.
    pub fn member(&mut self, owner: SymId, name: &str, term: bool, sig: Option<&str>) -> Option<SymId> {
        let o = self.sym(owner).clone();
        if o.package {
            let full = self.full_name(owner);
            if let Some(d) = self.decl(owner, name, term, sig) {
                return Some(d);
            }
            return self.library_class(&full, name, term).or_else(|| self.java_member(owner, name, term, sig));
        }
        // A term that is a module's value: its members are its class's; another term (a path's prefix, `new i.Inner`):
        // its value's class's.
        let owner = if o.term && o.flags.has(DefFlags::MODULE) {
            self.module_class(owner).unwrap_or(owner)
        } else if o.term {
            self.value_class(owner).unwrap_or(owner)
        } else {
            owner
        };
        if let Some(d) = self.decl(owner, name, term, sig) {
            return Some(d);
        }
        // The base classes in linearization order (`baseClasses`): the first that declares the member.
        for p in self.linearization(owner).into_iter().skip(1) {
            if let Some(d) = self.decl(p, name, term, sig) {
                return Some(d);
            }
        }
        // A class with a self type (`self: MyCodes =>`): the members its `this` has through that type (`ThisType`'s
        // underlying is the self type): each self class's own and its base classes', not its self type's (a self type's
        // members do not compose: scalac finds no `h` of `S0` in `trait S3 { self: S2 & S1 => }` where `S1`'s self is
        // `S0`). One level, each self class's linearization computed once.
        for sc in self.self_classes(owner).into_iter().filter(|&sc| sc != owner) {
            if let Some(d) = self.decl(sc, name, term, sig) {
                return Some(d);
            }
            for p in self.linearization(sc).into_iter().skip(1) {
                if let Some(d) = self.decl(p, name, term, sig) {
                    return Some(d);
                }
            }
        }
        if self.sym(owner).def.is_none() {
            return self.java_member(owner, name, term, sig);
        }
        None
    }

    /// `classSymbol` of a class's `this`, whose type's underlying is its self type (`C & S`): the class deriving from
    /// all the others, or none (`NoSymbol`, two unrelated traits).
    pub fn this_class(&mut self, c: SymId) -> Option<SymId> {
        let mut out = c;
        for sc in self.self_classes(c) {
            if sc == out || self.linearization(out).contains(&sc) {
                continue;
            }
            if self.linearization(sc).contains(&out) {
                out = sc;
            } else {
                return None;
            }
        }
        Some(out)
    }

    /// The classes a class's written self type names: an intersection's sides.
    fn self_classes(&mut self, c: SymId) -> Vec<SymId> {
        let Some((ix, addr)) = self.sym(c).def else { return Vec::new() };
        let ty = find_node(&self.files[ix as usize].nodes, addr)
            .and_then(|n| n.children.first())
            .filter(|t| t.kind == Kind::Template)
            .and_then(|t| t.children.iter().find(|x| x.selfdef))
            .and_then(|sd| sd.children.first())
            .and_then(|tpt| tpt.ty.clone());
        let mut out = Vec::new();
        let mut todo: Vec<TType> = ty.into_iter().collect();
        while let Some(t) = todo.pop() {
            match t {
                TType::And(a, b) => {
                    todo.push(*a);
                    todo.push(*b);
                }
                TType::Annotated(a, _) => todo.push(*a),
                other => {
                    if let Some(k) = self.type_class(ix, &other) {
                        if !out.contains(&k) {
                            out.push(k);
                        }
                    }
                }
            }
        }
        out
    }

    /// A member a reference names in a class the graph has no TASTy of (a Java class): entered as it is named.
    fn java_member(&mut self, owner: SymId, name: &str, term: bool, sig: Option<&str>) -> Option<SymId> {
        let class = !term && !self.sym(owner).package || self.sym(owner).package && !term;
        // `java.lang.Object`'s members as `Definitions` enters them: its constructor a plain method, `eq`, `ne` and
        // `synchronized` final ones; any other Java member is `JavaDefined`.
        let owner_name = self.full_name(owner);
        // The classes `Definitions` enters with no TASTy (`enterCompleteClassSymbol` adds `Open`; `TransparentType`
        // prints `transparent`).
        // `scala.AnyRef`: `Definitions`' alias of `java.lang.Object` (`AnyRefAlias`), dealiased.
        if class && owner_name == "scala" && name == "AnyRef" {
            return self.library_class("java.lang", "Object", false);
        }
        if class && owner_name == "scala" {
            let flags = match name {
                "Any" => Some(DefFlags::ABSTRACT | DefFlags::OPEN | DefFlags::TRANSPARENT),
                "Matchable" => Some(DefFlags::TRAIT | DefFlags::OPEN | DefFlags::TRANSPARENT),
                "Nothing" | "Null" => Some(DefFlags::ABSTRACT | DefFlags::FINAL | DefFlags::OPEN),
                _ => None,
            };
            if let Some(f) = flags {
                let id = self.push(Sym { name: name.to_string(), owner: Some(owner), flags: DefFlags(f), term, class, package: false, def: None, decls: Vec::new(), sig: None });
                return Some(id);
            }
        }
        // `String`'s `+`: `Definitions`' `String_+`, final, no Java member.
        if owner_name == "java.lang.String" && name == "+" && term {
            let id = self.push(Sym { name: name.to_string(), owner: Some(owner), flags: DefFlags(DefFlags::METHOD | DefFlags::FINAL), term, class: false, package: false, def: None, decls: Vec::new(), sig: sig.map(|s| s.to_string()) });
            return Some(id);
        }
        let object = owner_name == "java.lang.Object";
        // `scala.Any`'s members as `Definitions` enters them (`AnyMethods`): no Java class, no TASTy.
        let any = owner_name == "scala.Any";
        let mut flags = if object || any { 0 } else { DefFlags::JAVA };
        if sig.is_some() {
            flags |= DefFlags::METHOD;
        }
        if object && matches!(name, "eq" | "ne" | "synchronized") {
            flags |= DefFlags::FINAL;
        }
        if any {
            flags |= DefFlags::METHOD;
            match name {
                "==" | "!=" | "##" | "getClass" | "isInstanceOf" | "asInstanceOf" => flags |= DefFlags::FINAL,
                "$isInstanceOf$" | "$asInstanceOf$" => flags |= DefFlags::FINAL | DefFlags::SYNTHETIC | DefFlags::ARTIFACT,
                _ => {}
            }
        }
        let id = self.push(Sym { name: name.to_string(), owner: Some(owner), flags: DefFlags(flags), term, class, package: false, def: None, decls: Vec::new(), sig: sig.map(|s| s.to_string()) });
        Some(id)
    }

    /// The symbol a reference tree names, as the reference's `tree.symbol`: an `Ident`'s by its type, a `Select`'s
    /// by its `SELECTin` owner or its qualifier's class and its name and signature (`TreeUnpickler`'s
    /// `qual.tpe.select(name)`), an application's by its function.
    pub fn tree_sym(&mut self, ix: FileIx, n: &Node) -> Option<SymId> {
        match n.kind {
            Kind::Ident => self.resolve(ix, n.ty.as_ref()?),
            Kind::Select if n.path => self.resolve(ix, n.ty.as_ref()?),
            Kind::Select => {
                let file = self.files[ix as usize].file.clone();
                let nm = n.name?;
                let name = name_text(&file, nm);
                let sig = signed(&file, nm);
                let owner = match &n.ty {
                    Some(t) => self.resolve(ix, t)?,
                    None => self.tree_class(ix, n.children.first()?)?,
                };
                self.member(owner, &name, true, sig.as_deref())
            }
            Kind::Apply | Kind::TypeApply => self.tree_sym(ix, n.children.first()?),
            _ => None,
        }
    }

    /// The class of a tree's type, widened (`tpe.widenSingleton.classSymbol`), where the graph can tell it.
    pub fn tree_class(&mut self, ix: FileIx, n: &Node) -> Option<SymId> {
        match n.kind {
            Kind::This => self.resolve(ix, n.ty.as_ref()?),
            // `New`'s class: an alias dealiased.
            Kind::New => {
                let t = n.ty.clone()?;
                match self.ty_of(ix, &t, &HashMap::new()).and_then(|t| t.class()) {
                    Some(c) => Some(c),
                    None => self.resolve(ix, &t),
                }
            }
            Kind::Ident | Kind::Select => {
                let s = self.tree_sym(ix, n)?;
                self.value_class(s)
            }
            Kind::Apply | Kind::TypeApply => {
                let s = self.tree_sym(ix, n)?;
                if self.sym(s).name == "<init>" {
                    return self.sym(s).owner;
                }
                // The result's class as seen from the receiver and the type arguments, else as declared.
                if let Some(c) = self.tree_ty(ix, n).and_then(|t| t.class()) {
                    return Some(c);
                }
                // A cast (`Any`'s `asInstanceOf`, its `$asInstanceOf$`): its type argument's class.
                let name = self.sym(s).name.clone();
                if n.kind == Kind::TypeApply && matches!(name.as_str(), "asInstanceOf" | "$asInstanceOf$") {
                    if let Some(t) = n.children.get(1).and_then(|a| a.ty.clone()) {
                        if let Some(c) = self.type_class(ix, &t) {
                            return Some(c);
                        }
                    }
                }
                self.declared_class(s)
            }
            Kind::Typed => {
                let t = n.children.get(1)?;
                self.resolve(ix, t.ty.as_ref()?)
            }
            // A block's value: its expression's, its last tree (`new T {}`: the anonymous class's construction).
            Kind::Block => self.tree_class(ix, n.children.last()?),
            // A constant's class (`ConstantType.widen`).
            Kind::Literal => match n.ty.as_ref()? {
                TType::Const(c) => self.const_class(&c.clone()),
                _ => None,
            },
            _ => None,
        }
    }

    /// The class a value of the symbol `s` has: a package's or a class's own, a module's class, else its declared
    /// type's.
    pub fn value_class(&mut self, s: SymId) -> Option<SymId> {
        let sym = self.sym(s).clone();
        if sym.package || !sym.term {
            return Some(s);
        }
        if sym.flags.has(DefFlags::MODULE) {
            return self.module_class(s);
        }
        self.declared_class(s)
    }

    /// The class of a type for `classSymbol`: a class's own, an alias's or a union's.
    fn type_class(&mut self, ix: FileIx, t: &TType) -> Option<SymId> {
        if let TType::Annotated(a, _) = t {
            return self.type_class(ix, a);
        }
        if let TType::Or(a, b) = t {
            return self.or_class(ix, a, b);
        }
        if let TType::And(a, b) = t {
            return self.and_class(ix, a, b);
        }
        if let TType::Const(k) = t {
            return self.const_class(&k.clone());
        }
        match self.resolve(ix, t) {
            Some(c) if self.sym(c).class => Some(c),
            r => self.ty_of(ix, t, &HashMap::new()).and_then(|t| t.class()).or_else(|| r.and_then(|c| self.bound_class(c))),
        }
    }

    /// `Type.classSymbol` of `a & b`: the side's class deriving from the other's, else none.
    fn and_class(&mut self, ix: FileIx, a: &TType, b: &TType) -> Option<SymId> {
        let ca = self.type_class(ix, a)?;
        let cb = self.type_class(ix, b)?;
        if ca == cb || self.linearization(ca).contains(&cb) {
            Some(ca)
        } else if self.linearization(cb).contains(&ca) {
            Some(cb)
        } else {
            None
        }
    }

    /// `Type.classSymbol` of `a | b`: the other side's where one is `Nothing`, else the join's (`TypeOps.orDominator`): of
    /// the base classes the two share, those no other shared one derives from, and of those the one deriving from
    /// all the others (`AndType.classSymbol`), or none.
    fn or_class(&mut self, ix: FileIx, a: &TType, b: &TType) -> Option<SymId> {
        let ca = self.type_class(ix, a)?;
        let cb = self.type_class(ix, b)?;
        let nothing = |g: &mut Self, c: SymId| g.full_name(c) == "scala.Nothing";
        if nothing(self, ca) {
            return Some(cb);
        }
        if nothing(self, cb) {
            return Some(ca);
        }
        let la = self.linearization(ca);
        let lb = self.linearization(cb);
        let shared: Vec<SymId> = la.iter().copied().filter(|c| lb.contains(c)).collect();
        let mut lins: Vec<(SymId, Vec<SymId>)> = Vec::new();
        for &c in &shared {
            lins.push((c, self.linearization(c)));
        }
        // A dominator: no other shared class derives from it.
        let dominators: Vec<SymId> = shared.iter().copied().filter(|&c| !lins.iter().any(|(d, l)| *d != c && l.contains(&c))).collect();
        let mut out: Option<SymId> = None;
        for d in dominators {
            out = match out {
                None => Some(d),
                Some(o) => {
                    if self.linearization(d).contains(&o) {
                        Some(d)
                    } else if self.linearization(o).contains(&d) {
                        Some(o)
                    } else {
                        return None;
                    }
                }
            };
        }
        out
    }

    /// A constant's class, its type widened (`ConstantType.widen`).
    pub fn const_class(&mut self, c: &Const) -> Option<SymId> {
        let (pkg, name) = match c {
            Const::Unit => ("scala", "Unit"),
            Const::Bool(_) => ("scala", "Boolean"),
            Const::Byte(_) => ("scala", "Byte"),
            Const::Short(_) => ("scala", "Short"),
            Const::Char(_) => ("scala", "Char"),
            Const::Int(_) => ("scala", "Int"),
            Const::Long(_) => ("scala", "Long"),
            Const::Float(_) => ("scala", "Float"),
            Const::Double(_) => ("scala", "Double"),
            Const::Str(_) => ("java.lang", "String"),
            Const::Null => ("scala", "Null"),
            Const::Class(_) => ("java.lang", "Class"),
        };
        self.library_class(pkg, name, false)
    }

    /// The class of a val's type or a method's result type as its definition's tree writes it.
    pub fn declared_class(&mut self, s: SymId) -> Option<SymId> {
        let Some((ix, addr)) = self.sym(s).def else {
            // A member with no tree (a Java method, one `Definitions` enters): its signature's result class.
            let sig = self.signature(s)?;
            let result = sig.rsplit_once(')')?.1.to_string();
            let (pkg, name) = result.rsplit_once('.')?;
            let c = self.library_class(pkg, name, false)?;
            return self.sym(c).class.then_some(c);
        };
        let ty = {
            let d = find_node(&self.files[ix as usize].nodes, addr)?;
            match d.kind {
                // A pattern binder: its own type (`BIND`'s).
                Kind::Bind => d.ty.clone()?,
                _ => d.children.iter().find(|c| c.kind == Kind::Tpt)?.ty.clone()?,
            }
        };
        // A constant type (`final val f = 1`): its constant's class.
        if let TType::Const(k) = &ty {
            return self.const_class(&k.clone());
        }
        // A union (`A | B`): its join's class (`Type.classSymbol` of an `OrType`); an intersection, its meet's.
        if let TType::Or(a, b) = &ty {
            return self.or_class(ix, a, b);
        }
        if let TType::And(a, b) = &ty {
            return self.and_class(ix, a, b);
        }
        let c = self.resolve(ix, &ty)?;
        // A singleton type (`immutable.Set.type` of `Predef.Set`): its value's class.
        if self.sym(c).term && !self.sym(c).package {
            return self.value_class(c);
        }
        if self.sym(c).class || self.sym(c).package {
            return Some(c);
        }
        // An alias (`Predef.String`): its right side's class (`classSymbol` of the dealiased type).
        if let Some(k) = self.ty_of(ix, &ty, &HashMap::new()).and_then(|t| t.class()) {
            return Some(k);
        }
        // A type parameter or an abstract type (`A <: X`): its upper bound's class (`TypeRef.classSymbol` of a
        // non-class symbol, `superType.classSymbol`).
        self.bound_class(c)
    }

    /// The class of a type parameter's or an abstract type's upper bound; one being followed is not followed again.
    fn bound_class(&mut self, s: SymId) -> Option<SymId> {
        let (fix, addr) = self.sym(s).def?;
        if self.expanding.contains(&s) {
            return None;
        }
        let rhs = find_node(&self.files[fix as usize].nodes, addr).and_then(|n| n.children.first()).and_then(|c| c.ty.clone());
        let hi = match rhs? {
            TType::Bounds(_, hi) => *hi,
            _ => return None,
        };
        self.expanding.push(s);
        let out = self.type_class(fix, &hi);
        self.expanding.pop();
        out
    }

    /// The parameters of a method's `k`-th term clause as the reference's `margs` prints them: `=>` for a by-name
    /// parameter, `v` else; a method of the class path with no tree, from its signature (a Java method's are
    /// values).
    pub fn margs(&mut self, s: SymId, k: usize) -> Option<String> {
        match self.sym(s).def {
            Some((ix, addr)) => {
                let d = find_node(&self.files[ix as usize].nodes, addr)?;
                let start: usize = d.clauses.iter().take(k).map(|&c| c as usize).sum();
                let count = *d.clauses.get(k)? as usize;
                let params: Vec<&Node> = d.children.iter().filter(|c| c.kind == Kind::ValDef).collect();
                let items: Vec<&str> = params
                    .get(start..start + count)?
                    .iter()
                    .map(|p| match p.children.first().and_then(|t| t.ty.as_ref()) {
                        Some(TType::ByName(_)) => "=>",
                        _ => "v",
                    })
                    .collect();
                Some(format!("[{}]", items.join(",")))
            }
            None => {
                if k > 0 {
                    return None;
                }
                let sig = self.signature(s)?;
                let inner = sig.strip_prefix("#(")?.split(')').next()?.to_string();
                let n = inner.split(',').filter(|p| !p.is_empty() && !p.chars().all(|c| c.is_ascii_digit())).count();
                Some(format!("[{}]", vec!["v"; n].join(",")))
            }
        }
    }

    /// Whether a symbol is static (`Symbol.isStatic`): owned by a package, or by a module class that is.
    pub fn is_static(&mut self, s: SymId) -> bool {
        match self.sym(s).owner {
            None => true,
            Some(o) => {
                let os = self.sym(o).clone();
                os.package || (os.class && os.flags.has(DefFlags::MODULE) && self.is_static(o))
            }
        }
    }

    /// `tpd.prefixIsElidable` of a path `Select`'s prefix: a static owner's `this`, or a static module.
    pub fn prefix_elidable(&mut self, ix: FileIx, n: &Node) -> bool {
        let Some(q) = n.children.first() else { return false };
        match q.kind {
            Kind::This => match q.ty.as_ref().and_then(|t| self.resolve(ix, t)) {
                Some(c) => {
                    let cs = self.sym(c).clone();
                    cs.package || (cs.flags.has(DefFlags::MODULE) && self.is_static(c))
                }
                None => false,
            },
            Kind::Ident => match self.tree_sym(ix, q) {
                Some(p) => self.sym(p).flags.has(DefFlags::MODULE) && self.is_static(p),
                None => false,
            },
            _ => false,
        }
    }

    /// The class of a module value (`object O`'s `O$`).
    pub fn module_class(&mut self, val: SymId) -> Option<SymId> {
        let s = self.sym(val).clone();
        let owner = s.owner?;
        let name = format!("{}$", s.name);
        self.decl(owner, &name, false, None)
    }

    /// A class's linearization (SLS 5.1.2): itself, then `L(Tn) +⃗ ... +⃗ L(T1)` of its parents `T1 .. Tn`, where
    /// `A +⃗ B` is `A`'s elements not in `B`, then `B`.
    pub fn linearization(&mut self, class: SymId) -> Vec<SymId> {
        if let Some(l) = self.lin_cache.get(&class) {
            return l.clone();
        }
        // A class in progress (a reference its own parents' resolution makes, or a cycle of an illegal program):
        // itself, not cached.
        if self.parents_in_progress.contains(&class) {
            return vec![class];
        }
        self.parents_in_progress.push(class);
        let direct = self.direct_parents(class);
        let mut acc: Vec<SymId> = Vec::new();
        for p in direct {
            let lp = self.linearization(p);
            let seen: std::collections::HashSet<SymId> = acc.iter().copied().collect();
            let mut next: Vec<SymId> = lp.into_iter().filter(|x| !seen.contains(x)).collect();
            next.extend(acc);
            acc = next;
        }
        self.parents_in_progress.pop();
        let mut out = vec![class];
        out.extend(acc.into_iter().filter(|&x| x != class));
        // Computed once (`baseData`): a completed linearization is kept.
        self.lin_cache.insert(class, out.clone());
        out
    }

    /// A class's direct parents' classes, in the order its template writes them.
    fn direct_parents(&mut self, class: SymId) -> Vec<SymId> {
        let Some((ix, addr)) = self.sym(class).def else { return Vec::new() };
        let parents: Vec<TType> = {
            let Some(n) = find_node(&self.files[ix as usize].nodes, addr) else { return Vec::new() };
            let Some(t) = n.children.first() else { return Vec::new() };
            t.template_parents().iter().filter_map(parent_type).collect()
        };
        let mut out = Vec::new();
        for p in parents {
            if let Some(c) = self.resolve(ix, &p) {
                if !out.contains(&c) {
                    out.push(c);
                }
            }
        }
        out
    }

    /// A method's signature as the reference prints it, computed from its parameters' and result's erasure.
    pub fn signature(&mut self, id: SymId) -> Option<String> {
        if let Some(s) = &self.sym(id).sig {
            return Some(s.clone());
        }
        let s = self.sym(id).clone();
        if !s.flags.has(DefFlags::METHOD) {
            return None;
        }
        let (ix, addr) = s.def?;
        // A method with no parameter clause and no type parameters (`def g = ..`) has `Signature.NotAMethod`: no
        // parameters and an empty result, printed `#()`.
        {
            let n = find_node(&self.files[ix as usize].nodes, addr)?;
            if n.clauses.is_empty() && !n.children.iter().any(|c| c.kind == Kind::TypeDef) && s.name != "<init>" {
                self.syms[id.0 as usize].sig = Some("#()".to_string());
                return Some("#()".to_string());
            }
        }
        let (params, result) = {
            let n = find_node(&self.files[ix as usize].nodes, addr)?;
            let mut params: Vec<String> = Vec::new();
            let mut tparams = 0usize;
            let mut types: Vec<TType> = Vec::new();
            let mut result = None;
            for c in &n.children {
                match c.kind {
                    Kind::TypeDef => tparams += 1,
                    Kind::ValDef => {
                        if tparams > 0 {
                            params.push(tparams.to_string());
                            tparams = 0;
                        }
                        types.push(c.children.first().and_then(|t| t.ty.clone()).unwrap_or(TType::Unknown(0)));
                        params.push(String::new());
                    }
                    Kind::Tpt if result.is_none() => result = c.ty.clone(),
                    _ => {}
                }
            }
            if tparams > 0 {
                params.push(tparams.to_string());
            }
            (params.into_iter().zip_types(types), result)
        };
        let mut texts = Vec::new();
        for p in params {
            texts.push(match p {
                Either::Count(n) => n,
                Either::Type(t) => self.erased_name(ix, &t),
            });
        }
        let ctor = s.name == "<init>";
        // `Array`'s constructor: the class `Array` itself erases to `Object` in a signature.
        let array_ctor = ctor && s.owner.is_some_and(|o| self.full_name(o) == "scala.Array");
        let res = if array_ctor {
            "java.lang.Object".to_string()
        } else if ctor {
            s.owner.map(|o| self.class_sig_name(o)).unwrap_or_default()
        } else {
            result.map(|t| self.erased_name(ix, &t)).unwrap_or_default()
        };
        let sig = format!("#({}){}", texts.join(","), res);
        self.syms[id.0 as usize].sig = Some(sig.clone());
        Some(sig)
    }

    /// A class's name in a signature: its full name, a class of the empty package by its own name.
    pub fn class_sig_name(&self, class: SymId) -> String {
        let s = self.sym(class);
        // A local class: its enclosing class's name, then `_$` for each owner climbed to reach it, then its own
        // (`SymDenotation.fullNameSeparated`'s filler).
        if let Some(o) = s.owner {
            if !self.sym(o).class && !self.sym(o).package {
                let mut e = Some(o);
                let mut filler = String::new();
                while let Some(x) = e {
                    if self.sym(x).class || self.sym(x).package {
                        break;
                    }
                    filler.push_str("_$");
                    e = self.sym(x).owner;
                }
                if let Some(c) = e.filter(|&c| self.sym(c).class) {
                    return format!("{}.{}{}", self.class_sig_name(c), filler, s.name);
                }
            }
        }
        match s.owner {
            Some(o) if self.sym(o).package && self.sym(o).name == "<empty>" => s.name.clone(),
            Some(o) if o == self.root => s.name.clone(),
            Some(o) => format!("{}.{}", self.class_sig_name(o), s.name),
            None => s.name.clone(),
        }
    }

    /// The erasure of a type as a signature names it (`TypeErasure.sigName`).
    pub fn erased_name(&mut self, ix: FileIx, ty: &TType) -> String {
        match ty {
            TType::ByName(_) => "scala.Function0".into(),
            // An array: its element's name and `[]` (`Signature`'s `sigName` of `Array[T]`).
            TType::Applied(t, args) if args.len() == 1 && self.resolve(ix, t).is_some_and(|c| self.full_name(c) == "scala.Array") => {
                // An array of a type parameter (or an abstract type) erases to Object (`TypeErasure` of a generic
                // array).
                // `Array[T]` of a type parameter or an abstract type: `Object` where its bound may be a primitive
                // (`Any`), `Object[]` (its bound's array) where the bound is a reference type (`T <: AnyRef`).
                let elem = self.resolve(ix, &args[0]);
                let bound = match elem.map(|e| self.sym(e).clone()) {
                    Some(e) if !e.class && !e.package => match e.def {
                        Some((fix, addr)) => {
                            let rhs = find_node(&self.files[fix as usize].nodes, addr).and_then(|n| n.children.first()).and_then(|c| c.ty.clone());
                            match rhs {
                                Some(TType::Bounds(_, hi)) => Some(Some((fix, *hi))),
                                // An alias (`Predef.String`): its right side's array.
                                Some(TType::Alias(a)) => return format!("{}[]", self.erased_name(fix, &a)),
                                Some(t) => return format!("{}[]", self.erased_name(fix, &t)),
                                None => Some(None),
                            }
                        }
                        None => Some(None),
                    },
                    _ => None,
                };
                match bound {
                    None if !matches!(args[0], TType::ParamRef(..) | TType::Bounds(..)) => format!("{}[]", self.erased_name(ix, &args[0])),
                    Some(Some((fix, hi))) => {
                        if self.reference_bound(fix, &hi) { format!("{}[]", self.erased_name(fix, &hi)) } else { "java.lang.Object".into() }
                    }
                    _ => "java.lang.Object".into(),
                }
            }
            TType::Applied(t, _) | TType::Annotated(t, _) => self.erased_name(ix, t),
            TType::Refined(p, _) => self.erased_name(ix, p),
            // A singleton type: an object's is its class (`A.type` erases to `A$`), another's Object.
            TType::TermRef(..) | TType::LocalTerm(..) => match self.resolve(ix, ty) {
                Some(s) if self.sym(s).flags.has(DefFlags::MODULE) && self.sym(s).term => match self.module_class(s) {
                    Some(c) => self.class_sig_name(c),
                    None => "java.lang.Object".into(),
                },
                _ => "java.lang.Object".into(),
            },
            TType::And(a, _) => self.erased_name(ix, a),
            // A union with `Null`: its other side's erasure (`erasedLub`); another union's, Object.
            TType::Or(a, b) => {
                let null = |g: &mut Self, t: &TType| g.resolve(ix, t).is_some_and(|c| g.full_name(c) == "scala.Null");
                if null(self, b) {
                    self.erased_name(ix, a)
                } else if null(self, a) {
                    self.erased_name(ix, b)
                } else {
                    "java.lang.Object".into()
                }
            }
            // A parameterised alias's right side (`type Class[T] = java.lang.Class[T]`): its result's erasure.
            TType::Lambda { result, .. } => self.erased_name(ix, result),
            TType::Alias(t) => self.erased_name(ix, t),
            // Bounds (a higher-kinded parameter's `[X] =>> <: Domain[X]`): the upper bound's erasure (`TypeErasure`'s
            // of a `TypeBounds`).
            TType::Bounds(_, hi) => self.erased_name(ix, hi),
            _ => match self.resolve(ix, ty) {
                Some(c) => {
                    let s = self.sym(c).clone();
                    if s.class {
                        let full = self.class_sig_name(c);
                        // The function classes `Definitions` synthesizes, erased as `FunctionN` (`TypeErasure`).
                        for prefix in ["scala.ImpureContextFunction", "scala.ImpureFunction", "scala.ContextFunction"] {
                            if let Some(n) = full.strip_prefix(prefix).filter(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit())) {
                                return format!("scala.Function{}", n);
                            }
                        }
                        // A value class in a signature is its underlying type (`TypeErasure.sigName` of an erased
                        // value class): its one parameter's.
                        if let Some(u) = self.value_class_underlying(c) {
                            return u;
                        }
                        // `Definitions.specialErasure`, and `AnyRef` dealiased.
                        match full.as_str() {
                            "scala.Any" | "scala.AnyRef" | "scala.AnyVal" | "scala.Matchable" | "scala.caps.Pure" | "scala.Singleton" | "scala.*:" => "java.lang.Object".into(),
                            "scala.Tuple" | "scala.NonEmptyTuple" => "scala.Product".into(),
                            "scala.Nothing" => "scala.Nothing".into(),
                            "scala.Null" => "scala.runtime.Null$".into(),
                            "scala.<repeated>" => "scala.collection.immutable.Seq".into(),
                            _ => full,
                        }
                    } else if let Some((fix, addr)) = s.def {
                        // An alias: its right side's erasure; a type parameter or an abstract type: its upper bound's.
                        let rhs = find_node(&self.files[fix as usize].nodes, addr).and_then(|n| n.children.first()).and_then(|t| t.ty.clone());
                        match rhs {
                            Some(TType::Alias(t)) => self.erased_name(fix, &t),
                            Some(TType::Bounds(_, hi)) => self.erased_name(fix, &hi),
                            Some(t) => self.erased_name(fix, &t),
                            None => "java.lang.Object".into(),
                        }
                    } else {
                        self.class_sig_name(c)
                    }
                }
                None => "?".into(),
            },
        }
    }

    /// Whether a type parameter's upper bound is a reference type (`<: AnyRef`, which `Null` is too): its array erases
    /// to its bound's array; a bound of `Any`, `Matchable` or `AnyVal` (or one not read) to `Object`.
    fn reference_bound(&mut self, ix: FileIx, hi: &TType) -> bool {
        match hi {
            TType::Or(a, b) | TType::And(a, b) => {
                let (ra, rb) = (self.reference_bound(ix, a), self.reference_bound(ix, b));
                if matches!(hi, TType::Or(..)) { ra && rb } else { ra || rb }
            }
            TType::Annotated(a, _) => self.reference_bound(ix, a),
            _ => match self.resolve(ix, hi).map(|c| self.full_name(c)) {
                Some(n) => !matches!(n.as_str(), "scala.Any" | "scala.Matchable" | "scala.AnyVal"),
                None => false,
            },
        }
    }

    /// A value class's (a class of the class path or the program deriving from `AnyVal`, not a primitive) underlying
    /// type's erased name: its constructor's one parameter's.
    fn value_class_underlying(&mut self, c: SymId) -> Option<String> {
        let full = self.full_name(c);
        if full.starts_with("scala.") && matches!(&full["scala.".len()..], "Int" | "Long" | "Short" | "Byte" | "Char" | "Boolean" | "Float" | "Double" | "Unit" | "AnyVal") {
            return None;
        }
        let (ix, addr) = self.sym(c).def?;
        let direct: Vec<TType> = {
            let n = find_node(&self.files[ix as usize].nodes, addr)?;
            let t = n.children.first()?;
            t.template_parents().iter().filter_map(parent_type).collect()
        };
        let any_val = direct.iter().any(|p| self.resolve(ix, p).is_some_and(|k| self.full_name(k) == "scala.AnyVal"));
        if !any_val {
            return None;
        }
        // The constructor's parameter: the template's first parameter accessor (`PARAM`).
        let ty = {
            let n = find_node(&self.files[ix as usize].nodes, addr)?;
            let t = n.children.first()?;
            let p = t.children.iter().find(|c| c.kind == Kind::ValDef && c.flags.has(DefFlags::PARAM_ACCESSOR))?;
            p.children.first()?.ty.clone()?
        };
        Some(self.erased_name(ix, &ty))
    }

    /// A symbol's text as the reference's dump prints it: its owners' names joined by `.`, a method's with its
    /// signature.
    pub fn sym_text(&mut self, id: SymId) -> String {
        let s = self.sym(id).clone();
        if id == self.root {
            return "<root>".into();
        }
        if s.package {
            return self.full_name(id);
        }
        let own = s.owner.map(|o| self.sym_text(o)).unwrap_or_default();
        let sig = if s.flags.has(DefFlags::METHOD) { self.signature(id).unwrap_or_default() } else { String::new() };
        format!("{}.{}{}", own, s.name, sig)
    }
}

enum Either {
    Count(String),
    Type(TType),
}

trait ZipTypes {
    fn zip_types(self, types: Vec<TType>) -> Vec<Either>;
}

impl<I: Iterator<Item = String>> ZipTypes for I {
    fn zip_types(self, types: Vec<TType>) -> Vec<Either> {
        let mut ts = types.into_iter();
        self.map(|p| if p.is_empty() { Either::Type(ts.next().unwrap_or(TType::Unknown(0))) } else { Either::Count(p) }).collect()
    }
}

/// The type a parent tree of a template names: a type tree's, or the class a constructor call constructs.
/// A name's encoding in class file names, scalac's `NameTransformer.encode`: each operator character as `$` and its
/// name (`::` is `$colon$colon`).
fn encode_name(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        let code = match c {
            '~' => "$tilde",
            '=' => "$eq",
            '<' => "$less",
            '>' => "$greater",
            '!' => "$bang",
            '#' => "$hash",
            '%' => "$percent",
            '^' => "$up",
            '&' => "$amp",
            '|' => "$bar",
            '*' => "$times",
            '/' => "$div",
            '+' => "$plus",
            '-' => "$minus",
            ':' => "$colon",
            '\\' => "$bslash",
            '?' => "$qmark",
            '@' => "$at",
            _ => {
                out.push(c);
                continue;
            }
        };
        out.push_str(code);
    }
    out
}

fn parent_type(n: &Node) -> Option<TType> {
    match n.kind {
        Kind::Tpt => n.ty.clone(),
        Kind::New => n.ty.clone(),
        Kind::Apply | Kind::TypeApply | Kind::Select => n.children.first().and_then(parent_type),
        Kind::Block => n.children.last().and_then(parent_type),
        _ => None,
    }
}

/// The node of a definition at `addr`.
pub fn find_node(nodes: &[Node], addr: Addr) -> Option<&Node> {
    for n in nodes {
        if n.at == addr && matches!(n.kind, Kind::ValDef | Kind::DefDef | Kind::TypeDef | Kind::Bind) {
            return Some(n);
        }
        if let Some(f) = find_node(&n.children, addr) {
            return Some(f);
        }
        if let Some(c) = &n.call {
            if let Some(f) = find_node(std::slice::from_ref(c), addr) {
                return Some(f);
            }
        }
    }
    None
}

/// A tree's type as far as the view reads it (`tpe`): a class applied to its arguments, or a type parameter the
/// substitution did not reach. Enough for `tpe.widenSingleton.classSymbol` of a generic call's result.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Ty {
    Class(SymId, Vec<Ty>),
    /// A type parameter the substitution did not reach.
    Param,
}

impl Ty {
    pub fn class(&self) -> Option<SymId> {
        match self {
            Ty::Class(c, _) => Some(*c),
            Ty::Param => None,
        }
    }
}


impl Graph {
    /// A definition's type parameters, in order: a class's (its template's `TypeDef` parameters), a method's.
    pub fn type_params(&mut self, s: SymId) -> Vec<SymId> {
        let Some((ix, addr)) = self.sym(s).def else { return Vec::new() };
        let addrs: Vec<Addr> = {
            let Some(n) = find_node(&self.files[ix as usize].nodes, addr) else { return Vec::new() };
            let holder = if n.is_class_def() { n.children.first() } else { Some(n) };
            holder
                .map(|h| h.children.iter().filter(|c| c.kind == Kind::TypeDef && c.flags.has(DefFlags::PARAM)).map(|c| c.at).collect())
                .unwrap_or_default()
        };
        addrs.into_iter().filter_map(|a| self.defined_at(ix, a)).collect()
    }

    /// The `Ty` of a TASTy type read in file `ix`, the type parameters of `env` substituted.
    pub fn ty_of(&mut self, ix: FileIx, t: &TType, env: &HashMap<SymId, Ty>) -> Option<Ty> {
        match t {
            TType::Applied(f, args) => {
                let targs: Vec<Ty> = args.iter().map(|a| self.ty_of(ix, a, env).unwrap_or(Ty::Param)).collect();
                match self.ty_of(ix, f, env)? {
                    Ty::Class(c, _) => Some(Ty::Class(c, targs)),
                    // A higher-kinded parameter substituted by a constructor: the constructor applied.
                    Ty::Param => Some(Ty::Param),
                }
            }
            TType::Annotated(a, _) | TType::ByName(a) => self.ty_of(ix, a, env),
            TType::Bounds(_, hi) => self.ty_of(ix, hi, env),
            TType::Lambda { result, .. } => self.ty_of(ix, result, env),
            TType::TypeRef(..) | TType::LocalType(..) | TType::This(_) => {
                let s = self.resolve(ix, t)?;
                let sym = self.sym(s).clone();
                if sym.class || sym.package {
                    return Some(Ty::Class(s, Vec::new()));
                }
                if let Some(v) = env.get(&s) {
                    return Some(v.clone());
                }
                // An alias or an abstract type: its right side's, or its upper bound's; one being expanded is not
                // expanded again (a visited set: a legal program's aliases are not cyclic).
                let (fix, addr) = sym.def?;
                if self.expanding.contains(&s) {
                    return None;
                }
                let rhs = find_node(&self.files[fix as usize].nodes, addr).and_then(|n| n.children.first()).and_then(|c| c.ty.clone());
                self.expanding.push(s);
                let out = match rhs {
                    Some(TType::Alias(a)) => self.ty_of(fix, &a, env),
                    Some(r @ TType::Bounds(..)) if !sym.flags.has(DefFlags::PARAM) => self.ty_of(fix, &r, env),
                    Some(TType::Bounds(..)) | None => Some(Ty::Param),
                    // An alias's right side as the reader holds it (the type tree's type).
                    Some(r) => self.ty_of(fix, &r, env),
                };
                self.expanding.pop();
                out
            }
            TType::TermRef(..) | TType::LocalTerm(..) => {
                let s = self.resolve(ix, t)?;
                self.value_class(s).map(|c| Ty::Class(c, Vec::new()))
            }
            _ => None,
        }
    }

    /// The arguments `ty` gives the base class `target` (`baseType(target).args`), walking its class's parents with
    /// their arguments substituted.
    pub fn base_args(&mut self, ty: &Ty, target: SymId) -> Option<Vec<Ty>> {
        let Ty::Class(c, args) = ty else { return None };
        if *c == target {
            return Some(args.clone());
        }
        // Each class with its type arguments once a query: a diamond of parents reaches a class by many paths, whose
        // result is kept (with other arguments, as `baseType`'s intersection meets them, walked for those). A class on
        // the current path is not walked again (no legal class is its own parent).
        if self.base_depth == 0 {
            self.base_memo.clear();
        }
        if let Some(r) = self.base_memo.get(&(*c, args.clone())) {
            return r.clone();
        }
        if self.base_visiting.contains(c) {
            return None;
        }
        let params = self.type_params(*c);
        let env: HashMap<SymId, Ty> = params.into_iter().zip(args.iter().cloned()).collect();
        let (ix, addr) = self.sym(*c).def?;
        let parents: Vec<TType> = {
            let n = find_node(&self.files[ix as usize].nodes, addr)?;
            let t = n.children.first()?;
            t.template_parents().iter().filter_map(parent_type_applied).collect()
        };
        // Every parent's path to the target: `baseType` intersects them (`immutable.Set`'s `IterableOps` through
        // `collection.Set` and `immutable.Iterable`), each argument the class deriving from the others'.
        self.base_visiting.push(*c);
        self.base_depth += 1;
        let mut found: Option<Vec<Ty>> = None;
        for p in parents {
            if let Some(pt) = self.ty_of(ix, &p, &env) {
                if let Some(args) = self.base_args(&pt, target) {
                    found = Some(match found {
                        None => args,
                        Some(prev) if prev.len() == args.len() => prev.into_iter().zip(args).map(|(a, b)| self.meet(a, b)).collect(),
                        Some(prev) => prev,
                    });
                }
            }
        }
        self.base_depth -= 1;
        self.base_visiting.pop();
        self.base_memo.insert((*c, args.clone()), found.clone());
        found
    }

    /// The head class of `a & b` (`classSymbol` of an `AndType`): the one deriving from the other; an unknown side the
    /// other; two unrelated classes the first.
    fn meet(&mut self, a: Ty, b: Ty) -> Ty {
        match (&a, &b) {
            (Ty::Class(ca, _), Ty::Class(cb, _)) => {
                if ca == cb || self.linearization(*ca).contains(cb) {
                    a
                } else if self.linearization(*cb).contains(ca) {
                    b
                } else {
                    a
                }
            }
            (Ty::Param, _) => b,
            _ => a,
        }
    }

    /// A tree's type, where the view can read it: an application's result with its method's and its owner's type
    /// parameters substituted (`asSeenFrom` of the receiver's base type), a reference's declared type, `New`'s,
    /// `This`'s, an ascription's.
    pub fn tree_ty(&mut self, ix: FileIx, n: &Node) -> Option<Ty> {
        match n.kind {
            Kind::This | Kind::New => {
                let t = n.ty.clone()?;
                self.ty_of(ix, &t, &HashMap::new())
            }
            Kind::Typed => {
                let t = n.children.get(1)?.ty.clone()?;
                self.ty_of(ix, &t, &HashMap::new())
            }
            Kind::Apply | Kind::TypeApply | Kind::Ident | Kind::Select => {
                let s = self.tree_sym(ix, n)?;
                if self.sym(s).name == "<init>" {
                    return self.sym(s).owner.map(|o| Ty::Class(o, Vec::new()));
                }
                let sym = self.sym(s).clone();
                if sym.package || sym.class || sym.flags.has(DefFlags::MODULE) {
                    return self.value_class(s).map(|c| Ty::Class(c, Vec::new()));
                }
                // The environment: the method's type parameters from the `TypeApply`'s trees, the owner's from the
                // receiver's base type.
                let mut env: HashMap<SymId, Ty> = HashMap::new();
                let mut f = Some(n);
                let mut receiver = None;
                while let Some(x) = f {
                    match x.kind {
                        Kind::Apply => f = x.children.first(),
                        Kind::TypeApply => {
                            let tparams = self.type_params(s);
                            for (p, a) in tparams.into_iter().zip(x.children.iter().skip(1)) {
                                if let Some(at) = a.ty.clone() {
                                    if let Some(v) = self.ty_of(ix, &at, &HashMap::new()) {
                                        env.insert(p, v);
                                    }
                                }
                            }
                            f = x.children.first();
                        }
                        Kind::Select if !x.path => {
                            receiver = x.children.first();
                            break;
                        }
                        _ => break,
                    }
                }
                if let (Some(r), Some(owner)) = (receiver, sym.owner) {
                    if let Some(rt) = self.tree_ty(ix, r) {
                        if let Some(args) = self.base_args(&rt, owner) {
                            let ps = self.type_params(owner);
                            env.extend(ps.into_iter().zip(args));
                        }
                    }
                }
                let (dix, addr) = sym.def?;
                let rt = {
                    let d = find_node(&self.files[dix as usize].nodes, addr)?;
                    match d.kind {
                        Kind::Bind => d.ty.clone()?,
                        _ => d.children.iter().find(|c| c.kind == Kind::Tpt)?.ty.clone()?,
                    }
                };
                self.ty_of(dix, &rt, &env)
            }
            _ => None,
        }
    }
}

/// A parent's type with its arguments (`readParents`): a constructor call's `New` applied to the `TypeApply`'s
/// arguments where the `New` holds the constructor unapplied, else the type tree's.
fn parent_type_applied(n: &Node) -> Option<TType> {
    match n.kind {
        Kind::Tpt | Kind::New => n.ty.clone(),
        Kind::TypeApply => {
            let head = n.children.first().and_then(parent_type_applied)?;
            let args: Vec<TType> = n.children.iter().skip(1).filter_map(|a| a.ty.clone()).collect();
            match head {
                TType::Lambda { result, .. } => match *result {
                    TType::Applied(f, _) => Some(TType::Applied(f, args)),
                    other => Some(other),
                },
                TType::Applied(f, _) => Some(TType::Applied(f, args)),
                other if !args.is_empty() => Some(TType::Applied(Box::new(other), args)),
                other => Some(other),
            }
        }
        Kind::Apply | Kind::Select => n.children.first().and_then(parent_type_applied),
        Kind::Block => n.children.last().and_then(parent_type_applied),
        _ => None,
    }
}
