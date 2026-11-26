//! The platform lowering: an expression of the typed IR as nodes of the lowered IR
//! (`crate::lir`). A lowering decides and builds nodes; it writes nothing, and the code and the
//! constant pool refuse a write while one runs (`Gen::lowering`), since the order of what is
//! written is the output's bytes.

use crate::lir::{Arith, Capture, Cast, Catch, Caught, Comparing, ErasedType, ExceptionHandler, FieldRole, Invoke, Kind, LiftedFrom, LiftedShape, LirId, List, MethodRef, ParamRole, Place, Refusal, Relation, TempId};
use super::gen::{rank_type, CallRoute, CaptureArgument, CtorArguments, CtorDefault, DefaultRoute, ExtensionRoute, Extras, FieldRoute, Fit, Gen, GetterArgument, Held, HeldIn, NewRoute, PassedCapture, Referenced, ReturnTarget, Site, StaticCallRoute, StaticRoute, SuperArguments, ThisValue, ViaArguments, ViaRoute};
use super::lifted::{Asker, Placed};
use super::names::*;
use crate::ast::ListRef;
use crate::classfile::ACC_STATIC;
use crate::source::FileId;
use crate::symbols::*;
use crate::tir::*;
use crate::typer::loader::javaclass::JMember;
use crate::types::*;
use std::rc::Rc;

/// A condition reduced as dotty's `FirstTransform` reduces it (`Gen::reduced_condition`): a
/// constant with the expression it stands for, or the expression left to test.
#[derive(Clone, Copy)]
pub(super) enum Reduced {
    Constant(bool, TExprId),
    Expr(TExprId),
}

/// scalac's `MaxIndySlots`: a concatenation's call ends before the part that would bring it to
/// this many operand slots (`Gen::concat`).
const CONCAT_SLOTS: usize = 200;
/// What a constant of the class file holds, in bytes of modified UTF-8: the most text one call
/// of a concatenation holds.
const RECIPE_BYTES: usize = 65535;

/// Whether a text holds a character a concatenation's recipe reads as a tag, which scalac makes
/// an operand of its call (BCodeBodyBuilder.scala 1377-1380 at 3.8.4).
pub(super) fn holds_recipe_tag(text: &str) -> bool {
    text.contains(['\u{1}', '\u{2}'])
}

/// A text's bytes in a class file's constant (modified UTF-8).
fn recipe_bytes(text: &str) -> usize {
    text.chars().map(|c| if c == '\0' { 2 } else if c as u32 >= 0x10000 { 6 } else { c.len_utf8() }).sum()
}

/// What the first reference of an object runs (`Gen::object_inits`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum ObjectInits {
    /// Nothing: its class is an `isNoInitsRealClass`, the reference a `PurePath`.
    Quiet,
    /// Its class or a base runs code when the instance is made: the reference an `IdempotentPath`.
    Initialises,
    /// Nothing records it: a class-path object the typer never completed.
    NotRecorded,
}

impl<'a> Gen<'a> {
    /// The method's parameter at the position, at its declared type.
    pub(super) fn param(&mut self, i: usize) -> LirId {
        let ty = self.m.params[i].1.clone();
        self.lir.push(Kind::Param(i as u32), ty)
    }

    /// The position of the planned method's parameter of the role `role`, where it takes one.
    pub(super) fn param_of(&self, role: &ParamRole) -> Option<usize> {
        self.planned.as_ref()?.params.iter().position(|p| p.role == *role)
    }

    /// Runs a lowering with the writers refusing. No lowering runs while a method's code is written: every tree is
    /// lowered when its method is held, before the writer meets it.
    pub(super) fn lowering<T>(&mut self, f: impl FnOnce(&mut Self) -> T) -> T {
        assert!(!self.method_open, "a lowering runs while a method's code is written");
        let was = self.code.frozen;
        self.code.frozen = true;
        self.cw.cp.frozen = true;
        let out = f(self);
        self.code.frozen = was;
        self.cw.cp.frozen = was;
        // The local defs the lowering met are queued once it ends, in the context of the method it lowered for.
        if !was {
            self.queue_local_defs();
        }
        out
    }

    /// `e` brought to the type `want`.
    pub(super) fn lower_expr(&mut self, e: TExprId, want: &ErasedType) -> LirId {
        let id = self.lower_marked(e, want);
        // A jump where a value is wanted takes its arguments' positions: the operands it drops
        // under it are its place's.
        if !matches!(self.lir.node(id).kind, Kind::SelfTailCall { .. }) {
            self.place(id, e);
        }
        id
    }

    /// The node `id` as the whole of `e`, at its place (`lir::Place`): a selection's and a method
    /// call's line at its member's name, past the receiver; a static call's past its first
    /// argument, an extension's receiver or not (scalac takes another static call's line at its
    /// name); any other's at its start.
    pub(super) fn place(&mut self, id: LirId, e: TExprId) {
        let (cx, prog) = (self.cx, self.cx.input.prog);
        let point = prog.span_of(e).map_or(0, |(file, span)| {
            let receiver = match prog.expr(e) {
                TExpr::CallMethod(recv, ..) | TExpr::Field(recv, _) => Some(recv),
                TExpr::CallStatic(_, args) => prog.expr_list(args).first().copied(),
                _ => None,
            };
            receiver.map_or(span.start, |recv| cx.point_after(file, span, prog.span_of(recv)))
        });
        let call = matches!(prog.expr(e), TExpr::CallMethod(..) | TExpr::CallStatic(..) | TExpr::CallClosure(..) | TExpr::New(..) | TExpr::NewVia(..));
        self.lir.nodes[id.0 as usize].of = Some(Place { expr: e, point, call });
    }

    /// A form the target has no translation of, refused where it stands (`Kind::Refused`): no
    /// value, nothing of the form evaluated.
    pub(super) fn lower_refused(&mut self, refusal: Refusal) -> LirId {
        self.lir.push(Kind::Refused(refusal), ErasedType::V)
    }

    fn lower_marked(&mut self, e: TExprId, want: &ErasedType) -> LirId {
        let prog = self.cx.input.prog;
        if want.is_ref() {
            if let Some(wider) = self.widened_branches(e) {
                let inner = self.lower_expr(e, &wider);
                return self.lower_adapt(inner, &wider, want);
            }
        }
        match prog.expr(e) {
            TExpr::Block(stmts, res) => self.lower_block(stmts, res, want),
            TExpr::If(c, t, els) => self.lower_if(c, t, els, want),
            TExpr::Match(scrut, _) => {
                crate::tir::matcher::check(prog, crate::tir::matcher::Owner::Match(e), "jvm match");
                self.lower_match(scrut, crate::tir::matcher::Owner::Match(e), Some(want))
            }
            TExpr::Throw(inner, _) => {
                let value = self.lower_expr(inner, &ErasedType::L(Rc::from(THROWABLE)));
                self.lir.push(Kind::Throw(value), want.clone())
            }
            TExpr::Return(v) => self.lower_return(e, v, want),
            // A statement's `try` is written at its own type and what it leaves dropped after the join, as dotty's
            // `genStat` loads it (BCodeBodyBuilder.scala 135) and `genLoad` adapts the type `genLoadTry` gives
            // (363, 504-507); where a value is wanted, Erasure's `typedTry` has adapted each branch to it
            // (Erasure.scala 865-866). One of type `Nothing` leaves nothing to drop.
            TExpr::Try(index) => {
                crate::tir::matcher::check(prog, crate::tir::matcher::Owner::Try(index), "jvm try");
                let nothing = self.recorded_type(e).map(|t| self.cx.input.types.canonical(t)) == Some(NOTHING);
                let own = match self.recorded(e) {
                    Some(own) if *want == ErasedType::V && !nothing => own,
                    _ => want.clone(),
                };
                let t = &prog.tries[index as usize];
                let body = self.lower_expr(t.body, &own);
                let catch = (!t.cases.is_empty()).then(|| self.lower_catch(index, &own));
                let finalizer = t.finalizer.map(|f| self.lower_expr(f, &ErasedType::V));
                let node = self.push_protected(Kind::Try { body, catch, finalizer, returns: false }, own.clone());
                self.lower_adapt(node, &own, want)
            }
            // A def's call of itself in a tail position of a `return`'s value: the loop's jump
            // (`Gen::jump_call`); nothing follows it, so its type is the place's.
            TExpr::CallMethod(..) | TExpr::CallStatic(..) if self.jump_call(e).is_some() => {
                let (receiver, args) = self.jump_call(e).expect("a jump, by the guard");
                let kind = self.lower_self_tail_call(receiver, args);
                self.lir.push(kind, want.clone())
            }
            TExpr::Int(v) if matches!(want, ErasedType::D) => self.lir.push(Kind::Double(v as f64), ErasedType::D),
            TExpr::Int(v) if matches!(want, ErasedType::J) => self.lir.push(Kind::Long(v as i64), ErasedType::J),
            TExpr::Int(v) if matches!(want, ErasedType::F) => self.lir.push(Kind::Float(v as f32), ErasedType::F),
            TExpr::Double(v) if matches!(want, ErasedType::F) => self.lir.push(Kind::Float(v as f32), ErasedType::F),
            TExpr::Int(_) | TExpr::Long(_) | TExpr::Double(_) | TExpr::Bool(_) | TExpr::Char(_) | TExpr::Str(_) | TExpr::Unit | TExpr::Null
            | TExpr::Local(_) | TExpr::This | TExpr::Super(_) | TExpr::Static(_) | TExpr::Module(_) | TExpr::Field(..)
            | TExpr::CallStatic(..) | TExpr::CallMethod(..) | TExpr::CallClosure(..) | TExpr::New(..) | TExpr::NewVia(..) | TExpr::Default(..)
            | TExpr::Lambda(..) | TExpr::While(..) | TExpr::Assign(..) | TExpr::Prim(..) | TExpr::Unary(..) | TExpr::StrConcat(_)
            | TExpr::ToStr(..) | TExpr::Js(..) | TExpr::TypeTest(..) | TExpr::Cast(..) | TExpr::ClassOf(..) | TExpr::SeqLit(_)
            | TExpr::ArrayLit(_) | TExpr::Index(..) | TExpr::JsImport(_) | TExpr::JsGlobal(..) | TExpr::JsSelect(..) | TExpr::ObjLit(_)
            | TExpr::Spread(_) | TExpr::Splice(_) => {
                let node = self.lower_node(e);
                let t = self.lir.ty(node).clone();
                match self.fit_lowered(e, &t, want) {
                    Fit::ValueClass(class, args, null_safe) => self.lower_value_class(node, &t, class, args, null_safe, want, Some(e)),
                    Fit::Through(s) => {
                        let through = self.lower_adapt(node, &t, &s);
                        self.lower_adapt(through, &s, want)
                    }
                    Fit::Direct => self.lower_adapt(node, &t, want),
                }
            }
        }
    }

    /// `Gen::fit` for a node the lowering made: besides, the underlying value of a class with an outer wanted as
    /// the class (a method's default getter called on its box) is boxed as the class, as master writes it
    /// (`Cx::vc_with_outer`).
    fn fit_lowered(&mut self, e: TExprId, t: &ErasedType, want: &ErasedType) -> Fit {
        let fit = self.fit(e, t, want);
        if matches!(fit, Fit::ValueClass(..)) || t.is_ref() || *t == ErasedType::V {
            return fit;
        }
        let cx = self.cx;
        match want {
            ErasedType::L(name) => match cx.class_named(name).filter(|&c| cx.vc_with_outer(c)) {
                Some(c) => Fit::ValueClass(c, crate::types::EMPTY_LIST, false),
                None => fit,
            },
            _ => fit,
        }
    }

    /// One node of the typed IR at the type it leaves.
    pub(super) fn lower_node(&mut self, e: TExprId) -> LirId {
        let prog = self.cx.input.prog;
        match prog.expr(e) {
            TExpr::Int(v) => {
                // A literal the typer narrowed to the `Byte`, `Short` or `Char` expected keeps that
                // type, so that boxing it gives the narrow class (`(1: Byte): Any` is a `java.lang.Byte`).
                let ty = match self.recorded_type(e).map(|t| self.erase(t)) {
                    Some(narrow @ (ErasedType::B | ErasedType::S | ErasedType::C)) => narrow,
                    _ => ErasedType::I,
                };
                self.lir.push(Kind::Int(v), ty)
            }
            TExpr::Long(v) => self.lir.push(Kind::Long(v), ErasedType::J),
            // A literal the typer narrowed to the `Float` expected is a float constant.
            TExpr::Double(v) if self.recorded_type(e).map(|t| self.erase(t)) == Some(ErasedType::F) => self.lir.push(Kind::Float(v as f32), ErasedType::F),
            TExpr::Double(v) => self.lir.push(Kind::Double(v), ErasedType::D),
            TExpr::Bool(v) => self.lir.push(Kind::Bool(v), ErasedType::Z),
            TExpr::Char(c) => self.lir.push(Kind::Char(c), ErasedType::C),
            TExpr::Str(s) => self.lower_string(e, s),
            TExpr::Unit => self.lir.push(Kind::Unit, ErasedType::V),
            // A null that an ascription holds is dotty's `Typed`, no literal: `(null: T)`, a cast
            // that keeps no node, a plain inline method's expansion, an inline parameter's
            // bottom-typed argument and `{ null }` as an argument, each marked so by the typer
            // (`Program::is_ascribed`); so is a null an inline expansion wrote from its body
            // (`Program::is_inlined_null`), whose source's ascriptions no pickle keeps. A
            // comparison tests the other side alone only against the literal (`Gen::write_compare`).
            TExpr::Null if prog.is_ascribed(e) || prog.is_inlined_null(e) => {
                let null = self.lir.push(Kind::Null, ErasedType::object());
                self.lir.push(Kind::Typed(null), ErasedType::object())
            }
            TExpr::Null => self.lir.push(Kind::Null, ErasedType::object()),
            TExpr::Local(s) if self.is_lazy_local(s) => self.lower_lazy_read(s),
            // One the scope does not hold is reported where it is lowered (`Gen::lower_capture`).
            TExpr::Local(s) => self.lower_local(s).unwrap_or_else(|| self.lower_capture(s)),
            TExpr::Assign(target, value) => self.lower_assign(target, value),
            TExpr::Prim(op, a, b) => self.lower_prim(op, a, b),
            TExpr::Unary(op, a) => self.lower_unary(op, a),
            // A test's value is the condition's.
            TExpr::TypeTest(inner, test, tested) => {
                let cond = self.lower_test(inner, test, tested);
                let yes = self.lir.push(Kind::Bool(true), ErasedType::Z);
                let no = self.lir.push(Kind::Bool(false), ErasedType::Z);
                self.lir.push(Kind::If { cond, then: yes, otherwise: Some(no) }, ErasedType::Z)
            }
            TExpr::While(c, body) => {
                let cond = self.lower_cond(c);
                let body = self.lower_expr(body, &ErasedType::V);
                self.lir.push(Kind::While { cond: Some(cond), body }, ErasedType::V)
            }
            TExpr::Module(c) => self.lower_module(c),
            TExpr::ClassOf(c, dims) => self.lower_class_of(e, c, dims),
            TExpr::Cast(inner, op, _) => self.lower_cast(inner, op),
            TExpr::ToStr(inner, conv) => self.lower_to_str(inner, conv),
            TExpr::StrConcat(items) => self.lower_str_concat(items),
            TExpr::This | TExpr::Super(_) => self.lower_instance(),
            TExpr::Static(s) => self.lower_static(s),
            TExpr::Field(r, s) => self.lower_field_read(r, s),
            TExpr::CallClosure(f, args) => {
                let items: Vec<TExprId> = prog.expr_list(args).to_vec();
                let ft = self.function_type(items.len());
                let function = self.lower_expr(f, &ft);
                self.lower_application(function, &ft, items)
            }
            TExpr::CallStatic(s, args) => self.lower_call_static(s, args),
            TExpr::New(c, args) => self.lower_new(e, c, args),
            TExpr::NewVia(s, args) => self.lower_new_via(s, args),
            TExpr::Default(of, at, args) => self.lower_default(of, at, args),
            TExpr::CallMethod(r, s, args) => self.lower_call_method(e, r, s, args),
            TExpr::SeqLit(items) => self.lower_seq_literal(e, items),
            // An array of the kind its type names (an enum's `values` is a `Color[]`).
            TExpr::ArrayLit(items) => {
                let array = match self.recorded(e) {
                    Some(ErasedType::L(n)) if n.starts_with('[') => n,
                    _ => Rc::from(OBJECT_ARRAY),
                };
                self.lower_array(items, &array)
            }
            TExpr::Index(r, i) => {
                let array = self.lower_expr(r, &ErasedType::L(Rc::from(OBJECT_ARRAY)));
                let index = self.lir.push(Kind::Int(i as i32), ErasedType::I);
                self.lir.push(Kind::ArrayLoad { array, index }, ErasedType::object())
            }
            TExpr::Lambda(params, body) => self.lower_lambda(params, body),
            TExpr::Js(template, args) => self.lower_template(e, template, args),
            // `xs*` passed to a JavaScript member with repeated parameters (the typer's `js_call_args`):
            // the sequence as it is, at its own type.
            TExpr::Spread(inner) => {
                let t = self.static_type(inner);
                let value = self.lower_expr(inner, &t);
                self.lir.push(Kind::Typed(value), t)
            }
            TExpr::JsImport(_) | TExpr::JsGlobal(..) | TExpr::JsSelect(..) | TExpr::ObjLit(_) => self.lower_refused(Refusal::JavaScriptInterop),
            // Where the value leaves nothing (`Gen::lower_read_again` asks the node at the type it leaves).
            TExpr::Return(v) => self.lower_return(e, v, &ErasedType::V),
            TExpr::If(..) | TExpr::Match(..) | TExpr::Block(..) | TExpr::Throw(..) | TExpr::Try(_) => unreachable!("lowered by `Gen::lower_marked`"),
            TExpr::Splice(_) => unreachable!("a stored inline body is lowered nowhere"),
        }
    }

    /// An operation on two operands; an equality of references calls the runtime's `equal`, whose
    /// absence is reported before any method is written (`runtime::REQUIRED`).
    fn lower_prim(&mut self, op: PrimOp, a: TExprId, b: TExprId) -> LirId {
        use PrimOp::*;
        let (t, arith) = match op {
            IntAdd => (ErasedType::I, Arith::Add),
            IntSub => (ErasedType::I, Arith::Sub),
            IntMul => (ErasedType::I, Arith::Mul),
            IntDiv => (ErasedType::I, Arith::Div),
            IntRem => (ErasedType::I, Arith::Rem),
            IntAnd => (ErasedType::I, Arith::And),
            IntOr => (ErasedType::I, Arith::Or),
            IntXor => (ErasedType::I, Arith::Xor),
            IntShl => (ErasedType::I, Arith::Shl),
            IntShr => (ErasedType::I, Arith::Shr),
            IntUshr => (ErasedType::I, Arith::Ushr),
            LongAdd => (ErasedType::J, Arith::Add),
            LongSub => (ErasedType::J, Arith::Sub),
            LongMul => (ErasedType::J, Arith::Mul),
            LongDiv => (ErasedType::J, Arith::Div),
            LongRem => (ErasedType::J, Arith::Rem),
            LongAnd => (ErasedType::J, Arith::And),
            LongOr => (ErasedType::J, Arith::Or),
            LongXor => (ErasedType::J, Arith::Xor),
            LongShl => (ErasedType::J, Arith::Shl),
            LongShr => (ErasedType::J, Arith::Shr),
            LongUshr => (ErasedType::J, Arith::Ushr),
            DoubleAdd => (ErasedType::D, Arith::Add),
            DoubleSub => (ErasedType::D, Arith::Sub),
            DoubleMul => (ErasedType::D, Arith::Mul),
            DoubleDiv => (ErasedType::D, Arith::Div),
            DoubleRem => (ErasedType::D, Arith::Rem),
            FloatAdd => (ErasedType::F, Arith::Add),
            FloatSub => (ErasedType::F, Arith::Sub),
            FloatMul => (ErasedType::F, Arith::Mul),
            FloatDiv => (ErasedType::F, Arith::Div),
            FloatRem => (ErasedType::F, Arith::Rem),
            BoolStrictAnd => (ErasedType::Z, Arith::And),
            BoolStrictOr => (ErasedType::Z, Arith::Or),
            BoolXor => (ErasedType::Z, Arith::Xor),
            // Universal equality is the runtime's; `!=` is its negation as a value.
            Eq | Ne => {
                let helper = self.cx.helpers.required("equal");
                let method = self.mref(helper);
                let object = ErasedType::object();
                let left = self.lower_expr(a, &object);
                let right = self.lower_expr(b, &object);
                let args = self.lir.list(&[left, right]);
                let equal = self.lir.push(Kind::Helper { method, args }, ErasedType::Z);
                if op == Eq {
                    return equal;
                }
                let yes = self.lir.push(Kind::Bool(true), ErasedType::Z);
                return self.lir.push(Kind::Binary { op: Arith::Xor, left: equal, right: yes }, ErasedType::Z);
            }
            Lt | Le | Gt | Ge | RefEq | RefNe | Same | NotSame => return self.lower_compare(op, a, b),
            // A constant left operand folds (`foldCondition`).
            BoolAnd | BoolOr if matches!(self.reduced_condition(a), Reduced::Constant(..)) => {
                let Reduced::Constant(v, _) = self.reduced_condition(a) else { unreachable!("a constant, by the guard") };
                if v == (op == BoolAnd) {
                    return self.lower_cond(b);
                }
                return self.lir.push(Kind::Bool(v), ErasedType::Z);
            }
            BoolAnd => {
                let (left, right) = (self.lower_cond(a), self.lower_cond(b));
                return self.lir.push(Kind::And(left, right), ErasedType::Z);
            }
            BoolOr => {
                let (left, right) = (self.lower_cond(a), self.lower_cond(b));
                return self.lir.push(Kind::Or(left, right), ErasedType::Z);
            }
        };
        let left = self.lower_expr(a, &t);
        // The shift distance of a long is an int.
        let right = if matches!(op, LongShl | LongShr | LongUshr) {
            let tb = self.static_type(b);
            let right = self.lower_expr(b, &tb);
            self.lower_adapt(right, &tb, &ErasedType::I)
        } else {
            self.lower_expr(b, &t)
        };
        self.lir.push(Kind::Binary { op: arith, left, right }, t)
    }

    fn lower_unary(&mut self, op: UnOp, a: TExprId) -> LirId {
        use UnOp::*;
        // A literal widened to a double is the double constant, as scalac types the literal.
        if let (IntToDouble, TExpr::Int(v)) = (op, self.cx.input.prog.expr(a)) {
            return self.lir.push(Kind::Double(v as f64), ErasedType::D);
        }
        let (from, to) = match op {
            IntNeg | LongNeg | DoubleNeg | FloatNeg => {
                let t = match op {
                    IntNeg => ErasedType::I,
                    LongNeg => ErasedType::J,
                    DoubleNeg => ErasedType::D,
                    _ => ErasedType::F,
                };
                let value = self.lower_expr(a, &t);
                return self.lir.push(Kind::Neg(value), t);
            }
            BoolNot => {
                if let Reduced::Constant(v, _) = self.reduced_condition(a) {
                    return self.lir.push(Kind::Bool(!v), ErasedType::Z);
                }
                let value = self.lower_expr(a, &ErasedType::Z);
                return self.lir.push(Kind::Not(value), ErasedType::Z);
            }
            IntNot => {
                let left = self.lower_expr(a, &ErasedType::I);
                let right = self.lir.push(Kind::Int(-1), ErasedType::I);
                return self.lir.push(Kind::Binary { op: Arith::Xor, left, right }, ErasedType::I);
            }
            LongNot => {
                let left = self.lower_expr(a, &ErasedType::J);
                let right = self.lir.push(Kind::Long(-1), ErasedType::J);
                return self.lir.push(Kind::Binary { op: Arith::Xor, left, right }, ErasedType::J);
            }
            IntToLong => (ErasedType::I, ErasedType::J),
            LongToDouble => (ErasedType::J, ErasedType::D),
            LongToInt => (ErasedType::J, ErasedType::I),
            DoubleToInt => (ErasedType::D, ErasedType::I),
            DoubleToLong => (ErasedType::D, ErasedType::J),
            CharToInt => (ErasedType::C, ErasedType::I),
            CharToLong => (ErasedType::C, ErasedType::J),
            IntToChar => (ErasedType::I, ErasedType::C),
            IntToByte => (ErasedType::I, ErasedType::B),
            IntToShort => (ErasedType::I, ErasedType::S),
            IntToFloat => (ErasedType::I, ErasedType::F),
            IntToDouble => (ErasedType::I, ErasedType::D),
            ByteToShort => (ErasedType::B, ErasedType::S),
            ByteToInt => (ErasedType::B, ErasedType::I),
            ShortToInt => (ErasedType::S, ErasedType::I),
            LongToFloat => (ErasedType::J, ErasedType::F),
            FloatToInt => (ErasedType::F, ErasedType::I),
            FloatToLong => (ErasedType::F, ErasedType::J),
            FloatToDouble => (ErasedType::F, ErasedType::D),
            DoubleToFloat => (ErasedType::D, ErasedType::F),
        };
        let value = self.lower_expr(a, &from);
        self.lower_adapt(value, &from, &to)
    }

    /// A condition as dotty's `FirstTransform` leaves it, before the back end: a literal or a
    /// top-level inline val's boolean is a constant (`constToLiteral`), `!` of a constant one
    /// (`ConstFold`), and `&&` and `||` whose left operand is a constant fold (`foldCondition`:
    /// `true && b` is `b`, `false && b` false, `true || b` true, `false || b` is `b`); a
    /// constant on the right stays an operand. A constant stands for the tree the reduction
    /// leaves: `b` itself where the connective is `b`, the expression otherwise.
    pub(super) fn reduced_condition(&self, e: TExprId) -> Reduced {
        let cx = self.cx;
        match cx.input.prog.expr(e) {
            TExpr::Bool(v) => Reduced::Constant(v, e),
            TExpr::Static(s) => match self.inline_val_constant(s).flatten().map(|l| cx.input.types.lit_val(l)) {
                Some(LitVal::Bool(v)) => Reduced::Constant(v, e),
                _ => Reduced::Expr(e),
            },
            TExpr::Unary(UnOp::BoolNot, a) => match self.reduced_condition(a) {
                Reduced::Constant(v, _) => Reduced::Constant(!v, e),
                Reduced::Expr(_) => Reduced::Expr(e),
            },
            TExpr::Prim(op @ (PrimOp::BoolAnd | PrimOp::BoolOr), a, b) => match (op, self.reduced_condition(a)) {
                (PrimOp::BoolAnd, Reduced::Constant(true, _)) | (PrimOp::BoolOr, Reduced::Constant(false, _)) => self.reduced_condition(b),
                (_, Reduced::Constant(v, _)) => Reduced::Constant(v, e),
                (_, Reduced::Expr(_)) => Reduced::Expr(e),
            },
            _ => Reduced::Expr(e),
        }
    }

    /// The expression a selection reads its member on, as dotty's `FirstTransform` leaves the
    /// qualifier `r`: a block's result, and the branch an `if` over a constant takes
    /// (`Gen::reduced_condition`), each as far as they go.
    fn selected_operand(&self, r: TExprId) -> TExprId {
        let prog = self.cx.input.prog;
        let mut operand = r;
        loop {
            operand = match prog.expr(operand) {
                TExpr::Block(_, res) => res,
                TExpr::If(c, t, els) => match (self.reduced_condition(c), els) {
                    (Reduced::Constant(true, _), _) => t,
                    (Reduced::Constant(false, _), Some(e)) => e,
                    _ => return operand,
                },
                _ => return operand,
            };
        }
    }

    /// The branch an `if` over a constant takes, alone (`FirstTransform.transformIf`); none is
    /// `()` at the type wanted.
    fn lower_taken(&mut self, branch: Option<TExprId>, want: &ErasedType) -> LirId {
        match branch {
            Some(b) => self.lower_expr(b, want),
            None => {
                let nothing = self.lir.push(Kind::Unit, ErasedType::V);
                self.lower_adapt(nothing, &ErasedType::V, want)
            }
        }
    }

    /// `e` as a condition: the connectives and the comparisons under it are conditions too,
    /// evaluated only as far as the outcome needs; anything else is a boolean value. A constant
    /// the reduction leaves is a `Bool`, a reduced connective its remaining operand.
    pub(super) fn lower_cond(&mut self, e: TExprId) -> LirId {
        let prog = self.cx.input.prog;
        // A constant keeps the place of the tree it stands for (dotty's `constToLiteral`, `Literal(value)
        // .withSpan(tree.span)`; `foldCondition` leaves the right operand itself of `true && b` and `false || b`,
        // and the left literal of the others, which starts where the connective does): a block's constant result
        // is written as a value (`Gen::write_cond`), at its place.
        match self.reduced_condition(e) {
            // An inline val's read alone is lowered as the read, which holds its position.
            Reduced::Constant(v, of) if !matches!(prog.expr(e), TExpr::Static(_)) => {
                let id = self.lir.push(Kind::Bool(v), ErasedType::Z);
                self.place(id, of);
                return id;
            }
            Reduced::Expr(x) if x != e => return self.lower_cond(x),
            _ => {}
        }
        match prog.expr(e) {
            TExpr::Bool(_) => unreachable!("a constant, reduced above"),
            // A block's result is a condition too, at its own place, its line entry where `lower_expr` would mark it.
            TExpr::Block(stmts, res) => {
                let nodes = self.lower_block_statements(stmts);
                let result = self.lower_cond(res);
                let node = self.lir.node(result);
                if node.of.is_none() && !matches!(node.kind, Kind::SelfTailCall { .. }) {
                    self.place(result, res);
                }
                let result = self.expansion_tested(result);
                let stmts = self.lir.list(&nodes);
                let id = self.lir.push(Kind::Block { stmts, result }, ErasedType::Z);
                self.place(id, e);
                self.expansion_tested(id)
            }
            TExpr::Unary(UnOp::BoolNot, a) => {
                let value = self.lower_cond(a);
                self.lir.push(Kind::Not(value), ErasedType::Z)
            }
            TExpr::Prim(PrimOp::BoolAnd, a, b) => {
                let (left, right) = (self.lower_cond(a), self.lower_cond(b));
                self.lir.push(Kind::And(left, right), ErasedType::Z)
            }
            TExpr::Prim(PrimOp::BoolOr, a, b) => {
                let (left, right) = (self.lower_cond(a), self.lower_cond(b));
                self.lir.push(Kind::Or(left, right), ErasedType::Z)
            }
            TExpr::Prim(op @ (PrimOp::Lt | PrimOp::Le | PrimOp::Gt | PrimOp::Ge | PrimOp::RefEq | PrimOp::RefNe | PrimOp::Same | PrimOp::NotSame), a, b) => self.lower_compare(op, a, b),
            TExpr::TypeTest(inner, test, tested) => self.lower_test(inner, test, tested),
            TExpr::Prim(..) | TExpr::Unary(..) | TExpr::Int(_) | TExpr::Long(_) | TExpr::Double(_) | TExpr::Char(_) | TExpr::Str(_) | TExpr::Unit
            | TExpr::Null | TExpr::Local(_) | TExpr::This | TExpr::Super(_) | TExpr::Static(_) | TExpr::Module(_) | TExpr::Field(..)
            | TExpr::CallStatic(..) | TExpr::CallMethod(..) | TExpr::CallClosure(..) | TExpr::New(..) | TExpr::NewVia(..) | TExpr::Default(..)
            | TExpr::Lambda(..) | TExpr::If(..) | TExpr::While(..) | TExpr::Assign(..) | TExpr::Match(..)
            | TExpr::StrConcat(_) | TExpr::ToStr(..) | TExpr::Js(..) | TExpr::Cast(..) | TExpr::ClassOf(..) | TExpr::SeqLit(_)
            | TExpr::ArrayLit(_) | TExpr::Index(..) | TExpr::JsImport(_) | TExpr::JsGlobal(..) | TExpr::JsSelect(..) | TExpr::ObjLit(_)
            | TExpr::Spread(_) | TExpr::Return(_) | TExpr::Throw(..) | TExpr::Try(_) | TExpr::Splice(_) => {
                let id = self.lower_expr(e, &ErasedType::Z);
                match self.lir.node(id).kind {
                    Kind::Block { .. } => self.expansion_tested(id),
                    _ => id,
                }
            }
        }
    }

    /// A node of a condition that stands for an inline method's expansion, as its value tested: `Kind::Typed`
    /// over it, which takes its place. scalac tests a plain inline method's expansion so wherever it stands (the
    /// `Typed` of `PrepareInlineable.wrapRHS`, which `genCond` does not look into: BCodeBodyBuilder.scala 1663 at
    /// 3.8.4) and jumps on a transparent one's; teq tests one so where it is a block or a block's result, a
    /// transparent one's too, and lowers any other as the condition it is.
    fn expansion_tested(&mut self, id: LirId) -> LirId {
        let node = self.lir.node(id);
        let Some(of) = node.of.filter(|of| self.cx.input.prog.is_expansion(of.expr)) else { return id };
        if matches!(node.kind, Kind::Typed(_)) {
            return id;
        }
        let ty = node.ty.clone();
        self.lir.nodes[id.0 as usize].of = None;
        let typed = self.lir.push(Kind::Typed(id), ty);
        self.lir.nodes[typed.0 as usize].of = Some(of);
        typed
    }

    /// `inner.isInstanceOf[T]`: the value in a temporary and its test (`Gen::lower_test_on`), a constant where
    /// erasure decides it (`x.isInstanceOf[Matchable]`, a class no program of the build makes), the temporary's
    /// block then written in a condition as the constant tested (`Gen::write_cond`). No cast to the static type:
    /// `box.fruit.isInstanceOf[Apple]` asks because the static type may be wrong. The tested type goes with the
    /// test, as a pattern's does: an array's test reads its element there.
    fn lower_test(&mut self, inner: TExprId, test: TestId, tested: TypeId) -> LirId {
        let object = ErasedType::object();
        let init = self.lower_expr(inner, &object);
        let temp = self.lir.temp();
        let result = self.lower_test_on(temp, &object, test, Some(tested));
        let held = self.lir.push(Kind::Let { temp, init }, ErasedType::V);
        let stmts = self.lir.list(&[held]);
        self.lir.push(Kind::Block { stmts, result }, ErasedType::Z)
    }

    /// An order or an equality of two operands, compared as their static types say; `super eq that`
    /// is the typer's `super.equals(that)` where no class behind defines `equals`
    /// (`Typer::apply_super_any_member`): `Object.equals` as a super call.
    fn lower_compare(&mut self, op: PrimOp, a: TExprId, b: TExprId) -> LirId {
        let object = ErasedType::object();
        if let (PrimOp::Same, TExpr::Super(_)) = (op, self.cx.input.prog.expr(a)) {
            let that = self.lower_expr(b, &object);
            return self.lower_object_super_call(a, "equals", vec![object], ErasedType::Z, vec![that]);
        }
        let relation = match op {
            PrimOp::Lt => Relation::Lt,
            PrimOp::Le => Relation::Le,
            PrimOp::Gt => Relation::Gt,
            PrimOp::Ge => Relation::Ge,
            PrimOp::RefEq | PrimOp::Same => Relation::Eq,
            _ => Relation::Ne,
        };
        // `eq` and `ne` are reference identity, a string's and a box's too (scalac's `ID`).
        let (how, ta, tb) = if let PrimOp::Same | PrimOp::NotSame = op {
            (Comparing::Identity, object.clone(), object)
        } else {
            let (ta, tb) = (self.static_type(a), self.static_type(b));
            let is_eq = matches!(op, PrimOp::RefEq | PrimOp::RefNe);
            let kind = match (ta.numeric_rank(), tb.numeric_rank()) {
                (Some(x), Some(y)) => Some(rank_type(x.max(y))),
                (Some(x), None) if tb.is_ref() && !is_eq => Some(rank_type(x)),
                (None, Some(y)) if ta.is_ref() && !is_eq => Some(rank_type(y)),
                _ => None,
            };
            if let Some(k) = kind {
                (Comparing::Numbers, k.clone(), k)
            } else if !is_eq {
                // Strings and other comparables: `compareTo`.
                let comparable: Rc<str> = Rc::from("java/lang/Comparable");
                let params = vec![object.clone()];
                let desc = method_desc(&params, &ErasedType::I);
                let method = MethodRef { owner: comparable.clone(), owner_is_interface: true, name: "compareTo".to_string(), params, ret: ErasedType::I, desc, kind: Invoke::Interface };
                (Comparing::Ordering(Rc::new(method)), ErasedType::L(comparable), object)
            } else if ta == ErasedType::Z && tb == ErasedType::Z {
                (Comparing::Numbers, ErasedType::Z, ErasedType::Z)
            } else if ta == ErasedType::V && tb == ErasedType::V {
                (Comparing::Units, ErasedType::V, ErasedType::V)
            } else if ta.is_string() || tb.is_string() || !ta.is_ref() || !tb.is_ref() {
                // Strings and boxed primitives compare by value, as `===` does on the JS side.
                let params = vec![object.clone(), object.clone()];
                let desc = method_desc(&params, &ErasedType::Z);
                let method = MethodRef { owner: Rc::from("java/util/Objects"), owner_is_interface: false, name: "equals".to_string(), params, ret: ErasedType::Z, desc, kind: Invoke::Static };
                (Comparing::Equality(Rc::new(method)), object.clone(), object)
            } else if matches!(&ta, ErasedType::L(n) if n.starts_with('[')) {
                // An array's `==`, on the left of it: scalac tests a `null` side alone only where
                // the left is a class.
                (Comparing::ArrayEquality, object.clone(), object)
            } else {
                (Comparing::Identity, object.clone(), object)
            }
        };
        let left = self.lower_expr(a, &ta);
        let right = self.lower_expr(b, &tb);
        self.lir.push(Kind::Compare { relation, how, left, right }, ErasedType::Z)
    }

    /// A conditional whose value is wanted at `want`: each branch is brought to it; one with no
    /// `else` leaves the unit value where a value is wanted.
    fn lower_if(&mut self, c: TExprId, t: TExprId, els: Option<TExprId>, want: &ErasedType) -> LirId {
        if let Reduced::Constant(v, _) = self.reduced_condition(c) {
            return self.lower_taken(if v { Some(t) } else { els }, want);
        }
        let cond = self.lower_cond(c);
        let (then, otherwise) = match els {
            Some(els) => (self.lower_expr(t, want), Some(self.lower_expr(els, want))),
            None if *want == ErasedType::V => (self.lower_expr(t, &ErasedType::V), None),
            None => {
                let then = self.lower_expr(t, &ErasedType::V);
                let then = self.lower_adapt(then, &ErasedType::V, want);
                let nothing = self.lir.push(Kind::Unit, ErasedType::V);
                (then, Some(self.lower_adapt(nothing, &ErasedType::V, want)))
            }
        };
        self.lir.push(Kind::If { cond, then, otherwise }, want.clone())
    }

    /// `e` in return position: the branches of a conditional and the result of a block are in
    /// return position too, a call of the def by itself where it loops is the jump back, and
    /// anything else is returned at the def's result type.
    pub(super) fn lower_tail(&mut self, e: TExprId) -> LirId {
        let prog = self.cx.input.prog;
        // A widened value returned as a reference is boxed as one, not branch by branch.
        let boxed_whole = self.m.ret.is_ref() && self.widened_branches(e).is_some();
        let kind = match prog.expr(e) {
            TExpr::If(c, t, Some(els)) if !boxed_whole && matches!(self.reduced_condition(c), Reduced::Constant(..)) => {
                let Reduced::Constant(v, _) = self.reduced_condition(c) else { unreachable!("a constant, by the guard") };
                return self.lower_tail(if v { t } else { els });
            }
            TExpr::If(c, t, Some(els)) if !boxed_whole => {
                let cond = self.lower_cond(c);
                let then = self.lower_tail(t);
                let otherwise = Some(self.lower_tail(els));
                Kind::If { cond, then, otherwise }
            }
            TExpr::Block(stmts, res) if !boxed_whole => {
                let mut nodes = Vec::new();
                self.lower_lazy_holders(stmts, &mut nodes);
                for &s in &prog.stmts[stmts.range()] {
                    self.lower_statement(s, &mut nodes);
                }
                let result = self.lower_tail(res);
                let stmts = self.lir.list(&nodes);
                Kind::Block { stmts, result }
            }
            TExpr::Match(scrut, _) if !boxed_whole => {
                crate::tir::matcher::check(prog, crate::tir::matcher::Owner::Match(e), "jvm match");
                let id = self.lower_match(scrut, crate::tir::matcher::Owner::Match(e), None);
                self.place(id, e);
                return id;
            }
            // A return out of the method here is that return, its value the method's result (dotty's
            // `genReturn` sends the value to the method's return); out of a closure it is a throw,
            // after which the return of what it leaves is dead.
            TExpr::Return(v) if !self.m.closure => self.lower_local_return(v),
            TExpr::If(..) | TExpr::Block(..) | TExpr::Match(..) | TExpr::Return(_) | TExpr::Int(_) | TExpr::Long(_) | TExpr::Double(_)
            | TExpr::Bool(_) | TExpr::Char(_) | TExpr::Str(_) | TExpr::Unit | TExpr::Null | TExpr::Local(_) | TExpr::This | TExpr::Super(_)
            | TExpr::Static(_) | TExpr::Module(_) | TExpr::Field(..) | TExpr::CallStatic(..) | TExpr::CallMethod(..) | TExpr::CallClosure(..)
            | TExpr::New(..) | TExpr::NewVia(..) | TExpr::Default(..) | TExpr::Lambda(..) | TExpr::While(..) | TExpr::Assign(..)
            | TExpr::Prim(..) | TExpr::Unary(..) | TExpr::StrConcat(_) | TExpr::ToStr(..) | TExpr::Js(..) | TExpr::TypeTest(..)
            | TExpr::Cast(..) | TExpr::ClassOf(..) | TExpr::SeqLit(_) | TExpr::ArrayLit(_) | TExpr::Index(..) | TExpr::JsImport(_)
            | TExpr::JsGlobal(..) | TExpr::JsSelect(..) | TExpr::ObjLit(_) | TExpr::Spread(_) | TExpr::Throw(..) | TExpr::Try(_)
            | TExpr::Splice(_) => match self.m.tail.as_ref().and_then(|tail| prog.self_call(tail.sym, tail.params.len(), e)) {
                Some((receiver, args)) => self.lower_self_tail_call(receiver, args),
                None => {
                    let ret = self.m.ret.clone();
                    let value = self.lower_expr(e, &ret);
                    self.returned(value)
                }
            },
        };
        let id = self.lir.push(kind, ErasedType::V);
        self.place(id, e);
        id
    }

    /// The return of `value`: from each branch of a value class's null test
    /// (`Gen::lower_vc_open_null_safe`), through the block that holds its temporary, as the back end
    /// writes an `If` in return position (`BCodeBodyBuilder.genLoadIfTo` with
    /// `LoadDestination.Return`, 234 and 648 at 3.8.4).
    fn returned(&mut self, value: LirId) -> Kind {
        if !self.m.null_tests.contains(&value) {
            return Kind::Return(value);
        }
        match self.lir.node(value).kind.clone() {
            Kind::If { cond, then, otherwise: Some(otherwise) } => {
                let then = self.lir.push(Kind::Return(then), ErasedType::V);
                let otherwise = Some(self.lir.push(Kind::Return(otherwise), ErasedType::V));
                Kind::If { cond, then, otherwise }
            }
            Kind::Block { stmts, result } => {
                let kind = self.returned(result);
                Kind::Block { stmts, result: self.lir.push(kind, ErasedType::V) }
            }
            _ => unreachable!("a null test is an `If`, or a block around one"),
        }
    }

    /// `return v` where a value of the type `want` is wanted: nothing follows it, so the node's type
    /// is the place's (dotty's `genLoadTo` sends a `Return` to its destination and adapts nothing
    /// after it). Out of a closure it is the throw the def's handler takes by its key.
    fn lower_return(&mut self, e: TExprId, v: TExprId, want: &ErasedType) -> LirId {
        if self.m.closure {
            return self.lower_non_local_return(v, want);
        }
        let kind = self.lower_local_return(v);
        let id = self.lir.push(kind, want.clone());
        // The jump back stands for the whole `return`, whose position it takes.
        self.place(id, e);
        id
    }

    /// `return v` out of the def's own method: the jump back where `v` is the def's call of itself
    /// that loops (`Gen::jump_call`), else the method left with `v` (dotty's `genReturn`), through
    /// the finalizers and monitors around it.
    fn lower_local_return(&mut self, v: TExprId) -> Kind {
        match self.jump_call(v) {
            Some((receiver, args)) => self.lower_self_tail_call(receiver, args),
            None => {
                let ret = self.m.ret.clone();
                Kind::Return(self.lower_expr(v, &ret))
            }
        }
    }

    /// `return v` out of a closure: the throw of a `NonLocalReturnControl` of the key of the def it
    /// leaves and the value, as dotty's `NonLocalReturns.nonLocalReturnThrow` makes it. The value is the
    /// def's erased result, as Erasure adapts a `return`'s expression before NonLocalReturns passes it on
    /// as an `Object` (`ensureConforms`): a value class's underlying value, a primitive boxed.
    fn lower_non_local_return(&mut self, v: TExprId, want: &ErasedType) -> LirId {
        let object = ErasedType::object();
        let key = self.lower_return_key();
        let value = match self.m.return_target.clone() {
            ReturnTarget::Def(ErasedType::V) => self.lower_expr(v, &object),
            ReturnTarget::Def(ret) => {
                let value = self.lower_expr(v, &ret);
                self.lower_adapt(value, &ret, &object)
            }
            ReturnTarget::NoDef => self.lower_expr(v, &object),
            ReturnTarget::Unknown => {
                self.unsupported("internal error: a return out of a closure whose def is not known");
                self.lower_expr(v, &object)
            }
        };
        let ctor = method_ref(Invoke::Special, &Rc::from(NON_LOCAL_RETURN), false, "<init>", &[object.clone(), object], &ErasedType::V);
        let control = self.lir.make(&ctor, &[key, value]);
        self.lir.push(Kind::Throw(control), want.clone())
    }

    pub(super) fn lower_self_tail_call(&mut self, receiver: Option<TExprId>, args: ListRef) -> Kind {
        let cx = self.cx;
        let params = self.m.tail.as_ref().expect("a def that loops").params.clone();
        // A value class's extension method holds its instance unboxed (`$this`): the receiver is
        // lowered at the underlying type, as the call's lowering lowers it (`Gen::lower_extension_call`).
        let this_type = match (self.m.this_home, self.underlying().map(|(_, u)| u), self.this_class) {
            (None, Some(u), Some(_)) => u,
            (_, _, Some(c)) => self.class_type(c),
            (_, _, None) => ErasedType::object(),
        };
        let receiver = receiver.map(|r| self.lower_expr(r, &this_type));
        let mut nodes = Vec::new();
        for (&a, p) in cx.input.prog.expr_list(args).iter().zip(&params) {
            let t = self.m.locals[p].ty.clone();
            nodes.push(self.lower_expr(a, &t));
        }
        let args = self.lir.list(&nodes);
        Kind::SelfTailCall { receiver, args }
    }

    /// A block, its statements lowered (`Gen::lower_block_statements`), its result at `want`.
    fn lower_block(&mut self, stmts: ListRef, res: TExprId, want: &ErasedType) -> LirId {
        let prog = self.cx.input.prog;
        let nodes = self.lower_block_statements(stmts);
        // The pure result of an ascribed block that is a statement is not evaluated (`Program::removed_bits`).
        let result = if *want == ErasedType::V && prog.is_removed(res) { self.lir.push(Kind::Unit, ErasedType::V) } else { self.lower_expr(res, want) };
        let stmts = self.lir.list(&nodes);
        self.lir.push(Kind::Block { stmts, result }, want.clone())
    }

    /// The statements of a block: the holders of its lazy vals, then each statement.
    fn lower_block_statements(&mut self, stmts: ListRef) -> Vec<LirId> {
        let prog = self.cx.input.prog;
        let mut nodes = Vec::new();
        self.lower_lazy_holders(stmts, &mut nodes);
        for &s in &prog.stmts[stmts.range()] {
            self.lower_statement(s, &mut nodes);
        }
        nodes
    }

    /// A statement of a block as the nodes it is, in order; a pattern definition by the branch of dotty's
    /// `makePatDef` the typer took for it (`PatDefForm`).
    pub(super) fn lower_statement(&mut self, s: TStmt, nodes: &mut Vec<LirId>) {
        let cx = self.cx;
        let prog = cx.input.prog;
        if let TStmt::Pat(pat, ..) = s {
            crate::tir::matcher::check(prog, crate::tir::matcher::Owner::PatDef(pat), "jvm pattern definition");
        }
        match s {
            TStmt::Expr(x) if prog.is_removed(x) => {}
            TStmt::Expr(x) => nodes.push(self.lower_expr(x, &ErasedType::V)),
            // Its holder stands at the head of the list (`Gen::lower_lazy_holders`).
            TStmt::Val(sym, _) if self.is_lazy_local(sym) => {}
            TStmt::Val(sym, init) => self.lower_val(sym, init, nodes),
            // A local def evaluates nothing where it stands: the class writing the code writes its method
            // (`Gen::queue_local_defs`).
            TStmt::Fun(f) => {
                if cx.input.reach.funs[f.idx()] {
                    let sym = prog.funs[f.idx()].sym;
                    self.local_fun_shape(sym);
                    self.local_defs.push(sym);
                }
            }
            TStmt::Pat(pat, init, PatDefForm::General) => self.lower_variables_definition(pat, init, false, nodes),
            TStmt::Pat(pat, init, PatDefForm::ForAlias) => self.lower_variables_definition(pat, init, true, nodes),
            TStmt::Pat(pat, init, PatDefForm::SimpleTuple) if self.xxl_definition_items(pat).is_some() => self.lower_simple_xxl_definition(pat, init, nodes),
            TStmt::Pat(pat, init, PatDefForm::SimpleTuple) => self.lower_simple_tuple_definition(pat, init, nodes),
            TStmt::Pat(pat, init, PatDefForm::MatchableTuple) if self.xxl_definition_items(pat).is_some() => self.lower_xxl_definition(pat, init, nodes),
            TStmt::Pat(pat, init, PatDefForm::MatchableTuple) => self.lower_tuple_definition(pat, init, nodes),
            // `IdPattern`'s definition is a plain val: the parser gives an identifier alone no pattern (`Parser::parse_val`).
            TStmt::Pat(_, _, PatDefForm::Id) => unreachable!("an identifier's definition is a plain val (`Parser::parse_val`)"),
        }
    }

    /// The holders of the local lazy vals a statement list declares, before its statements, so
    /// that a local def or a lazy val above a definition reads its holder (`LazyVals.transformStats`,
    /// LazyVals.scala 199-209). Each copy of the list a writer writes (a `finally`'s) makes its own.
    pub(super) fn lower_lazy_holders(&mut self, stmts: ListRef, nodes: &mut Vec<LirId>) {
        let prog = self.cx.input.prog;
        for &s in &prog.stmts[stmts.range()] {
            if let TStmt::Val(sym, init) = s {
                if self.is_lazy_local(sym) {
                    self.an.lazy_inits.insert(sym, init);
                    nodes.push(self.lower_lazy_holder(sym));
                }
            }
        }
    }

    /// The local lazy val `s` declared as its holder, `val x$lzyN = new LazyX()`
    /// (`LazyVals.transformLocalDef`): the local holds the holder, which its accessor reads.
    pub(super) fn lower_lazy_holder(&mut self, s: SymId) -> LirId {
        let (class, _) = self.lazy_holder_class(s);
        let holder = ErasedType::L(class.clone());
        let desc = method_desc(&[], &ErasedType::V);
        let ctor = Rc::new(MethodRef { owner: class, owner_is_interface: false, name: "<init>".to_string(), params: Vec::new(), ret: ErasedType::V, desc, kind: Invoke::Special });
        let made = self.lir.make(&ctor, &[]);
        self.lower_declared.insert(s, (holder, false));
        self.lir.push(Kind::Val { sym: s, init: made, cell: None }, ErasedType::V)
    }

    fn lower_val(&mut self, sym: SymId, init: TExprId, nodes: &mut Vec<LirId>) {
        let ty = self.sym_type(sym);
        if !self.m.cells.contains(&sym) {
            let init = self.lower_expr(init, &ty);
            self.lower_declared.insert(sym, (ty, false));
            nodes.push(self.lir.push(Kind::Val { sym, init, cell: None }, ErasedType::V));
            return;
        }
        // The cell exists before the initialiser runs, which may refer to the val itself.
        let cell: Rc<str> = Rc::from(OBJECT_REF);
        let cell_type = ErasedType::L(cell.clone());
        let object = ErasedType::object();
        let null = self.lir.push(Kind::Null, object.clone());
        let args = self.lir.list(&[null]);
        let params = vec![object.clone()];
        let desc = method_desc(&params, &ErasedType::V);
        let ctor = Rc::new(MethodRef { owner: cell.clone(), owner_is_interface: false, name: "<init>".to_string(), params, ret: ErasedType::V, desc, kind: Invoke::Special });
        let made = self.lir.push(Kind::New { touch: None, ctor, args }, cell_type.clone());
        nodes.push(self.lir.push(Kind::Val { sym, init: made, cell: Some(ty.clone()) }, ErasedType::V));
        self.lower_declared.insert(sym, (ty.clone(), true));
        let holder = self.lir.push(Kind::Local(sym), cell_type);
        let value = self.lower_expr(init, &ty);
        let value = self.lower_adapt(value, &ty, &object);
        nodes.push(self.lir.push(Kind::FieldStore { receiver: holder, owner: cell, name: Rc::from("elem"), value }, ErasedType::V));
    }

    /// An assignment to a local or to a member; one to any other target refused.
    fn lower_assign(&mut self, target: TExprId, value: TExprId) -> LirId {
        let sym = match self.cx.input.prog.expr(target) {
            TExpr::Local(sym) => sym,
            TExpr::Field(r, s) => return self.lower_member_assign(Some(r), s, value),
            TExpr::Static(s) => return self.lower_member_assign(None, s, value),
            TExpr::Int(_) | TExpr::Long(_) | TExpr::Double(_) | TExpr::Bool(_) | TExpr::Char(_) | TExpr::Str(_) | TExpr::Unit | TExpr::Null
            | TExpr::This | TExpr::Super(_) | TExpr::Module(_) | TExpr::CallStatic(..) | TExpr::CallMethod(..) | TExpr::CallClosure(..)
            | TExpr::New(..) | TExpr::NewVia(..) | TExpr::Default(..) | TExpr::Lambda(..) | TExpr::If(..) | TExpr::While(..)
            | TExpr::Block(..) | TExpr::Assign(..) | TExpr::Match(..) | TExpr::Prim(..) | TExpr::Unary(..) | TExpr::StrConcat(_)
            | TExpr::ToStr(..) | TExpr::Js(..) | TExpr::TypeTest(..) | TExpr::Cast(..) | TExpr::ClassOf(..) | TExpr::SeqLit(_)
            | TExpr::ArrayLit(_) | TExpr::Index(..) | TExpr::JsImport(_) | TExpr::JsGlobal(..) | TExpr::JsSelect(..) | TExpr::ObjLit(_)
            | TExpr::Spread(_) | TExpr::Return(_) | TExpr::Throw(..) | TExpr::Try(_) | TExpr::Splice(_) => return self.lower_refused(Refusal::AssignmentTarget),
        };
        if !self.is_cell(sym) {
            // One the method does not hold as a local is reported where it is lowered: held
            // nowhere, or a capture held by value in a field.
            let Some(HeldIn::Local(ty)) = self.held_in(sym) else {
                self.unsupported("assignment to a local that is not in scope");
                return self.lir.push(Kind::Unit, ErasedType::V);
            };
            let value = self.lower_expr(value, &ty);
            return self.lir.push(Kind::Assign { sym, value }, ErasedType::V);
        }
        let declared = self.sym_type(sym);
        // A cell the scope does not hold is reported where it is lowered (`Gen::lower_capture`).
        let cell = self.lower_held(sym).unwrap_or_else(|| self.lower_capture(sym));
        let object = ErasedType::object();
        let value = self.lower_expr(value, &declared);
        let value = self.lower_adapt(value, &declared, &object);
        self.lir.push(Kind::FieldStore { receiver: cell, owner: Rc::from(OBJECT_REF), name: Rc::from("elem"), value }, ErasedType::V)
    }

    /// An assignment to the member `s`, on the receiver `r` where one is written: a Java field
    /// assigned, else the member's setter called, on the receiver, on the member's object or on
    /// the object of the file that declares the member; a Java instance's field with no receiver,
    /// which the typer does not make, refused as an internal error.
    fn lower_member_assign(&mut self, r: Option<TExprId>, s: SymId, value: TExprId) -> LirId {
        let cx = self.cx;
        if let Some((cf, JMember::Field(i))) = cx.input.java_member(s) {
            let f = &cf.fields[i as usize];
            let t = parse_type(&f.descriptor).0;
            let name: Rc<str> = Rc::from(f.name.as_str());
            // A static's receiver is not evaluated.
            let r = match (f.access & ACC_STATIC != 0, r) {
                (true, _) => {
                    let value = self.lower_expr(value, &t);
                    return self.lir.push(Kind::StaticFieldStore { owner: Rc::from(cf.name.as_str()), name, value }, ErasedType::V);
                }
                (false, Some(r)) => r,
                (false, None) => return self.lower_refused(Refusal::JavaFieldWithoutReceiver(name)),
            };
            let (receiver, owner) = self.lower_java_qualifier(r, s, &cf.name);
            let value = self.lower_expr(value, &t);
            return self.lir.push(Kind::FieldStore { receiver, owner, name, value }, ErasedType::V);
        }
        let (receiver, at, m) = match r {
            Some(r) => {
                let m = self.mref(s);
                let site = self.invoke_site(r, s, &m);
                (self.lower_receiver(r, s, &site), site.map(|site| (site.owner, site.interface)), m)
            }
            None => {
                let info = cx.input.syms.sym(s);
                let receiver = match info.owner {
                    Owner::Class(c) => self.lower_module(c),
                    _ => self.lower_file_module(info.file),
                };
                (receiver, None, self.mref(s))
            }
        };
        let value = self.lower_expr(value, &m.ret);
        let args = self.lir.list(&[value]);
        let setter = Rc::new(self.setter_ref(&m));
        self.lir.push(Kind::Call { method: setter, at, receiver, args, order: None }, ErasedType::V)
    }

    /// The receiver of a Java instance's field, and the class the reference names: the
    /// qualifier's where the reference points there (`Gen::qualifier_site`, as dotty's `fieldOp`
    /// names the receiver it is given), else the declaring one.
    fn lower_java_qualifier(&mut self, r: TExprId, s: SymId, declaring: &str) -> (LirId, Rc<str>) {
        match self.qualifier_site(r, declaring, Referenced::Sym(s)) {
            Some(site) => (self.lower_expr(r, &site.qualifier), site.owner),
            None => {
                let owner: Rc<str> = Rc::from(declaring);
                (self.lower_expr(r, &ErasedType::L(owner.clone())), owner)
            }
        }
    }

    /// What the scope holds for a local (`Gen::held_in`).
    pub(super) fn lower_held(&mut self, s: SymId) -> Option<LirId> {
        Some(match self.held_in(s)? {
            HeldIn::Local(ty) => self.lir.push(Kind::Local(s), ty),
            HeldIn::Field(field, ty) => {
                let receiver = self.lower_this()?;
                let owner = self.this_name.clone();
                self.lir.push(Kind::Field { receiver, owner, name: Rc::from(field) }, ty)
            }
        })
    }

    /// A local's value: what the scope holds for it, or the content of its cell.
    pub(super) fn lower_local(&mut self, s: SymId) -> Option<LirId> {
        let declared = if self.cx.input.syms.sym(s).by_name { self.function_type(0) } else { self.sym_type(s) };
        let held = self.lower_held(s)?;
        if !self.is_cell(s) {
            return Some(held);
        }
        let object = ErasedType::object();
        let content = self.lir.push(Kind::Field { receiver: held, owner: Rc::from(OBJECT_REF), name: Rc::from("elem") }, object.clone());
        Some(self.lower_adapt(content, &object, &declared))
    }

    /// Varargs packed into a sequence: a new instance of the class the build's standard library
    /// wraps an array in, over an array of the items as references. scala-library's
    /// `ArraySeq.ofRef` takes an array of references, the std's `ArraySeq` an array of any kind.
    /// Refused where the library has no such class.
    fn lower_seq_literal(&mut self, e: TExprId, items: ListRef) -> LirId {
        if self.cx.input.array_seq.is_none() {
            return self.lower_refused(Refusal::VarargsWithoutArraySeq);
        }
        let elem = self.seq_literal_element(e);
        let held = if elem.is_ref() { ErasedType::object() } else { elem };
        let items: Vec<TExprId> = self.cx.input.prog.expr_list(items).to_vec();
        let values: Vec<LirId> = items.into_iter().map(|item| self.lower_expr(item, &held)).collect();
        self.lower_varargs_values(&values, &held)
    }

    /// The argument of a Scala varargs parameter made of `values`, each of the type `held` (`Object` where the
    /// elements erase to a reference, else their primitive type; an `Int` among `Double`s already brought to
    /// `Double`, which the typer leaves implicit): the values boxed into an array of references, and the sequence
    /// class (`cx.input.array_seq`) made over it. The one writer of a varargs argument the lowered path has.
    pub(super) fn lower_varargs_values(&mut self, values: &[LirId], held: &ErasedType) -> LirId {
        let seq = self.cx.input.array_seq.expect("a varargs argument where the build has the sequence class (`Gen::lower_seq_literal`)");
        let name = self.class_name(seq);
        let array = ErasedType::L(Rc::from(OBJECT_ARRAY));
        let object = ErasedType::object();
        let boxed: Vec<LirId> = values.iter().map(|&value| self.lower_adapt(value, held, &object)).collect();
        let items = self.lir.list(&boxed);
        let boxed = self.lir.push(Kind::ArrayOf { elem: object.clone(), items }, array.clone());
        let param = if self.cx.linked_class(seq) { array.clone() } else { object };
        let wrapped = self.lower_adapt(boxed, &array, &param);
        let args = self.lir.list(&[wrapped]);
        let desc = method_desc(std::slice::from_ref(&param), &ErasedType::V);
        let ctor = Rc::new(MethodRef { owner: name.clone(), owner_is_interface: false, name: "<init>".to_string(), params: vec![param], ret: ErasedType::V, desc, kind: Invoke::Special });
        self.lir.push(Kind::New { touch: None, ctor, args }, ErasedType::L(name))
    }

    /// A new array of the class `array` holding the items, each at its elements' type.
    fn lower_array(&mut self, items: ListRef, array: &Rc<str>) -> LirId {
        let elem = parse_type(&array[1..]).0;
        let items: Vec<TExprId> = self.cx.input.prog.expr_list(items).to_vec();
        let lowered: Vec<LirId> = items.into_iter().map(|item| self.lower_expr(item, &elem)).collect();
        let items = self.lir.list(&lowered);
        self.lir.push(Kind::ArrayOf { elem, items }, ErasedType::L(array.clone()))
    }

    /// An object's instance; no value for an object that has none (`Gen::module_name`).
    pub(super) fn lower_module(&mut self, c: ClassId) -> LirId {
        match self.module_name(c) {
            Some(name) => self.lir.push(Kind::Module(name.clone()), ErasedType::L(name)),
            None => self.lir.push(Kind::Unit, ErasedType::V),
        }
    }

    /// `classOf[C]`, and with dimensions the class of the array of that many of `C`.
    /// The string literal `e` of the text `s`; where the text passes what a class file's constant
    /// holds, scalac's error at the literal (BCodeBodyBuilder.scala 588-593) and a null of its type
    /// written in its place.
    fn lower_string(&mut self, e: TExprId, s: StrRef) -> LirId {
        let string = ErasedType::L(Rc::from(STRING));
        let text = self.cx.input.prog.strings[s.idx()].as_str();
        if super::classfile::constant_text_fits(text, "") {
            return self.lir.push(Kind::Str(Rc::from(text)), string);
        }
        self.refuse(e, "String constant is too long for the JVM".to_string());
        self.lir.push(Kind::Null, string)
    }

    fn lower_class_of(&mut self, e: TExprId, c: ClassId, dims: u8) -> LirId {
        // An array of a known element says its dimensions (`TExpr::ClassOf`).
        let of = if dims > 0 {
            let element = match crate::emit::class_of_name(self.cx.input.syms, self.cx.input.interner, c) {
                Some(n) => n,
                None => self.class_name(c).to_string(),
            };
            ErasedType::L(Rc::from(crate::emit::array_class_name(&element, dims).replace('.', "/")))
        } else {
            // `classOf[Array[T]]` is the array class of `T`'s erasure, which the node's type says.
            let types = self.cx.input.types;
            let element = self.recorded_type(e).and_then(|t| match types.get(t) {
                Type::Class(_, args) => types.items(args).first().copied(),
                _ => None,
            }).and_then(|array| match types.get(array) {
                Type::Class(_, args) => types.items(args).first().copied(),
                _ => None,
            });
            self.class_kind(c, element)
        };
        let class = ErasedType::L(Rc::from("java/lang/Class"));
        // The class constant's text: a class's name, an array's descriptor (BCodeBodyBuilder.scala 609-614).
        if let ErasedType::L(name) = &of {
            if !super::classfile::constant_text_fits(name, "") {
                self.refuse(e, "Type name is too long for the JVM".to_string());
                return self.lir.push(Kind::Null, class);
            }
        }
        self.lir.push(Kind::ClassOf(of), class)
    }

    /// `inner.asInstanceOf[T]` as the typer decided it (`CastOp`), which is what dotty's erasure
    /// leaves of the cast: the operand as an `Object` and the test of the erasure the typer
    /// decided, the type's own (`TypeErasure.erasure`), before what consumes the value adapts
    /// or drops it, so that a cast discarded, widened or cast again is tested; the test of
    /// `scala.runtime.Null$` for `Null`, which the erasure of a type makes an `Object`; the
    /// unboxing (`Erasure.Boxing.unbox`); the failure of a cast to `Nothing`. A value class's
    /// cast leaves its value boxed and untested, the adaptation of what consumes it testing it
    /// there (`Erasure.Boxing.adaptToType`).
    fn lower_cast(&mut self, inner: TExprId, op: CastOp) -> LirId {
        let object = ErasedType::object();
        let (test, erased) = match op {
            CastOp::Written | CastOp::Erased => {
                let t = self.static_type(inner);
                let value = self.lower_expr(inner, &t);
                return self.lir.push(Kind::Cast { value, op: Cast::Erased }, t);
            }
            CastOp::Nothing => {
                let value = self.lower_expr(inner, &ErasedType::V);
                return self.lir.push(Kind::Cast { value, op: Cast::Nothing }, object);
            }
            // The two unboxings differ on JavaScript alone.
            CastOp::Unbox(_, erased) | CastOp::Adapt(_, erased) => {
                let prim = self.erase(erased);
                assert!(!prim.is_ref() && prim != ErasedType::V, "an unboxing to no primitive");
                let value = self.lower_expr(inner, &object);
                return self.lir.push(Kind::Cast { value, op: Cast::Unbox }, prim);
            }
            CastOp::Check(test, erased) => (test, erased),
        };
        let value = self.lower_expr(inner, &object);
        let tested = match self.cx.input.prog.tests[test.idx()] {
            TypeTest::Null => Some(Rc::from("scala/runtime/Null$")),
            // `FunctionXXL`, which teq's output has no class of: a function past 22 parameters is
            // an interface of its own arity here (docs/TARGETS.md), which no one class takes all of.
            TypeTest::Function(n) if n > 22 => None,
            _ => match self.erase(erased) {
                ErasedType::L(c) if &*c != OBJECT && self.value_class_type(erased).is_none() => Some(c),
                _ => None,
            },
        };
        match tested {
            Some(class) => self.lir.push(Kind::Cast { value, op: Cast::Checked }, ErasedType::L(class)),
            None => self.lir.push(Kind::Cast { value, op: Cast::Unchecked }, object),
        }
    }

    /// `inner` as a string: what a concatenation makes of an operand (a rendering) or the
    /// program's `inner.toString`.
    fn lower_to_str(&mut self, inner: TExprId, conv: StrConv) -> LirId {
        let string = ErasedType::L(Rc::from(STRING));
        // A value class's `toString` is its box's (`V@1`, a case class's `CV(1)`), as dotty's
        // `typedSelect` boxes an erased value class qualifier (`Erasure.scala` 733): a rendering
        // writes `null` for a null reference the box stayed.
        if self.expr_value_class(inner).is_some() && conv.is_rendering() {
            let value = self.lower_expr(inner, &ErasedType::object());
            return self.lir.push(Kind::StringOf { value }, string);
        }
        // The program's `x.toString` on a reference is the call, which `null` throws on, as
        // dotty's `genApply` invokes it, on the class the qualifier points at
        // (`Gen::object_member_site`, a value class's box).
        if !conv.is_rendering() && (self.expr_value_class(inner).is_some() || self.static_type(inner).is_ref()) {
            let (site, qualifier) = self.object_member_site(inner, "toString");
            let receiver = self.lower_expr(inner, &qualifier);
            let desc = method_desc(&[], &string);
            let method = Rc::new(MethodRef { owner: Rc::from(OBJECT), owner_is_interface: false, name: "toString".to_string(), params: Vec::new(), ret: string.clone(), desc, kind: Invoke::Virtual });
            let args = self.lir.list(&[]);
            return self.lir.push(Kind::Call { method, at: site.map(|site| (site.owner, site.interface)), receiver, args, order: None }, string);
        }
        let t = self.static_type(inner);
        let value = self.lower_expr(inner, &t);
        self.lir.push(Kind::StringOf { value }, string)
    }

    /// A concatenation, in a block where the head of its chain brings statements: they run
    /// before the first operand, and their locals end with the concatenation.
    fn lower_str_concat(&mut self, items: ListRef) -> LirId {
        let (statements, operands) = self.concat_route(items);
        if statements.is_empty() {
            return self.lower_concat(&operands);
        }
        let prog = self.cx.input.prog;
        let mut nodes = Vec::new();
        for stmts in statements {
            self.lower_lazy_holders(stmts, &mut nodes);
            for &s in &prog.stmts[stmts.range()] {
                self.lower_statement(s, &mut nodes);
            }
        }
        let result = self.lower_concat(&operands);
        let ty = self.lir.ty(result).clone();
        let stmts = self.lir.list(&nodes);
        self.lir.push(Kind::Block { stmts, result }, ty)
    }

    /// `a + b + c` on strings over the chain's operands (`Gen::concat_route`): a string
    /// literal and each constant the typer folds are constants of the result; any other operand
    /// is lowered at its own type, a value class's as its box and one of no value as the unit
    /// value, and is a constant of the result where it lowers to a string constant.
    pub(super) fn lower_concat(&mut self, operands: &[TExprId]) -> LirId {
        let prog = self.cx.input.prog;
        let string = ErasedType::L(Rc::from(STRING));
        // scalac's typer folds the constants a chain starts with into one literal.
        let value_of = |e: TExprId| prog.expr(e).rendering().map_or(e, |(inner, _)| inner);
        let constant = |e: TExprId| matches!(prog.expr(value_of(e)), TExpr::Str(_) | TExpr::Int(_) | TExpr::Long(_) | TExpr::Bool(_) | TExpr::Char(_));
        let folded = match operands.iter().take_while(|&&e| constant(e)).count() {
            1 => 0,
            n => n,
        };
        let mut parts = Vec::with_capacity(operands.len());
        for (i, &operand) in operands.iter().enumerate() {
            let value = value_of(operand);
            let text = match prog.expr(value) {
                // scalac drops the empty literals before it counts.
                TExpr::Str(text) if prog.strings[text.idx()].is_empty() => continue,
                // A text of the result, never an `ldc`: dotty checks only the `ldc` (BCodeBodyBuilder.scala 588-593), and a
                // text's length counts toward its call's (`Gen::concat`).
                TExpr::Str(text) => {
                    parts.push(self.lir.push(Kind::Str(Rc::from(prog.strings[text.idx()].as_str())), string.clone()));
                    continue;
                }
                // An inline val's read is its literal by the back end (dotty's `Inlining`, after the
                // typer, which folds nothing of it), a string literal text of the recipe.
                TExpr::Static(s) => match self.inline_val_constant(s).flatten().map(|l| self.cx.input.types.lit_val(l)) {
                    Some(LitVal::Str(n)) if self.cx.input.interner.get(n).is_empty() => continue,
                    Some(LitVal::Str(n)) => Some(self.cx.input.interner.get(n).to_string()),
                    _ => None,
                },
                TExpr::Int(v) if i < folded && self.static_type(value) == ErasedType::I => Some(v.to_string()),
                TExpr::Long(v) if i < folded => Some(v.to_string()),
                TExpr::Bool(v) if i < folded => Some(v.to_string()),
                TExpr::Char(v) if i < folded => char::from_u32(v as u32).map(|c| c.to_string()),
                _ => None,
            };
            if let Some(text) = text {
                parts.push(self.lir.push(Kind::Str(Rc::from(text)), string.clone()));
                continue;
            }
            let t = self.static_type(value);
            // A value class's instance is concatenated as its box, whatever it holds.
            let boxed = self.recorded_type(value).and_then(|rt| self.value_class_of(rt)).is_some();
            let part = if t == ErasedType::V {
                let dropped = self.lower_expr(value, &ErasedType::V);
                self.lower_adapt(dropped, &ErasedType::V, &ErasedType::L(Rc::from(BOXED_UNIT)))
            } else if boxed {
                self.lower_expr(value, &ErasedType::object())
            } else {
                self.lower_expr(value, &t)
            };
            // An operand that lowers to a string constant, the literal branch of a condition reduced as
            // `FirstTransform` reduces it, is the literal dotty's back end meets: a text of the result
            // (`Gen::concat` reads it off the part's kind), and dropped where empty.
            if !matches!(&self.lir.node(part).kind, Kind::Str(text) if text.is_empty()) {
                parts.push(part);
            }
        }
        self.concat(&parts)
    }

    /// The concatenation of `parts` in the calls scalac's back end makes of it (`Kind::Concat`;
    /// `genStringConcat`, BCodeBodyBuilder.scala 1358-1397 at 3.8.4): before each part, a text
    /// too, the call so far ends where the part's slots would bring it to `MaxIndySlots`; a text
    /// that holds a recipe tag is an operand of one slot. So is a text past the bytes a call's
    /// text holds, which scalac's back end does not bound (the class file's writer refuses it).
    pub(super) fn concat(&mut self, parts: &[LirId]) -> LirId {
        let (mut calls, mut operands) = (Vec::new(), Vec::new());
        let (mut slots, mut bytes) = (0, 0);
        for (i, &part) in parts.iter().enumerate() {
            let node = self.lir.node(part);
            let size = if matches!(node.ty, ErasedType::J | ErasedType::D) { 2 } else { 1 };
            if slots + size >= CONCAT_SLOTS {
                calls.push(i as u32);
                (slots, bytes) = (0, 0);
            }
            match &node.kind {
                Kind::Str(text) if !holds_recipe_tag(text) && bytes + recipe_bytes(text) <= RECIPE_BYTES => bytes += recipe_bytes(text),
                Kind::Str(_) => {
                    operands.push(i as u32);
                    slots += 1;
                    bytes += 1;
                }
                _ => {
                    slots += size;
                    bytes += 1;
                }
            }
        }
        let parts = self.lir.list(parts);
        self.lir.push(Kind::Concat { parts, calls: calls.into(), operands: operands.into() }, ErasedType::L(Rc::from(STRING)))
    }

    /// `lock.synchronized(body)`: the body at its own type, which is the value's.
    pub(super) fn lower_synchronized(&mut self, lock: TExprId, body: TExprId) -> LirId {
        let t = self.static_type(body);
        let lock = self.lower_expr(lock, &ErasedType::object());
        let body = self.lower_expr(body, &t);
        self.push_protected(Kind::Synchronized { lock, body, returns: false }, t)
    }

    /// A def's body that a closure it makes returns from, as dotty's `NonLocalReturns.nonLocalReturnTry` makes it
    /// (NonLocalReturns.scala 80-91): the `Let` of a fresh key of each run, which the closures capture
    /// (`Gen::body_key` while the body is lowered) and their `return`s throw in a `NonLocalReturnControl`; and the
    /// method's return of the body in a `try` whose handler of `NonLocalReturnControl` (`case ex:
    /// NonLocalReturnControl`, a handler of its own) gives the value of one carrying this run's key and throws any
    /// other on.
    pub(super) fn lower_keyed_body(&mut self, body: TExprId) -> (LirId, LirId) {
        debug_assert!(self.given_key().is_none(), "the method of a keyed body is given a key: {}", self.planned.as_ref().map_or("", |m| &*m.name));
        let object = ErasedType::object();
        let control: Rc<str> = Rc::from(NON_LOCAL_RETURN);
        let control_type = ErasedType::L(control.clone());
        let ret = self.m.ret.clone();
        let key = self.lir.temp();
        let ctor = method_ref(Invoke::Special, &Rc::from(OBJECT), false, "<init>", &[], &ErasedType::V);
        let made = self.lir.make(&ctor, &[]);
        let kept = self.lir.keep(key, made);
        let outer = self.body_key.replace(key);
        let value = self.lower_expr(body, &ret);
        self.body_key = outer;
        // A def that loops returns its body's value from inside the loop, as TailRec wraps the body it loops in a
        // return to the method (TailRec.scala 219-221) before NonLocalReturns wraps the whole in its `try`.
        let value = match self.m.tail {
            Some(_) => self.lir.push(Kind::Return(value), ret.clone()),
            None => value,
        };
        let (caught, done) = (self.lir.temp(), self.lir.label());
        let key_of = method_ref(Invoke::Virtual, &control, false, "key", &[], &object);
        let value_of = method_ref(Invoke::Virtual, &control, false, "value", &[], &object);
        let ex = self.lir.push(Kind::Temp(caught), control_type.clone());
        let theirs = self.lir.call(&key_of, ex, &[]);
        let mine = self.lir.push(Kind::Temp(key), object.clone());
        let same = self.lir.compare(Relation::Eq, Comparing::Identity, theirs, mine);
        let ex = self.lir.push(Kind::Temp(caught), control_type.clone());
        let carried = self.lir.call(&value_of, ex, &[]);
        let returned = self.lower_adapt(carried, &object, &ret);
        let ex = self.lir.push(Kind::Temp(caught), control_type.clone());
        let rethrown = self.lir.push(Kind::Throw(ex), ret.clone());
        let handled = self.lir.push(Kind::If { cond: same, then: returned, otherwise: Some(rethrown) }, ret.clone());
        let handler = self.lower_handler_value(handled, Vec::new(), done, &ret);
        let handlers = Rc::from([ExceptionHandler { class: control, caught: Caught::Temp(caught), body: handler }]);
        let protected = self.push_protected(Kind::Try { body: value, catch: Some(Catch { done, handlers }), finalizer: None, returns: false }, ret.clone());
        (kept, self.lir.push(Kind::Return(protected), ErasedType::V))
    }

    /// The `try` or `synchronized` node, its `returns` recorded (`Body::returns_through`).
    pub(super) fn push_protected(&mut self, kind: Kind, ty: ErasedType) -> LirId {
        let id = self.lir.push(kind, ty);
        let through = self.lir.returns_through(id);
        match &mut self.lir.nodes[id.0 as usize].kind {
            Kind::Try { returns, .. } | Kind::Synchronized { returns, .. } => *returns = through,
            _ => unreachable!("a `try` or a `synchronized` block"),
        }
        id
    }

    /// The instance of the object that holds a file's top-level definitions.
    fn lower_file_module(&mut self, file: FileId) -> LirId {
        let name = self.file_module_name(file);
        self.lir.push(Kind::Module(name.clone()), ErasedType::L(name))
    }

    /// A function value of the class `ft` applied: `scala.FunctionN.apply` over the arguments as
    /// references.
    fn lower_application(&mut self, function: LirId, ft: &ErasedType, items: Vec<TExprId>) -> LirId {
        let ErasedType::L(owner) = ft.clone() else { unreachable!("a function's class") };
        let object = ErasedType::object();
        let params = vec![object.clone(); items.len()];
        let desc = method_desc(&params, &object);
        let method = Rc::new(MethodRef { owner, owner_is_interface: true, name: "apply".to_string(), params, ret: object.clone(), desc, kind: Invoke::Interface });
        let lowered: Vec<LirId> = items.into_iter().map(|a| self.lower_expr(a, &object)).collect();
        let args = self.lir.list(&lowered);
        self.lir.push(Kind::Call { method, at: None, receiver: function, args, order: None }, object)
    }

    /// A member read with no receiver written.
    pub(super) fn lower_static(&mut self, s: SymId) -> LirId {
        let cx = self.cx;
        let info = cx.input.syms.sym(s);
        match self.static_route(s) {
            StaticRoute::GivenObject => self.lower_given_object(s),
            StaticRoute::EnumField(companion, e) => {
                let owner = self.class_name(companion);
                let t = self.class_type(e);
                let name = encode(cx.input.interner.get(info.name));
                self.lir.push(Kind::StaticField { owner, name: Rc::from(&*name) }, t)
            }
            StaticRoute::EnumValue(case) => self.lower_own_enum_value(info.owner, case),
            StaticRoute::InlineVal => match self.inline_val_constant(s).flatten() {
                Some(l) => self.lower_literal(l),
                None => self.lower_refused(Refusal::InlineValWithoutConstant(Rc::from(cx.input.interner.get(info.name)))),
            },
            StaticRoute::JavaField => {
                let Some((cf, JMember::Field(i))) = cx.input.java_member(s) else { unreachable!("a Java field, by the route") };
                let f = &cf.fields[i as usize];
                let t = parse_type(&f.descriptor).0;
                self.lir.push(Kind::StaticField { owner: Rc::from(cf.name.as_str()), name: Rc::from(f.name.as_str()) }, t)
            }
            StaticRoute::Field => {
                let m = self.mref(s);
                self.lir.push(Kind::StaticField { owner: m.owner.clone(), name: Rc::from(m.name.as_str()) }, m.ret.clone())
            }
            StaticRoute::Accessor => {
                let m = self.mref(s);
                let receiver = match info.owner {
                    Owner::Class(c) => self.lower_module(c),
                    _ => self.lower_file_module(info.file),
                };
                let args = self.lir.list(&[]);
                let ret = m.ret.clone();
                self.lir.push(Kind::Call { method: m, at: None, receiver, args, order: None }, ret)
            }
        }
    }

    /// The value of the enum case `case` held by its own class, read once the values are made:
    /// the companion `owner` initialised first where this build writes its body, else the enum,
    /// which makes its values when it is initialised. A read that initialised the value's class
    /// first would wait for the enum, its superclass, while another thread making the values
    /// waited for it.
    fn lower_own_enum_value(&mut self, owner: Owner, case: ClassId) -> LirId {
        let cx = self.cx;
        let name = self.class_name(case);
        let value = self.lir.static_field(&name, "$instance", ErasedType::L(name.clone()));
        let Owner::Class(companion) = owner else { return value };
        let first = if cx.layout.has_body(companion) && cx.input.reach.classes[companion.idx()] {
            let module = self.lower_module(companion);
            let t = self.lir.ty(module).clone();
            self.lower_adapt(module, &t, &ErasedType::V)
        } else if let Some(e) = cx.enum_of_case(case) {
            let enum_name = self.class_name(e);
            let touch = method_ref(Invoke::Static, &enum_name, false, "$touch", &[], &ErasedType::V);
            self.lir.static_call(&touch, &[])
        } else {
            return value;
        };
        self.lir.seq(&[first], value)
    }

    /// A given object's instance as a value of the given's declared type, which its class
    /// implements: taken as that with no check, so that a build that does not see the class's
    /// parents casts no more than one that does.
    pub(super) fn lower_given_object(&mut self, s: SymId) -> LirId {
        let &k = self.cx.given_objects.get(&s).expect("a given object, by the route");
        let name = self.class_name(k);
        let declared = self.sym_type(s);
        self.lir.push(Kind::Module(name), declared)
    }

    /// A call with no receiver written.
    fn lower_call_static(&mut self, s: SymId, args: ListRef) -> LirId {
        let cx = self.cx;
        match self.static_call_route(s, args) {
            StaticCallRoute::Lifted => self.lower_lifted_call(s, args),
            StaticCallRoute::Spread(array) => self.lower_wrapped_spread(array, None),
            StaticCallRoute::JavaStatic | StaticCallRoute::JdkStatic => self.lower_static_call(s, args),
            StaticCallRoute::LocalValue => self.lower_local_function_call(s, args),
            StaticCallRoute::FileModule => {
                let m = self.mref(s);
                let receiver = self.lower_file_module(cx.input.syms.sym(s).file);
                let (args, order) = self.lower_arguments(Some(s), args, &m.params);
                let ret = m.ret.clone();
                self.lir.push(Kind::Call { method: m, at: None, receiver, args, order }, ret)
            }
        }
    }

    /// `f(args)` of the local function value `s`, read as the scope holds it (a lazy val's by its
    /// accessor, a cell's content, a field of the instance for a captured one), then applied.
    pub(super) fn lower_local_function_call(&mut self, s: SymId, args: ListRef) -> LirId {
        let items: Vec<TExprId> = self.cx.input.prog.expr_list(args).to_vec();
        let ft = self.function_type(items.len());
        let read = match self.is_lazy_local(s) {
            true => self.lower_lazy_call(s, false),
            // One the scope does not hold is reported where it is lowered (`Gen::lower_capture`).
            false => self.lower_local(s).unwrap_or_else(|| self.lower_capture(s)),
        };
        let t = self.lir.ty(read).clone();
        let function = self.lower_adapt(read, &t, &ft);
        self.lower_application(function, &ft, items)
    }

    /// An array spread into a Scala varargs parameter, wrapped as it is, not copied: dotty's
    /// `ElimRepeated.arrayToSeq` (`tpd.wrapArray`), `ScalaRunTime.wrapRefArray` for an array of
    /// references, `wrapIntArray` and its siblings for a primitive one, `genericWrapArray` for one
    /// of a type parameter, so a later write to the array shows in the sequence. `qualifier`, the
    /// receiver the typer's conversion is written on, is evaluated for its effect first, in a
    /// block around the module alone: a block's locals end with it, and a slot the array's code
    /// holds is held past the call.
    fn lower_wrapped_spread(&mut self, array: TExprId, qualifier: Option<TExprId>) -> LirId {
        let t = self.static_type(array);
        let wrap = match &t {
            ErasedType::L(d) if &**d == "[Lscala/runtime/BoxedUnit;" => "wrapUnitArray",
            ErasedType::L(d) if d.starts_with("[L") || d.starts_with("[[") => "wrapRefArray",
            ErasedType::L(d) => match &d[..] {
                "[Z" => "wrapBooleanArray",
                "[B" => "wrapByteArray",
                "[C" => "wrapCharArray",
                "[D" => "wrapDoubleArray",
                "[F" => "wrapFloatArray",
                "[I" => "wrapIntArray",
                "[J" => "wrapLongArray",
                "[S" => "wrapShortArray",
                _ => "genericWrapArray",
            },
            _ => "genericWrapArray",
        };
        let param = match wrap {
            "wrapRefArray" => ErasedType::L(Rc::from("[Ljava/lang/Object;")),
            "genericWrapArray" => ErasedType::object(),
            _ => t.clone(),
        };
        let module: Rc<str> = Rc::from("scala/runtime/ScalaRunTime$");
        let module_type = ErasedType::L(module.clone());
        let dropped = qualifier.map(|q| self.lower_expr(q, &ErasedType::V));
        let mut receiver = self.lir.push(Kind::Module(module.clone()), module_type.clone());
        if let Some(dropped) = dropped {
            let stmts = self.lir.list(&[dropped]);
            receiver = self.lir.push(Kind::Block { stmts, result: receiver }, module_type);
        }
        let wrapped = self.lower_expr(array, &param);
        let args = self.lir.list(&[wrapped]);
        let seq = ErasedType::L(Rc::from("scala/collection/immutable/ArraySeq"));
        let desc = method_desc(std::slice::from_ref(&param), &seq);
        let method = Rc::new(MethodRef { owner: module, owner_is_interface: false, name: wrap.to_string(), params: vec![param], ret: seq.clone(), desc, kind: Invoke::Virtual });
        self.lir.push(Kind::Call { method, at: None, receiver, args, order: None }, seq)
    }

    /// A new instance and its constructor's call.
    pub(super) fn lower_new(&mut self, e: TExprId, c: ClassId, args: ListRef) -> LirId {
        let route = match self.new_route(e, c, args) {
            NewRoute::Instance(route) => route,
            NewRoute::Underlying(u, a) => {
                // The argument takes the constructor parameter's own representation (dotty types it
                // against the erased parameter, `Erasure.scala` 827, boxing a value class there,
                // 394-398): a value class over a universal trait passed as `U` is its box, which is
                // then this class's underlying value.
                let param = self.vc_unmarked(&u);
                let value = self.lower_expr(a, &param);
                return self.lir.push(Kind::Typed(value), u);
            }
            NewRoute::Js => return self.lower_refused(Refusal::JavaScriptClass),
        };
        let touch = self.touched_module(c);
        let name = self.class_name(c);
        let (nodes, params) = self.lower_new_arguments(c, args, route);
        self.lower_new_instance(touch, name, &nodes, params)
    }

    /// A new instance through the secondary constructor `s`.
    pub(super) fn lower_new_via(&mut self, s: SymId, args: ListRef) -> LirId {
        let (c, route) = match self.via_route(s, args) {
            ViaRoute::Unowned => return self.lower_refused(Refusal::SecondaryConstructorOfNoClass),
            ViaRoute::Js => return self.lower_refused(Refusal::JavaScriptClass),
            ViaRoute::Instance(c, route) => (c, route),
        };
        let touch = self.touched_module(c);
        let name = self.class_name(c);
        let (nodes, params) = self.lower_via_arguments(c, s, args, route);
        self.lower_new_instance(touch, name, &nodes, params)
    }

    /// A new instance of the class `name`, its constructor of the parameters `params` run over
    /// the arguments.
    fn lower_new_instance(&mut self, touch: Option<Rc<str>>, name: Rc<str>, nodes: &[LirId], params: Vec<ErasedType>) -> LirId {
        let args = self.lir.list(nodes);
        let desc = method_desc(&params, &ErasedType::V);
        let ctor = Rc::new(MethodRef { owner: name.clone(), owner_is_interface: false, name: "<init>".to_string(), params, ret: ErasedType::V, desc, kind: Invoke::Special });
        self.lir.push(Kind::New { touch, ctor, args }, ErasedType::L(name))
    }

    /// The arguments of the primary constructor of `c` as a creation passes them, in order, and
    /// the constructor's parameter types: what the class captures first.
    pub(super) fn lower_new_arguments(&mut self, c: ClassId, args: ListRef, route: CtorArguments) -> (Vec<LirId>, Vec<ErasedType>) {
        match route {
            CtorArguments::Captured => {
                let n = self.capture_count(c);
                let mut nodes = if n > 0 { self.lower_new_captures(c, args, n) } else { Vec::new() };
                let mut params = self.anon_ctor_params(c);
                let rest = ListRef { start: args.start + n as u32, len: args.len - n as u32 };
                // An anonymous class takes its superclass's arguments, evaluated where it is made.
                if let Some((sup, via, types)) = self.anon_super_params(c) {
                    nodes.extend(match self.anon_super_route(sup, via, rest) {
                        SuperArguments::Captured => {
                            let route = self.ctor_arguments_route(sup, rest);
                            self.lower_new_arguments(sup, rest, route).0
                        }
                        SuperArguments::Via(v) => {
                            let route = self.via_arguments_route(sup, v, rest);
                            self.lower_via_arguments(sup, v, rest, route).0
                        }
                        SuperArguments::Unwritten(outer, declared) => self.lower_unwritten_ctor_arguments(via, rest, outer, declared).0,
                        SuperArguments::Declared => self.lower_declared_arguments(rest, &types),
                    });
                    params.extend(types);
                }
                if self.cx.input.syms.class(c).kind != ClassKind::Anon {
                    let declared = self.ctor_param_types(c);
                    nodes.extend(self.lower_declared_arguments(rest, &declared));
                    params.extend(declared);
                }
                let (value, types) = self.lower_enum_value_arguments(c);
                nodes.extend(value);
                params.extend(types);
                (nodes, params)
            }
            CtorArguments::Unwritten(outer, declared) => self.lower_unwritten_ctor_arguments(None, args, outer, declared),
            CtorArguments::Declared => {
                let params = self.ctor_param_types(c);
                (self.lower_declared_arguments(args, &params), params)
            }
            // Its class file gives no one answer (two of its constructors fit the primary's
            // parameters, or it is not read): the enclosing instance the typer passes first is taken
            // by the owner's kind and the count, at the parameter's type where it is a local's read,
            // else at its own.
            CtorArguments::UnwrittenOuter(outer) => {
                let first = self.cx.input.prog.expr_list(args)[0];
                let at = if matches!(self.cx.input.prog.expr(first), TExpr::Local(_)) { outer.clone() } else { self.static_type(first) };
                let mut nodes = vec![self.lower_expr(first, &at)];
                let rest = ListRef { start: args.start + 1, len: args.len - 1 };
                let declared = self.ctor_param_types(c);
                nodes.extend(self.lower_declared_arguments(rest, &declared));
                (nodes, std::iter::once(outer).chain(declared).collect())
            }
        }
    }

    /// What a value of an enum of a block or a class passes after its captures: its name and its
    /// ordinal, and their types; nothing for a creation of any other class.
    pub(super) fn lower_enum_value_arguments(&mut self, c: ClassId) -> (Vec<LirId>, Vec<ErasedType>) {
        let cx = self.cx;
        let info = cx.input.syms.class(c);
        if info.kind != ClassKind::EnumCase || info.singleton.is_none() {
            return (Vec::new(), Vec::new());
        }
        let string = ErasedType::L(Rc::from(STRING));
        let name = self.lir.push(Kind::Str(Rc::from(cx.input.interner.get(info.name))), string.clone());
        let ordinal = self.lir.push(Kind::Int(info.ordinal as i32), ErasedType::I);
        (vec![name, ordinal], vec![string, ErasedType::I])
    }

    /// The arguments of the secondary constructor `s` of `c` as a creation passes them, in
    /// order, and the constructor's parameter types.
    pub(super) fn lower_via_arguments(&mut self, c: ClassId, s: SymId, args: ListRef, route: ViaArguments) -> (Vec<LirId>, Vec<ErasedType>) {
        match route {
            ViaArguments::Unwritten(outer, declared) => self.lower_unwritten_ctor_arguments(Some(s), args, outer, declared),
            ViaArguments::Captured(n) => {
                let mut nodes = self.lower_new_captures(c, args, n);
                let mut types = self.anon_ctor_params(c);
                let params = self.param_types(s);
                let rest = ListRef { start: args.start + n as u32, len: args.len - n as u32 };
                nodes.extend(self.lower_declared_arguments(rest, &params));
                types.extend(params);
                (nodes, types)
            }
            // A class nested in a class takes the enclosing instance first, as every one of its
            // constructors does (dotty's `ExplicitOuter`, the typer's `new_outer_arg`).
            ViaArguments::Outer(outer) => {
                let first = self.cx.input.prog.expr_list(args)[0];
                let mut nodes = vec![self.lower_expr(first, &outer)];
                let params = self.param_types(s);
                let rest = ListRef { start: args.start + 1, len: args.len - 1 };
                nodes.extend(self.lower_declared_arguments(rest, &params));
                (nodes, std::iter::once(outer).chain(params).collect())
            }
            ViaArguments::Declared => {
                let params = self.param_types(s);
                (self.lower_declared_arguments(args, &params), params)
            }
        }
    }

    /// The arguments of a constructor of a class this build does not write, where the typer
    /// passed the enclosing instance first, as the class file's constructor takes them
    /// (`Gen::unwritten_ctor_layout`): the instance at the type `outer` where the constructor
    /// takes one, not evaluated where it takes none.
    pub(super) fn lower_unwritten_ctor_arguments(&mut self, via: Option<SymId>, args: ListRef, outer: Option<ErasedType>, declared: Vec<ErasedType>) -> (Vec<LirId>, Vec<ErasedType>) {
        let rest = ListRef { start: args.start + 1, len: args.len - 1 };
        let Some(outer) = outer else { return (self.lower_declared_arguments(rest, &declared), declared) };
        // As the whole build passes it, so that the two builds' class files agree: through a
        // secondary constructor at the enclosing class (`Gen::lower_via_arguments`); through the
        // primary as a captured outer is (`Gen::lower_capture_argument`): a local at the enclosing
        // class, the capture's type, whatever subclass the instance is of, and any other value
        // at its own type, cast where the verifier does not take that type for the parameter's
        // (a member of a generic value, which is an `Object`).
        let first = self.cx.input.prog.expr_list(args)[0];
        let instance = if via.is_some() || matches!(self.cx.input.prog.expr(first), TExpr::Local(_)) {
            self.lower_expr(first, &outer)
        } else {
            let own = self.static_type(first);
            let value = self.lower_expr(first, &own);
            match (&own, &outer) {
                (ErasedType::L(from), ErasedType::L(to)) if from != to && !self.is_subclass(from, to) => self.lower_adapt(value, &own, &outer),
                _ => value,
            }
        };
        let mut nodes = vec![instance];
        nodes.extend(self.lower_declared_arguments(rest, &declared));
        (nodes, std::iter::once(outer).chain(declared).collect())
    }

    /// What a creation of the lifted class `c` passes for its captures (`Gen::passed_captures`),
    /// each as the scope holds it.
    fn lower_new_captures(&mut self, c: ClassId, args: ListRef, n: usize) -> Vec<LirId> {
        let mut nodes = Vec::new();
        for passed in self.passed_captures(c, args, n) {
            nodes.push(match passed {
                PassedCapture::Argument(a, p) => {
                    let route = self.capture_argument_route(a, p);
                    self.lower_capture_argument(a, route)
                }
                PassedCapture::Held(s) => self.lower_capture(s),
            });
        }
        let extras = self.class_extras(c);
        nodes.extend(self.lower_extras(&extras));
        nodes
    }

    /// The argument `a` that hands a lifted class a local, as the class holds it
    /// (`Gen::capture_argument_route`).
    pub(super) fn lower_capture_argument(&mut self, a: TExprId, route: CaptureArgument) -> LirId {
        match route {
            CaptureArgument::Value(t) => self.lower_expr(a, &t),
            CaptureArgument::Held(s) => self.lower_capture(s),
            CaptureArgument::Own => {
                let t = self.static_type(a);
                let value = self.lower_expr(a, &t);
                if t == ErasedType::V { self.lower_adapt(value, &t, &ErasedType::L(Rc::from(BOXED_UNIT))) } else { value }
            }
            CaptureArgument::Holder { lazy, value } => self.lower_filled_holder(lazy, value),
        }
    }

    /// A new holder of the local lazy val `lazy` (`Gen::lazy_holder_class`), initialised with the
    /// value of `value`: the holder, which `initialize` is called on and its result dropped.
    fn lower_filled_holder(&mut self, lazy: SymId, value: TExprId) -> LirId {
        let (class, held) = self.lazy_holder_class(lazy);
        let holder = ErasedType::L(class.clone());
        let ctor = method_ref(Invoke::Special, &class, false, "<init>", &[], &ErasedType::V);
        let initialize = method_ref(Invoke::Virtual, &class, false, "initialize", std::slice::from_ref(&held), &held);
        let temp = self.lir.temp();
        let made = self.lir.make(&ctor, &[]);
        let made = self.lir.keep(temp, made);
        let receiver = self.lir.kept(temp, holder.clone());
        let value = self.lower_expr(value, &held);
        let filled = self.lir.call(&initialize, receiver, &[value]);
        let filled = self.lower_adapt(filled, &held, &ErasedType::V);
        let result = self.lir.kept(temp, holder);
        self.lir.seq(&[made, filled], result)
    }

    /// Arguments as they are written, each at its parameter's type, as nodes in order.
    fn lower_declared_arguments(&mut self, args: ListRef, params: &[ErasedType]) -> Vec<LirId> {
        let (list, _) = self.lower_arguments(None, args, params);
        self.lir.lists[list.start as usize..(list.start + list.len) as usize].to_vec()
    }

    /// A default argument: its getter's call, or its expression where the class has no
    /// getters.
    pub(super) fn lower_default(&mut self, of: DefaultOf, at: u32, args: ListRef) -> LirId {
        let (held, route) = match self.default_route(of, at, args) {
            DefaultRoute::Refused(refusal) => return self.lower_refused(refusal),
            DefaultRoute::LocalDef(s, i) => return self.lower_local_default(s, i, args),
            DefaultRoute::Held(held, route) => (Some(held), *route),
            route => (None, route),
        };
        let Some(held) = held else { return self.lower_default_route(route, &[], None) };
        // The receiver first, then the arguments before the default, each in a temporary.
        let mut stmts = Vec::new();
        let receiver = match &route {
            DefaultRoute::Method { receiver, receiver_type, .. } | DefaultRoute::Extension { receiver, underlying: receiver_type, .. } => {
                let init = self.lower_expr(*receiver, receiver_type);
                let temp = self.lir.temp();
                stmts.push(self.lir.keep(temp, init));
                Some((temp, receiver_type.clone()))
            }
            _ => None,
        };
        let mut temps = Vec::with_capacity(held.len());
        for (e, ty) in held {
            let init = self.lower_expr(e, &ty);
            let temp = self.lir.temp();
            stmts.push(self.lir.keep(temp, init));
            temps.push((e, temp, ty));
        }
        let call = self.lower_default_route(route, &temps, receiver);
        self.lir.seq(&stmts, call)
    }

    /// A default's getter called by its route, the arguments `held` in temporaries read where the
    /// getter takes them, the receiver too where it is `receiver`'s.
    fn lower_default_route(&mut self, route: DefaultRoute, held: &[(TExprId, TempId, ErasedType)], receiver: Option<(TempId, ErasedType)>) -> LirId {
        match route {
            DefaultRoute::Ctor(c, recv, route, args, want) => {
                let args = self.lower_getter_arguments(&args, held);
                self.lower_ctor_default(c, route, recv, &args, &want)
            }
            DefaultRoute::FileMember(file, getter, args, want) => {
                let receiver = self.lower_file_module(file);
                let args = self.lower_getter_arguments(&args, held);
                self.lower_getter_call_at(receiver, None, getter, &args, &want)
            }
            DefaultRoute::Method { receiver: r, receiver_type, getter, at, args, want } => {
                let receiver = match receiver {
                    Some((temp, ty)) => self.lir.kept(temp, ty),
                    None => self.lower_expr(r, &receiver_type),
                };
                let args = self.lower_getter_arguments(&args, held);
                self.lower_getter_call_at(receiver, at, getter, &args, &want)
            }
            DefaultRoute::Extension { receiver: r, underlying, getter, args, want } => {
                let module = self.lir.push(Kind::Module(getter.owner.clone()), ErasedType::L(getter.owner.clone()));
                let value = match receiver {
                    Some((temp, ty)) => self.lir.kept(temp, ty),
                    None => self.lower_expr(r, &underlying),
                };
                let mut nodes = vec![self.lower_adapt(value, &underlying, &getter.params[0])];
                nodes.extend(self.lower_getter_arguments(&args, held));
                self.lower_getter_call_at(module, None, getter, &nodes, &want)
            }
            DefaultRoute::Held(..) | DefaultRoute::Refused(_) | DefaultRoute::LocalDef(..) => unreachable!("a route of a call, by `lower_default`"),
        }
    }

    /// The default `i` of the local def `s`: its lifted getter called with the def's captures and the arguments
    /// before the default's clause (`args`, in the declared order), each written where the getter takes it, as
    /// scalac's caller passes them: the typer lifts an argument that is not a path into a val, and an inline
    /// parameter's tree expanded there (`Worker::own_getter_expansion`) is evaluated once, in its place, after
    /// captures that have no effect. The getter takes the parameters of the clauses before the default's in its class
    /// file's order (`Gen::default_getter_shape`; a right-associative extension's own clause before its receiver's),
    /// the arguments for them evaluated in the list's order (`Gen::reordered_operands`).
    fn lower_local_default(&mut self, s: SymId, i: usize, args: ListRef) -> LirId {
        if self.cx.fun_of_sym[s.idx()] == u32::MAX {
            self.unsupported("the default of a local def without a body");
            let (def, _) = self.local_fun_shape(s);
            let want = def.params.get(i).cloned().unwrap_or_else(ErasedType::object);
            let nothing = self.lir.push(Kind::Null, ErasedType::object());
            return self.lower_adapt(nothing, &ErasedType::object(), &want);
        }
        let (lifted, body) = self.local_default_shape(s, i);
        let (def, _) = self.local_fun_shape(s);
        let before = self.default_getter_shape(s, i).before;
        let types: Vec<ErasedType> = def.params[..before.min(def.params.len())].to_vec();
        let priors: Vec<TExprId> = self.cx.input.prog.expr_list(args).to_vec();
        // The declared index of the argument for each parameter the getter takes, in its order.
        let taken: Vec<usize> = match self.jvm_param_order(s) {
            Some(order) => order[..before.min(order.len())].to_vec(),
            None => (0..before.min(priors.len())).collect(),
        };
        if taken.len() != types.len() || taken.iter().any(|&k| k >= priors.len()) {
            self.unsupported("the default of a right-associative extension's receiver, whose getter takes the arguments of a clause the call gives after it");
            let nothing = self.lir.push(Kind::Null, ErasedType::object());
            return self.lower_adapt(nothing, &ErasedType::object(), &lifted.ret);
        }
        let mut listed = taken.clone();
        listed.sort_unstable();
        let items: Vec<TExprId> = listed.iter().map(|&k| priors[k]).collect();
        let order: Vec<usize> = taken.iter().map(|k| listed.iter().position(|x| x == k).expect("a taken argument, listed")).collect();
        let captures = self.lower_captures(lifted.instance.is_some(), body.home, lifted.key, None, &body.captures);
        let (holds, args) = self.reordered_operands(None, false, &items, &order, |g, item, j| g.lower_argument_at(item, &types[j]));
        let captures = self.lir.list(&captures);
        let args = self.lir.list(&args);
        let ret = lifted.ret.clone();
        let method = self.lifted_call_ref(Placed::Default(s, i), &lifted, (self.this_name.clone(), self.is_interface));
        let call = self.lir.push(Kind::LiftedCall { method, from: LiftedFrom::Default(i as u32), lifted, captures, args, order: None }, ret);
        if holds.is_empty() { call } else { self.lir.seq(&holds, call) }
    }

    /// The default of a parameter of a constructor of `c`, of the type `want`, over the arguments
    /// before it (`Gen::ctor_default_route`); `recv` is the receiver the typer selected the getter
    /// on.
    pub(super) fn lower_ctor_default(&mut self, c: ClassId, route: CtorDefault, recv: Option<TExprId>, args: &[LirId], want: &ErasedType) -> LirId {
        match route {
            CtorDefault::InPlace(Some(d)) => {
                let value = self.lower_expr(d, want);
                self.lir.push(Kind::Typed(value), want.clone())
            }
            CtorDefault::InPlace(None) => {
                let nothing = self.lir.push(Kind::Unit, ErasedType::V);
                self.lower_adapt(nothing, &ErasedType::V, want)
            }
            CtorDefault::LocalCompanion(v, getter) => {
                let module = ErasedType::L(getter.owner.clone());
                let receiver = match recv {
                    Some(r) => self.lower_expr(r, &module),
                    None => {
                        let local = match self.is_lazy_local(v) {
                            true => self.lower_lazy_call(v, false),
                            false => self.lower_local(v).expect("held by the scope, by the route"),
                        };
                        let t = self.lir.ty(local).clone();
                        self.lower_adapt(local, &t, &module)
                    }
                };
                self.lower_getter_call_at(receiver, None, getter, args, want)
            }
            CtorDefault::InnerCompanion(accessor, getter) => {
                let outer = ErasedType::L(accessor.owner.clone());
                let instance = match recv {
                    Some(r) => self.lower_expr(r, &outer),
                    None => {
                        let first = self.class_captured(c)[0];
                        let held = self.lower_held(first).expect("held by the scope, by the route");
                        let t = self.lir.ty(held).clone();
                        self.lower_adapt(held, &t, &outer)
                    }
                };
                let module = accessor.ret.clone();
                let none = self.lir.list(&[]);
                let companion = self.lir.push(Kind::Call { method: Rc::new(accessor), at: None, receiver: instance, args: none, order: None }, module);
                self.lower_getter_call_at(companion, None, getter, args, want)
            }
            CtorDefault::Captured(captures, getter) => {
                let mut nodes = Vec::new();
                // The enclosing instance the creation passes stands for the first capture.
                if let Some(r) = recv {
                    let t = if captures > 0 { getter.params[0].clone() } else { ErasedType::object() };
                    nodes.push(self.lower_expr(r, &t));
                }
                for s in self.class_captures_after(c, recv.is_some() as usize) {
                    nodes.push(self.lower_held(s).expect("held by the scope, by the route"));
                }
                let extras = self.class_extras(c);
                nodes.extend(self.lower_extras(&extras));
                nodes.extend_from_slice(args);
                self.lower_static_getter_call(&nodes, getter, want)
            }
            CtorDefault::Companion(getter) => {
                let module = getter.owner.clone();
                let receiver = self.lir.push(Kind::Module(module.clone()), ErasedType::L(module));
                self.lower_getter_call_at(receiver, None, getter, args, want)
            }
            CtorDefault::Jar(getter) if getter.kind == Invoke::Virtual => {
                let module = getter.owner.clone();
                let receiver = self.lir.push(Kind::Module(module.clone()), ErasedType::L(module));
                self.lower_getter_call_at(receiver, None, getter, args, want)
            }
            CtorDefault::Jar(getter) | CtorDefault::Static(getter) => {
                self.lower_static_getter_call(args, getter, want)
            }
        }
    }

    /// A getter called on `receiver` over `args`, named at `at` where that is the receiver's class
    /// and not the declaring one's, its result as the type `want`.
    fn lower_getter_call_at(&mut self, receiver: LirId, at: Option<(Rc<str>, bool)>, getter: MethodRef, args: &[LirId], want: &ErasedType) -> LirId {
        let ret = getter.ret.clone();
        let args = self.lir.list(args);
        self.lower_as(Kind::Call { method: Rc::new(getter), at, receiver, args, order: None }, &ret, want)
    }

    /// A getter's arguments in the order it takes them, each at its type, then as its parameter's.
    /// A getter's arguments read from `held` where it holds them.
    fn lower_getter_arguments(&mut self, args: &[GetterArgument], held: &[(TExprId, TempId, ErasedType)]) -> Vec<LirId> {
        args.iter()
            .map(|a| {
                let value = match held.iter().find(|h| h.0 == a.expr) {
                    Some((_, temp, ty)) => self.lir.kept(*temp, ty.clone()),
                    None => self.lower_expr(a.expr, &a.ty),
                };
                self.lower_adapt(value, &a.ty, &a.param)
            })
            .collect()
    }

    /// A static getter called over `nodes`, its result as the type `want`.
    fn lower_static_getter_call(&mut self, nodes: &[LirId], getter: MethodRef, want: &ErasedType) -> LirId {
        let ret = getter.ret.clone();
        let args = self.lir.list(nodes);
        self.lower_as(Kind::StaticCall { method: Rc::new(getter), args, order: None }, &ret, want)
    }

    /// A node that leaves a value of the type `from`, as the type `to`: of `to` itself where
    /// the two are the same type (a value class's mark is the identity of `to`).
    fn lower_as(&mut self, kind: Kind, from: &ErasedType, to: &ErasedType) -> LirId {
        if from == to {
            return self.lir.push(kind, to.clone());
        }
        let node = self.lir.push(kind, from.clone());
        self.lower_adapt(node, from, to)
    }

    /// A static method's call: no receiver.
    fn lower_static_call(&mut self, s: SymId, args: ListRef) -> LirId {
        let m = self.mref(s);
        let (args, order) = self.lower_arguments(Some(s), args, &m.params);
        let ret = m.ret.clone();
        self.lir.push(Kind::StaticCall { method: m, args, order }, ret)
    }

    /// The instance the method runs on; None where the method has none.
    pub(super) fn lower_this(&mut self) -> Option<LirId> {
        Some(match self.this_value()? {
            ThisValue::Module(module) => self.lir.push(Kind::Module(module.clone()), ErasedType::L(module)),
            ThisValue::Slot(ty) => self.lir.push(Kind::This, ty),
        })
    }

    /// The instance the method runs on, reported where the method has none.
    pub(super) fn lower_instance(&mut self) -> LirId {
        if let Some(this) = self.lower_this() {
            return this;
        }
        self.unsupported("`this` outside of an instance");
        let t = match self.this_class {
            Some(c) => self.class_type(c),
            None => ErasedType::object(),
        };
        self.lir.push(Kind::Null, t)
    }

    /// The instance as a lifted method takes it: a value class's method runs on the underlying
    /// value, and its lifted code on the box.
    fn lower_boxed_instance(&mut self, class: Option<ClassId>) -> LirId {
        let this = self.lower_instance();
        let (Some((_, u)), Some(c)) = (self.underlying(), class) else { return this };
        let held = self.lir.ty(this).clone();
        let value = self.lower_adapt(this, &held, &u);
        let boxed = self.class_type(c);
        self.lower_value_class(value, &u, c, EMPTY_LIST, false, &boxed, None)
    }

    /// The instance of the class `h` as the code holds it: an object's own, the instance the
    /// method runs on, the one the method was given, or the field of its instance that holds
    /// it; reported where the code holds none.
    pub(super) fn lower_instance_of(&mut self, h: ClassId) -> LirId {
        let t = self.class_type(h);
        if self.static_home(h) {
            let module = self.class_name(h);
            return self.lir.push(Kind::Module(module), t);
        }
        let current = self.m.this_home.or(self.this_class);
        if current == Some(h) && self.m.this_module.is_none() {
            // In a value class's method its companion holds, `this` is the underlying value: its box.
            if self.underlying().is_some() && self.m.this_home.is_none() {
                return self.lower_boxed_instance(Some(h));
            }
            let held = self.lower_instance();
            let from = self.lir.ty(held).clone();
            return self.lower_adapt(held, &from, &t);
        }
        if let Some(i) = self.param_of(&ParamRole::Enclosing(h)) {
            return self.param(i);
        }
        if let Some(k) = current.filter(|_| self.m.this_module.is_none()) {
            let held: Option<Rc<str>> = match self.m.this_home {
                None => self.role_field(&FieldRole::Enclosing(h)).map(|f| f.name.clone()),
                Some(_) => self.class_extras(k).homes.iter().position(|&c| c == h).map(|i| Rc::from(self.home_field(h, i))),
            };
            if let Some(name) = held {
                let owner = self.class_name(k);
                let receiver = self.lower_instance();
                return self.lir.push(Kind::Field { receiver, owner, name }, t);
            }
        }
        let name = self.cx.input.interner.get(self.cx.input.syms.class(h).name).to_string();
        self.unsupported(&format!("the instance of {} where the generated method has none", name));
        self.lir.push(Kind::Null, t)
    }

    /// What a closure captures of a local: what the scope holds for it (`Gen::lower_held`),
    /// reported where the scope holds nothing.
    pub(super) fn lower_capture(&mut self, s: SymId) -> LirId {
        if let Some(held) = self.lower_held(s) {
            return held;
        }
        if let Some((field, ty, held)) = self.captured_field(s) {
            let receiver = self.lower_instance();
            let t = match held {
                Held::Value => ty,
                Held::Cell => ErasedType::L(Rc::from(OBJECT_REF)),
            };
            let owner = self.this_name.clone();
            return self.lir.push(Kind::Field { receiver, owner, name: Rc::from(field) }, t);
        }
        let name = self.cx.input.interner.get(self.cx.input.syms.sym(s).name).to_string();
        self.unsupported(&format!("the local {} is not in scope of the generated method", name));
        self.lir.push(Kind::Null, ErasedType::object())
    }

    /// The values a lifted method takes before its parameters, in its order: the instance
    /// (`home`'s where that is another class than the code's own), the key (the one travelling
    /// with the holder of the lazy val `key_of` where the method is that val's accessor or
    /// initialiser), then each capture as the scope holds it.
    pub(super) fn lower_captures(&mut self, instance: bool, home: Option<ClassId>, key: bool, key_of: Option<SymId>, captures: &[Capture]) -> Vec<LirId> {
        let mut nodes = Vec::new();
        if instance {
            nodes.push(match home {
                Some(h) if Some(h) != self.m.this_home.or(self.this_class) => self.lower_instance_of(h),
                _ => self.lower_boxed_instance(self.this_class),
            });
        }
        if key {
            nodes.push(match key_of {
                Some(x) => self.lower_lazy_key(x),
                None => self.lower_return_key(),
            });
        }
        for c in captures {
            nodes.push(if c.key { self.lower_lazy_key(c.sym) } else { self.lower_capture(c.sym) });
        }
        nodes
    }

    /// The key that travels with the holder of the local lazy val `x`, as the code holds it: the parameter it was
    /// given (`ParamRole::Key`'s holder, `ParamRole::CapturedKey`), the field of the instance that holds it
    /// (`FieldRole::CapturedKey`) where the instance is the class's own, or in the code declaring the val the key of
    /// that code's own non-local returns (`Gen::lower_own_key`); reported where the code holds none.
    pub(super) fn lower_lazy_key(&mut self, x: SymId) -> LirId {
        let object = ErasedType::object();
        let given = self.planned.as_ref().and_then(|m| {
            m.params.iter().position(|p| match p.role {
                ParamRole::Key { holder } => holder == Some(x),
                ParamRole::CapturedKey(k) => k == x,
                _ => false,
            })
        });
        if let Some(i) = given {
            return self.param(i);
        }
        if self.m.this_home.is_none() && self.m.this_module.is_none() {
            if let Some(name) = self.role_field(&FieldRole::CapturedKey(x)).map(|f| f.name.clone()) {
                let owner = self.this_name.clone();
                let receiver = self.lower_instance();
                return self.lir.push(Kind::Field { receiver, owner, name }, object);
            }
        }
        if let Some(own) = self.lower_own_key() {
            return own;
        }
        self.unsupported("a non-local return of a lazy val whose method makes no key");
        self.lir.push(Kind::Null, object)
    }

    /// The key of the non-local returns the code throws (`lower_own_key`), reported where it holds none.
    fn lower_return_key(&mut self) -> LirId {
        if let Some(own) = self.lower_own_key() {
            return own;
        }
        self.unsupported("a non-local return of a def that makes no key");
        self.lir.push(Kind::Null, ErasedType::object())
    }

    /// The key of the non-local returns of the code being lowered: the one a closure (or a local lazy val's accessor
    /// or initialiser) was given, or the one the def's keyed body makes (`Gen::lower_keyed_body`).
    fn lower_own_key(&mut self) -> Option<LirId> {
        match (self.given_key(), self.body_key) {
            (Some(i), _) => Some(self.param(i)),
            (None, Some(key)) => Some(self.lir.push(Kind::Temp(key), ErasedType::object())),
            (None, None) => None,
        }
    }

    /// The planned method's parameter of the role `Key`: the key a closure, or a local lazy val's accessor or
    /// initialiser, is given.
    fn given_key(&self) -> Option<usize> {
        self.planned.as_ref()?.params.iter().position(|p| matches!(p.role, ParamRole::Key { .. }))
    }

    /// A function value: its body `code` lifted to a method over what it captures, named where it is lowered
    /// (`Gen::closure_entry`), and the instance of `scala.FunctionN` whose `apply` calls that method.
    pub(super) fn lower_lambda(&mut self, params: ListRef, code: TExprId) -> LirId {
        let cx = self.cx;
        let params: Vec<SymId> = cx.input.prog.sym_list(params).to_vec();
        let free = self.an.of_lambda(cx, &params, code);
        let object = ErasedType::object();
        let taken = vec![object.clone(); params.len()];
        let (lifted, body) = self.lifted_shape(&free, taken.clone(), object.clone(), None, Some((params, code)));
        let name = self.closure_entry(code, (lifted.clone(), body.clone()));
        let target = lifted_method_ref(name, &lifted, (self.this_name.clone(), self.is_interface));
        let captures = self.lower_captures(lifted.instance.is_some(), body.home, lifted.key, None, &body.captures);
        let ft = self.function_type(taken.len());
        let ErasedType::L(owner) = ft.clone() else { unreachable!("a function's class") };
        let desc = method_desc(&taken, &object);
        let method = Rc::new(MethodRef { owner, owner_is_interface: true, name: "apply".to_string(), params: taken, ret: object, desc, kind: Invoke::Interface });
        let captures = self.lir.list(&captures);
        self.lir.push(Kind::Closure { captures, method, target }, ft)
    }

    /// A call of a local def: its lifted method over what it captures, then the arguments; a right-associative
    /// extension's in its class file's order (`Gen::jvm_param_order`), each operand held as `Gen::reordered_operands`
    /// holds a local trait member's, the holds standing before the call.
    pub(super) fn lower_lifted_call(&mut self, s: SymId, args: ListRef) -> LirId {
        let items: Vec<TExprId> = self.cx.input.prog.expr_list(args).to_vec();
        let (lifted, body) = self.local_fun_shape(s);
        let captures = self.lower_captures(lifted.instance.is_some(), body.home, lifted.key, None, &body.captures);
        let reordered = self.jvm_param_order(s).filter(|o| o.len() == items.len() && o.len() == lifted.params.len());
        let (holds, args) = match reordered {
            Some(order) => {
                let params = lifted.params.clone();
                let (holds, nodes) = self.reordered_operands(None, false, &items, &order, |g, item, j| g.lower_argument_at(item, &params[j]));
                (holds, self.lir.list(&nodes))
            }
            None => (Vec::new(), self.lower_arguments(Some(s), args, &lifted.params).0),
        };
        let captures = self.lir.list(&captures);
        let ret = lifted.ret.clone();
        let method = self.lifted_call_ref(Placed::Def(s), &lifted, (self.this_name.clone(), self.is_interface));
        let call = self.lir.push(Kind::LiftedCall { method, from: LiftedFrom::Def, lifted, captures, args, order: None }, ret);
        if holds.is_empty() { call } else { self.lir.seq(&holds, call) }
    }

    /// A read of the local lazy val `s`, by its accessor.
    fn lower_lazy_read(&mut self, s: SymId) -> LirId {
        self.lower_lazy_call(s, false)
    }

    /// A call of the accessor (`init` false) or the initialiser of the local lazy val `s`: the
    /// values it takes before its holder (the instance, the key that travels with the holder, the
    /// captured ones), then the holder (`Gen::lazy_shape`).
    pub(super) fn lower_lazy_call(&mut self, s: SymId, init: bool) -> LirId {
        let (lifted, body) = self.lazy_shape(s);
        let captures = self.lower_captures(lifted.instance.is_some(), body.home, lifted.key, Some(s), &body.captures);
        let holder = self.lower_capture(s);
        let captures = self.lir.list(&captures);
        let args = self.lir.list(&[holder]);
        let (from, placed) = if init { (LiftedFrom::LazyInitialiser, Placed::LazyInit(s)) } else { (LiftedFrom::LazyAccessor, Placed::Def(s)) };
        let ret = lifted.ret.clone();
        let owner = self.lazy_owner(s);
        let method = self.lifted_call_ref(placed, &lifted, owner);
        self.lir.push(Kind::LiftedCall { method, from, lifted, captures, args, order: None }, ret)
    }

    /// The method a call of the lifted method of `placed` calls, of the class `owner` and of the shape `lifted` the
    /// lowering has: its name by the class's table (`Gen::lifted_entry`), a lookup that queues nothing.
    fn lifted_call_ref(&mut self, placed: Placed, lifted: &LiftedShape, owner: (Rc<str>, bool)) -> Rc<MethodRef> {
        let (name, _) = self.lifted_entry(placed, Asker::LiftedCallRef, lifted);
        lifted_method_ref(name, lifted, owner)
    }

    /// A member read on a receiver: its field where this class's own code may name it, else its
    /// accessor's call.
    fn lower_field_read(&mut self, r: TExprId, s: SymId) -> LirId {
        match self.field_route(r, s) {
            FieldRoute::Js => self.lower_refused(Refusal::JavaScriptMember),
            FieldRoute::Object(c) => self.lower_after_receiver(r, |g| g.lower_module(c)),
            FieldRoute::GivenObject { receiver_evaluated: true } => self.lower_after_receiver(r, |g| g.lower_given_object(s)),
            FieldRoute::GivenObject { receiver_evaluated: false } => self.lower_given_object(s),
            FieldRoute::Early => self.lower_local(s).expect("a local of the constructor, by the route"),
            FieldRoute::Java => {
                let Some((cf, JMember::Field(i))) = self.cx.input.java_member(s) else { unreachable!("a Java field, by the route") };
                let f = &cf.fields[i as usize];
                let t = parse_type(&f.descriptor).0;
                let name: Rc<str> = Rc::from(f.name.as_str());
                // A static's receiver is not evaluated.
                if f.access & ACC_STATIC != 0 {
                    return self.lir.push(Kind::StaticField { owner: Rc::from(cf.name.as_str()), name }, t);
                }
                let (receiver, owner) = self.lower_java_qualifier(r, s, &cf.name);
                self.lir.push(Kind::Field { receiver, owner, name }, t)
            }
            // `missing[V].u` is the box's accessor on the reference, as scalac casts the qualifier
            // to the class and selects (`Erasure.scala` 733-737): a null throws. A block's result
            // is the qualifier, and so is the branch an `if` over a constant takes (dotty's
            // `FirstTransform` leaves that branch alone before erasure), where a conditional's, a
            // match's or a `try`'s branches are each adapted to the value class, a null to its zero.
            FieldRoute::Underlying(c) => {
                let u = self.vc_underlying(c);
                let operand = self.selected_operand(r);
                let outer = self.vc_select.replace(operand);
                let value = self.lower_expr(r, &u);
                self.vc_select = outer;
                self.lir.push(Kind::Underlying { value }, u)
            }
            FieldRoute::Own => {
                let ty = self.sym_type(s);
                let name: Rc<str> = Rc::from(self.field_name(s));
                let receiver = self.lower_this().expect("an instance, by `Gen::own_field`");
                let owner = self.this_name.clone();
                self.lir.push(Kind::Field { receiver, owner, name }, ty)
            }
            FieldRoute::PrivateParam(c) => {
                let ty = self.sym_type(s);
                let name: Rc<str> = Rc::from(self.field_name(s));
                let owner = self.class_name(c);
                let receiver = self.lower_expr(r, &ErasedType::L(owner.clone()));
                self.lir.push(Kind::Field { receiver, owner, name }, ty)
            }
            FieldRoute::Accessor => {
                let s = self.deferred_implementation(r, s).unwrap_or(s);
                let m = self.mref(s);
                let site = self.invoke_site(r, s, &m);
                let receiver = self.lower_receiver(r, s, &site);
                let args = self.lir.list(&[]);
                let ret = m.ret.clone();
                self.lir.push(Kind::Call { method: m, at: site.map(|site| (site.owner, site.interface)), receiver, args, order: None }, ret)
            }
        }
    }

    /// The receiver `r` evaluated for its effect, then the value `selected` makes, which does not
    /// read it: an object or a given object selected on it.
    fn lower_after_receiver(&mut self, r: TExprId, selected: impl FnOnce(&mut Self) -> LirId) -> LirId {
        let effect = self.lower_expr(r, &ErasedType::V);
        let value = selected(self);
        self.lir.seq(&[effect], value)
    }

    fn lower_receiver(&mut self, r: TExprId, s: SymId, site: &Option<Site>) -> LirId {
        let t = self.receiver_type(s, site);
        self.lower_expr(r, &t)
    }

    /// The arguments of a call of `callee` against its parameters' types, as they are written,
    /// and the order a right-associative extension takes them in (`Gen::jvm_param_order`).
    pub(super) fn lower_arguments(&mut self, callee: Option<SymId>, args: ListRef, params: &[ErasedType]) -> (List, Option<Rc<[usize]>>) {
        let order = callee.and_then(|s| self.jvm_param_order(s)).filter(|o| o.len() == params.len());
        let written = match &order {
            Some(order) => self.signature_order(order, params),
            None => params.to_vec(),
        };
        let items: Vec<TExprId> = self.cx.input.prog.expr_list(args).to_vec();
        let mut lowered = Vec::with_capacity(items.len());
        for (i, a) in items.into_iter().enumerate() {
            let want = written.get(i).cloned().unwrap_or_else(ErasedType::object);
            lowered.push(self.lower_argument_at(a, &want));
        }
        (self.lir.list(&lowered), order.map(Rc::from))
    }

    /// An argument for a parameter that takes an array of the class `array` (a Java varargs or
    /// array parameter). The elements of a varargs literal are a new array of the parameter's
    /// element class. An array spliced in (`Stream.of(arr*)`) is passed itself, as scalac passes
    /// it, where its class is the parameter's (`Gen::spliced_array`); another sequence
    /// (`Stream.of(xs*)`) is copied into an array of the parameter's element class, as scalac's
    /// `scala.runtime.Arrays.seqToArray` does; any other value is the array.
    fn lower_array_argument(&mut self, a: TExprId, array: &Rc<str>) -> LirId {
        let want = ErasedType::L(array.clone());
        if let Some(spliced) = self.spliced_array(a, array) {
            let t = self.static_type(spliced);
            let value = self.lower_expr(spliced, &t);
            return self.lower_adapt(value, &t, &want);
        }
        if let TExpr::SeqLit(items) = self.cx.input.prog.expr(a) {
            return self.lower_array(items, array);
        }
        let t = self.static_type(a);
        let value = self.lower_expr(a, &t);
        if !matches!(&t, ErasedType::L(name) if !name.starts_with('[') && &**name != OBJECT) {
            return self.lower_adapt(value, &t, &want);
        }
        let (seq, class, object) = (ErasedType::L(Rc::from(SEQ)), ErasedType::L(Rc::from("java/lang/Class")), ErasedType::object());
        let sequence = self.lower_adapt(value, &t, &seq);
        let element = self.lir.push(Kind::ClassOf(parse_type(&array[1..]).0), class.clone());
        let args = self.lir.list(&[sequence, element]);
        let params = vec![seq, class];
        let desc = method_desc(&params, &object);
        let method = Rc::new(MethodRef { owner: Rc::from("scala/runtime/Arrays"), owner_is_interface: false, name: "seqToArray".to_string(), params, ret: object.clone(), desc, kind: Invoke::Static });
        let copied = self.lir.push(Kind::StaticCall { method, args, order: None }, object.clone());
        self.lower_adapt(copied, &object, &want)
    }

    /// A call `e` of a member on a receiver.
    fn lower_call_method(&mut self, e: TExprId, r: TExprId, s: SymId, args: ListRef) -> LirId {
        match self.call_route(r, s, args) {
            CallRoute::Plain => {}
            // The receiver of a static is not evaluated.
            CallRoute::JavaStatic | CallRoute::JdkStatic => return self.lower_static_call(s, args),
            CallRoute::Spread(array) => return self.lower_wrapped_spread(array, Some(r)),
            CallRoute::Super => return self.lower_super_call(e, r, s, args),
            CallRoute::Interpolator(helper) => return self.lower_interpolator(r, s, helper, args),
            CallRoute::Proxied(t) => return self.lower_proxied_call(t, s, args),
            CallRoute::Lifted => return self.lower_lifted_member_call(r, s, args),
            CallRoute::Js => return self.lower_refused(Refusal::JavaScriptMember),
            CallRoute::Extension(c, route) => return self.lower_extension_call(r, s, args, c, route),
            CallRoute::Object(c) => return self.lower_after_receiver(r, |g| g.lower_module(c)),
        }
        let m = self.mref(s);
        let site = self.invoke_site(r, s, &m);
        let receiver = self.lower_receiver(r, s, &site);
        let (args, order) = self.lower_arguments(Some(s), args, &m.params);
        let ret = m.ret.clone();
        self.lir.push(Kind::Call { method: m, at: site.map(|site| (site.owner, site.interface)), receiver, args, order }, ret)
    }

    /// The parameters `params` of the def `s` in its class file's order (`Gen::jvm_param_order`): a
    /// right-associative extension method's own clause before the receiver's, which its class file's
    /// slots hold in that order (`Gen::emit_fun`, `Gen::extension_bodies`); else as they are.
    pub(super) fn class_file_params(&self, s: SymId, params: &[SymId]) -> Vec<SymId> {
        match self.jvm_param_order(s) {
            Some(order) if order.len() == params.len() => order.iter().map(|&k| params[k]).collect(),
            _ => params.to_vec(),
        }
    }

    /// `r.s(args)` of a method of the value class `c`, the method of its companion's instance
    /// over the underlying value (`Gen::extension_method`): the instance first, then the receiver
    /// in place; or, for a generic class's receiver evaluated before the instance, the receiver
    /// held in a temporary of a block that ends with the call (dotty's `evalOnce`, as
    /// `VCInlineMethods.rewire` leaves the receiver: in place, so that the companion is initialised
    /// before its effects, unless `emit::receiver_first`). A right-associative method takes its
    /// arguments in its class file's order (`Gen::jvm_param_order`), which the typed IR evaluates in
    /// its own (`Gen::reordered_operands`).
    /// The class's own static method, where this build writes no companion, takes the receiver
    /// through a temporary (`Gen::lower_static_extension_call`).
    fn lower_extension_call(&mut self, r: TExprId, s: SymId, args: ListRef, c: ClassId, route: ExtensionRoute) -> LirId {
        let cx = self.cx;
        if route == ExtensionRoute::Static {
            return self.lower_static_extension_call(r, s, args, c);
        }
        let method = Rc::new(self.extension_method(s, c, route));
        let u = self.vc_underlying(c);
        let receiver_first = crate::emit::receiver_first(cx.input.prog, cx.input.syms, c, r);
        if let Some((items, order)) = self.extension_order(s, args) {
            let receiver = self.lir.push(Kind::Module(method.owner.clone()), ErasedType::L(method.owner.clone()));
            let mut value = self.lower_expr(r, &u);
            if items.iter().any(|&e| self.reads_initialising_object(e)) {
                return self.extension_call_in_list_order(method, receiver, value, &u, receiver_first, &items, &order);
            }
            let (holds, rest) = self.reordered_operands(Some((r, &mut value)), receiver_first, &items, &order, |g, item, j| g.lower_argument_at(item, &method.params[1 + j]));
            let mut nodes = vec![self.lower_adapt(value, &u, &method.params[0])];
            nodes.extend(rest);
            let args = self.lir.list(&nodes);
            let ret = method.ret.clone();
            let call = self.lir.push(Kind::Call { method, at: None, receiver, args, order: None }, ret);
            return if holds.is_empty() { call } else { self.lir.seq(&holds, call) };
        }
        let held = receiver_first.then(|| {
            let init = self.lower_expr(r, &u);
            let temp = self.lir.temp();
            (temp, self.lir.keep(temp, init))
        });
        let receiver = self.lir.push(Kind::Module(method.owner.clone()), ErasedType::L(method.owner.clone()));
        let value = match held {
            Some((temp, _)) => self.lir.kept(temp, u.clone()),
            None => self.lower_expr(r, &u),
        };
        let mut nodes = vec![self.lower_adapt(value, &u, &method.params[0])];
        self.lower_rest(s, args, 0, &method.params[1..], &mut nodes);
        let args = self.lir.list(&nodes);
        let ret = method.ret.clone();
        let call = self.lir.push(Kind::Call { method, at: None, receiver, args, order: None }, ret);
        match held {
            Some((_, keep)) => self.lir.seq(&[keep], call),
            None => call,
        }
    }

    /// Whether the operand `e` is a path that reads an object whose first reference may run code
    /// (`Gen::operand_has_effect` of a path: `Gen::object_inits` other than quiet).
    fn reads_initialising_object(&self, e: TExprId) -> bool {
        self.default_operand_is_path(e) && self.operand_has_effect(e)
    }

    /// A right-associative call of a value class's extension through its companion where an operand
    /// reads an object with initialisers: the companion's instance, then the operands in the typed
    /// IR's order, each passed to the parameter that takes it (`Call`'s `order`), no operand held
    /// ahead of the call, as the old generator orders them. A hold there is right for an infix call
    /// and wrong for a written-out one, which the typed IR does not tell apart until the typer swaps
    /// the clauses at definition; that swap replaces this (docs/NATIVE.md, "A right-associative
    /// call's operands on the lowered path").
    fn extension_call_in_list_order(&mut self, method: Rc<MethodRef>, receiver: LirId, value: LirId, u: &ErasedType, receiver_first: bool, items: &[TExprId], order: &[usize]) -> LirId {
        let (holds, value) = if receiver_first {
            let temp = self.lir.temp();
            (vec![self.lir.keep(temp, value)], self.lir.kept(temp, u.clone()))
        } else {
            (Vec::new(), value)
        };
        let mut place = vec![0usize; items.len()];
        for (j, &k) in order.iter().enumerate() {
            place[k] = j;
        }
        let mut nodes = vec![self.lower_adapt(value, u, &method.params[0])];
        for (i, &item) in items.iter().enumerate() {
            nodes.push(self.lower_argument_at(item, &method.params[1 + place[i]]));
        }
        let args = self.lir.list(&nodes);
        let call_order: Rc<[usize]> = std::iter::once(0).chain(order.iter().map(|&k| 1 + k)).collect();
        let ret = method.ret.clone();
        let call = self.lir.push(Kind::Call { method, at: None, receiver, args, order: Some(call_order) }, ret);
        if holds.is_empty() { call } else { self.lir.seq(&holds, call) }
    }

    /// `r.s(args)` of a method of the value class `c` whose companion this build does not write
    /// (`Cx::vc_on_companion`): the class's own static `m$extension`, the receiver's underlying value
    /// held in a temporary, then the arguments.
    fn lower_static_extension_call(&mut self, r: TExprId, s: SymId, args: ListRef, c: ClassId) -> LirId {
        let method = Rc::new(self.extension_method(s, c, ExtensionRoute::Static));
        let u = self.vc_underlying(c);
        let mut value = self.lower_expr(r, &u);
        let (holds, rest) = match self.extension_order(s, args) {
            Some((items, order)) => self.reordered_operands(Some((r, &mut value)), true, &items, &order, |g, item, j| g.lower_argument_at(item, &method.params[1 + j])),
            None => {
                let temp = self.lir.temp();
                let keep = self.lir.keep(temp, value);
                value = self.lir.kept(temp, u.clone());
                let mut rest = Vec::new();
                self.lower_rest(s, args, 0, &method.params[1..], &mut rest);
                (vec![keep], rest)
            }
        };
        let mut nodes = vec![value];
        nodes.extend(rest);
        let args = self.lir.list(&nodes);
        let ret = method.ret.clone();
        let call = self.lir.push(Kind::StaticCall { method, args, order: None }, ret);
        self.lir.seq(&holds, call)
    }

    /// The arguments of a right-associative extension method `s` and the order its class file takes
    /// them in (`Gen::jvm_param_order`); none for another method.
    fn extension_order(&self, s: SymId, args: ListRef) -> Option<(Vec<TExprId>, Vec<usize>)> {
        let items: Vec<TExprId> = self.cx.input.prog.expr_list(args).to_vec();
        let order = self.jvm_param_order(s).filter(|o| o.len() == items.len())?;
        Some((items, order))
    }

    /// An argument at the type of the parameter that takes it.
    fn lower_argument_at(&mut self, a: TExprId, want: &ErasedType) -> LirId {
        match want {
            ErasedType::L(array) if array.starts_with('[') => self.lower_array_argument(a, array),
            _ => self.lower_expr(a, want),
        }
    }

    /// Whether evaluating the operand `e` may have an effect, as dotty's `isPureExpr` answers it (`exprPurity`
    /// below `Pure`, TreeInfo.scala 589-696 at 3.8.4): what is no path (`Gen::default_operand_is_path`), and a path
    /// through an object whose first reference may run code or of which nothing says, a module reference or an
    /// object's term (`Gen::object_inits`); a selection's purity is its prefix's at most.
    fn operand_has_effect(&self, e: TExprId) -> bool {
        if !self.default_operand_is_path(e) {
            return true;
        }
        let cx = self.cx;
        let loads = |s: SymId| matches!(cx.input.syms.sym(s).kind, SymKind::Object(c) if self.object_inits(c) != ObjectInits::Quiet);
        match cx.input.prog.expr(e) {
            TExpr::Module(c) => self.object_inits(c) != ObjectInits::Quiet,
            TExpr::Local(s) | TExpr::Static(s) => loads(s),
            TExpr::Field(q, s) | TExpr::CallMethod(q, s, _) => loads(s) || self.operand_has_effect(q),
            _ => false,
        }
    }

    /// Whether the operand `e`, a path, may throw (a read through a receiver that may be null): the back end's own
    /// complement of dotty's purity for its reordering (docs/NATIVE.md, "A right-associative call's operands").
    fn operand_may_throw(&self, e: TExprId) -> bool {
        let prog = self.cx.input.prog;
        match prog.expr(e) {
            TExpr::Field(q, _) | TExpr::CallMethod(q, _, _) => !matches!(prog.expr(q), TExpr::This | TExpr::Module(_)) || self.operand_may_throw(q),
            _ => false,
        }
    }

    /// What the first reference of the object of class `c` runs, for every asker that orders an operand around it:
    /// `typer::elaborate::no_inits` (`refPurity`: an object that is no `isNoInitsRealClass` is an `IdempotentPath`)
    /// where the class is completed; `NotRecorded` for a class-path object the typer never completed, which has
    /// neither its pickle's `NoInits` nor its record read and no base types, over which `no_inits` would answer from
    /// empty tables. An asker takes `NotRecorded` as `Initialises`, never guessing it quiet.
    pub(super) fn object_inits(&self, c: ClassId) -> ObjectInits {
        let cx = self.cx;
        if cx.input.syms.class(c).base_types.is_empty() {
            ObjectInits::NotRecorded
        } else if crate::typer::elaborate::no_inits(cx.input.syms, cx.input.interner, c) {
            ObjectInits::Quiet
        } else {
            ObjectInits::Initialises
        }
    }

    /// The operands of a call that evaluates `ahead` (its receiver, the expression and its lowered
    /// value) before its arguments `items`, which it takes in the order `order` (`order[j]`, the item
    /// of its `j`-th argument), each lowered by `lower` for its place: the holds that stand before the
    /// call, then the arguments in the call's order. The typed IR's list is the order of the effects,
    /// whatever the typer binds ahead of the call: an operand scalac's typer lifts there (dotty's
    /// `Typer.typedInfixOp` and `LiftImpure.liftArgs`, Typer.scala 3696-3720 and EtaExpansion.scala
    /// 100-108 at 3.8.4, as `Worker::lift_impure`) or a written-out call's operand. An item that has an
    /// effect (`Gen::operand_has_effect`) is held in a temporary where one that comes after it in the list
    /// and has an effect or may throw (`Gen::operand_may_throw`) is taken before it, and so is each such
    /// item before that one, so that the effects keep the list's order and none is lost to an exception
    /// the list raises after it; any other is read in its place. `ahead` is held first where `hold_ahead`
    /// asks it, or where an item is held and it is no path.
    pub(super) fn reordered_operands(&mut self, ahead: Option<(TExprId, &mut LirId)>, hold_ahead: bool, items: &[TExprId], order: &[usize], mut lower: impl FnMut(&mut Self, TExprId, usize) -> LirId) -> (Vec<LirId>, Vec<LirId>) {
        let n = items.len();
        let mut place = vec![0usize; n];
        for (j, &k) in order.iter().enumerate() {
            place[k] = j;
        }
        let effect: Vec<bool> = items.iter().map(|&e| self.operand_has_effect(e)).collect();
        let first = |k: usize| effect[k] || self.operand_may_throw(items[k]);
        let last = (0..n).filter(|&i| effect[i] && (i + 1..n).any(|k| place[k] < place[i] && first(k))).max();
        let mut holds = Vec::new();
        if let Some((r, value)) = ahead {
            if hold_ahead || last.is_some() && !self.default_operand_is_path(r) {
                let temp = self.lir.temp();
                let ty = self.lir.ty(*value).clone();
                holds.push(self.lir.keep(temp, *value));
                *value = self.lir.kept(temp, ty);
            }
        }
        let mut held: Vec<Option<LirId>> = vec![None; n];
        for i in (0..n).filter(|&i| effect[i] && last.is_some_and(|m| i <= m)) {
            let value = lower(self, items[i], place[i]);
            let temp = self.lir.temp();
            let ty = self.lir.ty(value).clone();
            holds.push(self.lir.keep(temp, value));
            held[i] = Some(self.lir.kept(temp, ty));
        }
        let args = order.iter().enumerate().map(|(j, &k)| match held[k] {
            Some(read) => read,
            None => lower(self, items[k], j),
        }).collect();
        (holds, args)
    }

    /// Whether `name` with descriptor `desc` is Object's `toString()`, `hashCode()` or
    /// `equals(Object)` as the Java interface `o` declares it. No interface holds a body for one of
    /// them (an interface's default cannot override Object's public methods), so a super call of
    /// it, written or through a trait's accessor, takes the body from the class chain and names the
    /// class's superclass. A written call names it as scalac's does (`java/util/ArrayList.toString`
    /// for an `ArrayList`'s `toString`, which the std declares on its `Collection`); a trait's
    /// accessor names it where scalac names the declaring class, `java/util/AbstractCollection`,
    /// which the std's model of the class does not hold: the JVM resolves both to the one method.
    pub(super) fn object_member_on_java_interface(&self, o: ClassId, name: &str, desc: &str) -> bool {
        self.java_interface(o) && matches!((name, desc), ("toString", "()Ljava/lang/String;") | ("hashCode", "()I") | ("equals", "(Ljava/lang/Object;)Z"))
    }

    /// `super.s(args)`, the call `e`: through the accessor a class that mixes the trait in
    /// implements where `super` is a trait's, else the member of the superclass or of the trait
    /// named, called as that class's own. A Java interface's method is called on the interface
    /// itself, whatever brings it: the receiver of a super call is the method's owner
    /// (`BCodeBodyBuilder.genCallMethod`), which the class then lists (`superCallTargets`); one of
    /// Object's three there names the superclass (`object_member_on_java_interface`).
    pub(super) fn lower_super_call(&mut self, e: TExprId, r: TExprId, s: SymId, args: ListRef) -> LirId {
        let cx = self.cx;
        let TExpr::Super(target) = cx.input.prog.expr(r) else { unreachable!("a super call, by the route") };
        let m = self.mref(s);
        let java = match cx.input.syms.sym(s).owner {
            Owner::Class(o) if self.java_interface(o) => Some(o),
            _ => None,
        };
        let owner = match target {
            SuperTarget::Mixin(t) => {
                let accessor = Rc::new(self.super_accessor_ref(t, s));
                let receiver = self.lower_receiver(r, s, &None);
                let (args, order) = self.lower_arguments(Some(s), args, &accessor.params);
                let ret = accessor.ret.clone();
                return self.lir.push(Kind::Call { method: accessor, at: None, receiver, args, order }, ret);
            }
            SuperTarget::Chain if java.is_some_and(|o| self.object_member_on_java_interface(o, &m.name, &m.desc)) => {
                self.this_class.and_then(|c| cx.input.syms.class(c).superclass)
            }
            _ if java.is_some() => java,
            SuperTarget::Chain => self.this_class.and_then(|c| cx.input.syms.class(c).superclass),
            SuperTarget::Class(c) => Some(c),
        };
        let method = match owner {
            Some(c) if !matches!(target, SuperTarget::Class(_)) || cx.input.syms.class(c).kind == ClassKind::Trait => {
                let mut special = (*m).clone();
                special.owner = self.class_name(c);
                special.owner_is_interface = cx.input.syms.class(c).kind == ClassKind::Trait;
                special.kind = Invoke::Special;
                Rc::new(special)
            }
            _ => {
                self.unsupported("not supported on the JVM yet: this super call");
                m
            }
        };
        // The receiver is the instance at its own class, as the verifier tracks it and `invokespecial` requires it
        // (JVMS 4.10.1.9), where a frame inside the arguments (a join, the code after a `return` or a `throw`) stands
        // over it; as `lower_object_super_call` takes it. A value class's extension method has no instance of its class
        // (`this` is the underlying value): its receiver stays the method's owner's.
        let receiver = if self.underlying().is_some() { self.lower_receiver(r, s, &None) } else { self.lower_instance() };
        let (args, order) = self.lower_arguments(Some(s), args, &method.params);
        let place = cx.input.prog.span_of(e).map(|(file, span)| (file, span.end));
        let ret = method.ret.clone();
        self.lir.push(Kind::SuperCall { method, receiver, args, order, place }, ret)
    }

    /// `super[T].s(args)` of a member of the local trait `t` that takes proxies, as scalac's
    /// forwarder calls it: the trait's static `s$` over the instance, the proxies as this code
    /// holds them, their extras, then the arguments (`Gen::lower_member_rest`).
    pub(super) fn lower_proxied_call(&mut self, t: ClassId, s: SymId, args: ListRef) -> LirId {
        let proxies = self.member_proxies(s).expect("a member that takes proxies, by the route");
        let declared = self.proxied_declared(s);
        let m = self.mref(s);
        let owner = self.class_name(t);
        let extras = self.extras_of(&declared, None);
        let pm = self.proxied_mref(&m, &proxies, &extras);
        let items: Vec<TExprId> = self.cx.input.prog.expr_list(args).to_vec();
        let k = declared.len();
        let mut nodes = vec![self.lower_instance()];
        nodes.extend(self.lower_proxy_arguments(&proxies, &declared, &items));
        let held = self.lower_member_rest(s, args, k, &m.params, None, &mut nodes);
        let mut params = vec![ErasedType::L(owner.clone())];
        params.extend(pm.params.iter().cloned());
        let desc = method_desc(&params, &pm.ret);
        let ret = pm.ret.clone();
        let method = Rc::new(MethodRef { owner, owner_is_interface: true, name: format!("{}$", pm.name), params, ret: ret.clone(), desc, kind: Invoke::Static });
        let args = self.lir.list(&nodes);
        let call = self.lir.push(Kind::StaticCall { method, args, order: None }, ret);
        if held.is_empty() { call } else { self.lir.seq(&held, call) }
    }

    /// `r.s(proxies, args)` of a local trait's lifted member, as dotty's `LambdaLift.addFreeArgs`
    /// leaves it: the interface's method over the proxies, their extras and the parameters
    /// (`Gen::lower_member_rest`), at the receiver's site. Each proxy's argument is
    /// what a lifted class would be handed for it (`Gen::capture_argument_route`): a local's read,
    /// or `this` where the proxy stands for an enclosing instance. On `super`, which binds to the
    /// trait's member where the class mixing it in is declared after the one calling (it has no
    /// forwarder yet), the trait's static `s$`, as scalac calls it.
    pub(super) fn lower_lifted_member_call(&mut self, r: TExprId, s: SymId, args: ListRef) -> LirId {
        let cx = self.cx;
        if matches!(cx.input.prog.expr(r), TExpr::Super(_)) {
            let Owner::Class(t) = cx.input.syms.sym(s).owner else { unreachable!("a local trait's member, by the route") };
            return self.lower_proxied_call(t, s, args);
        }
        let declared = self.proxied_declared(s);
        let items: Vec<TExprId> = cx.input.prog.expr_list(args).to_vec();
        let k = declared.len();
        let proxies = self.member_proxies(s).expect("a member that takes proxies, by the route");
        let m = self.mref(s);
        let extras = self.extras_of(&declared, None);
        let pm = self.proxied_mref(&m, &proxies, &extras);
        let site = self.invoke_site(r, s, &m);
        let mut receiver = self.lower_receiver(r, s, &site);
        let mut nodes = Vec::new();
        for &p in &proxies {
            let want = self.anon_capture_type(p);
            nodes.push(match declared.iter().position(|&d| d == p).and_then(|i| items.get(i).copied()) {
                Some(a) => {
                    let route = self.capture_argument_route(a, p);
                    let value = self.lower_capture_argument(a, route);
                    let t = self.lir.ty(value).clone();
                    self.lower_adapt(value, &t, &want)
                }
                None => self.lower_capture_as(p, &want),
            });
        }
        nodes.extend(self.lower_extras(&extras));
        let held = self.lower_member_rest(s, args, k, &m.params, Some((r, &mut receiver)), &mut nodes);
        let ret = pm.ret.clone();
        let args = self.lir.list(&nodes);
        let call = self.lir.push(Kind::Call { method: Rc::new(pm), at: site.map(|site| (site.owner, site.interface)), receiver, args, order: None }, ret);
        if held.is_empty() { call } else { self.lir.seq(&held, call) }
    }

    /// The values a local trait's member or initialiser takes first: each of its `proxies` as the
    /// call's own argument for it among `given` (by `declared`'s order) where that is no read of
    /// the proxy itself, else as this code holds the proxy; then their extras.
    fn lower_proxy_arguments(&mut self, proxies: &[SymId], declared: &[SymId], given: &[TExprId]) -> Vec<LirId> {
        let mut nodes = Vec::with_capacity(proxies.len());
        for &p in proxies {
            let want = self.anon_capture_type(p);
            nodes.push(match declared.iter().position(|&d| d == p).and_then(|i| given.get(i).copied()) {
                Some(a) if !matches!(self.cx.input.prog.expr(a), TExpr::Local(x) if x == p) => self.lower_expr(a, &want),
                _ => self.lower_capture_as(p, &want),
            });
        }
        let extras = self.extras_of(declared, None);
        nodes.extend(self.lower_extras(&extras));
        nodes
    }

    /// `T.$init$(this, proxies)` of the local trait `b`, whose body reads `proxies`, values of
    /// the block (`TClass::captures`), in the constructor of a class that mixes it in, `given`
    /// the class's own arguments for them.
    pub(super) fn lower_proxied_trait_init(&mut self, b: ClassId, this: &ErasedType, proxies: &[SymId], declared: &[SymId], given: &[TExprId]) -> LirId {
        let base = self.class_name(b);
        let mut types = vec![ErasedType::L(base.clone())];
        for &p in proxies {
            types.push(self.anon_capture_type(p));
        }
        let extras = self.extras_of(declared, None);
        types.extend(self.extras_types(&extras));
        let mut args = vec![self.lir.this(this.clone())];
        args.extend(self.lower_proxy_arguments(proxies, declared, given));
        let init = method_ref(Invoke::Static, &base, true, "$init$", &types, &ErasedType::V);
        self.lir.static_call(&init, &args)
    }

    /// The captures a local trait's member that takes proxies declares first.
    fn proxied_declared(&self, s: SymId) -> Vec<SymId> {
        let fun = self.proxied_fun(s).expect("a member that takes proxies, by the route");
        fun.params[..fun.captures as usize].to_vec()
    }

    /// What the scope holds for the local `s` (`lower_capture`) as a value of the type `want`.
    pub(super) fn lower_capture_as(&mut self, s: SymId, want: &ErasedType) -> LirId {
        let held = self.lower_capture(s);
        let t = self.lir.ty(held).clone();
        self.lower_adapt(held, &t, want)
    }

    /// The `Extras` `e` as the code holds them.
    pub(super) fn lower_extras(&mut self, e: &Extras) -> Vec<LirId> {
        let mut nodes: Vec<LirId> = e.homes.iter().map(|&h| self.lower_instance_of(h)).collect();
        nodes.extend(e.keys.iter().map(|&x| self.lower_lazy_key(x)));
        nodes
    }

    /// The arguments after the first `k` (a local trait's member's proxies) of a call of `s` at `params`, onto
    /// `nodes`, and the holds that stand before the call: a right-associative extension's in its class file's order
    /// (`Gen::jvm_param_order`), each operand the order passes held as `Gen::reordered_operands` holds it, the call's
    /// receiver `ahead` with them; any other's as written (`Gen::lower_rest`).
    fn lower_member_rest(&mut self, s: SymId, args: ListRef, k: usize, params: &[ErasedType], ahead: Option<(TExprId, &mut LirId)>, nodes: &mut Vec<LirId>) -> Vec<LirId> {
        let items: Vec<TExprId> = self.cx.input.prog.expr_list(args).iter().skip(k).copied().collect();
        let Some(order) = self.jvm_param_order(s).filter(|o| o.len() == params.len() && o.len() == items.len()) else {
            self.lower_rest(s, args, k, params, nodes);
            return Vec::new();
        };
        let (holds, rest) = self.reordered_operands(ahead, false, &items, &order, |g, item, j| g.lower_argument_at(item, &params[j]));
        nodes.extend(rest);
        holds
    }

    /// The arguments after the first `k` of a call of `s` at `params`, onto `nodes`.
    fn lower_rest(&mut self, s: SymId, args: ListRef, k: usize, params: &[ErasedType], nodes: &mut Vec<LirId>) {
        let rest = ListRef { start: args.start + k as u32, len: args.len.saturating_sub(k as u32) };
        let (rest, _) = self.lower_arguments(Some(s), rest, params);
        nodes.extend_from_slice(&self.lir.lists[rest.start as usize..(rest.start + rest.len) as usize]);
    }

    /// `context.s(args)` of scala-library, which has no bytecode: the runtime's `helper`, which
    /// lowers it as scalac does, applied to the context and the arguments at its parameters.
    fn lower_interpolator(&mut self, r: TExprId, s: SymId, helper: SymId, args: ListRef) -> LirId {
        let method = self.mref(helper);
        let params = method.params.clone();
        let context = self.lower_expr(r, &params[0]);
        let (args, None) = self.lower_arguments(Some(s), args, &params[1..]) else { unreachable!("an interpolator takes its arguments as written") };
        let mut items = vec![context];
        items.extend_from_slice(&self.lir.lists[args.start as usize..(args.start + args.len) as usize]);
        let args = self.lir.list(&items);
        let ret = method.ret.clone();
        self.lir.push(Kind::Helper { method, args }, ret)
    }

    /// `value`, of the type `from`, as a value of the type `to`.
    pub(super) fn lower_adapt(&mut self, value: LirId, from: &ErasedType, to: &ErasedType) -> LirId {
        if from == to {
            return value;
        }
        let how = self.adaptation(from, to);
        self.lir.push(Kind::Adapt { value, from: from.clone(), how }, to.clone())
    }
}

/// The static method of the class `owner` (an interface where `owner_is_interface`) named `name`, of the shape `lifted`:
/// the lifted method a closure or a call names.
fn lifted_method_ref(name: String, lifted: &LiftedShape, (owner, owner_is_interface): (Rc<str>, bool)) -> Rc<MethodRef> {
    let params = lifted.takes();
    let desc = method_desc(&params, &lifted.ret);
    Rc::new(MethodRef { owner, owner_is_interface, name, params, ret: lifted.ret.clone(), desc, kind: Invoke::Static })
}
