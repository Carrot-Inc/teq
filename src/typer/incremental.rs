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
use crate::source::FileId;
use crate::symbols::*;
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
        self.package_levels.clear();
        // The infix tuples noted by node (`Worker::infix_tuples`) are of the files' earlier parses.
        self.infix_tuples.retain(|&(f, _), _| !changes.iter().any(|c| c.file == f));
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
            self.forget_typing(change.file, change.remap);
            inferred.extend(self.open_inferred(change.file, &moved).into_iter().map(|(sym, sig)| (change.file, sym, sig)));
        }
        self.prog.main = None;
        let prefix = super::merge::Prefix::of(self);
        for change in changes {
            self.check_file(change.file);
        }
        self.complete_pending_sealed_files();
        self.run_deferred_matches();
        let files: Vec<FileId> = changes.iter().map(|c| c.file).collect();
        self.check_outer_accessors_implemented(Some(&files));
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
        let leftover: Vec<ClassId> = self.derive.scala_reserved.keys().copied().collect();
        for c in leftover {
            self.rebuild_scala_mirror(c);
        }
        self.complete_program_mirrors(Some(&files));
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

    /// Drops what typing the file's bodies produced and cached, the anonymous classes its
    /// expansions made included. The diagnostics of the bodies go, to be reported again; those
    /// of what is resolved once (a signature, a parent, an import) move with the edit.
    fn forget_typing(&mut self, file: FileId, remap: &Remap) {
        self.index_forget_bodies(file);
        self.forget_unused(file, remap);
        for sites in self.inline_deps.values_mut() {
            sites.remove(&file);
        }
        let syms = &self.syms;
        let sym_in = |s: &SymId| syms.sym(*s).file == file;
        if let Some(d) = self.deps.as_deref_mut() {
            d.forget(|c| match c {
                super::deps::Comp::Body(s) => syms.sym(s).file == file,
                super::deps::Comp::ClassBody(k) => syms.class_of_files(k, std::slice::from_ref(&file)),
                _ => false,
            });
        }
        let class_in = |c: &ClassId| syms.class_of_files(*c, std::slice::from_ref(&file));
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
        self.prog.template_calls.retain(|&(_, unit)| unit != file);
        self.prog.get_class_units.retain(|&unit| unit != file);
        self.prog_index = super::check::ProgIndex::default();
        self.roots_memo.clear();
        self.subclasses = super::overload::SubclassIndex::default();
        self.anon_sites.retain(|&(f, _, site), _| f != file && site.map_or(true, |(sf, _)| sf != file));
        self.mixin_supers.retain(|c, _| !class_in(c));
        self.super_problems_told.retain(|(c, _), _| !class_in(c));
        // The outer-this field of an inner class is named by the class's position, which the
        // edit may have moved.
        let renamed: Vec<(SymId, String)> = self
            .outer_this
            .local
            .iter()
            .filter(|&(c, _)| class_in(c))
            .map(|(&c, &s)| (s, format!("$this{}", self.prog.position(file, syms.class(c).span.start))))
            .collect();
        for (s, name) in renamed {
            let name = self.interner.intern(&name);
            self.syms.sym_mut(s).name = name;
        }
        self.forget_anon_subclasses(file);
        self.quote.forget_file(file);
        self.forget_capture(file);
        self.diags.items.retain_mut(|d| {
            if d.file != file {
                return true;
            }
            match moved(remap, d.span).filter(|_| !d.of_body) {
                Some(span) => {
                    d.span = span;
                    true
                }
                None => false,
            }
        });
        for (c, sym) in self.scala_mirrors_of_file(file) {
            self.reserve_scala_mirror(c, sym);
        }
    }

    /// The anonymous and local classes of a file are made anew when its bodies are typed again;
    /// the old ones no longer extend anything.
    fn forget_anon_subclasses(&mut self, file: FileId) {
        for i in 0..self.syms.classes.len() {
            let info = &self.syms.classes[i];
            if (info.file != file && info.made_at != Some(file)) || info.owner != Owner::Local {
                continue;
            }
            let Some(parent) = info.superclass else { continue };
            let c = ClassId(i as u32);
            self.syms.class_mut(parent).subclasses.retain(|&d| d != c);
        }
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
        ret == old.ret || self.same_up_to_refinement_params(ret, old.ret, 0)
    }

    /// Whether `a` and `b` are one type written with other symbols for the parameters of the
    /// methods their refinements declare, as a function type naming its parameters is each
    /// time it is typed (`(x: C) => x.M` of `depmeth` eta-expanded).
    fn same_up_to_refinement_params(&mut self, a: TypeId, b: TypeId, depth: u32) -> bool {
        if a == b {
            return true;
        }
        if depth > 16 {
            return false;
        }
        match (self.types.get(a), self.types.get(b)) {
            (Type::Class(c, xs), Type::Class(d, ys)) if c == d => {
                let (xs, ys) = (self.types.items(xs).to_vec(), self.types.items(ys).to_vec());
                xs.len() == ys.len() && xs.iter().zip(&ys).all(|(&x, &y)| self.same_up_to_refinement_params(x, y, depth + 1))
            }
            (Type::Refined(pa, ra), Type::Refined(pb, rb)) => {
                let (Refinement::Term(na, sa, la), Refinement::Term(nb, sb, lb)) = (self.types.refinement(ra), self.types.refinement(rb)) else {
                    return ra == rb && self.same_up_to_refinement_params(pa, pb, depth + 1);
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
                        self.same_up_to_refinement_params(x, y, depth + 1)
                    })
                    && self.same_up_to_refinement_params(pa, pb, depth + 1)
            }
            _ => false,
        }
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
