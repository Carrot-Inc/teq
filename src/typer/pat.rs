use super::resolve::{TermRef, TypeRef};
use super::Worker;
use crate::ast::{mods, Expr, ExprId, ListRef, Pat, PatId};
use crate::names;
use crate::source::Span;
use crate::symbols::*;
use crate::tir::*;
use crate::tir::capture::PatForm;
use crate::types::*;

impl<'a> Worker<'a> {
    pub fn type_match(
        &mut self,
        scrut: ExprId,
        cases: ListRef,
        span: Span,
        expected: Option<TypeId>,
    ) -> (TExprId, TypeId) {
        let ast = self.cur_ast();
        let partial = std::mem::take(&mut self.partial_match);
        let (ts, sty) = self.type_expr(scrut, None);
        // Over a parameter of the body under expansion the patterns are typed against the
        // parameter's declared type, as scalac typed them at the definition.
        let sty = match self.proxy_declared(ts) {
            Some(t) => t,
            None => self.solve_in(sty),
        };
        // A soft union scrutinee binds its cases' variables at the union, as dotc does, and
        // marks them soft for what is inferred from them.
        let soft = self.is_soft(ts).then_some(sty);
        let outer_soft = std::mem::replace(&mut self.soft_scrutinee, soft);
        let clauses = ast.case_list(cases).to_vec();
        let exp = self.branch_expected(expected);
        let mut out: Vec<TCase> = Vec::with_capacity(clauses.len());
        let mut body_types: Vec<TypeId> = Vec::with_capacity(clauses.len());
        let guide = if exp.is_none() { Some(self.branch_guide(expected)) } else { None };
        let mut caught_all = false;
        for c in &clauses {
            self.push_scope();
            let gadt_mark = self.gadt.len();
            let binders_mark = self.case_binders.len();
            let was_open = std::mem::replace(&mut self.gadt_open, true);
            let was_dead = std::mem::replace(&mut self.dead_case, caught_all);
            let pat = self.type_pattern(c.pat, sty);
            self.dead_case = was_dead;
            caught_all |= c.guard.is_none() && self.catches_all(c.pat);
            self.gadt_open = was_open;
            let guard = c.guard.map(|g| self.check_expr(g, self.b.t_boolean));
            let (body, bty) = match exp {
                Some(t) => self.type_expr_adapted(c.body, Some(t)),
                None => {
                    let (te, ty) = self.type_expr(c.body, guide);
                    let r = self.branch_to_var_bound(te, ty, expected, c.body);
                    self.guide_with(guide.unwrap(), r.1);
                    r
                }
            };
            self.gadt.truncate(gadt_mark);
            self.case_binders.truncate(binders_mark);
            self.pop_scope();
            out.push(TCase { pat, guard, body });
            body_types.push(bty);
        }
        let result_ty = match exp {
            Some(t) => self.joined_branches(t, &body_types),
            None => {
                let mut branches: Vec<(TExprId, TypeId)> =
                    out.iter().map(|c| c.body).zip(body_types.iter().copied()).collect();
                self.harmonize_literals(&mut branches);
                let mut acc = NOTHING;
                for &(_, t) in &branches {
                    let t = self.solve_bounded_in(t);
                    acc = if acc == NOTHING { t } else { self.lub(acc, t) };
                }
                self.solve_in(acc)
            }
        };
        let l = self.prog.cases.push_slice(&out);
        if !matches!(ast.expr(scrut), Expr::Unchecked(_)) {
            let sty = self.erased_named_tuple(sty);
            let broken = ast.broken_cases_in(cases);
            self.defer_match_check(sty, out, clauses, partial, span, broken);
        }
        self.soft_scrutinee = outer_soft;
        let e = self.prog.add(TExpr::Match(ts, l));
        if exp.is_none() {
            self.note_soft(e, result_ty);
        }
        (e, result_ty)
    }

    fn resolve_type_quiet(&mut self, ty: crate::ast::TyExprId) -> Option<TypeId> {
        let n = self.diags.items.len();
        let t = self.resolve_type(ty);
        self.drop_reported_since(n);
        (t != ERROR).then_some(t)
    }

    /// Resolves the class named by a constructor pattern.
    pub(super) fn pattern_class(&mut self, path: ExprId, report: bool) -> Option<ClassId> {
        let ast = self.cur_ast();
        let span = ast.expr_span(path);
        let found = match ast.expr(path) {
            Expr::SymRef(s) => match self.syms.sym(s).kind {
                SymKind::Object(o) => self.syms.class(o).companion,
                _ => None,
            },
            Expr::Ident(name) => match self.lookup_type(name) {
                Some(TypeRef::Class(c)) => Some(c),
                Some(TypeRef::Alias(a)) if self.loaded.is_some() => self.aliased_class(a),
                _ => match self.lookup_term(name) {
                    Some(TermRef::Class(c)) => Some(c),
                    Some(TermRef::Global(s) | TermRef::ModuleMember(_, s) | TermRef::This(_, s)) => {
                        match self.syms.sym(s).kind {
                            SymKind::Object(o) => self.syms.class(o).companion,
                            // `val :: : ::.type` of a package object stands for the object.
                            SymKind::Val if self.in_jar(self.syms.sym(s).file) => {
                                let ty = self.sig_of(s).ret;
                                match self.types.get(ty) {
                                    Type::Class(o, _) if self.syms.class(o).kind == ClassKind::Object => self.syms.class(o).companion,
                                    _ => None,
                                }
                            }
                            _ => None,
                        }
                    }
                    _ => None,
                },
            },
            Expr::Select(q, name) => match self.static_ref(q) {
                Some(TermRef::Package(p)) => match self.pkg_type(p, name) {
                    Some(TypeRef::Class(c)) => Some(c),
                    Some(TypeRef::Alias(a)) if self.loaded.is_some() => self.aliased_class(a),
                    _ => None,
                },
                _ => {
                    // Typed for its type alone: what the typing wrote goes.
                    let mark = self.attempt();
                    let (_, qty) = self.type_expr(q, None);
                    self.retract(mark);
                    // `Outer.this.Inner`: the class nested in the enclosing class, or in a
                    // class of its self type, which its `this` type intersects.
                    let mut classes = Vec::new();
                    let mut stack = vec![self.deref(qty)];
                    while let Some(t) = stack.pop() {
                        match self.types.get(t) {
                            Type::Inter(a, b) => {
                                stack.push(b);
                                stack.push(a);
                            }
                            _ => classes.extend(self.class_of(t)),
                        }
                    }
                    classes.into_iter().find_map(|o| match self.module_type(o, name) {
                        Some(TypeRef::Class(c)) => Some(c),
                        Some(TypeRef::Alias(a)) if self.loaded.is_some() => self.aliased_class(a),
                        // A case class a base trait nests (`Enum` of `SealedHierarchies` from a
                        // trait whose self type mixes it in).
                        _ => self.nested_in_bases(o, name),
                    })
                }
            },
            _ => None,
        };
        if found.is_none() && report {
            let mut head = path;
            while let Expr::Select(q, _) = self.cur_ast().expr(head) {
                head = q;
            }
            let unbound = match self.cur_ast().expr(head) {
                Expr::Ident(n) if self.lookup_term(n).is_none() && self.lookup_type(n).is_none() => Some(n),
                _ => None,
            };
            match unbound {
                Some(n) => {
                    let msg = format!("not found: {}", self.name_str(n));
                    self.not_found_error(n, span, msg);
                }
                None => self.error(span, "this is not a case class or enum case"),
            }
        }
        found
    }

    fn nested_in_bases(&mut self, c: ClassId, name: crate::intern::Name) -> Option<ClassId> {
        self.complete_class(c);
        let bases: Vec<ClassId> = self.syms.class(c).base_types.iter().map(|&(b, _)| b).collect();
        bases.into_iter().find_map(|b| {
            self.complete_class(b);
            self.syms.class(b).nested.get(&name).copied()
        })
    }

    pub fn type_pattern(&mut self, p: PatId, sty: TypeId) -> TPatId {
        let tp = self.type_pattern_now(p, sty);
        if self.loaded.as_ref().map_or(false, |l| !l.replay_files.is_empty() && l.replay_files.contains_key(&self.env.file)) {
            let test = self.cur_ast().reader.as_deref().map_or(false, |r| r.replay.test_pats.contains_key(&p));
            if let (true, TPat::Test(t, ..)) = (test, self.prog.pats[tp.idx()]) {
                self.prog.leaf_tests.insert(t, ());
            }
        }
        if self.index.is_some() {
            if let Pat::Ctor(path, _) = self.cur_ast().pat(p) {
                self.index_pattern(path, tp);
            }
        }
        tp
    }

    fn type_pattern_now(&mut self, p: PatId, sty: TypeId) -> TPatId {
        let ast = self.cur_ast();
        let span = ast.pat_spans[p.idx()];
        match ast.pat(p) {
            Pat::Wildcard => self.prog.add_pat(TPat::Wildcard),
            Pat::Error => {
                self.error_nodes += 1;
                self.prog.add_pat(TPat::Wildcard)
            }
            Pat::Bind(name, inner) => {
                let (inner_pat, ty) = match inner {
                    Some(i) => {
                        let tp = self.type_pattern(i, sty);
                        let ty = match self.extractor_binding_type(tp, sty) {
                            Some(t) => t,
                            None => self.narrowed_type(i, sty),
                        };
                        (Some(tp), ty)
                    }
                    None => (None, sty),
                };
                let sym = self.new_local(name, SymKind::Val, ty, span);
                if self.soft_scrutinee == Some(ty) {
                    self.soft_syms.insert(sym, ());
                }
                self.bind_local(name, sym);
                if ast.given_binds.contains(&p) {
                    self.syms.sym_mut(sym).mods |= mods::GIVEN;
                    self.bind_given(sym);
                }
                self.prog.add_pat(TPat::Bind(sym, inner_pat))
            }
            Pat::Typed(inner, ty) => {
                if let Some((te, t)) = self.singleton_pattern(ty) {
                    let eq = self.prog.add_pat(TPat::Equals(te, true));
                    return match ast.pat(inner) {
                        Pat::Bind(name, None) => {
                            let sym = self.new_local(name, SymKind::Val, t, span);
                            self.bind_local(name, sym);
                            self.prog.add_pat(TPat::Bind(sym, Some(eq)))
                        }
                        _ => eq,
                    };
                }
                let unchecked = type_unchecked(ast, ty);
                let binders = self.bind_pattern_type_vars(ty);
                let reads = self.inline.tparam_reads.get();
                let t = self.resolve_type(ty);
                self.bound_type_vars_by_params(&binders, t);
                let explicit = ast.reader.as_deref().and_then(|r| r.binder_bounds.get(&p)).cloned();
                if let Some(bounds) = explicit {
                    self.install_binder_bounds(&binders, &bounds);
                }
                // The JVM's test of an array sees its component's class: the test is checkable
                // as the written element type says, before the scrutinee fills its wildcards.
                let unchecked = unchecked || (self.jvm && (self.checkable_array_test(t) || array_element_args_wild(ast, ty)));
                let t = self.infer_type_pattern_args(t, sty);
                self.bound_type_pattern_vars(&binders, t, sty);
                if ast.reader.is_some() && super::loader::declared::dump_path().is_some() {
                    self.dump_binder_bounds(&binders, span, p);
                }
                self.refine_gadt(t, sty);
                // Conforming only by constraining an open variable of the scrutinee does not
                // make the test certain, and a reference may be `null`, which no type test takes.
                let mark = self.snapshot();
                let always = self.is_sub(sty, t) && self.snapshot() == mark && !self.null_conforms(t);
                self.rollback(mark);
                // A scrutinee that equals the pattern's type only by the refinement (a captured
                // wildcard fixed to `Int` by `case l: Leaf[Int]`) keeps its own type past the
                // case, so the binding takes the pattern's, as under scalac.
                let equal = always && {
                    let back = self.is_sub(t, sty) && self.snapshot() == mark;
                    self.rollback(mark);
                    back
                };
                // A pattern over an abstract type goes through the `TypeTest` or the `ClassTag`
                // in scope, unless its type is the scrutinee's (`tryWithTypeTest`, Typer.scala
                // 1382 to 1404; the guards at 1403 to 1404).
                if !equal && self.quote.level == 0 && self.abstract_pattern_type(t) {
                    if let Some(tp) = self.tag_pattern(p, inner, t, sty, span) {
                        return tp;
                    }
                }
                let test = if always {
                    self.prog.add_test(TypeTest::Always)
                } else {
                    self.check_sensical_test(t, sty, span);
                    self.test_for(t, sty, span, unchecked)
                };
                self.mark_leaf_test(test, reads);
                if self.inline.checking > 0 {
                    self.note_leaf_test(test, t);
                }
                let narrowed = if always && !equal { sty } else { self.pattern_binding_type(sty, t) };
                let tp = self.type_pattern(inner, narrowed);
                self.last_typed_pattern = Some((p, narrowed));
                self.prog.add_pat(TPat::Test(test, t, tp))
            }
            Pat::Lit(e) => {
                let (te, lit) = self.type_expr(e, Some(sty));
                let scrut = self.deref(sty);
                let te = self.literal_as(te, scrut);
                // `1 == x` on an `Any` is cooperative equality; only a primitive scrutinee
                // settles the literal's representation.
                let strict = self.is_numeric(scrut).is_some()
                    || scrut == self.b.t_string
                    || scrut == self.b.t_boolean
                    || scrut == self.b.t_unit
                    || matches!(self.prog.expr(te), TExpr::Null);
                if self.unused.on() {
                    self.mark_can_equal(lit, sty);
                }
                self.prog.add_pat(TPat::Equals(te, strict))
            }
            Pat::StableId(path) => {
                let (te, pty) = self.type_expr(path, None);
                let unstable = match self.prog.expr(te) {
                    TExpr::Local(s) | TExpr::Static(s) | TExpr::Field(_, s) => {
                        matches!(self.syms.sym(s).kind, SymKind::Var | SymKind::Def)
                    }
                    _ => false,
                };
                if unstable {
                    let ast = self.cur_ast();
                    let name = match ast.expr(path) {
                        Expr::Ident(n) | Expr::Select(_, n) => self.name_str(n),
                        _ => String::new(),
                    };
                    self.error(span, format!("Stable identifier required, but `{}` found", name));
                }
                self.refine_gadt(pty, sty);
                if self.unused.on() {
                    self.mark_can_equal(pty, sty);
                }
                let strict = match self.prog.expr(te) {
                    TExpr::Module(c) => !self.overrides_equals(c),
                    TExpr::Static(s) => matches!(self.syms.sym(s).kind, SymKind::EnumValue(_)),
                    _ => false,
                };
                self.prog.add_pat(TPat::Equals(te, strict))
            }
            Pat::Ctor(path, subs) => {
                let subs_list = ast.pat_list(subs).to_vec();
                let class = self.pattern_class(path, false);
                // `case Array(a, b)` of a value that is no array by its type takes an array of
                // any kind, which the parameter of scala-library's `unapplySeq` cannot say.
                if class == Some(self.b.array) && self.base_type(sty, self.b.array).is_none() {
                    return self.seq_pattern(self.b.array, subs_list, sty, span);
                }
                // A class that is no case class may still have an `unapply` in its companion.
                let is_case = class.map_or(false, |c| self.syms.class(c).mods & crate::ast::mods::CASE != 0 || self.syms.class(c).kind == ClassKind::EnumCase);
                if !is_case || class.map_or(false, |c| self.companion_unapply_takes(path, c, sty)) {
                    if let Some(p) = self.extractor_pattern(path, &subs_list, sty, span) {
                        return p;
                    }
                }
                let Some(c) = class.or_else(|| self.pattern_class(path, true)) else {
                    for &s in subs_list.iter() {
                        self.type_pattern(s, ERROR);
                    }
                    return self.prog.add_pat(TPat::Wildcard);
                };
                if self.is_seq_pattern_class(c) {
                    return self.seq_pattern(c, ast.pat_list(subs).to_vec(), sty, span);
                }
                self.class_pattern(c, ast.pat_list(subs).to_vec(), sty, span)
            }
            Pat::Rest(inner) => {
                self.error(span, "a sequence wildcard can only end a sequence pattern");
                self.type_pattern(inner, ERROR)
            }
            Pat::Quote(body) => self.type_quote_pattern(body, sty, span),
            Pat::QuoteType(t) => self.type_quote_type_pattern(t, sty, span),
            Pat::Interp(kind, parts, holes) => {
                let holes = ast.pat_list(holes).to_vec();
                self.interp_pattern(kind, parts, &holes, sty, span)
            }
            Pat::Tuple(subs) => {
                let subs = ast.pat_list(subs).to_vec();
                if subs.iter().any(|&s| matches!(ast.pat(s), Pat::NamedField(..))) {
                    return self.named_tuple_pattern(&subs, sty, span);
                }
                // A named tuple is matched as the tuple it erases to.
                let sty = match self.named_tuple_parts(sty) {
                    Some((_, values)) => self.tuple_type(&values),
                    None => sty,
                };
                let c = self.tuple_class(subs.len());
                if subs.len() > 22 {
                    if let Some(p) = self.xxl_tuple_pattern(c, &subs, sty, span) {
                        return p;
                    }
                }
                self.class_pattern(c, subs, sty, span)
            }
            Pat::NamedField(_, inner) => {
                self.error(span, "a named pattern is only allowed in a tuple or constructor pattern");
                self.type_pattern(inner, ERROR)
            }
            Pat::Alt(alts) => {
                let gadt_mark = self.gadt.len();
                // What every alternative establishes holds for the case.
                let mut common: Option<Vec<(TParamId, TypeId, i8)>> = None;
                let mut items = Vec::new();
                for &a in ast.pat_list(alts).to_vec().iter() {
                    items.push(self.type_pattern(a, sty));
                    let added: Vec<_> = self.gadt.drain(gadt_mark..).collect();
                    common = Some(match common.take() {
                        None => added,
                        Some(c) => c.into_iter().filter(|e| added.contains(e)).collect(),
                    });
                }
                self.gadt.extend(common.unwrap_or_default());
                let l = self.prog.pat_lists.push_slice(&items);
                self.prog.add_pat(TPat::Alt(l))
            }
        }
    }

    /// The class an alias of a library stands for (`scala.::` for `collection.immutable.::`).
    fn aliased_class(&mut self, a: AliasId) -> Option<ClassId> {
        self.complete_alias(a);
        let rhs = self.syms.aliases[a.idx()].rhs;
        let head = match self.types.get(rhs) {
            Type::Lambda(_, body) => body,
            _ => rhs,
        };
        match self.types.get(head) {
            Type::Class(c, _) | Type::Ctor(c) => Some(c),
            _ => None,
        }
    }

    /// The field types of a case class or tuple instance.
    pub(super) fn class_field_types(&mut self, t: TypeId) -> Option<Vec<TypeId>> {
        let Type::Class(c, args) = self.types.get(t) else { return None };
        self.complete_class(c);
        let info = self.syms.class(c);
        if info.mods & mods::CASE == 0 || info.kind == ClassKind::Object {
            return None;
        }
        let params: Vec<TypeId> = info.ctor.first()?.params.iter().map(|p| p.ty).collect();
        if params.is_empty() {
            return None;
        }
        if args == EMPTY_LIST {
            return Some(params);
        }
        let subst: Subst = info.tparams.iter().copied().zip(self.types.items(args).iter().copied()).collect();
        Some(params.iter().map(|&p| self.types.subst(p, &subst)).collect())
    }

    /// A pattern that can fail against `sty` is an error in a generator written without `case`
    /// and a warning in a pattern val, as in Scala 3.8 (dotty's `Checking.checkIrrefutable`): the
    /// first part that can fail is reported with its own type and the scrutinee type it is
    /// matched against, at that part in a generator (a typed pattern at its type), at the
    /// right-hand side (`rhs`) in a val.
    pub fn check_irrefutable(&mut self, tp: TPatId, pat: PatId, sty: TypeId, rhs: Option<Span>) {
        if sty == ERROR {
            return;
        }
        let sty = self.erased_named_tuple(sty);
        let at = self.cur_ast().pat_spans[pat.idx()];
        let Some((pat_ty, part_sty, part_at)) = self.refutable_part(tp, sty, Some(pat), at) else { return };
        let mark = self.snapshot();
        let narrows = self.is_sub(pat_ty, part_sty);
        self.rollback(mark);
        let problem = if narrows { "is more specialized than" } else { "does not match" };
        let msg = format!(
            "pattern's type {} {} the right hand side expression's type {}",
            self.show(pat_ty),
            problem,
            self.show(part_sty)
        );
        match rhs {
            Some(rhs) => self.warn(rhs, msg),
            None => self.error(part_at, msg),
        }
    }

    /// The first sub-pattern that may fail: the type it asks of its scrutinee, that scrutinee's
    /// type (a component's, as dotty's recursion carries each one's), and where it is written,
    /// read off the written pattern `ast` where the typed one follows its shape (a binder, a
    /// typed pattern, whose type is its place, a tuple or a case class, alternatives), else the
    /// nearest enclosing one's place `at`.
    fn refutable_part(&mut self, p: TPatId, sty: TypeId, ast: Option<PatId>, at: Span) -> Option<(TypeId, TypeId, Span)> {
        let conforms = |t: &mut Self, target: TypeId| {
            let mark = t.snapshot();
            let ok = t.is_sub(sty, target);
            t.rollback(mark);
            ok
        };
        let written = ast.map(|a| self.cur_ast().pat(a));
        let at = ast.map_or(at, |a| self.cur_ast().pat_spans[a.idx()]);
        let pat = self.prog.pats[p.idx()];
        match pat {
            TPat::Wildcard | TPat::Bind(_, None) => None,
            TPat::Bind(_, Some(inner)) => {
                let written = match written {
                    Some(Pat::Bind(_, Some(i))) => Some(i),
                    _ => None,
                };
                self.refutable_part(inner, sty, written, at)
            }
            TPat::Test(_, t, inner) => {
                if self.types.contains_error(t) {
                    return None;
                }
                let (written_inner, test_at) = match written {
                    Some(Pat::Typed(i, ty)) => (Some(i), self.cur_ast().ty_spans[ty.idx()]),
                    _ => (None, at),
                };
                if !conforms(self, t) || matches!(self.prog.pats[inner.idx()], TPat::Seq(..)) {
                    return Some((t, sty, test_at));
                }
                self.refutable_part(inner, sty, written_inner, at)
            }
            TPat::Class(_, t, _, subs) => {
                if !conforms(self, t) {
                    return Some((t, sty, at));
                }
                let subs = self.prog.pat_lists[subs.range()].to_vec();
                let written: Vec<Option<PatId>> = match written {
                    Some(Pat::Tuple(l)) | Some(Pat::Ctor(_, l)) => {
                        let l = self.cur_ast().pat_list(l).to_vec();
                        if l.len() == subs.len() { l.into_iter().map(Some).collect() } else { vec![None; subs.len()] }
                    }
                    _ => vec![None; subs.len()],
                };
                let fields = self.class_field_types(t).unwrap_or_default();
                subs.iter().zip(fields).zip(written).find_map(|((&s, fty), w)| self.refutable_part(s, fty, w, at))
            }
            // A literal against its own singleton type cannot fail (`val E(1) = x` for an
            // `unapply` typed `Some[1]`).
            TPat::Equals(e, _) if self.matches_own_singleton(e, sty) => None,
            TPat::Equals(e, _) => Some((
                match self.prog.expr(e) {
                    TExpr::Unit => return None,
                    TExpr::Int(_) => self.b.t_int,
                    TExpr::Long(_) => self.b.t_long,
                    TExpr::Double(_) => self.b.t_double,
                    TExpr::Bool(_) => self.b.t_boolean,
                    TExpr::Char(_) => self.b.t_char,
                    TExpr::Str(_) => self.b.t_string,
                    TExpr::Module(c) => self.types.class(c, &[]),
                    TExpr::Static(s) => self.sig_of(s).ret,
                    _ => sty,
                },
                sty,
                at,
            )),
            TPat::Alt(items) => {
                let items = self.prog.pat_lists[items.range()].to_vec();
                let written: Vec<Option<PatId>> = match written {
                    Some(Pat::Alt(l)) if self.cur_ast().pat_list(l).len() == items.len() => self.cur_ast().pat_list(l).iter().map(|&a| Some(a)).collect(),
                    _ => vec![None; items.len()],
                };
                items.iter().zip(written).find_map(|(&i, w)| self.refutable_part(i, sty, w, at))
            }
            // What cannot fail may still hold a sub-pattern that can (`val P(s: String, n) = x`).
            // A tuple pattern past 22 elements fails where one of its elements' patterns does.
            TPat::Unapply(_, call, inner) if self.irrefutable_pats.contains_key(&p) && self.xxl_tuple_elements(call, inner).is_some() => {
                let items = self.xxl_tuple_elements(call, inner).unwrap_or_default();
                let tuple_ty = self.dealias(sty);
                let elems = match self.types.get(tuple_ty) {
                    Type::Class(_, args) => self.types.items(args).to_vec(),
                    _ => Vec::new(),
                };
                let written: Vec<Option<PatId>> = match written {
                    Some(Pat::Tuple(l)) if self.cur_ast().pat_list(l).len() == items.len() => self.cur_ast().pat_list(l).iter().map(|&a| Some(a)).collect(),
                    _ => vec![None; items.len()],
                };
                items.iter().zip(elems).zip(written).find_map(|((&s, ety), w)| self.refutable_part(s, ety, w, at))
            }
            TPat::Unapply(_, _, inner) if self.irrefutable_pats.contains_key(&p) => match self.prog.pats[inner.idx()] {
                TPat::Class(_, result, _, _) => self.refutable_part(inner, result, None, at),
                _ => None,
            },
            TPat::Seq(..) | TPat::Unapply(..) => Some((sty, sty, at)),
        }
    }

    fn matches_own_singleton(&mut self, e: TExprId, sty: TypeId) -> bool {
        let value = match self.prog.expr(e) {
            TExpr::Int(n) => LitVal::Int(n),
            TExpr::Long(n) => LitVal::Long(n),
            TExpr::Double(d) => LitVal::Double(d.to_bits()),
            TExpr::Char(c) => LitVal::Char(c),
            TExpr::Bool(b) => LitVal::Bool(b),
            _ => return false,
        };
        self.deref(sty) == self.types.lit(value)
    }

    /// A `Char` scrutinee against `98` and an `Int` one against `'a'` compare as numbers in
    /// Scala; the literal takes the scrutinee's representation.
    fn literal_as(&mut self, te: TExprId, scrut: TypeId) -> TExprId {
        let lit = match self.prog.expr(te) {
            TExpr::Int(n) if scrut == self.b.t_char => TExpr::Char(n as u16),
            TExpr::Char(c) if scrut == self.b.t_int => TExpr::Int(c as i32),
            TExpr::Char(c) if scrut == self.b.t_long => TExpr::Long(c as i64),
            TExpr::Char(c) if scrut == self.b.t_double || scrut == self.b.t_float => TExpr::Double(c as f64),
            TExpr::Int(n) if scrut == self.b.t_float => TExpr::Double(n as f32 as f64),
            _ => return te,
        };
        self.prog.add(lit)
    }

    /// `case Obj` calls `Obj.equals` when the object defines one; without it the comparison is
    /// by identity, as `AnyRef.equals` is.
    fn overrides_equals(&mut self, c: ClassId) -> bool {
        self.complete_class(c);
        let info = self.syms.class(c);
        info.base_types.iter().any(|&(b, _)| self.syms.class(b).members.contains_key(&names::EQUALS))
    }

    /// `case _: x.type` tests identity with `x`; the value and its type, or None for `this.type`
    /// and for paths that are no stable values, which the type test handles.
    fn singleton_pattern(&mut self, ty: crate::ast::TyExprId) -> Option<(TExprId, TypeId)> {
        let ast = self.cur_ast();
        let crate::ast::TyExpr::Singleton(path) = ast.ty(ty) else { return None };
        let name = match ast.ty(path) {
            crate::ast::TyExpr::Name(n) | crate::ast::TyExpr::Select(_, n) => n,
            _ => return None,
        };
        if name == names::THIS {
            let c = self.this_class()?;
            let this = self.this_prefix(c);
            return Some((self.prog.add(TExpr::This), this));
        }
        let r = self.resolve_type_path(path)?;
        let span = ast.ty_spans[ty.idx()];
        match self.term_callee(r, name, span)? {
            super::apply::Callee::Value(te, t) => Some((te, self.solve_in(t))),
            _ => None,
        }
    }

    /// The static type a binder gets from `x @ pattern`.
    /// What a typed pattern binds: the pattern type, or its intersection with the scrutinee's
    /// where their classes are unrelated (a trait the value may also mix in), as in scalac
    /// (`case o: Constant` on a `TypeRepr` binds a `TypeRepr & Constant`).
    /// The type variables a type pattern binds (`case m: Memoize[a]`), outside an inline match:
    /// each a fresh type parameter in scope for the rest of the case, which the enclosing case
    /// truncates from `case_binders`.
    fn bind_pattern_type_vars(&mut self, ty: crate::ast::TyExprId) -> Vec<(TParamId, TypeId)> {
        let mut names = Vec::new();
        self.collect_type_vars(ty, &mut names);
        names.retain(|&n| self.unbound_type_var_name(n));
        let mut binders = Vec::with_capacity(names.len());
        for n in names {
            let p = self.syms.new_tparam(n, 0);
            self.note_pattern_tparam(p);
            let t = self.types.param(p);
            self.case_binders.push((n, t));
            // The case's own scope, where the body names the variable as a type.
            if let Some(super::Frame::Locals { tparams, .. }) = self.env.frames.last_mut() {
                tparams.push((n, p));
            }
            binders.push((p, t));
        }
        binders
    }

    /// The bounds a type pattern's variable takes from the type parameter it is an argument of,
    /// as scalac types a pattern's type arguments against their parameters' bounds
    /// (`Typer.typedAppliedTypeTree`): the `t` of `case _: (h *: t)` is a `Tuple`, since `*:`
    /// is `*:[+H, +T <: Tuple]`. A bound over the class's other parameters reads them as
    /// wildcards.
    fn bound_type_vars_by_params(&mut self, binders: &[(TParamId, TypeId)], t: TypeId) {
        if binders.is_empty() {
            return;
        }
        let mut pending = vec![t];
        while let Some(t) = pending.pop() {
            match self.types.get(t) {
                Type::Class(c, args) => {
                    let items = self.types.items(args).to_vec();
                    self.complete_class(c);
                    let params = self.syms.class(c).tparams.clone();
                    for (&a, &param) in items.iter().zip(&params) {
                        pending.push(a);
                        let Type::Param(p) = self.types.get(a) else { continue };
                        if !binders.iter().any(|&(b, _)| b == p) {
                            continue;
                        }
                        let mut upper = self.syms.tparam(param).upper;
                        if upper == ANY {
                            continue;
                        }
                        if self.mentions_tparam_of(upper, Some(&params)) {
                            let wild: Subst = params.iter().map(|&q| (q, WILD)).collect();
                            upper = self.types.subst(upper, &wild);
                        }
                        let current = self.syms.tparam(p).upper;
                        self.syms.tparams[p.idx()].upper = if current == ANY { upper } else { self.types.inter(current, upper) };
                    }
                }
                Type::Union(a, b) | Type::Inter(a, b) => pending.extend([a, b]),
                _ => {}
            }
        }
    }

    /// The bounds the scrutinee gives a type pattern's variables: `Memoize[a]` matched against
    /// an `Eval[A1]` makes `a <: A1` (`Eval` is covariant), an invariant position fixes the
    /// variable to the argument, as scalac's GADT reasoning does.
    fn bound_type_pattern_vars(&mut self, binders: &[(TParamId, TypeId)], t: TypeId, sty: TypeId) {
        if binders.is_empty() || self.types.contains_error(t) || self.types.contains_error(sty) {
            return;
        }
        let sty = if self.types.is_path(sty) { self.widen_path(sty) } else { sty };
        let subst: Subst = binders.iter().map(|&(p, _)| (p, self.fresh_var())).collect();
        let with_vars = self.types.subst(t, &subst);
        let mark = self.snapshot();
        let conforms = self.is_sub(with_vars, sty);
        let mut found: Vec<(TParamId, Option<TypeId>, Vec<TypeId>, Vec<TypeId>)> = Vec::new();
        if conforms {
            for (i, &(p, _)) in binders.iter().enumerate() {
                let Type::Var(v) = self.types.get(subst[i].1) else { continue };
                let inst = self.tvars[v].inst.map(|t| self.zonk(t)).filter(|&t| !self.types.has_vars(t) && t != ERROR);
                // `Any` below a variable fixes it, as `Nothing` above it does; the opposite
                // bounds say nothing.
                let bounds = |t: &mut Self, list: Vec<TypeId>, trivial: TypeId| -> Vec<TypeId> {
                    let zonked: Vec<TypeId> = list.into_iter().map(|b| t.zonk(b)).collect();
                    zonked.into_iter().filter(|&b| !t.types.has_vars(b) && b != ERROR && b != trivial).collect()
                };
                let (lower, upper) = (self.tvars[v].lower.clone(), self.tvars[v].upper.clone());
                let (lower, upper) = (bounds(self, lower, NOTHING), bounds(self, upper, ANY));
                found.push((p, inst, lower, upper));
            }
        }
        self.rollback(mark);
        for (p, inst, lower, upper) in found {
            // A lower bound the pattern states (a library body's `Box[? >: String]`) stays: the
            // scrutinee's joins it, and a scrutinee that fixes the variable below
            // it leaves the pattern's bounds, as scalac types the case.
            let stated_lower = self.syms.tparams[p.idx()].lower;
            if let Some(known) = inst {
                if stated_lower != NOTHING {
                    let mark = self.snapshot();
                    let above = self.is_sub(stated_lower, known);
                    self.rollback(mark);
                    if !above {
                        continue;
                    }
                }
                self.syms.tparams[p.idx()].upper = known;
                self.syms.tparams[p.idx()].lower = known;
                self.gadt.push((p, known, 0));
                continue;
            }
            if let Some(&first) = upper.first() {
                let mut u = first;
                for &b in &upper[1..] {
                    u = self.types.inter(u, b);
                }
                let declared = self.syms.tparams[p.idx()].upper;
                if declared != ANY && !self.is_sub(u, declared) {
                    u = self.types.inter(u, declared);
                }
                self.syms.tparams[p.idx()].upper = u;
                self.gadt.push((p, u, -1));
            }
            if let Some(&first) = lower.first() {
                let mut l = first;
                for &b in &lower[1..] {
                    l = self.lub(l, b);
                }
                if stated_lower != NOTHING {
                    l = self.lub(stated_lower, l);
                }
                self.syms.tparams[p.idx()].lower = l;
                self.gadt.push((p, l, 1));
            }
        }
    }

    /// The wildcard arguments of a type pattern that the scrutinee fixes: `case r: Const[_]`
    /// on an `A => B` is a `Const[B]` when `Const[X] extends (Any => X)`, as scalac infers
    /// them. An argument the scrutinee says nothing about stays a wildcard. A library body's
    /// pattern binds its wildcards as type variables of the case.
    fn infer_type_pattern_args(&mut self, t: TypeId, sty: TypeId) -> TypeId {
        let Type::Class(c, args) = self.types.get(t) else { return t };
        let items: Vec<TypeId> = self.types.items(args).to_vec();
        if !items.contains(&WILD) || self.types.contains_error(sty) {
            return t;
        }
        let (solved, conforms, fixed_below) = self.solve_pattern_class_bounded(c, items.len(), sty);
        let fixed: Vec<TypeId> = items
            .iter()
            .zip(&solved)
            .enumerate()
            .map(|(i, (&a, &s))| if a == WILD && conforms && (s != ANY || fixed_below[i]) && !self.types.has_vars(s) { s } else { a })
            .collect();
        self.types.class(c, &fixed)
    }

    fn pattern_binding_type(&mut self, sty: TypeId, t: TypeId) -> TypeId {
        if self.types.contains_error(t) || self.types.contains_error(sty) {
            return t;
        }
        // A scrutinee of an abstract type (`b: U` met by `case f: SttpFile`) binds `SttpFile &
        // U`, as scalac types the binder `body.tpe & pt` where the pattern's type does not
        // conform to it.
        let scrut = self.deref(sty);
        if matches!(self.types.get(scrut), Type::Param(_) | Type::Member(..) | Type::AppMember(..) | Type::Decl(_)) {
            let mark = self.snapshot();
            let conforms = self.is_sub(t, sty) && self.snapshot() == mark;
            self.rollback(mark);
            return if conforms { t } else { self.types.mk(Type::Inter(t, sty)) };
        }
        // Over a union each part binds its own: itself where it conforms to the pattern's
        // type, the pattern's type and it otherwise (`B & A | B2` for `case b: B` over an
        // `A | B2`), as scalac's `&` distributes over a union.
        if let Type::Union(..) = self.types.get(scrut) {
            let mut parts = Vec::new();
            self.union_parts(scrut, &mut parts, 16);
            let mut bound: Vec<TypeId> = Vec::new();
            for p in parts {
                let mark = self.snapshot();
                let conforms = self.is_sub(p, t) && self.snapshot() == mark;
                self.rollback(mark);
                // A part no value of the pattern's type can be (`String` for `case i: Int`)
                // binds nothing, as scalac's `&` of two disjoint classes is empty.
                let part = match conforms {
                    true => p,
                    false if self.disjoint_classes_of(p, t) => continue,
                    false => self.pattern_binding_type(p, t),
                };
                bound.push(part);
            }
            // A part below another is absorbed by it (`Shape & Greeter | Shape` is `Shape`), as
            // scalac's union simplifies.
            let mut kept: Vec<TypeId> = Vec::new();
            for (i, &p) in bound.iter().enumerate() {
                let absorbed = bound.iter().enumerate().any(|(j, &q)| {
                    if i == j || (q == p && j > i) {
                        return false;
                    }
                    let mark = self.snapshot();
                    let below = self.is_sub(p, q) && self.snapshot() == mark;
                    self.rollback(mark);
                    below && (q != p)
                });
                if !absorbed && !kept.contains(&p) {
                    kept.push(p);
                }
            }
            return kept.into_iter().reduce(|a, b| self.types.mk(Type::Union(a, b))).unwrap_or(t);
        }
        let (Some(pc), Some(sc)) = (self.class_of(t), self.class_of(sty)) else { return t };
        self.complete_class(pc);
        self.complete_class(sc);
        let derives = |s: &Self, a: ClassId, b: ClassId| a == b || s.syms.class(a).base_types.iter().any(|&(k, _)| k == b);
        if derives(self, pc, sc) || derives(self, sc, pc) {
            t
        } else {
            // The pattern's type first, as scalac's binder prints it (`B & A` for `case b: B`
            // over an `A`).
            self.types.mk(Type::Inter(t, sty))
        }
    }

    /// The members of a union, its nested unions flattened, `depth` levels down.
    fn union_parts(&mut self, t: TypeId, out: &mut Vec<TypeId>, depth: u32) {
        let t = self.deref(t);
        match self.types.get(t) {
            Type::Union(a, b) if depth > 0 => {
                self.union_parts(a, out, depth - 1);
                self.union_parts(b, out, depth - 1);
            }
            _ => out.push(t),
        }
    }

    /// Whether no value is of both types: two classes neither of which derives from the other,
    /// one final or neither a trait.
    fn disjoint_classes_of(&mut self, a: TypeId, b: TypeId) -> bool {
        let (Some(ca), Some(cb)) = (self.class_of(a), self.class_of(b)) else { return false };
        if self.derives_class(ca, cb) || self.derives_class(cb, ca) {
            return false;
        }
        let final_class = |w: &Self, c: ClassId| {
            let info = w.syms.class(c);
            let b = &w.b;
            matches!(info.kind, ClassKind::Object | ClassKind::EnumCase | ClassKind::Builtin) && c != b.any_ref && c != b.any_val || info.mods & crate::ast::mods::FINAL != 0
        };
        let trait_ = |w: &Self, c: ClassId| w.syms.class(c).kind == ClassKind::Trait;
        match (trait_(self, ca), trait_(self, cb)) {
            (false, false) => true,
            (true, true) => false,
            // A final class and a trait it does not extend, where its ancestors are known in full
            // (the lean std's `Integer` is `Serializable` though it does not say so).
            (true, false) => final_class(self, cb) && self.complete_ancestry(cb),
            (false, true) => final_class(self, ca) && self.complete_ancestry(ca),
        }
    }

    fn narrowed_type(&mut self, p: PatId, sty: TypeId) -> TypeId {
        let ast = self.cur_ast();
        match ast.pat(p) {
            Pat::Typed(..) if self.last_typed_pattern.map_or(false, |(q, _)| q == p) => self.last_typed_pattern.unwrap().1,
            Pat::Typed(_, ty) => match self.resolve_type_quiet(ty) {
                Some(t) => self.pattern_binding_type(sty, t),
                None => sty,
            },
            Pat::Ctor(path, _) => match self.pattern_class(path, false) {
                Some(c) => self.instantiate_pattern_class(c, sty).0,
                None => sty,
            },
            _ => sty,
        }
    }

    /// What `x @ Extractor(..)` binds, as scalac's `typedBind` types it: the scrutinee's type
    /// where it conforms to what `unapply` takes, and otherwise the intersection of the two,
    /// the type taken where that is the narrower (`Apply` for an `unapply(t: Apply)` over a
    /// `Tree`, `Apply & T` over a type parameter `T`, `B & A` for unrelated traits). None for a
    /// pattern that is no extractor.
    fn extractor_binding_type(&mut self, tp: TPatId, sty: TypeId) -> Option<TypeId> {
        match self.prog.pats[tp.idx()] {
            TPat::Unapply(..) => Some(self.tag_pats.get(&tp).copied().unwrap_or(sty)),
            TPat::Bind(_, Some(inner)) => self.extractor_binding_type(inner, sty),
            // A tag pattern behind its null test binds the type pattern's type.
            TPat::Test(_, _, inner) if self.tag_pats.contains_key(&inner) => self.tag_pats.get(&inner).copied(),
            TPat::Test(_, taken, inner) if matches!(self.prog.pats[inner.idx()], TPat::Unapply(..)) => {
                if self.types.contains_error(taken) || self.types.contains_error(sty) {
                    return Some(taken);
                }
                let mark = self.snapshot();
                let narrower = self.is_sub(taken, sty) && self.snapshot() == mark;
                self.rollback(mark);
                Some(if narrower { taken } else { self.types.mk(Type::Inter(taken, sty)) })
            }
            _ => None,
        }
    }

    pub fn union_alternatives(&mut self, t: TypeId, out: &mut Vec<TypeId>) {
        let t = self.deref(t);
        if let Type::Union(a, b) = self.types.get(t) {
            self.union_alternatives(a, out);
            self.union_alternatives(b, out);
        } else {
            out.push(t);
        }
    }

    /// The type arguments with which `c` conforms to `sty`, and whether it conforms at all.
    pub(crate) fn solve_pattern_class(&mut self, c: ClassId, arity: usize, sty: TypeId) -> (Vec<TypeId>, bool) {
        let (args, conforms, _) = self.solve_pattern_class_bounded(c, arity, sty);
        (args, conforms)
    }

    /// As `solve_pattern_class`, with which arguments a lower bound of `Any` fixed: a
    /// contravariant position of the scrutinee's `Any` (`Read[?, ..]` against a `Chan[Any, ..]`).
    fn solve_pattern_class_bounded(&mut self, c: ClassId, arity: usize, sty: TypeId) -> (Vec<TypeId>, bool, Vec<bool>) {
        // A scrutinee of a singleton type (`x$1.type`, a match on a field) is tested at its
        // widened type.
        let sty = if self.types.is_path(sty) { self.widen_path(sty) } else { sty };
        let first_var = self.tvars.len();
        let vars: Vec<TypeId> = (0..arity).map(|_| self.fresh_var()).collect();
        let class_ty = self.types.class(c, &vars);
        let mark = self.snapshot();
        let conforms = self.is_sub(class_ty, sty);
        if !conforms {
            self.rollback(mark);
        }
        let fixed_below: Vec<bool> = vars
            .iter()
            .map(|&v| match self.types.get(v) {
                Type::Var(v) => {
                    let lower = self.tvars[v].lower.clone();
                    lower.into_iter().any(|l| self.zonk(l) == ANY)
                }
                _ => false,
            })
            .collect();
        // Against a scrutinee that leaves its arguments to the enclosing method's parameters
        // (`Tree[X]`), a parameter of the class that nothing fixes is a fresh abstract type
        // (scalac's `C$1`), which the case's GADT constraint names: `X` is `List[C$1]` in
        // `case Node(x)` for a `Node[C] extends Tree[List[C]]`, and no `List[String]`.
        let fresh_abstract = !conforms && self.gadt_open && self.leaves_param(sty);
        let mut fresh: Vec<(TParamId, TParamId)> = Vec::new();
        for v in first_var..self.tvars.len() {
            let info = &self.tvars[v];
            let unconstrained = info.inst.is_none() && info.lower.is_empty() && info.upper.is_empty();
            let own_index = (v - first_var < arity).then_some(v - first_var);
            if let (true, true, Some(i)) = (unconstrained, fresh_abstract, own_index) {
                self.settle_class(c);
                let own = self.syms.class(c).tparams[i];
                let name = self.syms.tparam(own).name;
                let name = self.interner.intern(&format!("{}$1", self.name_str(name)));
                let p = self.syms.new_tparam(name, 0);
                fresh.push((p, own));
                self.tvars[v].inst = Some(self.types.param(p));
            } else if unconstrained {
                self.tvars[v].inst = Some(ANY);
            } else if info.inst.is_none() && info.lower.is_empty() {
                // Only an upper bound is known: the field can be anything below it.
                self.solve_var(self.tvars.id(v));
            } else {
                self.solve_var(self.tvars.id(v));
            }
        }
        let args: Vec<TypeId> = vars.iter().map(|&v| self.zonk(v)).collect();
        // The fresh types keep the declared bounds, over the pattern's own arguments
        // (`B$1 <: A$1`, `A$1 >: String` for `Node[A >: String, B <: A]`).
        if !fresh.is_empty() {
            self.settle_class(c);
            let subst: Subst = self.syms.class(c).tparams.iter().copied().zip(args.iter().copied()).collect();
            for (p, own) in fresh {
                let (lo, hi) = (self.syms.tparam(own).lower, self.syms.tparam(own).upper);
                let (lo, hi) = (self.types.subst(lo, &subst), self.types.subst(hi, &subst));
                self.syms.tparams[p.idx()].lower = lo;
                self.syms.tparams[p.idx()].upper = hi;
            }
        }
        (args, conforms, fixed_below)
    }

    /// Infers the type arguments of a pattern's class from the scrutinee type.
    pub(super) fn instantiate_pattern_class(&mut self, c: ClassId, sty: TypeId) -> (TypeId, Subst) {
        self.complete_class(c);
        let tparams = self.syms.class(c).tparams.clone();
        // `(String, Boolean) | (Tw, Boolean)` matched by a tuple pattern, or `Option[Int] |
        // Option[String]` by `Some(_)`: a field can come from any alternative the class conforms
        // to, so its sub-pattern is typed against the union of what they give.
        let mut joined: Option<Vec<TypeId>> = None;
        let scrut = self.deref(sty);
        if !tparams.is_empty() && matches!(self.types.get(scrut), Type::Union(..)) {
            let mut alternatives = Vec::new();
            self.union_alternatives(scrut, &mut alternatives);
            for alt in alternatives {
                let (args, conforms) = self.solve_pattern_class(c, tparams.len(), alt);
                if !conforms {
                    continue;
                }
                joined = Some(match joined {
                    Some(j) => j.iter().zip(&args).map(|(&a, &b)| self.types.union(a, b)).collect(),
                    None => args,
                });
            }
        }
        let args = match joined {
            Some(args) => args,
            None => self.solve_pattern_class(c, tparams.len(), sty).0,
        };
        let subst: Subst = tparams.iter().copied().zip(args.iter().copied()).collect();
        (self.types.class(c, &args), subst)
    }

    /// A pattern that fixes what the scrutinee type leaves to a type parameter tells what that
    /// parameter is inside the case: `case Key.UserId` with the type `Key[Int]` against a
    /// `Key[T]` makes `T` and `Int` interchangeable there.
    /// Whether the scrutinee type leaves an argument to a type parameter (`Tree[X]`).
    fn leaves_param(&mut self, sty: TypeId) -> bool {
        let scrut = self.deref(sty);
        let Type::Class(_, sargs) = self.types.get(scrut) else { return false };
        (0..self.types.items(sargs).len()).any(|i| {
            let s = self.deref(self.types.items(sargs)[i]);
            matches!(self.types.get(s), Type::Param(_))
        })
    }

    fn refine_gadt(&mut self, pat_ty: TypeId, sty: TypeId) {
        let scrut = self.deref(sty);
        let Type::Class(sc, sargs) = self.types.get(scrut) else { return };
        if sargs == EMPTY_LIST || !self.gadt_open || !self.leaves_param(sty) {
            return;
        }
        let Some(base) = self.base_type(pat_ty, sc) else { return };
        let Type::Class(_, pargs) = self.types.get(base) else { return };
        for i in 0..self.types.items(sargs).len().min(self.types.items(pargs).len()) {
            let s = self.deref(self.types.items(sargs)[i]);
            let Type::Param(p) = self.types.get(s) else { continue };
            let known = self.zonk(self.types.items(pargs)[i]);
            self.settle_class(sc);
            let variance = self.syms.tparam(self.syms.class(sc).tparams[i]).variance;
            let says_nothing = (variance == 1 && known == NOTHING) || (variance == -1 && known == ANY);
            if known == s || known == ERROR || says_nothing || self.types.has_vars(known) {
                continue;
            }
            self.gadt.push((p, known, variance));
        }
    }

    /// `List`, `Seq`, `Vector`, `Array` and the other sequence classes without a constructor
    /// pattern of their own; `::` keeps its case-class pattern.
    pub(crate) fn is_seq_pattern_class(&mut self, c: ClassId) -> bool {
        if c == self.b.array {
            return true;
        }
        // Completing the class enters `Seq`'s file when the class extends it.
        self.complete_class(c);
        let Some(seq) = self.seq_class() else { return false };
        // scala-library's `collection.Seq` (`case collection.Seq(a, b)` of http4s' `Rfc3986`)
        // is a base of the `Seq` the std binds, of the same name.
        self.complete_class(seq);
        let seq_name = self.syms.class(seq).name;
        let general = self.syms.class(seq).base_types.iter().map(|&(b, _)| b).find(|&b| b != seq && self.syms.class(b).name == seq_name);
        let info = self.syms.class(c);
        info.mods & mods::CASE == 0 && info.base_types.iter().any(|&(b, _)| b == seq || Some(b) == general)
    }

    /// Whether the companion of the case class `c`, which `path` names, declares an `unapply`
    /// of its own beside the case class's, and the scrutinee is no `c`: overload resolution on
    /// the scrutinee picks it (`case Constructor(params, ctor)` on a `Type[A]`).
    fn companion_unapply_takes(&mut self, path: ExprId, c: ClassId, sty: TypeId) -> bool {
        let scrut = self.deref(sty);
        let unrelated = match self.class_of(scrut) {
            Some(k) => self.base_type(scrut, c).is_none() && !self.derives_from(c, k),
            None => matches!(self.types.get(scrut), Type::Member(..) | Type::AppMember(..)),
        };
        if !unrelated {
            return false;
        }
        // Typed for its type alone: what the typing wrote goes.
        let mark = self.attempt();
        let (_, companion) = self.type_expr(path, None);
        self.retract(mark);
        if companion == ERROR {
            return false;
        }
        let Some((unapply, _)) = self.find_member(companion, names::UNAPPLY) else { return false };
        let alts = self.syms.alternatives(unapply).map_or_else(|| vec![unapply], |a| a.to_vec());
        alts.iter().any(|&a| {
            let first = self.sig_of(a).clauses.first().and_then(|cl| cl.params.first()).map(|p| p.ty);
            first.map_or(false, |t| self.class_of(t) != Some(c))
        })
    }

    /// Reports a pattern whose class is unrelated to the scrutinee type.
    fn check_related(&mut self, class_ty: TypeId, sty: TypeId, span: Span) {
        if self.inline.lenient_match > 0 {
            return;
        }
        let mark = self.snapshot();
        let related = self.is_sub(class_ty, sty) || {
            self.rollback(mark);
            self.is_sub(sty, class_ty)
        };
        self.rollback(mark);
        if related {
            return;
        }
        let scrut = self.deref(sty);
        let open_scrutinee = matches!(
            self.types.get(scrut),
            Type::Any | Type::Param(_) | Type::Union(..) | Type::Inter(..) | Type::Error
        ) || self.class_of(scrut).map_or(false, |sc| self.syms.class(sc).kind == ClassKind::Trait);
        if !open_scrutinee {
            let msg = format!(
                "this pattern of type {} can never match a value of type {}",
                self.show(class_ty),
                self.show(sty)
            );
            self.error(span, msg);
        }
    }

    /// `List(a, b)`, `Seq(x, rest*)`, `Array(k, v)`: the class test, then a length check and the
    /// elements, which the runtime's `$seqPat` reads from the sequence's iterator.
    fn seq_pattern(&mut self, c: ClassId, subs: Vec<PatId>, sty: TypeId, span: Span) -> TPatId {
        let _ = self.cur_ast();
        let scrut = self.deref(sty);
        let class_ty = match self.base_type(scrut, c) {
            Some(t) => t,
            None => self.instantiate_pattern_class(c, sty).0,
        };
        let elem = match self.types.get(class_ty) {
            Type::Class(_, args) if args != EMPTY_LIST => self.types.items(args)[0],
            _ => ANY,
        };
        self.check_related(class_ty, sty, span);
        let mark = self.snapshot();
        let always = self.is_sub(sty, class_ty);
        self.rollback(mark);
        let test = if always { self.prog.add_test(TypeTest::Always) } else { self.test_for(class_ty, sty, span, true) };
        let seq_ty = match self.seq_class() {
            Some(seq) => self.types.class(seq, &[elem]),
            None => ERROR,
        };
        let seq = self.seq_elements_pattern(seq_ty, &subs);
        if self.capturing() {
            self.capture_pat(seq, PatForm::Seq(class_ty));
        }
        self.prog.add_pat(TPat::Test(test, class_ty, seq))
    }

    /// `case (a1, ..., an)` past 22 elements, on scalac's TupleXXL: the scrutinee tested as one,
    /// `TupleXXL.unapplySeq` and the elements matched as a sequence of exactly `n`, each against
    /// its element's type. Over a tuple type of that arity the pattern is irrefutable where the
    /// sub-patterns are, as a tuple pattern is.
    fn xxl_tuple_pattern(&mut self, c: ClassId, subs: &[PatId], sty: TypeId, span: Span) -> Option<TPatId> {
        self.tuple_xxl_module()?;
        let (class_ty, _) = self.instantiate_pattern_class(c, sty);
        self.check_related(class_ty, sty, span);
        let elems: Vec<TypeId> = match self.types.get(class_ty) {
            Type::Class(_, args) if self.types.items(args).len() == subs.len() => self.types.items(args).to_vec(),
            _ => vec![ANY; subs.len()],
        };
        let mut items = Vec::with_capacity(subs.len());
        for (&p, &ety) in subs.iter().zip(&elems) {
            items.push(self.type_pattern(p, ety));
        }
        self.xxl_elements_pattern(class_ty, &items, sty, span)
    }

    /// The pattern of `xxl_tuple_pattern` over element patterns already typed, for the tuple
    /// type `class_ty`; a named tuple's comes here too.
    pub(super) fn xxl_elements_pattern(&mut self, class_ty: TypeId, items: &[TPatId], sty: TypeId, span: Span) -> Option<TPatId> {
        let module = self.tuple_xxl_module()?;
        let xxl = self.syms.class(module).companion?;
        let xxl_ty = self.types.class(xxl, &[]);
        let test = self.test_for(xxl_ty, sty, span, true);
        let scrut = self.fresh_local("u", xxl_ty, span);
        let arg = self.prog.add(TExpr::Local(scrut));
        self.prog.set_type(arg, xxl_ty);
        let recv = self.prog.add(TExpr::Module(module));
        let recv_ty = self.types.class(module, &[]);
        let lists = vec![super::apply::ArgList { args: vec![super::apply::ArgSrc::Typed(arg, xxl_ty)], using: false, span }];
        let (call, _) = self.apply_member(recv, recv_ty, names::UNAPPLY_SEQ, None, lists, span, None);
        let some = self.std_class("Some")?;
        self.complete_class(some);
        let field = self.syms.class(some).ctor.first().and_then(|cl| cl.params.first()).map(|p| p.sym)?;
        let l = self.prog.pat_lists.push_slice(items);
        let seq = self.prog.add_pat(TPat::Seq(l, None));
        if self.capturing() {
            self.capture_pat(seq, PatForm::Elements);
        }
        let seq_ty = match self.seq_class() {
            Some(sc) => self.types.class(sc, &[ANY]),
            None => ERROR,
        };
        let some_ty = self.types.class(some, &[seq_ty]);
        let fl = self.prog.syms(&[field]);
        let pl = self.prog.pat_lists.push_slice(&[seq]);
        let inner = self.prog.add_pat(TPat::Class(some, some_ty, fl, pl));
        let pat = self.prog.add_pat(TPat::Unapply(scrut, call, inner));
        let mark = self.snapshot();
        let total = self.is_sub(sty, class_ty);
        self.rollback(mark);
        if total {
            self.irrefutable_pats.insert(pat, ());
        }
        Some(self.prog.add_pat(TPat::Test(test, class_ty, pat)))
    }

    /// The element patterns of `xxl_tuple_pattern`'s extraction: an `unapplySeq` of TupleXXL's
    /// companion whose `Some` holds a sequence of exactly the elements.
    fn xxl_tuple_elements(&mut self, call: TExprId, inner: TPatId) -> Option<Vec<TPatId>> {
        let TExpr::CallMethod(recv, _, _) = self.prog.expr(call) else { return None };
        let TExpr::Module(m) = self.prog.expr(recv) else { return None };
        // Named, not demanded: a pattern that is no tuple's must not enter TupleXXL's std file.
        let info = self.syms.class(m);
        let runtime = matches!(info.owner, Owner::Package(p) if self.syms.pkg(p).parent == Some(self.b.scala_pkg) && self.name_str(self.syms.pkg(p).name) == "runtime");
        if !runtime || self.name_str(info.name) != "TupleXXL" {
            return None;
        }
        let TPat::Class(_, _, _, subs) = self.prog.pats[inner.idx()] else { return None };
        let &[seq] = &self.prog.pat_lists[subs.range()] else { return None };
        match self.prog.pats[seq.idx()] {
            TPat::Seq(items, None) => Some(self.prog.pat_lists[items.range()].to_vec()),
            _ => None,
        }
    }

    fn class_pattern(&mut self, c: ClassId, subs: Vec<PatId>, sty: TypeId, span: Span) -> TPatId {
        let (class_ty, subst) = self.instantiate_pattern_class(c, sty);
        self.refine_gadt(class_ty, sty);
        let info = self.syms.class(c);
        if info.mods & mods::CASE == 0 {
            let msg = format!("{} is not a case class, so it cannot be used as a pattern", self.name_str(info.name));
            self.error(span, msg);
        }
        self.check_related(class_ty, sty, span);
        let mut fields: Vec<ParamSig> = self.syms.class(c).ctor.first().map(|cl| cl.params.clone()).unwrap_or_default();
        // A class nested in a class names the enclosing instance's members in its fields
        // (`FromExpr[A](expr: ExprPromises.this.Expr[A])`), seen where the match stands.
        if self.class_in_class(c) && fields.iter().any(|f| self.types.has_paths(f.ty)) {
            for f in fields.iter_mut() {
                f.ty = self.seen_from_enclosing_this(f.ty);
            }
        }
        if subs.iter().any(|&s| matches!(self.cur_ast().pat(s), Pat::NamedField(..))) {
            return self.named_class_pattern(c, class_ty, &subst, &fields, &subs, span);
        }
        // `C(a, xs*)` and `C(a, x, y)` for a class whose last parameter is repeated: the
        // patterns past the fixed ones are a sequence pattern over that parameter's sequence.
        if let Some((last, fixed_fields)) = fields.split_last().filter(|(l, f)| l.repeated && subs.len() >= f.len()) {
            let mut field_syms: Vec<SymId> = fixed_fields.iter().map(|f| f.sym).collect();
            let mut sub_pats = Vec::with_capacity(fields.len());
            for (f, &s) in fixed_fields.iter().zip(&subs) {
                let fty = self.types.subst(f.ty, &subst);
                sub_pats.push(self.type_pattern(s, fty));
            }
            let elem = self.types.subst(last.ty, &subst);
            let seq_ty = match self.seq_class() {
                Some(seq) => self.types.class(seq, &[elem]),
                None => ERROR,
            };
            field_syms.push(last.sym);
            let elements = self.seq_elements_pattern(seq_ty, &subs[fixed_fields.len()..]);
            if self.capturing() {
                self.capture_pat(elements, PatForm::Elements);
            }
            sub_pats.push(elements);
            let fl = self.prog.syms(&field_syms);
            let pl = self.prog.pat_lists.push_slice(&sub_pats);
            return self.prog.add_pat(TPat::Class(c, class_ty, fl, pl));
        }
        // `Right(a, b)` for a class of one field of a tuple type: the patterns match the
        // tuple's elements, as scalac's auto-tupling of the pattern has it.
        if let [field] = fields.as_slice() {
            if subs.len() > 1 {
                let fty = self.types.subst(field.ty, &subst);
                let fty = self.deref(fty);
                if let Type::Class(tc, targs) = self.types.get(fty) {
                    if self.is_tuple_class(tc) && self.types.items(targs).len() == subs.len() {
                        let elem_tys = self.types.items(targs).to_vec();
                        let tuple_fields: Vec<SymId> = self.syms.class(tc).ctor_syms.concat();
                        let mut elem_pats = Vec::with_capacity(subs.len());
                        for (&s, &ety) in subs.iter().zip(&elem_tys) {
                            elem_pats.push(self.type_pattern(s, ety));
                        }
                        let xxl = if elem_tys.len() > 22 { self.xxl_elements_pattern(fty, &elem_pats, fty, span) } else { None };
                        let tuple_pat = match xxl {
                            Some(p) => p,
                            None => {
                                let tfl = self.prog.syms(&tuple_fields);
                                let tpl = self.prog.pat_lists.push_slice(&elem_pats);
                                self.prog.add_pat(TPat::Class(tc, fty, tfl, tpl))
                            }
                        };
                        let fl = self.prog.syms(&[field.sym]);
                        let pl = self.prog.pat_lists.push_slice(&[tuple_pat]);
                        return self.prog.add_pat(TPat::Class(c, class_ty, fl, pl));
                    }
                }
            }
        }
        if fields.len() != subs.len() {
            let msg = format!(
                "wrong number of patterns for {}: expected {}, found {}",
                self.name_str(self.syms.class(c).name),
                fields.len(),
                subs.len()
            );
            // The fields of a case class whose header the parser could not complete are unknown.
            if self.syms.class(c).mods & crate::ast::mods::INCOMPLETE != 0 {
                self.dependent_error(span, msg);
            } else {
                self.error(span, msg);
            }
        }
        let mut field_syms = Vec::with_capacity(subs.len());
        let mut sub_pats = Vec::with_capacity(subs.len());
        for (i, &s) in subs.iter().enumerate() {
            let fty = match fields.get(i) {
                Some(f) => {
                    field_syms.push(f.sym);
                    self.types.subst(f.ty, &subst)
                }
                None => ERROR,
            };
            sub_pats.push(self.type_pattern(s, fty));
        }
        sub_pats.truncate(field_syms.len());
        let fl = self.prog.syms(&field_syms);
        let pl = self.prog.pat_lists.push_slice(&sub_pats);
        self.prog.add_pat(TPat::Class(c, class_ty, fl, pl))
    }

    /// Whether a type pattern of type `t` is over an abstract type, which a `TypeTest` or a
    /// `ClassTag` in scope tests: dotty's `tree.tpt.tpe.dealias` a `TypeRef` whose symbol is no
    /// class, or such a reference applied (`tryWithTypeTest`, Typer.scala 1402 to 1404): a type
    /// parameter, an abstract type member, and an opaque type outside its scope, where its alias
    /// is not seen through.
    pub(super) fn abstract_pattern_type(&mut self, t: TypeId) -> bool {
        let t = self.deref_alias(t);
        match self.types.get(t) {
            Type::Param(_) | Type::AppParam(..) | Type::Member(..) | Type::AppMember(..) | Type::Decl(_) => true,
            Type::Class(c, _) => self.syms.class(c).kind == ClassKind::Opaque && !self.transparent.contains(&c),
            _ => false,
        }
    }

    /// The `TypeTest[S, T]` in scope at a type pattern over the abstract type `t` on a scrutinee
    /// of type `sty`, or else the `ClassTag[T]` (`tagged`, Typer.scala 1395 to 1397), each by a
    /// search alone (`withTag` 1383 to 1393 calls `inferImplicit`, which synthesizes nothing): the
    /// evidence and its type. A failed or ambiguous search is no tag.
    pub(super) fn pattern_tag(&mut self, t: TypeId, sty: TypeId, span: Span) -> Option<(TExprId, TypeId)> {
        let sty = self.zonk(sty);
        let s = if self.types.is_path(sty) { self.widen_path(sty) } else { sty };
        if self.types.has_vars(s) || self.types.contains_error(s) || self.types.contains_error(t) {
            return None;
        }
        if let Some(tt) = self.class_at(&["scala", "reflect", "TypeTest"]) {
            let target = self.types.class(tt, &[s, t]);
            if let Some(found) = self.resolve_given_search_only(target, span) {
                return Some(found);
            }
        }
        let ct = self.class_at(&["scala", "reflect", "ClassTag"])?;
        let target = self.types.class(ct, &[t]);
        self.resolve_given_search_only(target, span)
    }

    /// `case t: T` over an abstract `T` with a tag in scope, as `tryWithTypeTest` retypes it
    /// (Typer.scala 1382 to 1404): the extractor pattern `tag.unapply(x)`, whose `Some` holds the
    /// value the inner pattern matches. The binder is a `T`, whatever the scrutinee's type:
    /// scalac's `typedBind` moves it inside the extractor and types it as the extracted value
    /// (Typer.scala 1431 to 1437, `(y : T)` over an abstract scrutinee `S` too).
    fn tag_pattern(&mut self, p: PatId, inner: PatId, t: TypeId, sty: TypeId, span: Span) -> Option<TPatId> {
        let (tag, tag_ty) = self.pattern_tag(t, sty, span)?;
        let (scrut, call) = self.tag_call(tag, tag_ty, sty, span)?;
        let inner_tp = self.type_pattern(inner, t);
        let pat = self.tag_result_pattern(scrut, call, t, inner_tp, sty)?;
        self.last_typed_pattern = Some((p, t));
        Some(pat)
    }

    /// `tag.unapply(u)` for a scrutinee local `u` of type `sty`: the local and the call, typed
    /// as a call of the evidence's own `unapply` is, so that an overriding one is what runs.
    pub(super) fn tag_call(&mut self, tag: TExprId, tag_ty: TypeId, sty: TypeId, span: Span) -> Option<(SymId, TExprId)> {
        let scrut = self.fresh_local("u", sty, span);
        let arg = self.prog.add(TExpr::Local(scrut));
        self.prog.set_type(arg, sty);
        let lists = vec![super::apply::ArgList { args: vec![super::apply::ArgSrc::Typed(arg, sty)], using: false, span }];
        let n = self.diags.items.len();
        let (call, rty) = self.apply_member(tag, tag_ty, names::UNAPPLY, None, lists, span, None);
        if self.diags.items.len() > n || rty == ERROR {
            self.drop_reported_since(n);
            return None;
        }
        Some((scrut, call))
    }

    /// The pattern over the result of `tag_call`: a `Some[value]` holding `inner`, behind a
    /// test that the scrutinee (of type `sty`) is not `null`, unless it cannot be, as dotc's
    /// pattern matcher guards an extractor (PatternMatcher.scala 495 to 496, `NonNullTest`): the
    /// evidence's receiver is not evaluated and its `unapply` not called for a `null`. What
    /// `x @ pat` binds is `value`, the type pattern's.
    pub(super) fn tag_result_pattern(&mut self, scrut: SymId, call: TExprId, value: TypeId, inner: TPatId, sty: TypeId) -> Option<TPatId> {
        let some = self.std_class("Some")?;
        self.complete_class(some);
        let field = self.syms.class(some).ctor.first().and_then(|cl| cl.params.first()).map(|p| p.sym)?;
        let some_ty = self.types.class(some, &[value]);
        let fl = self.prog.syms(&[field]);
        let pl = self.prog.pat_lists.push_slice(&[inner]);
        let result = self.prog.add_pat(TPat::Class(some, some_ty, fl, pl));
        let pat = self.prog.add_pat(TPat::Unapply(scrut, call, result));
        self.tag_pats.insert(pat, value);
        if !self.scrutinee_may_be_null(sty) {
            return Some(pat);
        }
        let non_null = self.prog.add_test(TypeTest::AnyVal);
        Some(self.prog.add_pat(TPat::Test(non_null, sty, pat)))
    }

    /// The test of an abstract type without a tag, as scalac's erased test: TypeTestsCasts.scala
    /// 359 to 360 erases the tested type before `transformIsInstanceOf` (282 to 307), and the
    /// erasure of an abstract type is its upper bound's (TypeErasure.scala 767 to 769): a literal
    /// type's class (773 to 774, the proxy's underlying type), a union's erased lub (815 to 819),
    /// an intersection's erased glb (806 to 813, `erasedGlb` 481 to 482 by `compareErasedGlb`),
    /// an array's class; `Object` is everything but `null`. A composite the program writes as its
    /// pattern keeps its own test (`test_for`'s `Union` and `Inter`); only the bound is erased.
    fn bound_test(&mut self, t: TypeId, hi: TypeId, sty: TypeId, span: Span) -> TestId {
        let hi = self.deref_alias(hi);
        if hi == t || matches!(self.types.get(hi), Type::Any) {
            // Everything but `null`, as the test of `AnyVal` is.
            return self.prog.add_test(TypeTest::AnyVal);
        }
        match self.bound_erasure(hi, 0) {
            Some(super::site::Erased::Of(c, 0)) if c == self.b.any_ref => self.prog.add_test(TypeTest::AnyVal),
            Some(super::site::Erased::Of(c, dims)) => {
                let mut erased = self.types.class(c, &[]);
                for _ in 0..dims {
                    erased = self.types.class(self.b.array, &[erased]);
                }
                self.test_for(erased, sty, span, true)
            }
            // A bottom type, or a type the erasure cannot follow: its own test.
            _ => self.test_for(hi, sty, span, true),
        }
    }

    /// The runtime erasure of a bound, as `TypeErasure.apply` erases it (TypeErasure.scala 763
    /// to 819): an abstract type to its upper bound's erasure (767 to 769 and 773 to 779 through
    /// `checkedSuperType`, 901 to 902), so that an abstract constituent of a union or an
    /// intersection is erased before the union's erased lub (815 to 819) or the intersection's
    /// erased glb (806 to 813) is taken; `Any` and an unbounded type to `Object`; an opaque type
    /// to its underlying type's; a class, a literal, an array as `stable_erasure` has them.
    /// `stable_erasure` keeps its own answer (none for an abstract type) for the `ClassTag`
    /// synthesis, whose eligibility it decides.
    fn bound_erasure(&mut self, t: TypeId, depth: u32) -> Option<super::site::Erased> {
        use super::site::Erased;
        if depth > 32 {
            return None;
        }
        let t = self.deref_alias(t);
        let object = Erased::Of(self.b.any_ref, 0);
        match self.types.get(t) {
            Type::Any => Some(object),
            Type::Param(p) => {
                let hi = self.syms.tparam(p).upper;
                if hi == t { Some(object) } else { self.bound_erasure(hi, depth + 1) }
            }
            Type::AppParam(p, args) => match self.app_param_upper(p, args) {
                Some(hi) => self.bound_erasure(hi, depth + 1),
                None => Some(object),
            },
            Type::Member(..) | Type::AppMember(..) | Type::Decl(_) => {
                let (_, hi) = self.member_bounds(t);
                if hi == t { Some(object) } else { self.bound_erasure(hi, depth + 1) }
            }
            Type::Union(a, b) => {
                let (a, b) = (self.bound_erasure(a, depth + 1)?, self.bound_erasure(b, depth + 1)?);
                Some(self.erased_lub(a, b))
            }
            Type::Inter(a, b) => {
                let (a, b) = (self.bound_erasure(a, depth + 1)?, self.bound_erasure(b, depth + 1)?);
                Some(if self.compare_erased(a, b) != std::cmp::Ordering::Greater { a } else { b })
            }
            Type::Class(c, _) if self.syms.class(c).kind == ClassKind::Opaque => match self.opaque_erasure(t) {
                Some(under) => self.bound_erasure(under, depth + 1),
                None => Some(object),
            },
            Type::Refined(parent, _) => self.bound_erasure(parent, depth + 1),
            _ => self.stable_erasure(t),
        }
    }

    /// Whether a value of the scrutinee's type may be `null` at run time: the negation of
    /// dotc's `isNotNull` (Types.scala 384 to 395), which the pattern matcher asks before an
    /// extractor's null test (PatternMatcher.scala 495). A literal type and a value class cannot
    /// be `null`, nor an object; a proxy is its underlying type (an abstract type its upper
    /// bound, a path the type it stands for, an opaque type its underlying type in its scope and
    /// its bound, `Any`, outside); an intersection may be `null` only if both parts may, a union
    /// if either may. Not `Null`'s conformance (`null_conforms`), which asks the lower bound.
    fn scrutinee_may_be_null(&mut self, sty: TypeId) -> bool {
        self.may_be_null(sty, 0)
    }

    fn may_be_null(&mut self, t: TypeId, depth: u32) -> bool {
        if depth > 32 {
            return true;
        }
        let t = self.deref_alias(t);
        match self.types.get(t) {
            Type::Lit(_) => false,
            Type::Any => true,
            Type::Class(c, _) if c == self.b.null => true,
            Type::Class(c, _) if self.syms.class(c).kind == ClassKind::Opaque => {
                if self.transparent.contains(&c) {
                    match self.opaque_erasure(t) {
                        Some(under) => self.may_be_null(under, depth + 1),
                        None => true,
                    }
                } else {
                    true
                }
            }
            Type::Class(c, _) => self.syms.class(c).kind != ClassKind::Object && self.is_reference_class(c),
            Type::Union(a, b) => self.may_be_null(a, depth + 1) || self.may_be_null(b, depth + 1),
            Type::Inter(a, b) => self.may_be_null(a, depth + 1) && self.may_be_null(b, depth + 1),
            Type::Param(p) => {
                let hi = self.syms.tparam(p).upper;
                hi == t || self.may_be_null(hi, depth + 1)
            }
            Type::AppParam(p, args) => match self.app_param_upper(p, args) {
                Some(hi) => self.may_be_null(hi, depth + 1),
                None => true,
            },
            Type::Member(..) | Type::AppMember(..) | Type::Decl(_) => {
                let (_, hi) = self.member_bounds(t);
                hi == t || self.may_be_null(hi, depth + 1)
            }
            Type::Refined(parent, _) => self.may_be_null(parent, depth + 1),
            Type::Term(_) | Type::Select(..) | Type::This(_) | Type::Match(..) | Type::Alias(..) => match self.dependent_underlying(t) {
                Some(u) if u != t => self.may_be_null(u, depth + 1),
                _ => true,
            },
            _ => true,
        }
    }

    /// The value of the stable path type `t` as an expression: an unqualified term through the
    /// scope (`stable_path_expr`), a selection through its prefix, kept whole (`other.token`,
    /// not the enclosing instance's `token`).
    fn path_expr(&mut self, t: TypeId) -> Option<TExprId> {
        match self.types.get(t) {
            Type::Term(s) => self.stable_path_expr(s),
            Type::Select(p, s) => {
                let prefix = self.path_expr(p)?;
                Some(self.prog.add(TExpr::Field(prefix, s)))
            }
            Type::This(c) => Some(self.this_ref(c)),
            _ => None,
        }
    }

    /// scalac's check of a pattern's run-time type test (`TypeTestsCasts.checkSensical`, in its
    /// erasure phase): a test of the class `t` erases to that no value of the scrutinee's type
    /// `sty` can pass is an error, "Unreachable case" (E030), beside the space engine's warning.
    /// It fails where every class of the scrutinee (each part of a union, an intersection's
    /// narrower class) derives neither from the tested class nor it from them, and is final or
    /// meets a final class or a class where neither is a trait; primitive and non-primitive
    /// classes, value classes and JavaScript types are left alone, as are a union's parts, the
    /// tests of inlined code and of what no program file wrote. An intersection's parts are each
    /// a test (`case _: (C2 & T)`).
    pub(super) fn check_sensical_test(&mut self, t: TypeId, sty: TypeId, span: Span) {
        if self.dead_case || self.inline.depth > 0 || self.inline.lenient_match > 0 || self.checks_inline_definition() || self.quote.level > 0 || !self.program_source(self.env.file) {
            return;
        }
        let t = self.deref_alias(t);
        let test = match self.types.get(t) {
            Type::Inter(a, b) => {
                self.check_sensical_test(a, sty, span);
                return self.check_sensical_test(b, sty, span);
            }
            Type::Class(c, _) => c,
            _ => return,
        };
        // scalac tests the generic tuple types and arrays otherwise (`isInstanceOfTuple`).
        let tuples = [self.b.tuple_trait, self.b.cons_tuple, self.b.non_empty_tuple, self.b.empty_tuple];
        if test == self.b.array || tuples.contains(&Some(test)) {
            return;
        }
        let mut found = Vec::new();
        if !self.erased_classes(sty, &mut found, 8) {
            return;
        }
        let mut why = None;
        for f in found {
            match self.never_passes(f, test) {
                Some(w) => why = why.or(Some(w)),
                None => return,
            }
        }
        // An error of scalac's erasure phase, which does not run after a typer that reported one.
        if let Some(why) = why {
            let shown = self.show(sty);
            let file = self.env.file;
            self.diags.late_error(file, span, format!("unreachable case: type {} {}", shown, why));
        }
    }

    /// Whether a case's written pattern is a wildcard or a binder of one, after which scalac's
    /// pattern matcher drops the cases (not after a type test that always passes, `_: C1` over a
    /// `C1`, `t: Throwable`, which the typing may make a wildcard of).
    pub(super) fn catches_all(&self, p: crate::ast::PatId) -> bool {
        match self.cur_ast().pat(p) {
            Pat::Wildcard | Pat::Bind(_, None) => true,
            Pat::Bind(_, Some(q)) => self.catches_all(q),
            _ => false,
        }
    }

    /// The classes a value of type `t` erases to, one per part of a union, into `out`; `false`
    /// where one is no class the check can judge (an abstract type without a class bound, an
    /// intersection of two classes neither of which derives from the other, a bottom type).
    fn erased_classes(&mut self, t: TypeId, out: &mut Vec<ClassId>, depth: u32) -> bool {
        let Some(depth) = depth.checked_sub(1) else { return false };
        // A written union splits, through an alias as scalac's `dealias`; an abstract type's union
        // bound does not (`effective_class`).
        let t = self.deref(t);
        let t = if matches!(self.types.get(t), Type::Alias(..)) { self.deref_alias(t) } else { t };
        match self.types.get(t) {
            Type::Union(a, b) => self.erased_classes(a, out, depth) && self.erased_classes(b, out, depth),
            Type::Inter(a, b) => {
                let (mut l, mut r) = (Vec::new(), Vec::new());
                if !self.erased_classes(a, &mut l, depth) || !self.erased_classes(b, &mut r, depth) {
                    return false;
                }
                for x in l {
                    for &y in &r {
                        match (self.derives_class(x, y), self.derives_class(y, x)) {
                            (true, _) => out.push(x),
                            (_, true) => out.push(y),
                            _ => return false,
                        }
                    }
                }
                true
            }
            Type::Any | Type::Error | Type::Nothing => false,
            _ => match self.effective_class(t, 8) {
                Some(c) if c != self.b.null => {
                    out.push(if c == self.b.any_val { return false } else { c });
                    true
                }
                _ => false,
            },
        }
    }

    /// The class a value of the type `t` erases to, as scalac's `classSymbol` reads a type through
    /// its bounds: of an abstract type its whole upper bound's, of an intersection the operand's
    /// class that derives from the other's and none where neither does (`T <: Parent & Mark` is
    /// not judged, in either order), of a union its join's.
    fn effective_class(&mut self, t: TypeId, depth: u32) -> Option<ClassId> {
        let depth = depth.checked_sub(1)?;
        let t = self.deref(t);
        match self.types.get(t) {
            Type::Inter(a, b) => {
                let (x, y) = (self.effective_class(a, depth)?, self.effective_class(b, depth)?);
                match (self.derives_class(x, y), self.derives_class(y, x)) {
                    (true, _) => Some(x),
                    (_, true) => Some(y),
                    _ => None,
                }
            }
            Type::Param(p) => match self.syms.tparam(p).upper {
                ANY => None,
                upper => self.effective_class(upper, depth),
            },
            Type::AppParam(p, args) => {
                let upper = self.app_param_upper(p, args)?;
                self.effective_class(upper, depth)
            }
            Type::Match(..) | Type::Alias(..) | Type::This(_) | Type::Term(_) | Type::Select(..) | Type::Member(..) | Type::AppMember(..) | Type::Decl(_) | Type::Refined(..) => {
                let under = self.dependent_underlying(t)?;
                self.effective_class(under, depth)
            }
            _ => self.class_of(t),
        }
    }

    /// Whether the class `sub` derives from `sup`, every class of references from `AnyRef`.
    fn derives_class(&mut self, sub: ClassId, sup: ClassId) -> bool {
        let b = &self.b;
        let primitive = [b.int, b.long, b.double, b.byte, b.short, b.float, b.boolean, b.char, b.unit].contains(&sub);
        sub == sup || (sup == self.b.any_ref && !primitive) || {
            self.complete_class(sub);
            self.syms.class(sub).base_types.iter().any(|&(b, _)| b == sup)
        }
    }

    /// Why no value of the class `found` passes a test of the class `test` (scalac's `check` in
    /// `checkSensical`), or `None` where some can or the pair is not one the check judges.
    fn never_passes(&mut self, found: ClassId, test: ClassId) -> Option<String> {
        let primitive = |w: &Self, c: ClassId| {
            let b = &w.b;
            [b.int, b.long, b.double, b.byte, b.short, b.float, b.boolean, b.char, b.unit].contains(&c)
        };
        let value_class = |w: &mut Self, c: ClassId| !primitive(w, c) && (w.syms.class(c).value_class || w.derives_class(c, w.b.any_val));
        let js = |w: &Self, c: ClassId| w.syms.class(c).js != crate::symbols::JsKind::Scala;
        if primitive(self, found) != primitive(self, test) || value_class(self, found) || value_class(self, test) || js(self, found) || js(self, test) {
            return None;
        }
        if self.derives_class(found, test) || self.derives_class(test, found) {
            return None;
        }
        let final_class = |w: &Self, c: ClassId| {
            let info = w.syms.class(c);
            matches!(info.kind, ClassKind::Object | ClassKind::EnumCase) || info.mods & crate::ast::mods::FINAL != 0 || primitive(w, c) || c == w.b.string || c == w.b.array
        };
        let trait_ = |w: &Self, c: ClassId| w.syms.class(c).kind == ClassKind::Trait;
        let tested = format!("{} {}", if trait_(self, test) { "trait" } else { "class" }, self.name_ref(self.syms.class(test).name));
        // A trait against a final class is judged by the final class's ancestors, which the
        // builtin classes and the lean std's declarations of the JDK's list in part (`String`
        // implements `CharSequence`): judged only where they are the program's or a library's.
        let judged = |w: &mut Self, fin: ClassId, other: ClassId| !trait_(w, other) || w.complete_ancestry(fin);
        if final_class(self, found) && judged(self, found, test) {
            Some(format!("is not a subclass of {}", tested))
        } else if (final_class(self, test) && judged(self, test, found)) || (!trait_(self, test) && !trait_(self, found)) {
            Some(format!("and {} are unrelated", tested))
        } else {
            None
        }
    }

    /// Whether every ancestor of `c` is declared in full (the program's, or a library's from its
    /// class files and pickles): none a builtin class or one of the lean std's sources, but `Any`
    /// and `AnyRef`.
    fn complete_ancestry(&mut self, c: ClassId) -> bool {
        self.complete_class(c);
        let bases: Vec<ClassId> = self.syms.class(c).base_types.iter().map(|&(b, _)| b).collect();
        bases.into_iter().chain(std::iter::once(c)).all(|b| {
            let info = self.syms.class(b);
            b == self.b.any_ref
                || b == self.b.any_val
                || (info.kind != ClassKind::Builtin && info.def.is_some() && !self.source(info.file).is_std)
                || (info.kind != ClassKind::Builtin && info.def.is_none() && info.file != crate::source::FileId(0) && self.in_jar(info.file))
        })
    }

    /// The run-time test for `t` on a value of type `sty`; a type argument the scrutinee does
    /// not settle and an abstract type cannot be tested, which is a warning unless `@unchecked`.
    fn boxed_kinds(&self, c: ClassId) -> u8 {
        let info = self.syms.class(c);
        let Owner::Package(p) = info.owner else { return 0 };
        let pkg = if self.pkg_is(p, "java.lang") {
            "java.lang"
        } else if self.pkg_is(p, "java.io") {
            "java.io"
        } else {
            return 0;
        };
        crate::tir::boxed::ancestor(pkg, self.interner.get(info.name))
    }

    /// A box of `java.lang` is tested as its primitive is (`case _: Int`), its values being the
    /// primitives on JavaScript and in the interpreter and its class the JVM's box there.
    fn box_test(&mut self, c: ClassId) -> Option<TypeTest> {
        let info = self.syms.class(c);
        let Owner::Package(p) = info.owner else { return None };
        if !self.pkg_is(p, "java.lang") {
            return None;
        }
        let test = match self.interner.get(info.name) {
            "Integer" => TypeTest::Int,
            "Long" => TypeTest::Long,
            "Short" => TypeTest::Short,
            "Byte" => TypeTest::Byte,
            "Float" => TypeTest::Float,
            "Double" => TypeTest::Number,
            "Boolean" => TypeTest::Bool,
            "Character" if self.jvm || self.interp => TypeTest::Char,
            "Character" => TypeTest::Str,
            _ => return None,
        };
        Some(test)
    }

    /// On JavaScript the box of a primitive is the primitive, a JS string, number, bigint or
    /// boolean, so the test of a class the box extends (`CharSequence`, `Comparable`, `Number`)
    /// takes the primitive by its kind too, as Scala.js tests an ancestor of a hijacked class.
    fn boxed_test(&mut self, c: ClassId, test: TypeTest) -> TypeTest {
        let kinds = if self.jvm || self.interp { 0 } else { self.boxed_kinds(c) };
        if kinds == 0 {
            return test;
        }
        let mut boxed: Option<TestId> = None;
        for (bit, kind) in [
            (crate::tir::boxed::STR, TypeTest::Str),
            (crate::tir::boxed::NUMBER, TypeTest::Number),
            (crate::tir::boxed::LONG, TypeTest::Long),
            (crate::tir::boxed::BOOL, TypeTest::Bool),
            (crate::tir::boxed::UNIT, TypeTest::Unit),
        ] {
            if kinds & bit != 0 {
                let k = self.prog.add_test(kind);
                boxed = Some(match boxed {
                    Some(b) => self.prog.add_test(TypeTest::Or(b, k)),
                    None => k,
                });
            }
        }
        let class = self.prog.add_test(test);
        TypeTest::Or(boxed.unwrap(), class)
    }

    pub fn test_for(&mut self, t: TypeId, sty: TypeId, span: Span, unchecked: bool) -> TestId {
        let t = self.deref_alias(t);
        // An inline body's tests are checked where it expands, with the call's types, as scalac
        // checks them on the inlined code alone; a generator's pattern without `case` reports a
        // test it cannot check as refutable instead (`refutable_is_error`).
        let unchecked = unchecked || self.checks_inline_definition() || self.refutable_is_error;
        // Inside a quote a type parameter is filled in where the quote is instantiated, and in a
        // stored inline body where the body is instantiated, and the test is derived again there.
        if (self.quote.level > 0 || self.checks_inline_definition()) && matches!(self.types.get(t), Type::Param(_) | Type::AppParam(..)) {
            let test = self.prog.add_test(TypeTest::Always);
            self.prog.deferred_tests.insert(test, t);
            return test;
        }
        let test = match self.types.get(t) {
            Type::Any | Type::Error => TypeTest::Always,
            Type::Param(_) | Type::AppParam(..) => {
                if !unchecked {
                    let msg = format!(
                        "the type test for {} cannot be checked at runtime because it refers to a type parameter",
                        self.show(t)
                    );
                    self.warn(span, msg);
                }
                let hi = match self.types.get(t) {
                    Type::Param(p) => self.syms.tparam(p).upper,
                    // `F[Int]` under `F[X] <: Base` erases to its applied bound's erasure
                    // (TypeErasure.scala 773-779, `checkedSuperType`).
                    Type::AppParam(p, args) => self.app_param_upper(p, args).unwrap_or(ANY),
                    _ => ANY,
                };
                return self.bound_test(t, hi, sty, span);
            }
            Type::Union(a, b) => {
                let (x, y) = (self.test_for(a, sty, span, unchecked), self.test_for(b, sty, span, unchecked));
                TypeTest::Or(x, y)
            }
            Type::Inter(a, b) => {
                let (x, y) = (self.test_for(a, sty, span, unchecked), self.test_for(b, sty, span, unchecked));
                TypeTest::And(x, y)
            }
            // The refinement itself is not checked, as scalac tests the class alone.
            Type::Refined(parent, _) => return self.test_for(parent, sty, span, true),
            Type::Lit(l) => {
                let v = self.types.lit_val(l);
                TypeTest::Value(self.literal_expr(v))
            }
            Type::Class(c, args) => {
                if args != EMPTY_LIST && !unchecked && !self.args_determined(c, args, sty) {
                    let msg = format!(
                        "the type test for {} cannot be checked at runtime because its type arguments can't be determined from {}",
                        self.show(t),
                        self.show(sty)
                    );
                    self.warn(span, msg);
                }
                let boxed = self.box_test(c);
                let b = &self.b;
                if c == b.int {
                    TypeTest::Int
                } else if c == b.double {
                    TypeTest::Number
                } else if c == b.long {
                    TypeTest::Long
                } else if c == b.byte {
                    TypeTest::Byte
                } else if c == b.short {
                    TypeTest::Short
                } else if c == b.float {
                    TypeTest::Float
                } else if c == b.char && (self.jvm || self.interp) {
                    TypeTest::Char
                } else if c == b.string || c == b.char {
                    TypeTest::Str
                } else if c == b.boolean {
                    TypeTest::Bool
                } else if c == b.unit {
                    TypeTest::Unit
                } else if c == b.array {
                    TypeTest::Array
                } else if c == b.null {
                    TypeTest::Null
                } else if c == b.any_ref {
                    TypeTest::AnyRef
                } else if c == b.any_val {
                    TypeTest::AnyVal
                } else if self.is_function_class(c) {
                    TypeTest::Function((self.syms.class(c).tparams.len() - 1) as u8)
                } else if let Some(test) = boxed {
                    test
                } else if Some(c) == b.cons_tuple && b.non_empty_tuple.is_some() && !self.link_mode() {
                    // `h *: t` is any tuple of one element or more, as scalac tests it
                    // (`Tuples.isInstanceOfNonEmptyTuple`): the tuple classes extend `NonEmptyTuple`.
                    TypeTest::Trait(b.non_empty_tuple.unwrap())
                } else if self.is_tuple_class(c) && self.syms.class(c).tparams.len() > 22 && b.non_empty_tuple.is_some() {
                    // A tuple type past 22 elements, a TupleXXL's, is any tuple of one element or
                    // more to scalac's test (`Tuples.isInstanceOfNonEmptyTuple`), as `h *: t` is.
                    TypeTest::Trait(b.non_empty_tuple.unwrap())
                } else {
                    match (self.syms.class(c).kind, self.syms.class(c).js) {
                        // An opaque type erases to its underlying type everywhere, so its test
                        // is that type's; outside its scope the test is unchecked (Magnolia's
                        // `v.isInstanceOf[p]` on a default value of an opaque field).
                        (ClassKind::Opaque, _) => {
                            if !unchecked && !self.transparent.contains(&c) {
                                let msg = format!(
                                    "the type test for {} cannot be checked at runtime because it refers to an opaque type",
                                    self.show(t)
                                );
                                self.warn(span, msg);
                            }
                            match self.opaque_erasure(t) {
                                Some(u) => return self.test_for(u, sty, span, true),
                                None => TypeTest::Always,
                            }
                        }
                        // A JS trait leaves no trace on its instances, which are plain JS objects.
                        (ClassKind::Trait, JsKind::Native | JsKind::Object) => {
                            let msg = format!(
                                "{} is a JS trait, which cannot be tested at runtime; test a native class or a property instead",
                                self.name_str(self.syms.class(c).name)
                            );
                            self.error(span, msg);
                            TypeTest::Always
                        }
                        (ClassKind::Trait | ClassKind::Enum, _) => self.boxed_test(c, TypeTest::Trait(c)),
                        _ => self.boxed_test(c, TypeTest::Class(c)),
                    }
                }
            }
            // An abstract type member erases to its bound (cats' newtypes, `NonEmptySetImpl.Type[A]`
            // with `Type[A] <: AnyRef`), and the test is unchecked as a type parameter's is.
            Type::Member(..) | Type::AppMember(..) | Type::Decl(_) => {
                if !unchecked {
                    let msg = format!(
                        "the type test for {} cannot be checked at runtime because it refers to an abstract type member",
                        self.show(t)
                    );
                    self.warn(span, msg);
                }
                let (_, hi) = self.member_bounds(t);
                return self.bound_test(t, hi, sty, span);
            }
            // A singleton type's test is `isInstance` of the singleton (TypeTestsCasts.scala
            // 325-326), the identity test `case _: x.type` makes; a path the
            // typer cannot name here (a member of no enclosing class) is the error below.
            Type::Term(_) | Type::Select(..) if self.path_expr(t).is_some() => {
                let value = self.path_expr(t).unwrap();
                self.prog.set_type(value, t);
                TypeTest::Value(value)
            }
            _ => {
                let msg = format!("this type cannot be tested at runtime: {}", self.show(t));
                self.error(span, msg);
                TypeTest::Always
            }
        };
        self.prog.add_test(test)
    }

    /// Whether the scrutinee type fixes the type arguments a test asks for: `Some[Int]` on an
    /// `Option[Int]`, but not `List[Int]` on `Any`, whose elements the run time cannot see.
    fn args_determined(&mut self, c: ClassId, args: TList, sty: TypeId) -> bool {
        let asked = self.types.items(args).to_vec();
        let tested = self.types.class(c, &asked);
        let mark = self.snapshot();
        let below = self.is_sub(sty, tested);
        self.rollback(mark);
        if below {
            return true;
        }
        self.settle_class(c);
        let n = self.syms.class(c).tparams.len();
        let (known, _) = self.solve_pattern_class(c, n, sty);
        asked.iter().zip(&known).all(|(&a, &k)| {
            let a = self.deref(a);
            matches!(a, ANY | NOTHING | ERROR) || self.types.is_wild(a) || self.deref(k) == a || self.is_case_binder(a)
        })
    }

    fn checkable_array_test(&mut self, t: TypeId) -> bool {
        let t = self.deref_alias(t);
        match self.types.get(t) {
            Type::Class(c, args) if c == self.b.array => {
                let element = self.types.items(args).first().copied();
                element.map_or(true, |e| self.reified_element(e))
            }
            _ => false,
        }
    }

    /// Whether the JVM's test of an array sees what the element type says: an array carries
    /// its component's class, so a class is seen, with type arguments only where they say
    /// nothing, and an array of such; a type parameter and an abstract type are not.
    fn reified_element(&mut self, elem: TypeId) -> bool {
        let elem = self.deref_alias(elem);
        match self.types.get(elem) {
            Type::Class(c, args) if c == self.b.array => {
                let element = self.types.items(args).first().copied();
                element.map_or(true, |e| self.reified_element(e))
            }
            Type::Class(c, args) => {
                let items = self.types.items(args).to_vec();
                let mut all_wild = true;
                for a in items {
                    let a = self.deref(a);
                    all_wild &= self.types.is_wild(a) || a == ANY;
                }
                args == EMPTY_LIST || all_wild || self.args_determined(c, args, ANY)
            }
            Type::Union(a, b) | Type::Inter(a, b) => self.reified_element(a) && self.reified_element(b),
            // A singleton's test looks at the class the path's value has.
            Type::Any | Type::Nothing | Type::Lit(_) | Type::Wild | Type::BoundedWild(..) | Type::Term(_) | Type::Select(..) | Type::This(_) => true,
            _ => false,
        }
    }

    /// A type variable the pattern of the case binds (`case l: List[t]`), which a type test
    /// leaves undetermined as it does a wildcard.
    fn is_case_binder(&self, t: TypeId) -> bool {
        self.case_binders.iter().any(|&(_, b)| b == t)
    }
}

/// `Array[C[?, ? <: B]]` as written: the element's type arguments are wildcards, which the
/// class of an array's component says nothing about, so the test is checkable.
fn array_element_args_wild(ast: &crate::ast::Ast, ty: crate::ast::TyExprId) -> bool {
    use crate::ast::TyExpr;
    let TyExpr::Apply(_, args) = ast.ty(ty) else { return false };
    let [elem] = ast.ty_list(args) else { return false };
    let TyExpr::Apply(_, elem_args) = ast.ty(*elem) else { return false };
    let wild = |t: crate::ast::TyExprId| matches!(ast.ty(t), TyExpr::Wildcard | TyExpr::BoundedWildcard(..));
    !ast.ty_list(elem_args).is_empty() && ast.ty_list(elem_args).iter().all(|&a| wild(a))
}

/// `T @unchecked`, or a type with such an argument (`List[T @unchecked]`), which is not warned
/// about as a type test that cannot be checked.
fn type_unchecked(ast: &crate::ast::Ast, ty: crate::ast::TyExprId) -> bool {
    use crate::ast::TyExpr;
    match ast.ty(ty) {
        TyExpr::Unchecked(_) => true,
        TyExpr::Apply(_, args) => ast.ty_list(args).iter().any(|&a| type_unchecked(ast, a)),
        _ => false,
    }
}

