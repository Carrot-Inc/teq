//! `scala.Dynamic`: dotty's `typer/Dynamic.scala`. A selection, an application or an assignment on a qualifier whose
//! type derives from the marker trait `scala.Dynamic`, where the qualifier has no such member (or one that cannot be
//! accessed) and no extension method or conversion gives the name, is rewritten:
//!
//!   qual.name                       ~~> qual.selectDynamic("name")
//!   qual.name[Ts]                   ~~> qual.selectDynamic[Ts]("name")
//!   qual.name(args)                 ~~> qual.applyDynamic("name")(args)
//!   qual.name[Ts](args)             ~~> qual.applyDynamic[Ts]("name")(args)
//!   qual.name(x = a, b)             ~~> qual.applyDynamicNamed("name")(("x", a), ("", b))
//!   qual.name = value               ~~> qual.updateDynamic("name")(value)
//!
//! (`typedDynamicSelect`, `typedDynamicApply`, `typedDynamicAssign`). The rewritten call is typed by the ordinary
//! member application, so the dynamic method's own overloads, type parameters, using clauses and defaults are the
//! class's. A `js.Dynamic` receiver keeps its own path (`apply_dynamic_member`).

use super::apply::{ArgList, ArgSrc};
use super::Worker;
use crate::ast::{Expr, ListRef, TyExpr};
use crate::intern::Name;
use crate::names;
use crate::source::Span;
use crate::tir::{TExpr, TExprId};
use crate::types::*;

impl<'a> Worker<'a> {
    /// `scala.Dynamic`, where the program's classes have brought it in.
    fn dynamic_marker(&self) -> Option<ClassId> {
        self.syms.scoped(self.interner, self.b.scala_pkg, names::DYNAMIC).and_then(|e| e.class)
    }

    /// Whether `name` is one of the four methods the rewrite calls (`Dynamic.isDynamicMethod`): a reference to
    /// one of them is never rewritten again (`isDynamicExpansion`'s first clause).
    pub(super) fn is_dynamic_method(&self, name: Name) -> bool {
        matches!(name, names::SELECT_DYNAMIC | names::APPLY_DYNAMIC | names::UPDATE_DYNAMIC | names::APPLY_DYNAMIC_NAMED)
    }

    /// Whether `t` derives from `scala.Dynamic` and is no `js.Dynamic`: dotty's `Type.derivesFrom(defn.DynamicClass)`
    /// (`core/Types.scala` 283): a class by its base classes; a proxy (a path, an alias, a type parameter, an
    /// abstract member) by what it stands for; an intersection where either part does; a union where both parts do,
    /// a part that is `Nothing` or `Null` left out (`isLowerBottomType`).
    pub(super) fn derives_dynamic(&mut self, t: TypeId) -> bool {
        let Some(marker) = self.dynamic_marker() else { return false };
        self.derives_from_marker(t, marker, 64) && !self.is_js_dynamic(t)
    }

    /// Whether `c` is `scala.Dynamic` itself, for the emitters: a JavaScript class derives from it through
    /// `js.Dynamic` and reaches nothing of it.
    pub fn is_dynamic_marker(&self, c: ClassId) -> bool {
        self.dynamic_marker() == Some(c)
    }

    /// Whether the class `c` is a proper subclass of `scala.Dynamic` (`clazz.derivesFrom(defn.DynamicClass)` of
    /// a class that is not the marker itself).
    pub(super) fn class_derives_dynamic(&mut self, c: ClassId) -> bool {
        match self.dynamic_marker() {
            Some(marker) => c != marker && self.derives_from(c, marker),
            None => false,
        }
    }

    /// Whether `scala.Dynamic` itself is one of the parents the class `c` names (`parents.exists(_.tpe.isRef(
    /// defn.DynamicClass))`).
    pub(super) fn extends_dynamic_itself(&mut self, c: ClassId) -> bool {
        let Some(marker) = self.dynamic_marker() else { return false };
        let parents = self.syms.class(c).parents.clone();
        parents.into_iter().any(|p| {
            let p = self.dealias(p);
            matches!(self.types.get(p), Type::Class(k, _) if k == marker)
        })
    }

    fn derives_from_marker(&mut self, t: TypeId, marker: ClassId, depth: u32) -> bool {
        // A chain of proxies is followed as `class_of` follows it (a bound that grows at every step).
        let Some(depth) = depth.checked_sub(1) else { return false };
        let t = self.solve_in(t);
        let t = self.deref(t);
        match self.types.get(t) {
            Type::Class(c, _) => c == marker || self.derives_from(c, marker),
            Type::Nested(_, class) => self.derives_from_marker(class, marker, depth),
            Type::Inter(a, b) => self.derives_from_marker(a, marker, depth) || self.derives_from_marker(b, marker, depth),
            Type::Union(a, b, _) => {
                if self.is_lower_bottom(a) {
                    self.derives_from_marker(b, marker, depth)
                } else if self.is_lower_bottom(b) {
                    self.derives_from_marker(a, marker, depth)
                } else {
                    self.derives_from_marker(a, marker, depth) && self.derives_from_marker(b, marker, depth)
                }
            }
            Type::Param(p) => {
                let upper = self.syms.tparam(p).upper;
                upper != ANY && self.derives_from_marker(upper, marker, depth)
            }
            Type::AppParam(p, args) => match self.app_param_upper(p, args) {
                Some(upper) => self.derives_from_marker(upper, marker, depth),
                None => false,
            },
            Type::Lit(_) => {
                let class = self.widen_lit(t);
                class != t && self.derives_from_marker(class, marker, depth)
            }
            Type::Match(..) | Type::Alias(..) | Type::This(_) | Type::Term(_) | Type::Select(..) | Type::Member(..) | Type::AppMember(..) | Type::Decl(_) | Type::Refined(..) | Type::Rec(_) => {
                match self.dependent_underlying(t) {
                    Some(under) if under != t => self.derives_from_marker(under, marker, depth),
                    _ => false,
                }
            }
            _ => false,
        }
    }

    /// `Nothing`, or `Null` (the marker is no value class): a union's part that `derivesFrom` leaves out.
    fn is_lower_bottom(&mut self, t: TypeId) -> bool {
        let t = self.deref(t);
        t == NOTHING || matches!(self.types.get(t), Type::Class(c, _) if c == self.b.null)
    }

    /// The rewrite of `recv.name[targs](lists)`, or of `recv.name = rhs` where `dynamic_assign` holds the right
    /// side for this receiver, for a receiver that derives from `scala.Dynamic`: `Ok` with the rewritten call typed
    /// as a member application; `Err` with the lists, untouched, where nothing is rewritten (`dynamic_call`).
    #[cold]
    #[inline(never)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn dynamic_rewrite(
        &mut self,
        recv: TExprId,
        recv_ty: TypeId,
        name: Name,
        targs: Option<ListRef>,
        lists: Vec<ArgList>,
        span: Span,
        expected: Option<TypeId>,
    ) -> Result<(TExprId, TypeId), Vec<ArgList>> {
        if self.every_value_has(name, recv_ty) {
            return Err(lists);
        }
        // `ensuring`, `formatted` and `→`, which `scala.Predef` gives every value by a conversion that scalac
        // tries before the dynamic methods (`Typer.typedSelectWithAdapt`: `tryExtensionOrConversion`, then
        // `dynamicSelect`) and the lean std does not hold: refused there, never a dynamic method's call. The
        // `super` route (`dynamic_call` alone) takes no conversion and is not held back.
        if self.lean_std_lacks(name) && recv_ty != ERROR && self.derives_dynamic(recv_ty) {
            let msg = format!(
                "not supported yet: {}, which scala.Predef gives every value, on a value whose class derives from scala.Dynamic",
                self.name_ref(name)
            );
            self.error_by(crate::source::Group::Own, span, msg);
            self.type_args_for_errors(&lists);
            return Ok((self.prog.add(TExpr::Unit), ERROR));
        }
        // Never twice: the `apply` inserted on a call of one of the four dynamic methods, or on a context
        // function's application of one, is not rewritten (`isDynamicExpansion`'s second and third clauses); the
        // call takes no more parameters.
        if name == names::APPLY && self.inserted_apply == Some(recv) && recv_ty != ERROR {
            if let Some(method) = self.dynamic_expansion(recv) {
                if self.derives_dynamic(recv_ty) {
                    self.error(span, format!("method {method} does not take more parameters"));
                    self.type_args_for_errors(&lists);
                    return Ok((self.prog.add(TExpr::Unit), ERROR));
                }
            }
        }
        // `typedDynamicAssign`: the right side of the assignment whose selection this is.
        let assign = match self.dynamic_assign {
            Some((target, rhs)) if target == recv => Some(rhs),
            _ => None,
        };
        let spans = self.overload_arg_spans.len();
        let applied = match self.dynamic_call(recv_ty, name, lists, span, assign)? {
            None => (self.prog.add(TExpr::Unit), ERROR),
            Some((method, lists, span)) if assign.is_some() => {
                self.dynamic_assign = None;
                self.apply_member(recv, recv_ty, method, None, lists, span, None)
            }
            Some((method, lists, span)) => self.apply_member(recv, recv_ty, method, targs, lists, span, expected),
        };
        self.overload_arg_spans.truncate(spans);
        Ok(applied)
    }

    /// A name every value has, `Any`'s or `AnyRef`'s: scalac finds the member whatever the form it is used in, so
    /// the name is never a dynamic method's; a form of it that is not held here is refused as it is for every
    /// class. `clone` and `finalize` are protected: the member where the site may select it on the receiver
    /// (inside a class the receiver's class is or derives from), a dynamic method's name elsewhere, as scalac
    /// leaves a member it may not access to the dynamic methods.
    pub(super) fn every_value_has(&mut self, name: Name, recv_ty: TypeId) -> bool {
        match self.name_ref(name) {
            "==" | "!=" | "##" | "equals" | "hashCode" | "toString" | "getClass" | "isInstanceOf" | "asInstanceOf" | "eq" | "ne" | "synchronized"
            | "notify" | "notifyAll" | "wait" => true,
            "clone" | "finalize" => {
                let Some(rc) = (recv_ty != ERROR).then(|| self.class_of(recv_ty)).flatten() else { return false };
                let within: Vec<ClassId> = self.env.frames.iter().filter_map(|f| if let super::Frame::Class(k) | super::Frame::Ctor(k) = f { Some(*k) } else { None }).collect();
                within.into_iter().any(|k| k == rc || self.derives_from(rc, k))
            }
            _ => false,
        }
    }

    /// A class of the lean std among the base classes of `t`'s class, other than the marker and those a class, a
    /// case class, an enum or their companions have by their kind (`Any`, `AnyRef`, `AnyVal`, `Matchable`,
    /// `Product`, `Serializable`, `Equals`, `scala.reflect.Enum`, the `Mirror` traits, a JavaScript class's
    /// `Object` and `Any`); each part of an intersection or a union is asked, a union's `Null` or `Nothing` left
    /// out as `derives_dynamic` leaves it.
    fn lean_library_base(&mut self, t: TypeId) -> Option<ClassId> {
        let t = self.solve_in(t);
        let t = self.deref(t);
        if let Type::Inter(a, b) | Type::Union(a, b, _) = self.types.get(t) {
            let first = if self.is_lower_bottom(a) { None } else { self.lean_library_base(a) };
            return if first.is_some() || self.is_lower_bottom(b) { first } else { self.lean_library_base(b) };
        }
        let c = self.class_of(t)?;
        self.complete_class(c);
        let bases: Vec<ClassId> = self.syms.class(c).base_types.iter().map(|&(b, _)| b).collect();
        bases.into_iter().find(|&b| {
            let info = self.syms.class(b);
            // A class of the std's own sources: one read from a jar or from another module's products has every
            // member it was compiled with.
            let file = info.file.0 as usize;
            file < self.files.len()
                && !self.in_jar(info.file)
                && self.files[file].is_std
                && !matches!(self.name_ref(info.name), "Any" | "AnyRef" | "Object" | "AnyVal" | "Matchable" | "Product" | "Serializable" | "Equals" | "Enum" | "Mirror" | "Sum" | "Singleton" | "Dynamic")
        })
    }

    /// Whether `name` is one of the three names of `scala.Predef`'s any-receiver classes (`ArrowAssoc.→`,
    /// `Ensuring.ensuring`, `StringFormat.formatted`: six methods) that the lean std does not define.
    fn lean_std_lacks(&self, name: Name) -> bool {
        !self.scala_library_std() && matches!(self.name_ref(name), "ensuring" | "formatted" | "\u{2192}")
    }

    /// `Dynamic.isDynamicExpansion`'s second and third clauses, read off the value `te` an `apply` is inserted
    /// on: a call of one of the four dynamic methods, or a context function's application of one: the method that
    /// then takes no more parameters, as the reference names it.
    fn dynamic_expansion(&mut self, te: TExprId) -> Option<String> {
        match self.prog.expr(te) {
            TExpr::Block(_, inner) => self.dynamic_expansion(inner),
            TExpr::CallMethod(_, m, _) | TExpr::CallStatic(m, _) if self.is_dynamic_method(self.syms.sym(m).name) => Some(self.method_description(m)),
            TExpr::CallClosure(f, args) => {
                let inner = match self.prog.expr(f) {
                    TExpr::Block(_, inner) => inner,
                    _ => f,
                };
                let (TExpr::CallMethod(_, m, _) | TExpr::CallStatic(m, _)) = self.prog.expr(inner) else { return None };
                if !self.is_dynamic_method(self.syms.sym(m).name) {
                    return None;
                }
                // The dynamic method's result, a context function the application here applies.
                let fty = match self.prog.type_of(f) {
                    Some(t) => t,
                    None => self.sig_of(m).ret,
                };
                self.as_context_function(fty)?;
                Some(format!("apply in trait ContextFunction{}", self.prog.expr_list(args).len()))
            }
            _ => None,
        }
    }

    /// The reference's hint beside a member that is not found on a qualifier that derives from `scala.Dynamic`
    /// (`ErrorReporting`: "maybe a wrong Dynamic method signature?"), a `js.Dynamic` included; empty elsewhere.
    pub(super) fn dynamic_hint(&mut self, t: TypeId) -> &'static str {
        match self.dynamic_marker() {
            Some(marker) if t != ERROR && self.derives_from_marker(t, marker, 64) => "\npossible cause: maybe a wrong Dynamic method signature?",
            _ => "",
        }
    }

    /// The dynamic method and the lists a rewritten `q.name(lists)` applies, `q` of type `recv_ty`
    /// (`updateDynamic` for the right side `assign`): `Err` with the lists, untouched, where `recv_ty` does not
    /// derive from `scala.Dynamic` or `name` is one of the four methods themselves; `Ok(None)` where the rewrite
    /// is itself an error, which is reported.
    pub(super) fn dynamic_call(
        &mut self,
        recv_ty: TypeId,
        name: Name,
        lists: Vec<ArgList>,
        span: Span,
        assign: Option<crate::ast::ExprId>,
    ) -> Result<Option<(Name, Vec<ArgList>, Span)>, Vec<ArgList>> {
        if self.is_dynamic_method(name) || recv_ty == ERROR || !self.derives_dynamic(recv_ty) {
            return Err(lists);
        }
        // On the lean std a class of the std among the receiver's base classes may lack a member that
        // scala-library gives it, and scalac finds the member where the lean std would send its name here: no
        // dynamic call on such a receiver there.
        if !self.scala_library_std() {
            if let Some(base) = self.lean_library_base(recv_ty) {
                let msg = format!(
                    "not supported yet: {}, a name sent to a dynamic method on a value whose class also extends {}, on the lean standard library",
                    self.name_ref(name),
                    self.name_ref(self.syms.class(base).name)
                );
                self.error_by(crate::source::Group::Own, span, msg);
                self.type_args_for_errors(&lists);
                return Ok(None);
            }
        }
        // Two forms the reference builds in its own way and no other call takes: an infix operation's parenthesised
        // list is one tuple to its dynamic method (`Desugar.makeOp`'s `Apply(sel, tuple :: Nil)`, passed on by
        // `typedDynamicApply`), and a written `using` list is passed as the arguments: refused, neither built
        // as an ordinary call's.
        let written_using = lists.iter().any(|l| l.using);
        let infix_list = lists.iter().find(|l| !l.using).is_some_and(|l| l.fun.is_operand() && !l.fun.is_right_operand() && l.args.len() != 1);
        if written_using || infix_list {
            let what = if written_using { "a `using` argument list on a name sent to a dynamic method" } else { "an infix call with a parenthesised argument list on a value whose class derives from scala.Dynamic" };
            self.error_by(crate::source::Group::Own, span, format!("not supported yet: {what}"));
            self.type_args_for_errors(&lists);
            return Ok(None);
        }
        // The `unapply` the desugaring gives a case class's companion is a member under scalac and is none here
        // (the call `C.unapply(x)` is refused for every case class): no dynamic call stands in for it.
        if name == names::UNAPPLY && self.class_of(recv_ty).and_then(|o| self.companion_class(o)).is_some_and(|k| self.syms.class(k).mods & crate::ast::mods::CASE != 0) {
            return Err(lists);
        }
        // The rewritten application's tree is the envelope of the selection and its arguments
        // (`untpd.Apply(coreDynamic(..), args)`, no span of its own): where one list was written it ends with its
        // last argument, which is where a diagnostic of the whole call stands.
        let ast = self.cur_ast();
        let span = {
            let mut written = lists.iter().filter(|l| !l.using);
            match (written.next(), written.next()) {
                (Some(one), None) if assign.is_none() => {
                    let end = one.args.iter().filter_map(|a| match *a {
                        ArgSrc::Ast(e) | ArgSrc::Hoisted(e) => Some(ast.expr_span(e).end),
                        _ => None,
                    });
                    end.max().filter(|&end| end > span.start && end <= span.end).map_or(span, |end| Span::new(span.start, end))
                }
                _ => span,
            }
        };
        // The name as a string constant, of its singleton type (`Literal(Constant(name.toString))`).
        let text = self.name_str(name);
        let key = self.dynamic_name(&text, span);
        if let Some(rhs) = assign {
            let value = ArgList { args: vec![ArgSrc::Ast(rhs)], using: false, span, fun: ArgList::NO_FUN };
            return Ok(Some((names::UPDATE_DYNAMIC, vec![key, value], span)));
        }
        // `typedDynamicSelect`: no argument list was written.
        let Some(first) = lists.iter().position(|l| !l.using) else {
            let mut all = vec![key];
            all.extend(lists);
            return Ok(Some((names::SELECT_DYNAMIC, all, span)));
        };
        // `typedDynamicApply`: `applyDynamicNamed` where an argument of the list is named, each argument then a
        // pair of its name, or "", and its value.
        let is_named = |a: &ArgSrc| match *a {
            ArgSrc::Ast(e) | ArgSrc::Hoisted(e) => matches!(ast.expr(e), Expr::NamedArg(..)),
            ArgSrc::Named(..) => true,
            _ => false,
        };
        let is_splice = |a: &ArgSrc| match *a {
            ArgSrc::Ast(e) | ArgSrc::Hoisted(e) => {
                let v = match ast.expr(e) {
                    Expr::NamedArg(_, v) => v,
                    _ => e,
                };
                matches!(ast.expr(v), Expr::Typed(_, t) if matches!(ast.ty(t), TyExpr::Repeated(_)))
            }
            _ => false,
        };
        let has_named = lists[first].args.iter().any(is_named);
        let plain = lists[first].args.iter().all(|a| !matches!(a, ArgSrc::ForLambda(..)));
        let mut lists = lists;
        let method = if has_named && plain {
            if lists[first].args.iter().any(is_splice) {
                // The reference's error tree: the arguments are not typed.
                self.error(span, "applyDynamicNamed does not support passing a vararg parameter");
                return Ok(None);
            }
            // Each value is typed as the second component of the pair the method's parameter takes, where the
            // class has one `applyDynamicNamed` whose pairs' type does not depend on its own type parameters
            // (dotty types the untyped pairs against the parameter).
            let (label_expected, value_ty) = self.dynamic_named_pair_types(recv_ty);
            let args = std::mem::take(&mut lists[first].args);
            let mut pairs = Vec::with_capacity(args.len());
            for a in args {
                let (label, te, ty, written) = match a {
                    ArgSrc::Ast(e) | ArgSrc::Hoisted(e) => match ast.expr(e) {
                        Expr::NamedArg(n, v) => {
                            let (te, ty) = self.type_expr(v, value_ty);
                            (self.name_str(n), te, ty, Some(ast.expr_span(v)))
                        }
                        _ => {
                            let (te, ty) = self.type_expr(e, value_ty);
                            (String::new(), te, ty, Some(ast.expr_span(e)))
                        }
                    },
                    ArgSrc::Named(n, te, ty) => (self.name_str(n), te, ty, None),
                    ArgSrc::Typed(te, ty) => (String::new(), te, ty, None),
                    ArgSrc::ForLambda(..) => unreachable!("a for's lambda beside a named argument"),
                };
                let label_node = self.prog.add_str(&label);
                let label_node = self.prog.add(TExpr::Str(label_node));
                // The label is a string constant: of its own type where the parameter asks for it (`("x", Int)*`).
                let own = self.types.lit(LitVal::Str(self.interner.intern(&label)));
                let string = self.b.t_string;
                let narrow = label_expected.is_some_and(|exp| {
                    let mark = self.snapshot();
                    let narrow = self.is_sub(own, exp) && !self.is_sub(string, exp);
                    self.rollback(&mark);
                    narrow
                });
                let label_ty = if narrow { own } else { string };
                let ty = self.widen_lit(ty);
                let (pair, pair_ty) = self.construct_tuple(&[label_node, te], &[label_ty, ty], span);
                // The pair stands where its value was written (the reference's tuple has the value's span): a
                // diagnostic of its adaptation is told there (`overload_arg_spans`, which the caller cuts back).
                if let Some(at) = written {
                    self.overload_arg_spans.push((pair, at));
                }
                pairs.push(ArgSrc::Typed(pair, pair_ty));
            }
            lists[first].args = pairs;
            names::APPLY_DYNAMIC_NAMED
        } else {
            names::APPLY_DYNAMIC
        };
        let mut all = vec![key];
        all.extend(lists);
        Ok(Some((method, all, span)))
    }

    /// The list `("name")` of a rewritten call: the name's literal, of its singleton type.
    fn dynamic_name(&mut self, text: &str, span: Span) -> ArgList {
        let s = self.prog.add_str(text);
        let lit = self.prog.add(TExpr::Str(s));
        let lit_ty = self.types.lit(LitVal::Str(self.interner.intern(text)));
        ArgList { args: vec![ArgSrc::Typed(lit, lit_ty)], using: false, span, fun: ArgList::NO_FUN }
    }

    /// The types the label and the value of a named argument are typed against: the components of the `(L, V)`
    /// the second clause of the receiver's one `applyDynamicNamed` takes (a repeated parameter's element), each
    /// where it names none of the method's type parameters; none where the method is overloaded or of another
    /// shape (dotty types the untyped pair against the parameter).
    fn dynamic_named_pair_types(&mut self, recv_ty: TypeId) -> (Option<TypeId>, Option<TypeId>) {
        let found = (|| {
            let (m, owner_ty) = self.find_member(recv_ty, names::APPLY_DYNAMIC_NAMED)?;
            if self.syms.alternatives(m).is_some() {
                return None;
            }
            let sig = self.sig_arc(m);
            let clause = sig.clauses.iter().filter(|c| !c.is_using).nth(1)?;
            let [param] = clause.params.as_slice() else { return None };
            let subst = self.owner_subst(owner_ty);
            let pair = self.types.subst(param.ty, &subst);
            let [label, value] = self.tuple_elements(pair)?[..] else { return None };
            let free = |t: &Self, ty: TypeId| !t.mentions_tparam_of(ty, Some(&sig.tparams));
            Some((free(self, label).then_some(label), free(self, value).then_some(value)))
        })();
        found.unwrap_or((None, None))
    }
}
