//! A Scala 2 pickle rewritten as TASTy: the package clause of the class file, its classes,
//! objects and their members as definitions, every type spelt as TASTy spells it. A member with
//! a body gets an `ELIDED` one, so that it reads as concrete. References to definitions of the
//! pickle go by address; lengths and addresses take four bytes each, so that a forward reference
//! is patched in place once its definition is written.
//!
//! Left out is what a Scala 3 program cannot use: private members, fields, bridges, macros,
//! value classes' `$extension` methods. A type an existential binds reads as a wildcard where it
//! is an argument and as its upper bound elsewhere, as scalac 3 reads it. What has no TASTy
//! here (a refinement member that is a type constructor, an unknown tag) becomes a tag the TASTy
//! reader does not know, which the typer reports as unsupported where it is used.

use super::pickle::{self as pk, flags as f, LocalSym, Pickle};
use crate::intern::FxMap;
use crate::tasty::tags as t;

/// The tag of a type this rewriting has no TASTy for; its body is the reason as a string.
pub const UNSUPPORTED: u8 = 187;

/// The TASTy file of the pickle `raw` (the encoded `bytes` of `ScalaSignature`) found in a class
/// file of package `package` (`kantan/csv`). `is_package` tells a package from an object among
/// the owners of what the pickle refers to (`kantan/csv/codecs`).
pub fn to_tasty(raw: Vec<u8>, package: &str, is_package: &dyn Fn(&str) -> bool) -> Result<Vec<u8>, String> {
    let p = Pickle::parse(super::pickle::decode_signature(raw))?;
    // Which symbols get a definition decides how references are written: a first pass finds
    // them, the second writes the file.
    let mut plan = Writer::new(&p, is_package);
    plan.collect();
    plan.write_package(package);
    let mut w = Writer::new(&p, is_package);
    w.collect();
    w.planned = Some(plan.addrs.keys().map(|&k| (k, ())).collect());
    w.write_package(package);
    w.patch();
    Ok(w.finish())
}

#[derive(Clone, PartialEq, Eq, Hash)]
enum NameKey {
    Simple(String),
    Qualified(u32, u32),
    ObjectClass(u32),
}

struct Writer<'a> {
    p: &'a Pickle,
    is_package: &'a dyn Fn(&str) -> bool,
    names: Vec<NameKey>,
    name_ids: FxMap<NameKey, u32>,
    out: Vec<u8>,
    syms: FxMap<u32, LocalSym>,
    /// The symbols of the pickle by owner, in the pickle's order.
    children: FxMap<u32, Vec<u32>>,
    /// Where each definition written starts; a getter of a class parameter goes by the
    /// template's parameter.
    addrs: FxMap<u32, u32>,
    fixups: Vec<(usize, u32)>,
    /// The symbols the enclosing existential types bind.
    bound: Vec<u32>,
    /// The enclosing method and polymorphic types: the address of each and the symbols of
    /// its parameters, which references inside go to by position.
    binders: Vec<(u32, Vec<u32>)>,
    packages: FxMap<u32, bool>,
    /// The symbols the file defines, known from the first pass.
    planned: Option<FxMap<u32, ()>>,
    depth: u32,
}

const MAX_DEPTH: u32 = 100;

impl<'a> Writer<'a> {
    fn new(p: &'a Pickle, is_package: &'a dyn Fn(&str) -> bool) -> Writer<'a> {
        Writer {
            p,
            is_package,
            names: Vec::new(),
            name_ids: FxMap::default(),
            out: Vec::new(),
            syms: FxMap::default(),
            children: FxMap::default(),
            addrs: FxMap::default(),
            fixups: Vec::new(),
            bound: Vec::new(),
            binders: Vec::new(),
            packages: FxMap::default(),
            planned: None,
            depth: 0,
        }
    }

    fn collect(&mut self) {
        for i in 0..self.p.len() as u32 {
            let Some(s) = self.p.local_sym(i) else { continue };
            // Scalac 2 pickles a type parameter again where another class's signature names it.
            let duplicate = s.tag == pk::TYPESYM
                && s.flags & f::PARAM != 0
                && self.children.get(&s.owner).map_or(false, |cs| cs.iter().any(|c| self.syms[c].name == s.name));
            if duplicate {
                continue;
            }
            self.children.entry(s.owner).or_default().push(i);
            self.syms.insert(i, s);
        }
    }

    // ---- names ----

    fn name_id(&mut self, key: NameKey) -> u32 {
        if let Some(&id) = self.name_ids.get(&key) {
            return id;
        }
        let id = self.names.len() as u32;
        self.names.push(key.clone());
        self.name_ids.insert(key, id);
        id
    }

    fn simple(&mut self, s: &str) -> u32 {
        self.name_id(NameKey::Simple(s.to_string()))
    }

    fn qualified(&mut self, path: &str) -> u32 {
        let mut id: Option<u32> = None;
        for seg in path.split('/').filter(|s| !s.is_empty()) {
            let n = self.simple(seg);
            id = Some(match id {
                None => n,
                Some(prefix) => self.name_id(NameKey::Qualified(prefix, n)),
            });
        }
        match id {
            Some(id) => id,
            None => self.simple("<empty>"),
        }
    }

    /// The source name of a pickled name entry.
    fn source_name(&self, name: u32) -> String {
        decode_name(self.p.name(name))
    }

    // ---- bytes ----

    fn byte(&mut self, b: u8) {
        self.out.push(b);
    }

    fn nat(&mut self, x: u32) {
        write_nat(&mut self.out, x as u64);
    }

    fn fixed_nat(&mut self, at: usize, x: u32) {
        let bytes = [(x >> 21) as u8 & 0x7f, (x >> 14) as u8 & 0x7f, (x >> 7) as u8 & 0x7f, (x as u8 & 0x7f) | 0x80];
        self.out[at..at + 4].copy_from_slice(&bytes);
    }

    fn long_int(&mut self, x: i64) {
        let mut digits = vec![(x & 0x7f) as u8 | 0x80];
        let mut y = x >> 7;
        loop {
            let last = digits.last().unwrap() & 0x7f;
            if (y == 0 && last & 0x40 == 0) || (y == -1 && last & 0x40 != 0) {
                break;
            }
            digits.push((y & 0x7f) as u8);
            y >>= 7;
        }
        digits.reverse();
        self.out.extend_from_slice(&digits);
    }

    /// A tree of `tag` with a length, whose body `body` writes.
    fn sized(&mut self, tag: u8, body: impl FnOnce(&mut Self)) {
        self.byte(tag);
        let at = self.out.len();
        self.out.extend_from_slice(&[0, 0, 0, 0x80]);
        body(self);
        let len = (self.out.len() - at - 4) as u32;
        self.fixed_nat(at, len);
    }

    /// The address of the definition of `sym`, patched in once it is written.
    fn addr_of(&mut self, sym: u32) {
        let at = self.out.len();
        self.out.extend_from_slice(&[0, 0, 0, 0x80]);
        self.fixups.push((at, sym));
    }

    fn define(&mut self, sym: u32) {
        let at = self.out.len() as u32;
        self.addrs.entry(sym).or_insert(at);
    }

    fn patch(&mut self) {
        for (at, sym) in std::mem::take(&mut self.fixups) {
            let addr = self.addrs.get(&sym).copied().unwrap_or(0);
            self.fixed_nat(at, addr);
        }
    }

    fn finish(mut self) -> Vec<u8> {
        let asts_name = self.simple("ASTs");
        let mut names = Vec::new();
        for key in &self.names {
            match key {
                NameKey::Simple(s) => {
                    names.push(t::name::UTF8);
                    write_nat(&mut names, s.len() as u64);
                    names.extend_from_slice(s.as_bytes());
                }
                NameKey::Qualified(a, b) => {
                    let mut body = Vec::new();
                    write_nat(&mut body, *a as u64);
                    write_nat(&mut body, *b as u64);
                    names.push(t::name::QUALIFIED);
                    write_nat(&mut names, body.len() as u64);
                    names.extend_from_slice(&body);
                }
                NameKey::ObjectClass(u) => {
                    let mut body = Vec::new();
                    write_nat(&mut body, *u as u64);
                    names.push(t::name::OBJECTCLASS);
                    write_nat(&mut names, body.len() as u64);
                    names.extend_from_slice(&body);
                }
            }
        }
        let mut file = vec![0x5c, 0xa1, 0xab, 0x1f];
        write_nat(&mut file, crate::tasty::MAJOR_VERSION as u64);
        write_nat(&mut file, crate::tasty::MINOR_VERSION as u64);
        write_nat(&mut file, 0);
        let tooling = b"teq scala2 pickle";
        write_nat(&mut file, tooling.len() as u64);
        file.extend_from_slice(tooling);
        file.extend_from_slice(&[0; 16]);
        write_nat(&mut file, names.len() as u64);
        file.extend_from_slice(&names);
        write_nat(&mut file, asts_name as u64);
        write_nat(&mut file, self.out.len() as u64);
        file.append(&mut self.out);
        file
    }

    // ---- definitions ----

    fn write_package(&mut self, package: &str) {
        let tops: Vec<u32> = {
            let mut tops: Vec<u32> = self.syms.iter().filter(|(_, s)| !self.syms.contains_key(&s.owner)).map(|(&i, _)| i).collect();
            tops.sort_unstable();
            tops
        };
        let pkg = self.qualified(package);
        self.sized(t::PACKAGE, |w| {
            w.byte(t::TERMREFPKG);
            w.nat(pkg);
            for i in tops {
                w.member(i);
            }
        });
    }

    fn member(&mut self, i: u32) {
        let s = self.syms[&i].clone();
        match s.tag {
            pk::CLASSSYM if s.flags & f::MODULE != 0 => {
                // A module class is written with its module; one without is written alone.
                let has_module = self.children[&s.owner].iter().any(|&m| self.syms[&m].tag == pk::MODULESYM && self.module_class(m) == Some(i));
                if !has_module {
                    self.class(i, true);
                }
            }
            pk::CLASSSYM => self.class(i, false),
            pk::MODULESYM => self.module(i),
            pk::ALIASSYM => self.alias(i),
            pk::TYPESYM if s.flags & (f::PARAM | f::EXISTENTIAL) == 0 => self.abstract_type(i),
            pk::VALSYM => self.val_or_def(i),
            _ => {}
        }
    }

    fn skipped(&self, s: &LocalSym) -> bool {
        let name = self.p.name(s.name);
        let is_method = s.flags & f::METHOD != 0;
        name.ends_with(' ')
            || name.starts_with('<') && name != "<init>"
            || s.flags & (f::BRIDGE | f::SPECIALIZED | f::SUPERACCESSOR | f::ARTIFACT | f::MACRO | f::EXPANDEDNAME) != 0
            || s.flags & f::PARAMACCESSOR != 0
            || (s.flags & f::PRIVATE != 0 && s.within.is_none() && name != "<init>" && s.tag != pk::CLASSSYM && self.syms.contains_key(&s.owner))
            || name.ends_with("$extension")
            || (s.flags & f::CASEACCESSOR != 0 && name.contains("$access$"))
            || (!is_method && s.tag == pk::VALSYM && s.flags & f::LOCAL != 0)
    }

    fn parents(&self, info: u32) -> Vec<u32> {
        if self.p.tag(info) != pk::CLASSINFOTPE {
            return Vec::new();
        }
        let mut c = self.p.body(info);
        c.nat();
        c.refs()
    }

    /// Whether `owner` is the companion of a case class that Scala 2 made a function
    /// (`AbstractFunction1`): its synthetic `apply` implements the function's, so it is kept as a
    /// member of its own where a scalac 3 companion's is the constructor's.
    fn is_function_case_companion(&self, owner: u32) -> bool {
        let Some(s) = self.syms.get(&owner) else { return false };
        if s.flags & f::MODULE == 0 || !self.is_case_companion(owner) {
            return false;
        }
        let (_, info) = self.poly(s.info);
        self.parents(info).into_iter().any(|p| self.is_abstract_function(p))
    }

    fn is_case_companion(&self, module_class: u32) -> bool {
        self.case_class_of(module_class).is_some()
    }

    fn case_class_of(&self, module_class: u32) -> Option<u32> {
        let s = &self.syms[&module_class];
        self.children.get(&s.owner)?.iter().copied().find(|c| {
            let k = &self.syms[c];
            k.tag == pk::CLASSSYM && k.flags & (f::MODULE | f::CASE) == f::CASE && self.p.name(k.name) == self.p.name(s.name)
        })
    }

    /// Whether `owner` is the companion of a case class whose constructor is not public: Scala
    /// 2.13 keeps the synthetic `apply` public all the same, so it is a member of its own where
    /// a scalac 3 companion's is the constructor's.
    fn is_restricted_case_companion(&self, owner: u32) -> bool {
        let Some(s) = self.syms.get(&owner) else { return false };
        if s.flags & f::MODULE == 0 {
            return false;
        }
        let Some(class) = self.case_class_of(owner) else { return false };
        self.children.get(&class).map_or(false, |members| {
            members.iter().any(|m| {
                let k = &self.syms[m];
                self.p.name(k.name) == "<init>"
                    && (k.flags & (f::PRIVATE | f::PROTECTED) != 0 || k.within.map_or(false, |w| self.p.is_sym(w) && self.p.tag(w) != pk::NONESYM))
            })
        })
    }

    fn is_abstract_function(&self, ty: u32) -> bool {
        if self.p.tag(ty) != pk::TYPEREFTPE {
            return false;
        }
        let mut c = self.p.body(ty);
        c.nat();
        let sym = c.nat() as u32;
        if self.p.tag(sym) != pk::EXTREF {
            return false;
        }
        let (name, owner) = self.p.ext_ref(sym);
        let name = self.p.name(name);
        name.strip_prefix("AbstractFunction").map_or(false, |n| n.parse::<u32>().is_ok()) && owner.map_or(false, |o| self.p.name(self.p.ext_ref(o).0) == "runtime")
    }

    fn module_class(&self, module: u32) -> Option<u32> {
        let info = self.syms[&module].info;
        if self.p.tag(info) != pk::TYPEREFTPE {
            return None;
        }
        let mut c = self.p.body(info);
        c.nat();
        let sym = c.nat() as u32;
        self.syms.contains_key(&sym).then_some(sym)
    }

    fn module(&mut self, i: u32) {
        let s = self.syms[&i].clone();
        if self.skipped(&s) {
            return;
        }
        let Some(class) = self.module_class(i) else { return };
        let name = self.simple(&self.source_name(s.name));
        self.define(i);
        self.sized(t::VALDEF, |w| {
            w.nat(name);
            w.byte(t::TYPEREFDIRECT);
            w.addr_of(class);
            w.byte(t::ELIDED);
            w.byte(t::TYPEREFDIRECT);
            w.addr_of(class);
            w.byte(t::OBJECT);
            w.byte(t::LAZY);
            w.byte(t::FINAL);
            w.modifiers(&s, Kind::Module);
        });
        self.class(class, true);
    }

    fn class(&mut self, i: u32, module: bool) {
        let s = self.syms[&i].clone();
        if self.skipped(&s) {
            return;
        }
        let source = self.source_name(s.name);
        let simple = self.simple(&source);
        let name = if module { self.name_id(NameKey::ObjectClass(simple)) } else { simple };
        let (tparams, info) = self.poly(s.info);
        let parents = self.parents(info);
        let members: Vec<u32> = self.children.get(&i).cloned().unwrap_or_default();
        let ctor = members.iter().copied().find(|m| {
            let s = &self.syms[m];
            s.flags & f::METHOD != 0 && matches!(self.p.name(s.name), "<init>" | "$init$")
        });
        self.define(i);
        self.sized(t::TYPEDEF, |w| {
            w.nat(name);
            w.sized(t::TEMPLATE, |w| {
                for &tp in &tparams {
                    w.tparam(tp);
                }
                if let Some(ctor) = ctor {
                    w.class_params(i, ctor, &members);
                }
                for &p in &parents {
                    w.ty(p);
                }
                if let Some(self_ty) = s.extra.filter(|_| !module) {
                    let n = w.simple("self");
                    w.byte(t::SELFDEF);
                    w.nat(n);
                    w.ty(self_ty);
                }
                match ctor {
                    Some(ctor) => w.def(ctor, true),
                    None => {
                        let init = w.simple("<init>");
                        w.sized(t::DEFDEF, |w| {
                            w.nat(init);
                            w.byte(t::EMPTYCLAUSE);
                            w.unit();
                        });
                    }
                }
                for &m in &members {
                    let ms = &w.syms[&m];
                    let secondary_ctor = Some(m) != ctor && ms.flags & f::METHOD != 0 && w.p.name(ms.name) == "<init>";
                    if secondary_ctor {
                        if ms.flags & f::PRIVATE == 0 {
                            w.def(m, true);
                        }
                    } else if Some(m) != ctor && !(ms.tag == pk::TYPESYM && ms.flags & f::PARAM != 0) {
                        w.member(m);
                    }
                }
            });
            w.modifiers(&s, if module { Kind::ModuleClass } else { Kind::Class });
        });
    }

    /// The parameters of the primary constructor as the template's accessors: a parameter with
    /// a public getter is a `val`, one without a private field.
    fn class_params(&mut self, class: u32, ctor: u32, members: &[u32]) {
        let info = self.syms[&ctor].info;
        let (_, mut mt) = self.poly(info);
        let is_case = self.syms[&class].flags & f::CASE != 0;
        while matches!(self.p.tag(mt), pk::METHODTPE | pk::IMPLICITMETHODTPE) {
            let mut c = self.p.body(mt);
            let res = c.nat() as u32;
            for param in c.refs() {
                let Some(ps) = self.syms.get(&param).cloned() else { continue };
                let pname = self.p.name(ps.name).to_string();
                let getter = members.iter().copied().find(|m| {
                    let g = &self.syms[m];
                    g.flags & f::PARAMACCESSOR != 0 && g.flags & f::METHOD != 0 && g.flags & (f::PRIVATE | f::LOCAL) == 0 && self.p.name(g.name) == pname
                });
                let name = self.simple(&decode_name(&pname));
                let at = self.out.len() as u32;
                if let Some(g) = getter {
                    self.addrs.entry(g).or_insert(at);
                }
                self.sized(t::PARAM, |w| {
                    w.nat(name);
                    w.ty(ps.info);
                    match getter {
                        Some(g) => {
                            let gf = w.syms[&g].flags;
                            if gf & f::STABLE == 0 {
                                w.byte(t::MUTABLE);
                            }
                            if gf & f::CASEACCESSOR != 0 || is_case {
                                w.byte(t::CASEACCESSOR);
                            }
                            if gf & f::OVERRIDE != 0 {
                                w.byte(t::OVERRIDE);
                            }
                        }
                        None => {
                            w.byte(t::PRIVATE);
                            w.byte(t::LOCAL);
                        }
                    }
                    if ps.flags & f::IMPLICIT != 0 {
                        w.byte(t::IMPLICIT);
                    }
                });
            }
            mt = res;
        }
    }

    /// The type parameters and the underlying info of a `POLYtpe`, or none and the info.
    fn poly(&self, info: u32) -> (Vec<u32>, u32) {
        if self.p.tag(info) == pk::POLYTPE {
            let mut c = self.p.body(info);
            let res = c.nat() as u32;
            return (c.refs(), res);
        }
        (Vec::new(), info)
    }

    fn tparam(&mut self, tp: u32) {
        let Some(s) = self.syms.get(&tp).cloned() else { return };
        let name = self.simple(&self.source_name(s.name));
        self.define(tp);
        self.sized(t::TYPEPARAM, |w| {
            w.nat(name);
            w.bounds(s.info);
            if s.flags & f::COVARIANT != 0 {
                w.byte(t::COVARIANT);
            }
            if s.flags & f::CONTRAVARIANT != 0 {
                w.byte(t::CONTRAVARIANT);
            }
        });
    }

    /// The bounds of a type parameter or abstract type; a higher-kinded one is bounded by a
    /// type lambda over its parameters, whose own bounds are left open.
    fn bounds(&mut self, info: u32) {
        let (params, under) = self.poly(info);
        let (lo, hi) = if self.p.tag(under) == pk::TYPEBOUNDSTPE {
            let mut c = self.p.body(under);
            (Some(c.nat() as u32), Some(c.nat() as u32))
        } else {
            (None, None)
        };
        self.sized(t::TYPEBOUNDS, |w| {
            match lo {
                Some(lo) => w.ty(lo),
                None => w.scala_type("Nothing"),
            }
            if params.is_empty() {
                match hi {
                    Some(hi) => w.ty(hi),
                    None => w.scala_type("Any"),
                }
            } else {
                w.sized(t::TYPELAMBDATYPE, |w| {
                    w.scala_type("Any");
                    for &p in &params {
                        let n = w.syms.get(&p).map(|s| s.name).map(|n| decode_name(w.p.name(n))).unwrap_or_else(|| "_".to_string());
                        w.sized(t::TYPEBOUNDS, |w| {
                            w.scala_type("Nothing");
                            w.scala_type("Any");
                        });
                        let n = w.simple(&n);
                        w.nat(n);
                    }
                });
            }
        });
    }

    fn alias(&mut self, i: u32) {
        let s = self.syms[&i].clone();
        if self.skipped(&s) {
            return;
        }
        let name = self.simple(&self.source_name(s.name));
        let (tparams, rhs) = self.poly(s.info);
        self.define(i);
        self.sized(t::TYPEDEF, |w| {
            w.nat(name);
            if tparams.is_empty() {
                w.ty(rhs);
            } else {
                w.sized(t::LAMBDATPT, |w| {
                    for &tp in &tparams {
                        w.tparam(tp);
                    }
                    w.ty(rhs);
                });
            }
            w.modifiers(&s, Kind::Type);
        });
    }

    fn abstract_type(&mut self, i: u32) {
        let s = self.syms[&i].clone();
        if self.skipped(&s) {
            return;
        }
        let name = self.simple(&self.source_name(s.name));
        self.define(i);
        self.sized(t::TYPEDEF, |w| {
            w.nat(name);
            w.bounds(s.info);
            w.modifiers(&s, Kind::Type);
        });
    }

    fn val_or_def(&mut self, i: u32) {
        let s = self.syms[&i].clone();
        if self.skipped(&s) {
            return;
        }
        let name = self.p.name(s.name);
        let getter = s.flags & f::METHOD != 0 && (s.flags & f::ACCESSOR != 0 || s.flags & f::LAZY != 0) && !name.ends_with("_$eq");
        if s.flags & f::METHOD != 0 && !getter {
            self.def(i, false);
            return;
        }
        let name = self.simple(&self.source_name(s.name));
        let (_, ty) = self.poly(s.info);
        self.define(i);
        self.sized(t::VALDEF, |w| {
            w.nat(name);
            w.ty(ty);
            if s.flags & f::DEFERRED == 0 {
                w.byte(t::ELIDED);
                w.ty(ty);
            }
            if s.flags & f::METHOD != 0 && s.flags & (f::STABLE | f::LAZY) == 0 {
                w.byte(t::MUTABLE);
            }
            w.modifiers(&s, Kind::Term);
        });
    }

    fn def(&mut self, i: u32, ctor: bool) {
        let s = self.syms[&i].clone();
        let name = if ctor { self.simple("<init>") } else { self.simple(&self.source_name(s.name)) };
        self.define(i);
        self.sized(t::DEFDEF, |w| {
            w.nat(name);
            let mut info = s.info;
            let mut terms_open = false;
            loop {
                match w.p.tag(info) {
                    pk::POLYTPE => {
                        let (tps, res) = w.poly(info);
                        for &tp in &tps {
                            w.tparam(tp);
                        }
                        terms_open = false;
                        info = res;
                        if tps.is_empty() {
                            break;
                        }
                    }
                    tag @ (pk::METHODTPE | pk::IMPLICITMETHODTPE) => {
                        let mut c = w.p.body(info);
                        let res = c.nat() as u32;
                        let params = c.refs();
                        if params.is_empty() {
                            w.byte(t::EMPTYCLAUSE);
                            terms_open = false;
                        } else {
                            if terms_open {
                                w.byte(t::SPLITCLAUSE);
                            }
                            for p in params {
                                w.param(p, tag == pk::IMPLICITMETHODTPE);
                            }
                            terms_open = true;
                        }
                        info = res;
                    }
                    _ => break,
                }
            }
            if ctor {
                w.unit();
            } else {
                w.ty(info);
            }
            if s.flags & f::DEFERRED == 0 {
                w.byte(t::ELIDED);
                if ctor {
                    w.unit();
                } else {
                    w.ty(info);
                }
            }
            w.modifiers(&s, Kind::Term);
        });
    }

    fn param(&mut self, p: u32, implicit: bool) {
        let Some(s) = self.syms.get(&p).cloned() else { return };
        let name = self.simple(&self.source_name(s.name));
        self.define(p);
        self.sized(t::PARAM, |w| {
            w.nat(name);
            w.ty(s.info);
            if implicit || s.flags & f::IMPLICIT != 0 {
                w.byte(t::IMPLICIT);
            }
            if s.flags & f::TRAIT != 0 {
                w.byte(t::HASDEFAULT);
            }
        });
    }

    fn modifiers(&mut self, s: &LocalSym, kind: Kind) {
        let fl = s.flags;
        let qualified = s.within.filter(|&w| self.p.is_sym(w) && self.p.tag(w) != pk::NONESYM);
        if qualified.is_none() {
            if fl & f::PRIVATE != 0 {
                self.byte(t::PRIVATE);
            }
            if fl & f::PROTECTED != 0 {
                self.byte(t::PROTECTED);
            }
        }
        let class = matches!(kind, Kind::Class | Kind::ModuleClass);
        let trait_ = class && fl & f::TRAIT != 0;
        let simple: [(u64, u8, bool); 9] = [
            (f::ABSTRACT, t::ABSTRACT, class && !trait_),
            (f::FINAL, t::FINAL, kind != Kind::Module),
            (f::SEALED, t::SEALED, class),
            (f::CASE, t::CASE, class),
            (f::IMPLICIT, t::IMPLICIT, true),
            (f::LAZY, t::LAZY, kind == Kind::Term),
            (f::OVERRIDE, t::OVERRIDE, kind == Kind::Term),
            (f::TRAIT, t::TRAIT, class),
            (f::CASEACCESSOR, t::CASEACCESSOR, kind == Kind::Term),
        ];
        for (flag, tag, applies) in simple {
            if applies && fl & flag != 0 {
                self.byte(tag);
            }
        }
        if kind == Kind::ModuleClass {
            self.byte(t::OBJECT);
        }
        // The conversion of an implicit class is a definition of the source's, as scalac 3 has it.
        let default_getter = self.p.name(s.name).contains("$default$");
        let conversion = fl & (f::IMPLICIT | f::METHOD) == f::IMPLICIT | f::METHOD;
        let function_apply = kind == Kind::Term
            && self.p.name(s.name) == "apply"
            && (self.is_function_case_companion(s.owner) || self.is_restricted_case_companion(s.owner));
        if fl & f::SYNTHETIC != 0 && !default_getter && !conversion && !function_apply {
            self.byte(t::SYNTHETIC);
        }
        if let Some(within) = qualified {
            self.byte(if fl & f::PROTECTED != 0 { t::PROTECTEDQUALIFIED } else { t::PRIVATEQUALIFIED });
            self.this_prefix(within);
        }
    }

    // ---- types ----

    fn unit(&mut self) {
        self.scala_type("Unit");
    }

    fn scala_type(&mut self, name: &str) {
        let n = self.simple(name);
        let scala = self.simple("scala");
        self.byte(t::TYPEREF);
        self.nat(n);
        self.byte(t::TERMREFPKG);
        self.nat(scala);
    }

    fn unsupported(&mut self, what: &str) {
        let n = self.simple(what);
        self.sized(UNSUPPORTED, |w| {
            w.byte(t::STRINGCONST);
            w.nat(n);
        });
    }

    fn ty(&mut self, i: u32) {
        if self.depth > MAX_DEPTH {
            self.unsupported("a Scala 2 type nested too deeply");
            return;
        }
        self.depth += 1;
        self.ty_now(i);
        self.depth -= 1;
    }

    fn ty_now(&mut self, i: u32) {
        let mut c = self.p.body(i);
        match self.p.tag(i) {
            pk::THISTPE => {
                let sym = c.nat() as u32;
                self.this_prefix(sym);
            }
            pk::SINGLETPE => {
                let pre = c.nat() as u32;
                let sym = c.nat() as u32;
                self.term_ref(pre, sym);
            }
            pk::CONSTANTTPE => {
                let k = c.nat() as u32;
                self.constant(k);
            }
            pk::TYPEREFTPE => {
                let pre = c.nat() as u32;
                let sym = c.nat() as u32;
                let args = c.refs();
                // scalac 2 names a package in a prefix by its package class.
                if args.is_empty() && self.package_path(sym).is_some() {
                    self.this_prefix(sym);
                } else {
                    self.type_ref(pre, sym, &args);
                }
            }
            pk::TYPEBOUNDSTPE => {
                let lo = c.nat() as u32;
                let hi = c.nat() as u32;
                self.sized(t::TYPEBOUNDS, |w| {
                    w.ty(lo);
                    w.ty(hi);
                });
            }
            pk::REFINEDTPE => {
                let class = c.nat() as u32;
                let parents = c.refs();
                self.refined(class, &parents);
            }
            pk::ANNOTATEDTPE => {
                let refs = c.refs();
                let under = if refs.len() > 1 && self.p.is_sym(refs[0]) { refs[1] } else { refs.first().copied().unwrap_or(0) };
                self.ty(under);
            }
            pk::EXISTENTIALTPE => {
                let res = c.nat() as u32;
                let syms = c.refs();
                let n = self.bound.len();
                self.bound.extend(syms);
                self.ty(res);
                self.bound.truncate(n);
            }
            pk::POLYTPE => {
                let (params, res) = self.poly(i);
                if params.is_empty() {
                    self.ty(res);
                } else {
                    self.lambda(t::POLYTYPE, res, &params, false);
                }
            }
            tag @ (pk::METHODTPE | pk::IMPLICITMETHODTPE) => {
                let res = c.nat() as u32;
                let params = c.refs();
                let implicit = tag == pk::IMPLICITMETHODTPE || params.first().and_then(|p| self.syms.get(p)).map_or(false, |s| s.flags & f::IMPLICIT != 0);
                self.lambda(t::METHODTYPE, res, &params, implicit);
            }
            pk::SUPERTPE => {
                let this = c.nat() as u32;
                let sup = c.nat() as u32;
                self.sized(t::SUPERTYPE, |w| {
                    w.ty(this);
                    w.ty(sup);
                });
            }
            pk::NOPREFIXTPE => self.scala_type("Any"),
            other => self.unsupported(&format!("the Scala 2 type tag {}", other)),
        }
    }

    /// A method or polymorphic type of a refinement's member: its parameters as a TASTy lambda
    /// type, which references in the result name by position.
    fn lambda(&mut self, tag: u8, res: u32, params: &[u32], implicit: bool) {
        let binder = self.out.len() as u32;
        self.binders.push((binder, params.to_vec()));
        self.sized(tag, |w| {
            w.ty(res);
            for &p in params {
                let Some(s) = w.syms.get(&p).cloned() else { continue };
                if tag == t::POLYTYPE {
                    w.bounds(s.info);
                } else {
                    w.ty(s.info);
                }
                let n = w.simple(&decode_name(w.p.name(s.name)));
                w.nat(n);
            }
            if implicit {
                w.byte(t::IMPLICIT);
            }
        });
        self.binders.pop();
    }

    /// A reference to a parameter of an enclosing lambda type.
    fn param_ref(&mut self, sym: u32) -> bool {
        let Some((binder, num)) = self.binders.iter().rev().find_map(|(b, ps)| ps.iter().position(|&p| p == sym).map(|n| (*b, n as u32))) else {
            return false;
        };
        self.sized(t::PARAMTYPE, |w| {
            w.nat(binder);
            w.nat(num);
        });
        true
    }

    fn constant(&mut self, k: u32) {
        let mut c = self.p.body(k);
        match self.p.tag(k) {
            pk::LITERALUNIT => self.byte(t::UNITCONST),
            pk::LITERALBOOLEAN => self.byte(if c.long() != 0 { t::TRUECONST } else { t::FALSECONST }),
            pk::LITERALBYTE | pk::LITERALSHORT | pk::LITERALINT | pk::LITERALLONG | pk::LITERALFLOAT | pk::LITERALDOUBLE => {
                let v = c.long();
                let tag = match self.p.tag(k) {
                    pk::LITERALBYTE => t::BYTECONST,
                    pk::LITERALSHORT => t::SHORTCONST,
                    pk::LITERALINT => t::INTCONST,
                    pk::LITERALLONG => t::LONGCONST,
                    pk::LITERALFLOAT => t::FLOATCONST,
                    _ => t::DOUBLECONST,
                };
                self.byte(tag);
                self.long_int(if tag == t::FLOATCONST { v as i32 as u32 as i64 } else { v });
            }
            pk::LITERALCHAR => {
                let v = c.long();
                self.byte(t::CHARCONST);
                self.nat(v as u16 as u32);
            }
            pk::LITERALSTRING => {
                let name = c.nat() as u32;
                let s = self.p.name(name).to_string();
                let n = self.simple(&s);
                self.byte(t::STRINGCONST);
                self.nat(n);
            }
            pk::LITERALNULL => self.byte(t::NULLCONST),
            pk::LITERALCLASS => {
                let ty = c.nat() as u32;
                self.byte(t::CLASSCONST);
                self.ty(ty);
            }
            pk::LITERALENUM => {
                let sym = c.nat() as u32;
                let none = self.p.len() as u32;
                self.term_ref(none, sym);
            }
            _ => self.unsupported("a Scala 2 constant"),
        }
    }

    fn type_ref(&mut self, pre: u32, sym: u32, args: &[u32]) {
        if self.is_scala_member(sym, "<byname>") && args.len() == 1 {
            self.byte(t::BYNAMETYPE);
            self.ty(args[0]);
            return;
        }
        if self.bound.contains(&sym) {
            // A type an existential binds, outside the arguments where it reads as a wildcard.
            match self.syms.get(&sym).map(|s| s.info) {
                Some(info) if self.p.tag(info) == pk::TYPEBOUNDSTPE => {
                    let mut c = self.p.body(info);
                    c.nat();
                    let hi = c.nat() as u32;
                    self.ty(hi);
                }
                _ => self.scala_type("Any"),
            }
            return;
        }
        if args.is_empty() {
            self.type_ctor(pre, sym);
            return;
        }
        self.sized(t::APPLIEDTYPE, |w| {
            w.type_ctor(pre, sym);
            for &a in args {
                w.type_arg(a);
            }
        });
    }

    fn type_arg(&mut self, a: u32) {
        if self.p.tag(a) == pk::TYPEREFTPE {
            let mut c = self.p.body(a);
            c.nat();
            let sym = c.nat() as u32;
            if c.at_end() && self.bound.contains(&sym) {
                if let Some(info) = self.syms.get(&sym).map(|s| s.info).filter(|&i| self.p.tag(i) == pk::TYPEBOUNDSTPE) {
                    self.ty(info);
                    return;
                }
            }
        }
        self.ty(a);
    }

    fn type_ctor(&mut self, pre: u32, sym: u32) {
        if self.param_ref(sym) {
            return;
        }
        // A type member is seen through its prefix (`this.Out` in a subclass).
        let member = self.syms.get(&sym).map_or(false, |s| matches!(s.tag, pk::TYPESYM | pk::ALIASSYM) && s.flags & f::PARAM == 0);
        if self.defines(sym) && member && matches!(self.p.tag(pre), pk::THISTPE | pk::SINGLETPE) {
            self.byte(t::TYPEREFSYMBOL);
            self.addr_of(sym);
            self.ty(pre);
            return;
        }
        if self.defines(sym) {
            self.byte(t::TYPEREFDIRECT);
            self.addr_of(sym);
            return;
        }
        let tag = self.p.tag(sym);
        let raw_name = if matches!(tag, pk::EXTREF | pk::EXTMODCLASSREF) { self.p.ext_ref(sym).0 } else { self.p.sym_name(sym) };
        let simple = self.simple(&decode_name(self.p.name(raw_name)));
        let module_class = tag == pk::EXTMODCLASSREF || self.syms.get(&sym).map_or(false, |s| s.tag == pk::CLASSSYM && s.flags & f::MODULE != 0);
        let name = if module_class { self.name_id(NameKey::ObjectClass(simple)) } else { simple };
        self.byte(t::TYPEREF);
        self.nat(name);
        self.prefix(pre, sym);
    }

    fn term_ref(&mut self, pre: u32, sym: u32) {
        if self.param_ref(sym) {
            return;
        }
        if self.defines(sym) {
            self.byte(t::TERMREFDIRECT);
            self.addr_of(sym);
            return;
        }
        if matches!(self.p.tag(sym), pk::EXTREF | pk::EXTMODCLASSREF) && self.package_path(sym).is_some() {
            self.this_prefix(sym);
            return;
        }
        let raw_name = if matches!(self.p.tag(sym), pk::EXTREF | pk::EXTMODCLASSREF) { self.p.ext_ref(sym).0 } else { self.p.sym_name(sym) };
        let name = self.simple(&decode_name(self.p.name(raw_name)));
        self.byte(t::TERMREF);
        self.nat(name);
        self.prefix(pre, sym);
    }

    /// The prefix of a reference to `sym`: what the pickle states, or `sym`'s owner.
    fn prefix(&mut self, pre: u32, sym: u32) {
        match self.p.tag(pre) {
            pk::THISTPE | pk::SINGLETPE | pk::TYPEREFTPE | pk::REFINEDTPE | pk::EXISTENTIALTPE | pk::ANNOTATEDTPE => self.ty(pre),
            _ => match self.p.owner_of(sym) {
                Some(o) if self.p.is_sym(o) && self.p.tag(o) != pk::NONESYM => self.owner_prefix(o),
                _ => self.root(),
            },
        }
    }

    fn root(&mut self) {
        let n = self.simple("_root_");
        self.byte(t::TERMREFPKG);
        self.nat(n);
    }

    /// `C.this` for a class, the path of a package or an object, as a prefix.
    fn this_prefix(&mut self, sym: u32) {
        if self.defines(sym) {
            self.byte(t::THIS);
            self.byte(t::TYPEREFDIRECT);
            self.addr_of(sym);
            return;
        }
        if let Some(s) = self.syms.get(&sym).cloned() {
            let n = self.simple(&self.source_name(s.name));
            if s.tag == pk::CLASSSYM && s.flags & f::MODULE == 0 {
                self.byte(t::THIS);
                self.byte(t::TYPEREF);
            } else {
                self.byte(t::TERMREF);
            }
            self.nat(n);
            self.owner_prefix(s.owner);
            return;
        }
        if let Some(path) = self.package_path(sym) {
            if path.is_empty() {
                self.root();
                return;
            }
            let n = self.qualified(&path);
            self.byte(t::TERMREFPKG);
            self.nat(n);
            return;
        }
        let (name, owner) = self.p.ext_ref(sym);
        let text = decode_name(self.p.name(name));
        if self.p.tag(sym) == pk::EXTMODCLASSREF || !self.p.is_type_name(name) {
            let n = self.simple(&text);
            self.byte(t::TERMREF);
            self.nat(n);
        } else {
            let n = self.simple(&text);
            self.byte(t::THIS);
            self.byte(t::TYPEREF);
            self.nat(n);
        }
        match owner {
            Some(o) => self.owner_prefix(o),
            None => self.root(),
        }
    }

    /// The owner of a symbol as the prefix of a reference to it: a package or object as its
    /// path, a class as a projection.
    fn owner_prefix(&mut self, o: u32) {
        if self.syms.contains_key(&o) || self.package_path(o).is_some() || self.p.tag(o) == pk::EXTMODCLASSREF {
            self.this_prefix(o);
            return;
        }
        let (name, owner) = self.p.ext_ref(o);
        let n = self.simple(&decode_name(self.p.name(name)));
        self.byte(if self.p.is_type_name(name) { t::TYPEREF } else { t::TERMREF });
        self.nat(n);
        match owner {
            Some(o) => self.owner_prefix(o),
            None => self.root(),
        }
    }

    /// The path of the package `sym` stands for (`kantan/csv`), when it is one.
    fn package_path(&mut self, sym: u32) -> Option<String> {
        if !matches!(self.p.tag(sym), pk::EXTREF | pk::EXTMODCLASSREF) {
            return None;
        }
        let (name, owner) = self.p.ext_ref(sym);
        let text = self.p.name(name);
        if matches!(text, "<root>" | "_root_") && owner.is_none() {
            return Some(String::new());
        }
        if text == "<empty>" {
            return Some(String::new());
        }
        if self.p.tag(sym) == pk::EXTREF && self.p.is_type_name(name) {
            return None;
        }
        let parent = match owner {
            Some(o) => self.package_path(o)?,
            None => String::new(),
        };
        let path = if parent.is_empty() { text.to_string() } else { format!("{}/{}", parent, text) };
        if let Some(&known) = self.packages.get(&sym) {
            return known.then_some(path);
        }
        let is = (self.is_package)(&path);
        self.packages.insert(sym, is);
        is.then_some(path)
    }

    fn is_scala_member(&self, sym: u32, name: &str) -> bool {
        if self.p.tag(sym) != pk::EXTREF {
            return false;
        }
        let (n, owner) = self.p.ext_ref(sym);
        self.p.name(n) == name && owner.map_or(false, |o| self.p.name(self.p.ext_ref(o).0) == "scala")
    }

    /// Whether the file has a definition of `sym` that a reference can go to.
    fn defines(&self, sym: u32) -> bool {
        match &self.planned {
            Some(planned) => planned.contains_key(&sym),
            None => self.syms.contains_key(&sym),
        }
    }

    fn refined(&mut self, class: u32, parents: &[u32]) {
        let decls: Vec<u32> = self.children.get(&class).cloned().unwrap_or_default();
        let readable = decls.iter().all(|d| {
            let s = &self.syms[d];
            match s.tag {
                pk::ALIASSYM | pk::TYPESYM => self.p.tag(s.info) != pk::POLYTPE,
                pk::VALSYM => true,
                _ => false,
            }
        });
        if !readable {
            self.unsupported("a Scala 2 refinement with a type constructor member");
            return;
        }
        self.refinements(parents, &decls);
    }

    fn refinements(&mut self, parents: &[u32], decls: &[u32]) {
        let Some((&last, rest)) = decls.split_last() else {
            self.and(parents);
            return;
        };
        let s = self.syms[&last].clone();
        let name = self.simple(&self.source_name(s.name));
        self.sized(t::REFINEDTYPE, |w| {
            w.nat(name);
            w.refinements(parents, rest);
            match s.tag {
                pk::ALIASSYM => w.sized(t::TYPEBOUNDS, |w| w.ty(s.info)),
                pk::TYPESYM => w.bounds(s.info),
                _ if s.flags & f::METHOD != 0 && w.p.tag(s.info) == pk::POLYTPE && w.poly(s.info).0.is_empty() => {
                    let (_, ty) = w.poly(s.info);
                    w.byte(t::BYNAMETYPE);
                    w.ty(ty);
                }
                _ => w.ty(s.info),
            }
        });
    }

    fn and(&mut self, parents: &[u32]) {
        match parents {
            [] => self.scala_type("Any"),
            [one] => self.ty(*one),
            [first, rest @ ..] => self.sized(t::ANDTYPE, |w| {
                w.ty(*first);
                w.and(rest);
            }),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Class,
    ModuleClass,
    Module,
    Type,
    Term,
}

fn write_nat(out: &mut Vec<u8>, x: u64) {
    let mut digits = [0u8; 10];
    let mut n = 0;
    let mut y = x;
    loop {
        digits[n] = (y & 0x7f) as u8;
        n += 1;
        y >>= 7;
        if y == 0 {
            break;
        }
    }
    digits[0] |= 0x80;
    for i in (0..n).rev() {
        out.push(digits[i]);
    }
}

/// A name as scalac 2 encodes it (`$plus$plus`), back in source form (`++`), as
/// `scala.reflect.NameTransformer.decode` does for the operators.
pub fn decode_name(name: &str) -> String {
    if !name.contains('$') {
        return name.to_string();
    }
    const OPS: [(&str, char); 18] = [
        ("$tilde", '~'),
        ("$eq", '='),
        ("$less", '<'),
        ("$greater", '>'),
        ("$bang", '!'),
        ("$hash", '#'),
        ("$percent", '%'),
        ("$up", '^'),
        ("$amp", '&'),
        ("$bar", '|'),
        ("$times", '*'),
        ("$div", '/'),
        ("$plus", '+'),
        ("$minus", '-'),
        ("$colon", ':'),
        ("$bslash", '\\'),
        ("$qmark", '?'),
        ("$at", '@'),
    ];
    let mut out = String::with_capacity(name.len());
    let mut rest = name;
    while let Some(at) = rest.find('$') {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        match OPS.iter().find(|(code, _)| rest.starts_with(code)) {
            Some((code, c)) => {
                out.push(*c);
                rest = &rest[code.len()..];
            }
            None => {
                out.push('$');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}
