use super::apply::{ArgList, ArgSrc, ForBinder, ForRest};
use super::site::SiteOwner;
use super::{Frame, Worker};
use crate::ast::{self, mods, DefKind, Enumerator, Expr, ExprId, ListRef, Pat, Stmt, TyExpr, TyExprId};
use crate::intern::Name;
use crate::names;
use crate::source::{FileId, Span};
use crate::symbols::*;
use crate::tir::*;
use crate::tir::capture::{Form, Wrap};
use crate::types::*;
use std::sync::Arc;

/// What a lambda's body is to the partial function it is expected to be (`partial_body`).
#[derive(Clone, Copy, PartialEq)]
enum PartialBody {
    Match,
    /// The match that ends a block of statements, and the block (the match itself for a
    /// lambda of a tuple's elements, whose block is its bindings).
    BlockMatch(ExprId, ExprId),
    Other,
}

/// What a block's definitions are, for whether the value of the match that ends it names one
/// (`Worker::result_leaks`): the lambda at `outer`, the match at `inner`, the lambda's own
/// parameters `own` (the closure's, not the block's), and the names the block defines.
struct BlockLocals {
    outer: Span,
    inner: Span,
    own: Vec<SymId>,
    names: Vec<Name>,
}

/// The template a body withheld from its products is typed as.
pub const WITHHELD_TEMPLATE: &str = "$withheld";

impl<'a> Worker<'a> {
    pub fn push_scope(&mut self) {
        self.env.frames.push(Frame::locals());
    }

    pub fn pop_scope(&mut self) {
        self.env.frames.pop();
    }

    pub fn bind_local(&mut self, name: Name, sym: SymId) {
        if let Some(Frame::Locals { names, .. }) = self.env.frames.last_mut() {
            names.push((name, sym));
        }
    }

    pub fn bind_given(&mut self, sym: SymId) {
        if let Some(Frame::Locals { givens, .. }) = self.env.frames.last_mut() {
            givens.push(sym);
        }
    }

    fn bind_local_class(&mut self, name: Name, c: ClassId) {
        if let Some(Frame::Locals { classes, .. }) = self.env.frames.last_mut() {
            classes.push((name, c));
        }
    }

    fn local_class_named(&self, name: Name) -> Option<ClassId> {
        match self.env.frames.last() {
            Some(Frame::Locals { classes, .. }) => classes.iter().rev().find(|(n, _)| *n == name).map(|&(_, c)| c),
            _ => None,
        }
    }

    fn local_object_named(&self, name: Name) -> Option<ClassId> {
        let Some(Frame::Locals { names, .. }) = self.env.frames.last() else { return None };
        let s = names.iter().rev().find(|(n, _)| *n == name).map(|&(_, s)| s)?;
        let info = self.syms.sym(s);
        match self.types.get(info.sig.as_ref()?.ret) {
            Type::Class(c, _) if self.syms.class(c).local_module == Some(s) => Some(c),
            _ => None,
        }
    }

    /// A local class and a local object of one name in one block are companions, as at the
    /// top level.
    fn link_local_companion(&mut self, c: ClassId, name: Name) {
        let other = match self.syms.class(c).local_module {
            Some(_) => self.local_class_named(name),
            None => self.local_object_named(name),
        };
        if let Some(o) = other {
            self.syms.class_mut(c).companion = Some(o);
            self.syms.class_mut(o).companion = Some(c);
        }
    }

    /// A local object: an ordinary final class lifted with what it captures, and a lazy val
    /// holding its instance, which is made once per run of the block, as under scalac.
    fn enter_local_object(&mut self, file: FileId, d: ast::DefId) {
        let ast = self.cur_ast();
        let def = ast.def(d);
        let DefKind::Class(cls) = &def.kind else { return };
        let c = self.syms.new_class(def.name, ClassKind::Class, def.mods | mods::FINAL, Owner::Local, file, Some(d), def.span);
        self.def_classes.insert(file.0 as usize, d, c);
        self.syms.class_mut(c).has_exports = !cls.exports.is_empty();
        self.record_self_alias(c);
        self.enter_class_annots(file, def, c);
        self.enter_body(file, c, &cls.body);
        let ty = self.types.class(c, &[]);
        let sym = self.new_local(def.name, SymKind::Val, ty, def.span);
        self.syms.sym_mut(sym).mods |= mods::LAZY;
        self.syms.class_mut(c).local_module = Some(sym);
        self.anon_envs.insert(c, Arc::new(self.env.clone()));
        self.bind_local(def.name, sym);
        if def.mods & mods::IMPLICIT != 0 {
            self.bind_given(sym);
        }
        self.link_local_companion(c, def.name);
    }

    /// A local enum is entered as a top-level one, with its companion and values, and is
    /// checked not to capture anything, since its values are made once.
    fn enter_local_enum(&mut self, file: FileId, d: ast::DefId) {
        let name = self.cur_ast().def(d).name;
        self.enter_def(file, Owner::Local, d);
        let Some(&c) = self.def_classes.get(file.0 as usize, &d) else { return };
        let env = Arc::new(self.env.clone());
        self.anon_envs.insert(c, env.clone());
        self.bind_local_class(name, c);
        if let Some(companion) = self.syms.class(c).companion {
            self.anon_envs.insert(companion, env);
            if let Some(m) = self.syms.class(companion).module_sym {
                self.bind_local(name, m);
            }
        }
    }

    /// scalac's restriction (`PrepareInlineable.makeInlineable`, E162) on the block `items` of an
    /// inline method's body under the definition check: no case class, no case object and no
    /// enum case with parameters in the body of a method that is a class's or a package's
    /// member, each reported at its name; a local inline method, which is called only where it
    /// is defined, has none. Whether one was reported.
    fn refuses_case_classes(&mut self, items: &[Stmt]) -> bool {
        let Some(m) = self.inline_under_check() else { return false };
        if self.syms.sym(m).owner == Owner::Local {
            return false;
        }
        let ast = self.cur_ast();
        let class_msg = "Case class definitions are not allowed in inline methods or quoted code. Use a normal class instead.";
        let object_msg = "Case object definitions are not allowed in inline methods or quoted code. Use a normal object instead.";
        let mut refused: Vec<(Span, &str)> = Vec::new();
        for s in items {
            let Stmt::Def(d) = s else { continue };
            let def = ast.def(*d);
            let DefKind::Class(cls) = &def.kind else { continue };
            match cls.kind {
                ast::ClassKind::Object if def.mods & mods::CASE != 0 => refused.push((def.span, object_msg)),
                ast::ClassKind::Class if def.mods & mods::CASE != 0 => refused.push((def.span, class_msg)),
                ast::ClassKind::Enum => {
                    for c in &cls.body {
                        let Stmt::Def(cd) = c else { continue };
                        let case = ast.def(*cd);
                        if let DefKind::Class(k) = &case.kind {
                            if k.kind == ast::ClassKind::EnumCase && !k.clauses.is_empty() {
                                refused.push((case.span, class_msg));
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        for &(span, msg) in &refused {
            self.error(span, msg);
        }
        !refused.is_empty()
    }

    /// scalac's restriction: a case class in the body of an inline method would be defined anew
    /// at every expansion. The body is typed when it is expanded, so the definition is found
    /// through the site and reported once.
    fn check_case_class_in_inline(&mut self, span: Span) {
        let file = self.env.file;
        let site = self.inline.sites.iter().find(|s| s.body_file == file && s.body_span.start <= span.start && span.end <= s.body_span.end);
        let Some((def_file, def_span)) = site.map(|s| (s.body_file, s.body_span)) else { return };
        let msg = "Case class definitions are not allowed in inline methods or quoted code. Use a normal class instead.";
        // Reported at the inline body, as scalac reports it at the definition, once for all of
        // its expansions.
        if self.diags.items.iter().any(|d| d.file == def_file && d.span == def_span && d.msg == msg) {
            return;
        }
        self.diags.error(def_file, def_span, msg);
    }

    /// The class of the local object that `e` reads, when it is one.
    pub fn local_module_class(&self, e: TExprId) -> Option<ClassId> {
        let TExpr::Local(s) = self.prog.expr(e) else { return None };
        self.local_module_of_sym(s)
    }

    /// The class of the local object whose instance the lazy val `s` holds, when it is one.
    pub fn local_module_of_sym(&self, s: SymId) -> Option<ClassId> {
        let info = self.syms.sym(s);
        if info.mods & mods::LAZY == 0 {
            return None;
        }
        match self.types.get(info.sig.as_ref()?.ret) {
            Type::Class(c, _) if self.syms.class(c).local_module == Some(s) => Some(c),
            _ => None,
        }
    }

    /// The class of the object nested in a class or trait whose instance the lazy val `s` of the
    /// outer holds, when it is one.
    pub fn inner_object_of_sym(&self, s: SymId) -> Option<ClassId> {
        let info = self.syms.sym(s);
        if info.mods & mods::LAZY == 0 {
            return None;
        }
        match self.types.get(info.sig.as_ref()?.ret) {
            Type::Class(c, _) if self.syms.class(c).inner_object == Some(s) => Some(c),
            _ => None,
        }
    }

    /// The object nested in the class or trait `outer` under `name`, which a method of the name
    /// may stand beside (`def ci(args: Any*)` and `object ci` of case-insensitive's syntax).
    pub fn nested_object_of(&self, outer: ClassId, name: Name) -> Option<ClassId> {
        let s = *self.syms.class(outer).members.get(&name)?;
        match self.syms.alternatives(s) {
            Some(alts) => alts.iter().find_map(|&a| self.inner_object_of_sym(a)),
            None => self.inner_object_of_sym(s),
        }
    }

    /// `this` of a local object's class and the object itself are one type, as `File.type`
    /// is for a top-level object; and an object's type is the singleton of a stable val
    /// declared as that type (scala-library's `val Nil = immutable.Nil`, whose `Nil.type` is
    /// the object), since the val can hold nothing but the one instance. An object nested in a
    /// class has an instance per enclosing instance: its class is the singleton named without
    /// a prefix (`R.type` inside the class, `implicitly[foo.type]` of an implicit object), not
    /// one named through another path (`a.R.type` takes `a.R` alone).
    pub(super) fn local_object_sub(&mut self, a: TypeId, b: TypeId) -> Option<bool> {
        let (ta, tb) = (self.types.get(a), self.types.get(b));
        match (ta, tb) {
            (Type::This(c), Type::Term(s)) | (Type::Term(s), Type::This(c)) if self.syms.class(c).local_module == Some(s) => Some(true),
            (Type::Class(c, args), Type::Term(s)) if args == EMPTY_LIST && self.is_module_class(c) && self.val_of_object(s, c) => Some(true),
            (Type::Class(c, args), Type::Select(p, s))
                if args == EMPTY_LIST
                    && (self.syms.class(c).kind == ClassKind::Object || (self.is_module_class(c) && matches!(self.types.get(p), Type::This(_))))
                    && self.val_of_object(s, c) =>
            {
                Some(true)
            }
            _ => None,
        }
    }

    /// Whether `s` is a stable val declared as the type of the object `c`.
    fn val_of_object(&mut self, s: SymId, c: ClassId) -> bool {
        let info = self.syms.sym(s);
        if !matches!(info.kind, SymKind::Val | SymKind::Param) || info.by_name || info.mods & mods::MUTABLE != 0 {
            return false;
        }
        let Some(sig) = info.sig.clone() else { return false };
        let ret = self.dealias(sig.ret);
        matches!(self.types.get(ret), Type::Class(k, a) if k == c && a == EMPTY_LIST)
    }

    /// Inside the body of a local object its name stands for `this`.
    pub(super) fn enclosing_local_object(&self, s: SymId) -> Option<ClassId> {
        if self.syms.sym(s).mods & mods::LAZY == 0 {
            return None;
        }
        self.env.frames.iter().rev().find_map(|f| match f {
            Frame::Class(k) if self.syms.class(*k).local_module == Some(s) => Some(*k),
            _ => None,
        })
    }

    fn check_local_enum_captures(&mut self, c: ClassId, first: usize, span: Span) {
        let captures = self.captured_by(c, first);
        let Some(&s) = captures.first() else { return };
        let msg = format!(
            "a local enum cannot refer to {}: its values are made once; move the enum to the top level or make it a class",
            self.name_str(self.syms.sym(s).name)
        );
        self.error(span, msg);
    }

    /// A local trait has no constructor to take what it captures, so it is lifted as a
    /// top-level one and may not refer to the block's values.
    fn check_local_trait_captures(&mut self, c: ClassId, first: usize, span: Span) {
        let captures = self.captured_by(c, first);
        let Some(&s) = captures.first() else { return };
        let msg = format!(
            "not supported yet: a local trait referring to {} of its enclosing block; move the trait to the top level",
            self.name_str(self.syms.sym(s).name)
        );
        self.error(span, msg);
    }

    pub fn value_sig(&mut self, ty: TypeId) -> Arc<MethodSig> {
        self.value_sigs.entry(ty).or_insert_with(|| Arc::new(MethodSig::value(ty))).clone()
    }

    pub fn new_local(&mut self, name: Name, kind: SymKind, ty: TypeId, span: Span) -> SymId {
        let file = self.env.file;
        let sym = self.syms.new_sym(name, kind, 0, Owner::Local, file, None, span);
        let sig = self.value_sig(ty);
        let mut s = self.syms.sym_mut(sym);
        s.sig = Some(sig);
        s.state().done_fresh();
        if self.index.is_some() {
            self.index_local(sym);
        }
        sym
    }

    /// A local method symbol with the given parameter and result types, which a macro makes
    /// through `Symbol.newMethod`.
    pub fn new_local_method(&mut self, name: Name, params: &[TypeId], ret: TypeId, span: Span) -> SymId {
        let file = self.env.file;
        let sym = self.syms.new_sym(name, SymKind::Def, 0, Owner::Local, file, None, span);
        let mut clause = ClauseSig::default();
        for (i, &ty) in params.iter().enumerate() {
            let pname = self.interner.intern(&format!("x{}", i));
            let p = self.syms.new_sym(pname, SymKind::Param, 0, Owner::Local, file, None, span);
            let psig = self.value_sig(ty);
            let mut info = self.syms.sym_mut(p);
            info.sig = Some(psig);
            info.state().done_fresh();
            clause.params.push(ParamSig { name: pname, ty, by_name: false, repeated: false, has_default: false, sym: p });
        }
        let sig = Arc::new(MethodSig { tparams: Vec::new(), clauses: vec![clause], ret });
        let mut s = self.syms.sym_mut(sym);
        s.sig = Some(sig);
        s.state().done_fresh();
        sym
    }

    /// A compiler temporary, `<prefix>$<index>`. `$` is a letter of Scala's identifiers, so a
    /// program may name a definition alike: the JavaScript emitter names a local apart from what
    /// its scope reads (`src/emit/scope.rs`), and the other backends refer by symbol.
    pub fn fresh_local(&mut self, prefix: &'static str, ty: TypeId, span: Span) -> SymId {
        self.indexed_local(prefix, 0, ty, span)
    }

    pub fn indexed_local(&mut self, prefix: &'static str, index: u32, ty: TypeId, span: Span) -> SymId {
        let name = match self.temp_names.get(&(prefix, index)) {
            Some(&n) => n,
            None => {
                let n = self.interner.intern(&format!("{}${}", prefix, index));
                self.temp_names.insert((prefix, index), n);
                n
            }
        };
        self.new_local(name, SymKind::Val, ty, span)
    }

    /// An expected type is only useful for checking when it is not a bare unsolved variable.
    pub fn concrete_expected(&mut self, expected: Option<TypeId>) -> Option<TypeId> {
        let t = self.deref(expected?);
        match self.types.get(t) {
            Type::Var(_) => None,
            _ => Some(t),
        }
    }

    pub fn str_kind(&mut self, ty: TypeId) -> StrKind {
        let t = self.dealias(ty);
        if t == self.b.t_string || t == self.b.t_char {
            StrKind::Str
        } else if t == self.b.t_int || t == self.b.t_boolean || t == self.b.t_byte || t == self.b.t_short {
            StrKind::Plain
        } else if t == self.b.t_double || t == self.b.t_float {
            StrKind::Double
        } else if t == self.b.t_long {
            StrKind::Long
        } else {
            StrKind::Generic
        }
    }

    /// How a value of type `ty` becomes a string, unless it is one to the target.
    pub(super) fn str_conversion(&mut self, ty: TypeId) -> Option<StrKind> {
        match self.str_kind(ty) {
            // A Char is a String already in JavaScript, and a `char` on the JVM and in the interpreter.
            StrKind::Str if (self.jvm || self.interp) && self.dealias(ty) == self.b.t_char => Some(StrKind::Str),
            StrKind::Str => None,
            kind => Some(kind),
        }
    }

    /// An operand of a concatenation as the string the concatenation makes of it.
    pub fn to_str(&mut self, te: TExprId, ty: TypeId) -> TExprId {
        match self.str_conversion(ty) {
            Some(kind) => self.prog.rendering(te, kind),
            None => te,
        }
    }

    pub fn to_string_call(&mut self, te: TExprId, ty: TypeId) -> TExprId {
        match self.str_conversion(ty) {
            Some(kind) => self.prog.to_string_call(te, kind),
            None => te,
        }
    }

    pub fn check_expr(&mut self, e: ExprId, expected: TypeId) -> TExprId {
        self.check_expr_own(e, expected).0
    }

    /// `check_expr`, with the type the expression had before it was adapted.
    pub fn check_expr_own(&mut self, e: ExprId, expected: TypeId) -> (TExprId, TypeId) {
        let (te, ty) = self.type_expr(e, Some(expected));
        if expected == self.b.t_unit {
            self.warn_discarded(e, te, ty);
        }
        let span = self.cur_ast().expr_span(e);
        (self.adapt(te, ty, expected, span), ty)
    }

    /// A lambda whose body, typed under the result type the expected type gave (`body_ty`, the
    /// body's own type), has a type that holds the error type, which the lambda's own type does
    /// not show (`Worker::erroneous_lambdas`).
    fn note_erroneous_lambda(&mut self, lambda: TExprId, body_ty: Option<TypeId>) {
        if body_ty.is_some_and(|t| self.types.contains_error(t)) {
            self.erroneous_lambdas.push(lambda);
        }
    }

    /// scalac's warning for a pure value that a `Unit` swallows: `def f(): Unit = 5`.
    pub fn warn_discarded(&mut self, e: ExprId, te: TExprId, ty: TypeId) {
        let ty = self.deref(ty);
        let mut value = te;
        while let TExpr::Block(_, res) = self.prog.expr(value) {
            value = res;
        }
        if ty == self.b.t_unit || ty == NOTHING || ty == ERROR || !self.pure_value(value) {
            return;
        }
        let span = self.cur_ast().expr_span(self.result_expr(e));
        let msg = format!("Discarded non-Unit value of type {}. Add `: Unit` to discard silently.", self.show(ty));
        self.warn(span, msg);
    }

    /// scalac's warning for a statement that computes a value and drops it: `1; 2`. A closure
    /// the typer made, a method's eta-expansion (`f`, `f _`, `g(1)` short of a list), is an error
    /// instead (dotty's `checkStatementPurity`, E178), where a written function literal or an
    /// ascription to a function type is warned of.
    /// One whose typing failed past `mark` is checked for neither, as dotty checks no erroneous
    /// tree (a parser's recovery leaves such statements).
    pub fn warn_pure_statement(&mut self, e: ExprId, te: TExprId, ty: TypeId, mark: (usize, u32)) {
        let ty = self.deref(ty);
        if ty == self.b.t_unit || ty == ERROR || !self.pure_value(te) || self.failed_since(mark) {
            return;
        }
        let result = self.result_expr(e);
        let span = self.cur_ast().expr_span(result);
        if matches!(self.prog.expr(te), TExpr::Lambda(..)) && self.made_by_typer(result) {
            let msg = format!("missing argument list for value of type {}", self.show(ty));
            self.error(span, msg);
            return;
        }
        self.warn(span, "A pure expression does nothing in statement position");
    }

    /// Whether a closure typed of the expression is the typer's, no function literal the program
    /// wrote (`untpd.isFunction`) nor an ascription, whose typed tree dotty keeps around it.
    fn made_by_typer(&self, e: ExprId) -> bool {
        match self.cur_ast().expr(e) {
            Expr::Lambda(..) | Expr::PolyLambda(..) | Expr::Typed(..) => false,
            Expr::Parens(inner) => self.made_by_typer(inner),
            _ => true,
        }
    }

    /// Whether evaluating the expression can have no effect: a literal, a value, `this`, a
    /// function literal, or a tuple or block of such.
    fn pure_value(&self, te: TExprId) -> bool {
        match self.prog.expr(te) {
            TExpr::Int(_)
            | TExpr::Long(_)
            | TExpr::Double(_)
            | TExpr::Bool(_)
            | TExpr::Char(_)
            | TExpr::Str(_)
            | TExpr::Unit
            | TExpr::This
            | TExpr::Module(_)
            | TExpr::Lambda(..) => true,
            TExpr::Local(s) | TExpr::Static(s) => {
                let info = self.syms.sym(s);
                matches!(info.kind, SymKind::Val | SymKind::Var) && info.mods & mods::LAZY == 0
            }
            TExpr::Block(stmts, res) => stmts.is_empty() && self.pure_value(res),
            TExpr::New(c, args) => {
                self.is_tuple_class(c) && self.prog.expr_list(args).iter().all(|&a| self.pure_value(a))
            }
            _ => false,
        }
    }

    /// The expression a block ends with, where the warning about its value points.
    pub(super) fn result_expr(&self, e: ExprId) -> ExprId {
        let ast = self.cur_ast();
        match ast.expr(e) {
            Expr::Parens(inner) => self.result_expr(inner),
            Expr::Block(stmts) => match ast.stmt_list(stmts).last() {
                Some(&Stmt::Expr(last)) => self.result_expr(last),
                _ => e,
            },
            _ => e,
        }
    }

    /// A collection where a function of one argument is expected, as scala-library declares
    /// them (`Set[A] <: A => Boolean`, `Map[K, V] <: K => V`, `Seq[A] <: Int => A`): the std's
    /// collections are no functions at run time, so the value's `apply` is wrapped in one.
    fn collection_as_function(&mut self, te: TExprId, actual: TypeId, exp: TypeId, span: Span) -> Option<(TExprId, TypeId)> {
        let (params, ret) = self.as_function(exp)?;
        if params.len() != 1 {
            return None;
        }
        let c = self.class_of(actual)?;
        let bases = [self.b.set, self.b.map, self.b.seq, self.class_at(&["scala", "collection", "Set"])];
        if !bases.iter().flatten().any(|&k| c == k || self.derives_from(c, k)) {
            return None;
        }
        let mark = self.hoisted.len();
        let recv = self.hoist(te, actual, span);
        let x = self.indexed_local("elem", 1, params[0], span);
        let arg = self.prog.add(TExpr::Local(x));
        let list = super::apply::ArgList { args: vec![super::apply::ArgSrc::Typed(arg, params[0])], using: false, span };
        let (body, body_ty) = self.apply_member(recv, actual, names::APPLY, None, vec![list], span, Some(ret));
        if body_ty == ERROR {
            self.hoisted.truncate(mark);
            return None;
        }
        let l = self.prog.syms(&[x]);
        let lambda = self.prog.add(TExpr::Lambda(l, body));
        let ty = self.fun_type(&[params[0]], body_ty);
        self.prog.set_type(lambda, ty);
        Some((self.wrap_hoisted(mark, lambda), ty))
    }

    /// The value adapted to the expected type, with the type it has then: its own where it
    /// conforms as it stands by subtyping, the expected one where an adaptation gave it that.
    pub fn adapt_typed(&mut self, te: TExprId, actual: TypeId, expected: TypeId, span: Span) -> (TExprId, TypeId) {
        let exp = self.deref(expected);
        if exp == self.b.t_unit {
            let act = self.deref(actual);
            if act == self.b.t_unit || act == NOTHING {
                return (te, expected);
            }
            let stmts = self.prog.stmts.push_slice(&[TStmt::Expr(te)]);
            let unit = self.prog.add(TExpr::Unit);
            return (self.prog.add(TExpr::Block(stmts, unit)), expected);
        }
        if actual == exp && !self.is_unresolved_member(actual) {
            return (te, expected);
        }
        if let Some((applied, ty)) = self.apply_context_function(te, actual, exp, span) {
            return self.adapt_typed(applied, ty, expected, span);
        }
        let actual = self.capture_wildcards_in(te, actual);
        let mark = self.snapshot();
        if self.is_sub(actual, expected) {
            return (te, actual);
        }
        self.rollback(mark);
        if let Some(te) = self.named_to_tuple(te, actual, exp) {
            return (te, expected);
        }
        // A value typed ahead of the context function type expected of it (an argument typed
        // for overload resolution) is wrapped in one, as `type_expr` wraps the expression; one a
        // member's retry on its qualifier takes as its test typed it is not, as dotty's `adapt`
        // wraps no typed tree.
        if let Some((ptys, ret)) = self.as_context_function(exp) {
            if self.as_context_function(actual).is_none() && !self.retry_typed.contains_key(&te) {
                let syms: Vec<SymId> = ptys.iter().enumerate().map(|(i, &t)| self.indexed_local("contextual", i as u32 + 1, t, span)).collect();
                let ret = self.context_result(exp, &syms, ret);
                let inner = self.adapt(te, actual, ret, span);
                let l = self.prog.syms(&syms);
                let wrapped = self.prog.add(TExpr::Lambda(l, inner));
                let ty = self.closure_type(true, &syms, &ptys, ret, span);
                self.prog.set_type(wrapped, ty);
                return (wrapped, expected);
            }
        }
        if self.conforms_as_path(te, exp) || self.conforms_as_literal(te, actual, exp) {
            return (te, expected);
        }
        self.rollback(mark);
        if self.conforms_as_anon(te, exp) {
            return (te, expected);
        }
        self.rollback(mark);
        // The collection wrapped in a function is an attempt: where its `apply` fails or the
        // function does not fit, what it wrote goes.
        let attempt = self.attempt();
        if let Some((wrapped, ty)) = self.collection_as_function(te, actual, exp, span) {
            if self.is_sub(ty, expected) {
                self.close(attempt);
                return (wrapped, expected);
            }
        }
        self.retract(attempt);
        // An eta-expansion typed ahead of the trait with a single abstract method expected of
        // it (an argument typed for overload resolution, `map(f)` for a `SAM[Int, T]`) still
        // implements the trait, and so does a function literal a member's retry takes as typed
        // (dotty's `adapt` of a closure); a value of function type does not, as under scalac.
        let sam_convertible = self.eta_expansions.contains_key(&te) || (self.retry_typed.contains_key(&te) && matches!(self.prog.expr(te), TExpr::Lambda(..)));
        if let (true, Some((params, _))) = (sam_convertible, self.as_function(actual)) {
            if let Some((trait_ty, method, sig, subst)) = self.sam_method(exp, params.len()) {
                let sam_params: Vec<TypeId> = sig.clauses[0].params.iter().map(|p| self.types.subst(p.ty, &subst)).collect();
                let sam_ret = self.types.subst(sig.ret, &subst);
                let sam_fn = self.fun_type(&sam_params, sam_ret);
                let inner = self.snapshot();
                if self.is_sub(actual, sam_fn) {
                    return (self.sam_from_function(trait_ty, method, sig, subst, te, actual, span).0, expected);
                }
                self.rollback(inner);
            }
        }
        let act = self.deref(actual);
        let act = self.widen_lit(act);
        let exp_shape = self.deref_alias(exp);
        let target = match self.types.get(exp_shape) {
            Type::Union(..) => self.numeric_member(exp_shape).unwrap_or(exp),
            _ => exp,
        };
        if let Some(widened) = self.widen_numeric(te, act, target) {
            return (widened, expected);
        }
        if let Some(narrowed) = self.narrow_literal(te, act, target) {
            return (narrowed, expected);
        }
        if let Type::Var(v) = self.types.get(exp) {
            if let Some(converted) = self.widen_to_bound(te, act, v, expected) {
                return (converted, expected);
            }
            // dotc discards the value where the expected type derives from `Unit`, which a
            // variable bounded by it does (`eff { flyway.migrate() }` for a `Task[Unit]`).
            if let Some((discarded, _)) = self.discard_to_unit_bound(te, actual, v, exp, span) {
                return (discarded, expected);
            }
            if let Some(converted) = self.convert_to_var_bound(te, actual, v, expected, span) {
                return (converted, expected);
            }
        }
        if let Some(converted) = self.convert_to(te, actual, expected, span, true) {
            return (converted, expected);
        }
        // A conversion may take the number widened, as it takes an argument: `Byte.MinValue`
        // where an `implicit def` converts an `Int`.
        if self.is_numeric(act).is_some() {
            for wide in [self.b.t_int, self.b.t_long, self.b.t_float, self.b.t_double] {
                let Some(widened) = self.widen_numeric(te, act, wide) else { continue };
                if let Some(converted) = self.convert_to(widened, wide, expected, span, false) {
                    return (converted, expected);
                }
            }
        }
        // Against a singleton type a path is shown as the path it is.
        let found = match self.path_of(te) {
            Some(p) if self.types.has_paths(exp) => p,
            _ => actual,
        };
        let mut msg = format!("type mismatch: found {}, required {}", self.show(found), self.show(expected));
        if let Some(note) = self.match_type_note(actual, expected) {
            msg.push_str(&note);
        }
        let at = self.diags.items.len();
        self.error_unless_unknown(span, msg, &[found, expected]);
        if self.logging > 0 {
            self.mismatches.push((at, span, expected));
        }
        (te, expected)
    }

    pub fn adapt(&mut self, te: TExprId, actual: TypeId, expected: TypeId, span: Span) -> TExprId {
        self.adapt_typed(te, actual, expected, span).0
    }

    /// A value of type `A ?=> B` where a `B` is expected is applied to the given `A` in scope,
    /// as in Scala.
    #[inline]
    fn apply_context_function(&mut self, te: TExprId, actual: TypeId, expected: TypeId, span: Span) -> Option<(TExprId, TypeId)> {
        let (ptys, ret) = self.as_context_function(actual)?;
        self.apply_context_function_to(te, actual, expected, ptys, ret, span)
    }

    /// `apply_context_function` of a value whose type is the context function `ptys ?=> ret`.
    #[inline(never)]
    fn apply_context_function_to(&mut self, te: TExprId, actual: TypeId, expected: TypeId, ptys: Vec<TypeId>, ret: TypeId, span: Span) -> Option<(TExprId, TypeId)> {
        // An opaque type is its context function type where it is transparent, as dotty's
        // `isContextFunctionType` sees it there through the alias.
        let exp = self.deref(expected);
        let seen = self.opaque_underlying(exp).unwrap_or(exp);
        if expected == ERROR || self.as_context_function(seen).is_some() {
            return None;
        }
        // dotty's `adaptNoArgsOther` inserts the `apply` whatever the search finds, but on a
        // contextual closure as written (`isContextualClosure`): a `?=>` literal passed as `Any`
        // stays the function, its body not run, whatever givens are in scope. An ascription's
        // closure is none (`true: (Int ?=> Boolean)`, `(1: (Int ?=> Int))`): it is applied, and a
        // context function no given applies is an error (E172), not a value that conforms as it
        // stands, reported where the inserted application's arguments go, at the end of the value.
        if matches!(self.prog.expr(te), TExpr::Lambda(..)) && self.expr_marks.get(&te).map_or(true, |&m| m & super::MARK_ASCRIBED_CLOSURE == 0) {
            return None;
        }
        let mut args = Vec::with_capacity(ptys.len());
        for &p in &ptys {
            let arg = match self.resolve_given(p, span) {
                Some(arg) => arg,
                None => {
                    let msg = self.given_ambiguity.take().unwrap_or_else(|| format!("no given instance of type {} was found for parameter of {}", self.show(p), self.show(actual)));
                    let msg = msg + &self.given_failure_notes();
                    self.given_failure_error(Span::new(span.end, span.end), msg, p);
                    self.prog.add(TExpr::Unit)
                }
            };
            args.push(arg);
        }
        let ret = self.applied_named_result(actual, &args, &ptys).unwrap_or(ret);
        let l = self.prog.list(&args);
        let call = self.prog.add(TExpr::CallClosure(te, l));
        self.prog.set_type(call, ret);
        Some((call, ret))
    }

    /// A definition's initialiser or body of a context function type, where the definition
    /// declares no type, applied to the givens in scope: dotty's `adaptNoArgsOther` inserts the
    /// `apply` against an expected type that is no context function's, the wildcard of an inferred
    /// one among them; a context function literal (`isContextualClosure`) stays the function.
    /// One whose value comes out of a block or a branch is left as it is: dotty applies it where
    /// that result is typed, inside the scope of the block's givens, which is gone here.
    #[inline]
    pub(super) fn apply_inferred_context(&mut self, e: ExprId, te: TExprId, ty: TypeId) -> (TExprId, TypeId) {
        // Most definitions have a class's type that is no context function (an alias is no
        // class type here), told apart before anything else is read.
        if let Type::Class(c, _) = self.types.get(ty) {
            if !self.is_context_function_class(c) {
                return (te, ty);
            }
        }
        self.apply_inferred_context_rest(e, te, ty)
    }

    #[inline(never)]
    fn apply_inferred_context_rest(&mut self, e: ExprId, te: TExprId, ty: TypeId) -> (TExprId, TypeId) {
        self.sync_arity_if_stale();
        let shape = self.deref(ty);
        match self.types.get(shape) {
            Type::Class(c, _) if self.is_context_function_class(c) => {}
            // What may stand for one: an alias, an open variable, a function type with named
            // parameters.
            Type::Alias(..) | Type::Var(_) | Type::AppVar(..) | Type::Refined(..) => {}
            _ => return (te, ty),
        }
        if self.context_closure_literal(e) || self.compound_result(e) {
            return (te, ty);
        }
        let span = self.cur_ast().expr_span(e);
        self.apply_context_function(te, ty, ANY, span).unwrap_or((te, ty))
    }

    fn compound_result(&self, e: ExprId) -> bool {
        match self.cur_ast().expr(e) {
            Expr::Block(_) | Expr::If(..) | Expr::Match(..) | Expr::Try(_) | Expr::For(..) => true,
            Expr::Parens(inner) => self.compound_result(inner),
            _ => false,
        }
    }

    fn context_closure_literal(&self, e: ExprId) -> bool {
        let ast = self.cur_ast();
        match ast.expr(e) {
            Expr::Lambda(params, _) => ast.lambda_params[params.range()].first().is_some_and(|p| p.contextual),
            Expr::Parens(inner) => self.context_closure_literal(inner),
            _ => false,
        }
    }

    /// A stable expression has its singleton type as well as the widened one it was typed
    /// with, which is what a singleton expected type asks for (`val b: a.type = a`).
    pub(super) fn conforms_as_path(&mut self, te: TExprId, expected: TypeId) -> bool {
        match self.path_of(te) {
            Some(p) => self.is_sub(p, expected),
            None => false,
        }
    }

    /// A literal has its literal type as well, which a `Singleton` expected of it asks for
    /// (`val x: AnyRef & Singleton = "abc"`).
    fn conforms_as_literal(&mut self, te: TExprId, actual: TypeId, expected: TypeId) -> bool {
        if !matches!(self.prog.expr(te), TExpr::Int(_) | TExpr::Long(_) | TExpr::Double(_) | TExpr::Bool(_) | TExpr::Char(_) | TExpr::Str(_)) || !self.bounded_by_singleton(expected) {
            return false;
        }
        let Some(v) = self.fold_constant(te) else { return false };
        let lit = self.types.lit(v);
        if self.widen_lit(lit) != self.deref(actual) {
            return false;
        }
        let mark = self.snapshot();
        if self.is_sub(lit, expected) {
            return true;
        }
        self.rollback(mark);
        false
    }

    /// An anonymous class instance has the members its body defines, which a refinement type
    /// expected of it asks for (`val s: AnyRef { def m: Int } = new AnyRef { def m = 1 }`).
    fn conforms_as_anon(&mut self, te: TExprId, expected: TypeId) -> bool {
        if !matches!(self.types.get(expected), Type::Refined(..)) {
            return false;
        }
        let mut e = te;
        while let TExpr::Block(_, inner) = self.prog.expr(e) {
            e = inner;
        }
        match self.prog.expr(e) {
            TExpr::New(c, _) if self.syms.class(c).kind == ClassKind::Anon => {
                let own = self.types.class(c, &[]);
                if self.is_sub(own, expected) {
                    return true;
                }
                // A library body's anonymous class refining a `val` over the enclosing class's
                // path (`new Inspector(..) { val qctx: qctx.type = Inspector.this.qctx }` for an
                // `Inspector { val qctx: qctx.type }`): teq's `Inspector.this` inside a subclass
                // is the subclass's own, so the members are taken as scalac checked them.
                self.is_body_file(self.env.file) && self.anon_defines_refinement(own, expected)
            }
            _ => false,
        }
    }

    fn anon_defines_refinement(&mut self, own: TypeId, expected: TypeId) -> bool {
        let (parent, rs) = self.types.refinements_of(expected);
        if !self.is_sub(own, parent) {
            return false;
        }
        rs.into_iter().all(|r| match self.types.refinement(r) {
            Refinement::Term(name, ..) | Refinement::Val(name, ..) => self.find_member(own, name).is_some(),
            _ => self.refinement_holds(own, r),
        })
    }

    /// An `Int` literal whose value fits narrows to the `Byte`, `Short` or `Char` expected, and
    /// is a `Float` where one is expected, as scalac folds it; a `Char` literal narrows to the
    /// `Byte` or `Short` it fits (`Array[Byte]('G', 'I')`).
    fn narrow_literal(&mut self, te: TExprId, act: TypeId, target: TypeId) -> Option<TExprId> {
        // A plain call of a constant type still pending is its literal for the narrowing, as
        // scalac's typer folds it: expanded first; one of `Int` is no
        // constant, and one the narrowing does not take stays pending. One whose expansion failed,
        // negated or not, narrows by its type, its error the program's (`state::release_held`),
        // as scalac reports that error alone.
        let b = &self.b;
        let narrows = [b.t_byte, b.t_short, b.t_char, b.t_float].contains(&target) && (act == b.t_int || act == b.t_char);
        let call = match self.prog.expr(te) {
            TExpr::Unary(UnOp::IntNeg, inner) => inner,
            _ => te,
        };
        if narrows && self.pending_type(call).and_then(|t| self.fold_type(t)).is_some() && self.expand_pending_in(&[call]).is_some() {
            self.prog.set_type(te, target);
            return Some(te);
        }
        if act == self.b.t_char {
            let TExpr::Char(c) = self.prog.expr(te) else { return None };
            let fits = (target == self.b.t_byte && c <= i8::MAX as u16) || (target == self.b.t_short && c <= i16::MAX as u16);
            if !fits {
                return None;
            }
            self.prog.exprs[te.idx()] = TExpr::Int(c as i32);
            self.prog.set_type(te, target);
            return Some(te);
        }
        if act != self.b.t_int {
            return None;
        }
        let TExpr::Int(v) = self.prog.expr(te) else { return None };
        let b = &self.b;
        let fits = if target == b.t_byte {
            i8::try_from(v).is_ok()
        } else if target == b.t_short {
            i16::try_from(v).is_ok()
        } else if target == b.t_char {
            u16::try_from(v).is_ok()
        } else if target == b.t_float {
            self.prog.exprs[te.idx()] = TExpr::Double(v as f32 as f64);
            true
        } else {
            false
        };
        if !fits {
            return None;
        }
        if target == self.b.t_char {
            self.prog.exprs[te.idx()] = TExpr::Char(v as u16);
        }
        self.prog.set_type(te, target);
        Some(te)
    }

    /// The error of a number out of its type's range (`FromDigits.NumberTooLarge` and
    /// `NumberTooSmall`, which dotty's `Typer.typedNumber` reports).
    fn number_range(&mut self, large: bool, small: bool, span: Span) {
        if large {
            self.error(span, "number too large");
        } else if small {
            self.error(span, "number too small");
        }
    }

    /// A numeric value for an open variable bounded above by a wider numeric class widens to
    /// that class, which becomes the variable's lower bound: `Some(1)` for an `Option[Long]`.
    fn widen_to_bound(&mut self, te: TExprId, act: TypeId, v: TVarId, expected: TypeId) -> Option<TExprId> {
        let target = self.numeric_upper_bound(v)?;
        if !self.numeric_widening(act, target) {
            return None;
        }
        let mark = self.snapshot();
        if !self.is_sub(target, expected) {
            self.rollback(mark);
            return None;
        }
        // The conversion happens here, not again when the variable is solved.
        if let Some(entry) = self.pending_widenings.iter_mut().rev().find(|(e, _, _)| *e == te) {
            entry.1 = target;
        }
        self.widen_numeric(te, act, target)
    }

    /// The numeric class among the upper bounds of `v`, also through variables bounded by it
    /// and as the one numeric member of a union bound.
    fn numeric_upper_bound(&mut self, v: TVarId) -> Option<TypeId> {
        if self.tvars[v].upper.is_empty() {
            return None;
        }
        let mut vars = vec![v];
        for _ in 0..4 {
            let mut next = Vec::new();
            for v in vars {
                for i in 0..self.tvars[v].upper.len() {
                    let u = self.tvars[v].upper[i];
                    let u = self.deref(u);
                    match self.types.get(u) {
                        Type::Var(w) => next.push(w),
                        Type::Union(..) => {
                            if let Some(m) = self.numeric_member(u) {
                                return Some(m);
                            }
                        }
                        _ if self.is_numeric(u).is_some() => return Some(u),
                        _ => {}
                    }
                }
            }
            if next.is_empty() {
                return None;
            }
            vars = next;
        }
        None
    }

    /// The numeric class of a union that has exactly one, which is what a number expected to be
    /// `Double | String` or `Double | Unit` converts to, as scalac's numeric conversions take it
    /// there.
    pub(super) fn numeric_member(&mut self, union: TypeId) -> Option<TypeId> {
        let mut pending = vec![union];
        let mut found = None;
        while let Some(t) = pending.pop() {
            let t = self.deref(t);
            let numeric = match self.types.get(t) {
                Type::Union(a, b) => {
                    pending.extend([a, b]);
                    continue;
                }
                // An open member bounded by a numeric class stands for that class.
                Type::Var(v) => match self.numeric_upper_bound(v) {
                    Some(u) => u,
                    None => continue,
                },
                _ if self.is_numeric(t).is_some() => t,
                _ => continue,
            };
            match found {
                Some(f) if f != numeric => return None,
                _ => found = Some(numeric),
            }
        }
        found
    }

    /// A literal has its literal type where the expected type names one (`"fast" | "slow"`, or
    /// the argument of a `List[1]`); everywhere else its class, which is where Scala's widening
    /// of an inferred singleton type would take it anyway.
    fn literal_type(&mut self, expected: Option<TypeId>, value: LitVal, class: TypeId) -> TypeId {
        if self.expects_lit(expected) {
            self.types.lit(value)
        } else {
            class
        }
    }

    /// Whether the expected type asks a literal for its literal type: it names one. A class
    /// without arguments names none unless it is opaque, which is the common case.
    fn expects_lit(&mut self, expected: Option<TypeId>) -> bool {
        let Some(exp) = expected else { return false };
        if let Type::Class(c, EMPTY_LIST) = self.types.get(exp) {
            if self.syms.class(c).kind != ClassKind::Opaque {
                return false;
            }
        }
        self.mentions_lit(exp)
    }

    pub fn literal_expr(&mut self, value: LitVal) -> TExprId {
        let e = match value {
            LitVal::Int(v) => TExpr::Int(v),
            LitVal::Long(v) => TExpr::Long(v),
            LitVal::Double(bits) => TExpr::Double(f64::from_bits(bits)),
            LitVal::Char(c) => TExpr::Char(c),
            LitVal::Bool(b) => TExpr::Bool(b),
            LitVal::Str(s) => TExpr::Str(self.prog.add_str(self.interner.get(s))),
        };
        self.prog.add(e)
    }

    pub fn type_expr(&mut self, e: ExprId, expected: Option<TypeId>) -> (TExprId, TypeId) {
        if let Some(exp) = expected {
            if let Some((ptys, ret)) = self.as_context_function(exp) {
                if !self.is_contextual_lambda(e) {
                    return self.wrap_context_function(e, exp, &ptys, ret);
                }
            }
        }
        let (te, ty) = self.type_expr_untracked(e, expected);
        if self.index.is_some() {
            self.index_expr(e, te);
        }
        // A node handed back from elsewhere that another worker reads (the shared region's,
        // another worker's) is not changed under its readers: its copy takes the type.
        let typed = self.prog.typed(te, ty);
        if self.capturing() && typed != te {
            self.capture_forked_copy(te, typed);
        }
        let te = typed;
        if self.index.is_some() {
            self.index_node(e, te);
            if let Some(exp) = expected {
                self.index_expected(e, te, exp);
            }
        }
        if self.loaded.as_ref().map_or(false, |l| !l.replay_files.is_empty() && l.replay_files.contains_key(&self.env.file)) {
            self.replay_expr(e, te);
        }
        // A node handed back from elsewhere (an inline argument at its use) keeps its own source.
        if self.prog.records_spans() && self.prog.span_of(te).is_none() {
            let span = self.cur_ast().expr_span(e);
            self.prog.set_span(te, self.env.file, span);
            // A node of a body converted from a pickle: the tree it stands for, whose place the
            // pickle that holds an expansion of it writes.
            if self.capturing() {
                if let Some(r) = self.cur_ast().reader.as_deref() {
                    if let Some(addr) = r.places.whole(e) {
                        let file = r.places.file;
                        self.capture_tree(te, file, addr);
                    }
                }
            }
        } else if let Some(addr) = self.capturing().then(|| self.cur_ast().reader.as_deref().and_then(|r| r.places.whole_of_part(e).map(|a| (r.places.file, a)))).flatten() {
            // A node an enclosing tree of the pickle stands for whole (the block of an anonymous
            // class, typed as its instance), which the place of the tree is.
            self.capture_tree(te, addr.0, addr.1);
        } else if self.capturing() && self.inline.depth > 0 && matches!(self.cur_ast().expr(e), Expr::Splice(_)) && self.prog.expr(te).is_constant() {
            // A macro's constant in the place of its splice: the splice's place, where scalac's
            // folding of the expansion's `INLINED` leaves the literal. Any other tree keeps its
            // quote's.
            let span = self.cur_ast().expr_span(e);
            self.prog.set_span(te, self.env.file, span);
            if let Some(r) = self.cur_ast().reader.as_deref() {
                if let Some(&addr) = r.places.exprs.get(e.idx()) {
                    let file = r.places.file;
                    self.capture_tree(te, file, addr);
                }
            }
        }
        (te, ty)
    }

    /// Records of a product's node what its producer's typing recorded of the expansion it
    /// stands in (kind 8 version 2): the expansion with its method, a leaf, a
    /// type test of the call's type arguments.
    #[cold]
    #[inline(never)]
    fn replay_expr(&mut self, e: ExprId, te: TExprId) {
        let Some(r) = self.cur_ast().reader.as_deref() else { return };
        let (callee, leaf, test) = (r.replay.expansions.get(&e).copied(), r.replay.leaves.contains_key(&e), r.replay.test_exprs.contains_key(&e));
        if leaf {
            self.mark_leaf(te);
        }
        if let Some(callee) = callee {
            self.prog.note_expansion(te, crate::tir::Expansion { callee });
        }
        if let (true, TExpr::TypeTest(_, t)) = (test, self.prog.expr(te)) {
            self.prog.leaf_tests.insert(t, ());
        }
    }

    /// A chain of `+` goes on in its head through parentheses, a block, an interpolation, a
    /// splice, an inline parameter, a transparent inline method's call and the branch that a
    /// folded condition leaves, as scalac's backend finds it; behind anything else the source
    /// had around the head it ends. Where the typer writes such an expression as the node
    /// inside it (an ascription, a cast, a `toString` of a string, `synchronized`), the node
    /// says so, whatever it is when it is marked: a hole is filled and a call in a quote
    /// expanded later. Only a `String` heads a chain.
    pub(super) fn end_chain(&mut self, te: TExprId, ty: TypeId) {
        // Outside a quote a node stays what it is, and only these three carry a chain on. A
        // stored inline body is filled as a quote is: a parameter's node takes the mark for the
        // argument that stands for it (`substitution.rs`).
        let filled = self.quote.level > 0 || self.checks_inline_definition();
        let carries = filled || matches!(self.prog.expr(te), TExpr::StrConcat(_) | TExpr::Block(..) | TExpr::If(..));
        if carries && self.is_string(ty) {
            self.prog.mark_chain_end(te);
        }
    }

    pub fn is_string(&mut self, ty: TypeId) -> bool {
        self.stands_for(ty, self.b.t_string)
    }

    /// Whether `ty` is the class type `of` or an alias of it. A class that is no opaque type
    /// is itself, which keeps `dealias` off the types that most expansions and conditions have.
    fn stands_for(&mut self, ty: TypeId, of: TypeId) -> bool {
        if ty == of {
            return true;
        }
        let t = self.deref(ty);
        match self.types.get(t) {
            Type::Class(c, _) => t == of || (self.syms.class(c).kind == ClassKind::Opaque && self.dealias(t) == of),
            _ => self.dealias(t) == of,
        }
    }

    /// An ascription ends a chain of `+` and, to a type that is no literal type, keeps the
    /// condition it is from being a constant.
    fn ascribed(&mut self, te: TExprId, ty: TypeId) {
        self.end_chain(te, ty);
        if self.stands_for(ty, self.b.t_boolean) && self.fold_type(ty).is_none() {
            *self.expr_marks.entry(te).or_default() |= super::MARK_ASCRIBED;
        }
    }

    /// The branch that a folded condition leaves stands for its `if`, of type `ty`.
    pub fn mark_taken_branch(&mut self, cond: TExprId, te: TExprId, ty: TypeId) {
        if !self.is_string(ty) || !matches!(self.prog.expr(te), TExpr::If(..)) {
            return;
        }
        // A plain call still pending folds by its type alone, as scalac's typer leaves it: a
        // constant type (`inline def yes: true`), not a `Boolean` its expansion would be.
        // One that holds a pending call is folded once the call is expanded, as scalac folds it
        // after its `Inlining` phase (`fold_conditions_later`).
        let folded = if let Some(t) = self.pending_type(cond) {
            match self.fold_type(t) {
                Some(LitVal::Bool(b)) => Some(b),
                _ => None,
            }
        } else if self.holds_pending_call(cond) {
            self.attempts.fold_later.push((cond, te));
            None
        } else {
            self.folded_condition(cond)
        };
        if let Some(first) = folded {
            self.prog.mark_taken(te, first);
        }
    }

    /// The value of a condition that scalac folds away (`FirstTransform.transformIf` after
    /// `constToLiteral` and `ConstFold`, probed on 3.8.4): a constant whose evaluation is pure,
    /// so that nothing is lost with it.
    ///
    /// A constant is a literal, `null`, a `final val` without a type or an `inline val` reached
    /// by a stable path (`this`, a `val`, a `lazy val`, an object, a field of those; the path is
    /// not evaluated, so nothing on it is initialised), an operator over constants (`1 < 2`,
    /// `null == null`, `!false`, `("a" + "b") == "ab"`), `!`, `&`, `|` and `^` over conditions
    /// that are constants (`(if true then true else false) | false`), `true || x` and
    /// `false && x` whatever `x` is (where `true | x` evaluates `x`), an `if` on a constant
    /// whose branch is one, and a block of pure bindings (a constant `val`, a `def`) around
    /// one. It is no constant behind a `var`, a `def` or `new`, nor with a statement in front
    /// of it, nor where a cast or a `toString` was written
    /// (`true.asInstanceOf[Boolean]`, `"a".toString == "a"`), which the typer erased and marked
    /// (`MARK_CALL`). An ascription to a type that is no literal type (`(true: Boolean)`,
    /// `MARK_ASCRIBED`) keeps the condition it is from being a constant, while an operation
    /// over it sees the constant (`(true: Boolean) == true`, `!(false: Boolean)`), as scalac's
    /// `ConstantTree` looks through a `Typed`. The typer writes a constant member as its
    /// literal behind a stable path and keeps the selection behind any other receiver.
    pub fn folded_condition(&mut self, cond: TExprId) -> Option<bool> {
        if self.expr_marks.get(&cond).map_or(false, |&m| m & super::MARK_ASCRIBED != 0) {
            return None;
        }
        self.fold_pure_bool(cond)
    }

    fn fold_pure_bool(&mut self, e: TExprId) -> Option<bool> {
        match self.prog.expr(e) {
            TExpr::Bool(v) if !self.written_as_call(e) => Some(v),
            TExpr::Block(stmts, res) => {
                if !self.pure_bindings(stmts) {
                    return None;
                }
                self.fold_pure_bool(res)
            }
            TExpr::If(c, a, Some(b)) => {
                let branch = if self.fold_pure_bool(c)? { a } else { b };
                self.fold_pure_bool(branch)
            }
            TExpr::Unary(UnOp::BoolNot, a) => self.fold_pure_bool(a).map(|v| !v),
            TExpr::Prim(PrimOp::BoolOr, a, b) => match self.fold_pure_bool(a)? {
                true => Some(true),
                false => self.fold_pure_bool(b),
            },
            TExpr::Prim(PrimOp::BoolAnd, a, b) => match self.fold_pure_bool(a)? {
                true => self.fold_pure_bool(b),
                false => Some(false),
            },
            TExpr::Prim(op @ (PrimOp::BoolStrictAnd | PrimOp::BoolStrictOr | PrimOp::BoolXor), a, b) => {
                let (x, y) = (self.fold_pure_bool(a)?, self.fold_pure_bool(b)?);
                Some(match op {
                    PrimOp::BoolStrictAnd => x & y,
                    PrimOp::BoolStrictOr => x | y,
                    _ => x ^ y,
                })
            }
            TExpr::Prim(..) | TExpr::Unary(..) | TExpr::Local(_) | TExpr::Static(_) | TExpr::Field(..) if self.is_pure_constant(e) => {
                match (self.fold_constant_as_typed(e), self.prog.expr(e)) {
                    (Some(LitVal::Bool(v)), _) => Some(v),
                    // A top-level `final val`, which a reference names where a member's is
                    // written as its literal.
                    (None, TExpr::Static(s)) => match self.constant_value(s) {
                        Some(TExpr::Bool(v)) => Some(v),
                        _ => None,
                    },
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// Statements that scalac's constant folder sees through: a `def`, and a `val` bound to a
    /// constant.
    fn pure_bindings(&mut self, stmts: ListRef) -> bool {
        let items: Vec<TStmt> = self.prog.stmts[stmts.range()].to_vec();
        items.into_iter().all(|st| match st {
            TStmt::Fun(_) => true,
            TStmt::Val(s, init) => {
                let info = self.syms.sym(s);
                info.kind == SymKind::Val && info.mods & mods::LAZY == 0 && self.is_pure_constant(init) && self.fold_constant_as_typed(init).is_some()
            }
            TStmt::Expr(_) | TStmt::Pat(..) => false,
        })
    }

    fn written_as_call(&self, e: TExprId) -> bool {
        self.expr_marks.get(&e).map_or(false, |&m| m & super::MARK_CALL != 0)
    }

    fn is_pure_constant(&self, e: TExprId) -> bool {
        if self.written_as_call(e) {
            return false;
        }
        match self.prog.expr(e) {
            TExpr::Int(_) | TExpr::Long(_) | TExpr::Double(_) | TExpr::Bool(_) | TExpr::Char(_) | TExpr::Str(_) | TExpr::Null => true,
            TExpr::Local(_) | TExpr::Static(_) | TExpr::Field(..) => self.is_stable_path(e),
            TExpr::Unary(_, a) => self.is_pure_constant(a),
            TExpr::Prim(_, a, b) => self.is_pure_constant(a) && self.is_pure_constant(b),
            TExpr::StrConcat(l) => self.prog.expr_list(l).iter().all(|&i| self.is_pure_constant(i)),
            TExpr::Block(stmts, res) => stmts.is_empty() && self.is_pure_constant(res),
            _ => false,
        }
    }

    /// A path scalac takes for stable: no `var`, no by-name parameter and no call on the way.
    pub(super) fn is_stable_path(&self, e: TExprId) -> bool {
        let stable = |s: SymId| {
            let info = self.syms.sym(s);
            info.kind != SymKind::Var && !info.by_name
        };
        match self.prog.expr(e) {
            TExpr::Module(_) | TExpr::This | TExpr::Super(_) => true,
            TExpr::Local(s) | TExpr::Static(s) => stable(s),
            TExpr::Field(recv, s) => stable(s) && self.is_stable_path(recv),
            _ => false,
        }
    }

    /// `x ?=> body`, a context function literal, which is typed against the context function
    /// type expected of it rather than wrapped in one.
    fn is_contextual_lambda(&self, e: ExprId) -> bool {
        let ast = self.cur_ast();
        match ast.expr(e) {
            Expr::Lambda(params, _) => !params.is_empty() && ast.lambda_params[params.range()].iter().all(|p| p.contextual),
            Expr::Parens(inner) => self.is_contextual_lambda(inner),
            _ => false,
        }
    }

    /// An expression where `A ?=> B` is expected becomes `(using contextual$1: A) => e`, with
    /// the parameter a given while `e` is typed against `B`, as in Scala.
    fn wrap_context_function(&mut self, e: ExprId, expected: TypeId, ptys: &[TypeId], ret: TypeId) -> (TExprId, TypeId) {
        let span = self.cur_ast().expr_span(e);
        self.check_curried_dependent(expected, span);
        self.push_scope();
        let mut syms = Vec::with_capacity(ptys.len());
        for (i, &t) in ptys.iter().enumerate() {
            let sym = self.indexed_local("contextual", i as u32 + 1, t, span);
            self.bind_given(sym);
            syms.push(sym);
        }
        let ret = self.context_result(expected, &syms, ret);
        let ret = self.concrete_expected(Some(ret)).unwrap_or(ret);
        let body = self.check_expr(e, ret);
        self.pop_scope();
        let ty = self.closure_type(true, &syms, ptys, ret, span);
        let l = self.prog.syms(&syms);
        let te = self.prog.add(TExpr::Lambda(l, body));
        self.prog.set_type(te, ty);
        if self.prog.records_spans() && self.prog.span_of(te).is_none() {
            self.prog.set_span(te, self.env.file, span);
        }
        (te, ty)
    }

    fn type_expr_untracked(&mut self, e: ExprId, expected: Option<TypeId>) -> (TExprId, TypeId) {
        let ast = self.cur_ast();
        let span = ast.expr_span(e);
        match ast.expr(e) {
            Expr::IntLit(v) => {
                let mut exp = expected.map(|t| self.deref(t));
                match exp.map(|t| self.types.get(t)) {
                    Some(Type::Var(var)) => exp = self.numeric_upper_bound(var),
                    Some(Type::Union(..)) => exp = self.numeric_member(exp.unwrap()),
                    _ => {}
                }
                if exp == Some(self.b.t_long) {
                    return (self.prog.add(TExpr::Long(v)), self.b.t_long);
                }
                if exp == Some(self.b.t_double) {
                    return (self.prog.add(TExpr::Double(v as f64)), self.b.t_double);
                }
                if exp == Some(self.b.t_float) {
                    return (self.prog.add(TExpr::Double(v as f32 as f64)), self.b.t_float);
                }
                // An integer literal narrows to the Byte or Short that is expected when it fits.
                let narrow = if exp == Some(self.b.t_byte) && (i8::MIN as i64..=i8::MAX as i64).contains(&v) {
                    Some(self.b.t_byte)
                } else if exp == Some(self.b.t_short) && (i16::MIN as i64..=i16::MAX as i64).contains(&v) {
                    Some(self.b.t_short)
                } else {
                    None
                };
                if let Some(t) = narrow {
                    return (self.prog.add(TExpr::Int(v as i32)), t);
                }
                if v < i32::MIN as i64 || v > i32::MAX as i64 {
                    self.error(span, "number too large for Int; use an L suffix for Long");
                }
                let ty = self.literal_type(expected, LitVal::Int(v as i32), self.b.t_int);
                (self.prog.add(TExpr::Int(v as i32)), ty)
            }
            Expr::LongLit(v) => {
                let ty = self.literal_type(expected, LitVal::Long(v), self.b.t_long);
                (self.prog.add(TExpr::Long(v)), ty)
            }
            Expr::DoubleLit(v) => {
                let ty = self.literal_type(expected, LitVal::Double(v.to_bits()), self.b.t_double);
                (self.prog.add(TExpr::Double(v)), ty)
            }
            Expr::DecimalLit(s) => {
                // dotty's `Typer.typedNumber`: where a `Float` is expected the digits are read as
                // one (`FromDigits.floatFromDigits`), not as the `Double` rounded again; anywhere
                // else as a `Double` (`doubleFromDigits`). Out of the type's range the literal is
                // "number too large", or "number too small" when nonzero digits read as zero.
                let digits = ast.str(s);
                // Whether the digits are a zero's, read only of a value that is zero.
                let nonzero = || !digits.bytes().take_while(|&c| c != b'e' && c != b'E').all(|c| matches!(c, b'0' | b'.' | b'-' | b'+'));
                let float = expected.map(|t| self.deref(t)) == Some(self.b.t_float);
                let (v, ty) = if float {
                    let v = digits.parse::<f32>().unwrap_or(0.0);
                    self.number_range(v.is_infinite(), v == 0.0 && nonzero(), span);
                    (v as f64, self.b.t_float)
                } else {
                    let v = digits.parse::<f64>().unwrap_or(0.0);
                    self.number_range(v.is_infinite(), v == 0.0 && nonzero(), span);
                    (v, self.literal_type(expected, LitVal::Double(v.to_bits()), self.b.t_double))
                };
                (self.prog.add(TExpr::Double(v)), ty)
            }
            Expr::FloatLit(v) => (self.prog.add(TExpr::Double(v as f64)), self.b.t_float),
            Expr::BoolLit(v) => {
                let ty = self.literal_type(expected, LitVal::Bool(v), self.b.t_boolean);
                (self.prog.add(TExpr::Bool(v)), ty)
            }
            Expr::CharLit(v) => {
                let ty = self.literal_type(expected, LitVal::Char(v as u16), self.b.t_char);
                (self.prog.add(TExpr::Char(v as u16)), ty)
            }
            Expr::StringLit(s) => {
                let r = self.prog.add_str(ast.str(s));
                let ty = match self.expects_lit(expected) {
                    true => {
                        let name = self.interner.intern(ast.str(s));
                        self.types.lit(LitVal::Str(name))
                    }
                    false => self.b.t_string,
                };
                (self.prog.add(TExpr::Str(r)), ty)
            }
            Expr::UnitLit => (self.prog.add(TExpr::Unit), self.b.t_unit),
            Expr::Ident(_) | Expr::SymRef(_) | Expr::Select(..) | Expr::Apply(..) | Expr::UsingApply(..)
            | Expr::TypeApply(..) => {
                let part = self.body_part(super::profile::Part::Apply);
                let typed = self.type_application(e, expected);
                self.part_end(part);
                typed
            }
            Expr::This => match self.this_class() {
                Some(c) if self.parent_args_of == Some(c) => {
                    self.error(span, "this can be used only in a class, object, or template");
                    (self.prog.add(TExpr::Unit), ERROR)
                }
                Some(c) => {
                    self.complete_class(c);
                    if let Some((proxy, ty)) = self.inline_this(c) {
                        let te = match proxy {
                            Some(p) => match self.inline_arg(p) {
                                Some((receiver, _)) => receiver,
                                None => self.prog.add(TExpr::Local(p)),
                            },
                            None if self.syms.class(c).kind == ClassKind::Object => self.prog.add(TExpr::Module(c)),
                            None => self.prog.add(TExpr::This),
                        };
                        return (te, ty);
                    }
                    // `this` is the one value of `this.type`.
                    let ty = match expected.map(|e| (e, self.types.get(e))) {
                        Some((e, Type::This(k))) if k == c => e,
                        _ => self.syms.this_type(c),
                    };
                    (self.this_ref(c), ty)
                }
                None => {
                    self.error(span, "'this' is not available outside of a class");
                    (self.prog.add(TExpr::Unit), ERROR)
                }
            },
            Expr::ThisOf(t) => {
                let ty = self.resolve_type(t);
                match self.class_of(ty) {
                    Some(c) if self.syms.class(c).kind == ClassKind::Object && !self.env.frames.iter().any(|f| matches!(f, Frame::Class(k) if *k == c)) => {
                        (self.prog.add(TExpr::Module(c)), ty)
                    }
                    Some(c) => self.type_this_of(c),
                    None => (self.prog.add(TExpr::Unit), ERROR),
                }
            }
            Expr::QualThis(name) => match self.qualified_this_class(name).filter(|&c| self.parent_args_of != Some(c)) {
                Some(c) => {
                    self.complete_class(c);
                    self.type_this_of(c)
                }
                None => {
                    let msg = format!("{} is not an enclosing class", self.name_str(name));
                    self.error(span, msg);
                    (self.prog.add(TExpr::Unit), ERROR)
                }
            },
            Expr::ClassOf(t) => {
                let ty = self.resolve_type(t);
                self.type_class_of(ty, span)
            }
            Expr::Unsupported(what) => {
                let msg = format!("not supported yet: {}", self.cur_ast().strings[what.idx()]);
                self.error(span, msg);
                (self.prog.add(TExpr::Unit), ERROR)
            }
            Expr::Withheld(msg) => {
                let msg = self.cur_ast().strings[msg.idx()].clone();
                self.error(span, msg);
                self.withheld_met += 1;
                let note = self.diags.items.last().cloned();
                // What a run that reaches it stops at, in this interpreter or a later one that
                // reuses the typed body (a second macro expansion).
                let template = self.prog.add_str(WITHHELD_TEMPLATE);
                let none = self.prog.list(&[]);
                let te = self.prog.add(TExpr::Js(template, none));
                if let Some(d) = note {
                    self.withheld_notes.insert(te, d);
                }
                (te, ERROR)
            }
            Expr::Super(_) => {
                self.error(span, "super can only be used to select a member");
                (self.prog.add(TExpr::Unit), ERROR)
            }
            Expr::NamedArg(_, v) => {
                self.error(span, "named arguments are only allowed in argument lists");
                self.type_expr(v, expected)
            }
            Expr::Quote(body) => self.type_quote(body, span, expected),
            Expr::QuoteType(t) => self.type_quote_type(t, span),
            Expr::Splice(inner) => self.type_splice(inner, span, expected),
            Expr::SplicePat(p) => self.type_pattern_hole(p, span, expected),
            Expr::Infix(l, op, r) => {
                let part = self.body_part(super::profile::Part::Apply);
                let typed = match ast.reader.as_deref() {
                    Some(_) => self.type_infix_placed(e, l, op, r, span, expected),
                    None => self.type_infix(l, op, r, span, expected),
                };
                self.part_end(part);
                typed
            }
            Expr::Prefix(op, operand) => self.type_prefix(op, operand, span),
            Expr::Lambda(params, body) => {
                let part = self.body_part(super::profile::Part::Closure);
                self.sites.owners.push(SiteOwner::Lambda(span));
                let typed = self.type_lambda(params, body, span, expected);
                self.sites.owners.pop();
                self.part_end(part);
                typed
            }
            Expr::PolyLambda(tparams, lambda) => self.type_poly_lambda(e, tparams, lambda, span, expected),
            Expr::If(c, t, els) => self.type_plain_if(c, t, els, expected),
            Expr::InlineIf(c, t, els) => self.type_inline_if(c, t, els, span, expected),
            Expr::InlineMatch(scrut, cases) => self.type_inline_match(scrut, cases, span, expected),
            Expr::While(c, body) => {
                let tc = self.check_expr(c, self.b.t_boolean);
                let (tb, _) = self.type_expr(body, None);
                (self.prog.add(TExpr::While(tc, tb)), self.b.t_unit)
            }
            Expr::Block(stmts) => self.type_block(stmts, expected, Some(span)),
            Expr::Match(scrut, cases) => self.type_match(scrut, cases, span, expected),
            Expr::For(enums, body, is_yield) => {
                let rest = ForRest::Rest { idx: enums.start, end: enums.start + enums.len, body, is_yield };
                self.type_for(rest, span, expected)
            }
            Expr::Assign(lhs, rhs) => self.type_assign(lhs, rhs, span),
            Expr::Tuple(elems) => self.type_tuple(ast.expr_list(elems), expected),
            Expr::NamedTuple(names, values) => self.type_named_tuple(names, values, expected, span),
            Expr::Parens(inner) => self.type_expr(inner, expected),
            Expr::Unchecked(inner) => {
                let (te, ty) = self.type_expr(inner, expected);
                *self.expr_marks.entry(te).or_default() |= super::MARK_UNCHECKED;
                if self.capturing() {
                    self.capture_wrap(te, Wrap::Unchecked);
                }
                self.end_chain(te, ty);
                (te, ty)
            }
            Expr::Typed(inner, ty) => {
                if matches!(ast.ty(ty), TyExpr::Repeated(_)) {
                    // Where a library body reads a repeated parameter as a value (`m.attrs` of a
                    // `val attrs: Attr*`), scalac ascribes the read with the repeated type: the
                    // sequence.
                    if !self.is_body_file(self.env.file) {
                        self.error(span, "a vararg splice is only allowed as the last argument");
                    }
                    // An array spread as it is written (`f(arr*)`): the sequence a conversion
                    // makes of it is the typer's, which a Java parameter takes as the array.
                    let (te, ty) = self.type_expr(inner, expected);
                    if self.array_element(ty).is_some() {
                        self.prog.mark_spread(te);
                    }
                    return (te, ty);
                }
                let t = self.resolve_type(ty);
                // An ascription scalac inserted in a library body yields to the expression's own
                // type as a val's inferred type does.
                if self.in_converted_body() && ast.inferred_types.binary_search(&ty.0).is_ok() {
                    let typed = self.type_inferred_val(inner, t);
                    self.ascribed(typed.0, typed.1);
                    return typed;
                }
                let te = self.check_expr(inner, t);
                if matches!(self.prog.expr(te), TExpr::Lambda(..)) && self.as_context_function(t).is_some() {
                    *self.expr_marks.entry(te).or_default() |= super::MARK_ASCRIBED_CLOSURE;
                }
                // A value widened to a `Double` has no node of its own (`(i + 1).toDouble` is the
                // addition with `Double` recorded), so its ascription to a reference type stands
                // in a block of its own rather than record its type over the widened one; a
                // quote's tree keeps the ascription's shape for the macros that read it.
                // A literal narrowed to the `Byte`, `Short`, `Char` or `Float` expected is the same
                // node with that type recorded, which its ascription to a reference type would record
                // over, boxing it as the literal's own class: the block keeps the narrow type.
                let narrowed_literal = matches!(self.prog.expr(te), TExpr::Int(_) | TExpr::Double(_))
                    && self.prog.type_of(te).map_or(false, |it| [self.b.t_byte, self.b.t_short, self.b.t_char, self.b.t_float].contains(&it) && it != t);
                // A plain call still pending, whose node its expansion takes later, keeps the type
                // the typing gave it (a widening) under the ascription's.
                let pending_call = self.prog.type_of(te).map_or(false, |it| it != t && self.is_numeric(it).is_some()) && self.is_pending_call(te);
                let te = match self.prog.type_of(te) {
                    Some(inner_ty) if (inner_ty == self.b.t_double || narrowed_literal || pending_call) && self.is_numeric(t).is_none() && self.quote.level == 0 => {
                        self.prog.add(TExpr::Block(crate::ast::ListRef::EMPTY, te))
                    }
                    _ => te,
                };
                self.ascribed(te, t);
                if self.capturing() {
                    self.capture_wrap(te, Wrap::Ascribed(t));
                }
                if !self.inline.param_bindings.is_empty() {
                    *self.expr_marks.entry(te).or_default() |= super::MARK_RETYPED;
                }
                // `(Shape.Circle(1): Shape)` is a `Shape` to the member selected from it.
                if self.enum_case_new.map_or(false, |(e, _)| e == te) {
                    self.enum_case_new = None;
                }
                if self.constant_expr(te) {
                    self.ascribed_constant = Some(te);
                }
                // Whether or not `inner` is a constant yet (an inline parameter's read), as an
                // expansion's substitution gives it one.
                if self.fold_type(t).is_none() {
                    self.prog.mark_widened(te);
                }
                (te, t)
            }
            Expr::New(..) if self.in_converted_body() => self.type_hinted(e, expected, Self::type_new_expr),
            Expr::New(..) => self.type_new_expr(e, expected),
            Expr::NewAnon(d) => self.type_new_anon(d, span, expected),
            Expr::Interp(kind, parts, args) if kind == names::F_INTERP && self.scala_library_std() => self.type_linked_f_interp(parts, args, span),
            Expr::Interp(kind, parts, args) if kind != names::S_INTERP && kind != names::RAW_INTERP => {
                self.type_custom_interp(kind, parts, args, span, expected)
            }
            Expr::Interp(kind, parts, args) => self.type_interp(kind, parts, args, span),
            Expr::Derived => self.type_derived(expected, span),
            Expr::Return(value) => self.type_return(value, span),
            Expr::NullLit => self.type_null(),
            Expr::Throw(inner) => self.type_throw(inner, span),
            Expr::Try(index) => self.type_try(index, span, expected),
            Expr::Error => {
                self.error_nodes += 1;
                (self.prog.add(TExpr::Unit), ERROR)
            }
        }
    }

    pub(super) fn type_plain_if(&mut self, c: ExprId, t: ExprId, els: Option<ExprId>, expected: Option<TypeId>) -> (TExprId, TypeId) {
        {
            {
                let tc = self.check_expr(c, self.b.t_boolean);
                // Inside an inline expansion a condition that is a constant selects its branch,
                // as scalac's inliner does; the other branch is not typed, which is what ends
                // a recursion the arguments bound.
                if self.inline.depth > 0 {
                    let reads = self.inline.leaf_reads;
                    if let Some(LitVal::Bool(taken)) = self.fold_constant(tc) {
                        let (te, ty) = match (taken, els) {
                            (true, _) => self.type_expr_adapted(t, expected),
                            (false, Some(els)) => self.type_expr_adapted(els, expected),
                            (false, None) => (self.prog.add(TExpr::Unit), self.b.t_unit),
                        };
                        let read_leaf = self.inline.leaf_reads != reads;
                        self.mark_folded_leaf(te, |_| read_leaf);
                        return (te, ty);
                    }
                }
                let (te, ty) = self.type_if_branches(tc, t, els, expected);
                self.mark_taken_branch(tc, te, ty);
                // A condition the interpreter can evaluate leaves one branch in the output,
                // both having been typed.
                if self.inline.depth > 0 {
                    if let (TExpr::If(_, tt, tels), Some(LitVal::Bool(taken))) = (self.prog.expr(te), self.fold_by_eval(tc)) {
                        let te = match (taken, tels) {
                            (true, _) => tt,
                            (false, Some(x)) => x,
                            (false, None) => self.prog.add(TExpr::Unit),
                        };
                        self.mark_folded_leaf(te, |t| t.leaf_near(tc));
                        return (te, ty);
                    }
                }
                (te, ty)
            }
        }
    }

    fn type_if_branches(&mut self, tc: TExprId, t: ExprId, els: Option<ExprId>, expected: Option<TypeId>) -> (TExprId, TypeId) {
        {
            {
                let Some(els) = els else {
                    let tt = self.check_expr(t, self.b.t_unit);
                    return (self.prog.add(TExpr::If(tc, tt, None)), self.b.t_unit);
                };
                if let Some(exp) = self.branch_expected(expected) {
                    let (tt, ty1) = self.type_expr_adapted(t, Some(exp));
                    let (te, ty2) = self.type_expr_adapted(els, Some(exp));
                    let ty = self.joined_branches(exp, &[ty1, ty2]);
                    return (self.prog.add(TExpr::If(tc, tt, Some(te))), ty);
                }
                let guide = self.branch_guide(expected);
                let (tt, ty1) = self.type_expr(t, Some(guide));
                let (tt, ty1) = self.branch_to_var_bound(tt, ty1, expected, t);
                self.guide_with(guide, ty1);
                let (te, ty2) = self.type_expr(els, Some(guide));
                let (te, ty2) = self.branch_to_var_bound(te, ty2, expected, els);
                let mut branches = [(tt, ty1), (te, ty2)];
                self.harmonize_literals(&mut branches);
                // The variables the branches left open for the join are settled by it, as a
                // `match` settles its cases' (`if c then new Inv(1) else Inv.empty` is an `Inv[Int]`).
                let l = self.lub(branches[0].1, branches[1].1);
                let l = self.solve_bounded_in(l);
                let e = self.prog.add(TExpr::If(tc, tt, Some(te)));
                self.note_soft(e, l);
                (e, l)
            }
        }
    }

    /// A branch expected to be an open variable is converted into the variable's bound where it
    /// does not conform, as a single expression would be (`getOrElse(if c then u else n)` with
    /// the result expected to be a `Node` and a conversion of `u` to `Node`).
    pub(super) fn branch_to_var_bound(&mut self, te: TExprId, ty: TypeId, expected: Option<TypeId>, e: ExprId) -> (TExprId, TypeId) {
        if !matches!(expected.map(|t| self.deref(t)).map(|t| self.types.get(t)), Some(Type::Var(_))) {
            return (te, ty);
        }
        let span = self.cur_ast().expr_span(e);
        self.widen_lambda_result(te, ty, expected, span)
    }

    /// `new p.C(..)` of a library body hands `C` the prefix `p` as its enclosing instance, in
    /// the place of the capture the constructor takes first.
    #[cold]
    #[inline(never)]
    pub(super) fn pass_new_outer(&mut self, e: ExprId, te: TExprId) {
        let Some(&outer) = self.cur_ast().new_outers.get(&e) else { return };
        let TExpr::New(c, args) = self.prog.expr(te) else { return };
        if self.outer_class(c).is_none() || args.len == 0 {
            return;
        }
        let (outer_te, _) = self.type_expr(outer, None);
        let mut items = self.prog.expr_list(args).to_vec();
        items[0] = outer_te;
        let l = self.prog.list(&items);
        self.prog.exprs[te.idx()] = TExpr::New(c, l);
    }

    /// The types the elements of a tuple of `n` are typed against where `expected` is expected:
    /// those of a tuple type of as many; `None` where they are typed alone.
    pub(super) fn tuple_expected_elements(&mut self, expected: Option<TypeId>, n: usize) -> Option<Vec<TypeId>> {
        self.concrete_expected(expected).and_then(|t| {
            match self.types.get(t) {
                Type::Class(c, args) if self.is_tuple_class(c) => {
                    let items = self.types.items(args).to_vec();
                    (items.len() == n).then_some(items)
                }
                _ => None,
            }
        })
    }

    pub fn type_tuple(&mut self, ids: &[ExprId], expected: Option<TypeId>) -> (TExprId, TypeId) {
        let exp_elems = self.tuple_expected_elements(expected, ids.len());
        let mut tes = Vec::with_capacity(ids.len());
        let mut tys = Vec::with_capacity(ids.len());
        for (i, &id) in ids.iter().enumerate() {
            let exp = exp_elems.as_ref().map(|v| v[i]);
            // An element is an argument of `Tuple2.apply`, whose formal is a type variable: an
            // unsuffixed decimal there is a `Double` (`Typer.typedNumber`), which no expected
            // `Float` converts (`Constants.convertTo` takes no `Double` to a `Float`), as a
            // branch's or a block's result is.
            let typed_exp = match exp {
                Some(x) if self.deref(x) == self.b.t_float && self.decimal_result(id) => None,
                _ => exp,
            };
            let (te, ty) = self.type_expr(id, typed_exp);
            // An element expected to be an open variable takes what its bounds ask, as an
            // argument of `Tuple2.apply` does under scalac: `(xs.size, m)` for a `(B, M)`
            // whose `B` the expected `Long` bounds is widened.
            let (te, ty) = match exp.and_then(|x| self.concrete_expected(Some(x))) {
                Some(x) => (self.adapt(te, ty, x, self.cur_ast().expr_span(id)), x),
                None => self.branch_to_var_bound(te, ty, exp, id),
            };
            tes.push(te);
            tys.push(ty);
        }
        let ty = self.tuple_type(&tys);
        let class = self.tuple_class(ids.len());
        let span = ids.first().map_or(Span::default(), |&e| self.cur_ast().expr_span(e));
        let te = match (ids.len() > 22).then(|| self.tuple_xxl_value(&tes, span)).flatten() {
            Some(te) => te,
            None => {
                let l = self.prog.list(&tes);
                self.prog.add(TExpr::New(class, l))
            }
        };
        // A TupleXXL's construction is a call, which records its own.
        if self.capturing() && ids.len() <= 22 {
            self.capture_targs(te, &tys);
        }
        if self.deps.is_some() {
            self.deps_node(te, super::deps::Node::Apply(class));
        }
        (te, ty)
    }

    /// Whether a result of `e` is an unsuffixed decimal literal: itself, parenthesised, a
    /// block's last expression, a branch of an `if`, a `match` or a `try`.
    fn decimal_result(&self, e: ExprId) -> bool {
        let ast = self.cur_ast();
        match ast.expr(e) {
            Expr::DecimalLit(_) => true,
            Expr::Parens(inner) => self.decimal_result(inner),
            Expr::Block(l) => matches!(ast.stmt_list(l).last(), Some(&Stmt::Expr(last)) if self.decimal_result(last)),
            Expr::If(_, t, f) => self.decimal_result(t) || f.is_some_and(|f| self.decimal_result(f)),
            Expr::Match(_, cases) => ast.case_list(cases).iter().any(|c| self.decimal_result(c.body)),
            Expr::Try(i) => {
                let t = ast.try_expr(i);
                self.decimal_result(t.body) || ast.case_list(t.cases).iter().any(|c| self.decimal_result(c.body))
            }
            _ => false,
        }
    }

    /// `id"a${x}b"` stands for `StringContext("a", "b").id(x)`.
    /// `return e` leaves the enclosing method, which needs a declared result type, as in scalac.
    fn type_return(&mut self, value: Option<ExprId>, span: Span) -> (TExprId, TypeId) {
        // Under the definition check a method nested in the body is what a `return` leaves when
        // it has one; the inline method itself has none (`return_to` is taken for its body).
        let checked = self.inline_under_check().filter(|_| self.return_to.is_none());
        if let Some(callee) = self.inline.sites.last().map(|s| s.callee).or(checked) {
            let name = self.name_str(self.syms.sym(callee).name);
            self.error(span, format!("No explicit return allowed from inlineable method {}", name));
            let te = match value {
                Some(e) => self.type_expr(e, None).0,
                None => self.prog.add(TExpr::Unit),
            };
            return (te, NOTHING);
        }
        let target = match self.return_to {
            Some((name, Some(ty), _)) => Some((name, ty)),
            Some((name, None, _)) => {
                let name = self.interner.get(name).to_string();
                self.error(span, format!("method {} has a return statement; it needs a result type", name));
                None
            }
            None => {
                self.error(span, "return outside method definition");
                None
            }
        };
        let te = match (value, target) {
            (Some(e), Some((_, ty))) => self.check_expr(e, ty),
            (Some(e), None) => self.type_expr(e, None).0,
            (None, Some((_, ty))) => {
                let unit = self.prog.add(TExpr::Unit);
                self.adapt(unit, self.b.t_unit, ty, span)
            }
            (None, None) => self.prog.add(TExpr::Unit),
        };
        let ret = self.prog.add(TExpr::Return(te));
        if self.capturing() {
            if let Some((_, _, from)) = self.return_to {
                self.capture_form(ret, Form::Return(from));
            }
        }
        self.returns.push((ret, span));
        (ret, NOTHING)
    }

    /// `s"..."` and `raw"..."`: every argument is evaluated before any is rendered, as
    /// scalac's. Rendering as the arguments are evaluated can only differ where an effectful
    /// argument follows one whose rendering could see the effect, an object rather than a
    /// number or a string; the arguments are bound to temporaries first then.
    fn type_interp(&mut self, kind: Name, parts: ListRef, args: ListRef, span: Span) -> (TExprId, TypeId) {
        let ast = self.cur_ast();
        let part_ids = &ast.str_lists[parts.range()];
        let arg_ids = ast.expr_list(args);
        let mut typed = Vec::with_capacity(arg_ids.len());
        let mut object_seen = false;
        let mut reorders = false;
        for &a in arg_ids {
            let (te, ty) = self.type_expr(a, None);
            let ty = self.solve_in(ty);
            if object_seen && !self.is_stable(te) {
                reorders = true;
            }
            object_seen |= self.str_kind(ty) == StrKind::Generic;
            typed.push((te, ty));
        }
        let mark = self.hoisted.len();
        if reorders {
            for (te, ty) in typed.iter_mut() {
                *te = self.hoist(*te, *ty, span);
            }
        }
        let mut items = Vec::with_capacity(part_ids.len() + arg_ids.len());
        for (i, p) in part_ids.iter().enumerate() {
            let text = ast.str(*p);
            if kept_part(text, i) {
                let r = self.prog.add_str(text);
                items.push(self.prog.add(TExpr::Str(r)));
            }
            if let Some(&(te, ty)) = typed.get(i) {
                items.push(self.to_str(te, ty));
            }
        }
        let l = self.prog.list(&items);
        let te = self.prog.add(TExpr::StrConcat(l));
        self.interpolations.insert(te, ());
        if self.capturing() {
            self.capture_interp(te, kind, parts);
        }
        (self.wrap_hoisted(mark, te), self.b.t_string)
    }

    /// The interpolation `te` of the parts `parts`: each part as written, which the
    /// concatenation holds processed (an `s` interpolation's escapes) or leaves out (an empty
    /// one).
    #[cold]
    #[inline(never)]
    fn capture_interp(&mut self, te: TExprId, kind: Name, parts: ListRef) {
        let types: Vec<TypeId> = parts.range().map(|i| {
            let written = self.written_part(i);
            self.types.lit(LitVal::Str(self.interner.intern(&written)))
        }).collect();
        let parts = self.types.list(&types);
        self.capture_form(te, Form::Interp { kind, parts });
    }

    /// `f"..."` against scala-library, whose `StringContext.f` is a macro scalac expands into a
    /// `format`: the std's `formatInterpolation` over the parts and the arguments.
    fn type_linked_f_interp(&mut self, parts: ListRef, args: ListRef, span: Span) -> (TExprId, TypeId) {
        let ast = self.cur_ast();
        let part_ids: Vec<_> = ast.str_lists[parts.range()].to_vec();
        let arg_ids: Vec<ExprId> = ast.expr_list(args).to_vec();
        let Some(format) = self.std_def("formatInterpolation") else {
            self.error(span, "formatInterpolation is missing from the standard library");
            return (self.prog.add(TExpr::Unit), ERROR);
        };
        let mut strs = Vec::with_capacity(part_ids.len());
        for p in part_ids {
            let text = self.cur_ast().str(p).to_string();
            let r = self.prog.add_str(&text);
            strs.push(self.prog.add(TExpr::Str(r)));
        }
        let values: Vec<TExprId> = arg_ids.into_iter().map(|a| self.check_expr(a, ANY)).collect();
        let (t_string, array) = (self.b.t_string, self.b.array);
        let part_list = self.prog.list(&strs);
        let parts_array = self.prog.add(TExpr::ArrayLit(part_list));
        let parts_ty = self.types.class(array, &[t_string]);
        self.prog.set_type(parts_array, parts_ty);
        let value_list = self.prog.list(&values);
        let values_array = self.prog.add(TExpr::ArrayLit(value_list));
        let values_ty = self.types.class(array, &[ANY]);
        self.prog.set_type(values_array, values_ty);
        let call_args = self.prog.list(&[parts_array, values_array]);
        let call = super::apply::MethodCall { recv: None, sym: format, owner_subst: Vec::new(), ext_recv: None, prefix: None };
        (self.build_call(&call, call_args), t_string)
    }

    fn type_custom_interp(
        &mut self,
        id: Name,
        parts: ListRef,
        args: ListRef,
        span: Span,
        expected: Option<TypeId>,
    ) -> (TExprId, TypeId) {
        let ast = self.cur_ast();
        let list = ArgList {
            args: ast.expr_list(args).iter().map(|&a| ArgSrc::Ast(a)).collect(),
            using: false,
            span,
        };
        let Some(class) = self.string_context_class() else {
            self.error(span, "StringContext is missing from the standard library");
            return (self.prog.add(TExpr::Unit), ERROR);
        };
        self.complete_class(class);
        let mut items = Vec::with_capacity(parts.len as usize);
        for &p in &ast.str_lists[parts.range()] {
            let r = self.prog.add_str(ast.str(p));
            items.push(self.prog.add(TExpr::Str(r)));
        }
        let l = self.prog.list(&items);
        let seq = self.prog.add(TExpr::SeqLit(l));
        if self.capturing() {
            self.capture_form(seq, Form::Repeated(self.b.t_string));
            self.capture_written_parts(parts, &items);
        }
        let ctor_args = self.prog.list(&[seq]);
        let context = self.prog.add(TExpr::New(class, ctor_args));
        let context_ty = self.types.class(class, &[]);
        self.apply_member(context, context_ty, id, None, vec![list], span, expected)
    }

    /// A library body's local val or member with the type scalac inferred: where its
    /// initialiser does not check against that type, the val takes the initialiser's own; a
    /// member keeps its signature, which erases alike.
    pub(super) fn type_inferred_val(&mut self, rhs: ExprId, declared: TypeId) -> (TExprId, TypeId) {
        let key = (self.env.file, rhs, Some(declared));
        match self.inferred_val_outcomes.get(&key) {
            Some(true) => {
                let (init, ty) = self.type_expr(rhs, None);
                return (init, self.solve_inferred(ty));
            }
            Some(false) => return (self.check_expr(rhs, declared), declared),
            None => {}
        }
        // Each typing that may not check is an attempt of its own, which
        // fails where it reports an error: a type the reader left an error node reports none.
        let mark = self.attempt();
        let init = self.check_expr(rhs, declared);
        if !self.attempt_reported_error(&mark) {
            self.close(mark);
            return (init, declared);
        }
        self.retract(mark);
        let mark = self.attempt();
        let (init, ty) = self.type_expr(rhs, None);
        let own = ty != ERROR && !self.attempt_reported_error(&mark);
        self.inferred_val_outcomes.insert(key, own);
        if own {
            self.close(mark);
            return (init, self.solve_inferred(ty));
        }
        self.retract(mark);
        (self.check_expr(rhs, declared), declared)
    }

    fn type_new_expr(&mut self, e: ExprId, expected: Option<TypeId>) -> (TExprId, TypeId) {
        let Expr::New(ty, args) = self.cur_ast().expr(e) else { unreachable!() };
        let span = self.cur_ast().expr_span(e);
        let part = self.body_part(super::profile::Part::Apply);
        let prefixed = self.cur_ast().new_outers.contains_key(&e);
        let outer_prefixed = std::mem::replace(&mut self.new_prefixed, prefixed);
        let outer_new = std::mem::replace(&mut self.explicit_new, true);
        let typed = self.type_new(ty, &[(args, false)], false, span, expected);
        self.explicit_new = outer_new;
        self.new_prefixed = outer_prefixed;
        self.part_end(part);
        if !self.cur_ast().new_outers.is_empty() {
            self.pass_new_outer(e, typed.0);
        }
        typed
    }

    /// `new C[T](args)`, also with the argument lists an anonymous class expression collects.
    pub fn type_new(
        &mut self,
        ty: TyExprId,
        args: &[(ListRef, bool)],
        first_written: bool,
        span: Span,
        expected: Option<TypeId>,
    ) -> (TExprId, TypeId) {
        let ast = self.cur_ast();
        let (head, targs) = match ast.ty(ty) {
            TyExpr::Apply(f, ta) if self.dropped_type_list == Some(ta.start) => {
                self.dropped_type_list = None;
                (f, None)
            }
            TyExpr::Apply(f, ta) => (f, Some(ta)),
            _ => (ty, None),
        };
        let ctor = self.resolve_type_ctor(head);
        let alias_class = match self.types.get(ctor) {
            Type::Lambda(_, rhs) => {
                let rhs = self.dealias(rhs);
                match self.types.get(rhs) {
                    Type::Class(c, _) => Some(c),
                    _ => None,
                }
            }
            _ => None,
        };
        let applied_alias_class = match self.types.get(ctor) {
            Type::Alias(..) => {
                let instance = self.dealias(ctor);
                match self.types.get(instance) {
                    Type::Class(c, _) => Some((c, instance)),
                    _ => None,
                }
            }
            _ => None,
        };
        let class = match self.types.get(ctor) {
            Type::Ctor(c) | Type::Class(c, _) => c,
            // `new T[A]` for `type T[A] = C[A]`, a parameterised alias of a class (scala-library's
            // `scala.::` over `immutable.::`).
            Type::Lambda(ps, rhs) if matches!(self.types.get(rhs), Type::Class(_, args) if self.types.items(args).iter().zip(self.types.items(ps)).all(|(a, p)| a == p) && self.types.items(args).len() == self.types.items(ps).len()) => {
                match self.types.get(rhs) {
                    Type::Class(c, _) => c,
                    _ => unreachable!(),
                }
            }
            // `new A[T](args)` for `type A[X] = C[F, X]`, an alias that fixes some of its class's
            // arguments: the class constructed at the alias's expansion. A library body may spell
            // it with the class's own arguments (`new A[F, T]`), as Scala 3.3 pickles it.
            Type::Lambda(ps, _) if alias_class.is_some() => {
                let c = alias_class.unwrap();
                let written = targs.map_or(0, |l| l.len as usize);
                if written == self.class_arity(c) && written != self.types.items(ps).len() && self.is_body_file(self.env.file) {
                    let lists = self.new_arg_lists(args, span);
                    return self.apply_callee(super::apply::Callee::Ctor(c), targs, lists, span, expected);
                }
                // Unapplied, it constructs the class with all of the class's arguments inferred,
                // those the alias fixes included (scalac's `new A(1, 2)` is a `C[Int, Int]`).
                if targs.is_none() {
                    let lists = self.new_arg_lists(args, span);
                    return self.apply_callee(super::apply::Callee::Ctor(c), None, lists, span, expected);
                }
                let Some(l) = targs.filter(|_| written == self.types.items(ps).len()) else {
                    let alias = match ast.ty(head) {
                        TyExpr::Name(n) | TyExpr::Select(_, n) => self.name_str(n),
                        _ => String::new(),
                    };
                    let msg = if written > self.types.items(ps).len() { format!("Too many type arguments for {}", alias) } else { "only classes can be instantiated with new".to_string() };
                    self.error(span, msg);
                    return (self.prog.add(TExpr::Unit), ERROR);
                };
                let arg_tys: Vec<TypeId> = ast.ty_list(l).to_vec().into_iter().map(|a| self.resolve_type_ctor(a)).collect();
                let instance = self.types.apply_ctor(ctor, &arg_tys);
                let instance = self.dealias(instance);
                let lists = self.new_arg_lists(args, span);
                let (te, ty) = self.apply_callee(super::apply::Callee::Ctor(c), None, lists, span, Some(instance));
                return (te, if ty == ERROR { ty } else { instance });
            }
            // The same alias already applied, as a library body's `new` names it.
            Type::Alias(..) if targs.is_none() && applied_alias_class.is_some() => {
                let (c, instance) = applied_alias_class.unwrap();
                let lists = self.new_arg_lists(args, span);
                let (te, ty) = self.apply_callee(super::apply::Callee::Ctor(c), None, lists, span, Some(instance));
                return (te, if ty == ERROR { ty } else { instance });
            }
            _ => {
                if ctor != ERROR {
                    self.error(span, "only classes can be instantiated with new");
                }
                return (self.prog.add(TExpr::Unit), ERROR);
            }
        };
        // `new AnyRef`, the plain object of a lock or a token.
        if class == self.b.any_ref && args.iter().all(|(l, _)| l.is_empty()) {
            return (self.prog.add(TExpr::New(class, ListRef::EMPTY)), self.b.t_any_ref);
        }
        if class == self.b.array {
            if let [(l, false)] = args {
                if let [len] = ast.expr_list(*l) {
                    return self.new_array(targs, *len, span, expected);
                }
            }
        }
        // `new C` before the lists of a constructor whose first clause is a using clause
        // (`new C(using q)(a)`, and a jar body's `new C(q)(a)` once its given list is
        // dropped): nothing is written for the first clause.
        let args = match args {
            [(first, false), rest @ ..] if first.is_empty() && !rest.is_empty() && !first_written => {
                self.complete_class(class);
                match self.syms.class(class).ctor.first() {
                    Some(clause) if clause.is_using && !clause.params.is_empty() => rest,
                    _ => args,
                }
            }
            _ => args,
        };
        let lists: Vec<ArgList> = args
            .iter()
            .map(|&(l, using)| ArgList {
                args: ast.expr_list(l).iter().map(|&a| ArgSrc::Ast(a)).collect(),
                using,
                span,
            })
            .collect();
        // `new String(chars, offset, count)` and the other constructors of the builtin string
        // are the platform layer's `java.lang.String.newString`.
        if class == self.b.string {
            if let Some(c) = self.java_lang_object("String") {
                let recv = self.prog.add(TExpr::Module(c));
                let recv_ty = self.types.class(c, &[]);
                let name = self.interner.intern("newString");
                return self.apply_member(recv, recv_ty, name, None, lists, span, expected);
            }
        }
        self.apply_callee(super::apply::Callee::Ctor(class), targs, lists, span, expected)
    }

    fn new_arg_lists(&self, args: &[(ListRef, bool)], span: Span) -> Vec<ArgList> {
        let ast = self.cur_ast();
        args.iter().map(|&(l, using)| ArgList { args: ast.expr_list(l).iter().map(|&a| ArgSrc::Ast(a)).collect(), using, span }).collect()
    }

    /// `new Array[T](n)`: `n` elements holding the zero of `T`, through `scala.newArray`.
    fn new_array(&mut self, targs: Option<ListRef>, len: ExprId, span: Span, expected: Option<TypeId>) -> (TExprId, TypeId) {
        let elem = match targs.map(|l| self.cur_ast().ty_list(l).to_vec()) {
            Some(ts) if ts.len() == 1 => self.resolve_type(ts[0]),
            _ => {
                self.error(span, "new Array takes one type argument");
                ERROR
            }
        };
        let name = self.interner.intern("newArray");
        let Some(sym) = self.syms.pkg(self.b.scala_pkg).entries.get(&name).and_then(|e| e.term) else {
            self.error(span, "Array cannot be instantiated without the standard library");
            return (self.prog.add(TExpr::Unit), ERROR);
        };
        if let Some(made) = self.new_array_by_class_tag(elem, len, span) {
            return made;
        }
        let zero = self.zero_of(elem).unwrap_or_else(|| self.prog.add(TExpr::Null));
        let lists = vec![ArgList { args: vec![ArgSrc::Ast(len), ArgSrc::Typed(zero, elem)], using: false, span }];
        let callee = super::apply::Callee::Method { recv: None, sym, owner_subst: Vec::new(), prefix: None };
        self.apply_callee(callee, None, lists, span, expected)
    }

    /// `new Array[T](n)` for an element type only a `ClassTag` knows is scalac's
    /// `evidence.newArray(n)`: the array's kind and its zero come from the tag at run time.
    /// Outside the JVM, where evidence is erased and an array has no kind, the tag says the
    /// zero alone and only a type parameter needs one: any other element type is known where
    /// the array is made.
    fn new_array_by_class_tag(&mut self, elem: TypeId, len: ExprId, span: Span) -> Option<(TExprId, TypeId)> {
        let known = self.deref(elem);
        if matches!(self.types.get(known), Type::Class(..) | Type::Lit(_) | Type::Any | Type::Nothing) {
            return None;
        }
        if !self.jvm && !matches!(self.types.get(known), Type::Param(_) | Type::AppParam(..)) {
            return None;
        }
        let tag_class = self.sites.class_tag?;
        let target = self.types.class(tag_class, &[elem]);
        let Some(evidence) = self.resolve_given(target, span) else {
            let file = self.env.file;
            if !(self.source(file).is_std || self.in_jar(file) || self.is_body_file(file)) {
                let msg = format!("No ClassTag available for {}", self.show(elem));
                self.error(span, msg);
            }
            return None;
        };
        let name = self.interner.intern("newArray");
        let sym = *self.syms.class(tag_class).members.get(&name)?;
        let int = self.b.t_int;
        let (len, _) = self.type_expr(len, Some(int));
        let args = self.prog.list(&[len]);
        let array = self.types.class(self.b.array, &[elem]);
        let call = super::apply::MethodCall { recv: Some(evidence), sym, owner_subst: Vec::new(), ext_recv: None, prefix: None };
        let call = self.build_call(&call, args);
        self.prog.set_type(call, array);
        Some((call, array))
    }

    /// The class `this` names here: the innermost enclosing class that is no SAM lambda's,
    /// since a lambda has no `this` of its own.
    pub fn this_class(&self) -> Option<ClassId> {
        self.env.frames.iter().rev().find_map(|f| match f {
            Frame::Class(c) if !self.sam_classes.contains_key(c) => Some(*c),
            _ => None,
        })
    }

    /// `C.this` of an enclosing class; inside an inline expansion the receiver, with its type.
    pub(super) fn type_this_of(&mut self, c: ClassId) -> (TExprId, TypeId) {
        let this_ty = match self.inline_this_of(c) {
            Some(t) => t,
            None => self.syms.this_type(c),
        };
        (self.this_ref(c), this_ty)
    }

    /// The class `C.this` names: the innermost enclosing class called `name`.
    pub fn qualified_this_class(&self, name: Name) -> Option<ClassId> {
        self.env.frames.iter().rev().find_map(|f| match f {
            Frame::Class(c) if self.syms.class(*c).name == name => Some(*c),
            _ => None,
        })
    }

    /// Keeps the `self =>` alias of the class's definition on the class, where the lookup of
    /// every name reads it.
    pub(super) fn record_self_alias(&mut self, c: ClassId) {
        let info = self.syms.class(c);
        let alias = match info.def.map(|d| &self.ast(info.file).def(d).kind) {
            Some(ast::DefKind::Class(cls)) => cls.self_alias,
            Some(ast::DefKind::Given(g)) => g.self_alias,
            _ => return,
        };
        self.syms.class_mut(c).self_alias = alias;
    }

    /// The innermost class under typing, a SAM lambda's included: whose instance `this` is at
    /// run time.
    pub fn innermost_class(&self) -> Option<ClassId> {
        self.env.frames.iter().rev().find_map(|f| match f {
            Frame::Class(c) => Some(*c),
            _ => None,
        })
    }

    /// Scala's harmonisation of the branches of an `if` or `match` and of vararg elements: when
    /// every type is a primitive numeric one and the branches that are not `Int` literals all
    /// have one class, the literals become that class; any other mix is left to the union.
    pub fn harmonize_literals(&mut self, branches: &mut [(TExprId, TypeId)]) {
        // A plain call of a constant type still pending is its literal here, as scalac's typer
        // reads the constant type; one of `Int` stays a call.
        if self.attempts.pending_len() != 0 {
            for &(te, _) in branches.iter() {
                self.expand_constant_pending(te);
            }
        }
        let mut target = None;
        let mut has_literal = false;
        for &(te, ty) in branches.iter() {
            let ty = self.deref(ty);
            if self.is_numeric(ty).is_none() {
                return;
            }
            if ty == self.b.t_int && self.int_literal(te).is_some() {
                has_literal = true;
                continue;
            }
            match target {
                Some(t) if t != ty => return,
                _ => target = Some(ty),
            }
        }
        let Some(t) = target.filter(|&t| has_literal && t != self.b.t_int) else { return };
        if matches!(self.is_numeric(t), Some(super::prims::R_BYTE | super::prims::R_SHORT)) {
            return;
        }
        for (te, ty) in branches.iter_mut() {
            let Some((node, v)) = self.int_literal(*te) else { continue };
            self.prog.exprs[node.idx()] = if t == self.b.t_long {
                TExpr::Long(v as i64)
            } else if t == self.b.t_double {
                TExpr::Double(v as f64)
            } else if t == self.b.t_float {
                TExpr::Double(v as f32 as f64)
            } else {
                TExpr::Char(v as u16)
            };
            self.prog.set_type(node, t);
            *ty = t;
        }
    }

    /// The pending call of a constant type that `te` is, negates or ends as a block, expanded.
    fn expand_constant_pending(&mut self, te: TExprId) {
        let call = match self.prog.expr(te) {
            TExpr::Unary(_, inner) => inner,
            TExpr::Block(_, result) => return self.expand_constant_pending(result),
            _ => te,
        };
        if self.pending_type(call).and_then(|t| self.fold_type(t)).is_some() {
            self.expand_pending_in(&[call]);
        }
    }

    /// The node to rewrite and the value of an `Int` literal, also negated or ending a block.
    fn int_literal(&self, te: TExprId) -> Option<(TExprId, i32)> {
        match self.prog.expr(te) {
            TExpr::Int(v) => Some((te, v)),
            TExpr::Unary(UnOp::IntNeg, inner) => match self.prog.expr(inner) {
                TExpr::Int(v) => Some((te, v.wrapping_neg())),
                _ => None,
            },
            TExpr::Block(_, result) => self.int_literal(result),
            _ => None,
        }
    }

    /// Vararg elements typed against the open variable `pty` harmonise like branches; the `Int`
    /// bounds the literals recorded (from `lower_mark` on) then stand for the class they became.
    pub fn harmonize_varargs(&mut self, items: &[TExprId], pty: TypeId, lower_mark: usize) {
        let open = self.deref(pty);
        let Type::Var(v) = self.types.get(open) else { return };
        // Every numeric element left a pending widening; anything else rules harmonisation out.
        let pending = |t: &Self, te: TExprId| t.pending_widenings.iter().rev().find(|&&(e, _, _)| e == te).copied();
        if items.iter().any(|&te| pending(self, te).is_none()) {
            return;
        }
        let mut branches: Vec<(TExprId, TypeId)> =
            items.iter().map(|&te| (te, pending(self, te).map_or(ERROR, |(_, actual, _)| actual))).collect();
        self.harmonize_literals(&mut branches);
        let t_int = self.b.t_int;
        for &(te, ty) in &branches {
            if ty == t_int {
                continue;
            }
            if let Some(entry) = self.pending_widenings.iter_mut().rev().find(|(e, _, _)| *e == te) {
                if entry.1 == t_int {
                    entry.1 = ty;
                    for l in self.tvars[v].lower[lower_mark..].iter_mut() {
                        if *l == t_int {
                            *l = ty;
                        }
                    }
                }
            }
        }
    }

    fn type_assign(&mut self, lhs: ExprId, rhs: ExprId, span: Span) -> (TExprId, TypeId) {
        let ast = self.cur_ast();
        if let Expr::Apply(f, args) = ast.expr(lhs) {
            let (tf, fty) = self.type_expr(f, None);
            let mut items: Vec<ArgSrc> = ast.expr_list(args).iter().map(|&a| ArgSrc::Ast(a)).collect();
            items.push(ArgSrc::Ast(rhs));
            let lists = vec![ArgList { args: items, using: false, span }];
            return self.apply_member(tf, fty, names::UPDATE, None, lists, span, None);
        }
        let (tl, lty) = match ast.expr(lhs) {
            Expr::Select(q, name) => match self.select_for_assignment(lhs, q, name, rhs, span) {
                Ok(setter_call) => return setter_call,
                Err(selected) => selected,
            },
            Expr::Ident(name) => {
                let typed = self.type_expr(lhs, None);
                if let Some(setter_call) = self.unqualified_setter_call(typed.0, name, rhs, span) {
                    return setter_call;
                }
                typed
            }
            _ => self.type_expr(lhs, None),
        };
        // `dynamic.name = v` writes the property.
        if let TExpr::JsSelect(..) = self.prog.expr(tl) {
            let tr = self.check_expr(rhs, ANY);
            return (self.prog.add(TExpr::Assign(tl, tr)), self.b.t_unit);
        }
        let target = match self.prog.expr(tl) {
            TExpr::Local(s) | TExpr::Static(s) | TExpr::Field(_, s) => Some(s),
            _ => None,
        };
        match target {
            Some(s) if self.syms.sym(s).kind == SymKind::Var => {}
            Some(s) => {
                let msg = format!("reassignment to val {}", self.name_str(self.syms.sym(s).name));
                self.error(span, msg);
            }
            None => self.error(span, "this expression cannot be assigned to"),
        }
        let tr = self.check_expr(rhs, lty);
        (self.assignment(tl, tr), self.b.t_unit)
    }

    /// `[T] => (x: A) => body`: the lambda typed under its own type parameters, of the bounds
    /// it writes, against the function type an expected polymorphic function type gives,
    /// renamed to them, which gives the parameters' and the result's types alone (dotty's
    /// `typedPolyFunctionValue`, `makeClosure(tparams, ..)`); the expected type's bounds are
    /// what the literal's type conforms to. Its value is the lambda's, as the type erases to
    /// the function type.
    fn type_poly_lambda(&mut self, e: ExprId, tparams: ListRef, lambda: ExprId, span: Span, expected: Option<TypeId>) -> (TExprId, TypeId) {
        let names: Vec<Name> = self.cur_ast().name_lists[tparams.range()].to_vec();
        let bounds = self.cur_ast().poly_lambda_bounds.iter().find(|(l, _)| *l == e).map(|&(_, bl)| bl);
        let (ids, ps) = self.poly_params(&names, bounds);
        if self.index.is_some() {
            self.index_poly_tparams(span, &names, &ids);
        }
        let exp_fun = expected.and_then(|e| {
            let e = self.deref(e);
            let (qs, fun) = self.poly_binders(e)?;
            if qs.len() != ps.len() {
                return None;
            }
            let renaming: Subst = qs.iter().zip(&ps).map(|(&q, &p)| (q, p)).collect();
            Some(self.types.subst(fun, &renaming))
        });
        let outer = std::mem::replace(&mut self.poly_literal_fun, exp_fun);
        let (te, fun_ty) = match self.cur_ast().expr(lambda) {
            Expr::Lambda(params, body) => self.type_lambda(params, body, span, exp_fun),
            _ => self.type_expr(lambda, exp_fun),
        };
        self.poly_literal_fun = outer;
        self.env.frames.pop();
        let fun_ty = self.solve_in(fun_ty);
        // The literal's `apply` takes its own parameters, with their names (dotty's
        // `Typer.typedPolyFunctionValue`).
        let fun_ty = match self.prog.expr(te) {
            TExpr::Lambda(pl, _) => {
                let syms = self.prog.sym_list(pl).to_vec();
                self.literal_apply(fun_ty, &syms, span)
            }
            _ => fun_ty,
        };
        (te, self.poly_type(&ps, fun_ty))
    }

    /// Whether a function literal of a parameter without a type is typed where a member's
    /// application asks that it be left untyped (`untyped_lambda`): dotty's test leaves it
    /// untyped only where the formal gives the parameter no type, which a function type of its
    /// arity, a SAM, a partial function or a dependent function does, through a context function
    /// to its result (`functionWithUnknownParamType`, ProtoTypes.scala 475); a `{ case … }`
    /// literal and one with a contextual parameter are typed.
    fn lambda_kept_typed(&mut self, lps: &[ast::LambdaParam], given: Option<TypeId>) -> bool {
        if (lps.len() == 1 && lps[0].name == names::CASE_PARAM) || lps.iter().any(|p| p.contextual) {
            return true;
        }
        let Some(mut pt) = given else { return false };
        while let Some((_, ret)) = self.as_context_function(pt) {
            pt = ret;
        }
        let expected = self.function_bound(pt, lps.len());
        self.named_function(expected).is_some() || self.sam_method(expected, lps.len()).is_some() || self.as_partial_function(expected).is_some() || self.expected_function(expected, lps.len()).is_some()
    }

    fn type_lambda(
        &mut self,
        params: ListRef,
        body: ExprId,
        span: Span,
        expected: Option<TypeId>,
    ) -> (TExprId, TypeId) {
        let ast = self.cur_ast();
        let lps: Vec<ast::LambdaParam> = ast.lambda_params[params.range()].to_vec();
        // A member's application asking that a literal the formal gives no parameter types be
        // left untyped (`Worker::untyped_lambda`); a literal nested in this one is not asked.
        let untyped_ok = self.untyped_lambda.take().is_some();
        let given = expected;
        let contextual = !lps.is_empty() && lps.iter().all(|p| p.contextual);
        if contextual {
            return self.type_context_lambda(&lps, body, span, expected);
        }
        let expected = expected.map(|t| self.function_bound(t, lps.len()));
        if let Some(exp) = expected {
            self.check_curried_dependent(exp, span);
        }
        let named_any = expected.and_then(|t| self.named_function(t)).filter(|f| !f.ctx);
        let named_takes_cases = named_any.as_ref().map_or(false, |f| lps.len() == 1 && lps[0].name == names::CASE_PARAM && f.params.len() > 1);
        let named = named_any.filter(|f| f.params.len() == lps.len() || named_takes_cases);
        if let (Some(exp), None) = (expected, &named) {
            if let Some((trait_ty, method, sig, subst)) = self.sam_method(exp, lps.len()) {
                return self.sam_lambda(trait_ty, method, sig, subst, &lps, body, span);
            }
        }
        // A partial function of a tuple takes a lambda of the tuple's elements too, whose
        // parameters are bound from the one tuple parameter, as scalac's desugaring does.
        let exp_partial = expected
            .and_then(|t| self.as_partial_function(t))
            .filter(|&(a, _)| lps.len() == 1 || self.tuple_arity(a) == Some(lps.len()));
        let mut exp_fn = match exp_partial {
            Some((a, b)) => Some((vec![a], b)),
            None => expected.and_then(|t| self.expected_function(t, lps.len())),
        };
        self.push_scope();

        // `case (a, b) => ...` where a function of several parameters is expected: the
        // parameters are packed into the tuple that the cases match on.
        let expected_arity = exp_fn.as_ref().map_or(0, |(p, _)| p.len());
        if lps.len() == 1 && lps[0].name == names::CASE_PARAM && expected_arity > 1 {
            let ptys: Vec<TypeId> = exp_fn.as_ref().unwrap().0.clone();
            let ptys: Vec<TypeId> = ptys.iter().map(|&t| self.solve_if_var(t)).collect();
            let mut params = Vec::with_capacity(ptys.len());
            let mut refs = Vec::with_capacity(ptys.len());
            for (i, &t) in ptys.iter().enumerate() {
                let s = self.indexed_local("c", i as u32, t, span);
                params.push(s);
                refs.push(self.prog.add(TExpr::Local(s)));
            }
            let tuple_ty = self.tuple_type(&ptys);
            let tuple_sym = self.new_local(names::CASE_PARAM, SymKind::Val, tuple_ty, span);
            self.bind_local(names::CASE_PARAM, tuple_sym);
            if let (Some(f), Some(fun)) = (named.as_ref().filter(|f| f.params.len() == params.len()), exp_fn.as_mut()) {
                let paths: Vec<TypeId> = params.iter().map(|&s| self.types.mk(Type::Term(s))).collect();
                fun.1 = self.instantiate_named(f, &paths);
            }
            let ret_expected = exp_fn.as_ref().and_then(|&(_, r)| self.concrete_expected(Some(r)));
            let mut body_own = None;
            let (tb, bty) = match ret_expected {
                Some(r) => {
                    let (tb, own) = self.check_expr_own(body, r);
                    body_own = Some(own);
                    (tb, r)
                }
                None => self.type_expr(body, None),
            };
            self.pop_scope();
            let class = self.tuple_class(ptys.len());
            let packed = match (ptys.len() > 22).then(|| self.tuple_xxl_value(&refs, span)).flatten() {
                Some(te) => te,
                None => {
                    let l = self.prog.list(&refs);
                    self.prog.add(TExpr::New(class, l))
                }
            };
            if self.capturing() && ptys.len() <= 22 {
                self.capture_targs(packed, &ptys);
            }
            let stmts = self.prog.stmts.push_slice(&[TStmt::Val(tuple_sym, packed)]);
            let block = self.prog.add(TExpr::Block(stmts, tb));
            let ty = self.closure_type(false, &params, &ptys, bty, span);
            let pl = self.prog.syms(&params);
            let lambda = self.prog.add(TExpr::Lambda(pl, block));
            self.note_erroneous_lambda(lambda, body_own);
            return (lambda, ty);
        }
        // `pairs.map((a, b) => ...)`: a single tuple parameter is destructured.
        let mut untupled: Option<(SymId, ClassId)> = None;
        let mut param_tys: Vec<TypeId> = Vec::with_capacity(lps.len());
        let mut fn_param_tys: Vec<TypeId> = Vec::new();
        if let Some((ptys, _)) = &exp_fn {
            if ptys.len() == 1 && lps.len() > 1 {
                let pt = self.solve_if_var(ptys[0]);
                if let Type::Class(c, args) = self.types.get(pt) {
                    let elems = self.types.items(args).to_vec();
                    if self.is_tuple_class(c) && elems.len() == lps.len() {
                        if let Some(tys) = self.untupled_param_types(ast, &lps, &elems) {
                            let tuple_sym = self.fresh_local("t", pt, span);
                            untupled = Some((tuple_sym, c));
                            param_tys = tys;
                            fn_param_tys = vec![pt];
                        }
                    }
                }
            }
        }
        // Parameters that do not take the tuple's elements make a function of several.
        let exp_partial = exp_partial.filter(|_| lps.len() == 1 || untupled.is_some());
        let mut missing = false;
        if untupled.is_none() {
            for (i, lp) in lps.iter().enumerate() {
                let ty = match lp.ty {
                    // A library body's lambda typed `(fa: => F[Unit]) => ..` takes a thunk.
                    Some(t) => {
                        let declared = match ast.ty(t) {
                            crate::ast::TyExpr::ByName(inner) => {
                                let inner = self.resolve_type(inner);
                                self.by_name_type(inner)
                            }
                            _ => self.resolve_type(t),
                        };
                        let offered = exp_fn.as_ref().filter(|(p, _)| p.len() == lps.len()).map(|(p, _)| p[i]);
                        match offered {
                            Some(p) if self.in_converted_body() && ast.inferred_types.binary_search(&t.0).is_ok() && !self.admits(p, declared) => self.lambda_param_type(p),
                            _ => declared,
                        }
                    }
                    None => match exp_fn.as_ref().and_then(|(p, _)| p.get(i).copied()) {
                        Some(t) if exp_fn.as_ref().map_or(false, |(p, _)| p.len() == lps.len()) => {
                            self.lambda_param_type(t)
                        }
                        _ => match self.param_type_from_body(body, lp.name) {
                            Some(t) => t,
                            None => {
                                self.error_unless_unknown(lp.span, "missing parameter type".to_string(), &expected.into_iter().collect::<Vec<_>>());
                                missing = true;
                                ERROR
                            }
                        },
                    },
                };
                param_tys.push(ty);
            }
            fn_param_tys = param_tys.clone();
        }
        // dotty's applicability test leaves such a literal untyped (`functionWithUnknownParamType`,
        // ProtoTypes.scala 475); the application reports the missing types and types none of it,
        // so that the retry on the qualifier types it for the first time (`apply::type_arg_recorded`).
        if missing && untyped_ok && !self.lambda_kept_typed(&lps, given) {
            self.pop_scope();
            self.untyped_lambda = Some(true);
            return (self.prog.add(TExpr::Unit), ERROR);
        }
        let mut syms = Vec::with_capacity(lps.len());
        for (lp, &ty) in lps.iter().zip(&param_tys) {
            // A by-name parameter `=> A` of the function type is a thunk the body evaluates
            // where it reads the parameter.
            let by_name = self.by_name_arg(ty);
            let ty = by_name.unwrap_or(ty);
            // The body sees the parameter's wildcards captured; the function type keeps them.
            let captured = self.capture_wildcards(ty);
            let sym = self.new_local(lp.name, SymKind::Val, captured, lp.span);
            if by_name.is_some() {
                self.syms.sym_mut(sym).by_name = true;
            }
            self.bind_local(lp.name, sym);
            if lp.implicit {
                self.bind_given(sym);
            }
            syms.push(sym);
        }
        // Expected to name its parameters, the result names the lambda's own.
        if let (Some(f), Some(fun), None) = (named.as_ref().filter(|f| f.binders.len() == syms.len()), exp_fn.as_mut(), untupled) {
            let paths: Vec<TypeId> = syms.iter().map(|&s| self.types.mk(Type::Term(s))).collect();
            fun.1 = self.instantiate_named(f, &paths);
        }
        let ret_expected = exp_fn.as_ref().and_then(|&(_, r)| self.concrete_expected(Some(r)));
        // The cases of a partial function literal: `{ case ... }` or `x => x match { case ... }`.
        // A lambda whose body is a match, under an expected partial function, is one: its
        // cases decide `isDefinedAt`, whatever the scrutinee (`x => (x.a, x.b) match { .. }`
        // given to `collect`), as scalac's `ExpandSAMs` makes it.
        let shape = match exp_partial {
            Some(_) => self.partial_body(ast, body, untupled.is_some(), span),
            None => PartialBody::Other,
        };
        let matches_param = shape == PartialBody::Match;
        self.partial_match = matches_param;
        let match_mark = self.deferred_matches.len();
        let mut body_own = None;
        let (mut tb, bty) = match ret_expected {
            Some(r) => {
                let (tb, own) = self.check_expr_own(body, r);
                body_own = Some(own);
                (tb, r)
            }
            None => {
                let ret = exp_fn.as_ref().map(|&(_, r)| r);
                let (tb, bty) = self.type_expr(body, ret);
                let body_span = self.cur_ast().expr_span(body);
                let (tb, bty) = self.widen_lambda_result(tb, bty, ret, body_span);
                (tb, if ret.is_none() { self.widen_soft(tb, bty) } else { bty })
            }
        };
        self.pop_scope();
        if let PartialBody::BlockMatch(m, block) = shape {
            // The typed match: the body of a lambda of a tuple's elements, which binds them
            // around it below, or the result of the block.
            let typed = match (untupled, self.prog.expr(tb)) {
                (Some(_), _) => tb,
                (None, TExpr::Block(_, r)) => r,
                _ => tb,
            };
            // The lambda's own parameters are the closure's, outside the block; those bound from
            // a tuple are the block's vals.
            let names: Vec<Name> = match (untupled, ast.expr(block)) {
                (Some(_), _) => lps.iter().map(|p| p.name).collect(),
                (None, Expr::Block(stmts)) => ast
                    .stmt_list(stmts)
                    .iter()
                    .filter_map(|st| match st {
                        Stmt::Def(d) => Some(ast.def(*d).name),
                        _ => None,
                    })
                    .collect(),
                _ => Vec::new(),
            };
            let own = if untupled.is_some() { Vec::new() } else { syms.clone() };
            let locals = BlockLocals { outer: span, inner: ast.expr_span(m), own, names };
            if matches!(self.prog.expr(typed), TExpr::Match(..)) && !self.result_leaks(ast, m, typed, &locals) {
                self.match_not_partial(m, match_mark);
            }
        }
        let lambda_params = match untupled {
            Some((tuple_sym, class)) => {
                let fields: Vec<SymId> = self.syms.class(class).ctor_syms.concat();
                let elems: Vec<TypeId> = match self.types.get(fn_param_tys[0]) {
                    Type::Class(_, args) => self.types.items(args).to_vec(),
                    _ => Vec::new(),
                };
                let mut stmts = Vec::with_capacity(syms.len());
                for (i, &s) in syms.iter().enumerate() {
                    let t = self.prog.add(TExpr::Local(tuple_sym));
                    let f = if elems.len() > 22 { self.tuple_element(t, &fields, &elems, i, span) } else { self.prog.add(TExpr::Field(t, fields[i])) };
                    stmts.push(TStmt::Val(s, f));
                }
                let l = self.prog.stmts.push_slice(&stmts);
                tb = self.prog.add(TExpr::Block(l, tb));
                vec![tuple_sym]
            }
            None => syms,
        };
        // An untupled body binds its parameters before anything else, so that the partial
        // function is defined everywhere and a `match` in the body throws, as under scalac.
        if exp_partial.is_some() {
            return self.partial_function_literal(lambda_params[0], fn_param_tys[0], tb, bty, matches_param, span);
        }
        let ty = match untupled {
            Some(_) => self.fun_type(&fn_param_tys, bty),
            None => self.closure_type(false, &lambda_params, &fn_param_tys, bty, span),
        };
        let l = self.prog.syms(&lambda_params);
        let lambda = self.prog.add(TExpr::Lambda(l, tb));
        self.note_erroneous_lambda(lambda, body_own);
        (lambda, ty)
    }

    /// `(x: A) ?=> body`: a context function literal, its parameters givens inside the body.
    fn type_context_lambda(&mut self, lps: &[ast::LambdaParam], body: ExprId, span: Span, expected: Option<TypeId>) -> (TExprId, TypeId) {
        let mut exp_fn = expected.and_then(|t| self.as_context_function(t)).filter(|(p, _)| p.len() == lps.len());
        if let Some(exp) = expected {
            self.check_curried_dependent(exp, span);
        }
        let named = expected.and_then(|t| self.named_function(t)).filter(|f| f.ctx && f.params.len() == lps.len());
        self.push_scope();
        let mut syms = Vec::with_capacity(lps.len());
        let mut param_tys = Vec::with_capacity(lps.len());
        for (i, lp) in lps.iter().enumerate() {
            let ty = match lp.ty {
                Some(t) => self.resolve_type(t),
                None => match exp_fn.as_ref().map(|(p, _)| p[i]) {
                    Some(t) => self.lambda_param_type(t),
                    None => {
                        self.error_unless_unknown(lp.span, "missing parameter type".to_string(), &expected.into_iter().collect::<Vec<_>>());
                        ERROR
                    }
                },
            };
            let sym = self.new_local(lp.name, SymKind::Val, ty, lp.span);
            self.bind_local(lp.name, sym);
            self.bind_given(sym);
            syms.push(sym);
            param_tys.push(ty);
        }
        if let (Some(f), Some(fun)) = (&named, exp_fn.as_mut()) {
            let paths: Vec<TypeId> = syms.iter().map(|&s| self.types.mk(Type::Term(s))).collect();
            fun.1 = self.instantiate_named(f, &paths);
        }
        let ret_expected = exp_fn.as_ref().and_then(|&(_, r)| self.concrete_expected(Some(r)));
        let mut body_own = None;
        let (tb, bty) = match ret_expected {
            Some(r) => {
                let (tb, own) = self.check_expr_own(body, r);
                body_own = Some(own);
                (tb, r)
            }
            None => {
                let ret = exp_fn.as_ref().map(|&(_, r)| r);
                let (tb, bty) = self.type_expr(body, ret);
                let body_span = self.cur_ast().expr_span(body);
                self.widen_lambda_result(tb, bty, ret, body_span)
            }
        };
        self.pop_scope();
        let ty = self.closure_type(true, &syms, &param_tys, bty, span);
        let l = self.prog.syms(&syms);
        let lambda = self.prog.add(TExpr::Lambda(l, tb));
        self.note_erroneous_lambda(lambda, body_own);
        (lambda, ty)
    }

    /// The result of a lambda typed against an open variable widens to the numeric upper bound
    /// that the expected type put on the variable, as an argument would: `o.map(_.value)` for an
    /// `Option[Long]` maps an `Int` field to a `Long`; and converts into a class bound the
    /// variable has, as scalac adapts the body: `one(x => "s")` against a `Shown`.
    pub(in crate::typer) fn widen_lambda_result(&mut self, te: TExprId, ty: TypeId, expected: Option<TypeId>, span: Span) -> (TExprId, TypeId) {
        let Some(exp) = expected else { return (te, ty) };
        let head = self.deref(exp);
        let Type::Var(v) = self.types.get(head) else { return self.convert_lambda_result(te, ty, exp, span) };
        let act = self.deref(ty);
        let act = self.widen_lit(act);
        let mark = self.snapshot();
        let fits = self.is_sub(ty, exp);
        self.rollback(mark);
        if fits {
            return (te, ty);
        }
        if self.is_numeric(act).is_some() {
            if let Some(widened) = self.widen_to_bound(te, act, v, exp) {
                return (widened, self.numeric_upper_bound(v).unwrap_or(ty));
            }
        }
        if let Some(discarded) = self.discard_to_unit_bound(te, ty, v, exp, span) {
            return discarded;
        }
        match self.convert_to_var_bound_typed(te, ty, v, exp, span) {
            Some(converted) => converted,
            None => (te, ty),
        }
    }

    /// A variable bounded above by `Unit` is instantiated to it, and the result discarded, as
    /// scalac adapts the body again once the variable is `Unit`: `req(s => s.isEmpty)` for a
    /// `Box[Unit]` of a `req[A](run: String => A): Box[A]`.
    fn discard_to_unit_bound(&mut self, te: TExprId, ty: TypeId, v: TVarId, exp: TypeId, span: Span) -> Option<(TExprId, TypeId)> {
        let unit = self.b.t_unit;
        let bounded = (0..self.tvars[v].upper.len()).any(|i| {
            let u = self.tvars[v].upper[i];
            self.deref(u) == unit
        });
        if !bounded {
            return None;
        }
        let mark = self.snapshot();
        if !self.is_sub(unit, exp) {
            self.rollback(mark);
            return None;
        }
        Some((self.adapt(te, ty, unit, span), unit))
    }

    /// The body of an eta-expansion converted to the result type a function type expects
    /// (`val g: String => Node = show` for a `show(a: String): String` beside a conversion to
    /// `Node`), as scalac types the expansion's body against it.
    fn convert_lambda_result(&mut self, te: TExprId, ty: TypeId, exp: TypeId, span: Span) -> (TExprId, TypeId) {
        let mark = self.snapshot();
        let fits = self.is_sub(ty, exp);
        self.rollback(mark);
        if fits || self.types.has_vars(exp) {
            return (te, ty);
        }
        // A function type with a `Unit` result discards the value (`attach(runner.receive _)`
        // where a `String => Unit` is expected of a method answering an `Option`).
        let unit = self.b.t_unit;
        if self.dealias(exp) == unit {
            return (self.adapt(te, ty, unit, span), unit);
        }
        match self.convert_to_typed(te, ty, exp, span, false) {
            Some(converted) => converted,
            None => (te, ty),
        }
    }

    /// `newPartialFunction((x, default) => ..., x => ...)`: the cases with `default(x)` as the
    /// last resort, and the patterns and guards of the same cases without their bodies.
    fn partial_function_literal(
        &mut self,
        param: SymId,
        param_ty: TypeId,
        body: TExprId,
        body_ty: TypeId,
        matches_param: bool,
        span: Span,
    ) -> (TExprId, TypeId) {
        let default_ty = self.fun_type(&[param_ty], body_ty);
        let default = self.fresh_local("d", default_ty, span);
        let yes = self.prog.add(TExpr::Bool(true));
        let mut inner = body;
        while let TExpr::Block(stmts, res) = self.prog.expr(inner) {
            if stmts.len != 0 {
                break;
            }
            inner = res;
        }
        let (total, defined) = match self.prog.expr(inner) {
            TExpr::Match(scrut, cases) if matches_param => {
                let cases: Vec<TCase> = self.prog.cases[cases.range()].to_vec();
                let wildcard = self.prog.add_pat(TPat::Wildcard);
                let arg = self.prog.add(TExpr::Local(param));
                let args = self.prog.list(&[arg]);
                let callee = self.prog.add(TExpr::Local(default));
                let fallback = self.prog.add(TExpr::CallClosure(callee, args));
                let mut total = cases.clone();
                total.push(TCase { pat: wildcard, guard: None, body: fallback });
                let no = self.prog.add(TExpr::Bool(false));
                let mut tests: Vec<TCase> = cases.iter().map(|c| TCase { body: yes, ..*c }).collect();
                tests.push(TCase { pat: wildcard, guard: None, body: no });
                let total = self.prog.cases.push_slice(&total);
                let tests = self.prog.cases.push_slice(&tests);
                let scrut_again = self.copy_expr(scrut);
                (self.prog.add(TExpr::Match(scrut, total)), self.prog.add(TExpr::Match(scrut_again, tests)))
            }
            _ => (body, yes),
        };
        let total_params = self.prog.syms(&[param, default]);
        let defined_params = self.prog.syms(&[param]);
        let apply_or_else = self.prog.add(TExpr::Lambda(total_params, total));
        let is_defined_at = self.prog.add(TExpr::Lambda(defined_params, defined));
        let args = self.prog.list(&[apply_or_else, is_defined_at]);
        let template = self.prog.add_str("$pf($0, $1)");
        let ty = match self.partial_function_class() {
            Some(c) => self.types.class(c, &[param_ty, body_ty]),
            None => ERROR,
        };
        let te = self.prog.add(TExpr::Js(template, args));
        if self.capturing() {
            self.capture_form(te, Form::PartialFunction);
        }
        (te, ty)
    }

    /// The arity of the tuple type `t`, where it is one.
    fn tuple_arity(&mut self, t: TypeId) -> Option<usize> {
        let t = self.solve_if_var(t);
        match self.types.get(t) {
            Type::Class(c, args) if self.is_tuple_class(c) => Some(self.types.items(args).len()),
            _ => None,
        }
    }

    /// The parameter types of a lambda of a tuple's elements `elems`, or `None` where a declared
    /// one does not take its element (scalac's `ptIsCorrectProduct`): the declared type where
    /// there is one, the element's where there is none.
    fn untupled_param_types(&mut self, ast: &ast::Ast, lps: &[ast::LambdaParam], elems: &[TypeId]) -> Option<Vec<TypeId>> {
        let mut tys = Vec::with_capacity(elems.len());
        for (lp, &elem) in lps.iter().zip(elems) {
            let Some(t) = lp.ty else {
                tys.push(elem);
                continue;
            };
            let declared = self.resolve_type(t);
            // A library body's inferred type yields to the element, as in `type_lambda`.
            let inferred = self.in_converted_body() && ast.inferred_types.binary_search(&t.0).is_ok();
            if self.admits(elem, declared) {
                tys.push(declared);
            } else if inferred {
                tys.push(elem);
            } else {
                return None;
            }
        }
        Some(tys)
    }

    /// What the body of a lambda an expected partial function takes is to the partial function
    /// (dotc's `ExpandSAMs.partialFunRHS`): a match, in braces or not, whose cases are the
    /// function's; a block of statements that ends in a match, which is not; anything else.
    /// The lambda of a tuple's elements binds them in a block around its body
    /// (`desugar.makeTupledFunction`), so its match is such a match, and one in braces, a block
    /// in that block, neither: the braces the parser drops are read from the text between the
    /// body and the end of the lambda at `span`, as `apply::bare_in_list` reads them. So for a
    /// block whose last statement is a match in braces of its own (`{ val k = 1; { s match .. } }`),
    /// from the text before the match.
    fn partial_body(&self, ast: &ast::Ast, body: ExprId, untupled: bool, span: Span) -> PartialBody {
        if untupled {
            if !matches!(ast.expr(body), Expr::Match(..)) {
                return PartialBody::Other;
            }
            let text = &self.source(self.env.file).text;
            let mut at = ast.expr_span(body).end;
            loop {
                at = super::signature::skip_trivia(text, at, span.end);
                match text.as_bytes().get(at as usize) {
                    Some(b')') if at < span.end => at += 1,
                    Some(b'}') if at < span.end => return PartialBody::Other,
                    _ => return PartialBody::BlockMatch(body, body),
                }
            }
        }
        let mut e = body;
        loop {
            match ast.expr(e) {
                Expr::Match(..) => return PartialBody::Match,
                Expr::Block(stmts) => match ast.stmt_list(stmts) {
                    [Stmt::Expr(inner)] => e = *inner,
                    [.., Stmt::Expr(last)] if matches!(ast.expr(*last), Expr::Match(..)) => {
                        let text = &self.source(self.env.file).text;
                        return match braced_before(text, ast.expr_span(*last).start, ast.expr_span(e).start) {
                            true => PartialBody::Other,
                            false => PartialBody::BlockMatch(*last, e),
                        };
                    }
                    _ => return PartialBody::Other,
                },
                _ => return PartialBody::Other,
            }
        }
    }

    /// Whether the type of the value of the match `a` (typed `e`) that ends a block names one of the
    /// block's definitions, as dotc types it: `ensureNoLocalRefs` (`typedBlock`) then ascribes
    /// the block's result away from it, so that the block ends in no match and gets no E211.
    /// The match's cases are read through blocks, conditionals and matches, written and typed
    /// alike; an ascription's type is the written one (`(k: Int)`), anything else's is read
    /// from its typed tree (`value_names_local`). A result that has another branch's wider type
    /// in scalac's join counts as well, so that no warning is
    /// given that scalac might not give.
    fn result_leaks(&mut self, ast: &ast::Ast, a: ExprId, e: TExprId, locals: &BlockLocals) -> bool {
        match (ast.expr(a), self.prog.expr(e)) {
            (Expr::Typed(_, t), _) => ty_names_any(ast, t, &locals.names),
            (Expr::Parens(x) | Expr::Unchecked(x), _) => self.result_leaks(ast, x, e, locals),
            (Expr::Block(stmts), TExpr::Block(_, r)) => match ast.stmt_list(stmts).last() {
                Some(&Stmt::Expr(last)) => self.result_leaks(ast, last, r, locals),
                _ => self.value_names_local(e, true, locals),
            },
            (Expr::If(_, t, Some(f)), TExpr::If(_, tt, Some(tf))) => self.result_leaks(ast, t, tt, locals) || self.result_leaks(ast, f, tf, locals),
            (Expr::Match(_, cases), TExpr::Match(_, tcases)) if ast.case_list(cases).len() == tcases.len as usize => {
                let bodies: Vec<(ExprId, TExprId)> =
                    ast.case_list(cases).iter().zip(&self.prog.cases[tcases.range()]).map(|(c, t)| (c.body, t.body)).collect();
                bodies.into_iter().any(|(c, t)| self.result_leaks(ast, c, t, locals))
            }
            _ => self.value_names_local(e, true, locals),
        }
    }

    /// Whether the type dotc gives the typed `e` names a definition of the block `locals`
    /// describes: at the result's place (`top`) a reference to a val, var or parameterless def
    /// of the block, or to a field of one, is its singleton (an application's type is its
    /// result's: `k()`, `k(n)`); anywhere a class the block defines, in a construction, in a
    /// method's declared result or a symbol's type, and inside the arguments of a generic one
    /// (`List(new C)`) or the body of a function literal.
    fn value_names_local(&mut self, e: TExprId, top: bool, locals: &BlockLocals) -> bool {
        match self.prog.expr(e) {
            TExpr::Local(s) => (top && self.block_term(s, locals)) || self.sym_names_local_class(s, locals),
            TExpr::CallStatic(s, args) => {
                let reference = self.syms.sym(s).sig.as_ref().map_or(false, |sig| sig.clauses.is_empty());
                (top && reference && self.block_term(s, locals)) || self.call_names_local_class(None, s, args, locals)
            }
            TExpr::CallMethod(r, s, args) => self.call_names_local_class(Some(r), s, args, locals),
            TExpr::Field(r, f) => (top && self.value_names_local(r, true, locals)) || self.sym_names_local_class(f, locals),
            TExpr::New(c, args) => self.class_names_local(c, args, locals),
            TExpr::NewVia(ctor, args) => match self.syms.sym(ctor).owner {
                Owner::Class(c) => self.class_names_local(c, args, locals),
                _ => false,
            },
            TExpr::Block(_, r) => self.value_names_local(r, top, locals),
            TExpr::If(_, t, Some(f)) => self.value_names_local(t, top, locals) || self.value_names_local(f, top, locals),
            TExpr::Match(_, cases) => {
                let bodies: Vec<TExprId> = self.prog.cases[cases.range()].iter().map(|c| c.body).collect();
                bodies.into_iter().any(|b| self.value_names_local(b, top, locals))
            }
            TExpr::SeqLit(l) | TExpr::ArrayLit(l) => self.any_names_local(l, locals),
            TExpr::Lambda(params, body) => {
                let params: Vec<SymId> = self.prog.sym_list(params).to_vec();
                params.into_iter().any(|p| self.sym_names_local_class(p, locals)) || self.value_names_local(body, false, locals)
            }
            _ => false,
        }
    }

    /// A val, var or def of the block: made in the lambda, outside the match, and no parameter
    /// of the lambda's own.
    fn block_term(&self, s: SymId, locals: &BlockLocals) -> bool {
        let sym = self.syms.sym(s);
        sym.file == self.env.file && within(sym.span, locals.outer) && !within(sym.span, locals.inner) && !locals.own.contains(&s)
    }

    fn any_names_local(&mut self, l: ListRef, locals: &BlockLocals) -> bool {
        let items: Vec<TExprId> = self.prog.expr_list(l).to_vec();
        items.into_iter().any(|a| self.value_names_local(a, false, locals))
    }

    /// A construction of class `c`: one the block defines, or a generic one of a value of one.
    fn class_names_local(&mut self, c: ClassId, args: ListRef, locals: &BlockLocals) -> bool {
        self.local_class(c, locals) || (!self.syms.class(c).tparams.is_empty() && self.any_names_local(args, locals))
    }

    /// A call of `s`: a declared result naming a class of the block, or a generic method or
    /// receiver given a value of one.
    fn call_names_local_class(&mut self, recv: Option<TExprId>, s: SymId, args: ListRef, locals: &BlockLocals) -> bool {
        let Some(sig) = self.syms.sym(s).sig.clone() else { return false };
        self.type_names_local_class(sig.ret, locals, 0)
            || (!sig.tparams.is_empty() && self.any_names_local(args, locals))
            || recv.map_or(false, |r| self.value_names_local(r, false, locals))
    }

    fn sym_names_local_class(&mut self, s: SymId, locals: &BlockLocals) -> bool {
        match self.syms.sym(s).sig.clone() {
            Some(sig) if sig.clauses.is_empty() => self.type_names_local_class(sig.ret, locals, 0),
            _ => false,
        }
    }

    fn local_class(&self, c: ClassId, locals: &BlockLocals) -> bool {
        let info = self.syms.class(c);
        info.file == self.env.file && within(info.span, locals.outer) && !within(info.span, locals.inner)
    }

    /// Whether the type `t` names a class the block defines, inside compound types too.
    fn type_names_local_class(&mut self, t: TypeId, locals: &BlockLocals, depth: u32) -> bool {
        if depth > 16 {
            return false;
        }
        let t = self.deref(t);
        let d = depth + 1;
        match self.types.get(t) {
            Type::Class(c, args) => {
                self.local_class(c, locals) || self.types.items(args).to_vec().into_iter().any(|a| self.type_names_local_class(a, locals, d))
            }
            Type::AppParam(_, args) | Type::AppVar(_, args) | Type::Alias(_, args) => {
                self.types.items(args).to_vec().into_iter().any(|a| self.type_names_local_class(a, locals, d))
            }
            Type::Union(a, b) | Type::Inter(a, b) => self.type_names_local_class(a, locals, d) || self.type_names_local_class(b, locals, d),
            Type::Refined(p, _) => self.type_names_local_class(p, locals, d),
            Type::Lambda(_, body) | Type::Poly(_, body) => self.type_names_local_class(body, locals, d),
            _ => false,
        }
    }

    /// The match `m` that ends a block a partial function is made of: scalac's E211 at it, and
    /// the checks of the cases of the matches in it, which scalac's pattern matcher runs after,
    /// hidden by it (`UniqueMessagePositions`: a message hides a later one of no higher level
    /// whose place overlaps its own). The checks are those the body's typing deferred from
    /// `mark` on, quietened in place as `quiet_repeated_matches` does.
    fn match_not_partial(&mut self, m: ExprId, mark: usize) {
        let span = self.cur_ast().expr_span(m);
        self.warn(span, "match expression in result of block will not be used to synthesize partial function");
        let file = self.env.file;
        for d in &mut self.deferred_matches[mark..] {
            if d.file == file && d.span.start >= span.start && d.span.end <= span.end {
                d.quiet = true;
            }
        }
    }

    /// For `f(1, _)`: the parameter type comes from the matching parameter of `f`.
    pub(super) fn param_type_from_body(&mut self, body: ExprId, param: Name) -> Option<TypeId> {
        let ast = self.cur_ast();
        let Expr::Apply(f, args) = ast.expr(body) else { return None };
        let position = ast
            .expr_list(args)
            .iter()
            .position(|&a| matches!(ast.expr(a), Expr::Ident(n) if n == param))?;
        // `identity[Int](_)`: the parameter's type is the instantiated method's.
        let (f, targs) = match ast.expr(f) {
            Expr::TypeApply(g, targs) => (g, Some(targs)),
            _ => (f, None),
        };
        let callee = self.static_ref(f)?;
        if targs.is_some() && self.constructed_class(callee).is_some() {
            return None;
        }
        if let Some(c) = self.constructed_class(callee) {
            self.complete_class(c);
            let info = self.syms.class(c);
            if !info.tparams.is_empty() {
                return None;
            }
            let clause = info.ctor.iter().find(|c| !c.is_using)?;
            return clause.params.get(position).map(|p| p.ty);
        }
        let sym = callee.sym()?;
        // Kept while the type arguments are resolved against its type parameters.
        let sig = self.sig_arc(sym);
        if self.syms.sym(sym).kind == SymKind::Def {
            let clause = sig.clauses.iter().find(|c| !c.is_using)?;
            let pty = clause.params.get(position)?.ty;
            return match targs {
                None if sig.tparams.is_empty() => Some(pty),
                Some(targs) if ast.ty_list(targs).len() == sig.tparams.len() => {
                    // The arguments are resolved again with the call, which reports what fails.
                    let mark = self.diags.items.len();
                    let targs = ast.ty_list(targs).to_vec();
                    let subst: Subst = sig.tparams.iter().copied().zip(targs.into_iter().map(|t| self.resolve_type(t))).collect();
                    if self.diags.items.len() > mark {
                        self.drop_reported_since(mark);
                        return Some(ERROR);
                    }
                    Some(self.types.subst(pty, &subst))
                }
                _ => None,
            };
        }
        if targs.is_some() {
            return None;
        }
        let (ptys, _) = self.as_function(sig.ret)?;
        ptys.get(position).copied()
    }

    /// The type of a lambda parameter as the expected function type gives it. An open variable
    /// is solved from its bounds first, and one without any becomes `Any`, as scalac
    /// instantiates it before the body is typed (`sink(_.toString)` takes an `Any`).
    fn lambda_param_type(&mut self, t: TypeId) -> TypeId {
        let head = self.deref(t);
        if let Type::Var(v) = self.types.get(head) {
            if let Some(improved) = self.improved_lower_bound(v) {
                self.instantiate(v, improved);
                return improved;
            }
        }
        let solved = self.solve_if_var(t);
        let head = self.deref(t);
        let still_open = matches!(self.types.get(head), Type::Var(_));
        if solved == NOTHING && still_open {
            self.solve_maximized_in(t)
        } else {
            solved
        }
    }

    /// scalac's improvement of a variable bounded below by an empty collection only (`Nil`,
    /// `List.empty`): the collection class with a fresh variable for every covariant `Nothing`
    /// argument, so that the lambda body settles the element type
    /// (`foldLeft(Nil)((acc, x) => x :: acc)` is a `List[Int]`).
    fn improved_lower_bound(&mut self, v: TVarId) -> Option<TypeId> {
        let info = &self.tvars[v];
        if info.inst.is_some() || info.lower.len() != 1 {
            return None;
        }
        // scalac improves whatever the variable's other bounds (`IsFullyDefinedAccumulator
        // .instantiate`, Inferencing.scala:214-229) and keeps the improvement only where it
        // conforms to the full upper bound: `println(xs.foldLeft(Nil)(..))` bounds `B` by `Any`.
        let uppers: Vec<TypeId> = info.upper.clone();
        let mut lower = self.deref(info.lower[0]);
        if let Type::Class(c, EMPTY_LIST) = self.types.get(lower) {
            if self.syms.class(c).kind == ClassKind::Object {
                self.complete_class(c);
                lower = *self.syms.class(c).parents.first()?;
            }
        }
        let Type::Class(c, args) = self.types.get(lower) else { return None };
        let n = self.types.items(args).len();
        self.settle_class(c);
        // A covariant `Nothing` argument of a first-order parameter (dotty's `improve` leaves a
        // higher-kinded parameter's argument alone: "we'd get the wrong kind", Inferencing.scala:207-209).
        let empty_covariant = |t: &mut Self, i: usize| {
            let a = t.types.items(args)[i];
            let covariant = t.syms.class(c).tparams.get(i).map_or(false, |&p| {
                let info = t.syms.tparam(p);
                info.variance == 1 && info.arity == 0
            });
            covariant && t.deref(a) == NOTHING
        };
        if !(0..n).any(|i| empty_covariant(self, i)) {
            return None;
        }
        let mut fresh = Vec::with_capacity(n);
        for i in 0..n {
            let a = self.types.items(args)[i];
            fresh.push(if empty_covariant(self, i) { self.fresh_var() } else { a });
        }
        let improved = self.types.class(c, &fresh);
        let mark = self.snapshot();
        for u in uppers {
            if !self.is_sub(improved, u) {
                self.rollback(mark);
                return None;
            }
        }
        Some(improved)
    }

    /// Solves a still-open inference variable from the bounds collected so far.
    pub fn solve_if_var(&mut self, t: TypeId) -> TypeId {
        let t = self.deref(t);
        match self.types.get(t) {
            Type::Var(v) => {
                let info = &self.tvars[v];
                if info.lower.is_empty() && info.upper.is_empty() {
                    return NOTHING;
                }
                self.solve_var(v);
                self.zonk(t)
            }
            _ => self.zonk(t),
        }
    }

    /// A block, `span` its source where it is an expression of its own (a constructor's
    /// statements after the self call are none).
    pub(super) fn type_block(&mut self, stmts: ListRef, expected: Option<TypeId>, span: Option<Span>) -> (TExprId, TypeId) {
        let ast = self.cur_ast();
        let file = self.env.file;
        let items: Vec<Stmt> = ast.stmt_list(stmts).to_vec();
        // A class of a stored body's block is the body's own, copied at each expansion
        // (`InlineDefinition::classes`); an object, a trait, an enum or a case class of one holds
        // the body back.
        let stores = self.stores_classes();
        if self.checks_inline_definition() && self.refuses_case_classes(&items) {
            return (self.prog.add(TExpr::Unit), expected.unwrap_or(ERROR));
        }
        let held = |d: &crate::ast::DefId| match &ast.def(*d).kind {
            DefKind::Class(_) => !stores,
            _ => false,
        };
        if self.checks_inline_definition() && items.iter().any(|s| matches!(s, Stmt::Def(d) if held(d))) {
            self.note_held(crate::tir::HeldForm::LocalClass);
            return (self.prog.add(TExpr::Unit), expected.unwrap_or(ERROR));
        }
        let imports_scope = self.env.imports.len();
        let body_imports = self.inline.body_imports.len();
        let import_aliases = self.inline.import_aliases.len();
        let pending_imports = if self.inline.checking > 0 { self.pending_imports_mark() } else { None };
        self.push_scope();
        let cursor = items.iter().any(|s| matches!(s, Stmt::Def(d) if matches!(ast.def(*d).kind, DefKind::Fun(_)) && ast.def(*d).mods & mods::INLINE != 0));
        if cursor {
            let frame = self.env.frames.len() - 1;
            self.inline.blocks.push(super::inline::BlockCursor { frame, file, stmts, at: 0, imports_scope });
        }
        let mut defined: Vec<(Name, &'static str)> = Vec::new();
        for s in &items {
            let Stmt::Def(d) = s else { continue };
            let def = ast.def(*d);
            let is_object = matches!(&def.kind, DefKind::Class(cls) if cls.kind == ast::ClassKind::Object);
            if is_object || matches!(def.kind, DefKind::Fun(_) | DefKind::Val { pat: None, .. }) {
                // Local methods cannot be overloaded, as in Scala; an object is a term too.
                if let Some(&(_, kind)) = defined.iter().find(|(n, _)| *n == def.name) {
                    let msg = format!("{0} is already defined as {1} {0}", self.name_str(def.name), kind);
                    self.error(def.span, msg);
                }
                let kind = match def.kind {
                    DefKind::Fun(_) => "method",
                    DefKind::Class(_) => "object",
                    _ if def.mods & mods::MUTABLE != 0 => "variable",
                    _ => "value",
                };
                defined.push((def.name, kind));
            }
            match &def.kind {
                DefKind::Fun(f) => {
                    if def.mods & mods::INLINE != 0 && self.checks_inline_definition() {
                        self.error(def.span, "Implementation restriction: nested inline methods are not supported");
                    }
                    let sym = self.syms.new_sym(def.name, SymKind::Def, def.mods, Owner::Local, file, Some(*d), def.span);
                    let mut info = self.syms.sym_mut(sym);
                    info.is_extension = f.is_extension;
                    info.ext_tparams = f.ext_tparams;
                    info.ext_clauses = f.ext_clauses;
                    if self.index.is_some() {
                        self.index_local(sym);
                    }
                    self.def_syms.insert(file.0 as usize, *d, sym);
                    self.bind_local(def.name, sym);
                    if def.mods & mods::IMPLICIT != 0 {
                        self.bind_given(sym);
                    }
                }
                // A local class is entered like a top-level one and lifted with what it
                // captures, as an anonymous class is; its members see the block from here on.
                DefKind::Class(cls) => match cls.kind {
                    ast::ClassKind::Class | ast::ClassKind::Trait => {
                        if def.mods & mods::CASE != 0 {
                            self.check_case_class_in_inline(def.span);
                        }
                        self.enter_def(file, Owner::Local, *d);
                        let c = *self.def_classes.get(file.0 as usize, &d).expect("the local class was entered");
                        if stores {
                            self.note_stored_class(c);
                        } else {
                            self.name_retyped_product_class(c);
                        }
                        self.anon_envs.insert(c, Arc::new(self.env.clone()));
                        self.bind_local_class(def.name, c);
                        self.link_local_companion(c, def.name);
                    }
                    ast::ClassKind::Object => {
                        self.enter_local_object(file, *d);
                        if stores {
                            if let Some(&c) = self.def_classes.get(file.0 as usize, d) {
                                self.note_stored_class(c);
                            }
                        }
                    }
                    ast::ClassKind::Enum => {
                        self.enter_local_enum(file, *d);
                        if stores {
                            if let Some(&c) = self.def_classes.get(file.0 as usize, d) {
                                self.note_stored_class(c);
                                if let Some(companion) = self.syms.class(c).companion {
                                    self.note_stored_class(companion);
                                }
                            }
                        }
                    }
                    ast::ClassKind::EnumCase => {}
                },
                DefKind::TypeAlias { rhs, .. } if !rhs.map_or(false, |r| self.names_term_path(r)) => self.enter_local_alias(file, *d),
                // A lazy val with a declared type may be named before its definition, by the
                // lazy vals and methods above it.
                DefKind::Val { pat: None, ty: Some(t), rhs: Some(_) } if def.mods & mods::LAZY != 0 => {
                    let declared = self.resolve_type(*t);
                    let sym = self.new_local(def.name, SymKind::Val, declared, def.span);
                    self.syms.sym_mut(sym).mods |= def.mods & (mods::LAZY | mods::IMPLICIT);
                    self.def_syms.insert(file.0 as usize, *d, sym);
                    self.bind_local(def.name, sym);
                    if def.mods & mods::IMPLICIT != 0 {
                        self.bind_given(sym);
                    }
                }
                _ => {}
            }
        }
        let mut out: Vec<TStmt> = Vec::with_capacity(items.len());
        let mut result: Option<(TExprId, TypeId)> = None;
        let n = items.len();
        for (i, s) in items.iter().enumerate() {
            if cursor {
                if let Some(b) = self.inline.blocks.last_mut() {
                    b.at = i;
                }
            }
            match *s {
                Stmt::Expr(e) if i + 1 == n => result = Some(self.type_expr_adapted(e, expected)),
                Stmt::Expr(e) => {
                    let mark = self.failure_mark();
                    let (te, ty) = self.type_expr(e, None);
                    self.warn_pure_statement(e, te, ty, mark);
                    out.push(TStmt::Expr(te));
                }
                Stmt::Def(d) => {
                    self.type_local_def(d, &mut out);
                    if let (Some(s), true) = (span, self.capturing()) {
                        if let Some(&sym) = self.def_syms.get(file.0 as usize, &d).filter(|&&sym| self.is_inline_callee(sym)) {
                            let from = self.ast(file).def(d).span.start;
                            self.capture_block_stmts(s, out.len() as u32, from, vec![crate::tir::capture::BlockStmt::InlineDef(sym)]);
                        }
                    }
                }
                Stmt::Import(i) => {
                    let entered = self.enter_imports(ast.import_stmt(i), imports_scope);
                    if let (Some(s), true) = (span, self.capturing()) {
                        let from = ast.import_stmt(i).first().map_or(0, |c| c.span.start);
                        self.capture_block_imports(s, out.len() as u32, from, &entered);
                    }
                    if pending_imports.is_some() {
                        self.note_imports(out.len() as u32, entered);
                    }
                }
            }
        }
        if cursor {
            self.inline.blocks.pop();
        }
        self.pop_scope();
        self.env.imports.truncate(imports_scope);
        self.inline.body_imports.truncate(body_imports);
        for alias in self.inline.import_aliases.drain(import_aliases..) {
            self.inline.args.remove(&alias);
        }
        let (res, ty) = match result {
            Some(r) => r,
            None => (self.prog.add(TExpr::Unit), self.b.t_unit),
        };
        let l = self.prog.stmts.push_slice(&out);
        let block = self.prog.add(TExpr::Block(l, res));
        self.note_block_imports(block, pending_imports);
        (block, ty)
    }

    /// Types against an expected type when one is usable, adapting the result to it.
    pub fn type_expr_adapted(&mut self, e: ExprId, expected: Option<TypeId>) -> (TExprId, TypeId) {
        let Some(exp) = self.concrete_expected(expected) else { return self.type_expr(e, expected) };
        let (te, ty) = self.type_expr(e, Some(exp));
        if exp == self.b.t_unit {
            self.warn_discarded(e, te, ty);
        }
        let span = self.cur_ast().expr_span(e);
        // The expression keeps its own type where it conforms as it stands, as scalac's tree
        // does (`{ ..; new C }` is a `C`, a throwing branch a `Nothing`, a context function
        // applied its result); an adaptation gives it the expected one.
        let (adapted, ty) = self.adapt_typed(te, ty, exp, span);
        (adapted, if ty == ERROR { exp } else { ty })
    }

    /// The expected type of the branches of an `if` or `match`. An application of an unsolved
    /// type constructor variable (`G[B]` of a `traverse`) is left for the join of the branches to
    /// be unified with, so that `Some(x)` and `None` make `G` an `Option`.
    /// The type of an `if` or `match` checked against an expected type: the join of what its
    /// branches were typed as, which is what scalac gives the tree, where that is known and
    /// conforms; the expected type otherwise.
    pub fn joined_branches(&mut self, expected: TypeId, branches: &[TypeId]) -> TypeId {
        let mut acc = NOTHING;
        for &t in branches {
            // A branch that throws joins to the other branch.
            if t == NOTHING {
                continue;
            }
            if t == ERROR || self.types.has_vars(t) || self.mentions_lit(t) {
                return expected;
            }
            acc = if acc == NOTHING { t } else { self.lub(acc, t) };
        }
        if acc != NOTHING && acc != expected && self.is_sub(acc, expected) {
            acc
        } else {
            expected
        }
    }

    pub fn branch_expected(&mut self, expected: Option<TypeId>) -> Option<TypeId> {
        let t = self.concrete_expected(expected)?;
        match self.types.get(t) {
            // A constructor variable its bounds pin (`F` of `Kleisli { case .. }` against a
            // `Kleisli[[x] =>> OptionT[G, x], A, B]`) is that type for the branches.
            Type::AppVar(v, _) if self.pin_var(v) => Some(self.deref(t)),
            Type::AppVar(..) => None,
            _ => Some(t),
        }
    }

    /// Instantiates `v` to the one type without variables that all its bounds name.
    fn pin_var(&mut self, v: TVarId) -> bool {
        let info = &self.tvars[v];
        if info.inst.is_some() || info.lower.is_empty() || info.upper.is_empty() {
            return false;
        }
        let bounds: Vec<TypeId> = info.lower.iter().chain(info.upper.iter()).copied().collect();
        let first = self.zonk(bounds[0]);
        if first == ERROR || self.types.has_vars(first) || !bounds[1..].iter().all(|&b| self.zonk(b) == first) {
            return false;
        }
        self.instantiate(v, first);
        true
    }

    /// The expected type of the branches of an `if` or `match` without a usable one: an open
    /// variable that collects the types of the branches typed so far as lower bounds, next to
    /// the bounds of an open expected type. It is a hint for the type arguments a branch leaves
    /// open (`Map.empty` next to a `Map[Int, String]`); nothing is checked against it.
    pub fn branch_guide(&mut self, expected: Option<TypeId>) -> TypeId {
        let outer = match expected.map(|t| self.deref(t)).map(|t| self.types.get(t)) {
            Some(Type::Var(v)) => Some(v),
            _ => None,
        };
        if let Some(v) = outer.filter(|&v| self.tvars[v].guide) {
            return self.types.mk(Type::Var(v));
        }
        let g = self.fresh_var();
        let Type::Var(gv) = self.types.get(g) else { unreachable!() };
        if let Some(v) = outer {
            let (lower, upper) = (self.tvars[v].lower.clone(), self.tvars[v].upper.clone());
            self.tvars[gv].lower = lower;
            self.tvars[gv].upper = upper;
        }
        self.tvars[gv].guide = true;
        g
    }

    pub fn guide_with(&mut self, guide: TypeId, branch_ty: TypeId) {
        let Type::Var(g) = self.types.get(guide) else { return };
        let branch_ty = self.zonk(branch_ty);
        if branch_ty != ERROR && !self.tvars[g].lower.contains(&branch_ty) {
            self.tvars[g].lower.push(branch_ty);
        }
    }

    /// Whether a type names a member or the singleton of a value (`et.Underlying`, `x.type`).
    fn names_term_path(&self, t: ast::TyExprId) -> bool {
        let ast = self.cur_ast();
        match ast.ty(t) {
            TyExpr::Select(..) | TyExpr::Singleton(_) => true,
            TyExpr::Apply(f, args) => self.names_term_path(f) || ast.ty_list(args).iter().any(|&a| self.names_term_path(a)),
            TyExpr::Union(a, b) | TyExpr::Inter(a, b) => self.names_term_path(a) || self.names_term_path(b),
            TyExpr::Tuple(items) => ast.ty_list(items).iter().any(|&a| self.names_term_path(a)),
            TyExpr::Fun(items, r) => ast.ty_list(items).iter().any(|&a| self.names_term_path(a)) || self.names_term_path(r),
            _ => false,
        }
    }

    fn type_local_def(&mut self, d: ast::DefId, out: &mut Vec<TStmt>) {
        let ast = self.cur_ast();
        let file = self.env.file;
        let def = ast.def(d);
        // scalac's E200: `final` says nothing of a local val, var or def (a local class may be final).
        if def.mods & ast::mods::FINAL != 0 && matches!(def.kind, DefKind::Val { .. } | DefKind::Fun(_)) {
            self.error(def.span, "The final modifier is not allowed on local definitions");
        }
        if !def.annots.is_empty() {
            self.enter_interop_annots(file, Owner::Local, def, None);
        }
        if self.unused.on() {
            self.mark_annotations(file, d);
        }
        match &def.kind {
            // An alias over a path of the block (`type Elem = et.Underlying`) is entered where it
            // stands, once the path's value is.
            DefKind::TypeAlias { rhs: Some(r), .. } if self.names_term_path(*r) => self.enter_local_alias(file, d),
            DefKind::Class(cls) => {
                let Some(&c) = self.def_classes.get(file.0 as usize, &d) else { return };
                if self.deps.is_some() {
                    self.deps_local_class(c);
                }
                let env = Arc::new(self.env.clone());
                self.anon_envs.insert(c, env.clone());
                let first = self.prog.exprs.len();
                match cls.kind {
                    ast::ClassKind::Class => {
                        self.check_class(c);
                        self.finish_local_class(c, first);
                    }
                    // A local object is an instance of its lifted class, made once per run of
                    // the block on its first use, as scalac's lazy val holds it.
                    ast::ClassKind::Object => {
                        self.check_class(c);
                        self.finish_local_class(c, first);
                        if let Some(sym) = self.syms.class(c).local_module {
                            let init = self.new_instance(c, ast::ListRef::EMPTY, Span::default());
                            out.push(TStmt::Val(sym, init));
                        }
                    }
                    ast::ClassKind::Enum => {
                        if let Some(companion) = self.syms.class(c).companion {
                            self.anon_envs.insert(companion, env);
                        }
                        self.check_class(c);
                        self.check_local_enum_captures(c, first, def.span);
                    }
                    ast::ClassKind::Trait => {
                        self.check_class(c);
                        self.check_local_trait_captures(c, first, def.span);
                    }
                    _ => {}
                }
            }
            DefKind::Val { pat, ty, rhs } => {
                let Some(rhs) = *rhs else {
                    if def.mods & mods::INCOMPLETE == 0 {
                        self.error(def.span, "a local val needs an initializer");
                        return;
                    }
                    // An incomplete val's initializer is unknown, not missing: its name stands,
                    // with the type it declares or the error type.
                    self.dependent_error(def.span, "a local val needs an initializer");
                    if pat.is_none() {
                        let declared = ty.map_or(ERROR, |t| self.resolve_type(t));
                        let kind = if def.mods & mods::MUTABLE != 0 { SymKind::Var } else { SymKind::Val };
                        let sym = self.new_local(def.name, kind, declared, def.span);
                        self.bind_local(def.name, sym);
                    }
                    return;
                };
                if ast.default_inits.contains(&rhs) {
                    self.error(def.span, "Unbound placeholder parameter; incorrect use of _");
                }
                let declared = ty.map(|t| self.resolve_type(t));
                if let Some(p) = *pat {
                    if declared.is_none() {
                        if let Some(elems) = self.elementwise_tuple_val(p, rhs) {
                            let kind = if def.mods & mods::MUTABLE != 0 { SymKind::Var } else { SymKind::Val };
                            for (name, ty, e, span) in elems {
                                self.type_binder_val(name, kind, def.mods & mods::LAZY, ty, e, span, out);
                            }
                            return;
                        }
                    }
                }
                let owners_depth = self.sites.owners.len();
                if let Some(p) = *pat {
                    let pat_span = self.cur_ast().pat_spans[p.idx()];
                    match self.single_binder(p, rhs) {
                        Some(name) => {
                            let span = self.binder_span(p, name).unwrap_or(pat_span);
                            self.sites.owners.push(SiteOwner::Binder(name, span));
                        }
                        None => self.sites.owners.push(SiteOwner::PatternTemp(pat_span)),
                    }
                }
                // The name is in scope in its own initialiser, where it needs a declared type.
                let early = if pat.is_none() && def.mods & mods::LAZY != 0 { self.def_syms.get(file.0 as usize, &d).copied() } else { None };
                if let Some(sym) = early {
                    self.sites.owners.push(SiteOwner::Sym(sym));
                }
                let sym = if early.is_some() { early } else { pat.is_none().then(|| {
                    let kind = if def.mods & mods::MUTABLE != 0 { SymKind::Var } else { SymKind::Val };
                    let sym = self.new_local(def.name, kind, declared.unwrap_or(ERROR), def.span);
                    let mut s = self.syms.sym_mut(sym);
                    s.mods |= def.mods & (mods::LAZY | mods::IMPLICIT);
                    if declared.is_none() {
                        s.state().set(Completion::InProgress);
                    }
                    let lost = def.mods & !(mods::LAZY | mods::IMPLICIT | mods::MUTABLE);
                    if self.capturing() && lost != 0 {
                        self.capture_local(sym, |l| l.mods = lost);
                    }
                    self.bind_local(def.name, sym);
                    if def.mods & mods::IMPLICIT != 0 {
                        self.bind_given(sym);
                    }
                    self.sites.owners.push(SiteOwner::Sym(sym));
                    sym
                }) };
                let inferred = ty.filter(|t| self.in_converted_body() && pat.is_none() && early.is_none() && ast.inferred_types.binary_search(&t.0).is_ok());
                let (init, mut ity) = match declared {
                    Some(t) if inferred.is_some() => {
                        let (init, ity) = self.type_inferred_val(rhs, t);
                        if ity != t {
                            let sig = self.value_sig(ity);
                            self.syms.sym_mut(sym.unwrap()).sig = Some(sig);
                        }
                        (init, ity)
                    }
                    Some(t) => (self.check_expr(rhs, t), t),
                    // An incomplete val's type is unknown unless declared, as a member's is: its
                    // body is typed for its own errors alone.
                    None if def.mods & mods::INCOMPLETE != 0 => (self.type_expr(rhs, None).0, ERROR),
                    None => {
                        let (te, t) = self.type_expr(rhs, None);
                        let (te, t) = self.apply_inferred_context(rhs, te, t);
                        let t = self.solve_inferred(t);
                        (te, self.widen_soft(te, t))
                    }
                };
                if def.mods & mods::INLINE != 0 {
                    ity = self.check_inline_val(init, ity, declared.is_some(), rhs);
                    // One whose value is no constant at the definition is the expansion's to
                    // check and stand for its value (`Worker::walk_block`).
                    let kept = self.checks_inline_definition() && !matches!(self.types.get(ity), Type::Lit(_));
                    match (sym, kept) {
                        (Some(sym), false) => {
                            self.inline.args.insert(sym, super::inline::InlineArg { expr: init, ty: ity, source: None });
                        }
                        (Some(sym), true) => self.note_inline_val(sym),
                        (None, _) => {}
                    }
                }
                self.sites.owners.truncate(owners_depth);
                match (pat, sym) {
                    (Some(p), _) => out.push(self.pattern_val(*p, rhs, init, ity)),
                    (None, Some(sym)) => {
                        if declared.is_none() {
                            let sig = self.value_sig(ity);
                            let mut s = self.syms.sym_mut(sym);
                            s.sig = Some(sig);
                            s.state().set(Completion::Done);
                            self.note_inferred_val(sym);
                        }
                        out.push(TStmt::Val(sym, init));
                    }
                    (None, None) => {}
                }
            }
            DefKind::Fun(_) => {
                let Some(&sym) = self.def_syms.get(file.0 as usize, &d) else { return };
                if self.misplaced_abstract_given(def) {
                    return;
                }
                let sig = self.sig_arc(sym);
                if !self.fun_of_sym.contains_key(&sym) && !self.is_inline_callee(sym) {
                    let ret = sig.ret;
                    self.type_body(sym, &sig, Some(ret));
                } else if self.is_inline_callee(sym) {
                    self.check_local_inline_definition(sym);
                }
                if let Some(&f) = self.fun_of_sym.get(&sym) {
                    out.push(TStmt::Fun(f));
                }
            }
            DefKind::Given(g) => {
                let Some(alias) = g.alias else {
                    self.error(def.span, "local structural givens are not supported; use an alias given");
                    return;
                };
                if !g.clauses.is_empty() || !g.tparams.is_empty() {
                    self.error(def.span, "local givens cannot take parameters");
                }
                let ty = self.resolve_type(g.ty);
                // A `transparent inline given` is summoned at the type of its right-hand side,
                // which its expansion at the use has (`TransformerConfiguration[?] =
                // default.enableDefaultValues`); its value is the same wherever it is used.
                let transparent = def.mods & mods::INLINE != 0 && def.mods & mods::TRANSPARENT != 0;
                if transparent {
                    let (init, rhs_ty) = self.type_expr(alias, Some(ty));
                    let rhs_ty = self.solve_in(rhs_ty);
                    let (init, precise) = if self.is_sub(rhs_ty, ty) { (init, rhs_ty) } else { (self.check_expr(alias, ty), ty) };
                    let sym = self.new_local(def.name, SymKind::Val, precise, def.span);
                    self.syms.sym_mut(sym).mods |= mods::LAZY | mods::GIVEN | (def.mods & mods::ANONYMOUS);
                    self.bind_local(def.name, sym);
                    self.bind_given(sym);
                    out.push(TStmt::Val(sym, init));
                    return;
                }
                let sym = self.new_local(def.name, SymKind::Val, ty, def.span);
                self.syms.sym_mut(sym).mods |= mods::LAZY | mods::GIVEN | (def.mods & mods::ANONYMOUS);
                self.bind_local(def.name, sym);
                self.bind_given(sym);
                let outer_defining = self.defining.replace(sym);
                self.sites.owners.push(SiteOwner::Sym(sym));
                let init = self.check_expr(alias, ty);
                self.sites.owners.pop();
                self.defining = outer_defining;
                out.push(TStmt::Val(sym, init));
            }
            _ => {}
        }
    }

    /// `val (a, b) = (x, y)` over plain or typed binders is `val a = x; val b = y`, as scalac
    /// desugars it: the elements, each with the binder's declared type.
    fn elementwise_tuple_val(
        &mut self,
        pat: ast::PatId,
        rhs: ExprId,
    ) -> Option<Vec<(Name, Option<TypeId>, ExprId, Span)>> {
        let ast = self.cur_ast();
        let (Pat::Tuple(ps), Expr::Tuple(es)) = (ast.pat(pat), ast.expr(rhs)) else { return None };
        let (ps, es) = (ast.pat_list(ps), ast.expr_list(es));
        if ps.len() != es.len() || es.iter().any(|&e| matches!(ast.expr(e), Expr::NamedArg(..))) {
            return None;
        }
        let mut elems = Vec::with_capacity(ps.len());
        for (&p, &e) in ps.iter().zip(es) {
            let (name, ty) = match ast.pat(p) {
                Pat::Bind(n, None) => (n, None),
                Pat::Typed(inner, t) => match ast.pat(inner) {
                    Pat::Bind(n, None) => (n, Some(t)),
                    _ => return None,
                },
                _ => return None,
            };
            elems.push((name, ty, e, ast.pat_spans[p.idx()]));
        }
        Some(elems.into_iter().map(|(n, t, e, s)| (n, t.map(|t| self.resolve_type(t)), e, s)).collect())
    }

    fn type_binder_val(
        &mut self,
        name: Name,
        kind: SymKind,
        lazy: u32,
        declared: Option<TypeId>,
        rhs: ExprId,
        span: Span,
        out: &mut Vec<TStmt>,
    ) {
        self.sites.owners.push(SiteOwner::Binder(name, span));
        let (init, ity) = match declared {
            Some(t) => (self.check_expr(rhs, t), t),
            None => {
                let (te, t) = self.type_expr(rhs, None);
                (te, self.solve_inferred(t))
            }
        };
        self.sites.owners.pop();
        let sym = self.new_local(name, kind, ity, span);
        self.syms.sym_mut(sym).mods |= lazy;
        self.bind_local(name, sym);
        out.push(TStmt::Val(sym, init));
    }

    /// The one variable of a pattern val that scalac lowers to a plain val of that name
    /// (`val Some(x) = e` is `val x = e match { case Some(x) => x }`); a tuple of binders and
    /// wildcards counts the wildcards, as scalac's tuple optimisations do.
    fn single_binder(&self, pat: ast::PatId, rhs: ExprId) -> Option<Name> {
        let ast = self.cur_ast();
        if let (Pat::Tuple(ps), false) = (ast.pat(pat), matches!(ast.expr(rhs), Expr::Unchecked(_))) {
            let plain = ast.pat_list(ps).iter().all(|&p| match ast.pat(p) {
                Pat::Wildcard | Pat::Bind(_, None) => true,
                Pat::Typed(inner, _) => matches!(ast.pat(inner), Pat::Wildcard | Pat::Bind(_, None)),
                _ => false,
            });
            if plain {
                return None;
            }
        }
        let mut names = Vec::new();
        self.pattern_binders(pat, &mut names);
        match names[..] {
            [n] => Some(n),
            _ => None,
        }
    }

    fn binder_span(&self, pat: ast::PatId, name: Name) -> Option<Span> {
        let ast = self.cur_ast();
        match ast.pat(pat) {
            Pat::Bind(n, _) if n == name => Some(ast.pat_spans[pat.idx()]),
            Pat::Bind(_, Some(inner)) | Pat::Typed(inner, _) | Pat::Rest(inner) => self.binder_span(inner, name),
            Pat::Ctor(_, ps) | Pat::Tuple(ps) => ast.pat_list(ps).iter().find_map(|&p| self.binder_span(p, name)),
            _ => None,
        }
    }

    fn pattern_binders(&self, pat: ast::PatId, out: &mut Vec<Name>) {
        let ast = self.cur_ast();
        match ast.pat(pat) {
            Pat::Bind(n, inner) => {
                if !out.contains(&n) {
                    out.push(n);
                }
                if let Some(inner) = inner {
                    self.pattern_binders(inner, out);
                }
            }
            Pat::Typed(inner, _) | Pat::Rest(inner) | Pat::NamedField(_, inner) => self.pattern_binders(inner, out),
            Pat::Ctor(_, ps) | Pat::Tuple(ps) | Pat::Interp(_, _, ps) => {
                for &p in ast.pat_list(ps) {
                    self.pattern_binders(p, out);
                }
            }
            Pat::Wildcard | Pat::Lit(_) | Pat::StableId(_) | Pat::Alt(_) | Pat::Quote(_) | Pat::QuoteType(_) | Pat::Error => {}
        }
    }

    /// `val pat = rhs`: a pattern that can fail is a warning, unless the rhs is `: @unchecked`.
    fn pattern_val(&mut self, pat: ast::PatId, rhs: ExprId, init: TExprId, ity: TypeId) -> TStmt {
        let tp = self.type_pattern(pat, ity);
        let ast = self.cur_ast();
        if !matches!(ast.expr(rhs), Expr::Unchecked(_)) {
            self.check_irrefutable(tp, pat, ity, Some(ast.expr_span(rhs)));
        }
        TStmt::Pat(tp, init)
    }

    // ---- for comprehensions ----

    pub fn type_for(&mut self, rest: ForRest, span: Span, expected: Option<TypeId>) -> (TExprId, TypeId) {
        let ast = self.cur_ast();
        let ForRest::Rest { idx, end, body, is_yield } = rest else {
            return (self.prog.add(TExpr::Unit), ERROR);
        };
        let (pat, source, filtering) = match ast.enumerators[idx as usize] {
            Enumerator::Gen(pat, source) => (pat, source, false),
            Enumerator::CaseGen(pat, source) => (pat, source, true),
            _ => {
                self.error(span, "a for expression has to start with a generator");
                return (self.prog.add(TExpr::Unit), ERROR);
            }
        };
        let (mut recv, mut recv_ty) = self.type_expr(source, None);
        let mut next = idx + 1;
        let mut binder = ForBinder::Pat(pat);
        let mut filtered = filtering;
        if filtering {
            let lists = vec![ArgList { args: vec![ArgSrc::ForLambda(binder, ForRest::Filter)], using: false, span }];
            let (r, t) = self.apply_member(recv, recv_ty, names::WITH_FILTER, None, lists, span, None);
            recv = r;
            recv_ty = t;
            binder = ForBinder::CasePat(pat);
        }
        loop {
            while next < end {
                let Enumerator::Guard(g) = ast.enumerators[next as usize] else { break };
                let lists = vec![ArgList {
                    args: vec![ArgSrc::ForLambda(binder, ForRest::Guard(g))],
                    using: false,
                    span,
                }];
                let (r, t) = self.apply_member(recv, recv_ty, names::WITH_FILTER, None, lists, span, None);
                recv = r;
                recv_ty = t;
                filtered = true;
                next += 1;
            }
            // Value definitions followed by a guard travel along with the element.
            let mut val_end = next;
            while val_end < end && matches!(ast.enumerators[val_end as usize], Enumerator::Val(..)) {
                val_end += 1;
            }
            let guard_follows =
                val_end > next && val_end < end && matches!(ast.enumerators[val_end as usize], Enumerator::Guard(_));
            if !guard_follows {
                break;
            }
            let pack = ForRest::Pack { idx: next, end: val_end };
            let lists = vec![ArgList { args: vec![ArgSrc::ForLambda(binder, pack)], using: false, span }];
            let (r, t) = self.apply_member(recv, recv_ty, names::MAP, None, lists, span, None);
            recv = r;
            recv_ty = t;
            binder = ForBinder::Unpack(self.for_packs.len() as u32 - 1);
            next = val_end;
        }
        let has_more_generators = (next..end)
            .any(|i| matches!(ast.enumerators[i as usize], Enumerator::Gen(..) | Enumerator::CaseGen(..)));
        let method = match (is_yield, has_more_generators) {
            (false, _) => names::FOREACH,
            (true, true) => names::FLAT_MAP,
            (true, false) => names::MAP,
        };
        let inner = ForRest::Rest { idx: next, end, body, is_yield };
        let lists = vec![ArgList { args: vec![ArgSrc::ForLambda(binder, inner)], using: false, span }];
        let exp = if is_yield { expected } else { None };
        // Scala 3.8 leaves out a trailing `map` whose body is the generator's own pattern, or
        // `()` over `Unit` elements: the call is still typed, and the generator stands for it
        // when it has the call's type.
        let last = method == names::MAP && next == end && !filtered;
        let identity = last && matches!(binder, ForBinder::Pat(p) if self.expr_is_pattern(body, p));
        if identity || (last && matches!(ast.expr(body), Expr::UnitLit)) {
            let (te, ty) = self.apply_member(recv, recv_ty, method, None, lists, span, exp);
            let mark = self.snapshot();
            let same = self.is_sub(recv_ty, ty);
            self.rollback(mark);
            return if same { (recv, recv_ty) } else { (te, ty) };
        }
        self.apply_member(recv, recv_ty, method, None, lists, span, exp)
    }

    /// Whether `e` spells the variables `p` binds: the variable itself or a tuple of them.
    fn expr_is_pattern(&self, e: ExprId, p: ast::PatId) -> bool {
        let ast = self.cur_ast();
        match (ast.expr(e), ast.pat(p)) {
            (Expr::Parens(inner), _) => self.expr_is_pattern(inner, p),
            (Expr::Ident(n), Pat::Bind(m, None)) => n == m,
            (Expr::Tuple(es), Pat::Tuple(ps)) => {
                let (es, ps) = (ast.expr_list(es), ast.pat_list(ps));
                es.len() == ps.len() && es.iter().zip(ps).all(|(&e, &p)| self.expr_is_pattern(e, p))
            }
            _ => false,
        }
    }

    /// `y = rhs` in a for comprehension is a val named `y` to a macro in `rhs`.
    fn type_for_val(&mut self, pat: ast::PatId, value: ExprId) -> (TExprId, TypeId) {
        let ast = self.cur_ast();
        let binder = match ast.pat(pat) {
            Pat::Typed(inner, _) => ast.pat(inner),
            other => other,
        };
        let owners_depth = self.sites.owners.len();
        let pat_span = ast.pat_spans[pat.idx()];
        match binder {
            Pat::Bind(name, None) => self.sites.owners.push(SiteOwner::Binder(name, pat_span)),
            _ => self.sites.owners.push(SiteOwner::PatternTemp(pat_span)),
        }
        let (init, ity) = self.type_expr(value, None);
        self.sites.owners.truncate(owners_depth);
        (init, self.solve_inferred(ity))
    }

    /// Types the body of a synthesized for-comprehension lambda.
    pub fn type_for_rest(&mut self, rest: ForRest, span: Span, expected: Option<TypeId>) -> (TExprId, TypeId) {
        let ast = self.cur_ast();
        match rest {
            ForRest::Guard(g) => (self.check_expr(g, self.b.t_boolean), self.b.t_boolean),
            ForRest::Filter => (self.prog.add(TExpr::Bool(true)), self.b.t_boolean),
            ForRest::Pack { idx, end } => {
                let mut stmts = Vec::new();
                for i in idx..end {
                    let Enumerator::Val(pat, value) = ast.enumerators[i as usize] else { continue };
                    let (init, ity) = self.type_for_val(pat, value);
                    stmts.push(match ast.pat(pat) {
                        Pat::Bind(name, None) => {
                            let sym = self.new_local(name, SymKind::Val, ity, span);
                            if self.index.is_some() {
                                self.index_local_at(sym, ast.pat_spans[pat.idx()]);
                            }
                            self.bind_local(name, sym);
                            TStmt::Val(sym, init)
                        }
                        _ => self.pattern_val(pat, value, init, ity),
                    });
                }
                let base = self.for_bases.last().copied().unwrap_or(0);
                let mut vars: Vec<(Name, SymId)> = Vec::new();
                for frame in &self.env.frames[base..] {
                    if let Frame::Locals { names, .. } = frame {
                        for &(n, s) in names {
                            vars.retain(|&(existing, _)| existing != n);
                            vars.push((n, s));
                        }
                    }
                }
                let mut items = Vec::with_capacity(vars.len());
                let mut pack = Vec::with_capacity(vars.len());
                for (n, s) in vars {
                    let ty = self.sig_of(s).ret;
                    let given = self.syms.sym(s).mods & mods::GIVEN != 0;
                    pack.push((n, self.solve_in(ty), given));
                    items.push(self.prog.add(TExpr::Local(s)));
                }
                let tuple = if self.capturing() { self.pack_tuple(&pack) } else { None };
                self.for_packs.push(pack);
                let l = self.prog.list(&items);
                let packed = self.prog.add(TExpr::ArrayLit(l));
                if let Some(t) = tuple {
                    self.capture_form(packed, Form::Pack(t));
                }
                let sl = self.prog.stmts.push_slice(&stmts);
                (self.prog.add(TExpr::Block(sl, packed)), ANY)
            }
            ForRest::Rest { idx, end, body, .. } if idx == end => self.type_expr_adapted(body, expected),
            ForRest::Rest { idx, end, body, is_yield } => match ast.enumerators[idx as usize] {
                Enumerator::Gen(..) | Enumerator::CaseGen(..) => self.type_for(rest, span, expected),
                Enumerator::Val(pat, value) => {
                    self.push_scope();
                    let (init, ity) = self.type_for_val(pat, value);
                    let stmt = match ast.pat(pat) {
                        Pat::Bind(name, None) => {
                            let sym = self.new_local(name, SymKind::Val, ity, span);
                            if self.index.is_some() {
                                self.index_local_at(sym, ast.pat_spans[pat.idx()]);
                            }
                            self.bind_local(name, sym);
                            TStmt::Val(sym, init)
                        }
                        _ => self.pattern_val(pat, value, init, ity),
                    };
                    let inner = ForRest::Rest { idx: idx + 1, end, body, is_yield };
                    let (tb, ty) = self.type_for_rest(inner, span, expected);
                    self.pop_scope();
                    let l = self.prog.stmts.push_slice(&[stmt]);
                    (self.prog.add(TExpr::Block(l, tb)), ty)
                }
                Enumerator::Guard(_) => {
                    self.error(span, "a guard directly after a value definition is not supported; move it before the definition");
                    (self.prog.add(TExpr::Unit), ERROR)
                }
            },
        }
    }
}

/// Whether an interpolation's concatenation holds its `i`th part: the first always, another
/// where it is not empty.
fn kept_part(text: &str, i: usize) -> bool {
    !text.is_empty() || i == 0
}

/// Whether the span `at` lies within `span`.
fn within(at: Span, span: Span) -> bool {
    at.start >= span.start && at.end <= span.end
}

/// Whether the expression at `start` of `text` stands in braces of its own, which the parser
/// drops around a block's one expression: a `{` before it, after parentheses, blanks and
/// comments alone, and after `floor` (the enclosing block's start, whose own brace is not one).
fn braced_before(text: &str, start: u32, floor: u32) -> bool {
    let b = text.as_bytes();
    let mut i = start as usize;
    while i > floor as usize {
        let c = b[i - 1];
        if c.is_ascii_whitespace() || c == b'(' {
            i -= 1;
            continue;
        }
        // The end of a block comment: on before its start.
        if c == b'/' && i >= 2 && b[i - 2] == b'*' {
            match text[..i - 2].rfind("/*") {
                Some(open) => {
                    i = open;
                    continue;
                }
                None => return false,
            }
        }
        // The text of a line comment: on before its `//`.
        let line = text[..i].rfind('\n').map_or(0, |n| n + 1);
        if let Some(k) = line_comment(&b[line..i]) {
            i = line + k;
            continue;
        }
        return c == b'{' && i - 1 > floor as usize;
    }
    false
}

/// Where a line comment starts in the line `b`: its first `//` outside a string literal.
fn line_comment(b: &[u8]) -> Option<usize> {
    let mut quoted = false;
    let mut k = 0;
    while k < b.len() {
        match b[k] {
            b'\\' if quoted => k += 1,
            b'"' => quoted = !quoted,
            b'/' if !quoted && b.get(k + 1) == Some(&b'/') => return Some(k),
            _ => {}
        }
        k += 1;
    }
    None
}

/// Whether the written type `t` mentions one of `names`: a class or alias the block defines, or
/// a path through a value of it (`k.type`).
fn ty_names_any(ast: &ast::Ast, t: TyExprId, names: &[Name]) -> bool {
    let any = |l: ListRef| ast.ty_list(l).iter().any(|&x| ty_names_any(ast, x, names));
    match ast.ty(t) {
        TyExpr::Name(n) => names.contains(&n),
        TyExpr::Select(q, _) | TyExpr::Project(q, _) | TyExpr::Singleton(q) | TyExpr::ByName(q) | TyExpr::Repeated(q) => ty_names_any(ast, q, names),
        TyExpr::Unchecked(q) | TyExpr::UncheckedVariance(q) | TyExpr::Refined(q, _) => ty_names_any(ast, q, names),
        TyExpr::Apply(f, l) => ty_names_any(ast, f, names) || any(l),
        TyExpr::Fun(l, r) | TyExpr::CtxFun(l, r) => any(l) || ty_names_any(ast, r, names),
        TyExpr::Tuple(l) => any(l),
        TyExpr::NamedTuple(_, l) => any(l),
        TyExpr::Union(a, b) | TyExpr::Inter(a, b) | TyExpr::BoundedWildcard(a, b) => ty_names_any(ast, a, names) || ty_names_any(ast, b, names),
        TyExpr::Lambda(_, body) | TyExpr::PolyFun(_, body) => ty_names_any(ast, body, names),
        _ => false,
    }
}
