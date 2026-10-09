//! The typer's side of the capture of the bodies' typed form (`tir::capture`): the hooks the
//! elaboration sites call where a lowering loses what scalac's
//! pickle holds, and the capture's life: finalised when the typing is over (its types zonked,
//! the copies of other workers' nodes given their records), renumbered by the workers' merge,
//! dropped per file by a retype. Each hook tests the flag in line and records out of line, so
//! that a build without the flag pays a load and a branch per site.

use super::Worker;
use crate::intern::FxMap;
use crate::source::FileId;
use crate::tir::capture::{Capture, Form, InlineCall, Key, Local, PatForm, Wrap};
use crate::tir::{TExpr, TExprId, TPat, TPatId};
use crate::types::*;

impl<'a> Worker<'a> {
    /// Sets the capture for the build: the product modes', whose owned files (`owned`, by file,
    /// all of the program's when the build names no roots) are the ones whose bodies it keeps.
    /// The types of every expression are kept with it.
    pub fn enable_capture(&mut self, owned: Option<&[bool]>) {
        let n = self.files.len();
        let annotated: Vec<bool> = (0..n)
            .map(|f| {
                let file = FileId(f as u32);
                self.is_program_file(file) && !self.in_jar(file)
            })
            .collect();
        let owned: Vec<bool> = annotated.iter().enumerate().map(|(f, &a)| a && owned.map_or(true, |o| o.get(f).copied().unwrap_or(false))).collect();
        self.prog.capture = Some(Box::new(Capture::new(owned, annotated)));
        self.prog.record_types = true;
    }

    /// Whether the build captures the bodies' typed form.
    #[inline(always)]
    pub(crate) fn capturing(&self) -> bool {
        self.prog.capture.is_some()
    }

    /// Types the annotations of a program file's definition where `owner` sees them, for its
    /// pickle (`Capture::annotations`). Quietly, as teq reports no unknown annotation: one that
    /// does not type keeps no tree and leaves no diagnostic of its own; what typing it reports
    /// elsewhere (a definition it completes) stays.
    #[cold]
    #[inline(never)]
    pub(super) fn capture_annotations(&mut self, file: FileId, owner: crate::symbols::Owner, at: u32, annots: &[crate::ast::Annot], tparams: &[(crate::intern::Name, TParamId)]) {
        if annots.is_empty() || !self.prog.capture.as_deref().map_or(false, |c| c.annotates(file)) {
            return;
        }
        let env = self.env_at(file, owner, at);
        // scalac's dependency extraction reads no annotation of a definition.
        let deps = self.deps.take();
        let outer = std::mem::replace(&mut self.typing_annotation, true);
        self.with_env(env, |t| {
            if !tparams.is_empty() {
                t.env.frames.push(super::Frame::Locals { names: Vec::new(), tparams: tparams.to_vec(), givens: Vec::new(), classes: Vec::new(), aliases: Vec::new() });
            }
            for a in annots {
                let mark = t.diags.items.len();
                let frame = t.var_frame_begin();
                let pending = t.attempts.pending_len();
                let te = match t.java_annotation_class(a.instance) {
                    Some(c) => t.type_java_annotation(c, a.instance),
                    None => Some(t.type_expr(a.instance, None).0),
                };
                // Its plain inline calls expand before what it reported is sorted (4.4).
                if t.attempts.pending_len() > pending {
                    t.flush_pending_from(pending);
                }
                t.var_frame_end(frame);
                let span = t.ast(file).expr_span(a.instance);
                let own = |d: &crate::source::Diagnostic| d.file == file && d.span.start >= span.start && d.span.end <= span.end;
                let typed = !t.diags.items[mark..].iter().any(|d| own(d) && !d.is_warning);
                let elsewhere: Vec<crate::source::Diagnostic> = t.diags.items.drain(mark..).filter(|d| !own(d)).collect();
                t.diags.items.extend(elsewhere);
                if let (true, Some(te), Some(c)) = (typed, te, t.prog.capture.as_deref_mut()) {
                    c.annotations.insert((file, a.instance), te);
                }
            }
        });
        self.typing_annotation = outer;
        self.deps = deps;
    }

    /// Runs `f`, the typing of a definition an annotation's typing may ask for on demand (a body
    /// whose result is inferred, a class's check, an inline method's definition), outside the
    /// annotation: its flags say nothing of the definition's own trees.
    pub(super) fn outside_annotation<R>(&mut self, f: impl FnOnce(&mut Self) -> R) -> R {
        if !self.typing_annotation && !self.annotation_args {
            return f(self);
        }
        let typing = std::mem::take(&mut self.typing_annotation);
        let args = std::mem::take(&mut self.annotation_args);
        let r = f(self);
        self.typing_annotation = typing;
        self.annotation_args = args;
        r
    }

    /// The Java annotation interface the `new` of `e` names, where it names one.
    pub(super) fn java_annotation_class(&mut self, e: crate::ast::ExprId) -> Option<ClassId> {
        let crate::ast::Expr::New(ty, _) = self.cur_ast().expr(e) else { return None };
        let resolved = self.resolve_type_ctor(ty);
        match self.types.get(resolved) {
            Type::Class(c, _) | Type::Ctor(c) if self.syms.class(c).mods & crate::ast::mods::JAVA_ANNOTATION != 0 => Some(c),
            _ => None,
        }
    }

    /// A Java annotation's `new`, which constructs no instance: each argument typed against the
    /// element of its name (a lone positional one is `value`'s), a nested annotation likewise.
    pub(super) fn type_java_annotation(&mut self, c: ClassId, e: crate::ast::ExprId) -> Option<TExprId> {
        use crate::ast::Expr;
        let ast = self.cur_ast();
        let Expr::New(_, args) = ast.expr(e) else { return None };
        let args: Vec<crate::ast::ExprId> = ast.expr_list(args).to_vec();
        self.complete_class(c);
        let mut typed = Vec::with_capacity(args.len());
        for &arg in &args {
            let (name, value) = match ast.expr(arg) {
                Expr::NamedArg(n, v) => (n, v),
                _ if args.len() == 1 => (self.interner.intern("value"), arg),
                _ => return None,
            };
            let element = self.own_alternative(c, name, 0)?;
            let ty = self.sig_of(element).ret;
            let te = match self.java_annotation_class(value) {
                Some(k) => self.type_java_annotation(k, value)?,
                None => self.check_expr(value, ty),
            };
            self.capture_wrap(te, Wrap::Named(name));
            typed.push(te);
        }
        let list = self.prog.list(&typed);
        let te = self.prog.add(TExpr::New(c, list));
        self.capture_form(te, Form::JavaAnnotation);
        Some(te)
    }

    /// The annotations of the definition `d` (its own unless `own` is false: a class's are its
    /// check's), of its type parameters and of its parameters, as `owner` sees them.
    pub(super) fn capture_def_annotations(&mut self, file: FileId, owner: crate::symbols::Owner, d: crate::ast::DefId, own: bool) {
        use crate::ast::DefKind;
        if !self.prog.capture.as_deref().map_or(false, |c| c.annotates(file)) {
            return;
        }
        let ast = self.ast(file);
        let def = ast.def(d);
        let at = def.span.start;
        if own {
            self.capture_annotations(file, owner, at, &def.annots, &[]);
        }
        let (tparams, clauses): (&[crate::ast::TypeParam], &[crate::ast::ParamClause]) = match &def.kind {
            DefKind::Fun(f) => (&f.tparams, &f.clauses),
            DefKind::Class(c) => (&c.tparams, &c.clauses),
            DefKind::Given(g) => (&g.tparams, &g.clauses),
            DefKind::TypeAlias { tparams, .. } => (tparams, &[]),
            DefKind::Val { .. } => (&[], &[]),
        };
        // A method's parameters' annotations see its type parameters, as scalac's `annotContext`
        // lets them; a class's are in its own scope already.
        let annotated = tparams.iter().any(|tp| !tp.annots.is_empty()) || clauses.iter().flat_map(|c| c.params.iter()).any(|p| !ast.param_annots(p).is_empty());
        if !annotated {
            return;
        }
        let scope: Vec<(crate::intern::Name, TParamId)> = match (&def.kind, self.def_syms.get(file.0 as usize, &d).copied()) {
            (DefKind::Fun(_) | DefKind::Given(_), Some(s)) => {
                let ids = self.sig_of(s).tparams.clone();
                if ids.len() == tparams.len() {
                    tparams.iter().map(|tp| tp.name).zip(ids).collect()
                } else {
                    Vec::new()
                }
            }
            _ => Vec::new(),
        };
        for tp in tparams {
            self.capture_annotations(file, owner, at, &tp.annots, &scope);
        }
        for p in clauses.iter().flat_map(|c| c.params.iter()) {
            self.capture_annotations(file, owner, at, ast.param_annots(p), &scope);
        }
    }

    /// The capture, when the file being typed is an owned one and what is typed is part of
    /// its pickle: not inside the expansion of an ordinary inline method, which scalac's pickle
    /// holds as the call (a transparent one's expansion it holds). A quote's body is captured
    /// whatever file it is in, a library's included: a transparent macro's expansion copies it
    /// into an owned body.
    fn capture_of_unit(&mut self) -> Option<(FileId, &mut Capture)> {
        let unit = self.typing_unit();
        let pickled = self.inline.sites.iter().all(|s| self.syms.sym(s.callee).mods & crate::ast::mods::TRANSPARENT != 0);
        let quoted = self.quote.level > 0;
        let annotation = self.typing_annotation;
        let c = self.prog.capture.as_deref_mut()?;
        (pickled && (quoted || c.owns(unit) || annotation)).then_some((unit, c))
    }

    /// The class `c`, made while inline methods expand: not in the pickle where one of them is
    /// an ordinary one.
    #[cold]
    #[inline(never)]
    pub(super) fn capture_expansion_class(&mut self, c: ClassId) {
        let ordinary = self.inline.sites.iter().any(|s| self.syms.sym(s.callee).mods & crate::ast::mods::TRANSPARENT == 0);
        let unit = self.typing_unit();
        if let (true, Some(capture)) = (ordinary, self.prog.capture.as_deref_mut()) {
            capture.expansion_class(unit, c);
        }
    }

    /// The type arguments of the call `te` of a method or constructor with type parameters.
    #[cold]
    #[inline(never)]
    pub(super) fn capture_targs(&mut self, te: TExprId, targs: &[TypeId]) {
        if targs.is_empty() || self.capture_of_unit().is_none() {
            return;
        }
        // Zonked before they are interned: a list naming a variable the application solved
        // already would be a list of its own in every build of a session.
        let zonked: Vec<TypeId> = targs.iter().map(|&t| if self.types.has_vars(t) { self.zonk(t) } else { t }).collect();
        let list = self.types.list(&zonked);
        if let Some((unit, c)) = self.capture_of_unit() {
            c.targs(unit, te, list);
        }
    }

    #[cold]
    #[inline(never)]
    pub(crate) fn capture_form(&mut self, te: TExprId, f: Form) {
        if let Some((unit, c)) = self.capture_of_unit() {
            c.form(unit, te, f);
        }
    }

    /// The tree of a converted pickle the node `te` was typed from.
    #[cold]
    #[inline(never)]
    pub(super) fn capture_tree(&mut self, te: TExprId, file: u32, addr: u32) {
        if let Some((unit, c)) = self.capture_of_unit() {
            c.tree(unit, te, (file, addr));
        }
    }

    /// The receiver `r` the node `te` reached its member through and leaves out.
    #[cold]
    #[inline(never)]
    pub(super) fn capture_receiver(&mut self, te: TExprId, r: TExprId) {
        if let Some((unit, c)) = self.capture_of_unit() {
            c.receiver(unit, te, r);
        }
    }

    #[cold]
    #[inline(never)]
    pub(super) fn capture_wrap(&mut self, te: TExprId, w: Wrap) {
        if let Some((unit, c)) = self.capture_of_unit() {
            c.wrap(unit, te, w);
        }
    }

    #[cold]
    #[inline(never)]
    pub(super) fn capture_pat(&mut self, p: TPatId, f: PatForm) {
        if let Some((unit, c)) = self.capture_of_unit() {
            c.pat(unit, p, f);
        }
    }

    /// What the local `s` lost, changed by `f`.
    #[cold]
    #[inline(never)]
    pub(super) fn capture_local(&mut self, s: SymId, f: impl FnOnce(&mut Local)) {
        if let Some((unit, c)) = self.capture_of_unit() {
            f(c.local(unit, s));
        }
    }

    /// The call of a tuple's builtin member `name` that gave `te`.
    #[cold]
    #[inline(never)]
    pub(super) fn capture_builtin_call(&mut self, te: TExprId, name: crate::intern::Name, recv: TExprId, args: Vec<TExprId>) {
        if let Some((unit, c)) = self.capture_of_unit() {
            c.builtin_call(unit, te, crate::tir::capture::BuiltinCall { name, recv, args });
        }
    }

    /// The evidence the call `te` of a `@jvmEvidence` member passes where its signature erases it.
    #[cold]
    #[inline(never)]
    pub(super) fn capture_evidence(&mut self, te: TExprId, evidence: Vec<TExprId>) {
        if let Some((unit, c)) = self.capture_of_unit() {
            c.evidence(unit, te, evidence);
        }
    }

    /// The call an inline expansion that gave `te` stands for.
    #[cold]
    #[inline(never)]
    pub(super) fn capture_inline_call(&mut self, te: TExprId, call: InlineCall) {
        if let Some((unit, c)) = self.capture_of_unit() {
            c.inline_call(unit, te, call);
        }
    }

    /// The classes the block `block` defines, which the IR lifts out of it (`Capture::block_classes`).
    #[cold]
    #[inline(never)]
    pub(super) fn capture_block_classes(&mut self, block: TExprId, classes: Vec<ClassId>) {
        if let Some((unit, c)) = self.capture_of_unit() {
            c.block_classes(unit, block, classes);
        }
    }

    /// `to`, a copy of `from` with the same symbols and types, takes its records.
    #[cold]
    #[inline(never)]
    pub(super) fn capture_copy(&mut self, from: TExprId, to: TExprId) {
        if let Some((unit, c)) = self.capture_of_unit() {
            c.copy(unit, from, to);
        }
    }

    /// Publishes the records of the quote's body typed since `mark` for every worker's copier:
    /// of the workers typing apart, and of those of a later fork, which start with none of
    /// this worker's records.
    #[cold]
    #[inline(never)]
    pub(super) fn capture_publish(&mut self, mark: Option<CaptureMark>) {
        if let (Some(mark), Some(c)) = (mark, self.prog.capture.as_deref_mut()) {
            c.publish(mark.unit, mark.at);
        }
    }

    /// `to`, a copy of `from` whose symbols and types a copier renamed, takes its records
    /// renamed alike.
    #[cold]
    #[inline(never)]
    pub(super) fn capture_copy_renamed(
        &mut self,
        from: TExprId,
        to: TExprId,
        ty: &dyn Fn(&mut Worker<'a>, TypeId) -> TypeId,
        sym: &dyn Fn(SymId) -> SymId,
        class: &dyn Fn(ClassId) -> Option<ClassId>,
    ) {
        let Some((_, c)) = self.capture_of_unit() else { return };
        // A block's copy defines the copies of its classes the copier made.
        if let Some(classes) = c.classes_of_block(from) {
            let copies: Vec<ClassId> = classes.into_iter().filter_map(class).collect();
            self.capture_block_classes(to, copies);
        }
        let Some((_, c)) = self.capture_of_unit() else { return };
        let records = c.records_of(from);
        if records.is_empty() {
            return;
        }
        let (targs, form, wraps) = (records.targs, records.form, Some(records.wraps));
        if let Some(l) = targs {
            let items: Vec<TypeId> = self.types.items(self.types.import_list(l)).to_vec();
            let mapped: Vec<TypeId> = items.into_iter().map(|t| ty(self, t)).collect();
            self.capture_targs(to, &mapped);
        }
        if let Some(f) = form {
            let types = self.types.clone();
            let f = map_form(f, &types, &mut |t| ty(self, t), sym);
            self.capture_form(to, f);
        }
        for w in wraps.into_iter().flatten() {
            let w = map_wrap(w, &mut |t| ty(self, t));
            self.capture_wrap(to, w);
        }
        if let Some((file, addr)) = records.tree {
            self.capture_tree(to, file, addr);
        }
    }

    /// `id`, what the copier put for the hole or the parameter `e` (a copy of the tree that
    /// fills it), takes the layers written around `e` outside its own.
    #[cold]
    #[inline(never)]
    pub(super) fn capture_copy_layers(&mut self, e: TExprId, id: TExprId, ty: &dyn Fn(&mut Worker<'a>, TypeId) -> TypeId) {
        let Some((_, c)) = self.capture_of_unit() else { return };
        for w in c.records_of(e).wraps {
            let w = map_wrap(w, &mut |t| ty(self, t));
            self.capture_wrap(id, w);
        }
    }

    /// The check of a copy the quote copier made: every record of a node of the copied trees
    /// (`roots`, the body and the trees put for its holes and parameters, walked by
    /// `descendants` and not by the copier, with the subtrees the records name), of their
    /// patterns and of the binders renamed (`locals`, each with its fresh local) is on the copy
    /// (`copies`, `pats`). What is not counts as lost, which the census reports missing.
    #[cold]
    #[inline(never)]
    pub(super) fn capture_check_copy(&mut self, roots: &[TExprId], copies: &[(TExprId, TExprId)], pats: &[(TPatId, TPatId)], locals: &[(SymId, SymId)]) {
        if self.capture_of_unit().is_none() {
            return;
        }
        let c = self.prog.capture.as_deref().expect("capturing");
        let mut dests: FxMap<TExprId, Vec<TExprId>> = FxMap::default();
        for &(from, to) in copies {
            dests.entry(from).or_default().push(to);
        }
        let made: FxMap<TExprId, ()> = copies.iter().map(|&(_, to)| (to, ())).collect();
        let mut pat_dests: FxMap<TPatId, Vec<TPatId>> = FxMap::default();
        for &(from, to) in pats {
            pat_dests.entry(from).or_default().push(to);
        }
        let kind = |w: &Wrap| std::mem::discriminant(w);
        let mut checked = 0u64;
        let mut lost: Vec<String> = Vec::new();
        let place = |w: &Worker, e: Option<TExprId>, what: &str| -> String {
            let at = e.and_then(|e| w.prog.span_of(e)).map_or("-".to_string(), |(f, sp)| {
                let src = w.source(f);
                format!("{}:{}", src.path, crate::source::locate(&src.text, sp.start as usize).0)
            });
            format!("{}\t{}\t-", at, what)
        };
        let mut todo: Vec<TExprId> = roots.to_vec();
        let mut seen: FxMap<TExprId, ()> = FxMap::default();
        let mut source_pats: Vec<TPatId> = Vec::new();
        while let Some(root) = todo.pop() {
            for e in self.prog.descendants(root) {
                if seen.insert(e, ()).is_some() {
                    continue;
                }
                match self.prog.expr(e) {
                    TExpr::Match(_, cases) => source_pats.extend(self.prog.cases[cases.range()].iter().map(|k| k.pat)),
                    TExpr::Try(i) => {
                        let cases = self.prog.tries[i as usize].cases;
                        source_pats.extend(self.prog.cases[cases.range()].iter().map(|k| k.pat));
                    }
                    _ => {}
                }
                let from = c.records_of(e);
                if from.is_empty() {
                    continue;
                }
                if let Some(call) = &from.builtin {
                    todo.push(call.recv);
                    todo.extend(&call.args);
                }
                todo.extend(&from.evidence);
                for call in &from.inline_calls {
                    todo.extend(call.recv);
                    todo.extend(&call.args);
                }
                // Every copy of the node, one per time the copier met it, carries its records.
                let tos: Vec<Option<TExprId>> = match dests.get(&e) {
                    Some(tos) => tos.iter().map(|&to| Some(to)).collect(),
                    None => vec![None],
                };
                // A tree a call record names under the node is named under each copy of it;
                // one elsewhere is one the copy made.
                let under: FxMap<TExprId, ()> = if from.builtin.is_some() || !from.inline_calls.is_empty() || !from.evidence.is_empty() {
                    self.prog.descendants(e).map(|x| (x, ())).collect()
                } else {
                    FxMap::default()
                };
                let inside = |to: TExprId, source: &[TExprId], copied: &[TExprId]| -> bool {
                    let below: FxMap<TExprId, ()> = self.prog.descendants(to).map(|x| (x, ())).collect();
                    source.len() == copied.len() && source.iter().zip(copied).all(|(s, k)| if under.contains_key(s) { below.contains_key(k) } else { made.contains_key(k) })
                };
                let refs = |call: &InlineCall| -> Vec<TExprId> { call.recv.iter().chain(&call.args).copied().collect() };
                for to in tos {
                    let mut check = |held: bool, on_copy: &dyn Fn(TExprId) -> bool, what: &str| {
                        if !held {
                            return;
                        }
                        match to {
                            Some(to) if on_copy(to) => checked += 1,
                            _ => lost.push(place(self, Some(e), what)),
                        }
                    };
                    check(from.targs.is_some(), &|to| c.targs.contains_key(&to), "type arguments");
                    check(from.receiver.is_some(), &|to| c.receivers.contains_key(&to), "receiver");
                    check(from.tree.is_some(), &|to| c.trees.contains_key(&to), "tree");
                    let form = from.form.as_ref().map(std::mem::discriminant);
                    check(form.is_some(), &|to| c.forms.get(&to).map(std::mem::discriminant) == form, "form");
                    let ws = &from.wraps;
                    for w in ws {
                        let n = ws.iter().filter(|x| kind(x) == kind(w)).count();
                        check(true, &|to| c.wraps.get(&to).map_or(0, |t| t.iter().filter(|x| kind(x) == kind(w)).count()) >= n, "layer");
                    }
                    let calls = from.inline_calls.len();
                    check(calls > 0, &|to| {
                        let copied: &[InlineCall] = c.inline_calls.get(&to).map_or(&[], |v| v.as_slice());
                        copied.len() >= calls && from.inline_calls.iter().zip(copied).all(|(s, k)| inside(to, &refs(s), &refs(k)))
                    }, "inline call");
                    check(from.builtin.is_some(), &|to| match (&from.builtin, c.builtin_calls.get(&to)) {
                        (Some(s), Some(k)) => {
                            let source: Vec<TExprId> = std::iter::once(s.recv).chain(s.args.iter().copied()).collect();
                            let copied: Vec<TExprId> = std::iter::once(k.recv).chain(k.args.iter().copied()).collect();
                            inside(to, &source, &copied)
                        }
                        _ => false,
                    }, "builtin call");
                    check(!from.evidence.is_empty(), &|to| c.evidence.get(&to).map_or(false, |k| inside(to, &from.evidence, k)), "evidence");
                }
            }
        }
        while let Some(p) = source_pats.pop() {
            match self.prog.pats[p.idx()] {
                TPat::Wildcard | TPat::Equals(..) => {}
                TPat::Bind(_, inner) => source_pats.extend(inner),
                TPat::Test(_, _, inner) | TPat::Unapply(_, _, inner) => source_pats.push(inner),
                TPat::Class(_, _, _, subs) | TPat::Alt(subs) => source_pats.extend_from_slice(&self.prog.pat_lists[subs.range()]),
                TPat::Seq(items, rest) => {
                    source_pats.extend_from_slice(&self.prog.pat_lists[items.range()]);
                    source_pats.extend(rest);
                }
            }
            if c.pat_of(p).is_some() {
                match pat_dests.get(&p) {
                    Some(tos) => {
                        for to in tos {
                            if c.pats.contains_key(to) {
                                checked += 1;
                            } else {
                                lost.push(place(self, None, "pattern"));
                            }
                        }
                    }
                    None => lost.push(place(self, None, "pattern")),
                }
            }
        }
        for &(from, to) in locals {
            if c.local_of(from).is_some() {
                if c.locals.contains_key(&to) {
                    checked += 1;
                } else {
                    let name = self.name_str(self.syms.sym(from).name).to_string();
                    lost.push(place(self, None, &format!("local {}", name)));
                }
            }
        }
        let c = self.prog.capture.as_deref_mut().expect("capturing");
        c.copies_checked += checked;
        for at in lost {
            c.lost(at);
        }
    }

    /// A record of `e` a copy could not carry.
    #[cold]
    #[inline(never)]
    pub(super) fn capture_lost(&mut self, e: TExprId, what: &str) {
        if self.capture_of_unit().is_none() {
            return;
        }
        let at = self.prog.span_of(e).map_or("-".to_string(), |(f, sp)| {
            let src = self.source(f);
            format!("{}:{}", src.path, crate::source::locate(&src.text, sp.start as usize).0)
        });
        if let Some(c) = self.prog.capture.as_deref_mut() {
            c.lost(format!("{}\t{}\t-", at, what));
        }
    }

    /// The node `e` given the node of `expanded`, an expansion of the call `e` was, takes its
    /// records (`Capture::moved`).
    #[cold]
    #[inline(never)]
    pub(super) fn capture_moved(&mut self, expanded: TExprId, e: TExprId) {
        if let Some((unit, c)) = self.capture_of_unit() {
            c.moved(unit, expanded, e);
        }
    }

    /// The interpolation part at `i` of the current file's `str_lists` as written (its source
    /// by the span the parser kept), as scalac's scanner keeps it; the processed string where no
    /// span is kept (a library body's).
    pub(super) fn written_part(&self, i: usize) -> String {
        let ast = self.cur_ast();
        match ast.part_span(i) {
            Some(span) => written_part(&self.source(self.env.file).text[span.start as usize..span.end as usize]),
            None => ast.str(ast.str_lists[i]).to_string(),
        }
    }

    /// The string literals `items` a `StringContext` is made of for the parts `parts` (of a
    /// pattern or of an interpolator of a library): each part written otherwise than its
    /// literal holds (an `s` pattern's escapes processed) takes its text as written.
    #[cold]
    #[inline(never)]
    pub(super) fn capture_written_parts(&mut self, parts: crate::ast::ListRef, items: &[TExprId]) {
        let ast = self.cur_ast();
        for (i, &item) in parts.range().zip(items) {
            let written = self.written_part(i);
            if written != ast.str(ast.str_lists[i]) {
                let lit = self.types.lit(LitVal::Str(self.interner.intern(&written)));
                self.capture_form(item, Form::Written(lit));
            }
        }
    }

    /// `to`, a copy of the pattern `from` whose types a copier renamed, takes its record renamed
    /// alike.
    #[cold]
    #[inline(never)]
    pub(super) fn capture_copy_pat(&mut self, from: TPatId, to: TPatId, ty: &dyn Fn(&mut Worker<'a>, TypeId) -> TypeId) {
        let Some((_, c)) = self.capture_of_unit() else { return };
        let Some(f) = c.pat_of(from) else { return };
        let f = match f {
            PatForm::Seq(t) => PatForm::Seq(ty(self, t)),
            PatForm::Elements => PatForm::Elements,
        };
        self.capture_pat(to, f);
    }

    /// `fresh`, a local made like `s` for a copy, takes the record of `s`.
    #[cold]
    #[inline(never)]
    pub(super) fn capture_copy_local(&mut self, s: SymId, fresh: SymId) {
        let Some((unit, c)) = self.capture_of_unit() else { return };
        if let Some(l) = c.local_of(s) {
            *c.local(unit, fresh) = l;
        }
    }

    /// The record of the local `s` with its type under `subst`, as its signature's is.
    #[cold]
    #[inline(never)]
    pub(super) fn capture_subst_local(&mut self, s: SymId, subst: &Subst) {
        let Some((_, c)) = self.capture_of_unit() else { return };
        let Some(ty) = c.locals.get(&s).and_then(|l| l.ty) else { return };
        let ty = self.types.subst(ty, subst);
        self.capture_local(s, |l| l.ty = Some(ty));
    }

    /// Whether a tree a record names and the copy does not hold can be copied for the record
    /// alone: one that makes no class, which a copy would add to the program.
    pub(super) fn copyable_for_record(&self, e: TExprId) -> bool {
        self.prog.descendants(e).all(|x| !matches!(self.prog.expr(x), TExpr::New(c, _) if self.syms.class(c).kind == crate::symbols::ClassKind::Anon))
    }

    /// The node `typed` a worker made in the place of `te`, another worker's, to record its
    /// type: it takes the records of `te` once the workers' records are merged.
    #[cold]
    #[inline(never)]
    pub(super) fn capture_forked_copy(&mut self, te: TExprId, typed: TExprId) {
        if let Some((unit, c)) = self.capture_of_unit() {
            c.pending.push((te, typed, unit));
        }
    }

    /// What a retype of `file` leaves of the capture: nothing its bodies' typing made.
    pub(super) fn forget_capture(&mut self, file: FileId) {
        if let Some(c) = self.prog.capture.as_deref_mut() {
            c.forget(file);
        }
    }

    /// The capture once the typing is over: the copies of other workers' nodes take their
    /// records, and every type it holds is zonked, the variables being solved by now.
    pub fn finish_capture(&mut self) {
        self.finish_capture_of(None);
    }

    /// `finish_capture` after a retype of `files`, whose records alone it compacts.
    pub fn finish_capture_of(&mut self, only: Option<&[FileId]>) {
        if let Some(c) = self.prog.capture.as_deref_mut() {
            c.settle_copies();
        } else {
            return;
        }
        let steps = finish_steps();
        finish_measure("settled");
        if steps.compact {
            self.compact_capture(only);
            finish_measure("compacted");
        }
        let Some(mut c) = self.prog.capture.take() else { return };
        if !steps.zonk {
            self.prog.capture = Some(c);
            return;
        }
        let files: Vec<FileId> = match only {
            Some(files) => files.to_vec(),
            None => c.made.keys().copied().collect(),
        };
        // The typing is over, and no attempt is dropped after it.
        match only {
            Some(files) => files.iter().for_each(|f| {
                c.changed.remove(f);
            }),
            None => c.changed.clear(),
        }
        let zonk = |w: &mut Worker<'a>, t: TypeId| if t == crate::tir::NO_TYPE || !w.types.has_vars(t) { t } else { w.zonk(t) };
        for f in files {
            let keys = c.made.get(&f).cloned().unwrap_or_default();
            for k in keys {
                match k {
                    Key::Targs(e) => {
                        if let Some(l) = c.targs.get(&e).copied() {
                            let items: Vec<TypeId> = self.types.items(l).to_vec();
                            if items.iter().any(|&t| self.types.has_vars(t)) {
                                let z: Vec<TypeId> = items.into_iter().map(|t| zonk(self, t)).collect();
                                c.targs.insert(e, self.types.list(&z));
                            }
                        }
                    }
                    Key::Form(e) => {
                        if let Some(f) = c.forms.get_mut(&e) {
                            let types = self.types.clone();
                            *f = map_form(*f, &types, &mut |t| zonk(self, t), &|s| s);
                        }
                    }
                    Key::Wrap(e) => {
                        for w in c.wraps.get_mut(&e).into_iter().flatten() {
                            *w = map_wrap(*w, &mut |t| zonk(self, t));
                        }
                    }
                    Key::Inline(e) => {
                        for call in c.inline_calls.get_mut(&e).into_iter().flatten() {
                            let items: Vec<TypeId> = self.types.items(call.targs).to_vec();
                            if items.iter().any(|&t| self.types.has_vars(t)) {
                                let z: Vec<TypeId> = items.into_iter().map(|t| zonk(self, t)).collect();
                                call.targs = self.types.list(&z);
                            }
                        }
                    }
                    Key::Builtin(_) | Key::Receiver(_) | Key::Evidence(_) | Key::Tree(_) => {}
                    Key::Pat(p) => {
                        if let Some(PatForm::Seq(t)) = c.pats.get_mut(&p) {
                            *t = zonk(self, *t);
                        }
                    }
                    Key::Local(s) => {
                        if let Some(l) = c.locals.get_mut(&s) {
                            l.ty = l.ty.map(|t| zonk(self, t));
                        }
                    }
                    Key::Class(_) | Key::BlockClasses(_) => {}
                }
            }
            if let Some(keys) = c.made.get(&f) {
                c.sync_published(keys);
            }
        }
        self.prog.capture = Some(c);
        finish_measure("zonked");
    }
}

/// An interpolation's part as scalac's scanner keeps it (`Scanners.getStringPart`): as written,
/// but for `$$` and `$"`, which stand for `$` and `"`, and a unicode escape (`\uXXXX` after an
/// even run of backslashes, as the reader decodes one anywhere); an `s` interpolation processes
/// the other escapes where it runs.
pub(crate) fn written_part(text: &str) -> String {
    let b: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    let unicode = |at: usize| -> Option<(u32, usize)> {
        let mut run = 0;
        while at > run && b[at - run - 1] == '\\' {
            run += 1;
        }
        let mut j = at + 1;
        if run % 2 != 0 || b.get(j) != Some(&'u') {
            return None;
        }
        while b.get(j) == Some(&'u') {
            j += 1;
        }
        let digits = b.get(j..j + 4)?;
        if !digits.iter().all(|d| d.is_ascii_hexdigit()) {
            return None;
        }
        let code = digits.iter().fold(0, |n, d| n * 16 + d.to_digit(16).unwrap_or(0));
        Some((code, j + 4))
    };
    while i < b.len() {
        match b[i] {
            '\\' => match unicode(i) {
                Some((high @ 0xD800..=0xDBFF, next)) => match (b.get(next) == Some(&'\\')).then(|| unicode(next)).flatten() {
                    Some((low @ 0xDC00..=0xDFFF, after)) => {
                        out.push(char::from_u32(0x10000 + ((high - 0xD800) << 10) + (low - 0xDC00)).unwrap_or('\u{fffd}'));
                        i = after;
                    }
                    _ => {
                        out.push('\u{fffd}');
                        i = next;
                    }
                },
                Some((code, next)) => {
                    out.push(char::from_u32(code).unwrap_or('\u{fffd}'));
                    i = next;
                }
                None => {
                    out.push('\\');
                    i += 1;
                }
            },
            '$' if matches!(b.get(i + 1), Some('$') | Some('"')) => {
                out.push(b[i + 1]);
                i += 2;
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    out
}

/// The form with its types and symbols mapped.
pub(super) fn map_form(f: Form, types: &TypeStore, ty: &mut dyn FnMut(TypeId) -> TypeId, sym: &dyn Fn(SymId) -> SymId) -> Form {
    let list = |l: TList, ty: &mut dyn FnMut(TypeId) -> TypeId| {
        let items: Vec<TypeId> = types.items(types.import_list(l)).iter().map(|&t| ty(t)).collect();
        types.list(&items)
    };
    match f {
        Form::Repeated(t) => Form::Repeated(ty(t)),
        Form::Cast(t) => Form::Cast(ty(t)),
        Form::Test(t) => Form::Test(ty(t)),
        Form::Evidence(t) => Form::Evidence(ty(t)),
        Form::Pack(l) => Form::Pack(list(l, ty)),
        Form::Unpack(l) => Form::Unpack(list(l, ty)),
        Form::Interp { kind, parts } => Form::Interp { kind, parts: list(parts, ty) },
        Form::Written(t) => Form::Written(ty(t)),
        Form::Member(s) => Form::Member(sym(s)),
        Form::Constant(s) => Form::Constant(sym(s)),
        Form::Return(s) => Form::Return(sym(s)),
        Form::GivenCall(s) => Form::GivenCall(sym(s)),
        Form::DynamicLiteral(s) => Form::DynamicLiteral(sym(s)),
        Form::JsObject(t) => Form::JsObject(ty(t)),
        Form::ByName
        | Form::Folded(_)
        | Form::Default
        | Form::Promotion
        | Form::Widening
        | Form::Op(_)
        | Form::SwappedOp(_)
        | Form::SuperOp(_)
        | Form::EnumMember(_)
        | Form::Dynamic(_)
        | Form::CaseCopy
        | Form::PartialFunction
        | Form::SpliceQuotes
        | Form::CaseApply
        | Form::JavaAnnotation => f,
    }
}

pub(super) fn map_wrap(w: Wrap, ty: &mut dyn FnMut(TypeId) -> TypeId) -> Wrap {
    match w {
        Wrap::Ascribed(t) => Wrap::Ascribed(ty(t)),
        Wrap::Cast(t) => Wrap::Cast(ty(t)),
        Wrap::Splice(t) => Wrap::Splice(ty(t)),
        Wrap::Unchecked | Wrap::Named(_) | Wrap::Member(..) => w,
    }
}

impl super::merge::Remap {
    /// The capture's records under the merged ids: their keys, and the symbols, classes and
    /// types they hold.
    pub(super) fn capture(&self, types: &TypeStore, c: &mut Capture) {
        // Each table rebuilt in the order of its keys under the new ids, so that the types its
        // records hand the promotion follow the keys (`Remap::entries_in_order`).
        fn moved<K: Copy + Eq + std::hash::Hash + Ord, V>(r: &super::merge::Remap, m: &mut crate::intern::FxMap<K, V>, key: impl Fn(K) -> K, mut value: impl FnMut(&mut V)) {
            let old = std::mem::take(m);
            m.reserve(old.len());
            for (k, mut v) in r.entries_in_order(old, key) {
                value(&mut v);
                m.insert(k, v);
            }
        }
        let e = |x: TExprId| self.map_expr(x);
        let t = |x: TypeId| self.ty(x);
        // No attempt is under way across the merge.
        c.changed.clear();
        {
            let mut p = c.published.write().unwrap_or_else(|x| x.into_inner());
            moved(self, &mut p.targs, e, |l| *l = self.tlist(types, *l));
            moved(self, &mut p.forms, e, |f| *f = map_form(*f, types, &mut |x| t(x), &|s| self.map_sym(s)));
            moved(self, &mut p.wraps, e, |ws| ws.iter_mut().for_each(|w| *w = map_wrap(*w, &mut |x| t(x))));
            moved(self, &mut p.pats, |x| self.pat(x), |f| {
                if let PatForm::Seq(x) = f {
                    *x = t(*x);
                }
            });
            moved(self, &mut p.locals, |s| self.map_sym(s), |l| l.ty = l.ty.map(t));
            moved(self, &mut p.inline_calls, e, |calls| calls.iter_mut().for_each(|call| self.inline_call(types, call)));
            moved(self, &mut p.builtin_calls, e, |call| {
                call.recv = e(call.recv);
                call.args.iter_mut().for_each(|a| *a = e(*a));
            });
            moved(self, &mut p.receivers, e, |r| *r = e(*r));
            moved(self, &mut p.evidence, e, |v| v.iter_mut().for_each(|a| *a = e(*a)));
            moved(self, &mut p.trees, e, |_| {});
            moved(self, &mut p.block_classes, e, |v| v.iter_mut().for_each(|k| *k = self.map_class(*k)));
        }
        moved(self, &mut c.targs, e, |l| *l = self.tlist(types, *l));
        moved(self, &mut c.forms, e, |f| *f = map_form(*f, types, &mut |x| t(x), &|s| self.map_sym(s)));
        moved(self, &mut c.wraps, e, |ws| {
            for w in ws.iter_mut() {
                *w = map_wrap(*w, &mut |x| t(x));
            }
        });
        moved(self, &mut c.pats, |p| self.pat(p), |f| {
            if let PatForm::Seq(x) = f {
                *x = t(*x);
            }
        });
        moved(self, &mut c.locals, |s| self.map_sym(s), |l| l.ty = l.ty.map(t));
        moved(self, &mut c.inline_calls, e, |calls| calls.iter_mut().for_each(|call| self.inline_call(types, call)));
        moved(self, &mut c.expansion_classes, |k| self.map_class(k), |_| {});
        moved(self, &mut c.block_classes, e, |v| v.iter_mut().for_each(|k| *k = self.map_class(*k)));
        moved(self, &mut c.block_stmts, |k| k, |stmts| {
            use crate::tir::capture::BlockStmt;
            use super::ImportTarget as T;
            for (_, _, s) in stmts.iter_mut() {
                match s {
                    BlockStmt::InlineDef(x) => *x = self.map_sym(*x),
                    BlockStmt::Import(i) => {
                        i.target = match i.target {
                            T::ClassMember(k, n) => T::ClassMember(self.map_class(k), n),
                            T::ClassAll(k) => T::ClassAll(self.map_class(k)),
                            T::ClassGivens(k) => T::ClassGivens(self.map_class(k)),
                            other => other,
                        };
                    }
                }
            }
        });
        moved(self, &mut c.builtin_calls, e, |call| {
            call.recv = e(call.recv);
            for a in call.args.iter_mut() {
                *a = e(*a);
            }
        });
        moved(self, &mut c.receivers, e, |r| *r = e(*r));
        moved(self, &mut c.evidence, e, |v| v.iter_mut().for_each(|a| *a = e(*a)));
        moved(self, &mut c.trees, e, |_| {});
        moved(self, &mut c.annotations, |k| k, |a| *a = e(*a));
        for keys in c.made.values_mut() {
            for k in keys.iter_mut() {
                *k = match *k {
                    Key::Targs(x) => Key::Targs(e(x)),
                    Key::Form(x) => Key::Form(e(x)),
                    Key::Wrap(x) => Key::Wrap(e(x)),
                    Key::Inline(x) => Key::Inline(e(x)),
                    Key::Builtin(x) => Key::Builtin(e(x)),
                    Key::Receiver(x) => Key::Receiver(e(x)),
                    Key::Evidence(x) => Key::Evidence(e(x)),
                    Key::Tree(x) => Key::Tree(e(x)),
                    Key::Pat(p) => Key::Pat(self.pat(p)),
                    Key::Local(s) => Key::Local(self.map_sym(s)),
                    Key::Class(k) => Key::Class(self.map_class(k)),
                    Key::BlockClasses(x) => Key::BlockClasses(e(x)),
                };
            }
        }
        for (from, to, _) in c.pending.iter_mut() {
            *from = e(*from);
            *to = e(*to);
        }
    }
}

impl super::merge::Remap {
    fn inline_call(&self, types: &TypeStore, call: &mut InlineCall) {
        call.callee = self.map_sym(call.callee);
        call.recv = call.recv.map(|x| self.map_expr(x));
        for a in call.args.iter_mut() {
            *a = self.map_expr(*a);
        }
        call.targs = self.tlist(types, call.targs);
    }
}

/// The steps of the finalisation `TEQ_CAPTURE_FINISH` leaves in (`no-compaction`, `no-zonk`,
/// `none`: for measuring what each costs in one binary); both by default.
struct FinishSteps {
    compact: bool,
    zonk: bool,
}

fn finish_steps() -> FinishSteps {
    match std::env::var("TEQ_CAPTURE_FINISH").as_deref() {
        Ok("no-compaction") => FinishSteps { compact: false, zonk: true },
        Ok("no-zonk") => FinishSteps { compact: true, zonk: false },
        Ok("none") => FinishSteps { compact: false, zonk: false },
        _ => FinishSteps { compact: true, zonk: true },
    }
}

/// Whether `TEQ_CAPTURE_FINISH` is set: the finalisation then reports its steps.
pub(super) fn finish_trace() -> bool {
    std::env::var_os("TEQ_CAPTURE_FINISH").is_some()
}

/// The footprint and the allocator's holdings after a step of the finalisation, under
/// `TEQ_CAPTURE_FINISH`.
fn finish_measure(step: &str) {
    if !finish_trace() {
        return;
    }
    let h = crate::alloc::held();
    eprintln!(
        "capture finish: {} footprint {} MB, resident {} MB, allocator reserved {} MB, large {} MB (peak {} MB)",
        step,
        crate::alloc::footprint() >> 20,
        crate::alloc::resident_pages().rss >> 20,
        h.reserved >> 20,
        h.large >> 20,
        h.large_peak >> 20
    );
}

/// Whether `TEQ_CAPTURE` asks for the capture in a build of any mode: for the census over the
/// test corpora and for measuring it in a resident session, which the product modes are not.
pub fn forced() -> bool {
    static FORCED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *FORCED.get_or_init(|| std::env::var_os("TEQ_CAPTURE").is_some() || std::env::var_os("TEQ_CAPTURE_CENSUS").is_some())
}

impl<'a> Worker<'a> {
    /// The thunk that passes `te` to a by-name parameter.
    #[inline]
    pub(crate) fn by_name_thunk(&mut self, te: TExprId) -> TExprId {
        let thunk = self.prog.add(TExpr::Lambda(crate::ast::ListRef::EMPTY, te));
        if self.capturing() {
            self.capture_form(thunk, Form::ByName);
        }
        thunk
    }

    /// The `Unit` that stands for an omitted argument of a parameter with a default.
    #[inline]
    pub(super) fn default_placeholder(&mut self) -> TExprId {
        let unit = self.prog.add(TExpr::Unit);
        if self.capturing() {
            self.capture_form(unit, Form::Default);
        }
        unit
    }

    /// Marks the conversions `convert_rank` put between `outer` and `inner` with `form`.
    #[cold]
    #[inline(never)]
    pub(super) fn capture_conversion(&mut self, outer: TExprId, inner: TExprId, form: Form) {
        let mut e = outer;
        while e != inner {
            let TExpr::Unary(_, a) = self.prog.expr(e) else { break };
            self.capture_form(e, form);
            e = a;
        }
    }
}

impl<'a> Worker<'a> {
    /// The types of the tuple scalac packs a comprehension's variables into.
    pub(super) fn pack_tuple(&mut self, vars: &[(crate::intern::Name, TypeId, bool)]) -> Option<TList> {
        let elems: Vec<TypeId> = vars.iter().map(|&(_, t, _)| t).collect();
        Some(self.types.list(&elems))
    }
}

impl<'a> Worker<'a> {
    /// The imports a block of a captured body enters before its `at`-th statement, by the
    /// block's source (`Capture::block_imports`).
    #[cold]
    #[inline(never)]
    pub(super) fn capture_block_imports(&mut self, block: crate::source::Span, at: u32, from: u32, entered: &[super::ResolvedImport]) {
        let stmts: Vec<crate::tir::capture::BlockStmt> = entered.iter().map(|&i| crate::tir::capture::BlockStmt::Import(i)).collect();
        self.capture_block_stmts(block, at, from, stmts);
    }

    /// What a block of a captured body holds before its `at`-th statement that its typed form
    /// leaves out (`Capture::block_stmts`).
    #[cold]
    #[inline(never)]
    pub(super) fn capture_block_stmts(&mut self, block: crate::source::Span, at: u32, from: u32, stmts: Vec<crate::tir::capture::BlockStmt>) {
        let Some((unit, c)) = self.capture_of_unit() else { return };
        let list = c.block_stmts.entry((unit, block.start)).or_default();
        list.retain(|&(a, f, _)| (a, f) != (at, from));
        list.extend(stmts.into_iter().map(|s| (at, from, s)));
    }

    /// The call of the inline method `call.sym` whose expansion gave `e`.
    #[cold]
    #[inline(never)]
    pub(super) fn capture_inline(&mut self, call: &super::apply::MethodCall, subst: &Subst, args: &[TExprId], span: crate::source::Span, e: TExprId) {
        if self.capture_of_unit().is_none() {
            return;
        }
        let own: Vec<TypeId> = subst[call.owner_subst.len().min(subst.len())..].iter().map(|&(_, t)| if self.types.has_vars(t) { self.zonk(t) } else { t }).collect();
        let targs = self.types.list(&own);
        let transparent = self.syms.sym(call.sym).mods & crate::ast::mods::TRANSPARENT != 0;
        let binds = self.inline.last_binds;
        let site = Some((self.env.file, span));
        let record = InlineCall { callee: call.sym, recv: call.recv, args: args.to_vec(), targs, transparent, binds, site };
        self.capture_inline_call(e, record);
    }
}

impl<'a> Worker<'a> {
    /// The type arguments of `te` where it is a call node, not an inline expansion, whose own
    /// record holds them.
    #[cold]
    #[inline(never)]
    pub(crate) fn capture_call_targs(&mut self, te: TExprId, targs: &[TypeId]) {
        let call = matches!(self.prog.expr(te), TExpr::CallStatic(..) | TExpr::CallMethod(..) | TExpr::Field(..) | TExpr::Static(_) | TExpr::New(..) | TExpr::NewVia(..));
        let expansion = self.prog.capture.as_deref().map_or(false, |c| c.inline_calls.contains_key(&te));
        if call && !expansion {
            self.capture_targs(te, targs);
        }
    }
}

/// Where the capture of the file being typed stands, to drop what an attempt adds when the
/// attempt is given up (`Worker::capture_mark`).
#[derive(Clone, Copy)]
pub(super) struct CaptureMark {
    unit: FileId,
    at: (usize, usize),
}

impl<'a> Worker<'a> {
    #[inline]
    pub(super) fn capture_mark(&self) -> Option<CaptureMark> {
        let c = self.prog.capture.as_deref()?;
        let unit = self.typing_unit();
        Some(CaptureMark { unit, at: c.mark(unit) })
    }

    /// Puts the capture back where `mark` found it: an alternative tried and given up, whose
    /// nodes no body holds and whose changes to the records of others no body keeps.
    #[cold]
    #[inline(never)]
    pub(super) fn capture_drop_since(&mut self, mark: Option<CaptureMark>) {
        let (Some(mark), Some(c)) = (mark, self.prog.capture.as_deref_mut()) else { return };
        c.drop_since(mark.unit, mark.at);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arena::LOCAL_BASE;
    use crate::tir::capture::{BuiltinCall, InlineCall, Key};

    #[test]
    fn a_capture_record_naming_a_worker_id_fails_the_merge_check() {
        let types = std::sync::Arc::new(TypeStore::new());
        let mut c = Capture::new(vec![true], vec![true]);
        let unit = FileId(0);
        c.targs(unit, TExprId(3), types.list(&[ANY]));
        c.form(unit, TExprId(4), Form::Return(SymId(5)));
        let check = super::super::merge::Remap::checking(types.clone(), 0);
        check.capture(&types, &mut c);
        assert_eq!(check.left(), (0, 0, 0));
        let worker_class = types.class(ClassId(LOCAL_BASE + 5), &[]);
        c.form(unit, TExprId(6), Form::Test(worker_class));
        c.targs(unit, TExprId(LOCAL_BASE + 1), types.list(&[ANY]));
        c.builtin_call(unit, TExprId(7), BuiltinCall { name: crate::names::APPLY, recv: TExprId(LOCAL_BASE + 2), args: Vec::new() });
        let check = super::super::merge::Remap::checking(types.clone(), 0);
        check.capture(&types, &mut c);
        assert_eq!(check.left(), (0, 1, 3), "a type naming a worker's class; a worker's expression as a key, in the unit's list of what it made and as a receiver");
    }

    #[test]
    fn dropping_an_attempt_leaves_what_a_node_had_before_it() {
        let types = TypeStore::new();
        let mut c = Capture::new(vec![true], vec![true]);
        let unit = FileId(0);
        c.targs(unit, TExprId(1), types.list(&[ANY]));
        let mark = c.mark(unit);
        c.wrap(unit, TExprId(1), Wrap::Named(crate::names::APPLY));
        c.form(unit, TExprId(2), Form::ByName);
        assert_eq!(c.drop_since(unit, mark), 2);
        assert!(c.targs.contains_key(&TExprId(1)));
        assert!(!c.wraps.contains_key(&TExprId(1)) && !c.forms.contains_key(&TExprId(2)));
        assert_eq!(c.made[&unit], vec![Key::Targs(TExprId(1))]);
    }

    #[test]
    fn dropping_an_attempt_puts_back_what_it_changed_of_earlier_records() {
        let types = TypeStore::new();
        let mut c = Capture::new(vec![true], vec![true]);
        let unit = FileId(0);
        let e = TExprId(1);
        c.form(unit, e, Form::Widening);
        c.wrap(unit, e, Wrap::Ascribed(ANY));
        c.inline_call(unit, e, InlineCall { callee: SymId(7), recv: None, args: Vec::new(), targs: types.list(&[]), transparent: true, binds: false, site: None });
        c.local(unit, SymId(3)).ty = Some(ANY);
        c.targs(unit, e, types.list(&[ANY]));
        let mark = c.mark(unit);
        c.form(unit, e, Form::Promotion);
        c.wrap(unit, e, Wrap::Cast(NOTHING));
        c.inline_call(unit, e, InlineCall { callee: SymId(8), recv: None, args: Vec::new(), targs: types.list(&[]), transparent: false, binds: false, site: None });
        c.local(unit, SymId(3)).ty = Some(NOTHING);
        c.targs(unit, e, types.list(&[NOTHING]));
        c.form(unit, TExprId(2), Form::ByName);
        assert_eq!(c.drop_since(unit, mark), 1);
        assert_eq!(c.forms.get(&e), Some(&Form::Widening));
        assert_eq!(c.wraps[&e], vec![Wrap::Ascribed(ANY)]);
        assert_eq!(c.inline_calls[&e].iter().map(|call| call.callee).collect::<Vec<_>>(), vec![SymId(7)]);
        assert_eq!(c.locals[&SymId(3)].ty, Some(ANY));
        assert_eq!(c.targs[&e], types.list(&[ANY]));
        assert!(!c.forms.contains_key(&TExprId(2)));
        assert_eq!(c.mark(unit), mark);
    }

    #[test]
    fn dropping_a_move_gives_the_source_its_records_back() {
        let mut c = Capture::new(vec![true], vec![true]);
        let unit = FileId(0);
        let (from, to) = (TExprId(1), TExprId(2));
        c.form(unit, from, Form::Test(ANY));
        c.wrap(unit, from, Wrap::Ascribed(ANY));
        let mark = c.mark(unit);
        c.moved(unit, from, to);
        assert_eq!(c.forms.get(&to), Some(&Form::Test(ANY)));
        assert!(!c.forms.contains_key(&from));
        c.drop_since(unit, mark);
        assert_eq!(c.forms.get(&from), Some(&Form::Test(ANY)));
        assert_eq!(c.wraps.get(&from), Some(&vec![Wrap::Ascribed(ANY)]));
        assert!(!c.forms.contains_key(&to) && !c.wraps.contains_key(&to));
    }

    #[test]
    fn a_record_deleted_then_made_again_takes_back_its_first_value() {
        let mut c = Capture::new(vec![true], vec![true]);
        let unit = FileId(0);
        let (from, to) = (TExprId(1), TExprId(2));
        c.form(unit, to, Form::Widening);
        c.wrap(unit, to, Wrap::Unchecked);
        let mark = c.mark(unit);
        c.moved(unit, from, to);
        assert!(!c.forms.contains_key(&to));
        c.form(unit, to, Form::Promotion);
        c.drop_since(unit, mark);
        assert_eq!(c.forms.get(&to), Some(&Form::Widening));
        assert_eq!(c.wraps.get(&to), Some(&vec![Wrap::Unchecked]));
        assert_eq!(c.mark(unit), mark);
    }

    #[test]
    fn nested_attempts_put_back_each_their_own_changes() {
        let mut c = Capture::new(vec![true], vec![true]);
        let unit = FileId(0);
        let e = TExprId(1);
        c.form(unit, e, Form::Widening);
        let outer = c.mark(unit);
        c.form(unit, e, Form::Promotion);
        let inner = c.mark(unit);
        c.form(unit, e, Form::Default);
        c.drop_since(unit, inner);
        assert_eq!(c.forms.get(&e), Some(&Form::Promotion));
        c.drop_since(unit, outer);
        assert_eq!(c.forms.get(&e), Some(&Form::Widening));
    }

    #[test]
    fn a_worker_reads_what_another_published_and_a_retype_takes_it_back() {
        let mut main = Capture::new(vec![true], vec![true]);
        let worker = main.attached();
        let unit = FileId(0);
        let mark = main.mark(unit);
        main.form(unit, TExprId(1), Form::Test(ANY));
        main.pat(unit, TPatId(2), PatForm::Elements);
        main.local(unit, SymId(3)).ty = Some(ANY);
        assert!(worker.records_of(TExprId(1)).is_empty());
        main.publish(unit, mark);
        assert_eq!(worker.records_of(TExprId(1)).form, Some(Form::Test(ANY)));
        assert_eq!(worker.pat_of(TPatId(2)), Some(PatForm::Elements));
        assert_eq!(worker.local_of(SymId(3)).and_then(|l| l.ty), Some(ANY));
        main.forget(unit);
        assert!(worker.records_of(TExprId(1)).is_empty() && worker.pat_of(TPatId(2)).is_none() && worker.local_of(SymId(3)).is_none());
    }

    #[test]
    fn a_publication_dropped_with_its_attempt_leaves_nothing_published() {
        let mut main = Capture::new(vec![true], vec![true]);
        let worker = main.attached();
        let unit = FileId(0);
        let e = TExprId(1);
        let mark = main.mark(unit);
        main.form(unit, e, Form::Test(ANY));
        main.publish(unit, mark);
        assert_eq!(worker.records_of(e).form, Some(Form::Test(ANY)));
        main.drop_since(unit, mark);
        main.forget(unit);
        assert!(worker.records_of(e).is_empty());
        assert_eq!(main.published_orphans(), 0);
    }

    #[test]
    fn a_published_change_put_back_is_published_as_put_back() {
        let mut main = Capture::new(vec![true], vec![true]);
        let worker = main.attached();
        let unit = FileId(0);
        let e = TExprId(1);
        let quote = main.mark(unit);
        main.form(unit, e, Form::Widening);
        main.publish(unit, quote);
        let attempt = main.mark(unit);
        main.form(unit, e, Form::Promotion);
        main.sync_published(&[Key::Form(e)]);
        assert_eq!(worker.records_of(e).form, Some(Form::Promotion));
        main.drop_since(unit, attempt);
        assert_eq!(worker.records_of(e).form, Some(Form::Widening));
    }

    #[test]
    fn a_published_record_with_no_owner_is_an_orphan() {
        let mut main = Capture::new(vec![true], vec![true]);
        let unit = FileId(0);
        let mark = main.mark(unit);
        main.form(unit, TExprId(1), Form::Test(ANY));
        main.publish(unit, mark);
        assert_eq!(main.published_orphans(), 0);
        main.forms.remove(&TExprId(1));
        assert_eq!(main.published_orphans(), 1);
    }

    #[test]
    fn a_part_is_kept_as_written_but_for_dollars_and_unicode_escapes() {
        assert_eq!(written_part(r"\n"), r"\n");
        assert_eq!(written_part(r"\\n"), r"\\n");
        assert_eq!(written_part("a$$b"), "a$b");
        assert_eq!(written_part(r"\u0041\t"), "A\\t");
        assert_eq!(written_part(r"\\u0041"), r"\\u0041");
        assert_eq!(written_part(r"\uu0042"), "B");
        assert_eq!(written_part(r"\uD83D\uDE00"), "\u{1F600}");
        assert_eq!(written_part(r#"a\n"q""#), r#"a\n"q""#);
    }

    #[test]
    fn a_retype_drops_the_records_its_file_made_and_no_other() {
        let types = TypeStore::new();
        let mut c = Capture::new(vec![true, true], vec![true, true]);
        let (edited, other) = (FileId(0), FileId(1));
        c.targs(edited, TExprId(1), types.list(&[ANY]));
        c.form(edited, TExprId(1), Form::Default);
        c.wrap(edited, TExprId(2), Wrap::Named(crate::names::APPLY));
        c.local(edited, SymId(3)).ty = Some(ANY);
        c.form(other, TExprId(4), Form::ByName);
        c.forget(edited);
        assert_eq!(c.records(), 1);
        assert_eq!(c.forms.get(&TExprId(4)), Some(&Form::ByName));
        assert!(c.made.get(&edited).is_none());
        assert_eq!(c.made.get(&other).map(|k| k.as_slice()), Some(&[Key::Form(TExprId(4))][..]));
    }

    #[test]
    fn a_name_and_a_sequence_wrap_an_argument_once_and_a_copy_takes_them() {
        let mut c = Capture::new(vec![true], vec![true]);
        let unit = FileId(0);
        let (a, b) = (crate::names::APPLY, crate::names::UNARY_PLUS);
        c.wrap(unit, TExprId(1), Wrap::Named(a));
        c.wrap(unit, TExprId(1), Wrap::Ascribed(ANY));
        c.wrap(unit, TExprId(1), Wrap::Named(b));
        assert_eq!(c.wraps[&TExprId(1)], vec![Wrap::Named(b), Wrap::Ascribed(ANY)]);
        assert_eq!(c.made[&unit].len(), 1);
        c.form(unit, TExprId(1), Form::ByName);
        c.copy(unit, TExprId(1), TExprId(2));
        assert_eq!(c.wraps[&TExprId(2)], c.wraps[&TExprId(1)]);
        assert_eq!(c.forms.get(&TExprId(2)), Some(&Form::ByName));
    }
}
