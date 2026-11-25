use super::apply::{ArgList, ArgSrc, FrameKind, FunEnd, SelectionFrame};
use super::{Frame, Worker};
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
pub(super) enum OpClass {
    Arith,
    Compare,
    Bits,
    Shift,
}

/// The prefix operator a `unary_` method name stands for.
/// Where the operator of the infix node `e` ends (`Expr::Infix`).
fn infix_op_end(ast: &crate::ast::Ast, e: ExprId) -> u32 {
    match ast.expr(e) {
        Expr::Infix(_, _, _, op_end) => op_end,
        _ => crate::ast::NO_END,
    }
}

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

pub(super) fn classify(op: Name) -> Option<OpClass> {
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

/// What the source wrote on the left of a compound assignment `l op= r`: a member named alone (an identifier),
/// a selection `q.x` with the tree of `q`, or anything else.
#[derive(Clone, Copy)]
enum WrittenLeft {
    Alone,
    Selected(ExprId),
    Other,
}

/// Whether a kept failure of an assignment operator's first try is held to the try itself at every use
/// (`Worker::check_op_memo`): in the unit tests, and under `TEQ_CHECK_OPMEMO`.
fn op_memo_checked() -> bool {
    static CHECKED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    cfg!(test) || *CHECKED.get_or_init(|| std::env::var_os("TEQ_CHECK_OPMEMO").is_some_and(|v| !v.is_empty()))
}

/// What the left side of `fn(args) op= r`, applied as it is written, gives the assignment form
/// (`Worker::assign_op_by_application`).
enum Lowered {
    /// `fn.update(args, fn.apply(args) op r)`.
    Done((TExprId, TypeId)),
    /// No assignment form: the reference's `typedAssign` meets no `apply` it lowers.
    Refused,
    /// The application gives no argument typings to read (an overloaded `apply`, written type arguments,
    /// `super`): lowered over the arguments typed alone (`assign_op_indexed`), by the `apply` it selected.
    Unknown(Option<SymId>),
}

/// The head `fn` of an applied left side `fn(args)`.
#[derive(Clone, Copy)]
enum AppliedHead {
    /// A value, typed, with its type.
    Value(TExprId, TypeId),
    /// `super`, with the parent it names and its place.
    Super(Name, Span),
}

impl<'a> Worker<'a> {
    /// The name the enum `e`'s lookups give it in their messages, dotty's `enumClass.fullName`:
    /// a local enum's under the class whose body defines it, `_$` before its name
    /// (`fullNameSeparated`), a top-level body's class being its file's `$package` object.
    fn enum_full_name(&mut self, e: ClassId) -> String {
        if self.syms.class(e).owner != Owner::Local {
            return self.syms.scala_full_name(self.interner, e);
        }
        let name = self.name_str(self.syms.class(e).name);
        let frames = match self.anon_envs.get(&e) {
            Some(env) => env.frames.clone(),
            None => {
                self.env_of_peer_class(e);
                Vec::new()
            }
        };
        let enclosing = frames.iter().rev().find_map(|f| match f {
            super::Frame::Class(k) | super::Frame::Ctor(k) if self.syms.class(*k).owner != Owner::Local => Some(*k),
            _ => None,
        });
        let local = match enclosing {
            Some(k) => self.library_class_name(k, ""),
            None => {
                let file = self.syms.class(e).file;
                let stem = crate::tasty::write::file_stem(&self.source(file).path);
                let top: Vec<crate::ast::DefId> = self.ast(file).top_level.to_vec();
                let pkg = top.iter().find_map(|d| match self.def_syms.get(file.0 as usize, d).map(|&s| self.syms.sym(s).owner) {
                    Some(Owner::Package(p)) => Some(p),
                    _ => self.def_classes.get(file.0 as usize, d).and_then(|&c| match self.syms.class(c).owner {
                        Owner::Package(p) => Some(p),
                        _ => None,
                    }),
                });
                let path = pkg.map(|p| self.pkg_path(p)).unwrap_or_default();
                if path.is_empty() { format!("{}$package$", stem) } else { format!("{}.{}$package$", path, stem) }
            }
        };
        let full = format!("{}._${}", local, name);
        if !self.prog.local_enum_names.iter().any(|&(x, _)| x == e) {
            self.prog.local_enum_names.push((e, full.clone()));
        }
        full
    }

    pub fn is_primitive(&self, t: TypeId) -> bool {
        let t = self.canon_head(t);
        self.is_numeric(t).is_some() || t == self.b.t_boolean || t == self.b.t_string || t == self.b.t_unit
    }

    /// Converts a numeric operand to the numeric type of the given rank.
    #[inline(always)]
    pub fn convert_rank(&mut self, te: TExprId, from: TypeId, rank: u8) -> TExprId {
        let from_rank = self.is_numeric(from).unwrap_or(R_INT);
        let mut te = self.unbox_erased(te, from);
        for &op in convert_ops(from_rank, rank) {
            te = self.prog.add(TExpr::Unary(op, te));
        }
        te
    }

    /// A value of a primitive type that a call with an erased result gave (`is_erased_result`: a
    /// `java.util.HashMap[String, Int]`'s `get`, a type parameter's result), consumed as the
    /// primitive: unboxed, as dotty's `Erasure.Boxing.adaptToType` unboxes a tree of a reference
    /// type where a primitive is expected (`BoxesRunTime.unboxToInt`, `null` the zero). Where the
    /// value is expected as `Any` (an argument of `println`, an interpolation) it stays as it is.
    /// The JVM's backend erases for itself.
    /// Whether `t` is one of the primitive classes' types as it stands, a test cheap enough for
    /// every expected type.
    #[inline]
    pub(super) fn is_primitive_type_id(&self, t: TypeId) -> bool {
        self.b.primitive_ids.checked_shr(self.canon(t).0).is_some_and(|bits| bits & 1 != 0)
    }

    #[inline(always)]
    pub(super) fn unbox_erased(&mut self, te: TExprId, ty: TypeId) -> TExprId {
        if self.jvm {
            return te;
        }
        match self.prog.expr(te) {
            // Most calls' results erase as their types do, which a memo per member answers.
            TExpr::CallMethod(_, s, _) | TExpr::CallStatic(s, _) | TExpr::Field(_, s) if self.unbox_candidates.get(s) == Some(false) => te,
            TExpr::CallMethod(..) | TExpr::CallStatic(..) | TExpr::Field(..) | TExpr::Js(..) | TExpr::Block(..) | TExpr::If(_, _, Some(_)) => self.unbox_erased_call(te, ty),
            _ => te,
        }
    }

    /// Whether a call of `sym` may need unboxing where its primitive is consumed: its result erases
    /// to a reference (`erases_to_reference`), or it is an inline method's, whose expansion's may;
    /// kept per member once its signature is complete.
    fn unbox_candidate(&mut self, sym: SymId) -> bool {
        if let Some(v) = self.unbox_candidates.get(sym) {
            return v;
        }
        let v = self.syms.sym(sym).mods & crate::ast::mods::INLINE != 0 || self.result_erases(sym);
        if self.syms.sym_cells.get(sym.0) == crate::symbols::Completion::Done {
            self.unbox_candidates.set(sym, v);
        }
        v
    }

    /// `erases_to_reference`, kept per member once its signature is complete.
    fn result_erases(&mut self, sym: SymId) -> bool {
        if let Some(v) = self.erased_results.get(sym) {
            return v;
        }
        let v = self.erases_to_reference(sym);
        if self.syms.sym_cells.get(sym.0) == crate::symbols::Completion::Done {
            self.erased_results.set(sym, v);
        }
        v
    }

    #[inline(never)]
    fn unbox_erased_call(&mut self, te: TExprId, ty: TypeId) -> TExprId {
        if self.inline_val_rhs {
            return te;
        }
        // An inline call's expansion, whose result typing does not read (the inlining phase's
        // rule): its consumption noted, the expansion unboxed once placed (`apply_inline_unboxes`).
        if !self.unboxing_after && self.prog.is_expansion(te) {
            self.inline_unboxes.push((te, ty));
            return te;
        }
        // A block's value is its result's, which dotty's `typedBlock` adapts, and a conditional's
        // its branches', which `typedIf` adapts each.
        let sym = match self.prog.expr(te) {
            TExpr::Block(stmts, res) => {
                let unboxed = self.unbox_erased(res, ty);
                if unboxed != res {
                    self.rewrite_expr(te, TExpr::Block(stmts, unboxed));
                }
                return te;
            }
            TExpr::If(c, a, Some(b)) => {
                let (ua, ub) = (self.unbox_erased(a, ty), self.unbox_erased(b, ty));
                if (ua, ub) != (a, b) {
                    self.rewrite_expr(te, TExpr::If(c, ua, Some(ub)));
                }
                return te;
            }
            // `call_sym`'s.
            TExpr::CallMethod(_, s, _) | TExpr::CallStatic(s, _) | TExpr::Field(_, s) => s,
            TExpr::Js(t, _) => match self.prog.template_syms.get(&t) {
                Some(&s) => s,
                None => return te,
            },
            _ => return te,
        };
        if !self.unbox_candidate(sym) {
            return te;
        }
        // A value of a constant type is the constant to scalac, which folds it before erasure
        // (an inline expansion's `constValue`, a `lazy val b: T` at `T = true`).
        if matches!(self.types.get(ty), Type::Lit(_)) {
            return te;
        }
        // The node keeps the call's type (`n.type` of a selector `erasedValue[n.type]`, which a
        // wildcard pattern takes), its primitive the unboxing's.
        // A type parameter bounded by a primitive erases to it (`TypeErasure`), as an alias does.
        let prim = if self.is_primitive_type_id(ty) { ty } else { self.erased_primitive(ty).unwrap_or_else(|| self.dealias(ty)) };
        if !self.is_primitive(prim) || self.canon(prim) == self.b.t_unit {
            return te;
        }
        if !self.result_erases(sym) {
            // An inline call expanded later: its expansion, unboxed once it takes the call's node.
            if !self.unboxing_after && self.syms.sym(sym).mods & crate::ast::mods::INLINE != 0 && self.is_pending_call(te) {
                self.inline_unboxes.push((te, ty));
            }
            return te;
        }
        // `cast_lowering(Any, prim)`'s unboxing: the primitive's class test, as `test_for` makes it.
        let Type::Class(c, _) = self.types.get(prim) else { return te };
        let Some(test) = self.primitive_test(c) else { return te };
        let test = self.prog.add_test(test);
        // The erasure is the primitive's class, which takes no arguments: the primitive's own type.
        let unboxed = self.prog.add(TExpr::Cast(te, CastOp::Adapt(test, prim), prim));
        self.prog.set_type(unboxed, ty);
        self.prog.copy_span(te, unboxed);
        // No cast to scalac: its tree is the call, which a pickle writes and a macro sees.
        if self.capturing() {
            self.capture_form(unboxed, Form::Promotion);
        }
        unboxed
    }

    /// `unbox_erased` of the node `te` in place, for the nodes that refer to it: its value moved to
    /// a copy, which keeps what the capture recorded of it, under the unboxing.
    pub(super) fn unbox_erased_in_place(&mut self, te: TExprId, target: TypeId) -> bool {
        if !self.unboxing_after && self.prog.is_expansion(te) {
            self.inline_unboxes.push((te, target));
            return false;
        }
        // A block's result and a conditional's branches, which `unbox_erased` unboxes in place; a node of
        // another kind than a call's, a field's or a template's it leaves as it is (an expansion's literal,
        // its operator).
        match self.prog.expr(te) {
            TExpr::Block(_, res) => {
                self.unbox_erased(te, target);
                return !matches!(self.prog.expr(te), TExpr::Block(_, r) if r == res);
            }
            TExpr::If(_, a, Some(b)) => {
                self.unbox_erased(te, target);
                return !matches!(self.prog.expr(te), TExpr::If(_, x, Some(y)) if x == a && y == b);
            }
            TExpr::CallMethod(..) | TExpr::CallStatic(..) | TExpr::Field(..) | TExpr::Js(..) => {}
            _ => return false,
        }
        // Decided on the node as it stands, as on a copy of it, which is neither an expansion nor a pending
        // call: its value moves to a copy only where it is unboxed.
        let outer = std::mem::replace(&mut self.unboxing_after, true);
        let unboxed = self.unbox_erased(te, target);
        self.unboxing_after = outer;
        if unboxed == te {
            return false;
        }
        let TExpr::Cast(_, op, prim) = self.prog.expr(unboxed) else { return false };
        let original_type = self.prog.type_of(te);
        let original = self.prog.add(self.prog.expr(te));
        if let Some(t) = original_type {
            self.prog.set_type(original, t);
        }
        self.prog.copy_span(te, original);
        self.rewrite_expr(te, TExpr::Cast(original, op, prim));
        self.move_declared(te, original);
        if self.capturing() {
            self.capture_moved(te, original);
            self.capture_form(te, Form::Promotion);
        }
        true
    }

    /// The inline calls and expansions consumed as a primitive (`inline_unboxes`) whose expansions are
    /// placed, unboxed in place as an erased result is where it is consumed; a call still pending
    /// waits for its expansion. Dotty's `Erasure` adapts an `Inlined` tree after `Inlining`: run
    /// where the expansions are placed (an item's end, a unit's, an attempt's commit), it is no
    /// reading of an expansion's result by typing.
    pub(super) fn apply_inline_unboxes(&mut self) {
        let entries = std::mem::take(&mut self.inline_unboxes);
        let mut waiting = Vec::new();
        let outer = std::mem::replace(&mut self.unboxing_after, true);
        for (e, ty) in entries {
            // One of a call the inlining phase expands waits with the phase's records, which the
            // merge carries (`Inlining::unboxes`).
            if self.inlining.holds(e) {
                let ty = self.zonk(ty);
                self.inlining.unboxes.push((e, ty));
                continue;
            }
            if self.is_pending_call(e) {
                waiting.push((e, ty));
                continue;
            }
            let ty = self.zonk(ty);
            self.unbox_erased_in_place(e, ty);
        }
        self.unboxing_after = outer;
        self.inline_unboxes.extend(waiting);
    }

    /// `te`, the typer's form of the call of the builtin member `name` of its receiver.
    fn builtin_member(&mut self, te: TExprId, name: Name) -> TExprId {
        if self.capturing() {
            self.capture_form(te, Form::Op(name));
        }
        te
    }

    /// The lookup `te` of type `ty` of an enum's companion held in the local `held`, where it is
    /// an instance: bound to its receiver first.
    fn enum_lookup_held(&mut self, held: Option<SymId>, receiver: TExprId, te: TExprId, ty: TypeId) -> TExprId {
        let Some(h) = held else { return te };
        let stmts = self.prog.stmts.push_slice(&[TStmt::Val(h, receiver)]);
        let block = self.prog.add(TExpr::Block(stmts, te));
        self.prog.set_type(block, ty);
        block
    }

    /// An operand of a builtin operator converted to the operator's rank. The conversions stay
    /// untyped, which tells them from a written `x.toLong` in a macro's view of the tree
    /// (`Interp::unpromoted`).
    #[inline(always)]
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
        // A constant type is no target: `3` is no `3.14`.
        if matches!(self.types.get(to), Type::Lit(_)) {
            return false;
        }
        let (from, to) = (self.canon(from), self.canon(to));
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
            None => self.type_infix(l, op, r, infix_op_end(self.cur_ast(), e), span, expected),
        };
        self.body_node = outer;
        typed
    }

    /// A library body's operator whose TASTy names its declaration: where
    /// the left operand's name is overloaded, the declared alternative is the one called.
    fn type_declared_infix(&mut self, d: crate::ast::DeclRef, e: ExprId, l: ExprId, op: Name, r: ExprId, span: Span, expected: Option<TypeId>) -> (TExprId, TypeId) {
        let outer_took = self.declared_took.take();
        self.infix_declared = Some(d);
        let (te, ty) = self.type_infix(l, op, r, infix_op_end(self.cur_ast(), e), span, expected);
        self.infix_declared = None;
        let took = std::mem::replace(&mut self.declared_took, outer_took);
        if super::loader::declared::dump_path().is_some() {
            self.dump_declared(d, e, span, Some(ty), took);
        }
        (te, ty)
    }

    /// `l op r`, the operator ending at `op_end` (`Expr::Infix`), where the function tree of a
    /// left-associative operator's application ends (`Desugar.binop`'s `Select(l, op)`).
    pub fn type_infix(&mut self, l: ExprId, op: Name, r: ExprId, op_end: u32, span: Span, expected: Option<TypeId>) -> (TExprId, TypeId) {
        // The operands a deeper selection of the operator typed are the operation's alone.
        let watch = self.watch_infix(op, op_end);
        let typed = self.type_infix_now(l, op, r, op_end, span, expected);
        if let Some(outer) = watch {
            let of = self.unwatch_infix(outer);
            self.check_infix_expr(op, r, op_end, of);
        }
        // A right-associative operator's selection is on its right operand.
        if !self.selection_typed.is_empty() {
            for operand in [l, r] {
                let owner = self.qualifier_span(operand);
                self.drop_selection_typed(owner);
            }
        }
        typed
    }

    #[inline(always)]
    fn type_infix_now(&mut self, l: ExprId, op: Name, r: ExprId, op_end: u32, span: Span, expected: Option<TypeId>) -> (TExprId, TypeId) {
        // A library body's operator's declaration (`type_declared_infix`), this operator's
        // alone: taken before its operands are typed.
        let declared = self.infix_declared.take();
        if op == names::EQEQ || op == names::NEQ {
            return self.type_equality(l, r, op == names::NEQ, span);
        }
        let op_text = self.name_ref(op);
        if op_text.ends_with(':') {
            return self.type_right_assoc(l, op, r, Vec::new(), span, expected);
        }
        if crate::lexer::is_op_assignment_name(op_text) {
            return self.type_assign_op(l, op, r, op_end, span, expected, declared);
        }
        // The left operand is the qualifier of the operator (dotty's `typedInfixOp` through
        // `Desugar.binop`), `&&` and `||` alike: their short circuit is a `Boolean`'s
        // (`binop_or_member`), on anything else they are methods.
        let frame = SelectionFrame { file: self.env.file, q: l, name: op, at: r, app: span, expected, kind: FrameKind::Operand { more: false } };
        let (tl, lty) = self.type_selected(l, frame);
        let Some(d) = declared else { return self.binop_on_typed(tl, lty, op, r, op_end, span, expected) };
        let outer = self.declared_call.replace((tl, op, d));
        let typed = self.binop_on_typed(tl, lty, op, r, op_end, span, expected);
        self.declared_call = outer;
        typed
    }

    /// `l op= r` (the reference's `isOpAssign`) as `Applications.typedApply` types it (1523-1533): the application
    /// as written first (`realApply`), its left side the expression it is and the operator found as any member
    /// is; only where that fails, the left side typed again by itself and the assignment `l = l op r` made by its
    /// typed shape (`typedOpAssign`); where both fail the first failure stands (`failedState.commit()`).
    #[inline(never)]
    #[allow(clippy::too_many_arguments)]
    fn type_assign_op(&mut self, l: ExprId, op: Name, r: ExprId, op_end: u32, span: Span, expected: Option<TypeId>, declared: Option<crate::ast::DeclRef>) -> (TExprId, TypeId) {
        let base = self.name_ref(op);
        let base = base[..base.len() - 1].to_string();
        let base = self.interner.intern(&base);
        // A first try known to fail with nothing to try (`Worker::op_memo`): the assignment form at once.
        if self.op_memo_ops.contains(&op) {
            if let Some(assigned) = self.assign_op_known_failed(l, op, base, r, op_end, span, expected, declared) {
                return assigned;
            }
        }
        let hoisted = self.hoisted.len();
        // What a typing of the operands that is dropped made is forgotten (`drop_made`).
        let outer_watch = std::mem::replace(&mut self.made_watch, true);
        let made = self.made_local.len();
        let counts = self.made_counts.len();
        let first = self.attempt();
        // The flag is this operation's while its first try types: an operation typed inside its right side has
        // its own, and gives this one's back.
        let outer_nothing = std::mem::replace(&mut self.op_nothing, false);
        let (written, applied) = self.assign_op_written(l, op, r, op_end, span, expected, declared);
        let nothing = std::mem::replace(&mut self.op_nothing, outer_nothing);
        // The try's bound is `realApply`'s: it failed only by what it reported before the operator's list was
        // applied (the left side, the operator's search, an argument that does not fit); what follows (the
        // operator's own using clauses, the result against the expected type) is told and opens nothing.
        if !self.attempt_failed(&first) || applied.is_some_and(|(diags, error_nodes)| !self.attempt_failed_before(&first, diags as usize, error_nodes)) {
            self.close(first);
            self.made_end(outer_watch);
            return written;
        }
        let first_hoisted = self.hoisted.split_off(hoisted);
        let first_told = self.attempt_told_error(&first);
        let aside = self.set_aside(first);
        let first_made = self.made_local.len();
        let first_counts = self.made_counts.len();
        let first_drop = self.drop_made(made, first_made, counts, span);
        let second = self.attempt();
        let (assigned, var) = self.assign_op_assigned(l, base, r, op_end, span);
        match assigned {
            // The assignment form failed where it TOLD an error of the typer's own (`tryEither`'s
            // `reporter.hasErrors`, Typer.scala 134-137). An operand that is an error told at its cause alone (a
            // name of an import that did not resolve: an error type that reports nothing to the reference) or
            // whose record is a later phase's (a default getter's refusal, a tuple's index) fails nothing where the
            // first try told its own failure: the form stands as it stood before the first try was asked.
            Some(assigned) if !self.attempt_failed(&second) || (first_told && !self.attempt_told_error(&second)) => {
                self.close(second);
                self.made_end(outer_watch);
                // The first try met nothing to try and the left side is a var: kept for the var's type here.
                if let (true, None, Some(lty)) = (nothing, applied, var) {
                    if let (Some((_, t)), Some((file, limit, defining))) = (self.op_memo_type(lty), self.op_memo_context()) {
                        self.op_memo.insert((t, op, file, limit, defining), ());
                        if !self.op_memo_ops.contains(&op) {
                            self.op_memo_ops.push(op);
                        }
                    }
                }
                assigned
            }
            _ => {
                // The second try's own go, the first try's stand again.
                let second_made = self.made_local.len();
                self.drop_made(first_made, second_made, first_counts, span);
                self.restore_made(made, first_made, counts, first_drop);
                self.retract(second);
                self.hoisted.truncate(hoisted);
                self.restore(aside);
                self.hoisted.extend(first_hoisted);
                self.made_end(outer_watch);
                written
            }
        }
    }

    /// The typing of an assignment operator's operands that made the classes `made_local[from..to]` is dropped
    /// for another one: an operand is typed once for the tree that is kept, and what this typing made of the
    /// operation's own (a class defined inside `span`) is forgotten: an anonymous class is no longer its
    /// definition's (`def_classes`: the next typing makes it again, its body typed and told again, bound to
    /// that typing's locals), and each class is marked (`ClassInfo::dropped`: nothing nested in it is a class
    /// the program's output roots). What the typing ADVANCED goes back with them: each count a class made inside
    /// `span` took of its position since `counts` (`made_counts`: `anon_name`, `local_class_name`; the place is
    /// the outermost call site's for a class an expansion made), so that the class the next typing makes there
    /// takes the name it has where nothing was typed and dropped before it (the bytes do not tell a dropped
    /// typing; the inliner's own log of the counts, `Inlining::anon_log`, is kept in step). The drop's number,
    /// for `restore_made`.
    fn drop_made(&mut self, from: usize, to: usize, counts: usize, span: Span) -> u32 {
        self.made_drops += 1;
        let drop = self.made_drops;
        let here = self.env.file;
        for i in counts..self.made_counts.len() {
            let c = self.made_counts[i];
            let (file, start) = c.key.2.unwrap_or((c.key.0, c.key.1));
            if c.drop != 0 || file != here || start < span.start || start >= span.end {
                continue;
            }
            self.made_counts[i].drop = drop;
            if let Some(n) = self.anon_sites.get_mut(&c.key) {
                *n -= 1;
                if *n == 0 {
                    self.anon_sites.remove(&c.key);
                }
            }
            if let Some(log) = self.inlining.anon_log.as_mut() {
                if let Some(at) = log.iter().rposition(|k| *k == c.key) {
                    log.remove(at);
                }
            }
        }
        for i in from..to {
            let m = self.made_local[i];
            let (at, file) = (self.syms.class(m.class).span, self.syms.class(m.class).file);
            if m.drop != 0 || m.file != here || file != here || at.start < span.start || at.end > span.end {
                continue;
            }
            self.made_local[i].drop = drop;
            if let Some(d) = m.anon {
                if self.def_classes.get(here.0 as usize, &d) == Some(&m.class) {
                    self.def_classes.remove(here.0 as usize, &d);
                }
            }
            if self.syms.classes.writable(m.class.0) {
                self.syms.class_mut(m.class).dropped = true;
            }
        }
        drop
    }

    /// The typing whose classes the drop `drop` took, among `made_local[from..to]`, is the kept one after all
    /// (the first try, where the assignment form fails too): they stand again, and so do the counts they took
    /// since `counts`.
    fn restore_made(&mut self, from: usize, to: usize, counts: usize, drop: u32) {
        for i in counts..self.made_counts.len() {
            let c = self.made_counts[i];
            if c.drop != drop {
                continue;
            }
            self.made_counts[i].drop = 0;
            *self.anon_sites.entry(c.key).or_insert(0) += 1;
            if let Some(log) = self.inlining.anon_log.as_mut() {
                log.push(c.key);
            }
        }
        for i in from..to {
            let m = self.made_local[i];
            if m.drop != drop {
                continue;
            }
            self.made_local[i].drop = 0;
            if let Some(d) = m.anon {
                self.def_classes.insert(m.file.0 as usize, d, m.class);
            }
            if self.syms.classes.writable(m.class.0) {
                self.syms.class_mut(m.class).dropped = false;
            }
        }
    }

    /// The transaction ends: the classes made are the enclosing transaction's to drop, or nobody's.
    fn made_end(&mut self, outer_watch: bool) {
        self.made_watch = outer_watch;
        if !outer_watch {
            self.made_local.clear();
            self.made_counts.clear();
        }
    }

    /// The first try of `l op= r`: `l.op=(r)`, the left operand typed under the operator's selection, and where
    /// the operator's application applied its list (`Worker::retry_applied`, watched for that list alone:
    /// `Worker::op_try`), None where it did not.
    #[allow(clippy::too_many_arguments)]
    fn assign_op_written(&mut self, l: ExprId, op: Name, r: ExprId, op_end: u32, span: Span, expected: Option<TypeId>, declared: Option<crate::ast::DeclRef>) -> ((TExprId, TypeId), Option<(u32, u32)>) {
        // The operation's target, both times it is typed: its own typing marks no use (`use_expr`) and keeps its
        // selection in an expansion's body (`assign_op_target`).
        let outer_target = self.assign_op_target.replace(l);
        let frame = SelectionFrame { file: self.env.file, q: l, name: op, at: r, app: span, expected, kind: FrameKind::Operand { more: false } };
        let (tl, lty) = self.type_selected(l, frame);
        self.assign_op_target = outer_target;
        self.assign_op_receiver = None;
        let own = FunEnd::operand_at(op_end);
        // A tree with no place for its operator (one read from TASTy, typed by scalac already) has no list to
        // tell: nothing is watched and the try stands only where it reported nothing.
        if own == ArgList::NO_FUN {
            let outer = declared.map(|d| self.declared_call.replace((tl, op, d)));
            let typed = self.binop_on_typed(tl, lty, op, r, op_end, span, expected);
            if let Some(outer) = outer {
                self.declared_call = outer;
            }
            return (typed, None);
        }
        let watch = (self.retry_depth.replace(self.app_depth), self.retry_applied.take(), self.op_try.replace(own));
        let outer = declared.map(|d| self.declared_call.replace((tl, op, d)));
        let typed = self.binop_on_typed(tl, lty, op, r, op_end, span, expected);
        if let Some(outer) = outer {
            self.declared_call = outer;
        }
        let applied = self.retry_applied;
        (self.retry_depth, self.retry_applied, self.op_try) = watch;
        (typed, applied)
    }

    /// The second try of `l op= r` (`typedOpAssign`, Applications.scala 1512-1521): the left side typed again by
    /// itself and the assignment `l = l base r` made by what it is: an element read by `apply` written through
    /// `update`, a parameterless member through its setter, a `var`. None where it has no such form; with the
    /// left side's type where it is a `var` assigned as one (`Worker::op_memo`).
    fn assign_op_assigned(&mut self, l: ExprId, base: Name, r: ExprId, op_end: u32, span: Span) -> (Option<(TExprId, TypeId)>, Option<TypeId>) {
        if let Expr::Apply(f, args) = self.cur_ast().expr(l) {
            return (self.assign_op_applied(l, f, args, base, r, op_end, span), None);
        }
        let outer_target = self.assign_op_target.replace(l);
        self.assign_op_receiver = None;
        let (tl, lty) = self.type_expr(l, None);
        self.assign_op_target = outer_target;
        let receiver = match self.assign_op_receiver.take() {
            None if matches!(self.cur_ast().expr(l), Expr::Ident(_)) => self.this_receiver(tl),
            receiver => receiver,
        };
        // What the source wrote on the left: a member named alone, or a selection with its prefix.
        let written = match self.cur_ast().expr(l) {
            Expr::Ident(_) => WrittenLeft::Alone,
            Expr::Select(q, _) => WrittenLeft::Selected(q),
            _ => WrittenLeft::Other,
        };
        if let Some(receiver) = receiver {
            let at = self.cur_ast().expr_span(l);
            if let Some(done) = self.assign_op_through_setter(tl, lty, receiver, written, base, r, op_end, at, span) {
                return (Some(done), None);
            }
        }
        let var = self.is_plain_var(tl);
        let assigned = self.assign_op_to_var(tl, lty, written, base, r, op_end, span);
        let var = (var && assigned.is_some()).then_some(lty);
        (assigned, var)
    }

    /// Whether the typed left side is a `var` read where it stands: a local, a top-level or an object's, a field.
    fn is_plain_var(&self, tl: TExprId) -> bool {
        match self.prog.expr(tl) {
            TExpr::Local(s) | TExpr::Static(s) | TExpr::Field(_, s) => self.syms.sym(s).kind == SymKind::Var,
            _ => false,
        }
    }

    /// The implicit context a kept failure of an assignment operator's first try holds in (`Worker::op_memo`): the
    /// file, how many of its imports the site sees (`import_count`: the file's imports written before the site's
    /// limit) and the alias given being defined; None where the site's environment adds to what the member's search
    /// reads: an import of its own, a local given (a using parameter is one), an enclosing class with a given
    /// index, a class's parent arguments.
    fn op_memo_context(&mut self) -> Option<(crate::source::FileId, u32, Option<SymId>)> {
        if !self.env.imports.is_empty() {
            return None;
        }
        for f in 0..self.env.frames.len() {
            let class = match &self.env.frames[f] {
                Frame::Locals { givens, .. } if givens.is_empty() => continue,
                Frame::Locals { .. } => return None,
                Frame::Ctor(_) => continue,
                Frame::Class(c) => *c,
            };
            if self.parent_args_of == Some(class) || self.class_given_index(class).is_some() {
                return None;
            }
        }
        Some((self.env.file, self.import_count() as u32, self.defining))
    }

    /// The type a kept failure is for, as the member is sought on it (`binop_on_typed`) and as the memo's key:
    /// a class type that stands for itself: no path's type, no alias, no opaque or abstract member type, which
    /// the search goes on from to what they stand for.
    fn op_memo_type(&mut self, lty: TypeId) -> Option<(TypeId, TypeId)> {
        let solved = self.solve_in(lty);
        let solved = self.instantiate_receiver_var(solved);
        if self.types.is_path(solved) || matches!(self.types.get(solved), Type::Member(..) | Type::AppMember(..) | Type::Decl(_) | Type::Alias(..)) {
            return None;
        }
        let widened = self.dealias(solved);
        match self.types.get(widened) {
            Type::Class(c, _) if self.syms.class(c).kind != ClassKind::Opaque => Some((widened, self.canon(widened))),
            _ => None,
        }
    }

    /// `l op= r` where the first try is known to fail with nothing to try for the left side's type in this
    /// context (`Worker::op_memo`): the left side, a var named alone or by a path, typed by itself once, and the
    /// assignment made over that typing. What is not the context's is asked here, at the site: the type's own
    /// member of the name and the lexical extensions of the name (the application's own gatherer). None where
    /// nothing is kept for it or the assignment fails: the reference's order runs whole then.
    #[allow(clippy::too_many_arguments)]
    fn assign_op_known_failed(&mut self, l: ExprId, op: Name, base: Name, r: ExprId, op_end: u32, span: Span, expected: Option<TypeId>, declared: Option<crate::ast::DeclRef>) -> Option<(TExprId, TypeId)> {
        // A library body's operator is the call scalac typed.
        if declared.is_some() {
            return None;
        }
        let ast = self.cur_ast();
        let mut at = l;
        loop {
            match ast.expr(at) {
                Expr::Ident(_) | Expr::This => break,
                Expr::Select(q, _) => at = q,
                _ => return None,
            }
        }
        let (file, limit, defining) = self.op_memo_context()?;
        let hoisted = self.hoisted.len();
        let outer_watch = std::mem::replace(&mut self.made_watch, true);
        let made = self.made_local.len();
        let counts = self.made_counts.len();
        let pre = self.attempt();
        let outer_target = self.assign_op_target.replace(l);
        let (tl, lty) = self.type_expr(l, None);
        self.assign_op_target = outer_target;
        self.assign_op_receiver = None;
        let mut assigned = None;
        if self.is_plain_var(tl) {
            if let Some((sought, t)) = self.op_memo_type(lty) {
                if self.op_memo.contains_key(&(t, op, file, limit, defining)) && self.find_member(sought, op).is_none() && self.lexical_extensions_for(op, sought).0.is_empty() {
                    if op_memo_checked() {
                        self.check_op_memo(l, op, r, op_end, span, expected, declared);
                    }
                    let written = match self.cur_ast().expr(l) {
                        Expr::Ident(_) => WrittenLeft::Alone,
                        Expr::Select(q, _) => WrittenLeft::Selected(q),
                        _ => WrittenLeft::Other,
                    };
                    assigned = self.assign_op_to_var(tl, lty, written, base, r, op_end, span).filter(|_| !self.attempt_failed(&pre));
                }
            }
        }
        match assigned {
            Some(assigned) => {
                self.close(pre);
                self.op_memo_hits += 1;
                self.made_end(outer_watch);
                Some(assigned)
            }
            None => {
                let to = self.made_local.len();
                self.drop_made(made, to, counts, span);
                self.retract(pre);
                self.hoisted.truncate(hoisted);
                self.made_end(outer_watch);
                None
            }
        }
    }

    /// The memo's check (`op_memo_checked`): the first try run where a kept answer says it fails, and the answer
    /// held to it: the try reports before its list is applied and meets no candidate; then taken back.
    #[allow(clippy::too_many_arguments)]
    fn check_op_memo(&mut self, l: ExprId, op: Name, r: ExprId, op_end: u32, span: Span, expected: Option<TypeId>, declared: Option<crate::ast::DeclRef>) {
        let hoisted = self.hoisted.len();
        let made = self.made_local.len();
        let counts = self.made_counts.len();
        let tried = self.attempt();
        let outer_nothing = std::mem::replace(&mut self.op_nothing, false);
        let (_, applied) = self.assign_op_written(l, op, r, op_end, span, expected, declared);
        let nothing = std::mem::replace(&mut self.op_nothing, outer_nothing);
        let failed = self.attempt_failed(&tried) && applied.is_none();
        let to = self.made_local.len();
        self.drop_made(made, to, counts, span);
        self.retract(tried);
        self.hoisted.truncate(hoisted);
        assert!(failed && nothing, "the memo of an assignment operator's first try kept a wrong answer: `{}` at {}:{} (failed {}, nothing met {})", self.name_str(op), self.source(self.env.file).path, span.start, failed, nothing);
    }

    /// `fn(args) base= r`, the left side an application: lowered by the application typed as it is written
    /// (`assign_op_by_application`), or, where that gives no argument typings, over the arguments typed alone
    /// (`assign_op_indexed`).
    #[allow(clippy::too_many_arguments)]
    fn assign_op_applied(&mut self, l: ExprId, f0: ExprId, args: ListRef, base: Name, r: ExprId, op_end: u32, span: Span) -> Option<(TExprId, TypeId)> {
        let ast = self.cur_ast();
        // The written type arguments are `apply`'s and `update`'s (`MaybePoly`).
        let (f, targs) = match ast.expr(f0) {
            Expr::TypeApply(f, targs) => (f, Some(targs)),
            _ => (f0, None),
        };
        let hoisted = self.hoisted.len();
        let made = self.made_local.len();
        let counts = self.made_counts.len();
        let inner = self.attempt();
        let member = match self.assign_op_by_application(l, f0, f, targs, args, base, r, op_end, span) {
            Lowered::Done(done) => {
                self.close(inner);
                return Some(done);
            }
            Lowered::Refused => {
                self.close(inner);
                return None;
            }
            Lowered::Unknown(member) => member,
        };
        // The application typed to learn the `apply` goes, with what it made.
        let trial_made = self.made_local.len();
        let lspan = self.cur_ast().expr_span(l);
        self.drop_made(made, trial_made, counts, lspan);
        self.retract(inner);
        self.hoisted.truncate(hoisted);
        let ast = self.cur_ast();
        if !ast.expr_list(args).iter().all(|&a| !matches!(ast.expr(a), Expr::NamedArg(..) | Expr::Typed(..))) {
            return None;
        }
        self.assign_op_indexed(f, targs, args, member, base, r, op_end, span)
    }

    /// The head of an applied left side `fn(args)` with the tree that names it: a value, `q` of a written
    /// `q.apply(args)`, `super` of `super.apply(args)`. None where `fn(args)` is a method's application or a
    /// constructor's, no `apply` on a value.
    fn assign_op_head(&mut self, f: ExprId) -> Option<(ExprId, AppliedHead)> {
        let ast = self.cur_ast();
        match ast.expr(f) {
            Expr::Select(q, n) if n == names::APPLY => match ast.expr(q) {
                Expr::Super(parent) => Some((q, AppliedHead::Super(parent, ast.expr_span(q)))),
                _ => {
                    let (tq, qty) = self.type_expr(q, None);
                    let qty = self.solve_in(qty);
                    Some((q, AppliedHead::Value(tq, qty)))
                }
            },
            _ => {
                if !self.is_indexable(f) {
                    return None;
                }
                let (tf, fty) = self.indexed_value(f)?;
                let fty = self.solve_in(fty);
                Some((f, AppliedHead::Value(tf, fty)))
            }
        }
    }

    /// The member `name` of the head applied to `lists`.
    fn apply_on_head(&mut self, head: AppliedHead, name: Name, targs: Option<ListRef>, lists: Vec<ArgList>, span: Span) -> (TExprId, TypeId) {
        match head {
            AppliedHead::Value(tf, fty) => self.apply_member(tf, fty, name, targs, lists, span, None),
            AppliedHead::Super(parent, at) => self.apply_super_member(parent, name, targs, lists, at, span, None),
        }
    }

    /// The left side `fn[targs](args)` typed again by itself (`typedExpr(lhs)`, Applications.scala 1514) and
    /// lowered by what it is (`liftAssigned`, EtaExpansion.scala 77-87; `typedAssign`, Typer.scala 1430-1441):
    /// a member `apply` applied to its one plain list, the arguments in the parameters' order, is
    /// `fn.update[targs](args, fn.apply[targs](args) base r)`, `fn` and each argument bound once unless a pure
    /// path, an argument of a by-name or `inline` parameter of the `apply` selected never (`liftArgs`), an
    /// inserted default bound like an argument. An `apply` that is an extension's, one with a using clause, a
    /// structural receiver's `applyDynamic`, arguments the application bound in a block (named out of order),
    /// a method's result: no assignment form.
    #[allow(clippy::too_many_arguments)]
    fn assign_op_by_application(&mut self, l: ExprId, f0: ExprId, f: ExprId, targs: Option<ListRef>, args: ListRef, base: Name, r: ExprId, op_end: u32, span: Span) -> Lowered {
        let mark = self.hoisted.len();
        let Some((head_expr, head)) = self.assign_op_head(f) else { return Lowered::Refused };
        let AppliedHead::Value(tf, fty) = head else { return Lowered::Unknown(None) };
        if fty == ERROR || self.applied_dynamically(fty) {
            return Lowered::Refused;
        }
        let written: Vec<ExprId> = self.cur_ast().expr_list(args).to_vec();
        let lspan = self.cur_ast().expr_span(l);
        let lists = vec![ArgList { args: written.iter().map(|&a| ArgSrc::Ast(a)).collect(), using: false, span: lspan, fun: FunEnd::tree(f0) }];
        // The application as it is written: its arguments typed against their formals, their typings asked
        // back from the member's cache (`Worker::keep_args`).
        let outer_keep = self.keep_args.replace((self.arg_cache_depth(), self.env.file));
        let (cur, cty) = self.apply_member(tf, fty, names::APPLY, targs, lists, lspan, None);
        let handed = std::mem::replace(&mut self.keep_args, outer_keep).is_none();
        let kept = if handed { std::mem::take(&mut self.kept_args) } else { Vec::new() };
        let cty = self.solve_in(cty);
        let cty = self.dealias(cty);
        if cty == ERROR {
            return Lowered::Refused;
        }
        let (member, nodes) = match self.prog.expr(cur) {
            // Arguments bound in a block, or an expansion: no `Apply` for `liftAssigned`.
            TExpr::Block(..) => return Lowered::Refused,
            TExpr::CallMethod(_, m, list) => (Some(m), self.prog.expr_list(list).to_vec()),
            _ => (None, Vec::new()),
        };
        let mut params: Vec<ParamSig> = Vec::new();
        // `liftArgs` leaves an argument of a by-name or `inline` parameter only where the method selected is no
        // polymorphic one (`methRef.widen match case mt: MethodType`): a polymorphic `apply`'s are all bound.
        let mut leaves = false;
        if let Some(m) = member {
            let info = self.syms.sym(m);
            if info.name != names::APPLY || info.ext_clauses != 0 || info.kind != SymKind::Def {
                return Lowered::Refused;
            }
            let sig = self.sig_arc(m);
            if sig.clauses.len() != 1 || sig.clauses[0].is_using || sig.clauses[0].is_implicit {
                return Lowered::Refused;
            }
            params = sig.clauses[0].params.clone();
            leaves = sig.tparams.is_empty();
        }
        // The typings of the arguments as written, one each.
        let ast = self.cur_ast();
        let keys = written.iter().map(|&a| match ast.expr(a) {
            Expr::NamedArg(_, v) => v,
            Expr::Typed(inner, t) if matches!(ast.ty(t), crate::ast::TyExpr::Repeated(_)) => inner,
            _ => a,
        });
        if kept.len() != written.len() || !kept.iter().zip(keys).all(|(&(key, typed), want)| key == want && typed.is_some()) {
            return Lowered::Unknown(member);
        }
        // The parameter each written argument goes to: a named one's by its name, the others in their order.
        let named = written.iter().any(|&a| matches!(ast.expr(a), Expr::NamedArg(..)));
        let at: Vec<Option<usize>> = written
            .iter()
            .enumerate()
            .map(|(k, &a)| match ast.expr(a) {
                Expr::NamedArg(n, _) => params.iter().position(|p| p.name == n),
                _ => Some(k),
            })
            .collect();
        let defaults = nodes.iter().filter(|&&n| matches!(self.prog.expr(n), TExpr::Default(..))).count();
        // One node a parameter, the written arguments in their order between the defaults; else (a repeated
        // parameter's sequence, an intrinsic's node) the arguments as written.
        let by_parameter = member.is_some() && nodes.len() == kept.len() + defaults && nodes.len() <= params.len();
        if (defaults > 0 || named) && !by_parameter {
            return Lowered::Unknown(member);
        }
        // Each argument with its type and whether it stays where it stands.
        let mut slots: Vec<(TExprId, TypeId, bool)> = Vec::new();
        if by_parameter {
            let m = member.unwrap();
            let seen = (defaults > 0).then(|| {
                let sig = self.sig_arc(m);
                self.sig_seen_from(sig, fty, m)
            });
            for (p, &node) in nodes.iter().enumerate() {
                if matches!(self.prog.expr(node), TExpr::Default(..)) {
                    slots.push((node, seen.as_ref().unwrap().clauses[0].params[p].ty, false));
                    continue;
                }
                let Some(k) = at.iter().position(|&x| x == Some(p)) else { return Lowered::Unknown(member) };
                let (key, typed) = kept[k];
                let (ta, aty) = typed.unwrap();
                let aty = self.solve_in(aty);
                let formal = &params[p];
                let stays = (leaves && (formal.by_name || self.is_inline_param(formal, Some((m, p))))) || self.named_alone_stays(key, ta);
                slots.push((ta, aty, stays));
            }
        } else {
            let exempt = self.apply_formals_left(fty, kept.len());
            for (k, &(key, typed)) in kept.iter().enumerate() {
                let (ta, aty) = typed.unwrap();
                let aty = self.solve_in(aty);
                let stays = exempt[k] || self.named_alone_stays(key, ta);
                slots.push((ta, aty, stays));
            }
        }
        Lowered::Done(self.assign_op_update(head_expr, head, targs, cur, cty, slots, base, r, op_end, span, mark))
    }

    /// Whether `fn(args)` on a head of type `fty` is a dynamic method's application: the head derives from
    /// `scala.Dynamic` and has no `apply` (`fn.applyDynamic("apply")(args)`, `typer::dynamic`). The application as
    /// written is the first try's; the reference makes no `update` of a dynamic application (`typedOpAssign`
    /// assigns to the application it typed), so there is no assignment form.
    fn applied_dynamically(&mut self, fty: TypeId) -> bool {
        fty != ERROR && self.derives_dynamic(fty) && self.find_member(fty, names::APPLY).is_none() && !self.has_lexical_extension_for(fty, names::APPLY)
    }

    /// `fn(i) base= rhs` over the head and the arguments typed alone, as `fn.update(i, fn(i) base rhs)`; `member`
    /// the `apply` the application as written selected, where it told. None where `fn(i)` is no `apply` on a
    /// value.
    #[allow(clippy::too_many_arguments)]
    fn assign_op_indexed(&mut self, f: ExprId, targs: Option<ListRef>, args: ListRef, member: Option<SymId>, base: Name, r: ExprId, op_end: u32, span: Span) -> Option<(TExprId, TypeId)> {
        let mark = self.hoisted.len();
        let (head_expr, head) = self.assign_op_head(f)?;
        if matches!(head, AppliedHead::Value(_, fty) if self.applied_dynamically(fty)) {
            return None;
        }
        let written: Vec<ExprId> = self.cur_ast().expr_list(args).to_vec();
        let mut typed = Vec::new();
        for &a in &written {
            let (ta, aty) = self.type_expr(a, None);
            let aty = self.solve_in(aty);
            typed.push((ta, aty));
        }
        let lists = vec![ArgList { args: typed.iter().map(|&(ta, aty)| ArgSrc::Typed(ta, aty)).collect(), using: false, span, fun: ArgList::NO_FUN }];
        let (cur, cty) = self.apply_on_head(head, names::APPLY, targs, lists, span);
        let cty = self.solve_in(cty);
        let cty = self.dealias(cty);
        if cty == ERROR {
            return None;
        }
        // `liftArgs` leaves an argument of a by-name or an `inline` formal of the `apply` selected.
        let exempt: Vec<bool> = match (member, head) {
            (Some(m), _) => {
                let sig = self.sig_arc(m);
                let leaves = sig.tparams.is_empty();
                (0..typed.len()).map(|k| leaves && sig.clauses[0].params.get(k).is_some_and(|p| p.by_name || self.is_inline_param(p, Some((m, k))))).collect()
            }
            (None, AppliedHead::Value(_, fty)) => self.apply_formals_left(fty, typed.len()),
            (None, AppliedHead::Super(..)) => vec![false; typed.len()],
        };
        let slots = typed.into_iter().enumerate().map(|(k, (ta, aty))| (ta, aty, exempt[k] || self.named_alone_stays(written[k], ta))).collect();
        Some(self.assign_op_update(head_expr, head, targs, cur, cty, slots, base, r, op_end, span, mark))
    }

    /// The assignment form `fn.update(args, fn.apply(args) base r)` over the typed head and the arguments
    /// (`slots`: each with its type and whether it stays where it stands): `liftAssigned` binds the head and
    /// each argument that does not stay and is no pure path; the read is applied again over what was bound,
    /// and `read`, the one typed before, which stands in no tree, gives up its pending call.
    #[allow(clippy::too_many_arguments)]
    fn assign_op_update(&mut self, head_expr: ExprId, head: AppliedHead, targs: Option<ListRef>, read: TExprId, cty: TypeId, slots: Vec<(TExprId, TypeId, bool)>, base: Name, r: ExprId, op_end: u32, span: Span, mark: usize) -> (TExprId, TypeId) {
        let head = match head {
            AppliedHead::Value(tf, fty) => {
                self.drop_pending_copied(&[read], tf);
                let tf = if self.named_alone_stays(head_expr, tf) { tf } else { self.lift_assigned(tf, fty, span) };
                AppliedHead::Value(tf, fty)
            }
            head => head,
        };
        let mut index = Vec::new();
        for (ta, aty, stays) in slots {
            let ta = if stays { ta } else { self.lift_assigned(ta, aty, span) };
            index.push(ArgSrc::Typed(ta, aty));
        }
        let lists = vec![ArgList { args: index.clone(), using: false, span, fun: ArgList::NO_FUN }];
        let (cur, _) = self.apply_on_head(head, names::APPLY, targs, lists, span);
        let (value, vty) = self.binop_or_member(base, cur, cty, cty, r, FunEnd::operand_at(op_end), span, None);
        index.push(ArgSrc::Typed(value, vty));
        let lists = vec![ArgList { args: index, using: false, span, fun: ArgList::NO_FUN }];
        let (te, ty) = self.apply_on_head(head, names::UPDATE, targs, lists, span);
        (self.wrap_hoisted(mark, te), ty)
    }

    /// A union of one primitive class's constants (a join of branches, `0 | 97`) as that class,
    /// its join (`OrType.join`), whose operators and comparisons it has; any other type itself.
    pub(super) fn union_as_class(&mut self, t: TypeId) -> TypeId {
        match self.types.get(t) {
            Type::Union(..) => self.primitive_side(t).unwrap_or(t),
            _ => t,
        }
    }

    /// The type whose builtin operators a left operand of type `widened` has: its own where it is
    /// a primitive (most operands), a numeric opaque type's bound outside its scope, the primitive
    /// side of an intersection or a union, a type parameter's primitive bound.
    pub(super) fn operator_type(&mut self, widened: TypeId) -> TypeId {
        if self.is_primitive(widened) {
            return widened;
        }
        self.numeric_bound(widened).or_else(|| self.primitive_side(widened)).or_else(|| self.primitive_bound(widened)).unwrap_or(widened)
    }

    /// `(l op r)(args)..`, which is `l.op(r)(args)..`: the further lists fill the clauses of the
    /// operator method after its first (an implicit clause passed explicitly), or apply what a
    /// builtin operator gives. None for an operator the typer reads apart from a method call.
    pub(super) fn type_infix_applied(
        &mut self,
        l: ExprId,
        op: Name,
        r: ExprId,
        op_end: u32,
        lists: Vec<ArgList>,
        span: Span,
        expected: Option<TypeId>,
    ) -> Option<(TExprId, TypeId)> {
        let watch = self.watch_infix(op, op_end);
        let typed = self.type_infix_applied_now(l, op, r, op_end, lists, span, expected);
        if let Some(outer) = watch {
            let of = self.unwatch_infix(outer);
            if typed.is_some() {
                self.check_infix_expr(op, r, op_end, of);
            }
        }
        // A right-associative operator's selection is on its right operand.
        if !self.selection_typed.is_empty() {
            for operand in [l, r] {
                let owner = self.qualifier_span(operand);
                self.drop_selection_typed(owner);
            }
        }
        typed
    }

    #[inline(always)]
    fn type_infix_applied_now(
        &mut self,
        l: ExprId,
        op: Name,
        r: ExprId,
        op_end: u32,
        lists: Vec<ArgList>,
        span: Span,
        expected: Option<TypeId>,
    ) -> Option<(TExprId, TypeId)> {
        let op_text = self.name_ref(op);
        let assign_op = crate::lexer::is_op_assignment_name(op_text);
        if matches!(op, names::EQEQ | names::NEQ) || assign_op {
            return None;
        }
        // `(l op: r)(b)`: the application `r.op:(l)` is the function part the lists apply, as
        // dotty's `Desugar.binop` makes it before `typedApply` types it with their prototype, so a
        // later clause's default is the method's (an extension's `rightAssocParams` order).
        if op_text.ends_with(':') {
            return Some(self.type_right_assoc(l, op, r, lists, span, expected));
        }
        let more = lists.iter().any(|l| !l.using);
        let frame = SelectionFrame { file: self.env.file, q: l, name: op, at: r, app: span, expected, kind: FrameKind::Operand { more } };
        let (tl, lty) = self.type_selected(l, frame);
        let solved = self.solve_in(lty);
        let solved = self.instantiate_receiver_var(solved);
        let widened = self.dealias(solved);
        let prim_lty = self.operator_type(widened);
        let prim_lty = self.canon(prim_lty);
        // `(l && r)(ev)`: a Boolean's `&&` short-circuits and its result is applied to the lists
        // as a value; on anything else `&&` is a method, whose further clauses the lists fill
        // (zio-test's `TestTrace.&&` takes `A <:< Boolean`).
        if matches!(op, names::AMPAMP | names::BARBAR) && self.dealias_canonical(prim_lty) == self.b.t_boolean {
            let tr = self.check_right_operand(op, tl, r);
            let prim = if op == names::AMPAMP { PrimOp::BoolAnd } else { PrimOp::BoolOr };
            let te = self.prog.add(TExpr::Prim(prim, tl, tr));
            return Some(self.apply_callee(super::apply::Callee::Value(te, self.b.t_boolean), None, lists, span, expected));
        }
        if (self.is_primitive(prim_lty) || self.canon(prim_lty) == self.b.t_string) && classify(op).is_some() {
            let (te, ty) = self.binop_or_member(op, tl, prim_lty, widened, r, FunEnd::operand_at(op_end), span, None);
            let ty = self.const_folded(te, ty, op, tl, lty);
            return Some(self.apply_callee(super::apply::Callee::Value(te, ty), None, lists, span, expected));
        }
        let keeps_type = self.types.is_path(solved) || matches!(self.types.get(solved), Type::Member(..) | Type::AppMember(..) | Type::Decl(_) | Type::Alias(..));
        let recv_ty = if keeps_type { solved } else { widened };
        // The function tree `Desugar.binop` makes, `Select(l, op)`, ends with the operator.
        let mut all = vec![ArgList { args: self.infix_args(r), using: false, span, fun: FunEnd::operand_at(op_end) }];
        all.extend(lists);
        // A literal operand keeps its own type for a view or an extension where the lookup's is its class.
        if recv_ty != solved && self.is_literal_type(solved) {
            return Some(self.with_own_receiver(tl, solved, |this| this.apply_member(tl, recv_ty, op, None, all, span, expected)));
        }
        Some(self.apply_member(tl, recv_ty, op, None, all, span, expected))
    }

    /// `q.y op= r` where `y` is a method without parameters and `q` has a setter `y_=`: `q.y_=(q.y
    /// base r)`, `q` evaluated once. `receiver` is the typed `q` of `tl`, `at` the place of `q.y`;
    /// `written` what the source wrote on the left.
    #[allow(clippy::too_many_arguments)]
    fn assign_op_through_setter(&mut self, tl: TExprId, lty: TypeId, receiver: (TExprId, TypeId), written: WrittenLeft, base: Name, r: ExprId, op_end: u32, at: Span, span: Span) -> Option<(TExprId, TypeId)> {
        let (tq, qty) = receiver;
        let TExpr::CallMethod(recv, getter, args) = self.prog.expr(tl) else { return None };
        if recv != tq || args.len != 0 || self.syms.sym(getter).kind != SymKind::Def || qty == ERROR {
            return None;
        }
        let solved = self.solve_in(lty);
        let solved = self.instantiate_receiver_var(solved);
        let widened = self.dealias(solved);
        let keeps_type = self.types.is_path(solved) || matches!(self.types.get(solved), Type::Member(..) | Type::AppMember(..) | Type::Decl(_) | Type::Alias(..));
        let lty = if keeps_type { solved } else { widened };
        if lty == ERROR {
            return None;
        }
        let name = self.syms.sym(getter).name;
        let setter = self.interner.intern(&format!("{}_=", self.name_ref(name)));
        if self.find_member(qty, setter).is_none() && !self.has_lexical_extension_for(qty, setter) {
            return None;
        }
        let mark = self.hoisted.len();
        // The read of an inline getter is the member's application over the receiver, at the left
        // operand's place, kept for expansion as the operand's own call was (dotty's `typedOpAssign`
        // types the assignment over the one typed left operand, whose inline call `Inlining`
        // expands like any other): a call node made here would be one nothing expands. The
        // operand's own call stands in no tree, and is given up before the receiver is bound and
        // the right operand typed, where a call still waiting is expanded ahead of them.
        let inline = self.is_inline_callee(getter);
        if inline {
            self.drop_pending_copied(&[tl], tq);
        }
        let tq = self.lift_prefix(tq, qty, getter, written, span);
        let current = if inline {
            self.apply_member(tq, qty, name, None, Vec::new(), at, None).0
        } else {
            let current = self.prog.add(TExpr::CallMethod(tq, getter, args));
            self.prog.set_type(current, lty);
            current
        };
        let (value, vty) = self.binop_or_member(base, current, lty, lty, r, FunEnd::operand_at(op_end), span, None);
        let lists = vec![ArgList { args: vec![ArgSrc::Typed(value, vty)], using: false, span, fun: ArgList::NO_FUN }];
        let (call, _) = self.apply_member(tq, qty, setter, None, lists, span, None);
        Some((self.wrap_hoisted(mark, call), self.b.t_unit))
    }

    /// Whether `t` is a literal's type, which `dealias` widens to its class for a lookup.
    fn is_literal_type(&mut self, t: TypeId) -> bool {
        let t = self.deref(t);
        matches!(self.types.get(t), Type::Lit(_))
    }

    /// `l op r` on the typed `l`: a builtin operator of its type, or the member's application.
    #[allow(clippy::too_many_arguments)]
    fn binop_on_typed(&mut self, tl: TExprId, lty: TypeId, op: Name, r: ExprId, op_end: u32, span: Span, expected: Option<TypeId>) -> (TExprId, TypeId) {
        let own_ty = lty;
        let solved = self.solve_in(lty);
        let solved = self.instantiate_receiver_var(solved);
        let widened = self.dealias(solved);
        // A receiver of a singleton type keeps it for the member call: `this += x` is
        // `this.type` where `+=` is; so does one of an abstract member type, whose implicit
        // scope its bound lacks; the builtin operators see the type it stands for.
        let keeps_type = self.types.is_path(solved) || matches!(self.types.get(solved), Type::Member(..) | Type::AppMember(..) | Type::Decl(_) | Type::Alias(..));
        let lty = if keeps_type { solved } else { widened };
        // A numeric opaque type has the operators of its bound outside its scope; a binder of
        // an intersection with a primitive (`n: Int & T` of a typed pattern) has the primitive's,
        // and a type parameter bounded by a primitive or `String` its bound's.
        let prim_lty = self.operator_type(widened);
        let prim_lty = self.canon(prim_lty);
        // A literal operand keeps its own type for a view or an extension where the lookup's is its class (an
        // opaque type's own scope reads it as its underlying type, as the lookup does).
        let (te, ty) = if lty != solved && self.is_literal_type(solved) {
            self.binop_owning(op, tl, solved, prim_lty, lty, r, op_end, span, expected)
        } else {
            self.binop_or_member(op, tl, prim_lty, lty, r, FunEnd::operand_at(op_end), span, expected)
        };
        (te, self.const_folded(te, ty, op, tl, own_ty))
    }

    /// `binop_or_member` of a literal operand `tl` of type `own`, kept as its own type (`with_own_receiver`): out of
    /// `binop_on_typed`'s line.
    #[inline(never)]
    #[allow(clippy::too_many_arguments)]
    fn binop_owning(&mut self, op: Name, tl: TExprId, own: TypeId, prim_lty: TypeId, lty: TypeId, r: ExprId, op_end: u32, span: Span, expected: Option<TypeId>) -> (TExprId, TypeId) {
        self.with_own_receiver(tl, own, |this| this.binop_or_member(op, tl, prim_lty, lty, r, FunEnd::operand_at(op_end), span, expected))
    }

    /// `l base= r` on the left side typed by itself as `tl`, a `var`: `l = l base r`, a field's prefix
    /// evaluated once; `written` what the source wrote on the left. None where `tl` is no `var`.
    #[allow(clippy::too_many_arguments)]
    fn assign_op_to_var(&mut self, tl: TExprId, lty: TypeId, written: WrittenLeft, base: Name, r: ExprId, op_end: u32, span: Span) -> Option<(TExprId, TypeId)> {
        let solved = self.solve_in(lty);
        let solved = self.instantiate_receiver_var(solved);
        let widened = self.dealias(solved);
        let keeps_type = self.types.is_path(solved) || matches!(self.types.get(solved), Type::Member(..) | Type::AppMember(..) | Type::Decl(_) | Type::Alias(..));
        let lty = if keeps_type { solved } else { widened };
        // A `var` of the block across a trait a class owns: the setter of the proxy the class
        // of the block holds, of `l base r` read through its getter (`proxies.rs`), in a program
        // that has one.
        if let (true, TExpr::CallMethod(recv, g, _)) = (self.prog.has_block_class_traits(), self.prog.expr(tl)) {
            if let Some(&super::HeldOf::Get(v)) = self.held_proxy_of.get(&g) {
                if self.syms.sym(v).kind == SymKind::Var {
                    let Owner::Class(holder) = self.syms.sym(g).owner else { unreachable!("a proxy is a member") };
                    let (value, vty) = self.binop_or_member(base, tl, lty, lty, r, FunEnd::operand_at(op_end), span, None);
                    let value = self.adapt(value, vty, lty, span);
                    return Some((self.held_write(holder, v, recv, value), self.b.t_unit));
                }
            }
        }
        let target = match self.prog.expr(tl) {
            TExpr::Local(s) | TExpr::Static(s) | TExpr::Field(_, s) => Some(s),
            _ => None,
        };
        if !target.is_some_and(|s| self.syms.sym(s).kind == SymKind::Var) {
            return None;
        }
        // A compound assignment is an Assign (Applications.scala 1513, typedOpAssign): through another instance it
        // marks a private var assigned non-locally as `o.x = v` does (`note_private_selection`), its receiver as
        // written, before it is evaluated once.
        if let TExpr::Field(recv, s) = self.prog.expr(tl) {
            self.note_private_selection(s, recv, true);
        }
        let mark = self.hoisted.len();
        let tl = self.field_target_once(tl, written, span);
        let (value, vty) = self.binop_or_member(base, tl, lty, lty, r, FunEnd::operand_at(op_end), span, None);
        let value = self.adapt(value, vty, lty, span);
        let assign = self.assignment(tl, value);
        Some((self.wrap_hoisted(mark, assign), self.b.t_unit))
    }

    /// The builtin primitive or `String` that bounds the type parameter `t` from above
    /// (`L <: String`), whose operators the parameter has as scalac finds them on the bound.
    pub(super) fn primitive_bound(&mut self, t: TypeId) -> Option<TypeId> {
        let mut t = t;
        for _ in 0..8 {
            let Type::Param(p) = self.types.get(t) else { return None };
            let upper = self.dealias_canonical(self.syms.tparam(p).upper);
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

    /// The builtin primitive that an intersection type contains, if one of its sides is one, or
    /// that a union joins to.
    pub(crate) fn primitive_side(&mut self, t: TypeId) -> Option<TypeId> {
        // A union of constants of one primitive class (a join of branches, `1 | 2`) has that
        // class's operators, its members those of its join (`OrType.join`).
        if let Type::Union(..) = self.types.get(t) {
            let j = self.union_join(t)?;
            let j = self.dealias_canonical(j);
            return (j == self.b.t_boolean || j == self.b.t_string || self.is_numeric(j).is_some()).then_some(j);
        }
        let Type::Inter(a, b) = self.types.get(t) else { return None };
        for side in [a, b] {
            let side = self.dealias_canonical(side);
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
        fun: FunEnd,
        span: Span,
        expected: Option<TypeId>,
    ) -> (TExprId, TypeId) {
        // The short-circuit operators of a Boolean, which `b ||= c` reaches as `b = b || c`.
        if (op == names::AMPAMP || op == names::BARBAR) && self.dealias_canonical(prim_lty) == self.b.t_boolean {
            let mark = if op == names::AMPAMP { self.push_tested(tl) } else { self.tested.len() };
            let (tr, typed, rty) = self.checked_operand_typed(r, self.b.t_boolean);
            self.pop_tested(mark);
            let prim = if op == names::AMPAMP { PrimOp::BoolAnd } else { PrimOp::BoolOr };
            let te = self.prog.add(TExpr::Prim(prim, tl, tr));
            self.binop_rhs = Some((te, typed, rty));
            return (te, self.b.t_boolean);
        }
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
        let lists = vec![ArgList { args, using: false, span, fun }];
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
        if self.is_numeric(rty).is_some() || self.canon(rty) == self.b.t_string {
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
                let lists = vec![ArgList { args: vec![ArgSrc::Typed(tr, rty)], using: false, span, fun: ArgList::NO_FUN }];
                let applied = self.apply_member(converted, rty, op, None, lists, span, expected);
                self.close(mark);
                return Some(applied);
            }
        }
        self.retract(mark);
        None
    }

    /// `l op: r` is `r.op:(l)`, typed as any application and then lifted as `Typer.typedInfixOp` lifts it
    /// (Typer.scala 3696-3718): each completed argument of the application that is not pure, for a parameter
    /// neither by-name nor inline, is bound ahead of it (`lifted_list`, `Worker::lift_impure`), in the arguments'
    /// order; not in a quote pattern, and not beside a receiver whose evaluation can do nothing, where the binding
    /// could not be told from its absence (`Worker::is_inert_path`). In an inline method's body the receiver is
    /// what an expansion makes of it (a parameter stands for its argument, `this` for the prefix): the argument is
    /// bound there whatever the receiver, and the expansion takes the binding back beside an inert one
    /// (`InlineDefinition::lifted_args`, `unhoist`). An extension method `op:` takes `l` as
    /// its leading parameter and `r` as its argument, and is looked for in scope and in the
    /// implicit scope of `r`.
    fn type_right_assoc(
        &mut self,
        l: ExprId,
        op: Name,
        r: ExprId,
        more: Vec<ArgList>,
        span: Span,
        expected: Option<TypeId>,
    ) -> (TExprId, TypeId) {
        let mark = self.hoisted.len();
        // The right operand is the qualifier of the operator (`Desugar.binop`'s `r.op(l)`).
        let further = more.iter().any(|l| !l.using);
        let frame = SelectionFrame { file: self.env.file, q: r, name: op, at: l, app: span, expected, kind: FrameKind::Operand { more: further } };
        let (tr, rty) = self.type_selected(r, frame);
        // The receiver's variables as a selection's receiver's (`type_receiver`): one its
        // interpolation left open stays open, so that the body of `foldLeft(Nil)((acc, x) => x ::
        // acc)` settles the element of the improved `acc: List[?X]` rather than the first `::`
        // fixing it to `Nothing`.
        let rty = self.receiver_vars(rty);
        // `typedInfixOp` lifts nothing under `Mode.QuotedExprPattern`.
        let lifts = !self.in_quote_pattern();
        if rty == ERROR || self.has_member_or_std_extension(rty, op) || (op == names::CONS_TUPLE && self.tuple_elements(rty).is_some()) {
            let args = self.infix_args(l);
            // `Desugar.binop`'s `Select(r, op)` of a right-associative operator spans the operator and the right
            // operand, which the left one, its argument, stands before.
            let fun = FunEnd::operand_tree(r);
            let mut lists = vec![ArgList { args, using: false, span, fun }];
            lists.extend(more);
            let lifted = (lifts && (self.inline.checking > 0 || !self.is_inert_path(tr))).then_some(fun);
            let outer = std::mem::replace(&mut self.lifted_list, lifted);
            let (te, ty) = self.apply_member(tr, rty, op, None, lists, span, expected);
            self.lifted_list = outer;
            return (self.wrap_hoisted(mark, te), ty);
        }
        let (tl, lty) = self.type_expr(l, None);
        let lty = self.solve_in(lty);
        if lty == ERROR {
            self.type_args_for_errors(&more);
            return (self.prog.add(TExpr::Unit), ERROR);
        }
        let mut first = vec![ArgList { args: vec![ArgSrc::Typed(tr, rty)], using: false, span, fun: ArgList::NO_FUN }];
        first.extend(more.iter().cloned());
        let mut lists = Some(first);
        if let Some(res) = self.try_extensions(tl, lty, rty, op, None, &mut lists, span, expected) {
            return res;
        }
        // `r` converted to something with the member (`LazyList`'s `#::` through `toDeferrer`): the completed
        // receiver is the conversion's application, no pure path, and the operand, passed typed, is lifted in the
        // application on it (`lifted_operand`).
        let mut member_lists = vec![ArgList { args: vec![ArgSrc::Typed(tl, lty)], using: false, span, fun: ArgList::NO_FUN }];
        member_lists.extend(more.iter().cloned());
        let outer = std::mem::replace(&mut self.lifted_operand, lifts.then_some(tl));
        let converted = self.apply_through_conversion(tr, rty, op, None, &member_lists, span, expected);
        self.lifted_operand = outer;
        if let Some((te, ty)) = converted {
            return (self.wrap_hoisted(mark, te), ty);
        }
        // The right operand derives from `scala.Dynamic` (`typer::dynamic`): the operator is its dynamic method's
        // name, the left operand, passed typed, lifted in the application as for a converted receiver.
        let outer = std::mem::replace(&mut self.lifted_operand, lifts.then_some(tl));
        let rewritten = self.dynamic_rewrite(tr, rty, op, None, member_lists, span, expected);
        self.lifted_operand = outer;
        if let Ok((te, ty)) = rewritten {
            return (self.wrap_hoisted(mark, te), ty);
        }
        let hint = self.dynamic_hint(rty);
        let msg = format!("value {} is not a member of {}{}", self.name_str(op), self.show(self.widen_lit(rty)), hint);
        self.error(span, msg);
        self.type_args_for_errors(&more);
        (self.prog.add(TExpr::Unit), ERROR)
    }

    /// For each of the `n` arguments of `f(args)`, whether the formal of the `apply` that `fty` has is by-name
    /// or `inline`: `Lifter.liftArgs` (EtaExpansion.scala 100-108) leaves such an argument as it is, whatever
    /// its purity. An `apply` with alternatives, or one that is an extension, has no one list of formals to
    /// ask: none is left.
    fn apply_formals_left(&mut self, fty: TypeId, n: usize) -> Vec<bool> {
        let mut left = vec![false; n];
        let Some((m, _)) = self.find_member(fty, names::APPLY) else { return left };
        if self.syms.alternatives(m).is_some() || self.syms.sym(m).kind != SymKind::Def || !self.sig_of(m).tparams.is_empty() {
            return left;
        }
        let params: Vec<ParamSig> = match self.sig_of(m).clauses.iter().find(|c| !c.is_using) {
            Some(clause) => clause.params.clone(),
            None => return left,
        };
        for (k, p) in params.iter().enumerate().take(n) {
            left[k] = p.by_name || self.is_inline_param(p, Some((m, k)));
        }
        left
    }

    /// `LiftComplex.noLift` (EtaExpansion.scala 185-187): what the lift of a compound assignment's left side
    /// leaves where it stands: a pure path by the reference's purity. A plain inline call still pending is
    /// none, as for the operands of a right-associative operator (`lifted_as_impure`).
    fn assigned_stays(&mut self, te: TExprId) -> bool {
        use super::elaborate::Purity;
        !self.is_pending_call(te) && self.expr_purity_under(te, Purity::IMPURE).at_least(Purity::PURE_PATH)
    }

    /// `Lifter.lift` (EtaExpansion.scala 55-73) for an operand of a compound assignment's left side, which is
    /// evaluated once and stands on both sides of `lhs = lhs op r` (`Applications.typedOpAssign`,
    /// `liftAssigned`): bound to a value in front unless it is a pure path. What `hoist` binds is bound by it
    /// as before, a pending call expanded first. What `hoist` leaves in place and the reference's purity does
    /// not call a pure path (an `inline` parameter, an object with an initialiser, a lazy value) is bound
    /// here: a value of the body, noted neither as hoisted nor as an operator's lifted argument, so that an
    /// expansion copies it and nothing takes it back.
    fn lift_assigned(&mut self, te: TExprId, ty: TypeId, span: Span) -> TExprId {
        if self.assigned_stays(te) {
            return te;
        }
        // The binding has the widened type of what it binds (`expr.tpe.widen`): an object's is its class's. A
        // body written to products is read back with it, so the whole build gives the value that type too.
        let own = if self.canon(ty) != ANY {
            ty
        } else {
            match (self.prog.type_of(te), self.prog.expr(te)) {
                (Some(t), _) => t,
                (None, TExpr::Module(o)) => self.types.class(o, &[]),
                (None, _) => ty,
            }
        };
        let own = self.widen_type(own);
        // An object nested in a class or trait, reached by a path, is bound under the path's own type
        // (`k.Counter.type`): its class through a prefix has no name of its own in a pickle.
        let own = match self.types.get(own) {
            Type::Nested(_, k) if matches!(self.types.get(k), Type::Class(c, _) if self.syms.class(c).inner_object.is_some()) => self.path_of(te).unwrap_or(own),
            _ => own,
        };
        if !self.is_stable(te) {
            return self.hoist(te, own, span);
        }
        let tmp = self.indexed_local("h", self.hoisted.len() as u32, own, span);
        self.hoisted.push(TStmt::Val(tmp, te));
        self.prog.add(TExpr::Local(tmp))
    }

    /// An identifier that names a static object's member alone stays an identifier for the reference
    /// (`TypeAssigner.prefixIsElidable`): its purity is the member's own (`TreeInfo.refPurity`), and the
    /// object, which the typed node selects it on, is no prefix the source wrote.
    fn named_alone_stays(&mut self, written: ExprId, te: TExprId) -> bool {
        use super::elaborate::Purity;
        if !matches!(self.cur_ast().expr(written), Expr::Ident(_)) {
            return false;
        }
        match self.prog.expr(te) {
            TExpr::Field(r, s) if matches!(self.prog.expr(r), TExpr::Module(_)) => self.symbol_purity(s).at_least(Purity::PURE_PATH),
            _ => false,
        }
    }

    /// Whether `te` is an object nested in a class or trait whose own prefix is no path.
    fn pathless_inner_object(&mut self, te: TExprId) -> bool {
        let Some(t) = self.prog.type_of(te) else { return false };
        let t = self.widen_type(t);
        let inner = matches!(self.types.get(t), Type::Nested(_, k) if matches!(self.types.get(k), Type::Class(c, _) if self.syms.class(c).inner_object.is_some()));
        inner && !self.is_stable(te) && self.path_of(te).is_none()
    }

    /// The prefix `recv` of the member `member` on a compound assignment's left side, lifted
    /// (`lift_assigned`), with what the reference leaves where it stands. Where the source named the member
    /// alone and the reference elides its prefix (a static object's: `prefixIsElidable`), `liftAssigned` meets
    /// an identifier and lifts nothing; where the prefix itself is such an identifier, its purity is the
    /// member's own (`named_alone_stays`). Where an inline method's body reads the member through an accessor
    /// of its class on the companion's module, the binding is made and the read stays on the module:
    /// `PrepareInlineable` rewires the reference to the accessor and drops a qualifier that is no instance of
    /// the accessor's class (`AccessProxies.rewire`).
    fn lift_prefix(&mut self, recv: TExprId, ty: TypeId, member: SymId, written: WrittenLeft, span: Span) -> TExprId {
        let module = matches!(self.prog.expr(recv), TExpr::Module(_));
        match written {
            WrittenLeft::Alone if module => return recv,
            WrittenLeft::Selected(q) if self.named_alone_stays(q, recv) => return recv,
            _ => {}
        }
        let bound = self.lift_assigned(recv, ty, span);
        let by_accessor = module
            && bound != recv
            && self.bodies_in_progress.iter().rev().any(|&b| {
                let info = self.syms.sym(b);
                info.mods & crate::ast::mods::INLINE != 0 && matches!(info.owner, Owner::Class(c) if self.needs_inline_accessor(c, member))
            });
        if by_accessor { recv } else { bound }
    }

    /// The typed `f` of `f(i) op= rhs`; None when `f` is `q.m` for a method `m`: `q.m(i)` is that method's
    /// application, which the operation as written took (`gen.map(f) >>= g`).
    fn indexed_value(&mut self, f: ExprId) -> Option<(TExprId, TypeId)> {
        let ast = self.cur_ast();
        let Expr::Select(q, name) = ast.expr(f) else { return Some(self.type_expr(f, None)) };
        if self.static_ref(f).is_some() {
            return Some(self.type_expr(f, None));
        }
        let (tq, qty) = self.type_expr(q, None);
        let qty = self.solve_in(qty);
        let method = match self.find_member(qty, name) {
            Some((sym, _)) => self.syms.sym(sym).kind == SymKind::Def && self.sig_of(sym).clauses.iter().any(|c| !c.is_using),
            None => qty != ERROR && self.has_lexical_extension_for(qty, name),
        };
        if method {
            return None;
        }
        // A name the value lacks, on a receiver that derives from `scala.Dynamic`: the application as written is
        // `q.applyDynamic("name")(args)` (`typer::dynamic`), which the first try typed; there is no value
        // `q.selectDynamic("name")` to index here, and the reference makes no `update` of a dynamic application.
        if qty != ERROR && self.find_member(qty, name).is_none() && self.derives_dynamic(qty) {
            return None;
        }
        Some(self.apply_member(tq, qty, name, None, Vec::new(), ast.expr_span(f), None))
    }

    /// `e.x op= rhs` reads and writes `e.x`, so `e` is evaluated once (`lift_assigned`). The selection over the
    /// binding is the one the source wrote: it keeps that node's type and place, which a stored body is
    /// written to products with.
    fn field_target_once(&mut self, tl: TExprId, written: WrittenLeft, span: Span) -> TExprId {
        match self.prog.expr(tl) {
            TExpr::Field(recv, s) => {
                let bound = self.lift_prefix(recv, ANY, s, written, span);
                if bound == recv {
                    return tl;
                }
                let field = self.prog.add(TExpr::Field(bound, s));
                // An object nested in a class or trait behind a prefix that is no path (`make().Bin.n += 1`): no
                // path's type stands for what was bound and its class through that prefix has no name in a
                // pickle, so the selection states no type, as before: the body stays out of a pickle, as it did.
                if self.pathless_inner_object(recv) {
                    return field;
                }
                self.prog.copy_span(tl, field);
                if let Some(t) = self.prog.type_of(tl) {
                    self.prog.set_type(field, t);
                }
                field
            }
            _ => tl,
        }
    }

    /// The elements of a tuple literal are the arguments of an infix operation: `a op (b, c)`
    /// is `a.op(b, c)`. Which tuple they were is kept (`Worker::infix_tuples`) for a member's
    /// retry, where dotty applies a unary member to the tuple as written (`ApplyKind.InfixTuple`,
    /// Desugar.scala 1780; `needsTupledDual`, Applications.scala 1671). An operand in parentheses
    /// of its own is the one argument (`Desugar.binop`'s `Parens(arg)`, 1775): `a op ((b, c))`
    /// passes the tuple.
    pub(super) fn infix_args(&mut self, operand: ExprId) -> Vec<ArgSrc> {
        let ast = self.cur_ast();
        match ast.expr(operand) {
            Expr::Tuple(elems) => {
                let elems = ast.expr_list(elems);
                if let [first, _, ..] = *elems {
                    self.infix_tuples.insert((self.env.file, first), operand);
                }
                elems.iter().map(|&e| ArgSrc::Ast(e)).collect()
            }
            Expr::Parens(inner) => vec![ArgSrc::Ast(inner)],
            // A named tuple's elements as named arguments, re-tupled for a method of one parameter.
            Expr::NamedTuple(..) if !ast.notes.named_operands.is_empty() => match ast.notes.named_operands.binary_search_by_key(&operand, |&(e, _)| e) {
                Ok(i) => {
                    let args = ast.expr_list(ast.notes.named_operands[i].1);
                    self.infix_tuples.insert((self.env.file, args[0]), operand);
                    args.iter().map(|&e| ArgSrc::Ast(e)).collect()
                }
                Err(_) => vec![ArgSrc::Ast(operand)],
            },
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

    /// On the JVM, the null literal written alone in braces as an argument, `x eq { null }`: dotty's tree there is
    /// `Block(Nil, null)`, no literal, which its back end loads beside the other side (`Gen::null_literal`); its
    /// `ConstantTree` is the block's type's, no constant. The parser keeps no node of the block (`Ast::is_braced`),
    /// so the null is marked as an ascription is (`Program::is_ascribed`), no constant noted. A receiver's block is
    /// lifted by dotty and left unmarked.
    pub(super) fn mark_braced_null_argument(&mut self, written: ExprId, te: TExprId) {
        if self.jvm && matches!(self.prog.expr(te), TExpr::Null) && self.cur_ast().is_braced(written) {
            self.prog.mark_ascribed(te);
        }
    }

    fn type_equality(&mut self, l: ExprId, r: ExprId, negate: bool, span: Span) -> (TExprId, TypeId) {
        let (tl, lty) = self.type_expr(l, None);
        let (tr, rty) = self.type_expr(r, None);
        self.mark_braced_null_argument(r, tr);
        let (te, ty) = self.typed_equality(tl, lty, tr, rty, negate, span);
        let op = if negate { names::NEQ } else { names::EQEQ };
        (te, self.const_folded_binary(te, ty, op, (tl, lty), (tr, rty)))
    }

    /// `l == r` or `l != r` over operands typed already, as the typer reads the operator in
    /// source: also a macro's reconstruction of `a != b`.
    pub fn typed_equality(&mut self, mut tl: TExprId, lty: TypeId, mut tr: TExprId, rty: TypeId, negate: bool, span: Span) -> (TExprId, TypeId) {
        // The operands are related with the variables their interpolation left open, as dotty's
        // `assumedCanEqual` relates them under the default equality (`ltp <:< lift(rtp) || rtp
        // <:< lift(ltp)`, which constrains them): `Map("a" -> 1) == Map(1 -> 1)` equates the key
        // variables, where solving each operand alone had made two unrelated maps.
        if !self.strict_equality() && (self.types.has_vars(lty) || self.types.has_vars(rty)) {
            let snap = self.snapshot();
            if !self.is_sub(lty, rty) {
                self.rollback(&snap);
                if !self.is_sub(rty, lty) {
                    self.rollback(&snap);
                }
            }
        }
        let lty = self.solve_in(lty);
        let lty = self.dealias(lty);
        let lty = self.union_as_class(lty);
        let lty = self.canon(lty);
        let rty = self.solve_in(rty);
        let rty = self.dealias(rty);
        let rty = self.union_as_class(rty);
        let rty = self.canon(rty);
        if self.unused.on() {
            self.mark_can_equal(lty, rty);
        }
        let numeric = (self.is_numeric(lty), self.is_numeric(rty));
        if numeric.0.is_some() && numeric.1.is_some() {
            tl = self.unbox_erased(tl, lty);
            tr = self.unbox_erased(tr, rty);
        }
        if numeric.0.is_none() {
            let op = if negate { names::NEQ } else { names::EQEQ };
            if self.declares_equality_for(tl, lty, op, rty) {
                let lists = vec![ArgList { args: vec![ArgSrc::Typed(tr, rty)], using: false, span, fun: ArgList::NO_FUN }];
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
                let abstract_ok = if strict { self.can_equal_given(lty, rty, true, span) } else { self.compares_with_null(other) };
                if !self.null_conforms(other) && other != self.b.t_null && !abstract_ok {
                    let msg = format!("Values of types {} and {} cannot be compared with == or !=", self.show(lty), self.show(rty));
                    self.error(span, msg);
                }
                true
            }
            _ => {
                if !self.can_equal(lty, rty, span) {
                    let msg = format!(
                        "Values of types {} and {} cannot be compared with == or !=",
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
    /// have none and are not asked. The class is the receiver's, named plainly or through a stable path
    /// (`named_class`), the parameter's type seen from that path; through a type's projection there is no
    /// path to see it from, and `Any`'s stays. An alternative the site may not access is none (dotty's
    /// `TypeAssigner.accessibleType`: the accessible alternatives, else the inherited member, here `Any`'s).
    fn declares_equality_for(&mut self, recv: TExprId, lty: TypeId, op: Name, rty: TypeId) -> bool {
        let (c, prefix) = match self.types.get(lty) {
            Type::Class(c, _) => (c, None),
            Type::Nested(..) => match self.named_class(lty) {
                Some((c, _, Some(p))) if self.is_stable_type(p) => (c, Some(p)),
                _ => return false,
            },
            _ => return false,
        };
        // `Any`'s `==` is no member: a completed class that neither declares one nor inherits
        // one is done with here.
        let declared_nowhere = self.syms.class_done(c).map_or(false, |info| {
            !info.members.contains_key(&op) && !info.base_types.iter().skip(1).any(|&(b, _)| self.syms.class(b).members.contains_key(&op))
        });
        if declared_nowhere {
            return false;
        }
        let Some((m, owner_ty)) = self.find_member(lty, op) else { return false };
        let alternatives: Vec<SymId> = match self.syms.alternatives(m) {
            Some(alts) => alts.to_vec(),
            None => vec![m],
        };
        let through_this = matches!(self.prog.expr(recv), TExpr::This | TExpr::Super(_));
        let subst = self.owner_subst(owner_ty);
        alternatives.into_iter().any(|a| {
            if !self.is_accessible(a) {
                return false;
            }
            if self.syms.sym(a).mods & crate::ast::mods::PROTECTED != 0 {
                let declared = self.access_declaration_of(a);
                if self.protected_denied(declared, through_this, lty).is_some() {
                    return false;
                }
            }
            let pty = {
                let sig = self.sig_of(a);
                // The operand's clause first, using clauses after it (`def ==(i: Int)(using Boolean)`).
                let Some((clause, rest)) = sig.clauses.split_first() else { return false };
                if clause.is_using || !rest.iter().all(|c| c.is_using) {
                    return false;
                }
                let [param] = clause.params.as_slice() else { return false };
                param.ty
            };
            let pty = self.types.subst(pty, &subst);
            let pty = match (prefix, self.syms.class(c).owner) {
                (Some(p), Owner::Class(o)) => self.as_seen_from(pty, p, o),
                _ => pty,
            };
            let mark = self.snapshot();
            let fits = self.is_sub(rty, pty);
            self.rollback(&mark);
            fits
        })
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
    /// Whether a pattern compared by `==` (a literal, a stable identifier) asks for the comparison's
    /// evidence: dotty's `checkEqualityEvidence` (Typer.scala 5233 at 3.8.4) asks it unless the
    /// scrutinee's type conforms to the pattern's, read before the pattern refines the scrutinee.
    pub(super) fn pattern_asks_equality(&mut self, pattern: TypeId, scrutinee: TypeId) -> bool {
        let snap = self.snapshot();
        let conforms = self.is_sub(scrutinee, pattern);
        self.rollback(&snap);
        !conforms
    }

    /// The evidence of `pattern == <a value of the scrutinee's type>`, which `checkEqualityEvidence`
    /// types: the test a comparison in an expression runs, the pattern on the left. Numbers compare,
    /// `null` with a reference, a `==` the pattern's class declares for the scrutinee is called, and
    /// the rest asks `CanEqual`.
    pub(super) fn check_pattern_equality(&mut self, te: TExprId, pattern: TypeId, scrutinee: TypeId, span: Span) {
        // Under strict equality alone: without it the comparison's defaults (`validEqAnyArgs`) are not the
        // reference's whole rule here, and a pattern would be refused where `==` is refused wrongly.
        if !self.strict_equality() || self.assumed_pattern_equality(te, pattern, scrutinee, span) {
            return;
        }
        let normal = |t: &mut Self, ty: TypeId| {
            let ty = t.solve_in(ty);
            let ty = t.dealias(ty);
            let ty = t.union_as_class(ty);
            t.canon(ty)
        };
        let (lty, rty) = (normal(self, pattern), normal(self, scrutinee));
        if self.is_numeric(lty).is_some() && self.is_numeric(rty).is_some() {
            return;
        }
        let refused = if lty == self.b.t_null || rty == self.b.t_null {
            let other = if lty == self.b.t_null { rty } else { lty };
            let abstract_ok = if self.strict_equality() { self.can_equal_given(lty, rty, true, span) } else { self.compares_with_null(other) };
            !self.null_conforms(other) && other != self.b.t_null && !abstract_ok
        } else if self.is_numeric(lty).is_none() && self.declares_equality_for(te, lty, names::EQEQ, rty) {
            false
        } else {
            !self.can_equal(lty, rty, span)
        };
        if refused {
            let msg = format!("Values of types {} and {} cannot be compared with == or !=", self.show(lty), self.show(rty));
            self.error(span, msg);
        }
    }

    /// dotty's `assumedCanEqual` for a pattern under strict equality (Implicits.scala 1051 at 3.8.4): an
    /// erroneous side, or, with `experimental.strictEqualityPatternMatching`, an enum value's or a case
    /// object's pattern that conforms to the scrutinee lifted. The scrutinee as written is taken too: an
    /// alias of an abstract type is its target here, which the reference's lift keeps unlifted.
    fn assumed_pattern_equality(&mut self, te: TExprId, pattern: TypeId, scrutinee: TypeId, span: Span) -> bool {
        let (l, r) = (self.deref_canonical(pattern), self.deref_canonical(scrutinee));
        if l == ERROR || r == ERROR {
            return true;
        }
        let l = self.lift_for_equality(l);
        let mark = self.snapshot();
        let assumed = if self.strict_equality() {
            let adt = match self.prog.expr(te) {
                TExpr::Module(c) => self.syms.class(c).mods & crate::ast::mods::CASE != 0,
                TExpr::Static(s) | TExpr::Field(_, s) => matches!(self.syms.sym(s).kind, SymKind::EnumValue(_)),
                _ => false,
            };
            let feature = self.interner.intern("experimental.strictEqualityPatternMatching");
            adt && self.feature_enabled(self.env.file, span.start, feature) && {
                let lifted = self.lift_for_equality(r);
                self.is_sub(l, lifted) || self.is_sub(l, r)
            }
        } else {
            false
        };
        self.rollback(&mark);
        assumed
    }

    /// dotty's `lift` in `assumedCanEqual` (Implicits.scala 1054): a singleton widened, a non-opaque
    /// abstract type its upper bound and a refinement its parent at any variance; the parts of any
    /// other type lifted where the position is covariant.
    fn lift_for_equality(&mut self, t: TypeId) -> TypeId {
        self.lift_at(t, true, &mut Vec::new())
    }

    /// `seen`: the types being replaced by a bound or a widening; one met again inside its own
    /// bound (`T <: Seq[T]`) is kept, as the reference keeps a recursive bound's `LazyRef`.
    fn lift_at(&mut self, t: TypeId, covariant: bool, seen: &mut Vec<TypeId>) -> TypeId {
        let t = self.deref(t);
        let widened = if self.types.is_path(t) { let w = self.widen_path(t); self.widen_lit(w) } else { self.widen_lit(t) };
        if widened != t {
            return self.lift_through(t, widened, covariant, seen);
        }
        match self.types.get(t) {
            Type::Param(p) => {
                let upper = self.syms.tparam(p).upper;
                self.lift_through(t, upper, covariant, seen)
            }
            // An alias and an opaque type are kept, as the reference keeps them.
            Type::Member(..) | Type::AppMember(..) | Type::Decl(_) if self.abstract_type_member(t) => {
                let (lower, upper) = self.member_bounds(t);
                if lower == upper || upper == t { t } else { self.lift_through(t, upper, covariant, seen) }
            }
            Type::Refined(parent, _) => self.lift_at(parent, covariant, seen),
            Type::Union(a, b, _) if covariant => {
                let (la, lb) = (self.lift_at(a, true, seen), self.lift_at(b, true, seen));
                if (la, lb) == (a, b) { t } else { self.types.union(la, lb) }
            }
            Type::Inter(a, b) if covariant => {
                let (la, lb) = (self.lift_at(a, true, seen), self.lift_at(b, true, seen));
                if (la, lb) == (a, b) { t } else { self.types.inter(la, lb) }
            }
            Type::Class(c, args) if covariant => {
                let args: Vec<TypeId> = self.types.items(args).to_vec();
                self.unmade_read(c, args.len());
                let variances: Vec<i8> = self.syms.class(c).tparams.iter().map(|&p| self.syms.tparam(p).variance).collect();
                if args.is_empty() || args.len() != variances.len() {
                    return t;
                }
                let lifted: Vec<TypeId> = args.iter().zip(&variances).map(|(&a, &v)| self.lift_at(a, v > 0, seen)).collect();
                if lifted == args { t } else { self.types.class(c, &lifted) }
            }
            _ => t,
        }
    }

    fn lift_through(&mut self, t: TypeId, to: TypeId, covariant: bool, seen: &mut Vec<TypeId>) -> TypeId {
        if seen.contains(&t) {
            return t;
        }
        seen.push(t);
        let lifted = self.lift_at(to, covariant, seen);
        seen.pop();
        lifted
    }

    /// Whether `t` names a type member declared with bounds (`type T <: U`), not an alias.
    fn abstract_type_member(&mut self, t: TypeId) -> bool {
        match self.types.get(t) {
            Type::Member(prefix, name) => matches!(self.type_member(prefix, name), Some(super::members::MemberInfo::Bounds(..))),
            Type::AppMember(m, _) => self.abstract_type_member(m),
            Type::Decl(a) => {
                self.complete_alias(a);
                self.syms.aliases[a.idx()].bounds.is_some()
            }
            _ => false,
        }
    }

    /// Whether `CanEqual[l, r]` holds.
    fn can_equal(&mut self, l: TypeId, r: TypeId, span: Span) -> bool {
        if self.equality_in_progress.contains(&(l, r, false)) {
            return false;
        }
        self.equality_in_progress.push((l, r, false));
        let holds = self.can_equal_exact(l, r, true, span) || self.can_equal_by_bounds(l, r, span);
        self.equality_in_progress.pop();
        holds
    }

    /// The evidence is contravariant (`CanEqual[-L, -R]`), so an instance for a side's upper bound
    /// answers for it: a type parameter, an abstract member or an opaque type stands for its bound
    /// (`T <: String` compares as `String`), an intersection for either part. Only evidence: the
    /// defaults of multiversal equality are decided on the operands as written, as dotty's
    /// `validEqAnyArgs` tests them, never on a bound that has lost an operand's own instance.
    fn can_equal_by_bounds(&mut self, l: TypeId, r: TypeId, span: Span) -> bool {
        if self.equality_in_progress.contains(&(l, r, true)) {
            return false;
        }
        self.equality_in_progress.push((l, r, true));
        let holds = self.evidence_by_bounds(l, r, span);
        self.equality_in_progress.pop();
        holds
    }

    fn evidence_by_bounds(&mut self, l: TypeId, r: TypeId, span: Span) -> bool {
        let (ul, ur) = (self.equality_upcast(l), self.equality_upcast(r));
        if (ul, ur) != (l, r) && (self.can_equal_exact(ul, ur, false, span) || self.can_equal_by_bounds(ul, ur, span)) {
            return true;
        }
        for (side, other, left) in [(l, r, true), (r, l, false)] {
            let side = self.deref(side);
            if let Type::Inter(a, b) = self.types.get(side) {
                for part in [a, b] {
                    let (x, y) = if left { (part, other) } else { (other, part) };
                    if self.can_equal_exact(x, y, false, span) || self.can_equal_by_bounds(x, y, span) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// One step of `can_equal`'s upcast: a type parameter's, an abstract member's or an opaque
    /// type's upper bound, the type itself where it has none narrower than `Any`.
    fn equality_upcast(&mut self, t: TypeId) -> TypeId {
        let mut seen: Vec<TypeId> = Vec::new();
        let mut t = self.deref(t);
        loop {
            let upper = match self.types.get(t) {
                Type::Param(p) => self.syms.tparam(p).upper,
                Type::Member(..) | Type::AppMember(..) | Type::Decl(_) => self.member_upper(t),
                Type::Class(c, _) if self.syms.class(c).kind == ClassKind::Opaque => self.opaque_bound(t).unwrap_or(ANY),
                _ => return t,
            };
            let upper = self.deref(upper);
            if upper == ANY || upper == t || seen.contains(&upper) {
                return t;
            }
            seen.push(t);
            t = upper;
        }
    }

    fn can_equal_exact(&mut self, l: TypeId, r: TypeId, defaults: bool, span: Span) -> bool {
        match self.can_equal_predefined(l, r, defaults, span) {
            Some(true) => true,
            // A given decides, and one of the pair itself answers where the standard instances
            // refuse it (`given CanEqual[Option[Camera], Option[Camera]]` in a companion), as
            // `Implicits.checkCanEqual` searches the pair's implicit scope; two tuples have the
            // standard `canEqualTuple` alone, which the element-wise rule is.
            Some(false) if self.both_tuples(l, r) => false,
            Some(false) | None => self.can_equal_given(l, r, defaults, span),
        }
    }

    fn both_tuples(&mut self, l: TypeId, r: TypeId) -> bool {
        let (l, r) = (self.deref(l), self.deref(r));
        match (self.class_of(l), self.class_of(r)) {
            (Some(lc), Some(rc)) => self.is_tuple_class(lc) && self.is_tuple_class(rc),
            _ => false,
        }
    }

    /// The part of the rule that needs no given: `Some(true)` when the instance is predefined,
    /// `Some(false)` when a standard type rules the comparison out, `None` when a given decides.
    /// `defaults`: whether multiversal equality's defaults apply (the operands as written).
    fn can_equal_predefined(&mut self, l: TypeId, r: TypeId, defaults: bool, span: Span) -> Option<bool> {
        let l = self.deref_canonical(l);
        let r = self.deref_canonical(r);
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
        let loose = defaults && !strict;
        if l == r && (loose || self.is_primitive(l)) {
            return Some(true);
        }
        if loose {
            let mark = self.snapshot();
            let related = self.is_sub(l, r) || self.is_sub(r, l);
            self.rollback(&mark);
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
        if self.derived_can_equal(l, r, span) {
            return Some(true);
        }
        if loose && !self.has_reflexive_can_equal(l) && !self.has_reflexive_can_equal(r) {
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
        if self.defined_equality_only == Some(target) || self.can_equal_predefined(l, r, true, span) != Some(true) {
            return None;
        }
        let module = self.syms.class(c).companion?;
        let derived = self.interner.intern("derived");
        let derived = self.module_term(module, derived)?.sym()?;
        let recv = self.prog.add(TExpr::Module(module));
        Some(self.prog.add(TExpr::CallMethod(recv, derived, ListRef::EMPTY)))
    }

    /// The instance `derives CanEqual` stands for (Deriving.scala's `deriveCanEqual` at 3.8.4):
    /// `CanEqual[C[Ls], C[Rs]]` given a `CanEqual[L, R]` for each parameter of kind `*`. The given the
    /// desugaring writes relates `C[A]` to itself alone, so the rule is applied here.
    fn derived_can_equal(&mut self, l: TypeId, r: TypeId, span: Span) -> bool {
        let derived = self.interner.intern("derived$CanEqual");
        for class in [self.class_of(l), self.class_of(r)].into_iter().flatten() {
            let Some(module) = self.syms.class(class).companion else { continue };
            let givens = self.syms.class(module).givens.clone();
            if !givens.into_iter().any(|g| self.syms.sym(g).name == derived) {
                continue;
            }
            let (Some(lb), Some(rb)) = (self.base_type(l, class), self.base_type(r, class)) else { continue };
            let (Type::Class(_, la), Type::Class(_, ra)) = (self.types.get(lb), self.types.get(rb)) else { continue };
            let kinds: Vec<u8> = self.syms.class(class).tparams.iter().map(|&p| self.syms.tparam(p).arity).collect();
            let (la, ra) = (self.types.items(la).to_vec(), self.types.items(ra).to_vec());
            if la.is_empty() || la.len() != kinds.len() || ra.len() != kinds.len() {
                continue;
            }
            if (0..kinds.len()).all(|i| kinds[i] != 0 || self.can_equal(la[i], ra[i], span)) {
                return true;
            }
        }
        false
    }

    /// The standard instances: `Option`, `Seq` and `Set` compare regardless of their elements
    /// unless equality is strict; `Either`, `Map` and tuples compare element by element.
    fn std_can_equal(&mut self, l: TypeId, r: TypeId, span: Span) -> Option<bool> {
        // dotty's `CanEqual` givens, over `scala.collection.{Seq, Set, Map}`.
        let b = &self.b;
        let loose = [b.option, b.seq, b.collection_set.or(b.set)];
        let elementwise = [b.either, b.collection_map.or(b.map)];
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
        let standard = [b.option, b.seq, b.collection_set.or(b.set), b.collection_map.or(b.map), b.either];
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

    /// Whether a given `CanEqual[l, r]` is found; without `defaults` only a definition counts.
    fn can_equal_given(&mut self, l: TypeId, r: TypeId, defaults: bool, span: Span) -> bool {
        let Some(can_equal) = self.can_equal_class() else { return true };
        let target = self.applied_class(can_equal, &[l, r]);
        let target = self.zonk(target);
        let outer = std::mem::replace(&mut self.defined_equality_only, (!defaults).then_some(target));
        let found = self.resolve_given(target, span).is_some();
        self.defined_equality_only = outer;
        self.given_ambiguity = None;
        found
    }

    pub(super) fn mentions_param(&self, t: TypeId) -> bool {
        self.mentions_tparam_of(t, None)
    }

    /// Whether `t` names a type parameter: any one, or one of `tps` when given.
    #[inline(always)]
    pub fn mentions_tparam_of(&self, t: TypeId, tps: Option<&[TParamId]>) -> bool {
        if self.semantic {
            return self.mentions_tparam_of_retaining(t, tps);
        }
        self.mentions_tparam_of_in::<false>(t, tps)
    }

    #[cold]
    #[inline(never)]
    fn mentions_tparam_of_retaining(&self, t: TypeId, tps: Option<&[TParamId]>) -> bool {
        self.mentions_tparam_of_in::<true>(t, tps)
    }

    /// `mentions_tparam_of` in a typing whose types may carry retained forms (`FORMS`, the semantic typing) or not.
    fn mentions_tparam_of_in<const FORMS: bool>(&self, t: TypeId, tps: Option<&[TParamId]>) -> bool {
        // A retained form first: its record's types, then what it is made over (`existsPart`: no annotation's
        // arguments).
        if FORMS {
            if let Some((under, r)) = self.types.form_of(t) {
                return self.mentions_tparam_of_form(t, tps, under, r);
            }
        }
        match self.types.get(t) {
            Type::Param(p) => tps.map_or(true, |tps| tps.contains(&p)),
            Type::AppParam(p, args) => {
                tps.map_or(true, |tps| tps.contains(&p)) || self.types.items(args).iter().any(|&a| self.mentions_tparam_of_in::<FORMS>(a, tps))
            }
            // Through every part of the type (a refinement's, a path's prefix, an annotation's
            // parent, a match type's cases), as dotty's `existsPart` walks it.
            _ => {
                let mut parts = Vec::new();
                self.types.parts(t, &mut parts);
                parts.into_iter().any(|part| self.mentions_tparam_of_in::<FORMS>(part, tps))
            }
        }
    }

    #[cold]
    #[inline(never)]
    fn mentions_tparam_of_form(&self, t: TypeId, tps: Option<&[TParamId]>, under: TypeId, r: crate::types::RetainId) -> bool {
        let mut ctx = vec![super::FORM_MENTIONS_TPARAM, tps.is_some() as u32];
        ctx.extend(tps.unwrap_or(&[]).iter().map(|p| p.0));
        self.types.ask_form_once(&ctx, t, under, r, false, &mut |x| self.mentions_tparam_of_in::<true>(x, tps))
    }

    /// Built-in binary operators on primitives.
    fn prim_binop(&mut self, op: Name, tl: TExprId, lty: TypeId, r: ExprId, span: Span) -> Binop {
        let b_string = self.b.t_string;
        let b_bool = self.b.t_boolean;
        if self.same_type(lty, b_string) {
            if op == names::PLUS {
                let (tr, rty) = self.typed_operand(r, None);
                let rty = self.solve_in(rty);
                let rs = self.to_str(tr, rty);
                let l = self.prog.list(&[tl, rs]);
                let te = self.prog.add(TExpr::StrConcat(l));
                self.binop_rhs = Some((te, tr, rty));
                return Binop::Done(te, b_string);
            }
            if classify(op) == Some(OpClass::Compare) {
                let (tr, typed, rty) = self.checked_operand_typed(r, b_string);
                let te = self.prog.add(TExpr::Prim(compare_op(op), tl, tr));
                self.binop_rhs = Some((te, typed, rty));
                return Binop::Done(te, b_bool);
            }
            return Binop::NotBuiltin;
        }
        if self.same_type(lty, b_bool) {
            let prim = match op {
                names::AMP => PrimOp::BoolStrictAnd,
                names::BAR => PrimOp::BoolStrictOr,
                names::CARET => PrimOp::BoolXor,
                _ => return Binop::NotBuiltin,
            };
            let (tr, typed, rty) = self.checked_operand_typed(r, b_bool);
            let te = self.prog.add(TExpr::Prim(prim, tl, tr));
            self.binop_rhs = Some((te, typed, rty));
            return Binop::Done(te, b_bool);
        }
        let (Some(lrank), Some(class)) = (self.is_numeric(lty), classify(op)) else { return Binop::NotBuiltin };
        let literal_hint = if self.is_const_expr(r) && lrank >= R_LONG { Some(lty) } else { None };
        let (tr, operand_ty) = match (literal_hint, self.cur_ast().expr(r)) {
            // A number literal of the source is the argument of an overloaded operator, which `typedNumber` reads
            // with no expected type: an `Int` by `intFromDigits` (`n + 0xffffffff` adds -1 to a `Long`), that constant
            // then converted to the left operand's class as for any constant.
            (Some(hint), Expr::Number(_)) => {
                let (tr, ty) = self.typed_operand(r, None);
                let at = self.cur_ast().expr_span(r);
                self.int_constant_as(tr, ty, hint, at)
            }
            _ => self.typed_operand(r, literal_hint),
        };
        let done = self.prim_binop_typed(op, tl, lty, lrank, class, tr, operand_ty, span);
        if let Binop::Done(te, _) = done {
            self.binop_rhs = Some((te, tr, operand_ty));
        }
        done
    }

    /// `l op r` as a builtin operator over operands typed already (a macro's reconstruction of
    /// `n * 2`), or `None` where `op` is no builtin operator of the left type.
    pub fn typed_binop(&mut self, op: Name, tl: TExprId, lty: TypeId, tr: TExprId, rty: TypeId, span: Span) -> Option<(TExprId, TypeId)> {
        let (b_string, b_bool) = (self.b.t_string, self.b.t_boolean);
        let lty = self.widen_path(lty);
        let lty = self.widen_lit(lty);
        let lty = self.canon(lty);
        let rty = self.widen_path(rty);
        let rty = self.widen_lit(rty);
        if lty == b_string {
            if op == names::PLUS {
                let rs = self.to_str(tr, rty);
                let l = self.prog.list(&[tl, rs]);
                return Some((self.prog.add(TExpr::StrConcat(l)), b_string));
            }
            if classify(op) == Some(OpClass::Compare) && self.deref_canonical(rty) == b_string {
                return Some((self.prog.add(TExpr::Prim(compare_op(op), tl, tr)), b_bool));
            }
            return None;
        }
        if lty == b_bool && self.deref_canonical(rty) == b_bool {
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
        // As the left operand's (`binop`): a type parameter bounded by a primitive takes the
        // bound's operator, `Int.+(x: Int)` taking a `T <: Int` as scalac's overload does; a union
        // of one primitive class's constants (a join of branches) is an argument of that class to
        // the operator's overloads, as `1 | 2 <: Int` makes it.
        let rty = match self.types.get(rty) {
            Type::Inter(..) | Type::Union(..) => self.primitive_side(rty),
            Type::Param(_) => self.primitive_bound(rty),
            _ => self.numeric_bound(rty),
        }
        .unwrap_or(rty);
        let rty = self.canon(rty);
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
        let t = self.dealias_canonical(t);
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
        let t = self.widen_lit(t);
        self.canon(t) == self.b.t_string
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
        let (folded, fty) = self.binop_or_member(name, te, ty, ty, r, list.fun, span, expected);
        Some((folded, self.const_folded(folded, fty, name, te, ty)))
    }

    fn prefix_on(&mut self, op: Name, operand: TExprId, operand_ty: TypeId, span: Span) -> (TExprId, TypeId) {
        let (te, ty) = self.prefix_on_typed(op, operand, operand_ty, span);
        let unop = format!("unary_{}", self.name_ref(op));
        (te, self.const_folded_select(te, ty, &unop, operand, operand_ty))
    }

    /// `ConstFold.Apply` (`typer/ConstFold.scala` 31-45) of the builtin operation `te` (`x op y`), of type `ty`, the
    /// first operand `x` of the type `x_ty` the typer gave it, the second the one `prim_binop` typed with the node
    /// (`Worker::binop_rhs`): an operation of `foldedBinops` over two `ConstantTree`s has the type of the constant
    /// `foldBinop` gives (`withFoldedType`); the tree and its effects stay as they are.
    #[inline]
    pub(super) fn const_folded(&mut self, te: TExprId, ty: TypeId, op: Name, x: TExprId, x_ty: TypeId) -> TypeId {
        // An operand that can be no `ConstantTree` folds nothing, the common case, decided before anything else.
        if !self.maybe_constant_tree(x, x_ty) {
            return ty;
        }
        let y = match self.binop_rhs {
            Some((node, tr, rty)) if node == te => (tr, rty),
            _ => return ty,
        };
        self.const_folded_binary(te, ty, op, (x, x_ty), y)
    }

    /// `ConstFold.Apply` over operands typed with their types.
    #[inline(never)]
    pub(super) fn const_folded_binary(&mut self, te: TExprId, ty: TypeId, op: Name, x: (TExprId, TypeId), y: (TExprId, TypeId)) -> TypeId {
        if ty == ERROR || matches!(self.types.get(ty), Type::Lit(_)) || !self.maybe_constant_tree(x.0, x.1) {
            return ty;
        }
        let name = self.name_ref(op);
        if !FOLDED_BINOPS.contains(&name) {
            return ty;
        }
        let name = name.to_string();
        let Some(cx) = self.constant_tree(x.0, x.1) else { return ty };
        let Some(cy) = self.constant_tree(y.0, y.1) else { return ty };
        let folded = fold_binop(&name, cx, cy, |a, b| self.interner.intern(&format!("{}{}", self.interner.get(a), self.interner.get(b))));
        self.with_folded_type(te, ty, folded)
    }

    /// `ConstFold.Select` (47-52) of the operation `te` (`x.op`: a prefix operator `unary_op`, a conversion), of type
    /// `ty`: one of `foldedUnops` over a `ConstantTree` has the type of the constant `foldUnop` gives.
    pub(super) fn const_folded_select(&mut self, te: TExprId, ty: TypeId, op: &str, x: TExprId, x_ty: TypeId) -> TypeId {
        if ty == ERROR || matches!(self.types.get(ty), Type::Lit(_)) || !self.maybe_constant_tree(x, x_ty) || !FOLDED_UNOPS.contains(&op) {
            return ty;
        }
        let Some(FoldConst::Val(cx)) = self.constant_tree(x, x_ty) else { return ty };
        let folded = fold_unop(op, cx).map(FoldConst::Val);
        self.with_folded_type(te, ty, folded)
    }

    /// `withFoldedType` (73-75): the tree typed with the folded constant, where its kind is the operation's own type's
    /// (a `Float`'s value kept a `Float`, `folded_as`).
    fn with_folded_type(&mut self, te: TExprId, ty: TypeId, folded: Option<FoldConst>) -> TypeId {
        let Some(FoldConst::Val(v)) = folded else { return ty };
        let v = self.folded_as(v, ty);
        let lit = self.types.lit(v);
        let _ = te;
        if self.widen_lit(lit) == self.widen_lit(self.canon(ty)) {
            lit
        } else {
            ty
        }
    }

    /// Whether the tree `e` of type `ty` can be a `ConstantTree` at all, a cheap test before the reading: a literal, a
    /// `null`, an ascription, or a type that may stand for a constant (a literal type, a path, a member, an
    /// intersection or a union, a match).
    #[inline]
    fn maybe_constant_tree(&mut self, e: TExprId, ty: TypeId) -> bool {
        let ty = self.deref_keeping(ty);
        match self.types.get(ty) {
            Type::Lit(_) | Type::Term(_) | Type::Select(..) | Type::Member(..) | Type::Inter(..) | Type::Union(..) | Type::Match(..) | Type::Alias(..) => true,
            // A literal typed at a class (a hint's) is the class's operand alone; `null` is typed `Null` (read by its class,
            // which a retained form shares with its canonical type).
            Type::Class(c, _) if c == self.b.null => self.prog.is_widened(e) || matches!(self.prog.expr(e), TExpr::Null),
            _ => self.prog.is_widened(e),
        }
    }

    /// `ConstantTree` of an ascription's inner tree `inner` of its own type `ty` (`Typed(expr, _)`'s `expr`), kept by
    /// the ascription's node `node` (`Worker::ascription_constants`); out of `type_expr`'s line.
    #[inline(never)]
    pub(super) fn note_ascribed_tree_constant(&mut self, node: TExprId, inner: TExprId, ty: TypeId) {
        // The answer kept whatever it is: a negative one is no structural evaluation's to make later.
        let c = if self.maybe_constant_tree(inner, ty) || self.literal_tree(inner) { self.constant_tree(inner, ty) } else { None };
        self.ascription_constants.insert(node, AscribedLayer::Typed(c));
    }

    /// An adaptation's numeric conversion of the node `te` in place (`widened` the same node) over an ascription:
    /// dotty's conversion is a tree of its own around the `Typed`, read by its type (`ConstantTree`'s fallback), so it is
    /// the node's outer layer now (`AscribedLayer::Cast`).
    pub(super) fn note_conversion_layer(&mut self, te: TExprId, widened: TExprId) {
        if widened == te && self.prog.is_widened(te) {
            self.ascription_constants.insert(te, AscribedLayer::Cast(None));
        }
    }

    /// Whether `e` is a literal tree (a literal node, or a block ending in one, as `adaptConstant` makes): a `Literal`'s
    /// type is its constant in dotty, which the node's value is here.
    fn literal_tree(&self, e: TExprId) -> bool {
        match self.prog.expr(e) {
            TExpr::Int(_) | TExpr::Long(_) | TExpr::Double(_) | TExpr::Char(_) | TExpr::Bool(_) | TExpr::Str(_) | TExpr::Null => true,
            TExpr::Block(_, res) => self.literal_tree(res),
            _ => false,
        }
    }

    /// `ConstFold.ConstantTree` (62-71) of the tree `e`, of the type `ty` the typer gave it: `Inlined(_, Nil, expr)` an
    /// expansion's, which `ty` is; `Typed(expr, _)` the ascribed tree's own (an ascription keeps no node of its own:
    /// the node's constant, `ascribed_constant`); `Literal(null)` the null constant; a literal its value; any other tree
    /// its type's, `tpe.widenTermRefExpr.normalized.simplified` (`constant_of_simplified`): a call's, a block's, a
    /// cast's (its target's) alike.
    fn constant_tree(&mut self, e: TExprId, ty: TypeId) -> Option<FoldConst> {
        if self.prog.is_widened(e) || self.prog.is_ascribed(e) {
            match self.ascription_constants.get(&e) {
                // `Typed(expr, _)`: the inner tree as the ascription's typing read it, a negative answer as well.
                Some(&AscribedLayer::Typed(c)) => return c,
                // A cast over it (`TypeApply(Select(_, asInstanceOf), _)`): its own type, never the tree inside.
                Some(AscribedLayer::Cast(_)) => return self.constant_of_simplified(ty).map(FoldConst::Val),
                None => {}
            }
        }
        if self.prog.is_widened(e) {
            return self.ascribed_constant(e).map(FoldConst::Val);
        }
        // A cast that keeps no node (`ascribed_bits` without the ascription's `widened_bits`), or a plain inline
        // call's expansion: its tree's type, the cast's target, the call's declared result.
        if self.prog.is_ascribed(e) {
            return self.constant_of_simplified(ty).map(FoldConst::Val);
        }
        let literal = |t: &Self, v: LitVal| (t.dialect.interpreted_constants || !t.inline.evaluated.contains_key(&e)).then_some(FoldConst::Val(v));
        match self.prog.expr(e) {
            TExpr::Null => return Some(FoldConst::Null),
            TExpr::Int(v) => {
                let v = self.narrowed_literal(v, ty);
                return literal(self, v);
            }
            TExpr::Long(v) => return literal(self, LitVal::Long(v)),
            TExpr::Double(v) => {
                let v = self.folded_as(LitVal::Double(v.to_bits()), ty);
                return literal(self, v);
            }
            TExpr::Bool(v) => return literal(self, LitVal::Bool(v)),
            TExpr::Char(v) => return literal(self, LitVal::Char(v)),
            TExpr::Str(s) => {
                let v = LitVal::Str(self.interner.intern(&self.prog.strings[s.idx()]));
                return literal(self, v);
            }
            // A block's type is its result's: `adaptConstant`'s `Block(tree :: Nil, lit)` its literal.
            TExpr::Block(_, res) if self.literal_tree(res) => return self.constant_tree(res, ty),
            _ => {}
        }
        self.constant_of_simplified(ty).map(FoldConst::Val)
    }

    /// An integer literal's constant by its tag: one typed `Byte` or `Short` is the narrowed constant
    /// `Constant.convertTo` made (`adapt_constant_node`'s literal, a library body's `Typed(Literal(BYTEconst))`), with
    /// its `ByteTag` or `ShortTag`; else an `Int`.
    fn narrowed_literal(&mut self, v: i32, ty: TypeId) -> LitVal {
        let ty = self.deref_keeping(ty);
        let ty = self.canon_head(ty);
        if ty == self.b.t_byte {
            if let Ok(b) = i8::try_from(v) {
                return LitVal::Byte(b);
            }
        } else if ty == self.b.t_short {
            if let Ok(h) = i16::try_from(v) {
                return LitVal::Short(h);
            }
        }
        LitVal::Int(v)
    }

    /// The constant of an ascribed tree (`Typed(expr, _)`'s `expr`), whose own type the node no longer carries: a
    /// literal its value, a value or a call of a constant result (its declared result's, a call's type where it
    /// takes no type parameter), a block its result's, an operation its folding.
    fn ascribed_constant(&mut self, e: TExprId) -> Option<LitVal> {
        match self.prog.expr(e) {
            TExpr::CallStatic(s, _) | TExpr::CallMethod(_, s, _) => {
                let sig = self.sig_of(s);
                if !sig.tparams.is_empty() {
                    return None;
                }
                let ret = sig.ret;
                self.constant_of_simplified(ret)
            }
            TExpr::Block(_, res) => self.ascribed_constant(res),
            _ => self.fold_typed_by_pending_types(e),
        }
    }

    /// `ConstFold.apply`'s `classOf[T]` (`Constant(targ.tpe)`) and `ConstFold.Apply`'s primitive `getClass`: the class
    /// constant of the type `Class[T]` (dotty's `ClazzTag`), which widens to it.
    pub(super) fn class_constant_type(&mut self, class: TypeId) -> TypeId {
        if class == ERROR {
            return class;
        }
        self.types.lit(LitVal::Class(class))
    }

    /// `widenTermRefExpr.normalized`'s constant (`Checking.checkAnnotArgs`, `Annotations.argumentConstant`): a path's
    /// type widened to what it stands for (`widen_path`), normalized, a literal type; an annotated one none
    /// (`normalized` keeps the annotation).
    pub(super) fn constant_of_type(&mut self, ty: TypeId) -> Option<LitVal> {
        let ty = self.deref_keeping(ty);
        if matches!(self.types.get(ty), Type::Annotated(_)) {
            return None;
        }
        let ty = self.widen_path(ty);
        let ty = self.normalize(ty);
        let ty = self.deref_keeping(ty);
        match self.types.get(ty) {
            Type::Lit(l) => Some(self.types.lit_val(l)),
            _ => None,
        }
    }

    /// `ConstFold.ConstantTree`'s type test: `tpe.widenTermRefExpr.normalized.simplified` a constant type.
    pub(super) fn constant_of_simplified(&mut self, ty: TypeId) -> Option<LitVal> {
        let ty = self.deref_keeping(ty);
        if matches!(self.types.get(ty), Type::Annotated(_)) {
            return None;
        }
        let ty = self.widen_path(ty);
        // A class's type that nothing reduces (no type-level operation) is no constant whatever `simplified` does: read
        // no further (a path's widened `Int`, the common operand).
        let shape = self.types.get(ty);
        if matches!(shape, Type::Class(..)) && !self.types.head_reducible(shape) {
            return None;
        }
        let ty = self.normalize(ty);
        self.simplify_constant(ty)
    }

    /// `TypeOps.simplify` (`TypeOps.scala` 133-206) as it bears on a constant: a projection through a prefix reduced
    /// (`reduceProjection`: the member's alias), a match type or a match alias normalized (`tryNormalize`), `l & r`
    /// the glb of the two (`"x" & String` is `"x"`: a side's constant conforming to the other), `l | r` the lub (one
    /// constant), an inference variable its instance; an annotated type keeps its annotation (no constant).
    fn simplify_constant(&mut self, ty: TypeId) -> Option<LitVal> {
        let ty = self.deref_keeping(ty);
        match self.types.get(ty) {
            Type::Lit(l) => Some(self.types.lit_val(l)),
            Type::Member(prefix, name) => match self.type_member(prefix, name)? {
                super::members::MemberInfo::Alias(a) if a != ty => self.simplify_constant(a),
                _ => None,
            },
            Type::Alias(..) | Type::Match(..) => {
                let n = self.normalize(ty);
                if n != ty {
                    self.simplify_constant(n)
                } else {
                    None
                }
            }
            Type::Inter(a, b) => {
                for (x, y) in [(a, b), (b, a)] {
                    if let Some(v) = self.simplify_constant(x) {
                        let lit = self.types.lit(v);
                        if self.is_sub(lit, y) {
                            return Some(v);
                        }
                    }
                }
                None
            }
            Type::Union(a, b, _) => {
                let x = self.simplify_constant(a)?;
                (self.simplify_constant(b)? == x).then_some(x)
            }
            _ => None,
        }
    }

    fn prefix_on_typed(&mut self, op: Name, te: TExprId, ty: TypeId, span: Span) -> (TExprId, TypeId) {
        let ty = self.dealias(ty);
        let ty = match self.types.get(ty) {
            Type::Union(..) => self.primitive_side(ty).unwrap_or(ty),
            _ => ty,
        };
        let ty = self.widen_lit(ty);
        // A type parameter bounded by a primitive has the bound's prefix operators (`!x` of a
        // `T <: Boolean`), as scalac finds `unary_!` on the bound.
        let ty = self.primitive_bound(ty).unwrap_or(ty);
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
        if let (names::PLUS, Some(rank)) = (op, self.is_numeric(ty)) {
            if self.capturing() {
                self.capture_wrap(te, Wrap::Member(names::UNARY_PLUS, span));
            }
            let file = self.env.file;
            if let Some(ix) = self.index.as_mut() {
                ix.identity_prefixes.insert((file, span.start), ());
            }
            // `Byte`'s, `Short`'s and `Char`'s `unary_+` is an `Int` (`def unary_+ : Int`), promoted as `-` and `~` are.
            if matches!(rank, R_BYTE | R_SHORT | R_CHAR) {
                return (self.promote(te, ty, R_INT), t_int);
            }
            if self.denotation_marks.is_some() {
                self.mark_collapsed_call(te);
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
        // A union of numeric constants (`1 | 2`) is its join's (`primitive_side`).
        let recv_ty = match self.types.get(recv_ty) {
            Type::Union(..) => self.primitive_side(recv_ty)?,
            _ => recv_ty,
        };
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
        let conversion = NUMERIC_CONVERSIONS.contains(&text).then(|| text.to_string());
        let te = self.convert_rank(recv, recv_ty, to);
        if te == recv && self.denotation_marks.is_some() {
            self.mark_collapsed_call(te);
        }
        let ty = self.rank_type(to);
        // A conversion of a constant tree has the converted constant's type (dotty's
        // `ConstFold.Select`, `foldUnop`'s `toInt` and kin): `2.5f.toInt` is a `2`. A `Byte` or
        // `Short` one keeps its class, which teq has no constant type of.
        if let Some(op) = conversion {
            return Some((te, self.const_folded_select(te, ty, &op, recv, recv_ty)));
        }
        Some((te, ty))
    }

    /// `recv.asInstanceOf[t]` for a receiver of type `recv_ty`, lowered as dotty's erasure lowers
    /// a cast (`TypeTestsCasts.transformAsInstanceOf`, `cast_lowering`); a cast of a stored inline
    /// body or of a quote is kept as written, its types filled in where it is instantiated.
    pub(crate) fn lower_cast(&mut self, recv: TExprId, recv_ty: TypeId, t: TypeId, span: Span) -> (TExprId, TypeId) {
        self.lower_cast_written(recv, recv_ty, t, span, false)
    }

    /// `lower_cast` of a cast whose target, as written, names a JavaScript type (`js_written`).
    pub(super) fn lower_cast_written(&mut self, recv: TExprId, recv_ty: TypeId, t: TypeId, span: Span, js_written: bool) -> (TExprId, TypeId) {
        if self.quote.level > 0 || self.checks_inline_definition() {
            return self.cast_node(recv, CastOp::Written, t);
        }
        let lowering = self.cast_lowering(recv_ty, t, js_written);
        if lowering == CastLowering::Fail {
            // `tpd.primitiveConversion`'s warning, where scalac's erasure meets the cast: of the
            // reporting policy's phases, `Erasure`'s (`producer_rank`), and with no position in
            // scalac, so that no place hides it.
            let from = self.dealias(recv_ty);
            let (from, to) = (self.widen_lit(from), self.dealias(t));
            let msg = format!("conversion from {} to {} will always fail at runtime.", self.show(from), self.show(to));
            self.warn_as(span, msg, crate::source::Warning { unplaced: true, ..crate::source::Warning::of_phase(crate::source::PHASE_ERASURE) });
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
                TExpr::TypeTest(x, test, _) => {
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
        let tr = self.checked_operand(r, self.b.t_boolean);
        self.pop_tested(mark);
        tr
    }

    /// The right operand `r` of a builtin operator, typed with `hint`, or as the deeper selection
    /// typed it (`apply::SelectionTyped`), which a deeper round alone has done.
    #[inline(always)]
    fn typed_operand(&mut self, r: ExprId, hint: Option<TypeId>) -> (TExprId, TypeId) {
        if self.selection_typed.is_empty() {
            return self.type_expr(r, hint);
        }
        self.typed_operand_cached(r, hint)
    }

    #[cold]
    #[inline(never)]
    fn typed_operand_cached(&mut self, r: ExprId, hint: Option<TypeId>) -> (TExprId, TypeId) {
        match self.selection_typed_of(r).flatten() {
            Some(typed) => typed,
            None => self.type_expr(r, hint),
        }
    }

    /// The right operand `r` of a builtin operator checked against `expected`, the deeper
    /// selection's typing of it adapted (`typedArg` of a cached typing).
    #[inline(always)]
    fn checked_operand(&mut self, r: ExprId, expected: TypeId) -> TExprId {
        if self.selection_typed.is_empty() {
            return self.check_expr(r, expected);
        }
        self.checked_operand_cached(r, expected)
    }

    /// `checked_operand`, with the operand as typed before it was adapted, and its type (`ConstFold`'s `ConstantTree`
    /// reads them): the adapted tree, the typed one, its type.
    fn checked_operand_typed(&mut self, r: ExprId, expected: TypeId) -> (TExprId, TExprId, TypeId) {
        let cached = if self.selection_typed.is_empty() { None } else { self.selection_typed_of(r).flatten() };
        let (te, ty) = match cached {
            Some(typed) => typed,
            None => self.type_expr(r, Some(expected)),
        };
        let discards = cached.is_none() && self.canon(expected) == self.b.t_unit;
        if discards {
            self.warn_discarded(r, te, ty);
        }
        let span = self.cur_ast().expr_span(r);
        let adapted = self.adapt(te, ty, expected, span);
        if discards {
            self.mark_removed_discard(r, te, adapted);
        }
        (adapted, te, ty)
    }

    #[cold]
    #[inline(never)]
    fn checked_operand_cached(&mut self, r: ExprId, expected: TypeId) -> TExprId {
        match self.selection_typed_of(r).flatten() {
            Some((te, ty)) => {
                let span = self.cur_ast().expr_span(r);
                self.adapt(te, ty, expected, span)
            }
            None => self.check_expr(r, expected),
        }
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
                self.prog.mark_cast(recv);
                self.end_chain(recv, t);
                // The value is of the cast's erased target, as an ascription's: an upcast is one
                // (`TypeTestsCasts.transformAsInstanceOf`: "Replace cast by type ascription"), and a
                // cast a passed test leaves untested (`untested_lowering`) dotty's cast tree.
                self.prog.mark_ascribed(recv);
                // The cast is the node's outer layer now, an ascription's constant inside it no longer read; its
                // own type is kept where it may stand for a constant (no class's) or is an opaque type, which a
                // build that records no types has nowhere else (`Worker::cast_layer`).
                if self.prog.is_widened(recv) || !matches!(self.types.get(t), Type::Class(..)) || self.is_opaque_class_type(t) {
                    self.ascription_constants.insert(recv, AscribedLayer::Cast(Some(t)));
                } else if matches!(self.ascription_constants.get(&recv), Some(AscribedLayer::Cast(_))) {
                    // A cast under this one is no longer the outer layer.
                    self.ascription_constants.insert(recv, AscribedLayer::Cast(Some(t)));
                }
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
                    self.prog.mark_cast(unit);
                    unit
                } else {
                    self.mark_removed_made(recv);
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
                self.prog.mark_cast(zero);
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
    pub(super) fn cast_lowering(&mut self, from: TypeId, to: TypeId, js_written: bool) -> CastLowering {
        use super::site::Erased;
        if from == ERROR || to == ERROR {
            return CastLowering::Same;
        }
        // A cast to a JavaScript type is never tested, as under Scala.js (`JSCodeGen.genAsInstanceOf` of
        // the IR type `any`): a target written as the std's `js.FunctionN`, an alias of scala's function
        // type, is the value itself.
        if js_written {
            return CastLowering::Same;
        }
        let target = self.cast_erasure(to);
        let source = self.cast_erasure(from);
        let unit = Erased::Of(self.b.unit, 0);
        // An ascription to `Unit` is adapted to the primitive (`Erasure.Typer.typedTyped`,
        // `Boxing.adaptToType`): the value then `()`, `null` included.
        if self.dealias_canonical(to) == self.b.t_unit {
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
        // Outside the JVM a function is a JavaScript function whatever its arity, and Scala.js's
        // `js.FunctionN` are scala's function types here, so a cast from a function type to one of
        // another arity is the one Scala.js leaves untested, a cast to a JavaScript type: the value
        // itself (docs/COMPATIBILITY.md).
        if !self.jvm {
            if let (Erased::Of(f, 0), Erased::Of(t, 0)) = (source, target) {
                if self.is_function_class(f) && self.is_function_class(t) {
                    return CastLowering::Same;
                }
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
            Type::Term(_) | Type::Select(..) | Type::This(_) | Type::Refined(..) | Type::Rec(_) => match self.dependent_underlying(t) {
                Some(u) if u != t => self.found_primitive_at(u, depth + 1),
                _ => None,
            },
            _ => None,
        }
    }

    /// The erasure of a cast's type (`TypeErasure.erasure`, `bound_erasure`), `Object` where it has
    /// none.
    pub(super) fn cast_erasure(&mut self, t: TypeId) -> super::site::Erased {
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
    pub(super) fn erased_sub(&mut self, from: super::site::Erased, to: super::site::Erased) -> bool {
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
                    || (!self.is_primitive_class(a) && !self.is_primitive_class(b) && self.derives_from(a, b))
            }
            _ => false,
        }
    }

    /// Whether `c` is a function class past 22 parameters, whose erasure is `FunctionXXL`
    /// (`Definitions.functionTypeErasure`), one class for every such arity.
    fn function_xxl(&self, c: ClassId) -> bool {
        self.is_function_class(c) && self.syms.class(c).tparams.len() > 23
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
            t = self.applied_class(self.b.array, &[t]);
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
        let t = self.dealias_canonical(t);
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
        self.universal_member_over(recv, recv_ty, name, targs, lists, None, span)
    }

    /// `universal_member`, an enum's `valueOf` or `fromOrdinal` taking `param` where one is given: its argument
    /// typed already, which no list holds, the parameter of the reference without arguments
    /// (`enum_lookup_value`, the one caller that gives one).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn universal_member_over(
        &mut self,
        recv: TExprId,
        recv_ty: TypeId,
        name: Name,
        targs: Option<ListRef>,
        lists: &[ArgList],
        param: Option<TExprId>,
        span: Span,
    ) -> Option<(TExprId, TypeId)> {
        let no_args = lists.is_empty() || (lists.len() == 1 && lists[0].args.is_empty());
        let one_arg = lists.len() == 1 && lists[0].args.len() == 1;
        let recv_class = self.dealias(recv_ty);
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
            // `AnyRef.synchronized`: on JavaScript the one thread holds every monitor, so the body
            // runs as it is, after the receiver.
            names::SYNCHRONIZED if one_arg && !matches!(self.is_numeric(recv_class), Some(_)) => {
                let (body, ty) = self.type_expr(arg(0)?, None);
                // Under `teq interp` and on the JVM, whose threads hold monitors, the body runs holding
                // the receiver's (src/interp/threads.rs; dotty's `genSynchronized`, src/jvm/intrinsic.rs).
                if self.interp || self.jvm {
                    let s = self.prog.add_str("$synchronized($0, $1)");
                    let l = self.prog.list(&[recv, body]);
                    let te = self.prog.add(TExpr::Js(s, l));
                    self.end_chain(te, ty);
                    return Some((self.builtin_member(te, name), ty));
                }
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
            // The method, which a boxed number answers by its class and value and an object by
            // its own `equals`, called even on itself; `==` is cooperative (`PrimOp::Eq`). A
            // primitive receiver is boxed as its own type, a `Float` a `java.lang.Float` (dotty's
            // `Erasure.Boxing.box` of the receiver, then `Object.equals`).
            names::EQUALS if one_arg => {
                let (tr, _) = self.type_expr(arg(0)?, None);
                let s = self.prog.add_str("$equals($0, $1)");
                let l = self.prog.list(&[recv, tr]);
                let te = self.prog.add(TExpr::Js(s, l));
                Some((self.builtin_member(te, name), self.b.t_boolean))
            }
            names::EQ | names::NE if one_arg => {
                let (tr, _) = self.type_expr(arg(0)?, None);
                self.mark_braced_null_argument(arg(0)?, tr);
                let prim = match name {
                    names::EQ => PrimOp::Same,
                    _ => PrimOp::NotSame,
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
                    let js_written = self.written_js_alias(ids[0]);
                    let t = self.resolve_declared_type(ids[0]);
                    let cast = self.lower_cast_written(recv, recv_ty, t, span, js_written);
                    // The cast the source wrote, whatever its lowering made of it (`Program::written_cast_bits`); not a
                    // library body's typeCast nor the converter's own cast (`Ast::typer_cast_lists`).
                    let typer_cast = targs.is_some_and(|l| self.cur_ast().typer_cast_lists.binary_search(&l.start).is_ok());
                    if !typer_cast {
                        self.prog.mark_written_cast(cast.0);
                    }
                    self.note_written_this(cast.0, ids[0]);
                    if cast.0 == recv && self.denotation_marks.is_some() {
                        self.mark_collapsed_call(cast.0);
                    }
                    // The type argument is a type tree of the cast (`InlineDefinition::type_trees`).
                    if self.notes_type_trees() {
                        self.note_type_tree(cast.0, t);
                    }
                    return Some(cast);
                }
                let t = self.resolve_type(ids[0]);
                // `x.isInstanceOf[T @unchecked]` as a pattern's `case _: T @unchecked` is.
                let unchecked = self.expr_marks.get(&recv).map_or(false, |&m| m & super::MARK_UNCHECKED != 0)
                    || super::pat::type_unchecked(self.cur_ast(), ids[0]);
                let test = self.test_for(t, recv_ty, span, unchecked);
                self.mark_leaf_test(test, reads);
                if self.inline.checking > 0 {
                    self.note_leaf_test(test, t);
                }
                // The type the JVM's array test erases keeps a `*:` chain as it is spelled, as a cast's does
                // (`resolve_declared_type`: scalac erases such a chain to `Product`, `TypeErasure.erasePair`),
                // under `@unchecked` too. Its pickle and its type tree stay the resolved type's, as they were.
                let written = match self.cur_ast().ty(ids[0]) {
                    crate::ast::TyExpr::Unchecked(inner) => inner,
                    _ => ids[0],
                };
                let tested = match self.types.get(t) {
                    Type::Class(c, _) if c == self.b.array => self.spelled_cons(written, t).unwrap_or(t),
                    _ => t,
                };
                let te = self.prog.add(TExpr::TypeTest(recv, test, tested));
                if self.notes_type_trees() {
                    self.note_type_tree(te, t);
                }
                if self.capturing() {
                    self.capture_form(te, Form::Test(t));
                }
                Some((te, self.b.t_boolean))
            }
            // dotty's `DesugarEnums.enumLookupMethods`: `values` and `valueOf` where every case is
            // a value, `values` the clone of the companion's `$values`, stored after the cases
            // (`enumScaffolding`), `valueOf` the value case of the name; `fromOrdinal` of every
            // enum, `$values(ordinal)` where every case is a value, else the value case of the
            // ordinal. The arguments: the companion, the key, the enum's full name, then the value
            // cases with their names or ordinals; each target stores and reads `$values`.
            names::VALUES | names::VALUE_OF | names::FROM_ORDINAL => {
                // The companion through a path too (`p.F` of an instance's own companion).
                let Some(module) = self.class_of(recv_ty) else { return None };
                let info = self.syms.class(module);
                let enum_class = info.companion.filter(|_| self.is_enum_companion(module))?;
                if self.syms.class(enum_class).kind != ClassKind::Enum {
                    return None;
                }
                let cases = self.syms.class(enum_class).children.clone();
                let values: Vec<(SymId, u32, Name)> = cases
                    .iter()
                    .filter_map(|&c| {
                        let ci = self.syms.class(c);
                        ci.singleton.map(|s| (s, ci.ordinal, ci.name))
                    })
                    .collect();
                let enumeration = !cases.is_empty() && values.len() == cases.len();
                if cases.is_empty() || (name != names::FROM_ORDINAL && !enumeration) {
                    return None;
                }
                // The type arguments differ from case to case, which is `E[?]` in Scala.
                let arity = self.class_arity(enum_class);
                let enum_ty = self.applied_class(enum_class, &vec![WILD; arity]);
                // The enum through the companion's path (`a.E` of `a.E.fromOrdinal`), an enum
                // nested in a class being one type per enclosing instance.
                let enum_ty = self.as_seen_from(enum_ty, recv_ty, module);
                let full = self.enum_full_name(enum_class);
                let full = self.prog.add_str(&full);
                let full = self.prog.add(TExpr::Str(full));
                // A companion that is an instance (a local object's, an object's nested in a class)
                // is held, evaluated once before the argument, and its value cases are its fields,
                // read from it: what its `$values` holds, which the lookups take as they are.
                let instance = self.syms.class(module).local_module.is_some() || self.syms.class(module).inner_object.is_some();
                let held = instance.then(|| self.fresh_local("enum", recv_ty, Span::default()));
                let receiver = recv;
                let recv = match held {
                    Some(h) => {
                        let r = self.prog.add(TExpr::Local(h));
                        let companion = self.types.class(module, &[]);
                        self.prog.set_type(r, companion);
                        r
                    }
                    // The companion, after a receiver that is no path to it.
                    None => {
                        let module_ref = self.prog.add(TExpr::Module(module));
                        if self.is_path(recv) {
                            module_ref
                        } else {
                            let stmts = self.prog.stmts.push_slice(&[TStmt::Expr(recv)]);
                            self.prog.add(TExpr::Block(stmts, module_ref))
                        }
                    }
                };
                let mut refs: Vec<TExprId> = Vec::with_capacity(values.len());
                for &(v, _, _) in &values {
                    refs.push(match held {
                        Some(h) => {
                            let r = self.prog.add(TExpr::Local(h));
                            self.prog.add(TExpr::Field(r, v))
                        }
                        None => self.prog.add(TExpr::Static(v)),
                    });
                }
                if name == names::VALUES {
                    if !no_args {
                        return None;
                    }
                    let array = self.applied_class(self.b.array, &[enum_ty]);
                    let te = match held {
                        // A new array of the companion's value cases, as `$values.clone()` is.
                        Some(_) => {
                            let items = self.prog.list(&refs);
                            let all = self.prog.add(TExpr::ArrayLit(items));
                            self.prog.set_type(all, array);
                            all
                        }
                        None => {
                            let s = self.prog.add_str("$enumValues($0)");
                            let l = self.prog.list(&[recv]);
                            self.prog.add(TExpr::Js(s, l))
                        }
                    };
                    if self.capturing() {
                        self.capture_form(te, Form::EnumMember(name));
                    }
                    return Some((self.enum_lookup_held(held, receiver, te, array), array));
                }
                let key = match param {
                    Some(key) => key,
                    None => {
                        if !one_arg {
                            return None;
                        }
                        let expected = if name == names::VALUE_OF { self.b.t_string } else { self.b.t_int };
                        let (key, _) = self.type_expr(arg(0)?, Some(expected));
                        key
                    }
                };
                let refs = self.prog.list(&refs);
                let refs = self.prog.add(TExpr::ArrayLit(refs));
                let (template, keys): (&str, Vec<TExprId>) = if name == names::VALUE_OF {
                    let names: Vec<TExprId> = values
                        .iter()
                        .map(|&(_, _, n)| {
                            let n = self.prog.add_str(self.interner.get(n));
                            self.prog.add(TExpr::Str(n))
                        })
                        .collect();
                    ("$enumValueOf($0, $1, $2, $3, $4)", names)
                } else if enumeration && held.is_none() {
                    ("$enumFromOrdinal($0, $1, $2)", Vec::new())
                } else {
                    let ordinals: Vec<TExprId> = values.iter().map(|&(_, o, _)| self.prog.add(TExpr::Int(o as i32))).collect();
                    ("$enumCaseOfOrdinal($0, $1, $2, $3, $4)", ordinals)
                };
                let args = if template.starts_with("$enumFromOrdinal") {
                    self.prog.list(&[recv, key, full])
                } else {
                    let keys = self.prog.list(&keys);
                    let keys = self.prog.add(TExpr::ArrayLit(keys));
                    self.prog.list(&[recv, key, full, refs, keys])
                };
                let s = self.prog.add_str(template);
                let te = self.prog.add(TExpr::Js(s, args));
                if self.capturing() {
                    self.capture_form(te, Form::EnumMember(name));
                }
                let te = self.enum_lookup_held(held, receiver, te, enum_ty);
                Some((te, enum_ty))
            }
            names::GET_CLASS if no_args => {
                // A receiver whose static type is a primitive has the primitive's class, as scalac
                // rewrites it (`1.getClass` is `classOf[Int]`, `int` printed); it runs for its effects.
                // A constant's is its class's (`classOf[Boolean]` for `true`).
                let class = self.widen_lit(self.canon_head(recv_ty));
                let primitive = self.is_numeric(class).is_some() || class == self.b.t_boolean || class == self.b.t_unit;
                if primitive {
                    // `ConstFold.Apply`'s `getClass` of a primitive value: the class constant (`Constant(qual.tpe.widen)`).
                    let (of, ty) = self.type_class_of(class, span);
                    let ty = self.class_constant_type(ty);
                    self.mark_removed_made(recv);
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
                let ty = self.applied_class(class, &[arg]);
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

    /// A call of `Object.wait`, `notify` or `notifyAll` (`object_members`), typed as any member's
    /// is: the std's `java.lang.objectWait0` and its like on the receiver
    /// (std/javalib/concurrent.scala), the JDK's methods on the JVM, the receiver evaluated before
    /// the arguments. Where the std has no such def (`--std=scala-library`), the member's call.
    #[cold]
    #[inline(never)]
    pub(super) fn object_monitor_call(&mut self, call: &super::apply::MethodCall, args: ListRef) -> TExprId {
        let Some(recv) = call.recv else { return self.prog.add(TExpr::CallStatic(call.sym, args)) };
        let Some(def) = self.object_monitor_def(call.sym) else { return self.prog.add(TExpr::CallMethod(recv, call.sym, args)) };
        let items: Vec<TExprId> = std::iter::once(recv).chain(self.prog.expr_list(args).iter().copied()).collect();
        let l = self.prog.list(&items);
        // Built as any call of the def is, so that its target's template stands for it.
        let call = super::apply::MethodCall { recv: None, sym: def, owner_subst: Vec::new(), ext_recv: None, prefix: None, refined: None };
        self.build_call(&call, l)
    }

    /// The std's def that a call of `Object`'s member `sym` is lowered to.
    fn object_monitor_def(&mut self, sym: SymId) -> Option<SymId> {
        let arity = self.syms.sym(sym).sig.as_ref().map_or(0, |sig| sig.clauses.first().map_or(0, |c| c.params.len()));
        let def = match (self.syms.sym(sym).name, arity) {
            (names::WAIT, 0) => "objectWait0",
            (names::WAIT, 1) => "objectWait1",
            (names::WAIT, _) => "objectWait",
            (names::NOTIFY, _) => "objectNotify",
            _ => "objectNotifyAll",
        };
        let p = self.java_lang_pkg()?;
        let def = self.interner.intern(def);
        match self.pkg_term(p, def) {
            Some(super::resolve::TermRef::Global(def)) => Some(def),
            _ => None,
        }
    }

    pub fn is_const_expr(&self, e: ExprId) -> bool {
        matches!(
            self.cur_ast().expr(e),
            Expr::IntLit(_) | Expr::Number(_) | Expr::LongLit(_) | Expr::DoubleLit(_) | Expr::DecimalLit(_) | Expr::FloatLit(_) | Expr::BoolLit(_) | Expr::CharLit(_) | Expr::StringLit(_)
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

/// The outer layer of a node an ascription or a cast keeps (`Worker::ascription_constants`).
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum AscribedLayer {
    /// `Typed(expr, _)`: `expr`'s constant, or none (a negative answer kept).
    Typed(Option<FoldConst>),
    /// A cast, read by its own type: the type it casts to, where one is kept (a same-type cast's
    /// that may stand for a constant); none for a numeric conversion made in place.
    Cast(Option<TypeId>),
}

/// A `ConstantTree`'s constant (`Constant`): a value, or `null` (`NullTag`), which only `==` and `!=` fold.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum FoldConst {
    Val(LitVal),
    Null,
}

/// `ConstFold.foldedBinops` and `foldedUnops`.
const FOLDED_BINOPS: [&str; 19] = ["||", "|", "^", "&&", "&", "==", "!=", "<", ">", "<=", ">=", "<<", ">>>", ">>", "+", "-", "*", "/", "%"];
const FOLDED_UNOPS: [&str; 9] = ["unary_!", "unary_~", "unary_+", "unary_-", "toChar", "toInt", "toFloat", "toLong", "toDouble"];

/// dotty's `Constants` tags, which order the numeric ones (`ByteTag` to `DoubleTag`).
const BOOLEAN_TAG: u8 = 2;
const BYTE_TAG: u8 = 3;
const SHORT_TAG: u8 = 4;
const CHAR_TAG: u8 = 5;
const INT_TAG: u8 = 6;
const LONG_TAG: u8 = 7;
const FLOAT_TAG: u8 = 8;
const DOUBLE_TAG: u8 = 9;
const STRING_TAG: u8 = 10;
const NULL_TAG: u8 = 11;
const CLAZZ_TAG: u8 = 12;

/// A constant's tag (dotty's `Constant.tag`).
fn const_tag(v: FoldConst) -> u8 {
    match v {
        FoldConst::Null => NULL_TAG,
        FoldConst::Val(LitVal::Bool(_)) => BOOLEAN_TAG,
        FoldConst::Val(LitVal::Byte(_)) => BYTE_TAG,
        FoldConst::Val(LitVal::Short(_)) => SHORT_TAG,
        FoldConst::Val(LitVal::Char(_)) => CHAR_TAG,
        FoldConst::Val(LitVal::Int(_)) => INT_TAG,
        FoldConst::Val(LitVal::Long(_)) => LONG_TAG,
        FoldConst::Val(LitVal::Float(_)) => FLOAT_TAG,
        FoldConst::Val(LitVal::Double(_)) => DOUBLE_TAG,
        FoldConst::Val(LitVal::Str(_)) => STRING_TAG,
        FoldConst::Val(LitVal::Class(_)) => CLAZZ_TAG,
    }
}

/// dotty's `Constant.isNumeric`: a tag from `ByteTag` to `DoubleTag`.
fn const_numeric(v: FoldConst) -> bool {
    (BYTE_TAG..=DOUBLE_TAG).contains(&const_tag(v))
}

/// `Constant.intValue`: per its tag, a `Long` narrowed, a `Float` or a `Double` converted as the JVM converts it
/// (saturating, `NaN` to 0), never through a `Long`.
fn const_int(v: LitVal) -> i32 {
    match v {
        LitVal::Byte(b) => b as i32,
        LitVal::Short(h) => h as i32,
        LitVal::Char(c) => c as i32,
        LitVal::Int(i) => i,
        LitVal::Long(l) => l as i32,
        LitVal::Float(f) => f32::from_bits(f) as i32,
        LitVal::Double(d) => f64::from_bits(d) as i32,
        _ => 0,
    }
}

/// `Constant.longValue`: per its tag, a `Float` or a `Double` converted as the JVM converts it.
fn const_long(v: LitVal) -> i64 {
    match v {
        LitVal::Byte(b) => b as i64,
        LitVal::Short(h) => h as i64,
        LitVal::Char(c) => c as i64,
        LitVal::Int(i) => i as i64,
        LitVal::Long(l) => l,
        LitVal::Float(f) => f32::from_bits(f) as i64,
        LitVal::Double(d) => f64::from_bits(d) as i64,
        _ => 0,
    }
}

fn const_double(v: LitVal) -> f64 {
    match v {
        LitVal::Float(f) => f32::from_bits(f) as f64,
        LitVal::Double(d) => f64::from_bits(d),
        v => const_long(v) as f64,
    }
}

fn const_float(v: LitVal) -> f32 {
    match v {
        LitVal::Float(f) => f32::from_bits(f),
        LitVal::Double(d) => f64::from_bits(d) as f32,
        v => const_long(v) as f32,
    }
}

/// `ConstFold.foldUnop` (77-101).
fn fold_unop(op: &str, x: LitVal) -> Option<LitVal> {
    use LitVal::*;
    let numeric = const_numeric(FoldConst::Val(x));
    Some(match (op, x) {
        ("unary_!", Bool(b)) => Bool(!b),
        ("unary_~", Int(i)) => Int(!i),
        ("unary_~", Long(l)) => Long(!l),
        ("unary_+", Int(_) | Long(_) | Float(_) | Double(_)) => x,
        ("unary_-", Int(i)) => Int(i.wrapping_neg()),
        ("unary_-", Long(l)) => Long(l.wrapping_neg()),
        ("unary_-", Float(f)) => Float((-f32::from_bits(f)).to_bits()),
        ("unary_-", Double(d)) => Double((-f64::from_bits(d)).to_bits()),
        // `Constant.charValue` is `intValue.toChar`.
        ("toChar", v) if numeric => Char(const_int(v) as u16),
        ("toInt", v) if numeric => Int(const_int(v)),
        ("toLong", v) if numeric => Long(const_long(v)),
        ("toFloat", v) if numeric => Float(const_float(v).to_bits()),
        ("toDouble", v) if numeric => Double(const_double(v).to_bits()),
        _ => return None,
    })
}

/// `ConstFold.foldBinop` (190-211): the operation's tag the operands' common one, the larger of two numeric ones,
/// `NullTag` where either is `null`, else none (`"a" + 1` folds not); a division by zero none (`ArithmeticException`).
/// `concat` makes the string of two.
fn fold_binop(op: &str, x: FoldConst, y: FoldConst, concat: impl FnOnce(Name, Name) -> Name) -> Option<FoldConst> {
    use LitVal::*;
    let (tx, ty) = (const_tag(x), const_tag(y));
    let tag = if tx == ty {
        tx
    } else if const_numeric(x) && const_numeric(y) {
        tx.max(ty)
    } else if tx == NULL_TAG || ty == NULL_TAG {
        NULL_TAG
    } else {
        return None;
    };
    // `foldNullOp`: `==` and `!=` by the tags.
    if tag == NULL_TAG {
        return match op {
            "==" => Some(FoldConst::Val(Bool(tx == ty))),
            "!=" => Some(FoldConst::Val(Bool(tx != ty))),
            _ => None,
        };
    }
    let (FoldConst::Val(x), FoldConst::Val(y)) = (x, y) else { return None };
    Some(FoldConst::Val(match tag {
        BOOLEAN_TAG => {
            let (Bool(a), Bool(b)) = (x, y) else { return None };
            match op {
                "||" | "|" => Bool(a | b),
                "^" => Bool(a ^ b),
                "&&" | "&" => Bool(a & b),
                "==" => Bool(a == b),
                "!=" => Bool(a != b),
                _ => return None,
            }
        }
        // dotty's `ByteTag | ShortTag | CharTag | IntTag`.
        BYTE_TAG..=INT_TAG => {
            let (a, b) = (const_long(x) as i32, const_long(y) as i32);
            match op {
                "|" => Int(a | b),
                "^" => Int(a ^ b),
                "&" => Int(a & b),
                "<<" => Int(a.wrapping_shl(b as u32)),
                ">>>" => Int(((a as u32).wrapping_shr(b as u32)) as i32),
                ">>" => Int(a.wrapping_shr(b as u32)),
                "==" => Bool(a == b),
                "!=" => Bool(a != b),
                "<" => Bool(a < b),
                ">" => Bool(a > b),
                "<=" => Bool(a <= b),
                ">=" => Bool(a >= b),
                "+" => Int(a.wrapping_add(b)),
                "-" => Int(a.wrapping_sub(b)),
                "*" => Int(a.wrapping_mul(b)),
                "/" if b != 0 => Int(a.wrapping_div(b)),
                "%" if b != 0 => Int(a.wrapping_rem(b)),
                _ => return None,
            }
        }
        LONG_TAG => {
            let (a, b) = (const_long(x), const_long(y));
            let narrow = tx <= INT_TAG;
            match op {
                "|" => Long(a | b),
                "^" => Long(a ^ b),
                "&" => Long(a & b),
                "<<" if narrow => Int((a as i32).wrapping_shl(b as u32)),
                ">>>" if narrow => Int(((a as i32 as u32).wrapping_shr(b as u32)) as i32),
                ">>" if narrow => Int((a as i32).wrapping_shr(b as u32)),
                "<<" => Long(a.wrapping_shl(b as u32)),
                ">>>" => Long(((a as u64).wrapping_shr(b as u32)) as i64),
                ">>" => Long(a.wrapping_shr(b as u32)),
                "==" => Bool(a == b),
                "!=" => Bool(a != b),
                "<" => Bool(a < b),
                ">" => Bool(a > b),
                "<=" => Bool(a <= b),
                ">=" => Bool(a >= b),
                "+" => Long(a.wrapping_add(b)),
                "-" => Long(a.wrapping_sub(b)),
                "*" => Long(a.wrapping_mul(b)),
                "/" if b != 0 => Long(a.wrapping_div(b)),
                "%" if b != 0 => Long(a.wrapping_rem(b)),
                _ => return None,
            }
        }
        FLOAT_TAG => {
            let (a, b) = (const_float(x), const_float(y));
            match op {
                "==" => Bool(a == b),
                "!=" => Bool(a != b),
                "<" => Bool(a < b),
                ">" => Bool(a > b),
                "<=" => Bool(a <= b),
                ">=" => Bool(a >= b),
                "+" => Float((a + b).to_bits()),
                "-" => Float((a - b).to_bits()),
                "*" => Float((a * b).to_bits()),
                "/" => Float((a / b).to_bits()),
                "%" => Float((a % b).to_bits()),
                _ => return None,
            }
        }
        DOUBLE_TAG => {
            let (a, b) = (const_double(x), const_double(y));
            match op {
                "==" => Bool(a == b),
                "!=" => Bool(a != b),
                "<" => Bool(a < b),
                ">" => Bool(a > b),
                "<=" => Bool(a <= b),
                ">=" => Bool(a >= b),
                "+" => Double((a + b).to_bits()),
                "-" => Double((a - b).to_bits()),
                "*" => Double((a * b).to_bits()),
                "/" => Double((a / b).to_bits()),
                "%" => Double((a % b).to_bits()),
                _ => return None,
            }
        }
        STRING_TAG => {
            let (Str(a), Str(b)) = (x, y) else { return None };
            match op {
                "+" => Str(concat(a, b)),
                "==" => Bool(a == b),
                "!=" => Bool(a != b),
                _ => return None,
            }
        }
        _ => return None,
    }))
}

#[cfg(test)]
mod op_memo_tests {
    /// What every scenario's program starts with: an extension of `+=` for `Int` to be a given's, an object's to
    /// import, and a class with the operator to convert to.
    const PRELUDE: &str = "import scala.language.implicitConversions\ntrait Ops:\n  extension (n: Int) def +=(k: Int): Unit\nobject Provider:\n  extension (n: Int) def +=(k: Int): Unit = ()\nclass Acc(val n: Int):\n  def +=(k: Int): Unit = ()\nobject First:\n  def f(): Int = { var x = 1; x += 1; x }\n";

    /// How many assignment operators of the scenario took the assignment form by a kept answer (`Worker::op_memo`).
    /// The program must type without an error; under `cfg(test)` every use of a kept answer runs the first try and
    /// panics where it does not fail with nothing met (`Worker::check_op_memo`), so a count returned is a count of
    /// answers that held.
    fn hits(name: &str, body: &str) -> u32 {
        let text = format!("{}{}", PRELUDE, body);
        crate::with_typed_program(name, &text, &[], |w| {
            let errors: Vec<String> = w.diags.items.iter().filter(|d| !d.is_warning).map(|d| d.msg.clone()).collect();
            assert!(errors.is_empty(), "{}: {:?}", name, errors);
            w.op_memo_hits
        })
    }

    /// The differential test of the memo of an assignment operator's first try: after a first site of the type in
    /// the file (`First.f`, which keeps the answer for `Int` and `+=`), each thing that can provide the operator
    /// is brought in the ways a site can come to see it, and the sites that see it take no kept answer (their
    /// first try applies: the program types), while the sites before and after it do.
    #[test]
    fn a_kept_failure_is_used_only_where_nothing_can_provide_the_operator() {
        // Nothing provides it: every later site of the type takes the kept answer; another type keeps its own.
        assert_eq!(hits("opmemo-plain", "object A:\n  def g(): Int = { var x = 1; x += 1; x += 2; x }\n  def s(): String = { var t = \"a\"; t += 1; t += 2; t }\n  class C { var n = 0; def bump(): Unit = { n += 1; this.n += 1 } }\n"), 5);
        // An operation in the right side of another, each by a kept answer: the outer one's first try (run by the
        // check) types the inner one, whose own flag must not be the outer's. The inner one is counted twice: typed
        // by the outer's check and by the typing that is kept.
        assert_eq!(hits("opmemo-nested", "object A:\n  def g(): Int = { var x = 1; var y = 1; x += 1; x += { y += 1; y }; x }\n  def h(): Int = { var t = 0; t += List(1, 2).map(k => { var d = k; d += 1; d }).sum; t }\n"), 7);
        // A given's extension in a nested frame: the sites before and after the frame, not the one inside.
        assert_eq!(hits("opmemo-given-frame", "object A:\n  def g(): Int =\n    var x = 1\n    x += 1\n    {\n      given Ops with\n        extension (n: Int) def +=(k: Int): Unit = ()\n      x += 1\n    }\n    x += 1\n    x\n"), 2);
        // A using parameter of the method is a given of its frame.
        assert_eq!(hits("opmemo-using-parameter", "object A:\n  def g(using o: Ops): Int = { var x = 1; x += 1; x }\n  def h(): Int = { var x = 1; x += 1; x }\n"), 1);
        // An enclosing class with a given; an enclosing class whose member is the extension (a lexical one).
        assert_eq!(hits("opmemo-class-given", "class H:\n  given Ops with\n    extension (n: Int) def +=(k: Int): Unit = ()\n  def run(): Int = { var x = 1; x += 1; x }\nclass P:\n  def run(): Int = { var x = 1; x += 1; x }\n"), 1);
        assert_eq!(hits("opmemo-class-extension", "class H:\n  extension (n: Int) def +=(k: Int): Unit = ()\n  def run(): Int = { var x = 1; x += 1; x }\nclass P:\n  def run(): Int = { var x = 1; x += 1; x }\n"), 1);
        // A local import, a local conversion, an implicit class of a block.
        assert_eq!(hits("opmemo-local-import", "object A:\n  def g(): Int =\n    var x = 1\n    x += 1\n    {\n      import Provider.*\n      x += 1\n    }\n    x += 1\n    x\n"), 2);
        assert_eq!(hits("opmemo-local-conversion", "object A:\n  def g(): Int =\n    var x = 1\n    {\n      given Conversion[Int, Acc] = n => new Acc(n)\n      x += 1\n    }\n    x += 1\n    x\n"), 1);
        assert_eq!(hits("opmemo-implicit-class", "object A:\n  def g(): Int =\n    var x = 1\n    {\n      implicit class More(val n: Int) { def +=(k: Int): Unit = () }\n      x += 1\n    }\n    x += 1\n    x\n"), 1);
        // The file's import written after the first sites: the sites before it take the kept answer, the sites
        // after it see the import and call the extension.
        assert_eq!(hits("opmemo-file-import", "object Before:\n  def g(): Int = { var x = 1; x += 1; x }\nimport Provider.*\nobject After:\n  def g(): Int = { var x = 1; x += 1; x }\n"), 1);
        // A package-level given and conversion of the file, written after the first site: nothing is kept at all
        // (the first try applies everywhere), and nothing is taken.
        assert_eq!(hits("opmemo-package-given", "given Ops with\n  extension (n: Int) def +=(k: Int): Unit = ()\nobject A:\n  def g(): Int = { var x = 1; x += 1; x }\n"), 0);
        // A class type whose companion has the extension: the first try applies, nothing is kept for it; a var of
        // an alias of Int is not a kept type's.
        assert_eq!(hits("opmemo-companion", "class E(val n: Int)\nobject E:\n  extension (e: E) def +=(i: Int): Unit = ()\nobject A:\n  type I = Int\n  def g(): Unit = { var e = new E(0); e += 1; e += 2 }\n  def h(): Int = { var i: I = 1; i += 1; i }\n"), 1);
    }
}

#[cfg(test)]
mod written_mark_tests {
    /// The written casts and the creations of a companion's apply marked in a typed program
    /// (`Program::written_cast_bits`, `Program::apply_creation_bits`), counted over its expressions: the casts the source
    /// writes, whatever their lowering, and the case class creations written as the companion's apply, never `new`.
    fn marks(name: &str, text: &str) -> (usize, usize) {
        crate::with_typed_program(name, text, &[], |w| {
            let errors: Vec<String> = w.diags.items.iter().filter(|d| !d.is_warning).map(|d| d.msg.clone()).collect();
            assert!(errors.is_empty(), "{}: {:?}", name, errors);
            // Every typed expression: the test's program types no standard library body, and its builds record no spans.
            let ids: Vec<crate::tir::TExprId> = (0..w.prog.exprs.len() as u32).map(crate::tir::TExprId).collect();
            let written = ids.iter().filter(|&&e| w.prog.is_written_cast(e)).count();
            let created = ids.iter().filter(|&&e| w.prog.is_apply_creation(e)).count();
            (written, created)
        })
    }

    #[test]
    fn the_source_s_casts_and_companion_applies_are_marked_and_nothing_else() {
        // Two casts written, one an upcast its erasure makes the operand itself, one a downcast that keeps a node.
        assert_eq!(marks("written-two", "object A:\n  def f(x: Any): Int = x.asInstanceOf[String].length\n  def g(s: String): Any = s.asInstanceOf[Any]\n"), (2, 0));
        // No cast written: a pattern's test and the typer's own reading of its binder are not marked.
        assert_eq!(marks("written-none", "object A:\n  def f(x: Any): Int = x match\n    case s: String => s.length\n    case _ => 0\n"), (0, 0));
        // A case class made through its companion's apply is marked; one made with `new` is not. The case class's own
        // members write no cast: `fromProduct`'s element is dotty's `ensureConforms`, an Any_typeCast.
        assert_eq!(marks("created-apply", "case class CI(i: Int)\nobject A:\n  def f(): CI = CI(1)\n"), (0, 1));
        assert_eq!(marks("created-new", "case class CI(i: Int)\nobject A:\n  def f(): CI = new CI(1)\n"), (0, 0));
        // The synthesized TypeTest writes `s.asInstanceOf[s.type & T]`: marked. A product's mirror is the companion
        // cast to the mirror's type, dotty's `mirrorRef.cast`: unmarked.
        assert_eq!(marks("type-test", "import scala.reflect.TypeTest\nobject A:\n  def f(x: Any): Option[String] = summon[TypeTest[Any, String]].unapply(x)\n"), (1, 0));
        assert_eq!(marks("mirror", "case class P(a: Int)\nobject A:\n  def m = summon[scala.deriving.Mirror.ProductOf[P]]\n"), (0, 0));
    }

    /// The body of the member `member` of the library class at `path`, read from scala-library's jar and typed as the
    /// reach pass types it (`deferred_body`: its class compiled from its TASTy first): the written casts marked among the
    /// body's own expressions (outside the expansions of its inline calls), and the casts its file's conversion noted as
    /// no written one (`Ast::typer_cast_lists`).
    fn library_body_marks(path: &[&str], member: &str) -> (usize, usize) {
        crate::with_typed_program(&format!("library-body-{member}"), "object A:\n  val t = (1, 2)\n", &["--std=scala-library"], |w| {
            let c = w.class_at(path).expect("the library's class");
            w.complete_class(c);
            let name = w.interner.intern(member);
            let s = w.syms.class(c).members.get(&name).copied().expect("the member");
            let s = match w.syms.sym(s).kind {
                crate::symbols::SymKind::Overloaded(_) => w.syms.alternatives(s).and_then(|a| a.first().copied()).expect("an alternative"),
                _ => s,
            };
            let Some(super::super::check::DeferredBody::Fun(f)) = w.deferred_body(s) else { panic!("{member}'s body") };
            let body = w.prog.funs[f.idx()].body.expect("a body");
            // The body's own expressions: an inline call's expansion in it is the callee's body, whose casts are its own.
            let expanded: std::collections::HashSet<crate::tir::TExprId> =
                w.prog.descendants(body).filter(|e| w.prog.expansions.contains_key(e)).flat_map(|x| w.prog.descendants(x)).collect();
            let written = w.prog.descendants(body).filter(|e| !expanded.contains(e) && w.prog.is_written_cast(*e)).count();
            (written, w.ast(w.syms.sym(s).file).typer_cast_lists.len())
        })
    }

    #[test]
    fn a_library_body_s_type_cast_is_no_written_cast() {
        // IteratorStepperBase.estimateSize holds one `$asInstanceOf$` in its TASTy and no other cast of its own: dotty's Any_typeCast,
        // which `tpd.cast` makes. It is noted, and unmarked.
        let (written, noted) = library_body_marks(&["scala", "collection", "convert", "impl", "IteratorStepperBase"], "estimateSize");
        assert_eq!(written, 0, "IteratorStepperBase.estimateSize's typeCast marked as written");
        assert!(noted >= 1, "IteratorStepperBase.estimateSize's typeCast not noted: {noted}");
        // ScalaRunTime.anyValClass writes `classTag[T].runtimeClass.asInstanceOf[jl.Class[T]]`: Any_asInstanceOf, marked.
        let (written, _) = library_body_marks(&["scala", "runtime", "ScalaRunTime"], "anyValClass");
        assert_eq!(written, 1, "ScalaRunTime.anyValClass' written cast");
    }
}
