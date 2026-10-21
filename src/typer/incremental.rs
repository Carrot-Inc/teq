//! Re-typing the bodies of edited files against the existing symbol table, for `teq compiler watch`. The
//! edit changed nothing outside bodies (`shape::compare` paired the old and new definitions), so
//! every symbol of the file stays what it was and only where it stands in the new AST changes;
//! what the file's old typing put into the program is dropped and its bodies are typed again.
//! Other files can observe nothing of that unless an inferred type came out different, which
//! ends in a full build. The program's arrays keep what the old typing put there, dead: a
//! session grows by that until its memory sends a build down the full path (`watch.rs`).

use super::Worker;
use crate::ast::DefId;
use crate::shape::Remap;
use crate::source::{FileId, Span};
use crate::symbols::*;
use crate::intern::FxMap;
use crate::types::*;
use std::sync::Arc;

/// An AST that was replaced: a file, or one `package p:` block of it.
pub struct Change<'r> {
    pub file: FileId,
    pub remap: &'r Remap,
}

impl<'a> super::Typer<'a> {
    /// Types the bodies of the changed files anew. `Err` names why a full build is needed
    /// instead; `Ok` leaves the diagnostics of the files in `diags`. An inferred signature that
    /// came out different is decided by the definition it is of: where inferring it reported an
    /// error, the new type may be nonsense and the old signature is put back, so that what
    /// other files were typed against is kept (`Ok` lists the files of those, which a later
    /// build has to type again to settle them); where it reported none, the rest of the program
    /// was typed against a type that no longer holds, whatever errors stand elsewhere, and the
    /// build takes the full path. The diagnostics kept for a file (`Diagnostic::of_body`) stand
    /// while its builds are incremental and keep none of them so: a full build has its own.
    pub fn retype(&mut self, changes: &[Change]) -> Result<Vec<FileId>, String> {
        self.on_thread(|w| {
            // The files' classes are entered again: what their bases were searched for by name
            // is found again (`members::InheritedNamed`).
            crate::symbols::type_members_changed();
            // A build starts without the files the last one's macros read (`interp::files`).
            crate::interp::forget_files();
            // The macros' info messages are the retype's to print once it stands; one that
            // gives way to a full build leaves them to the build.
            w.infos = Some(Vec::new());
            let r = w.retype_files(changes);
            match r {
                Ok(_) => w.flush_infos(),
                Err(_) => w.infos = None,
            }
            if r.is_ok() {
                let files: Vec<FileId> = changes.iter().map(|c| c.file).collect();
                w.report_unused(Some(&files));
            }
            w.register_suppressions();
            w.tell_cacheable_warnings();
            w.index_settle();
            // The buffers the stores outgrew during the step, freed once no reference into them
            // is alive. A build that ends with the process keeps them: freeing them changes how
            // the system allocator places what follows (the mmap threshold), which moved the
            // classpath programs' instructions by 1.3% either way.
            w.types.release_retired();
            w.interner.release_retired();
            r
        })
    }
}

impl<'a> Worker<'a> {
    fn retype_files(&mut self, changes: &[Change]) -> Result<Vec<FileId>, String> {
        if self.scan_wildcards() != self.wildcards_used.load(std::sync::atomic::Ordering::Relaxed) {
            return Err("wildcard arguments are newly used or no longer used".to_string());
        }
        for change in changes {
            let f = change.file;
            if self.inferred_parent_args.local.keys().any(|&c| self.syms.class(c).file == f) {
                return Err(format!("{}: an enum case infers the type arguments of its enum", self.source(f).path));
            }
            if self.inferred_trait_args.local.keys().any(|&c| self.syms.class(c).file == f) {
                return Err(format!("{}: a class infers the type arguments of a trait it extends", self.source(f).path));
            }
            // What was reported once for the file stays with it, where the edit moved it to.
            if self.diags.items.iter().any(|d| d.file == f && !d.of_body && moved(change.remap, d.span).is_none()) {
                return Err(format!("{}: a diagnostic of its signatures stands where the edit leaves no place for it", self.source(f).path));
            }
        }
        self.given_fast.clear();
        self.head_sigs.clear();
        self.base_type_cache.clear();
        self.class_base_memo.clear();
        self.target_name_files.clear();
        self.package_levels.clear();
        // A member's body may come or go with the edit.
        self.lone_abstract_defs.clear();
        self.path_sigs.clear();
        self.unbox_candidates.clear();
        self.erased_results.clear();
        self.inner_chain_members.clear();
        self.plain_name_members.clear();
        self.java_members_exhausted.clear();
        // The infix tuples noted by node (`Worker::infix_tuples`) are of the files' earlier parses.
        self.infix_tuples.retain(|&(f, _), _| !changes.iter().any(|c| c.file == f));
        // So are the binders' `@nowarn`s, which their retype registers again.
        self.binder_nowarns.retain(|n| !changes.iter().any(|c| c.file == n.file));
        self.inference_failed.clear();
        // A warning of the session's options, told once (an object with cacheable state made anew):
        // the answer that carried it is over.
        self.diags.items.retain(|d| !(d.file == crate::source::NO_FILE && d.is_warning));
        let mut inferred = Vec::new();
        // A constant val without a type stands for its literal type wherever its singleton is
        // read, which its signature (`Int` for `final val one = 1`) does not carry.
        let mut constants: Vec<(SymId, TypeId)> = Vec::new();
        for change in changes {
            self.index_move(change.file, change.remap);
            let moved = self.move_symbols(change.file, change.remap);
            for &s in &moved {
                if let Some(ret) = self.syms.sym(s).sig.as_ref().map(|sig| sig.ret) {
                    if let Some(literal) = self.constant_type(s, ret) {
                        constants.push((s, literal));
                    }
                }
            }
            self.move_forwarders(change.file, change.remap);
            self.forget_typing(std::slice::from_ref(&change.file), Some(change.remap));
            inferred.extend(self.open_inferred(change.file, &moved).into_iter().map(|(sym, sig)| (change.file, sym, sig)));
            self.reopen_getters(change.file);
        }
        // The build begins here: the interpreter's next run makes the program's objects anew.
        self.prog.build_start = self.prog.shifts;
        self.prog.main = None;
        let prefix = super::merge::Prefix::of(self);
        let exprs_mark = self.prog.exprs.len();
        self.begin_demands();
        self.checked_imports.clear();
        self.import_heads.clear();
        for change in changes {
            self.check_file(change.file);
        }
        self.complete_pending_sealed_files();
        self.fill_getters();
        // The retyped files' kept calls, and those an earlier build with an error left, expanded.
        self.inlining_phase();
        // The files' top-level value forwarders, forgotten with their typing: made again as the
        // full path makes them once the bodies are typed.
        self.define_top_value_forwarders();
        self.run_deferred_matches();
        let files: Vec<FileId> = changes.iter().map(|c| c.file).collect();
        self.check_outer_accessors_implemented(Some(&files));
        if self.emits_bodies {
            self.number_contextual_params(exprs_mark);
        }
        if self.profile.on {
            self.merge(&prefix);
        }
        self.bind_mixin_supers();
        // The joined entries the re-typed classes made are settled as the final passes settle
        // them; then the names of the alternatives stand, and the re-typed classes need their
        // bridges. Those alone: the other classes keep the bridges their check gave them,
        // whatever names were settled since (a library method named apart while the last
        // build reached it), as a fresh build's do.
        self.name_late_alternatives(prefix.overloads as usize);
        let files: Vec<FileId> = changes.iter().map(|c| c.file).collect();
        self.add_bridges_of(&files);
        self.finish_capture_of(Some(&files));
        self.choose_entry_point();
        self.check_deferred_bounds();
        for (s, old) in constants {
            let ret = self.sig_of(s).ret;
            if self.constant_type(s, ret) != Some(old) {
                return Err(format!("the constant value of {} changed", self.name_str(self.syms.sym(s).name)));
            }
        }
        let mut restored = Vec::new();
        for (file, sym, old) in inferred {
            if self.same_result(sym, &old) {
                continue;
            }
            if !self.inference_failed.contains_key(&sym) {
                return Err(format!("the inferred type of {} changed", self.name_str(self.syms.sym(sym).name)));
            }
            let mut s = self.syms.sym_mut(sym);
            s.sig = Some(old);
            s.state().set(Completion::Done);
            if !restored.contains(&file) {
                restored.push(file);
            }
        }
        Ok(restored)
    }


    /// The files a round of the inlining phase gave up typed again from their trees, as dotty's
    /// later run compiles a suspended unit from its untyped tree (`Run.compileSuspendedUnits`,
    /// whose `TyperPhase` types it again): what their typing and their rounds put into the
    /// program dropped, their inferred signatures opened, their bodies typed again with the
    /// expansions typing makes (a transparent macro's run among them), their plain calls kept.
    pub(super) fn retype_suspended(&mut self, files: &[FileId]) {
        // What reads signatures, whose inferred ones open, is read again.
        self.given_fast.clear();
        self.head_sigs.clear();
        self.path_sigs.clear();
        self.erased_results.clear();
        self.inner_chain_members.clear();
        self.inference_failed.clear();
        self.forget_typing(files, None);
        // The files' symbols with a definition, collected in one walk of the table.
        let mut defined: FxMap<u32, Vec<SymId>> = FxMap::default();
        for &f in files {
            defined.insert(f.0, Vec::new());
        }
        for (i, s) in self.syms.syms.iter().enumerate() {
            if s.def.is_some() {
                if let Some(of_file) = defined.get_mut(&s.file.0) {
                    of_file.push(SymId(i as u32));
                }
            }
        }
        for &file in files {
            let of_file = defined.remove(&file.0).unwrap_or_default();
            self.open_inferred(file, &of_file);
            self.reopen_getters(file);
            self.forget_body_definitions(file);
        }
        self.reopen_suspended();
        self.checked_imports.retain(|&(f, _), _| !files.iter().any(|x| x.0 == f));
        self.import_heads.retain(|&(f, _), _| !files.iter().any(|x| x.0 == f));
        for &file in files {
            self.check_file(file);
        }
        self.complete_pending_sealed_files();
        self.fill_getters();
    }

    /// The completions a macro of the run suspended, of any file, completed again by their next
    /// reader, as a later run's completers are new (`Run.compileSuspendedUnits`): a declared
    /// signature's, an alias's (`suspend_alias`), a default getter's. An inferred signature of a
    /// retyped file is opened with the rest (`open_inferred`).
    fn reopen_suspended(&mut self) {
        for i in 0..self.syms.syms.len() {
            if self.syms.sym_cells.get(i as u32) == Completion::Suspended {
                self.syms.syms[i].sig = None;
                self.syms.sym_cells.set(i as u32, Completion::NotStarted);
            }
        }
        for i in 0..self.syms.aliases.len() {
            let a = crate::types::AliasId(i as u32);
            if self.syms.alias(a).state() == Completion::Suspended {
                self.syms.alias(a).state().set(Completion::NotStarted);
            }
        }
        for i in 0..self.syms.classes.len() {
            let c = ClassId(i as u32);
            if self.syms.class(c).state() == Completion::Suspended {
                self.syms.class(c).state().set(Completion::NotStarted);
            }
        }
    }

    /// The file's definitions inside its bodies forgotten by their definitions (`def_syms`,
    /// `def_classes`, `def_aliases`), made anew with the bodies, as a session's retype keeps
    /// those outside alone (`Remap::defs`): a block's class found by its definition would be
    /// the old typing's, which the program no longer holds.
    fn forget_body_definitions(&mut self, file: FileId) {
        let f = file.0 as usize;
        for (d, s) in self.def_syms.entries_in(f) {
            if self.in_body(self.syms.sym(s).owner) {
                self.def_syms.remove(f, &d);
            }
        }
        for (d, c) in self.def_classes.entries_in(f) {
            if self.in_body(self.syms.class(c).owner) {
                self.def_classes.remove(f, &d);
            }
        }
        for (d, a) in self.def_aliases.entries_in(f) {
            if self.in_body(self.syms.alias(a).owner) {
                self.def_aliases.remove(f, &d);
            }
        }
    }

    /// Whether a definition of `owner` stands in a body: a block's, or a member of a class that does.
    fn in_body(&self, owner: Owner) -> bool {
        match owner {
            Owner::Local => true,
            Owner::Class(c) => self.in_body(self.syms.class(c).owner),
            _ => false,
        }
    }

    /// Gives the value forwarders of the file's export clauses the positions a build of the new
    /// text gives them: the selector found again in its clause, the alternative's offset kept
    /// (`value_exports::selector_span`).
    fn move_forwarders(&mut self, file: FileId, remap: &Remap) {
        if remap.exports.is_empty() {
            return;
        }
        let ast = self.ast(file);
        let moved = |t: &Self, fwd: &Forward| -> Option<(Span, Span)> {
            let &(_, clause) = remap.exports.iter().find(|(old, _)| *old == fwd.clause)?;
            let imp = ast.exports.iter().chain(&ast.top_exports).find(|i| i.span == clause)?;
            let at = t.selector_span(imp);
            Some((Span { start: at.start + fwd.alt.min(at.end.saturating_sub(at.start)), end: at.end }, clause))
        };
        let classes: Vec<ClassId> = (0..self.syms.classes.len() as u32).map(ClassId).filter(|&c| self.syms.class(c).file == file && !self.syms.class(c).forwarders.is_empty()).collect();
        for c in classes {
            let mut fwds = self.syms.class(c).forwarders.clone();
            for (s, fwd) in fwds.iter_mut() {
                if let Some((span, clause)) = moved(self, fwd) {
                    self.syms.sym_mut(*s).span = span;
                    *fwd = std::sync::Arc::new(Forward { clause, ..(**fwd).clone() });
                }
            }
            self.syms.class_mut(c).forwarders = fwds;
        }
        for i in 0..self.syms.pkgs.len() as u32 {
            let p = PkgId(i);
            if !self.syms.pkg(p).forwarders.iter().any(|&(s, _)| self.syms.sym(s).file == file) {
                continue;
            }
            let mut fwds = self.syms.pkg(p).forwarders.clone();
            for (s, fwd) in fwds.iter_mut() {
                if self.syms.sym(*s).file != file {
                    continue;
                }
                if let Some((span, clause)) = moved(self, fwd) {
                    self.syms.sym_mut(*s).span = span;
                    *fwd = std::sync::Arc::new(Forward { clause, ..(**fwd).clone() });
                }
            }
            self.syms.pkgs[p.idx()].forwarders = fwds;
        }
    }

    /// Points the symbols of the file at their definitions in the new AST and gives them the
    /// positions they have there, and returns the symbols with a definition among them. A
    /// position moves with the definition or parameter it came from, which is not always the
    /// symbol's own definition: an explicit companion merged into an enum's keeps the enum's.
    /// Locals of the old bodies and the members of old anonymous classes stay behind unchanged;
    /// nothing reaches them any more.
    fn move_symbols(&mut self, file: FileId, remap: &Remap) -> Vec<SymId> {
        let ast = self.ast(file);
        let mut moved = Vec::new();
        for (i, s) in self.syms.syms.iter_mut().enumerate().filter(|(_, s)| s.file == file) {
            let new_def = s.def.and_then(|d| remap.defs.get(&d).copied());
            if s.def.is_some() && new_def.is_none() {
                continue;
            }
            if s.def.is_none() && s.owner == Owner::Local {
                continue;
            }
            if let Some(&span) = remap.spans.get(&s.span.start) {
                s.span = span;
            } else if let Some(nd) = new_def {
                s.span = ast.def(nd).span;
            }
            if let Some(nd) = new_def {
                s.def = Some(nd);
                moved.push(SymId(i as u32));
            }
        }
        for c in self.syms.classes.iter_mut().filter(|c| c.file == file) {
            let new_def = c.def.and_then(|d| remap.defs.get(&d).copied());
            if c.def.is_some() && new_def.is_none() {
                continue;
            }
            if c.def.is_none() && c.owner == Owner::Local {
                continue;
            }
            if let Some(&span) = remap.spans.get(&c.span.start) {
                c.span = span;
            } else if let Some(nd) = new_def {
                c.span = ast.def(nd).span;
            }
            if let Some(nd) = new_def {
                c.def = Some(nd);
            }
        }
        for a in self.syms.aliases.iter_mut().filter(|a| a.file == file) {
            if let Some(nd) = a.def.and_then(|d| remap.defs.get(&d)) {
                a.def = Some(*nd);
            }
        }
        // What a `T @uncheckedVariance` of a signature resolved to is kept by its type
        // expression; those of the old bodies go, the new bodies' are resolved with them.
        let annotated: Vec<(crate::ast::TyExprId, TypeId)> =
            self.unchecked_variance.iter().filter(|((of, _), _)| *of == file).map(|(&(_, t), &resolved)| (t, resolved)).collect();
        self.unchecked_variance.retain(|(of, _), _| *of != file);
        for (t, resolved) in annotated {
            if let Some(&now) = remap.unchecked.get(&t) {
                self.unchecked_variance.insert((file, now), resolved);
            }
        }
        let f = file.0 as usize;
        self.def_syms.own_mut()[f] = remapped(&self.def_syms.own()[f], remap);
        self.def_classes.own_mut()[f] = remapped(&self.def_classes.own()[f], remap);
        self.def_aliases.own_mut()[f] = remapped(&self.def_aliases.own()[f], remap);
        moved
    }

    /// Drops what typing the files' bodies produced and cached, the anonymous classes their
    /// expansions made included. The diagnostics of the bodies go, to be reported again; those
    /// of what is resolved once (a signature, a parent, an import) move with the edit, or stay
    /// where they are without one (`remap` none: the same trees typed again, the phase's retype of
    /// its suspended files; a session's edit is one file's). Each table of the program is walked
    /// once for the files.
    fn forget_typing(&mut self, files: &[FileId], remap: Option<&Remap>) {
        debug_assert!(remap.is_none() || files.len() == 1, "an edit's remap is one file's");
        for &file in files {
            self.index_forget_bodies(file);
            self.forget_unused(file, remap);
            for sites in self.inline_deps.values_mut() {
                sites.remove(&file);
            }
            self.inlining.forget_file(file, &self.prog);
        }
        let mut set = vec![false; self.files.len()];
        for &f in files {
            if let Some(b) = set.get_mut(f.0 as usize) {
                *b = true;
            }
        }
        let of = |f: FileId| set.get(f.0 as usize).copied().unwrap_or(false);
        let syms = &self.syms;
        let sym_in = |s: &SymId| of(syms.sym(*s).file);
        // The same tree typed again (the phase's retype) takes back what its own typing made: a
        // class an expansion at a site in another file made of this file's template is that file's.
        let in_place = remap.is_none();
        // The file a class is taken back with: the typing's that made it in place, the edited one's.
        let typed_in = move |c: ClassId| -> Option<FileId> {
            let info = syms.class(c);
            if in_place {
                Some(info.made_at.unwrap_or(info.file)).filter(|&f| of(f))
            } else {
                (of(info.file) || info.made_at.is_some_and(|f| of(f))).then(|| files[0])
            }
        };
        let made_here = |c: ClassId| typed_in(c).is_some();
        if let Some(d) = self.deps.as_deref_mut() {
            d.forget(|c| match c {
                super::deps::Comp::Body(s) => of(syms.sym(s).file),
                super::deps::Comp::ClassBody(k) => made_here(k),
                _ => false,
            });
        }
        let class_in = |c: &ClassId| made_here(*c);
        let unchecked: Vec<ClassId> = self.class_done.local.keys().copied().filter(|c| class_in(c)).collect();
        self.class_done.retain(|c, _| !class_in(c));
        for c in unchecked {
            self.syms.check_cells.set(c.0, Completion::NotStarted);
        }
        self.class_imports.retain(|c, _| !class_in(c));
        self.fun_of_sym.retain(|s, _| !sym_in(s));
        self.val_init.retain(|s, _| !sym_in(s));
        self.inline_definitions.retain(|s, _| !sym_in(s));
        self.inline.stored_indexes.retain(|s, _| !sym_in(s));
        self.entry_points.retain(|(s, object)| match object {
            Some(c) => !class_in(c),
            None => !sym_in(s),
        });
        let bound = self.mixins_bound.0.min(self.prog.classes.len());
        let unbound = self.prog.classes.iter().take(bound).filter(|tc| class_in(&tc.id)).count();
        self.mixins_bound.0 = bound - unbound;
        self.prog.classes.retain(|tc| !class_in(&tc.id));
        self.prog.shifts += 1;
        let funs = &self.prog.funs;
        self.prog.top_funs.own_mut().retain(|f| !sym_in(&funs[f.idx()].sym));
        self.prog.top_vals.own_mut().retain(|(s, _)| !sym_in(s));
        self.prog.template_calls.retain(|&(_, unit)| !of(unit));
        self.prog.get_class_units.retain(|&unit| !of(unit));
        self.prog_index = super::check::ProgIndex::default();
        self.roots_memo.clear();
        self.subclasses = super::overload::SubclassIndex::default();
        self.anon_sites.retain(|&(f, _, site), _| if in_place { !of(site.map_or(f, |(sf, _)| sf)) } else { !of(f) && site.map_or(true, |(sf, _)| !of(sf)) });
        self.mixin_supers.retain(|c, _| !class_in(c));
        self.super_problems_told.retain(|(c, _), _| !class_in(c));
        // The outer-this field of an inner class is named by the class's position, which the
        // edit may have moved.
        let renamed: Vec<(SymId, String)> = self
            .outer_this
            .local
            .iter()
            .filter_map(|(&c, &s)| typed_in(c).map(|file| (s, format!("$this{}", self.prog.position(file, syms.class(c).span.start)))))
            .collect();
        // The anonymous and local classes made anew no longer extend anything.
        let unlinked: Vec<(ClassId, ClassId)> = (0..syms.classes.len())
            .filter_map(|i| {
                let c = ClassId(i as u32);
                let info = &syms.classes[i];
                if info.owner != Owner::Local || !made_here(c) {
                    return None;
                }
                info.superclass.map(|parent| (parent, c))
            })
            .collect();
        for (s, name) in renamed {
            let name = self.interner.intern(&name);
            self.syms.sym_mut(s).name = name;
        }
        for (parent, c) in unlinked {
            self.syms.class_mut(parent).subclasses.retain(|&d| d != c);
        }
        for &file in files {
            self.quote.forget_file(file);
            self.forget_capture(file);
        }
        self.diags.items.retain_mut(|d| {
            if !of(d.file) {
                return true;
            }
            let Some(remap) = remap else { return !d.of_body };
            match moved(remap, d.span).filter(|_| !d.of_body) {
                Some(span) => {
                    d.span = span;
                    // Its owner, a completion of the same file, moves with it: a definition's
                    // place, a signature's keeping its mark (`DiagOwner::sig`). One the edit
                    // gives no place stands for no completion.
                    if d.owner.file == d.file.0 {
                        let mark = d.owner.place & crate::source::DiagOwner::SIG;
                        d.owner = match remap.spans.get(&d.owner.at()) {
                            Some(at) => crate::source::DiagOwner::new(d.file, at.start | mark),
                            None => crate::source::DiagOwner::NONE,
                        };
                    }
                    true
                }
                None => false,
            }
        });
    }

    /// Forgets the signatures of the vals and defs among `moved` whose type is inferred, so that
    /// typing the new bodies computes them again, and returns the old ones.
    fn open_inferred(&mut self, file: FileId, moved: &[SymId]) -> Vec<(SymId, Arc<MethodSig>)> {
        let mut out = Vec::new();
        for &sym in moved {
            let i = sym.idx();
            let s = &self.syms.syms[i];
            if s.owner == Owner::Local || !matches!(s.kind, SymKind::Val | SymKind::Var | SymKind::Def) {
                continue;
            }
            let inferred = self.result_inferred(file, s.def, s.owner, s.kind);
            let Some(sig) = s.sig.clone().filter(|_| inferred) else { continue };
            let s = &mut self.syms.syms[i];
            s.sig = None;
            self.syms.sym_cells.set(sym.0, Completion::NotStarted);
            if let Some(d) = self.deps.as_deref_mut() {
                d.forget(|c| c == super::deps::Comp::Sig(sym));
            }
            out.push((sym, sig));
        }
        out
    }

    /// Whether the new signature of `sym` gives the same result type as `old`, read with the
    /// old type parameters, which is all other files see of it.
    fn same_result(&mut self, sym: SymId, old: &MethodSig) -> bool {
        let Some(new) = self.syms.sym(sym).sig.clone() else { return false };
        if new.tparams.len() != old.tparams.len() {
            return false;
        }
        let subst: Subst = new.tparams.iter().zip(&old.tparams).map(|(&n, &o)| (n, self.types.param(o))).collect();
        let ret = self.types.subst(new.ret, &subst);
        // A result that names the method's own parameters (`x.M` of a `depmeth(x: C) = x.m`)
        // names the ones the retype made: they are the old ones by position.
        let renamed: Vec<(SymId, TypeId)> = new
            .clauses
            .iter()
            .zip(&old.clauses)
            .flat_map(|(a, b)| a.params.iter().zip(&b.params).map(|(x, y)| (x.sym, y.sym)).collect::<Vec<_>>())
            .filter(|&(x, y)| x != y)
            .map(|(x, y)| (x, self.types.mk(Type::Term(y))))
            .collect();
        let ret = if renamed.is_empty() { ret } else { self.subst_paths(ret, &renamed) };
        self.same_up_to_binders(ret, old.ret)
    }

    /// Whether `a` and `b` are one type as dotty's structural equality has it (`Type.equals(that, bs)`, Types.scala
    /// 2147-2153, each type's `iso`), with the binders of the two paired: what a type's resolution makes anew each time a
    /// signature is typed. A polymorphic function type's type parameters (`LambdaType.iso`, 4058-4068) and a type
    /// lambda's (`HKTypeLambda.iso`, 4468-4484) pair by position (`ParamRef.iso`, 4896-4899), their names and bounds the
    /// same; the parameters of the methods a refinement declares likewise (a function type naming its parameters,
    /// `(x: C) => x.M` of `depmeth` eta-expanded), and the members a refinement stands for, as an anonymous class's are
    /// each time it is typed (`RefinedType.iso`, 3316-3320). A union and an intersection by their sides (`OrType.iso`
    /// 3727-3729, `AndType.iso` 3577-3579), an applied class, alias or parameter by its arguments (`AppliedType.iso`,
    /// 4853-4855), a recursive type and an annotated one by what they wrap (`RecType.iso`, 3423-3425); the rest by
    /// identity, teq's types being hash-consed.
    fn same_up_to_binders(&mut self, a: TypeId, b: TypeId) -> bool {
        if a == b {
            return true;
        }
        match (self.types.get(a), self.types.get(b)) {
            (Type::Class(c, xs), Type::Class(d, ys)) if c == d => self.same_lists(xs, ys),
            (Type::Alias(c, xs), Type::Alias(d, ys)) if c == d => self.same_lists(xs, ys),
            (Type::AppParam(p, xs), Type::AppParam(q, ys)) if p == q => self.same_lists(xs, ys),
            (Type::Union(x1, y1, k1), Type::Union(x2, y2, k2)) => k1 == k2 && self.same_up_to_binders(x1, x2) && self.same_up_to_binders(y1, y2),
            (Type::Inter(x1, y1), Type::Inter(x2, y2)) => self.same_up_to_binders(x1, x2) && self.same_up_to_binders(y1, y2),
            (Type::Annotated(x), Type::Annotated(y)) => self.same_up_to_binders(x, y),
            (Type::Poly(la, fa), Type::Poly(lb, fb)) => {
                let (pa, pb) = (self.types.poly_params(la).to_vec(), self.types.poly_params(lb).to_vec());
                let Some(renaming) = self.paired_binders(&pa, &pb) else { return false };
                let (ba, bb) = (self.types.poly_bounds(la).to_vec(), self.types.poly_bounds(lb).to_vec());
                ba.len() == bb.len()
                    && ba.iter().zip(&bb).all(|(&x, &y)| {
                        let x = self.types.subst(x, &renaming);
                        self.same_up_to_binders(x, y)
                    })
                    && {
                        let fa = self.types.subst(fa, &renaming);
                        self.same_up_to_binders(fa, fb)
                    }
            }
            (Type::Lambda(la, ra), Type::Lambda(lb, rb)) => {
                let (pa, pb) = (self.types.items(la).to_vec(), self.types.items(lb).to_vec());
                let Some(renaming) = self.paired_binders(&pa, &pb) else { return false };
                let bounds_same = pa.iter().zip(&pb).all(|(&x, &y)| {
                    let (Type::Param(p), Type::Param(q)) = (self.types.get(x), self.types.get(y)) else { return false };
                    let (lo, hi) = (self.syms.tparam(p).lower, self.syms.tparam(p).upper);
                    let (lo, hi) = (self.types.subst(lo, &renaming), self.types.subst(hi, &renaming));
                    let (wlo, whi) = (self.syms.tparam(q).lower, self.syms.tparam(q).upper);
                    self.same_up_to_binders(lo, wlo) && self.same_up_to_binders(hi, whi)
                });
                bounds_same && {
                    let ra = self.types.subst(ra, &renaming);
                    self.same_up_to_binders(ra, rb)
                }
            }
            // A self-referential refinement (an anonymous class's bound, closed over its `this`).
            (Type::Rec(x), Type::Rec(y)) => self.same_up_to_binders(x, y),
            (Type::Refined(pa, ra), Type::Refined(pb, rb)) => {
                // A value's refinement by its name and type: its symbol stands for the member, which
                // the retype entered again (an anonymous class's `val item`).
                if let (Refinement::Val(na, _, ta), Refinement::Val(nb, _, tb)) = (self.types.refinement(ra), self.types.refinement(rb)) {
                    return na == nb && self.same_up_to_binders(ta, tb) && self.same_up_to_binders(pa, pb);
                }
                let (Refinement::Term(na, sa, la), Refinement::Term(nb, sb, lb)) = (self.types.refinement(ra), self.types.refinement(rb)) else {
                    return ra == rb && self.same_up_to_binders(pa, pb);
                };
                let (siga, sigb) = (self.sig_arc(sa), self.sig_arc(sb));
                let shape = |s: &MethodSig| s.clauses.iter().map(|c| c.params.len()).collect::<Vec<_>>();
                if na != nb || !siga.tparams.is_empty() || !sigb.tparams.is_empty() || shape(&siga) != shape(&sigb) {
                    return false;
                }
                let renamed: Vec<(SymId, TypeId)> = siga
                    .clauses
                    .iter()
                    .zip(&sigb.clauses)
                    .flat_map(|(x, y)| x.params.iter().zip(&y.params).map(|(p, q)| (p.sym, q.sym)).collect::<Vec<_>>())
                    .map(|(p, q)| (p, self.types.mk(Type::Term(q))))
                    .collect();
                let (xs, ys) = (self.types.items(la).to_vec(), self.types.items(lb).to_vec());
                xs.len() == ys.len()
                    && xs.iter().zip(&ys).all(|(&x, &y)| {
                        let x = self.subst_paths(x, &renamed);
                        self.same_up_to_binders(x, y)
                    })
                    && self.same_up_to_binders(pa, pb)
            }
            _ => false,
        }
    }

    /// Two lists of types alike, element by element (`equalElements`, Types.scala 7308-7311).
    fn same_lists(&mut self, xs: crate::types::TList, ys: crate::types::TList) -> bool {
        let (xs, ys) = (self.types.items(xs).to_vec(), self.types.items(ys).to_vec());
        xs.len() == ys.len() && xs.iter().zip(&ys).all(|(&x, &y)| self.same_up_to_binders(x, y))
    }

    /// The binders `xs` of one type paired with `ys` of the other by position (`SomeBinderPairs`), each pair of one
    /// name (`paramNames.eqElements`): the renaming of the first's to the second's.
    fn paired_binders(&mut self, xs: &[TypeId], ys: &[TypeId]) -> Option<Subst> {
        if xs.len() != ys.len() {
            return None;
        }
        let mut renaming = Subst::new();
        for (&x, &y) in xs.iter().zip(ys) {
            let (Type::Param(p), Type::Param(q)) = (self.types.get(x), self.types.get(y)) else { return None };
            if self.syms.tparam(p).name != self.syms.tparam(q).name {
                return None;
            }
            renaming.push((p, y));
        }
        Some(renaming)
    }
}

/// Where the edit left what stood at `span` outside the bodies, under `shape::compare_moving`:
/// the span of what the new syntax tree pairs with it.
fn moved(remap: &Remap, span: crate::source::Span) -> Option<crate::source::Span> {
    remap.moved.get(&span).copied()
}

fn remapped<V: Copy>(map: &crate::intern::FxMap<DefId, V>, remap: &Remap) -> crate::intern::FxMap<DefId, V> {
    map.iter().filter_map(|(d, v)| remap.defs.get(d).map(|&nd| (nd, *v))).collect()
}
