//! The elaborated application: dotty's `Applications.TypedApply` over `Application.matchArgs`,
//! clause by clause. An argument the source leaves out is the application of its default
//! getter (`defaultArgument`: `findDefaultGetter` selects it on the receiver, `spliceMeth`
//! applies it to the earlier clauses' arguments), after the function part is lifted
//! (`liftFun`, `EtaExpansion.Lifter.liftApp`) so that the receiver and those arguments are
//! evaluated once, before the clause's own; arguments written out of parameter order are lifted
//! in that order and sorted back to the order written, the defaults after them (`result`,
//! `Lifter.liftArgs`). The plan is ordinary typed bindings and `TExpr::Default` nodes that
//! every consumer executes as given, and a pickle writes as scalac's tree.

use super::check::Agreement;
use super::Worker;
use crate::ast::mods;
use crate::source::Span;
use crate::symbols::*;
use crate::tir::*;
use crate::types::*;
use crate::intern::Name;

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

/// What `type_clause_args` leaves to the application's plan (`Worker::elaborate_clause`).
#[derive(Default)]
pub struct ClauseArgs {
    /// The method and where the clause's parameters start among its: a parameter without a
    /// default of its own takes one a member it overrides declares (`hasDefaultParams`).
    pub inherit: Option<(SymId, usize)>,
    /// Whose default getters the clause's omitted arguments apply, and where the clause's
    /// parameters start among the callee's (`getters`).
    pub getters: Option<(DefaultOf, usize)>,
    /// The steps the callee's signature was seen through from the receiver, which a getter's
    /// type is seen through as well (`Worker::replay_sig_view`).
    pub view: Vec<super::apply::ViewStep>,
    /// The type of each omitted argument's getter, seen so, by the parameter's place in the
    /// clause: what the getter's application has (`elaborate_written`).
    pub getter_types: Vec<(usize, TypeId)>,
    /// The parameters the arguments left to their defaults, in order.
    pub omitted: Vec<usize>,
    /// The parameter of each argument, in the order written.
    pub order: Vec<usize>,
}

impl ClauseArgs {
    /// Whether the clause needs its application's plan: an argument left out, or the arguments
    /// written out of order.
    pub fn planned(&self) -> bool {
        !self.omitted.is_empty() || !self.order.is_empty()
    }
}

/// An application under elaboration, its clauses typed one after the other.
pub(super) struct CallPlan {
    /// Whose default getters the omitted arguments apply.
    pub of: Option<DefaultOf>,
    /// The callee's signature, whose parameters say which arguments stay in place.
    pub sig: std::sync::Arc<MethodSig>,
    /// `LiftComplex` where the method has default parameters, its own or an overridden
    /// member's, `LiftImpure` otherwise (`TypedApply.lifter`, `methRef.symbol.hasDefaultParams`):
    /// decided where first needed.
    pub complex: Option<bool>,
    /// No lifting at all: an annotation's constructor (`isAnnotConstr`).
    pub annotation: bool,
    /// Whether the function part was lifted for the clause being elaborated: once per
    /// application (`liftedDefs`).
    pub lifted_fun: bool,
    /// The getter applications made, which take the call's type arguments (`spliceMeth`'s
    /// `TypeApply`).
    pub defaults: Vec<TExprId>,
}

impl CallPlan {
    pub fn new(of: DefaultOf, sig: std::sync::Arc<MethodSig>, annotation: bool) -> CallPlan {
        CallPlan { of: Some(of), sig, complex: None, annotation, lifted_fun: false, defaults: Vec::new() }
    }

    /// The parameter `i`, counted across the clauses.
    fn param(&self, i: usize) -> Option<&ParamSig> {
        self.sig.clauses.iter().flat_map(|c| c.params.iter()).nth(i)
    }
}

/// How many of the parameters of `sig`, flattened, come before the clause of parameter `i`:
/// what its default getter takes (`getterParamss`).
pub(crate) fn clause_start(sig: &MethodSig, i: usize) -> usize {
    let mut start = 0;
    for c in &sig.clauses {
        if i < start + c.params.len() {
            return start;
        }
        start += c.params.len();
    }
    i
}

/// `SymDenotation.isNoInitsRealClass` over the symbol tables alone, for the emitters, which have
/// no worker: a class or an object, no trait, which is assured (`isAssuredNoInits`: `Some`, a
/// tuple class, an object whose companion is one) or every base of which (`base_types`, the
/// traits among them) runs nothing when an instance is made (`NoInits`), as the typer recorded
/// it at the class's completion from its source or its pickle (`ClassInfo::no_inits_recorded`,
/// `Worker::class_no_inits`), else as its records hold it.
pub(crate) fn no_inits(syms: &Symbols, interner: &crate::intern::Interner, c: ClassId) -> bool {
    if syms.class(c).kind == ClassKind::Trait {
        return false;
    }
    if is_assured_no_inits(syms, interner, c) {
        return true;
    }
    syms.class(c).base_types.iter().all(|&(k, _)| class_no_inits_recorded(syms, k))
}

/// `Definitions.isAssuredNoInits` over the symbol tables: `scala.Some`, `scala.Tuple1` to
/// `scala.Tuple22` (`MaxTupleArity`), or an object whose companion is one.
fn is_assured_no_inits(syms: &Symbols, interner: &crate::intern::Interner, c: ClassId) -> bool {
    let assured = |k: ClassId| {
        let info = syms.class(k);
        let in_scala = matches!(info.owner, Owner::Package(p) if interner.get(syms.pkg(p).name) == "scala" && syms.pkg(p).parent.is_some_and(|q| syms.pkg(q).parent.is_none()));
        let name = interner.get(info.name);
        in_scala && (name == "Some" || name.strip_prefix("Tuple").is_some_and(|n| n.parse::<u32>().is_ok_and(|n| (1..=22).contains(&n)))) // dotty's `Definitions.MaxTupleArity`, `isAssuredNoInits`
    };
    let info = syms.class(c);
    if info.kind == ClassKind::Object {
        return info.companion.is_some_and(|k| syms.class(k).kind != ClassKind::Object && assured(k));
    }
    assured(c)
}

/// Whether the class `k` runs nothing of its own when an instance is made: as the typer recorded
/// it at the class's completion (`no_inits_recorded`, `TreeInfo.parentsKind` and `defKind` over its
/// source, `TreeUnpickler.indexStats` over its pickle), else as its records hold it: no statement,
/// no val with an initialiser (a module's, an enum case's and a parameterless given's among them),
/// no companion object made for its nested classes' default getters.
pub(crate) fn class_no_inits_recorded(syms: &Symbols, k: ClassId) -> bool {
    let info = syms.class(k);
    match info.no_inits_recorded {
        1 => return true,
        2 => return false,
        _ => {}
    }
    if info.has_statements {
        return false;
    }
    let fields = info.member_order.iter().any(|&m| {
        let s = syms.sym(m);
        let valued = match s.kind {
            SymKind::Val | SymKind::Var | SymKind::Object(_) | SymKind::EnumValue(_) => true,
            SymKind::Given => s.sig.as_ref().map_or(true, |sig| sig.clauses.is_empty()),
            _ => false,
        };
        valued && s.mods & mods::ABSTRACT == 0 && !info.ctor_syms.iter().any(|cl| cl.contains(&m))
    });
    let getters_companion = info.nested.values().any(|&n| {
        let k = syms.class(n);
        matches!(k.kind, ClassKind::Class | ClassKind::Trait) && k.companion.is_none() && k.ctor.iter().any(|cl| cl.params.iter().any(|p| p.has_default))
    });
    !fields && !getters_companion
}

impl<'a> Worker<'a> {
    /// dotty's `TreeInfo.exprPurity` over the typed IR; a reference as it was written where the
    /// IR's node says otherwise (`Worker::note_written_ref`).
    pub(super) fn expr_purity(&mut self, e: TExprId) -> Purity {
        if let Some(&p) = self.written_purity.get(&e) {
            return p;
        }
        match self.prog.expr(e) {
            TExpr::Int(_) | TExpr::Long(_) | TExpr::Double(_) | TExpr::Bool(_) | TExpr::Char(_) | TExpr::Str(_) | TExpr::Unit | TExpr::Null => Purity::PURE_PATH,
            TExpr::This | TExpr::Super(_) => Purity::PURE_PATH,
            // `classOf[T]`, `Predef.classOf`.
            TExpr::ClassOf(..) => Purity::PURE,
            TExpr::Local(s) => self.ref_purity(s),
            // A member of an object: its selection on the object.
            TExpr::Static(s) => {
                let owner = match self.syms.sym(s).owner {
                    Owner::Class(o) => self.module_purity(o),
                    _ => Purity::PURE_PATH,
                };
                self.ref_purity(s).min(owner)
            }
            TExpr::Module(c) => self.module_purity(c),
            TExpr::Field(r, s) => self.ref_purity(s).min(self.expr_purity(r)),
            TExpr::Lambda(..) => Purity::PURE,
            // A constructor of a class without initialisation is stable, as a case class's
            // synthetic `apply` is (`isPureApply`, `isPureSyntheticCaseApply`).
            TExpr::New(c, args) if self.no_inits(c) => {
                let args = self.prog.expr_list(args).to_vec();
                args.into_iter().fold(Purity::PURE, |p, a| p.min(self.expr_purity(a)))
            }
            // `minOf(exprPurity(expr), stats.map(statPurity))`.
            TExpr::Block(stmts, res) => {
                let mut p = self.expr_purity(res);
                for s in self.prog.stmts[stmts.range()].to_vec() {
                    p = p.min(self.stat_purity(s));
                }
                p
            }
            // `e.asInstanceOf[T]`, `Any_typeCast`.
            TExpr::Cast(r, ..) => Purity::PURE.min(self.expr_purity(r)),
            // A primitive or string operation the typer folds to a constant, over operands as pure
            // (`isPureApply`: an application of a known pure operation whose type is a constant).
            TExpr::Unary(_, a) | TExpr::ToStr(a, _) if self.fold_constant_as_typed(e).is_some() => Purity::PURE.min(self.expr_purity(a)),
            TExpr::Prim(_, a, b) if self.fold_constant_as_typed(e).is_some() => Purity::PURE.min(self.expr_purity(a)).min(self.expr_purity(b)),
            TExpr::StrConcat(l) if self.fold_constant_as_typed(e).is_some() => {
                let parts = self.prog.expr_list(l).to_vec();
                parts.into_iter().fold(Purity::PURE, |p, x| p.min(self.expr_purity(x)))
            }
            // A lazy val read through its accessor (`fn.symbol.isStableMember`): idempotent, its
            // first read running the initialiser.
            TExpr::CallStatic(s, l) if l.len == 0 && self.lazy_accessor(s) => Purity::IDEMPOTENT,
            TExpr::CallMethod(r, s, l) if l.len == 0 && self.lazy_accessor(s) => self.expr_purity(r).min(Purity::IDEMPOTENT),
            // `isPureApply`: a constant expression of a known pure operation (a primitive value
            // class's or `String`'s method, one of `Any`'s `pureMethods`) over pure arguments, and a
            // case class's synthetic `apply` whose class has no initialisation
            // (`isPureSyntheticCaseApply`).
            TExpr::CallStatic(s, l) if self.is_pure_synthetic_case_apply(s) => {
                let args = self.prog.expr_list(l).to_vec();
                args.into_iter().fold(Purity::PURE, |p, a| p.min(self.expr_purity(a)))
            }
            TExpr::CallStatic(s, l) if self.is_known_pure_op(s) && self.fold_constant_as_typed(e).is_some() => {
                let args = self.prog.expr_list(l).to_vec();
                args.into_iter().fold(Purity::PURE, |p, a| p.min(self.expr_purity(a)))
            }
            TExpr::CallMethod(r, s, l) if self.is_known_pure_op(s) && self.fold_constant_as_typed(e).is_some() => {
                let args = self.prog.expr_list(l).to_vec();
                let p = self.expr_purity(r).min(Purity::PURE);
                args.into_iter().fold(p, |p, a| p.min(self.expr_purity(a)))
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
            TExpr::CallMethod(r, s, l) if l.len == 0 && self.stable_member_def(s) => self.expr_purity(r).min(Purity::PURE_PATH),
            _ => Purity::IMPURE,
        }
    }

    /// A method that is a stable member (`StableRealizable`): an export's forwarder of a stable path.
    fn stable_member_def(&self, s: SymId) -> bool {
        let info = self.syms.sym(s);
        info.kind == SymKind::Def && info.stable
    }

    /// dotty's `TreeInfo.statPurity`: a local method is pure, an immutable val its value's purity
    /// capped at `Pure` (no path), a `var` or any other statement impure.
    pub(super) fn stat_purity(&mut self, s: TStmt) -> Purity {
        match s {
            TStmt::Val(v, _) if self.syms.sym(v).kind == SymKind::Var || self.syms.sym(v).mods & mods::MUTABLE != 0 => Purity::IMPURE,
            TStmt::Val(_, x) => self.expr_purity(x).min(Purity::PURE),
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
    fn is_pure_synthetic_case_apply(&mut self, s: SymId) -> bool {
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
    fn ref_purity(&mut self, s: SymId) -> Purity {
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

    /// The method whose parameter `index`, counted across its clauses, has the default a call
    /// of `m` applies: `m` where its parameter declares one, else the nearest member it
    /// overrides that does (dotty's `SymDenotation.hasDefaultParams` counts the overridden
    /// symbols, `Applications.findDefaultGetter` selects the getter on the receiver).
    pub(crate) fn default_declarer(&mut self, m: SymId, index: usize) -> Option<SymId> {
        let declares = |w: &mut Self, s: SymId| w.sig_of(s).clauses.iter().flat_map(|c| c.params.iter()).nth(index).map_or(false, |p| p.has_default);
        if declares(self, m) {
            return Some(m);
        }
        let (name, is_ext, owner, kind) = {
            let s = self.syms.sym(m);
            (s.name, s.is_extension, s.owner, s.kind)
        };
        let Owner::Class(c) = owner else { return None };
        if is_ext || name == crate::names::INIT || kind != SymKind::Def {
            return None;
        }
        self.complete_class(c);
        let msig = self.sig_arc(m);
        for j in 1..self.syms.class(c).base_types.len() {
            let b = self.syms.class(c).base_types[j].0;
            let bt = self.base_instance_at(c, j);
            let mut k = 0;
            while let Some(p) = self.inherited_member(b, name, false, k) {
                k += 1;
                if self.is_private(p) || self.syms.sym(p).kind != SymKind::Def {
                    continue;
                }
                let mark = self.snapshot();
                let agreement = self.compare_sigs(c, msig.clone(), None, p, bt);
                self.rollback(mark);
                if agreement != Agreement::Params && declares(self, p) {
                    return Some(p);
                }
            }
        }
        None
    }

    /// Whether the argument of parameter `i` is never lifted: a by-name parameter's
    /// (`exprLifter`, `NoLift`) or an inline one's (`InlineParamAnnot`), whatever the callee; an
    /// inline method's ordinary getter is a method like any other (`Lifter.liftArgs`, `wouldLift`).
    fn stays_in_place(&self, plan: &CallPlan, i: usize) -> bool {
        plan.param(i).is_some_and(|p| p.by_name || self.syms.sym(p.sym).mods & mods::INLINE != 0)
    }

    /// `methRef.symbol.hasDefaultParams`: whether the callee has a default parameter, its own
    /// or one of a member it overrides, which makes the lifter `LiftComplex`.
    fn plan_complex(&mut self, plan: &mut CallPlan) -> bool {
        if let Some(c) = plan.complex {
            return c;
        }
        let own = plan.sig.clauses.iter().any(|c| c.params.iter().any(|p| p.has_default));
        let complex = own
            || match plan.of {
                Some(DefaultOf::Method(m)) if matches!(self.syms.sym(m).owner, Owner::Class(_)) => {
                    let count = plan.sig.clauses.iter().map(|c| c.params.len()).sum::<usize>();
                    (0..count).any(|i| self.default_declarer(m, i).is_some())
                }
                _ => false,
            };
        plan.complex = Some(complex);
        complex
    }

    /// The plan of a clause `type_clause_args_planned` typed into `args[first..]`, the
    /// omitted parameters' types read from the clause under `subst`.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn elaborate_written(
        &mut self,
        plan: &mut CallPlan,
        recv: &mut Option<TExprId>,
        args: &mut [TExprId],
        first: usize,
        written: &ClauseArgs,
        clause: &ClauseSig,
        subst: &Subst,
        span: Span,
    ) {
        if written.omitted.is_empty() && written.order.is_empty() {
            return;
        }
        let mut types: Vec<TypeId> = if written.omitted.is_empty() { Vec::new() } else { clause.params.iter().map(|p| self.param_type(p.ty, subst)).collect() };
        // An omitted argument is its getter's application, of the getter's type as the arguments'
        // typing saw it (`ClauseArgs::getter_types`: `tryDefault`'s `defaultArg.tpe.widen`).
        for &(j, t) in &written.getter_types {
            if j < types.len() {
                types[j] = t;
            }
        }
        self.elaborate_clause(plan, recv, args, first, written, &types, span);
    }

    /// `liftFun`: the receiver and the earlier clauses' arguments, bound where their evaluation
    /// could be told from a later one's (`Lifter.liftApp`): the receiver unless a literal, `this`,
    /// a `new`'s or an idempotent reference (`liftPrefix`) where the method has defaults, unless
    /// idempotent otherwise (`liftNonIdempotentPrefix`); each argument unless a path (`LiftComplex`)
    /// or pure (`LiftImpure`), a by-name or inline parameter's never.
    pub(super) fn lift_fun(&mut self, plan: &mut CallPlan, recv: &mut Option<TExprId>, earlier: &mut [TExprId], span: Span) {
        if plan.lifted_fun || plan.annotation {
            return;
        }
        plan.lifted_fun = true;
        if let Some(r) = *recv {
            if !matches!(self.prog.expr(r), TExpr::Super(_)) {
                self.expand_pending_arg(r);
                let keep = match self.prog.expr(r) {
                    TExpr::Int(_) | TExpr::Long(_) | TExpr::Double(_) | TExpr::Bool(_) | TExpr::Char(_) | TExpr::Str(_) | TExpr::Unit | TExpr::Null | TExpr::This => true,
                    TExpr::Local(_) | TExpr::Static(_) | TExpr::Module(_) | TExpr::Field(..) => self.expr_purity(r).at_least(Purity::IDEMPOTENT),
                    _ => !self.plan_complex(plan) && self.expr_purity(r).at_least(Purity::IDEMPOTENT),
                };
                if !keep {
                    *recv = Some(self.lift(r, None, true, span));
                }
            }
        }
        for (i, a) in earlier.iter_mut().enumerate() {
            if self.stays_in_place(plan, i) {
                continue;
            }
            self.expand_pending_arg(*a);
            if !self.no_lift(plan, *a) {
                let param = plan.param(i).map(|p| p.name);
                let value = self.expand_lifted_inline_getters(plan, *a, span);
                *a = self.lift(value, param, false, span);
            }
        }
    }

    /// Whether the lifter leaves an argument in place: `LiftComplex.noLift` (a path) or
    /// `LiftImpure.noLift` (pure).
    fn no_lift(&mut self, plan: &mut CallPlan, a: TExprId) -> bool {
        let p = self.expr_purity(a);
        if self.plan_complex(plan) {
            p == Purity::PURE_PATH
        } else {
            p.at_least(Purity::PURE)
        }
    }

    /// The purity of the reference `te` as written, where its node says otherwise: an `Ident`'s is
    /// its symbol's alone (`exprPurity`, `refPurity`), not its owner's the node selects it on, and
    /// one written through an export is the forwarder's, a stable member (`exported`).
    pub(super) fn note_written_ref(&mut self, te: TExprId, exported: bool) {
        // Recorded whatever the node's purity is now: an object's class may complete later,
        // with initialisation its node's purity then sees. Of a name's that is no path, the
        // node's is the same whatever its owner's (an idempotent path at least): not recorded.
        let written = match self.prog.expr(te) {
            TExpr::Module(_) if exported => Purity::PURE_PATH,
            TExpr::Field(r, _) if !exported && matches!(self.prog.expr(r), TExpr::This) => return,
            TExpr::Field(_, s) | TExpr::Static(s) => {
                let info = self.syms.sym(s);
                if exported && matches!(info.kind, SymKind::Val | SymKind::Object(_)) && info.mods & (mods::LAZY | mods::MUTABLE) == 0 {
                    Purity::PURE_PATH
                } else {
                    self.ref_purity(s)
                }
            }
            _ => return,
        };
        if !exported && written != Purity::PURE_PATH {
            return;
        }
        self.written_purity.insert(te, written);
    }

    /// A plain inline call still pending among the arguments counts by its expansion, as where
    /// every call expanded as typed.
    fn expand_pending_arg(&mut self, a: TExprId) {
        if self.is_pending_call(a) {
            self.expand_pending_kept(&[a]);
        }
    }

    /// An inline callee's ordinary getter application lifted out of the call: an inline
    /// parameter's getter application among its arguments is inline (`Desugar.addDefaultGetters`)
    /// and expanded here, where the call's binding does not see it.
    fn expand_lifted_inline_getters(&mut self, plan: &CallPlan, e: TExprId, span: Span) -> TExprId {
        let Some(DefaultOf::Method(callee)) = plan.of else { return e };
        let TExpr::Default(DefaultOf::Method(m), ..) = self.prog.expr(e) else { return e };
        // The callee's own getter, or the forwarded member's an inline forwarder's stands for.
        let own = m == callee || (self.is_inline_forwarder(callee) && self.syms.forward_of(callee).is_some_and(|f| f.target == m));
        if !own || !self.is_inline_callee(m) {
            return e;
        }
        let Some(body) = self.inline_body(m) else { return e };
        let sig = self.sig_arc(m);
        self.own_getter_expansion(m, &sig, &Vec::new(), &body, None, e, span)
    }

    /// `Lifter.lift`: `e` bound to a temporary in front of the call, its reference returned,
    /// named as `UniqueName.fresh(prefix)` names it: an argument's after its parameter
    /// (`liftArgs`, `xs$1`) unless that name has a `$`, the function part's of no prefix (`$1`).
    /// The temporary has the value's type, widened (`liftedExprType`), a receiver's as it is;
    /// a type an inference still constrains is `Any`, the value's recorded for the pickle, as
    /// the typer's other temporaries are.
    fn lift(&mut self, e: TExprId, param: Option<Name>, receiver: bool, span: Span) -> TExprId {
        // A reference the typer gives no node type: an object's, its class's (`widen`), a
        // local's or a member's, its symbol's.
        let own = self.prog.type_of(e).or_else(|| match self.prog.expr(e) {
            TExpr::Module(c) => Some(self.types.class(c, &[])),
            TExpr::Local(s) | TExpr::Static(s) => self.syms.sym(s).sig.as_ref().map(|sig| sig.ret),
            // An object nested in a class selected on its prefix, a companion whose `apply` the
            // call applies: its module's type, which its members are found on.
            TExpr::Field(_, s) | TExpr::CallMethod(_, s, _) if receiver => {
                let ret = self.syms.sym(s).sig.as_ref().map(|sig| sig.ret)?;
                self.class_of(ret).is_some_and(|m| self.syms.class(m).inner_object == Some(s)).then_some(ret)
            }
            _ => None,
        });
        let ty = match own {
            Some(t) if receiver => t,
            Some(t) if !self.types.has_vars(t) => {
                let t = self.widen_path(t);
                self.widen_lit(t)
            }
            _ => ANY,
        };
        let prefix = param.map(|n| self.name_ref(n)).filter(|n| !n.contains('$')).unwrap_or("");
        let name = self.interner.intern(&format!("{}${}", prefix, self.hoisted.len() + 1));
        let tmp = self.new_local(name, SymKind::Val, ty, span);
        self.syms.sym_mut(tmp).mods |= mods::LIFTED;
        let r = self.prog.add(TExpr::Local(tmp));
        if self.capturing() {
            if let Some(own) = own.filter(|&t| t != ty) {
                self.capture_local(tmp, |l| l.ty = Some(own));
            }
            // A named argument keeps its name on the reference (`Lifter.liftArg`).
            if let Some(n) = self.captured_name(e) {
                self.capture_unname(e);
                self.capture_wrap(r, crate::tir::capture::Wrap::Named(n));
            }
        }
        self.hoisted.push(TStmt::Val(tmp, e));
        r
    }

    /// One clause's plan, its arguments typed at `args[first..]` (`TypedApply` over
    /// `matchArgs`): the omitted ones become their getters' applications once the function part
    /// is lifted, and where the arguments were written out of parameter order and one has an
    /// effect, they are lifted in parameter order and the bindings sorted back to the order
    /// written, the defaults after them (`result`).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn elaborate_clause(
        &mut self,
        plan: &mut CallPlan,
        recv: &mut Option<TExprId>,
        args: &mut [TExprId],
        first: usize,
        written: &ClauseArgs,
        param_types: &[TypeId],
        span: Span,
    ) {
        plan.lifted_fun = false;
        let n = args.len() - first;
        if !written.omitted.is_empty() {
            // `liftedFunType`: the clause has fewer arguments than parameters (`needLiftFun`).
            if !plan.annotation {
                let (earlier, _) = args.split_at_mut(first);
                self.lift_fun(plan, recv, earlier, span);
            }
            for &j in &written.omitted {
                let ty = param_types.get(j).copied().unwrap_or(ANY);
                let d = self.default_application(plan, *recv, &args[..first], first + j, ty);
                let d = if plan.param(first + j).is_some_and(|p| p.by_name) { self.by_name_thunk(d) } else { d };
                args[first + j] = d;
            }
        }
        // `sameSeq(args, orderedArgs)`: every argument at its parameter's place.
        let same = {
            let mut next = 0usize;
            let mut ok = true;
            for &p in &written.order {
                if p == next {
                    next += 1;
                } else if p + 1 != next {
                    ok = false;
                    break;
                }
            }
            ok
        };
        if same || plan.annotation {
            return;
        }
        // `typedArgs.forall(isSafeArg)`: pure, or a default getter's application.
        for i in 0..n {
            self.expand_pending_arg(args[first + i]);
        }
        let mut safe = true;
        for &a in &args[first..first + n] {
            if !matches!(self.prog.expr(a), TExpr::Default(..)) && !self.expr_purity(a).at_least(Purity::PURE) {
                safe = false;
                break;
            }
        }
        if safe {
            return;
        }
        {
            let (earlier, _) = args.split_at_mut(first);
            self.lift_fun(plan, recv, earlier, span);
        }
        // `liftArgs` in the parameters' order, then the bindings sorted by where each argument
        // was written, the defaults last in that order (`stableSort` over `originalIndex`).
        let mark = self.hoisted.len();
        let mut keys: Vec<usize> = Vec::new();
        for j in 0..n {
            let a = args[first + j];
            if self.stays_in_place(plan, first + j) {
                continue;
            }
            if self.no_lift(plan, a) {
                continue;
            }
            let param = plan.param(first + j).map(|p| p.name);
            let value = self.expand_lifted_inline_getters(plan, a, span);
            args[first + j] = self.lift(value, param, false, span);
            keys.push(written.order.iter().position(|&p| p == j).unwrap_or(usize::MAX));
        }
        let lifted: Vec<TStmt> = self.hoisted.drain(mark..).collect();
        let mut sorted: Vec<(usize, TStmt)> = keys.into_iter().zip(lifted).collect();
        sorted.sort_by_key(|&(k, _)| k);
        self.hoisted.extend(sorted.into_iter().map(|(_, s)| s));
    }

    /// The applications of the default getters of `of` among the arguments `args`, or the values
    /// of the temporaries the call's plan lifted them into (`liftedFunType`), made the getters of
    /// `to` selected on a fresh `recv` each (`DEFAULT_RECEIVER`), where the receiver joins the call
    /// after its plan: an inner class's enclosing instance (dotty's getter on the creation's
    /// prefix, `o.C.<init>$default$N`), the class of the block holding a local def.
    pub(super) fn retarget_defaults(&mut self, args: crate::ast::ListRef, of: DefaultOf, to: DefaultOf, mut recv: impl FnMut(&mut Self) -> TExprId) {
        for a in self.prog.expr_list(args).to_vec() {
            let d = match self.prog.expr(a) {
                TExpr::Local(t) => self.hoisted.iter().rev().find_map(|st| match *st {
                    TStmt::Val(x, init) if x == t => Some(init),
                    _ => None,
                }),
                _ => Some(a),
            };
            let Some(d) = d.map(|d| self.passed_default(d)) else { continue };
            let TExpr::Default(x, at, priors) = self.prog.expr(d) else { continue };
            if x != of || at & DEFAULT_RECEIVER != 0 {
                continue;
            }
            let mut items = vec![recv(self)];
            items.extend_from_slice(self.prog.expr_list(priors));
            let l = self.prog.list(&items);
            self.prog.exprs[d.idx()] = TExpr::Default(to, at | DEFAULT_RECEIVER, l);
        }
    }

    /// The receivers `retarget_defaults` gave the getters of the constructor `of` (a class's
    /// primary, or a secondary one) in the creation `te` (among its arguments, or bound before it
    /// in the blocks around it) made copies of `outer`: the prefix's value that takes the outer
    /// placeholder's place (`pass_outer`).
    pub(super) fn pass_outer_to_defaults(&mut self, te: TExprId, of: DefaultOf, outer: TExprId) {
        let mut found = Vec::new();
        let mut at = te;
        loop {
            match self.prog.expr(at) {
                TExpr::Block(stmts, value) => {
                    for st in self.prog.stmt_list(stmts).to_vec() {
                        if let TStmt::Val(_, init) = st {
                            found.push(init);
                        }
                    }
                    at = value;
                }
                // The closure of an application eta-expanded over a missing clause
                // (`pass_outer_applied`).
                TExpr::Lambda(_, body) => at = body,
                _ => break,
            }
        }
        if let TExpr::New(_, args) | TExpr::NewVia(_, args) = self.prog.expr(at) {
            found.extend_from_slice(self.prog.expr_list(args));
        }
        for d in found {
            let d = self.passed_default(d);
            let TExpr::Default(x, at, l) = self.prog.expr(d) else { continue };
            if x != of || at & DEFAULT_RECEIVER == 0 {
                continue;
            }
            let mut items = self.prog.expr_list(l).to_vec();
            items[0] = self.prior_copy(outer);
            let l = self.prog.list(&items);
            self.prog.exprs[d.idx()] = TExpr::Default(x, at, l);
        }
    }

    /// The getter's application an argument passes: itself, or a by-name parameter's, under the
    /// thunk that passes it (a `Lambda` without parameters).
    pub(super) fn passed_default(&self, a: TExprId) -> TExprId {
        match self.prog.expr(a) {
            TExpr::Lambda(params, body) if params.len == 0 && matches!(self.prog.expr(body), TExpr::Default(..)) => body,
            _ => a,
        }
    }

    /// The application of the default getter of parameter `index` (counted across the
    /// clauses) to the receiver and the arguments of the clauses before it (`spliceMeth`): each
    /// a fresh node, a by-name one a fresh copy of its tree.
    pub(super) fn default_application(&mut self, plan: &mut CallPlan, recv: Option<TExprId>, earlier: &[TExprId], index: usize, ty: TypeId) -> TExprId {
        let mut of = plan.of.expect("a callee with defaults");
        let mut recv = recv;
        if let Some((k, outer)) = self.synthetic_apply_class(of, recv) {
            of = DefaultOf::Ctor(k);
            recv = outer;
        }
        // A constructor's default of a class nested in a class is selected on the companion of the
        // creation's prefix (`Applications.findDefaultGetter`), its receiver from the start: the
        // enclosing instance the creation was given before its arguments (`creation_outer`), or
        // `O.this` where the code stands in `O`; a local class's on its local companion.
        if let (DefaultOf::Ctor(c), None) = (of, recv) {
            recv = match self.creation_outer.filter(|&(k, _)| k == c) {
                Some((_, o)) => Some(o),
                None => self.enclosing_outer_of(c).or_else(|| self.local_companion_ref(c)),
            };
        }
        let mut items = Vec::with_capacity(earlier.len() + 1);
        let mut at = index as u32;
        // An inline forwarder's getter is the forwarded member's on the forwarder's path, through
        // the inline forwarders it relays where each path stands anywhere (`Chain.f` of
        // `export Facade.f` of `export Lib.f`), as their getters forward one another.
        let mut forwarded = match of {
            DefaultOf::Method(m) => self.forwarded_default(m),
            DefaultOf::Ctor(_) => None,
        };
        while let Some((DefaultOf::Method(t), path)) = forwarded {
            if !self.is_inline_forwarder(t) || !path.map_or(true, |p| self.context_free_path(p)) {
                break;
            }
            match self.forwarded_default(t) {
                Some(next) if next.1.map_or(true, |p| self.context_free_path(p)) => forwarded = Some(next),
                _ => break,
            }
        }
        match forwarded {
            Some((member, path)) => {
                of = member;
                if let Some(p) = path {
                    items.push(p);
                    at |= DEFAULT_RECEIVER;
                }
            }
            None => {
                if let Some(r) = recv {
                    items.push(self.prior_copy(r));
                    at |= DEFAULT_RECEIVER;
                }
            }
        }
        for &a in earlier {
            items.push(self.prior_copy(a));
        }
        self.note_super_getter(of, recv, index);
        let l = self.prog.list(&items);
        let d = self.prog.add(TExpr::Default(of, at, l));
        self.prog.set_type(d, ty);
        plan.defaults.push(d);
        d
    }

    /// The class whose constructor's getters stand for the defaults of `of`, a jar's companion's
    /// `apply` its pickle marks synthesized, which the call applies (`constructor_next_to_apply`):
    /// dotty names the getter of a synthesized `apply` after the constructor, selected on the
    /// call's receiver (`Applications.findDefaultGetter`), which for a companion nested in a class
    /// is the companion of the enclosing instance it is selected on (`module_prefix`).
    fn synthetic_apply_class(&mut self, of: DefaultOf, recv: Option<TExprId>) -> Option<(ClassId, Option<TExprId>)> {
        let DefaultOf::Method(m) = of else { return None };
        if self.syms.sym(m).name != crate::names::APPLY || !self.loaded.as_ref().is_some_and(|l| l.synthetic_members.contains_key(&m)) {
            return None;
        }
        let Owner::Class(o) = self.syms.sym(m).owner else { return None };
        let k = self.syms.class(o).companion?;
        if self.syms.class(o).inner_object.is_none() {
            return (self.syms.class(o).kind == ClassKind::Object).then_some((k, None));
        }
        let outer = self.module_prefix(recv?, o)?;
        Some((k, Some(outer)))
    }

    /// The local object beside the local class `c`, written or made for its defaults
    /// (`Desugar.classDef`), which its constructor's default getters are selected on: its lazy val.
    fn local_companion_ref(&mut self, c: ClassId) -> Option<TExprId> {
        if self.syms.class(c).owner != Owner::Local {
            return None;
        }
        let m = self.syms.class(c).companion?;
        let v = self.syms.class(m).local_module?;
        let r = self.prog.add(TExpr::Local(v));
        let ty = self.types.class(m, &[]);
        self.prog.set_type(r, ty);
        Some(r)
    }

    /// A path that means the same wherever it stands: an object, a top-level definition, a val
    /// selected on such a path.
    fn context_free_path(&self, e: TExprId) -> bool {
        match self.prog.expr(e) {
            TExpr::Module(_) | TExpr::Static(_) => true,
            TExpr::Field(r, _) => self.context_free_path(r),
            _ => false,
        }
    }

    /// A getter selected on `super` inside a trait goes through an accessor of its own, which
    /// the classes mixing the trait in implement (`SuperAccessors`, `ResolveSuper`).
    pub(super) fn note_super_getter(&mut self, of: DefaultOf, recv: Option<TExprId>, index: usize) {
        if let (DefaultOf::Method(m), Some(r)) = (of, recv) {
            if let TExpr::Super(crate::tir::SuperTarget::Mixin(t)) = self.prog.expr(r) {
                self.note_mixin_super_getter(t, m, index as u32);
            }
        }
    }

    /// A node of its own for a getter's argument, the call's being the call's: a reference or a
    /// literal made again, any other tree copied with fresh locals (`spliceMeth`'s
    /// `duplicateByNameArg`, a `TreeTypeMap`), so that each definition stands once.
    pub(super) fn prior_copy(&mut self, e: TExprId) -> TExprId {
        let ty = self.prog.type_of(e);
        let node = match self.prog.expr(e) {
            x @ (TExpr::Local(_) | TExpr::This | TExpr::Module(_) | TExpr::Static(_) | TExpr::Int(_) | TExpr::Long(_) | TExpr::Double(_) | TExpr::Bool(_) | TExpr::Char(_) | TExpr::Str(_) | TExpr::Unit | TExpr::Null) => x,
            _ => {
                let mut binders = Vec::new();
                self.tree_binders(e, &mut binders);
                let c = if binders.is_empty() {
                    self.copy_expr(e)
                } else {
                    let renames: crate::intern::FxMap<SymId, SymId> = binders.into_iter().map(|b| (b, self.clone_local(b))).collect();
                    let none = crate::intern::FxMap::default();
                    self.instantiate_quote(e, &none, &none, &Subst::default(), &renames)
                };
                if self.capturing() {
                    self.capture_unname(c);
                }
                return c;
            }
        };
        let id = self.prog.add(node);
        if let Some(t) = ty {
            self.prog.set_type(id, t);
        }
        self.prog.copy_span(e, id);
        id
    }
}
