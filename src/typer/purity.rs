//! dotty's purity of trees (ast/TreeInfo.scala 566-702 at 3.8.4: statPurity, exprPurity, refPurity and the
//! predicates over them; inlines/Inliner.scala 39-104: isElideableExpr) over the typed IR: one family, which every
//! caller asks with the view of the reference it holds and the reading of a cast its construct takes.
use super::Worker;
use crate::ast::mods;
use crate::symbols::*;
use crate::tir::*;
use crate::types::*;

/// dotty's `TreeInfo.PurityLevel`: what an expression's evaluation can be told from.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct Purity(u8);

impl Purity {
    pub const IMPURE: Purity = Purity(0);
    pub const IDEMPOTENT: Purity = Purity(1);
    pub const PURE: Purity = Purity(3);
    pub const IDEMPOTENT_PATH: Purity = Purity(5);
    pub const PURE_PATH: Purity = Purity(7);

    /// dotty's `min`, the levels' bits in common.
    pub fn min(self, other: Purity) -> Purity {
        Purity(self.0 & other.0)
    }

    /// dotty's `>=`: the level holds every bit of `that` (an idempotent path is not pure).
    pub fn at_least(self, that: Purity) -> bool {
        self.0 & that.0 == that.0
    }
}

/// How the source wrote a reference, which the caller supplies (tpd.ref's elided prefix): an `Ident`, judged by its
/// symbol alone, or a `Select` on a qualifier, judged by the symbol and then by the qualifier.
// No reader yet: a constant's reference and an inline argument's binding are the first.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub(crate) enum View {
    Ident,
    Select(TExprId),
}

/// A reference to `sym` written in `view`.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub(crate) struct RefView {
    pub sym: SymId,
    pub view: View,
}

impl<'a> Worker<'a> {
    /// dotty's `TreeInfo.exprPurity` over the typed IR; a reference as it was written where the
    /// IR's node says otherwise (`Worker::note_written_ref`).
    #[inline]
    pub(super) fn expr_purity(&mut self, e: TExprId) -> Purity {
        self.expr_purity_under(e, Purity::PURE)
    }

    /// `expr_purity` with the purity of a cast the source writes given by the asker. `exprPurity`'s `TypeApply`
    /// arm calls the compiler's own cast (`Any_typeCast`) pure and a written `asInstanceOf` impure; the typed IR
    /// keeps a written cast as its node, or, where its erasure makes it redundant, as its operand marked
    /// (`lower_cast_as`). `Purity::PURE` is what `expr_purity`'s readers ask: the cast's node pure over its
    /// operand, the marked operand what it is without the mark. `Purity::IMPURE` is the reference's, which the
    /// lift of a right-associative operator's arguments asks (`Worker::lift_impure`). The erasure's unboxing
    /// (`CastOp::Adapt`) is pure for either. Any other cast in the IR is read as a written one, the ones the
    /// typer makes itself too (a mirror's, a tuple element's): the IR does not tell them apart.
    pub(super) fn expr_purity_under(&mut self, e: TExprId, cast: Purity) -> Purity {
        if cast != Purity::PURE && self.prog.is_cast(e) {
            return cast;
        }
        if let Some(&p) = self.written_purity.get(&e) {
            return p;
        }
        match self.prog.expr(e) {
            TExpr::Int(_) | TExpr::Long(_) | TExpr::Double(_) | TExpr::Bool(_) | TExpr::Char(_) | TExpr::Str(_) | TExpr::Unit | TExpr::Null => Purity::PURE_PATH,
            TExpr::This | TExpr::Super(_) => Purity::PURE_PATH,
            // `classOf[T]`, `Predef.classOf`.
            TExpr::ClassOf(..) => Purity::PURE,
            TExpr::Local(s) => self.symbol_purity(s),
            // A member of an object: its selection on the object.
            TExpr::Static(s) => {
                let owner = match self.syms.sym(s).owner {
                    Owner::Class(o) => self.module_purity(o),
                    _ => Purity::PURE_PATH,
                };
                self.symbol_purity(s).min(owner)
            }
            TExpr::Module(c) => self.module_purity(c),
            TExpr::Field(r, s) => self.symbol_purity(s).min(self.expr_purity_under(r, cast)),
            TExpr::Lambda(..) => Purity::PURE,
            // A constructor of a class without initialisation is stable, as a case class's
            // synthetic `apply` is (`isPureApply`, `isPureSyntheticCaseApply`).
            TExpr::New(c, args) if self.no_inits(c) => {
                let args = self.prog.expr_list(args).to_vec();
                let own = self.creation_purity_under(c, cast);
                args.into_iter().fold(own, |p, a| p.min(self.expr_purity_under(a, cast)))
            }
            // `minOf(exprPurity(expr), stats.map(statPurity))`.
            TExpr::Block(stmts, res) => {
                let mut p = self.expr_purity_under(res, cast);
                for s in self.prog.stmts[stmts.range()].to_vec() {
                    p = p.min(self.stat_purity_under(s, cast));
                }
                p
            }
            // `e.asInstanceOf[T]`: the asker's purity of a written cast; `Any_typeCast`, the erasure's own.
            TExpr::Cast(r, CastOp::Adapt(..), _) => Purity::PURE.min(self.expr_purity_under(r, cast)),
            TExpr::Cast(r, ..) => cast.min(Purity::PURE).min(self.expr_purity_under(r, cast)),
            // A primitive or string operation the typer folds to a constant, over operands as pure
            // (`isPureApply`: an application of a known pure operation whose type is a constant).
            TExpr::Unary(_, a) | TExpr::ToStr(a, _) if self.fold_constant_as_typed(e).is_some() => Purity::PURE.min(self.expr_purity_under(a, cast)),
            TExpr::Prim(_, a, b) if self.fold_constant_as_typed(e).is_some() => Purity::PURE.min(self.expr_purity_under(a, cast)).min(self.expr_purity_under(b, cast)),
            TExpr::StrConcat(l) if self.fold_constant_as_typed(e).is_some() => {
                let parts = self.prog.expr_list(l).to_vec();
                parts.into_iter().fold(Purity::PURE, |p, x| p.min(self.expr_purity_under(x, cast)))
            }
            // A lazy val read through its accessor (`fn.symbol.isStableMember`): idempotent, its
            // first read running the initialiser.
            TExpr::CallStatic(s, l) if l.len == 0 && self.lazy_accessor(s) => Purity::IDEMPOTENT,
            TExpr::CallMethod(r, s, l) if l.len == 0 && self.lazy_accessor(s) => self.expr_purity_under(r, cast).min(Purity::IDEMPOTENT),
            // `isPureApply`: a constant expression of a known pure operation (a primitive value
            // class's or `String`'s method, one of `Any`'s `pureMethods`) over pure arguments, and a
            // case class's synthetic `apply` whose class has no initialisation
            // (`isPureSyntheticCaseApply`).
            TExpr::CallStatic(s, l) if self.is_pure_synthetic_case_apply(s) => {
                let args = self.prog.expr_list(l).to_vec();
                let own = match self.syms.sym(s).owner {
                    Owner::Class(o) if cast != Purity::PURE => self.module_purity(o).min(Purity::PURE),
                    _ => Purity::PURE,
                };
                args.into_iter().fold(own, |p, a| p.min(self.expr_purity_under(a, cast)))
            }
            TExpr::CallStatic(s, l) if self.is_known_pure_op(s) && self.fold_constant_as_typed(e).is_some() => {
                let args = self.prog.expr_list(l).to_vec();
                args.into_iter().fold(Purity::PURE, |p, a| p.min(self.expr_purity_under(a, cast)))
            }
            TExpr::CallMethod(r, s, l) if self.is_known_pure_op(s) && self.fold_constant_as_typed(e).is_some() => {
                let args = self.prog.expr_list(l).to_vec();
                let p = self.expr_purity_under(r, cast).min(Purity::PURE);
                args.into_iter().fold(p, |p, a| p.min(self.expr_purity_under(a, cast)))
            }
            // `scala.quoted.Type.of[T]` and `scala.compiletime.erasedValue[T]` (`exprPurity`'s
            // `TypeApply` cases).
            TExpr::CallStatic(s, l) if l.len == 0 && self.is_pure_type_apply(s) => Purity::PURE,
            // A stable member's reference, an export's forwarder of an object or of a val
            // (`final def O: p.O.type`, `isStableMember`): a path (`refPurity`).
            TExpr::CallStatic(s, l) if l.len == 0 && self.stable_member_def(s) => {
                let owner = match self.syms.sym(s).owner {
                    Owner::Class(o) => self.module_purity(o),
                    _ => Purity::PURE_PATH,
                };
                owner.min(Purity::PURE_PATH)
            }
            TExpr::CallMethod(r, s, l) if l.len == 0 && self.stable_member_def(s) => self.expr_purity_under(r, cast).min(Purity::PURE_PATH),
            _ => Purity::IMPURE,
        }
    }

    /// A method that is a stable member (`StableRealizable`): an export's forwarder of a stable path.
    pub(super) fn stable_member_def(&self, s: SymId) -> bool {
        let info = self.syms.sym(s);
        info.kind == SymKind::Def && info.stable
    }

    /// dotty's `TreeInfo.statPurity`: a local method is pure, an immutable val its value's purity
    /// capped at `Pure` (no path), a `var` or any other statement impure.
    pub(super) fn stat_purity(&mut self, s: TStmt) -> Purity {
        self.stat_purity_under(s, Purity::PURE)
    }

    /// `stat_purity` for `expr_purity_under`'s asker.
    fn stat_purity_under(&mut self, s: TStmt, cast: Purity) -> Purity {
        match s {
            TStmt::Val(v, _) if self.syms.sym(v).kind == SymKind::Var || self.syms.sym(v).mods & mods::MUTABLE != 0 => Purity::IMPURE,
            TStmt::Val(_, x) => self.expr_purity_under(x, cast).min(Purity::PURE),
            TStmt::Fun(_) => Purity::PURE,
            TStmt::Expr(_) | TStmt::Pat(..) => Purity::IMPURE,
        }
    }

    /// Whether `s` is a lazy val's accessor.
    fn lazy_accessor(&self, s: SymId) -> bool {
        let info = self.syms.sym(s);
        info.mods & mods::LAZY != 0 && matches!(info.kind, SymKind::Val | SymKind::Given)
    }

    /// `isKnownPureOp`: a method of a primitive value class or of `String`, or one of `Any`'s
    /// `pureMethods` (`==`, `!=`, `equals`, `hashCode`, `toString`, `##`, `getClass`,
    /// `isInstanceOf`, `eq`, `ne`).
    fn is_known_pure_op(&self, s: SymId) -> bool {
        let info = self.syms.sym(s);
        match info.owner {
            Owner::Class(c) => {
                if c == self.b.string || self.syms.class(c).kind == ClassKind::Builtin {
                    return true;
                }
                let any = c == self.b.any_ref || self.class_path(c) == "scala.Any";
                any && matches!(self.name_ref(info.name), "==" | "!=" | "equals" | "hashCode" | "toString" | "##" | "getClass" | "isInstanceOf" | "eq" | "ne")
            }
            _ => false,
        }
    }

    /// `isPureSyntheticCaseApply`: the synthetic `apply` of a case class's companion whose class
    /// has no initialisation (`isNoInitsRealClass`).
    pub(super) fn is_pure_synthetic_case_apply(&mut self, s: SymId) -> bool {
        let info = self.syms.sym(s);
        if info.name != crate::names::APPLY || info.def.is_some() {
            return false;
        }
        let Owner::Class(o) = info.owner else { return false };
        if self.syms.class(o).kind != ClassKind::Object {
            return false;
        }
        let Some(k) = self.syms.class(o).companion else { return false };
        self.syms.class(k).mods & mods::CASE != 0 && self.no_inits_real_class(k)
    }

    /// `exprPurity`'s pure `TypeApply` cases: `Type.of[T]` and `erasedValue[T]` (`classOf` and
    /// `asInstanceOf` have nodes of their own).
    fn is_pure_type_apply(&self, s: SymId) -> bool {
        let info = self.syms.sym(s);
        let name = self.name_ref(info.name);
        match info.owner {
            Owner::Class(c) => name == "of" && self.class_path(c) == "scala.quoted.Type",
            Owner::Package(p) => name == "erasedValue" && self.pkg_path(p) == "scala.compiletime",
            Owner::Local => false,
        }
    }

    /// dotty's `TreeInfo.refPurity` of a reference to `s`: a method that takes parameters, a
    /// reference whose type is a constant, a stable member (a module's by its initialisation, a
    /// lazy one idempotent, an inline parameter impure), else impure.
    pub(super) fn symbol_purity(&mut self, s: SymId) -> Purity {
        let (kind, mods) = (self.syms.sym(s).kind, self.syms.sym(s).mods);
        // `tree.tpe.isInstanceOf[ConstantType]`: a value declared of a constant type.
        if matches!(kind, SymKind::Val | SymKind::Param) && mods & (mods::MUTABLE | mods::LAZY) == 0 {
            if let Some(sig) = self.syms.sym(s).sig.as_ref() {
                if sig.clauses.is_empty() && sig.tparams.is_empty() && matches!(self.types.get(sig.ret), Type::Lit(_)) {
                    return Purity::PURE_PATH;
                }
            }
        }
        match kind {
            SymKind::Var => Purity::IMPURE,
            _ if mods & mods::MUTABLE != 0 => Purity::IMPURE,
            SymKind::Object(c) => self.module_purity(c),
            SymKind::Param if mods & mods::INLINE != 0 => Purity::IMPURE,
            SymKind::Val | SymKind::Param | SymKind::EnumValue(_) if mods & mods::LAZY == 0 => Purity::PURE_PATH,
            // A given with parameters is a method, whose reference is a path (`Parsers.givenDef`).
            SymKind::Given if self.syms.sym(s).sig.as_ref().is_some_and(|sig| !sig.clauses.is_empty() || !sig.tparams.is_empty()) => Purity::PURE_PATH,
            SymKind::Val | SymKind::Param | SymKind::EnumValue(_) | SymKind::Given => Purity::IDEMPOTENT_PATH,
            // A method's reference is a path unless it is parameterless (`refPurity`'s
            // `isParameterless`), where it is no stable member but a stable forwarder
            // (`StableRealizable`).
            SymKind::Def => match self.syms.sym(s).sig.as_ref() {
                Some(sig) if sig.clauses.is_empty() && !self.syms.sym(s).stable => Purity::IMPURE,
                _ => Purity::PURE_PATH,
            },
            SymKind::Overloaded(_) => Purity::PURE_PATH,
        }
    }

    /// dotty's `TreeInfo.exprPurity` of the reference `tpd.singleton` makes of the path type `t`:
    /// a constant's literal and `this` are pure paths; a member whose prefix `tpd.ref` elides
    /// (`prefixIsElidable`: a static object, a package, the block; `this` counts for nothing) is
    /// an `Ident`, its own `refPurity`; any other member is a `Select`, its `refPurity` under
    /// its prefix's. An object nested in an instance is selected on a path its type does not
    /// hold: impure, no more being known.
    pub(super) fn singleton_purity(&mut self, t: TypeId) -> Purity {
        match self.types.get(t) {
            Type::Lit(_) | Type::This(_) => Purity::PURE_PATH,
            Type::Term(s) => self.symbol_purity(s),
            Type::Select(p, s) => {
                let own = self.symbol_purity(s);
                let elided = match self.types.get(p) {
                    Type::Term(m) => matches!(self.syms.sym(m).kind, SymKind::Object(o) if self.static_object(o)),
                    Type::Class(o, _) => self.syms.class(o).kind == ClassKind::Object && self.static_object(o),
                    _ => false,
                };
                if elided {
                    own
                } else {
                    own.min(self.singleton_purity(p))
                }
            }
            Type::Class(o, _) if self.syms.class(o).kind == ClassKind::Object && self.static_object(o) => self.module_purity(o),
            _ => Purity::IMPURE,
        }
    }

    /// The purity of a creation of `c`, a class without initialisation, before its arguments': pure
    /// (`isPureApply`). For the asker that decides an order of evaluation (`cast` below pure: the
    /// lift of an operator's arguments), a case class's creation is its companion's `apply`, whose
    /// qualifier is the companion: idempotent where the companion has initialisation (`refPurity` of
    /// a module), which every back end runs in front of the creation (`emit::touched_companion`), a
    /// creation written with `new` included.
    pub(super) fn creation_purity_under(&mut self, c: ClassId, cast: Purity) -> Purity {
        if cast == Purity::PURE {
            return Purity::PURE;
        }
        let info = self.syms.class(c);
        if info.kind != ClassKind::Class || info.mods & mods::CASE == 0 {
            return Purity::PURE;
        }
        match info.companion {
            Some(m) if self.syms.class(m).kind == ClassKind::Object => self.module_purity(m).min(Purity::PURE),
            _ => Purity::PURE,
        }
    }

    /// An object's reference: a path where its class has no initialisation, else idempotent
    /// (`refPurity`, `isNoInitsRealClass`).
    fn module_purity(&mut self, c: ClassId) -> Purity {
        if self.no_inits(c) {
            Purity::PURE_PATH
        } else {
            Purity::IDEMPOTENT_PATH
        }
    }

    /// `isNoInitsRealClass` (`no_inits_real_class`): the one rule the purity of an object's
    /// reference and of a construction read.
    pub(super) fn no_inits(&mut self, c: ClassId) -> bool {
        self.no_inits_real_class(c)
    }

    /// TreeInfo.refPurity of a reference written in `r.view`: an `Ident` by its symbol alone; a `Select` by its symbol,
    /// then by its qualifier's exprPurity, a written cast impure there.
    #[allow(dead_code)]
    pub(crate) fn ref_purity(&mut self, r: RefView) -> Purity {
        let own = self.symbol_purity(r.sym);
        match r.view {
            View::Ident => own,
            View::Select(q) => own.min(self.expr_purity_under(q, Purity::IMPURE)),
        }
    }

    /// TreeInfo.isPureExpr (`exprPurity >= Pure`), a written cast impure; a plain inline call still pending is impure,
    /// as the reference's Apply of an inline method is before the inlining.
    pub(crate) fn is_pure_expr(&mut self, e: TExprId) -> bool {
        !self.is_pending_call(e) && self.expr_purity_under(e, Purity::IMPURE).at_least(Purity::PURE)
    }

    /// TreeInfo.isIdempotentExpr (`exprPurity >= Idempotent`).
    #[allow(dead_code)]
    pub(crate) fn is_idempotent_expr(&mut self, e: TExprId) -> bool {
        !self.is_pending_call(e) && self.expr_purity_under(e, Purity::IMPURE).at_least(Purity::IDEMPOTENT)
    }

    /// TreeInfo.isPurePath: of a constant type, `exprPurity >= Pure`; else `exprPurity == PurePath`.
    // No reader yet: the call order keeps its own reading (`Worker::order_pure_path`).
    #[allow(dead_code)]
    pub(crate) fn is_pure_path(&mut self, e: TExprId) -> bool {
        if self.is_pending_call(e) {
            return false;
        }
        let p = self.expr_purity_under(e, Purity::IMPURE);
        if self.prog.type_of(e).is_some_and(|t| matches!(self.types.get(t), Type::Lit(_))) {
            p.at_least(Purity::PURE)
        } else {
            p == Purity::PURE_PATH
        }
    }

    /// TreeInfo.isIdempotentPath: of a constant type, `exprPurity >= Idempotent`; else `exprPurity >= IdempotentPath`.
    #[allow(dead_code)]
    pub(crate) fn is_idempotent_path(&mut self, e: TExprId) -> bool {
        if self.is_pending_call(e) {
            return false;
        }
        let p = self.expr_purity_under(e, Purity::IMPURE);
        if self.prog.type_of(e).is_some_and(|t| matches!(self.types.get(t), Type::Lit(_))) {
            p.at_least(Purity::IDEMPOTENT)
        } else {
            p.at_least(Purity::IDEMPOTENT_PATH)
        }
    }

    /// TreeInfo.isPureBinding (`statPurity >= Pure`).
    #[allow(dead_code)]
    pub(crate) fn is_pure_binding(&mut self, s: TStmt) -> bool {
        self.stat_purity_under(s, Purity::IMPURE).at_least(Purity::PURE)
    }

    /// Inliner.isElideableExpr, a predicate of its own: the argument's shape (`elideable_argument`: a reference, a
    /// literal, a closure, a creation, a pure block) and the purity of what it evaluates (`elideable_by_purity`).
    pub(crate) fn is_elideable(&mut self, e: TExprId) -> bool {
        self.elideable_argument(e) && self.elideable_by_purity(e)
    }
}
