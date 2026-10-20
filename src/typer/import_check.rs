//! An import's or an export's prefix, checked as `Checking.checkLegalImportOrExportPath` checks
//! the typed path: its stability (`checkStable`, `Types.isStable`), its realizability
//! (`checkRealizable`, `CheckRealizable.realizability`) and its purity (`isIdempotentExpr`,
//! `TreeInfo.exprPurity`), each reporting in that order.

use super::elaborate::Purity;
use super::Worker;
use crate::ast::mods;
use crate::intern::{FxMap, Name};
use crate::source::Span;
use crate::tir::TExprId;
use crate::symbols::*;
use super::members::MemberInfo;
use crate::types::*;

/// A path's realizability (`CheckRealizable.Realizability`): what makes a type inhabited by a
/// value whose type members have good bounds, or the first problem met.
#[derive(Clone, Debug)]
pub(super) enum Realizability {
    Realizable,
    NotConcrete,
    NotFinal(SymId),
    HasProblemBounds(Name, TypeId, TypeId),
    HasProblemBaseArg(TypeId, TypeId, TypeId),
    /// Two base types of one class the type has (an intersection's), `HasProblemBase`.
    HasProblemBase(TypeId, TypeId),
    HasProblemField(SymId, Box<Realizability>),
    /// The path, as a message shows it, and its underlying type's problem.
    ProblemInUnderlying(String, Box<Realizability>),
}

impl Realizability {
    fn ok(&self) -> bool {
        matches!(self, Realizability::Realizable)
    }

}

/// A step of an import's typed path: what `typedImportQualifier` types each segment as.
/// An import's qualifier typed as an expression (`Worker::import_head_step`): the tree and its
/// type, and for a bare reference the `TermRef`'s path (`steps`, shown as `shown`).
#[derive(Clone)]
pub(crate) struct ImportHead {
    pub te: TExprId,
    pub ty: TypeId,
    pub steps: Vec<PathStep>,
    pub shown: String,
}

/// The fields one realizability analysis has seen (`CheckRealizable.checkedFields`): one set
/// through the whole recursion of one `checkRealizable`, which a recursive field ends.
#[derive(Default)]
pub(super) struct Checked {
    fields: FxMap<SymId, ()>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum PathStep {
    /// An object's reference (`TermRef` of its module): its class.
    Object(ClassId),
    /// A term selected on the path before it, or named without one.
    Term(SymId),
    /// The enclosing instance a member is read on (`C.this`, the `ThisType` prefix of the
    /// `TermRef` dotty types the qualifier as).
    This(ClassId),
}

impl<'a> Worker<'a> {
    /// `Checking.checkLegalImportPath` where the traversal reaches an import statement
    /// (`Typer.typedImport`, in a template's, a block's or the file's statement order): the value
    /// path the clause resolved to, checked. The resolution itself is the namer's completer's
    /// (`Namer.Completer.typeSig`'s `Import` case types the qualifier when a lookup first goes
    /// through the import), which reports a path through a method or a var where it finds one.
    pub(super) fn check_import_target(&mut self, target: super::ImportTarget, path: &[Name], span: Span, word: &str) {
        // A library's or an upstream product's import comes from its pickle, typed when the
        // library was compiled: `TreeUnpickler` reads it, no `typedImport` checks it again.
        if self.in_jar(self.env.file) {
            return;
        }
        if word == "import" {
            self.check_language_import(target, path, span);
            self.check_import_selectors_at(target, span);
        }
        // A qualifier typed as an expression (`import_head_step`): the typed tree checked, a
        // bare reference as the `TermRef` it is.
        if let Some(h) = self.import_heads.get(&(self.env.file.0, span.start)).cloned() {
            if h.steps.is_empty() {
                self.check_legal_import_expr(h.te, h.ty, span, word);
            } else {
                self.check_legal_import_path(&h.steps, &h.shown, span, word);
            }
            return;
        }
        let v = match target {
            super::ImportTarget::ValueAll(v) | super::ImportTarget::ValueMember(v, _) | super::ImportTarget::ValueGivens(v) => v,
            _ => return,
        };
        let value = self.import_values[v.0 as usize];
        let steps = self.import_value_steps(value);
        // The path as its `TermRef` prints: the member's object or enclosing instance before
        // it (`Test.q`, `Use.this.l`), not the clause's words.
        let shown = steps.iter().map(|&st| self.step_shown(st)).collect::<Vec<_>>().join(".");
        let _ = path;
        self.check_legal_import_path(&steps, &shown, span, word);
    }

    /// `Checking.checkLegalImportPath`'s language part: a clause that looks like a language import
    /// (`TreeInfo.languageImport`, by its written path) must refer to `scala.language`, or to its
    /// `experimental` or `deprecated` object; one that does not look like one must not reach them
    /// through an alias (`path.tpe.classSymbols`).
    fn check_language_import(&mut self, target: super::ImportTarget, path: &[Name], span: Span) {
        use super::ImportTarget;
        let shape = match path {
            [crate::names::ROOT, crate::names::SCALA, rest @ ..] | [crate::names::SCALA, rest @ ..] => rest,
            rest => rest,
        };
        let looks_like = match shape {
            [crate::names::LANGUAGE] => Some(None),
            [crate::names::LANGUAGE, sub] if *sub == crate::names::EXPERIMENTAL || *sub == crate::names::DEPRECATED => Some(Some(*sub)),
            _ => None,
        };
        // The classes the path's type names (`path.tpe.classSymbols`): an object's class, or
        // those of a value's type, an intersection's united.
        let classes: Vec<ClassId> = match target {
            ImportTarget::ClassAll(c) | ImportTarget::ClassMember(c, _) | ImportTarget::ClassGivens(c) => vec![c],
            ImportTarget::ValueAll(v) | ImportTarget::ValueMember(v, _) | ImportTarget::ValueGivens(v) => {
                let t = self.import_value_type(v);
                let t = self.widen_type(t);
                self.class_symbols(t)
            }
            _ => Vec::new(),
        };
        match looks_like {
            Some(sub) => {
                let Some(required) = self.language_module(sub) else { return };
                // `path.symbol != required`: the path's own symbol, an object's class here.
                if matches!(target, ImportTarget::ClassAll(c) | ImportTarget::ClassMember(c, _) | ImportTarget::ClassGivens(c) if c == required) {
                    return;
                }
                let located = match target {
                    ImportTarget::ClassAll(c) | ImportTarget::ClassMember(c, _) | ImportTarget::ClassGivens(c) => {
                        format!("{}{}", self.class_description(c), self.located_in(self.syms.class(c).owner))
                    }
                    ImportTarget::PkgAll(p) | ImportTarget::PkgMember(p, _) | ImportTarget::PkgGivens(p) => format!("package {}", self.package_display(p)),
                    ImportTarget::ValueAll(v) | ImportTarget::ValueMember(v, _) | ImportTarget::ValueGivens(v) => {
                        let (s, _, _) = self.import_values[v.0 as usize];
                        format!("{}{}", self.sym_kind_shown(s), self.located_in(self.syms.sym(s).owner))
                    }
                    _ => return,
                };
                let msg = format!("import looks like a language import, but refers to something else: {}", located);
                self.error(span, msg);
            }
            None => {
                if classes.is_empty() {
                    return;
                }
                let language = self.language_module(None);
                let experimental = self.language_module(Some(crate::names::EXPERIMENTAL));
                if classes.iter().any(|&c| Some(c) == language || Some(c) == experimental) {
                    self.error(span, "no aliases can be used to refer to a language import".to_string());
                }
            }
        }
    }

    /// The standard library's `scala.language` object (`defn.LanguageModule`), or its
    /// `experimental` or `deprecated` object, where the library defines it.
    fn language_module(&mut self, sub: Option<Name>) -> Option<ClassId> {
        let scala = self.b.scala_pkg;
        let s = self.pkg_term(scala, crate::names::LANGUAGE)?.sym()?;
        let SymKind::Object(c) = self.syms.sym(s).kind else { return None };
        match sub {
            None => Some(c),
            Some(sub) => {
                let m = self.module_term(c, sub)?.sym()?;
                match self.syms.sym(m).kind {
                    SymKind::Object(k) => Some(k),
                    _ => None,
                }
            }
        }
    }

    /// `Checking.checkLegalImportOrExportPath` of the typed qualifier `te` of type `ty`, no
    /// bare reference: `checkStable` of its type, `checkRealizable` of it, `isIdempotentExpr` of
    /// the tree, each unless the type is erroneous.
    fn check_legal_import_expr(&mut self, te: TExprId, ty: TypeId, span: Span, word: &str) {
        let ty = self.zonk(ty);
        if self.is_erroneous(ty) {
            return;
        }
        let full = self.path_type_shown(ty);
        if !self.is_stable_type(ty) {
            let msg = format!("{} is not a valid {} prefix, since it is not an immutable path", full, word);
            self.error(span, msg);
        }
        let mut checked = Checked::default();
        let r = self.realizability_of(ty, &mut checked);
        if !r.ok() {
            let msg = format!("{} is not a legal path since it{}", full, self.realizability_msg(&r));
            self.error(span, msg);
        }
        if !self.expr_purity(te).at_least(Purity::IDEMPOTENT) {
            self.error(span, "import prefix is not a pure expression".to_string());
        }
    }

    /// A type as a message shows it: a path with its underlying type, `(p.x : T)`.
    fn path_type_shown(&mut self, ty: TypeId) -> String {
        match self.types.get(ty) {
            Type::Term(_) | Type::Select(..) => {
                let mut path = String::new();
                self.show_path(ty, &mut path);
                let under = self.widen_path(ty);
                format!("({} : {})", path, self.show(under))
            }
            _ => self.show(ty),
        }
    }

    /// `Checking.checkLegalImportOrExportPath` of the path `steps` (the last its qualifier's
    /// end), shown as `shown` where a message names it: each check reports, in dotty's order.
    /// Whether one reported.
    pub(super) fn check_legal_import_path(&mut self, steps: &[PathStep], shown: &str, span: Span, word: &str) -> bool {
        let Some(&last) = steps.last() else { return false };
        let prefix = &steps[..steps.len() - 1];
        let mut reported = false;
        let erroneous = self.path_erroneous(last);
        // A path whose type is erroneous reports nothing: `checkStable` and the purity check read
        // `isErroneous`, and `checkRealizable`'s message would name the error type, which the
        // reporter hides as nonsensical (`Message.isNonSensical`, `Reporter.isHidden`).
        if erroneous {
            return false;
        }
        // The path's type as dotty shows it: a `TermRef`'s `(p.x : T)`, an application's result.
        let full = match last {
            PathStep::Term(s) if self.applied_ret(s).is_some() => self.path_info_shown(last),
            _ => format!("({} : {})", shown, self.path_info_shown(last)),
        };
        // `checkStable`: `Types.isStable` of the path's type, unless erroneous.
        if !self.path_stable(prefix, last) && !erroneous {
            let msg = format!("{} is not a valid {} prefix, since it is not an immutable path", full, word);
            self.error(span, msg);
            reported = true;
        }
        // `checkRealizable`, before the typer is over: one analysis (`new CheckRealizable()`),
        // whose fields seen are one set through the recursion (`checkedFields`).
        let mut checked = Checked::default();
        let r = self.path_realizability(prefix, last, &full, &mut checked);
        if !r.ok() {
            let msg = format!("{} is not a legal path since it{}", full, self.realizability_msg(&r));
            self.error(span, msg);
            reported = true;
        }
        // `isIdempotentExpr`, unless the type is erroneous.
        if !self.path_purity(steps).at_least(Purity::IDEMPOTENT) && !erroneous {
            self.error(span, "import prefix is not a pure expression".to_string());
            reported = true;
        }
        reported
    }

    /// The result of a method whose clauses are all using clauses, which `typedImportQualifier`
    /// applies to its givens (`AnySelectionProto`): the path's type is the application's.
    fn applied_ret(&mut self, s: SymId) -> Option<TypeId> {
        if !matches!(self.syms.sym(s).kind, SymKind::Def | SymKind::Given) {
            return None;
        }
        let sig = self.sig_arc(s);
        (!sig.clauses.is_empty() && sig.clauses.iter().all(|c| c.is_using)).then_some(sig.ret)
    }

    /// The type a path's step shows in a message: a parameterless method's `=> T`.
    fn path_info_shown(&mut self, step: PathStep) -> String {
        match step {
            PathStep::Term(s) if self.applied_ret(s).is_some() => {
                let r = self.applied_ret(s).unwrap_or(ERROR);
                self.show(r)
            }
            PathStep::Term(s) => {
                let parameterless_def = self.syms.sym(s).kind == SymKind::Def && self.sig_of(s).clauses.is_empty();
                let ty = self.sig_of(s).ret;
                format!("{}{}", if parameterless_def { "=> " } else { "" }, self.show(ty))
            }
            PathStep::Object(c) => self.show(self.types.class(c, &[])),
            PathStep::This(c) => {
                let t = self.types.mk(Type::This(c));
                self.show(t)
            }
        }
    }

    /// `Types.isErroneous` of the step's type: widened, any part an error.
    fn path_erroneous(&mut self, step: PathStep) -> bool {
        match step {
            PathStep::Term(s) => {
                let t = self.sig_of(s).ret;
                self.is_erroneous(t)
            }
            _ => false,
        }
    }

    /// `Types.isStable` of the `TermRef` the step is on the path `prefix`.
    fn path_stable(&mut self, prefix: &[PathStep], step: PathStep) -> bool {
        match step {
            PathStep::Object(_) | PathStep::This(_) => true,
            PathStep::Term(s) if self.applied_ret(s).is_some() => {
                let r = self.applied_ret(s).unwrap_or(ERROR);
                self.is_stable_type(r)
            }
            PathStep::Term(s) => {
                let prefix_stable = match prefix.split_last() {
                    Some((&p, rest)) => self.path_stable(rest, p),
                    None => true,
                };
                (self.is_stable_member(s) && prefix_stable) || self.info_stable(s)
            }
        }
    }

    /// Whether the info of `s` is stable (`TermRef.info.isStable`): a val's singleton type, a
    /// parameterless method's `ExprType` of one.
    fn info_stable(&mut self, s: SymId) -> bool {
        let sig = self.sig_arc(s);
        if !sig.clauses.is_empty() || !sig.tparams.is_empty() {
            return false;
        }
        self.is_stable_type(sig.ret)
    }

    /// `SymDenotation.isStableMember`: a type, a stable forwarder (`StableRealizable`), or a term
    /// neither mutable, a method, by-name (`ExprType`) nor an inline parameter.
    pub(super) fn is_stable_member(&mut self, s: SymId) -> bool {
        let info = self.syms.sym(s);
        if info.stable || self.stable_realizable.contains_key(&s) {
            return true;
        }
        let (kind, mods, by_name) = (info.kind, info.mods, info.by_name);
        let mutable = kind == SymKind::Var || mods & mods::MUTABLE != 0;
        let method = match kind {
            SymKind::Def | SymKind::Overloaded(_) => true,
            SymKind::Given => self.is_method_sym(s),
            _ => false,
        };
        let inline_param = kind == SymKind::Param && mods & mods::INLINE != 0;
        !mutable && !method && !by_name && !inline_param
    }

    /// `Types.isStable` over teq's types (`stripTypeVar` first): a path's, a singleton's, a
    /// refinement's or recursive type's parent's, an annotation's parent's, an intersection's
    /// where one side is stable and the other realizable, an applied compiletime operation's of
    /// stable arguments (`AppliedType.computeIsStable`).
    pub(super) fn is_stable_type(&mut self, t: TypeId) -> bool {
        let t = self.deref(t);
        match self.types.get(t) {
            Type::Term(s) => self.is_stable_member(s) || self.info_stable(s),
            Type::Select(p, s) => (self.is_stable_member(s) && self.is_stable_type(p)) || self.info_stable(s),
            Type::This(_) | Type::Lit(_) | Type::RecThis(_) => true,
            Type::Class(c, args) if self.types.items(args).is_empty() && self.syms.class(c).kind == ClassKind::Object => true,
            Type::Refined(p, _) | Type::Rec(p) | Type::Annotated(p) => self.is_stable_type(p),
            Type::Inter(a, b) => {
                let mut checked = Checked::default();
                (self.is_stable_type(a) && self.realizability_of(b, &mut checked).ok()) || (self.is_stable_type(b) && self.realizability_of(a, &mut checked).ok())
            }
            // `AppliedType.computeIsStable`: an application of one of `scala.compiletime.ops`'
            // operation types, `S` or `NamedTuple.From` (`isCompiletimeAppliedType`), of stable
            // arguments.
            Type::Alias(_, args) | Type::AppMember(_, args) | Type::Class(_, args) if self.compiletime_applied(t) => {
                let items = self.types.items(args).to_vec();
                items.into_iter().all(|a| self.is_stable_type(a))
            }
            _ => false,
        }
    }

    /// Whether `t` is an application of a compiletime operation type (`Definitions
    /// .isCompiletimeAppliedType`): `scala.compiletime.ops.S`, `scala.NamedTuple.From`, or a type
    /// of the `ops` objects (`any`, `int`, `long`, `float`, `double`, `boolean`, `string`) among
    /// the names each declares (`compiletimePackage*Types`), whichever record teq's std gives it.
    fn compiletime_applied(&self, t: TypeId) -> bool {
        let (name, owner) = match self.types.get(t) {
            Type::Alias(a, args) if !self.types.items(args).is_empty() => (self.syms.alias(a).name, self.syms.alias(a).owner),
            Type::AppMember(m, args) if !self.types.items(args).is_empty() => match self.types.get(m) {
                Type::Member(p, n) => match self.types.get(p) {
                    Type::Class(c, _) | Type::This(c) => (n, Owner::Class(c)),
                    _ => return false,
                },
                Type::Decl(a) => (self.syms.alias(a).name, self.syms.alias(a).owner),
                _ => return false,
            },
            Type::Class(c, args) if !self.types.items(args).is_empty() => (self.syms.class(c).name, self.syms.class(c).owner),
            _ => return false,
        };
        let text = self.name_ref(name);
        let owner_path = match owner {
            Owner::Class(c) => self.class_path(c),
            Owner::Package(p) => self.pkg_path(p),
            Owner::Local => return false,
        };
        const NUMERIC: &[&str] = &["+", "-", "*", "/", "%", "<", ">", ">=", "<=", "Abs", "Negate", "Min", "Max"];
        const ANY_TYPES: &[&str] = &["==", "!=", "IsConst", "ToString"];
        const INT: &[&str] = &["ToString", "NumberOfLeadingZeros", "ToLong", "ToFloat", "ToDouble", "^", "BitwiseAnd", "BitwiseOr", ">>", "<<", ">>>"];
        const LONG: &[&str] = &["NumberOfLeadingZeros", "ToInt", "ToFloat", "ToDouble", "^", "BitwiseAnd", "BitwiseOr", ">>", "<<", ">>>"];
        const FLOAT: &[&str] = &["ToInt", "ToLong", "ToDouble"];
        const DOUBLE: &[&str] = &["ToInt", "ToLong", "ToFloat"];
        const BOOLEAN: &[&str] = &["!", "^", "&&", "||"];
        const STRING: &[&str] = &["+", "Length", "Substring", "Matches", "CharAt"];
        match owner_path.as_str() {
            "scala.compiletime.ops" => text == "S",
            "scala.NamedTuple" => text == "From",
            "scala.compiletime.ops.any" => ANY_TYPES.contains(&text),
            "scala.compiletime.ops.int" => NUMERIC.contains(&text) || INT.contains(&text),
            "scala.compiletime.ops.long" => NUMERIC.contains(&text) || LONG.contains(&text),
            "scala.compiletime.ops.float" => NUMERIC.contains(&text) || FLOAT.contains(&text),
            "scala.compiletime.ops.double" => NUMERIC.contains(&text) || DOUBLE.contains(&text),
            "scala.compiletime.ops.boolean" => BOOLEAN.contains(&text),
            "scala.compiletime.ops.string" => STRING.contains(&text),
            _ => false,
        }
    }

    /// `Types.isErroneous`: the type widened (`widen_type`), then any part an error, a variable
    /// read through its instance and a lambda's parameter bounds included (`ExistsAccumulator`).
    pub(super) fn is_erroneous(&mut self, t: TypeId) -> bool {
        let t = self.zonk(t);
        let w = self.widen_type(t);
        self.types.contains_error(w) || self.lambda_bounds_erroneous(w)
    }

    /// `ExistsAccumulator` over a type lambda's parameter infos, which the parts the type store
    /// walks do not hold (a parameter's bounds are its record's).
    fn lambda_bounds_erroneous(&mut self, t: TypeId) -> bool {
        match self.types.get(t) {
            Type::Lambda(ps, body) => {
                let params: Vec<TypeId> = self.types.items(ps).to_vec();
                for p in params {
                    if let Type::Param(q) = self.types.get(p) {
                        let (lo, hi) = (self.syms.tparam(q).lower, self.syms.tparam(q).upper);
                        if self.types.contains_error(lo) || self.types.contains_error(hi) || self.lambda_bounds_erroneous(lo) || self.lambda_bounds_erroneous(hi) {
                            return true;
                        }
                    }
                }
                self.lambda_bounds_erroneous(body)
            }
            Type::Class(_, args) | Type::Alias(_, args) | Type::AppParam(_, args) | Type::AppVar(_, args) | Type::AppMember(_, args) => {
                let items: Vec<TypeId> = self.types.items(args).to_vec();
                items.into_iter().any(|a| self.lambda_bounds_erroneous(a))
            }
            Type::Union(a, b, _) | Type::Inter(a, b) | Type::BoundedWild(a, b) | Type::Nested(a, b) => self.lambda_bounds_erroneous(a) || self.lambda_bounds_erroneous(b),
            Type::Refined(p, _) | Type::Rec(p) | Type::Annotated(p) | Type::Select(p, _) | Type::Member(p, _) => self.lambda_bounds_erroneous(p),
            _ => false,
        }
    }

    /// `Types.classSymbols` (`parentSymbols(_.isClass)`): a class's type is its class; a proxy
    /// (an alias, an abstract type, a parameter, a singleton, a refinement) its underlying type's
    /// (`superType`); an intersection's are both sides' united, a union's both sides' intersected.
    pub(super) fn class_symbols(&mut self, t: TypeId) -> Vec<ClassId> {
        let t = self.deref(t);
        match self.types.get(t) {
            Type::Class(c, _) | Type::Ctor(c) => vec![c],
            Type::Nested(_, class) => self.class_symbols(class),
            Type::Inter(a, b) => {
                let mut out = self.class_symbols(a);
                for c in self.class_symbols(b) {
                    if !out.contains(&c) {
                        out.push(c);
                    }
                }
                out
            }
            Type::Union(a, b, _) => {
                let right = self.class_symbols(b);
                self.class_symbols(a).into_iter().filter(|c| right.contains(c)).collect()
            }
            Type::Alias(..) | Type::Match(..) | Type::Term(_) | Type::Select(..) | Type::This(_) | Type::Lit(_) | Type::Member(..) | Type::AppMember(..) | Type::Decl(_) | Type::Param(_) | Type::AppParam(..) | Type::Refined(..) | Type::Rec(_) | Type::Annotated(_) | Type::RecThis(_) => {
                match self.dependent_underlying(t) {
                    Some(u) if u != t => self.class_symbols(u),
                    _ => match self.types.get(t) {
                        Type::Refined(p, _) | Type::Rec(p) | Type::Annotated(p) => self.class_symbols(p),
                        Type::Param(p) => {
                            let hi = self.syms.tparam(p).upper;
                            if hi == ANY { Vec::new() } else { self.class_symbols(hi) }
                        }
                        _ => Vec::new(),
                    },
                }
            }
            _ => Vec::new(),
        }
    }

    /// `CheckRealizable.realizability` of the `TermRef` the step is on the path `prefix`, within
    /// the analysis `checked` belongs to.
    fn path_realizability(&mut self, prefix: &[PathStep], step: PathStep, shown: &str, checked: &mut Checked) -> Realizability {
        match step {
            PathStep::Object(_) | PathStep::This(_) => Realizability::Realizable,
            PathStep::Term(s) if self.applied_ret(s).is_some() => {
                let r = self.applied_ret(s).unwrap_or(ERROR);
                self.realizability_of(r, checked)
            }
            PathStep::Term(s) => self.term_realizability(s, prefix, shown, checked),
        }
    }

    /// `realizability(tp.prefix)` of the path before a step.
    fn prefix_realizability(&mut self, prefix: &[PathStep], checked: &mut Checked) -> Realizability {
        match prefix.split_last() {
            Some((&p, rest)) => {
                let shown = format!("({} : {})", self.step_shown(p), self.path_info_shown(p));
                self.path_realizability(rest, p, &shown, checked)
            }
            None => Realizability::Realizable,
        }
    }

    /// The `TermRef` case: a stable, realizable member's prefix (`StableRealizable`); a stable
    /// member not late-initialized; one not effectively final; otherwise its info's. On success
    /// the member is stable and realizable from then on; a failure is recovered where its info
    /// is stable and realizable.
    fn term_realizability(&mut self, s: SymId, prefix: &[PathStep], shown: &str, checked: &mut Checked) -> Realizability {
        if self.syms.sym(s).stable || self.stable_realizable.contains_key(&s) {
            return self.prefix_realizability(prefix, checked);
        }
        let stable_member = self.is_stable_member(s);
        let info = self.sig_of(s).ret;
        let r = if stable_member && !self.is_late_initialized(s) {
            Realizability::Realizable
        } else if !self.is_effectively_final(s) {
            Realizability::NotFinal(s)
        } else {
            match self.realizability_of(info, checked) {
                Realizability::Realizable => Realizability::Realizable,
                r => Realizability::ProblemInUnderlying(shown.to_string(), Box::new(r)),
            }
        };
        let r = if r.ok() {
            if stable_member {
                self.stable_realizable.insert(s, ());
            }
            self.prefix_realizability(prefix, checked)
        } else {
            r
        };
        if r.ok() {
            return r;
        }
        // A mutable path is stable and realizable after all where its type is a realizable
        // singleton.
        if self.info_stable(s) && self.realizability_of(info, checked).ok() {
            self.stable_realizable.insert(s, ());
            return Realizability::Realizable;
        }
        r
    }

    /// `isLateInitialized`: a lazy val, not an object.
    fn is_late_initialized(&self, s: SymId) -> bool {
        let info = self.syms.sym(s);
        info.mods & mods::LAZY != 0 && !matches!(info.kind, SymKind::Object(_))
    }

    /// `SymDenotation.isEffectivelyFinal`: final or private, an inline member not deferred, a Java
    /// value that is no method, a constructor, or one not deferred whose owner is not an
    /// extensible class.
    fn is_effectively_final(&self, s: SymId) -> bool {
        let info = self.syms.sym(s);
        let deferred = info.mods & (mods::DEFERRED | mods::ABSTRACT) != 0;
        if info.mods & (mods::FINAL | mods::PRIVATE) != 0 {
            return true;
        }
        if info.mods & mods::INLINE != 0 && !deferred {
            return true;
        }
        if info.java_defined && info.kind != SymKind::Def {
            return true;
        }
        if info.name == crate::names::INIT {
            return true;
        }
        let extensible = match info.owner {
            Owner::Class(c) => self.is_extensible_class(c),
            _ => false,
        };
        !deferred && !extensible
    }

    /// `isExtensibleClass`: a class, neither final, an object nor anonymous.
    fn is_extensible_class(&self, c: ClassId) -> bool {
        let k = self.syms.class(c);
        !matches!(k.kind, ClassKind::Object | ClassKind::Anon | ClassKind::GivenImpl | ClassKind::EnumCase) && k.mods & mods::FINAL == 0
    }

    /// `realizability` of a type (`dealias` first): a path's `TermRef` with its prefix; a
    /// singleton is realizable; any other type must be concrete, then have good bounds, then
    /// good members, within the one analysis `checked` belongs to.
    pub(super) fn realizability_of(&mut self, t: TypeId, checked: &mut Checked) -> Realizability {
        let t = self.dealias_realizable(t);
        match self.types.get(t) {
            Type::Term(s) => {
                let shown = self.show(t);
                let steps = self.path_steps_of(t);
                let (last, prefix) = steps.split_last().map(|(l, p)| (*l, p.to_vec())).unwrap_or((PathStep::Term(s), Vec::new()));
                match last {
                    PathStep::Term(x) => self.term_realizability(x, &prefix, &shown, checked),
                    PathStep::Object(_) | PathStep::This(_) => Realizability::Realizable,
                }
            }
            Type::Select(_, s) => {
                let shown = self.show(t);
                let steps = self.path_steps_of(t);
                let prefix: Vec<PathStep> = steps[..steps.len().saturating_sub(1)].to_vec();
                self.term_realizability(s, &prefix, &shown, checked)
            }
            Type::This(_) | Type::Lit(_) | Type::RecThis(_) => Realizability::Realizable,
            // An object's type is its module's path (a `TermRef`, a `SingletonType`).
            Type::Class(c, args) if self.types.items(args).is_empty() && self.syms.class(c).kind == ClassKind::Object => Realizability::Realizable,
            _ => {
                if !self.is_concrete(t) {
                    return Realizability::NotConcrete;
                }
                let bounds = self.bounds_realizability(t);
                if !bounds.ok() {
                    return bounds;
                }
                self.member_realizability(t, checked)
            }
        }
    }

    /// The steps of the path type `t` (a `TermRef`'s prefixes): an object's member on its object,
    /// a member on the path before it, a top-level or local val alone.
    fn path_steps_of(&mut self, t: TypeId) -> Vec<PathStep> {
        let t = self.deref(t);
        match self.types.get(t) {
            Type::Term(s) => match self.syms.sym(s).owner {
                Owner::Class(c) if self.syms.class(c).kind == ClassKind::Object => vec![PathStep::Object(c), PathStep::Term(s)],
                _ => vec![PathStep::Term(s)],
            },
            Type::Select(p, s) => {
                let mut steps = self.path_steps_of(p);
                steps.push(PathStep::Term(s));
                steps
            }
            Type::Class(c, args) if self.types.items(args).is_empty() && self.syms.class(c).kind == ClassKind::Object => vec![PathStep::Object(c)],
            Type::This(c) if self.syms.class(c).kind == ClassKind::Object => vec![PathStep::Object(c)],
            Type::This(c) => vec![PathStep::This(c)],
            _ => Vec::new(),
        }
    }

    /// `dealias`: an alias's expansion, a variable's instance.
    fn dealias_realizable(&mut self, t: TypeId) -> TypeId {
        let mut t = self.deref(t);
        while let Type::Alias(a, args) = self.types.get(t) {
            match self.alias_expansion(a, args) {
                Some(e) if e != t => t = self.deref(e),
                _ => break,
            }
        }
        t
    }

    /// `MatchTypes.isConcrete` of the type normalized (a match type reduced where it reduces): a
    /// class's type; an alias's expansion; an applied type whose constructor and upper bound are;
    /// a type lambda; a non-parameter term's underlying type; a proxy's underlying type; both
    /// sides of a union or an intersection. Not a type parameter, an abstract member, an
    /// unsolved variable or a match type that does not reduce.
    pub(super) fn is_concrete(&mut self, t: TypeId) -> bool {
        let t = self.deref(t);
        let t = self.normalize(t);
        let t = self.deref(t);
        match self.types.get(t) {
            Type::Any | Type::Nothing | Type::Class(..) | Type::Ctor(_) | Type::Nested(..) => true,
            Type::Alias(a, args) => match self.alias_expansion(a, args) {
                Some(e) if e != t => self.is_concrete(e),
                _ => false,
            },
            Type::Lambda(..) | Type::Poly(..) => true,
            Type::Term(s) | Type::Select(_, s) => {
                if self.syms.sym(s).kind == SymKind::Param {
                    return false;
                }
                let u = self.sig_of(s).ret;
                self.is_concrete(u)
            }
            Type::This(_) | Type::Lit(_) | Type::RecThis(_) => true,
            Type::Param(_) | Type::AppParam(..) | Type::Var(_) | Type::AppVar(..) | Type::Match(..) | Type::Decl(_) | Type::Member(..) | Type::AppMember(..) => false,
            Type::Annotated(p) | Type::Refined(p, _) | Type::Rec(p) => self.is_concrete(p),
            Type::Union(a, b, _) | Type::Inter(a, b) => self.is_concrete(a) && self.is_concrete(b),
            _ => false,
        }
    }

    /// `boundsRealizability`: every non-class type member of the type, as seen from it
    /// (`nonClassTypeMembers`: the names its classes and their bases declare, each member's info
    /// `tp.member(name)` on the type, an intersection's merged), with `lo <: hi`; every
    /// type-named refinement (`refinedNames`) likewise; then every base type of the type
    /// (`baseClasses.map(_.baseTypeOf(tp))`): one an intersection gives twice, differently, is a
    /// conflict (`HasProblemBase`), else each wildcard argument has `lo <: hi`. The first problem
    /// wins.
    fn bounds_realizability(&mut self, t: TypeId) -> Realizability {
        let classes = self.class_symbols(t);
        let mut bases: Vec<ClassId> = Vec::new();
        for &c in &classes {
            self.complete_class(c);
            for i in 0..self.syms.class(c).base_types.len() {
                let b = self.syms.class(c).base_types[i].0;
                if !bases.contains(&b) {
                    bases.push(b);
                }
            }
        }
        // A class's type parameters are its abstract type members too (`nonClassTypeMembers`):
        // `One[A]`'s `A` seen from `One[String] & One[Int]` is the two sides' aliases met
        // (`TypeAlias(String) & TypeAlias(Int)`, `TypeBounds.&`: `>: String | Int <: String & Int`),
        // a problem where the bounds conflict; one side's alias is its own bounds.
        for &b in &bases {
            self.complete_class(b);
            if self.syms.class(b).tparams.is_empty() {
                continue;
            }
            let (first, second) = self.base_type_sides(t, b);
            let (Some(x), Some(y)) = (first, second) else { continue };
            let (Type::Class(_, xa), Type::Class(_, ya)) = (self.types.get(x), self.types.get(y)) else { continue };
            let (xs, ys) = (self.types.items(xa).to_vec(), self.types.items(ya).to_vec());
            for i in 0..self.syms.class(b).tparams.len().min(xs.len()).min(ys.len()) {
                let (ax, ay) = (self.zonk(xs[i]), self.zonk(ys[i]));
                if ax == ay {
                    continue;
                }
                let mark = self.snapshot();
                let within = self.is_sub(ax, ay) && self.is_sub(ay, ax);
                self.rollback(mark);
                if within {
                    continue;
                }
                let (lo, hi) = (self.types.union(ax, ay), self.types.inter(ax, ay));
                if !self.is_sub(lo, hi) {
                    let p = self.syms.class(b).tparams[i];
                    return Realizability::HasProblemBounds(self.syms.tparam(p).name, lo, hi);
                }
            }
        }
        let mut names: Vec<Name> = Vec::new();
        for &b in &bases {
            self.complete_class(b);
            let mut own: Vec<(crate::types::AliasId, Name)> = self.syms.class(b).type_aliases.iter().map(|(&n, &a)| (a, n)).collect();
            own.sort_by_key(|&(a, _)| a.0);
            for (_, n) in own {
                if !names.contains(&n) {
                    names.push(n);
                }
            }
        }
        for name in names {
            if let Some((lo, hi)) = self.type_member_bounds_on(t, name) {
                if !self.is_sub(lo, hi) {
                    return Realizability::HasProblemBounds(name, lo, hi);
                }
            }
        }
        // The refinements' type names, each member's merged info on the type.
        let mut refined: Vec<Name> = Vec::new();
        self.refined_names(t, &mut refined);
        for name in refined {
            if let Some((lo, hi)) = self.type_member_bounds_on(t, name) {
                if !self.is_sub(lo, hi) {
                    return Realizability::HasProblemBounds(name, lo, hi);
                }
            }
        }
        // The base types as seen from the type.
        for b in bases {
            let (first, second) = self.base_type_sides(t, b);
            if let (Some(x), Some(y)) = (first, second) {
                return Realizability::HasProblemBase(x, y);
            }
            let Some(bt) = first else { continue };
            if let Type::Class(_, args) = self.types.get(bt) {
                for &a in self.types.items(args).to_vec().iter() {
                    if let Type::BoundedWild(lo, hi) = self.types.get(a) {
                        if !self.is_sub(lo, hi) {
                            return Realizability::HasProblemBaseArg(bt, lo, hi);
                        }
                    }
                }
            }
        }
        Realizability::Realizable
    }

    /// `tp.member(name)` for a type name on the type `t`: the member's bounds as seen from `t`, an
    /// alias's its own type, an intersection's merged (`member_in_type`'s `bounds_meet`).
    fn type_member_bounds_on(&mut self, t: TypeId, name: Name) -> Option<(TypeId, TypeId)> {
        match self.type_member(t, name)? {
            MemberInfo::Bounds(lo, hi) => Some((lo, hi)),
            MemberInfo::Alias(a) => Some((a, a)),
        }
    }

    /// `baseTypeOf(tp)` for the class `k`: an intersection whose sides each have one, differing,
    /// gives both (dotty's `AndType` base); else the one base type, as seen from `t`.
    fn base_type_sides(&mut self, t: TypeId, k: ClassId) -> (Option<TypeId>, Option<TypeId>) {
        let t = self.deref(t);
        match self.types.get(t) {
            Type::Inter(a, b) => {
                let (x, y) = (self.base_type_sides(a, k), self.base_type_sides(b, k));
                if let (Some(_), Some(_)) = x {
                    return x;
                }
                if let (Some(_), Some(_)) = y {
                    return y;
                }
                match (x.0, y.0) {
                    (Some(p), Some(q)) => {
                        let (p, q) = (self.zonk(p), self.zonk(q));
                        if p == q { (Some(p), None) } else { (Some(p), Some(q)) }
                    }
                    (Some(p), None) | (None, Some(p)) => (Some(p), None),
                    (None, None) => (None, None),
                }
            }
            Type::Refined(p, _) | Type::Rec(p) | Type::Annotated(p) => self.base_type_sides(p, k),
            _ => (self.base_type(t, k), None),
        }
    }

    /// `refinedNames`: the names the type's refinements bind, through refinements' parents, both
    /// sides of unions and intersections, aliases' expansions and proxies' underlying types.
    fn refined_names(&mut self, t: TypeId, out: &mut Vec<Name>) {
        let t = self.dealias_realizable(t);
        match self.types.get(t) {
            Type::Refined(p, r) => {
                match self.types.refinement(r) {
                    crate::types::Refinement::Bounds(n, ..) | crate::types::Refinement::Alias(n, _) => {
                        if !out.contains(&n) {
                            out.push(n);
                        }
                    }
                    _ => {}
                }
                self.refined_names(p, out);
            }
            Type::Union(a, b, _) | Type::Inter(a, b) => {
                self.refined_names(a, out);
                self.refined_names(b, out);
            }
            Type::Annotated(p) | Type::Rec(p) => self.refined_names(p, out),
            Type::Term(_) | Type::Select(..) | Type::Member(..) | Type::AppMember(..) | Type::Decl(_) | Type::Param(_) | Type::AppParam(..) => {
                if let Some(u) = self.dependent_underlying(t) {
                    if u != t {
                        self.refined_names(u, out);
                    }
                }
            }
            _ => {}
        }
    }

    /// `memberRealizability`: from `-source future` on, each field of the type (`tp.fields`: the
    /// members of its classes and their bases that are no methods, their infos as seen from the
    /// type) that is neither private, mutable nor lazy, not seen yet in this analysis
    /// (`checkedFields`), has a realizable type.
    fn member_realizability(&mut self, t: TypeId, checked: &mut Checked) -> Realizability {
        if !self.source_future() {
            return Realizability::Realizable;
        }
        let classes = self.class_symbols(t);
        let mut bases: Vec<ClassId> = Vec::new();
        for &c in &classes {
            self.complete_class(c);
            for i in 0..self.syms.class(c).base_types.len() {
                let b = self.syms.class(c).base_types[i].0;
                if !bases.contains(&b) {
                    bases.push(b);
                }
            }
        }
        for b in bases {
            self.complete_class(b);
            let (seen_from, _) = self.base_type_sides(t, b);
            let subst = match seen_from {
                Some(bt) => self.owner_subst(bt),
                None => Vec::new(),
            };
            let fields: Vec<SymId> = self.syms.class(b).member_order.clone();
            for f in fields {
                let info = self.syms.sym(f);
                if !matches!(info.kind, SymKind::Val | SymKind::Var) || info.mods & (mods::PRIVATE | mods::MUTABLE | mods::LAZY) != 0 || info.kind == SymKind::Var {
                    continue;
                }
                if checked.fields.insert(f, ()).is_some() {
                    continue;
                }
                let ft = self.sig_of(f).ret;
                let ft = self.types.subst(ft, &subst);
                let ft = self.as_seen_from(ft, t, b);
                let r = self.realizability_of(ft, checked);
                if !r.ok() {
                    return Realizability::HasProblemField(f, Box::new(r));
                }
            }
        }
        Realizability::Realizable
    }

    /// Whether the unit is typed at `-source future`, from which fields count for a path's
    /// realizability (`memberRealizability`).
    fn source_future(&self) -> bool {
        self.cur_ast().source_version.is_at_least(crate::source_version::SourceVersion::FUTURE)
    }

    /// The message of a realizability problem after "since it" (`Realizability.msg`).
    pub(super) fn realizability_msg(&mut self, r: &Realizability) -> String {
        match r {
            Realizability::Realizable => String::new(),
            Realizability::NotConcrete => " is not a concrete type".to_string(),
            Realizability::NotFinal(s) => format!(" refers to nonfinal {}", self.sym_kind_shown(*s)),
            Realizability::HasProblemBounds(n, lo, hi) => {
                format!(" has a member {} with possibly conflicting bounds {} <: ... <: {}", self.name_str(*n), self.show(*lo), self.show(*hi))
            }
            Realizability::HasProblemBase(a, b) => format!(" has conflicting base types {} and {}", self.show(*a), self.show(*b)),
            Realizability::HasProblemBaseArg(bt, lo, hi) => {
                format!(" has a base type {} with possibly conflicting parameter bounds {} <: ... <: {}", self.show(*bt), self.show(*lo), self.show(*hi))
            }
            Realizability::HasProblemField(f, inner) => {
                let name = self.name_str(self.syms.sym(*f).name).to_string();
                let ty = self.sig_of(*f).ret;
                let inner = self.realizability_msg(inner);
                format!(" has a member {} which is not a legal path\nsince {}: {}{}", name, name, self.show(ty), inner)
            }
            Realizability::ProblemInUnderlying(shown, inner) => {
                let inner = self.realizability_msg(inner);
                format!("s underlying type {}{}", shown, inner)
            }
        }
    }

    /// A path's step as a message names it.
    pub(super) fn step_shown(&self, step: PathStep) -> String {
        match step {
            PathStep::Term(s) => self.name_str(self.syms.sym(s).name).to_string(),
            PathStep::Object(c) => self.name_str(self.syms.class(c).name).to_string(),
            PathStep::This(c) => format!("{}.this", self.name_str(self.syms.class(c).name)),
        }
    }

    /// A symbol as a message names it: `lazy value lz`, `value v`, `method m`.
    fn sym_kind_shown(&self, s: SymId) -> String {
        let info = self.syms.sym(s);
        let what = match info.kind {
            SymKind::Val if info.mods & mods::LAZY != 0 => "lazy value",
            SymKind::Val | SymKind::Param => "value",
            SymKind::Var => "variable",
            SymKind::Given => "given instance",
            _ => "method",
        };
        format!("{} {}", what, self.name_str(info.name))
    }

    /// `TreeInfo.exprPurity` of the typed path (`Ident`/`Select` steps, `This`): the least of each
    /// reference's purity (`refPurity`) and its qualifier's.
    fn path_purity(&mut self, steps: &[PathStep]) -> Purity {
        let mut p = Purity::PURE_PATH;
        for &step in steps {
            let r = match step {
                PathStep::Object(c) => self.module_purity_of(c),
                // `This(_)` is a pure path (`exprPurity`).
                PathStep::This(_) => Purity::PURE_PATH,
                PathStep::Term(s) => self.import_ref_purity(s),
            };
            p = p.min(r);
        }
        p
    }

    /// `refPurity` of a path's reference to `s`, as `typedImportQualifier` types it: a method that
    /// still takes parameters is a path; one applied to its using clause is its application's
    /// purity (`exprPurity` of an `Apply`, impure but where `isPureApply` holds); a member that is
    /// not stable is impure; an object's by its initialisation; a lazy val idempotent; an inline
    /// parameter impure.
    fn import_ref_purity(&mut self, s: SymId) -> Purity {
        let info = self.syms.sym(s);
        let (kind, mods) = (info.kind, info.mods);
        if kind == SymKind::Def || (kind == SymKind::Given && self.is_method_sym(s)) {
            let sig = self.sig_arc(s);
            let takes_terms = sig.clauses.iter().any(|c| !c.is_using);
            if takes_terms {
                return Purity::PURE_PATH;
            }
            // Parameterless, or applied to its givens: a stable member's reference or impure.
            return if self.is_stable_member(s) { Purity::PURE_PATH } else { Purity::IMPURE };
        }
        if !self.is_stable_member(s) {
            return Purity::IMPURE;
        }
        if let SymKind::Object(c) = kind {
            return self.module_purity_of(c);
        }
        if mods & mods::LAZY != 0 || kind == SymKind::Given {
            return Purity::IDEMPOTENT_PATH;
        }
        if kind == SymKind::Param && mods & mods::INLINE != 0 {
            return Purity::IMPURE;
        }
        Purity::PURE_PATH
    }

    /// An object's reference: a path where its class is a no-inits real class, idempotent
    /// otherwise (`refPurity`, `isNoInitsRealClass`).
    fn module_purity_of(&mut self, c: ClassId) -> Purity {
        if self.no_inits_real_class(c) {
            Purity::PURE_PATH
        } else {
            Purity::IDEMPOTENT_PATH
        }
    }
}
