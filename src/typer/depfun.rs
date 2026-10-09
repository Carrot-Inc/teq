//! Function types that name their parameters, `(c: Ctx) => c.T`: as in scalac, a `FunctionN`
//! (`ContextFunctionN` under `?=>`) refined by an `apply` whose signature holds the parameters,
//! `Function1[Ctx, R'] { def apply(c: Ctx): c.T }`. The refinement keeps the precise result, which
//! may name the parameters; the parent's result `R'` is the least supertype of it that names none
//! (scalac's `nonDependentResultApprox`), which is what the type is where a plain function type
//! is asked for (`as_function`, the backends' erasure).

use super::members::MemberInfo;
use super::{Frame, Worker};
use crate::ast::{ListRef, TyExpr, TyExprId};
use crate::intern::Name;
use crate::names;
use crate::source::{FileId, Span};
use crate::symbols::*;
use crate::types::*;
use std::sync::Arc;

/// A function type with named parameters, seen through its `apply`: the parameter types, the
/// parameters' symbols that the result names, and the result.
pub struct DepFun {
    pub ctx: bool,
    pub params: Vec<TypeId>,
    pub binders: Vec<SymId>,
    pub ret: TypeId,
}

/// A type approximated where it names a parameter: exact, or the range of types it may be at a
/// position whose variance does not pick a side (scalac's `ApproximatingTypeMap`).
#[derive(Clone, Copy)]
enum Approx {
    Exact(TypeId),
    Range(TypeId, TypeId),
}

impl Approx {
    fn lo(self) -> TypeId {
        match self {
            Approx::Exact(t) | Approx::Range(t, _) => t,
        }
    }

    fn hi(self) -> TypeId {
        match self {
            Approx::Exact(t) | Approx::Range(_, t) => t,
        }
    }
}

fn range(lo: TypeId, hi: TypeId, variance: i8) -> Approx {
    if variance > 0 {
        Approx::Exact(hi)
    } else if variance < 0 || lo == hi {
        Approx::Exact(lo)
    } else {
        Approx::Range(lo, hi)
    }
}

const MAX_APPROX_DEPTH: u32 = 32;

/// The head of an applied type the approximation rebuilds.
#[derive(Clone, Copy)]
enum Tycon {
    Class(ClassId),
    Param(TParamId),
    /// A type member, `Member` through its prefix.
    Member(TypeId),
}

impl<'a> Worker<'a> {
    /// `(c: Ctx) => c.T` written with its parameter names: each parameter is in scope for the
    /// types after it and for the result. A parameter type that names an earlier parameter is
    /// refused, as scalac refuses it.
    pub(super) fn resolve_named_fun(&mut self, params: ListRef, names_list: ListRef, ret: TyExprId, ctx: bool, span: Span) -> TypeId {
        let ast = self.cur_ast();
        let names: Vec<Name> = ast.name_lists[names_list.range()].to_vec();
        let param_tys: Vec<TyExprId> = ast.ty_list(params).to_vec();
        let file = self.env.file;
        self.env.frames.push(Frame::locals());
        let mut sig_params = Vec::with_capacity(names.len());
        for (&name, &p) in names.iter().zip(&param_tys) {
            let ty = match self.cur_ast().ty(p) {
                TyExpr::ByName(inner) => {
                    let t = self.resolve_type(inner);
                    self.by_name_type(t)
                }
                _ => self.resolve_type(p),
            };
            let pspan = self.cur_ast().ty_spans[p.idx()];
            let sym = self.syms.new_sym(name, SymKind::Param, 0, Owner::Local, file, None, pspan);
            {
                let mut s = self.syms.sym_mut(sym);
                s.sig = Some(Arc::new(MethodSig::value(ty)));
                s.state().set(Completion::Done);
            }
            if let Some(Frame::Locals { names, .. }) = self.env.frames.last_mut() {
                names.push((name, sym));
            }
            sig_params.push(ParamSig { name, ty, by_name: false, repeated: false, has_default: false, sym });
        }
        let r = self.resolve_type(ret);
        self.env.frames.pop();
        if sig_params.iter().any(|p| p.ty == ERROR) || r == ERROR {
            return ERROR;
        }
        let syms: Vec<SymId> = sig_params.iter().map(|p| p.sym).collect();
        // A parameter written as a wildcard is no prefix: scalac's approximation of the result
        // cannot select through its bounds.
        for (&p, param) in param_tys.iter().zip(&sig_params) {
            let bounds = match self.cur_ast().ty(p) {
                TyExpr::Wildcard => String::new(),
                TyExpr::BoundedWildcard(lo, hi) => {
                    let mut b = String::new();
                    let (lo, hi) = (self.resolve_type(lo), self.resolve_type(hi));
                    if lo != NOTHING {
                        b.push_str(" >: ");
                        b.push_str(&self.show(lo));
                    }
                    if hi != ANY {
                        b.push_str(" <: ");
                        b.push_str(&self.show(hi));
                    }
                    b
                }
                _ => continue,
            };
            if let Some(sel) = self.selection_through(r, param.sym) {
                let (name, shown) = (self.name_str(param.name), self.show(sel));
                self.error(span, format!("invalid new prefix {bounds} cannot replace ({name} : {bounds}) in type {shown}"));
                return ERROR;
            }
        }
        if sig_params.iter().enumerate().any(|(i, p)| self.types.names_term(p.ty, &syms[..i])) {
            let shown = self.show_method_type(&sig_params, r);
            self.error(span, format!("{} is an illegal function type because it has inter-parameter dependencies", shown));
            return ERROR;
        }
        self.named_fun_type(ctx, sig_params, r, span)
    }

    /// A member selected through the path of `param` in `t`, `f.A` for `f`.
    fn selection_through(&self, t: TypeId, param: SymId) -> Option<TypeId> {
        if !self.types.names_term(t, &[param]) {
            return None;
        }
        let list = |l: TList| self.types.items(l).iter().find_map(|&x| self.selection_through(x, param));
        match self.types.get(t) {
            Type::Member(p, _) | Type::Select(p, _) if matches!(self.types.get(p), Type::Term(s) if s == param) => Some(t),
            Type::Member(p, _) | Type::Select(p, _) => self.selection_through(p, param),
            Type::AppMember(m, args) => self.selection_through(m, param).or_else(|| list(args)),
            Type::Class(_, args) | Type::AppParam(_, args) | Type::Alias(_, args) => list(args),
            Type::Union(a, b) | Type::Inter(a, b) | Type::BoundedWild(a, b) => self.selection_through(a, param).or_else(|| self.selection_through(b, param)),
            Type::Refined(p, r) => self.selection_through(p, param).or_else(|| match self.types.refinement(r) {
                Refinement::Alias(_, x) | Refinement::Val(_, _, x) => self.selection_through(x, param),
                Refinement::Bounds(_, lo, hi) => self.selection_through(lo, param).or_else(|| self.selection_through(hi, param)),
                Refinement::Term(_, _, l) => list(l),
            }),
            _ => None,
        }
    }

    /// `(c: Ctx, x: c.T): c.T`, a method type as scalac shows it.
    fn show_method_type(&mut self, params: &[ParamSig], ret: TypeId) -> String {
        let mut out = String::from("(");
        for (i, p) in params.iter().enumerate() {
            if i > 0 {
                out.push_str(", ");
            }
            out.push_str(&self.name_str(p.name));
            out.push_str(": ");
            out.push_str(&self.show(p.ty));
        }
        out.push_str("): ");
        out.push_str(&self.show(ret));
        out
    }

    /// The function type with the parameters `params` and the result `ret`, which may name
    /// them: the refinement of the function class by an `apply` of that signature.
    pub fn named_fun_type(&mut self, ctx: bool, params: Vec<ParamSig>, ret: TypeId, span: Span) -> TypeId {
        let file = self.env.file;
        self.named_fun_type_in(file, ctx, params, ret, span)
    }

    /// `named_fun_type` with the `apply` a symbol of the file `file` (a library's, read).
    pub fn named_fun_type_in(&mut self, file: FileId, ctx: bool, params: Vec<ParamSig>, ret: TypeId, span: Span) -> TypeId {
        let syms: Vec<SymId> = params.iter().map(|p| p.sym).collect();
        let ptys: Vec<TypeId> = params.iter().map(|p| p.ty).collect();
        let approx = self.non_dependent_approx(ret, &syms);
        let parent = if ctx { self.ctx_fun_type(&ptys, approx) } else { self.fun_type(&ptys, approx) };
        let apply = self.syms.new_sym(names::APPLY, SymKind::Def, crate::ast::mods::ABSTRACT, Owner::Local, file, None, span);
        let sig = MethodSig { tparams: Vec::new(), clauses: vec![ClauseSig { params, is_using: ctx, is_implicit: false }], ret };
        let l = self.sig_types(&sig);
        {
            let mut s = self.syms.sym_mut(apply);
            s.sig = Some(Arc::new(sig));
            s.state().set(Completion::Done);
        }
        let r = self.types.refine(Refinement::Term(names::APPLY, apply, l));
        self.types.mk(Type::Refined(parent, r))
    }

    /// The type of a lambda or an eta-expansion over the parameters `syms` of the types `ptys`
    /// with the result `ret`: a function type, refined by its `apply` where the result names a
    /// parameter, as scalac's closure takes its method's dependent result.
    pub fn closure_type(&mut self, ctx: bool, syms: &[SymId], ptys: &[TypeId], ret: TypeId, span: Span) -> TypeId {
        if !self.types.names_term(ret, syms) {
            return if ctx { self.ctx_fun_type(ptys, ret) } else { self.fun_type(ptys, ret) };
        }
        let names: Vec<Name> = syms.iter().map(|&s| self.syms.sym(s).name).collect();
        self.closure_type_named(ctx, &names, syms, ptys, ret, span)
    }

    /// `closure_type` with the parameters shown by `names`, an eta-expansion's by its method's.
    pub fn closure_type_named(&mut self, ctx: bool, names: &[Name], syms: &[SymId], ptys: &[TypeId], ret: TypeId, span: Span) -> TypeId {
        if syms.len() == ptys.len() && names.len() == syms.len() && self.types.names_term(ret, syms) {
            let mut ret = ret;
            let mut params: Vec<ParamSig> = syms
                .iter()
                .zip(ptys)
                .map(|(&sym, &ty)| ParamSig { name: self.syms.sym(sym).name, ty, by_name: false, repeated: false, has_default: false, sym })
                .collect();
            // Shown by other names, the parameters are symbols of those names, for the result's
            // paths to read as the parameters do.
            if params.iter().zip(names).any(|(p, &n)| p.name != n) {
                let file = self.env.file;
                let mut renamed = Vec::with_capacity(params.len());
                for (p, &name) in params.iter_mut().zip(names) {
                    let sym = self.syms.new_sym(name, SymKind::Param, 0, Owner::Local, file, None, span);
                    {
                        let mut s = self.syms.sym_mut(sym);
                        s.sig = Some(Arc::new(MethodSig::value(p.ty)));
                        s.state().set(Completion::Done);
                    }
                    renamed.push((p.sym, self.types.mk(Type::Term(sym))));
                    p.name = name;
                    p.sym = sym;
                }
                ret = self.subst_paths(ret, &renamed);
            }
            return self.named_fun_type(ctx, params, ret, span);
        }
        if ctx { self.ctx_fun_type(ptys, ret) } else { self.fun_type(ptys, ret) }
    }

    /// The function type `fun` of a polymorphic function type with its parameters named, as
    /// dotty's `Desugar.makePolyFunctionType` names an unnamed one's `x$1`, `x$2`, .. (a
    /// context function's a using clause); one that names them already, or no function type,
    /// as it is.
    pub(super) fn poly_function_apply(&mut self, fun: TypeId, span: Span) -> TypeId {
        if fun == ERROR || self.named_function(fun).is_some() {
            return fun;
        }
        let (ctx, (ptys, ret)) = match self.as_function(fun) {
            Some(parts) => (false, parts),
            None => match self.as_context_function(fun) {
                Some(parts) => (true, parts),
                None => return fun,
            },
        };
        let file = self.env.file;
        let params: Vec<ParamSig> = ptys
            .iter()
            .enumerate()
            .map(|(i, &ty)| {
                let name = self.interner.intern(&format!("x${}", i + 1));
                let sym = self.syms.new_sym(name, SymKind::Param, 0, Owner::Local, file, None, span);
                {
                    let mut s = self.syms.sym_mut(sym);
                    s.sig = Some(Arc::new(MethodSig::value(ty)));
                    s.state().set(Completion::Done);
                }
                ParamSig { name, ty, by_name: false, repeated: false, has_default: false, sym }
            })
            .collect();
        self.named_fun_type(ctx, params, ret, span)
    }

    /// The function type `fun` of a polymorphic function literal of the parameters `syms` with
    /// them named: its `apply`'s parameters are the literal's.
    pub(super) fn literal_apply(&mut self, fun: TypeId, syms: &[SymId], span: Span) -> TypeId {
        if fun == ERROR || self.named_function(fun).is_some() {
            return fun;
        }
        let (ctx, (ptys, ret)) = match self.as_function(fun) {
            Some(parts) => (false, parts),
            None => match self.as_context_function(fun) {
                Some(parts) => (true, parts),
                None => return fun,
            },
        };
        if ptys.len() != syms.len() {
            return fun;
        }
        let params: Vec<ParamSig> = syms
            .iter()
            .zip(&ptys)
            .map(|(&sym, &ty)| ParamSig { name: self.syms.sym(sym).name, ty, by_name: false, repeated: false, has_default: false, sym })
            .collect();
        self.named_fun_type(ctx, params, ret, span)
    }

    /// The signature of the `apply` a function type with named parameters is refined by, as
    /// the refinement has it.
    pub(super) fn named_apply_sig(&mut self, t: TypeId) -> Option<Arc<MethodSig>> {
        let t = self.deref_alias(t);
        self.named_fun_parent(t)?;
        let Type::Refined(_, r) = self.types.get(t) else { return None };
        let refinement = self.types.refinement(r);
        Some(self.refinement_sig(refinement))
    }

    /// The function class a refinement by `apply` refines, for the views that read a function
    /// type with named parameters as the plain one (scalac's `dropDependentRefinement`).
    #[inline]
    pub(super) fn named_fun_parent(&self, t: TypeId) -> Option<TypeId> {
        let Type::Refined(parent, r) = self.types.get(t) else { return None };
        match self.types.refinement(r) {
            Refinement::Term(n, ..) if n == names::APPLY => Some(parent),
            _ => None,
        }
    }

    /// The function type with named parameters a value of type `t` is: `t` itself, what a path
    /// or a type parameter's upper bound stands for, a part of an intersection, or the parent
    /// of other refinements (`((c: C) => c.T) { type Tag = Int }`).
    pub(super) fn named_function_within(&mut self, t: TypeId) -> Option<TypeId> {
        let mut todo = vec![t];
        for _ in 0..MAX_APPROX_DEPTH {
            let t = todo.pop()?;
            let t = self.deref_alias(t);
            if self.named_fun_parent(t).is_some() {
                return Some(t);
            }
            match self.types.get(t) {
                Type::This(_) | Type::Term(_) | Type::Select(..) => todo.push(self.widen_path(t)),
                Type::Param(p) => todo.push(self.syms.tparam(p).upper),
                Type::Inter(a, b) => todo.extend([b, a]),
                Type::Refined(p, _) => todo.push(p),
                _ => {}
            }
        }
        None
    }

    /// A function type with named parameters, `(c: Ctx) => c.T` or `(c: Ctx) ?=> List[c.T]`,
    /// read through its `apply`.
    pub fn named_function(&mut self, t: TypeId) -> Option<DepFun> {
        let t = self.deref_alias(t);
        let parent = self.named_fun_parent(t)?;
        let Type::Refined(_, r) = self.types.get(t) else { return None };
        let parent = self.deref_alias(parent);
        let Type::Class(c, args) = self.types.get(parent) else { return None };
        let ctx = if self.is_function_class(c) {
            false
        } else if self.is_context_function_class(c) {
            true
        } else {
            return None;
        };
        let refinement = self.types.refinement(r);
        let sig = self.refinement_sig(refinement);
        let [clause] = sig.clauses.as_slice() else { return None };
        let items = self.types.items(args);
        // An `apply` over other parameter types is an overload beside the class's, no view of it.
        if !sig.tparams.is_empty() || clause.params.len() + 1 != items.len() || clause.params.iter().zip(items).any(|(p, &a)| p.ty != a) {
            return None;
        }
        let params = items[..clause.params.len()].to_vec();
        let binders = clause.params.iter().map(|p| p.sym).collect();
        Some(DepFun { ctx, params, binders, ret: sig.ret })
    }

    /// The result of a function type with named parameters for arguments whose paths are
    /// `paths`, one per parameter.
    pub fn instantiate_named(&mut self, f: &DepFun, paths: &[TypeId]) -> TypeId {
        let terms: Vec<(SymId, TypeId)> = f.binders.iter().copied().zip(paths.iter().copied()).collect();
        self.subst_paths(f.ret, &terms)
    }

    /// The result a context function of the type `expected` has inside its literal, whose
    /// parameters are `syms`: `ret`, or the precise result naming them.
    pub(super) fn context_result(&mut self, expected: TypeId, syms: &[SymId], ret: TypeId) -> TypeId {
        match self.named_function(expected) {
            Some(f) if f.ctx && f.binders.len() == syms.len() => {
                let paths: Vec<TypeId> = syms.iter().map(|&s| self.types.mk(Type::Term(s))).collect();
                self.instantiate_named(&f, &paths)
            }
            _ => ret,
        }
    }

    /// The parameters a value of the function type `fun` is called with: the parameter types
    /// `ptys`, named as the type names them (`c` of `(c: Ctx) => c.T`) so that an argument can
    /// be passed by name.
    pub(super) fn closure_clause(&mut self, fun: TypeId, ptys: &[TypeId]) -> ClauseSig {
        let names: Vec<Name> = match self.named_function(fun) {
            Some(f) if f.binders.len() == ptys.len() => f.binders.iter().map(|&b| self.syms.sym(b).name).collect(),
            _ => vec![names::EMPTY; ptys.len()],
        };
        let params = ptys
            .iter()
            .zip(names)
            .map(|(&ty, name)| ParamSig { name, ty, by_name: false, repeated: false, has_default: false, sym: SymId(0) })
            .collect();
        ClauseSig { params, is_using: false, is_implicit: false }
    }

    /// The result of a value of the function type `fun` with named parameters applied to
    /// `args`, whose parameter types are `ptys`: the refinement's result, which may be more
    /// precise than the parent's, with the arguments' paths. An argument bound to a temporary
    /// to keep the order it was written in stands for itself.
    pub(super) fn applied_named_result(&mut self, fun: TypeId, args: &[crate::tir::TExprId], ptys: &[TypeId]) -> Option<TypeId> {
        let f = self.named_function(fun)?;
        if f.binders.len() != args.len() {
            return None;
        }
        let paths: Vec<TypeId> = args
            .iter()
            .zip(ptys)
            .map(|(&a, &p)| {
                let a = self.hoisted_value(a).unwrap_or(a);
                let typed = self.arg_types_seen.iter().rev().find(|&&(e, _)| e == a).map_or(p, |&(_, t)| t);
                self.arg_path(a, typed)
            })
            .collect();
        Some(self.instantiate_named(&f, &paths))
    }

    /// The expression a temporary of the call under way was bound to.
    fn hoisted_value(&self, a: crate::tir::TExprId) -> Option<crate::tir::TExprId> {
        let crate::tir::TExpr::Local(s) = self.prog.expr(a) else { return None };
        self.hoisted.iter().rev().find_map(|st| match *st {
            crate::tir::TStmt::Val(t, init) if t == s => Some(init),
            _ => None,
        })
    }

    /// The least supertype of `t` that names none of `params` (scalac's
    /// `nonDependentResultApprox`): a parameter's path is anything between `Nothing` and its
    /// type, so `c.T` is `Ctx#T` where the result is read, `Nothing` where it is written
    /// (`List[c.T] => Int` is a `List[Nothing] => Int`), and some type within its bounds in an
    /// invariant position (`Box[c.T]` is a `Box[?]`).
    pub fn non_dependent_approx(&mut self, t: TypeId, params: &[SymId]) -> TypeId {
        if !self.types.names_term(t, params) {
            return t;
        }
        self.approx(t, params, 1, 0).hi()
    }

    fn approx(&mut self, t: TypeId, params: &[SymId], v: i8, depth: u32) -> Approx {
        if !self.types.names_term(t, params) {
            return Approx::Exact(t);
        }
        if depth > MAX_APPROX_DEPTH {
            return range(NOTHING, ANY, v);
        }
        let d = depth + 1;
        let t = self.deref(t);
        match self.types.get(t) {
            Type::Term(s) if params.contains(&s) => {
                let under = self.path_underlying(t);
                let hi = self.approx(under, params, 1, d).hi();
                range(NOTHING, hi, v)
            }
            Type::Select(p, s) => match self.approx(p, params, v.max(0), d) {
                Approx::Exact(np) if self.types.is_path(np) => Approx::Exact(self.types.mk(Type::Select(np, s))),
                pre => {
                    // The val's type seen through the upper bound of the prefix.
                    let hi = self.member_val_type(pre.hi(), s);
                    range(NOTHING, hi, v)
                }
            },
            Type::Member(p, name) => match self.approx(p, params, v.max(0), d) {
                Approx::Exact(np) => {
                    let np = if self.types.is_path(np) { np } else { self.widen_path(np) };
                    match self.member_type(np, name) {
                        Some(m) if !self.types.names_term(m, params) => Approx::Exact(m),
                        _ => range(NOTHING, ANY, v),
                    }
                }
                Approx::Range(_, hi) => {
                    let hi = self.widen_path(hi);
                    match self.type_member(hi, name) {
                        Some(MemberInfo::Alias(a)) => self.approx(a, params, v, d),
                        Some(MemberInfo::Bounds(lo, up)) => {
                            let lo = self.approx(lo, params, -v, d).lo();
                            let up = self.approx(up, params, v, d).hi();
                            range(lo, up, v)
                        }
                        None => range(NOTHING, ANY, v),
                    }
                }
            },
            Type::AppMember(m, args) => self.approx_applied_member(m, args, params, v, d),
            Type::AppParam(p, args) => {
                let items: Vec<TypeId> = self.types.items(args).to_vec();
                let variances = self.syms.tparam(p).hk_variances.clone();
                self.approx_applied(Tycon::Param(p), &items, &variances, params, v, d)
            }
            Type::Class(c, args) => self.approx_class(c, args, params, v, d),
            Type::Union(a, b) | Type::Inter(a, b) => {
                let union = matches!(self.types.get(t), Type::Union(..));
                let (x, y) = (self.approx(a, params, v, d), self.approx(b, params, v, d));
                let join = |w: &mut Self, p: TypeId, q: TypeId| if union { w.types.union(p, q) } else { w.types.inter(p, q) };
                match (x, y) {
                    (Approx::Exact(p), Approx::Exact(q)) => Approx::Exact(join(self, p, q)),
                    _ => {
                        let lo = join(self, x.lo(), y.lo());
                        let hi = join(self, x.hi(), y.hi());
                        range(lo, hi, v)
                    }
                }
            }
            Type::Refined(p, r) => self.approx_refined(t, p, r, params, v, d),
            // Bounds that name a parameter leave no exact type, as their lambda's parameter
            // infos would need approximating (`TypeMap.mapOverLambda`).
            Type::Poly(ps, fun) if !self.types.poly_bounds(ps).iter().any(|&b| self.types.names_term(b, params)) => match self.approx(fun, params, v, d) {
                Approx::Exact(f) => Approx::Exact(self.types.mk(Type::Poly(ps, f))),
                _ => range(NOTHING, ANY, v),
            },
            Type::Alias(..) => {
                let e = self.deref_alias(t);
                if e == t {
                    range(NOTHING, ANY, v)
                } else {
                    self.approx(e, params, v, d)
                }
            }
            _ => range(NOTHING, ANY, v),
        }
    }

    /// `c.F[A]`, an abstract type member applied: through a prefix that names a parameter the
    /// member's bounds applied to the arguments, as an unapplied member takes its bounds, and
    /// through another prefix the arguments approximated at the variances of the member's.
    fn approx_applied_member(&mut self, m: TypeId, args: TList, params: &[SymId], v: i8, d: u32) -> Approx {
        let items: Vec<TypeId> = self.types.items(args).to_vec();
        let Type::Member(p, name) = self.types.get(m) else { return range(NOTHING, ANY, v) };
        let applied = |w: &mut Self, t: TypeId| match w.types.get(t) {
            Type::Lambda(..) | Type::Ctor(_) | Type::Param(_) | Type::Member(..) => w.types.apply_ctor(t, &items),
            _ => t,
        };
        match self.approx(p, params, v.max(0), d) {
            Approx::Exact(np) => {
                let np = if self.types.is_path(np) { np } else { self.widen_path(np) };
                let tycon = if np == p { Some(m) } else { self.member_type(np, name) };
                let Some(tycon) = tycon else { return range(NOTHING, ANY, v) };
                if !matches!(self.types.get(tycon), Type::Member(..)) {
                    let t = applied(self, tycon);
                    return self.approx(t, params, v, d);
                }
                let lambda_params = match self.type_member(np, name) {
                    Some(MemberInfo::Bounds(lo, up)) => [up, lo].into_iter().find_map(|b| match self.types.get(b) {
                        Type::Lambda(ps, _) => Some(ps),
                        _ => None,
                    }),
                    _ => None,
                };
                let variances: Vec<i8> = match lambda_params {
                    Some(ps) => {
                        let ps: Vec<TypeId> = self.types.items(ps).to_vec();
                        ps.iter()
                            .map(|&q| match self.types.get(q) {
                                Type::Param(id) => self.syms.tparam(id).variance,
                                _ => 0,
                            })
                            .collect()
                    }
                    None => Vec::new(),
                };
                self.approx_applied(Tycon::Member(tycon), &items, &variances, params, v, d)
            }
            Approx::Range(_, hi) => {
                let hi = self.widen_path(hi);
                match self.type_member(hi, name) {
                    Some(MemberInfo::Alias(a)) => {
                        let t = applied(self, a);
                        self.approx(t, params, v, d)
                    }
                    Some(MemberInfo::Bounds(lo, up)) => {
                        let (lo, up) = (applied(self, lo), applied(self, up));
                        let lo = self.approx(lo, params, -v, d).lo();
                        let up = self.approx(up, params, v, d).hi();
                        range(lo, up, v)
                    }
                    None => range(NOTHING, ANY, v),
                }
            }
        }
    }

    /// The widened type of the val `s` as a member of `prefix`.
    fn member_val_type(&mut self, prefix: TypeId, s: SymId) -> TypeId {
        let prefix = self.widen_path(prefix);
        let name = self.syms.sym(s).name;
        if let Some(refined) = self.refined_term(prefix, name) {
            return self.refinement_sig(refined).ret;
        }
        match self.find_member(prefix, name) {
            Some((found, owner_ty)) => {
                let ret = self.sig_of(found).ret;
                let subst = self.owner_subst(owner_ty);
                let ty = self.types.subst(ret, &subst);
                self.widen_path(ty)
            }
            None => ANY,
        }
    }

    /// `C[..]` with its arguments approximated at the variance of their parameters.
    fn approx_class(&mut self, c: ClassId, args: TList, params: &[SymId], v: i8, d: u32) -> Approx {
        self.complete_class(c);
        let tparams = self.syms.class(c).tparams.clone();
        let items: Vec<TypeId> = self.types.items(args).to_vec();
        let variances: Vec<i8> = tparams.iter().map(|&p| self.syms.tparam(p).variance).collect();
        self.approx_applied(Tycon::Class(c), &items, &variances, params, v, d)
    }

    /// A type constructor applied to `items` with the arguments approximated at `variances`, a
    /// missing one invariant: a range that an invariant parameter of a class cannot take is a
    /// wildcard where the type is read; one of a type parameter or a type member leaves nothing
    /// but `Any`, as scalac approximates an unreducible wildcard application.
    fn approx_applied(&mut self, tycon: Tycon, items: &[TypeId], variances: &[i8], params: &[SymId], v: i8, d: u32) -> Approx {
        let variance = |i: usize| variances.get(i).copied().unwrap_or(0);
        let mut approxed = Vec::with_capacity(items.len());
        for (i, &a) in items.iter().enumerate() {
            let arg = match self.types.get(a) {
                Type::BoundedWild(lo, hi) if self.types.names_term(a, params) => {
                    let lo = self.approx(lo, params, -1, d).lo();
                    let hi = self.approx(hi, params, 1, d).hi();
                    Approx::Exact(self.types.bounded_wild(lo, hi))
                }
                _ => self.approx(a, params, v * variance(i), d),
            };
            approxed.push(arg);
        }
        if approxed.iter().all(|a| matches!(a, Approx::Exact(_))) {
            let exact: Vec<TypeId> = approxed.iter().map(|a| a.lo()).collect();
            return Approx::Exact(self.apply_tycon(tycon, &exact));
        }
        let as_bounds: Vec<TypeId> = approxed
            .iter()
            .map(|a| match *a {
                Approx::Exact(t) => t,
                Approx::Range(lo, hi) => self.types.bounded_wild(lo, hi),
            })
            .collect();
        let class = matches!(tycon, Tycon::Class(_));
        if v > 0 && class {
            return Approx::Exact(self.apply_tycon(tycon, &as_bounds));
        }
        let mut los = Vec::with_capacity(items.len());
        let mut his = Vec::with_capacity(items.len());
        let mut distributed = true;
        for (i, a) in approxed.iter().enumerate() {
            match *a {
                Approx::Exact(t) => {
                    los.push(t);
                    his.push(t);
                }
                Approx::Range(lo, hi) if variance(i) > 0 => {
                    los.push(lo);
                    his.push(hi);
                }
                Approx::Range(lo, hi) if variance(i) < 0 => {
                    los.push(hi);
                    his.push(lo);
                }
                Approx::Range(..) => distributed = false,
            }
        }
        if distributed {
            let lo = self.apply_tycon(tycon, &los);
            let hi = self.apply_tycon(tycon, &his);
            range(lo, hi, v)
        } else if class {
            let hi = self.apply_tycon(tycon, &as_bounds);
            range(NOTHING, hi, v)
        } else {
            range(NOTHING, ANY, v)
        }
    }

    fn apply_tycon(&mut self, tycon: Tycon, args: &[TypeId]) -> TypeId {
        match tycon {
            Tycon::Class(c) => self.types.class(c, args),
            Tycon::Param(p) => {
                let l = self.types.list(args);
                self.types.mk(Type::AppParam(p, l))
            }
            Tycon::Member(m) => self.types.apply_ctor(m, args),
        }
    }

    /// `P { type A = x.T }` and the other refinements with their member approximated: an alias
    /// becomes bounds where the type is read, and nothing but `Nothing` fits below it.
    fn approx_refined(&mut self, t: TypeId, p: TypeId, r: RefineId, params: &[SymId], v: i8, d: u32) -> Approx {
        let parent = self.approx(p, params, v, d);
        let refinement = self.types.refinement(r);
        if let (Approx::Exact(np), Some(exact)) = (parent, self.approx_refinement(refinement, params, 0, d)) {
            return Approx::Exact(if np == p && exact == refinement { t } else { self.refined_by(np, exact) });
        }
        let upper = match self.approx_refinement(refinement, params, 1, d) {
            Some(m) => self.refined_by(parent.hi(), m),
            None => parent.hi(),
        };
        let lower = match self.approx_refinement(refinement, params, -1, d) {
            Some(m) => self.refined_by(parent.lo(), m),
            None => NOTHING,
        };
        range(lower, upper, v)
    }

    fn refined_by(&mut self, parent: TypeId, m: Refinement) -> TypeId {
        if let Refinement::Bounds(_, NOTHING, ANY) = m {
            return parent;
        }
        let r = self.types.refine(m);
        self.types.mk(Type::Refined(parent, r))
    }

    /// A refinement's member approximated at the variance `w` of the refinement: the looser
    /// member for 1, the tighter for -1, the member itself for 0, or None where there is none.
    fn approx_refinement(&mut self, m: Refinement, params: &[SymId], w: i8, d: u32) -> Option<Refinement> {
        let exact = |a: Approx| match a {
            Approx::Exact(t) => Some(t),
            Approx::Range(..) => None,
        };
        Some(match m {
            Refinement::Alias(n, x) => match self.approx(x, params, 0, d) {
                Approx::Exact(x) => Refinement::Alias(n, x),
                Approx::Range(lo, hi) if w > 0 => Refinement::Bounds(n, lo, hi),
                Approx::Range(..) => return None,
            },
            Refinement::Bounds(n, lo, hi) => {
                let lo = exact(self.approx(lo, params, -w, d))?;
                let hi = exact(self.approx(hi, params, w, d))?;
                Refinement::Bounds(n, lo, hi)
            }
            Refinement::Val(n, s, x) => Refinement::Val(n, s, exact(self.approx(x, params, w, d))?),
            Refinement::Term(n, s, l) => {
                let items: Vec<TypeId> = self.types.items(l).to_vec();
                let last = items.len().saturating_sub(1);
                let mut out = Vec::with_capacity(items.len());
                for (i, &x) in items.iter().enumerate() {
                    out.push(exact(self.approx(x, params, if i == last { w } else { -w }, d))?);
                }
                Refinement::Term(n, s, self.types.list(&out))
            }
        })
    }

    /// scalac's implementation restriction: a lambda expected to be a function type with named
    /// parameters whose result is a context function type (dotty's `decomposeProtoFunction`).
    /// A polymorphic function literal's own function is its `apply`, which dotty types by
    /// `typedPolyFunctionValue` and never asks this of.
    pub fn check_curried_dependent(&mut self, expected: TypeId, span: Span) {
        if self.poly_literal_fun == Some(expected) {
            return;
        }
        let Some(f) = self.named_function(expected) else { return };
        if self.as_context_function(f.ret).is_some() {
            let shown = self.show(expected);
            self.error(span, format!("Implementation restriction: Expected result type {}\nis a curried dependent context function type. Such types are not yet supported.", shown));
        }
    }
}
