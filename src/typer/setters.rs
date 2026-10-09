//! The setter of an abstract var. A `var x: T` that a class declares without a value stands for
//! two abstract members, the getter `x` and the setter `x_=(x$1: T): Unit`, which a subclass
//! implements with a var or with a `def x` and a `def x_=`. The namer makes the setter beside
//! the var (`mods::SETTER`, without a definition); a pickle declares it, and the loader marks it
//! so. An assignment through such a var calls the setter, and a read calls the getter, unless
//! the var belongs to a JavaScript type, whose vars are properties.

use super::*;
use crate::ast::mods;
use crate::intern::Interner;
use crate::symbols::Symbols;
use crate::tir::TExpr;
use std::sync::Arc;

impl<'a> Worker<'a> {
    /// The signature of the setter the namer made for an abstract var: the var's type taken by
    /// a parameter of its own, named as scalac names it.
    pub(super) fn abstract_setter_sig(&mut self, setter: SymId) -> MethodSig {
        let (file, span) = {
            let s = self.syms.sym(setter);
            (s.file, s.span)
        };
        let ty = match var_of_setter(&self.syms, self.interner, setter) {
            Some(var) => self.sig_of(var).ret,
            None => ERROR,
        };
        let name = self.interner.intern("x$1");
        let param = self.syms.new_sym(name, SymKind::Param, 0, Owner::Local, file, None, span);
        {
            let mut p = self.syms.sym_mut(param);
            p.sig = Some(Arc::new(MethodSig::value(ty)));
            p.state().set(crate::symbols::Completion::Done);
        }
        let params = vec![ParamSig { name, ty, by_name: false, repeated: false, has_default: false, sym: param }];
        MethodSig { tparams: Vec::new(), clauses: vec![ClauseSig { params, is_using: false, is_implicit: false }], ret: self.b.t_unit }
    }

    /// The setter through which the member `s` is assigned: an abstract var's, outside a
    /// JavaScript type.
    pub(super) fn setter_route(&mut self, s: SymId) -> Option<SymId> {
        let info = self.syms.sym(s);
        if info.kind != SymKind::Var || !matches!(info.owner, Owner::Class(_)) || self.syms.js_member(s) || !self.is_abstract_member(s) {
            return None;
        }
        let setter = setter_of_var(&self.syms, self.interner, s)?;
        self.sig_of(setter);
        Some(setter)
    }

    /// `this` and its type, where the typer read an unqualified name `y` as the method `this.y`:
    /// the receiver of the setter `y_=` that `y = v` and `y op= v` call.
    pub(super) fn this_receiver(&mut self, tl: TExprId) -> Option<(TExprId, TypeId)> {
        let TExpr::CallMethod(recv, getter, args) = self.prog.expr(tl) else { return None };
        if !matches!(self.prog.expr(recv), TExpr::This) || args.len != 0 || self.syms.sym(getter).kind != SymKind::Def {
            return None;
        }
        let c = self.this_class()?;
        Some((recv, self.syms.this_type(c)))
    }

    /// `y = v` for a method `y` of `this` with a setter `y_=`: the setter's call.
    pub(super) fn unqualified_setter_call(&mut self, tl: TExprId, name: Name, rhs: crate::ast::ExprId, span: Span) -> Option<(TExprId, TypeId)> {
        let (recv, ty) = self.this_receiver(tl)?;
        let setter = self.interner.intern(&format!("{}_=", self.name_ref(name)));
        self.find_member(ty, setter)?;
        let lists = vec![ArgList { args: vec![ArgSrc::Ast(rhs)], using: false, span }];
        let (call, _) = self.apply_member(recv, ty, setter, None, lists, span, None);
        Some((call, self.b.t_unit))
    }

    /// `target = value`: the assignment, or the setter's call for an abstract var.
    pub(crate) fn assignment(&mut self, target: TExprId, value: TExprId) -> TExprId {
        if let TExpr::Field(recv, s) = self.prog.expr(target) {
            if let Some(setter) = self.setter_route(s) {
                let args = self.prog.list(&[value]);
                let call = self.prog.add(TExpr::CallMethod(recv, setter, args));
                self.prog.set_type(call, self.b.t_unit);
                return call;
            }
        }
        self.prog.add(TExpr::Assign(target, value))
    }
}

/// The var whose setter `setter` is: the member of its class named without the `_=`, beside
/// the methods of that name it may be overloaded with.
pub fn var_of_setter(syms: &Symbols, interner: &Interner, setter: SymId) -> Option<SymId> {
    let info = syms.sym(setter);
    let Owner::Class(c) = info.owner else { return None };
    let name = interner.lookup(interner.get(info.name).strip_suffix("_=")?)?;
    own_var(syms, c, name)
}

/// The var `name` that the class `c` itself declares.
pub fn own_var(syms: &Symbols, c: ClassId, name: Name) -> Option<SymId> {
    let entry = syms.class(c).members.get(&name).copied()?;
    let single = [entry];
    let candidates = syms.alternatives(entry).unwrap_or(&single);
    candidates.iter().copied().find(|&v| syms.sym(v).kind == SymKind::Var && syms.sym(v).owner == Owner::Class(c))
}

/// The setter of the abstract var `var`, which its class declares beside it.
pub fn setter_of_var(syms: &Symbols, interner: &Interner, var: SymId) -> Option<SymId> {
    let info = syms.sym(var);
    let Owner::Class(c) = info.owner else { return None };
    let name = interner.lookup(&format!("{}_=", interner.get(info.name)))?;
    let entry = syms.class(c).members.get(&name).copied()?;
    let single = [entry];
    let candidates = syms.alternatives(entry).unwrap_or(&single);
    candidates.iter().copied().find(|&s| syms.sym(s).owner == Owner::Class(c) && syms.sym(s).kind == SymKind::Def && syms.sym(s).mods & mods::SETTER != 0)
}

/// Whether `s` is an abstract var, whose setter is declared beside it.
pub fn is_abstract_var(syms: &Symbols, interner: &Interner, s: SymId) -> bool {
    syms.sym(s).kind == SymKind::Var && setter_of_var(syms, interner, s).is_some()
}

/// The abstract var whose setter `s` is: the var an assignment through which calls `s`.
pub fn abstract_var_of_setter(syms: &Symbols, interner: &Interner, s: SymId) -> Option<SymId> {
    if syms.sym(s).mods & mods::SETTER == 0 {
        return None;
    }
    var_of_setter(syms, interner, s).filter(|&v| setter_of_var(syms, interner, v) == Some(s))
}
