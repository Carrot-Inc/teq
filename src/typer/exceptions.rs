use super::apply::{ArgList, ArgSrc};
use super::Worker;
use crate::ast::{ExprId, ListRef, Pat};
use crate::intern::Name;
use crate::names;
use crate::source::Span;
use crate::symbols::*;
use crate::tir::*;
use crate::types::*;

impl<'a> Worker<'a> {
    /// `java.lang.Throwable`, the type of what is thrown and caught.
    pub fn throwable_type(&mut self, span: Span) -> TypeId {
        match self.throwable_class() {
            Some(c) => self.types.class(c, &[]),
            None => {
                self.error(span, "java.lang.Throwable is missing from the standard library");
                ERROR
            }
        }
    }

    pub fn type_throw(&mut self, e: ExprId, span: Span) -> (TExprId, TypeId) {
        let throwable = self.throwable_type(span);
        let (te, ty) = self.type_expr(e, Some(throwable));
        let te = self.adapt(te, ty, throwable, self.cur_ast().expr_span(e));
        let ty = self.solve_in(ty);
        let constructed = match self.prog.expr(te) {
            TExpr::New(c, _) => Some(c),
            _ => None,
        };
        let unwrap = match constructed {
            Some(c) => Some(c) == self.b.js_exception,
            None => self.may_be_js_exception(ty),
        };
        (self.prog.add(TExpr::Throw(te, unwrap)), NOTHING)
    }

    /// Whether a value of type `t` can be a `JavaScriptException`, whose raw value JavaScript
    /// sees when it is thrown and which a catch has to wrap for.
    fn may_be_js_exception(&mut self, t: TypeId) -> bool {
        let Some(jse) = self.js_exception_class() else { return false };
        let jse_ty = self.types.class(jse, &[]);
        let mark = self.snapshot();
        let can = self.is_sub(jse_ty, t);
        self.rollback(mark);
        can
    }

    pub fn type_try(&mut self, index: u32, span: Span, expected: Option<TypeId>) -> (TExprId, TypeId) {
        let ast = self.cur_ast();
        let t = ast.try_expr(index);
        if t.cases.is_empty() && t.handler.is_none() && t.finalizer.is_none() {
            let msg = "A try without catch or finally is equivalent to putting its body in a block; no exceptions are handled.";
            self.warn(span, msg);
        }
        let exp = self.branch_expected(expected);
        let guide = if exp.is_none() { Some(self.branch_guide(expected)) } else { None };
        let branch = |this: &mut Self, e: ExprId| -> (TExprId, TypeId) {
            match exp {
                Some(t) => (this.check_expr(e, t), t),
                None => {
                    let r = this.type_expr(e, guide);
                    this.guide_with(guide.unwrap(), r.1);
                    r
                }
            }
        };
        let (body, bty) = branch(self, t.body);
        let throwable = self.throwable_type(span);
        let mut cases: Vec<TCase> = Vec::with_capacity(t.cases.len as usize + 1);
        let mut branches: Vec<(TExprId, TypeId)> = vec![(body, bty)];
        let mut caught_all = false;
        for c in ast.case_list(t.cases).to_vec() {
            self.push_scope();
            let binders_mark = self.case_binders.len();
            let was_open = std::mem::replace(&mut self.gadt_open, true);
            let was_dead = std::mem::replace(&mut self.dead_case, caught_all);
            let pat = self.type_pattern(c.pat, throwable);
            self.dead_case = was_dead;
            caught_all |= c.guard.is_none() && self.catches_all(c.pat);
            self.gadt_open = was_open;
            let guard = c.guard.map(|g| self.check_expr(g, self.b.t_boolean));
            let (te, ty) = branch(self, c.body);
            self.case_binders.truncate(binders_mark);
            self.pop_scope();
            cases.push(TCase { pat, guard, body: te });
            branches.push((te, ty));
        }
        if let Some(h) = t.handler {
            let (case, ty) = self.handler_case(h, throwable, exp, guide, span);
            cases.push(case);
            branches.push((case.body, ty));
        }
        let finalizer = t.finalizer.map(|f| self.check_expr(f, self.b.t_unit));
        let result_ty = match exp {
            Some(t) => t,
            None => {
                self.harmonize_literals(&mut branches);
                let mut acc = NOTHING;
                for &(_, t) in &branches {
                    let t = self.solve_bounded_in(t);
                    acc = if acc == NOTHING { t } else { self.lub(acc, t) };
                }
                self.solve_in(acc)
            }
        };
        let wraps = self.catch_wraps(&cases);
        let cases = self.prog.cases.push_slice(&cases);
        let i = self.prog.tries.len() as u32;
        self.prog.tries.push(TTry { body, cases, finalizer, wraps });
        let e = self.prog.add(TExpr::Try(i));
        if exp.is_none() {
            self.note_soft(e, result_ty);
        }
        (e, result_ty)
    }

    /// `catch handler` with an expression: a case that takes every `Throwable` and applies the
    /// handler to it, through `applyOrElse` with a rethrow for a partial function, as scalac
    /// desugars it.
    fn handler_case(
        &mut self,
        handler: ExprId,
        throwable: TypeId,
        exp: Option<TypeId>,
        guide: Option<TypeId>,
        span: Span,
    ) -> (TCase, TypeId) {
        let ex = self.fresh_local("e", throwable, span);
        let test = self.prog.add_test(TypeTest::Always);
        let bind = self.prog.add_pat(TPat::Bind(ex, None));
        let pat = self.prog.add_pat(TPat::Test(test, throwable, bind));
        let expected_fn = exp.map(|r| self.fun_type(&[throwable], r));
        let (th, hty) = self.type_expr(handler, expected_fn);
        let hty = self.solve_in(hty);
        let arg = self.prog.add(TExpr::Local(ex));
        self.prog.set_type(arg, throwable);
        let partial = self.as_partial_function(hty).is_some();
        let (method, args) = if partial {
            let t = self.fresh_local("t", throwable, span);
            let thrown = self.prog.add(TExpr::Local(t));
            self.prog.set_type(thrown, throwable);
            let rethrow = self.prog.add(TExpr::Throw(thrown, true));
            let params = self.prog.syms(&[t]);
            let lambda = self.prog.add(TExpr::Lambda(params, rethrow));
            let fn_ty = self.fun_type(&[throwable], NOTHING);
            (names::APPLY_OR_ELSE, vec![ArgSrc::Typed(arg, throwable), ArgSrc::Typed(lambda, fn_ty)])
        } else {
            (names::APPLY, vec![ArgSrc::Typed(arg, throwable)])
        };
        let lists = vec![ArgList { args, using: false, span }];
        let (body, ty) = self.apply_member(th, hty, method, None, lists, span, exp.or(guide));
        let (body, ty) = match exp {
            Some(t) => (self.adapt(body, ty, t, span), t),
            None => {
                self.guide_with(guide.unwrap(), ty);
                (body, ty)
            }
        };
        (TCase { pat, guard: None, body }, ty)
    }

    /// Whether a case could take a `JavaScriptException`, so that a caught value that is no
    /// `Throwable` has to be wrapped in one before the cases see it.
    fn catch_wraps(&mut self, cases: &[TCase]) -> bool {
        let Some(jse) = self.js_exception_class() else { return false };
        let jse_ty = self.types.class(jse, &[]);
        cases.iter().any(|c| self.pattern_takes(c.pat, jse, jse_ty))
    }

    fn pattern_takes(&mut self, p: TPatId, jse: ClassId, jse_ty: TypeId) -> bool {
        match self.prog.pats[p.idx()] {
            TPat::Wildcard | TPat::Unapply(..) => true,
            TPat::Bind(_, inner) => inner.map_or(true, |i| self.pattern_takes(i, jse, jse_ty)),
            TPat::Test(_, t, inner) => {
                let mark = self.snapshot();
                let below = self.is_sub(jse_ty, t);
                self.rollback(mark);
                below && self.pattern_takes(inner, jse, jse_ty)
            }
            TPat::Class(c, ..) => c == jse,
            TPat::Alt(items) => {
                let items = self.prog.pat_lists[items.range()].to_vec();
                items.into_iter().any(|i| self.pattern_takes(i, jse, jse_ty))
            }
            TPat::Equals(..) | TPat::Seq(..) => false,
        }
    }

    pub fn type_null(&mut self) -> (TExprId, TypeId) {
        (self.prog.add(TExpr::Null), self.b.t_null)
    }

    /// `Null <: t`: every reference type takes `null`, a value type, `Nothing`, a literal type
    /// and an opaque type do not.
    pub fn null_conforms(&mut self, t: TypeId) -> bool {
        let t = self.deref(t);
        match self.types.get(t) {
            Type::Any => true,
            Type::Class(c, _) => c == self.b.null || self.is_reference_class(c),
            Type::Union(a, b) => self.null_conforms(a) || self.null_conforms(b),
            Type::Inter(a, b) => self.null_conforms(a) && self.null_conforms(b),
            Type::Term(_) | Type::This(_) | Type::Select(..) => match self.dependent_underlying(t) {
                Some(under) => self.null_conforms(under),
                None => false,
            },
            // An abstract type takes `null` through a lower bound only (`T >: Null`), as scalac
            // has it: a `T <: AnyRef` may still stand for a class without `null`.
            Type::Param(p) => {
                let lower = self.syms.tparam(p).lower;
                lower != NOTHING && self.null_conforms(lower)
            }
            Type::Member(..) | Type::AppMember(..) | Type::Decl(_) => {
                let (lower, _) = self.member_bounds(t);
                lower != NOTHING && lower != t && self.null_conforms(lower)
            }
            Type::Refined(parent, _) => self.null_conforms(parent),
            Type::Match(..) | Type::Alias(..) | Type::Nested(..) => match self.dependent_underlying(t) {
                Some(u) => self.null_conforms(u),
                None => false,
            },
            _ => false,
        }
    }

    /// `t <: AnyRef`: a class, trait, function, tuple or `String`, `Null`, and a type parameter
    /// bounded by one; not `Any`, a value type or an opaque type.
    pub fn is_reference(&mut self, t: TypeId) -> bool {
        let t = self.deref(t);
        match self.types.get(t) {
            Type::Nothing => true,
            Type::Class(c, _) => c == self.b.null || self.is_reference_class(c),
            Type::Union(a, b) => self.is_reference(a) && self.is_reference(b),
            Type::Inter(a, b) => self.is_reference(a) || self.is_reference(b),
            Type::Param(p) => {
                let upper = self.syms.tparam(p).upper;
                upper != ANY && self.is_reference(upper)
            }
            Type::Lit(_) => {
                let class = self.widen_lit(t);
                self.is_reference(class)
            }
            Type::This(_) | Type::Term(_) | Type::Select(..) | Type::Member(..) | Type::AppMember(..) | Type::Decl(_) | Type::Refined(..) | Type::Match(..) | Type::Alias(..) | Type::Nested(..) => {
                match self.dependent_underlying(t) {
                    Some(u) => self.is_reference(u),
                    None => false,
                }
            }
            _ => false,
        }
    }

    pub(super) fn is_reference_class(&self, c: ClassId) -> bool {
        let b = &self.b;
        if [b.int, b.long, b.double, b.byte, b.short, b.float, b.boolean, b.char, b.unit, b.null, b.any_val].contains(&c) {
            false
        } else if c == b.string || c == b.array || c == b.any_ref {
            true
        } else {
            let info = self.syms.class(c);
            !matches!(info.kind, ClassKind::Opaque) && !info.value_class
        }
    }

    /// Whether every value of `t` is a primitive or an instance of a value class: what
    /// conforms to `AnyVal`.
    pub fn is_value(&mut self, t: TypeId) -> bool {
        let t = self.deref(t);
        match self.types.get(t) {
            Type::Nothing => true,
            Type::Class(c, _) => {
                let b = &self.b;
                [b.int, b.long, b.double, b.float, b.byte, b.short, b.boolean, b.char, b.unit, b.any_val].contains(&c) || self.syms.class(c).value_class
            }
            Type::Union(a, b) => self.is_value(a) && self.is_value(b),
            Type::Inter(a, b) => self.is_value(a) || self.is_value(b),
            Type::Param(p) => {
                let upper = self.syms.tparam(p).upper;
                upper != ANY && self.is_value(upper)
            }
            Type::Lit(_) => {
                let class = self.widen_lit(t);
                self.is_value(class)
            }
            Type::This(_) | Type::Term(_) | Type::Select(..) | Type::Member(..) | Type::AppMember(..) | Type::Decl(_) | Type::Refined(..) | Type::Match(..) | Type::Alias(..) | Type::Nested(..) => {
                match self.dependent_underlying(t) {
                    Some(u) => self.is_value(u),
                    None => false,
                }
            }
            _ => false,
        }
    }

    /// `x(p)` where the stable identifier `x`, an object, a val or a path, has an `unapply`
    /// method: the scrutinee goes through it and the sub-patterns match its result, one for
    /// an `Option[T]`, several for an `Option[(A, B, ..)]`; or an `unapplySeq`, whose
    /// `Option[Seq[T]]` the sub-patterns match element by element, `rest*` included.
    pub fn extractor_pattern(&mut self, path: ExprId, subs: &[crate::ast::PatId], sty: TypeId, span: Span) -> Option<TPatId> {
        let n = self.diags.items.len();
        let (recv, rty) = self.type_expr(path, None);
        self.drop_reported_since(n);
        // A library body's extractor is what scalac typed, an interpolator's object on a
        // `StringContext` built in the pattern included.
        if rty == ERROR || !(self.stable_receiver(recv) || self.is_body_file(self.env.file)) {
            return None;
        }
        self.extractor_pattern_on(recv, rty, subs, sty, span)
    }

    /// `case s"a-$x"`: `StringContext(parts).s.unapplySeq(scrutinee)` with the holes as the
    /// sub-patterns, `s` being the std's or an extension's object, as scalac desugars it.
    pub fn interp_pattern(&mut self, kind: Name, parts: ListRef, holes: &[crate::ast::PatId], sty: TypeId, span: Span) -> TPatId {
        let Some(class) = self.string_context_class() else {
            self.error(span, "StringContext is missing from the standard library");
            return self.prog.add_pat(TPat::Wildcard);
        };
        self.complete_class(class);
        let ast = self.cur_ast();
        let mut items = Vec::with_capacity(parts.len as usize);
        for &p in &ast.str_lists[parts.range()] {
            let r = self.prog.add_str(ast.str(p));
            items.push(self.prog.add(TExpr::Str(r)));
        }
        let l = self.prog.list(&items);
        let seq = self.prog.add(TExpr::SeqLit(l));
        if self.capturing() {
            self.capture_form(seq, crate::tir::capture::Form::Repeated(self.b.t_string));
            self.capture_written_parts(parts, &items);
        }
        let ctor_args = self.prog.list(&[seq]);
        let context = self.prog.add(TExpr::New(class, ctor_args));
        let context_ty = self.types.class(class, &[]);
        let n = self.diags.items.len();
        let (recv, rty) = self.apply_member(context, context_ty, kind, None, Vec::new(), span, None);
        if self.diags.items.len() > n || rty == ERROR {
            return self.prog.add_pat(TPat::Wildcard);
        }
        match self.extractor_pattern_on(recv, rty, holes, sty, span) {
            Some(p) => p,
            None => {
                let msg = format!("{} is not an extractor: it has no unapply or unapplySeq for an interpolated pattern", self.name_str(kind));
                self.error(span, msg);
                self.prog.add_pat(TPat::Wildcard)
            }
        }
    }

    /// Whether the alternatives of an overloaded extractor all take the scrutinee only through
    /// a type test and none of their inputs, seen from the receiver, is more specific than the
    /// rest: dotty's overload resolution of `unapply` against the selector, every alternative
    /// applicable through the test it adds, reports the ambiguity (`neg/i2378`).
    fn ambiguous_unapply(&mut self, set: SymId, owner_ty: TypeId, recv: TExprId, rty: TypeId, sty: TypeId) -> bool {
        let alts: Vec<SymId> = self.syms.alternatives(set).map(|a| a.to_vec()).unwrap_or_default();
        let owner_subst = self.owner_subst(owner_ty);
        let prefix = self.path_of(recv).unwrap_or(rty);
        let c = self.class_of(rty);
        let mut inputs = Vec::new();
        for s in alts {
            let usig = self.sig_of(s);
            if !usig.tparams.is_empty() {
                return false;
            }
            let Some(pty) = usig.clauses.iter().find(|cl| !cl.is_using && !cl.is_implicit).and_then(|cl| (cl.params.len() == 1).then(|| cl.params[0].ty)) else { continue };
            let pty = self.types.subst(pty, &owner_subst);
            let pty = match c {
                Some(c) if self.types.has_paths(pty) => self.as_seen_from(pty, prefix, c),
                _ => pty,
            };
            if self.types.contains_error(pty) {
                return false;
            }
            inputs.push(pty);
        }
        if inputs.len() < 2 {
            return false;
        }
        let mark = self.snapshot();
        let any_conforms = inputs.iter().any(|&t| self.is_sub(sty, t));
        self.rollback(mark);
        if any_conforms {
            return false;
        }
        let most_specific = (0..inputs.len()).any(|i| {
            (0..inputs.len()).all(|j| {
                let mark = self.snapshot();
                let holds = i == j || self.is_sub(inputs[i], inputs[j]);
                self.rollback(mark);
                holds
            })
        });
        !most_specific
    }

    fn extractor_pattern_on(&mut self, recv: TExprId, rty: TypeId, subs: &[crate::ast::PatId], sty: TypeId, span: Span) -> Option<TPatId> {
        let (name, unapply, owner_ty) = match self.find_member(rty, names::UNAPPLY) {
            Some((s, o)) => (names::UNAPPLY, s, o),
            None => {
                let (s, o) = self.find_member(rty, names::UNAPPLY_SEQ)?;
                (names::UNAPPLY_SEQ, s, o)
            }
        };
        let overloaded = self.syms.alternatives(unapply).is_some_and(|alts| alts.len() > 1);
        if overloaded && self.ambiguous_unapply(unapply, owner_ty, recv, rty, sty) {
            let msg = format!(
                "Ambiguous overload. The overloaded alternatives of method {} in {} both match a scrutinee of type {}",
                self.name_str(name),
                self.class_description(match self.syms.sym(unapply).owner {
                    Owner::Class(c) => c,
                    _ => return None,
                }),
                self.show(sty)
            );
            self.error(span, msg);
            for &s in subs {
                self.type_pattern(s, ERROR);
            }
            return Some(self.prog.add_pat(TPat::Wildcard));
        }
        let unapply = self.syms.alternatives(unapply).and_then(|alts| alts.first().copied()).unwrap_or(unapply);
        // A scrutinee wider than what `unapply` takes is tested against that type first.
        // The parameter is seen from the receiver's type (`m: Matcher[A, B]`).
        // The scrutinee goes to the first clause that is not a using clause: an extractor of a
        // macro takes `(using Quotes)` first.
        let (param, tparams) = {
            let usig = self.sig_of(unapply);
            let param = usig.clauses.iter().find(|cl| !cl.is_using && !cl.is_implicit).and_then(|cl| cl.params.first()).map(|p| p.ty);
            (param, usig.tparams.clone())
        };
        let owner_subst = self.owner_subst(owner_ty);
        let taken = match param {
            Some(pty) if !self.types.has_vars(pty) && !self.types.contains_error(pty) => {
                let pty = self.types.subst(pty, &owner_subst);
                // Seen from the receiver as its call sees it (`Path` of `i.Path.AtField` is `i.Path`).
                let pty = match self.class_of(rty) {
                    Some(c) if self.types.has_paths(pty) => {
                        let prefix = self.path_of(recv).unwrap_or(rty);
                        self.as_seen_from(pty, prefix, c)
                    }
                    _ => pty,
                };
                if tparams.is_empty() { pty } else { self.generic_extractor_input(pty, &tparams, sty, &owner_subst) }
            }
            _ => sty,
        };
        let mark = self.snapshot();
        let conforms = self.is_sub(sty, taken);
        self.rollback(mark);
        // An abstract input type goes through the `TypeTest` or the `ClassTag` in scope, as
        // `typedUnApply` retypes it through `tryWithTypeTest` (Applications.scala 1958): the
        // extractor sees only what the tag's `unapply` gives.
        let tag = if !conforms && self.quote.level == 0 && self.abstract_pattern_type(taken) {
            self.pattern_tag(taken, sty, span).and_then(|(tag, tag_ty)| self.tag_call(tag, tag_ty, sty, span))
        } else {
            None
        };
        if !conforms && tag.is_none() {
            // An overloaded `unapply` is typed by its first alternative, not by scalac's choice.
            if !overloaded {
                self.check_sensical_test(taken, sty, span);
            }
            // An input of an abstract type no tag tests (`Types.this.Type[A$1]`, `Plain.this.T`):
            // the test passes any value, which scalac warns of (E092).
            if self.abstract_input(taken) && !self.checks_inline_definition() && self.inline.depth == 0 && !self.input_unchecked(unapply) {
                let msg = format!("the type test for {} cannot be checked at runtime because it refers to an abstract type member or type parameter", self.show(taken));
                self.warn(span, msg);
            }
        }
        let (narrowed, test) = if conforms || tag.is_some() { (if conforms { sty } else { taken }, None) } else { (taken, Some(self.test_for(taken, sty, span, true))) };
        let scrut = self.fresh_local("u", narrowed, span);
        let arg = self.prog.add(TExpr::Local(scrut));
        self.prog.set_type(arg, narrowed);
        let lists = vec![ArgList { args: vec![ArgSrc::Typed(arg, narrowed)], using: false, span }];
        let (call, rty) = self.apply_member(recv, rty, name, None, lists, span, None);
        let rty = self.solve_in(rty);
        let rty = self.widen_lit(rty);
        // The extractor's result at the pattern, whose shape signature help shows.
        if self.index.is_some() {
            self.index_value(span, call, rty);
        }
        let option = self.b.option.and_then(|o| self.base_type(rty, o));
        let inner = match option.map(|o| self.types.get(o)) {
            Some(Type::Class(_, args)) if name == names::UNAPPLY_SEQ => {
                let value = self.types.items(args)[0];
                self.option_seq_pattern(value, subs)
            }
            Some(Type::Class(_, args)) => {
                let value = self.types.items(args)[0];
                self.option_pattern(value, subs, span)
            }
            _ if rty == self.b.t_boolean && subs.is_empty() => {
                let yes = self.prog.add(TExpr::Bool(true));
                self.prog.add_pat(TPat::Equals(yes, true))
            }
            // An irrefutable extractor: a product (a tuple or a case class) whose fields are
            // matched, as Scala 3's product match has it.
            _ if self.product_result(rty).is_some() => {
                let (tc, value) = self.product_result(rty).unwrap();
                match self.class_field_types(value) {
                    Some(fields) if fields.len() == subs.len() => {
                        let field_syms: Vec<SymId> = self.syms.class(tc).ctor_syms.concat();
                        let mut pats = Vec::with_capacity(subs.len());
                        for (&p, &fty) in subs.iter().zip(&fields) {
                            pats.push(self.type_pattern(p, fty));
                        }
                        match self.xxl_result_pattern(tc, value, &pats, span) {
                            Some(p) => p,
                            None => {
                                let fl = self.prog.syms(&field_syms);
                                let pl = self.prog.pat_lists.push_slice(&pats);
                                self.prog.add_pat(TPat::Class(tc, value, fl, pl))
                            }
                        }
                    }
                    _ => {
                        let msg = format!("wrong number of patterns for an extractor of {}: found {}", self.show(value), subs.len());
                        self.error(span, msg);
                        self.prog.add_pat(TPat::Wildcard)
                    }
                }
            }
            // A name-based extractor (`isEmpty`/`get`) is left out: the caller reports the class.
            _ => return None,
        };
        let pat = self.prog.add_pat(TPat::Unapply(scrut, call, inner));
        // An `unapply` typed as a `Some` cannot fail, nor can one whose result is a product (a
        // tuple or a case class): the pattern is irrefutable where the sub-patterns are, over
        // what passed the test of its input.
        let some = self.std_class("Some");
        let irrefutable = match option {
            Some(_) => some.is_some() && self.class_of(rty) == some,
            None => self.product_result(rty).is_some(),
        };
        if irrefutable {
            self.irrefutable_pats.insert(pat, ());
        }
        if let Some((tag_scrut, tag_call)) = tag {
            return self.tag_result_pattern(tag_scrut, tag_call, narrowed, pat, sty);
        }
        Some(match test {
            Some(t) => self.prog.add_pat(TPat::Test(t, narrowed, pat)),
            None => pat,
        })
    }

    /// What a generic `unapply` takes, as scalac's `typedUnApply` instantiates it: the
    /// scrutinee's type where it conforms to the parameter, the parameters inferred from it by
    /// the call; otherwise the parameter with each type parameter bounded by the scrutinee and
    /// maximised, as `maximizeType` does: up to its upper bound, or down to its lower bound
    /// where the parameter's type is contravariant in it (`unapply[T <: Apply](t: T)` over a
    /// `Tree` takes an `Apply`, `unapply[T](t: List[T])` over an `Any` a `List[Any]`,
    /// `unapply[T](s: Sink[T])` over an `Any` a `Sink[Nothing]`). The bounds are seen from the
    /// receiver's type, as the parameter is.
    fn generic_extractor_input(&mut self, pty: TypeId, tparams: &[TParamId], sty: TypeId, owner_subst: &Subst) -> TypeId {
        let mark = self.snapshot();
        let vars: Subst = tparams.iter().map(|&p| (p, self.fresh_var())).collect();
        for &(p, v) in &vars {
            let (upper, lower) = (self.syms.tparam(p).upper, self.syms.tparams[p.idx()].lower);
            if upper != ANY {
                let upper = self.types.subst(upper, owner_subst);
                let upper = self.types.subst(upper, &vars);
                self.is_sub(v, upper);
            }
            if lower != NOTHING {
                let lower = self.types.subst(lower, owner_subst);
                let lower = self.types.subst(lower, &vars);
                self.is_sub(lower, v);
            }
        }
        let with_vars = self.types.subst(pty, &vars);
        let bounded = self.snapshot();
        if self.is_sub(sty, with_vars) {
            self.rollback(mark);
            return sty;
        }
        self.rollback(bounded);
        // A class or a type parameter of the `unapply` is a type its input is tested against,
        // and so is an abstract type over a scrutinee of a class (`Types.this.Type[A$1]` over an
        // `Any`, unchecked); over a scrutinee of an abstract type (chimney's extractors'
        // `Types.this.Type[A]`, which teq may fail to equate with the scrutinee's
        // `IterableOrArrays.this.Type[M]` where scalac does) the scrutinee is passed as it is,
        // the call inferring the parameters.
        let with_vars_head = self.dealias(with_vars);
        if !matches!(self.types.get(with_vars_head), Type::Class(..) | Type::Var(_)) && self.abstract_scrutinee(sty) {
            self.rollback(mark);
            return sty;
        }
        if !self.is_sub(with_vars, sty) {
            self.rollback(bounded);
        }
        let variances = self.var_variances(with_vars);
        let mut chosen: Subst = Vec::with_capacity(vars.len());
        let mut unsettled = false;
        for &(p, v) in &vars {
            let Type::Var(tv) = self.types.get(v) else { continue };
            let downwards = variances.iter().any(|&(w, sign)| w == tv && sign < 0);
            let invariant = variances.iter().any(|&(w, sign)| w == tv && sign == 0);
            let info = &self.tvars[tv];
            let choice = match (info.inst, downwards) {
                (Some(_), _) => self.zonk(v),
                // At an invariant position (`unapply[T](b: Box[T])` over an `Any`) nothing to
                // maximise to: a fresh abstract type within the parameter's bounds, as
                // `maximizeType` instantiates the variable with a pattern-bound symbol (`T$1`)
                // where its bounds do not meet.
                (None, _) if invariant => self.fresh_extractor_param(tv, p),
                (None, true) if info.lower.is_empty() => NOTHING,
                (None, true) => {
                    self.solve_var(tv);
                    self.zonk(v)
                }
                (None, false) => self.meet_of_uppers(tv),
            };
            // A parameter its bounds leave nothing to maximise to (`C <: SeqOps[A, CC, C]` of `+:`
            // over a `View`) gives no type to test: the call reports the scrutinee, as before.
            unsettled |= (choice == NOTHING && !downwards) || self.types.has_vars(choice);
            chosen.push((p, choice));
        }
        let taken = self.types.subst(pty, &chosen);
        let taken = self.zonk(taken);
        self.rollback(mark);
        if unsettled || self.types.has_vars(taken) { sty } else { taken }
    }

    /// The type an extractor's type parameter `p`, left open by the scrutinee at an invariant
    /// position of what `unapply` takes, binds: its variable's upper bound where that is below
    /// its lower one, else a fresh abstract type between them (`T$1`), as dotty's
    /// `maximizeType` makes one.
    fn fresh_extractor_param(&mut self, tv: TVarId, p: TParamId) -> TypeId {
        let upper = self.meet_of_uppers(tv);
        let lowers = self.tvars[tv].lower.clone();
        let mut lower = NOTHING;
        for l in lowers {
            let l = self.zonk(l);
            lower = if lower == NOTHING { l } else { self.types.mk(Type::Union(lower, l)) };
        }
        let mark = self.snapshot();
        let collapsed = self.is_sub(upper, lower) && self.snapshot() == mark;
        self.rollback(mark);
        // Bounds that meet, or an upper bound of a final class (`A <: String`), instantiate the
        // variable to its upper bound, as `maximizeType` does.
        if collapsed || self.final_bound(upper) {
            return upper;
        }
        let name = self.syms.tparam(p).name;
        let name = self.interner.intern(&format!("{}$1", self.name_str(name)));
        let fresh = self.syms.new_tparam(name, 0);
        let info = &mut self.syms.tparams[fresh.idx()];
        (info.upper, info.lower) = (upper, lower);
        self.types.param(fresh)
    }

    /// Whether the parameter `unapply` takes the scrutinee by is written `T @unchecked`, which
    /// silences its test's warning as on a type pattern.
    fn input_unchecked(&mut self, unapply: SymId) -> bool {
        let info = self.syms.sym(unapply);
        let (Some(d), file) = (info.def, info.file) else { return false };
        let ast = self.ast(file);
        let crate::ast::DefKind::Fun(f) = &ast.def(d).kind else { return false };
        let Some(ty) = f.clauses.iter().find(|c| !c.is_using && !c.is_implicit).and_then(|c| c.params.first()).map(|p| p.ty) else { return false };
        matches!(ast.ty(ty), crate::ast::TyExpr::Unchecked(_))
    }

    /// Whether an extractor's input type is an abstract type member's (applied or not), whose
    /// test cannot be checked at run time.
    fn abstract_input(&mut self, taken: TypeId) -> bool {
        let t = self.deref(taken);
        matches!(self.types.get(t), Type::Member(..) | Type::AppMember(..))
    }

    /// Whether the scrutinee `sty` is, or has a part that is, an abstract type: no class whose
    /// test an extractor's abstract input type stands in for.
    fn abstract_scrutinee(&mut self, sty: TypeId) -> bool {
        let t = self.deref(sty);
        match self.types.get(t) {
            Type::Class(..) | Type::Any | Type::Lit(_) => false,
            Type::Union(a, b) | Type::Inter(a, b) => self.abstract_scrutinee(a) || self.abstract_scrutinee(b),
            Type::Alias(..) => {
                let e = self.deref_alias(t);
                e == t || self.abstract_scrutinee(e)
            }
            _ => true,
        }
    }

    /// Whether the type `t` is of a final class (`String`, `Int`, a `final class`), which no
    /// other class's values are of.
    fn final_bound(&mut self, t: TypeId) -> bool {
        let t = self.deref(t);
        let Type::Class(c, _) = self.types.get(t) else { return false };
        let info = self.syms.class(c);
        let b = &self.b;
        // `AnyVal` and `AnyRef` are no final classes, whatever the builtin layer's modifiers say.
        if c == b.any_val || c == b.any_ref {
            return false;
        }
        info.mods & crate::ast::mods::FINAL != 0
            || matches!(info.kind, ClassKind::Object | ClassKind::EnumCase)
            || [b.int, b.long, b.double, b.byte, b.short, b.float, b.boolean, b.char, b.unit, b.string].contains(&c)
    }

    /// The intersection of a variable's upper bounds, `Any` without one.
    fn meet_of_uppers(&mut self, v: TVarId) -> TypeId {
        let uppers = self.tvars[v].upper.clone();
        let mut meet = ANY;
        for u in uppers {
            let u = self.zonk(u);
            meet = if meet == ANY { u } else { self.types.inter(meet, u) };
        }
        self.narrower_of_meet(meet)
    }

    /// `A & B` as the narrower of the two where one conforms to the other, as scalac's `glb`
    /// normalises the bounds it meets (`Apply & Tree` is `Apply`).
    fn narrower_of_meet(&mut self, t: TypeId) -> TypeId {
        let Type::Inter(a, b) = self.types.get(t) else { return t };
        let (a, b) = (self.narrower_of_meet(a), self.narrower_of_meet(b));
        let mark = self.snapshot();
        let a_below = self.is_sub(a, b) && self.snapshot() == mark;
        self.rollback(mark);
        if a_below {
            return a;
        }
        let b_below = self.is_sub(b, a) && self.snapshot() == mark;
        self.rollback(mark);
        if b_below { b } else { self.types.mk(Type::Inter(a, b)) }
    }

    /// A path: an object, `this`, a local, a top-level or object member, and a field or a
    /// parameterless call through one of those. scalac takes a `var` or a `def` too, reading
    /// it when the pattern runs.
    fn stable_receiver(&self, e: TExprId) -> bool {
        match self.prog.expr(e) {
            TExpr::Module(_) | TExpr::This | TExpr::Local(_) | TExpr::Static(_) => true,
            TExpr::CallStatic(_, args) => self.prog.expr_list(args).is_empty(),
            TExpr::Field(r, _) => self.stable_receiver(r),
            TExpr::CallMethod(r, _, args) => self.prog.expr_list(args).is_empty() && self.stable_receiver(r),
            _ => false,
        }
    }

    /// `Some(seq)` of an `unapplySeq`, with the sub-patterns against the elements of the
    /// sequence, as the sequence patterns of `List(a, b, rest*)` have them.
    fn option_seq_pattern(&mut self, value: TypeId, subs: &[crate::ast::PatId]) -> TPatId {
        let some = self.std_class("Some");
        let Some(some) = some else { return self.prog.add_pat(TPat::Wildcard) };
        self.complete_class(some);
        let field = self.syms.class(some).ctor.first().and_then(|cl| cl.params.first()).map(|p| p.sym);
        let Some(field) = field else { return self.prog.add_pat(TPat::Wildcard) };
        let value = self.dealias(value);
        // `Some((name, children))`: the fields before the last match their patterns, the last
        // one, a sequence, the patterns that remain.
        if let Some((tc, _)) = self.product_result(value) {
            if let Some(fields) = self.class_field_types(value) {
                let fixed_n = fields.len() - 1;
                if subs.len() >= fixed_n && fields.last().map_or(false, |&l| self.seq_class().map_or(false, |seq| self.base_type(l, seq).is_some())) {
                    let field_syms: Vec<SymId> = self.syms.class(tc).ctor_syms.concat();
                    let mut pats = Vec::with_capacity(fields.len());
                    for (i, &fty) in fields[..fixed_n].iter().enumerate() {
                        pats.push(self.type_pattern(subs[i], fty));
                    }
                    let elements = self.seq_elements_pattern(fields[fixed_n], &subs[fixed_n..]);
                    if self.capturing() {
                        self.capture_pat(elements, crate::tir::capture::PatForm::Elements);
                    }
                    pats.push(elements);
                    let fl = self.prog.syms(&field_syms);
                    let pl = self.prog.pat_lists.push_slice(&pats);
                    let tuple = self.prog.add_pat(TPat::Class(tc, value, fl, pl));
                    let some_ty = self.types.class(some, &[value]);
                    let sfl = self.prog.syms(&[field]);
                    let spl = self.prog.pat_lists.push_slice(&[tuple]);
                    return self.prog.add_pat(TPat::Class(some, some_ty, sfl, spl));
                }
            }
        }
        let seq = self.seq_elements_pattern(value, subs);
        if self.capturing() {
            self.capture_pat(seq, crate::tir::capture::PatForm::Elements);
        }
        let some_ty = self.types.class(some, &[value]);
        let fl = self.prog.syms(&[field]);
        let pl = self.prog.pat_lists.push_slice(&[seq]);
        self.prog.add_pat(TPat::Class(some, some_ty, fl, pl))
    }

    /// The patterns against the elements of a sequence of type `value`, `rest*` included.
    pub(super) fn seq_elements_pattern(&mut self, value: TypeId, subs: &[crate::ast::PatId]) -> TPatId {
        let value = self.dealias(value);
        let seq_base = self.seq_class().and_then(|seq| self.base_type(value, seq));
        let elem = match seq_base.map(|t| self.types.get(t)) {
            Some(Type::Class(_, args)) if args != EMPTY_LIST => self.types.items(args)[0],
            _ => ANY,
        };
        let ast = self.cur_ast();
        let (fixed, rest) = match subs.split_last() {
            Some((&last, fixed)) => match ast.pat(last) {
                Pat::Rest(inner) => (fixed, Some(inner)),
                _ => (subs, None),
            },
            None => (subs, None),
        };
        let mut items = Vec::with_capacity(fixed.len());
        for &s in fixed {
            items.push(self.type_pattern(s, elem));
        }
        let rest = rest.map(|inner| {
            let seq_ty = match self.seq_class() {
                Some(seq) => self.types.class(seq, &[elem]),
                None => ERROR,
            };
            self.type_pattern(inner, seq_ty)
        });
        let l = self.prog.pat_lists.push_slice(&items);
        self.prog.add_pat(TPat::Seq(l, rest))
    }

    /// A tuple or a case class, whose fields a product match takes apart.
    fn product_result(&mut self, rty: TypeId) -> Option<(ClassId, TypeId)> {
        let value = self.dealias(rty);
        match self.types.get(value) {
            Type::Class(c, _) if self.is_tuple_class(c) => Some((c, value)),
            Type::Class(c, _) if self.syms.class(c).mods & crate::ast::mods::CASE != 0 && self.syms.class(c).kind == ClassKind::Class => Some((c, value)),
            _ => None,
        }
    }

    /// `Some(value)` with the sub-patterns against the value, or against the fields of the
    /// tuple it is when there are several.
    /// An extractor's result of a tuple type past 22 elements is no product of its elements to
    /// scalac, which wants it matched as one tuple pattern (`case E((a1, ..., a23))`): the
    /// patterns are rejected, as scalac rejects them. None for any other product.
    fn xxl_result_pattern(&mut self, tc: ClassId, value: TypeId, pats: &[TPatId], span: Span) -> Option<TPatId> {
        if !self.is_tuple_class(tc) || pats.len() <= 22 {
            return None;
        }
        let msg = format!(
            "wrong number of patterns for an extractor of {}: found {}; a tuple of more than 22 elements is matched as one pattern",
            self.show(value),
            pats.len()
        );
        self.error(span, msg);
        Some(self.prog.add_pat(TPat::Wildcard))
    }

    fn option_pattern(&mut self, value: TypeId, subs: &[crate::ast::PatId], span: Span) -> TPatId {
        let some = self.std_class("Some");
        let Some(some) = some else { return self.prog.add_pat(TPat::Wildcard) };
        self.complete_class(some);
        let field = self.syms.class(some).ctor.first().and_then(|cl| cl.params.first()).map(|p| p.sym);
        let Some(field) = field else { return self.prog.add_pat(TPat::Wildcard) };
        let sub = match subs {
            [] => {
                self.error(span, "an extractor that returns an Option takes a pattern");
                self.prog.add_pat(TPat::Wildcard)
            }
            [one] => self.type_pattern(*one, value),
            many => {
                let value = self.deref(value);
                let tuple = self.product_result(value).map(|(tc, _)| tc);
                let value = self.dealias(value);
                match (tuple, self.class_field_types(value)) {
                    (Some(tc), Some(fields)) if fields.len() == many.len() => {
                        let field_syms: Vec<SymId> = self.syms.class(tc).ctor_syms.concat();
                        let mut pats = Vec::with_capacity(many.len());
                        for (&p, &fty) in many.iter().zip(&fields) {
                            pats.push(self.type_pattern(p, fty));
                        }
                        match self.xxl_result_pattern(tc, value, &pats, span) {
                            Some(p) => p,
                            None => {
                                let fl = self.prog.syms(&field_syms);
                                let pl = self.prog.pat_lists.push_slice(&pats);
                                self.prog.add_pat(TPat::Class(tc, value, fl, pl))
                            }
                        }
                    }
                    _ => {
                        let msg = format!("wrong number of patterns for an extractor of {}: found {}", self.show(value), many.len());
                        self.error(span, msg);
                        self.prog.add_pat(TPat::Wildcard)
                    }
                }
            }
        };
        let some_ty = self.types.class(some, &[value]);
        let fl = self.prog.syms(&[field]);
        let pl = self.prog.pat_lists.push_slice(&[sub]);
        self.prog.add_pat(TPat::Class(some, some_ty, fl, pl))
    }

}
