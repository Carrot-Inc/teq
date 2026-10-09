//! The declarations a library body's selections name: a `SELECTin` states the class that
//! declares the member, its name as declared
//! and its signature. The converter keeps the three (`DeclRef`); the typer resolves them when it
//! types the selection, as `TreeUnpickler` does, and calls the member it finds without looking
//! the name up on the receiver again.

use super::super::Worker;
use super::bodies::Conv;
use super::super::apply::{ArgList, Callee, StaticPath};
use crate::ast::{DeclRef, Expr, ExprId, ListRef};
use crate::source::Span;
use crate::tir::{TExpr, TExprId};
use crate::intern::Name;
use crate::names;
use crate::symbols::*;
use crate::tasty::tree::{Clause, Decoder, TType};
use crate::tasty::{NameRef, TName};
use crate::types::*;
use super::compile::STD_BINDINGS;

/// What a declaration reference resolved to, memoised per reference in the loader's tables.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Declared {
    /// A member the owner declares.
    Own(SymId),
    /// A member of one of the owner's supers, where the owner declares none that matches: moved
    /// up, or an override removed since the body was compiled.
    Inherited(SymId),
    /// Selected by name on the receiver, as the typer selects in source, for the reason given.
    ByName(ByName),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ByName {
    /// The owner is a builtin (`Any`, `AnyRef`, the numbers, `String`) or a class of the body.
    Builtin,
    /// The owner stands for a class of teq's std (the std adaptation A1), whose
    /// signatures are not scala-library's.
    StdOwner,
    /// The owner is a Java class: the platform layer or the JDK's class file.
    JavaOwner,
    /// No member of the owner or its supers matches the name and the signature.
    Missing,
    /// Several members match: the erasure the typer compares does not tell them apart.
    Ambiguous,
    /// An inline method, which its expansion resolves.
    Inline,
}

/// What a library body's selection took, for the listing: the declared alternative of an
/// overloaded name, or the one member the receiver has of the name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Took {
    Declared(SymId),
    One(SymId),
}

impl Declared {
    pub fn symbol(self) -> Option<SymId> {
        match self {
            Declared::Own(s) | Declared::Inherited(s) => Some(s),
            Declared::ByName(_) => None,
        }
    }
}

impl<'a> Worker<'a> {
    /// The declaration a `SELECTin` names, for the typer to resolve: its signed name and where
    /// its owner's type is. Nothing is looked up here, so that converting a class reads no file
    /// its bodies' owners name.
    pub(super) fn declared_in(&mut self, cv: &mut Conv, n: NameRef, owner_at: crate::tasty::tree::Addr) -> Option<DeclRef> {
        matches!(cv.tasty().names.get(n as usize), Some(TName::Signed { .. })).then_some(DeclRef { file: cv.file(), name: n, owner_at })
    }

    /// The member a library body's selection calls, resolved once per reference under the
    /// loader's lock; `None` where the typer selects by name.
    pub(in crate::typer) fn declared_member(&mut self, d: DeclRef) -> Option<SymId> {
        if let Some(&r) = self.loaded.as_ref()?.declared.get(&d) {
            return r.symbol();
        }
        self.with_loader_for(crate::measure::Hold::JarLookup, |w| w.resolve_declaration(d)).symbol()
    }

    pub(in crate::typer) fn resolve_declaration(&mut self, d: DeclRef) -> Declared {
        if let Some(&r) = self.loaded.as_ref().unwrap().declared.get(&d) {
            return r;
        }
        let r = self.look_up_declaration(d);
        self.loaded_mut().declared.insert(d, r);
        r
    }

    /// The class a declaration's owner type names: `None` for a builtin (`Any`, the numbers),
    /// whose members the typer computes, and for a class of the body itself.
    fn declaration_owner(&mut self, d: DeclRef) -> Option<ClassId> {
        let tasty = self.tasty(d.file);
        let owner = Decoder::new(&tasty).type_at(d.owner_at);
        let entered = &self.file_tables(d.file).classes;
        if mentions_local(&owner, &|addr| entered.contains_key(&addr)) {
            return None;
        }
        let mut cx = super::MapCx::new(d.file);
        let ctor = self.map_type_ctor(&mut cx, &owner);
        match self.types.get(ctor) {
            Type::Class(k, _) | Type::Ctor(k) if self.syms.class(k).kind != ClassKind::Builtin => Some(k),
            _ => None,
        }
    }

    fn look_up_declaration(&mut self, d: DeclRef) -> Declared {
        let Some(owner) = self.declaration_owner(d) else { return Declared::ByName(ByName::Builtin) };
        if !self.is_library_class(owner) {
            let java = self.is_java_class(owner) || self.is_java_placeholder(owner);
            return Declared::ByName(if java { ByName::JavaOwner } else { ByName::StdOwner });
        }
        let tasty = self.tasty(d.file);
        let Some(TName::Signed { original, target, params, .. }) = tasty.names.get(d.name as usize) else {
            return Declared::ByName(ByName::Missing);
        };
        let (original, target, params) = (*original, *target, params.clone());
        let name = self.lname(&tasty, original);
        let target = target.map(|t| self.lname(&tasty, t));
        let wanted: Vec<SigParam> = params
            .iter()
            .map(|&p| if p < 0 { SigParam::Types(-p as usize) } else { SigParam::Term(self.pickled_sig_name(&tasty.name(p as u32))) })
            .collect();
        self.complete_class(owner);
        let supers: Vec<ClassId> = self.syms.class(owner).base_types.iter().map(|&(b, _)| b).filter(|&b| b != owner).collect();
        for (i, k) in std::iter::once(owner).chain(supers).enumerate() {
            let found = self.declared_alternatives(k, name, target, &wanted);
            match found[..] {
                [] => continue,
                // An inline method is resolved where it expands, to the receiver's own override of
                // it when it is deferred (Magnolia's `deriveSubtype`), as scalac's inliner does.
                [s] if self.syms.sym(s).mods & crate::ast::mods::INLINE != 0 => return Declared::ByName(ByName::Inline),
                [s] if i == 0 => return Declared::Own(s),
                [s] => return Declared::Inherited(s),
                _ => return Declared::ByName(ByName::Ambiguous),
            }
        }
        Declared::ByName(ByName::Missing)
    }

    /// The members named by the signed name `n` that the class `k` declares itself, for a
    /// document's occurrence index (`occurrences.rs`); `None` for a name that is not signed.
    pub(super) fn signed_alternatives(&mut self, k: ClassId, tasty: &crate::tasty::TastyFile, n: NameRef) -> Option<Vec<SymId>> {
        let Some(TName::Signed { original, target, params, .. }) = tasty.names.get(n as usize) else { return None };
        let (original, target, params) = (*original, *target, params.clone());
        let name = self.lname(tasty, original);
        let target = target.map(|t| self.lname(tasty, t));
        let wanted: Vec<SigParam> = params
            .iter()
            .map(|&p| if p < 0 { SigParam::Types(-p as usize) } else { SigParam::Term(self.pickled_sig_name(&tasty.name(p as u32))) })
            .collect();
        Some(self.declared_alternatives(k, name, target, &wanted))
    }

    /// The members named `name` that the class `k` declares itself whose target name and erased
    /// parameters are those of the signature.
    fn declared_alternatives(&mut self, k: ClassId, name: Name, target: Option<Name>, wanted: &[SigParam]) -> Vec<SymId> {
        self.complete_class(k);
        let Some(&m) = self.syms.class(k).members.get(&name) else { return Vec::new() };
        let alternatives: Vec<SymId> = match self.syms.alternatives(m) {
            Some(alts) => alts.to_vec(),
            None => vec![m],
        };
        let type_params: usize = wanted.iter().map(|p| if let SigParam::Types(n) = p { *n } else { 0 }).sum();
        let term_params = wanted.iter().filter(|p| matches!(p, SigParam::Term(_))).count();
        // The target name and the parameters' counts, read from the member's own TASTy where the
        // loader entered it, which maps none of its types.
        let mut counted = Vec::new();
        for s in alternatives {
            if self.syms.sym(s).owner != Owner::Class(k) || !matches!(self.syms.sym(s).kind, SymKind::Def | SymKind::Val | SymKind::Var | SymKind::Given) {
                continue;
            }
            let shape = match self.loaded.as_ref().and_then(|l| l.syms.get(&s).copied()) {
                Some(ls) => {
                    let tasty = self.tasty(ls.file);
                    let sig = Decoder::new(&tasty).def_sig(ls.addr);
                    let own_target = sig.mods.target_name.map(|t| self.lname(&tasty, t));
                    let types: usize = sig.clauses.iter().map(|c| if let Clause::Types(ps) = c { ps.len() } else { 0 }).sum();
                    let terms: usize = sig.clauses.iter().map(|c| if let Clause::Terms(ps) = c { ps.len() } else { 0 }).sum();
                    (own_target, types, terms)
                }
                None => {
                    let sig = self.sig_arc(s);
                    let own_target = self.loaded.as_ref().and_then(|l| l.target_names.get(&s).copied());
                    (own_target, sig.tparams.len(), sig.clauses.iter().map(|c| c.params.len()).sum())
                }
            };
            let (own_target, types, terms) = shape;
            if own_target.filter(|&t| t != name) == target.filter(|&t| t != name) && types == type_params && terms == term_params {
                counted.push(s);
            }
        }
        let mut out = Vec::new();
        for s in counted {
            let sig = self.sig_arc(s);
            if self.signature_matches(&sig, wanted, self.is_java_defined(s)) {
                out.push(s);
            }
        }
        out
    }

    /// The alternatives whose parameters and result erase as `signature` states, a callee's as
    /// the origins record it (`params:result`, a clause of type parameters `[n]`; two
    /// `@targetName` overloads may differ in the result alone); every one where the record has
    /// none.
    pub(in crate::typer) fn alternatives_of_signature(&mut self, alternatives: &[SymId], signature: &str) -> Vec<SymId> {
        let Some((params, result)) = signature.rsplit_once(':') else { return alternatives.to_vec() };
        let result = self.pickled_sig_name(result);
        let wanted: Vec<SigParam> = params
            .split(',')
            .filter(|p| !p.is_empty())
            .map(|p| match p.strip_prefix('[').and_then(|n| n.strip_suffix(']')).and_then(|n| n.parse().ok()) {
                Some(n) => SigParam::Types(n),
                None => SigParam::Term(self.pickled_sig_name(p)),
            })
            .collect();
        let type_params: usize = wanted.iter().map(|p| if let SigParam::Types(n) = p { *n } else { 0 }).sum();
        let mut out = Vec::new();
        for &s in alternatives {
            let sig = self.sig_arc(s);
            let result_matches = match (&result, self.sig_name(sig.ret, false, 0, None)) {
                (Some(w), Some(have)) => *w == have,
                _ => true,
            };
            if sig.tparams.len() == type_params && result_matches && self.signature_matches(&sig, &wanted, self.is_java_defined(s)) {
                out.push(s);
            }
        }
        out
    }

    /// Whether a member's parameters erase as the signature states: as many term parameters,
    /// each erasing to the class the signature names, where teq can tell it.
    fn signature_matches(&mut self, sig: &MethodSig, wanted: &[SigParam], java: bool) -> bool {
        let terms: Vec<&Option<String>> = wanted.iter().filter_map(|p| if let SigParam::Term(n) = p { Some(n) } else { None }).collect();
        let params: Vec<ParamSig> = sig.clauses.iter().flat_map(|c| c.params.iter().cloned()).collect();
        if terms.len() != params.len() {
            return false;
        }
        params.iter().zip(terms).all(|(p, want)| match (want, self.param_sig_name(p, java)) {
            (Some(w), Some(have)) => *w == have,
            _ => true,
        })
    }

    /// A signature's parameter name as teq names the class: a class of scala-library the std
    /// defines in another package under the std's name (`scala.collection.immutable.List` as
    /// `scala.List`); `None` for scalac's placeholder of a type it could not erase.
    fn pickled_sig_name(&mut self, pickled: &str) -> Option<String> {
        if pickled.is_empty() || pickled == "?" || pickled.starts_with('<') {
            return None;
        }
        Some(self.std_sig_name(pickled))
    }

    fn std_sig_name(&mut self, name: &str) -> String {
        if !self.std_binds() {
            return name.to_string();
        }
        let base = name.trim_end_matches("[]");
        let arrays = &name[base.len()..];
        let Some((pkg, simple)) = base.rsplit_once('.') else { return name.to_string() };
        match STD_BINDINGS.iter().find(|&&(from, n, _)| from == pkg && n == simple) {
            Some(&(_, _, to)) => format!("{}.{}{}", to, simple, arrays),
            None => name.to_string(),
        }
    }

    /// The name a parameter erases to in a signature, as scalac's `TypeErasure.sigName` writes
    /// it (`java.lang.Object`, `scala.Int[]`, `pe.A$.Token`); `None` where teq's type does not
    /// tell (a union, an intersection, a type it cannot read).
    fn param_sig_name(&mut self, p: &ParamSig, java: bool) -> Option<String> {
        if p.by_name {
            return Some("scala.Function0".to_string());
        }
        if p.repeated && java {
            return self.java_varargs_sig_name(p.ty, None).map(|n| self.std_sig_name(&n));
        }
        if p.repeated {
            return Some(self.std_sig_name("scala.collection.immutable.Seq"));
        }
        self.sig_name(p.ty, false, 0, None)
    }

    /// `in_array`: the type is an array's element, where a value class erases to what its
    /// class wraps, its arguments not substituted (`Array[Wrap[String]]` is `Object[]`).
    fn sig_name(&mut self, t: TypeId, in_array: bool, depth: u32, lib: Option<&str>) -> Option<String> {
        if depth > 16 {
            return None;
        }
        if lib.is_some() && self.names_js_undef_or(t) {
            return Some("scala.scalajs.js.|".to_string());
        }
        let t = self.dealias(t);
        match self.types.get(t) {
            Type::Any => Some("java.lang.Object".to_string()),
            Type::Nothing => Some("scala.Nothing".to_string()),
            Type::Class(c, args) => {
                if c == self.b.array {
                    let elem = *self.types.items(args).first()?;
                    return self.array_sig_name(elem, depth, lib);
                }
                let info = self.syms.class(c);
                if info.kind == ClassKind::Opaque {
                    self.complete_class(c);
                    let info = self.syms.class(c);
                    let under = info.underlying?;
                    let subst: Subst = info.tparams.iter().copied().zip(self.types.items(args).iter().copied()).collect();
                    let under = self.types.subst(under, &subst);
                    return self.sig_name(under, in_array, depth + 1, lib);
                }
                if info.value_class && info.kind != ClassKind::Builtin {
                    return self.value_class_sig_name(c, args, in_array, depth, lib);
                }
                if Some(c) == self.b.cons_tuple {
                    let tuples = &self.b.tuples;
                    return Some(match self.types.cons_arity(t, c, &|k| tuples.iter().position(|&n| n == Some(k))) {
                        Some(n) if n <= 22 => format!("scala.Tuple{}", n),
                        Some(_) => "scala.runtime.TupleXXL".to_string(),
                        None => "scala.Product".to_string(),
                    });
                }
                Some(self.sig_class_name(c, lib))
            }
            Type::Param(p) => {
                let upper = self.syms.tparam(p).upper;
                self.sig_name(upper, in_array, depth + 1, lib)
            }
            // `CC[A]` of a parameter bounded by a type lambda erases as the bound applied.
            Type::AppParam(p, args) => {
                let upper = self.syms.tparam(p).upper;
                let upper = match self.types.get(upper) {
                    Type::Lambda(..) => {
                        let args = self.types.items(args).to_vec();
                        self.types.apply_ctor(upper, &args)
                    }
                    _ => upper,
                };
                self.sig_name(upper, in_array, depth + 1, lib)
            }
            Type::Wild => Some("java.lang.Object".to_string()),
            // A polymorphic function type is scalac's refinement of `PolyFunction`.
            Type::Poly(..) => Some("scala.PolyFunction".to_string()),
            Type::BoundedWild(_, hi) => self.sig_name(hi, in_array, depth + 1, lib),
            Type::Lit(v) => {
                let widened = match self.types.lit_val(v) {
                    LitVal::Int(_) => self.b.t_int,
                    LitVal::Long(_) => self.b.t_long,
                    LitVal::Double(_) => self.b.t_double,
                    LitVal::Char(_) => self.b.t_char,
                    LitVal::Bool(_) => self.b.t_boolean,
                    LitVal::Str(_) => self.b.t_string,
                };
                self.sig_name(widened, in_array, depth + 1, lib)
            }
            Type::This(c) | Type::Ctor(c) => {
                let ty = self.syms.this_type(c);
                if ty == t {
                    return None;
                }
                self.sig_name(ty, in_array, depth + 1, lib)
            }
            Type::Term(_) | Type::Select(..) => {
                let w = self.widen_path(t);
                if w == t {
                    return None;
                }
                self.sig_name(w, in_array, depth + 1, lib)
            }
            // The writer's erasures of the shapes the reader matches any name against.
            Type::Refined(parent, _) if lib.is_some() => self.sig_name(parent, in_array, depth + 1, lib),
            Type::Inter(a, b) if lib.is_some() => self.sig_glb(a, b, in_array, depth, lib),
            Type::Union(a, b) if lib.is_some() => self.sig_lub(a, b, in_array, depth, lib),
            Type::Member(..) | Type::AppMember(..) | Type::Decl(_) | Type::Match(..) if lib.is_some() => {
                let w = self.upper_bound_of(t);
                if w == t {
                    return None;
                }
                self.sig_name(w, in_array, depth + 1, lib)
            }
            _ => None,
        }
    }

    /// scalac's `erasedGlb` of `A & B`: the erasure of the one whose class derives from the
    /// other's, else of the one that is no trait, else `A`'s.
    fn sig_glb(&mut self, a: TypeId, b: TypeId, in_array: bool, depth: u32, lib: Option<&str>) -> Option<String> {
        let (ca, cb) = (self.sig_erased_class(a), self.sig_erased_class(b));
        let pick_b = match (ca, cb) {
            (Some(x), Some(y)) if self.sig_derives(y, x) && !self.sig_derives(x, y) => true,
            (Some(x), Some(y)) => self.syms.class(x).kind == ClassKind::Trait && self.syms.class(y).kind != ClassKind::Trait && !self.sig_derives(x, y),
            (None, Some(_)) => true,
            _ => false,
        };
        self.sig_name(if pick_b { b } else { a }, in_array, depth + 1, lib)
    }

    /// scalac's `erasedLub` of `A | B`: the most specific class both derive from, a common trait
    /// where they share no class but `Object`, `Object` otherwise.
    fn sig_lub(&mut self, a: TypeId, b: TypeId, in_array: bool, depth: u32, lib: Option<&str>) -> Option<String> {
        let ea = self.sig_name(a, in_array, depth + 1, lib)?;
        let eb = self.sig_name(b, in_array, depth + 1, lib)?;
        if ea == eb {
            return Some(ea);
        }
        let (Some(ca), Some(cb)) = (self.sig_erased_class(a), self.sig_erased_class(b)) else { return Some("java.lang.Object".to_string()) };
        let bases: Vec<ClassId> = self.syms.class(ca).base_types.iter().map(|&(k, _)| k).collect();
        let common: Vec<ClassId> = bases.into_iter().filter(|&k| self.sig_derives(cb, k)).collect();
        let class = common.iter().copied().find(|&k| self.syms.class(k).kind != ClassKind::Trait);
        let pick = match class {
            Some(k) if self.full_class_name(k) != "java.lang.Object" && self.full_class_name(k) != "scala.Any" && self.full_class_name(k) != "scala.AnyRef" => Some(k),
            _ => common.iter().copied().find(|&k| self.syms.class(k).kind == ClassKind::Trait),
        };
        match pick {
            Some(k) => Some(self.sig_class_name(k, lib)),
            None => Some("java.lang.Object".to_string()),
        }
    }

    /// The class a type erases to, where it is one.
    fn sig_erased_class(&mut self, t: TypeId) -> Option<ClassId> {
        let d = self.dealias(t);
        match self.types.get(d) {
            Type::Class(c, _) => Some(c),
            Type::Refined(p, _) => self.sig_erased_class(p),
            Type::Param(p) | Type::AppParam(p, _) => {
                let u = self.syms.tparam(p).upper;
                if u == d { None } else { self.sig_erased_class(u) }
            }
            _ => None,
        }
    }

    fn sig_derives(&mut self, c: ClassId, base: ClassId) -> bool {
        self.complete_class(c);
        self.syms.class(c).base_types.iter().any(|&(k, _)| k == base)
    }

    /// The upper bound of an abstract type, a type member or a match type, by which it erases.
    fn upper_bound_of(&mut self, t: TypeId) -> TypeId {
        let w = self.widen_path(t);
        if w != t {
            return w;
        }
        match self.types.get(t) {
            Type::Match(_, m) => self.types.match_info(m).bound,
            _ => t,
        }
    }

    /// An array erases to `Object` where its element may be a primitive or a reference
    /// (scalac's `isGenericArrayElement`: a type parameter or a wildcard bounded by `Any`,
    /// `AnyVal`, `Matchable`), to the array of the element's erasure otherwise:
    /// `Array[T <: AnyRef]` is `Object[]`, `Array[T <: Int]` is `Int[]`.
    fn array_sig_name(&mut self, elem: TypeId, depth: u32, lib: Option<&str>) -> Option<String> {
        // A wildcard is looked at before `dealias`, which reads it as its upper bound.
        let elem = self.deref(elem);
        let bound = match self.types.get(elem) {
            Type::Wild => Some(ANY),
            Type::BoundedWild(_, hi) => Some(hi),
            _ => None,
        };
        let elem = if bound.is_some() { elem } else { self.dealias(elem) };
        let bound = match self.types.get(elem) {
            _ if bound.is_some() => bound,
            // `Array[?]` (`Object`) and `Array[Any]` (`Object[]`) are one type to teq.
            Type::Any => return None,
            Type::Param(p) | Type::AppParam(p, _) => Some(self.syms.tparam(p).upper),
            _ => None,
        };
        if let Some(b) = bound {
            if !self.fits_jvm_array(b, 0) {
                return Some("java.lang.Object".to_string());
            }
        }
        self.sig_name(elem, true, depth + 1, lib).map(|e| e + "[]")
    }

    /// Whether a type is the std's `js.UndefOr[A]` by an alias: scalajs-library's is `A | Unit`
    /// of its Scala 2 class `js.|`, which a signature names.
    fn names_js_undef_or(&mut self, t: TypeId) -> bool {
        let mut t = self.deref(t);
        for _ in 0..16 {
            let Type::Alias(a, args) = self.types.get(t) else { return false };
            let info = self.syms.alias(a);
            let js = matches!(info.owner, Owner::Package(p) if self.pkg_description(p) == "scala.scalajs.js");
            if js && self.name_str(info.name) == "UndefOr" && self.source(info.file).is_std && !self.in_jar(info.file) {
                return true;
            }
            match self.alias_expansion(a, args) {
                Some(e) if e != t => t = self.deref(e),
                _ => return false,
            }
        }
        false
    }

    /// Whether one JVM array holds every value of a type bounded by `t`: a reference array for
    /// a class, a primitive one for a primitive, none for `Any`, `AnyVal` or `Matchable`.
    fn fits_jvm_array(&mut self, t: TypeId, depth: u32) -> bool {
        if depth > 16 {
            return false;
        }
        let t = self.dealias(t);
        match self.types.get(t) {
            Type::Any => false,
            Type::Class(c, _) => !matches!(self.full_class_name(c).as_str(), "scala.Any" | "scala.AnyVal" | "scala.Matchable" | "scala.Singleton"),
            Type::Param(p) | Type::AppParam(p, _) => {
                let upper = self.syms.tparam(p).upper;
                self.fits_jvm_array(upper, depth + 1)
            }
            Type::BoundedWild(_, hi) => self.fits_jvm_array(hi, depth + 1),
            Type::Inter(a, b) => self.fits_jvm_array(a, depth + 1) || self.fits_jvm_array(b, depth + 1),
            // scala-library's `T <: AnyRef | Null`.
            Type::Union(a, b) => self.fits_jvm_array(a, depth + 1) && self.fits_jvm_array(b, depth + 1),
            _ => false,
        }
    }

    /// A value class erases to the erasure of what it wraps, its arguments substituted
    /// (`Wrap[String]` is `String`) except inside an array (`Array[Wrap[String]]` is
    /// `Object[]`), a type parameter wrapped at a primitive being the primitive's box
    /// (`Wrap[Int]` is `java.lang.Integer`, `Wrap[WrapI]` is `Int`) and a wrapped array the
    /// array as the class declares it (`WrapArr[String]` over `Array[T]` is `Object`).
    fn value_class_sig_name(&mut self, c: ClassId, args: TList, in_array: bool, depth: u32, lib: Option<&str>) -> Option<String> {
        self.complete_class(c);
        let info = self.syms.class(c);
        let generic = info.ctor.first().and_then(|cl| cl.params.first()).map(|p| p.ty)?;
        let tparams = info.tparams.clone();
        let generic_ty = self.dealias(generic);
        if matches!(self.types.get(generic_ty), Type::Class(k, _) if k == self.b.array) {
            return self.sig_name(generic, false, depth + 1, lib);
        }
        let underlying = if in_array {
            generic
        } else {
            let subst: Subst = tparams.iter().copied().zip(self.types.items(args).iter().copied()).collect();
            self.types.subst(generic, &subst)
        };
        let erased = self.sig_name(underlying, false, depth + 1, lib)?;
        let underlying_ty = self.dealias(underlying);
        let wraps_value_class = matches!(self.types.get(underlying_ty), Type::Class(k, _) if self.syms.class(k).value_class && self.syms.class(k).kind != ClassKind::Builtin);
        let generic_primitive = matches!(self.types.get(generic_ty), Type::Class(..));
        match boxed_primitive(&erased) {
            Some(boxed) if !wraps_value_class && !generic_primitive => Some(boxed.to_string()),
            _ => Some(erased),
        }
    }

    /// A class's name in a signature: its full name, `Any` and its kin as `java.lang.Object`,
    /// `Unit` as `scala.runtime.BoxedUnit`.
    fn sig_class_name(&mut self, c: ClassId, lib: Option<&str>) -> String {
        let full = match lib {
            Some(local) => self.library_class_name(c, local),
            None => self.full_class_name(c),
        };
        match full.as_str() {
            "scala.Any" | "scala.AnyRef" | "scala.AnyVal" | "scala.Matchable" | "scala.Singleton" => "java.lang.Object".to_string(),
            "scala.Tuple" | "scala.NonEmptyTuple" => "scala.Product".to_string(),
            "scala.Unit" => "scala.runtime.BoxedUnit".to_string(),
            "scala.String" => "java.lang.String".to_string(),
            _ => match full.strip_prefix("scala.ContextFunction") {
                Some(arity) => format!("scala.Function{}", arity),
                None => full,
            },
        }
    }

    /// scalac's `fullName` of a class: an object's class ends in `$`, and a member of an object
    /// is named under it (`pe.A$.Token`), as is a Java class's static nested class, a member of
    /// its companion (`fix.Outer$.Nested`, where the inner `fix.Outer.Inner` is the class's),
    /// whether the loader entered it under the class or, not read yet, under a package of the
    /// class's name.
    fn full_class_name(&mut self, c: ClassId) -> String {
        let info = self.syms.class(c);
        let mut name = self.name_str(info.name);
        if info.kind == ClassKind::Object && !name.ends_with('$') {
            name.push('$');
        }
        match info.owner {
            Owner::Class(o) => {
                let outer = self.full_class_name(o);
                let java_static = self.loaded.as_ref().and_then(|l| l.java.classes.get(&c)).map_or(false, |j| !j.inner && j.companion_of.is_none());
                let separator = if java_static && !outer.ends_with('$') { "$." } else { "." };
                format!("{}{}{}", outer, separator, name)
            }
            Owner::Package(p) if p != ROOT_PKG => {
                let cname = self.syms.class(c).name;
                let separator = if self.is_java_placeholder(c) && self.placeholder_static_nested(p, cname) == Some(true) { "$." } else { "." };
                format!("{}{}{}", self.pkg_description(p), separator, name)
            }
            _ => name,
        }
    }
}

impl<'a> Worker<'a> {
    /// The name a type erases to in a signature as scala-library names classes.
    pub(crate) fn library_sig_name(&mut self, t: TypeId, local: &str, result: bool) -> Option<String> {
        let r = self.sig_name(t, false, 0, Some(local))?;
        Some(if result && r == "scala.runtime.BoxedUnit" { "scala.Unit".to_string() } else { r })
    }

    /// The signature scalac 3.8.4 gives the declaration `s` (`TypeErasure.sigName`), as a
    /// pickle names it: per clause of type parameters its length, per term parameter the
    /// class it erases to, then the result's; `local` names the class a local class of the body
    /// being written is in (`p.C$._$Loc`). `None` where teq's types do not tell the erasure.
    pub(crate) fn pickled_signature(&mut self, s: SymId, local: &str) -> Option<(Vec<PickledSigParam>, String)> {
        self.pickled_signature_seen(s, local, &Vec::new())
    }

    /// `pickled_signature` of `s` with its owner's type parameters bound by `subst`: the
    /// signature of the member as a class that inherits it sees it (scalac's `asSeenFrom`).
    pub(crate) fn pickled_signature_seen(&mut self, s: SymId, local: &str, subst: &crate::types::Subst) -> Option<(Vec<PickledSigParam>, String)> {
        let info = (*self.syms.sym(s)).clone();
        let mut sig = self.sig_of(s).clone();
        if !subst.is_empty() {
            for p in sig.clauses.iter_mut().flat_map(|c| c.params.iter_mut()) {
                p.ty = self.types.subst(p.ty, subst);
            }
            sig.ret = self.types.subst(sig.ret, subst);
        }
        if let Some(&t) = self.inline_results.get(&s) {
            sig.ret = t;
        }
        let (ext_tparams, ext_clauses) = if info.is_extension { (info.ext_tparams as usize, info.ext_clauses as usize) } else { (sig.tparams.len(), 0) };
        let mut params = Vec::new();
        let push_tparams = |params: &mut Vec<PickledSigParam>, n: usize| {
            if n > 0 {
                params.push(PickledSigParam::Types(n));
            }
        };
        // A secondary constructor takes its class's type parameters.
        if info.name == crate::names::INIT && sig.tparams.is_empty() {
            if let Owner::Class(c) = info.owner {
                let n = self.syms.class(c).own_tparams().len();
                push_tparams(&mut params, n);
            }
        }
        push_tparams(&mut params, ext_tparams.min(sig.tparams.len()));
        let java = self.is_java_defined(s);
        for (ci, clause) in sig.clauses.iter().enumerate() {
            if ci == ext_clauses && ext_clauses > 0 {
                push_tparams(&mut params, sig.tparams.len().saturating_sub(ext_tparams));
            }
            for p in &clause.params {
                let erased = if p.by_name {
                    "scala.Function0".to_string()
                } else if p.repeated && java {
                    self.java_varargs_sig_name(p.ty, Some(local))?
                } else if p.repeated {
                    "scala.collection.immutable.Seq".to_string()
                } else {
                    self.sig_name(p.ty, false, 0, Some(local))?
                };
                params.push(PickledSigParam::Term(erased));
            }
        }
        if ext_clauses >= sig.clauses.len() && ext_clauses > 0 {
            push_tparams(&mut params, sig.tparams.len().saturating_sub(ext_tparams));
        }
        let given_class = info.impl_class.filter(|_| info.kind == SymKind::Given && !(sig.tparams.is_empty() && sig.clauses.is_empty()));
        let result = if info.name == crate::names::INIT {
            match info.owner {
                Owner::Class(c) => self.sig_class_name(c, Some(local)),
                _ => return None,
            }
        } else if let Some(k) = given_class {
            // A given with a body and parameters returns its class.
            self.sig_class_name(k, Some(local))
        } else {
            let r = self.sig_name(sig.ret, false, 0, Some(local))?;
            if r == "scala.runtime.BoxedUnit" { "scala.Unit".to_string() } else { r }
        };
        Some((params, result))
    }

    /// Whether `s` is a method of a Java class read from its class file, whose repeated parameter
    /// is Java's varargs, an array in a signature.
    fn is_java_defined(&self, s: SymId) -> bool {
        matches!(self.syms.sym(s).owner, Owner::Class(c) if self.loaded.as_ref().map_or(false, |l| l.java.classes.contains_key(&c)))
    }

    /// A Java varargs parameter of the element type `elem` as a signature names it: the element's
    /// erasure and `[]`, `java.lang.Object[]` for a type parameter (`<T> f(T... xs)`), which Java
    /// erases to its bound, where Scala's generic array erases to `Object`.
    fn java_varargs_sig_name(&mut self, elem: TypeId, lib: Option<&str>) -> Option<String> {
        let elem_now = self.deref(elem);
        match self.types.get(elem_now) {
            Type::Any => Some("java.lang.Object[]".to_string()),
            _ => self.sig_name(elem, true, 0, lib).map(|e| e + "[]"),
        }
    }

    /// The signature of the default getter of the `index`-th parameter (counted over the
    /// clauses) of a method with these type parameters and clauses: the type parameters' section,
    /// the parameters of the clauses before the parameter's, the parameter's type.
    pub(crate) fn pickled_default_signature(&mut self, tparams: usize, clauses: &[ClauseSig], index: usize, local: &str) -> Option<(Vec<PickledSigParam>, String)> {
        let mut params = Vec::new();
        if tparams > 0 {
            params.push(PickledSigParam::Types(tparams));
        }
        let mut at = 0;
        for clause in clauses {
            if at + clause.params.len() > index {
                let p = &clause.params[index - at];
                let r = self.sig_name(p.ty, false, 0, Some(local))?;
                let r = if r == "scala.runtime.BoxedUnit" { "scala.Unit".to_string() } else { r };
                return Some((params, r));
            }
            for p in &clause.params {
                let erased = if p.by_name {
                    "scala.Function0".to_string()
                } else if p.repeated {
                    "scala.collection.immutable.Seq".to_string()
                } else {
                    self.sig_name(p.ty, false, 0, Some(local))?
                };
                params.push(PickledSigParam::Term(erased));
            }
            at += clause.params.len();
        }
        None
    }

    /// The signature of a class's primary constructor: its type parameters' section, its
    /// clauses' parameters, the class.
    pub(crate) fn pickled_ctor_signature(&mut self, c: ClassId, local: &str) -> Option<(Vec<PickledSigParam>, String)> {
        self.complete_class(c);
        let info = (*self.syms.class(c)).clone();
        let mut params = Vec::new();
        let n = info.own_tparams().len();
        if n > 0 {
            params.push(PickledSigParam::Types(n));
        }
        for clause in &info.ctor {
            for p in &clause.params {
                let erased = if p.by_name {
                    "scala.Function0".to_string()
                } else if p.repeated {
                    "scala.collection.immutable.Seq".to_string()
                } else {
                    self.sig_name(p.ty, false, 0, Some(local))?
                };
                params.push(PickledSigParam::Term(erased));
            }
        }
        Some((params, self.sig_class_name(c, Some(local))))
    }

    /// The first type of a signature `pickled_signature` cannot erase, described, for the
    /// census of the bodies withheld for it.
    pub(crate) fn unerasable_in(&mut self, s: SymId, local: &str) -> String {
        let sig = self.sig_of(s).clone();
        let types: Vec<TypeId> = sig.clauses.iter().flat_map(|c| c.params.iter().filter(|p| !p.by_name && !p.repeated).map(|p| p.ty)).chain(std::iter::once(sig.ret)).collect();
        for t in types {
            if self.sig_name(t, false, 0, Some(local)).is_none() {
                let d = self.dealias(t);
                let kind = format!("{:?}", self.types.get(d));
                return kind.split('(').next().unwrap_or("").to_string();
            }
        }
        String::new()
    }

    /// A class's name as scala-library and scalac name it in a signature: the std's classes by
    /// scala-library's names, an object's class with `$`, a local class under the class whose
    /// body defines it (`local`), `_$` before its name.
    pub(crate) fn library_class_name(&mut self, c: ClassId, local: &str) -> String {
        let info = self.syms.class(c);
        if info.owner == Owner::Local {
            let name = match info.kind {
                ClassKind::Anon => "$anon".to_string(),
                // A local object's module class.
                _ if info.local_module.is_some() => format!("{}$", self.name_str(info.name)),
                _ => self.name_str(info.name).to_string(),
            };
            return format!("{}._${}", local, name);
        }
        let mut name = crate::tasty::write::scala_name(self, c);
        if info.kind == ClassKind::Object && !name.ends_with('$') {
            name.push('$');
        }
        name
    }
}

/// A parameter of a pickled signature: a clause of type parameters by its length, or the class
/// a term parameter erases to.
pub(crate) enum PickledSigParam {
    Types(usize),
    Term(String),
}

/// The class that boxes a primitive's erasure.
fn boxed_primitive(erased: &str) -> Option<&'static str> {
    Some(match erased {
        "scala.Int" => "java.lang.Integer",
        "scala.Long" => "java.lang.Long",
        "scala.Short" => "java.lang.Short",
        "scala.Byte" => "java.lang.Byte",
        "scala.Char" => "java.lang.Character",
        "scala.Float" => "java.lang.Float",
        "scala.Double" => "java.lang.Double",
        "scala.Boolean" => "java.lang.Boolean",
        "scala.runtime.BoxedUnit" => "scala.runtime.BoxedUnit",
        _ => return None,
    })
}

/// A parameter of a signature: a section of type parameters by its length, or a term
/// parameter by the name of the class it erases to (`None` for scalac's placeholder of a type
/// it could not erase).
enum SigParam {
    Types(usize),
    Term(Option<String>),
}

/// A node of a converted body the listing places.
pub(in crate::typer) enum Node {
    Expr(ExprId),
    Pat(crate::ast::PatId),
}

/// `TEQ_READER_DUMP=<file>`: the file the reader's listing is appended to, read once.
pub fn dump_path() -> Option<&'static std::path::Path> {
    static PATH: std::sync::OnceLock<Option<std::path::PathBuf>> = std::sync::OnceLock::new();
    PATH.get_or_init(|| std::env::var_os("TEQ_READER_DUMP").map(std::path::PathBuf::from)).as_deref()
}

/// Appends one line of the listing; the workers' lines interleave, which a sort puts in order.
pub fn dump_line(line: String) {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let Some(path) = dump_path() else { return };
    let _held = LOCK.lock();
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(f, "{}", line);
    }
}

impl<'a> Worker<'a> {
    /// The listing's line of a library body's selection: where it stands, the declaration its
    /// TASTy names, and the member the typer called or why it selected by name.
    pub(in crate::typer) fn dump_declared(&mut self, d: DeclRef, node: crate::ast::ExprId, at: crate::source::Span, ty: Option<TypeId>, took: Option<Took>) {
        let (r, owner) = self.with_loader_for(crate::measure::Hold::JarLookup, |w| (w.resolve_declaration(d), w.declaration_owner(d)));
        let tasty = self.tasty(d.file);
        let pickled = self.cur_ast().reader.as_ref().and_then(|r| r.pickled_owners.get(&node)).cloned().unwrap_or_default();
        let owner = match owner {
            Some(k) => self.class_path(k),
            None => pickled.clone(),
        };
        let owner = if pickled.is_empty() || pickled == owner { owner } else { format!("{} (the pickle's {})", owner, pickled) };
        let declared = format!("{}.{}", owner, crate::tasty::dump::name_text(&tasty, d.name));
        let resolved = match (took, r) {
            (Some(Took::Declared(s)), Declared::Inherited(_)) => format!("inherited {}", self.member_signature(s)),
            (Some(Took::Declared(s)), _) => format!("declared {}", self.member_signature(s)),
            (Some(Took::One(s)), _) => format!("one member {}", self.member_signature(s)),
            (None, Declared::ByName(why)) => format!("by name ({})", match why {
                ByName::Builtin => "a builtin's member",
                ByName::StdOwner => "the std's class",
                ByName::JavaOwner => "a Java class or one the classpath lacks",
                ByName::Missing => "no member matches",
                ByName::Ambiguous => "several members match",
                ByName::Inline => "an inline method, resolved where it expands",
            }),
            (None, _) => "by name (typed apart from a member's lookup, or on a receiver of the std's class)".to_string(),
        };
        let ty = ty.map_or(String::new(), |t| format!("\t: {}", self.show(t)));
        let place = self.body_place(at, Some(Node::Expr(node)));
        dump_line(format!("select\t{}\t{}\t{}{}", place, declared, resolved, ty));
    }

    /// The listing's line of a reference by address to one alternative of an overloaded name.
    pub(in crate::typer) fn dump_addressed(&mut self, s: SymId, node: ExprId, at: crate::source::Span) {
        let place = self.body_place(at, Some(Node::Expr(node)));
        let member = self.member_signature(s);
        dump_line(format!("address\t{}\t{}", place, member));
    }

    /// The bounds a library body's typed pattern states for its variables (`BIND`), on the
    /// case's type parameters beside what the class's parameters give them; the scrutinee's
    /// then narrow them as in source.
    #[cold]
    pub(in crate::typer) fn install_binder_bounds(&mut self, binders: &[(TParamId, TypeId)], bounds: &[(Name, Option<crate::ast::TyExprId>, Option<crate::ast::TyExprId>)]) {
        for &(name, lo, hi) in bounds {
            let Some(&(p, _)) = binders.iter().find(|&&(p, _)| self.syms.tparam(p).name == name) else { continue };
            if let Some(hi) = hi {
                let hi = self.resolve_type(hi);
                let current = self.syms.tparam(p).upper;
                self.syms.tparams[p.idx()].upper = if current == ANY || self.is_sub(hi, current) { hi } else { self.types.inter(current, hi) };
            }
            if let Some(lo) = lo {
                let lo = self.resolve_type(lo);
                let current = self.syms.tparam(p).lower;
                self.syms.tparams[p.idx()].lower = if current == NOTHING { lo } else { self.lub(current, lo) };
            }
        }
    }

    /// The listing's `bounds` line per variable of a library body's typed pattern: the bounds
    /// the typer gave it.
    #[cold]
    pub(in crate::typer) fn dump_binder_bounds(&mut self, binders: &[(TParamId, TypeId)], at: crate::source::Span, pat: crate::ast::PatId) {
        let place = self.body_place(at, Some(Node::Pat(pat)));
        for &(p, _) in binders {
            let (name, lo, hi) = (self.syms.tparam(p).name, self.syms.tparam(p).lower, self.syms.tparam(p).upper);
            let line = format!("bounds\t{}\t{} >: {} <: {}", place, self.name_str(name), self.show_bound(lo), self.show_bound(hi));
            dump_line(line);
        }
    }

    /// A bound as the listing shows it: a library's class by its full name, which tells a
    /// class of the program named `Any` from scala's.
    fn show_bound(&mut self, t: TypeId) -> String {
        match self.types.get(t) {
            Type::Class(c, args) if args == EMPTY_LIST && self.is_library_class(c) => self.class_path(c),
            _ => self.show(t),
        }
    }

    /// `pkg.C.m(x: T): R` with its target name, as the listing shows a member.
    fn member_signature(&mut self, s: SymId) -> String {
        let sig = self.sig_arc(s);
        let params: Vec<String> = sig.clauses.iter().map(|c| format!("({})", c.params.iter().map(|p| self.show(p.ty)).collect::<Vec<_>>().join(", "))).collect();
        let target = self.loaded.as_ref().and_then(|l| l.target_names.get(&s).copied()).map_or(String::new(), |t| format!(" @{}", self.name_str(t)));
        format!("{}{}: {}{}", self.sym_path(s), params.join(""), self.show(sig.ret), target)
    }

    /// The pseudo file's class and the member a node of it stands in: the line of the pseudo
    /// file's text its span is on (`def put`), after the jar's name; then where the node stands
    /// in the library's source.
    pub(in crate::typer) fn body_place(&mut self, at: crate::source::Span, node: Option<Node>) -> String {
        let src = self.source(self.env.file);
        let class = src.path.rsplit_once('!').map_or(src.path.as_str(), |(_, c)| c);
        let text = &src.text;
        let start = text[..(at.start as usize).min(text.len())].rfind('\n').map_or(0, |i| i + 1);
        let end = text[start..].find('\n').map_or(text.len(), |i| start + i);
        let shown = format!("{}\t{}", class, &text[start..end]);
        let file = self.env.file;
        let place = match node {
            Some(Node::Expr(e)) => self.expr_place(file, e),
            Some(Node::Pat(p)) => self.pat_place(file, p),
            None => None,
        };
        let position = place.map_or_else(|| "-".to_string(), |p| p.listed());
        format!("{}\t{}", shown, position)
    }
}

/// The simple name of the class a declaration's owner type names, where it is a reference by
/// name (a class of the body itself, by address, has none).
fn owner_simple_name<'t>(tasty: &'t crate::tasty::TastyFile, owner: &TType) -> Option<&'t str> {
    match owner {
        TType::TypeRef(_, n) => tasty.simple(tasty.source_name(*n)),
        TType::Applied(f, _) | TType::This(f) => owner_simple_name(tasty, f),
        _ => None,
    }
}

/// Whether a type names a definition of its own file by address that the loader did not enter
/// (a local class of a body, a type parameter), which only the body's conversion reads.
fn mentions_local(t: &TType, entered: &dyn Fn(crate::tasty::tree::Addr) -> bool) -> bool {
    match t {
        TType::LocalType(addr, prefix) => !entered(*addr) || prefix.as_deref().map_or(false, |p| mentions_local(p, entered)),
        TType::LocalTerm(..) | TType::ParamRef(..) | TType::RecThis(_) => true,
        TType::TypeRef(p, _) | TType::TermRef(p, _) | TType::This(p) => mentions_local(p, entered),
        TType::Applied(f, args) => mentions_local(f, entered) || args.iter().any(|a| mentions_local(a, entered)),
        _ => false,
    }
}

impl<'a> Worker<'a> {
    /// A library body's selection of the member its TASTy declares: typed as
    /// the selection is, with the declaration taking the alternative where the name the
    /// receiver has is overloaded (`declared_call`, `take_declared`), so that no overload
    /// resolution runs again.
    #[cold]
    #[inline(never)]
    pub(in crate::typer) fn apply_declared(&mut self, d: DeclRef, node: ExprId, q: ExprId, name: Name, targs: Option<ListRef>, lists: Vec<ArgList>, span: Span, expected: Option<TypeId>) -> (TExprId, TypeId) {
        let outer_took = self.declared_took.take();
        let typed = match self.resolve_static_path(q, name) {
            StaticPath::Callee(callee) => {
                let callee = self.static_declared(callee, d);
                self.apply_callee(callee, targs, lists, span, expected)
            }
            StaticPath::Missing(pkg) => {
                let msg = format!("value {} is not a member of {}", self.name_str(name), self.package_display(pkg));
                self.error(span, msg);
                self.type_args_for_errors(&lists);
                (self.prog.add(TExpr::Unit), ERROR)
            }
            StaticPath::NotStatic => 'typed: {
                // A class reference's companion `apply` and a facade constructor's default, which
                // the selection by name takes before typing a receiver, are taken the same way.
                let default = self.ctor_default_index(name);
                if name == names::APPLY || default.is_some() {
                    if let Some((c, class_targs)) = self.ctor_ref(q) {
                        if name == names::APPLY {
                            break 'typed self.apply_companion_apply(c, targs.or(class_targs), lists, span, expected);
                        }
                        if let Some(ty) = default.and_then(|n| self.ctor_param_default(c, n)) {
                            break 'typed (self.prog.add(TExpr::Unit), ty);
                        }
                    }
                }
                let receiver = self.cur_ast().expr(q);
                let selected = name == names::APPLY && matches!(receiver, Expr::TypeApply(..));
                let bare = matches!(receiver, Expr::Ident(_) | Expr::Select(..) | Expr::TypeApply(..));
                let (tq, qty) = self.type_receiver(q, selected, bare);
                let outer = self.declared_call.replace((tq, name, d));
                let typed = self.apply_member(tq, qty, name, targs, lists, span, expected);
                self.declared_call = outer;
                typed
            }
        };
        let took = std::mem::replace(&mut self.declared_took, outer_took);
        if dump_path().is_some() {
            self.dump_declared(d, node, span, Some(typed.1), took);
        }
        typed
    }

    /// The member a library body's selection calls where the receiver's name is overloaded:
    /// the alternative its TASTy declares, when the typer resolves it; `None` leaves the
    /// overloads to the resolution by the arguments.
    #[cold]
    #[inline(never)]
    pub(in crate::typer) fn take_declared(&mut self, recv: TExprId, recv_ty: TypeId, name: Name, found: SymId) -> Option<(SymId, TypeId)> {
        let (r, n, d) = self.declared_call?;
        if r != recv || n != name {
            return None;
        }
        self.declared_call = None;
        if self.syms.alternatives(found).is_none() {
            self.declared_took = Some(Took::One(found));
            return None;
        }
        if !self.may_derive_from_owner(d, recv_ty) {
            return None;
        }
        let s = self.declared_member(d)?;
        let owner_ty = self.declared_owner_type(recv_ty, s)?;
        self.declared_took = Some(Took::Declared(s));
        Some((s, owner_ty))
    }

    /// Whether the receiver's class has a base class of the simple name of the declaration's
    /// owner. Without one the receiver's member is not the declaration (scala-library's
    /// `MapOps.+` on the lean std's `Map`), and resolving it would enter the owner's class for
    /// nothing. A receiver no one class stands for (an intersection, a type parameter, a
    /// refinement) may derive from it through any part, so the declaration is resolved.
    fn may_derive_from_owner(&mut self, d: DeclRef, recv_ty: TypeId) -> bool {
        let recv = self.widen_path(recv_ty);
        let recv = self.deref(recv);
        let Type::Class(c, _) = self.types.get(recv) else { return true };
        let tasty = self.tasty(d.file);
        let owner = Decoder::new(&tasty).type_at(d.owner_at);
        let Some(simple) = owner_simple_name(&tasty, &owner) else { return true };
        let simple = simple.trim_end_matches('$');
        self.syms.class(c).base_types.iter().any(|&(b, _)| self.name_ref(self.syms.class(b).name).trim_end_matches('$') == simple)
    }

    /// `take_declared` for a static path's overloaded member (`RdNamed.sum` of an object).
    fn static_declared(&mut self, callee: Callee, d: DeclRef) -> Callee {
        let Callee::Overloaded { recv, recv_ty, .. } = callee else { return callee };
        let Some(s) = self.declared_member(d) else { return callee };
        self.declared_took = Some(Took::Declared(s));
        match (recv, recv_ty) {
            (Some(recv), Some(recv_ty)) => {
                let owner_ty = self.declared_owner_type(recv_ty, s);
                self.member_callee_in(recv, recv_ty, owner_ty, s)
            }
            _ => Callee::Method { recv: None, sym: s, owner_subst: Vec::new(), prefix: None },
        }
    }

    /// A library body's reference by address names one alternative of an overloaded name
    /// (`RdPick.this.put` as `TERMREFsymbol`): the call takes it rather than the name's
    /// overloads the receiver has. An inline alternative is resolved where
    /// it expands.
    #[cold]
    #[inline(never)]
    pub(in crate::typer) fn referenced_alternative(&mut self, callee: Callee, s: SymId, node: ExprId, span: Span) -> Callee {
        let Callee::Overloaded { recv: Some(recv), recv_ty: Some(recv_ty), .. } = callee else { return callee };
        if !self.is_body_file(self.env.file) || self.syms.alternatives(s).is_some() || !self.is_method_sym(s) || self.is_inline_callee(s) {
            return callee;
        }
        match self.declared_owner_type(recv_ty, s) {
            Some(owner_ty) => {
                if dump_path().is_some() {
                    self.dump_addressed(s, node, span);
                }
                self.member_callee_in(recv, recv_ty, Some(owner_ty), s)
            }
            None => callee,
        }
    }

    /// The base type of `recv_ty` at the class that declares `sym`, which the member's
    /// signature is seen from; the receiver's type itself for an object's member.
    fn declared_owner_type(&mut self, recv_ty: TypeId, sym: SymId) -> Option<TypeId> {
        let Owner::Class(owner) = self.syms.sym(sym).owner else { return None };
        let recv_ty = self.widen_path(recv_ty);
        self.base_type(recv_ty, owner)
    }
}
