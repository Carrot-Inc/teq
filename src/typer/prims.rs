use super::apply::{ArgList, ArgSrc};
use super::Worker;
use crate::ast::{Expr, ExprId, ListRef};
use crate::intern::Name;
use crate::names;
use crate::source::Span;
use crate::symbols::*;
use crate::tir::*;
use crate::tir::capture::{Form, Wrap};
use crate::types::*;

/// What a cast is once erased (`Worker::cast_lowering`).
#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum CastLowering {
    /// The value itself: dotty's ascription of a cast its erasure makes redundant.
    Same,
    /// The value, then `()`.
    Unit,
    /// The numeric conversion to the rank.
    Convert(u8),
    /// A primitive to a primitive with no conversion between them (`tpd.primitiveConversion`
    /// when the source has no `toX`): `throw new ClassCastException()`, the operand not run.
    Fail,
    /// A node that does something at run time.
    Op(CastOp),
}

#[derive(Clone, Copy, PartialEq)]
enum OpClass {
    Arith,
    Compare,
    Bits,
    Shift,
}

/// The prefix operator a `unary_` method name stands for.
fn prefix_op_of(name: Name) -> Option<Name> {
    Some(match name {
        names::UNARY_MINUS => names::MINUS,
        names::UNARY_BANG => names::BANG,
        names::UNARY_TILDE => names::TILDE,
        names::UNARY_PLUS => names::PLUS,
        _ => return None,
    })
}

/// The `unary_` method name a prefix operator stands for.
pub(super) fn unary_of(op: Name) -> Option<Name> {
    Some(match op {
        names::MINUS => names::UNARY_MINUS,
        names::BANG => names::UNARY_BANG,
        names::TILDE => names::UNARY_TILDE,
        names::PLUS => names::UNARY_PLUS,
        _ => return None,
    })
}

fn classify(op: Name) -> Option<OpClass> {
    Some(match op {
        names::PLUS | names::MINUS | names::STAR | names::SLASH | names::PERCENT => OpClass::Arith,
        names::LT | names::LE | names::GT | names::GE => OpClass::Compare,
        names::AMP | names::BAR | names::CARET => OpClass::Bits,
        names::SHL | names::SHR | names::USHR => OpClass::Shift,
        _ => return None,
    })
}

/// The numeric types in the order of scalac's widening, `Byte <: Short <: Int <: Long <: Float
/// <: Double`, with `Char` placed where it widens to `Int` and up and nothing widens to it. An
/// operator promotes its operands to `Int` at least.
pub const R_BYTE: u8 = 0;
pub const R_SHORT: u8 = 1;
pub const R_CHAR: u8 = 2;
pub const R_INT: u8 = 3;
pub const R_LONG: u8 = 4;
pub const R_FLOAT: u8 = 5;
pub const R_DOUBLE: u8 = 6;

/// Whether a value of rank `from` widens to rank `to` without a conversion method.
pub fn widens(from: u8, to: u8) -> bool {
    from < to && to != R_CHAR
}

/// The operations that take a value of rank `from` to rank `to`; none between `Byte`, `Short`
/// and `Int` narrowing aside, whose values are one representation on every target, but a node
/// for every widening between two kinds of box (`UnOp::same_number`: the JVM unboxes the
/// operand as its own kind before it widens, JavaScript writes the operand).
pub fn convert_ops(from: u8, to: u8) -> &'static [UnOp] {
    use UnOp::*;
    match (from, to) {
        _ if from == to => &[],
        (R_BYTE, R_SHORT) => &[ByteToShort],
        (R_BYTE, R_INT) => &[ByteToInt],
        (R_SHORT, R_INT) => &[ShortToInt],
        (R_SHORT | R_INT, R_BYTE) => &[IntToByte],
        (R_CHAR, R_BYTE) => &[CharToInt, IntToByte],
        (R_LONG, R_BYTE) => &[LongToInt, IntToByte],
        (R_FLOAT, R_BYTE) => &[FloatToInt, IntToByte],
        (R_DOUBLE, R_BYTE) => &[DoubleToInt, IntToByte],
        (R_INT, R_SHORT) => &[IntToShort],
        (R_CHAR, R_SHORT) => &[CharToInt, IntToShort],
        (R_LONG, R_SHORT) => &[LongToInt, IntToShort],
        (R_FLOAT, R_SHORT) => &[FloatToInt, IntToShort],
        (R_DOUBLE, R_SHORT) => &[DoubleToInt, IntToShort],
        (R_BYTE | R_SHORT | R_INT, R_CHAR) => &[IntToChar],
        (R_LONG, R_CHAR) => &[LongToInt, IntToChar],
        (R_FLOAT, R_CHAR) => &[FloatToInt, IntToChar],
        (R_DOUBLE, R_CHAR) => &[DoubleToInt, IntToChar],
        (R_CHAR, R_INT) => &[CharToInt],
        (R_LONG, R_INT) => &[LongToInt],
        (R_FLOAT, R_INT) => &[FloatToInt],
        (R_DOUBLE, R_INT) => &[DoubleToInt],
        (R_BYTE | R_SHORT | R_INT, R_LONG) => &[IntToLong],
        (R_CHAR, R_LONG) => &[CharToLong],
        (R_FLOAT, R_LONG) => &[FloatToLong],
        (R_DOUBLE, R_LONG) => &[DoubleToLong],
        (R_BYTE | R_SHORT | R_INT, R_FLOAT) => &[IntToFloat],
        (R_CHAR, R_FLOAT) => &[CharToInt, IntToFloat],
        (R_LONG, R_FLOAT) => &[LongToFloat],
        (R_DOUBLE, R_FLOAT) => &[DoubleToFloat],
        (R_BYTE | R_SHORT | R_INT, R_DOUBLE) => &[IntToDouble],
        (R_CHAR, R_DOUBLE) => &[CharToInt, IntToDouble],
        (R_LONG, R_DOUBLE) => &[LongToDouble],
        (R_FLOAT, R_DOUBLE) => &[FloatToDouble],
        _ => &[],
    }
}

/// The template that unboxes a reflective call's result to a primitive type (`reflective.rs`), over
/// the value and the type's zero, which the JVM backend and the interpreter know by its text.
pub const UNBOX: &str = "($0 ?? $1)";

/// What `prim_binop` made of `l op r`.
enum Binop {
    Done(TExprId, TypeId),
    /// `op` is not a builtin operator of the left type; the right operand is untyped.
    NotBuiltin,
    /// The right operand, typed, is not one the builtin operator takes.
    Operand(TExprId, TypeId),
}


/// The conversions between numbers a numeric type has without a member symbol, which a
/// conversion to it is indexed under too (`Integer2int(i).toLong` for `i.toLong`).
pub(super) const NUMERIC_CONVERSIONS: &[&str] = &["toByte", "toShort", "toChar", "toInt", "toLong", "toFloat", "toDouble"];

impl<'a> Worker<'a> {
    pub fn is_primitive(&self, t: TypeId) -> bool {
        self.is_numeric(t).is_some() || t == self.b.t_boolean || t == self.b.t_string || t == self.b.t_unit
    }

    /// Converts a numeric operand to the numeric type of the given rank.
    #[inline]
    pub fn convert_rank(&mut self, te: TExprId, from: TypeId, rank: u8) -> TExprId {
        let from_rank = self.is_numeric(from).unwrap_or(R_INT);
        let mut te = te;
        for &op in convert_ops(from_rank, rank) {
            te = self.prog.add(TExpr::Unary(op, te));
        }
        te
    }

    /// `te`, the typer's form of the call of the builtin member `name` of its receiver.
    fn builtin_member(&mut self, te: TExprId, name: Name) -> TExprId {
        if self.capturing() {
            self.capture_form(te, Form::Op(name));
        }
        te
    }

    /// An operand of a builtin operator converted to the operator's rank.
    fn promote(&mut self, te: TExprId, from: TypeId, rank: u8) -> TExprId {
        let promoted = self.convert_rank(te, from, rank);
        if self.capturing() {
            self.capture_conversion(promoted, te, Form::Promotion);
        }
        promoted
    }

    pub fn rank_type(&self, rank: u8) -> TypeId {
        let b = &self.b;
        match rank {
            R_BYTE => b.t_byte,
            R_SHORT => b.t_short,
            R_CHAR => b.t_char,
            R_INT => b.t_int,
            R_LONG => b.t_long,
            R_FLOAT => b.t_float,
            _ => b.t_double,
        }
    }

    /// Whether a value of type `from` widens to the numeric type `to` as scalac's numeric
    /// widening conversion has it.
    pub fn numeric_widening(&self, from: TypeId, to: TypeId) -> bool {
        match (self.is_numeric(from), self.is_numeric(to)) {
            (Some(f), Some(t)) => widens(f, t),
            _ => false,
        }
    }

    /// `te` of type `from` widened to the numeric type `to`, typed as `to`.
    pub fn widen_numeric(&mut self, te: TExprId, from: TypeId, to: TypeId) -> Option<TExprId> {
        if !self.numeric_widening(from, to) {
            return None;
        }
        let rank = self.is_numeric(to).unwrap();
        let widened = self.convert_rank(te, from, rank);
        if self.capturing() {
            self.capture_conversion(widened, te, Form::Widening);
        }
        if widened == te {
            // No conversion node (an `Int` to a `Double`): the receiver itself is retyped, which
            // an attempt that is retracted takes back (`state::retype_expr`).
            self.retype_expr(te, to);
        } else {
            self.prog.set_type(widened, to);
        }
        Some(widened)
    }

    /// A library body's operator, the place of a diagnostic of its typing, declared where
    /// its TASTy names the declaration.
    #[cold]
    #[inline(never)]
    pub(super) fn type_infix_placed(&mut self, e: ExprId, l: ExprId, op: Name, r: ExprId, span: Span, expected: Option<TypeId>) -> (TExprId, TypeId) {
        let outer = self.body_node.replace(super::BodyNode { file: self.env.file, selection: e, call: e });
        let declared = self.cur_ast().reader.as_deref().and_then(|t| super::apply::declaration_of(t, e));
        let typed = match declared {
            Some(d) => self.type_declared_infix(d, e, l, op, r, span, expected),
            None => self.type_infix(l, op, r, span, expected),
        };
        self.body_node = outer;
        typed
    }

    /// A library body's operator whose TASTy names its declaration: where
    /// the left operand's name is overloaded, the declared alternative is the one called.
    fn type_declared_infix(&mut self, d: crate::ast::DeclRef, e: ExprId, l: ExprId, op: Name, r: ExprId, span: Span, expected: Option<TypeId>) -> (TExprId, TypeId) {
        let outer_took = self.declared_took.take();
        self.infix_declared = Some(d);
        let (te, ty) = self.type_infix(l, op, r, span, expected);
        self.infix_declared = None;
        let took = std::mem::replace(&mut self.declared_took, outer_took);
        if super::loader::declared::dump_path().is_some() {
            self.dump_declared(d, e, span, Some(ty), took);
        }
        (te, ty)
    }

    pub fn type_infix(
        &mut self,
        l: ExprId,
        op: Name,
        r: ExprId,
        span: Span,
        expected: Option<TypeId>,
    ) -> (TExprId, TypeId) {
        // A library body's operator's declaration (`type_declared_infix`), this operator's
        // alone: taken before its operands are typed.
        let declared = self.infix_declared.take();
        let t_bool = self.b.t_boolean;
        if op == names::AMPAMP || op == names::BARBAR {
            // The short-circuit operators of a Boolean; on anything else `&&` is a method.
            let (tl, lty) = self.type_expr(l, Some(t_bool));
            let lty = self.solve_in(lty);
            if self.dealias(lty) != t_bool {
                let lists = vec![ArgList { args: self.infix_args(r), using: false, span }];
                return self.apply_member(tl, lty, op, None, lists, span, expected);
            }
            let tr = self.check_right_operand(op, tl, r);
            let prim = if op == names::AMPAMP { PrimOp::BoolAnd } else { PrimOp::BoolOr };
            return (self.prog.add(TExpr::Prim(prim, tl, tr)), t_bool);
        }
        if op == names::EQEQ || op == names::NEQ {
            return self.type_equality(l, r, op == names::NEQ, span);
        }
        let op_text = self.name_ref(op);
        if op_text.ends_with(':') {
            return self.type_right_assoc(l, op, r, span, expected);
        }
        let is_assign_op = op_text.len() > 1
            && op_text.ends_with('=')
            && !op_text.starts_with('=')
            && !matches!(op_text, "<=" | ">=" | "!=");
        let base_text = is_assign_op.then(|| op_text[..op_text.len() - 1].to_string());
        let base = base_text.map(|text| self.interner.intern(&text));
        if let Some(base) = base {
            let ast = self.cur_ast();
            if let Expr::Apply(f, args) = ast.expr(l) {
                let plain = ast.expr_list(args).iter().all(|&a| !matches!(ast.expr(a), Expr::NamedArg(..) | Expr::Typed(..)));
                if plain && self.is_indexable(f) {
                    return self.type_indexed_assign_op(f, args, op, base, r, span, expected);
                }
            }
        }
        let outer_target = std::mem::replace(&mut self.assign_op_target, base.map(|_| l));
        self.assign_op_receiver = None;
        let (tl, lty) = self.type_expr(l, None);
        self.assign_op_target = outer_target;
        let receiver = match self.assign_op_receiver.take() {
            None if base.is_some() && matches!(self.cur_ast().expr(l), Expr::Ident(_)) => self.this_receiver(tl),
            receiver => receiver,
        };
        if let (Some(base), Some(receiver)) = (base, receiver) {
            if let Some(done) = self.assign_op_through_setter(tl, lty, receiver, op, base, r, span) {
                return done;
            }
        }
        let Some(d) = declared else { return self.assign_op_or_binop(tl, lty, op, base, r, span, expected) };
        let outer = self.declared_call.replace((tl, op, d));
        let typed = self.assign_op_or_binop(tl, lty, op, base, r, span, expected);
        self.declared_call = outer;
        typed
    }

    /// `(l op r)(args)..`, which is `l.op(r)(args)..`: the further lists fill the clauses of the
    /// operator method after its first (an implicit clause passed explicitly), or apply what a
    /// builtin operator gives. None for an operator the typer reads apart from a method call.
    pub(super) fn type_infix_applied(
        &mut self,
        l: ExprId,
        op: Name,
        r: ExprId,
        lists: Vec<ArgList>,
        span: Span,
        expected: Option<TypeId>,
    ) -> Option<(TExprId, TypeId)> {
        let op_text = self.name_ref(op);
        let assign_op = op_text.len() > 1 && op_text.ends_with('=') && !op_text.starts_with('=') && !matches!(op_text, "<=" | ">=" | "!=");
        if matches!(op, names::EQEQ | names::NEQ) || op_text.ends_with(':') || assign_op {
            return None;
        }
        // `(l && r)(ev)`: on anything but a Boolean `&&` is a method, whose further clauses the
        // lists fill (zio-test's `TestTrace.&&` takes `A <:< Boolean`); a Boolean's result is
        // applied to them as a value.
        if op == names::AMPAMP || op == names::BARBAR {
            let t_bool = self.b.t_boolean;
            let (tl, lty) = self.type_expr(l, Some(t_bool));
            let lty = self.solve_in(lty);
            if self.dealias(lty) != t_bool {
                let mut all = vec![ArgList { args: self.infix_args(r), using: false, span }];
                all.extend(lists);
                return Some(self.apply_member(tl, lty, op, None, all, span, expected));
            }
            let tr = self.check_right_operand(op, tl, r);
            let prim = if op == names::AMPAMP { PrimOp::BoolAnd } else { PrimOp::BoolOr };
            let te = self.prog.add(TExpr::Prim(prim, tl, tr));
            return Some(self.apply_callee(super::apply::Callee::Value(te, t_bool), None, lists, span, expected));
        }
        let (tl, lty) = self.type_expr(l, None);
        let solved = self.solve_in(lty);
        let widened = self.dealias(solved);
        let prim_lty = self.numeric_bound(widened).or_else(|| self.primitive_side(widened)).or_else(|| self.primitive_bound(widened)).unwrap_or(widened);
        if (self.is_primitive(prim_lty) || prim_lty == self.b.t_string) && classify(op).is_some() {
            let (te, ty) = self.binop_or_member(op, tl, prim_lty, widened, r, span, None);
            return Some(self.apply_callee(super::apply::Callee::Value(te, ty), None, lists, span, expected));
        }
        let keeps_type = self.types.is_path(solved) || matches!(self.types.get(solved), Type::Member(..) | Type::AppMember(..) | Type::Decl(_) | Type::Alias(..));
        let recv_ty = if keeps_type { solved } else { widened };
        let mut all = vec![ArgList { args: self.infix_args(r), using: false, span }];
        all.extend(lists);
        Some(self.apply_member(tl, recv_ty, op, None, all, span, expected))
    }

    /// `q.y op= r` where `y` is a method without parameters and `q` has a setter `y_=`: `q.y_=(q.y
    /// base r)`, `q` evaluated once, unless the value has an `op=` of its own. `receiver` is the
    /// typed `q` of `tl`.
    fn assign_op_through_setter(&mut self, tl: TExprId, lty: TypeId, receiver: (TExprId, TypeId), op: Name, base: Name, r: ExprId, span: Span) -> Option<(TExprId, TypeId)> {
        let (tq, qty) = receiver;
        let TExpr::CallMethod(recv, getter, args) = self.prog.expr(tl) else { return None };
        if recv != tq || args.len != 0 || self.syms.sym(getter).kind != SymKind::Def || qty == ERROR {
            return None;
        }
        let solved = self.solve_in(lty);
        let widened = self.dealias(solved);
        let keeps_type = self.types.is_path(solved) || matches!(self.types.get(solved), Type::Member(..) | Type::AppMember(..) | Type::Decl(_) | Type::Alias(..));
        let lty = if keeps_type { solved } else { widened };
        if lty == ERROR || self.find_member(lty, op).is_some() || self.has_lexical_extension_for(lty, op) {
            return None;
        }
        let name = self.syms.sym(getter).name;
        let setter = self.interner.intern(&format!("{}_=", self.name_ref(name)));
        if self.find_member(qty, setter).is_none() && !self.has_lexical_extension_for(qty, setter) {
            return None;
        }
        let mark = self.hoisted.len();
        let tq = if self.is_stable(tq) { tq } else { self.hoist(tq, qty, span) };
        let current = self.prog.add(TExpr::CallMethod(tq, getter, args));
        self.prog.set_type(current, lty);
        let (value, vty) = self.binop_or_member(base, current, lty, lty, r, span, None);
        let lists = vec![ArgList { args: vec![ArgSrc::Typed(value, vty)], using: false, span }];
        let (call, _) = self.apply_member(tq, qty, setter, None, lists, span, None);
        Some((self.wrap_hoisted(mark, call), self.b.t_unit))
    }

    /// `l op r` on the typed `l`; `base` is `op` without its trailing `=` when the operator
    /// is an assignment operator, which stands for `l = l base r` on a var without an `op=`.
    fn assign_op_or_binop(
        &mut self,
        tl: TExprId,
        lty: TypeId,
        op: Name,
        base: Option<Name>,
        r: ExprId,
        span: Span,
        expected: Option<TypeId>,
    ) -> (TExprId, TypeId) {
        let solved = self.solve_in(lty);
        let widened = self.dealias(solved);
        // A receiver of a singleton type keeps it for the member call: `this += x` is
        // `this.type` where `+=` is; so does one of an abstract member type, whose implicit
        // scope its bound lacks; the builtin operators see the type it stands for.
        let keeps_type = self.types.is_path(solved) || matches!(self.types.get(solved), Type::Member(..) | Type::AppMember(..) | Type::Decl(_) | Type::Alias(..));
        let lty = if keeps_type { solved } else { widened };
        if let Some(base) = base {
            let has_member = self.find_member(lty, op).is_some() || self.has_lexical_extension_for(lty, op);
            let target = match self.prog.expr(tl) {
                TExpr::Local(s) | TExpr::Static(s) | TExpr::Field(_, s) => Some(s),
                _ => None,
            };
            if !has_member && target.map_or(false, |s| self.syms.sym(s).kind == SymKind::Var) {
                let mark = self.hoisted.len();
                let tl = self.field_target_once(tl, span);
                let (value, vty) = self.binop_or_member(base, tl, lty, lty, r, span, None);
                let value = self.adapt(value, vty, lty, span);
                let assign = self.assignment(tl, value);
                return (self.wrap_hoisted(mark, assign), self.b.t_unit);
            }
        }
        // A numeric opaque type has the operators of its bound outside its scope; a binder of
        // an intersection with a primitive (`n: Int & T` of a typed pattern) has the primitive's,
        // and a type parameter bounded by a primitive or `String` its bound's.
        let prim_lty = self.numeric_bound(widened).or_else(|| self.primitive_side(widened)).or_else(|| self.primitive_bound(widened)).unwrap_or(widened);
        self.binop_or_member(op, tl, prim_lty, lty, r, span, expected)
    }

    /// The builtin primitive or `String` that bounds the type parameter `t` from above
    /// (`L <: String`), whose operators the parameter has as scalac finds them on the bound.
    pub(super) fn primitive_bound(&mut self, t: TypeId) -> Option<TypeId> {
        let mut t = t;
        for _ in 0..8 {
            let Type::Param(p) = self.types.get(t) else { return None };
            let upper = self.dealias(self.syms.tparam(p).upper);
            if upper == self.b.t_boolean || upper == self.b.t_string || self.is_numeric(upper).is_some() {
                return Some(upper);
            }
            if let Some(side) = self.primitive_side(upper) {
                return Some(side);
            }
            t = upper;
        }
        None
    }

    /// The builtin primitive that an intersection type contains, if one of its sides is one.
    pub(super) fn primitive_side(&mut self, t: TypeId) -> Option<TypeId> {
        let Type::Inter(a, b) = self.types.get(t) else { return None };
        for side in [a, b] {
            let side = self.dealias(side);
            if side == self.b.t_boolean || side == self.b.t_string || self.is_numeric(side).is_some() {
                return Some(side);
            }
            if let Some(inner) = self.primitive_side(side) {
                return Some(inner);
            }
        }
        None
    }

    /// `l op r` as a builtin operator of `prim_lty`, else as a call of the member or extension
    /// method `op` of `lty`. A right operand the builtin operator does not take goes to such a
    /// call when one exists, so that `3 * money` reaches an `extension (n: Int) def *(m: Money)`.
    pub(super) fn binop_or_member(
        &mut self,
        op: Name,
        tl: TExprId,
        prim_lty: TypeId,
        lty: TypeId,
        r: ExprId,
        span: Span,
        expected: Option<TypeId>,
    ) -> (TExprId, TypeId) {
        let args = match self.prim_binop(op, tl, prim_lty, r, span) {
            Binop::Done(te, ty) => return (te, ty),
            Binop::NotBuiltin => self.infix_args(r),
            Binop::Operand(tr, rty) => {
                if rty == ERROR {
                    return (tl, ERROR);
                }
                if self.find_member(lty, op).is_none() && !self.has_lexical_extension_for(lty, op) {
                    if let Some(done) = self.binop_with_converted_operand(op, tl, prim_lty, tr, rty, span) {
                        return done;
                    }
                    if let Some(done) = self.binop_with_converted_receiver(op, tl, lty, tr, rty, span, expected) {
                        return done;
                    }
                    let msg = format!(
                        "operator {} cannot be applied to {} and {}",
                        self.name_str(op),
                        self.show(lty),
                        self.show(rty)
                    );
                    self.error(span, msg);
                    return (tl, ERROR);
                }
                vec![ArgSrc::Typed(tr, rty)]
            }
        };
        let lists = vec![ArgList { args, using: false, span }];
        self.apply_member(tl, lty, op, None, lists, span, expected)
    }

    /// `l op r` for a numeric `l` and an `r` that a conversion makes numeric, as `Integer2int`
    /// does for a boxed operand. The narrowest numeric type a conversion reaches wins, as the
    /// most specific alternative of the operator does under scalac.
    fn binop_with_converted_operand(
        &mut self,
        op: Name,
        tl: TExprId,
        lty: TypeId,
        tr: TExprId,
        rty: TypeId,
        span: Span,
    ) -> Option<(TExprId, TypeId)> {
        let (Some(lrank), Some(class)) = (self.is_numeric(lty), classify(op)) else { return None };
        if self.is_numeric(rty).is_some() || rty == self.b.t_string {
            return None;
        }
        for rank in [R_BYTE, R_SHORT, R_CHAR, R_INT, R_LONG, R_FLOAT, R_DOUBLE] {
            let target = self.rank_type(rank);
            // Each rank is an attempt of its own.
            let mark = self.attempt();
            if let Some(converted) = self.convert_to(tr, rty, target, span, false) {
                if converted != tr {
                    if let Binop::Done(te, ty) = self.prim_binop_typed(op, tl, lty, lrank, class, converted, target, span) {
                        self.close(mark);
                        return Some((te, ty));
                    }
                }
            }
            self.retract(mark);
        }
        None
    }

    /// `2 * x` for an `x` whose class has `*` and a conversion from the number in its implicit
    /// scope, as `BigDecimal.int2bigDecimal`: the number converted, and the operator of `x`'s class.
    fn binop_with_converted_receiver(
        &mut self,
        op: Name,
        tl: TExprId,
        lty: TypeId,
        tr: TExprId,
        rty: TypeId,
        span: Span,
        expected: Option<TypeId>,
    ) -> Option<(TExprId, TypeId)> {
        if self.is_numeric(lty).is_none() || self.is_numeric(rty).is_some() || self.find_member(rty, op).is_none() {
            return None;
        }
        let mark = self.attempt();
        if let Some(converted) = self.convert_to(tl, lty, rty, span, false) {
            if converted != tl {
                let lists = vec![ArgList { args: vec![ArgSrc::Typed(tr, rty)], using: false, span }];
                let applied = self.apply_member(converted, rty, op, None, lists, span, expected);
                self.close(mark);
                return Some(applied);
            }
        }
        self.retract(mark);
        None
    }

    /// `l op: r` is `r.op:(l)` with `l` evaluated first. An extension method `op:` takes `l` as
    /// its leading parameter and `r` as its argument, and is looked for in scope and in the
    /// implicit scope of `r`.
    fn type_right_assoc(
        &mut self,
        l: ExprId,
        op: Name,
        r: ExprId,
        span: Span,
        expected: Option<TypeId>,
    ) -> (TExprId, TypeId) {
        let mark = self.hoisted.len();
        let (tr, rty) = self.type_expr(r, None);
        // The receiver's variables that have bounds are solved, as a selection's receiver's are
        // (`type_receiver`); one nothing bounds yet stays open, so that the body of
        // `foldLeft(Nil)((acc, x) => x :: acc)` settles the element of the improved `acc:
        // List[?X]` rather than the first `::` fixing it to `Nothing`.
        let rty = self.solve_bounded_in(rty);
        if rty == ERROR || self.has_member_or_std_extension(rty, op) || (op == names::CONS_TUPLE && self.tuple_elements(rty).is_some()) {
            // The order is only observable when both operands have effects.
            let hoist = !self.is_stable(tr);
            let args = self
                .infix_args(l)
                .into_iter()
                .map(|a| match a {
                    ArgSrc::Ast(e) if hoist => ArgSrc::Hoisted(e),
                    a => a,
                })
                .collect();
            let lists = vec![ArgList { args, using: false, span }];
            let (te, ty) = self.apply_member(tr, rty, op, None, lists, span, expected);
            return (self.wrap_hoisted(mark, te), ty);
        }
        let (tl, lty) = self.type_expr(l, None);
        let lty = self.solve_in(lty);
        if lty == ERROR {
            return (self.prog.add(TExpr::Unit), ERROR);
        }
        let mut lists = Some(vec![ArgList { args: vec![ArgSrc::Typed(tr, rty)], using: false, span }]);
        if let Some(res) = self.try_extensions(tl, lty, rty, op, None, &mut lists, span, expected) {
            return res;
        }
        // `r` converted to something with the member (`LazyList`'s `#::` through `toDeferrer`).
        let member_lists = [ArgList { args: vec![ArgSrc::Typed(tl, lty)], using: false, span }];
        if let Some((te, ty)) = self.apply_through_conversion(tr, rty, op, None, &member_lists, span, expected) {
            return (self.wrap_hoisted(mark, te), ty);
        }
        let msg = format!("value {} is not a member of {}", self.name_str(op), self.show(rty));
        self.error(span, msg);
        (self.prog.add(TExpr::Unit), ERROR)
    }

    /// `f(i) op= rhs` is `f.update(i, f(i) op rhs)`, `f` and `i` evaluated once, unless the
    /// element has a member `op=` of its own.
    fn type_indexed_assign_op(
        &mut self,
        f: ExprId,
        args: ListRef,
        op: Name,
        base: Name,
        r: ExprId,
        span: Span,
        expected: Option<TypeId>,
    ) -> (TExprId, TypeId) {
        let mark = self.hoisted.len();
        let (tf, fty) = match self.indexed_target(f, args, span) {
            Ok(target) => target,
            Err((tl, lty)) => return self.assign_op_or_binop(tl, lty, op, Some(base), r, span, expected),
        };
        let fty = self.solve_in(fty);
        let tf = self.hoist(tf, fty, span);
        let mut index = Vec::new();
        for a in self.cur_ast().expr_list(args).to_vec() {
            let (ta, aty) = self.type_expr(a, None);
            let aty = self.solve_in(aty);
            let ta = self.hoist(ta, aty, span);
            index.push(ArgSrc::Typed(ta, aty));
        }
        let lists = vec![ArgList { args: index.clone(), using: false, span }];
        let (cur, cty) = self.apply_member(tf, fty, names::APPLY, None, lists, span, None);
        let cty = self.solve_in(cty);
        let cty = self.dealias(cty);
        if cty == ERROR {
            return (self.wrap_hoisted(mark, cur), ERROR);
        }
        let (te, ty) = if self.find_member(cty, op).is_some() || self.has_lexical_extension_for(cty, op) {
            let lists = vec![ArgList { args: self.infix_args(r), using: false, span }];
            self.apply_member(cur, cty, op, None, lists, span, expected)
        } else {
            let (value, vty) = self.binop_or_member(base, cur, cty, cty, r, span, None);
            index.push(ArgSrc::Typed(value, vty));
            let lists = vec![ArgList { args: index, using: false, span }];
            self.apply_member(tf, fty, names::UPDATE, None, lists, span, None)
        };
        (self.wrap_hoisted(mark, te), ty)
    }

    /// The typed `f` of `f(i) op= rhs`, or `Err` with the typed application when `f` is `q.m`
    /// for a method `m`: `q.m(i)` is then the receiver of `op=`, as `gen.map(f) >>= g` reads.
    fn indexed_target(&mut self, f: ExprId, args: ListRef, span: Span) -> Result<(TExprId, TypeId), (TExprId, TypeId)> {
        let ast = self.cur_ast();
        let Expr::Select(q, name) = ast.expr(f) else { return Ok(self.type_expr(f, None)) };
        if self.static_ref(f).is_some() {
            return Ok(self.type_expr(f, None));
        }
        let (tq, qty) = self.type_expr(q, None);
        let qty = self.solve_in(qty);
        let method = match self.find_member(qty, name) {
            Some((sym, _)) => self.syms.sym(sym).kind == SymKind::Def && self.sig_of(sym).clauses.iter().any(|c| !c.is_using),
            None => qty != ERROR && self.has_lexical_extension_for(qty, name),
        };
        if method {
            let args = ast.expr_list(args).iter().map(|&a| ArgSrc::Ast(a)).collect();
            let lists = vec![ArgList { args, using: false, span }];
            return Err(self.apply_member(tq, qty, name, None, lists, span, None));
        }
        Ok(self.apply_member(tq, qty, name, None, Vec::new(), ast.expr_span(f), None))
    }

    /// `e.x op= rhs` reads and writes `e.x`, so `e` is evaluated once.
    fn field_target_once(&mut self, tl: TExprId, span: Span) -> TExprId {
        match self.prog.expr(tl) {
            TExpr::Field(recv, s) if !self.is_stable(recv) => {
                let recv = self.hoist(recv, ANY, span);
                self.prog.add(TExpr::Field(recv, s))
            }
            _ => tl,
        }
    }

    /// The elements of a tuple literal are the arguments of an infix operation: `a op (b, c)`
    /// is `a.op(b, c)`. Which tuple they were is kept (`Worker::infix_tuples`) for a member's
    /// retry, where dotty applies a unary member to the tuple as written (`ApplyKind.InfixTuple`,
    /// Desugar.scala 1780; `needsTupledDual`, Applications.scala 1671).
    fn infix_args(&mut self, operand: ExprId) -> Vec<ArgSrc> {
        let ast = self.cur_ast();
        match ast.expr(operand) {
            Expr::Tuple(elems) => {
                let elems = ast.expr_list(elems);
                if let [first, _, ..] = *elems {
                    self.infix_tuples.insert((self.env.file, first), ());
                }
                elems.iter().map(|&e| ArgSrc::Ast(e)).collect()
            }
            _ => vec![ArgSrc::Ast(operand)],
        }
    }

    /// Strict equality holds under `--strict-equality` and in a file that imports
    /// `language.strictEquality`, as scalac switches it on per file. The std is written without
    /// CanEqual instances and never checked, as scalac's library is compiled apart.
    pub fn strict_equality(&self) -> bool {
        (self.dialect.strict_equality || self.cur_ast().strict_equality) && !self.source(self.env.file).is_std
    }

    /// An abstract type compares with `null` unless it is known to be a value type, as scalac
    /// has it without `strictEquality`: a type parameter or member whose upper bound takes `null`.
    fn compares_with_null(&mut self, t: TypeId) -> bool {
        let t = self.deref(t);
        match self.types.get(t) {
            Type::Param(p) => {
                let upper = self.syms.tparam(p).upper;
                upper == ANY || self.null_conforms(upper)
            }
            Type::Member(..) | Type::AppMember(..) | Type::Decl(_) => {
                let (_, upper) = self.member_bounds(t);
                upper == ANY || self.null_conforms(upper)
            }
            // `V[T, S]` for `V[_, _] <: Map[_, _]`: the constructor's bound applied.
            Type::AppParam(p, _) => {
                let upper = self.syms.tparam(p).upper;
                match self.types.get(upper) {
                    Type::Lambda(_, body) => self.null_conforms(body) || self.compares_with_null(body),
                    _ => upper == ANY || self.null_conforms(upper),
                }
            }
            Type::Var(_) => true,
            _ => false,
        }
    }

    fn type_equality(&mut self, l: ExprId, r: ExprId, negate: bool, span: Span) -> (TExprId, TypeId) {
        let (tl, lty) = self.type_expr(l, None);
        let (tr, rty) = self.type_expr(r, None);
        self.typed_equality(tl, lty, tr, rty, negate, span)
    }

    /// `l == r` or `l != r` over operands typed already, as the typer reads the operator in
    /// source: also a macro's reconstruction of `a != b`.
    pub fn typed_equality(&mut self, mut tl: TExprId, lty: TypeId, mut tr: TExprId, rty: TypeId, negate: bool, span: Span) -> (TExprId, TypeId) {
        let lty = self.solve_in(lty);
        let lty = self.dealias(lty);
        let rty = self.solve_in(rty);
        let rty = self.dealias(rty);
        if self.unused.on() {
            self.mark_can_equal(lty, rty);
        }
        let numeric = (self.is_numeric(lty), self.is_numeric(rty));
        if numeric.0.is_none() {
            let op = if negate { names::NEQ } else { names::EQEQ };
            if self.declares_equality_for(lty, op, rty) {
                let lists = vec![ArgList { args: vec![ArgSrc::Typed(tr, rty)], using: false, span }];
                return self.apply_member(tl, lty, op, None, lists, span, None);
            }
        }
        let mut swapped = false;
        let strict = match numeric {
            (Some(a), Some(b)) => {
                if a != b {
                    let rank = a.max(b).max(R_INT);
                    tl = self.promote(tl, lty, rank);
                    tr = self.promote(tr, rty, rank);
                }
                true
            }
            // `x == null` is the identity test; a reference type takes it, a value type does not.
            // Under strict equality an abstract type takes it through a given `CanEqual`.
            _ if lty == self.b.t_null || rty == self.b.t_null => {
                let other = if lty == self.b.t_null { rty } else { lty };
                let strict = self.strict_equality();
                let abstract_ok = if strict { self.can_equal_given(lty, rty, span) } else { self.compares_with_null(other) };
                if !self.null_conforms(other) && other != self.b.t_null && !abstract_ok {
                    let msg = format!("values of types {} and {} cannot be compared with == or !=", self.show(lty), self.show(rty));
                    self.error(span, msg);
                }
                true
            }
            _ => {
                if !self.can_equal(lty, rty, span) {
                    let msg = format!(
                        "values of types {} and {} cannot be compared with == or !=",
                        self.show(lty),
                        self.show(rty)
                    );
                    self.error(span, msg);
                }
                if self.compares_with_numbers(rty) && !self.compares_with_numbers(lty) && self.evaluates_quietly(tl) {
                    std::mem::swap(&mut tl, &mut tr);
                    swapped = true;
                }
                self.is_primitive(lty) && lty == rty
            }
        };
        let prim = match (strict, negate) {
            (true, false) => PrimOp::RefEq,
            (true, true) => PrimOp::RefNe,
            (false, false) => PrimOp::Eq,
            (false, true) => PrimOp::Ne,
        };
        let te = self.prog.add(TExpr::Prim(prim, tl, tr));
        if self.capturing() {
            let op = if negate { names::NEQ } else { names::EQEQ };
            self.capture_form(te, if swapped { Form::SwappedOp(op) } else { Form::Op(op) });
        }
        (te, self.b.t_boolean)
    }

    /// A class's own `==` or `!=` that takes the right operand, which overloads `Any`'s and is
    /// the more specific: scala-library's `SizeCompareOps.==(Int)` of `xs.sizeIs == 1`. Numbers
    /// have none and are not asked.
    fn declares_equality_for(&mut self, lty: TypeId, op: Name, rty: TypeId) -> bool {
        let Type::Class(c, _) = self.types.get(lty) else { return false };
        // `Any`'s `==` is no member: a completed class that neither declares one nor inherits
        // one is done with here.
        let declared_nowhere = self.syms.class_done(c).map_or(false, |info| {
            !info.members.contains_key(&op) && !info.base_types.iter().skip(1).any(|&(b, _)| self.syms.class(b).members.contains_key(&op))
        });
        if declared_nowhere {
            return false;
        }
        let Some((m, owner_ty)) = self.find_member(lty, op) else { return false };
        let sig = self.sig_of(m);
        // The operand's clause first, using clauses after it (`def ==(i: Int)(using Boolean)`).
        let Some((clause, rest)) = sig.clauses.split_first() else { return false };
        if clause.is_using || !rest.iter().all(|c| c.is_using) {
            return false;
        }
        let [param] = clause.params.as_slice() else { return false };
        let pty = param.ty;
        let subst = self.owner_subst(owner_ty);
        let pty = self.types.subst(pty, &subst);
        let mark = self.snapshot();
        let fits = self.is_sub(rty, pty);
        self.rollback(mark);
        fits
    }

    /// `scala.math.BigInt` and `BigDecimal`, which `BoxesRunTime.equals` lets compare themselves
    /// with a number on the left of `==`: the operands trade places, so that the class's
    /// `equals` runs where the runtime's equality would stop at the number.
    fn compares_with_numbers(&mut self, t: TypeId) -> bool {
        let Some(c) = self.class_of(t) else { return false };
        let info = self.syms.class(c);
        let Owner::Package(p) = info.owner else { return false };
        matches!(self.interner.get(info.name), "BigDecimal" | "BigInt") && self.pkg_is(p, "scala.math")
    }

    fn evaluates_quietly(&self, e: TExprId) -> bool {
        matches!(self.prog.expr(e), TExpr::Int(_) | TExpr::Long(_) | TExpr::Double(_) | TExpr::Char(_) | TExpr::Local(_) | TExpr::Static(_))
    }

    /// scalac's rule for `l == r`. By default the types have to be related by subtyping, share a
    /// standard `CanEqual` instance, or both lack a reflexive one; with `--strict-equality` a
    /// `CanEqual[L, R]` has to exist: predefined for primitives and the standard types, given
    /// for everything else.
    fn can_equal(&mut self, l: TypeId, r: TypeId, span: Span) -> bool {
        match self.can_equal_predefined(l, r, span) {
            Some(known) => known,
            None => self.can_equal_given(l, r, span),
        }
    }

    /// The part of the rule that needs no given: `Some(true)` when the instance is predefined,
    /// `Some(false)` when a standard type rules the comparison out, `None` when a given decides.
    fn can_equal_predefined(&mut self, l: TypeId, r: TypeId, span: Span) -> Option<bool> {
        let l = self.deref(l);
        let r = self.deref(r);
        if l == ERROR || r == ERROR || l == NOTHING || r == NOTHING {
            return Some(true);
        }
        if self.types.has_vars(l) || self.types.has_vars(r) {
            return Some(true);
        }
        if self.is_numeric(l).is_some() && self.is_numeric(r).is_some() {
            return Some(true);
        }
        if self.is_numeric(l).is_some() && self.is_boxed_number(r) || self.is_numeric(r).is_some() && self.is_boxed_number(l) {
            return Some(true);
        }
        if self.is_box_of(l, r) || self.is_box_of(r, l) {
            return Some(true);
        }
        let strict = self.strict_equality();
        if l == r && (!strict || self.is_primitive(l)) {
            return Some(true);
        }
        if !strict {
            let mark = self.snapshot();
            let related = self.is_sub(l, r) || self.is_sub(r, l);
            self.rollback(mark);
            if related || self.mentions_param(l) || self.mentions_param(r) {
                return Some(true);
            }
        }
        if (self.is_primitive(l) || self.is_primitive(r)) && strict {
            return Some(false);
        }
        if let Some(standard) = self.std_can_equal(l, r, span) {
            return Some(standard);
        }
        if !strict && !self.has_reflexive_can_equal(l) && !self.has_reflexive_can_equal(r) {
            return Some(true);
        }
        None
    }

    /// A class below `java.lang.Number` (`BigInt`, `BigDecimal`), which scalac compares with any
    /// numeric primitive.
    /// Whether `boxed` is the `java.lang` class of the primitive `prim` that is no number.
    fn is_box_of(&mut self, prim: TypeId, boxed: TypeId) -> bool {
        let name = if prim == self.b.t_boolean {
            "Boolean"
        } else if prim == self.b.t_char {
            "Character"
        } else {
            return false;
        };
        let class = self.class_of(boxed);
        class.is_some() && class == self.java_lang_class(name)
    }

    fn is_boxed_number(&mut self, t: TypeId) -> bool {
        let Some(number) = self.b.number else { return false };
        self.class_of(t).map_or(false, |c| c == number || self.syms.class(c).base_types.iter().any(|&(b, _)| b == number))
    }

    /// `CanEqual.derived` for a `CanEqual[L, R]` that the rule grants without a given, which is
    /// what scalac synthesizes for a using clause or `summon`.
    pub fn synthesized_can_equal(&mut self, c: ClassId, target: TypeId, span: Span) -> Option<TExprId> {
        if Some(c) != self.b.can_equal {
            return None;
        }
        let Type::Class(_, args) = self.types.get(target) else { return None };
        let &[l, r] = self.types.items(args) else { return None };
        if self.can_equal_predefined(l, r, span) != Some(true) {
            return None;
        }
        let module = self.syms.class(c).companion?;
        let derived = self.interner.intern("derived");
        let derived = self.module_term(module, derived)?.sym()?;
        let recv = self.prog.add(TExpr::Module(module));
        Some(self.prog.add(TExpr::CallMethod(recv, derived, ListRef::EMPTY)))
    }

    /// The standard instances: `Option`, `Seq` and `Set` compare regardless of their elements
    /// unless equality is strict; `Either`, `Map` and tuples compare element by element.
    fn std_can_equal(&mut self, l: TypeId, r: TypeId, span: Span) -> Option<bool> {
        let b = &self.b;
        let loose = [b.option, b.seq, b.set];
        let elementwise = [b.either, b.map];
        for class in loose.into_iter().chain(elementwise).flatten() {
            let (Some(lb), Some(rb)) = (self.base_type(l, class), self.base_type(r, class)) else { continue };
            if !self.strict_equality() && loose.contains(&Some(class)) {
                return Some(true);
            }
            return Some(self.args_can_equal(lb, rb, span));
        }
        let (Some(lc), Some(rc)) = (self.class_of(l), self.class_of(r)) else { return None };
        if self.is_tuple_class(lc) && self.is_tuple_class(rc) {
            return Some(lc == rc && self.args_can_equal(l, r, span));
        }
        None
    }

    fn args_can_equal(&mut self, l: TypeId, r: TypeId, span: Span) -> bool {
        let (Type::Class(_, la), Type::Class(_, ra)) = (self.types.get(l), self.types.get(r)) else { return true };
        let pairs: Vec<(TypeId, TypeId)> =
            self.types.items(la).iter().copied().zip(self.types.items(ra).iter().copied()).collect();
        pairs.into_iter().all(|(a, b)| self.can_equal(a, b, span))
    }

    /// Whether `CanEqual[T, T]` exists: for primitives, the standard types and a class whose
    /// companion holds a `CanEqual` given (`derives CanEqual`).
    fn has_reflexive_can_equal(&mut self, t: TypeId) -> bool {
        if self.is_primitive(t) {
            return true;
        }
        let Some(c) = self.class_of(t) else { return false };
        if self.is_tuple_class(c) {
            return true;
        }
        let b = &self.b;
        let standard = [b.option, b.seq, b.set, b.map, b.either];
        if standard.into_iter().flatten().any(|s| self.base_type(t, s).is_some()) {
            return true;
        }
        let Some(can_equal) = self.can_equal_class() else { return false };
        let info = self.syms.class(c);
        let module = if info.kind == ClassKind::Object { Some(c) } else { info.companion };
        let Some(module) = module else { return false };
        let givens = self.syms.class(module).givens.clone();
        givens.into_iter().any(|g| {
            let ret = self.sig_of(g).ret;
            self.class_of(ret) == Some(can_equal)
        })
    }

    fn can_equal_given(&mut self, l: TypeId, r: TypeId, span: Span) -> bool {
        let Some(can_equal) = self.can_equal_class() else { return true };
        let target = self.types.class(can_equal, &[l, r]);
        let found = self.resolve_given(target, span).is_some();
        self.given_ambiguity = None;
        found
    }

    pub(super) fn mentions_param(&self, t: TypeId) -> bool {
        self.mentions_tparam_of(t, None)
    }

    /// Whether `t` names a type parameter: any one, or one of `tps` when given.
    pub fn mentions_tparam_of(&self, t: TypeId, tps: Option<&[TParamId]>) -> bool {
        match self.types.get(t) {
            Type::Param(p) | Type::AppParam(p, _) => tps.map_or(true, |tps| tps.contains(&p)),
            Type::Class(_, args) => self.types.items(args).iter().any(|&a| self.mentions_tparam_of(a, tps)),
            Type::Lambda(_, b) => self.mentions_tparam_of(b, tps),
            Type::Union(a, b) | Type::Inter(a, b) => {
                self.mentions_tparam_of(a, tps) || self.mentions_tparam_of(b, tps)
            }
            _ => false,
        }
    }

    /// Built-in binary operators on primitives.
    fn prim_binop(&mut self, op: Name, tl: TExprId, lty: TypeId, r: ExprId, span: Span) -> Binop {
        let b_string = self.b.t_string;
        let b_bool = self.b.t_boolean;
        if lty == b_string {
            if op == names::PLUS {
                let (tr, rty) = self.type_expr(r, None);
                let rty = self.solve_in(rty);
                let rs = self.to_str(tr, rty);
                let l = self.prog.list(&[tl, rs]);
                return Binop::Done(self.prog.add(TExpr::StrConcat(l)), b_string);
            }
            if classify(op) == Some(OpClass::Compare) {
                let tr = self.check_expr(r, b_string);
                return Binop::Done(self.prog.add(TExpr::Prim(compare_op(op), tl, tr)), b_bool);
            }
            return Binop::NotBuiltin;
        }
        if lty == b_bool {
            let prim = match op {
                names::AMP => PrimOp::BoolStrictAnd,
                names::BAR => PrimOp::BoolStrictOr,
                names::CARET => PrimOp::BoolXor,
                _ => return Binop::NotBuiltin,
            };
            let tr = self.check_expr(r, b_bool);
            return Binop::Done(self.prog.add(TExpr::Prim(prim, tl, tr)), b_bool);
        }
        let (Some(lrank), Some(class)) = (self.is_numeric(lty), classify(op)) else { return Binop::NotBuiltin };
        let literal_hint = if self.is_const_expr(r) && lrank >= R_LONG { Some(lty) } else { None };
        let (tr, operand_ty) = self.type_expr(r, literal_hint);
        self.prim_binop_typed(op, tl, lty, lrank, class, tr, operand_ty, span)
    }

    /// `l op r` as a builtin operator over operands typed already (a macro's reconstruction of
    /// `n * 2`), or `None` where `op` is no builtin operator of the left type.
    pub fn typed_binop(&mut self, op: Name, tl: TExprId, lty: TypeId, tr: TExprId, rty: TypeId, span: Span) -> Option<(TExprId, TypeId)> {
        let (b_string, b_bool) = (self.b.t_string, self.b.t_boolean);
        let lty = self.widen_path(lty);
        let lty = self.widen_lit(lty);
        let rty = self.widen_path(rty);
        let rty = self.widen_lit(rty);
        if lty == b_string {
            if op == names::PLUS {
                let rs = self.to_str(tr, rty);
                let l = self.prog.list(&[tl, rs]);
                return Some((self.prog.add(TExpr::StrConcat(l)), b_string));
            }
            if classify(op) == Some(OpClass::Compare) && self.deref(rty) == b_string {
                return Some((self.prog.add(TExpr::Prim(compare_op(op), tl, tr)), b_bool));
            }
            return None;
        }
        if lty == b_bool && self.deref(rty) == b_bool {
            let prim = match op {
                names::AMP => PrimOp::BoolStrictAnd,
                names::BAR => PrimOp::BoolStrictOr,
                names::AMPAMP => PrimOp::BoolAnd,
                names::BARBAR => PrimOp::BoolOr,
                names::CARET => PrimOp::BoolXor,
                _ => return None,
            };
            return Some((self.prog.add(TExpr::Prim(prim, tl, tr)), b_bool));
        }
        let (Some(lrank), Some(class)) = (self.is_numeric(lty), classify(op)) else { return None };
        match self.prim_binop_typed(op, tl, lty, lrank, class, tr, rty, span) {
            Binop::Done(te, ty) => Some((te, ty)),
            _ => None,
        }
    }

    fn prim_binop_typed(
        &mut self,
        op: Name,
        tl: TExprId,
        lty: TypeId,
        lrank: u8,
        class: OpClass,
        tr: TExprId,
        operand_ty: TypeId,
        span: Span,
    ) -> Binop {
        let b_string = self.b.t_string;
        let b_bool = self.b.t_boolean;
        let operand_ty = self.solve_in(operand_ty);
        let rty = self.dealias(operand_ty);
        let rty = self.numeric_bound(rty).unwrap_or(rty);
        if rty == b_string && op == names::PLUS {
            let ls = self.to_str(tl, lty);
            let l = self.prog.list(&[ls, tr]);
            return Binop::Done(self.prog.add(TExpr::StrConcat(l)), b_string);
        }
        let Some(rrank) = self.is_numeric(rty) else { return Binop::Operand(tr, operand_ty) };
        if class == OpClass::Compare && lrank == R_CHAR && rrank == R_CHAR {
            return Binop::Done(self.prog.add(TExpr::Prim(compare_op(op), tl, tr)), b_bool);
        }
        let rank = if class == OpClass::Shift { lrank.max(R_INT) } else { lrank.max(rrank).max(R_INT) };
        let fractional = if class == OpClass::Shift { rank.max(rrank) } else { rank };
        if matches!(class, OpClass::Bits | OpClass::Shift) && fractional >= R_FLOAT {
            let msg = format!("operator {} cannot be applied to {}", self.name_str(op), self.show(self.rank_type(fractional)));
            self.error(span, msg);
            return Binop::Done(tl, ERROR);
        }
        let a = self.promote(tl, lty, rank);
        let b = if class == OpClass::Shift {
            let as_int = self.convert_rank(tr, rty, rrank.max(R_INT));
            let count = match (rrank.max(R_INT), rank) {
                (R_INT, R_LONG) => self.prog.add(TExpr::Unary(UnOp::IntToLong, as_int)),
                (R_LONG, R_INT) => self.prog.add(TExpr::Unary(UnOp::LongToInt, as_int)),
                _ => as_int,
            };
            if self.capturing() {
                self.capture_conversion(count, tr, Form::Promotion);
            }
            count
        } else {
            self.promote(tr, rty, rank)
        };
        if class == OpClass::Compare {
            return Binop::Done(self.prog.add(TExpr::Prim(compare_op(op), a, b)), b_bool);
        }
        match arith_op(op, rank) {
            Some(prim) => Binop::Done(self.prog.add(TExpr::Prim(prim, a, b)), self.rank_type(rank)),
            None => Binop::Operand(tr, operand_ty),
        }
    }

    pub fn type_prefix(&mut self, op: Name, operand: ExprId, span: Span) -> (TExprId, TypeId) {
        let (te, ty) = self.type_expr(operand, None);
        let ty = self.solve_in(ty);
        self.prefix_on(op, te, ty, span)
    }

    /// The operators a builtin type has without a member symbol: `!` and the boolean
    /// connectives, the arithmetic, comparison, bit and shift operators of a number, `+` and
    /// the comparisons of a String.
    pub(super) fn prim_op_applies(&mut self, t: TypeId, op: Name) -> bool {
        let t = self.dealias(t);
        if t == self.b.t_boolean {
            return matches!(op, names::UNARY_BANG | names::AMPAMP | names::BARBAR | names::AMP | names::BAR | names::CARET);
        }
        if t == self.b.t_string {
            return op == names::PLUS || classify(op) == Some(OpClass::Compare);
        }
        match self.is_numeric(t) {
            Some(rank) => match op {
                names::UNARY_MINUS | names::UNARY_PLUS => true,
                names::UNARY_TILDE => rank < R_FLOAT,
                _ => classify(op).is_some() || NUMERIC_CONVERSIONS.contains(&self.name_ref(op)),
            },
            None => false,
        }
    }

    /// Whether a receiver of type `t` is a `String`, through an alias, a literal type or an
    /// intersection.
    pub(super) fn stands_for_string(&mut self, t: TypeId) -> bool {
        let t = self.primitive_side(t).unwrap_or(t);
        let t = self.dealias(t);
        self.widen_lit(t) == self.b.t_string
    }

    /// The argument of `String.+` passed by its name, `x$0`, as the one it names.
    pub(super) fn string_plus_argument(&mut self, mut lists: Vec<ArgList>) -> Vec<ArgList> {
        if let [list] = lists.as_mut_slice() {
            if let [ArgSrc::Ast(arg)] = list.args.as_mut_slice() {
                if let Expr::NamedArg(n, value) = self.cur_ast().expr(*arg) {
                    if self.name_ref(n) == "x$0" {
                        *arg = value;
                    }
                }
            }
        }
        lists
    }

    /// `name` on a receiver that a conversion has just made a builtin: the operator the
    /// builtin has instead of a member, or nothing when `name` is no such operator.
    pub fn prim_op_on(
        &mut self,
        te: TExprId,
        ty: TypeId,
        name: Name,
        targs: Option<ListRef>,
        lists: &[ArgList],
        span: Span,
        expected: Option<TypeId>,
    ) -> Option<(TExprId, TypeId)> {
        let ty = self.primitive_side(ty).unwrap_or(ty);
        if targs.is_some() || !self.prim_op_applies(ty, name) {
            return None;
        }
        if let Some(converted) = self.prim_member(te, ty, name, lists) {
            return Some(converted);
        }
        if let Some(op) = prefix_op_of(name) {
            return lists.is_empty().then(|| self.prefix_on(op, te, ty, span));
        }
        let [list] = lists else { return None };
        let (false, [ArgSrc::Ast(r)]) = (list.using, &list.args[..]) else { return None };
        let r = *r;
        if matches!(name, names::AMPAMP | names::BARBAR) {
            let tr = self.check_right_operand(name, te, r);
            let prim = if name == names::AMPAMP { PrimOp::BoolAnd } else { PrimOp::BoolOr };
            return Some((self.prog.add(TExpr::Prim(prim, te, tr)), self.b.t_boolean));
        }
        Some(self.binop_or_member(name, te, ty, ty, r, span, expected))
    }

    fn prefix_on(&mut self, op: Name, te: TExprId, ty: TypeId, span: Span) -> (TExprId, TypeId) {
        let ty = self.dealias(ty);
        let ty = self.widen_lit(ty);
        let b = &self.b;
        let (t_int, t_long, t_double, t_float, t_bool) = (b.t_int, b.t_long, b.t_double, b.t_float, b.t_boolean);
        let unop = match op {
            names::MINUS if ty == t_int => Some((UnOp::IntNeg, t_int)),
            names::MINUS if ty == t_long => Some((UnOp::LongNeg, t_long)),
            names::MINUS if ty == t_double => Some((UnOp::DoubleNeg, t_double)),
            names::MINUS if ty == t_float => Some((UnOp::FloatNeg, t_float)),
            names::BANG if ty == t_bool => Some((UnOp::BoolNot, t_bool)),
            names::TILDE if ty == t_int => Some((UnOp::IntNot, t_int)),
            names::TILDE if ty == t_long => Some((UnOp::LongNot, t_long)),
            _ => None,
        };
        if let Some((u, t)) = unop {
            // scalac's constant folding: `!true` is `false` (`!LinkingInfo.productionMode`).
            if let (UnOp::BoolNot, TExpr::Bool(v)) = (u, self.prog.expr(te)) {
                let folded = self.prog.add(TExpr::Bool(!v));
                if self.capturing() {
                    self.capture_form(folded, Form::Folded(names::UNARY_BANG));
                }
                return (folded, t);
            }
            return (self.prog.add(TExpr::Unary(u, te)), t);
        }
        // The narrower types are promoted to Int first.
        if matches!(op, names::MINUS | names::TILDE) && matches!(self.is_numeric(ty), Some(R_BYTE | R_SHORT | R_CHAR)) {
            let as_int = self.promote(te, ty, R_INT);
            let u = if op == names::MINUS { UnOp::IntNeg } else { UnOp::IntNot };
            return (self.prog.add(TExpr::Unary(u, as_int)), t_int);
        }
        if op == names::PLUS && self.is_numeric(ty).is_some() {
            if self.capturing() {
                self.capture_wrap(te, Wrap::Member(names::UNARY_PLUS, span));
            }
            let file = self.env.file;
            if let Some(ix) = self.index.as_mut() {
                ix.identity_prefixes.insert((file, span.start), ());
            }
            return (te, ty);
        }
        let method = match op {
            names::MINUS => names::UNARY_MINUS,
            names::BANG => names::UNARY_BANG,
            names::TILDE => names::UNARY_TILDE,
            _ => names::UNARY_PLUS,
        };
        self.apply_member(te, ty, method, None, Vec::new(), span, None)
    }

    /// Conversions between primitives.
    pub fn prim_member(
        &mut self,
        recv: TExprId,
        recv_ty: TypeId,
        name: Name,
        lists: &[ArgList],
    ) -> Option<(TExprId, TypeId)> {
        if !lists.is_empty() {
            return None;
        }
        let rank = self.is_numeric(recv_ty)?;
        let text = self.name_ref(name);
        let to = match text {
            "toByte" => R_BYTE,
            "toShort" => R_SHORT,
            "toChar" => R_CHAR,
            "toInt" => R_INT,
            "toLong" => R_LONG,
            "toFloat" => R_FLOAT,
            "toDouble" => R_DOUBLE,
            // The members of java.lang.Number, which a boxed number has under scalac.
            "byteValue" if rank != R_CHAR => R_BYTE,
            "shortValue" if rank != R_CHAR => R_SHORT,
            "intValue" if rank != R_CHAR => R_INT,
            "longValue" if rank != R_CHAR => R_LONG,
            "floatValue" if rank != R_CHAR => R_FLOAT,
            "doubleValue" if rank != R_CHAR => R_DOUBLE,
            _ => return None,
        };
        Some((self.convert_rank(recv, recv_ty, to), self.rank_type(to)))
    }

    /// `recv.asInstanceOf[t]` for a receiver of type `recv_ty`, lowered as dotty's erasure lowers
    /// a cast (`TypeTestsCasts.transformAsInstanceOf`, `cast_lowering`); a cast of a stored inline
    /// body or of a quote is kept as written, its types filled in where it is instantiated.
    pub(super) fn lower_cast(&mut self, recv: TExprId, recv_ty: TypeId, t: TypeId, span: Span) -> (TExprId, TypeId) {
        if self.quote.level > 0 || self.checks_inline_definition() {
            return self.cast_node(recv, CastOp::Written, t);
        }
        let lowering = self.cast_lowering(recv_ty, t);
        if lowering == CastLowering::Fail {
            // `tpd.primitiveConversion`'s warning, where scalac's erasure meets the cast.
            let from = self.dealias(recv_ty);
            let (from, to) = (self.widen_lit(from), self.dealias(t));
            let msg = format!("conversion from {} to {} will always fail at runtime.", self.show(from), self.show(to));
            self.warn(span, msg);
        }
        let lowering = self.untested_lowering(recv, lowering);
        self.lower_cast_as(recv, recv_ty, t, lowering)
    }

    /// The tests a condition that holds passes, of stable locals, put in scope for the branch it
    /// guards until `pop_tested`: the conjuncts of `&&` and `&` too.
    pub(super) fn push_tested(&mut self, cond: TExprId) -> usize {
        let mark = self.tested.len();
        let mut work = vec![cond];
        while let Some(c) = work.pop() {
            match self.prog.expr(c) {
                TExpr::TypeTest(x, test) => {
                    if let TExpr::Local(s) = self.prog.expr(x) {
                        let info = self.syms.sym(s);
                        if matches!(info.kind, SymKind::Val | SymKind::Param) && !info.by_name {
                            self.tested.push((s, test));
                        }
                    }
                }
                TExpr::Prim(PrimOp::BoolAnd | PrimOp::BoolStrictAnd, a, b) => work.extend([a, b]),
                _ => {}
            }
        }
        mark
    }

    pub(super) fn pop_tested(&mut self, mark: usize) {
        self.tested.truncate(mark);
    }

    /// The right operand of a Boolean's `&&` or `||`, `&&`'s typed where its left one passed
    /// (`that.isInstanceOf[C] && that.asInstanceOf[C].x == x`).
    fn check_right_operand(&mut self, op: Name, left: TExprId, r: ExprId) -> TExprId {
        let mark = if op == names::AMPAMP { self.push_tested(left) } else { self.tested.len() };
        let tr = self.check_expr(r, self.b.t_boolean);
        self.pop_tested(mark);
        tr
    }

    /// A cast that the condition around it proved, a test of the same stable local for the same
    /// erasure having passed, is the value itself: its check cannot fail, as a pattern's binder
    /// after the pattern's test is the value tested.
    pub(super) fn untested_lowering(&self, recv: TExprId, lowering: CastLowering) -> CastLowering {
        let (CastLowering::Op(CastOp::Check(test, _) | CastOp::Unbox(test, _)), TExpr::Local(s)) = (lowering, self.prog.expr(recv)) else { return lowering };
        if self.tested.iter().any(|&(t, passed)| t == s && self.same_test(passed, test)) {
            CastLowering::Same
        } else {
            lowering
        }
    }

    /// Whether two tests take the same values.
    fn same_test(&self, a: TestId, b: TestId) -> bool {
        match (self.prog.tests[a.idx()], self.prog.tests[b.idx()]) {
            (TypeTest::Or(a1, a2), TypeTest::Or(b1, b2)) | (TypeTest::And(a1, a2), TypeTest::And(b1, b2)) => self.same_test(a1, b1) && self.same_test(a2, b2),
            (TypeTest::Value(_), _) | (_, TypeTest::Value(_)) => false,
            (x, y) => x == y,
        }
    }

    /// The node of `recv.asInstanceOf[t]` once `cast_lowering` decided it: the receiver retyped
    /// for a cast its erasure makes redundant (dotty's `Typed`), marked as a cast written, which
    /// is no path and no constant and does something in statement position (`TreeInfo.exprPurity`
    /// of a `TypeApply`); the receiver then `()` for `Unit`; the numeric conversion; the zero of
    /// the primitive for a `null` literal unboxed; else the cast's own node.
    pub(super) fn lower_cast_as(&mut self, recv: TExprId, recv_ty: TypeId, t: TypeId, lowering: CastLowering) -> (TExprId, TypeId) {
        match lowering {
            CastLowering::Same => {
                *self.expr_marks.entry(recv).or_default() |= super::MARK_CAST | super::MARK_CALL | super::MARK_RETYPED;
                self.end_chain(recv, t);
                if self.capturing() {
                    self.capture_wrap(recv, Wrap::Cast(t));
                }
                (recv, t)
            }
            CastLowering::Unit => {
                // `Erasure.Boxing.constant`: the operand's effects, if it has any, then `()`; a
                // `null` literal is `()` alone, its zero.
                let unit = self.prog.add(TExpr::Unit);
                let te = if matches!(self.prog.expr(recv), TExpr::Null) {
                    *self.expr_marks.entry(unit).or_default() |= super::MARK_CAST;
                    unit
                } else {
                    let l = self.prog.stmts.push_slice(&[TStmt::Expr(recv)]);
                    self.prog.add(TExpr::Block(l, unit))
                };
                *self.expr_marks.entry(te).or_default() |= super::MARK_CALL;
                if self.capturing() {
                    self.capture_form(te, Form::Cast(t));
                }
                (te, t)
            }
            CastLowering::Convert(to) => {
                let converted = self.convert_rank(recv, recv_ty, to);
                *self.expr_marks.entry(converted).or_default() |= super::MARK_CALL;
                if self.capturing() {
                    self.capture_conversion(converted, recv, Form::Cast(t));
                }
                (converted, self.rank_type(to))
            }
            CastLowering::Fail => {
                // `Throw(New(ClassCastException))` of `tpd.primitiveConversion`, the operand
                // dropped: no message, and nothing of the operand runs.
                let Some(cce) = self.class_at(&["java", "lang", "ClassCastException"]) else {
                    return (recv, t);
                };
                let null = self.prog.add(TExpr::Null);
                self.prog.set_type(null, self.b.t_null);
                let l = self.prog.list(&[null]);
                let made = self.prog.add(TExpr::New(cce, l));
                self.prog.set_type(made, self.types.class(cce, &[]));
                let te = self.prog.add(TExpr::Throw(made, false));
                *self.expr_marks.entry(te).or_default() |= super::MARK_CALL;
                if self.capturing() {
                    self.capture_conversion(te, recv, Form::Cast(t));
                }
                (te, t)
            }
            CastLowering::Op(CastOp::Unbox(test, _)) if matches!(self.prog.expr(recv), TExpr::Null) => {
                // `null.asInstanceOf[Int]`, `BoxesRunTime.unboxToInt(null)`: the zero, the literal
                // having no effect to keep (`Erasure.Boxing.constant`).
                let zero = self.zero_of_test(test);
                self.prog.set_type(zero, t);
                *self.expr_marks.entry(zero).or_default() |= super::MARK_CAST | super::MARK_CALL;
                if self.capturing() {
                    self.capture_form(zero, Form::Cast(t));
                }
                (zero, t)
            }
            CastLowering::Op(op) => self.cast_node(recv, op, t),
        }
    }

    fn cast_node(&mut self, recv: TExprId, op: CastOp, t: TypeId) -> (TExprId, TypeId) {
        let te = self.prog.add(TExpr::Cast(recv, op, t));
        self.prog.set_type(te, t);
        self.end_chain(te, t);
        if self.capturing() {
            self.capture_form(te, Form::Cast(t));
        }
        (te, t)
    }

    /// What `x.asInstanceOf[to]` of an `x` of type `from` is at run time, decided on the erased
    /// types as `TypeTestsCasts.transformAsInstanceOf` decides it (TypeTestsCasts.scala 281 to
    /// 308): the value itself where `from`'s erasure conforms to `to`'s (an ascription, 287 to
    /// 288; to `Unit` the ascription is adapted to it, the value then `()`); the value then `()`
    /// where `to` erases to the box of `()` (289 to 291); a `ClassCastException` for `Nothing`
    /// (299 to 304); for a source of a primitive class, `Unit` among them (`foundClasses`, 283 to
    /// 285), the conversion to a primitive (`tpd.primitiveConversion`), a failure where it has
    /// none, else the test of its box (292 to 294); a reference's unboxing to a primitive
    /// (`Erasure.Boxing.unbox`, 295 to 296); and else the test of `to`'s erasure, which `null`
    /// passes (`checkcast`, 306 to 307). `Nothing` comes before the primitive branch, where scalac
    /// tests it after: its box's cast to `Nothing` would be a test of `Nothing$`, which a type
    /// test cannot make here (`test_for`), so a primitive operand fails with `Nothing`'s own
    /// message. A value class is tested as its box, which it always is outside the JVM; a
    /// JavaScript type is never tested, as under Scala.js.
    pub(super) fn cast_lowering(&mut self, from: TypeId, to: TypeId) -> CastLowering {
        use super::site::Erased;
        if from == ERROR || to == ERROR {
            return CastLowering::Same;
        }
        let target = self.cast_erasure(to);
        let source = self.cast_erasure(from);
        let unit = Erased::Of(self.b.unit, 0);
        // An ascription to `Unit` is adapted to the primitive (`Erasure.Typer.typedTyped`,
        // `Boxing.adaptToType`): the value then `()`, `null` included.
        if self.dealias(to) == self.b.t_unit {
            return if source == unit { CastLowering::Same } else { CastLowering::Unit };
        }
        if self.erased_conforms(source, target) {
            return CastLowering::Same;
        }
        if target == unit {
            return CastLowering::Unit;
        }
        if target == Erased::Nothing(0) {
            return CastLowering::Op(CastOp::Nothing);
        }
        let primitive_target = match target {
            Erased::Of(c, 0) if self.is_primitive_class(c) => Some(c),
            _ => None,
        };
        let erased = self.erased_type(target);
        match (self.found_primitive(from), primitive_target) {
            (Some(f), Some(t)) => {
                let (from_ty, to_ty) = (self.types.class(f, &[]), self.types.class(t, &[]));
                return match (self.is_numeric(from_ty), self.is_numeric(to_ty)) {
                    (Some(_), Some(rank)) => CastLowering::Convert(rank),
                    _ => CastLowering::Fail,
                };
            }
            (None, Some(_)) => {
                let test = self.test_for(erased, from, Span::default(), true);
                return CastLowering::Op(CastOp::Unbox(test, erased));
            }
            _ => {}
        }
        if let Erased::Of(c, 0) = target {
            if self.syms.class(c).js != JsKind::Scala {
                return CastLowering::Same;
            }
        }
        let test = self.test_for(erased, from, Span::default(), true);
        CastLowering::Op(CastOp::Check(test, erased))
    }

    /// The primitive class of a cast's source, `Unit` among them, where its type has one class
    /// (`TypeTestsCasts.foundClasses` of the widened source, 283 to 285, before erasure): an
    /// abstract type's by its bound, a union of two classes none, an intersection the class of
    /// the part that is not erased to `Object`.
    fn found_primitive(&mut self, t: TypeId) -> Option<ClassId> {
        self.found_primitive_at(t, 0)
    }

    fn found_primitive_at(&mut self, t: TypeId, depth: u32) -> Option<ClassId> {
        if depth > 32 {
            return None;
        }
        let t = self.deref_alias(t);
        match self.types.get(t) {
            Type::Class(c, _) if self.is_primitive_class(c) || c == self.b.unit => Some(c),
            Type::Lit(_) => self.found_primitive_at(self.widen_lit(t), depth + 1),
            Type::Param(p) => {
                let hi = self.syms.tparam(p).upper;
                if hi == t { None } else { self.found_primitive_at(hi, depth + 1) }
            }
            Type::Member(..) | Type::AppMember(..) | Type::Decl(_) => {
                let (_, hi) = self.member_bounds(t);
                if hi == t { None } else { self.found_primitive_at(hi, depth + 1) }
            }
            Type::Inter(a, b) => {
                let object = super::site::Erased::Of(self.b.any_ref, 0);
                match (self.found_primitive_at(a, depth + 1), self.found_primitive_at(b, depth + 1)) {
                    (Some(x), Some(y)) if x == y => Some(x),
                    (Some(x), None) if self.bound_erasure(b, 0) == Some(object) => Some(x),
                    (None, Some(y)) if self.bound_erasure(a, 0) == Some(object) => Some(y),
                    _ => None,
                }
            }
            Type::Term(_) | Type::Select(..) | Type::This(_) | Type::Refined(..) => match self.dependent_underlying(t) {
                Some(u) if u != t => self.found_primitive_at(u, depth + 1),
                _ => None,
            },
            _ => None,
        }
    }

    /// The erasure of a cast's type (`TypeErasure.erasure`, `bound_erasure`), `Object` where it has
    /// none.
    fn cast_erasure(&mut self, t: TypeId) -> super::site::Erased {
        self.bound_erasure(t, 0).unwrap_or(super::site::Erased::Of(self.b.any_ref, 0))
    }

    /// Whether the cast of a value of the erasure `from` to `to` is the value itself with nothing
    /// done: dotty's `erasure(expr.tpe) <:< testType` (TypeTestsCasts.scala 287), the strict
    /// relation (`erased_sub`), and a primitive's cast to `Object`, its box's (292 to 294), which
    /// passes whatever the value.
    fn erased_conforms(&mut self, from: super::site::Erased, to: super::site::Erased) -> bool {
        use super::site::Erased;
        let primitive = matches!(from, Erased::Of(a, 0) if self.is_primitive_class(a));
        self.erased_sub(from, to) || (primitive && to == Erased::Of(self.b.any_ref, 0))
    }

    /// Erased subtyping (`TypeComparer` on erased types): `Nothing` is the bottom, `Null` below
    /// every reference, `Object` above every reference but no primitive; an array of `Nothing`
    /// is an array, two arrays conform as their elements do and an array to `Object` alone
    /// (TypeComparer.scala 1119 to 1122, `JavaArrayType`), so `int[]` is no `Object[]`; an array
    /// of `Nothing` is none of a primitive, as its representation has it (below).
    fn erased_sub(&mut self, from: super::site::Erased, to: super::site::Erased) -> bool {
        use super::site::Erased;
        let object = self.b.any_ref;
        match (from, to) {
            (Erased::Nothing(0), _) => true,
            (Erased::Of(a, 0), Erased::Of(c, 0)) if c == object => !self.is_primitive_class(a),
            (_, Erased::Of(c, 0)) if c == object => true,
            (Erased::Null(0), Erased::Null(0)) => true,
            (Erased::Null(0), Erased::Of(c, d)) => d > 0 || !self.is_primitive_class(c),
            _ if from.dims() > 0 && to.dims() > 0 => {
                // An array of `Nothing` is a `Nothing$[]` at run time, an array of references: the
                // comparer takes `Nothing <: Int`, but the ascription it makes is adapted at erasure
                // from the tree's own array type (`Erasure.Boxing.adaptToType`, then `cast`, Erasure
                // .scala 333), which is no primitive array, so scalac's output checks and throws.
                let (f, t) = (from.element(), to.element());
                if f == Erased::Nothing(0) && matches!(t, Erased::Of(c, 0) if self.is_primitive_class(c)) {
                    return false;
                }
                self.erased_sub(f, t)
            }
            (Erased::Of(a, 0), Erased::Of(b, 0)) => {
                a == b
                    || (self.function_xxl(a) && self.function_xxl(b))
                    || (!self.is_primitive_class(a) && !self.is_primitive_class(b) && (self.derives_from(a, b) || self.product_by_rule(a, b)))
            }
            _ => false,
        }
    }

    /// Whether `c` is a function class past 22 parameters, whose erasure is `FunctionXXL`
    /// (`Definitions.functionTypeErasure`), one class for every such arity.
    fn function_xxl(&self, c: ClassId) -> bool {
        self.is_function_class(c) && self.syms.class(c).tparams.len() > 23
    }

    /// Whether `b` is a parent a product by rule has (`Symbols::is_synthetic_parent`: `Product`,
    /// `Equals`, `java.io.Serializable`) and `a` such a product, which extends none of them.
    fn product_by_rule(&mut self, a: ClassId, b: ClassId) -> bool {
        self.syms.is_synthetic_parent(&self.interner, b) && {
            self.complete_class(a);
            let info = self.syms.class(a);
            info.is_product_by_rule() || info.base_types.iter().any(|&(x, _)| self.syms.class(x).is_product_by_rule())
        }
    }

    /// The type an erasure stands for, which its test is made of.
    fn erased_type(&mut self, e: super::site::Erased) -> TypeId {
        use super::site::Erased;
        let (mut t, dims) = match e {
            Erased::Of(c, d) => (self.types.class(c, &[]), d),
            Erased::Null(d) => (self.b.t_null, d),
            Erased::Nothing(d) => (NOTHING, d),
        };
        for _ in 0..dims {
            t = self.types.class(self.b.array, &[t]);
        }
        t
    }

    /// The zero of the primitive an unboxing's test is of: what `null` unboxes to.
    pub(super) fn zero_of_test(&mut self, test: TestId) -> TExprId {
        let zero = match self.prog.tests[test.idx()] {
            TypeTest::Long => TExpr::Long(0),
            TypeTest::Number | TypeTest::Float => TExpr::Double(0.0),
            TypeTest::Bool => TExpr::Bool(false),
            TypeTest::Char => TExpr::Char(0),
            // A `Char` on JavaScript, whose test is a string's.
            TypeTest::Str => TExpr::Char(0),
            _ => TExpr::Int(0),
        };
        self.prog.add(zero)
    }

    pub(super) fn zero_of(&mut self, t: TypeId) -> Option<TExprId> {
        let t = self.dealias(t);
        let b = &self.b;
        let zero = if t == b.t_int || t == b.t_byte || t == b.t_short {
            TExpr::Int(0)
        } else if t == b.t_long {
            TExpr::Long(0)
        } else if t == b.t_double || t == b.t_float {
            TExpr::Double(0.0)
        } else if t == b.t_boolean {
            TExpr::Bool(false)
        } else if t == b.t_char {
            TExpr::Char(0)
        } else if t == b.t_unit {
            TExpr::Unit
        } else {
            return None;
        };
        Some(self.prog.add(zero))
    }

    /// Members available on every type.
    pub fn universal_member(
        &mut self,
        recv: TExprId,
        recv_ty: TypeId,
        name: Name,
        targs: Option<ListRef>,
        lists: &[ArgList],
        span: Span,
    ) -> Option<(TExprId, TypeId)> {
        let no_args = lists.is_empty() || (lists.len() == 1 && lists[0].args.is_empty());
        let one_arg = lists.len() == 1 && lists[0].args.len() == 1;
        let recv_class = self.dealias(recv_ty);
        let fractional = matches!(self.is_numeric(recv_class), Some(R_DOUBLE | R_FLOAT));
        let arg = |i: usize| match lists[0].args[i] {
            ArgSrc::Ast(e) => Some(e),
            _ => None,
        };
        match name {
            names::TO_STRING if no_args => {
                let te = self.to_string_call(recv, recv_ty);
                if te == recv {
                    self.end_chain(te, self.b.t_string);
                    *self.expr_marks.entry(te).or_default() |= super::MARK_CALL;
                    if self.capturing() {
                        self.capture_wrap(te, Wrap::Member(name, span));
                    }
                }
                Some((te, self.b.t_string))
            }
            // `AnyRef.synchronized`: the one thread holds every monitor, so the body runs as
            // it is, after the receiver.
            names::SYNCHRONIZED if one_arg && !matches!(self.is_numeric(recv_class), Some(_)) => {
                let (body, ty) = self.type_expr(arg(0)?, None);
                let l = self.prog.stmts.push_slice(&[TStmt::Expr(recv)]);
                let te = self.prog.add(TExpr::Block(l, body));
                self.end_chain(te, ty);
                Some((self.builtin_member(te, name), ty))
            }
            names::HASH_CODE if no_args => {
                let s = self.prog.add_str("$hashCode($0)");
                let l = self.prog.list(&[recv]);
                let te = self.prog.add(TExpr::Js(s, l));
                Some((self.builtin_member(te, name), self.b.t_int))
            }
            names::EQUALS if one_arg && fractional => {
                let (tr, _) = self.type_expr(arg(0)?, None);
                let s = self.prog.add_str("$doubleEquals($0, $1)");
                let l = self.prog.list(&[recv, tr]);
                let te = self.prog.add(TExpr::Js(s, l));
                Some((self.builtin_member(te, name), self.b.t_boolean))
            }
            // The method, which a boxed number answers by its class and value and an object by
            // its own `equals`, called even on itself; `==` is cooperative (`PrimOp::Eq`).
            names::EQUALS if one_arg => {
                let (tr, _) = self.type_expr(arg(0)?, None);
                let s = self.prog.add_str("$equals($0, $1)");
                let l = self.prog.list(&[recv, tr]);
                let te = self.prog.add(TExpr::Js(s, l));
                Some((self.builtin_member(te, name), self.b.t_boolean))
            }
            names::EQ | names::NE if one_arg => {
                let (tr, _) = self.type_expr(arg(0)?, None);
                let prim = match name {
                    names::EQ => PrimOp::RefEq,
                    _ => PrimOp::RefNe,
                };
                let te = self.prog.add(TExpr::Prim(prim, recv, tr));
                if self.capturing() {
                    self.capture_form(te, Form::Op(name));
                }
                Some((te, self.b.t_boolean))
            }
            // `f.asInstanceOf[Int => Int](41)`: the cast, and its result applied, `()` too
            // (`f.asInstanceOf[() => Int]()`, dotty's `Apply` of the `TypeApply` with no arguments).
            names::AS_INSTANCE_OF if !lists.is_empty() => {
                let (cast, cast_ty) = self.universal_member(recv, recv_ty, name, targs, &[], span)?;
                Some(self.apply_callee(super::apply::Callee::Value(cast, cast_ty), None, lists.to_vec(), span, None))
            }
            names::IS_INSTANCE_OF if !no_args => {
                let (cast, cast_ty) = self.universal_member(recv, recv_ty, name, targs, &[], span)?;
                Some(self.apply_callee(super::apply::Callee::Value(cast, cast_ty), None, lists.to_vec(), span, None))
            }
            names::IS_INSTANCE_OF | names::AS_INSTANCE_OF => {
                let ids = self.cur_ast().ty_list(targs?).to_vec();
                if ids.len() != 1 {
                    self.error(span, "exactly one type argument is expected");
                    return Some((recv, ERROR));
                }
                let reads = self.inline.tparam_reads.get();
                if name == names::AS_INSTANCE_OF {
                    // The type as written, a `*:` chain kept as its spelling erases it
                    // (`TypeErasure.tupleArity`).
                    let t = self.resolve_declared_type(ids[0]);
                    return Some(self.lower_cast(recv, recv_ty, t, span));
                }
                let t = self.resolve_type(ids[0]);
                let unchecked = self.expr_marks.get(&recv).map_or(false, |&m| m & super::MARK_UNCHECKED != 0);
                let test = self.test_for(t, recv_ty, span, unchecked);
                self.mark_leaf_test(test, reads);
                if self.inline.checking > 0 {
                    self.note_leaf_test(test, t);
                }
                let te = self.prog.add(TExpr::TypeTest(recv, test));
                if self.capturing() {
                    self.capture_form(te, Form::Test(t));
                }
                Some((te, self.b.t_boolean))
            }
            names::VALUES | names::VALUE_OF | names::FROM_ORDINAL => {
                let Type::Class(module, _) = self.types.get(recv_ty) else { return None };
                let info = self.syms.class(module);
                let enum_class = info.companion.filter(|_| info.kind == ClassKind::Object)?;
                if self.syms.class(enum_class).kind != ClassKind::Enum {
                    return None;
                }
                // Only an enum whose cases are all values has these members.
                let cases = self.syms.class(enum_class).children.clone();
                let values: Option<Vec<SymId>> =
                    cases.iter().map(|&c| self.syms.class(c).singleton).collect();
                let items: Vec<TExprId> =
                    values?.into_iter().map(|s| self.prog.add(TExpr::Static(s))).collect();
                let items = self.prog.list(&items);
                let all = self.prog.add(TExpr::ArrayLit(items));
                if self.capturing() && name == names::VALUES {
                    self.capture_form(all, Form::EnumMember(name));
                }
                // The type arguments differ from case to case, which is `E[?]` in Scala.
                let arity = self.class_arity(enum_class);
                let enum_ty = self.types.class(enum_class, &vec![WILD; arity]);
                if name == names::VALUES {
                    if !no_args {
                        return None;
                    }
                    let args = self.types.list(&[enum_ty]);
                    let array = self.types.mk(Type::Class(self.b.array, args));
                    return Some((all, array));
                }
                if !one_arg {
                    return None;
                }
                let (expected, helper) = if name == names::VALUE_OF {
                    (self.b.t_string, "$enumValueOf($0, $1, $2)")
                } else {
                    (self.b.t_int, "$enumFromOrdinal($0, $1, $2)")
                };
                let (key, _) = self.type_expr(arg(0)?, Some(expected));
                let s = self.prog.add_str(helper);
                let enum_name = self.prog.add_str(self.interner.get(self.syms.class(enum_class).name));
                let enum_name = self.prog.add(TExpr::Str(enum_name));
                let l = self.prog.list(&[all, key, enum_name]);
                let te = self.prog.add(TExpr::Js(s, l));
                if self.capturing() {
                    self.capture_form(te, Form::EnumMember(name));
                }
                Some((te, enum_ty))
            }
            names::GET_CLASS if no_args => {
                // A receiver whose static type is a primitive has the primitive's class, as scalac
                // rewrites it (`1.getClass` is `classOf[Int]`, `int` printed); it runs for its effects.
                let primitive = self.is_numeric(recv_ty).is_some() || recv_ty == self.b.t_boolean || recv_ty == self.b.t_unit;
                if primitive {
                    let (of, ty) = self.type_class_of(recv_ty, span);
                    let l = self.prog.stmts.push_slice(&[TStmt::Expr(recv)]);
                    let te = self.prog.add(TExpr::Block(l, of));
                    self.end_chain(te, ty);
                    return Some((self.builtin_member(te, name), ty));
                }
                let class = self.java_lang_class("Class")?;
                let unit = self.typing_unit();
                if !self.prog.get_class_units.contains(&unit) && !self.checks_inline_definition() {
                    self.prog.get_class_units.push(unit);
                }
                let s = self.prog.add_str("$getClass($0)");
                let l = self.prog.list(&[recv]);
                // `Class[? <: T]`, as scalac's `Any.getClass` types it: a subclass's instance is
                // the receiver's.
                let arg = self.types.bounded_wild(NOTHING, recv_ty);
                let ty = self.types.class(class, &[arg]);
                let te = self.prog.add(TExpr::Js(s, l));
                Some((self.builtin_member(te, name), ty))
            }
            names::PRODUCT_PREFIX if no_args => {
                let c = self.class_of(recv_ty)?;
                if self.is_tuple_class(c) {
                    // Past 22 elements the tuple is a TupleXXL, whose prefix is `Tuple`.
                    let name = if self.syms.class(c).tparams.len() > 22 { "Tuple".to_string() } else { self.name_str(self.syms.class(c).name) };
                    let s = self.prog.add_str(&name);
                    let prefix = self.prog.add(TExpr::Str(s));
                    if self.capturing() {
                        self.capture_builtin_call(prefix, names::PRODUCT_PREFIX, recv, Vec::new());
                    }
                    return Some((prefix, self.b.t_string));
                }
                let info = self.syms.class(c);
                let is_product = matches!(info.kind, ClassKind::Enum | ClassKind::EnumCase)
                    || info.mods & crate::ast::mods::CASE != 0;
                if !is_product {
                    return None;
                }
                let s = self.prog.add_str("$productPrefix($0)");
                let l = self.prog.list(&[recv]);
                let te = self.prog.add(TExpr::Js(s, l));
                Some((self.builtin_member(te, name), self.b.t_string))
            }
            names::ORDINAL if no_args => {
                let c = self.class_of(recv_ty)?;
                let is_enum = self.syms.class(c).kind == ClassKind::Enum
                    || self.syms.class(c).kind == ClassKind::EnumCase;
                if !is_enum {
                    return None;
                }
                let s = self.prog.add_str("$0.$ordinal");
                let l = self.prog.list(&[recv]);
                let te = self.prog.add(TExpr::Js(s, l));
                Some((self.builtin_member(te, name), self.b.t_int))
            }
            _ => None,
        }
    }

    pub fn is_const_expr(&self, e: ExprId) -> bool {
        matches!(
            self.cur_ast().expr(e),
            Expr::IntLit(_) | Expr::LongLit(_) | Expr::DoubleLit(_) | Expr::DecimalLit(_) | Expr::FloatLit(_) | Expr::BoolLit(_) | Expr::CharLit(_) | Expr::StringLit(_)
        )
    }
}

fn compare_op(op: Name) -> PrimOp {
    match op {
        names::LT => PrimOp::Lt,
        names::LE => PrimOp::Le,
        names::GT => PrimOp::Gt,
        _ => PrimOp::Ge,
    }
}

fn arith_op(op: Name, rank: u8) -> Option<PrimOp> {
    use PrimOp::*;
    Some(match (op, rank) {
        (names::PLUS, R_INT) => IntAdd,
        (names::MINUS, R_INT) => IntSub,
        (names::STAR, R_INT) => IntMul,
        (names::SLASH, R_INT) => IntDiv,
        (names::PERCENT, R_INT) => IntRem,
        (names::AMP, R_INT) => IntAnd,
        (names::BAR, R_INT) => IntOr,
        (names::CARET, R_INT) => IntXor,
        (names::SHL, R_INT) => IntShl,
        (names::SHR, R_INT) => IntShr,
        (names::USHR, R_INT) => IntUshr,
        (names::PLUS, R_LONG) => LongAdd,
        (names::MINUS, R_LONG) => LongSub,
        (names::STAR, R_LONG) => LongMul,
        (names::SLASH, R_LONG) => LongDiv,
        (names::PERCENT, R_LONG) => LongRem,
        (names::AMP, R_LONG) => LongAnd,
        (names::BAR, R_LONG) => LongOr,
        (names::CARET, R_LONG) => LongXor,
        (names::SHL, R_LONG) => LongShl,
        (names::SHR, R_LONG) => LongShr,
        (names::USHR, R_LONG) => LongUshr,
        (names::PLUS, R_FLOAT) => FloatAdd,
        (names::MINUS, R_FLOAT) => FloatSub,
        (names::STAR, R_FLOAT) => FloatMul,
        (names::SLASH, R_FLOAT) => FloatDiv,
        (names::PERCENT, R_FLOAT) => FloatRem,
        (names::PLUS, R_DOUBLE) => DoubleAdd,
        (names::MINUS, R_DOUBLE) => DoubleSub,
        (names::STAR, R_DOUBLE) => DoubleMul,
        (names::SLASH, R_DOUBLE) => DoubleDiv,
        (names::PERCENT, R_DOUBLE) => DoubleRem,
        _ => return None,
    })
}
