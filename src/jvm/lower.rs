//! The platform lowering: an expression of the typed IR as nodes of the lowered IR
//! (`crate::lir`). A lowering decides and builds nodes; it writes nothing, and the code and the
//! constant pool refuse a write while one runs (`Gen::lowering`), since the order of what is
//! written is the output's bytes.

use super::gen::{CallRoute, FieldRoute, Fit, Gen, Invoke, MRef, Site, ThisValue};
use super::names::*;
use crate::ast::ListRef;
use crate::lir::{Kind, LirId, List};
use crate::tir::*;
use crate::types::*;
use std::rc::Rc;

impl<'a> Gen<'a> {
    /// Runs a lowering with the writers refusing.
    pub(super) fn lowering<T>(&mut self, f: impl FnOnce(&mut Self) -> T) -> T {
        let was = self.code.frozen;
        self.code.frozen = true;
        self.cw.cp.frozen = true;
        let out = f(self);
        self.code.frozen = was;
        self.cw.cp.frozen = was;
        out
    }

    /// `e` brought to the type `want`.
    pub(super) fn lower_expr(&mut self, e: TExprId, want: &JType) -> LirId {
        let id = self.lower_marked(e, want);
        let node = &mut self.lir.nodes[id.0 as usize];
        // A node not lowered takes its position as it is generated.
        if !matches!(node.kind, Kind::Legacy(_)) {
            node.of = Some(e);
        }
        id
    }

    fn lower_legacy(&mut self, e: TExprId, want: &JType) -> LirId {
        self.lir.push(Kind::Legacy(e), want.clone())
    }

    fn lower_marked(&mut self, e: TExprId, want: &JType) -> LirId {
        let prog = self.cx.input.prog;
        if want.is_ref() {
            if let Some(wider) = self.widened_branches(e) {
                let inner = self.lower_expr(e, &wider);
                return self.lower_adapt(inner, &wider, want);
            }
        }
        match prog.expr(e) {
            TExpr::If(..) | TExpr::Match(..) | TExpr::Block(..) | TExpr::Throw(..) => self.lower_legacy(e, want),
            TExpr::Try(index) => {
                let t = &prog.tries[index as usize];
                let body = self.lower_expr(t.body, want);
                let finalizer = t.finalizer.map(|f| self.lower_expr(f, &JType::V));
                self.lir.push(Kind::Try { index, body, finalizer }, want.clone())
            }
            TExpr::Int(v) if matches!(want, JType::D) => self.lir.push(Kind::Double(v as f64), JType::D),
            TExpr::Int(v) if matches!(want, JType::J) => self.lir.push(Kind::Long(v as i64), JType::J),
            TExpr::Int(v) if matches!(want, JType::F) => self.lir.push(Kind::Float(v as f32), JType::F),
            TExpr::Double(v) if matches!(want, JType::F) => self.lir.push(Kind::Float(v as f32), JType::F),
            _ => {
                let Some(node) = self.lower_node(e) else { return self.lower_legacy(e, want) };
                let t = self.lir.ty(node).clone();
                match self.fit(e, &t, want) {
                    Fit::ValueClass(class, args, null_safe) => self.lir.push(Kind::ValueClass { value: node, from: t, class, args, null_safe }, want.clone()),
                    Fit::Through(s) => {
                        let through = self.lower_adapt(node, &t, &s);
                        self.lower_adapt(through, &s, want)
                    }
                    Fit::Direct => self.lower_adapt(node, &t, want),
                }
            }
        }
    }

    /// One node of the typed IR at the type it leaves; None for a node not lowered yet.
    fn lower_node(&mut self, e: TExprId) -> Option<LirId> {
        let prog = self.cx.input.prog;
        Some(match prog.expr(e) {
            TExpr::Int(v) => {
                // A literal the typer narrowed to the `Byte`, `Short` or `Char` expected keeps that
                // type, so that boxing it gives the narrow class (`(1: Byte): Any` is a `java.lang.Byte`).
                let ty = match self.recorded_type(e).map(|t| self.erase(t)) {
                    Some(narrow @ (JType::B | JType::S | JType::C)) => narrow,
                    _ => JType::I,
                };
                self.lir.push(Kind::Int(v), ty)
            }
            TExpr::Long(v) => self.lir.push(Kind::Long(v), JType::J),
            // A literal the typer narrowed to the `Float` expected is a float constant.
            TExpr::Double(v) if self.recorded_type(e).map(|t| self.erase(t)) == Some(JType::F) => self.lir.push(Kind::Float(v as f32), JType::F),
            TExpr::Double(v) => self.lir.push(Kind::Double(v), JType::D),
            TExpr::Bool(v) => self.lir.push(Kind::Bool(v), JType::Z),
            TExpr::Char(c) => self.lir.push(Kind::Char(c), JType::C),
            TExpr::Str(s) => self.lir.push(Kind::Str(s), JType::L(Rc::from(STRING))),
            TExpr::Unit => self.lir.push(Kind::Unit, JType::V),
            // A lazy local, a captured one and a cell are not lowered yet.
            TExpr::Local(s) => {
                let ty = self.plain_local(s)?.ty.clone();
                self.lir.push(Kind::Local(s), ty)
            }
            // An object without a body that only its exports are reached through is not emitted,
            // and a Java class's statics have no object (`Gen::load_module`).
            TExpr::Module(c) if !self.cx.input.reach.classes[c.idx()] || self.cx.input.java_class(c).is_some() => self.lir.push(Kind::Unit, JType::V),
            TExpr::Module(c) => {
                let name = self.class_name(c);
                self.lir.push(Kind::Module(name.clone()), JType::L(name))
            }
            TExpr::This | TExpr::Super(_) => return self.lower_this(),
            TExpr::Field(r, s) => return self.lower_field_read(r, s),
            // A function value applied: `scala.FunctionN.apply` over the arguments as references.
            TExpr::CallClosure(f, args) => {
                let items: Vec<TExprId> = prog.expr_list(args).to_vec();
                let ft = self.function_type(items.len());
                let JType::L(owner) = ft.clone() else { unreachable!("a function's class") };
                let object = JType::object();
                let params = vec![object.clone(); items.len()];
                let desc = method_desc(&params, &object);
                let method = Rc::new(MRef { owner, owner_is_interface: true, name: "apply".to_string(), params, ret: object.clone(), desc, kind: Invoke::Interface });
                let receiver = self.lower_expr(f, &ft);
                let lowered: Vec<LirId> = items.into_iter().map(|a| self.lower_expr(a, &object)).collect();
                let args = self.lir.list(&lowered);
                self.lir.push(Kind::Call { method, at: None, receiver, args, order: None }, object)
            }
            TExpr::CallMethod(r, s, args) => return self.lower_call_method(r, s, args),
            _ => return None,
        })
    }

    /// The instance the method runs on; None where the method has none.
    fn lower_this(&mut self) -> Option<LirId> {
        Some(match self.this_value()? {
            ThisValue::Module(module) => self.lir.push(Kind::Module(module.clone()), JType::L(module)),
            ThisValue::Slot(_, ty) => self.lir.push(Kind::This, ty),
        })
    }

    /// A member read on a receiver: its field where this class's own code may name it, else its
    /// accessor's call; None for the routes not lowered yet.
    fn lower_field_read(&mut self, r: TExprId, s: SymId) -> Option<LirId> {
        match self.field_route(r, s) {
            FieldRoute::Js | FieldRoute::Java | FieldRoute::Object(_) | FieldRoute::GivenObject | FieldRoute::Underlying(_) | FieldRoute::Early => None,
            FieldRoute::Own => {
                let ty = self.sym_type(s);
                let name: Rc<str> = Rc::from(self.field_name(s));
                let receiver = self.lower_this().expect("an instance, by `Gen::own_field`");
                let owner = self.this_name.clone();
                Some(self.lir.push(Kind::Field { receiver, owner, name }, ty))
            }
            FieldRoute::PrivateParam(c) => {
                let ty = self.sym_type(s);
                let name: Rc<str> = Rc::from(self.field_name(s));
                let owner = self.class_name(c);
                let receiver = self.lower_expr(r, &JType::L(owner.clone()));
                Some(self.lir.push(Kind::Field { receiver, owner, name }, ty))
            }
            FieldRoute::Accessor => {
                let s = self.deferred_implementation(r, s).unwrap_or(s);
                let m = self.mref(s);
                let site = self.invoke_site(r, s, &m);
                let receiver = self.lower_receiver(r, s, &site);
                let args = self.lir.list(&[]);
                let ret = m.ret.clone();
                Some(self.lir.push(Kind::Call { method: m, at: site.map(|site| (site.owner, site.interface)), receiver, args, order: None }, ret))
            }
        }
    }

    fn lower_receiver(&mut self, r: TExprId, s: SymId, site: &Option<Site>) -> LirId {
        let t = self.receiver_type(s, site);
        self.lower_expr(r, &t)
    }

    /// The arguments of a call of `callee` against its parameters' types, as they are written,
    /// and the order a right-associative extension takes them in (`Gen::jvm_param_order`).
    pub(super) fn lower_arguments(&mut self, callee: Option<SymId>, args: ListRef, params: &[JType]) -> (List, Option<Rc<[usize]>>) {
        let order = callee.and_then(|s| self.jvm_param_order(s)).filter(|o| o.len() == params.len());
        let written = match &order {
            Some(order) => self.signature_order(order, params),
            None => params.to_vec(),
        };
        let items: Vec<TExprId> = self.cx.input.prog.expr_list(args).to_vec();
        let mut lowered = Vec::with_capacity(items.len());
        for (i, a) in items.into_iter().enumerate() {
            let want = written.get(i).cloned().unwrap_or_else(JType::object);
            lowered.push(match &want {
                JType::L(n) if n.starts_with('[') => self.lir.push(Kind::JavaArrayArgument(a), want.clone()),
                _ => self.lower_expr(a, &want),
            });
        }
        (self.lir.list(&lowered), order.map(Rc::from))
    }

    /// A call of a member on a receiver; None for the routes not lowered yet.
    fn lower_call_method(&mut self, r: TExprId, s: SymId, args: ListRef) -> Option<LirId> {
        let CallRoute::Plain = self.call_route(r, s, args) else { return None };
        let m = self.mref(s);
        let site = self.invoke_site(r, s, &m);
        let receiver = self.lower_receiver(r, s, &site);
        let (args, order) = self.lower_arguments(Some(s), args, &m.params);
        let ret = m.ret.clone();
        Some(self.lir.push(Kind::Call { method: m, at: site.map(|site| (site.owner, site.interface)), receiver, args, order }, ret))
    }

    /// `value`, of the type `from`, as a value of the type `to`.
    pub(super) fn lower_adapt(&mut self, value: LirId, from: &JType, to: &JType) -> LirId {
        if from == to {
            return value;
        }
        self.lir.push(Kind::Adapt { value, from: from.clone() }, to.clone())
    }
}
