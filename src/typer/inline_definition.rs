//! The definition check of inline methods: an inline method's body typed once, in the scope of
//! its definition, into the stored form `tir::InlineDefinition`, as scalac's
//! typer types it before any call expands it. The parameters stand as themselves at their
//! declared types. What scalac leaves for an expansion is kept rather than done: a call of an
//! inline method or of an intrinsic of `scala.compiletime` stays a call with its record among
//! the deferred calls, as in a quote (`Worker::defer_inline`); an `inline if` or `inline match`
//! is typed with all of its branches; a top-level splice is typed as the macro's code and kept
//! as `TExpr::Splice`; a class the body makes is typed and kept with the record. A body with a
//! form the check does not type is held back (`HeldForm`). An expansion copies the stored body
//! (`substitution.rs`). What the check finds is the build's, reported at the definition once,
//! and kept with the record; `TEQ_INLINE_CENSUS=<file>` appends a line per body.

use super::site::SiteOwner;
use super::{Frame, Worker};
use crate::ast::{mods, DefKind};
use crate::source::{Diagnostic, FileId, Span};
use crate::symbols::*;
use crate::tir::*;
use crate::intern::FxMap;
use crate::types::*;
use std::sync::Arc;

/// What a body under the definition check keeps for an expansion, gathered as it is typed.
pub struct Notes {
    tparams: Vec<TParamId>,
    pattern_tparams: Vec<TParamId>,
    reducible: Vec<(TExprId, ReducibleSource)>,
    deferred: Vec<TExprId>,
    type_args: Vec<(TExprId, Vec<TypeId>)>,
    splices: Vec<TExprId>,
    leaf_tests: Vec<TestId>,
    imports: Vec<BlockImport>,
    hoisted: Vec<SymId>,
    /// The imports of the blocks being typed, each with the statement it stands before, until
    /// the block's node is made.
    pending_imports: Vec<(u32, super::ResolvedImport)>,
    /// The names bound to a value's member by the statement being typed, then with the statement
    /// they stand before, until the block's node is made.
    unplaced_aliases: Vec<(SymId, TExprId)>,
    pending_aliases: Vec<(u32, SymId, TExprId)>,
    aliases: Vec<StoredAlias>,
    /// The classes the body makes, typed with it and kept apart from the program's
    /// (`InlineDefinition::classes`), and their bodies once checked.
    classes: Vec<ClassId>,
    class_bodies: Vec<TClass>,
    inline_vals: Vec<SymId>,
    inferred_vals: Vec<SymId>,
    held: Option<HeldForm>,
    /// The method, for the message a `return` in its body gets.
    pub sym: SymId,
}


impl<'a> Worker<'a> {
    /// Whether the code being typed is an inline body under the definition check, outside any
    /// expansion.
    #[inline]
    pub(super) fn checks_inline_definition(&self) -> bool {
        self.inline.checking > 0 && self.inline.depth == 0
    }

    /// Whether a call of an inline method is kept as a call with its record among the deferred
    /// calls rather than expanded: in quoted code, which expands where the quote is spliced,
    /// and in a body under the definition check.
    #[inline]
    pub(super) fn keeps_inline_calls(&self) -> bool {
        (self.quote.level > 0 && self.quote.macro_depth == 0) || self.checks_inline_definition()
    }

    /// The notes of the body under the definition check, for code of the body's own level (not
    /// a quote's inside it).
    fn level_notes(&mut self) -> Option<&mut Notes> {
        if self.checks_inline_definition() && self.quote.level == 0 {
            self.inline.notes.last_mut()
        } else {
            None
        }
    }

    pub(super) fn note_deferred(&mut self, call: TExprId) {
        if let Some(n) = self.level_notes() {
            n.deferred.push(call);
        }
    }

    /// `te`, the node an `inline if` or `inline match` of the body was typed to, written at
    /// `source`.
    pub(super) fn note_reducible(&mut self, te: TExprId, source: ReducibleSource) {
        if matches!(self.prog.expr(te), TExpr::If(..) | TExpr::Match(..)) {
            if let Some(n) = self.level_notes() {
                n.reducible.push((te, source));
            }
        }
    }

    /// The type arguments of the call `te` of a method with type parameters.
    pub(super) fn note_type_args(&mut self, te: TExprId, subst: &[(TParamId, TypeId)]) {
        if subst.is_empty() || !self.checks_inline_definition() || self.quote.level > 0 {
            return;
        }
        let targs: Vec<TypeId> = subst.iter().map(|&(_, t)| self.zonk(t)).collect();
        if let Some(n) = self.level_notes() {
            n.type_args.push((te, targs));
        }
    }

    /// A type variable that a type pattern of the body binds.
    pub(super) fn note_pattern_tparam(&mut self, p: TParamId) {
        if let Some(n) = self.level_notes() {
            n.pattern_tparams.push(p);
        }
    }

    pub(super) fn note_splice(&mut self, te: TExprId) {
        if let Some(n) = self.level_notes() {
            n.splices.push(te);
        }
    }

    /// Where the imports and the names bound to a value's member of a block that starts being
    /// typed begin among the pending ones, for a body under the definition check.
    pub(super) fn pending_imports_mark(&mut self) -> Option<(usize, usize)> {
        self.level_notes().map(|n| (n.pending_imports.len(), n.pending_aliases.len()))
    }

    /// The imports a block's statement entered, and the names it bound to a value's member,
    /// standing before its statement `at`.
    pub(super) fn note_imports(&mut self, at: u32, entered: Vec<super::ResolvedImport>) {
        if let Some(n) = self.level_notes() {
            n.pending_imports.extend(entered.into_iter().map(|i| (at, i)));
            let aliases: Vec<(SymId, TExprId)> = n.unplaced_aliases.drain(..).collect();
            n.pending_aliases.extend(aliases.into_iter().map(|(local, tree)| (at, local, tree)));
        }
    }

    /// A name an import binds to a value's member (`import v.{m as a}`): the local and the
    /// selection it stands for (`StoredAlias`).
    pub(super) fn note_value_alias(&mut self, local: SymId, tree: TExprId) {
        if let Some(n) = self.level_notes() {
            n.unplaced_aliases.push((local, tree));
        }
    }

    /// The imports and the names bound to a value's member the block `block` entered since
    /// `mark`.
    pub(super) fn note_block_imports(&mut self, block: TExprId, mark: Option<(usize, usize)>) {
        let Some((imports, aliases)) = mark else { return };
        if let Some(n) = self.level_notes() {
            if n.pending_imports.len() > imports {
                let entered: Vec<(u32, super::ResolvedImport)> = n.pending_imports.drain(imports..).collect();
                n.imports.extend(entered.into_iter().map(|(at, import)| BlockImport { block, at, import }));
            }
            if n.pending_aliases.len() > aliases {
                let bound: Vec<(u32, SymId, TExprId)> = n.pending_aliases.drain(aliases..).collect();
                n.aliases.extend(bound.into_iter().map(|(at, local, tree)| StoredAlias { block, at, local, tree }));
            }
        }
    }

    /// A temporary `h` the typing hoisted an operand of the body into.
    pub(super) fn note_hoisted(&mut self, h: SymId) {
        if let Some(n) = self.level_notes() {
            n.hoisted.push(h);
        }
    }

    /// Whether a class the code being typed makes is the stored body's own (`InlineDefinition::
    /// classes`): in a body under the definition check, at its own level.
    pub(super) fn stores_classes(&self) -> bool {
        self.checks_inline_definition() && self.quote.level == 0
    }

    /// An `inline val` of the body whose value an inline call kept for the expansion gives.
    pub(super) fn note_inline_val(&mut self, s: SymId) {
        if let Some(n) = self.level_notes() {
            n.inline_vals.push(s);
        }
    }

    /// A val of the body whose type the definition inferred from its initialiser.
    pub(super) fn note_inferred_val(&mut self, s: SymId) {
        if let Some(n) = self.level_notes() {
            n.inferred_vals.push(s);
        }
    }

    /// The class `c` made by the body under the check, kept with its record.
    pub(super) fn note_stored_class(&mut self, c: ClassId) {
        if let Some(n) = self.level_notes() {
            n.classes.push(c);
        }
    }

    /// A class nested in `outer`, a class the stored body makes, is the body's too, copied with
    /// `outer` at each expansion; a case class or a case object there is scalac's E162 in a
    /// method that is a member (`PrepareInlineable.makeInlineable` walks the whole body), as one
    /// of the body's blocks is.
    pub(super) fn note_nested_stored_class(&mut self, outer: ClassId, nested: ClassId) {
        let stored = self.level_notes().map_or(false, |n| n.classes.contains(&outer));
        if !stored {
            return;
        }
        let info = self.syms.class(nested);
        let (is_case, is_object, span, file) = (info.mods & crate::ast::mods::CASE != 0 && info.kind != ClassKind::EnumCase, info.kind == ClassKind::Object, info.span, info.file);
        let member = self.inline_under_check().map_or(false, |m| self.syms.sym(m).owner != Owner::Local);
        if is_case && member {
            let msg = if is_object {
                "Case object definitions are not allowed in inline methods or quoted code. Use a normal object instead."
            } else {
                "Case class definitions are not allowed in inline methods or quoted code. Use a normal class instead."
            };
            self.diags.error(file, span, msg);
            return;
        }
        self.note_stored_class(nested);
    }

    /// The body of a class checked: the stored body's own class's kept with the notes, where
    /// it is one (`true`).
    pub(super) fn keep_stored_class(&mut self, tclass: TClass) -> Result<(), TClass> {
        match self.inline.notes.last_mut() {
            Some(n) if n.classes.contains(&tclass.id) => {
                n.class_bodies.push(tclass);
                Ok(())
            }
            _ => Err(tclass),
        }
    }

    /// The body of the class `c` just checked, a stored class's among the notes.
    pub(super) fn checked_class_mut(&mut self, c: ClassId) -> Option<&mut TClass> {
        let stored = self.inline.notes.last().map_or(false, |n| n.classes.contains(&c));
        if stored {
            let n = self.inline.notes.last_mut().expect("the notes");
            return n.class_bodies.iter_mut().rfind(|t| t.id == c);
        }
        self.prog.classes.rfind_mut(|t| t.id == c)
    }

    /// The trees of the stored classes' bodies: their methods' and constructors' bodies and
    /// defaults, their initialisers, their constructor defaults and the arguments to their parent.
    pub(super) fn class_roots(&self, classes: &[TClass]) -> Vec<TExprId> {
        let mut out = Vec::new();
        for tc in classes {
            for &f in tc.methods.iter().chain(&tc.ctors) {
                let fun = &self.prog.funs[f.idx()];
                out.extend(fun.defaults.iter().flatten().copied());
                out.extend(fun.body);
            }
            out.extend(tc.ctor_defaults.iter().flatten().copied());
            out.extend(tc.init.iter().filter_map(|i| match *i {
                TInit::Field(_, e) | TInit::Stmt(e) => Some(e),
                TInit::Parent(..) => None,
            }));
            if let Some(l) = tc.parent_args {
                out.extend(self.prog.expr_list(l).iter().copied());
            }
        }
        out
    }

    /// A form the check does not type: the body is held back.
    pub(super) fn note_held(&mut self, form: HeldForm) {
        if self.checks_inline_definition() {
            if let Some(n) = self.inline.notes.last_mut() {
                n.held.get_or_insert(form);
            }
        }
    }

    /// A type test of `t`, which an expansion takes from its type arguments when `t` names a type
    /// parameter of the method.
    pub(super) fn note_leaf_test(&mut self, test: TestId, t: TypeId) {
        if !self.checks_inline_definition() || self.quote.level > 0 {
            return;
        }
        let Some(n) = self.inline.notes.last() else { return };
        if n.tparams.is_empty() {
            return;
        }
        let tparams = n.tparams.clone();
        if self.mentions_tparam_of(t, Some(&tparams)) {
            if let Some(n) = self.inline.notes.last_mut() {
                n.leaf_tests.push(test);
            }
        }
    }

    /// The inline method whose body is under the definition check.
    pub(super) fn inline_under_check(&self) -> Option<SymId> {
        if self.checks_inline_definition() {
            self.inline.notes.last().map(|n| n.sym)
        } else {
            None
        }
    }

    /// Whether the class `c` is defined in the body of the inline method `m` (a `super` of such a
    /// class is its own, scalac's `typedSuper` refusing any other in an inlineable method).
    pub(super) fn defined_in_body_of(&self, c: ClassId, m: SymId) -> bool {
        let (file, def) = {
            let s = self.syms.sym(m);
            (s.file, s.def)
        };
        let Some(def) = def else { return false };
        let ast = self.ast(file);
        let body = match &ast.def(def).kind {
            DefKind::Fun(f) => f.body,
            DefKind::Given(g) => g.alias,
            _ => None,
        };
        let Some(body) = body else { return false };
        let within = ast.expr_span(body);
        let info = self.syms.class(c);
        info.file == file && info.span.start >= within.start && info.span.end <= within.end
    }

    /// The definition check of the inline method `sym`, which the walk reaches in the place of
    /// typing its body: a body of the program's own files, checked once. An override of a method
    /// that is not inline (`is_retained_inline`) has two roles, kept apart: the method the walk
    /// types for dynamic dispatch, and the inline body checked here and stored for its
    /// expansions.
    pub(super) fn check_inline_definition(&mut self, sym: SymId) {
        self.outside_annotation(|t| t.check_inline_definition_now(sym))
    }

    fn check_inline_definition_now(&mut self, sym: SymId) {
        if self.syms.sym(sym).owner == Owner::Local {
            return;
        }
        let (file, owner, span) = {
            let s = self.syms.sym(sym);
            (s.file, s.owner, s.span)
        };
        let env = self.env_at(file, owner, span.start);
        self.check_definition_in(sym, env);
    }

    /// The body of a std inline method the program may never expand, typed as the definition
    /// check types a program's, for the language server's records of its document alone
    /// (`index::demand_std_document`): nothing stored, no diagnostic and no deferred check kept,
    /// so that the expansions and the builds that follow are what they were.
    pub(super) fn record_std_inline_body(&mut self, sym: SymId) {
        if self.syms.sym(sym).owner == Owner::Local || self.inline_definitions.contains_key(&sym) || self.prog.capture.is_some() {
            return;
        }
        let (file, owner, span) = {
            let s = self.syms.sym(sym);
            (s.file, s.owner, s.span)
        };
        let env = self.env_at(file, owner, span.start);
        let sig = self.sig_arc(sym);
        let marks = (self.diags.items.len(), self.deferred_matches.len(), self.deferred_bounds.len());
        let notes = Notes { sym, tparams: sig.tparams.clone(), pattern_tparams: Vec::new(), reducible: Vec::new(), deferred: Vec::new(), type_args: Vec::new(), splices: Vec::new(), leaf_tests: Vec::new(), imports: Vec::new(), hoisted: Vec::new(), pending_imports: Vec::new(), unplaced_aliases: Vec::new(), pending_aliases: Vec::new(), aliases: Vec::new(), classes: Vec::new(), class_bodies: Vec::new(), inline_vals: Vec::new(), inferred_vals: Vec::new(), held: None };
        let case_binders = std::mem::take(&mut self.case_binders);
        let nowarn = std::mem::replace(&mut self.nowarn, 0);
        self.outside_annotation(|t| {
            t.outside_search(|t| {
                t.outside_inline(|t| {
                    t.inline.checking += 1;
                    t.inline.notes.push(notes);
                    t.with_env(env, |t| t.type_inline_definition(sym, &sig));
                    t.inline.notes.pop();
                    t.inline.checking -= 1;
                })
            })
        });
        self.nowarn = nowarn;
        self.case_binders = case_binders;
        self.drop_reported_since(marks.0);
        self.deferred_matches.truncate(marks.1);
        self.deferred_bounds.truncate(marks.2);
    }

    /// The definition check of the local inline method `sym`, which a block of a body the walk
    /// types defines: in the block's scope, the enclosing body's locals and type parameters
    /// in it, and only there, not in a body being expanded or checked, which is copied whole.
    pub(super) fn check_local_inline_definition(&mut self, sym: SymId) {
        if self.inline.depth > 0 || self.inline.checking > 0 || self.quote.level > 0 {
            return;
        }
        // A call before the definition read an import across a val (`InlineState::forward_refs`);
        // the calls from here on are the definition's.
        self.inline.forward_refs.remove(&sym);
        let env = self.env.clone();
        self.check_definition_in(sym, env);
    }

    /// The definition's check is its body's typing whatever asked for it, a call's demand in a
    /// signature or under a typing that reports no warnings included: its diagnostics are the
    /// body's (`again`), which `check_definition_in_body` reads to tell a failed definition, its
    /// warnings reported, as the walk's own check reports them.
    pub(super) fn check_definition_in(&mut self, sym: SymId, env: super::Env) {
        let nowarn = std::mem::replace(&mut self.nowarn, 0);
        self.again(|t| t.check_definition_in_body(sym, env));
        self.nowarn = nowarn;
    }

    fn check_definition_in_body(&mut self, sym: SymId, env: super::Env) {
        if self.inline_definitions.contains_key(&sym) {
            return;
        }
        let (file, def_id) = {
            let s = self.syms.sym(sym);
            (s.file, s.def)
        };
        let Some(def_id) = def_id else { return };
        if !self.program_file(file) || self.is_body_file(file) {
            return;
        }
        let def = self.ast(file).def(def_id);
        let body = match &def.kind {
            DefKind::Fun(f) => f.body,
            DefKind::Given(g) => g.alias,
            _ => None,
        };
        let Some(body) = body else { return };
        // What the definition spans, its header to the end of its body: where what the check
        // reports belongs to the definition.
        let body_span = self.ast(file).expr_span(body);
        let def_span = Span { start: def.span.start.min(body_span.start), end: def.span.end.max(body_span.end) };
        let sig = self.sig_arc(sym);
        let params: Vec<SymId> = sig.clauses.iter().flat_map(|c| c.params.iter().map(|p| p.sym)).collect();
        self.inline_definitions.insert(sym, Arc::new(InlineDefinition::in_progress(params.clone())));
        let diag_mark = self.diags.items.len();
        let (matches_mark, bounds_mark) = (self.deferred_matches.len(), self.deferred_bounds.len());
        // What the check records of the body its expansions by substitution record again.
        let keeps_index = self.index.is_some();
        let index = if keeps_index { None } else { self.index.take() };
        let index_mark = self.index_mark();
        let notes = Notes { sym, tparams: sig.tparams.clone(), pattern_tparams: Vec::new(), reducible: Vec::new(), deferred: Vec::new(), type_args: Vec::new(), splices: Vec::new(), leaf_tests: Vec::new(), imports: Vec::new(), hoisted: Vec::new(), pending_imports: Vec::new(), unplaced_aliases: Vec::new(), pending_aliases: Vec::new(), aliases: Vec::new(), classes: Vec::new(), class_bodies: Vec::new(), inline_vals: Vec::new(), inferred_vals: Vec::new(), held: None };
        let case_binders = std::mem::take(&mut self.case_binders);
        // The expansion by substitution reads the type of every node of the stored body, which
        // a build that keeps no types keeps aside for the record alone.
        let aside = self.prog.keep_types_aside();
        let capture_mark = match self.prog.capture.as_deref() {
            Some(c) => c.made.get(&file).map_or(0, |keys| keys.len()),
            None => usize::MAX,
        };
        let ((body, ty, defaults), notes) = self.outside_search(|t| {
            t.outside_inline(|t| {
                t.inline.checking += 1;
                t.inline.notes.push(notes);
                let typed = t.with_env(env, |t| t.type_inline_definition(sym, &sig));
                let notes = t.inline.notes.pop().expect("the notes of the body under the check");
                t.inline.checking -= 1;
                (typed, notes)
            })
        });
        self.case_binders = case_binders;
        if keeps_index {
            self.index_keep_body(sym, index_mark);
        } else {
            self.index = index;
        }
        let held = notes.held;
        // The selections the body's blocks bind names to, and the bodies of the classes it
        // makes, are the record's trees too.
        let mut extra_roots: Vec<TExprId> = notes.aliases.iter().map(|a| a.tree).collect();
        extra_roots.extend(self.class_roots(&notes.class_bodies));
        let node_types = match body.filter(|_| held.is_none()) {
            Some(b) => self.stored_node_types(std::iter::once(b).chain(defaults.iter().flatten().copied()).chain(extra_roots.iter().copied())),
            None => FxMap::default(),
        };
        if aside {
            self.prog.end_types_aside();
        }
        if capture_mark != usize::MAX {
            if let Some(keys) = self.prog.capture.as_deref().and_then(|c| c.made.get(&file)) {
                let made: Vec<crate::tir::capture::Key> = keys.get(capture_mark..).unwrap_or(&[]).to_vec();
                self.check_capture_keys.extend(made);
            }
            // Another worker's expansion copies the stored body, whose copier reads the records
            // of its nodes as a quote's body's: published, as `type_quote` publishes those.
            if let Some(c) = self.prog.capture.as_deref_mut() {
                c.publish(file, (capture_mark, 0));
            }
        }
        // The checks the body left for the end of the typing (a match's exhaustivity, a type
        // argument's bounds) are the definition's, reported there once, called or not, as scalac
        // reports them (a plain match's in its typer, the bounds in `PostTyper`, which leaves the
        // method inline), and not at the expansions, which check neither again
        // (`match_checked_at_definition`); those of a body typed on the way stay.
        let in_definition = |f: FileId, s: Span| f == file && s.start >= def_span.start && s.end <= def_span.end;
        let mut later = self.deferred_matches.split_off(matches_mark);
        let reported = held.is_none();
        later.retain(|m| reported || !in_definition(m.file, m.span));
        for m in later.iter_mut().filter(|m| in_definition(m.file, m.span)) {
            m.definition = Some(sym);
        }
        self.deferred_matches.append(&mut later);
        let mut later = self.deferred_bounds.split_off(bounds_mark);
        let in_definition_site = |b: &super::resolve::DeferredBound| {
            let (f, start, end) = b.site();
            in_definition(f, Span { start, end })
        };
        later.retain(|b| reported || !in_definition_site(b));
        for b in later.iter_mut().filter(|b| in_definition_site(b)) {
            b.definition = Some(sym);
        }
        self.deferred_bounds.append(&mut later);
        // What the check reported leaves the list and comes back in another order: no promoted
        // range keeps a position past the mark (`state::diags_cut_at`); the demanded check's own
        // range, promoted after it, covers what stays.
        self.diags_cut_at(diag_mark);
        let reported: Vec<Diagnostic> = self.diags.items.drain(diag_mark..).collect();
        let within = |d: &Diagnostic| d.of_body && d.file == file && d.span.start >= def_span.start && d.span.end <= def_span.end;
        let (own, other): (Vec<Diagnostic>, Vec<Diagnostic>) = reported.into_iter().partition(within);
        self.diags.items.extend(other);
        // A body with an error fails, held back or not: its errors are the build's, reported here
        // once, and a call of it is the plain call. One held back without an error keeps its
        // forms' checks for the retype path, which types it at each call.
        let failed = own.iter().any(|d| !d.is_warning);
        let state = match held {
            _ if failed => DefinitionState::Failed,
            Some(form) => DefinitionState::Held(form),
            None => DefinitionState::Checked,
        };
        if held.is_none() || failed {
            self.diags.items.extend(own.iter().cloned());
        }
        let typed_body = body;
        let body = body.filter(|_| held.is_none());
        let mut notes = notes;
        let mut binders = Vec::new();
        let mut leaves = Vec::new();
        let mut widened = Vec::new();
        let mut opaque = Vec::new();
        let mut spread = Vec::new();
        let mut erased = Vec::new();
        if let Some(b) = body {
            // What the notes name of a typing the body dropped (an alternative tried and
            // discarded) is no part of it.
            let roots = self.stored_roots(std::iter::once(b).chain(defaults.iter().flatten().copied()).chain(extra_roots.iter().copied()));
            let mut nodes: FxMap<TExprId, ()> = FxMap::default();
            for &root in &roots {
                nodes.extend(self.prog.descendants(root).map(|e| (e, ())));
            }
            notes.reducible.retain(|(e, _)| nodes.contains_key(e));
            notes.deferred.retain(|e| nodes.contains_key(e));
            notes.type_args.retain(|(e, _)| nodes.contains_key(e));
            notes.splices.retain(|e| nodes.contains_key(e));
            notes.imports.retain(|i| nodes.contains_key(&i.block));
            notes.aliases.retain(|a| nodes.contains_key(&a.block));
            for &root in &roots {
                self.tree_binders(root, &mut binders);
            }
            widened = nodes.keys().copied().filter(|&e| self.prog.is_widened(e)).collect();
            widened.sort();
            opaque = nodes.keys().copied().filter(|&e| self.prog.is_opaque(e)).collect();
            opaque.sort();
            spread = nodes.keys().copied().filter(|&e| self.prog.is_spread(e)).collect();
            spread.sort();
            // The lean library's call is the intrinsic's, deferred: each expansion evaluates it anew.
            erased = nodes.keys().copied().filter(|&e| !notes.deferred.contains(&e) && self.calls_erased_value(e)).collect();
            erased.sort();
            let body_roots = self.stored_roots(std::iter::once(b));
            self.definition_leaves(&body_roots, &params, &notes.deferred, &mut leaves);
        }
        // Whatever the typing made for the body, a stored body or one held back, is no part of
        // the output: its tests and patterns are left out where the output's classes are
        // numbered from every test of the program.
        if let Some(b) = typed_body {
            for root in self.stored_roots(std::iter::once(b).chain(defaults.iter().flatten().copied()).chain(extra_roots.iter().copied())) {
                self.store_tests_and_pats(root);
            }
        }
        notes.leaf_tests.retain(|t| body.is_some() && self.prog.stored_tests.contains_key(t));
        // A search's variables may be solved after a kept call was built: its record names what
        // they were solved to, for the copies to substitute.
        if body.is_some() {
            for &e in &notes.deferred {
                self.zonk_deferred(e);
            }
        }
        let hoisted: Vec<SymId> = if body.is_some() { notes.hoisted.iter().copied().filter(|h| binders.contains(h)).collect() } else { Vec::new() };
        let record = InlineDefinition {
            state,
            body,
            ty,
            params,
            defaults: if held.is_none() { defaults } else { Vec::new() },
            binders,
            pattern_tparams: if body.is_some() { notes.pattern_tparams } else { Vec::new() },
            reducible: notes.reducible.iter().map(|(e, _)| *e).collect(),
            reducible_sources: notes.reducible.into_iter().map(|(_, s)| s).collect(),
            deferred: notes.deferred,
            // A search's variables may be solved after the call was built.
            type_args: notes.type_args.into_iter().map(|(e, ts)| (e, ts.into_iter().map(|t| self.zonk(t)).collect())).collect(),
            splices: notes.splices,
            leaves,
            leaf_tests: notes.leaf_tests,
            diagnostics: own,
            widened,
            opaque,
            spread,
            erased,
            hoisted,
            imports: if body.is_some() { notes.imports } else { Vec::new() },
            node_types,
            aliases: if body.is_some() { notes.aliases } else { Vec::new() },
            classes: if body.is_some() { notes.class_bodies } else { Vec::new() },
            inline_vals: if body.is_some() { notes.inline_vals } else { Vec::new() },
            inferred_vals: if body.is_some() { notes.inferred_vals } else { Vec::new() },
            walk_lack: None,
        };
        let mut record = record;
        if record.body.is_some() {
            record.walk_lack = self.walk_lack(&record);
        }
        if census::on() {
            let problem = self.record_problem(&record, &sig.tparams);
            census::note(self, sym, file, &record, problem);
        }
        if record.body.is_some() {
            super::substitution::counts::record_made();
        }
        self.inline_definitions.insert(sym, Arc::new(record));
    }

    /// The record of the kept call `e` with its type variables replaced by what they were solved
    /// to.
    fn zonk_deferred(&mut self, e: TExprId) {
        let Some(d) = self.quote.deferred.get(&e).cloned() else { return };
        let mut changed = false;
        let mut z = |w: &mut Self, t: TypeId| {
            let r = if w.types.has_vars(t) { w.zonk(t) } else { t };
            changed |= r != t;
            r
        };
        let owner_subst: Subst = d.owner_subst.iter().map(|&(p, t)| (p, z(self, t))).collect();
        let subst: Subst = d.subst.iter().map(|&(p, t)| (p, z(self, t))).collect();
        let prefix = d.prefix.map(|t| z(self, t));
        let ret_ty = z(self, d.ret_ty);
        let expected = d.expected.map(|t| z(self, t));
        if changed {
            let zonked = super::quoted::DeferredInline { owner_subst, subst, prefix, ret_ty, expected, ..(*d).clone() };
            self.quote.deferred.insert(e, Arc::new(zonked));
        }
    }

    /// The type of a call of `sym` whose definition failed
    /// the check: the call is not expanded, its body's errors having been reported at the
    /// definition, and it stands as a call of a method of the definition's type, as scalac drops
    /// the inline flags of a definition whose typing failed (`PrepareInlineable`): the declared
    /// result's instance at the call (`ret_ty`), or where the result was to be inferred the type
    /// the check inferred, instantiated with the call's type arguments and its arguments' paths
    /// (`inline def generic[T](x: T) = { x.noSuch; x }` called at `Int` is an `Int`), an error
    /// where that type is one (`inline def bad(x: Int) = x.noSuch`). A definition not checked
    /// yet is checked first, so that what the call gives does not depend on which of the two
    /// was typed first.
    pub(super) fn failed_definition_call(&mut self, call: &super::apply::MethodCall, subst: &Subst, args: &[TExprId], ret_ty: TypeId) -> Option<TypeId> {
        let sym = call.sym;
        if self.checks_inline_definition() || !self.is_inline_callee(sym) {
            return None;
        }
        // A local method called before its definition is checked on the call
        // (`demand_local_definition`), as any other is.
        self.demand_definition(sym);
        let record = self.inline_definitions.get(&sym)?.clone();
        if record.state != DefinitionState::Failed {
            return None;
        }
        if !self.types.contains_error(ret_ty) {
            return Some(ret_ty);
        }
        if self.types.contains_error(record.ty) {
            return Some(ERROR);
        }
        Some(self.inferred_at_call(call, subst, args, &record))
    }

    /// The type of a plain call of `sym`, a method of the program whose result is inferred,
    /// where its definition passed the check: the result the check inferred, which scalac's
    /// method type keeps (`Namer.inferredResultType`), not the expansion's own (a
    /// `constValue[ToString[N]]` body's call at `1` is a `ToString[1]`, which is `"1"`).
    pub(super) fn inferred_call_result(&mut self, call: &super::apply::MethodCall, subst: &Subst, args: &[TExprId]) -> Option<TypeId> {
        let record = self.inline_definitions.get(&call.sym)?.clone();
        if record.state != DefinitionState::Checked || self.types.contains_error(record.ty) {
            return None;
        }
        Some(self.inferred_at_call(call, subst, args, &record))
    }

    /// The type the check inferred, instantiated with the call's type arguments and its
    /// arguments' paths, reduced (`ToString[7]` the `"7"` it is).
    fn inferred_at_call(&mut self, call: &super::apply::MethodCall, subst: &Subst, args: &[TExprId], record: &InlineDefinition) -> TypeId {
        let mut subst = subst.clone();
        subst.extend(call.owner_subst.iter().copied());
        let ty = self.types.subst(record.ty, &subst);
        let paths: Vec<(SymId, TypeId)> = record.params.iter().zip(args).filter_map(|(&p, &a)| self.argument_path(a).map(|path| (p, path))).collect();
        let ty = if paths.is_empty() || !self.types.has_paths(ty) { ty } else { self.subst_paths(ty, &paths) };
        self.normalize(ty)
    }

    /// Whether the match at `span` belongs to the body of an inline method the definition check
    /// typed, being expanded: its checks are the definition's, reported there once, as scalac checks a plain match of an inline body
    /// where it is defined, called or not.
    pub(super) fn match_checked_at_definition(&self, span: Span) -> bool {
        if self.inline.depth == 0 {
            return false;
        }
        let Some(site) = self.inline.sites.last() else { return false };
        site.body_file == self.env.file
            && span.start >= site.body_span.start
            && span.end <= site.body_span.end
            && self.inline_definitions.get(&site.callee).map_or(false, |d| d.body.is_some())
    }

    /// A copy of a stored body for one expansion, through the copier of quotes: the parameters
    /// replaced by `args`, the type parameters by `subst`, every binder, every type parameter
    /// of a local method and every type variable of a type pattern renamed afresh, the calls it
    /// keeps registered for the site as a quote's are (`QuoteState::copied_deferred`). Every
    /// fresh identity is made first; then the fresh type parameters' bounds, the fresh binders'
    /// signatures (their type parameters, their parameters, its types) and the copied types are
    /// rewritten under `subst` with the fresh type parameters and with the paths of the renamed
    /// binders and of the parameters moved to what stands for them. What it made of the stored
    /// body (`Instance`): each node and test with its copy, and the record's metadata on the
    /// copy's nodes, the leaves marked as the output's (`Program::leaf_bits`, `leaf_tests`).
    /// The whole body at once: the census's copies and the capture's; the walk demands its copy
    /// part by part (`begin_demand`, `demand_roots`).
    pub(super) fn instantiate_definition(&mut self, def: &InlineDefinition, bindings: &Bindings) -> Option<Instance> {
        let body = def.body?;
        let index = Arc::new(self.stored_index(def));
        let (mut demand, mut inst) = self.begin_demand(def, index, bindings.clone(), false);
        let stored: Vec<TExprId> = std::iter::once(body).chain(def.defaults.iter().flatten().copied()).collect();
        let copies = self.settling_classes(|t| t.demand_roots(&mut demand, def, &mut inst, &stored, None));
        inst.root = copies[0];
        let mut copied_defaults = copies[1..].iter().copied();
        inst.defaults = def.defaults.iter().map(|d| d.and_then(|_| copied_defaults.next())).collect();
        Some(inst)
    }

    /// The index of the record `def` of `sym`, made on its first copy and kept while the record
    /// is the one the program holds.
    pub(super) fn stored_index_of(&mut self, sym: SymId, def: &Arc<InlineDefinition>) -> Arc<StoredIndex> {
        if let Some((of, index)) = self.inline.stored_indexes.get(&sym) {
            if Arc::ptr_eq(of, def) {
                return index.clone();
            }
        }
        let index = Arc::new(self.stored_index(def));
        self.inline.stored_indexes.insert(sym, (def.clone(), index.clone()));
        index
    }

    fn stored_index(&mut self, def: &InlineDefinition) -> StoredIndex {
        let mut meta: FxMap<TExprId, Vec<Meta>> = FxMap::default();
        let mut note = |e: TExprId, m: Meta| meta.entry(e).or_default().push(m);
        for (i, &e) in def.reducible.iter().enumerate() {
            note(e, Meta::Reducible(i as u32));
        }
        for (i, &e) in def.deferred.iter().enumerate() {
            note(e, Meta::Deferred(i as u32));
        }
        for (i, &(e, _)) in def.type_args.iter().enumerate() {
            note(e, Meta::TypeArgs(i as u32));
        }
        for (i, &e) in def.splices.iter().enumerate() {
            note(e, Meta::Splice(i as u32));
        }
        for (i, &e) in def.leaves.iter().enumerate() {
            note(e, Meta::Leaf(i as u32));
        }
        for (i, &e) in def.widened.iter().enumerate() {
            note(e, Meta::Widened(i as u32));
        }
        for (i, &e) in def.opaque.iter().enumerate() {
            note(e, Meta::Opaque(i as u32));
        }
        for (i, &e) in def.spread.iter().enumerate() {
            note(e, Meta::Spread(i as u32));
        }
        for (i, &e) in def.erased.iter().enumerate() {
            note(e, Meta::Erased(i as u32));
        }
        for (i, imp) in def.imports.iter().enumerate() {
            note(imp.block, Meta::Import(i as u32));
        }
        for (i, alias) in def.aliases.iter().enumerate() {
            note(alias.block, Meta::Alias(i as u32));
        }
        let nodes = match (def.node_types.keys().map(|e| e.0).min(), def.node_types.keys().map(|e| e.0).max()) {
            (Some(lo), Some(hi)) => (lo, hi - lo + 1),
            _ => (0, 0),
        };
        let node = self.stored_nodes(def, nodes, &meta);
        StoredIndex {
            meta,
            sizes: std::sync::Mutex::new(FxMap::default()),
            reducible: def.reducible.iter().map(|&e| (e, ())).collect(),
            leaf_tests: def.leaf_tests.iter().map(|&t| (t, ())).collect(),
            local_tparams: self.local_tparams(def),
            nodes,
            node,
            binds: !def.binders.is_empty() || !def.classes.is_empty(),
        }
    }

    /// What a copy reads of each node of the record's range (`StoredIndex::node`): its type in
    /// the record and the marks it carries, read once here rather than looked up for every copy.
    /// A stored node's marks and records are settled when its record is checked; the
    /// assertion-enabled build compares them with the live ones at each copy (`Copier::mapped`).
    fn stored_nodes(&self, def: &InlineDefinition, nodes: (u32, u32), meta: &FxMap<TExprId, Vec<Meta>>) -> Vec<StoredNode> {
        let (lo, n) = nodes;
        let mut out = vec![StoredNode { ty: NO_TYPE, flags: 0 }; n as usize];
        let stored_classes: Vec<ClassId> = def.classes.iter().flat_map(|tc| std::iter::once(tc.id).chain(self.syms.class(tc.id).companion)).collect();
        let mut closed: FxMap<TypeId, bool> = FxMap::default();
        for (&e, &ty) in &def.node_types {
            let slot = &mut out[(e.0 - lo) as usize];
            slot.ty = ty;
            slot.flags |= StoredNode::TYPED;
            let shut = match closed.get(&ty) {
                Some(&c) => c,
                None => {
                    let c = self.closed_type(ty, &stored_classes);
                    closed.insert(ty, c);
                    c
                }
            };
            if shut {
                slot.flags |= StoredNode::CLOSED;
            }
        }
        for (i, slot) in out.iter_mut().enumerate() {
            let e = TExprId(lo + i as u32);
            let marks = self.output_marks(e);
            slot.flags |= marks;
            if self.quote.deferred.contains_key(&e) {
                slot.flags |= StoredNode::DEFERRED;
            }
            if meta.contains_key(&e) {
                slot.flags |= StoredNode::META;
            }
        }
        out
    }

    /// The marks of the output the node `e` carries that a copy of it takes (`Copier::mapped`).
    pub(super) fn output_marks(&self, e: TExprId) -> u16 {
        let mut m = 0;
        if self.prog.is_leaf(e) {
            m |= StoredNode::LEAF;
        }
        if self.prog.is_expansion(e) {
            m |= StoredNode::EXPANSION;
        }
        if self.inline.evaluated.contains_key(&e) {
            m |= StoredNode::EVALUATED;
        }
        if self.interpolations.contains_key(&e) {
            m |= StoredNode::INTERPOLATION;
        }
        if self.soft_exprs.contains_key(&e) {
            m |= StoredNode::SOFT;
        }
        m
    }

    /// Whether the record's type `t` is the same in every copy's terms: it names no type
    /// parameter, no path and no class of the body (`stored`), so that no substitution, no path
    /// and no class copy moves it (`Copier::ty_under`).
    fn closed_type(&self, t: TypeId, stored: &[ClassId]) -> bool {
        let t = self.types.in_view_here(t);
        if self.types.has_paths(t) {
            return false;
        }
        let all = |l: crate::types::TList| self.types.items(l).iter().all(|&a| self.closed_type(a, stored));
        match self.types.get(t) {
            Type::Any | Type::Nothing | Type::Lit(_) => true,
            Type::Class(c, args) => !stored.contains(&c) && all(args),
            Type::Alias(_, args) => all(args),
            Type::Union(a, b) | Type::Inter(a, b) => self.closed_type(a, stored) && self.closed_type(b, stored),
            _ => false,
        }
    }

    /// The start of a copy of a stored body (`instantiate_definition`'s contract): the fresh type
    /// parameters of its local methods and of its patterns made and their bounds rewritten, and,
    /// for a copy made whole, every binder renamed. A copy `on_demand` leaves the branches of each
    /// `if` and the guards and bodies of each reduced match's cases for the walk to demand
    /// (`demand_roots`), and renames the binders of each part as the part is copied.
    pub(super) fn begin_demand(&mut self, def: &InlineDefinition, index: Arc<StoredIndex>, bindings: Bindings, on_demand: bool) -> (Demand, Instance) {
        let mut demand = Demand::empty(index.clone(), self.inline.no_subst.clone());
        let mut inst = Instance::empty();
        self.begin_demand_in(def, index, bindings, on_demand, &mut demand, &mut inst);
        (demand, inst)
    }

    /// `begin_demand` into a demand and an instance kept from an earlier copy, whose lists keep
    /// their room: every field set for this copy, the tables over the record's nodes, the maps
    /// new ones.
    pub(super) fn begin_demand_in(&mut self, def: &InlineDefinition, index: Arc<StoredIndex>, bindings: Bindings, on_demand: bool, demand: &mut Demand, inst: &mut Instance) {
        let Bindings { args, subst, paths, this, this_paths, mark_leaves } = bindings;
        let mut subst = subst;
        inst.tparams.clear();
        for &p in &index.local_tparams {
            let info = self.syms.tparam(p);
            let (name, variance, arity) = (info.name, info.variance, info.arity);
            let hk_variances = info.hk_variances.clone();
            let fresh = self.syms.new_tparam(name, variance);
            self.syms.tparams[fresh.idx()].arity = arity;
            self.syms.tparams[fresh.idx()].hk_variances = hk_variances;
            inst.tparams.push((p, fresh));
            subst.push((p, self.types.param(fresh)));
        }
        let nodes = index.nodes;
        // A body without type parameters, called without type arguments, shares the empty
        // substitution rather than making one.
        demand.subst = if subst.is_empty() { self.inline.no_subst.clone() } else { Arc::new(subst) };
        demand.args = args;
        demand.paths = paths;
        demand.this = this;
        demand.this_paths = this_paths;
        demand.mark_leaves = mark_leaves;
        demand.on_demand = on_demand;
        demand.names_classes = on_demand;
        demand.bound = (0, 0);
        demand.left.reset(nodes);
        demand.index = index;
        demand.members = FxMap::default();
        demand.copied_classes = FxMap::default();
        demand.part.start();
        demand.part.copied.reserve(nodes.1 as usize);
        demand.copied = 0;
        inst.root = TExprId(0);
        inst.defaults.clear();
        inst.renames = FxMap::default();
        inst.exprs.reset(nodes);
        inst.tests = FxMap::default();
        inst.types.reset(self.prog.exprs.len() as u32, 2 * nodes.1 as usize);
        // The record's counts size the lists, which a copy fills once each.
        macro_rules! clear_for {
            ($($list:expr => $room:expr),* $(,)?) => {$(
                $list.clear();
                $list.reserve($room);
            )*};
        }
        clear_for!(
            inst.reducible => def.reducible.len(),
            inst.reducible_at => def.reducible.len(),
            inst.this_copies => 0,
            inst.classes => 0,
            inst.deferred => def.deferred.len(),
            inst.type_args => def.type_args.len(),
            inst.splices => def.splices.len(),
            inst.leaves => def.leaves.len(),
            inst.leaf_tests => def.leaf_tests.len(),
            inst.imports => def.imports.len(),
            inst.aliases => def.aliases.len(),
        );
        let subst = demand.subst.clone();
        if !on_demand {
            self.rename_binders(demand, inst, &def.binders, &subst);
        }
        for i in 0..inst.tparams.len() {
            let (p, fresh) = inst.tparams[i];
            let (upper, lower) = (self.syms.tparam(p).upper, self.syms.tparam(p).lower);
            let (upper, lower) = (self.demand_type(demand, &subst, upper), self.demand_type(demand, &subst, lower));
            let info = &mut self.syms.tparams[fresh.idx()];
            info.upper = upper;
            info.lower = lower;
        }
    }

    /// `t` in the copy's terms: the substitution `subst` (the call's type arguments and the fresh
    /// type parameters, with a case's specialisation where the part is under one), the renamed
    /// binders' and the parameters' paths moved, `C.this` seen from the receiver's path.
    fn demand_type(&mut self, d: &Demand, subst: &Subst, t: TypeId) -> TypeId {
        // A stored body's types are its record's, a peer's or the base's where another worker or
        // the loader typed it: in this reader's view before the substitution.
        let t = self.types.in_view_here(t);
        let mut t = self.types.subst(t, subst);
        if self.types.has_paths(t) {
            t = self.subst_paths(t, &d.paths);
            for &(c, path) in d.this_paths.iter().rev() {
                t = self.as_seen_from(t, path, c);
            }
        }
        t
    }

    /// `t` with each class of a stored body that `classes` maps replaced by its copy.
    pub(super) fn with_class_copies(&mut self, t: TypeId, classes: &FxMap<ClassId, ClassId>) -> TypeId {
        let class = |c: ClassId| classes.get(&c).copied().unwrap_or(c);
        let list = |w: &mut Self, l: crate::types::TList| -> Option<crate::types::TList> {
            let items = w.types.items(l).to_vec();
            let out: Vec<TypeId> = items.iter().map(|&a| w.with_class_copies(a, classes)).collect();
            (out != items).then(|| w.types.list(&out))
        };
        match self.types.get(t) {
            Type::Class(c, args) => {
                let n = list(self, args);
                if class(c) == c && n.is_none() { t } else { self.types.mk(Type::Class(class(c), n.unwrap_or(args))) }
            }
            Type::This(c) if class(c) != c => self.types.mk(Type::This(class(c))),
            Type::Ctor(c) if class(c) != c => self.types.mk(Type::Ctor(class(c))),
            Type::AppParam(p, args) => list(self, args).map_or(t, |n| self.types.mk(Type::AppParam(p, n))),
            Type::Alias(a, args) => list(self, args).map_or(t, |n| self.types.mk(Type::Alias(a, n))),
            Type::Lambda(ps, body) => {
                let b = self.with_class_copies(body, classes);
                if b == body { t } else { self.types.mk(Type::Lambda(ps, b)) }
            }
            Type::Union(a, b) => {
                let (na, nb) = (self.with_class_copies(a, classes), self.with_class_copies(b, classes));
                if na == a && nb == b { t } else { self.types.union(na, nb) }
            }
            Type::Inter(a, b) => {
                let (na, nb) = (self.with_class_copies(a, classes), self.with_class_copies(b, classes));
                if na == a && nb == b { t } else { self.types.inter(na, nb) }
            }
            _ => t,
        }
    }

    /// The binders `binders` of the stored body, those not renamed yet, each renamed to a fresh
    /// local, their paths moved, then each fresh one's signature rewritten in the copy's terms:
    /// its type parameters the fresh ones, its parameters the renamed ones.
    fn rename_binders(&mut self, d: &mut Demand, inst: &mut Instance, binders: &[SymId], subst: &Subst) -> Vec<SymId> {
        let mut fresh: Vec<SymId> = Vec::new();
        for &b in binders {
            if inst.renames.contains_key(&b) {
                continue;
            }
            let f = self.clone_local(b);
            inst.renames.insert(b, f);
            d.paths.push((b, self.types.mk(Type::Term(f))));
            fresh.push(f);
        }
        for &f in &fresh {
            let mut sig = (*self.sig_of(f)).clone();
            for p in &mut sig.tparams {
                *p = inst.tparams.iter().find(|&&(old, _)| old == *p).map_or(*p, |&(_, new)| new);
            }
            for clause in &mut sig.clauses {
                for p in &mut clause.params {
                    p.sym = inst.renames.get(&p.sym).copied().unwrap_or(p.sym);
                    p.ty = self.demand_type(d, subst, p.ty);
                }
            }
            sig.ret = self.demand_type(d, subst, sig.ret);
            self.syms.sym_mut(f).sig = Some(Arc::new(sig));
        }
        fresh
    }

    /// The signatures of the binders `fresh` renamed for a copy, in the terms of the classes the
    /// copy made, which the renaming before it could not name.
    fn binders_in_class_copies(&mut self, fresh: &[SymId], classes: &FxMap<ClassId, ClassId>) {
        for &f in fresh {
            let mut sig = (*self.sig_of(f)).clone();
            for clause in &mut sig.clauses {
                for p in &mut clause.params {
                    p.ty = self.with_class_copies(p.ty, classes);
                }
            }
            sig.ret = self.with_class_copies(sig.ret, classes);
            self.syms.sym_mut(f).sig = Some(Arc::new(sig));
        }
    }

    /// The stored trees `roots` copied now: each either a root of the stored body or a node
    /// the copy left for a
    /// demand, copied with the specialisation of the reduced cases around it and `specialise`, a
    /// case's type variables' solutions, applied; the binders it defines renamed first; the
    /// record's metadata on its nodes published in the instance before anything in it resolves,
    /// in the record's order; the nodes it leaves for later demands kept with their
    /// specialisation. A deferred test of a type variable the specialisation fixes is derived
    /// again, a leaf test of the expansion.
    fn demand_parts(&mut self, d: &mut Demand, def: &InlineDefinition, inst: &mut Instance, roots: &[TExprId], specialise: Option<&Subst>) {
        let base = match roots.first().and_then(|&r| d.left.get(r)) {
            Some(s) => s.clone(),
            None => d.subst.clone(),
        };
        let subst: Arc<Subst> = match specialise.filter(|s| !s.is_empty()) {
            Some(spec) => Arc::new(base.iter().map(|&(p, t)| (p, self.types.subst(t, spec))).collect()),
            None => base,
        };
        for &r in roots {
            d.left.remove(r);
        }
        let mut fresh = Vec::new();
        if d.on_demand && d.index.binds {
            let mut binders = Vec::new();
            let mut seen = Vec::new();
            let index = d.index.clone();
            for &r in roots {
                self.region_binders(r, &index.reducible, &def.classes, &mut seen, &mut binders);
            }
            fresh = self.rename_binders(d, inst, &binders, &subst);
        } else if cfg!(debug_assertions) && d.on_demand {
            let (mut binders, mut seen) = (Vec::new(), Vec::new());
            for &r in roots {
                self.region_binders(r, &d.index.reducible, &def.classes, &mut seen, &mut binders);
            }
            debug_assert!(binders.is_empty(), "a binder in a body whose record lists none");
        }
        let mark = self.prog.exprs.len();
        // The classes of an expansion's copy take the output's names; one of the census or a
        // measurement takes none.
        let names_classes = d.names_classes;
        let stored_copy = super::quoted::StoredCopy { index: &d.index, this: d.this, this_paths: &d.this_paths, bound: d.bound, leaves_for_demand: d.on_demand.then_some(&d.index.reducible), classes: &def.classes, names_classes };
        d.part.start();
        let map = super::quoted::CopyMap { exprs: &mut inst.exprs, tests: &mut inst.tests, types: &mut inst.types, part: &mut d.part };
        self.instantiate_tree_mapped(roots, &d.args, &subst, &inst.renames, &d.paths, stored_copy, map, &mut d.members, &mut d.copied_classes);
        d.copied += (self.prog.exprs.len() - mark) as u64;
        if !d.copied_classes.is_empty() {
            self.binders_in_class_copies(&fresh, &d.copied_classes);
        }
        let specialised = !Arc::ptr_eq(&subst, &d.subst);
        for i in 0..d.part.left.len() {
            let l = d.part.left[i];
            d.left.insert(l, subst.clone());
        }
        d.part.copied.sort_unstable();
        d.part.copied.dedup();
        d.part.tests_copied.sort_unstable();
        d.part.tests_copied.dedup();
        self.publish_demanded(d, def, inst, &subst, specialised);
        inst.classes.extend(d.part.classes.iter().copied());
    }

    /// The copy of the stored tree `root` (`demand_parts`).
    pub(super) fn demand_root(&mut self, d: &mut Demand, def: &InlineDefinition, inst: &mut Instance, root: TExprId, specialise: Option<&Subst>) -> TExprId {
        self.demand_parts(d, def, inst, &[root], specialise);
        d.part.roots[0]
    }

    /// The copies of the stored trees `roots`, in their order (`demand_parts`).
    pub(super) fn demand_roots(&mut self, d: &mut Demand, def: &InlineDefinition, inst: &mut Instance, roots: &[TExprId], specialise: Option<&Subst>) -> Vec<TExprId> {
        self.demand_parts(d, def, inst, roots, specialise);
        d.part.roots.clone()
    }

    /// The record's metadata on the nodes a demand just copied, in the record's order: the
    /// reducible nodes with their index, the deferred calls, the calls' type arguments and the
    /// casts in the copy's terms, the splices, the leaves (marked for a copy that marks them),
    /// the leaf tests and the block imports.
    fn publish_demanded(&mut self, d: &Demand, def: &InlineDefinition, inst: &mut Instance, subst: &Subst, specialised: bool) {
        let map = &d.part;
        let mut found: Vec<(Meta, TExprId)> = Vec::new();
        let mut this_copies: Vec<TExprId> = Vec::new();
        for &e in &map.copied {
            let copy = *inst.exprs.get(e).expect("the copy of a node the part copied");
            // A node of the record's range is named by the metadata where its index says so.
            if d.index.node(e).map_or(true, |n| n.has(StoredNode::META)) {
                if let Some(ms) = d.index.meta.get(&e) {
                    found.extend(ms.iter().map(|&m| (m, copy)));
                }
            }
            if matches!(self.prog.expr(e), TExpr::This) && !map.inner_this.contains(&copy) {
                this_copies.push(copy);
            }
        }
        this_copies.sort();
        inst.this_copies.extend(this_copies);
        // The record's order, whatever the map's.
        found.sort_by_key(|&(m, _)| m);
        for (m, copy) in found {
            match m {
                Meta::Reducible(i) => {
                    inst.reducible.push(copy);
                    inst.reducible_at.push(i as usize);
                }
                Meta::Deferred(_) => inst.deferred.push(copy),
                Meta::TypeArgs(i) => {
                    let ts: Vec<TypeId> = def.type_args[i as usize].1.iter().map(|&t| self.demand_type(d, subst, t)).collect();
                    inst.type_args.push((copy, ts));
                }
                Meta::Splice(_) => inst.splices.push(copy),
                Meta::Leaf(_) => {
                    if d.mark_leaves {
                        self.prog.note_leaf(copy);
                    }
                    inst.leaves.push(copy);
                }
                Meta::Widened(_) => self.prog.mark_widened(copy),
                Meta::Opaque(_) => self.prog.mark_opaque(copy),
                Meta::Spread(_) => self.prog.mark_spread(copy),
                Meta::Erased(_) => self.note_erased_value(copy, Span::default()),
                Meta::Import(i) => {
                    let imp = def.imports[i as usize];
                    inst.imports.push(BlockImport { block: copy, ..imp });
                }
                Meta::Alias(i) => {
                    let alias = def.aliases[i as usize];
                    inst.aliases.push(StoredAlias { block: copy, ..alias });
                }
            }
        }
        let tests = &inst.tests;
        let part_tests = || map.tests_copied.iter().map(|t| (*t, tests[t]));
        let mut leaf_tests: Vec<(TestId, TestId)> = part_tests().filter(|(t, _)| d.index.leaf_tests.contains_key(t)).collect();
        leaf_tests.sort();
        for (_, c) in leaf_tests {
            self.prog.leaf_tests.insert(c, ());
            inst.leaf_tests.push(c);
        }
        // A test of a type variable a reduced case fixes, derived again from what it stands for:
        // a leaf test of the expansion, as the call's type arguments' are.
        if specialised && self.records_expansions() {
            let mut derived: Vec<TestId> = Vec::new();
            for (stored, copy) in part_tests() {
                let Some(&tested) = self.prog.deferred_tests.get(&stored) else { continue };
                let plain = self.types.subst(tested, &d.subst);
                let here = self.types.subst(tested, subst);
                if plain != here {
                    derived.push(copy);
                }
            }
            derived.sort();
            for c in derived {
                self.prog.leaf_tests.insert(c, ());
            }
        }
    }

    /// The binders the stored tree `e` defines outside the parts a copy on demand leaves (as
    /// `tree_binders` finds them): not under an `if`'s branches, nor under a reduced match's
    /// guards and bodies (`reducible`); the defaults of its local functions included.
    fn region_binders(&self, e: TExprId, reducible: &FxMap<TExprId, ()>, classes: &[TClass], seen: &mut Vec<ClassId>, out: &mut Vec<SymId>) {
        let prog = &self.prog;
        let list = |t: &Self, l: crate::ast::ListRef, seen: &mut Vec<ClassId>, out: &mut Vec<SymId>| {
            for &x in prog.expr_list(l) {
                t.region_binders(x, reducible, classes, seen, out);
            }
        };
        match prog.expr(e) {
            TExpr::Field(r, _) | TExpr::Unary(_, r) | TExpr::ToStr(r, _) | TExpr::TypeTest(r, _) | TExpr::Cast(r, ..) | TExpr::Index(r, _) | TExpr::Spread(r) | TExpr::Return(r) | TExpr::Throw(r, _) | TExpr::JsSelect(r, _) => {
                self.region_binders(r, reducible, classes, seen, out)
            }
            // A splice's code is copied whole, nothing left in it for the walk to demand.
            TExpr::Splice(_) => self.tree_binders(e, out),
            TExpr::Lambda(params, body) => {
                out.extend_from_slice(prog.sym_list(params));
                self.region_binders(body, reducible, classes, seen, out);
            }
            // A class the body makes is copied where it is made, the binders of its body with it
            // but for the parts its walk demands.
            TExpr::New(c, args) if classes.iter().any(|tc| tc.id == c) => {
                list(self, args, seen, out);
                if seen.contains(&c) {
                    return;
                }
                seen.push(c);
                for root in self.class_roots(classes.iter().filter(|tc| tc.id == c).cloned().collect::<Vec<_>>().as_slice()) {
                    self.region_binders(root, reducible, classes, seen, out);
                }
            }
            TExpr::CallStatic(_, args) | TExpr::New(_, args) | TExpr::NewVia(_, args) | TExpr::StrConcat(args) | TExpr::Js(_, args) | TExpr::SeqLit(args) | TExpr::ArrayLit(args) | TExpr::ObjLit(args) => {
                list(self, args, seen, out)
            }
            TExpr::CallMethod(r, _, args) | TExpr::CallClosure(r, args) => {
                self.region_binders(r, reducible, classes, seen, out);
                list(self, args, seen, out);
            }
            TExpr::If(c, ..) => self.region_binders(c, reducible, classes, seen, out),
            TExpr::While(a, b) | TExpr::Assign(a, b) | TExpr::Prim(_, a, b) => {
                self.region_binders(a, reducible, classes, seen, out);
                self.region_binders(b, reducible, classes, seen, out);
            }
            TExpr::Block(stmts, res) => {
                for st in &prog.stmts[stmts.range()] {
                    match *st {
                        TStmt::Expr(x) => self.region_binders(x, reducible, classes, seen, out),
                        TStmt::Val(v, x) => {
                            out.push(v);
                            self.region_binders(x, reducible, classes, seen, out);
                        }
                        TStmt::Pat(p, x) => {
                            self.pat_binders(p, out);
                            self.region_binders(x, reducible, classes, seen, out);
                        }
                        TStmt::Fun(f) => {
                            let fun = &prog.funs[f.idx()];
                            out.push(fun.sym);
                            out.extend_from_slice(&fun.params);
                            for &d in fun.defaults.iter().flatten() {
                                self.region_binders(d, reducible, classes, seen, out);
                            }
                            if let Some(b) = fun.body {
                                self.region_binders(b, reducible, classes, seen, out);
                            }
                        }
                    }
                }
                self.region_binders(res, reducible, classes, seen, out);
            }
            TExpr::Match(scrut, cases) => {
                self.region_binders(scrut, reducible, classes, seen, out);
                let leaves = reducible.contains_key(&e);
                for c in &prog.cases[cases.range()] {
                    self.pat_binders(c.pat, out);
                    if leaves {
                        continue;
                    }
                    if let Some(g) = c.guard {
                        self.region_binders(g, reducible, classes, seen, out);
                    }
                    self.region_binders(c.body, reducible, classes, seen, out);
                }
            }
            TExpr::Try(i) => {
                let t = &prog.tries[i as usize];
                self.region_binders(t.body, reducible, classes, seen, out);
                for c in &prog.cases[t.cases.range()] {
                    self.pat_binders(c.pat, out);
                    self.region_binders(c.body, reducible, classes, seen, out);
                }
                if let Some(f) = t.finalizer {
                    self.region_binders(f, reducible, classes, seen, out);
                }
            }
            _ => {}
        }
    }

    /// The path a parameter has in the types of a copy, as scalac's binding gives it
    /// (`Inliner.computeParamBindings`): the literal type of an argument that is a constant of
    /// the language (`constant`, which a value the interpreter evaluated is not), so that
    /// `constValue[x.type]` of `value(3)` is 3; a stable argument's own path (a val, a
    /// parameter); and the proxy's otherwise, the proxy typed at the argument's own type, which
    /// for an object is the object's (the object's class is no path in teq's types) and for a
    /// synthesized mirror the refinement its members' types read.
    pub(super) fn parameter_path(&mut self, arg: TExprId, constant: Option<LitVal>, proxy: SymId) -> TypeId {
        if let Some(v) = constant {
            return self.types.lit(v);
        }
        // The argument's own type can say more than its symbol's: a given a search found for a
        // mirror has the refinement the proxy keeps (`InlineState::args`), which its members'
        // types read.
        let own = self.sig_of(proxy).ret;
        let refined = matches!(self.types.get(own), Type::Refined(..));
        if let Some(path) = self.argument_path(arg).filter(|_| !refined) {
            return path;
        }
        self.types.mk(Type::Term(proxy))
    }

    /// The subtree `root` of the copy `inst` specialised by `subst`, the type variables a case of
    /// a reduced match bound: the types of its nodes, the records of its deferred calls, the type
    /// arguments of its calls, its casts, its tests (one of a variable derived again from what the
    /// variable stands for, a leaf of the expansion as the call's type arguments' are), the types
    /// of its patterns and the signatures of the binders it defines, before anything in it is
    /// resolved.
    pub(super) fn specialise_instance(&mut self, inst: &mut Instance, root: TExprId, subst: &Subst) {
        if subst.is_empty() {
            return;
        }
        let roots = self.stored_roots(std::iter::once(root));
        let nodes: Vec<TExprId> = roots.iter().flat_map(|&r| self.prog.descendants(r)).collect();
        let within: FxMap<TExprId, ()> = nodes.iter().map(|&e| (e, ())).collect();
        let mut binders = Vec::new();
        for &r in &roots {
            self.tree_binders(r, &mut binders);
        }
        let mut pats: Vec<TPatId> = Vec::new();
        for &e in &nodes {
            if let Some(t) = inst.types.get(e) {
                let s = self.types.subst(t, subst);
                if s != t {
                    inst.types.insert(e, s);
                    self.prog.set_type(e, s);
                }
            }
            if let Some(d) = self.quote.deferred.get(&e).cloned() {
                let over = |w: &mut Self, s: &Subst| -> Subst { s.iter().map(|&(p, t)| (p, w.types.subst(t, subst))).collect() };
                let moved = super::quoted::DeferredInline {
                    owner_subst: over(self, &d.owner_subst),
                    subst: over(self, &d.subst),
                    prefix: d.prefix.map(|t| self.types.subst(t, subst)),
                    ret_ty: self.types.subst(d.ret_ty, subst),
                    expected: d.expected.map(|t| self.types.subst(t, subst)),
                    ..(*d).clone()
                };
                self.quote.deferred.insert(e, Arc::new(moved));
            }
            match self.prog.expr(e) {
                TExpr::TypeTest(a, test) => {
                    let specialised = self.specialise_test(test, subst);
                    if specialised != test {
                        self.prog.exprs[e.idx()] = TExpr::TypeTest(a, specialised);
                    }
                }
                TExpr::Cast(a, op, to) => {
                    let specialised = self.types.subst(to, subst);
                    if specialised != to {
                        self.prog.exprs[e.idx()] = TExpr::Cast(a, op, specialised);
                    }
                }
                TExpr::Match(_, cases) => pats.extend(self.prog.case_list(cases).iter().map(|c| c.pat)),
                TExpr::Try(i) => pats.extend(self.prog.case_list(self.prog.tries[i as usize].cases).iter().map(|c| c.pat)),
                TExpr::Block(stmts, _) => pats.extend(self.prog.stmt_list(stmts).iter().filter_map(|s| match *s {
                    TStmt::Pat(p, _) => Some(p),
                    _ => None,
                })),
                _ => {}
            }
        }
        for (e, ts) in inst.type_args.iter_mut() {
            if within.contains_key(e) {
                for t in ts.iter_mut() {
                    *t = self.types.subst(*t, subst);
                }
            }
        }
        while let Some(p) = pats.pop() {
            let specialised = match self.prog.pats[p.idx()] {
                TPat::Test(test, t, inner) => {
                    pats.push(inner);
                    TPat::Test(self.specialise_test(test, subst), self.types.subst(t, subst), inner)
                }
                TPat::Class(c, t, fields, subs) => {
                    pats.extend_from_slice(self.prog.pat_list(subs));
                    TPat::Class(c, self.types.subst(t, subst), fields, subs)
                }
                other => {
                    match other {
                        TPat::Bind(_, inner) => pats.extend(inner),
                        TPat::Unapply(_, _, inner) => pats.push(inner),
                        TPat::Alt(subs) => pats.extend_from_slice(self.prog.pat_list(subs)),
                        TPat::Seq(items, rest) => {
                            pats.extend_from_slice(self.prog.pat_list(items));
                            pats.extend(rest);
                        }
                        _ => {}
                    }
                    continue;
                }
            };
            self.prog.pats[p.idx()] = specialised;
        }
        for b in binders {
            let mut sig = (*self.sig_of(b)).clone();
            for clause in &mut sig.clauses {
                for p in &mut clause.params {
                    p.ty = self.types.subst(p.ty, subst);
                    let own = (*self.sig_of(p.sym)).clone();
                    let ret = self.types.subst(own.ret, subst);
                    if ret != own.ret {
                        self.syms.sym_mut(p.sym).sig = Some(Arc::new(MethodSig { ret, ..own }));
                    }
                }
            }
            sig.ret = self.types.subst(sig.ret, subst);
            self.syms.sym_mut(b).sig = Some(Arc::new(sig));
        }
    }

    /// A test of a specialised case: one of a type variable derived again from what `subst`
    /// makes of it, a leaf test of the expansion.
    fn specialise_test(&mut self, test: TestId, subst: &Subst) -> TestId {
        if let Some(&tested) = self.prog.deferred_tests.get(&test) {
            let filled = self.types.subst(tested, subst);
            if filled == tested {
                return test;
            }
            let derived = if matches!(self.types.get(filled), Type::Param(_) | Type::AppParam(..)) {
                let copy = self.prog.add_test(TypeTest::Always);
                self.prog.deferred_tests.insert(copy, filled);
                copy
            } else {
                self.test_for(filled, ANY, Span::default(), true)
            };
            if self.records_expansions() {
                self.prog.leaf_tests.insert(derived, ());
            }
            return derived;
        }
        match self.prog.tests[test.idx()] {
            TypeTest::Or(a, b) | TypeTest::And(a, b) => {
                let (x, y) = (self.specialise_test(a, subst), self.specialise_test(b, subst));
                if (x, y) == (a, b) {
                    return test;
                }
                let combined = match self.prog.tests[test.idx()] {
                    TypeTest::Or(..) => TypeTest::Or(x, y),
                    _ => TypeTest::And(x, y),
                };
                self.prog.add_test(combined)
            }
            TypeTest::Outer(accessor, inner) => {
                let x = self.specialise_test(inner, subst);
                if x == inner {
                    return test;
                }
                self.prog.add_test(TypeTest::Outer(accessor, x))
            }
            _ => test,
        }
    }

    /// The type parameters the stored body defines, which a copy makes afresh: its local
    /// methods' and its type patterns' variables.
    pub(super) fn local_tparams(&mut self, def: &InlineDefinition) -> Vec<TParamId> {
        let mut out: Vec<TParamId> = def.binders.iter().flat_map(|&b| self.sig_of(b).tparams.clone()).collect();
        out.extend(def.pattern_tparams.iter().copied());
        out
    }

    /// The census's check of a checked body's record (`TEQ_INLINE_CENSUS`): what the contract
    /// promises of it and of its copies. What fails, when something does.
    fn record_problem(&mut self, def: &InlineDefinition, tparams: &[TParamId]) -> Option<String> {
        // A link build writes every class of the program, its copies' classes included, which
        // no walk settles: the census copies no record whose body makes a class there.
        let classes = self.link_mode() && !def.classes.is_empty();
        self.calls_without_type_args(def).or_else(|| self.splices_unbound(def)).or_else(|| if classes { None } else { self.copies_apart(def, tparams) })
    }

    /// Every splice holds its code as the context function of the `Quotes` it binds, a binder
    /// of the body, which a copy renames.
    fn splices_unbound(&self, def: &InlineDefinition) -> Option<String> {
        def.splices.iter().find_map(|&s| {
            let TExpr::Splice(code) = self.prog.expr(s) else { return Some("a splice that is no TExpr::Splice".to_string()) };
            match self.prog.expr(code) {
                TExpr::Lambda(params, _) if matches!(self.prog.sym_list(params), [q] if def.binders.contains(q)) => None,
                _ => Some("a splice without the Quotes it binds".to_string()),
            }
        })
    }

    /// Every call in the body of a method with type parameters has its type arguments in
    /// `type_args`.
    fn calls_without_type_args(&mut self, def: &InlineDefinition) -> Option<String> {
        let body = def.body?;
        let roots = self.stored_roots(std::iter::once(body).chain(def.defaults.iter().flatten().copied()));
        let with: FxMap<TExprId, ()> = def.type_args.iter().map(|&(e, _)| (e, ())).collect();
        let nodes: Vec<TExprId> = roots.iter().flat_map(|&r| self.prog.descendants(r)).collect();
        for e in nodes {
            let callee = match self.prog.expr(e) {
                TExpr::CallStatic(s, _) | TExpr::CallMethod(_, s, _) => s,
                _ => continue,
            };
            if !with.contains_key(&e) && !self.sig_of(callee).tparams.is_empty() {
                return Some(format!("a call of {} without its type arguments", self.name_str(self.syms.sym(callee).name)));
            }
        }
        None
    }

    /// Two instantiations of the body, the parameters bound to fresh locals and the type
    /// parameters to `Any`, share no binder and no type parameter of a local method or of a
    /// type pattern, and neither their trees, nor their patterns' types, nor the signatures of
    /// their binders with the bounds of their type parameters, nor the types recorded on their
    /// nodes, nor the records of their deferred calls name a binder, a parameter or a type
    /// parameter of the stored body, the method's, a local method's or a pattern's; and each
    /// copy's metadata stands on its own nodes (`instance_problem`).
    fn copies_apart(&mut self, def: &InlineDefinition, tparams: &[TParamId]) -> Option<String> {
        def.body?;
        let subst: Subst = tparams.iter().map(|&p| (p, ANY)).collect();
        // What the stored body names of its own, read from the record rather than from what the
        // copy freshens: the method's type parameters, its local methods' and its patterns'.
        let method_tparams: Vec<TParamId> = def.binders.iter().flat_map(|&b| self.sig_of(b).tparams.clone()).collect();
        let tparams: &[TParamId] = &[tparams, &method_tparams, &def.pattern_tparams].concat();
        let deferred_mark = self.quote.copied_deferred.len();
        let mut copies = Vec::new();
        let mut arguments = Vec::new();
        for _ in 0..2 {
            let mut args: FxMap<SymId, TExprId> = FxMap::default();
            for &p in &def.params {
                let fresh = self.clone_local(p);
                args.insert(p, self.prog.add(TExpr::Local(fresh)));
            }
            let paths = def.params.iter().filter_map(|&p| args.get(&p).and_then(|&a| self.argument_path(a)).map(|path| (p, path))).collect();
            let bindings = Bindings { args, subst: subst.clone(), paths, this: None, this_paths: Vec::new(), mark_leaves: true };
            copies.push(self.instantiate_definition(def, &bindings)?);
            arguments.push(bindings.args);
        }
        let copied_deferred = self.quote.copied_deferred.split_off(deferred_mark);
        // The census's copies are no part of the output: their tests and patterns stay out of
        // the numbering of the output's classes, as the stored body's do.
        for copy in &copies {
            for root in self.stored_roots(std::iter::once(copy.root).chain(copy.defaults.iter().flatten().copied())) {
                self.store_tests_and_pats(root);
            }
        }
        let (first, second) = (&copies[0], &copies[1]);
        if first.renames.values().any(|s| second.renames.values().any(|t| s == t)) {
            return Some("two copies share a binder".to_string());
        }
        if first.tparams.iter().any(|&(_, p)| second.tparams.iter().any(|&(_, q)| p == q)) {
            return Some("two copies share a type parameter".to_string());
        }
        let old: Vec<SymId> = def.binders.iter().chain(&def.params).copied().collect();
        let name = |w: &Self, s: SymId| w.name_str(w.syms.sym(s).name);
        for (copy, args) in copies.iter().zip(&arguments) {
            // The copy's trees, and the bodies of the classes it copied, where the metadata of
            // their kept calls stands.
            let bodies: Vec<TClass> = copy.classes.iter().filter_map(|&(_, c)| self.prog_index.class(&self.prog, c).map(|i| self.prog.classes[i].clone())).collect();
            let class_roots = self.class_roots(&bodies);
            let roots = self.stored_roots(std::iter::once(copy.root).chain(copy.defaults.iter().flatten().copied()).chain(class_roots));
            let nodes: Vec<TExprId> = roots.iter().flat_map(|&r| self.prog.descendants(r)).collect();
            for &e in &nodes {
                if let TExpr::Local(s) | TExpr::CallStatic(s, _) = self.prog.expr(e) {
                    if old.contains(&s) {
                        return Some(format!("a copy names {} of the stored body", name(self, s)));
                    }
                }
                if let Some(t) = self.prog.type_of(e).filter(|&t| self.names_stored(t, &old, tparams)) {
                    return Some(format!("a copy's node has the type {} of the stored body", self.show(t)));
                }
            }
            let mut binders = Vec::new();
            for &root in &roots {
                self.tree_binders(root, &mut binders);
            }
            if let Some(&s) = binders.iter().find(|s| old.contains(s)) {
                return Some(format!("a copy binds {} of the stored body", name(self, s)));
            }
            for &fresh in copy.renames.values() {
                let sig = self.sig_arc(fresh);
                for &p in &sig.tparams {
                    if tparams.contains(&p) {
                        return Some(format!("the signature of a copy's {} keeps the type parameter {} of the stored body", name(self, fresh), self.name_str(self.syms.tparam(p).name)));
                    }
                }
                let params = sig.clauses.iter().flat_map(|c| c.params.iter());
                if let Some(p) = params.clone().find(|p| old.contains(&p.sym)) {
                    return Some(format!("the signature of a copy's {} names the parameter {} of the stored body", name(self, fresh), name(self, p.sym)));
                }
                let types: Vec<TypeId> = params.map(|p| p.ty).chain(std::iter::once(sig.ret)).collect();
                if let Some(t) = types.into_iter().find(|&t| self.names_stored(t, &old, tparams)) {
                    return Some(format!("the signature of a copy's {} has the type {} of the stored body", name(self, fresh), self.show(t)));
                }
            }
            for &(_, p) in &copy.tparams {
                let (upper, lower) = (self.syms.tparam(p).upper, self.syms.tparam(p).lower);
                if let Some(t) = [upper, lower].into_iter().find(|&t| self.names_stored(t, &old, tparams)) {
                    return Some(format!("a bound of a copy's type parameter {} is the type {} of the stored body", self.name_str(self.syms.tparam(p).name), self.show(t)));
                }
            }
            if let Some(problem) = self.instance_problem(def, copy, args, &nodes, &old, tparams) {
                return Some(problem);
            }
        }
        for e in copied_deferred {
            let Some(d) = self.quote.deferred.get(&e).cloned() else { return Some("a copy's deferred call without its record".to_string()) };
            if old.contains(&d.sym) {
                return Some(format!("a copy's deferred call names {} of the stored body", name(self, d.sym)));
            }
            let types: Vec<TypeId> = d.subst.iter().chain(&d.owner_subst).map(|&(_, t)| t).chain(d.prefix).chain(std::iter::once(d.ret_ty)).chain(d.expected).collect();
            if let Some(t) = types.into_iter().find(|&t| self.names_stored(t, &old, tparams)) {
                return Some(format!("a copy's deferred call of {} has the type {} of the stored body", name(self, d.sym), self.show(t)));
            }
            // The callee's signature names its own parameters and type parameters, which are the
            // stored body's where the body calls its own method.
            let own_params: Vec<SymId> = d.sig.clauses.iter().flat_map(|c| c.params.iter().map(|p| p.sym)).collect();
            let old_here: Vec<SymId> = old.iter().copied().filter(|s| !own_params.contains(s)).collect();
            let tparams_here: Vec<TParamId> = tparams.iter().copied().filter(|p| !d.sig.tparams.contains(p)).collect();
            let sig_types: Vec<TypeId> = d.sig.clauses.iter().flat_map(|c| c.params.iter().map(|p| p.ty)).chain(std::iter::once(d.sig.ret)).collect();
            if let Some(t) = sig_types.into_iter().find(|&t| self.names_stored(t, &old_here, &tparams_here)) {
                return Some(format!("a copy's deferred call of {} has the type {} of the stored body", name(self, d.sym), self.show(t)));
            }
        }
        None
    }

    /// The metadata of one copy (`Instance`) against its record: every node of the stored body
    /// has its copy among the copy's nodes; the reducible nodes are `If` and `Match` nodes, the
    /// deferred calls calls with their records, the splices `Splice` nodes, each as many as the
    /// record's; each call with type arguments is a node of the copy, its types naming nothing
    /// of the stored body; a leaf is marked, and a parameter's is the copy of its argument; the
    /// patterns' tests are the copy's own, and its leaf tests are among the output's leaf tests;
    /// the patterns' types name nothing of the stored body.
    fn instance_problem(&mut self, def: &InlineDefinition, copy: &Instance, args: &FxMap<SymId, TExprId>, nodes: &[TExprId], old: &[SymId], tparams: &[TParamId]) -> Option<String> {
        let stored_roots = self.stored_roots(def.body.into_iter().chain(def.defaults.iter().flatten().copied()));
        if copy.defaults.len() != def.defaults.len() || copy.defaults.iter().zip(&def.defaults).any(|(c, d)| c.is_some() != d.is_some()) {
            return Some("a default without its copy".to_string());
        }
        let own: FxMap<TExprId, ()> = nodes.iter().map(|&e| (e, ())).collect();
        for root in stored_roots {
            for e in self.prog.descendants(root) {
                match copy.exprs.get(e) {
                    Some(c) if own.contains_key(c) => {}
                    Some(_) => return Some("a node's copy is not in the copy's tree".to_string()),
                    None => return Some("a node of the stored body without its copy".to_string()),
                }
            }
        }
        if copy.reducible.len() != def.reducible.len() || copy.reducible.iter().any(|&e| !matches!(self.prog.expr(e), TExpr::If(..) | TExpr::Match(..))) {
            return Some("a copy's reducible node is no If or Match".to_string());
        }
        if copy.deferred.len() != def.deferred.len() || copy.deferred.iter().any(|e| !self.quote.deferred.contains_key(e) || !own.contains_key(e)) {
            return Some("a copy's deferred call without its record".to_string());
        }
        if copy.splices.len() != def.splices.len() || copy.splices.iter().any(|&e| !matches!(self.prog.expr(e), TExpr::Splice(_))) {
            return Some("a copy's splice is no Splice".to_string());
        }
        if copy.type_args.len() != def.type_args.len() {
            return Some("a call with type arguments without its copy".to_string());
        }
        for ((stored, _), (e, ts)) in def.type_args.iter().zip(&copy.type_args) {
            if !own.contains_key(e) || std::mem::discriminant(&self.prog.expr(*stored)) != std::mem::discriminant(&self.prog.expr(*e)) {
                return Some("a copy's call with type arguments is no copy of the call".to_string());
            }
            if let Some(&t) = ts.iter().find(|&&t| self.names_stored(t, old, tparams)) {
                return Some(format!("a copy's call has the type argument {} of the stored body", self.show(t)));
            }
        }
        if copy.leaves.len() != def.leaves.len() || copy.leaves.iter().any(|&l| !self.prog.is_leaf(l)) {
            return Some("a copy's leaf is not marked".to_string());
        }
        for (&stored, &leaf) in def.leaves.iter().zip(&copy.leaves) {
            let TExpr::Local(p) = self.prog.expr(stored) else { continue };
            let Some(TExpr::Local(a)) = args.get(&p).map(|&a| self.prog.expr(a)) else { continue };
            if !self.prog.descendants(leaf).any(|e| matches!(self.prog.expr(e), TExpr::Local(s) if s == a)) {
                return Some("a parameter's leaf is not the copy of its argument".to_string());
            }
        }
        let stored_tests: Vec<TestId> = copy.tests.keys().copied().collect();
        if copy.tests.values().any(|t| stored_tests.contains(t)) {
            return Some("a copy's test is the stored body's".to_string());
        }
        if copy.leaf_tests.len() != def.leaf_tests.len() || copy.leaf_tests.iter().any(|t| !self.prog.leaf_tests.contains_key(t)) {
            return Some("a copy's leaf test is not marked".to_string());
        }
        let mut pats: Vec<TPatId> = Vec::new();
        for &e in nodes {
            match self.prog.expr(e) {
                TExpr::Match(_, cases) => pats.extend(self.prog.case_list(cases).iter().map(|c| c.pat)),
                TExpr::Try(i) => pats.extend(self.prog.case_list(self.prog.tries[i as usize].cases).iter().map(|c| c.pat)),
                _ => {}
            }
        }
        while let Some(p) = pats.pop() {
            match self.prog.pats[p.idx()] {
                TPat::Test(_, t, _) | TPat::Class(_, t, _, _) if self.names_stored(t, old, tparams) => {
                    return Some(format!("a copy's pattern has the type {} of the stored body", self.show(t)));
                }
                TPat::Bind(_, inner) => pats.extend(inner),
                TPat::Test(_, _, inner) | TPat::Unapply(_, _, inner) => pats.push(inner),
                TPat::Class(_, _, _, subs) | TPat::Alt(subs) => pats.extend_from_slice(self.prog.pat_list(subs)),
                TPat::Seq(items, rest) => {
                    pats.extend_from_slice(self.prog.pat_list(items));
                    pats.extend(rest);
                }
                TPat::Wildcard | TPat::Equals(..) => {}
            }
        }
        None
    }

    /// Whether `t` names one of the terms `old` or of the type parameters `tparams`.
    fn names_stored(&self, t: TypeId, old: &[SymId], tparams: &[TParamId]) -> bool {
        (self.types.has_paths(t) && old.iter().any(|&s| self.mentions_term(t, s))) || (!tparams.is_empty() && self.mentions_tparam_of(t, Some(tparams)))
    }

    /// The body of `sym` typed as the definition check types it: in the environment of the
    /// definition, with the parameters entered as themselves, against the declared result type
    /// when there is one. What it answers: the body, its type and the defaults.
    fn type_inline_definition(&mut self, sym: SymId, sig: &MethodSig) -> (Option<TExprId>, TypeId, Vec<Option<TExprId>>) {
        if self.forked {
            self.refresh_std_if_stale();
            self.sync_arity_tables();
        }
        let (file, def_id) = {
            let s = self.syms.sym(sym);
            (s.file, s.def)
        };
        let Some(def_id) = def_id else { return (None, ERROR, Vec::new()) };
        let def = self.ast(file).def(def_id);
        let (body, declared, default_exprs): (Option<crate::ast::ExprId>, bool, Vec<Option<crate::ast::ExprId>>) = match &def.kind {
            DefKind::Fun(f) => (f.body, f.ret.is_some(), f.clauses.iter().flat_map(|c| c.params.iter().map(|p| p.default)).collect()),
            DefKind::Given(g) => (g.alias, true, g.clauses.iter().flat_map(|c| c.params.iter().map(|p| p.default)).collect()),
            _ => (None, false, Vec::new()),
        };
        let Some(body) = body else { return (None, ERROR, Vec::new()) };
        let tparams = sig.tparams.iter().map(|&p| (self.syms.tparam(p).name, p)).collect();
        let frame = self.env.frames.len();
        self.env.frames.push(Frame::Locals { names: Vec::new(), tparams, givens: Vec::new(), classes: Vec::new(), aliases: Vec::new(), owner: self.sites.owners.len() as u32 + 1 });
        let outer_ext_scope = self.ext_scope;
        if let Some(scope) = self.ext_scope_of(sym, sig, def) {
            self.ext_scope = Some(scope);
        }
        let owners_depth = self.sites.owners.len();
        self.sites.owners.push(SiteOwner::Sym(sym));
        self.bodies_in_progress.push(sym);
        let var_frame = self.var_frame_begin();
        self.body_depth += 1;
        let outer_defining = self.defining;
        if let DefKind::Given(g) = &def.kind {
            self.defining = g.alias.map(|_| sym);
        }
        let outer_return = self.return_to.take();
        let outer_returns = std::mem::take(&mut self.returns);
        let own_nowarn = def.annots.iter().any(|a| a.name == crate::names::NOWARN) as u32;
        let outer_nowarn = self.nowarn;
        self.nowarn = self.enclosing_nowarn(sym) + own_nowarn;
        let transparent = def.mods & mods::TRANSPARENT != 0;
        let defaults = self.enter_params(sig, &default_exprs, frame);
        let (te, ty) = if declared {
            (self.check_expr(body, sig.ret), sig.ret)
        } else {
            let (te, ty) = self.type_expr(body, None);
            let ty = if transparent { self.solve_in(ty) } else { self.solve_inferred(ty) };
            (te, ty)
        };
        self.nowarn = outer_nowarn;
        self.returns = outer_returns;
        self.return_to = outer_return;
        self.defining = outer_defining;
        self.body_depth -= 1;
        self.var_frame_end(var_frame);
        self.bodies_in_progress.pop();
        self.sites.owners.truncate(owners_depth);
        self.ext_scope = outer_ext_scope;
        self.env.frames.truncate(frame);
        (Some(te), ty, defaults)
    }

    /// The roots of a stored body's trees: `roots` and the defaults of the local functions under
    /// them, which `Program::descendants` leaves out.
    fn stored_roots(&self, roots: impl Iterator<Item = TExprId>) -> Vec<TExprId> {
        let mut out: Vec<TExprId> = roots.collect();
        let mut next = 0;
        while next < out.len() {
            let root = out[next];
            next += 1;
            let mut found = Vec::new();
            for e in self.prog.descendants(root) {
                if let TExpr::Block(stmts, _) = self.prog.expr(e) {
                    for s in self.prog.stmt_list(stmts) {
                        if let TStmt::Fun(f) = *s {
                            found.extend(self.prog.funs[f.idx()].defaults.iter().flatten().copied());
                        }
                    }
                }
            }
            out.extend(found);
        }
        out
    }

    /// The type of every node of the stored trees `roots` and of the defaults of their local
    /// functions, as the typing gave it (kept aside where the build keeps no types), its
    /// variables solved.
    fn stored_node_types(&mut self, roots: impl Iterator<Item = TExprId>) -> FxMap<TExprId, TypeId> {
        let roots = self.stored_roots(roots);
        let nodes: Vec<TExprId> = roots.iter().flat_map(|&r| self.prog.descendants(r)).collect();
        let mut out = FxMap::default();
        for &e in &nodes {
            if let Some(t) = self.prog.type_or_aside(e) {
                let t = self.zonk(t);
                out.insert(e, t);
            }
        }
        // A node the typer made on the side has no type of its own: the one its kind gives it,
        // children first.
        for &e in nodes.iter().rev() {
            if !out.contains_key(&e) {
                if let Some(t) = self.side_node_type(e, &out) {
                    out.insert(e, t);
                }
            }
        }
        out
    }

    /// The type of a node the typer made on the side, which its kind and its parts give: a
    /// string conversion's, a primitive operation's, a literal's, a reference's declared type,
    /// a block's result's.
    fn side_node_type(&mut self, e: TExprId, typed: &FxMap<TExprId, TypeId>) -> Option<TypeId> {
        let b = &self.b;
        let (int, long, double, float, boolean, string, unit, char) = (b.t_int, b.t_long, b.t_double, b.t_float, b.t_boolean, b.t_string, b.t_unit, b.t_char);
        Some(match self.prog.expr(e) {
            TExpr::ToStr(..) | TExpr::StrConcat(_) | TExpr::Str(_) => string,
            TExpr::Int(_) => int,
            TExpr::Long(_) => long,
            TExpr::Double(_) => double,
            TExpr::Bool(_) | TExpr::TypeTest(..) => boolean,
            TExpr::Cast(_, _, to) => to,
            TExpr::Char(_) => char,
            TExpr::Unit | TExpr::While(..) | TExpr::Assign(..) => unit,
            TExpr::Throw(..) | TExpr::Return(_) => NOTHING,
            TExpr::Local(s) | TExpr::Static(s) | TExpr::Field(_, s) => {
                let sig = self.sig_of(s);
                if !sig.tparams.is_empty() || !sig.clauses.is_empty() {
                    return None;
                }
                sig.ret
            }
            TExpr::Block(_, r) => return typed.get(&r).copied(),
            TExpr::Prim(op, ..) => {
                use PrimOp::*;
                match op {
                    IntAdd | IntSub | IntMul | IntDiv | IntRem | IntAnd | IntOr | IntXor | IntShl | IntShr | IntUshr => int,
                    LongAdd | LongSub | LongMul | LongDiv | LongRem | LongAnd | LongOr | LongXor | LongShl | LongShr | LongUshr => long,
                    DoubleAdd | DoubleSub | DoubleMul | DoubleDiv | DoubleRem => double,
                    FloatAdd | FloatSub | FloatMul | FloatDiv | FloatRem => float,
                    Lt | Le | Gt | Ge | RefEq | RefNe | Eq | Ne | BoolAnd | BoolOr | BoolXor | BoolStrictAnd | BoolStrictOr => boolean,
                }
            }
            TExpr::Unary(op, _) => {
                use UnOp::*;
                match op {
                    IntNeg | IntNot | LongToInt | DoubleToInt | CharToInt | FloatToInt | IntToByte | IntToShort | ByteToShort | ByteToInt | ShortToInt => int,
                    LongNeg | LongNot | IntToLong | DoubleToLong | CharToLong | FloatToLong => long,
                    DoubleNeg | LongToDouble | FloatToDouble | IntToDouble => double,
                    FloatNeg | IntToFloat | LongToFloat | DoubleToFloat => float,
                    BoolNot => boolean,
                    IntToChar => char,
                }
            }
            _ => return None,
        })
    }

    /// Records the type tests and patterns under `root` as a stored body's
    /// (`Program::stored_tests`, `stored_pats`).
    pub(super) fn store_tests_and_pats(&mut self, root: TExprId) {
        let mut tests: Vec<TestId> = Vec::new();
        let mut pats: Vec<TPatId> = Vec::new();
        for e in self.prog.descendants(root) {
            match self.prog.expr(e) {
                TExpr::TypeTest(_, t) | TExpr::Cast(_, CastOp::Check(t, _) | CastOp::Unbox(t, _), _) => tests.push(t),
                TExpr::Match(_, cases) => pats.extend(self.prog.case_list(cases).iter().map(|c| c.pat)),
                TExpr::Try(i) => pats.extend(self.prog.case_list(self.prog.tries[i as usize].cases).iter().map(|c| c.pat)),
                TExpr::Block(stmts, _) => pats.extend(self.prog.stmt_list(stmts).iter().filter_map(|s| match *s {
                    TStmt::Pat(p, _) => Some(p),
                    _ => None,
                })),
                _ => {}
            }
        }
        self.store_pats_and_tests(pats, tests);
    }

    /// Records the patterns `pats` and the tests `tests` and the tests under them as a stored
    /// body's, which no output holds.
    pub(super) fn store_pats_and_tests(&mut self, mut pats: Vec<TPatId>, mut tests: Vec<TestId>) {
        self.store_listed(&mut pats, &mut tests);
    }

    /// `store_pats_and_tests` of the patterns of the cases `cases`, on lists kept for it.
    pub(super) fn store_case_pats(&mut self, cases: crate::ast::ListRef) {
        let (mut pats, mut tests) = std::mem::take(&mut self.inline.stored_lists);
        pats.clear();
        tests.clear();
        pats.extend(self.prog.case_list(cases).iter().map(|c| c.pat));
        self.store_listed(&mut pats, &mut tests);
        self.inline.stored_lists = (pats, tests);
    }

    /// The patterns `pats` and the tests `tests`, and those under them, recorded as a stored
    /// body's, the lists left empty.
    fn store_listed(&mut self, pats: &mut Vec<TPatId>, tests: &mut Vec<TestId>) {
        while let Some(p) = pats.pop() {
            if self.prog.stored_pats.insert(p, ()).is_some() {
                continue;
            }
            match self.prog.pats[p.idx()] {
                TPat::Bind(_, inner) => pats.extend(inner),
                TPat::Test(t, _, inner) => {
                    tests.push(t);
                    pats.push(inner);
                }
                TPat::Unapply(_, _, inner) => pats.push(inner),
                TPat::Class(_, _, _, subs) | TPat::Alt(subs) => pats.extend_from_slice(self.prog.pat_list(subs)),
                TPat::Seq(items, rest) => {
                    pats.extend_from_slice(self.prog.pat_list(items));
                    pats.extend(rest);
                }
                TPat::Wildcard | TPat::Equals(..) => {}
            }
        }
        while let Some(t) = tests.pop() {
            if self.prog.stored_tests.insert(t, ()).is_some() {
                continue;
            }
            match self.prog.tests[t.idx()] {
                TypeTest::Or(a, b) | TypeTest::And(a, b) => tests.extend([a, b]),
                TypeTest::Outer(_, inner) => tests.push(inner),
                _ => {}
            }
        }
    }

    /// The nodes of the body's trees `roots` that an expansion takes from its call site: the
    /// references to the parameters and to `this`, and the deferred calls of the intrinsics
    /// whose value is the site's.
    fn definition_leaves(&mut self, roots: &[TExprId], params: &[SymId], deferred: &[TExprId], out: &mut Vec<TExprId>) {
        let nodes: Vec<TExprId> = roots.iter().flat_map(|&r| self.prog.descendants(r)).collect();
        for e in nodes {
            match self.prog.expr(e) {
                TExpr::Local(s) if params.contains(&s) => out.push(e),
                TExpr::This => out.push(e),
                _ if deferred.contains(&e) => {
                    let Some(callee) = self.quote.deferred.get(&e).map(|d| d.sym) else { continue };
                    let site_valued = matches!(
                        self.intrinsic_of(callee),
                        Some(super::inline::Intrinsic::ConstValue | super::inline::Intrinsic::ConstValueOpt | super::inline::Intrinsic::ConstValueTuple | super::inline::Intrinsic::SummonInline | super::inline::Intrinsic::SummonAll)
                    );
                    if site_valued {
                        out.push(e);
                    }
                }
                _ => {}
            }
        }
        out.sort();
    }
}

/// What an instantiation binds (`Worker::instantiate_definition`): each parameter to the node
/// that stands for it in the copy (its proxy's `Local`, which the expansion replaces by the
/// argument where the argument is substituted), the type parameters to the call's type
/// arguments, each parameter's path in the copy's types (a literal argument's type, a stable
/// argument's path, the proxy's otherwise), and the receiver: what stands for `This` and the
/// path `C.this` is seen from in the types.
#[derive(Clone)]
pub struct Bindings {
    pub args: FxMap<SymId, TExprId>,
    pub subst: Subst,
    pub paths: Vec<(SymId, TypeId)>,
    pub this: Option<TExprId>,
    pub this_paths: Vec<(ClassId, TypeId)>,
    /// Whether the copy's leaves are marked as the output's: not for the expansion by
    /// substitution, whose bindings mark the arguments as the retype path's do.
    pub mark_leaves: bool,
}

/// One copy of a stored body (`Worker::instantiate_definition`): its root, the fresh binders
/// and type parameters by the stored ones, each node and type test of the stored body with its
/// copy (a parameter's `Local` with the copy of its argument), and the record's metadata as it
/// stands on the copy: its reducible nodes, deferred calls (registered for the site), splices,
/// leaves and leaf tests, and the calls' type arguments rewritten as the copy's types are.
pub struct Instance {
    pub root: TExprId,
    /// The index in the record's `reducible` of each of `reducible`.
    pub reducible_at: Vec<usize>,
    /// The copies of the stored body's `This` nodes, those that stand for the receiver.
    pub this_copies: Vec<TExprId>,
    /// The classes the stored body makes, each with its copy (`Copier::stored_class`).
    pub classes: Vec<(ClassId, ClassId)>,
    /// The copies of the defaults, parallel to the record's.
    pub defaults: Vec<Option<TExprId>>,
    pub renames: FxMap<SymId, SymId>,
    pub tparams: Vec<(TParamId, TParamId)>,
    pub exprs: super::quoted::StoredTable<TExprId>,
    pub tests: FxMap<TestId, TestId>,
    /// The type of each node of the copy, in the copy's terms.
    pub types: super::quoted::CopyTypes,
    pub reducible: Vec<TExprId>,
    pub deferred: Vec<TExprId>,
    pub type_args: Vec<(TExprId, Vec<TypeId>)>,
    pub splices: Vec<TExprId>,
    pub leaves: Vec<TExprId>,
    pub leaf_tests: Vec<TestId>,
    /// The record's block imports on the copy's blocks.
    pub imports: Vec<BlockImport>,
    /// The record's names bound to a value's member on the copy's blocks, their selections the
    /// stored trees, which the walk copies where it enters them.
    pub aliases: Vec<StoredAlias>,
}

/// What a copy of a stored body keeps between its parts (`Worker::begin_demand`,
/// `demand_roots`): the bindings, the substitution with the fresh type parameters, the paths of
/// the renamed binders so far, and, for a copy on demand, the stored nodes left in its tree for
/// the walk's demands, each with the specialisation of the reduced cases around it.
pub struct Demand {
    args: FxMap<SymId, TExprId>,
    subst: Arc<Subst>,
    paths: Vec<(SymId, TypeId)>,
    this: Option<TExprId>,
    this_paths: Vec<(ClassId, TypeId)>,
    mark_leaves: bool,
    /// Whether the copy leaves the branches and the reduced matches' cases for the walk.
    on_demand: bool,
    /// Whether the classes the copy makes take the output's names: an expansion's, made on
    /// demand or, under the capture, whole; not the census's or a measurement's.
    pub names_classes: bool,
    /// The nodes the expansion made for its bindings (`StoredCopy::bound`), none for a copy
    /// made whole.
    pub bound: (u32, u32),
    index: Arc<StoredIndex>,
    pub left: super::quoted::StoredTable<Arc<Subst>>,
    /// The members and parameters of the classes the copy made so far, each with its copy's.
    members: FxMap<SymId, SymId>,
    /// The classes the copy made so far, each with its copy.
    copied_classes: FxMap<ClassId, ClassId>,
    /// What the part being copied adds (`demand_parts`), its room kept for the next part.
    part: super::quoted::CopyPart,
    /// The nodes the copy made so far.
    pub copied: u64,
}

impl Instance {
    /// An instance before any copy: `begin_demand_in` sets it for one.
    pub(super) fn empty() -> Instance {
        Instance {
            root: TExprId(0),
            defaults: Vec::new(),
            renames: FxMap::default(),
            tparams: Vec::new(),
            exprs: super::quoted::StoredTable::default(),
            tests: FxMap::default(),
            types: super::quoted::CopyTypes::default(),
            reducible: Vec::new(),
            reducible_at: Vec::new(),
            this_copies: Vec::new(),
            classes: Vec::new(),
            deferred: Vec::new(),
            type_args: Vec::new(),
            splices: Vec::new(),
            leaves: Vec::new(),
            leaf_tests: Vec::new(),
            imports: Vec::new(),
            aliases: Vec::new(),
        }
    }
}

impl Demand {
    /// A demand before any copy: `begin_demand_in` sets it for one.
    pub(super) fn empty(index: Arc<StoredIndex>, subst: Arc<Subst>) -> Demand {
        Demand {
            args: FxMap::default(),
            subst,
            paths: Vec::new(),
            this: None,
            this_paths: Vec::new(),
            mark_leaves: false,
            on_demand: false,
            names_classes: false,
            bound: (0, 0),
            index,
            left: super::quoted::StoredTable::default(),
            members: FxMap::default(),
            copied_classes: FxMap::default(),
            part: super::quoted::CopyPart::default(),
            copied: 0,
        }
    }

    /// The tables the copy filled, emptied and taken out for the next copy's bindings: the
    /// parameters' nodes (read by key alone: a quote's copy takes its by-name proxies from it as
    /// a set), the parameters' paths and the receiver's.
    pub fn take_lists(&mut self) -> (FxMap<SymId, TExprId>, Vec<(SymId, TypeId)>, Vec<(ClassId, TypeId)>) {
        let (mut args, mut paths, mut this_paths) = (std::mem::take(&mut self.args), std::mem::take(&mut self.paths), std::mem::take(&mut self.this_paths));
        args.clear();
        paths.clear();
        this_paths.clear();
        (args, paths, this_paths)
    }

    /// The copy puts `node` where the stored body names the local `s`.
    pub fn stand_for(&mut self, s: SymId, node: TExprId) {
        self.args.insert(s, node);
    }

    /// Whether `e` is a stored node the copy left for a demand.
    pub fn is_left(&self, e: TExprId) -> bool {
        self.left.contains(e)
    }

    /// How many nodes the stored tree `root` holds, counted at its first prune and kept with
    /// the record's index; a tree outside the record's range counted each time.
    pub fn pruned_size(&self, prog: &Program, root: TExprId) -> u64 {
        let count = || prog.descendants(root).count() as u64;
        if self.index.node(root).is_none() {
            return count();
        }
        let mut sizes = self.index.sizes.lock().unwrap_or_else(|e| e.into_inner());
        *sizes.entry(root).or_insert_with(count)
    }

    /// What the copy binds of the call: the type arguments with the fresh type parameters after
    /// them, the parameters' paths with the renamed binders' after them, and the receiver's paths.
    pub fn bindings(&self) -> (&Subst, &[(SymId, TypeId)], &[(ClassId, TypeId)]) {
        (&self.subst, &self.paths, &self.this_paths)
    }
}

/// A kind of the record's metadata on a node, with its index in the record's list.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Meta {
    Reducible(u32),
    Deferred(u32),
    TypeArgs(u32),
    Splice(u32),
    Leaf(u32),
    Widened(u32),
    Opaque(u32),
    Spread(u32),
    Erased(u32),
    Import(u32),
    Alias(u32),
}

/// What every copy of a record reads of it, made once per record (`Worker::stored_index_of`):
/// the record's metadata by stored node, in the record's order within a kind; its reducible
/// matches and leaf tests as sets; the type parameters a copy makes afresh.
pub(super) struct StoredIndex {
    meta: FxMap<TExprId, Vec<Meta>>,
    /// The size of each tree of the record a walk pruned, counted at its first prune
    /// (`Demand::pruned_size`).
    sizes: std::sync::Mutex<FxMap<TExprId, u64>>,
    reducible: FxMap<TExprId, ()>,
    leaf_tests: FxMap<TestId, ()>,
    local_tparams: Vec<TParamId>,
    /// The first of the record's nodes and how far they run (`StoredTable`).
    nodes: (u32, u32),
    /// Each node of that range as a copy reads it (`Worker::stored_nodes`).
    node: Vec<StoredNode>,
    /// Whether the body defines a binder or makes a class, which a part's copy renames or names:
    /// a body without either is copied without looking for them.
    pub(super) binds: bool,
}

impl StoredIndex {
    /// Whether `t` is one of the record's leaf tests.
    pub(super) fn is_leaf_test(&self, t: TestId) -> bool {
        self.leaf_tests.contains_key(&t)
    }

    /// The node `e` of the record's range as a copy reads it; none for a node outside it (an
    /// argument's, the receiver's), which a copy reads live.
    #[inline]
    pub(super) fn node(&self, e: TExprId) -> Option<StoredNode> {
        self.node.get(e.0.wrapping_sub(self.nodes.0) as usize).copied()
    }
}

/// A node of a record's range (`StoredIndex::node`): its type in the record (`NO_TYPE` where the
/// record has none) and what a copy of it reads beside the tree.
#[derive(Clone, Copy)]
pub(super) struct StoredNode {
    pub ty: TypeId,
    pub flags: u16,
}

impl StoredNode {
    /// The record has the node's type.
    pub const TYPED: u16 = 1;
    /// That type is the same in every copy's terms (`Worker::closed_type`).
    pub const CLOSED: u16 = 2;
    pub const LEAF: u16 = 4;
    pub const EXPANSION: u16 = 8;
    pub const EVALUATED: u16 = 16;
    pub const INTERPOLATION: u16 = 32;
    pub const SOFT: u16 = 64;
    /// The node is a kept call with its record among the deferred calls.
    pub const DEFERRED: u16 = 128;
    /// The record's metadata names the node (`StoredIndex::meta`).
    pub const META: u16 = 256;
    /// The marks of the output a copy takes.
    pub const MARKS: u16 = Self::LEAF | Self::EXPANSION | Self::EVALUATED | Self::INTERPOLATION | Self::SOFT;

    #[inline]
    pub fn has(self, flag: u16) -> bool {
        self.flags & flag != 0
    }
}

impl InlineDefinition {
    fn in_progress(params: Vec<SymId>) -> InlineDefinition {
        InlineDefinition {
            state: DefinitionState::InProgress,
            body: None,
            ty: ERROR,
            params,
            defaults: Vec::new(),
            binders: Vec::new(),
            pattern_tparams: Vec::new(),
            reducible: Vec::new(),
            reducible_sources: Vec::new(),
            deferred: Vec::new(),
            type_args: Vec::new(),
            splices: Vec::new(),
            leaves: Vec::new(),
            leaf_tests: Vec::new(),
            diagnostics: Vec::new(),
            widened: Vec::new(),
            opaque: Vec::new(),
            erased: Vec::new(),
            spread: Vec::new(),
            hoisted: Vec::new(),
            imports: Vec::new(),
            node_types: FxMap::default(),
            aliases: Vec::new(),
            classes: Vec::new(),
            inline_vals: Vec::new(),
            inferred_vals: Vec::new(),
            walk_lack: None,
        }
    }
}

/// `TEQ_INLINE_CENSUS=<file>`: a line per body the definition check met, appended to the file,
/// `<state>\t<what>\t<file>:<line>\t<method>` with the state `checked`, `failed` or `held`,
/// and what failed (the first error's first line) or the form held back; a checked body whose
/// record fails the census's check (`Worker::record_problem`) is `record` with what failed.
mod census {
    use super::*;
    use std::io::Write;
    use std::sync::{Mutex, OnceLock};

    fn census_file() -> Option<&'static Mutex<std::fs::File>> {
        static FILE: OnceLock<Option<Mutex<std::fs::File>>> = OnceLock::new();
        FILE.get_or_init(|| {
            let path = std::env::var_os("TEQ_INLINE_CENSUS")?;
            std::fs::OpenOptions::new().create(true).append(true).open(path).ok().map(Mutex::new)
        })
        .as_ref()
    }

    pub(super) fn on() -> bool {
        census_file().is_some()
    }

    pub(super) fn note(w: &Worker, sym: SymId, file: FileId, def: &InlineDefinition, copies: Option<String>) {
        let Some(out) = census_file() else { return };
        let (word, what) = match (def.state, copies) {
            (DefinitionState::Checked, Some(failure)) => ("record", failure),
            (DefinitionState::Checked | DefinitionState::InProgress, _) => ("checked", String::from("-")),
            (DefinitionState::Failed, _) => ("failed", def.diagnostics.iter().find(|d| !d.is_warning).map_or(String::new(), |d| d.msg.lines().next().unwrap_or("").to_string())),
            (DefinitionState::Held(form), _) => ("held", form.name().to_string()),
        };
        let src = w.source(file);
        let (line, _, _) = crate::source::locate(&src.text, w.syms.sym(sym).span.start as usize);
        let text = format!("{}\t{}\t{}:{}\t{}\n", word, what, src.path, line, w.sym_path(sym));
        if let Ok(mut f) = out.lock() {
            let _ = f.write_all(text.as_bytes());
        }
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;

    /// The record of the program's inline method `name`.
    pub(in crate::typer) fn record_of(w: &Worker, name: &str) -> (SymId, Arc<InlineDefinition>) {
        let found = w.inline_definitions.local.iter().find(|(&s, _)| w.name_str(w.syms.sym(s).name) == name);
        let (&sym, record) = found.unwrap_or_else(|| panic!("no record of {}", name));
        (sym, record.clone())
    }

    /// Every node of the stored body and of its defaults.
    pub(in crate::typer) fn stored_nodes(w: &Worker, def: &InlineDefinition) -> Vec<TExprId> {
        let roots = w.stored_roots(def.body.into_iter().chain(def.defaults.iter().flatten().copied()));
        roots.iter().flat_map(|&r| w.prog.descendants(r)).collect()
    }

    const TYPED: &str = "
inline def f(x: Int, y: String = \"d\"): String =
  val n = x + 1
  val s = s\"$y:$n\"
  val t = (x.toLong * 2L, -x, !(n > 2))
  if n > 2 then y + n + s + t else y
@main def run(): Unit = println(f(3))
";

    /// A lean build keeps no types, and the record keeps its own: one for every node of the
    /// body and of the defaults, the program's table blind to them.
    #[test]
    fn the_record_keeps_the_type_of_every_node_in_a_build_that_keeps_none() {
        crate::with_typed_program("typed_lean", TYPED, &[], |w| {
            assert!(!w.prog.record_types);
            let (_, def) = record_of(w, "f");
            assert_eq!(def.state, DefinitionState::Checked);
            let nodes = stored_nodes(w, &def);
            assert!(nodes.len() > 8);
            let missing: Vec<TExpr> = nodes.iter().filter(|e| !def.node_types.contains_key(e)).map(|&e| w.prog.expr(e)).collect();
            assert!(missing.is_empty(), "nodes without their types: {:?}", missing);
            assert!(nodes.iter().all(|&e| w.prog.type_of(e).is_none()));
            let body_ty = def.node_types[&def.body.unwrap()];
            assert_eq!(w.show(body_ty), "String");
            let default_ty = def.node_types[&def.defaults[1].unwrap()];
            assert_eq!(w.show(default_ty), "String");
        });
    }

    const RECEIVER: &str = "
class Box[A](val a: A):
  type Self = Box[A]
  inline def get: A = this.a
  inline def self: this.type = this
  inline def widened: Self = this
  inline def bumped(x: Int): Int = a.hashCode + x
trait Holder:
  type T
  def make: T
  inline def fresh: T = make
object H extends Holder:
  type T = Int
  def make = 1
@main def run(): Unit =
  println(H.fresh)
  val b = Box(1)
  println(b.get)
  println(b.self.a)
  println(b.widened.a)
  println(b.bumped(2))
";

    /// The receiver's node and path for a call of a method of `Box[Int]` on a proxy of it.
    fn receiver(w: &mut Worker, sym: SymId) -> (SymId, Bindings) {
        let Owner::Class(c) = w.syms.sym(sym).owner else { panic!("a member") };
        let int = w.b.t_int;
        let subst: Subst = w.syms.class(c).tparams.iter().map(|&a| (a, int)).collect();
        let targs: Vec<TypeId> = subst.iter().map(|&(_, t)| t).collect();
        let boxed = w.types.class(c, &targs);
        let name = w.interner.intern("this");
        let proxy = w.new_local(name, SymKind::Val, boxed, Span::default());
        let this = w.prog.add(TExpr::Local(proxy));
        let path = w.types.mk(Type::Term(proxy));
        let params = w.inline_definitions.get(&sym).unwrap().params.clone();
        let mut args = FxMap::default();
        let mut paths = Vec::new();
        for p in params {
            let arg = w.clone_local(p);
            args.insert(p, w.prog.add(TExpr::Local(arg)));
            paths.push((p, w.types.mk(Type::Term(arg))));
        }
        (proxy, Bindings { args, subst, paths, this: Some(this), this_paths: vec![(c, path)], mark_leaves: true })
    }

    /// The receiver stands for `This` in the copy's tree, a node of its own at each use, and
    /// `C.this` in its types is the receiver's path; the members the body selects on it take
    /// the receiver's type arguments.
    #[test]
    fn the_receiver_stands_for_this_in_the_tree_and_its_path_in_the_types() {
        crate::with_typed_program("receiver", RECEIVER, &[], |w| {
            for name in ["get", "self", "widened", "bumped", "fresh"] {
                let (sym, def) = record_of(w, name);
                assert_eq!(def.state, DefinitionState::Checked, "{}", name);
                let (proxy, bindings) = receiver(w, sym);
                let copy = w.instantiate_definition(&def, &bindings).expect("an instance");
                let nodes: Vec<TExprId> = w.prog.descendants(copy.root).collect();
                assert!(nodes.iter().all(|&e| !matches!(w.prog.expr(e), TExpr::This)), "{}: a This left", name);
                let uses: Vec<TExprId> = nodes.iter().copied().filter(|&e| matches!(w.prog.expr(e), TExpr::Local(s) if s == proxy)).collect();
                assert!(!uses.is_empty(), "{}: the receiver unused", name);
                assert!(!uses.contains(&bindings.this.unwrap()), "{}: the receiver's node shared", name);
                let Owner::Class(c) = w.syms.sym(sym).owner else { unreachable!() };
                for &e in &nodes {
                    let t = copy.types.get(e).unwrap();
                    assert!(!w.mentions_this_of(t, c), "{}: a copy's type names C.this: {}", name, w.show(t));
                }
                let root = copy.types.get(copy.root).unwrap();
                let shown = w.show(root);
                match name {
                    "get" => assert_eq!(shown, "Int"),
                    "self" => assert_eq!(shown, "Box[Int]"),
                    "widened" => assert!(w.mentions_term(root, proxy) || shown == "Box[Int]", "widened: {}", shown),
                    // `Holder.this.T` seen from the receiver.
                    "fresh" => assert!(w.mentions_term(root, proxy), "fresh: {}", shown),
                    _ => assert_eq!(shown, "Int"),
                }
            }
        });
    }

    const IMPORTS: &str = "
import scala.compiletime.summonInline
object Values:
  given Int = 7
object Names:
  given String = \"n\"
  val plain = 1
inline def f: Int =
  import Values.given
  summonInline[Int]
inline def g: String =
  val a = 1
  import Names.plain
  val b = plain
  {
    import Names.given
    summonInline[String]
  }
@main def run(): Unit =
  println(f)
  println(g)
";

    /// The record keeps each import of the body's blocks with its block and the statement it
    /// stands before, resolved as at the definition; a copy has them on its own blocks.
    #[test]
    fn the_record_keeps_the_imports_of_the_body_on_the_copy_s_blocks() {
        crate::with_typed_program("imports", IMPORTS, &[], |w| {
            let (_, f) = record_of(w, "f");
            assert_eq!(f.imports.len(), 1);
            let i = f.imports[0];
            assert_eq!((i.block, i.at), (f.body.unwrap(), 0));
            assert!(matches!(i.import.target, crate::typer::ImportTarget::ClassGivens(c) if w.name_str(w.syms.class(c).name) == "Values"));
            let (_, g) = record_of(w, "g");
            let mut at: Vec<(bool, u32)> = g.imports.iter().map(|i| (i.block == g.body.unwrap(), i.at)).collect();
            at.sort();
            assert_eq!(at, vec![(false, 0), (true, 1)]);
            let inner = g.imports.iter().find(|i| i.block != g.body.unwrap()).unwrap().block;
            assert!(w.prog.descendants(g.body.unwrap()).any(|e| e == inner) && matches!(w.prog.expr(inner), TExpr::Block(..)));
            let bindings = Bindings { args: FxMap::default(), subst: Vec::new(), paths: Vec::new(), this: None, this_paths: Vec::new(), mark_leaves: true };
            let copy = w.instantiate_definition(&g, &bindings).expect("an instance");
            assert_eq!(copy.imports.len(), 2);
            for (stored, copied) in g.imports.iter().zip(&copy.imports) {
                assert_eq!(copy.exprs.get(stored.block), Some(&copied.block));
                assert_eq!(stored.at, copied.at);
                assert!(w.prog.descendants(copy.root).any(|e| e == copied.block));
            }
        });
    }

    const CASES: &str = "
import scala.compiletime.{constValue, erasedValue}
inline def pick[T]: Any = inline erasedValue[T] match
  case _: Option[a] =>
    val none: Option[a] = None
    (x: Any) => x match
      case _: a => constValue[a]
      case _ => none
  case _ => 0
@main def run(): Unit = println(pick[Option[1]])
";

    /// The case a match reduces to, specialised by what its type variable stands for: every
    /// type, deferred call, test, pattern and binder of the case in the specialised terms,
    /// before anything in it is resolved; the other case untouched.
    #[test]
    fn a_reduced_case_is_specialised_before_its_calls_resolve() {
        crate::with_typed_program("cases", CASES, &[], |w| {
            let (sym, def) = record_of(w, "pick");
            assert_eq!(def.state, DefinitionState::Checked);
            assert_eq!(def.pattern_tparams.len(), 1);
            let t = w.sig_of(sym).tparams[0];
            let bindings = Bindings { args: FxMap::default(), subst: vec![(t, ANY)], paths: Vec::new(), this: None, this_paths: Vec::new(), mark_leaves: true };
            let mut copy = w.instantiate_definition(&def, &bindings).expect("an instance");
            let fresh = copy.tparams.iter().find(|(p, _)| *p == def.pattern_tparams[0]).map(|&(_, f)| f).expect("the fresh variable");
            let TExpr::Match(_, cases) = w.prog.expr(copy.reducible[0]) else { panic!("a match") };
            let (case, other) = (w.prog.case_list(cases)[0].body, w.prog.case_list(cases)[1].body);
            let one = w.types.lit(LitVal::Int(1));
            let names = |w: &Worker, t: TypeId| w.mentions_tparam_of(t, Some(&[fresh]));
            let deferred = |w: &Worker, root: TExprId| -> Vec<TExprId> { w.prog.descendants(root).filter(|e| w.quote.deferred.contains_key(e)).collect() };
            let calls = deferred(w, case);
            assert_eq!(calls.len(), 1);
            assert!(w.quote.deferred.get(&calls[0]).unwrap().subst.iter().any(|&(_, t)| names(w, t)));
            w.specialise_instance(&mut copy, case, &vec![(fresh, one)]);
            let record = w.quote.deferred.get(&calls[0]).unwrap().clone();
            assert!(record.subst.iter().all(|&(_, t)| t == one), "the call's type argument");
            assert!(!names(w, record.ret_ty));
            for e in w.prog.descendants(case).collect::<Vec<_>>() {
                if let Some(t) = copy.types.get(e) {
                    assert!(!names(w, t), "a node's type names the variable: {}", w.show(t));
                }
            }
            let mut binders = Vec::new();
            w.tree_binders(case, &mut binders);
            let none = binders.iter().copied().find(|&b| w.name_str(w.syms.sym(b).name) == "none").expect("the val");
            let ret = w.sig_of(none).ret;
            assert_eq!(w.show(ret), "Option[1]");
            let inner = w.prog.descendants(case).find(|&e| matches!(w.prog.expr(e), TExpr::Match(..))).expect("the inner match");
            let TExpr::Match(_, inner_cases) = w.prog.expr(inner) else { unreachable!() };
            let TPat::Test(test, ty, _) = w.prog.pats[w.prog.case_list(inner_cases)[0].pat.idx()] else { panic!("a type test") };
            assert_eq!(ty, one);
            assert!(!w.prog.deferred_tests.contains_key(&test) && matches!(w.prog.tests[test.idx()], TypeTest::Value(_)));
            assert!(w.prog.leaf_tests.contains_key(&test) || !w.records_expansions());
            for e in w.prog.descendants(other).collect::<Vec<_>>() {
                if let Some(t) = copy.types.get(e) {
                    assert!(!w.types.contains_error(t));
                }
            }
        });
    }

    /// A copy on demand leaves the reduced match's cases for their demands, renaming none of
    /// their binders; the case demanded is copied with its type variable's solution, in the
    /// specialised terms throughout (its deferred call, its binder's signature, its test, a leaf
    /// test of the expansion), and the other case stays the stored node, never copied.
    #[test]
    fn a_case_demanded_is_copied_in_the_specialised_terms() {
        crate::with_typed_program("demand", CASES, &[], |w| {
            let (sym, def) = record_of(w, "pick");
            let t = w.sig_of(sym).tparams[0];
            let bindings = Bindings { args: FxMap::default(), subst: vec![(t, ANY)], paths: Vec::new(), this: None, this_paths: Vec::new(), mark_leaves: false };
            let index = Arc::new(w.stored_index(&def));
            let (mut demand, mut inst) = w.begin_demand(&def, index, bindings.clone(), true);
            let root = w.demand_roots(&mut demand, &def, &mut inst, &[def.body.unwrap()], None)[0];
            let fresh = inst.tparams.iter().find(|(p, _)| *p == def.pattern_tparams[0]).map(|&(_, f)| f).expect("the fresh variable");
            let reduced = w.prog.descendants(root).find(|&e| matches!(w.prog.expr(e), TExpr::Match(..))).expect("the match");
            assert_eq!(inst.reducible, vec![reduced]);
            let TExpr::Match(_, cases) = w.prog.expr(reduced) else { unreachable!() };
            let (case, other) = (w.prog.case_list(cases)[0].body, w.prog.case_list(cases)[1].body);
            assert!(demand.is_left(case) && demand.is_left(other));
            assert!(inst.renames.is_empty(), "a binder of a part left renamed");
            let one = w.types.lit(LitVal::Int(1));
            let copy = w.demand_roots(&mut demand, &def, &mut inst, &[case], Some(&vec![(fresh, one)]))[0];
            assert!(copy != case && !demand.is_left(case) && demand.is_left(other));
            let calls: Vec<TExprId> = w.prog.descendants(copy).filter(|e| inst.deferred.contains(e)).collect();
            assert_eq!(calls.len(), 1);
            assert!(w.quote.deferred.get(&calls[0]).unwrap().subst.iter().all(|&(_, t)| t == one), "the call's type argument");
            let none = inst.renames.iter().find(|(&b, _)| w.name_str(w.syms.sym(b).name) == "none").map(|(_, &f)| f).expect("the val renamed");
            let none_ty = w.sig_of(none).ret;
            assert_eq!(w.show(none_ty), "Option[1]");
            let inner = w.prog.descendants(copy).find(|&e| matches!(w.prog.expr(e), TExpr::Match(..))).expect("the inner match");
            let TExpr::Match(_, inner_cases) = w.prog.expr(inner) else { unreachable!() };
            let TPat::Test(test, ty, _) = w.prog.pats[w.prog.case_list(inner_cases)[0].pat.idx()] else { panic!("a type test") };
            assert_eq!(ty, one);
            assert!(matches!(w.prog.tests[test.idx()], TypeTest::Value(_)));
            assert!(w.prog.leaf_tests.contains_key(&test) || !w.records_expansions());
            assert!(!demand.is_left(w.prog.case_list(inner_cases)[0].body), "a plain match's case left");
        });
    }

    const SUMMON_FROM: &str = "
import scala.compiletime.{summonFrom, summonInline}
final class Show[A](val show: A => String)
object Show:
  inline def apply[A]: Show[A] =
    import Instances.given
    summonInline[Show[A]]
object Instances:
  given Show[Int] = new Show(a => s\"int $a\")
  inline given pair[A, B]: Show[(A, B)] = summonFrom {
    case sa: Show[A] =>
      summonFrom {
        case sb: Show[B] => new Show(p => sa.show(p._1) + \", \" + sb.show(p._2))
        case _ => new Show(p => sa.show(p._1) + \", no show\")
      }
    case _ => new Show(_ => \"no show for the first\")
  }
@main def run(): Unit = println(Show[(Int, Int)].show((1, 2)))
";

    /// A `summonFrom` is typed at the definition and stored as a match the walk reduces: the
    /// inline given of the kittens shape, two of them nested, is checked, its cases' binders
    /// are the body's, and a call of it is taken by the walk.
    #[test]
    fn a_summon_from_is_stored_for_the_walk() {
        crate::with_typed_program("summon_from", SUMMON_FROM, &[], |w| {
            let (sym, def) = record_of(w, "pair");
            assert_eq!(def.state, DefinitionState::Checked);
            let kinds: Vec<bool> = def.reducible_sources.iter().map(|s| matches!(s, ReducibleSource::SummonFrom { .. })).collect();
            assert_eq!(kinds, vec![true, true]);
            for &m in &def.reducible {
                let TExpr::Match(_, cases) = w.prog.expr(m) else { panic!("a match") };
                let TPat::Bind(binder, Some(_)) = w.prog.pats[w.prog.case_list(cases)[0].pat.idx()] else { panic!("a binder") };
                assert!(def.binders.contains(&binder));
            }
            assert!(w.substitution_entry(sym).is_ok(), "the walk does not take the given");
        });
    }

    const RETAINED: &str = "
abstract class Formatter:
  def plain(x: Int): String = s\"dynamic $x\"
class Bold extends Formatter:
  inline override def plain(x: Int): String = s\"inline $x\"
@main def run(): Unit =
  val b = Bold()
  println(b.plain(1))
  val a: Formatter = b
  println(a.plain(2))
";

    /// An inline override of a method that is not inline keeps its two roles apart: the method
    /// the walk types for dispatch, and the body the definition check stores, which the walk
    /// takes for its expansions.
    #[test]
    fn a_retained_override_keeps_its_stored_body_apart_from_its_dispatch_method() {
        crate::with_typed_program("retained", RETAINED, &[], |w| {
            let (sym, def) = record_of(w, "plain");
            assert!(w.is_retained_inline(sym));
            assert_eq!(def.state, DefinitionState::Checked);
            let f = *w.fun_of_sym.get(&sym).expect("the dispatch method");
            let dispatch = w.prog.funs[f.idx()].body.expect("the dispatch body");
            assert!(def.body.is_some() && def.body != Some(dispatch));
            assert!(w.substitution_entry(sym).is_ok(), "the walk does not take the override");
        });
    }

    const CLASSES: &str = "
trait Show[A]:
  def show(a: A): String
trait Step:
  def next(i: Int): Int
inline def make[A](label: String): Show[A] = new Show[A]:
  def show(a: A) = label + a
inline def stepper(k: Int): Step = i => i * k
@main def run(): Unit = println(0)
";

    /// The classes a stored body makes are the record's, out of the program's classes, and
    /// their definition check consumes none of the names of the output: an uncalled method
    /// leaves the counters of the anonymous classes' names as they were.
    #[test]
    fn a_stored_body_s_classes_are_the_record_s_and_take_no_name() {
        crate::with_typed_program("classes", CLASSES, &[], |w| {
            for name in ["make", "stepper"] {
                let (sym, def) = record_of(w, name);
                assert_eq!(def.state, DefinitionState::Checked, "{}", name);
                assert_eq!(def.classes.len(), 1, "{}", name);
                let c = def.classes[0].id;
                assert!(w.prog.classes.iter().all(|tc| tc.id != c), "{}'s class among the program's", name);
                assert!(w.substitution_entry(sym).is_ok());
                let span = w.syms.class(c).span;
                assert!(w.anon_sites.keys().all(|&(_, start, _)| start != span.start), "{}'s class took a name", name);
            }
        });
    }

    const PATHS: &str = "
import scala.compiletime.constValue
trait B
object O extends B
inline def value(x: Int): x.type = constValue[x.type]
inline def wrap(x: B): List[x.type] = List(x)
def three(): Int = 3
@main def run(): Unit =
  println(value(3))
  println(wrap(O))
";

    /// A constant argument's parameter has the literal's type in the copy's types, a stable
    /// argument's the argument's path, any other the proxy's, which has the argument's widened
    /// type.
    #[test]
    fn a_parameter_has_the_literal_s_the_path_s_or_the_proxy_s_type() {
        crate::with_typed_program("paths", PATHS, &[], |w| {
            let int = w.b.t_int;
            let (value, def) = record_of(w, "value");
            let x = def.params[0];
            let bind = |w: &mut Worker, arg: TExprId, constant: Option<LitVal>, ty: TypeId| -> (Bindings, SymId) {
                let name = w.syms.sym(x).name;
                let proxy = w.new_local(name, SymKind::Val, ty, Span::default());
                let path = w.parameter_path(arg, constant, proxy);
                let mut args = FxMap::default();
                args.insert(x, w.prog.add(TExpr::Local(proxy)));
                (Bindings { args, subst: Vec::new(), paths: vec![(x, path)], this: None, this_paths: Vec::new(), mark_leaves: true }, proxy)
            };
            let literal = w.prog.add(TExpr::Int(3));
            let (bindings, _) = bind(w, literal, Some(LitVal::Int(3)), int);
            let copy = w.instantiate_definition(&def, &bindings).unwrap();
            let three = w.types.lit(LitVal::Int(3));
            assert_eq!(copy.types.get(copy.root), Some(three));
            let call = copy.deferred[0];
            assert!(w.quote.deferred.get(&call).unwrap().subst.iter().all(|&(_, t)| t == three));
            let Owner::Package(_) = w.syms.sym(value).owner else { panic!("top level") };
            let three_fn = w.syms.syms.iter().position(|s| w.name_str(s.name) == "three" && s.owner != Owner::Local).map(|i| SymId(i as u32)).unwrap();
            let computed = w.prog.add(TExpr::CallStatic(three_fn, crate::ast::ListRef::EMPTY));
            let (bindings, proxy) = bind(w, computed, None, int);
            let copy = w.instantiate_definition(&def, &bindings).unwrap();
            let proxy_path = w.types.mk(Type::Term(proxy));
            assert_eq!(copy.types.get(copy.root), Some(proxy_path));
            assert_eq!(w.sig_of(proxy).ret, int);
            let (_, wrap) = record_of(w, "wrap");
            let o = w.syms.syms.iter().position(|s| w.name_str(s.name) == "O" && matches!(s.kind, SymKind::Object(_))).map(|i| SymId(i as u32)).unwrap();
            let SymKind::Object(oc) = w.syms.sym(o).kind else { unreachable!() };
            let module = w.prog.add(TExpr::Module(oc));
            let y = wrap.params[0];
            let otype = w.syms.this_type(oc);
            let name = w.syms.sym(y).name;
            let proxy = w.new_local(name, SymKind::Val, otype, Span::default());
            let path = w.parameter_path(module, None, proxy);
            let mut args = FxMap::default();
            args.insert(y, w.prog.add(TExpr::Local(proxy)));
            let copy = w.instantiate_definition(&wrap, &Bindings { args, subst: Vec::new(), paths: vec![(y, path)], this: None, this_paths: Vec::new(), mark_leaves: true }).unwrap();
            let root = copy.types.get(copy.root).unwrap();
            assert!(w.mentions_term(root, proxy), "an object's parameter is its proxy's path: {}", w.show(root));
        });
    }

    /// A build that keeps types gives the record the same table, and the capture leaves the
    /// program's table as it was.
    #[test]
    fn the_record_keeps_the_types_the_program_keeps() {
        crate::with_typed_program("typed_jvm", TYPED, &["--target", "jvm"], |w| {
            assert!(w.prog.record_types);
            let (_, def) = record_of(w, "f");
            for e in stored_nodes(w, &def) {
                assert!(def.node_types.contains_key(&e));
                if let Some(t) = w.prog.type_of(e) {
                    assert_eq!(def.node_types[&e], w.zonk(t));
                }
            }
            assert_eq!(w.prog.types_aside, 0);
        });
    }
}
