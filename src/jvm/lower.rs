//! The platform lowering: an expression of the typed IR as nodes of the lowered IR
//! (`crate::lir`). A lowering decides and builds nodes; it writes nothing, and the code and the
//! constant pool refuse a write while one runs (`Gen::lowering`), since the order of what is
//! written is the output's bytes.

use crate::lir::{Arith, Capture, Cases, Cast, Comparing, ErasedType, Invoke, Kind, LiftedFrom, LirId, List, MethodRef, Relation, Unlowered};
use super::gen::{rank_type, CallRoute, CaptureArgument, ConcatRoute, CtorArguments, CtorDefault, DefaultRoute, Extras, FieldRoute, Fit, Gen, Held, HeldIn, NewRoute, PassedCapture, Referenced, Site, StaticCallRoute, StaticRoute, SuperArguments, ThisValue, ViaArguments, ViaRoute};
use super::names::*;
use crate::ast::ListRef;
use crate::classfile::ACC_STATIC;
use crate::source::FileId;
use crate::symbols::*;
use crate::tir::*;
use crate::typer::loader::javaclass::JMember;
use crate::types::*;
use std::rc::Rc;

impl<'a> Gen<'a> {
    /// Runs a lowering with the writers refusing.
    /// Writes the body of the method begun as the tree `compose` makes of it: a body the back
    /// end composes itself, with no typed IR behind it.
    pub(super) fn body(&mut self, compose: impl FnOnce(&mut Self) -> LirId) {
        let tree = self.lowering(compose);
        self.write_body(tree);
    }

    /// The method's parameter at the position, at its declared type.
    pub(super) fn param(&mut self, i: usize) -> LirId {
        let ty = self.m.params[i].1.clone();
        self.lir.push(Kind::Param(i as u32), ty)
    }

    pub(super) fn lowering<T>(&mut self, f: impl FnOnce(&mut Self) -> T) -> T {
        let was = self.code.frozen;
        if !was {
            self.lir.typed_ir_left = false;
        }
        // A lowering the writer starts stands where the writer is: the locals an earlier lowering
        // declared are in its scope only once the writer has stored them (`Gen::declared_by_lowering`).
        let earlier = (!was).then(|| std::mem::take(&mut self.lower_declared));
        self.code.frozen = true;
        self.cw.cp.frozen = true;
        let out = f(self);
        self.code.frozen = was;
        self.cw.cp.frozen = was;
        if let Some(earlier) = earlier {
            let own = std::mem::replace(&mut self.lower_declared, earlier);
            self.lower_declared.extend(own);
        }
        out
    }

    /// `e` brought to the type `want`.
    pub(super) fn lower_expr(&mut self, e: TExprId, want: &ErasedType) -> LirId {
        let id = self.lower_marked(e, want);
        let node = &mut self.lir.nodes[id.0 as usize];
        // A node not lowered takes its position as it is generated, and a jump where a value is
        // wanted its arguments': the operands it drops under it are its place's.
        if !matches!(node.kind, Kind::Legacy(..) | Kind::SelfTailCall { .. }) {
            node.of = Some(e);
        }
        id
    }

    fn lower_legacy(&mut self, e: TExprId, want: &ErasedType, why: Unlowered) -> LirId {
        self.lir.push(Kind::Legacy(e, why), want.clone())
    }

    /// Whether what the lowering asks the analysis of captures now is asked in the order the
    /// generator of the typed IR asks it: nothing of the running lowering has been left to that
    /// generator, which asks where it writes. The analysis keeps what it was asked first, and
    /// two local defs that call each other capture in the order of the one asked first, so the
    /// order of the asking is the order of a lifted method's parameters. A closure, a local
    /// def's declaration and its call met after typed IR was left are lowered where they are
    /// written.
    fn analysis_in_order(&self) -> bool {
        !self.lir.typed_ir_left
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
            TExpr::Block(stmts, res) => match self.block_lowers(stmts) {
                Ok(()) => self.lower_block(stmts, res, want),
                Err(why) => self.lower_legacy(e, want, why),
            },
            TExpr::If(c, t, els) => self.lower_if(c, t, els, want),
            TExpr::Match(scrut, cases) => match self.match_lowers(scrut, cases) {
                Ok(()) => self.lower_match(scrut, cases, Some(want)),
                Err(why) => self.lower_legacy(e, want, why),
            },
            TExpr::Throw(inner, _) => {
                let value = self.lower_expr(inner, &ErasedType::L(Rc::from(THROWABLE)));
                self.lir.push(Kind::Throw(value), want.clone())
            }
            TExpr::Try(index) => {
                let t = &prog.tries[index as usize];
                let body = self.lower_expr(t.body, want);
                let cases = (!t.cases.is_empty()).then(|| match prog.cases[t.cases.range()].iter().try_for_each(|case| self.pattern_lowers(case.pat)) {
                    Ok(()) => self.lower_catch(t.cases, want),
                    Err(why) => Cases::Legacy(index, why),
                });
                // Cases the lowering does not take are written between the body and the finalizer.
                self.lir.typed_ir_left |= matches!(cases, Some(Cases::Legacy(..)));
                let finalizer = t.finalizer.map(|f| self.lower_expr(f, &ErasedType::V));
                self.lir.push(Kind::Try { body, cases, finalizer }, want.clone())
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
            | TExpr::Spread(_) | TExpr::Return(_) | TExpr::Splice(_) => {
                let node = match self.lower_node(e) {
                    Ok(node) => node,
                    Err(why) => return self.lower_legacy(e, want, why),
                };
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

    /// One node of the typed IR at the type it leaves, or why it is not lowered yet.
    fn lower_node(&mut self, e: TExprId) -> Result<LirId, Unlowered> {
        let prog = self.cx.input.prog;
        Ok(match prog.expr(e) {
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
            TExpr::Str(s) => self.lir.push(Kind::Str(s), ErasedType::L(Rc::from(STRING))),
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
            TExpr::Local(s) if self.is_lazy_local(s) => return self.lower_lazy_read(s),
            // One the scope does not hold is reported where it is lowered (`Gen::lower_capture`).
            TExpr::Local(s) => self.lower_local(s).unwrap_or_else(|| self.lower_capture(s)),
            TExpr::Assign(target, value) => return self.lower_assign(target, value),
            TExpr::Prim(op, a, b) => return self.lower_prim(op, a, b),
            TExpr::Unary(op, a) => self.lower_unary(op, a),
            TExpr::TypeTest(inner, test, tested) => self.lower_test(inner, test, tested),
            TExpr::While(c, body) => {
                let cond = self.lower_cond(c);
                let body = self.lower_expr(body, &ErasedType::V);
                self.lir.push(Kind::While { cond, body }, ErasedType::V)
            }
            TExpr::Module(c) => self.lower_module(c),
            TExpr::ClassOf(c, dims) => self.lower_class_of(e, c, dims),
            TExpr::Cast(inner, op, _) => self.lower_cast(inner, op),
            TExpr::ToStr(inner, conv) => self.lower_to_str(inner, conv),
            TExpr::StrConcat(items) => return self.lower_str_concat(items),
            TExpr::This | TExpr::Super(_) => return self.lower_this().ok_or(Unlowered::NoInstance),
            TExpr::Static(s) => return self.lower_static(s),
            TExpr::Field(r, s) => return self.lower_field_read(r, s),
            TExpr::CallClosure(f, args) => {
                let items: Vec<TExprId> = prog.expr_list(args).to_vec();
                let ft = self.function_type(items.len());
                let function = self.lower_expr(f, &ft);
                self.lower_application(function, &ft, items)
            }
            TExpr::CallStatic(s, args) => return self.lower_call_static(s, args),
            TExpr::New(c, args) => return self.lower_new(e, c, args),
            TExpr::NewVia(s, args) => return self.lower_new_via(s, args),
            TExpr::Default(of, at, args) => return self.lower_default(of, at, args),
            TExpr::CallMethod(r, s, args) => return self.lower_call_method(e, r, s, args),
            TExpr::SeqLit(items) => return self.lower_seq_literal(e, items),
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
            TExpr::Lambda(params, body) if self.analysis_in_order() => self.lower_lambda(params, body),
            TExpr::Lambda(..) => return Err(Unlowered::AfterEscape),
            TExpr::Js(template, args) => return self.lower_template(e, template, args),
            TExpr::Return(_) if self.m.closure => return Err(Unlowered::NonLocalReturn),
            TExpr::Return(_) => return Err(Unlowered::ReturnInValuePosition),
            TExpr::Spread(_) => return Err(Unlowered::SpreadArgument),
            TExpr::JsImport(_) | TExpr::JsGlobal(..) | TExpr::JsSelect(..) | TExpr::ObjLit(_) => return Err(Unlowered::JavaScriptOnly),
            TExpr::If(..) | TExpr::Match(..) | TExpr::Block(..) | TExpr::Throw(..) | TExpr::Try(_) => unreachable!("lowered by `Gen::lower_marked`"),
            TExpr::Splice(_) => unreachable!("a stored inline body is lowered nowhere"),
        })
    }

    /// An operation on two operands; not an equality of a build whose runtime lacks the helper
    /// (`Gen::equal_without_helper`).
    fn lower_prim(&mut self, op: PrimOp, a: TExprId, b: TExprId) -> Result<LirId, Unlowered> {
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
                let &helper = self.cx.helpers.defs.get("equal").ok_or(Unlowered::EqualityWithoutHelper)?;
                let method = self.mref(helper);
                let object = ErasedType::object();
                let left = self.lower_expr(a, &object);
                let right = self.lower_expr(b, &object);
                let args = self.lir.list(&[left, right]);
                let equal = self.lir.push(Kind::Helper { method, args }, ErasedType::Z);
                if op == Eq {
                    return Ok(equal);
                }
                let yes = self.lir.push(Kind::Bool(true), ErasedType::Z);
                return Ok(self.lir.push(Kind::Binary { op: Arith::Xor, left: equal, right: yes }, ErasedType::Z));
            }
            Lt | Le | Gt | Ge | RefEq | RefNe | Same | NotSame => return Ok(self.lower_compare(op, a, b)),
            BoolAnd => {
                let (left, right) = (self.lower_cond(a), self.lower_cond(b));
                return Ok(self.lir.push(Kind::And(left, right), ErasedType::Z));
            }
            BoolOr => {
                let (left, right) = (self.lower_cond(a), self.lower_cond(b));
                return Ok(self.lir.push(Kind::Or(left, right), ErasedType::Z));
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
        Ok(self.lir.push(Kind::Binary { op: arith, left, right }, t))
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

    /// `e` as a condition: the connectives and the comparisons under it are conditions too,
    /// evaluated only as far as the outcome needs; anything else is a boolean value.
    pub(super) fn lower_cond(&mut self, e: TExprId) -> LirId {
        match self.cx.input.prog.expr(e) {
            TExpr::Bool(v) => self.lir.push(Kind::Bool(v), ErasedType::Z),
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
            | TExpr::Lambda(..) | TExpr::If(..) | TExpr::While(..) | TExpr::Block(..) | TExpr::Assign(..) | TExpr::Match(..)
            | TExpr::StrConcat(_) | TExpr::ToStr(..) | TExpr::Js(..) | TExpr::Cast(..) | TExpr::ClassOf(..) | TExpr::SeqLit(_)
            | TExpr::ArrayLit(_) | TExpr::Index(..) | TExpr::JsImport(_) | TExpr::JsGlobal(..) | TExpr::JsSelect(..) | TExpr::ObjLit(_)
            | TExpr::Spread(_) | TExpr::Return(_) | TExpr::Throw(..) | TExpr::Try(_) | TExpr::Splice(_) => self.lower_expr(e, &ErasedType::Z),
        }
    }

    /// No cast to the static type: `box.fruit.isInstanceOf[Apple]` asks because the static type
    /// may be wrong. The tested type goes with the test, as a pattern's does: an array's test
    /// reads its element there.
    fn lower_test(&mut self, inner: TExprId, test: TestId, tested: TypeId) -> LirId {
        let value = self.lower_expr(inner, &ErasedType::object());
        self.lir.push(Kind::Test { value, test, tested }, ErasedType::Z)
    }

    /// An order or an equality of two operands, compared as their static types say.
    fn lower_compare(&mut self, op: PrimOp, a: TExprId, b: TExprId) -> LirId {
        let object = ErasedType::object();
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
            TExpr::If(c, t, Some(els)) if !boxed_whole => {
                let cond = self.lower_cond(c);
                let then = self.lower_tail(t);
                let otherwise = Some(self.lower_tail(els));
                Kind::If { cond, then, otherwise }
            }
            TExpr::Block(stmts, res) if !boxed_whole => match self.block_lowers(stmts) {
                Ok(()) => {
                    let mut nodes = Vec::new();
                    self.lower_lazy_holders(stmts, &mut nodes);
                    for &s in &prog.stmts[stmts.range()] {
                        self.lower_statement(s, &mut nodes);
                    }
                    let result = self.lower_tail(res);
                    let stmts = self.lir.list(&nodes);
                    Kind::Block { stmts, result }
                }
                Err(why) => Kind::LegacyTail(e, why),
            },
            TExpr::Match(scrut, cases) if !boxed_whole => match self.match_lowers(scrut, cases) {
                Ok(()) => {
                    let id = self.lower_match(scrut, cases, None);
                    self.lir.nodes[id.0 as usize].of = Some(e);
                    return id;
                }
                Err(why) => Kind::LegacyTail(e, why),
            },
            // A return out of the method here is that return, its value the method's result (dotty's
            // `genReturn` sends the value to the method's return); out of a closure it is a throw,
            // after which the return of what it leaves is dead.
            TExpr::Return(v) if !self.m.closure => match self.jump_call(v) {
                Some((receiver, args)) => self.lower_self_tail_call(receiver, args),
                None => {
                    let ret = self.m.ret.clone();
                    Kind::Return(self.lower_expr(v, &ret))
                }
            },
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
                    Kind::Return(self.lower_expr(e, &ret))
                }
            },
        };
        let id = self.lir.push(kind, ErasedType::V);
        self.lir.nodes[id.0 as usize].of = Some(e);
        id
    }

    pub(super) fn lower_self_tail_call(&mut self, receiver: Option<TExprId>, args: ListRef) -> Kind {
        let cx = self.cx;
        let params = self.m.tail.as_ref().expect("a def that loops").params.clone();
        // A value class's extension method holds its instance unboxed (`$this`): the receiver is
        // lowered at the underlying type, as the call's lowering lowers it (`Gen::call_extension`).
        let this_type = match (self.m.this_home, self.m.this_underlying.clone(), self.this_class) {
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

    /// A block whose statements are lowered whole (`Gen::block_lowers`), its result at `want`.
    fn lower_block(&mut self, stmts: ListRef, res: TExprId, want: &ErasedType) -> LirId {
        let prog = self.cx.input.prog;
        let mut nodes = Vec::new();
        self.lower_lazy_holders(stmts, &mut nodes);
        for &s in &prog.stmts[stmts.range()] {
            self.lower_statement(s, &mut nodes);
        }
        // The pure result of an ascribed block that is a statement is not evaluated (`Program::removed_bits`).
        let result = if *want == ErasedType::V && prog.is_removed(res) { self.lir.push(Kind::Unit, ErasedType::V) } else { self.lower_expr(res, want) };
        let stmts = self.lir.list(&nodes);
        self.lir.push(Kind::Block { stmts, result }, want.clone())
    }

    /// A statement of a block as the nodes it is, in order; a pattern definition is not lowered
    /// yet (`Gen::block_statements`).
    pub(super) fn lower_statement(&mut self, s: TStmt, nodes: &mut Vec<LirId>) {
        let cx = self.cx;
        let prog = cx.input.prog;
        match s {
            TStmt::Expr(x) if prog.is_removed(x) => {}
            TStmt::Expr(x) => nodes.push(self.lower_expr(x, &ErasedType::V)),
            // Its holder stands at the head of the list (`Gen::lower_lazy_holders`).
            TStmt::Val(sym, _) if self.is_lazy_local(sym) => {}
            TStmt::Val(sym, init) => self.lower_val(sym, init, nodes),
            TStmt::Fun(f) => {
                if cx.input.reach.funs[f.idx()] {
                    let sym = prog.funs[f.idx()].sym;
                    if self.analysis_in_order() {
                        self.local_fun_shape(sym);
                    }
                    nodes.push(self.lir.push(Kind::LocalFun(sym), ErasedType::V));
                }
            }
            TStmt::Pat(..) => unreachable!("a pattern definition is not lowered (`Gen::block_lowers`)"),
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

    /// An assignment to a local or to a member, or why it is not lowered yet (`Gen::assign`).
    fn lower_assign(&mut self, target: TExprId, value: TExprId) -> Result<LirId, Unlowered> {
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
            | TExpr::Spread(_) | TExpr::Return(_) | TExpr::Throw(..) | TExpr::Try(_) | TExpr::Splice(_) => return Err(Unlowered::AssignOtherTarget),
        };
        if !self.is_cell(sym) {
            // One the method does not hold as a local is reported where it is lowered, as
            // `Gen::store_local` did: held nowhere, or a capture held by value in a field.
            let Some(HeldIn::Local(ty)) = self.held_in(sym) else {
                self.unsupported("assignment to a local that is not in scope");
                return Ok(self.lir.push(Kind::Unit, ErasedType::V));
            };
            let value = self.lower_expr(value, &ty);
            return Ok(self.lir.push(Kind::Assign { sym, value }, ErasedType::V));
        }
        let declared = self.sym_type(sym);
        // A cell the scope does not hold is reported where it is lowered (`Gen::lower_capture`).
        let cell = self.lower_held(sym).unwrap_or_else(|| self.lower_capture(sym));
        let object = ErasedType::object();
        let value = self.lower_expr(value, &declared);
        let value = self.lower_adapt(value, &declared, &object);
        Ok(self.lir.push(Kind::FieldStore { receiver: cell, owner: Rc::from(OBJECT_REF), name: Rc::from("elem"), value }, ErasedType::V))
    }

    /// An assignment to the member `s`, on the receiver `r` where one is written: a Java
    /// instance's field assigned, else the member's setter called, on the receiver or on the
    /// object of the file that declares the member. Not a Java class's static field, a Java
    /// field with no receiver nor a setter on the member's own object, not lowered yet.
    fn lower_member_assign(&mut self, r: Option<TExprId>, s: SymId, value: TExprId) -> Result<LirId, Unlowered> {
        let cx = self.cx;
        if let Some((cf, JMember::Field(i))) = cx.input.java_member(s) {
            let f = &cf.fields[i as usize];
            let r = match (f.access & ACC_STATIC != 0, r) {
                (true, _) => return Err(Unlowered::AssignJavaStatic),
                (false, None) => return Err(Unlowered::AssignJavaFieldNoReceiver),
                (false, Some(r)) => r,
            };
            let t = parse_type(&f.descriptor).0;
            let (receiver, owner) = self.lower_java_qualifier(r, s, &cf.name);
            let value = self.lower_expr(value, &t);
            return Ok(self.lir.push(Kind::FieldStore { receiver, owner, name: Rc::from(f.name.as_str()), value }, ErasedType::V));
        }
        let (receiver, at, m) = match r {
            Some(r) => {
                let m = self.mref(s);
                let site = self.invoke_site(r, s, &m);
                (self.lower_receiver(r, s, &site), site.map(|site| (site.owner, site.interface)), m)
            }
            None => {
                let info = cx.input.syms.sym(s);
                if let Owner::Class(_) = info.owner {
                    return Err(Unlowered::AssignObjectMemberNoReceiver);
                }
                (self.lower_file_module(info.file), None, self.mref(s))
            }
        };
        let value = self.lower_expr(value, &m.ret);
        let args = self.lir.list(&[value]);
        let setter = Rc::new(self.setter_ref(&m));
        Ok(self.lir.push(Kind::Call { method: setter, at, receiver, args, order: None }, ErasedType::V))
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
    /// Not where the library has no such class.
    fn lower_seq_literal(&mut self, e: TExprId, items: ListRef) -> Result<LirId, Unlowered> {
        let seq = self.cx.input.array_seq.ok_or(Unlowered::VarargsWithoutArraySeq)?;
        let name = self.class_name(seq);
        let array = ErasedType::L(Rc::from(OBJECT_ARRAY));
        let elem = self.seq_literal_element(e);
        let boxed = self.lower_boxed_array(items, &elem);
        let param = if self.cx.linked_class(seq) { array.clone() } else { ErasedType::object() };
        let wrapped = self.lower_adapt(boxed, &array, &param);
        let args = self.lir.list(&[wrapped]);
        let desc = method_desc(std::slice::from_ref(&param), &ErasedType::V);
        let ctor = Rc::new(MethodRef { owner: name.clone(), owner_is_interface: false, name: "<init>".to_string(), params: vec![param], ret: ErasedType::V, desc, kind: Invoke::Special });
        Ok(self.lir.push(Kind::New { touch: None, ctor, args }, ErasedType::L(name)))
    }

    /// An array of references holding the items boxed, each brought to `elem` first: an `Int`
    /// where the sequence's elements are `Double`s is a `Double`, which the typer leaves implicit.
    fn lower_boxed_array(&mut self, items: ListRef, elem: &ErasedType) -> LirId {
        let object = ErasedType::object();
        let held = if elem.is_ref() { &object } else { elem };
        let items: Vec<TExprId> = self.cx.input.prog.expr_list(items).to_vec();
        let mut lowered = Vec::with_capacity(items.len());
        for item in items {
            let value = self.lower_expr(item, held);
            lowered.push(self.lower_adapt(value, held, &object));
        }
        let items = self.lir.list(&lowered);
        self.lir.push(Kind::ArrayOf { elem: object, items }, ErasedType::L(Rc::from(OBJECT_ARRAY)))
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
        self.lir.push(Kind::ClassOf(of), ErasedType::L(Rc::from("java/lang/Class")))
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
    /// before the first operand, and their locals end with the concatenation. Not where one of
    /// them is not lowered yet (`Gen::str_concat`).
    fn lower_str_concat(&mut self, items: ListRef) -> Result<LirId, Unlowered> {
        let (statements, operands) = match self.concat_route(items) {
            ConcatRoute::Lowered(statements, operands) => (statements, operands),
            ConcatRoute::Statements(why, ..) => return Err(why),
        };
        if statements.is_empty() {
            return Ok(self.lower_concat(&operands));
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
        Ok(self.lir.push(Kind::Block { stmts, result }, ty))
    }

    /// `a + b + c` on strings over the chain's operands (`Gen::concat_route`): a string
    /// literal and each constant the typer folds are constants of the result, any other operand
    /// a value at its own type, a value class's as its box and one of no value as the unit value.
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
                TExpr::Str(text) => {
                    parts.push(self.lir.push(Kind::Str(text), string.clone()));
                    continue;
                }
                TExpr::Int(v) if i < folded && self.static_type(value) == ErasedType::I => Some(v.to_string()),
                TExpr::Long(v) if i < folded => Some(v.to_string()),
                TExpr::Bool(v) if i < folded => Some(v.to_string()),
                TExpr::Char(v) if i < folded => char::from_u32(v as u32).map(|c| c.to_string()),
                _ => None,
            };
            if let Some(text) = text {
                parts.push(self.lir.push(Kind::Text(Rc::from(text)), string.clone()));
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
            // The writer reads a constant of the result off a part's kind.
            assert!(!matches!(self.lir.node(part).kind, Kind::Str(_) | Kind::Text(_)), "a value part that reads as a constant");
            parts.push(part);
        }
        let parts = self.lir.list(&parts);
        self.lir.push(Kind::Concat { parts }, string)
    }

    /// `lock.synchronized(body)`: the body at its own type, which is the value's.
    pub(super) fn lower_synchronized(&mut self, lock: TExprId, body: TExprId) -> LirId {
        let t = self.static_type(body);
        let lock = self.lower_expr(lock, &ErasedType::object());
        let body = self.lower_expr(body, &t);
        self.lir.push(Kind::Synchronized { lock, body }, t)
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

    /// A member read with no receiver written, or why its route is not lowered yet.
    pub(super) fn lower_static(&mut self, s: SymId) -> Result<LirId, Unlowered> {
        let cx = self.cx;
        let info = cx.input.syms.sym(s);
        match self.static_route(s) {
            StaticRoute::GivenObject => Ok(self.lower_given_object(s)),
            StaticRoute::EnumField(companion, e) => {
                let owner = self.class_name(companion);
                let t = self.class_type(e);
                let name = encode(cx.input.interner.get(info.name));
                Ok(self.lir.push(Kind::StaticField { owner, name: Rc::from(&*name) }, t))
            }
            StaticRoute::EnumValue(_) => Err(Unlowered::EnumValueOwnClass),
            // One with no constant type is refused (`Gen::load_static`). A boolean's is the value a
            // condition tests where `Gen::write_cond` would take a `Bool` node for a constant
            // condition and write no test; a string's is a value of a concatenation where a `Text`
            // node would be text of its recipe.
            StaticRoute::InlineVal => match info.sig.as_ref().map(|sig| cx.input.types.get(sig.ret)) {
                Some(Type::Lit(l)) if !matches!(cx.input.types.lit_val(l), LitVal::Bool(_) | LitVal::Str(_)) => Ok(self.lower_literal(l)),
                Some(Type::Lit(_)) => Err(Unlowered::InlineValBooleanOrString),
                _ => Err(Unlowered::InlineValNoConstant),
            },
            StaticRoute::JavaField => {
                let Some((cf, JMember::Field(i))) = cx.input.java_member(s) else { unreachable!("a Java field, by the route") };
                let f = &cf.fields[i as usize];
                let t = parse_type(&f.descriptor).0;
                Ok(self.lir.push(Kind::StaticField { owner: Rc::from(cf.name.as_str()), name: Rc::from(f.name.as_str()) }, t))
            }
            StaticRoute::Field => {
                let m = self.mref(s);
                Ok(self.lir.push(Kind::StaticField { owner: m.owner.clone(), name: Rc::from(m.name.as_str()) }, m.ret.clone()))
            }
            StaticRoute::Accessor => {
                let m = self.mref(s);
                let receiver = match info.owner {
                    Owner::Class(c) => self.lower_module(c),
                    _ => self.lower_file_module(info.file),
                };
                let args = self.lir.list(&[]);
                let ret = m.ret.clone();
                Ok(self.lir.push(Kind::Call { method: m, at: None, receiver, args, order: None }, ret))
            }
        }
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

    /// A call with no receiver written, or why its route is not lowered yet.
    fn lower_call_static(&mut self, s: SymId, args: ListRef) -> Result<LirId, Unlowered> {
        let cx = self.cx;
        match self.static_call_route(s, args) {
            StaticCallRoute::Lifted => self.analysis_in_order().then(|| self.lower_lifted_call(s, args)).ok_or(Unlowered::AfterEscape),
            StaticCallRoute::Spread(array) => Ok(self.lower_wrapped_spread(array, None)),
            StaticCallRoute::JavaStatic | StaticCallRoute::JdkStatic => Ok(self.lower_static_call(s, args)),
            // A lazy local, a captured one and a cell are not lowered yet.
            StaticCallRoute::LocalValue => {
                let t = self.plain_local(s).ok_or(Unlowered::LocalFunctionValueNotPlain)?;
                let items: Vec<TExprId> = cx.input.prog.expr_list(args).to_vec();
                let ft = self.function_type(items.len());
                let local = self.lir.push(Kind::Local(s), t.clone());
                let function = self.lower_adapt(local, &t, &ft);
                Ok(self.lower_application(function, &ft, items))
            }
            StaticCallRoute::FileModule => {
                let m = self.mref(s);
                let receiver = self.lower_file_module(cx.input.syms.sym(s).file);
                let (args, order) = self.lower_arguments(Some(s), args, &m.params);
                let ret = m.ret.clone();
                Ok(self.lir.push(Kind::Call { method: m, at: None, receiver, args, order }, ret))
            }
        }
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

    /// A new instance and its constructor's call, or why its route is not lowered yet.
    pub(super) fn lower_new(&mut self, e: TExprId, c: ClassId, args: ListRef) -> Result<LirId, Unlowered> {
        // What a class that captures takes is decided by the analysis of captures.
        if !self.analysis_in_order() && self.asks_captures(c) {
            return Err(Unlowered::AfterEscape);
        }
        let route = match self.new_route(e, c, args) {
            NewRoute::Instance(CtorArguments::CapturedUnlowered) => return Err(Unlowered::NewCapturedUnlowered),
            NewRoute::Instance(CtorArguments::UnwrittenOuter(_)) => return Err(Unlowered::NewUnwrittenOuter),
            NewRoute::Instance(route) => route,
            NewRoute::Underlying(u, a) => {
                // The argument takes the constructor parameter's own representation (dotty types it
                // against the erased parameter, `Erasure.scala` 827, boxing a value class there,
                // 394-398): a value class over a universal trait passed as `U` is its box, which is
                // then this class's underlying value.
                let param = self.vc_unmarked(&u);
                let value = self.lower_expr(a, &param);
                return Ok(self.lir.push(Kind::Typed(value), u));
            }
            NewRoute::Js => return Err(Unlowered::JavaScriptOnly),
        };
        let touch = self.touched_module(c);
        let name = self.class_name(c);
        let (nodes, params) = self.lower_new_arguments(c, args, route);
        Ok(self.lower_new_instance(touch, name, &nodes, params))
    }

    /// A new instance through the secondary constructor `s`, or why its route is not lowered yet.
    pub(super) fn lower_new_via(&mut self, s: SymId, args: ListRef) -> Result<LirId, Unlowered> {
        if !self.analysis_in_order() && self.ctor_class(s).is_some_and(|c| self.via_asks_captures(c, s)) {
            return Err(Unlowered::AfterEscape);
        }
        let (c, route) = match self.via_route(s, args) {
            ViaRoute::Unowned => return Err(Unlowered::NewViaUnowned),
            ViaRoute::Js(_) => return Err(Unlowered::JavaScriptOnly),
            ViaRoute::Instance(_, ViaArguments::CapturedUnlowered(_)) => return Err(Unlowered::NewCapturedUnlowered),
            ViaRoute::Instance(c, route) => (c, route),
        };
        let touch = self.touched_module(c);
        let name = self.class_name(c);
        let (nodes, params) = self.lower_via_arguments(c, s, args, route);
        Ok(self.lower_new_instance(touch, name, &nodes, params))
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
    /// the constructor's parameter types: what the class captures first (`Gen::new_arguments`).
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
            CtorArguments::CapturedUnlowered | CtorArguments::UnwrittenOuter(_) => unreachable!("not lowered (`Gen::new_arguments`)"),
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
        let name = self.lir.push(Kind::Text(Rc::from(cx.input.interner.get(info.name))), string.clone());
        let ordinal = self.lir.push(Kind::Int(info.ordinal as i32), ErasedType::I);
        (vec![name, ordinal], vec![string, ErasedType::I])
    }

    /// The arguments of the secondary constructor `s` of `c` as a creation passes them, in
    /// order, and the constructor's parameter types (`Gen::via_arguments`).
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
            ViaArguments::CapturedUnlowered(_) => unreachable!("not lowered (`Gen::via_arguments`)"),
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
        // secondary constructor at the enclosing class (`Gen::via_arguments`); through the
        // primary as a captured outer is (`Gen::load_capture_arg`): a local at the enclosing
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
                PassedCapture::Held(s) => self.lower_held(s).expect("held by the scope, by the route"),
            });
        }
        nodes
    }

    /// The argument `a` that hands a lifted class a local, as the class holds it
    /// (`Gen::capture_argument_route`).
    pub(super) fn lower_capture_argument(&mut self, a: TExprId, route: CaptureArgument) -> LirId {
        match route {
            CaptureArgument::Value(t) => self.lower_expr(a, &t),
            CaptureArgument::Held(s) => self.lower_held(s).expect("held by the scope, by the route"),
            CaptureArgument::Own => {
                let t = self.static_type(a);
                let value = self.lower_expr(a, &t);
                if t == ErasedType::V { self.lower_adapt(value, &t, &ErasedType::L(Rc::from(BOXED_UNIT))) } else { value }
            }
            CaptureArgument::Holder(_) => unreachable!("not lowered (`Gen::load_capture_arg`)"),
        }
    }

    /// Arguments as they are written, each at its parameter's type, as nodes in order.
    fn lower_declared_arguments(&mut self, args: ListRef, params: &[ErasedType]) -> Vec<LirId> {
        let (list, _) = self.lower_arguments(None, args, params);
        self.lir.lists[list.start as usize..(list.start + list.len) as usize].to_vec()
    }

    /// A default argument: its getter's call, or its expression where the class has no
    /// getters, or why its route is not lowered yet.
    pub(super) fn lower_default(&mut self, of: DefaultOf, at: u32, args: ListRef) -> Result<LirId, Unlowered> {
        // Where the class captures, the getter's route asks the analysis of captures.
        if !self.analysis_in_order() && self.default_ctor_class(of).is_some_and(|c| self.asks_captures(c)) {
            return Err(Unlowered::AfterEscape);
        }
        Ok(match self.default_route(of, at, args) {
            DefaultRoute::Unlowered(why) => return Err(why),
            DefaultRoute::Ctor(c, recv, route, want) => self.lower_ctor_default(c, route, recv, &want),
            DefaultRoute::FileMember(file, getter, want) => {
                let receiver = self.lower_file_module(file);
                self.lower_getter_call(receiver, getter, &want)
            }
        })
    }

    /// The default of a parameter of a constructor of `c`, of the type `want`, where no
    /// argument precedes it (`Gen::ctor_default_route`); `recv` is the receiver the typer
    /// selected the getter on.
    pub(super) fn lower_ctor_default(&mut self, c: ClassId, route: CtorDefault, recv: Option<TExprId>, want: &ErasedType) -> LirId {
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
                        let local = self.lower_local(v).expect("held by the scope, by the route");
                        let t = self.lir.ty(local).clone();
                        self.lower_adapt(local, &t, &module)
                    }
                };
                self.lower_getter_call(receiver, getter, want)
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
                let args = self.lir.list(&[]);
                let companion = self.lir.push(Kind::Call { method: Rc::new(accessor), at: None, receiver: instance, args, order: None }, module);
                self.lower_getter_call(companion, getter, want)
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
                self.lower_static_getter_call(&nodes, getter, want)
            }
            CtorDefault::Companion(getter) => {
                let module = getter.owner.clone();
                let receiver = self.lir.push(Kind::Module(module.clone()), ErasedType::L(module));
                self.lower_getter_call(receiver, getter, want)
            }
            CtorDefault::Jar(getter) if getter.kind == Invoke::Virtual => {
                let module = getter.owner.clone();
                let receiver = self.lir.push(Kind::Module(module.clone()), ErasedType::L(module));
                self.lower_getter_call(receiver, getter, want)
            }
            CtorDefault::Jar(getter) | CtorDefault::Static(getter) => self.lower_static_getter_call(&[], getter, want),
        }
    }

    /// A getter that takes no argument called on `receiver`, its result as the type `want`.
    fn lower_getter_call(&mut self, receiver: LirId, getter: MethodRef, want: &ErasedType) -> LirId {
        let ret = getter.ret.clone();
        let args = self.lir.list(&[]);
        self.lower_as(Kind::Call { method: Rc::new(getter), at: None, receiver, args, order: None }, &ret, want)
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
        let (Some(u), Some(c)) = (self.m.this_underlying.clone(), class) else { return this };
        let held = self.lir.ty(this).clone();
        let value = self.lower_adapt(this, &held, &u);
        let boxed = self.class_type(c);
        self.lir.push(Kind::ValueClass { value, from: u, class: c, args: EMPTY_LIST, null_safe: false }, boxed)
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
            if self.m.this_underlying.is_some() && self.m.this_home.is_none() {
                return self.lower_boxed_instance(Some(h));
            }
            let held = self.lower_instance();
            let from = self.lir.ty(held).clone();
            return self.lower_adapt(held, &from, &t);
        }
        if self.m.home_slots.iter().any(|(k, _)| *k == h) {
            return self.lir.push(Kind::GivenInstance(h), t);
        }
        if let Some(k) = current.filter(|_| self.m.this_module.is_none()) {
            let held = if self.m.this_home.is_none() { self.class_homes.iter().position(|(c, _)| *c == h) } else { self.class_extras(k).homes.iter().position(|&c| c == h) };
            if let Some(i) = held {
                let owner = self.class_name(k);
                let name: Rc<str> = Rc::from(self.home_field(h, i));
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
        let object = ErasedType::object();
        let mut nodes = Vec::new();
        if instance {
            nodes.push(match home {
                Some(h) if Some(h) != self.m.this_home.or(self.this_class) => self.lower_instance_of(h),
                _ => self.lower_boxed_instance(self.this_class),
            });
        }
        if key {
            nodes.push(match key_of {
                Some(x) => self.lir.push(Kind::LazyKey(x), object.clone()),
                None => self.lir.push(Kind::ReturnKey, object.clone()),
            });
        }
        for c in captures {
            nodes.push(if c.key { self.lir.push(Kind::LazyKey(c.sym), object.clone()) } else { self.lower_capture(c.sym) });
        }
        nodes
    }

    /// A function value: its body lifted to a method over what it captures, and the instance
    /// of `scala.FunctionN` whose `apply` calls that method.
    pub(super) fn lower_lambda(&mut self, params: ListRef, body: TExprId) -> LirId {
        let cx = self.cx;
        let params: Vec<SymId> = cx.input.prog.sym_list(params).to_vec();
        let free = self.an.of_lambda(cx, &params, body);
        let object = ErasedType::object();
        let taken = vec![object.clone(); params.len()];
        let (lifted, body) = self.lifted_shape(&free, taken.clone(), object.clone(), None, Some((params, body)));
        let captures = self.lower_captures(lifted.instance.is_some(), body.home, lifted.key, None, &body.captures);
        let ft = self.function_type(taken.len());
        let ErasedType::L(owner) = ft.clone() else { unreachable!("a function's class") };
        let desc = method_desc(&taken, &object);
        let method = Rc::new(MethodRef { owner, owner_is_interface: true, name: "apply".to_string(), params: taken, ret: object, desc, kind: Invoke::Interface });
        let captures = self.lir.list(&captures);
        self.lir.push(Kind::Closure { captures, method, lifted, body }, ft)
    }

    /// A call of a local def: its lifted method over what it captures, then the arguments.
    pub(super) fn lower_lifted_call(&mut self, s: SymId, args: ListRef) -> LirId {
        let (lifted, body) = self.local_fun_shape(s);
        let captures = self.lower_captures(lifted.instance.is_some(), body.home, lifted.key, None, &body.captures);
        let (args, order) = self.lower_arguments(Some(s), args, &lifted.params);
        let captures = self.lir.list(&captures);
        let ret = lifted.ret.clone();
        self.lir.push(Kind::LiftedCall { def: s, from: LiftedFrom::Def, lifted, captures, args, order }, ret)
    }

    /// A read of the local lazy val `s`; not while typed IR has been left to the writer
    /// (`Gen::analysis_in_order`), its accessor's shape asking the capture analysis.
    fn lower_lazy_read(&mut self, s: SymId) -> Result<LirId, Unlowered> {
        self.analysis_in_order().then(|| self.lower_lazy_call(s, false)).ok_or(Unlowered::AfterEscape)
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
        let from = if init { LiftedFrom::LazyInitialiser } else { LiftedFrom::LazyAccessor };
        let ret = lifted.ret.clone();
        self.lir.push(Kind::LiftedCall { def: s, from, lifted, captures, args, order: None }, ret)
    }

    /// A member read on a receiver: its field where this class's own code may name it, else its
    /// accessor's call; or why its route is not lowered yet.
    fn lower_field_read(&mut self, r: TExprId, s: SymId) -> Result<LirId, Unlowered> {
        match self.field_route(r, s) {
            FieldRoute::Js => Err(Unlowered::JavaScriptOnly),
            FieldRoute::Object(_) => Err(Unlowered::ObjectThroughOwner),
            FieldRoute::Early => Err(Unlowered::EarlyParameter),
            FieldRoute::GivenObject { receiver_evaluated: true } => Err(Unlowered::GivenObjectThroughReceiver),
            FieldRoute::GivenObject { receiver_evaluated: false } => Ok(self.lower_given_object(s)),
            FieldRoute::Java => {
                let Some((cf, JMember::Field(i))) = self.cx.input.java_member(s) else { unreachable!("a Java field, by the route") };
                let f = &cf.fields[i as usize];
                let t = parse_type(&f.descriptor).0;
                let name: Rc<str> = Rc::from(f.name.as_str());
                // A static's receiver is not evaluated.
                if f.access & ACC_STATIC != 0 {
                    return Ok(self.lir.push(Kind::StaticField { owner: Rc::from(cf.name.as_str()), name }, t));
                }
                let (receiver, owner) = self.lower_java_qualifier(r, s, &cf.name);
                Ok(self.lir.push(Kind::Field { receiver, owner, name }, t))
            }
            // `missing[V].u` is the box's accessor on the reference, as scalac casts the qualifier
            // to the class and selects (`Erasure.scala` 733-737): a null throws. A block's result
            // is the qualifier, where a conditional's, a match's or a `try`'s branches are each
            // adapted to the value class, a null to its zero.
            FieldRoute::Underlying(c) => {
                let u = self.vc_underlying(c);
                let mut operand = r;
                while let TExpr::Block(_, res) = self.cx.input.prog.expr(operand) {
                    operand = res;
                }
                let outer = self.vc_select.replace(operand);
                let value = self.lower_expr(r, &u);
                self.vc_select = outer;
                Ok(self.lir.push(Kind::Underlying { value, operand }, u))
            }
            FieldRoute::Own => {
                let ty = self.sym_type(s);
                let name: Rc<str> = Rc::from(self.field_name(s));
                let receiver = self.lower_this().expect("an instance, by `Gen::own_field`");
                let owner = self.this_name.clone();
                Ok(self.lir.push(Kind::Field { receiver, owner, name }, ty))
            }
            FieldRoute::PrivateParam(c) => {
                let ty = self.sym_type(s);
                let name: Rc<str> = Rc::from(self.field_name(s));
                let owner = self.class_name(c);
                let receiver = self.lower_expr(r, &ErasedType::L(owner.clone()));
                Ok(self.lir.push(Kind::Field { receiver, owner, name }, ty))
            }
            FieldRoute::Accessor => {
                let s = self.deferred_implementation(r, s).unwrap_or(s);
                let m = self.mref(s);
                let site = self.invoke_site(r, s, &m);
                let receiver = self.lower_receiver(r, s, &site);
                let args = self.lir.list(&[]);
                let ret = m.ret.clone();
                Ok(self.lir.push(Kind::Call { method: m, at: site.map(|site| (site.owner, site.interface)), receiver, args, order: None }, ret))
            }
        }
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
            lowered.push(match &want {
                ErasedType::L(array) if array.starts_with('[') => self.lower_array_argument(a, array),
                _ => self.lower_expr(a, &want),
            });
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

    /// A call `e` of a member on a receiver, or why its route is not lowered yet.
    fn lower_call_method(&mut self, e: TExprId, r: TExprId, s: SymId, args: ListRef) -> Result<LirId, Unlowered> {
        match self.call_route(r, s, args) {
            CallRoute::Plain => {}
            // The receiver of a static is not evaluated.
            CallRoute::JavaStatic | CallRoute::JdkStatic => return Ok(self.lower_static_call(s, args)),
            CallRoute::Spread(array) => return Ok(self.lower_wrapped_spread(array, Some(r))),
            CallRoute::Super => return Ok(self.lower_super_call(e, r, s, args)),
            CallRoute::Interpolator(helper, Some(held)) => return Ok(self.lower_interpolator(r, s, helper, held, args)),
            // The proxies are the capture analysis's, asked in order only (`Gen::analysis_in_order`).
            CallRoute::Proxied(t) => return self.analysis_in_order().then(|| self.lower_proxied_call(t, s, args)).unwrap_or(Err(Unlowered::AfterEscape)),
            CallRoute::Lifted => return self.analysis_in_order().then(|| self.lower_lifted_member_call(r, s, args)).unwrap_or(Err(Unlowered::AfterEscape)),
            CallRoute::Js => return Err(Unlowered::JavaScriptOnly),
            CallRoute::Extension(..) => return Err(Unlowered::ValueClassMethodCall),
            CallRoute::Interpolator(_, None) => return Err(Unlowered::InterpolatorWithoutRuntime),
            CallRoute::Object(_) => return Err(Unlowered::ObjectThroughOwner),
        }
        let m = self.mref(s);
        let site = self.invoke_site(r, s, &m);
        let receiver = self.lower_receiver(r, s, &site);
        let (args, order) = self.lower_arguments(Some(s), args, &m.params);
        let ret = m.ret.clone();
        Ok(self.lir.push(Kind::Call { method: m, at: site.map(|site| (site.owner, site.interface)), receiver, args, order }, ret))
    }

    /// `super.s(args)`, the call `e`: through the accessor a class that mixes the trait in
    /// implements where `super` is a trait's, else the member of the superclass or of the trait
    /// named, called as that class's own. A Java interface's method is called on the interface
    /// itself, whatever brings it: the receiver of a super call is the method's owner
    /// (`BCodeBodyBuilder.genCallMethod`), which the class then lists (`superCallTargets`).
    fn lower_super_call(&mut self, e: TExprId, r: TExprId, s: SymId, args: ListRef) -> LirId {
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
        let receiver = self.lower_receiver(r, s, &None);
        let (args, order) = self.lower_arguments(Some(s), args, &method.params);
        let place = cx.input.prog.span_of(e).map(|(file, span)| (file, span.end));
        let ret = method.ret.clone();
        self.lir.push(Kind::SuperCall { method, receiver, args, order, place }, ret)
    }

    /// `super[T].s(args)` of a member of the local trait `t` that takes proxies, as scalac's
    /// forwarder calls it: the trait's static `s$` over the instance, the proxies as this code
    /// holds them, their extras, then the arguments. Not a right-associative member, whose class
    /// file takes the arguments in another order.
    pub(super) fn lower_proxied_call(&mut self, t: ClassId, s: SymId, args: ListRef) -> Result<LirId, Unlowered> {
        if self.jvm_param_order(s).is_some() {
            return Err(Unlowered::LocalTraitMemberRightAssociative);
        }
        let proxies = self.member_proxies(s).expect("a member that takes proxies, by the route");
        let declared = self.proxied_declared(s);
        let m = self.mref(s);
        let owner = self.class_name(t);
        let extras = self.extras_of(&declared, None);
        let pm = self.proxied_mref(&m, &proxies, &extras);
        let mut nodes = vec![self.lower_instance()];
        let items: Vec<TExprId> = self.cx.input.prog.expr_list(args).to_vec();
        let k = declared.len();
        for &p in &proxies {
            let want = self.anon_capture_type(p);
            let arg = declared.iter().position(|&d| d == p).and_then(|i| items.get(i).copied());
            nodes.push(match arg {
                Some(a) if !matches!(self.cx.input.prog.expr(a), TExpr::Local(x) if x == p) => self.lower_expr(a, &want),
                _ => self.lower_capture_as(p, &want),
            });
        }
        let extras = self.extras_of(&declared, None);
        nodes.extend(self.lower_extras(&extras));
        self.lower_rest(s, args, k, &m.params, &mut nodes);
        let mut params = vec![ErasedType::L(owner.clone())];
        params.extend(pm.params.iter().cloned());
        let desc = method_desc(&params, &pm.ret);
        let ret = pm.ret.clone();
        let method = Rc::new(MethodRef { owner, owner_is_interface: true, name: format!("{}$", pm.name), params, ret: ret.clone(), desc, kind: Invoke::Static });
        let args = self.lir.list(&nodes);
        Ok(self.lir.push(Kind::StaticCall { method, args, order: None }, ret))
    }

    /// `r.s(proxies, args)` of a local trait's lifted member, as dotty's `LambdaLift.addFreeArgs`
    /// leaves it, on a receiver that is no `super`: the interface's method over the proxies, their
    /// extras and the parameters, at the receiver's site (`Gen::call_lifted`). Not on `super`, a
    /// right-associative member, nor where an argument for a proxy is no read of a local (a lazy
    /// val's new holder: `Gen::load_capture_arg`).
    pub(super) fn lower_lifted_member_call(&mut self, r: TExprId, s: SymId, args: ListRef) -> Result<LirId, Unlowered> {
        let cx = self.cx;
        if matches!(cx.input.prog.expr(r), TExpr::Super(_)) {
            return Err(Unlowered::LocalTraitMemberUnplain);
        }
        if self.jvm_param_order(s).is_some() {
            return Err(Unlowered::LocalTraitMemberRightAssociative);
        }
        let declared = self.proxied_declared(s);
        let items: Vec<TExprId> = cx.input.prog.expr_list(args).to_vec();
        let k = declared.len();
        // The arguments for the proxies, by the cases of `Gen::load_capture_arg` that are a tree.
        let value_of_other = |g: &Self, p: SymId, x: SymId| x != p && g.held(p) == Held::Value && !g.is_local_def(p) && !cx.input.syms.sym(p).by_name;
        let plain = declared.iter().zip(&items[..k.min(items.len())]).all(|(&p, &a)| match cx.input.prog.expr(a) {
            TExpr::Local(x) => value_of_other(self, p, x) || self.is_capturable(x),
            _ => false,
        });
        if !plain {
            return Err(Unlowered::LocalTraitMemberUnplain);
        }
        let proxies = self.member_proxies(s).expect("a member that takes proxies, by the route");
        let m = self.mref(s);
        let extras = self.extras_of(&declared, None);
        let pm = self.proxied_mref(&m, &proxies, &extras);
        let site = self.invoke_site(r, s, &m);
        let receiver = self.lower_receiver(r, s, &site);
        let mut nodes = Vec::new();
        for &p in &proxies {
            let want = self.anon_capture_type(p);
            let arg = declared.iter().position(|&d| d == p).and_then(|i| items.get(i).copied());
            nodes.push(match arg.map(|a| (a, cx.input.prog.expr(a))) {
                Some((a, TExpr::Local(x))) if value_of_other(self, p, x) => self.lower_expr(a, &want),
                Some((_, TExpr::Local(x))) => self.lower_capture_as(x, &want),
                _ => self.lower_capture_as(p, &want),
            });
        }
        nodes.extend(self.lower_extras(&extras));
        self.lower_rest(s, args, k, &m.params, &mut nodes);
        let ret = pm.ret.clone();
        let args = self.lir.list(&nodes);
        Ok(self.lir.push(Kind::Call { method: Rc::new(pm), at: site.map(|site| (site.owner, site.interface)), receiver, args, order: None }, ret))
    }

    /// The captures a local trait's member that takes proxies declares first.
    fn proxied_declared(&self, s: SymId) -> Vec<SymId> {
        let fun = self.proxied_fun(s).expect("a member that takes proxies, by the route");
        fun.params[..fun.captures as usize].to_vec()
    }

    /// What the scope holds for the local `s` (`lower_capture`) as a value of the type `want`.
    fn lower_capture_as(&mut self, s: SymId, want: &ErasedType) -> LirId {
        let held = self.lower_capture(s);
        let t = self.lir.ty(held).clone();
        self.lower_adapt(held, &t, want)
    }

    /// The `Extras` `e` as the code holds them (`Gen::load_extras`).
    fn lower_extras(&mut self, e: &Extras) -> Vec<LirId> {
        let mut nodes: Vec<LirId> = e.homes.iter().map(|&h| self.lower_instance_of(h)).collect();
        nodes.extend(e.keys.iter().map(|&x| self.lir.push(Kind::LazyKey(x), ErasedType::object())));
        nodes
    }

    /// The arguments after the first `k` of a call of `s` at `params`, onto `nodes`.
    fn lower_rest(&mut self, s: SymId, args: ListRef, k: usize, params: &[ErasedType], nodes: &mut Vec<LirId>) {
        let rest = ListRef { start: args.start + k as u32, len: args.len.saturating_sub(k as u32) };
        let (rest, _) = self.lower_arguments(Some(s), rest, params);
        nodes.extend_from_slice(&self.lir.lists[rest.start as usize..(rest.start + rest.len) as usize]);
    }

    /// `context.s(args)` of scala-library, which has no bytecode: the runtime's definition
    /// `held` of the name of `helper`, which lowers it as scalac does, applied to the context
    /// and the arguments at `helper`'s parameters.
    fn lower_interpolator(&mut self, r: TExprId, s: SymId, helper: SymId, held: SymId, args: ListRef) -> LirId {
        let params = self.mref(helper).params.clone();
        let context = self.lower_expr(r, &params[0]);
        let (args, None) = self.lower_arguments(Some(s), args, &params[1..]) else { unreachable!("an interpolator takes its arguments as written") };
        let mut items = vec![context];
        items.extend_from_slice(&self.lir.lists[args.start as usize..(args.start + args.len) as usize]);
        let args = self.lir.list(&items);
        let method = self.mref(held);
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
