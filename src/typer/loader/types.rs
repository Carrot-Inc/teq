//! `TType`, a type as TASTy states it, onto `TypeId`. What `Type` cannot state becomes a
//! `Type::Blocked` carrying the shape and spelling, so that a signature always loads and a
//! use of it reports what it met.

use super::super::resolve::{TermRef, TypeRef};
use super::super::Worker;
use crate::intern::Name;
use crate::symbols::*;
use crate::tasty::shapes::{function_class, Shape};
use crate::tasty::show::Printer;
use crate::tasty::tree::{Addr, Const, Decoder, LambdaKind, LocalKind, TType};
use crate::symbols::{ClauseSig, MethodSig, ParamSig};
use crate::tasty::TastyFile;
use crate::types::*;

pub struct MapCx {
    pub file: u32,
    /// The type lambdas being mapped, innermost last, for `ParamRef`.
    binders: Vec<(Addr, Vec<TParamId>)>,
    /// The method types of refinements being mapped, innermost last, with their parameters'
    /// addresses and symbols: a type names a parameter by a `ParamRef` to the binder, a type
    /// tree by a `LocalTerm` to the parameter's definition (`c.T` of `(c: Ctx) => c.T`).
    term_binders: Vec<(Addr, Vec<(Addr, SymId)>)>,
    /// How many arguments the type constructor being mapped is applied to, which gives a
    /// Java class its arity.
    applied_to: Option<usize>,
}

impl MapCx {
    pub fn new(file: u32) -> MapCx {
        MapCx { file, binders: Vec::new(), term_binders: Vec::new(), applied_to: None }
    }
}

/// Where a qualified prefix leads: a package, or a class whose members it names (an object on
/// a static path, or the class of a `C.this`).
pub(super) enum Scope {
    Pkg(PkgId),
    Class(ClassId),
}

/// How many type arguments a type parameter takes, read off its bounds.
pub fn hk_arity(info: &TType) -> u8 {
    match info {
        TType::Bounds(_, hi) => hk_arity(hi),
        TType::Lambda { kind: LambdaKind::Type, params, .. } => params.len() as u8,
        _ => 0,
    }
}

/// The element type of a repeated parameter's type, `T*` written as `Seq[T] @Repeated` or as
/// `<repeated>[T]`.
pub fn repeated_element<'t>(tasty: &TastyFile, t: &'t TType) -> Option<&'t TType> {
    match t {
        TType::Annotated(u, annot) => {
            let name = match &**annot {
                TType::TypeRef(_, n) => tasty.simple(*n),
                TType::Applied(tycon, _) => match &**tycon {
                    TType::TypeRef(_, n) => tasty.simple(*n),
                    _ => None,
                },
                _ => None,
            };
            if name != Some("Repeated") {
                return None;
            }
            match &**u {
                TType::Applied(_, args) if args.len() == 1 => Some(&args[0]),
                _ => None,
            }
        }
        TType::Applied(tycon, args) if args.len() == 1 => match &**tycon {
            TType::TypeRef(prefix, n) if tasty.simple(*n) == Some("<repeated>") && is_scala_pkg(tasty, prefix) => Some(&args[0]),
            _ => None,
        },
        _ => None,
    }
}

fn is_scala_pkg(tasty: &TastyFile, t: &TType) -> bool {
    match t {
        TType::Package(p) => tasty.simple(*p) == Some("scala"),
        TType::This(inner) => is_scala_pkg(tasty, inner),
        _ => false,
    }
}

/// The first blocked type inside `t`.
pub fn first_blocked(types: &TypeStore, t: TypeId) -> Option<BlockedId> {
    match types.get(t) {
        Type::Blocked(b) => Some(b),
        Type::Class(_, args) | Type::AppParam(_, args) | Type::AppVar(_, args) => {
            types.items(args).iter().find_map(|&a| first_blocked(types, a))
        }
        Type::Lambda(_, body) => first_blocked(types, body),
        Type::Union(a, b) | Type::Inter(a, b) => first_blocked(types, a).or_else(|| first_blocked(types, b)),
        _ => None,
    }
}

impl<'a> Worker<'a> {
    /// A type in a value position.
    pub(super) fn map_type(&mut self, cx: &mut MapCx, t: &TType) -> TypeId {
        let mapped = self.map_type_ctor(cx, t);
        if let Type::Ctor(c) = self.types.get(mapped) {
            let n = self.class_arity(c);
            let args = vec![ERROR; n];
            return self.types.class(c, &args);
        }
        mapped
    }

    /// A type that may be a bare type constructor: an argument of a higher-kinded parameter or
    /// the right-hand side of an alias. A Java class or package the type names is not read:
    /// the class becomes a placeholder, the package a sub-package.
    pub(super) fn map_type_ctor(&mut self, cx: &mut MapCx, t: &TType) -> TypeId {
        let was_mapping = self.mapping_signature;
        self.mapping_signature = true;
        let mapped = self.map_type_ctor_in(cx, t);
        self.mapping_signature = was_mapping;
        mapped
    }

    fn map_type_ctor_in(&mut self, cx: &mut MapCx, t: &TType) -> TypeId {
        match t {
            TType::Package(_) => self.blocked(cx, Shape::Unknown, t),
            TType::TypeRef(prefix, n) => self.map_type_ref(cx, prefix, *n, t),
            TType::TermRef(..) | TType::LocalTerm(..) => match self.path_singleton(cx, t) {
                Some(ty) => ty,
                None => self.blocked(cx, Shape::Singleton, t),
            },
            TType::LocalType(addr, prefix) => self.map_local_type(cx, *addr, prefix.as_deref(), t),
            TType::This(inner) => match self.loaded_this_class(cx, inner) {
                Some(c) => self.this_prefix(c),
                None => self.blocked(cx, Shape::Singleton, t),
            },
            TType::Applied(tycon, args) => self.map_applied(cx, tycon, args, t),
            TType::Bounds(..) => WILD,
            TType::Alias(a) | TType::BoundedAlias(_, a) => self.map_type_ctor(cx, a),
            // `x.type & T`, scalac's cast of a `var` to what it holds: the singleton of a var
            // is no type, so the cast is to `T`.
            TType::And(a, b) => {
                if self.var_singleton(cx, a) {
                    return self.map_type(cx, b);
                }
                if self.var_singleton(cx, b) {
                    return self.map_type(cx, a);
                }
                let (x, y) = (self.map_type(cx, a), self.map_type(cx, b));
                self.types.inter(x, y)
            }
            TType::Or(a, b) => {
                let (x, y) = (self.map_type(cx, a), self.map_type(cx, b));
                self.types.union(x, y)
            }
            TType::ByName(inner) => {
                let a = self.map_type(cx, inner);
                self.by_name_type(a)
            }
            TType::Annotated(u, _) => self.map_type_ctor(cx, u),
            TType::Refined(parent, members) => {
                // `PolyFunction { def apply[A](a: A): A }` is a polymorphic function type; one
                // refining more than its `apply` (`; val tag: Int`) is a refinement as any other,
                // as the source's is.
                let poly = matches!(&**parent, TType::TypeRef(_, n) if self.tasty(cx.file).simple(*n) == Some("PolyFunction"));
                if poly {
                    if let Some(ty) = self.map_poly_function(cx, members) {
                        return ty;
                    }
                    if members.len() == 1 {
                        return self.blocked(cx, Shape::PolyFunction, t);
                    }
                }
                match self.map_refined(cx, parent, members) {
                    Some(ty) => ty,
                    None => self.blocked(cx, Shape::Refinement, t),
                }
            }
            TType::Rec(..) | TType::RecThis(_) => self.blocked(cx, Shape::Recursive, t),
            TType::Super(..) => self.blocked(cx, Shape::SuperType, t),
            TType::Lambda { kind: LambdaKind::Type, binder, params, result } => {
                let tasty = self.tasty(cx.file);
                let ids: Vec<TParamId> = params
                    .iter()
                    .map(|p| {
                        let text = tasty.name(tasty.source_name(p.name));
                        let n = self.interner.intern(&text);
                        let id = self.syms.new_tparam(n, 0);
                        self.syms.tparams[id.idx()].arity = hk_arity(&p.info);
                        if p.addr != 0 {
                            self.loaded_tables_mut(cx).tparams.insert(p.addr, id);
                        }
                        id
                    })
                    .collect();
                cx.binders.push((*binder, ids.clone()));
                for (p, &id) in params.iter().zip(&ids) {
                    if let TType::Bounds(lo, hi) = &p.info {
                        let (l, h) = (self.map_type(cx, lo), self.map_type(cx, hi));
                        let info = &mut self.syms.tparams[id.idx()];
                        info.lower = l;
                        info.upper = h;
                    }
                }
                let body = self.map_type_ctor(cx, result);
                cx.binders.pop();
                let ps: Vec<TypeId> = ids.iter().map(|&p| self.types.param(p)).collect();
                let l = self.types.list(&ps);
                // `[A] =>> List[A]`, the eta-expanded form TASTy gives a type constructor
                // argument, is `List`. teq's products write a constructor as itself, a lambda
                // only where the typer held one (an alias's, `type F[A] = List[A]` passed as `F`),
                // which stays the lambda it was.
                let loaded = self.loaded.as_ref().unwrap();
                let product = loaded.cp.is_products(loaded.file(cx.file).cp);
                match self.types.get(body) {
                    Type::Class(c, args) if args == l && !product => self.types.mk(Type::Ctor(c)),
                    Type::AppParam(p, args) if args == l && !product => self.types.param(p),
                    _ => self.types.mk(Type::Lambda(l, body)),
                }
            }
            TType::Lambda { .. } => self.blocked(cx, Shape::PolyFunction, t),
            TType::ParamRef(binder, num) => {
                let found = cx.binders.iter().rev().find(|(b, _)| b == binder).and_then(|(_, ps)| ps.get(*num as usize).copied());
                if let Some(p) = found {
                    return self.types.param(p);
                }
                let term = cx.term_binders.iter().rev().find(|(b, _)| b == binder).and_then(|(_, ps)| ps.get(*num as usize).map(|&(_, s)| s));
                match term {
                    Some(s) => self.types.mk(Type::Term(s)),
                    None => self.blocked(cx, Shape::Unknown, t),
                }
            }
            TType::Match { scrutinee, bound, cases } => self.map_match(cx, scrutinee, bound.as_deref(), cases, t),
            TType::MatchCase(..) => self.blocked(cx, Shape::Unknown, t),
            TType::Flexible(u) => self.map_type_ctor(cx, u),
            TType::Const(c) => {
                let tasty = self.tasty(cx.file);
                let value = match c {
                    Const::Int(v) | Const::Byte(v) | Const::Short(v) => LitVal::Int(*v),
                    Const::Long(v) => LitVal::Long(*v),
                    Const::Double(bits) => LitVal::Double(*bits),
                    // The types have no float literal: a float constant's type is `Float`.
                    Const::Float(_) => return self.b.t_float,
                    Const::Char(v) => LitVal::Char(*v as u16),
                    Const::Bool(b) => LitVal::Bool(*b),
                    Const::Str(n) => {
                        let text = tasty.name(*n);
                        LitVal::Str(self.interner.intern(&text))
                    }
                    _ => return self.blocked(cx, Shape::ConstantType, t),
                };
                self.types.lit(value)
            }
            TType::Unknown(_) => self.blocked(cx, Shape::Unknown, t),
        }
    }

    fn loaded_file(&self, cx: &MapCx) -> &super::LoadedFile {
        self.loaded.as_ref().unwrap().file(cx.file)
    }

    fn loaded_tables(&self, cx: &MapCx) -> &super::FileTables {
        self.file_tables(cx.file)
    }

    fn loaded_tables_mut(&mut self, cx: &MapCx) -> &mut super::FileTables {
        &mut self.loaded_mut().tables[cx.file as usize]
    }

    fn blocked(&mut self, cx: &MapCx, shape: Shape, t: &TType) -> TypeId {
        let tasty = self.tasty(cx.file);
        let spelling = Printer::new(&tasty, false).ty(t);
        let description = format!("{}: {}", shape.describe(), spelling);
        self.loaded_mut().note_blocked(&description);
        self.types.blocked(&description)
    }

    fn self_type_of(&mut self, c: ClassId) -> TypeId {
        let targs: Vec<TypeId> = self.syms.class(c).tparams.clone().iter().map(|&p| self.types.param(p)).collect();
        self.types.class(c, &targs)
    }

    fn map_local_type(&mut self, cx: &mut MapCx, addr: Addr, prefix: Option<&TType>, t: &TType) -> TypeId {
        let lf = self.loaded_tables(cx);
        if let Some(&p) = lf.tparams.get(&addr) {
            return self.types.param(p);
        }
        if let Some(&c) = lf.classes.get(&addr) {
            if self.is_inner_class(c) {
                let through = match prefix {
                    Some(p) => self.prefix_type(cx, p),
                    None => match self.syms.class(c).owner {
                        Owner::Class(o) => Some(self.this_prefix(o)),
                        _ => None,
                    },
                };
                if let Some(ty) = through.and_then(|p| self.inner_class_through(c, p)) {
                    return ty;
                }
            }
            return self.class_ref(c);
        }
        if let Some(&a) = lf.aliases.get(&addr) {
            let (abstract_member, owner, name) = {
                let info = &self.syms.aliases[a.idx()];
                (info.is_abstract(), info.owner, info.name)
            };
            if !abstract_member {
                // An alias a base class declares over its parameters, seen from a subclass.
                if let (Some(TType::This(inner)), Owner::Class(o)) = (prefix, owner) {
                    if let Some(c) = self.loaded_this_class(cx, inner).filter(|&c| c != o) {
                        let through = self.this_prefix(c);
                        if let Some(ty) = self.member_type(through, name) {
                            return ty;
                        }
                    }
                }
                return self.type_ref_to_type(TypeRef::Alias(a), crate::source::Span::default(), false);
            }
        }
        let tasty = self.tasty(cx.file);
        // A type variable of a body's pattern that the case does not name, inside a type the
        // converter hands over whole (`HashMap[E, _$3]#HashKeySet`): a wildcard.
        if Decoder::new(&tasty).tag_at(addr) == crate::tasty::tags::BIND {
            return WILD;
        }
        let kind = Decoder::new(&tasty).local_type_kind(addr);
        let shape = match kind {
            LocalKind::AbstractMember => match prefix {
                Some(TType::TermRef(..)) | Some(TType::LocalTerm(..)) => Shape::PathDependent,
                _ => Shape::TypeMember,
            },
            LocalKind::TypeParam => Shape::Unknown,
            _ => Shape::Unknown,
        };
        if kind == LocalKind::AbstractMember {
            if let (Some(p), Some(n)) = (prefix, Decoder::new(&tasty).name_at(addr)) {
                let name = self.intern_name(&tasty, n);
                if let Some(prefix_ty) = self.prefix_type(cx, p) {
                    if let Some(ty) = self.member_type(prefix_ty, name) {
                        return ty;
                    }
                }
            }
        }
        self.blocked(cx, shape, t)
    }

    /// `S match { case P => T ... }` from a library: each case's binders become type parameters
    /// registered by the address of their `BIND`, which the pattern and the body name.
    fn map_match(&mut self, cx: &mut MapCx, scrutinee: &TType, bound: Option<&TType>, cases: &[crate::tasty::tree::TMatchCase], t: &TType) -> TypeId {
        let s = self.map_type(cx, scrutinee);
        let b = bound.map_or(ANY, |b| self.map_type(cx, b));
        let mut out = Vec::with_capacity(cases.len());
        for c in cases {
            let tasty = self.tasty(cx.file);
            let mut ids: Vec<TParamId> = c
                .binders
                .iter()
                .map(|&(addr, name, _)| {
                    let n = self.lname(&tasty, name);
                    let id = self.syms.new_tparam(n, 0);
                    self.loaded_tables_mut(cx).tparams.insert(addr, id);
                    id
                })
                .collect();
            for ((_, _, bounds), &id) in c.binders.iter().zip(&ids) {
                if let Some(info) = bounds {
                    self.set_tparam_bounds(cx, id, info);
                }
            }
            if let Some((binder, params)) = &c.lambda {
                let lambda_ids: Vec<TParamId> = params
                    .iter()
                    .map(|p| {
                        let n = self.lname(&tasty, p.name);
                        let id = self.syms.new_tparam(n, 0);
                        self.syms.tparams[id.idx()].arity = hk_arity(&p.info);
                        id
                    })
                    .collect();
                cx.binders.push((*binder, lambda_ids.clone()));
                for (p, &id) in params.iter().zip(&lambda_ids) {
                    self.set_tparam_bounds(cx, id, &p.info);
                }
                ids.extend(lambda_ids);
            }
            let pattern = self.map_type(cx, &c.pattern);
            let body = self.map_type(cx, &c.body);
            if c.lambda.is_some() {
                cx.binders.pop();
            }
            let binders: Vec<TypeId> = ids.iter().map(|&p| self.types.param(p)).collect();
            let binders = self.types.list(&binders);
            out.push(MatchCase { binders, pattern, body });
        }
        if out.is_empty() {
            return self.blocked(cx, Shape::MatchType, t);
        }
        self.types.match_type(s, &out, b)
    }

    /// The class of a `C.this`.
    fn loaded_this_class(&mut self, cx: &mut MapCx, inner: &TType) -> Option<ClassId> {
        match inner {
            TType::LocalType(addr, _) => self.loaded_tables(cx).classes.get(addr).copied(),
            TType::TypeRef(..) => {
                // The class whatever its prefix (`classSymbol` of a `TypeRef(p, C)`).
                let ty = self.map_type_ctor(cx, inner);
                match self.types.get(ty) {
                    Type::Ctor(c) => Some(c),
                    Type::Lambda(_, body) => self.types.named_class(body),
                    _ => self.types.named_class(ty),
                }
            }
            _ => None,
        }
    }

    /// The prefix a type member is seen through: `C.this`, a path, or a class type for a
    /// projection.
    fn prefix_type(&mut self, cx: &mut MapCx, prefix: &TType) -> Option<TypeId> {
        match prefix {
            TType::This(inner) => {
                let c = self.loaded_this_class(cx, inner)?;
                Some(self.this_prefix(c))
            }
            TType::TermRef(..) | TType::LocalTerm(..) | TType::ParamRef(..) => self.path_singleton(cx, prefix),
            TType::RecThis(_) | TType::Rec(..) => None,
            other => {
                // A class through a prefix (`o.Inner[A]#T`, dotty's `TypeRef(o, Inner)`) is a
                // prefix with its own: the member is seen from it.
                let ty = self.map_type(cx, other);
                let ty = self.deref_alias(ty);
                match self.types.get(ty) {
                    Type::Class(..) | Type::Nested(..) | Type::Refined(..) => Some(ty),
                    _ => None,
                }
            }
        }
    }

    /// `T { type A = X; def m: R }`: the parent refined member by member; a term member gets a
    /// symbol carrying its signature. None for a member of a shape teq does not read.
    fn map_refined(&mut self, cx: &mut MapCx, parent: &TType, members: &[(u32, Option<TType>)]) -> Option<TypeId> {
        let mut t = self.map_type(cx, parent);
        if self.types.contains_error(t) {
            return None;
        }
        let tasty = self.tasty(cx.file);
        let file_id = self.loaded_file(cx).file_id;
        for (n, info) in members {
            let name = self.intern_name(&tasty, *n);
            let info = info.as_ref()?;
            let r = match info {
                TType::Alias(a) => Refinement::Alias(name, self.map_type_ctor(cx, a)),
                TType::Bounds(lo, hi) => Refinement::Bounds(name, self.map_bound(cx, lo, false), self.map_bound(cx, hi, true)),
                TType::Lambda { kind: LambdaKind::Type, result, .. } => match &**result {
                    TType::Bounds(..) => Refinement::Bounds(name, self.map_bound(cx, info, false), self.map_bound(cx, info, true)),
                    _ => Refinement::Alias(name, self.map_type_ctor(cx, info)),
                },
                TType::ByName(ret) => {
                    let ty = self.map_type(cx, ret);
                    let sym = self.syms.new_sym(name, SymKind::Def, crate::ast::mods::ABSTRACT, Owner::Local, file_id, None, crate::source::Span::default());
                    let mut s = self.syms.sym_mut(sym);
                    s.sig = Some(std::sync::Arc::new(MethodSig::value(ty)));
                    s.state().set(Completion::Done);
                    Refinement::Term(name, sym, self.types.list(&[ty]))
                }
                TType::Lambda { kind: LambdaKind::Method | LambdaKind::Poly, .. } => self.map_refined_method(cx, name, info)?,
                other => {
                    let ty = self.map_type(cx, other);
                    let sym = self.syms.new_sym(name, SymKind::Val, crate::ast::mods::ABSTRACT, Owner::Local, file_id, None, crate::source::Span::default());
                    let mut s = self.syms.sym_mut(sym);
                    s.sig = Some(std::sync::Arc::new(MethodSig::value(ty)));
                    s.state().set(Completion::Done);
                    let r = self.types.refine(Refinement::Val(name, sym, ty));
                    t = self.types.mk(Type::Refined(t, r));
                    continue;
                }
            };
            let r = self.types.refine(r);
            t = self.types.mk(Type::Refined(t, r));
        }
        Some(t)
    }

    /// A refinement's method, `def run(c: Ctx)(x: c.T): c.T` or `def run[A](a: A): a.type`: a
    /// symbol carrying its signature, the type parameters bound while the clauses map and each
    /// clause's parameters from its own parameter types on. None for a shape teq does not read.
    fn map_refined_method(&mut self, cx: &mut MapCx, name: Name, info: &TType) -> Option<Refinement> {
        let file_id = self.loaded_file(cx).file_id;
        let (tparams, mut cur) = match info {
            TType::Lambda { kind: LambdaKind::Poly, binder, params, result } => (self.map_lambda_tparams(cx, *binder, params), &**result),
            _ => (Vec::new(), info),
        };
        let mark = (cx.binders.len(), cx.term_binders.len());
        if !tparams.is_empty() {
            let TType::Lambda { kind: LambdaKind::Poly, binder, .. } = info else { unreachable!() };
            cx.binders.push((*binder, tparams.clone()));
        }
        let mut clauses = Vec::new();
        while let TType::Lambda { kind: LambdaKind::Method, binder, params, result } = cur {
            clauses.push(self.map_clause(cx, *binder, params));
            cur = result;
        }
        let ret = if matches!(cur, TType::Lambda { .. }) { None } else { Some(self.map_type(cx, cur)) };
        cx.binders.truncate(mark.0);
        cx.term_binders.truncate(mark.1);
        let ret = ret?;
        let sym = self.syms.new_sym(name, SymKind::Def, crate::ast::mods::ABSTRACT, Owner::Local, file_id, None, crate::source::Span::default());
        let sig = MethodSig { tparams, clauses, ret };
        let l = self.sig_types(&sig);
        let mut s = self.syms.sym_mut(sym);
        s.sig = Some(std::sync::Arc::new(sig));
        s.state().set(Completion::Done);
        Some(Refinement::Term(name, sym, l))
    }

    /// A method type's clause: its parameters' symbols, in scope (`term_binders`) for their
    /// own types, `(c: Ctx, x: c.T)`, and after them until the caller takes them out.
    fn map_clause(&mut self, cx: &mut MapCx, binder: Addr, params: &[crate::tasty::tree::TParam]) -> ClauseSig {
        let tasty = self.tasty(cx.file);
        let file_id = self.loaded_file(cx).file_id;
        let syms: Vec<(Name, SymId)> = params
            .iter()
            .map(|p| {
                let pname = self.intern_name(&tasty, p.name);
                (pname, self.syms.new_sym(pname, SymKind::Param, 0, Owner::Local, file_id, None, crate::source::Span::default()))
            })
            .collect();
        cx.term_binders.push((binder, params.iter().zip(&syms).map(|(tp, &(_, s))| (tp.addr, s)).collect()));
        let mut ps = Vec::with_capacity(params.len());
        for (p, &(pname, psym)) in params.iter().zip(&syms) {
            let (ty, by_name, repeated) = match &p.info {
                TType::ByName(inner) => (self.map_type(cx, inner), true, false),
                other => match repeated_element(&tasty, other) {
                    Some(elem) => (self.map_type(cx, elem), false, true),
                    None => (self.map_type(cx, other), false, false),
                },
            };
            let mut s = self.syms.sym_mut(psym);
            s.sig = Some(std::sync::Arc::new(MethodSig::value(ty)));
            s.state().set(Completion::Done);
            ps.push(ParamSig { name: pname, ty, by_name, repeated, has_default: false, sym: psym });
        }
        let flags = params.first().map(|p| p.flags);
        let is_using = flags.map_or(false, |f| f.has(crate::tasty::tags::GIVEN));
        let is_implicit = flags.map_or(false, |f| f.has(crate::tasty::tags::IMPLICIT));
        ClauseSig { params: ps, is_using, is_implicit }
    }

    /// A polymorphic method type's type parameters, their bounds mapped with them in scope.
    fn map_lambda_tparams(&mut self, cx: &mut MapCx, binder: Addr, params: &[crate::tasty::tree::TParam]) -> Vec<TParamId> {
        let tasty = self.tasty(cx.file);
        let ids: Vec<TParamId> = params
            .iter()
            .map(|p| {
                let n = self.intern_name(&tasty, tasty.source_name(p.name));
                let id = self.syms.new_tparam(n, 0);
                self.syms.tparams[id.idx()].arity = hk_arity(&p.info);
                if p.addr != 0 {
                    self.loaded_tables_mut(cx).tparams.insert(p.addr, id);
                }
                id
            })
            .collect();
        cx.binders.push((binder, ids.clone()));
        for (p, &id) in params.iter().zip(&ids) {
            if let TType::Bounds(lo, hi) = &p.info {
                let (l, h) = (self.map_type(cx, lo), self.map_type(cx, hi));
                let info = &mut self.syms.tparams[id.idx()];
                info.lower = l;
                info.upper = h;
            }
        }
        cx.binders.pop();
        ids
    }

    /// The std package of a Java class that scala-library's `scala` package object aliases
    /// (`scala.package.Throwable` is `java.lang.Throwable`), when the std binds and defines it.
    fn scala_package_java_alias(&mut self, pkg: PkgId, name: Name) -> Option<PkgId> {
        if pkg != self.b.scala_pkg || !self.std_binds() {
            return None;
        }
        let to = SCALA_PACKAGE_JAVA_ALIASES.iter().find(|(n, _)| *n == self.interner.get(name)).map(|(_, to)| *to)?;
        let mut target = ROOT_PKG;
        for seg in to.split('.') {
            let n = self.interner.intern(seg);
            target = self.syms.pkg(target).entries.get(&n).and_then(|e| e.pkg)?;
        }
        match self.pkg_type(target, name)? {
            TypeRef::Class(c) if !self.in_jar(self.syms.class(c).file) => Some(target),
            TypeRef::Alias(a) if !self.in_jar(self.syms.aliases[a.idx()].file) => Some(target),
            _ => None,
        }
    }

    fn class_ref(&mut self, c: ClassId) -> TypeId {
        let c = self.loaded.as_ref().unwrap().builtin_of.get(&c).copied().unwrap_or(c);
        if self.class_arity(c) == 0 {
            self.types.class(c, &[])
        } else {
            self.types.mk(Type::Ctor(c))
        }
    }

    fn map_type_ref(&mut self, cx: &mut MapCx, prefix: &TType, n: u32, t: &TType) -> TypeId {
        let tasty = self.tasty(cx.file);
        // scalac's alias of `java.lang.Object` where a Java member takes or gives one, in its
        // `<special-ops>` package: `Any`, so that a boxed value passes, as the class files'
        // `Object` is read.
        if tasty.simple(n) == Some("<FromJavaObject>") && matches!(prefix, TType::Package(p) if tasty.simple(*p) == Some("<special-ops>")) {
            return ANY;
        }
        // `p.package.T` is the package object's alias of what the std defines as `p.T` itself,
        // and binds there when the std is present.
        if let TType::TermRef(pkg, obj) = prefix {
            let package_object = matches!(&**pkg, TType::Package(_)) && is_package_object_name(tasty.simple(tasty.source_name(*obj)));
            if package_object {
                let TType::Package(p) = &**pkg else { unreachable!() };
                let name = self.intern_name(&tasty, n);
                let pkg_id = self.resolve_package(cx, *p);
                let std_defined = match self.pkg_type(pkg_id, name) {
                    Some(TypeRef::Class(c)) => !self.in_jar(self.syms.class(c).file),
                    Some(TypeRef::Alias(a)) => !self.in_jar(self.syms.aliases[a.idx()].file),
                    _ => false,
                };
                if std_defined {
                    return self.type_in_pkg(cx, pkg_id, name, t);
                }
                if let Some(java_pkg) = self.scala_package_java_alias(pkg_id, name) {
                    return self.type_in_pkg(cx, java_pkg, name, t);
                }
            }
        }
        if tasty.is_object_class(n) {
            let name = self.intern_name(&tasty, tasty.source_name(n));
            return match self.object_in(cx, prefix, name) {
                Some(c) => self.types.class(c, &[]),
                None => self.blocked(cx, Shape::Singleton, t),
            };
        }
        // `({ type lambda[X, Y] = C[(X, Y)] })#lambda`, the type lambda spelt as a refinement:
        // the alias itself.
        if let TType::Refined(_, members) = prefix {
            let wanted = tasty.name(n);
            if let Some((_, Some(member))) = members.iter().find(|(m, _)| tasty.name(*m) == wanted) {
                return match member {
                    TType::Bounds(_, hi) => self.map_type_ctor(cx, hi),
                    other => self.map_type_ctor(cx, other),
                };
            }
        }
        // `TypeBox[L, H]#CAP` is scalac's capture of a wildcard `? >: L <: H`: what its upper
        // bound allows.
        if let (TType::Applied(box_ref, bounds), Some("CAP")) = (prefix, tasty.simple(n)) {
            if let (TType::TypeRef(box_prefix, box_name), [_, hi]) = (&**box_ref, bounds.as_slice()) {
                if tasty.simple(*box_name) == Some("TypeBox") && matches!(&**box_prefix, TType::Package(p) if tasty.name(*p) == "scala.runtime") {
                    return self.map_type(cx, hi);
                }
            }
        }
        let name = self.intern_name(&tasty, n);
        let package = match prefix {
            TType::Package(p) => Some(*p),
            TType::This(inner) => match &**inner {
                TType::Package(p) => Some(*p),
                _ => None,
            },
            _ => None,
        };
        if let Some(p) = package {
            let pkg = self.resolve_package(cx, p);
            return self.type_in_pkg(cx, pkg, name, t);
        }
        match prefix {
            TType::RecThis(_) => self.blocked(cx, Shape::Recursive, t),
            other => match self.scope_of(cx, other) {
                Some(Scope::Class(c)) => {
                    let found = match self.member_type_of(c, name) {
                        Some(TypeRef::Alias(a)) if self.syms.aliases[a.idx()].is_abstract() => {
                            let prefix = self.this_prefix(c);
                            self.member_type(prefix, name).unwrap_or_else(|| self.blocked(cx, Shape::TypeMember, t))
                        }
                        // An alias a base class declares over its parameters (`K0.LiftP` for
                        // `Kind`'s) is seen through the prefix.
                        Some(TypeRef::Alias(a)) if self.syms.aliases[a.idx()].owner != Owner::Class(c) && self.syms.class(c).kind == ClassKind::Object => {
                            let prefix = self.this_prefix(c);
                            match self.member_type(prefix, name) {
                                Some(ty) => ty,
                                None => self.type_ref_to_type(TypeRef::Alias(a), crate::source::Span::default(), false),
                            }
                        }
                        // One a base class or the self type declares names that class's `this`,
                        // which is `c`'s here (`Impl.this.Type` in `Exs`'s `?<` read in `Impl`).
                        // Over the base class's parameters, what `c` extends it with (`Value`
                        // of `FiberRef[A]` in `PatchFiber[Value0]`).
                        Some(TypeRef::Alias(a)) if self.syms.aliases[a.idx()].owner != Owner::Class(c) => {
                            let prefix = self.this_prefix(c);
                            match self.member_type(prefix, name) {
                                Some(ty) => ty,
                                None => {
                                    let expanded = self.type_ref_to_type(TypeRef::Alias(a), crate::source::Span::default(), false);
                                    self.as_seen_from(expanded, prefix, c)
                                }
                            }
                        }
                        // An opaque type of a trait through an object deriving from it is the
                        // object's own (`HtmlTagOf.Tag` of `TagLite`).
                        Some(TypeRef::Class(k))
                            if self.is_opaque_path_class(k) && self.syms.class(c).kind == ClassKind::Object && self.syms.class(k).owner != Owner::Class(c) =>
                        {
                            let d = self.derived_opaque(k, c);
                            self.class_ref(d)
                        }
                        // A class nested in a class through the path written (an object nested in
                        // a class, `Outer.this.Existential.Bounded`), or the class's `this`.
                        Some(TypeRef::Class(k)) if self.is_inner_class(k) => {
                            let path = match other {
                                TType::This(_) => None,
                                _ => self.prefix_type(cx, other).filter(|&p| self.types.is_path(p)),
                            };
                            let prefix = path.unwrap_or_else(|| self.this_prefix(c));
                            self.inner_class_through(k, prefix).unwrap_or_else(|| self.class_ref(k))
                        }
                        Some(r) => self.type_ref_to_type(r, crate::source::Span::default(), false),
                        None if matches!(other, TType::Applied(..)) && self.projected_type_argument(cx, other, name).is_some() => {
                            self.projected_type_argument(cx, other, name).unwrap()
                        }
                        None => {
                            let prefix = self.this_prefix(c);
                            // `this.T` in a trait whose self type is an object (`this: JsonDecoder.type
                            // =>`) is that object's member, and in one whose self type names traits
                            // (`_: FocusBase & KeywordParserBase =>`) a member of those.
                            match self.member_type(prefix, name) {
                                Some(found) => found,
                                None => {
                                    let through_self = self.self_type_classes(c).into_iter().find_map(|k| self.member_type_of(k, name));
                                    match through_self {
                                        Some(r) => self.type_ref_to_type(r, crate::source::Span::default(), false),
                                        None => self.blocked(cx, Shape::TypeMember, t),
                                    }
                                }
                            }
                        }
                    };
                    self.seen_from_inner_object_path(cx, other, c, found)
                }
                Some(Scope::Pkg(p)) => self.type_in_pkg(cx, p, name, t),
                None => {
                    // `C[A]#Inner` names the class nested in `C`, with `C`'s arguments.
                    if let Some(nested) = self.projected_class(cx, other, name) {
                        if self.is_inner_class(nested) {
                            let prefix = self.map_type(cx, other);
                            if let Some(ty) = self.inner_class_through(nested, prefix) {
                                return ty;
                            }
                        }
                        return self.types.class(nested, &[]);
                    }
                    // `C[?]#T` for a type parameter `T` of `C`: the argument at its position, a
                    // wildcard read as its upper bound (`EndpointInput[?]#T`, which scalac
                    // infers for a call on a wildcard-typed receiver).
                    if let Some(arg) = self.projected_type_argument(cx, other, name) {
                        return arg;
                    }
                    let shape = match other {
                        TType::TermRef(..) | TType::LocalTerm(..) => Shape::PathDependent,
                        TType::This(_) => Shape::TypeMember,
                        _ => Shape::Projection,
                    };
                    let found = match self.prefix_type(cx, other) {
                        Some(p) => self
                            .member_type(p, name)
                            .or_else(|| self.nested_class_on_path(p, name))
                            .or_else(|| self.object_type_on_path(p, name))
                            .or_else(|| self.class_param_argument(p, name)),
                        None => None,
                    };
                    match found {
                        Some(ty) => ty,
                        None => self.blocked(cx, shape, t),
                    }
                }
            },
        }
    }

    /// `C[A, ?]#T` where `T` is a type parameter of `C`: the argument at its position, `A` for
    /// the first, the wildcard itself for the second, so that it agrees with the argument a
    /// receiver of type `C[?]` gives the member's `T`.
    fn projected_type_argument(&mut self, cx: &mut MapCx, prefix: &TType, name: Name) -> Option<TypeId> {
        if !matches!(prefix, TType::Applied(..)) {
            return None;
        }
        let mapped = self.map_type(cx, prefix);
        let Type::Class(c, targs) = self.types.get(self.types.strip_nested(mapped)) else { return None };
        self.complete_class_tparams(c);
        let index = self.syms.class(c).tparams.iter().position(|&tp| self.syms.tparam(tp).name == name)?;
        self.types.items(targs).get(index).copied()
    }

    /// `x.A` for a path whose class has the type parameter `A`: scalac's capture of the
    /// argument, the argument itself (`refs0.A` for `refs0: Iterator[? <: T]`).
    fn class_param_argument(&mut self, prefix: TypeId, name: Name) -> Option<TypeId> {
        let under = self.dealias(prefix);
        let Type::Class(c, targs) = self.types.get(self.types.strip_nested(under)) else { return None };
        self.complete_class_tparams(c);
        let index = self.syms.class(c).tparams.iter().position(|&tp| self.syms.tparam(tp).name == name)?;
        self.types.items(targs).get(index).copied()
    }

    fn type_in_pkg(&mut self, cx: &mut MapCx, pkg: PkgId, name: Name, t: &TType) -> TypeId {
        if pkg == self.b.scala_pkg && self.interner.get(name) == "AnyKind" {
            return self.blocked(cx, Shape::AnyKind, t);
        }
        if let Some(special) = self.special_scala_type(pkg, name) {
            return special;
        }
        // `Predef.String` and the other aliases of `scala.Predef` when no jar defines the
        // object: the types they stand for, which the std has at the level of package `scala`.
        if self.is_predef_pkg(pkg) {
            if self.interner.get(name) == "String" {
                return self.b.t_string;
            }
            // `Predef.Class`, scala-library's alias of `java.lang.Class`.
            if self.interner.get(name) == "Class" {
                if let Some(r) = self.java_lang_pkg().and_then(|lang| self.pkg_type(lang, name)) {
                    return self.type_ref_to_type(r, crate::source::Span::default(), false);
                }
            }
            if let Some(special) = self.special_scala_type(self.b.scala_pkg, name) {
                return special;
            }
            if let Some(r) = self.pkg_type(self.b.scala_pkg, name) {
                return self.type_ref_to_type(r, crate::source::Span::default(), false);
            }
        }
        match self.pkg_type(pkg, name) {
            Some(r) => self.type_ref_to_type(r, crate::source::Span::default(), false),
            None => match self.placeholder_class(pkg, name, cx.applied_to.unwrap_or(0)) {
                Some(c) => self.class_ref(c),
                None => self.blocked(cx, Shape::Unknown, t),
            },
        }
    }

    /// A type member `name` of the class `c`: a nested class, an alias or an opaque type, of
    /// `c` or of its ancestors, entered when the class was.
    /// The classes a trait's declared self type names, an intersection flattened.
    pub(in crate::typer) fn self_type_classes(&mut self, c: ClassId) -> Vec<ClassId> {
        let mut out = Vec::new();
        let Some(declared) = self.syms.class(c).declared_self else { return out };
        let mut stack = vec![declared];
        while let Some(t) = stack.pop() {
            match self.types.get(t) {
                Type::Class(k, _) => out.push(k),
                Type::Nested(_, class) => out.extend(self.types.named_class(class)),
                Type::Inter(a, b) => {
                    stack.push(b);
                    stack.push(a);
                }
                _ => {}
            }
        }
        out
    }

    /// `PolyFunction { def apply[T](a: A, b: B): C }` as `[T] => (A, B) => C`: the one member's
    /// polymorphic method type, its type parameters bound while its parameters and result map.
    fn map_poly_function(&mut self, cx: &mut MapCx, members: &[(u32, Option<TType>)]) -> Option<TypeId> {
        let tasty = self.tasty(cx.file);
        let [(name, Some(info))] = members else { return None };
        if tasty.simple(*name) != Some("apply") {
            return None;
        }
        let TType::Lambda { kind: LambdaKind::Poly, binder, params, result } = info else { return None };
        let ids = self.map_lambda_tparams(cx, *binder, params);
        cx.binders.push((*binder, ids.clone()));
        let fun = match &**result {
            TType::Lambda { kind: LambdaKind::Method, binder: m, params: term_params, result: ret } => {
                let mark = cx.term_binders.len();
                let clause = self.map_clause(cx, *m, term_params);
                let r = self.map_type(cx, ret);
                cx.term_binders.truncate(mark);
                Some(self.poly_apply_fun(cx, clause, r))
            }
            _ => None,
        };
        cx.binders.pop();
        let fun = fun?;
        let ps: Vec<TypeId> = ids.iter().map(|&p| self.types.param(p)).collect();
        Some(self.poly_type(&ps, fun))
    }

    /// The function type of a polymorphic function's `apply` of the clause `clause` and the
    /// result `r`, read whole: a function type with named parameters, the names the pickle
    /// holds (scalac's `x$1` too, which a call may name), as `TreeUnpickler.readMethodic` keeps
    /// what it reads and teq types `[A] => A => A` from the source (`makePolyFunctionType`).
    fn poly_apply_fun(&mut self, cx: &MapCx, clause: ClauseSig, r: TypeId) -> TypeId {
        let file = self.loaded_file(cx).file_id;
        self.named_fun_type_in(file, clause.is_using, clause.params, r, crate::source::Span::default())
    }

    /// A type member `name` of the class `c` or its ancestors: a nested class or an alias. An
    /// object nested under the same name is no type (`object Expr` beside `type Expr[A]`).
    pub(super) fn member_type_of(&mut self, c: ClassId, name: Name) -> Option<TypeRef> {
        if self.syms.class(c).state() == Completion::NotStarted {
            self.complete_class(c);
        }
        let mut owners = vec![c];
        owners.extend(self.syms.class(c).base_types.iter().skip(1).map(|&(b, _)| b));
        for k in owners {
            if let Some(&n) = self.syms.class(k).nested.get(&name) {
                if !self.is_object_class(n) {
                    return Some(TypeRef::Class(n));
                }
            }
            if let Some(&a) = self.syms.class(k).type_aliases.get(&name) {
                return Some(TypeRef::Alias(a));
            }
        }
        None
    }

    fn is_object_class(&self, c: ClassId) -> bool {
        self.syms.class(c).kind == ClassKind::Object || self.syms.class(c).inner_object.is_some()
    }

    fn intern_name(&mut self, tasty: &TastyFile, n: u32) -> Name {
        let text = tasty.name(n);
        self.interner.intern(&text)
    }

    /// The types of package `scala` that teq defines itself, and the Java classes it stands
    /// in for.
    fn special_scala_type(&mut self, pkg: PkgId, name: Name) -> Option<TypeId> {
        if pkg == self.b.scala_pkg {
            let text = self.interner.get(name);
            return match text {
                "Any" | "Matchable" | "Singleton" => Some(ANY),
                "AnyRef" => Some(self.b.t_any_ref),
                "AnyVal" => Some(self.b.t_any_val),
                "Nothing" => Some(NOTHING),
                _ => {
                    if let Some((context, arity)) = function_class(text) {
                        let c = if context { self.context_function_class(arity) } else { self.function_class(arity) };
                        return Some(self.types.mk(Type::Ctor(c)));
                    }
                    if let Some(digits) = text.strip_prefix("Tuple") {
                        if let Ok(n) = digits.parse::<usize>() {
                            if n > 0 && n <= 22 {
                                let c = self.tuple_class(n);
                                return Some(self.types.mk(Type::Ctor(c)));
                            }
                        }
                    }
                    None
                }
            };
        }
        if self.is_java_lang(pkg) {
            let text = self.interner.get(name);
            return match text {
                // What a Scala source wrote as `AnyRef`; a Java signature's `Object` would be
                // `Any`, as scalac reads it, once a class file reader supplies one.
                "Object" => Some(self.b.t_any_ref),
                "String" => Some(self.b.t_string),
                _ => None,
            };
        }
        None
    }

    /// A class nested in the object a path leads to: `q.reflect.Symbol` for `q: Quotes`, whose
    /// `reflect` the std defines as an object.
    /// The class of an object a path selects, which a type of that name stands for where the
    /// std has an object for scalac's trait and given (`q.reflect.TypeReprMethods`).
    fn object_type_on_path(&mut self, prefix: TypeId, name: Name) -> Option<TypeId> {
        let under = self.widen_path(prefix);
        let (s, _) = self.find_member(under, name)?;
        match self.syms.sym(s).kind {
            SymKind::Object(c) => Some(self.types.class(c, &[])),
            _ => None,
        }
    }

    fn nested_class_on_path(&mut self, prefix: TypeId, name: Name) -> Option<TypeId> {
        let nested = self.nested_class_through(prefix, name)?;
        if self.is_inner_class(nested) {
            if let Some(ty) = self.inner_class_through(nested, prefix) {
                return Some(ty);
            }
        }
        Some(self.class_ref(nested))
    }

    fn is_predef_pkg(&self, pkg: PkgId) -> bool {
        let info = self.syms.pkg(pkg);
        self.interner.get(info.name) == "Predef" && info.parent == Some(self.b.scala_pkg)
    }

    fn is_java_lang(&self, pkg: PkgId) -> bool {
        let info = self.syms.pkg(pkg);
        self.interner.get(info.name) == "lang"
            && info.parent.map_or(false, |p| self.interner.get(self.syms.pkg(p).name) == "java" && self.syms.pkg(p).parent == Some(ROOT_PKG))
    }

    /// A class that no TASTy file defines, so one written in Java (of the JDK, or of a jar's
    /// own Java sources such as `scala.math.ScalaNumber`): a trait without members until it is
    /// completed, which reads its class file.
    pub(super) fn placeholder_class(&mut self, pkg: PkgId, name: Name, arity: usize) -> Option<ClassId> {
        let file = self.loaded.as_ref().unwrap().java.jdk_file;
        let c = self.syms.new_class(name, ClassKind::Trait, 0, Owner::Package(pkg), file, None, crate::source::Span::default());
        let tparams: Vec<TParamId> = (0..arity)
            .map(|i| {
                let n = self.interner.intern(&format!("T{}", i + 1));
                self.syms.new_tparam(n, 0)
            })
            .collect();
        let targs: Vec<TypeId> = tparams.iter().map(|&p| self.types.param(p)).collect();
        let self_ty = self.types.class(c, &targs);
        let mut info = self.syms.class_mut(c);
        info.tparams = tparams;
        info.base_types.push((c, self_ty));
        self.syms.pkgs[pkg.idx()].entries.entry(name).or_default().class = Some(c);
        self.loaded_mut().java.add_placeholder(c, pkg, name);
        Some(c)
    }

    pub(super) fn resolve_package(&mut self, cx: &MapCx, n: u32) -> PkgId {
        if let Some(&p) = self.loaded_tables(cx).packages.get(&n) {
            return p;
        }
        let tasty = self.tasty(cx.file);
        let mut segments = Vec::new();
        tasty.segments(n, &mut segments);
        let mut p = ROOT_PKG;
        for seg in segments {
            let text = tasty.name(seg);
            if text == "<empty>" || text == "_root_" {
                continue;
            }
            let name = self.interner.intern(&text);
            // A package of the std's locked Scala.js layer (`org.portablescala.reflect`) opens
            // the layer as a program's path through it would; created bare instead, it would
            // hide the layer's definitions from every later lookup.
            p = match self.demand_std_package(p, name) {
                Some(sub) => sub,
                None => self.syms.sub_pkg(p, name),
            };
        }
        self.loaded_tables_mut(cx).packages.insert(n, p);
        p
    }

    /// The object `name` in the scope the prefix names.
    pub(super) fn object_in(&mut self, cx: &mut MapCx, prefix: &TType, name: Name) -> Option<ClassId> {
        let sym = match self.scope_of(cx, prefix)? {
            Scope::Pkg(p) => self.pkg_term(p, name)?.sym()?,
            Scope::Class(c) => {
                self.complete_class(c);
                match self.module_term(c, name) {
                    Some(r) => r.sym()?,
                    // An object a base trait nests (`Promise.this.Strategy` of `PromiseModule`'s),
                    // or a member of the object a trait's self type names (`this: JsonDecoder.type =>`).
                    None => match self.inherited_module_term(c, name) {
                        Some(s) => s,
                        None => {
                            let k = self.self_type_object(c)?;
                            self.complete_class(k);
                            self.module_term(k, name)?.sym()?
                        }
                    },
                }
            }
        };
        match self.syms.sym(sym).kind {
            SymKind::Object(c) => Some(c),
            // A val that stands for an object (`val JsonError = zio.json.JsonError`): the object.
            SymKind::Val => {
                let ret = self.sig_of(sym).ret;
                match self.types.get(ret) {
                    Type::Class(o, _) if self.syms.class(o).kind == ClassKind::Object || self.syms.class(o).inner_object.is_some() => Some(o),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// A member of an object nested in a class, reached through a path to it
    /// (`TransformerMacros.this.ExistentialType.UpperBounded[..]`), names the enclosing class's
    /// `this` as the path's prefix (`Existentials.this.Type` there is `TransformerMacros.this.Type`).
    fn seen_from_inner_object_path(&mut self, cx: &mut MapCx, path: &TType, c: ClassId, t: TypeId) -> TypeId {
        let TType::TermRef(outer, _) = path else { return t };
        if !self.types.has_paths(t) || self.syms.class(c).inner_object.is_none() {
            return t;
        }
        let Owner::Class(k) = self.syms.class(c).owner else { return t };
        // The object's own `this` is the path to it, and its enclosing class's what the path
        // selects it on (`AsSeenFromMap.toPrefix` through the object's `TermRef`).
        if let Some(object) = self.prefix_type(cx, path).filter(|&p| self.types.is_path(p)) {
            return self.as_seen_from(t, object, c);
        }
        match self.prefix_type(cx, outer) {
            Some(p) => self.as_seen_from(t, p, k),
            None => t,
        }
    }

    fn inherited_module_term(&mut self, c: ClassId, name: Name) -> Option<SymId> {
        let bases: Vec<ClassId> = self.syms.class(c).base_types.iter().map(|&(b, _)| b).filter(|&b| b != c).collect();
        bases.into_iter().find_map(|b| {
            self.complete_class(b);
            self.module_term(b, name)?.sym()
        })
    }

    /// The object a class's declared self type names, if any.
    fn self_type_object(&mut self, c: ClassId) -> Option<ClassId> {
        let declared = self.syms.class(c).declared_self?;
        match self.types.get(declared) {
            Type::Class(k, _) if self.syms.class(k).kind == ClassKind::Object => Some(k),
            _ => None,
        }
    }

    /// The package or object a static path names.
    pub(super) fn scope_of(&mut self, cx: &mut MapCx, t: &TType) -> Option<Scope> {
        let tasty = self.tasty(cx.file);
        match t {
            TType::Package(p) => Some(Scope::Pkg(self.resolve_package(cx, *p))),
            TType::This(inner) => match &**inner {
                TType::Package(p) => Some(Scope::Pkg(self.resolve_package(cx, *p))),
                TType::LocalType(addr, _) => {
                    let c = self.loaded_tables(cx).classes.get(addr).copied()?;
                    Some(Scope::Class(c))
                }
                TType::TypeRef(prefix, n) if !tasty.is_object_class(*n) => {
                    let name = self.intern_name(&tasty, *n);
                    let r = match self.scope_of(cx, prefix)? {
                        Scope::Pkg(p) => self.pkg_type(p, name)?,
                        Scope::Class(c) => self.member_type_of(c, name)?,
                    };
                    match r {
                        TypeRef::Class(c) => Some(Scope::Class(c)),
                        _ => None,
                    }
                }
                other => self.scope_of(cx, other),
            },
            TType::TermRef(prefix, n) => {
                let name = self.intern_name(&tasty, *n);
                // A package selected from its parent is a term reference too. `Predef.Map` of a
                // body where no jar defines `Predef` is the std's object of package `scala`.
                if let Some(Scope::Pkg(p)) = self.scope_of(cx, prefix) {
                    let p = if self.is_predef_pkg(p) && self.pkg_term(self.b.scala_pkg, name).is_some() { self.b.scala_pkg } else { p };
                    // The statics of a Java class are its companion's, which reading it makes.
                    if let Some(TermRef::Class(c)) = self.pkg_term(p, name) {
                        if self.is_java_placeholder(c) {
                            self.complete_class(c);
                        }
                    }
                    match self.pkg_term(p, name) {
                        Some(TermRef::Package(sub)) => return Some(Scope::Pkg(sub)),
                        Some(r) => {
                            let s = r.sym()?;
                            return match self.syms.sym(s).kind {
                                SymKind::Object(c) => Some(Scope::Class(c)),
                                _ => None,
                            };
                        }
                        None => return Some(Scope::Pkg(self.syms.sub_pkg(p, name))),
                    }
                }
                let c = self.object_in(cx, prefix, name)?;
                Some(Scope::Class(c))
            }
            TType::TypeRef(prefix, n) if tasty.is_object_class(*n) => {
                let name = self.intern_name(&tasty, tasty.source_name(*n));
                let c = self.object_in(cx, prefix, name)?;
                Some(Scope::Class(c))
            }
            TType::LocalTerm(addr, _) => {
                let sym = self.loaded_tables(cx).terms.get(addr).copied()?;
                match self.syms.sym(sym).kind {
                    SymKind::Object(c) => Some(Scope::Class(c)),
                    _ => None,
                }
            }
            TType::LocalType(addr, _) => {
                let c = self.loaded_tables(cx).classes.get(addr).copied()?;
                (self.syms.class(c).kind == ClassKind::Object).then_some(Scope::Class(c))
            }
            _ => None,
        }
    }

    fn var_singleton(&mut self, cx: &mut MapCx, t: &TType) -> bool {
        if !matches!(t, TType::TermRef(..) | TType::LocalTerm(..)) {
            return false;
        }
        match self.path_term(cx, t) {
            Some(s) => self.syms.sym(s).kind == SymKind::Var,
            None => false,
        }
    }

    fn projected_class(&mut self, cx: &mut MapCx, prefix: &TType, name: Name) -> Option<ClassId> {
        if !matches!(prefix, TType::Applied(..) | TType::TypeRef(..)) {
            return None;
        }
        let ctor = self.map_type_ctor(cx, prefix);
        let outer = match self.types.get(ctor) {
            Type::Ctor(k) => k,
            Type::Lambda(_, body) => self.types.named_class(body)?,
            _ => self.types.named_class(ctor).filter(|_| !matches!(self.types.get(ctor), Type::This(_)))?,
        };
        self.complete_class(outer);
        self.syms.class(outer).nested.get(&name).copied()
    }

    /// The symbol a path names: a val, a parameter or an object's val.
    /// `p.package.x`, a term of the package object of `p`, is the std's `p.x` where the std
    /// defines one (`scala.package.Either` is the std's `Either` object).
    fn std_package_object_term(&mut self, cx: &mut MapCx, prefix: &TType, name: Name) -> Option<TermRef> {
        let tasty = self.tasty(cx.file);
        let TType::TermRef(pkg, obj) = prefix else { return None };
        let TType::Package(p) = &**pkg else { return None };
        if !is_package_object_name(tasty.simple(tasty.source_name(*obj))) {
            return None;
        }
        let pkg_id = self.resolve_package(cx, *p);
        let r = self.pkg_term(pkg_id, name)?;
        let std_defined = match r {
            TermRef::Class(c) => !self.in_jar(self.syms.class(c).file),
            other => other.sym().map_or(false, |s| !self.in_jar(self.syms.sym(s).file)),
        };
        std_defined.then_some(r)
    }

    pub(super) fn path_term(&mut self, cx: &mut MapCx, t: &TType) -> Option<SymId> {
        let tasty = self.tasty(cx.file);
        match t {
            TType::LocalTerm(addr, _) => self.loaded_tables(cx).terms.get(addr).copied(),
            TType::TermRef(prefix, n) => {
                let name = self.intern_name(&tasty, *n);
                if let Some(r) = self.std_package_object_term(cx, prefix, name) {
                    return r.sym();
                }
                let r = match self.scope_of(cx, prefix)? {
                    Scope::Pkg(p) => self.pkg_term(p, name)?,
                    Scope::Class(c) => {
                        self.complete_class(c);
                        self.module_term(c, name)?
                    }
                };
                r.sym()
            }
            _ => None,
        }
    }

    /// The singleton type of a path: an object's class, `x.type` for a val or a parameter,
    /// `p.x.type` for a val selected from a path that is no object.
    /// A value parameter of a class named by its address in the type of another member
    /// (`(val q: Quotes)(val t: q.reflect.TypeRepr)`): the parameter's accessor on `this`.
    fn class_param_path(&mut self, cx: &mut MapCx, addr: Addr) -> Option<TypeId> {
        let tasty = self.tasty(cx.file);
        let decoder = Decoder::new(&tasty);
        if decoder.tag_at(addr) != crate::tasty::tags::PARAM {
            return None;
        }
        let name = self.intern_name(&tasty, decoder.name_at(addr)?);
        let owner = self
            .loaded_tables(cx)
            .classes
            .iter()
            .filter(|&(&start, _)| start < addr && decoder.tree_end(start) > addr)
            .max_by_key(|&(&start, _)| start)
            .map(|(_, &c)| c)?;
        self.complete_class(owner);
        let this = self.self_type_of(owner);
        let (sym, _) = self.find_member(this, name)?;
        let prefix = if self.syms.class(owner).kind == ClassKind::Object { self.types.class(owner, &[]) } else { self.types.mk(Type::This(owner)) };
        Some(self.types.mk(Type::Select(prefix, sym)))
    }

    fn path_singleton(&mut self, cx: &mut MapCx, t: &TType) -> Option<TypeId> {
        if let TType::ParamRef(..) = t {
            let path = self.map_type(cx, t);
            return self.types.is_path(path).then_some(path);
        }
        if let TType::LocalTerm(addr, None) = t {
            let param = cx.term_binders.iter().rev().flat_map(|(_, ps)| ps.iter()).find(|&&(a, _)| a == *addr && a != 0);
            if let Some(&(_, s)) = param {
                return Some(self.types.mk(Type::Term(s)));
            }
        }
        let tasty = self.tasty(cx.file);
        let (prefix, sym) = match t {
            // A local selected through a path (`this.subtype.typeInfo`, as an export forwarder
            // is typed) keeps the path as its prefix.
            TType::LocalTerm(addr, path) => {
                let Some(sym) = self.loaded_tables(cx).terms.get(addr).copied() else {
                    return self.class_param_path(cx, *addr);
                };
                let prefix = match path.as_deref() {
                    Some(this @ TType::This(_)) => match self.scope_of(cx, this) {
                        Some(Scope::Class(c)) if self.syms.class(c).kind != ClassKind::Object => Some(self.types.mk(Type::This(c))),
                        _ => None,
                    },
                    Some(p @ (TType::LocalTerm(..) | TType::TermRef(..))) => self.path_singleton(cx, p),
                    _ => None,
                };
                (prefix, sym)
            }
            TType::TermRef(prefix, n) => {
                let name = self.intern_name(&tasty, *n);
                if let Some(r) = self.std_package_object_term(cx, prefix, name) {
                    match r {
                        TermRef::Class(c) => return Some(self.class_ref(c)),
                        r => (None, r.sym()?)
                    }
                } else {
                match self.scope_of(cx, prefix) {
                    Some(Scope::Pkg(p)) => match self.pkg_term(p, name)? {
                        TermRef::Class(c) => return Some(self.class_ref(c)),
                        r => (None, r.sym()?),
                    },
                    Some(Scope::Class(c)) => {
                        self.complete_class(c);
                        // An inherited member, and one of a trait's self type (`_: FocusBase
                        // =>`), is selected on `this` too.
                        let s = match self.module_term(c, name) {
                            Some(r) => r.sym()?,
                            None => {
                                let mut classes = vec![c];
                                classes.extend(self.self_type_classes(c));
                                let mut found = None;
                                for k in classes {
                                    let kt = self.self_type_of(k);
                                    if let Some((s, _)) = self.find_member(kt, name) {
                                        found = Some(s);
                                        break;
                                    }
                                }
                                found?
                            }
                        };
                        let prefix = (self.syms.class(c).kind != ClassKind::Object).then(|| self.types.mk(Type::This(c)));
                        (prefix, s)
                    }
                    // A member of a structural type (`x.asInstanceOf[{ def ctx: C }].ctx`):
                    // no path, the member's type.
                    None if matches!(&**prefix, TType::Refined(..)) => {
                        let refined = self.map_type(cx, prefix);
                        let member = self.refined_term(refined, name)?;
                        return Some(self.refinement_sig(member).ret);
                    }
                    None => {
                        let p = self.path_singleton(cx, prefix)?;
                        let under = self.widen_path(p);
                        let (s, _) = self.find_member(under, name)?;
                        (Some(p), s)
                    }
                }
                }
            }
            _ => return None,
        };
        // A member of an object is a path of its own, as the typer makes it.
        let prefix = prefix.filter(|&p| !matches!(self.types.get(p), Type::Class(c, _) if self.syms.class(c).kind == ClassKind::Object));
        let info = self.syms.sym(sym);
        match info.kind {
            SymKind::Object(c) => Some(self.types.class(c, &[])),
            // An export forwarder of an object (`final def T: O.type`) is the object's path.
            SymKind::Def => {
                self.sig_of(sym);
                let sig = self.syms.sig(sym);
                let ret = sig.ret;
                match self.types.get(ret) {
                    Type::Class(c, _) if sig.tparams.is_empty() && sig.clauses.is_empty() && self.syms.class(c).kind == ClassKind::Object => Some(ret),
                    _ => None,
                }
            }
            SymKind::Var | SymKind::Overloaded(_) => None,
            _ if info.by_name => None,
            // A val standing for an object (`val Either = scala.util.Either` of `scala.package`)
            // has the object's type, as the object does.
            SymKind::Val => {
                let ret = self.sig_of(sym).ret;
                match self.types.get(ret) {
                    Type::Class(c, _) if self.syms.class(c).kind == ClassKind::Object => Some(ret),
                    _ => Some(match prefix {
                        Some(p) => self.types.mk(Type::Select(p, sym)),
                        None => self.types.mk(Type::Term(sym)),
                    }),
                }
            }
            _ => Some(match prefix {
                Some(p) => self.types.mk(Type::Select(p, sym)),
                None => self.types.mk(Type::Term(sym)),
            }),
        }
    }

    /// A wildcard argument read by the variance of the parameter it fills: its upper bound in a
    /// covariant position, its lower one in a contravariant, else a wildcard.
    pub(super) fn wildcard_arg(&mut self, cx: &mut MapCx, variance: i8, lo: &TType, hi: &TType) -> TypeId {
        let open_lo = self.is_open_bound(cx, lo, false);
        let open_hi = self.is_open_bound(cx, hi, true);
        match variance {
            1 => {
                if open_hi {
                    ANY
                } else {
                    self.map_type(cx, hi)
                }
            }
            -1 => {
                if open_lo {
                    NOTHING
                } else {
                    self.map_type(cx, lo)
                }
            }
            // `? >: A <: A`, which scalac writes for an inferred argument, is `A`.
            _ => {
                let l = if open_lo { NOTHING } else { self.map_type(cx, lo) };
                let h = if open_hi { ANY } else { self.map_type(cx, hi) };
                self.types.bounded_wild(l, h)
            }
        }
    }

    /// The variances of a constructor's parameters, an unknown one being invariant.
    pub(super) fn ctor_variances(&mut self, ctor: TypeId) -> Vec<i8> {
        match self.types.get(ctor) {
            Type::Ctor(c) => {
                self.complete_class_tparams(c);
                self.syms.class(c).tparams.iter().map(|&p| self.syms.tparam(p).variance).collect()
            }
            _ => Vec::new(),
        }
    }

    fn map_applied(&mut self, cx: &mut MapCx, tycon: &TType, args: &[TType], t: &TType) -> TypeId {
        let tasty = self.tasty(cx.file);
        if let TType::TypeRef(prefix, n) = tycon {
            if is_scala_pkg(&tasty, prefix) {
                let name = tasty.simple(*n).unwrap_or("");
                if let Some((context, arity)) = function_class(name) {
                    if arity + 1 == args.len() {
                        let ps: Vec<TypeId> = args[..arity].iter().map(|a| self.map_type(cx, a)).collect();
                        let r = self.map_type(cx, &args[arity]);
                        return if context { self.ctx_fun_type(&ps, r) } else { self.fun_type(&ps, r) };
                    }
                }
                if name == "<repeated>" && args.len() == 1 {
                    let elem = self.map_type(cx, &args[0]);
                    return match self.seq_class() {
                        Some(seq) => self.types.class(seq, &[elem]),
                        None => elem,
                    };
                }
            }
        }
        cx.applied_to = Some(args.len());
        let ctor = self.map_type_ctor(cx, tycon);
        cx.applied_to = None;
        let params: Vec<TParamId> = match self.types.get(ctor) {
            Type::Ctor(c) => {
                self.complete_class_tparams(c);
                self.syms.class(c).tparams.clone()
            }
            Type::Lambda(l, _) => self
                .types
                .items(l)
                .iter()
                .filter_map(|&p| match self.types.get(p) {
                    Type::Param(p) => Some(p),
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        };
        let mut resolved = Vec::with_capacity(args.len());
        for (i, a) in args.iter().enumerate() {
            let variance = params.get(i).map_or(0, |&p| self.syms.tparam(p).variance);
            let mapped = match a {
                TType::Bounds(lo, hi) => self.wildcard_arg(cx, variance, lo, hi),
                other => self.map_type_ctor(cx, other),
            };
            resolved.push(mapped);
        }
        if self.types.contains_error(ctor) {
            return ctor;
        }
        if let Type::Ctor(c) = self.types.get(ctor) {
            if self.class_arity(c) != resolved.len() {
                return self.blocked(cx, Shape::Unknown, t);
            }
        }
        let applied = self.types.apply_ctor(ctor, &resolved);
        if applied == ERROR {
            return self.blocked(cx, Shape::Unknown, t);
        }
        applied
    }
}

/// Whether the term parameters of a method type are a `using` clause: the `apply` scalac gives the
/// refinement of a polymorphic function type over a context function (`[A] => T[A] ?=> R`).
pub(super) fn is_using_clause(params: &[crate::tasty::tree::TParam]) -> bool {
    params.first().map_or(false, |p| p.flags.has(crate::tasty::tags::GIVEN))
}

/// The object scalac keeps a package's top-level definitions in: `package` for a package
/// object, `X$package` for the top-level definitions of the file `X.scala`.
/// The Java classes scala-library's `scala` package object aliases, with their package.
const SCALA_PACKAGE_JAVA_ALIASES: &[(&str, &str)] = &[
    ("Cloneable", "java.lang"),
    ("Serializable", "java.io"),
    ("Throwable", "java.lang"),
    ("Exception", "java.lang"),
    ("Error", "java.lang"),
    ("RuntimeException", "java.lang"),
    ("NullPointerException", "java.lang"),
    ("ClassCastException", "java.lang"),
    ("IndexOutOfBoundsException", "java.lang"),
    ("ArrayIndexOutOfBoundsException", "java.lang"),
    ("StringIndexOutOfBoundsException", "java.lang"),
    ("UnsupportedOperationException", "java.lang"),
    ("IllegalArgumentException", "java.lang"),
    ("NoSuchElementException", "java.util"),
    ("NumberFormatException", "java.lang"),
    ("AbstractMethodError", "java.lang"),
    ("InterruptedException", "java.lang"),
];

pub(super) fn is_package_object_name(stem: Option<&str>) -> bool {
    stem.map_or(false, |s| s == "package" || s.ends_with("$package"))
}
