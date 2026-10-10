//! Quotes, splices and macros. A quote `'{ ... }` is typed at its definition site into a
//! tree-building form: its body with a hole local where each splice `${ e }` stands, applied
//! through the template `$quote` to the expressions that fill the holes and to the `Type` values
//! of the type parameters the body mentions; the interpreter copies the body with the holes
//! filled when the quote runs (`src/interp/quoted.rs`). A splice at the top level of an inline
//! method body is a macro: the call is expanded by running the splice's expression in the
//! interpreter, with `Quotes` bound, and taking the tree it returns as the expansion.

use super::inline_definition::StoredNode;
use super::profile::{About, Kind, Outcome};
use super::Worker;
use crate::ast::{DefKind, Expr, ExprId, PatId, TyExpr, TyExprId};
use crate::intern::{FxMap, Name};
use crate::interp::{Failure, Interp, Limits, MacroCtx, TreeRef, Value};
use crate::source::{FileId, Span};
use crate::symbols::*;
use crate::tir::*;
use crate::types::*;
use std::sync::Arc;

/// The interpreter's budget for one macro expansion.
const MACRO_STEPS: u64 = 1_000_000_000;
const MACRO_DEPTH: u32 = 2_000;

#[derive(Default)]
pub struct QuoteState {
    /// How many quotes enclose the expression being typed, less the splices inside them.
    pub level: u32,
    /// The quotes being typed, innermost last, with the holes their splices opened so far.
    frames: Vec<Vec<(SymId, TExprId)>>,
    /// The ranges of `Program::exprs` that hold the code of the holes: level-0 code inside a
    /// quote, which an anonymous class of the quote does not capture from. Cleared as each
    /// outermost quote begins and left after it, so that a worker's at the join are stale.
    pub(super) hole_ranges: Vec<(usize, usize)>,
    /// Set while the body of a quote pattern is typed: a splice `$x` binds `x` to the code it
    /// stands for instead of splicing.
    pattern: Option<PatternHoles>,
    /// How many macro expansions are under way.
    pub macro_depth: u32,
    classes: Option<QuotedClasses>,
    /// How many copies each class got from the runs of quotes under each outermost expansion
    /// site, which tell the copies of one site apart: a count that depends on the site's
    /// expansions alone, not on what was typed before them.
    copies: FxMap<(ClassId, Option<(FileId, u32)>), u32>,
    /// How many fresh names (`Symbol.freshName`) the macro runs under each outermost expansion
    /// site made, which number the next: one count per site whatever runs nest under it,
    /// begun anew with the site's expansion.
    pub(crate) fresh: FxMap<(FileId, u32), u32>,
    /// The calls of inline methods in quoted code, which expand where the quote is spliced:
    /// what their expansion needs besides the call's receiver and arguments.
    /// A registry: a quote of a library or std body is typed under the loader's
    /// lock by one worker and run by any worker's macro.
    pub(super) deferred: crate::arena::Layered<TExprId, Arc<DeferredInline>>,
    /// The deferred calls the runs of quotes copied, in order, for the macro that ran them to
    /// expand at its site: in the tree and in the bodies of its anonymous classes alike.
    pub(super) copied_deferred: Vec<TExprId>,
}

#[derive(Clone)]
pub(super) struct DeferredInline {
    pub sym: SymId,
    pub owner_subst: Subst,
    pub prefix: Option<TypeId>,
    pub sig: Arc<MethodSig>,
    pub subst: Subst,
    pub ret_ty: TypeId,
    pub span: Span,
    pub expected: Option<TypeId>,
    /// A given's expansion stands at the end of the site, as a using argument does.
    pub at_end: bool,
}

impl QuoteState {
    pub(super) fn deferred_at_end(&mut self, te: TExprId) {
        let Some(d) = self.deferred.get(&te) else { return };
        if d.at_end {
            return;
        }
        let mut moved = (**d).clone();
        moved.at_end = true;
        self.deferred.insert(te, Arc::new(moved));
    }

    pub(super) fn fork(&mut self) {
        self.deferred.fork();
    }

    pub(super) fn to_shared(&mut self, on: bool) {
        self.deferred.to_shared = on;
    }

    pub(super) fn apply_pending(&mut self) {
        self.deferred.apply_pending();
    }

    pub(super) fn merge_own(&mut self) {
        self.deferred.merge_own();
    }

    /// Whether the expression at `index` belongs to the code of a hole.
    pub(super) fn in_hole_filler(&self, index: usize) -> bool {
        self.hole_ranges.iter().any(|&(start, end)| start <= index && index < end)
    }
}

/// What a macro run takes (`Worker::run_macro`): the local of its `Quotes`, its code, the type
/// its tree stands at, the splice's position, where its diagnostics begin and the clock of its
/// typing.
pub(super) struct MacroRun {
    pub quotes: SymId,
    pub code: TExprId,
    pub ret: TypeId,
    pub span: Span,
    pub mark: usize,
    pub t_pre: Option<std::time::Instant>,
}

#[derive(Default)]
struct PatternHoles {
    /// The hole local, the pattern the code standing there is matched against, and the type
    /// of that code.
    holes: Vec<(SymId, PatId, TypeId)>,
}

impl QuoteState {
    /// The state under the merge's ids (`merge.rs`): the deferred calls of quoted code, which
    /// a run after the merge expands, and the copies each class got.
    pub(super) fn remap(&mut self, r: &mut super::merge::Remap) {
        for frame in &mut self.frames {
            for (s, e) in frame {
                *s = r.map_sym(*s);
                *e = r.map_expr(*e);
            }
        }
        let deferred = std::mem::take(&mut self.deferred.local);
        for (e, mut d) in r.entries_in_order(deferred, |e| r.map_expr(e)) {
            r.deferred_inline(&mut d);
            self.deferred.local.insert(e, d);
        }
        for e in &mut self.copied_deferred {
            *e = r.map_expr(*e);
        }
        let copies = std::mem::take(&mut self.copies);
        self.copies.extend(copies.into_iter().map(|((c, site), n)| ((r.map_class(c), site), n)));
    }

    /// An outermost expansion site begins: its fresh names are numbered from the first.
    pub(super) fn begin_site(&mut self, file: FileId, start: u32) {
        self.fresh.remove(&(file, start));
    }

    /// Another worker's state: what the quotes typed before the fork left (the deferred
    /// calls, the copies counted, the classes found), nothing of what is under way.
    pub(super) fn attach(&self) -> QuoteState {
        QuoteState { classes: self.classes, copies: self.copies.clone(), fresh: self.fresh.clone(), deferred: self.deferred.attach(), ..Default::default() }
    }

    /// Takes over another worker's state after the bodies: the copies made under its
    /// expansion sites and the deferred calls of its quotes.
    pub(super) fn absorb(&mut self, other: &mut QuoteState) {
        debug_assert!(other.frames.is_empty() && other.copied_deferred.is_empty());
        self.copies.extend(std::mem::take(&mut other.copies));
        self.fresh.extend(std::mem::take(&mut other.fresh));
        self.deferred.absorb(other.deferred.take_local());
        if self.classes.is_none() {
            self.classes = other.classes;
        }
    }

    /// Forgets the copies made under the expansion sites of `file`, whose bodies are typed
    /// again (`incremental.rs`).
    pub(super) fn forget_file(&mut self, file: FileId) {
        self.copies.retain(|&(_, site), _| site.map_or(true, |(f, _)| f != file));
        self.fresh.retain(|&(f, _), _| f != file);
    }
}

#[derive(Clone, Copy)]
pub(crate) struct QuotedClasses {
    pub(crate) quotes: Option<ClassId>,
    quotes_impl: Option<ClassId>,
    pub(crate) expr: Option<ClassId>,
    pub(crate) ty: Option<ClassId>,
    stop: Option<ClassId>,
}

impl<'a> Worker<'a> {
    pub(crate) fn quoted_classes(&mut self) -> QuotedClasses {
        if let Some(c) = self.quote.classes {
            return c;
        }
        let c = QuotedClasses {
            quotes: self.class_at(&["scala", "quoted", "Quotes"]),
            quotes_impl: self.class_at(&["scala", "quoted", "QuotesImpl"]),
            expr: self.class_at(&["scala", "quoted", "Expr"]),
            ty: self.class_at(&["scala", "quoted", "Type"]),
            stop: self.class_at(&["scala", "quoted", "runtime", "StopMacroExpansion"]),
        };
        self.quote.classes = Some(c);
        c
    }

    /// `scala.quoted.Expr[t]`.
    fn expr_type(&mut self, t: TypeId) -> TypeId {
        match self.quoted_classes().expr {
            Some(c) => self.types.class(c, &[t]),
            None => ERROR,
        }
    }

    /// `scala.quoted.Type[t]`.
    pub(super) fn quoted_type_of(&mut self, t: TypeId) -> TypeId {
        match self.quoted_classes().ty {
            Some(c) => self.types.class(c, &[t]),
            None => ERROR,
        }
    }

    /// The `T` of an `Expr[T]`, or of a `Type[T]` when `of_type` is set.
    fn quoted_arg(&mut self, t: TypeId, of_type: bool) -> Option<TypeId> {
        let classes = self.quoted_classes();
        let class = if of_type { classes.ty } else { classes.expr }?;
        let t = self.zonk(t);
        let base = self.base_type(t, class)?;
        match self.types.get(base) {
            Type::Class(_, args) => self.types.items(args).first().copied(),
            _ => None,
        }
    }

    /// `f` typed outside the quotes under way: a body typed on demand from inside one is its
    /// definition's, at level 0, its inline calls expanded as the walk expands them, where a
    /// quote's are kept for the site it is spliced at (dotty's `Inlining.InliningTreeMap` keeps
    /// a quote's level for the trees inside it alone).
    pub(super) fn outside_quotes<T>(&mut self, f: impl FnOnce(&mut Self) -> T) -> T {
        // Outside every quote, which most bodies are, there is nothing to set aside.
        if self.quote.level == 0 && self.quote.frames.is_empty() && self.quote.hole_ranges.is_empty() && self.quote.pattern.is_none() {
            return f(self);
        }
        let level = std::mem::replace(&mut self.quote.level, 0);
        let frames = std::mem::take(&mut self.quote.frames);
        let holes = std::mem::take(&mut self.quote.hole_ranges);
        let pattern = self.quote.pattern.take();
        let result = f(self);
        self.quote.level = level;
        self.quote.frames = frames;
        self.quote.hole_ranges = holes;
        self.quote.pattern = pattern;
        result
    }

    /// Whether the code being typed is a splice's inside a quote (a hole's).
    fn in_splice_of_quote(&self) -> bool {
        self.quote.frames.len() > self.quote.level as usize
    }

    /// A `Quotes` the given search found inside a splice of a quote, which scalac's search finds
    /// as the splice's own context parameter: a copy of the node marked for the pickle (the node
    /// itself may be an answer the search remembers, which a search outside the splice copies).
    pub(super) fn splice_quotes(&mut self, te: TExprId, target: TypeId) -> TExprId {
        let quotes = self.quoted_classes().quotes;
        if !self.in_splice_of_quote() || quotes.is_none() || self.class_of(target) != quotes {
            return te;
        }
        let copied = self.copy_expr(te);
        self.capture_form(copied, crate::tir::capture::Form::SpliceQuotes);
        copied
    }

    /// A `Quotes` given has to be in scope for a quote, as the interpreter binds one for a
    /// splice and a macro implementation takes one: the one found.
    fn require_quotes(&mut self, span: Span) -> Option<TExprId> {
        let Some(c) = self.quoted_classes().quotes else {
            self.error(span, "scala.quoted.Quotes is missing from the standard library");
            return None;
        };
        let ty = self.types.class(c, &[]);
        let mark = self.diags.items.len();
        let found = self.resolve_given(ty, span);
        if found.is_none() {
            self.drop_reported_since(mark);
            self.error(span, "No given instance of type scala.quoted.Quotes was found for a quote");
        }
        found
    }

    /// The `Quotes` in scope where nothing requires one (a `Type` value the search synthesizes, a
    /// quote pattern), searched for in a build that captures the bodies alone, for the pickle.
    fn captured_quotes(&mut self, span: Span) -> Option<TExprId> {
        if !self.capturing() {
            return None;
        }
        let quotes = self.quoted_classes().quotes?;
        let ty = self.types.class(quotes, &[]);
        let mark = self.diags.items.len();
        let found = self.resolve_given_for_capture(ty, span);
        self.drop_reported_since(mark);
        found
    }

    // ---- quotes ----

    pub(super) fn type_quote(&mut self, body: ExprId, span: Span, expected: Option<TypeId>) -> (TExprId, TypeId) {
        let quotes = self.require_quotes(span);
        let expected_body = expected.and_then(|t| self.quoted_arg(t, false));
        if self.quote.frames.is_empty() {
            self.quote.hole_ranges.clear();
        }
        self.quote.level += 1;
        self.quote.frames.push(Vec::new());
        self.push_scope();
        let captured = self.capture_mark();
        let (te, ty) = self.type_expr(body, expected_body);
        // A stable path has its singleton type where the expected type asks for it (`'{ Nil }`
        // for an `Expr[Nil.type]`, scala-library's `Nil` being a val).
        let ty = match expected_body {
            Some(exp) if self.types.is_path(exp) && ty != exp && self.conforms_as_path(te, exp) => exp,
            _ => ty,
        };
        // A macro reads the tree's own type: a block is typed against the expected type and
        // answers with it, where the tree is its result expression's (`'{ val fa = $a; fa *:
        // $acc }` against `Expr[Any]`).
        let ty = match self.prog.expr(te) {
            TExpr::Block(_, res) => {
                let own = self.prog.type_of(res).unwrap_or(ty);
                self.prog.set_type(te, own);
                own
            }
            _ => ty,
        };
        self.pop_scope();
        let holes = self.quote.frames.pop().unwrap_or_default();
        self.quote.level -= 1;
        let ty = self.solve_inferred(ty);
        let ty = self.widen_lit(ty);
        let types = self.quote_type_params(Some(te), ty, span);
        let mut binders = Vec::new();
        self.tree_binders(te, &mut binders);
        if captured.is_some() {
            self.capture_publish(captured);
        }
        let quote = self.prog.quotes.len();
        self.prog.quotes.push(TQuote { body: Some(te), ty, holes: holes.clone(), types: types.clone(), binders, quotes });
        let out = self.quote_template(quote, holes.iter().map(|&(_, e)| e).chain(types.iter().map(|&(_, e)| e)).collect());
        (out, self.expr_type(ty))
    }

    /// `'[T]`, and the `Type[T]` value the given search synthesizes: the type with the `Type`
    /// values of its parameters.
    pub(super) fn type_quote_type(&mut self, t: TyExprId, span: Span) -> (TExprId, TypeId) {
        let quotes = self.require_quotes(span);
        let ty = self.resolve_type(t);
        let ty = self.zonk(ty);
        match self.quoted_type_value_with(ty, span, quotes) {
            Some(te) => (te, self.quoted_type_of(ty)),
            None => {
                let msg = format!("no Type is available for {} in this scope", self.show(ty));
                self.error(span, msg);
                (self.prog.add(TExpr::Unit), ERROR)
            }
        }
    }

    /// `Type[t]` synthesized for a `t`, with the `Type` values in scope for its type
    /// parameters; a parameter without one stays a parameter of the type, which a macro that
    /// only inspects it reads as such (`DeriveJsonCodec.gen[Box[A]]` under `given [A: Codec]`,
    /// and `Schema.schemaForMap[K, V](..)` expanded inside a `given [K: TextKey, V: Schema]`,
    /// whose macro names the abstract `K`), as scalac's `Type.of[K]` at such a call site.
    pub(super) fn quoted_type_value(&mut self, t: TypeId, span: Span) -> Option<TExprId> {
        if self.types.has_vars(t) {
            return None;
        }
        let quotes = self.captured_quotes(span);
        self.quoted_type_value_with(t, span, quotes)
    }

    fn quoted_type_value_with(&mut self, t: TypeId, span: Span, quotes: Option<TExprId>) -> Option<TExprId> {
        if self.types.has_vars(t) {
            return None;
        }
        let (t, mut types) = self.heal_path_members(t, span);
        let mut params = Vec::new();
        self.collect_type_params(t, &mut params);
        // A bare parameter is what the search that asks for this value looks for: asking
        // again would only recurse to the depth limit.
        params.retain(|&p| self.types.param(p) != t);
        for p in params {
            let pty = self.types.param(p);
            let target = self.quoted_type_of(pty);
            let mark = self.diags.items.len();
            let found = self.resolve_given_typed(target, span);
            self.drop_reported_since(mark);
            if let Some((e, _)) = found {
                types.push((p, e));
            }
        }
        let quote = self.prog.quotes.len();
        self.prog.quotes.push(TQuote { body: None, ty: t, holes: Vec::new(), types: types.clone(), binders: Vec::new(), quotes });
        Some(self.quote_template(quote, types.iter().map(|&(_, e)| e).collect()))
    }

    /// A type member of a value inside the quoted type (`Path.Select[init.Underlying, ..]`)
    /// takes the `Type` value in scope for it, as scalac heals a reference from the wrong
    /// staging level: it becomes a fresh parameter of the quote, bound to that value.
    fn heal_path_members(&mut self, t: TypeId, span: Span) -> (TypeId, Vec<(TParamId, TExprId)>) {
        if !self.types.has_paths(t) || matches!(self.types.get(t), Type::Member(..)) {
            return (t, Vec::new());
        }
        let mut members = Vec::new();
        self.collect_path_members(t, &mut members);
        let mut pairs = Vec::new();
        let mut types = Vec::new();
        for m in members {
            let target = self.quoted_type_of(m);
            let mark = self.diags.items.len();
            let found = self.resolve_given_typed(target, span);
            self.drop_reported_since(mark);
            if let Some((e, _)) = found {
                let name = match self.types.get(m) {
                    Type::Member(_, n) => n,
                    _ => continue,
                };
                let p = self.syms.new_tparam(name, 0);
                pairs.push((m, self.types.param(p)));
                types.push((p, e));
            }
        }
        (self.types.replace(t, &pairs), types)
    }

    fn collect_path_members(&self, t: TypeId, out: &mut Vec<TypeId>) {
        if !self.types.has_paths(t) {
            return;
        }
        match self.types.get(t) {
            Type::Member(p, _) if matches!(self.types.get(p), Type::Term(_) | Type::Select(..)) => {
                if !out.contains(&t) {
                    out.push(t);
                }
            }
            Type::Class(_, args) | Type::AppParam(_, args) | Type::Alias(_, args) => {
                for &a in self.types.items(args) {
                    self.collect_path_members(a, out);
                }
            }
            Type::AppMember(m, args) => {
                self.collect_path_members(m, out);
                for &a in self.types.items(args) {
                    self.collect_path_members(a, out);
                }
            }
            Type::Lambda(_, b) => self.collect_path_members(b, out),
            Type::Union(a, b) | Type::Inter(a, b) => {
                self.collect_path_members(a, out);
                self.collect_path_members(b, out);
            }
            _ => {}
        }
    }

    /// The `Type[X]` given the search falls back to, as scalac's `Type.of[X]`.
    pub fn quoted_type_given(&mut self, class: ClassId, target: TypeId, span: Span) -> Option<TExprId> {
        if self.quoted_classes().ty != Some(class) {
            return None;
        }
        // The argument may still be a variable of the search that asks, bounded by what the
        // target fixed it to.
        let target = self.solve_bounded_in(target);
        let Type::Class(_, args) = self.types.get(target) else { return None };
        let &[arg] = self.types.items(args) else { return None };
        let arg = self.deref(arg);
        self.quoted_type_value(arg, span)
    }

    fn quote_template(&mut self, quote: usize, args: Vec<TExprId>) -> TExprId {
        let mut all = vec![self.prog.add(TExpr::Int(quote as i32))];
        all.extend(args);
        let l = self.prog.list(&all);
        let s = self.prog.add_str("$quote");
        self.prog.add(TExpr::Js(s, l))
    }

    /// The type parameters that the quote mentions and the `Type` values in scope for them.
    /// One without a `Type` stays a parameter of the tree, which is what scalac rejects as a
    /// reference from the wrong staging level; here the expansion keeps it as it is.
    fn quote_type_params(&mut self, body: Option<TExprId>, ty: TypeId, span: Span) -> Vec<(TParamId, TExprId)> {
        let mut params = Vec::new();
        self.collect_type_params(ty, &mut params);
        if let Some(body) = body {
            self.tree_type_params(body, &mut params);
        }
        self.type_values_for(&params, span)
    }

    /// The `Type` values in scope for type parameters, for those that have one.
    fn type_values_for(&mut self, params: &[TParamId], span: Span) -> Vec<(TParamId, TExprId)> {
        let mut out = Vec::new();
        for &p in params {
            let pty = self.types.param(p);
            let target = self.quoted_type_of(pty);
            let mark = self.diags.items.len();
            let found = self.resolve_given_typed(target, span);
            self.drop_reported_since(mark);
            if let Some((te, _)) = found {
                out.push((p, te));
            }
        }
        out
    }

    pub fn collect_type_params(&self, t: TypeId, out: &mut Vec<TParamId>) {
        match self.types.get(t) {
            Type::Param(p) => {
                if !out.contains(&p) {
                    out.push(p);
                }
            }
            Type::AppParam(p, args) => {
                if !out.contains(&p) {
                    out.push(p);
                }
                for &a in self.types.items(args) {
                    self.collect_type_params(a, out);
                }
            }
            Type::Class(_, args) | Type::AppMember(_, args) | Type::Alias(_, args) | Type::AppVar(_, args) => {
                for &a in self.types.items(args) {
                    self.collect_type_params(a, out);
                }
            }
            Type::Lambda(_, b) => self.collect_type_params(b, out),
            Type::Union(a, b) | Type::Inter(a, b) => {
                self.collect_type_params(a, out);
                self.collect_type_params(b, out);
            }
            Type::Select(p, _) | Type::Member(p, _) | Type::Match(p, _) => self.collect_type_params(p, out),
            Type::Refined(p, _) => {
                let (_, rs) = self.types.refinements_of(t);
                self.collect_type_params(p, out);
                for r in rs {
                    match self.types.refinement(r) {
                        Refinement::Alias(_, a) => self.collect_type_params(a, out),
                        Refinement::Bounds(_, l, u) => {
                            self.collect_type_params(l, out);
                            self.collect_type_params(u, out);
                        }
                        Refinement::Val(_, _, ty) => self.collect_type_params(ty, out),
                        Refinement::Term(..) => {}
                    }
                }
            }
            _ => {}
        }
    }

    /// The locals a tree defines: vals, local defs and their parameters, lambda parameters,
    /// pattern binders.
    pub(super) fn tree_binders(&self, e: TExprId, out: &mut Vec<SymId>) {
        let prog = &self.prog;
        let list = |t: &Self, l: crate::ast::ListRef, out: &mut Vec<SymId>| {
            for &x in prog.expr_list(l) {
                t.tree_binders(x, out);
            }
        };
        match prog.expr(e) {
            TExpr::Field(r, _) | TExpr::Unary(_, r) | TExpr::ToStr(r, _) | TExpr::TypeTest(r, _) | TExpr::Cast(r, ..) | TExpr::Index(r, _) | TExpr::Spread(r) | TExpr::Return(r) | TExpr::Throw(r, _) | TExpr::JsSelect(r, _) | TExpr::Splice(r) => {
                self.tree_binders(r, out)
            }
            TExpr::Lambda(params, body) => {
                out.extend_from_slice(prog.sym_list(params));
                self.tree_binders(body, out);
            }
            TExpr::CallStatic(_, args) | TExpr::New(_, args) | TExpr::NewVia(_, args) | TExpr::StrConcat(args) | TExpr::Js(_, args) | TExpr::SeqLit(args) | TExpr::ArrayLit(args) | TExpr::ObjLit(args) => {
                list(self, args, out)
            }
            TExpr::CallMethod(r, _, args) | TExpr::CallClosure(r, args) => {
                self.tree_binders(r, out);
                list(self, args, out);
            }
            TExpr::If(c, t, els) => {
                self.tree_binders(c, out);
                self.tree_binders(t, out);
                if let Some(x) = els {
                    self.tree_binders(x, out);
                }
            }
            TExpr::While(a, b) | TExpr::Assign(a, b) | TExpr::Prim(_, a, b) => {
                self.tree_binders(a, out);
                self.tree_binders(b, out);
            }
            TExpr::Block(stmts, res) => {
                for s in &prog.stmts[stmts.range()] {
                    match *s {
                        TStmt::Expr(x) => self.tree_binders(x, out),
                        TStmt::Val(v, x) => {
                            out.push(v);
                            self.tree_binders(x, out);
                        }
                        TStmt::Pat(p, x) => {
                            self.pat_binders(p, out);
                            self.tree_binders(x, out);
                        }
                        TStmt::Fun(f) => {
                            let fun = &prog.funs[f.idx()];
                            out.push(fun.sym);
                            out.extend_from_slice(&fun.params);
                            if let Some(b) = fun.body {
                                self.tree_binders(b, out);
                            }
                        }
                    }
                }
                self.tree_binders(res, out);
            }
            TExpr::Match(scrut, cases) => {
                self.tree_binders(scrut, out);
                for c in &prog.cases[cases.range()] {
                    self.pat_binders(c.pat, out);
                    if let Some(g) = c.guard {
                        self.tree_binders(g, out);
                    }
                    self.tree_binders(c.body, out);
                }
            }
            TExpr::Try(i) => {
                let t = &prog.tries[i as usize];
                self.tree_binders(t.body, out);
                for c in &prog.cases[t.cases.range()] {
                    self.pat_binders(c.pat, out);
                    self.tree_binders(c.body, out);
                }
                if let Some(f) = t.finalizer {
                    self.tree_binders(f, out);
                }
            }
            _ => {}
        }
    }

    pub(super) fn pat_binders(&self, p: TPatId, out: &mut Vec<SymId>) {
        let prog = &self.prog;
        match prog.pats[p.idx()] {
            TPat::Bind(s, inner) => {
                out.push(s);
                if let Some(i) = inner {
                    self.pat_binders(i, out);
                }
            }
            TPat::Test(_, _, inner) => self.pat_binders(inner, out),
            TPat::Class(_, _, _, subs) | TPat::Alt(subs) => {
                for &s in &prog.pat_lists[subs.range()] {
                    self.pat_binders(s, out);
                }
            }
            TPat::Seq(items, rest) => {
                for &s in &prog.pat_lists[items.range()] {
                    self.pat_binders(s, out);
                }
                if let Some(r) = rest {
                    self.pat_binders(r, out);
                }
            }
            TPat::Unapply(s, _, inner) => {
                out.push(s);
                self.pat_binders(inner, out);
            }
            TPat::Wildcard | TPat::Equals(..) => {}
        }
    }

    /// A local like `s`, for the definitions of a quote's body renamed at each run.
    pub fn clone_local(&mut self, s: SymId) -> SymId {
        let (name, kind, mods, file, span, sig) = {
            let info = self.syms.sym(s);
            (info.name, info.kind, info.mods, info.file, info.span, info.sig.clone())
        };
        let fresh = self.syms.new_sym(name, kind, mods, Owner::Local, file, None, span);
        let mut info = self.syms.sym_mut(fresh);
        info.sig = sig;
        info.state().set(Completion::Done);
        if self.capturing() {
            self.capture_copy_local(s, fresh);
        }
        fresh
    }

    pub fn subst_local_type(&mut self, s: SymId, subst: &Subst) {
        let mut sig = self.sig_of(s).clone();
        sig.ret = self.types.subst(sig.ret, subst);
        self.syms.sym_mut(s).sig = Some(Arc::new(sig));
        if self.capturing() {
            self.capture_subst_local(s, subst);
        }
    }

    /// The type parameters in the recorded types of a tree.
    fn tree_type_params(&self, e: TExprId, out: &mut Vec<TParamId>) {
        if let Some(t) = self.prog.type_of(e) {
            self.collect_type_params(t, out);
        }
        // An inline call the quote keeps for its expansion names types in its type arguments
        // alone (`'{ summonEncoders[et & Tuple] }`).
        if let Some(d) = self.quote.deferred.get(&e) {
            for &(_, t) in d.subst.iter().chain(&d.owner_subst) {
                self.collect_type_params(t, out);
            }
        }
        let prog = &self.prog;
        let list = |t: &Self, l: crate::ast::ListRef, out: &mut Vec<TParamId>| {
            for &x in prog.expr_list(l) {
                t.tree_type_params(x, out);
            }
        };
        match prog.expr(e) {
            TExpr::Field(r, _) | TExpr::Unary(_, r) | TExpr::ToStr(r, _) | TExpr::TypeTest(r, _) | TExpr::Cast(r, ..) | TExpr::Index(r, _) | TExpr::Spread(r) | TExpr::Return(r) | TExpr::Throw(r, _) | TExpr::JsSelect(r, _) | TExpr::Lambda(_, r) => {
                self.tree_type_params(r, out)
            }
            TExpr::CallStatic(_, args) | TExpr::New(_, args) | TExpr::NewVia(_, args) | TExpr::StrConcat(args) | TExpr::Js(_, args) | TExpr::SeqLit(args) | TExpr::ArrayLit(args) | TExpr::ObjLit(args) => {
                list(self, args, out)
            }
            TExpr::CallMethod(r, _, args) | TExpr::CallClosure(r, args) => {
                self.tree_type_params(r, out);
                list(self, args, out);
            }
            TExpr::If(c, t, els) => {
                self.tree_type_params(c, out);
                self.tree_type_params(t, out);
                if let Some(x) = els {
                    self.tree_type_params(x, out);
                }
            }
            TExpr::While(a, b) | TExpr::Assign(a, b) | TExpr::Prim(_, a, b) => {
                self.tree_type_params(a, out);
                self.tree_type_params(b, out);
            }
            TExpr::Block(stmts, res) => {
                for s in &prog.stmts[stmts.range()] {
                    match *s {
                        TStmt::Expr(x) | TStmt::Val(_, x) | TStmt::Pat(_, x) => self.tree_type_params(x, out),
                        TStmt::Fun(f) => {
                            if let Some(b) = prog.funs[f.idx()].body {
                                self.tree_type_params(b, out);
                            }
                        }
                    }
                }
                self.tree_type_params(res, out);
            }
            TExpr::Match(scrut, cases) => {
                self.tree_type_params(scrut, out);
                for c in &prog.cases[cases.range()] {
                    if let Some(g) = c.guard {
                        self.tree_type_params(g, out);
                    }
                    self.tree_type_params(c.body, out);
                }
            }
            TExpr::Try(i) => {
                let t = &prog.tries[i as usize];
                self.tree_type_params(t.body, out);
                for c in &prog.cases[t.cases.range()] {
                    self.tree_type_params(c.body, out);
                }
                if let Some(f) = t.finalizer {
                    self.tree_type_params(f, out);
                }
            }
            _ => {}
        }
    }

    // ---- splices ----

    pub(super) fn type_splice(&mut self, e: ExprId, span: Span, expected: Option<TypeId>) -> (TExprId, TypeId) {
        if self.quote.level > 0 {
            return self.type_hole(e, span, expected);
        }
        if self.inline.depth > 0 {
            return self.expand_macro(e, span, expected);
        }
        if self.checks_inline_definition() {
            return self.keep_splice(e, span, expected);
        }
        if self.inline.retained > 0 {
            // The retained body of an inline method is never run: what a call of it expands to
            // is settled where the call stands.
            return (self.prog.add(TExpr::Unit), expected.unwrap_or(ERROR));
        }
        self.error(span, "Splice ${...} outside quotes or inline method");
        (self.prog.add(TExpr::Unit), ERROR)
    }

    /// A macro's splice in a body under the definition check: its code typed as an expansion
    /// types it, with `Quotes` given, and kept for the expansions to run as the context function
    /// of that `Quotes` scalac keeps (`${ (using q: Quotes) => impl(q) }`).
    fn keep_splice(&mut self, e: ExprId, span: Span, expected: Option<TypeId>) -> (TExprId, TypeId) {
        let classes = self.quoted_classes();
        let Some(quotes) = classes.quotes else {
            self.error(span, "scala.quoted.Quotes is missing from the standard library");
            return (self.prog.add(TExpr::Unit), ERROR);
        };
        let quotes_ty = self.types.class(quotes, &[]);
        let ret = expected.map(|t| self.zonk(t)).filter(|&t| !self.types.has_vars(t));
        // The definition's result type is what the splice's `Expr` holds, as scalac checks it,
        // any `Expr` where `Unit` is declared, whose value is discarded; an expansion leaves a
        // transparent macro's narrower tree to its call (`expand_macro`).
        let required = self.splice_code_type(ret.unwrap_or(ANY));
        self.push_scope();
        let q = self.fresh_local("quotes", quotes_ty, span);
        self.bind_given(q);
        let (code, code_ty) = self.type_expr(e, ret.map(|_| required));
        let code_ty = self.solve_inferred(code_ty);
        self.pop_scope();
        // `Expr`'s variance is its declaration's, which a definition checked before anything
        // else names the class has not read yet.
        if let Some(expr) = classes.expr {
            self.complete_class(expr);
        }
        if code_ty != ERROR && !self.is_sub(code_ty, required) {
            let msg = format!("type mismatch: found {}, required {}", self.show(code_ty), self.show(required));
            self.error(span, msg);
        }
        let ty = match ret {
            Some(t) => t,
            None => self.quoted_arg(code_ty, false).unwrap_or(ANY),
        };
        let binder = self.prog.syms(&[q]);
        let lambda = self.prog.add(TExpr::Lambda(binder, code));
        let context_function = self.context_function_class(1);
        let lambda_ty = self.types.class(context_function, &[quotes_ty, code_ty]);
        self.prog.set_type(lambda, lambda_ty);
        let te = self.prog.add(TExpr::Splice(lambda));
        self.prog.set_type(te, ty);
        self.note_splice(te);
        (te, ty)
    }

    /// The `Expr` a macro's splice is typed against where the macro returns `ret`: `Expr[Any]`
    /// for `Unit`, whose value is discarded, as scalac types `exprSplice[T]` there.
    fn splice_code_type(&mut self, ret: TypeId) -> TypeId {
        self.expr_type(if ret == self.b.t_unit { ANY } else { ret })
    }

    /// `${ e }` inside a quote: `e` is code of the enclosing level that yields the `Expr` the
    /// hole takes when the quote runs.
    fn type_hole(&mut self, e: ExprId, span: Span, expected: Option<TypeId>) -> (TExprId, TypeId) {
        let expected_expr = expected.map(|t| self.expr_type(t));
        self.quote.level -= 1;
        let start = self.prog.exprs.len();
        let (te, ty) = self.type_expr(e, expected_expr);
        self.quote.hole_ranges.push((start, self.prog.exprs.len()));
        self.quote.level += 1;
        let ty = self.solve_inferred(ty);
        let inner = match self.quoted_arg(ty, false) {
            Some(t) => t,
            None if ty == ERROR => ERROR,
            None => {
                let msg = format!("a splice takes an Expr, but this has type {}", self.show(ty));
                self.error(span, msg);
                ERROR
            }
        };
        let hole = self.fresh_local("hole", inner, span);
        if let Some(frame) = self.quote.frames.last_mut() {
            frame.push((hole, te));
        }
        let local = self.prog.add(TExpr::Local(hole));
        (local, inner)
    }

    /// `$x` or `${ p }` in a quote pattern: the code that stands where the hole is, whose type
    /// is what the position asks for, is matched against the pattern.
    pub(super) fn type_pattern_hole(&mut self, p: PatId, span: Span, expected: Option<TypeId>) -> (TExprId, TypeId) {
        if self.quote.pattern.is_none() {
            self.error(span, "a pattern splice is only allowed inside a quote pattern");
            return (self.prog.add(TExpr::Unit), ERROR);
        }
        let ty = expected.map(|t| self.zonk(t)).filter(|&t| !self.types.has_vars(t)).unwrap_or(ANY);
        let hole = self.fresh_local("hole", ty, span);
        if let Some(holes) = &mut self.quote.pattern {
            holes.holes.push((hole, p, ty));
        }
        let local = self.prog.add(TExpr::Local(hole));
        (local, ty)
    }

    /// Inside a quote, a member with a parameter of type `E & X` implements one whose parameter
    /// is `E`: the two are compared where the quote runs under scalac, with `E` filled in, and
    /// coincide there when the filled type is an `X`.
    pub(super) fn quoted_param_match(&mut self, a: TypeId, b: TypeId) -> bool {
        if self.quote.level == 0 {
            return false;
        }
        let parts = |t: &Self, x: TypeId| match t.types.get(x) {
            Type::Inter(p, q) => vec![p, q],
            _ => vec![x],
        };
        let (pa, pb) = (parts(self, a), parts(self, b));
        pa.iter().any(|&x| pb.iter().any(|&y| x == y || self.is_same(x, y)))
    }

    // ---- quote patterns ----

    /// `case '{ ... }` against an `Expr`: the body is typed as a quote whose splices bind, the
    /// match runs the interpreter's structural comparison through the template `$quoteMatch`,
    /// whose result carries the code of the holes and the types of the type variables.
    /// Whether the body of a quote pattern is typed now (dotty's `Mode.isQuotedPattern`).
    pub(super) fn in_quote_pattern(&self) -> bool {
        self.quote.pattern.is_some()
    }

    pub(super) fn type_quote_pattern(&mut self, body: ExprId, sty: TypeId, span: Span) -> TPatId {
        let scrutinee_arg = self.quoted_arg(sty, false);
        let (declared, body) = self.declared_pattern_type_vars(body);
        let mut var_names: Vec<Name> = declared.iter().map(|&(n, _, _)| n).collect();
        self.scan_pattern_type_vars(body, &mut var_names);
        let type_params = self.bind_quote_type_vars(&var_names, span);
        for (&(_, lower, upper), &(p, _)) in declared.iter().zip(&type_params) {
            if let Some(u) = upper {
                let u = self.resolve_type(u);
                self.syms.tparams[p.idx()].upper = u;
            }
            if let Some(l) = lower {
                let l = self.resolve_type(l);
                self.syms.tparams[p.idx()].lower = l;
            }
        }
        self.quote.pattern = Some(PatternHoles::default());
        self.quote.level += 1;
        self.quote.frames.push(Vec::new());
        let (te, _) = self.type_expr(body, scrutinee_arg);
        self.quote.frames.pop();
        self.quote.level -= 1;
        let holes = self.quote.pattern.take().map_or(Vec::new(), |p| p.holes);
        let mut binders = Vec::new();
        for &(_, pat, ty) in &holes {
            let bty = self.expr_type(ty);
            let tp = self.type_pattern(pat, bty);
            binders.push((tp, bty));
        }
        for &(p, given) in &type_params {
            let t = self.types.param(p);
            let ty = self.quoted_type_of(t);
            let tp = self.prog.add_pat(TPat::Bind(given, None));
            binders.push((tp, ty));
        }
        let mut mentioned = Vec::new();
        self.tree_type_params(te, &mut mentioned);
        for &(_, _, ty) in &holes {
            self.collect_type_params(ty, &mut mentioned);
        }
        mentioned.retain(|p| !type_params.iter().any(|&(q, _)| q == *p));
        let types = self.type_values_for(&mentioned, span);
        let quotes = self.captured_quotes(span);
        // The variables the pattern binds (not those it declares) in a contravariant position,
        // where their first occurrence stands in the holes' types, in the source's order.
        let vars: Vec<TParamId> = type_params.iter().map(|&(p, _)| p).collect();
        let hole_types: Vec<TypeId> = holes.iter().map(|&(_, _, ty)| ty).collect();
        let mut from_above = self.pattern_vars_from_above(&hole_types, &vars);
        for f in from_above.iter_mut().take(declared.len()) {
            *f = false;
        }
        let pat = self.prog.quote_pats.len();
        self.prog.quote_pats.push(TQuotePat {
            body: Some(te),
            ty: scrutinee_arg.unwrap_or(ANY),
            holes: holes.iter().map(|&(h, _, _)| h).collect(),
            type_params: type_params.iter().map(|&(p, _)| p).collect(),
            declared: declared.len() as u32,
            from_above,
            types: types.clone(),
            quotes,
        });
        self.quote_match_pattern(pat, sty, binders, types.iter().map(|&(_, e)| e).collect(), span)
    }

    /// Whether each of `vars` is first met in a contravariant position of `types`, read in order
    /// and covariantly at their roots, as dotty's `TreeMapWithVariance` meets a pattern's `Bind`s.
    fn pattern_vars_from_above(&mut self, types: &[TypeId], vars: &[TParamId]) -> Vec<bool> {
        let mut first: Vec<Option<i8>> = vec![None; vars.len()];
        for &t in types {
            self.first_variances(t, 1, vars, &mut first);
        }
        first.into_iter().map(|v| v.map_or(false, |v| v < 0)).collect()
    }

    fn first_variances(&mut self, t: TypeId, variance: i8, vars: &[TParamId], first: &mut [Option<i8>]) {
        let t = self.zonk(t);
        match self.types.get(t) {
            Type::Param(p) => {
                if let Some(i) = vars.iter().position(|&q| q == p) {
                    first[i].get_or_insert(variance);
                }
            }
            Type::Class(c, args) => {
                let args = self.types.items(args).to_vec();
                let variances: Vec<i8> = self.syms.class(c).tparams.iter().map(|&q| self.syms.tparam(q).variance).collect();
                for (k, a) in args.into_iter().enumerate() {
                    self.first_variances(a, variance * variances.get(k).copied().unwrap_or(0), vars, first);
                }
            }
            Type::AppParam(p, args) => {
                let args = self.types.items(args).to_vec();
                let variances = self.syms.tparam(p).hk_variances.clone();
                for (k, a) in args.into_iter().enumerate() {
                    self.first_variances(a, variance * variances.get(k).copied().unwrap_or(0), vars, first);
                }
            }
            Type::Union(a, b) | Type::Inter(a, b) => {
                self.first_variances(a, variance, vars, first);
                self.first_variances(b, variance, vars, first);
            }
            Type::BoundedWild(lo, hi) => {
                self.first_variances(lo, -variance, vars, first);
                self.first_variances(hi, variance, vars, first);
            }
            Type::Lambda(_, body) | Type::Poly(_, body) => self.first_variances(body, variance, vars, first),
            _ => {}
        }
    }

    /// `case '[T]` against a `Type`.
    pub(super) fn type_quote_type_pattern(&mut self, t: TyExprId, sty: TypeId, span: Span) -> TPatId {
        let mut var_names = Vec::new();
        self.collect_type_vars(t, &mut var_names);
        let type_params = self.bind_quote_type_vars(&var_names, span);
        let named: Vec<(Name, TParamId)> = var_names.iter().copied().zip(type_params.iter().map(|&(p, _)| p)).collect();
        self.bound_pattern_type_vars(t, &named);
        let ty = self.resolve_type(t);
        let mut binders: Vec<(TPatId, TypeId)> = Vec::new();
        for &(p, given) in &type_params {
            let pt = self.types.param(p);
            let ty = self.quoted_type_of(pt);
            let tp = self.prog.add_pat(TPat::Bind(given, None));
            binders.push((tp, ty));
        }
        let mut mentioned = Vec::new();
        self.collect_type_params(ty, &mut mentioned);
        mentioned.retain(|p| !type_params.iter().any(|&(q, _)| q == *p));
        let types = self.type_values_for(&mentioned, span);
        let quotes = self.captured_quotes(span);
        let vars: Vec<TParamId> = type_params.iter().map(|&(p, _)| p).collect();
        let from_above = self.pattern_vars_from_above(&[ty], &vars);
        let pat = self.prog.quote_pats.len();
        self.prog.quote_pats.push(TQuotePat { body: None, ty, holes: Vec::new(), type_params: vars, declared: 0, from_above, types: types.clone(), quotes });
        self.quote_match_pattern(pat, sty, binders, types.iter().map(|&(_, e)| e).collect(), span)
    }

    /// A type variable of a quote type pattern takes the bound of the parameter it stands for,
    /// as scalac infers it: `case '[t *: ts]` binds `ts <: Tuple`.
    fn bound_pattern_type_vars(&mut self, t: TyExprId, vars: &[(Name, TParamId)]) {
        let ast = self.cur_ast();
        match ast.ty(t) {
            TyExpr::Apply(f, args) => {
                let args: Vec<TyExprId> = ast.ty_list(args).to_vec();
                let ctor = self.resolve_type_ctor(f);
                let params = match self.types.get(ctor) {
                    // A jar's class reads its parameters' bounds when it completes.
                    Type::Ctor(c) => {
                        self.complete_class(c);
                        self.syms.class(c).tparams.clone()
                    }
                    _ => Vec::new(),
                };
                for (i, &a) in args.iter().enumerate() {
                    if let (TyExpr::TypeVar(n), Some(&cp)) = (self.cur_ast().ty(a), params.get(i)) {
                        if let Some(&(_, p)) = vars.iter().find(|&&(m, _)| m == n) {
                            let upper = self.syms.tparam(cp).upper;
                            if upper != ANY && !self.mentions_tparam_of(upper, Some(&params)) {
                                self.syms.tparams[p.idx()].upper = upper;
                            }
                        }
                    }
                    self.bound_pattern_type_vars(a, vars);
                }
            }
            TyExpr::Union(a, b) | TyExpr::Inter(a, b) => {
                self.bound_pattern_type_vars(a, vars);
                self.bound_pattern_type_vars(b, vars);
            }
            TyExpr::Tuple(items) => {
                for a in ast.ty_list(items).to_vec() {
                    self.bound_pattern_type_vars(a, vars);
                }
            }
            _ => {}
        }
    }

    /// The type variables of a quote pattern become type names of the case's scope, each with a
    /// `Type` given the match binds.
    fn bind_quote_type_vars(&mut self, names: &[Name], span: Span) -> Vec<(TParamId, SymId)> {
        let mut out = Vec::new();
        for &n in names {
            let p = self.syms.new_tparam(n, 0);
            let pt = self.types.param(p);
            if let Some(super::Frame::Locals { tparams, .. }) = self.env.frames.last_mut() {
                tparams.push((n, p));
            }
            self.case_binders.push((n, pt));
            let ty = self.quoted_type_of(pt);
            let given_name = self.interner.intern(&format!("{}$Type", self.name_ref(n)));
            let given = self.new_local(given_name, SymKind::Val, ty, span);
            self.bind_local(given_name, given);
            self.bind_given(given);
            out.push((p, given));
        }
        out
    }

    /// The type variables a quote pattern declares before its expression, `'{ type t <:
    /// Product; $m: M[t] }`, with their bounds, and the expression.
    fn declared_pattern_type_vars(&self, body: ExprId) -> (Vec<(Name, Option<TyExprId>, Option<TyExprId>)>, ExprId) {
        let ast = self.cur_ast();
        let Expr::Block(stmts) = ast.expr(body) else { return (Vec::new(), body) };
        let (last, decls) = match ast.stmt_list(stmts).split_last() {
            Some((crate::ast::Stmt::Expr(e), decls)) => (*e, decls),
            _ => return (Vec::new(), body),
        };
        let mut out = Vec::new();
        for s in decls {
            let crate::ast::Stmt::Def(d) = s else { return (Vec::new(), body) };
            match &ast.def(*d).kind {
                DefKind::TypeAlias { tparams, rhs: None, lower, upper } if tparams.is_empty() => out.push((ast.def(*d).name, *lower, *upper)),
                _ => return (Vec::new(), body),
            }
        }
        if out.is_empty() { (out, body) } else { (out, last) }
    }

    /// The lowercase type names of a quote pattern's body that are no type in scope: the type
    /// variables the pattern binds (`case '{ $x: t }`, `case '{ ($xs: List[t]).head }`).
    fn scan_pattern_type_vars(&mut self, e: ExprId, out: &mut Vec<Name>) {
        let ast = self.cur_ast();
        let mut tys: Vec<TyExprId> = Vec::new();
        self.collect_ascriptions(e, &mut tys);
        for t in tys {
            let mut names = Vec::new();
            self.lowercase_type_names(t, &mut names);
            for n in names {
                if !out.contains(&n) && self.lookup_type(n).is_none() {
                    out.push(n);
                }
            }
        }
        let _ = ast;
    }

    fn collect_ascriptions(&self, e: ExprId, out: &mut Vec<TyExprId>) {
        let ast = self.cur_ast();
        match ast.expr(e) {
            Expr::Typed(inner, t) => {
                out.push(t);
                self.collect_ascriptions(inner, out);
            }
            Expr::TypeApply(f, targs) => {
                self.collect_ascriptions(f, out);
                out.extend_from_slice(ast.ty_list(targs));
            }
            Expr::Apply(f, args) | Expr::UsingApply(f, args) => {
                self.collect_ascriptions(f, out);
                for &a in ast.expr_list(args) {
                    self.collect_ascriptions(a, out);
                }
            }
            Expr::Select(q, _) | Expr::Parens(q) | Expr::Splice(q) | Expr::Quote(q) | Expr::NamedArg(_, q) => self.collect_ascriptions(q, out),
            Expr::Infix(l, _, r) => {
                self.collect_ascriptions(l, out);
                self.collect_ascriptions(r, out);
            }
            Expr::Block(stmts) => {
                for s in ast.stmt_list(stmts) {
                    if let crate::ast::Stmt::Expr(x) = s {
                        self.collect_ascriptions(*x, out);
                    }
                }
            }
            Expr::Lambda(params, body) => {
                for p in &ast.lambda_params[params.range()] {
                    if let Some(t) = p.ty {
                        out.push(t);
                    }
                }
                self.collect_ascriptions(body, out);
            }
            Expr::Tuple(items) => {
                for &a in ast.expr_list(items) {
                    self.collect_ascriptions(a, out);
                }
            }
            Expr::New(t, args) => {
                out.push(t);
                for &a in ast.expr_list(args) {
                    self.collect_ascriptions(a, out);
                }
            }
            Expr::If(c, t, e) => {
                self.collect_ascriptions(c, out);
                self.collect_ascriptions(t, out);
                if let Some(e) = e {
                    self.collect_ascriptions(e, out);
                }
            }
            _ => {}
        }
    }

    fn lowercase_type_names(&self, t: TyExprId, out: &mut Vec<Name>) {
        let ast = self.cur_ast();
        match ast.ty(t) {
            TyExpr::Name(n) | TyExpr::TypeVar(n) => {
                if self.name_ref(n).starts_with(|c: char| c.is_lowercase()) && !out.contains(&n) {
                    out.push(n);
                }
            }
            TyExpr::Apply(f, args) => {
                self.lowercase_type_names(f, out);
                for &a in ast.ty_list(args) {
                    self.lowercase_type_names(a, out);
                }
            }
            TyExpr::Tuple(items) | TyExpr::Fun(items, _) => {
                for &a in ast.ty_list(items) {
                    self.lowercase_type_names(a, out);
                }
                if let TyExpr::Fun(_, r) = ast.ty(t) {
                    self.lowercase_type_names(r, out);
                }
            }
            TyExpr::Union(a, b) | TyExpr::Inter(a, b) => {
                self.lowercase_type_names(a, out);
                self.lowercase_type_names(b, out);
            }
            // `Mirror.Product { type MirroredElemLabels = labels }`
            TyExpr::Refined(parent, defs) => {
                self.lowercase_type_names(parent, out);
                for &d in ast.def_list(defs) {
                    if let DefKind::TypeAlias { rhs, lower, upper, .. } = &ast.def(d).kind {
                        for t in [*rhs, *lower, *upper].into_iter().flatten() {
                            self.lowercase_type_names(t, out);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// The pattern that runs `$quoteMatch` on the scrutinee and binds its results: an
    /// `Option` of the binders' values, one value or a tuple of them.
    fn quote_match_pattern(&mut self, pat: usize, sty: TypeId, binders: Vec<(TPatId, TypeId)>, types: Vec<TExprId>, span: Span) -> TPatId {
        let scrut = self.fresh_local("u", sty, span);
        let arg = self.prog.add(TExpr::Local(scrut));
        let index = self.prog.add(TExpr::Int(pat as i32));
        let mut args = vec![arg, index];
        args.extend(types);
        let l = self.prog.list(&args);
        let s = self.prog.add_str("$quoteMatch");
        let call = self.prog.add(TExpr::Js(s, l));
        let (value_ty, value_pat) = match binders.len() {
            0 => (self.b.t_unit, self.prog.add_pat(TPat::Wildcard)),
            1 => (binders[0].1, binders[0].0),
            n => {
                let tc = self.tuple_class(n);
                let tys: Vec<TypeId> = binders.iter().map(|&(_, t)| t).collect();
                let tuple_ty = self.tuple_type(&tys);
                let fields: Vec<SymId> = self.syms.class(tc).ctor_syms.concat();
                let pats: Vec<TPatId> = binders.iter().map(|&(p, _)| p).collect();
                let fl = self.prog.syms(&fields);
                let pl = self.prog.pat_lists.push_slice(&pats);
                (tuple_ty, self.prog.add_pat(TPat::Class(tc, tuple_ty, fl, pl)))
            }
        };
        let Some(some) = self.std_class("Some") else {
            return self.prog.add_pat(TPat::Wildcard);
        };
        self.complete_class(some);
        let field = self.syms.class(some).ctor_syms.first().and_then(|f| f.first().copied());
        let Some(field) = field else { return self.prog.add_pat(TPat::Wildcard) };
        let some_ty = self.types.class(some, &[value_ty]);
        let fl = self.prog.syms(&[field]);
        let pl = self.prog.pat_lists.push_slice(&[value_pat]);
        let inner = self.prog.add_pat(TPat::Class(some, some_ty, fl, pl));
        self.prog.add_pat(TPat::Unapply(scrut, call, inner))
    }

    // ---- macros ----

    /// The expansion of a macro: the splice's expression, typed with `Quotes` in scope, runs in
    /// the interpreter and the tree it returns replaces the call.
    fn expand_macro(&mut self, e: ExprId, span: Span, expected: Option<TypeId>) -> (TExprId, TypeId) {
        let p = self.phase(super::profile::Phase::Macro);
        let expanded = self.expand_macro_now(e, span, expected);
        self.phase_end(p);
        expanded
    }

    /// scalac runs a macro's code compiled, so what the splice names outside its quotes cannot
    /// come from the file being expanded: each such definition is "Cannot call macro m defined
    /// in the same source file" at the call, in the order scalac collects them.
    fn macro_dependencies_elsewhere(&mut self, splice: TExprId, site_file: crate::source::FileId, site_span: Span) -> bool {
        let mut deps: Vec<SymId> = Vec::new();
        self.macro_dependencies(splice, &mut deps);
        let mut ok = true;
        for s in deps.into_iter().rev() {
            if self.syms.sym(s).file != site_file || self.source(site_file).is_std || self.is_body_file(site_file) {
                continue;
            }
            let kind = match self.syms.sym(s).kind {
                SymKind::Object(_) => "object",
                SymKind::Def => "method",
                _ => "value",
            };
            let msg = format!("Cannot call macro {} {} defined in the same source file", kind, self.name_str(self.syms.sym(s).name));
            self.diags.error(site_file, site_span, msg);
            ok = false;
        }
        ok
    }

    fn macro_dependencies(&self, e: TExprId, out: &mut Vec<SymId>) {
        let push = |s: SymId, out: &mut Vec<SymId>| {
            if self.syms.sym(s).owner != Owner::Local {
                out.push(s);
            }
        };
        match self.prog.expr(e) {
            TExpr::Js(t, _) if self.prog.strings[t.0 as usize] == "$quote" => {}
            TExpr::CallStatic(s, args) => {
                push(s, out);
                for &a in self.prog.expr_list(args) {
                    self.macro_dependencies(a, out);
                }
            }
            TExpr::CallMethod(r, s, args) => {
                push(s, out);
                // An object receiver is `Main.m` or the `m` of the object itself, which scalac
                // tells apart and the IR does not: only the method is counted.
                if !matches!(self.prog.expr(r), TExpr::Module(_)) {
                    self.macro_dependencies(r, out);
                }
                for &a in self.prog.expr_list(args) {
                    self.macro_dependencies(a, out);
                }
            }
            TExpr::Static(s) => push(s, out),
            TExpr::Field(r, s) => {
                push(s, out);
                self.macro_dependencies(r, out);
            }
            TExpr::Module(c) => {
                if let Some(m) = self.syms.class(c).module_sym {
                    push(m, out);
                }
            }
            _ => {}
        }
    }

    /// The warnings the build's runs left about the objects with cacheable state, told as the
    /// build's own once its diagnostics are settled.
    pub(super) fn tell_cacheable_warnings(&mut self) {
        for msg in crate::interp::take_cacheable_warnings() {
            self.diags.warn(crate::source::NO_FILE, Span::default(), msg);
        }
    }

    fn expand_macro_now(&mut self, e: ExprId, span: Span, expected: Option<TypeId>) -> (TExprId, TypeId) {
        let classes = self.quoted_classes();
        let Some(quotes) = classes.quotes else {
            self.error(span, "scala.quoted.Quotes is missing from the standard library");
            return (self.prog.add(TExpr::Unit), ERROR);
        };
        let quotes_ty = self.types.class(quotes, &[]);
        let ret = expected.map(|t| self.zonk(t)).unwrap_or(ANY);
        let expr_ty = self.expr_type(ret);
        self.push_scope();
        let q = self.fresh_local("quotes", quotes_ty, span);
        self.bind_given(q);
        let mark = self.diags.items.len();
        let t_pre = self.profile.on.then(std::time::Instant::now);
        self.quote.macro_depth += 1;
        // The expected type guides the macro's type arguments, any `Expr` under `Unit` as at the
        // definition (`splice_code_type`); what the macro is declared to
        // return (`Expr[Any]` of a transparent one) need not conform to it, the expansion does,
        // and an open expected type is left for the expansion to settle.
        let hint = if self.types.has_vars(ret) { None } else { Some(self.splice_code_type(ret)) };
        let (te, te_ty) = self.type_expr(e, hint);
        let te_ty = self.solve_inferred(te_ty);
        let snap = self.snapshot();
        let fits = te_ty == ERROR || self.is_sub(te_ty, expr_ty);
        self.rollback(snap);
        if !fits {
            let any_expr = self.expr_type(ANY);
            if !self.is_sub(te_ty, any_expr) {
                let msg = format!("type mismatch: found {}, required {}", self.show(te_ty), self.show(expr_ty));
                self.error(span, msg);
            }
        }
        self.quote.macro_depth -= 1;
        self.pop_scope();
        if self.diags.items[mark..].iter().any(|d| !d.is_warning) {
            return (self.prog.add(TExpr::Unit), ERROR);
        }
        self.run_macro(MacroRun { quotes: q, code: te, ret, span, mark, t_pre })
    }

    /// One macro run, the lifecycle of every splice's expansion whatever its code came from: the
    /// caches the runs share restored and kept again (dropped where the run ended on its budget
    /// or depth, which may leave an object half made), the run's identity and epoch, the shared
    /// state it touched watched, the inline calls its quotes bring expanded at the site where it
    /// gives a tree and dropped where it fails: the same on success, on an abort and on
    /// exhaustion.
    pub(super) fn run_macro(&mut self, run: MacroRun) -> (TExprId, TypeId) {
        // The classes the run's quotes make are fixed up when its deferred calls expand.
        self.settling_classes(|t| t.run_macro_settling(run))
    }

    fn run_macro_settling(&mut self, run: MacroRun) -> (TExprId, TypeId) {
        let MacroRun { quotes: q, code: te, ret, span, mark, t_pre } = run;
        let classes = self.quoted_classes();
        let Some(quotes_impl) = classes.quotes_impl else {
            self.error(span, "scala.quoted.Quotes is missing from the standard library");
            return (self.prog.add(TExpr::Unit), ERROR);
        };
        let at = |s: &super::inline::InlineSite| (s.file, if s.at_end { Span { start: s.span.end, end: s.span.end } } else { s.span });
        let callee = self.inline.sites.last().map_or(SymId(0), |s| s.callee);
        let (site_file, site_span) = self.inline.sites.last().map_or((self.env.file, span), at);
        let (expansion_file, expansion_span) = self.inline.sites.first().map_or((site_file, site_span), at);
        // The file being compiled is the one of the outermost inline call, where the expansion
        // lands however deep the macro is nested in inline methods.
        let (unit_file, unit_span) = self.inline.sites.first().map_or((site_file, site_span), |s| (s.file, s.span));
        if !self.macro_dependencies_elsewhere(te, unit_file, unit_span) {
            return (self.prog.add(TExpr::Unit), ERROR);
        }
        let pre_ns = t_pre.map_or(0, |t| t.elapsed().as_nanos() as u64);
        let owners = self.sites.owners.clone();
        let owner_class = self.this_class();
        // A parameter whose argument was not bound to a local stands for the argument itself
        // in a quote, as scalac's `'v` of an inline parameter does; the quote's body, typed
        // with the definition, names the parameter, the expansion its proxy.
        // The argument a quote takes for a parameter has the type the argument has where the
        // body expands, as the retype path's typing of the parameter's name gives the quote a
        // tree of the argument's own type (a constant the binding folded to has none of its own).
        let mut params: FxMap<SymId, TExprId> = FxMap::default();
        let proxies: Vec<(SymId, SymId)> = self.inline.param_proxies.iter().map(|(&p, &x)| (p, x)).collect();
        for (param, proxy) in proxies {
            if let (Some(&(arg, _)), Some(bound)) = (self.inline.param_bindings.get(&proxy), self.inline.args.get(&proxy).copied()) {
                let arg = if self.prog.type_of(arg).is_none() { self.prog.typed(arg, bound.ty) } else { arg };
                params.insert(param, arg);
                params.insert(proxy, arg);
            }
        }
        // A receiver's proxy that stands for a stable receiver, which a stored body's quote names
        // for the method's class's `this` (`'x` of a field `x`), stands for it in the run too.
        for &(_, proxy, _) in &self.inline.this {
            if let Some(arg) = proxy.and_then(|p| self.inline.args.get(&p)) {
                params.entry(proxy.expect("the proxy")).or_insert(arg.expr);
            }
        }
        let prof = self.prof(Kind::Macro, span, About::Sym(callee));
        let receivers = self.inline.this.clone();
        let mut inline_path: Vec<(FileId, u32, u32)> = self.inline.sites.iter().map(|s| (s.file, s.span.start, s.span.end)).collect();
        if inline_path.is_empty() {
            inline_path.push((site_file, site_span.start, site_span.end));
        }
        let census_name = census_on().then(|| self.method_description(callee));
        let caches = crate::interp::take_caches();
        let deferred_mark = self.quote.copied_deferred.len();
        let class_mark = self.prog.classes.len();
        crate::measure::macro_run(1);
        let mut interp = Interp::new(self, Limits { steps: MACRO_STEPS, depth: MACRO_DEPTH });
        if let Some(c) = caches {
            interp.restore(*c);
        }
        interp.pure = true;
        let run = interp.next_run();
        let infos = std::cell::Cell::new(0);
        interp.macro_ctx = Some(std::rc::Rc::new(MacroCtx { run, site_file, site_span, expansion_file, expansion_span, unit_file, unit_span, owners, owner_class, params, inline_path, infos }));
        if prof.is_some() {
            interp.start_profile();
        }
        let t_run = prof.map(|_| super::profile::RunClock::start());
        // The run in an epoch of its own, watched for state other runs share (`interp::watch`).
        let outer_epoch = crate::interp::enter_epoch();
        if let Some(name) = census_name {
            crate::interp::name_epoch(|| format!("the run of the macro {}", name));
        }
        let run_guard = crate::interp::enter_run();
        let result = interp.module_value(quotes_impl).and_then(|quotes_value| {
            let mut vals = vec![(q, quotes_value)];
            let mut ctx = crate::interp::Ctx::plain();
            // The receivers of the expansions under way: an object is its instance, which the
            // splice may name through `this` or the proxy the expander bound.
            for &(c, proxy, _) in receivers.iter().rev() {
                let value = interp.module_value(c)?;
                match proxy {
                    Some(p) => vals.push((p, value)),
                    None => {
                        ctx.this = value;
                        ctx.class = Some(c);
                    }
                }
            }
            let env = crate::interp::Env { frame: crate::interp::Frame::with(None, vals), ctx };
            interp.eval_expr(te, Some(&env))
        });
        let hashes = crate::interp::run_hashes();
        drop(run_guard);
        crate::interp::leave_epoch(outer_epoch);
        let shared = crate::interp::take_touched();
        let run = t_run.map_or((0, 0), |t| t.stop());
        let histogram = interp.take_profile();
        let steps = MACRO_STEPS - interp.steps_left();
        let failed_typing = interp.typer.diags.items[mark..].iter().any(|d| !d.is_warning && !d.msg.starts_with("expected"));
        // A run that ended on its budget or depth may have left an object half made: the caches
        // go, and the next expansion makes the std's objects again.
        let exhausted = matches!(&result, Err(f) if matches!(f, Failure::Budget) || interp.ended_on_depth(f));
        let outcome = match result {
            Ok(_) if failed_typing => Err(String::new()),
            Ok(Value::Tree(TreeRef::Expr(t))) => Ok(t),
            Ok(other) => {
                let shown = interp.describe_value(&other);
                Err(format!("the macro returned {} where an Expr was expected", shown))
            }
            Err(Failure::Thrown(v)) => {
                let stopped = matches!((&v, classes.stop), (Value::Obj(o), Some(c)) if o.class == c);
                if stopped {
                    Err(String::new())
                } else {
                    let text = interp.describe(&Failure::Thrown(v));
                    Err(format!("exception in macro: {}", text.trim_start_matches("Exception in thread \"main\" ")))
                }
            }
            Err(Failure::Budget) => Err(format!("the macro exceeded the interpreter's budget of {} steps", MACRO_STEPS)),
            Err(Failure::Depth) => Err(format!("the macro exceeded the interpreter's call depth of {}", MACRO_DEPTH)),
            Err(Failure::Stale(msg)) => Err(msg),
            Err(Failure::Withheld) => Err(interp.describe(&Failure::Withheld)),
            Err(f @ Failure::Exit(_)) => Err(interp.describe(&f)),
            Err(Failure::Unsupported(msg)) => Err(match msg.strip_prefix("no builtin for scala.quoted.") {
                Some(rest) => format!("not supported yet: scala.quoted {}", rest.trim_end_matches(" (a @js template without a Scala body)")),
                None => format!("not supported yet in a macro: {}", msg),
            }),
        };
        let caches = interp.into_caches();
        crate::measure::macro_run(-1);
        crate::interp::keep_caches(if exhausted { None } else { Some(Box::new(caches)) });
        if crate::interp::watching() {
            let name = self.method_description(callee);
            census_note(&name, shared.as_ref().map(|t| t.what.as_str()), hashes);
            if let Some(t) = shared {
                self.shared_state_changed(|| format!("the macro {} changed {}, which other runs share", name, t.what), t.module);
            }
        }
        if let Some(p) = prof {
            self.profile.note_steps(p, steps);
            if let Some(h) = &histogram {
                self.profile.note_macro(callee, pre_ns, run, h);
            }
            self.profile.exit(p, if outcome.is_ok() { Outcome::Found } else { Outcome::NotFound });
        }
        match outcome {
            Ok(tree) => {
                self.expand_deferred_inlines(tree, deferred_mark, class_mark, span);
                let ty = self.prog.type_of(tree).unwrap_or(ret);
                (tree, ty)
            }
            Err(msg) => {
                self.quote.copied_deferred.truncate(deferred_mark);
                if !msg.is_empty() {
                    let macro_name = self.method_description(callee);
                    self.error(span, format!("{} (called by {})", msg, macro_name));
                    if let Some(d) = self.diags.items.last_mut() {
                        d.at_expansion = true;
                    }
                } else if !self.diags.items[mark..].iter().any(|d| !d.is_warning) {
                    self.error(span, "the macro expansion was aborted without an error message");
                }
                (self.prog.add(TExpr::Unit), ERROR)
            }
        }
    }

    /// Keeps a call of an inline method in quoted code as a call, for the site the quote is
    /// spliced at to expand: scalac inlines level-0 code only.
    pub(super) fn defer_inline(&mut self, call: &super::apply::MethodCall, sig: &Arc<MethodSig>, subst: &Subst, te: TExprId, ret_ty: TypeId, span: Span, expected: Option<TypeId>) {
        let d = DeferredInline {
            sym: call.sym,
            owner_subst: call.owner_subst.clone(),
            prefix: call.prefix,
            sig: sig.clone(),
            subst: subst.clone(),
            ret_ty,
            span,
            expected,
            at_end: false,
        };
        self.quote.deferred.insert(te, Arc::new(d));
        self.note_deferred(te);
    }

    /// The inline calls a macro's expansion brings from its quotes, expanded at the site: what
    /// they know of their position (`Position.ofMacroExpansion`) is the macro's call.
    fn expand_deferred_inlines(&mut self, tree: TExprId, mark: usize, class_mark: usize, site: Span) {
        if self.quote.copied_deferred.len() <= mark {
            return;
        }
        let copied: Vec<TExprId> = self.quote.copied_deferred.drain(mark..).collect();
        // What the expansion keeps: its tree and the bodies of the classes its quotes made; a
        // copy the macro built and dropped expands nowhere, as in scalac.
        let mut roots = vec![tree];
        for tc in &self.prog.classes[class_mark.min(self.prog.classes.len())..] {
            for &f in tc.methods.iter().chain(&tc.ctors) {
                roots.extend(self.prog.funs[f.idx()].body);
                roots.extend(self.prog.funs[f.idx()].defaults.iter().flatten().copied());
            }
            roots.extend(tc.ctor_defaults.iter().flatten().copied());
            roots.extend(tc.init.iter().filter_map(|i| match *i {
                TInit::Field(_, e) | TInit::Stmt(e) => Some(e),
                TInit::Parent(..) => None,
            }));
            if let Some(l) = tc.parent_args {
                roots.extend(self.prog.expr_list(l).iter().copied());
            }
        }
        let mut kept: FxMap<TExprId, ()> = FxMap::default();
        for r in roots {
            for e in self.prog.descendants(r) {
                kept.insert(e, ());
            }
        }
        let calls: Vec<TExprId> = copied.into_iter().filter(|e| kept.contains_key(e)).collect();
        let mut expanded_any = false;
        for e in calls {
            let Some(d) = self.quote.deferred.get(&e).cloned() else { continue };
            let (recv, args) = match self.prog.expr(e) {
                TExpr::CallMethod(r, _, args) => (Some(r), self.prog.expr_list(args).to_vec()),
                TExpr::CallStatic(_, args) => (None, self.prog.expr_list(args).to_vec()),
                TExpr::Field(r, _) => (Some(r), Vec::new()),
                _ => continue,
            };
            let call = super::apply::MethodCall { recv, sym: d.sym, owner_subst: d.owner_subst.clone(), ext_recv: None, prefix: d.prefix };
            self.inline.site_at_end = d.at_end;
            let expanded = match self.inline.site_env.clone() {
                Some(env) => {
                    let site_span = self.inline.sites.first().map_or(site, |s| s.span);
                    self.with_env((*env).clone(), |t| t.expand_inline(&call, &d.sig, &d.subst, &args, d.ret_ty, site_span, d.expected))
                }
                None => self.expand_inline(&call, &d.sig, &d.subst, &args, d.ret_ty, site, d.expected),
            };
            self.inline.site_at_end = false;
            if let Some((expanded, ty)) = expanded {
                let node = self.prog.expr(expanded);
                self.prog.exprs[e.idx()] = node;
                self.prog.set_type(e, ty);
                self.prog.copy_chain_marks(expanded, e);
                if self.capturing() {
                    self.capture_moved(expanded, e);
                }
                expanded_any = true;
            }
        }
        // A condition is classified once its calls have expanded (`transparent inline def yes
        // = true` in `if yes then ..`): the copy classified it as a call, which is no constant.
        if expanded_any {
            for (&e, _) in kept.iter() {
                if let (TExpr::If(c, ..), None, Some(ty)) = (self.prog.expr(e), self.prog.taken(e), self.prog.type_of(e)) {
                    self.mark_taken_branch(c, e, ty);
                }
            }
        }
    }

    /// The node `to`, a copy of `from` that a macro made by reflection, expands at the site as
    /// `from` does where `from` is a call a quote left for it.
    pub fn copy_deferred(&mut self, from: TExprId, to: TExprId) {
        if let Some(d) = self.quote.deferred.get(&from).cloned() {
            self.quote.deferred.insert(to, d);
            self.quote.copied_deferred.push(to);
        }
    }

    // ---- instantiation ----

    /// The body of a quote copied for one run: the holes take the trees that filled them, the
    /// proxies of a macro's parameters the arguments they stand for, and the type parameters
    /// that had a `Type` value the types they were given.
    pub fn instantiate_quote(&mut self, body: TExprId, holes: &FxMap<SymId, TExprId>, params: &FxMap<SymId, TExprId>, subst: &Subst, renames: &FxMap<SymId, SymId>) -> TExprId {
        self.instantiate_tree(body, holes, params, subst, renames, &[])
    }

    /// The reads of the expansion's by-name parameters in the copy `root` of a stored body's
    /// quote, which the definition typed as calls of the parameters' thunks: each a reference to
    /// the proxy among `proxies`, which a run fills with its argument's code (`MacroCtx::params`),
    /// as the retype path's typing of the quote at the call makes it the argument itself (`'expr`
    /// of `expr: => Boolean`). A by-name parameter the quote defines keeps its thunk's call.
    pub(super) fn read_by_name_params(&mut self, root: TExprId, proxies: &[SymId]) {
        let reads: Vec<(TExprId, SymId)> = self
            .prog
            .descendants(root)
            .filter_map(|e| match self.prog.expr(e) {
                TExpr::CallClosure(f, args) if args.len == 0 => match self.prog.expr(f) {
                    TExpr::Local(s) if proxies.contains(&s) => Some((e, s)),
                    _ => None,
                },
                _ => None,
            })
            .collect();
        for (e, s) in reads {
            self.prog.exprs[e.idx()] = TExpr::Local(s);
        }
    }

    /// `instantiate_quote` with the recorded types' paths of the renamed terms moved as well:
    /// `paths` pairs each with the path of what stands for it.
    pub(super) fn instantiate_tree(&mut self, body: TExprId, holes: &FxMap<SymId, TExprId>, params: &FxMap<SymId, TExprId>, subst: &Subst, renames: &FxMap<SymId, SymId>, paths: &[(SymId, TypeId)]) -> TExprId {
        let copies = self.capturing().then(FxMap::default);
        let mut copier = Copier { holes, params, subst, renames, paths, extra: FxMap::default(), classes: FxMap::default(), added: FxMap::default(), locals: None, carries_chain: false, map: None, copies, occurrences: Vec::new(), pat_copies: Vec::new(), filled: Vec::new(), stored: None, deferring: false, class_depth: 0, suspended: Vec::new(), scratch: Scratch::default(), reducing: false };
        let copied = copier.expr(self, body);
        if copier.copies.is_some() {
            let roots: Vec<TExprId> = std::iter::once(body).chain(copier.filled.iter().copied()).collect();
            let locals: Vec<(SymId, SymId)> = renames.iter().chain(copier.extra.iter()).map(|(&a, &b)| (a, b)).collect();
            self.capture_check_copy(&roots, &copier.occurrences, &copier.pat_copies, &locals);
        }
        copied
    }

    /// `instantiate_tree` for a stored inline body (`instantiate_definition`), which keeps what
    /// it made of each node: every expression and type test with its copy, a test copied
    /// whatever its kind, and the marks of the output an expression carries (an expansion's
    /// boundary and origin, a leaf, a folded literal, an interpolation, a soft join) given to
    /// its copy as `copy_expr` gives them. A quote's copy shares the tests of its kinds that
    /// hold no tree and takes no marks, as it did.
    /// `stored` gives the record's nodes as a copy reads them (`StoredIndex::node`), the
    /// receiver's node for `This` and the paths of `C.this` in the types.
    /// `map` holds what the earlier parts of the copy made, which this part adds to, and the
    /// lists of the part, which take the copies of `roots` last (`CopyMap`); `members` and
    /// `classes` what the parts copied of the classes the body makes.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn instantiate_tree_mapped(&mut self, roots: &[TExprId], params: &FxMap<SymId, TExprId>, subst: &Subst, renames: &FxMap<SymId, SymId>, paths: &[(SymId, TypeId)], stored: StoredCopy<'_>, map: CopyMap<'_>, members: &mut FxMap<SymId, SymId>, classes: &mut FxMap<ClassId, ClassId>) {
        let holes = FxMap::default();
        let copies = self.capturing().then(FxMap::default);
        let (extra, copied_classes) = (std::mem::take(members), std::mem::take(classes));
        let scratch = std::mem::take(&mut map.part.scratch);
        let mut copier = Copier { holes: &holes, params, subst, renames, paths, extra, classes: copied_classes, added: FxMap::default(), locals: None, carries_chain: false, map: Some(map), copies, occurrences: Vec::new(), pat_copies: Vec::new(), filled: Vec::new(), stored: Some(stored), deferring: stored.leaves_for_demand.is_some(), class_depth: 0, suspended: Vec::new(), scratch, reducing: false };
        for &r in roots {
            let copy = copier.expr(self, r);
            copier.map.as_mut().expect("the copy's map").part.roots.push(copy);
        }
        if copier.copies.is_some() {
            let sources: Vec<TExprId> = roots.iter().chain(&copier.filled).copied().collect();
            let locals: Vec<(SymId, SymId)> = renames.iter().chain(copier.extra.iter()).map(|(&a, &b)| (a, b)).collect();
            // A stored body's copy is checked as a quote's is, its losses reported; what it
            // carries is not counted with the quotes' copies, which a build's census tallies
            // whether or not it expands by substitution.
            let checked = self.prog.capture.as_deref().map_or(0, |c| c.copies_checked);
            self.capture_check_copy(&sources, &copier.occurrences, &copier.pat_copies, &locals);
            if let Some(c) = self.prog.capture.as_deref_mut() {
                c.copies_checked = checked;
            }
        }
        *members = std::mem::take(&mut copier.extra);
        *classes = std::mem::take(&mut copier.classes);
        let scratch = std::mem::take(&mut copier.scratch);
        copier.map.expect("the copy's map").part.scratch = scratch;
    }
}

/// What a copy of a stored body made of its nodes (`instantiate_tree_mapped`), over all its parts
/// so far, the instance's tables borrowed for a part: each expression with its copy, a
/// parameter's `Local` with the copy of its argument, and each type test with its copy; the type
/// of each copy, which a build that keeps no types keeps here alone; and the lists of the part
/// being copied (`CopyPart`).
pub struct CopyMap<'m> {
    pub exprs: &'m mut StoredTable<TExprId>,
    pub tests: &'m mut FxMap<TestId, TestId>,
    pub types: &'m mut CopyTypes,
    pub part: &'m mut CopyPart,
}

/// What the part of a copy being made adds, kept with the copy and cleared for each part so
/// that its lists grow once: the expressions and tests it copies, what it leaves, the classes,
/// the class bodies' `this`, and the copies of its roots.
#[derive(Default)]
pub struct CopyPart {
    pub copied: Vec<TExprId>,
    pub tests_copied: Vec<TestId>,
    /// The stored nodes a copy on demand left in the copy's tree in place of their copies, in
    /// the order met (`StoredCopy::leaves_for_demand`).
    pub left: Vec<TExprId>,
    /// The classes the stored body makes, each with its copy, in the order copied.
    pub classes: Vec<(ClassId, ClassId)>,
    /// The copies of `this` in those classes' bodies: the class's own, no receiver's.
    pub inner_this: Vec<TExprId>,
    /// The copies of the part's roots, in their order.
    pub roots: Vec<TExprId>,
    scratch: Scratch,
}

impl CopyPart {
    /// Ready for the next part: its lists empty, their room kept.
    pub fn start(&mut self) {
        self.copied.clear();
        self.tests_copied.clear();
        self.left.clear();
        self.classes.clear();
        self.inner_this.clear();
        self.roots.clear();
    }
}

/// A table of the stored nodes of a record, dense over the range of its nodes
/// (`StoredIndex::nodes`), a node outside it (an argument's or the receiver's proxy, which a copy
/// also copies, one entry for each such node however often it is copied) kept aside in the order
/// it came.
pub struct StoredTable<V> {
    lo: u32,
    dense: Vec<Option<V>>,
    outside: Vec<(TExprId, V)>,
}

impl<V> Default for StoredTable<V> {
    fn default() -> Self {
        StoredTable { lo: 0, dense: Vec::new(), outside: Vec::new() }
    }
}

impl<V: Clone> StoredTable<V> {
    /// Empty, over the record's nodes `nodes`, its room kept.
    pub fn reset(&mut self, nodes: (u32, u32)) {
        self.lo = nodes.0;
        self.dense.clear();
        self.dense.resize(nodes.1 as usize, None);
        self.outside.clear();
    }

    #[inline]
    fn slot(&self, e: TExprId) -> Option<usize> {
        let i = e.0.wrapping_sub(self.lo) as usize;
        (i < self.dense.len()).then_some(i)
    }

    #[inline]
    pub fn get(&self, e: TExprId) -> Option<&V> {
        match self.slot(e) {
            Some(i) => self.dense[i].as_ref(),
            None => self.outside.iter().find(|(k, _)| *k == e).map(|(_, v)| v),
        }
    }

    #[inline]
    pub fn contains(&self, e: TExprId) -> bool {
        self.get(e).is_some()
    }

    pub fn insert(&mut self, e: TExprId, v: V) {
        match self.slot(e) {
            Some(i) => self.dense[i] = Some(v),
            None => match self.outside.iter_mut().find(|(k, _)| *k == e) {
                Some(entry) => entry.1 = v,
                None => self.outside.push((e, v)),
            },
        }
    }

    pub fn remove(&mut self, e: TExprId) -> Option<V> {
        match self.slot(e) {
            Some(i) => self.dense[i].take(),
            None => {
                let at = self.outside.iter().position(|(k, _)| *k == e)?;
                Some(self.outside.remove(at).1)
            }
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (TExprId, &V)> + '_ {
        let lo = self.lo;
        self.dense.iter().enumerate().filter_map(move |(i, v)| v.as_ref().map(|v| (TExprId(lo + i as u32), v))).chain(self.outside.iter().map(|(e, v)| (*e, v)))
    }
}

/// The types of the nodes of a copy, dense from the first node it made (`base`), the nodes
/// before it (an argument's, the receiver's) kept aside.
#[derive(Default)]
pub struct CopyTypes {
    base: u32,
    dense: Vec<TypeId>,
    below: FxMap<TExprId, TypeId>,
}

impl CopyTypes {
    /// Empty, for a copy whose nodes begin at `base`, its room kept.
    pub fn reset(&mut self, base: u32, nodes: usize) {
        self.base = base;
        self.dense.clear();
        self.dense.reserve(nodes);
        self.below = FxMap::default();
    }

    #[inline]
    pub fn get(&self, e: TExprId) -> Option<TypeId> {
        match e.0.checked_sub(self.base) {
            Some(i) => self.dense.get(i as usize).copied().filter(|&t| t != crate::tir::NO_TYPE),
            None => self.below.get(&e).copied(),
        }
    }

    #[inline]
    pub fn insert(&mut self, e: TExprId, t: TypeId) {
        match e.0.checked_sub(self.base) {
            // A copy's nodes are made in order, each typed as it is made: the next one is pushed.
            Some(i) if i as usize == self.dense.len() => self.dense.push(t),
            Some(i) => {
                let i = i as usize;
                if i > self.dense.len() {
                    self.dense.resize(i + 1, crate::tir::NO_TYPE);
                }
                self.dense[i] = t;
            }
            None => {
                self.below.insert(e, t);
            }
        }
    }
}

/// What a copy of a stored inline body reads beside the tree: the record's types of its nodes,
/// what stands for `This` (the receiver's proxy or its argument) and the path each class's
/// `C.this` in a type is seen from.
#[derive(Clone, Copy)]
pub struct StoredCopy<'c> {
    /// The record's nodes as a copy reads them: their types and marks (`StoredIndex::node`).
    pub index: &'c super::inline_definition::StoredIndex,
    pub this: Option<TExprId>,
    pub this_paths: &'c [(ClassId, TypeId)],
    /// The nodes the expansion made for its bindings, from the first to the last (the
    /// parameters' and the receiver's proxies): new, they carry no mark and no deferred record.
    pub bound: (u32, u32),
    /// For a copy the walk demands part by part, the stored body's reducible matches: the
    /// branches of every `if` and the guards and bodies of those matches' cases stay the stored
    /// nodes in the copy's tree, each copied when the walk selects it (`Worker::demand_roots`).
    pub leaves_for_demand: Option<&'c FxMap<TExprId, ()>>,
    /// The classes the stored body makes, which a copy copies where it makes them.
    pub classes: &'c [TClass],
    /// Whether the copies of those classes take the output's names (the walk's) or none (the
    /// census's), which consumes none of the counters of the names.
    pub names_classes: bool,
}

struct Copier<'c> {
    holes: &'c FxMap<SymId, TExprId>,
    params: &'c FxMap<SymId, TExprId>,
    subst: &'c Subst,
    /// The binders of the quotes being run, each with the fresh local of this run.
    renames: &'c FxMap<SymId, SymId>,
    /// The terms whose paths the copied types name anew, each with the path standing for it.
    paths: &'c [(SymId, TypeId)],
    /// The members and parameters of the anonymous classes copied for this run, each with its
    /// fresh symbol.
    extra: FxMap<SymId, SymId>,
    /// The anonymous classes of the quote, each with its copy for this run.
    classes: FxMap<ClassId, ClassId>,
    /// The locals a copied class captures beyond those of the original: what the filled holes
    /// and the arguments of the macro's parameters name at the site.
    added: FxMap<ClassId, Vec<SymId>>,
    /// While a class body is copied, the locals its copy refers to.
    locals: Option<Vec<SymId>>,
    /// Under the capture, each node copied with its copy: what a record naming another node
    /// (`capture::BuiltinCall`) of the copy names instead.
    copies: Option<FxMap<TExprId, TExprId>>,
    /// Under the capture, every copy made, with what it copies: a tree put for two holes is
    /// copied twice, each copy with records of its own.
    occurrences: Vec<(TExprId, TExprId)>,
    /// Under the capture, each pattern copied with its copy, and the trees put for holes and
    /// parameters, for the check of the copy (`Worker::capture_check_copy`).
    pat_copies: Vec<(TPatId, TPatId)>,
    filled: Vec<TExprId>,
    /// The node being copied can carry a chain of `+` on: set by its arm of `expr` once the
    /// children are copied, read where the copy is added, which takes the node's marks then.
    /// A local of `expr` costs the copy of every node a register saved and restored, 0.15% of
    /// the instructions of a program of macros, and a function that sets this 1%.
    carries_chain: bool,
    /// For a stored inline body's copy, what it made of each node (`instantiate_tree_mapped`).
    map: Option<CopyMap<'c>>,
    stored: Option<StoredCopy<'c>>,
    /// Whether the node being copied is one the walk visits, where a copy on demand leaves the
    /// branches and the reduced cases' parts for their demands: not a test's value nor a
    /// splice's code, which the walk does not enter.
    deferring: bool,
    /// How many bodies of classes the stored body makes enclose the node being copied.
    class_depth: u32,
    /// The receiver each of those classes suspended and the frames of the place that creates it,
    /// for the arguments an anonymous class passes its superclass, which that place evaluates.
    suspended: Vec<(Option<TExprId>, usize)>,
    /// The copies of the lists being copied, innermost last (`Copier::list`).
    scratch: Scratch,
    /// Whether the patterns being copied are a reducible match's, which a copy on demand copies
    /// for the walk's reduction alone and never for the output: their tests are the stored ones,
    /// and a pattern the copy leaves as it was is the stored one (`Copier::pat`).
    reducing: bool,
}

/// The stacks the copies of lists, statements, cases and patterns are gathered on before they
/// are interned, each list's at the end while it is copied.
#[derive(Default)]
struct Scratch {
    exprs: Vec<TExprId>,
    stmts: Vec<TStmt>,
    cases: Vec<TCase>,
    pats: Vec<TPatId>,
}

/// The classes the type `t` names, appended to `out`.
fn classes_named(types: &TypeStore, t: TypeId, out: &mut Vec<ClassId>) {
    match types.get(t) {
        Type::Class(c, args) => {
            out.push(c);
            for &a in types.items(args) {
                classes_named(types, a, out);
            }
        }
        Type::This(c) | Type::Ctor(c) => out.push(c),
        Type::AppParam(_, args) | Type::Alias(_, args) => {
            for &a in types.items(args) {
                classes_named(types, a, out);
            }
        }
        Type::Lambda(_, body) => classes_named(types, body, out),
        Type::Poly(ps, body) => {
            for &b in types.poly_bounds(ps) {
                classes_named(types, b, out);
            }
            classes_named(types, body, out);
        }
        Type::Union(a, b) | Type::Inter(a, b) => {
            classes_named(types, a, out);
            classes_named(types, b, out);
        }
        _ => {}
    }
}

impl<'c> Copier<'c> {
    /// A type of the tree copied, in the reader's view (a quote or a body the loader typed
    /// records the base's), with the substitution and the paths applied.
    fn ty(&self, t: &mut Worker, ty: TypeId) -> TypeId {
        self.ty_under(t, ty, self.subst)
    }

    /// `ty` with `subst` for the copy's type arguments.
    fn ty_under(&self, t: &mut Worker, ty: TypeId, subst: &Subst) -> TypeId {
        let ty = t.types.import(ty);
        let mut ty = t.types.subst(ty, subst);
        if self.stored.is_some() && !self.classes.is_empty() {
            ty = t.with_class_copies(ty, &self.classes);
        }
        if !t.types.has_paths(ty) {
            return ty;
        }
        if !self.paths.is_empty() {
            ty = t.subst_paths(ty, self.paths);
        }
        if let Some(stored) = self.stored {
            for &(c, path) in stored.this_paths.iter().rev() {
                ty = t.as_seen_from(ty, path, c);
            }
        }
        ty
    }

    /// The type of the node `e` as recorded: the record's for a stored body's node, the
    /// program's otherwise, the declared type of a local an argument's node names.
    fn recorded(&self, t: &Worker, e: TExprId) -> Option<TypeId> {
        self.recorded_at(t, e, self.stored.and_then(|s| s.index.node(e)))
    }

    /// Whether `e` is a node the expansion made for its bindings (`StoredCopy::bound`).
    #[inline]
    fn bound_node(&self, e: TExprId) -> bool {
        self.stored.map_or(false, |s| e.0 >= s.bound.0 && e.0 < s.bound.1)
    }

    /// `recorded` of `e`, the record's node `node` where it is one.
    #[inline]
    fn recorded_at(&self, t: &Worker, e: TExprId, node: Option<StoredNode>) -> Option<TypeId> {
        if self.stored.is_none() {
            return t.prog.type_of(e);
        }
        // The record's types are those of its range's nodes alone.
        if let Some(n) = node.filter(|n| n.has(StoredNode::TYPED)) {
            return Some(n.ty);
        }
        if let Some(ty) = t.prog.type_of(e) {
            return Some(ty);
        }
        match t.prog.expr(e) {
            TExpr::Local(s) => t.syms.sym(s).sig.as_ref().filter(|sig| sig.tparams.is_empty() && sig.clauses.is_empty()).map(|sig| sig.ret),
            _ => None,
        }
    }

    /// The type of a copy: the program's where it keeps types, the map's for a stored body.
    fn copy_type(&self, t: &Worker, id: TExprId) -> Option<TypeId> {
        match &self.map {
            Some(map) if self.stored.is_some() => map.types.get(id),
            _ => t.prog.type_of(id),
        }
    }

    fn set_copy_type(&mut self, t: &mut Worker, id: TExprId, ty: TypeId) {
        t.prog.set_type(id, ty);
        if let (Some(map), Some(_)) = (&mut self.map, self.stored) {
            map.types.insert(id, ty);
        }
    }

    /// A quote of a stored body (in a splice's code, or in the body), whose record (`TQuote`, the
    /// `$quote` call's first argument) names the definition's type parameters, parameters' paths
    /// and binders: a record of the copy's own, its type and its body's types in the copy's terms,
    /// its body naming the copy's binders (the parameters, the holes, the quote's own binders and
    /// the type parameters whose `Type` values it takes stay, which each run fills, renames and
    /// gives the values the copy's arguments of the `$quote` call evaluate to), so that the run
    /// that reads it builds the expansion's tree (`showType[List[Int]]` shows `List[Int]`, a
    /// quote of `inline def twice[T: Type](e: Expr[T])` called with a macro's `T` that macro's).
    fn stored_quote(&mut self, t: &mut Worker, stored_args: crate::ast::ListRef, args: crate::ast::ListRef) -> crate::ast::ListRef {
        let items: Vec<TExprId> = t.prog.expr_list(args).to_vec();
        let Some(&first) = items.first() else { return args };
        let TExpr::Int(index) = t.prog.expr(first) else { return args };
        let q = t.prog.quotes[index as usize].clone();
        let subst: Subst = self.subst.iter().filter(|&&(p, _)| !q.types.iter().any(|&(k, _)| k == p)).copied().collect();
        // The body names what stands for each parameter in the copy, its proxy, which a run
        // takes the argument for where the proxy stands for its argument (`MacroCtx::params`), as
        // a quote the retype path types at the call does; a using parameter's proxy is the given
        // the definition's search found.
        let by_name: Vec<SymId> = self.params.iter().filter(|&(&p, _)| t.syms.sym(p).by_name).filter_map(|(_, &node)| match t.prog.expr(node) {
            TExpr::Local(proxy) => Some(proxy),
            _ => None,
        }).collect();
        // The locals the quote defines, fresh for the copy, their signatures in the copy's terms
        // as its trees are (`val y: x.type = x` a `y: O.type` for the argument `O`, which a
        // macro reads through `ValDef.tpt`); each run renames these afresh.
        let mut renames: FxMap<SymId, SymId> = self.renames.clone();
        let mut paths: Vec<(SymId, TypeId)> = self.paths.to_vec();
        let mut binders: Vec<SymId> = Vec::with_capacity(q.binders.len());
        let mut binder_paths: Vec<(SymId, TypeId)> = Vec::with_capacity(q.binders.len());
        for &b in &q.binders {
            let fresh = t.clone_local(b);
            // A by-name parameter of a method the quote defines stays one, as its signature says.
            let by_name = t.syms.sym(b).by_name;
            t.syms.sym_mut(fresh).by_name = by_name;
            renames.insert(b, fresh);
            let path = t.types.mk(Type::Term(fresh));
            binder_paths.push((b, path));
            paths.push((b, path));
            binders.push(fresh);
        }
        for &fresh in &binders {
            let mut sig = (*t.sig_of(fresh)).clone();
            for clause in &mut sig.clauses {
                for p in &mut clause.params {
                    p.sym = renames.get(&p.sym).copied().unwrap_or(p.sym);
                    let ty = self.ty_under(t, p.ty, &subst);
                    p.ty = t.subst_paths(ty, &binder_paths);
                }
            }
            let ret = self.ty_under(t, sig.ret, &subst);
            sig.ret = t.subst_paths(ret, &binder_paths);
            t.syms.sym_mut(fresh).sig = Some(Arc::new(sig));
        }
        let receiver = self.stored.and_then(|s| s.this);
        let body = q.body.map(|b| {
            let none = FxMap::default();
            let copied = t.instantiate_tree(b, &none, self.params, &subst, &renames, &paths);
            t.read_by_name_params(copied, &by_name);
            // The quote's `this` is the method's class's, which the receiver stands for where the
            // body expands (`'x` of a field `x` of the class reads the receiver's).
            if let Some(receiver) = receiver {
                let this_nodes: Vec<TExprId> = t.prog.descendants(copied).filter(|&e| matches!(t.prog.expr(e), TExpr::This)).collect();
                for node in this_nodes {
                    let id = self.expr(t, receiver);
                    t.prog.exprs[node.idx()] = t.prog.expr(id);
                    if let Some(ty) = t.prog.type_of(id) {
                        t.prog.set_type(node, ty);
                    }
                }
            }
            copied
        });
        let ty = self.ty_under(t, q.ty, &subst);
        let quotes = q.quotes.map(|e| self.expr(t, e));
        let copied = t.prog.quotes.len();
        t.prog.quotes.push(TQuote { body, ty, binders, quotes, ..q });
        let mut items = items;
        items[0] = t.prog.add(TExpr::Int(copied as i32));
        if let (Some(map), Some(&stored_index)) = (&mut self.map, t.prog.expr_list(stored_args).first()) {
            map.exprs.insert(stored_index, items[0]);
            map.part.copied.push(stored_index);
        }
        t.prog.list(&items)
    }

    /// The body of `c` where it is a class the stored body makes (`InlineDefinition::classes`).
    fn stored_class_body(&self, c: ClassId) -> Option<&'c TClass> {
        self.stored?.classes.iter().find(|tc| tc.id == c)
    }

    /// Whether `c` is a class of the stored body without a body of its own, the companion of
    /// one that has a body: a local enum's companion, which holds the values of its cases.
    fn bodiless_stored(&self, t: &Worker, c: ClassId) -> bool {
        self.stored.is_some() && self.stored_class_body(c).is_none() && t.syms.class(c).companion.map_or(false, |k| self.stored_class_body(k).is_some())
    }

    /// The class whose copy, with its graph, copies `c`, where `c` is a class of the stored body.
    fn stored_trigger(&self, t: &Worker, c: ClassId) -> Option<ClassId> {
        if self.stored.is_none() {
            return None;
        }
        if self.stored_class_body(c).is_some() {
            return Some(c);
        }
        self.bodiless_stored(t, c).then(|| t.syms.class(c).companion).flatten()
    }

    /// The copy of the stored body's class `c`, copied with its graph if it is not yet.
    fn stored_class_copy(&mut self, t: &mut Worker, c: ClassId) -> ClassId {
        if let Some(&nc) = self.classes.get(&c) {
            return nc;
        }
        if let Some(k) = self.stored_trigger(t, c) {
            self.stored_class(t, k);
        }
        self.classes.get(&c).copied().unwrap_or(c)
    }

    /// Whether the copy leaves the branches of an `if` and, for a stored reducible match, its
    /// cases' guards and bodies for their demands.
    fn leaves(&self, e: TExprId, reducible_only: bool) -> bool {
        if !self.deferring {
            return false;
        }
        match self.stored.and_then(|s| s.leaves_for_demand) {
            Some(reducible) => !reducible_only || reducible.contains_key(&e),
            None => false,
        }
    }

    /// The stored node `e` left in the copy's tree for a demand.
    fn leave(&mut self, e: TExprId) -> TExprId {
        if let Some(map) = &mut self.map {
            map.part.left.push(e);
        }
        e
    }

    /// A member a copied node selects: the copy's own, of a stored body's class and of a quote's
    /// anonymous class alike, which the class file's references name (`Class.member`; the
    /// JavaScript output selects by name and never told them apart).
    fn member_ref(&self, s: SymId) -> SymId {
        self.extra.get(&s).copied().unwrap_or(s)
    }

    fn sym(&self, s: SymId) -> SymId {
        match self.extra.get(&s) {
            Some(&n) => n,
            None => self.renames.get(&s).copied().unwrap_or(s),
        }
    }

    fn note_local(&mut self, s: SymId) {
        if let Some(locals) = &mut self.locals {
            if !locals.contains(&s) {
                locals.push(s);
            }
        }
    }

    /// An anonymous class of the quote is a class of its own at each run, as scalac makes one
    /// from the unpickled tree: its members and their bodies copied with the holes filled and the
    /// type parameters substituted, and the locals of the site that the filled holes name added
    /// to what it captures.
    fn class(&mut self, t: &mut Worker, c: ClassId) -> ClassId {
        // The walk does not enter a class's bodies: they are copied whole.
        let deferring = std::mem::replace(&mut self.deferring, false);
        let nc = self.class_now(t, c);
        self.deferring = deferring;
        nc
    }

    fn class_now(&mut self, t: &mut Worker, c: ClassId) -> ClassId {
        if let Some(&nc) = self.classes.get(&c) {
            return nc;
        }
        let info = t.syms.class(c).info.clone();
        // Named by the outermost site of the expansion that runs the quote, as the anonymous
        // classes an inline body makes are (`anon.rs`), so that the name is the site's whatever
        // else expanded before it. The copies of one class under one site are numbered from
        // the second on, in the order the site's expansion makes them.
        let site = t.inline.sites.first().map(|s| (s.file, s.span.start));
        let repeat = {
            let n = t.quote.copies.entry((c, site)).or_insert(0);
            *n += 1;
            *n
        };
        let mut name = t.name_str(info.name);
        match site {
            Some((file, start)) => {
                name.push_str(&format!("${}", t.prog.position(file, start)));
                if repeat > 1 {
                    name.push_str(&format!("${}", repeat));
                }
            }
            // No run of a quote is known to come without a site; its copies would be numbered.
            None => name.push_str(&format!("${}", repeat)),
        }
        let name = t.interner.intern(&name);
        // Made at the site, whose module holds it and whose typing again makes it anew.
        let nc = match t.prog_index.class(&t.prog, c) {
            Some(index) => {
                let tc = t.prog.classes[index].clone();
                self.copy_class(t, c, name, tc)
            }
            None => {
                let nc = t.syms.new_class(name, info.kind, info.mods, info.owner, info.file, info.def, info.span);
                t.mark_inner_class(nc);
                self.classes.insert(c, nc);
                nc
            }
        };
        t.note_made_at(nc, info.span, false);
        nc
    }

    /// A class the stored body of an inline method makes, copied for the expansion: named as
    /// the retype path names the class it types there (`Worker::anon_name`, the class's span in
    /// the definition's file, the expansion's outermost site, the repetition in the order the
    /// expansion makes them), or none of the output's names for a copy that is no part of it
    /// (the census's), recorded as made at the site; its bodies copied with `this` its own, the
    /// class's frame around them, so that a class it makes in turn is named inside it. What it
    /// captures is the walk's to settle (`Worker::finish_class_copy`).
    fn stored_class(&mut self, t: &mut Worker, c: ClassId) -> ClassId {
        if let Some(&nc) = self.classes.get(&c) {
            return nc;
        }
        let stored = self.stored.expect("a stored body's copy");
        // A class nested in another the body makes is copied with it.
        if let Owner::Class(o) = t.syms.class(c).owner {
            if let Some(k) = self.stored_trigger(t, o).filter(|_| !self.classes.contains_key(&o)) {
                self.stored_class(t, k);
                if let Some(&nc) = self.classes.get(&c) {
                    return nc;
                }
            }
        }
        // The graph the copy of `c` names: the classes of the body it extends and those nested
        // in any of them, each given its identity before any is filled, so that their members,
        // parents and owners name the copies.
        let graph = self.stored_graph(t, c);
        for &k in &graph {
            let info = t.syms.class(k).info.clone();
            let name = match (stored.names_classes, info.kind) {
                (true, ClassKind::Anon) => t.anon_name(info.span),
                // An enum's case is named inside its companion's copy, which tells the copies
                // apart, by its own name, which its values print.
                (true, ClassKind::EnumCase) => info.name,
                (true, _) => t.local_class_name(info.span, info.name),
                (false, ClassKind::Anon) => t.interner.intern("$stored"),
                (false, _) => info.name,
            };
            let owner = match info.owner {
                Owner::Class(o) => Owner::Class(self.classes.get(&o).copied().unwrap_or(o)),
                other => other,
            };
            let nk = t.syms.new_class(name, info.kind, info.mods, owner, info.file, info.def, info.span);
            t.mark_inner_class(nk);
            self.classes.insert(k, nk);
        }
        // The members of each, and the local that stands for an owner's `C.this` in the classes
        // nested in it, the copy's own, which those copies capture and read: what the bodies
        // copied next name.
        for &k in &graph {
            let nk = self.classes[&k];
            let members: Vec<SymId> = {
                let info = t.syms.class(k);
                info.member_order.iter().chain(&info.extensions).copied().collect()
            };
            for m in members {
                self.member(t, m, nk);
            }
            if let Some(&old) = t.outer_this.get(&k) {
                let fresh = t.outer_this_sym(nk);
                self.extra.insert(old, fresh);
            }
        }
        let receiver = self.stored.as_mut().and_then(|s| s.this.take());
        self.class_depth += 1;
        self.suspended.push((receiver, t.env.frames.len()));
        for &k in &graph {
            let nk = self.classes[&k];
            let Some(ktc) = self.stored_class_body(k).cloned() else {
                self.copy_info(t, k, nk);
                continue;
            };
            let frame = t.env.frames.len();
            let mut owners = Vec::new();
            let mut at = t.syms.class(nk).owner;
            while let Owner::Class(o) = at {
                owners.push(o);
                at = t.syms.class(o).owner;
            }
            t.env.frames.extend(owners.into_iter().rev().map(super::Frame::Class));
            t.env.frames.push(super::Frame::Class(nk));
            self.fill_class(t, k, nk, ktc);
            t.env.frames.truncate(frame);
        }
        self.suspended.pop();
        self.class_depth -= 1;
        if let Some(s) = self.stored.as_mut() {
            s.this = receiver;
        }
        for &k in &graph {
            let nk = self.classes[&k];
            if let Some(accessor) = t.outer_accessors.get(&k).and_then(|a| self.extra.get(a)).copied() {
                t.outer_accessors.insert(nk, accessor);
            }
            if stored.names_classes {
                t.note_made_at(nk, t.syms.class(k).span, false);
            }
            if t.sam_classes.contains_key(&k) {
                t.sam_classes.insert(nk, ());
            }
            t.anon_envs.insert(nk, Arc::new(t.env.clone()));
            if let Some(map) = &mut self.map {
                map.part.classes.push((k, nk));
            }
        }
        self.classes[&c]
    }

    /// The classes of the stored body the copy of `c` has to copy with it, none copied yet: the
    /// classes it extends, outermost first, then `c`, then the classes nested in any of those,
    /// each after its owner, with the classes those extend before them.
    fn stored_graph(&self, t: &Worker, c: ClassId) -> Vec<ClassId> {
        let mut graph: Vec<ClassId> = Vec::new();
        let mut pending = vec![c];
        while let Some(k) = pending.pop() {
            if graph.contains(&k) || self.classes.contains_key(&k) {
                continue;
            }
            // What `k` extends among the body's classes, its superclass chain and the traits it
            // mixes in, farthest first.
            let mut chain = vec![k];
            let bases: Vec<ClassId> = t.syms.class(k).base_types.iter().map(|&(b, _)| b).collect();
            for &b in bases.iter().skip(1) {
                if self.stored_class_body(b).is_some() && !self.classes.contains_key(&b) && !graph.contains(&b) && !chain.contains(&b) {
                    chain.push(b);
                }
            }
            for &x in chain.iter().rev() {
                if !graph.contains(&x) {
                    graph.push(x);
                }
            }
            if let Some(stored) = self.stored {
                for tc in stored.classes.iter().rev() {
                    if chain.iter().any(|&x| t.syms.class(tc.id).owner == Owner::Class(x)) {
                        pending.push(tc.id);
                    }
                }
            }
            // A companion, and a class of the body a member's signature names (`object P { def
            // make(x: Int): P }`), whose copy that signature names.
            let mut named: Vec<ClassId> = Vec::new();
            for &x in &chain {
                let info = t.syms.class(x);
                named.extend(info.companion);
                for &m in info.member_order.iter().chain(&info.extensions) {
                    if let Some(sig) = t.syms.sym(m).sig.as_deref() {
                        for &ty in sig.clauses.iter().flat_map(|c| c.params.iter().map(|p| &p.ty)).chain(std::iter::once(&sig.ret)) {
                            classes_named(&t.types, ty, &mut named);
                        }
                    }
                }
            }
            for n in named.into_iter().rev() {
                if (self.stored_class_body(n).is_some() || self.bodiless_stored(t, n)) && !graph.contains(&n) && !self.classes.contains_key(&n) {
                    pending.push(n);
                }
            }
        }
        graph
    }

    /// The arguments of the creation of `nc`, the copy of an anonymous class a stored body makes,
    /// whose stored creation passes what the class captured at the definition, then its parent's
    /// arguments: those the copy of the class holds (`Worker::anon_parent_args`), the one copy
    /// the creation evaluates, after the captures copied where the copy is made whole. A copy on
    /// demand leaves the captures uncopied (counted as pruned), the walk making the creation
    /// anew with what the copy captures (`Worker::finish_class_copy`).
    fn stored_anon_args(&mut self, t: &mut Worker, nc: ClassId, args: crate::ast::ListRef) -> crate::ast::ListRef {
        let items: Vec<TExprId> = t.prog.expr_list(args).to_vec();
        let parent: Vec<TExprId> = t.anon_parent_args.get(&nc).map(|p| t.prog.expr_list(p.args).to_vec()).unwrap_or_default();
        let own = items.len().saturating_sub(parent.len());
        let mut copied: Vec<TExprId> = Vec::with_capacity(items.len());
        if self.deferring {
            let left: u64 = items[..own].iter().map(|&x| t.prog.descendants(x).count() as u64).sum();
            super::substitution::counts::pruned(left);
        } else {
            copied.extend(items[..own].iter().map(|&x| self.expr(t, x)));
        }
        copied.extend(parent);
        t.prog.list(&copied)
    }

    /// The list `l` copied as at the place that creates the stored class being copied, whose
    /// `this` is the receiver or the enclosing class's.
    fn outside_class(&mut self, t: &mut Worker, l: crate::ast::ListRef) -> crate::ast::ListRef {
        let (receiver, frames) = self.suspended.last().copied().unwrap_or((None, t.env.frames.len()));
        if let Some(s) = self.stored.as_mut() {
            s.this = receiver;
        }
        // In the frames of the place that creates the class, as the retype path types the
        // arguments before it enters the body: a class made among them is named there.
        let inside = t.env.frames.split_off(frames.min(t.env.frames.len()));
        self.class_depth -= 1;
        let copied = self.list(t, l);
        self.class_depth += 1;
        t.env.frames.extend(inside);
        if let Some(s) = self.stored.as_mut() {
            s.this = None;
        }
        copied
    }

    /// The class `c` copied under the name `name`, its body `tc`.
    fn copy_class(&mut self, t: &mut Worker, c: ClassId, name: Name, tc: TClass) -> ClassId {
        let info = t.syms.class(c).info.clone();
        let nc = t.syms.new_class(name, info.kind, info.mods, info.owner, info.file, info.def, info.span);
        t.mark_inner_class(nc);
        self.fill_class(t, c, nc, tc)
    }

    /// The copy `nc` of the class `c` given its members, its info in the run's types and its
    /// body `tc` copied.
    fn fill_class(&mut self, t: &mut Worker, c: ClassId, nc: ClassId, tc: TClass) -> ClassId {
        let info = t.syms.class(c).info.clone();
        self.copy_info(t, c, nc);
        self.fill_body(t, c, nc, tc, &info)
    }

    /// The description of the copy `nc` of the class `c`: its members, parents, base types and
    /// links, in the copy's terms.
    fn copy_info(&mut self, t: &mut Worker, c: ClassId, nc: ClassId) {
        let info = t.syms.class(c).info.clone();
        self.classes.insert(c, nc);
        let mut members: FxMap<Name, SymId> = FxMap::default();
        let mut member_order = Vec::with_capacity(info.member_order.len());
        for &s in &info.member_order {
            let ns = self.member(t, s, nc);
            members.insert(t.syms.sym(ns).name, ns);
            member_order.push(ns);
        }
        let extensions: Vec<SymId> = info.extensions.iter().map(|&s| self.member(t, s, nc)).collect();
        let givens: Vec<SymId> = info.givens.iter().map(|&s| self.sym(s)).collect();
        let parents: Vec<TypeId> = info.parents.iter().map(|&p| self.ty(t, p)).collect();
        let base_types: Vec<(ClassId, TypeId)> =
            info.base_types.iter().map(|&(k, ty)| (if k == c { nc } else { self.classes.get(&k).copied().unwrap_or(k) }, self.ty(t, ty))).collect();
        let this_type = info.this_type.map(|x| self.ty(t, x));
        let declared_self = info.declared_self.map(|x| self.ty(t, x));
        // A stored body's object, enum and case class keep their links in the copies: the
        // companion, the enum's cases and their values, the lazy val of a local object.
        let class_of = |k: ClassId| self.classes.get(&k).copied().unwrap_or(k);
        let links = self.stored.is_some().then(|| {
            (
                info.companion.map(class_of),
                info.children.iter().map(|&k| class_of(k)).collect::<Vec<ClassId>>(),
                info.singleton.map(|s| self.sym(s)),
                info.module_sym.map(|s| self.sym(s)),
                info.local_module.map(|s| self.sym(s)),
                info.inner_object.map(|s| self.sym(s)),
            )
        });
        let mut copy = t.syms.class_mut(nc);
        if let Some((companion, children, singleton, module_sym, local_module, inner_object)) = links {
            copy.companion = companion;
            copy.children = children;
            copy.singleton = singleton;
            copy.module_sym = module_sym;
            copy.local_module = local_module;
            copy.inner_object = inner_object;
            copy.ordinal = info.ordinal;
            copy.value_class = info.value_class;
        }
        copy.parents = parents;
        let superclass = info.superclass.map(|k| self.classes.get(&k).copied().unwrap_or(k));
        copy.superclass = superclass;
        copy.nested = info.nested.iter().map(|(&n, &k)| (n, self.classes.get(&k).copied().unwrap_or(k))).collect();
        copy.members = members;
        copy.member_order = member_order;
        copy.extensions = extensions;
        copy.givens = givens;
        copy.base_types = base_types;
        copy.this_type = this_type;
        copy.declared_self = declared_self;
        copy.self_alias = info.self_alias;
        copy.state().set(Completion::Done);
        copy.js = info.js;
        copy.has_overloads = info.has_overloads;
        copy.has_statements = info.has_statements;
        copy.stateful = info.stateful;
        drop(copy);
        if let Some(sup) = superclass.filter(|&k| Some(k) != info.superclass && !t.syms.class(k).subclasses.contains(&nc)) {
            t.syms.class_mut(sup).subclasses.push(nc);
        }
    }

    /// The body `tc` of the class `c` (described by `info`) copied as the body of `nc`.
    fn fill_body(&mut self, t: &mut Worker, c: ClassId, nc: ClassId, tc: TClass, info: &ClassInfo) -> ClassId {
        // A named local class a stored body makes takes its constructor's clauses, as the class
        // file's constructor is declared from them.
        if self.stored.map_or(false, |s| s.classes.iter().any(|k| k.id == c)) && !info.ctor.is_empty() {
            let ctor: Vec<ClauseSig> = info
                .ctor
                .iter()
                .map(|clause| {
                    let mut clause = clause.clone();
                    for p in &mut clause.params {
                        p.sym = self.sym(p.sym);
                        p.ty = self.ty(t, p.ty);
                    }
                    clause
                })
                .collect();
            let ctor_syms: Vec<Vec<SymId>> = info.ctor_syms.iter().map(|l| l.iter().map(|&p| self.sym(p)).collect()).collect();
            let mut copy = t.syms.class_mut(nc);
            copy.ctor = ctor;
            copy.ctor_syms = ctor_syms;
        }
        let outer = self.locals.replace(Vec::new());
        let ctor_params: Vec<SymId> = tc.ctor_params.iter().map(|&p| self.sym(p)).collect();
        let ctor_defaults: Vec<Option<TExprId>> = tc.ctor_defaults.iter().map(|d| d.map(|e| self.expr(t, e))).collect();
        let created_outside = self.class_depth > 0 && info.kind == ClassKind::Anon;
        let parent_args = tc.parent_args.map(|l| if created_outside { self.outside_class(t, l) } else { self.list(t, l) });
        let parent_prelude = self.stmts(t, tc.parent_prelude);
        let init: Vec<TInit> = tc
            .init
            .iter()
            .map(|i| match *i {
                TInit::Field(s, e) => TInit::Field(self.sym(s), self.expr(t, e)),
                TInit::Stmt(e) => TInit::Stmt(self.expr(t, e)),
                TInit::Parent(k, call) => TInit::Parent(k, self.parent_call(t, call)),
            })
            .collect();
        let methods: Vec<FunId> = tc.methods.iter().map(|&f| self.fun(t, f)).collect();
        let ctors: Vec<FunId> = tc.ctors.iter().map(|&f| self.fun(t, f)).collect();
        let named = std::mem::replace(&mut self.locals, outer).unwrap_or_default();
        let mut bound: Vec<SymId> = ctor_params.clone();
        for &f in methods.iter().chain(&ctors) {
            let (params, body, defaults) = {
                let fun = &t.prog.funs[f.idx()];
                (fun.params.clone(), fun.body, fun.defaults.clone())
            };
            bound.extend(params);
            if let Some(b) = body {
                t.tree_binders(b, &mut bound);
            }
            for d in defaults.into_iter().flatten() {
                t.tree_binders(d, &mut bound);
            }
        }
        for i in &init {
            if let TInit::Field(_, e) | TInit::Stmt(e) = *i {
                t.tree_binders(e, &mut bound);
            }
        }
        // A stored body's class captures what the walk finds it reads (`Worker::settle_class_body`).
        let stored = self.stored.map_or(false, |s| s.classes.iter().any(|k| k.id == c));
        let added: Vec<SymId> = if stored { Vec::new() } else { named.into_iter().filter(|s| !bound.contains(s) && t.syms.sym(*s).owner == Owner::Local).collect() };
        let mut all_params = ctor_params;
        all_params.extend(added.iter().copied());
        let mut all_defaults = ctor_defaults;
        all_defaults.extend(added.iter().map(|_| None));
        t.prog.classes.push(TClass {
            id: nc,
            ctor_params: all_params.clone(),
            ctor_defaults: all_defaults,
            captures: tc.captures,
            parent_args,
            parent_via: tc.parent_via.map(|s| self.sym(s)),
            parent_prelude,
            inherited_case_members: tc.inherited_case_members,
            init,
            methods,
            ctors,
            forwarders: tc.forwarders.iter().map(|&(a, b)| (self.sym(a), self.sym(b))).collect(),
            super_accessors: tc.super_accessors.iter().map(|sa| SuperAccessor { of_trait: sa.of_trait, member: self.sym(sa.member), target: sa.target.map(|s| self.sym(s)) }).collect(),
            bridges: tc.bridges.iter().map(|&(a, b)| (self.sym(a), self.sym(b))).collect(),
            deferred_givens: tc.deferred_givens.iter().map(|&(a, b)| (self.sym(a), self.sym(b))).collect(),
        });
        if t.forked && t.tclass_settling_scopes > 0 && !t.prog.classes.alloc_shared {
            t.tclasses_settling.push((t.prog.classes.len() - 1) as u32);
        }
        t.class_done.insert(nc, ());
        t.anon_captures.insert(nc, all_params);
        if let Some(call) = t.anon_parent_args.get(&c).copied() {
            // A stored body's class passes its parent the one copy of the arguments its record
            // holds (`TClass::parent_args` is the same list), which its creation evaluates.
            let copied = match parent_args.filter(|_| stored) {
                Some(args) => ParentCall { prelude: self.stmts(t, call.prelude), args, via: call.via.map(|s| self.sym(s)) },
                None => self.parent_call(t, call),
            };
            t.anon_parent_args.insert(nc, copied);
        }
        self.added.insert(nc, added);
        nc
    }

    /// A member of a copied class: the same definition under the copy, its signature in the
    /// run's types, with fresh parameters.
    fn member(&mut self, t: &mut Worker, s: SymId, nc: ClassId) -> SymId {
        if let Some(&ns) = self.extra.get(&s).filter(|&&ns| t.syms.sym(ns).owner == Owner::Class(nc)) {
            return ns;
        }
        let mut info = t.syms.sym(s).info.clone();
        info.owner = Owner::Class(nc);
        // An enum's value and an object's val name their class, which has its copy.
        info.kind = match info.kind {
            SymKind::EnumValue(k) => SymKind::EnumValue(self.classes.get(&k).copied().unwrap_or(k)),
            SymKind::Object(k) => SymKind::Object(self.classes.get(&k).copied().unwrap_or(k)),
            other => other,
        };
        if let Some(sig) = &info.sig {
            let mut sig = (**sig).clone();
            for clause in &mut sig.clauses {
                for p in &mut clause.params {
                    let np = t.clone_local(p.sym);
                    t.subst_local_type(np, self.subst);
                    self.extra.insert(p.sym, np);
                    p.sym = np;
                    p.ty = self.ty(t, p.ty);
                }
            }
            sig.ret = self.ty(t, sig.ret);
            info.sig = Some(Arc::new(sig));
        }
        let ns = SymId(t.syms.syms.len() as u32);
        t.syms.syms.push(info);
        t.syms.sym_cells.set(ns.0, Completion::Done);
        self.extra.insert(s, ns);
        ns
    }

    fn fun(&mut self, t: &mut Worker, f: FunId) -> FunId {
        let (sym, params, defaults, body) = {
            let fun = &t.prog.funs[f.idx()];
            (fun.sym, fun.params.clone(), fun.defaults.clone(), fun.body)
        };
        let defaults = defaults.into_iter().map(|d| d.map(|x| self.expr(t, x))).collect();
        let body = body.map(|b| self.expr(t, b));
        let params = params.into_iter().map(|p| self.sym(p)).collect();
        t.prog.add_fun(TFun { sym: self.sym(sym), params, defaults, body })
    }

    fn stmts(&mut self, t: &mut Worker, l: crate::ast::ListRef) -> crate::ast::ListRef {
        if l.len == 0 {
            return l;
        }
        let items: Vec<TStmt> = t.prog.stmts[l.range()].to_vec();
        let copied: Vec<TStmt> = items.into_iter().map(|s| self.stmt(t, s)).collect();
        t.prog.stmts.push_slice(&copied)
    }

    fn parent_call(&mut self, t: &mut Worker, call: ParentCall) -> ParentCall {
        ParentCall { prelude: self.stmts(t, call.prelude), args: self.list(t, call.args), via: call.via.map(|s| self.sym(s)) }
    }

    fn syms(&self, t: &mut Worker, l: crate::ast::ListRef) -> crate::ast::ListRef {
        if self.renames.is_empty() {
            return l;
        }
        let items: Vec<SymId> = t.prog.sym_list(l).iter().map(|&s| self.sym(s)).collect();
        t.prog.syms(&items)
    }

    #[inline]
    fn expr(&mut self, t: &mut Worker, e: TExprId) -> TExprId {
        let node = self.stored.and_then(|s| s.index.node(e));
        let id = self.expr_node(t, e, node);
        if self.map.is_some() {
            self.mapped(t, e, id, node);
        }
        id
    }

    /// What a reducible match's pattern holds or reaches beyond patterns and tests (a value, an
    /// extractor's call, a class of the body its test copies), copied as any part is: the
    /// patterns of its own matches its own, never shared with the record.
    fn outside_reduction<R>(&mut self, copy: impl FnOnce(&mut Self) -> R) -> R {
        let reducing = std::mem::replace(&mut self.reducing, false);
        let r = copy(self);
        self.reducing = reducing;
        r
    }

    /// `id`, the copy of `e` (the record's node `node` where it is one), kept in the map with the
    /// marks of `e`.
    #[cold]
    fn mapped(&mut self, t: &mut Worker, e: TExprId, id: TExprId, node: Option<StoredNode>) {
        if let Some(map) = &mut self.map {
            map.exprs.insert(e, id);
            map.part.copied.push(e);
        }
        // A record's node carries the marks its record read once, a node of the expansion's
        // bindings none; any other node is read live.
        let marks = match node {
            Some(n) => {
                debug_assert_eq!(n.flags & StoredNode::MARKS, t.output_marks(e), "the marks of the stored node {} moved since its record's index", e.0);
                n.flags & StoredNode::MARKS
            }
            None if self.bound_node(e) => {
                debug_assert_eq!(t.output_marks(e), 0, "a binding's node {} with marks", e.0);
                0
            }
            None => t.output_marks(e),
        };
        if marks == 0 {
            return;
        }
        if marks & StoredNode::LEAF != 0 {
            t.prog.note_leaf(id);
        }
        if marks & StoredNode::EXPANSION != 0 {
            match t.prog.expansions.get(&e) {
                Some(&x) => t.prog.note_expansion(id, x),
                None => t.prog.mark_expansion(id),
            }
        }
        if marks & StoredNode::EVALUATED != 0 {
            t.inline.evaluated.insert(id, ());
        }
        if marks & StoredNode::INTERPOLATION != 0 {
            t.interpolations.insert(id, ());
        }
        if marks & StoredNode::SOFT != 0 {
            t.soft_exprs.insert(id, ());
        }
    }

    fn expr_node(&mut self, t: &mut Worker, e: TExprId, node: Option<StoredNode>) -> TExprId {
        let recorded = self.recorded_at(t, e, node);
        let copied = match t.prog.expr(e) {
            TExpr::This if self.stored.map_or(false, |s| s.this.is_some()) => {
                let receiver = self.stored.and_then(|s| s.this).expect("the receiver");
                let id = self.expr(t, receiver);
                if t.capturing() {
                    self.put_for(t, e, receiver, id);
                }
                return id;
            }
            TExpr::Local(s) => {
                if let Some(&filled) = self.holes.get(&s).or_else(|| self.params.get(&s)) {
                    let id = self.expr(t, filled);
                    if t.prog.ends_chain(e) {
                        t.prog.mark_chain_end(id);
                    }
                    if t.capturing() {
                        self.put_for(t, e, filled, id);
                    }
                    return id;
                }
                let s = self.sym(s);
                self.note_local(s);
                TExpr::Local(s)
            }
            // A value of an enum the stored body defines, its companion and a class it names stand
            // for their copies (the enum and its cases copied with the graph).
            TExpr::Static(s) if matches!(t.syms.sym(s).owner, Owner::Class(o) if self.stored_trigger(t, o).is_some()) => {
                let Owner::Class(o) = t.syms.sym(s).owner else { unreachable!() };
                self.stored_class_copy(t, o);
                TExpr::Static(self.member_ref(s))
            }
            TExpr::Module(c) if self.stored_trigger(t, c).is_some() => TExpr::Module(self.stored_class_copy(t, c)),
            TExpr::ClassOf(c) if self.stored_trigger(t, c).is_some() => TExpr::ClassOf(self.stored_class_copy(t, c)),
            TExpr::Int(_) | TExpr::Long(_) | TExpr::Double(_) | TExpr::Bool(_) | TExpr::Char(_) | TExpr::Str(_) | TExpr::Unit | TExpr::This | TExpr::Super(_) | TExpr::Static(_) | TExpr::Module(_) | TExpr::JsImport(_) | TExpr::JsGlobal(..) | TExpr::Null | TExpr::ClassOf(_) => t.prog.expr(e),
            TExpr::Field(r, s) => {
                let r = self.expr(t, r);
                TExpr::Field(r, self.member_ref(s))
            }
            TExpr::CallStatic(s, args) => {
                let s = self.sym(s);
                self.note_local(s);
                TExpr::CallStatic(s, self.list(t, args))
            }
            TExpr::CallMethod(r, s, args) => {
                let r = self.expr(t, r);
                let s = self.member_ref(s);
                TExpr::CallMethod(r, s, self.list(t, args))
            }
            TExpr::CallClosure(f, args) => {
                let f = self.expr(t, f);
                TExpr::CallClosure(f, self.list(t, args))
            }
            TExpr::New(c, args) if self.stored_class_body(c).is_some() => {
                let nc = self.stored_class(t, c);
                let l = if t.syms.class(nc).kind == ClassKind::Anon { self.stored_anon_args(t, nc, args) } else { self.list(t, args) };
                let id = t.prog.add(TExpr::New(nc, l));
                let ty = t.types.class(nc, &[]);
                t.prog.set_type(id, ty);
                t.prog.copy_span(e, id);
                if t.capturing() {
                    self.captured(t, e, id);
                    self.copy_named(t, e, id);
                }
                return id;
            }
            TExpr::New(c, args) if t.syms.class(c).kind == ClassKind::Anon => {
                let nc = self.class(t, c);
                let count = t.anon_captures.get(&c).map_or(0, |v| v.len());
                let mut items: Vec<TExprId> = t.prog.expr_list(args).to_vec();
                let mut copied: Vec<TExprId> = items.drain(..count.min(items.len())).map(|x| self.expr(t, x)).collect();
                for &s in self.added.get(&nc).cloned().unwrap_or_default().iter() {
                    copied.push(t.prog.add(TExpr::Local(s)));
                }
                copied.extend(items.into_iter().map(|x| self.expr(t, x)));
                let l = t.prog.list(&copied);
                let id = t.prog.add(TExpr::New(nc, l));
                let ty = t.types.class(nc, &[]);
                t.prog.set_type(id, ty);
                t.prog.copy_span(e, id);
                if t.capturing() {
                    self.captured(t, e, id);
                    self.copy_named(t, e, id);
                }
                return id;
            }
            TExpr::New(c, args) => TExpr::New(c, self.list(t, args)),
            TExpr::NewVia(s, args) => TExpr::NewVia(s, self.list(t, args)),
            TExpr::Lambda(params, body) => {
                let params = self.syms(t, params);
                TExpr::Lambda(params, self.expr(t, body))
            }
            TExpr::If(c, a, b) => {
                let c = self.expr(t, c);
                let (a, b) = if self.leaves(e, false) {
                    (self.leave(a), b.map(|x| self.leave(x)))
                } else {
                    (self.expr(t, a), b.map(|x| self.expr(t, x)))
                };
                self.carries_chain = true;
                TExpr::If(c, a, b)
            }
            TExpr::While(c, b) => {
                let c = self.expr(t, c);
                TExpr::While(c, self.expr(t, b))
            }
            TExpr::Block(stmts, res) => {
                let mark = self.scratch.stmts.len();
                for i in stmts.range() {
                    let st = t.prog.stmts[i];
                    let copy = self.stmt(t, st);
                    self.scratch.stmts.push(copy);
                }
                let l = t.prog.stmts.push_slice(&self.scratch.stmts[mark..]);
                self.scratch.stmts.truncate(mark);
                let res = self.expr(t, res);
                self.carries_chain = true;
                TExpr::Block(l, res)
            }
            TExpr::Assign(a, b) => {
                let a = self.expr(t, a);
                TExpr::Assign(a, self.expr(t, b))
            }
            TExpr::Match(scrut, cases) => {
                let scrut = self.expr(t, scrut);
                let leaves = self.leaves(e, true);
                TExpr::Match(scrut, self.cases(t, cases, leaves))
            }
            TExpr::Prim(op, a, b) => {
                let a = self.expr(t, a);
                TExpr::Prim(op, a, self.expr(t, b))
            }
            TExpr::Unary(op, a) => TExpr::Unary(op, self.expr(t, a)),
            TExpr::StrConcat(l) => {
                let l = self.list(t, l);
                self.carries_chain = true;
                TExpr::StrConcat(l)
            }
            TExpr::ToStr(a, k) => TExpr::ToStr(self.expr(t, a), k),
            TExpr::Js(s, stored_args) => {
                let args = self.list(t, stored_args);
                match self.stored {
                    Some(_) if t.prog.strings[s.idx()] == "$quote" => TExpr::Js(s, self.stored_quote(t, stored_args, args)),
                    _ => TExpr::Js(s, args),
                }
            }
            TExpr::TypeTest(a, test) => {
                let a = self.expr(t, a);
                TExpr::TypeTest(a, self.test(t, test))
            }
            TExpr::Cast(a, op, to) => {
                let a = self.expr(t, a);
                let to = self.ty(t, to);
                let op = match op {
                    CastOp::Check(test, erased) => CastOp::Check(self.test(t, test), self.ty(t, erased)),
                    CastOp::Unbox(test, erased) => CastOp::Unbox(self.test(t, test), self.ty(t, erased)),
                    CastOp::Written | CastOp::Nothing => op,
                };
                // A quote's cast is decided where the quote runs, its holes filled (a stored
                // body's where the walk has its receiver).
                match op {
                    CastOp::Written if self.stored.is_none() => {
                        let from = self.copy_type(t, a).unwrap_or(ANY);
                        match t.cast_lowering(from, to, false) {
                            super::prims::CastLowering::Op(op) if !matches!((op, t.prog.expr(a)), (CastOp::Unbox(..), TExpr::Null)) => TExpr::Cast(a, op, to),
                            // The tree that fills a hole takes the cast's type where its own
                            // does not conform to it (an abstract type member's cast its erasure
                            // makes the value itself).
                            lowering => {
                                let (id, ty) = t.lower_cast_as(a, from, to, lowering);
                                if id != a {
                                    t.prog.copy_span(e, id);
                                    self.set_copy_type(t, id, ty);
                                } else if !t.is_sub(from, ty) {
                                    self.set_copy_type(t, id, ty);
                                }
                                return id;
                            }
                        }
                    }
                    _ => TExpr::Cast(a, op, to),
                }
            }
            TExpr::SeqLit(l) => TExpr::SeqLit(self.list(t, l)),
            TExpr::ArrayLit(l) => TExpr::ArrayLit(self.list(t, l)),
            TExpr::Index(a, i) => TExpr::Index(self.expr(t, a), i),
            TExpr::JsSelect(a, n) => TExpr::JsSelect(self.expr(t, a), n),
            TExpr::ObjLit(l) => TExpr::ObjLit(self.list(t, l)),
            TExpr::Spread(a) => TExpr::Spread(self.expr(t, a)),
            TExpr::Return(a) => TExpr::Return(self.expr(t, a)),
            TExpr::Throw(a, wraps) => TExpr::Throw(self.expr(t, a), wraps),
            TExpr::Splice(a) => {
                let deferring = std::mem::replace(&mut self.deferring, false);
                let a = self.expr(t, a);
                self.deferring = deferring;
                TExpr::Splice(a)
            }
            TExpr::Try(i) => {
                let (body, cases, finalizer, wraps) = {
                    let tr = &t.prog.tries[i as usize];
                    (tr.body, tr.cases, tr.finalizer, tr.wraps)
                };
                let body = self.expr(t, body);
                let cases = self.cases(t, cases, false);
                let finalizer = finalizer.map(|f| self.expr(t, f));
                t.prog.tries.push(TTry { body, cases, finalizer, wraps });
                TExpr::Try(t.prog.tries.len() as u32 - 1)
            }
        };
        // A block that ends in a hole has the type of the tree that fills it, as scalac types
        // the spliced code.
        let from_result = match (t.prog.expr(e), copied) {
            (TExpr::Block(_, r0), TExpr::Block(_, r1)) if recorded.is_some() && recorded == self.recorded(t, r0) => self.copy_type(t, r1),
            _ => None,
        };
        let id = t.prog.add(copied);
        if matches!(copied, TExpr::This) && self.class_depth > 0 {
            if let Some(map) = &mut self.map {
                map.part.inner_this.push(id);
            }
        }
        if let Some(ty) = from_result {
            self.set_copy_type(t, id, ty);
        } else if let Some(ty) = recorded {
            // A record's type that names nothing a copy moves is the copy's as it is.
            let ty = match node {
                Some(n) if n.has(StoredNode::CLOSED) => t.types.import(ty),
                _ => self.ty(t, ty),
            };
            self.set_copy_type(t, id, ty);
        }
        t.prog.copy_span(e, id);
        if t.capturing() {
            self.captured(t, e, id);
        }
        if self.carries_chain {
            self.carries_chain = false;
            self.keep_chain_marks(t, e, id);
        }
        let deferred = match node {
            Some(n) => {
                debug_assert_eq!(n.has(StoredNode::DEFERRED), t.quote.deferred.contains_key(&e), "the deferred record of the stored node {} moved since its record's index", e.0);
                n.has(StoredNode::DEFERRED).then(|| t.quote.deferred.get(&e).cloned()).flatten()
            }
            None if self.bound_node(e) => {
                debug_assert!(!t.quote.deferred.contains_key(&e), "a binding's node {} with a deferred record", e.0);
                None
            }
            None => t.quote.deferred.get(&e).cloned(),
        };
        if let Some(d) = deferred {
            // The expansion takes the node of the call, with the end an ascription around the
            // call put on it.
            t.prog.copy_chain_marks(e, id);
            let subst = |t: &mut Worker, s: &Subst| -> Subst { s.iter().map(|&(p, ty)| (p, self.ty(t, ty))).collect() };
            let copy = DeferredInline {
                sym: d.sym,
                owner_subst: subst(t, &d.owner_subst),
                prefix: d.prefix.map(|x| self.ty(t, x)),
                sig: crate::symbols::sig_mapped(&d.sig, &mut |x| t.types.import(x)),
                subst: subst(t, &d.subst),
                ret_ty: self.ty(t, d.ret_ty),
                span: d.span,
                expected: d.expected.map(|x| self.ty(t, x)),
                at_end: d.at_end,
            };
            t.quote.deferred.insert(id, Arc::new(copy));
            t.quote.copied_deferred.push(id);
        }
        if t.capturing() {
            self.copy_named(t, e, id);
        }
        id
    }

    /// The records of `to`, the copy of `from` just made, that name other nodes (a builtin
    /// call's receiver and arguments, an inline call's) name the copies of those in this copy:
    /// the ones just made of the nodes under `from`, else copies made here for the record alone,
    /// with their own records in turn. What a copy for a record makes is the capture's alone:
    /// it changes neither the locals a class being copied captures nor the chain of `+` the
    /// node heads.
    fn copy_named(&mut self, t: &mut Worker, from: TExprId, to: TExprId) {
        let Some(records) = t.prog.capture.as_deref().map(|c| c.records_of(from)) else { return };
        if records.builtin.is_none() && records.inline_calls.is_empty() && records.receiver.is_none() && records.evidence.is_empty() {
            return;
        }
        let locals = self.locals.take();
        let chain = std::mem::replace(&mut self.carries_chain, false);
        if let Some(r) = records.receiver {
            match self.copy_for_record(t, r) {
                Some(r) => t.capture_receiver(to, r),
                None => t.capture_lost(from, "receiver"),
            }
        }
        if !records.evidence.is_empty() {
            match records.evidence.iter().map(|&a| self.copy_for_record(t, a)).collect::<Option<Vec<_>>>() {
                Some(evidence) => t.capture_evidence(to, evidence),
                None => t.capture_lost(from, "evidence"),
            }
        }
        if let Some(call) = records.builtin {
            let recv = self.copy_for_record(t, call.recv);
            let args = call.args.iter().map(|&a| self.copy_for_record(t, a)).collect::<Option<Vec<_>>>();
            match (recv, args) {
                (Some(recv), Some(args)) => t.capture_builtin_call(to, call.name, recv, args),
                _ => t.capture_lost(from, "builtin call"),
            }
        }
        for mut call in records.inline_calls {
            let recv = match call.recv {
                Some(r) => self.copy_for_record(t, r).map(Some),
                None => Some(None),
            };
            let args = call.args.iter().map(|&a| self.copy_for_record(t, a)).collect::<Option<Vec<_>>>();
            let (Some(recv), Some(args)) = (recv, args) else {
                t.capture_lost(from, "inline call");
                continue;
            };
            call.recv = recv;
            call.args = args;
            call.callee = self.sym(call.callee);
            let targs = t.types.import_list(call.targs);
            let items: Vec<TypeId> = t.types.items(targs).to_vec();
            let mapped: Vec<TypeId> = items.into_iter().map(|x| self.ty(t, x)).collect();
            call.targs = t.types.list(&mapped);
            t.capture_inline_call(to, call);
        }
        self.locals = locals;
        self.carries_chain = chain;
    }

    /// The copy of `e` a record of a copy names: the copy's, else one made for the record where
    /// `e` can be copied apart.
    fn copy_for_record(&mut self, t: &mut Worker, e: TExprId) -> Option<TExprId> {
        if let Some(&c) = self.copies.as_ref().and_then(|m| m.get(&e)) {
            return Some(c);
        }
        t.copyable_for_record(e).then(|| self.expr(t, e))
    }

    /// `id`, the copy of `e`, takes its records.
    fn captured(&mut self, t: &mut Worker, e: TExprId, id: TExprId) {
        let this = &*self;
        t.capture_copy_renamed(e, id, &|w, ty| this.ty(w, ty), &|s| this.sym(s), &|c| this.classes.get(&c).copied());
        if let Some(copies) = self.copies.as_mut() {
            copies.insert(e, id);
            self.occurrences.push((e, id));
        }
    }

    /// `id`, the copy of `filled` put for the hole or the parameter `e`, takes the layers
    /// written around `e`.
    fn put_for(&mut self, t: &mut Worker, e: TExprId, filled: TExprId, id: TExprId) {
        let this = &*self;
        t.capture_copy_layers(e, id, &|w, ty| this.ty(w, ty));
        self.filled.push(filled);
        if let Some(copies) = self.copies.as_mut() {
            copies.insert(e, id);
            self.occurrences.push((e, id));
        }
    }

    /// What `e` says of the chain of `+` it heads, said of its copy `id`. A condition that a
    /// hole stood in is folded away or not by what filled the hole.
    fn keep_chain_marks(&mut self, t: &mut Worker, e: TExprId, id: TExprId) {
        t.prog.copy_chain_marks(e, id);
        if let (TExpr::If(cond, ..), None, Some(ty)) = (t.prog.expr(id), t.prog.taken(e), t.prog.type_of(id)) {
            t.mark_taken_branch(cond, id, ty);
        }
    }

    fn list(&mut self, t: &mut Worker, l: crate::ast::ListRef) -> crate::ast::ListRef {
        let mark = self.scratch.exprs.len();
        for i in l.range() {
            let x = t.prog.expr_lists[i];
            let copy = self.expr(t, x);
            self.scratch.exprs.push(copy);
        }
        let copied = t.prog.list(&self.scratch.exprs[mark..]);
        self.scratch.exprs.truncate(mark);
        copied
    }

    fn stmt(&mut self, t: &mut Worker, s: TStmt) -> TStmt {
        match s {
            TStmt::Expr(x) => TStmt::Expr(self.expr(t, x)),
            TStmt::Val(v, x) => TStmt::Val(self.sym(v), self.expr(t, x)),
            TStmt::Pat(p, x) => {
                let p = self.pat(t, p);
                TStmt::Pat(p, self.expr(t, x))
            }
            TStmt::Fun(f) => {
                let (sym, params, defaults, body) = {
                    let fun = &t.prog.funs[f.idx()];
                    (fun.sym, fun.params.clone(), fun.defaults.clone(), fun.body)
                };
                let defaults = defaults.into_iter().map(|d| d.map(|x| self.expr(t, x))).collect();
                let body = body.map(|b| self.expr(t, b));
                let params = params.into_iter().map(|p| self.sym(p)).collect();
                let nf = t.prog.add_fun(TFun { sym: self.sym(sym), params, defaults, body });
                TStmt::Fun(nf)
            }
        }
    }

    /// The cases of a match, their guards and bodies left for their demands where `leaves`.
    fn cases(&mut self, t: &mut Worker, l: crate::ast::ListRef, leaves: bool) -> crate::ast::ListRef {
        let mark = self.scratch.cases.len();
        for i in l.range() {
            let c = t.prog.cases[i];
            let pat = if leaves && !t.capturing() {
                let outer = std::mem::replace(&mut self.reducing, true);
                let pat = self.pat(t, c.pat);
                self.reducing = outer;
                pat
            } else {
                self.pat(t, c.pat)
            };
            let copy = if leaves {
                let guard = c.guard.map(|g| self.leave(g));
                TCase { pat, guard, body: self.leave(c.body) }
            } else {
                let guard = c.guard.map(|g| self.expr(t, g));
                TCase { pat, guard, body: self.expr(t, c.body) }
            };
            self.scratch.cases.push(copy);
        }
        let copied = t.prog.cases.push_slice(&self.scratch.cases[mark..]);
        self.scratch.cases.truncate(mark);
        copied
    }

    fn pat(&mut self, t: &mut Worker, p: TPatId) -> TPatId {
        let copied = match t.prog.pats[p.idx()] {
            TPat::Wildcard => return p,
            TPat::Bind(s, inner) => TPat::Bind(self.sym(s), inner.map(|i| self.pat(t, i))),
            TPat::Test(stored_test, stored_ty, stored_inner) => {
                let test = self.test(t, stored_test);
                let ty = self.ty(t, stored_ty);
                let inner = self.pat(t, stored_inner);
                if self.reducing && (test, ty, inner) == (stored_test, stored_ty, stored_inner) {
                    return p;
                }
                TPat::Test(test, ty, inner)
            }
            TPat::Equals(e, strict) => TPat::Equals(self.outside_reduction(|c| c.expr(t, e)), strict),
            TPat::Class(c, ty, fields, subs) if self.stored_trigger(t, c).is_some() => {
                let nc = self.outside_reduction(|k| k.stored_class_copy(t, c));
                let ty = self.ty(t, ty);
                let names: Vec<SymId> = t.prog.sym_list(fields).iter().map(|&s| self.member_ref(s)).collect();
                let fields = t.prog.sym_lists.push_slice(&names);
                TPat::Class(nc, ty, fields, self.pats(t, subs))
            }
            TPat::Class(c, stored_ty, fields, stored_subs) => {
                let ty = self.ty(t, stored_ty);
                let subs = self.pats(t, stored_subs);
                if self.reducing && (ty, subs) == (stored_ty, stored_subs) {
                    return p;
                }
                TPat::Class(c, ty, fields, subs)
            }
            TPat::Alt(stored) => {
                let l = self.pats(t, stored);
                if self.reducing && l == stored {
                    return p;
                }
                TPat::Alt(l)
            }
            TPat::Seq(items, rest) => {
                let items = self.pats(t, items);
                TPat::Seq(items, rest.map(|r| self.pat(t, r)))
            }
            TPat::Unapply(s, call, inner) => {
                let call = self.outside_reduction(|c| c.expr(t, call));
                TPat::Unapply(self.sym(s), call, self.pat(t, inner))
            }
        };
        let id = t.prog.add_pat(copied);
        if t.capturing() {
            let this = &*self;
            t.capture_copy_pat(p, id, &|w, ty| this.ty(w, ty));
            self.pat_copies.push((p, id));
        }
        id
    }

    fn pats(&mut self, t: &mut Worker, l: crate::ast::ListRef) -> crate::ast::ListRef {
        let mark = self.scratch.pats.len();
        let mut same = true;
        for i in l.range() {
            let x = t.prog.pat_lists[i];
            let copy = self.pat(t, x);
            same &= copy == x;
            self.scratch.pats.push(copy);
        }
        if self.reducing && same {
            self.scratch.pats.truncate(mark);
            return l;
        }
        let copied = t.prog.pat_lists.push_slice(&self.scratch.pats[mark..]);
        self.scratch.pats.truncate(mark);
        copied
    }

    fn test(&mut self, t: &mut Worker, test: TestId) -> TestId {
        // A reducible match's pattern is the reduction's, which reads no test: the stored one
        // stands where its copy would be a plain duplicate of it (`shared_test`).
        if self.reducing && self.shared_test(t, test) {
            return test;
        }
        let copy = self.test_node(t, test);
        if let Some(map) = &mut self.map {
            map.tests.insert(test, copy);
            map.part.tests_copied.push(test);
        }
        copy
    }

    /// Whether the copy of the stored test `test` would be a duplicate of it and nothing else
    /// (`test_node`'s last arms): no test the copy derives again from a type (a deferred one,
    /// which may report or make a value's node), no value it copies, no class of the body it
    /// copies, no leaf test it publishes.
    fn shared_test(&self, t: &Worker, test: TestId) -> bool {
        if t.prog.deferred_tests.contains_key(&test) || self.stored.map_or(true, |s| s.index.is_leaf_test(test)) {
            return false;
        }
        match t.prog.tests[test.idx()] {
            TypeTest::Value(_) | TypeTest::Outer(..) => false,
            TypeTest::Or(a, b) | TypeTest::And(a, b) => self.shared_test(t, a) && self.shared_test(t, b),
            TypeTest::Class(c) | TypeTest::Trait(c) => self.stored_trigger(t, c).is_none(),
            _ => true,
        }
    }

    fn test_node(&mut self, t: &mut Worker, test: TestId) -> TestId {
        if let Some(&tested) = t.prog.deferred_tests.get(&test) {
            let filled = self.ty(t, tested);
            // A stored body's test of a type variable of its patterns stays deferred on the copy's
            // fresh variable, which the case that binds it specialises (`specialise_instance`).
            if self.stored.is_some() && filled != tested && matches!(t.types.get(filled), Type::Param(_) | Type::AppParam(..)) {
                let copy = t.prog.add_test(TypeTest::Always);
                t.prog.deferred_tests.insert(copy, filled);
                return copy;
            }
            if filled != tested {
                return t.test_for(filled, ANY, Span::default(), true);
            }
            if self.map.is_none() {
                return test;
            }
            let copy = t.prog.add_test(t.prog.tests[test.idx()]);
            t.prog.deferred_tests.insert(copy, tested);
            return copy;
        }
        let copied = match t.prog.tests[test.idx()] {
            TypeTest::Value(e) => {
                let deferring = std::mem::replace(&mut self.deferring, false);
                let e = self.outside_reduction(|c| c.expr(t, e));
                self.deferring = deferring;
                TypeTest::Value(e)
            }
            TypeTest::Or(a, b) => {
                let a = self.test(t, a);
                TypeTest::Or(a, self.test(t, b))
            }
            TypeTest::And(a, b) => {
                let a = self.test(t, a);
                TypeTest::And(a, self.test(t, b))
            }
            TypeTest::Outer(accessor, inner) => TypeTest::Outer(accessor, self.test(t, inner)),
            TypeTest::Class(c) if self.stored_trigger(t, c).is_some() => TypeTest::Class(self.outside_reduction(|k| k.stored_class_copy(t, c))),
            TypeTest::Trait(c) if self.stored_trigger(t, c).is_some() => TypeTest::Trait(self.outside_reduction(|k| k.stored_class_copy(t, c))),
            other if self.map.is_some() => other,
            _ => return test,
        };
        t.prog.add_test(copied)
    }
}

/// The runs of each macro and how many of them touched state other runs share, the first
/// thing each touched: the census `TEQ_MACRO_CENSUS=1` prints at the end of the build.
/// Each row also counts the identity hashes the macro's runs asked and the ones they computed
/// (`interp::run_hashes`), every request apart.
static CENSUS: std::sync::Mutex<Vec<(String, u32, u32, Option<String>, (u64, u64))>> = std::sync::Mutex::new(Vec::new());

fn census_on() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("TEQ_MACRO_CENSUS").is_some())
}

fn census_note(name: &str, shared: Option<&str>, hashes: (u64, u64)) {
    let mut c = CENSUS.lock().unwrap_or_else(|e| e.into_inner());
    let i = match c.iter().position(|(n, ..)| n == name) {
        Some(i) => i,
        None => {
            c.push((name.to_string(), 0, 0, None, (0, 0)));
            c.len() - 1
        }
    };
    let row = &mut c[i];
    row.1 += 1;
    row.4 = (row.4 .0 + hashes.0, row.4 .1 + hashes.1);
    if let Some(what) = shared {
        row.2 += 1;
        if row.3.is_none() {
            row.3 = Some(what.to_string());
        }
    }
}

/// The census of the macros' runs (`census_note`), by macro.
pub fn print_census() {
    let mut c = std::mem::take(&mut *CENSUS.lock().unwrap_or_else(|e| e.into_inner()));
    c.sort_by(|a, b| a.0.cmp(&b.0));
    let (runs, touched, asked): (u32, u32, u64) = c.iter().fold((0, 0, 0), |(r, t, h), row| (r + row.1, t + row.2, h + row.4 .0));
    eprintln!("macro census: {} runs of {} macros, {} touched state other runs share, {} identity hashes asked", runs, c.len(), touched, asked);
    for (name, runs, touched, what, (asked, computed)) in c {
        let hashes = if asked > 0 { format!(", {} identity hashes asked, {} computed", asked, computed) } else { String::new() };
        eprintln!("  {}: {} runs, {} touched{}{}", name, runs, touched, hashes, what.map_or(String::new(), |w| format!(" ({})", w)));
    }
}
