//! What the measurement of the parallel typer
//! adds to the typer, kept out of the modules it measures so that their code stays as it is with the
//! measurement off: the macro reach's modes and make-up, the bodies counted by their origin and
//! reason, the types the workers made and which of them escape.

use super::Worker;
use crate::source::FileId;
use crate::symbols::*;
use crate::tir::TPat;
use crate::types::*;

/// `TEQ_MACRO_REACH`: `quotes` (the default, the files with quoted code typed before the fork)
/// or `off` (none); none for another value, which the driver refuses (`reach_mode_error`).
fn reach_mode() -> Option<bool> {
    match std::env::var("TEQ_MACRO_REACH").ok().as_deref() {
        None | Some("") | Some("quotes") => Some(true),
        Some("off") => Some(false),
        Some(_) => None,
    }
}

/// The message for a value of `TEQ_MACRO_REACH` no build takes (`full`, an earlier mode, is gone).
pub fn reach_mode_error() -> Option<String> {
    reach_mode().is_none().then(|| "TEQ_MACRO_REACH is quotes (the default, the files with quoted code typed before the fork) or off (none)".to_string())
}

impl<'a> Worker<'a> {
    /// The program files typed before the fork: the files with quoted code (`quoted_files`), or
    /// with `TEQ_MACRO_REACH=off` none (`reach_mode`); their make-up recorded while the
    /// measurement's counters run.
    pub(super) fn prefix_reach(&self, order: &[FileId]) -> Vec<FileId> {
        if reach_mode() == Some(false) {
            return Vec::new();
        }
        let files = self.quoted_files(order);
        if crate::measure::on() {
            let n = self.files.len().min(self.asts.len());
            let program = (0..n).filter(|&i| !self.source(FileId(i as u32)).is_std).count();
            crate::measure::reach_measured(crate::measure::Reach { mode: "quotes", program_files: program, rings: vec![files.len()], names: Vec::new(), files: files.len(), fanout: Vec::new() });
        }
        files
    }

    /// `check_class`'s check under the measurement: a std or jar class's check charged to its
    /// category of the loader's lock's holds, a std class's key captured.
    #[cold]
    #[inline(never)]
    pub(super) fn check_class_counted(&mut self, c: ClassId) {
        let _measured = if self.forked && !self.program_file(self.syms.class(c).file) {
            crate::measure::hold_for(if self.is_library_class(c) { crate::measure::Hold::LibraryClassCheck } else { crate::measure::Hold::StdClassCheck })
        } else {
            None
        };
        if super::prep::capturing() && !self.is_library_class(c) {
            self.prep_note_class('K', c);
        }
        self.check_class_timed(c);
    }

    /// `type_body` under the measurement (`measure`, `prep`): the body counted by its origin
    /// and reason, with its own time, and its key captured.
    #[cold]
    #[inline(never)]
    pub(super) fn type_body_counted(&mut self, sym: SymId, sig: &MethodSig, expected: Option<TypeId>) -> TypeId {
        let inferred = crate::measure::typing_inferred(false);
        let measured = crate::measure::on();
        if measured {
            crate::measure::body_begin();
        }
        let ty = self.type_body_plain(sym, sig, expected);
        let inline = self.is_inline_callee(sym);
        if measured {
            self.body_measured(sym, inferred, inline);
        }
        if super::prep::capturing() && !inline {
            self.prep_note_member('B', sym);
        }
        ty
    }

    #[cold]
    fn body_measured(&self, sym: SymId, inferred: bool, inline: bool) {
        use crate::measure::{BodyOrigin, BodyReason};
        // A library body's pseudo file, and the anonymous classes an expansion copies from a
        // library's quoted code, which stand in it, are the library's.
        let origin = if self.is_library_member(sym) || self.in_jar(self.syms.sym(sym).file) {
            BodyOrigin::Library
        } else if self.source(self.syms.sym(sym).file).is_std {
            BodyOrigin::Std
        } else {
            BodyOrigin::Program
        };
        let reason = if inferred {
            BodyReason::Inferred
        } else if inline {
            BodyReason::Inline
        } else if crate::measure::macro_runs() > 0 || self.inline.folding || self.walk.done {
            BodyReason::Demand
        } else {
            BodyReason::Walk
        };
        crate::measure::body_end(origin, reason);
    }

    /// Of the types the forked workers made (from the fork to the join), how many a record kept
    /// past the merge reaches, directly or through the type the merge made of it again
    /// (`measure::TypeCounts`, `TEQ_WORKERS_TYPES=1`): whether they could have stayed in a
    /// store of their worker's, an overlay, with the escaping
    /// ones interned into the prefix.
    pub(super) fn types_escaping(&self, types_from: u32, remade: impl Fn(TypeId) -> Option<TypeId>, at_join: u32) {
        if !self.types.overlays().is_empty() {
            return;
        }
        let types = &self.types;
        let n = types.len().0 as usize;
        let mut marked = vec![false; n];
        let mut todo: Vec<TypeId> = Vec::new();
        let mut mark = |t: TypeId, todo: &mut Vec<TypeId>| {
            if let Some(m) = marked.get_mut(t.0 as usize) {
                if !*m {
                    *m = true;
                    todo.push(t);
                }
            }
        };
        let mut counts = crate::measure::TypeCounts { at_fork: self.types_at_fork, at_join, after_merge: n as u32, ..Default::default() };
        let sig_types = |sig: &MethodSig, todo: &mut Vec<TypeId>, mark: &mut dyn FnMut(TypeId, &mut Vec<TypeId>)| {
            mark(sig.ret, todo);
            for c in &sig.clauses {
                for p in &c.params {
                    mark(p.ty, todo);
                }
            }
        };
        for s in self.syms.syms.iter() {
            if let Some(sig) = &s.sig {
                counts.sigs += 1;
                sig_types(sig, &mut todo, &mut mark);
            }
        }
        for c in self.syms.classes.iter() {
            for &t in &c.parents {
                mark(t, &mut todo);
            }
            for &(_, t) in &c.base_types {
                mark(t, &mut todo);
            }
            for t in [c.underlying, c.declared_self, c.this_type].into_iter().flatten() {
                mark(t, &mut todo);
            }
            for clause in &c.ctor {
                for p in &clause.params {
                    mark(p.ty, &mut todo);
                }
            }
        }
        for p in self.syms.tparams.iter() {
            mark(p.upper, &mut todo);
            mark(p.lower, &mut todo);
        }
        for a in self.syms.aliases.iter() {
            mark(a.rhs, &mut todo);
            if let Some((lo, hi)) = a.bounds {
                mark(lo, &mut todo);
                mark(hi, &mut todo);
            }
        }
        for q in self.prog.quotes.iter() {
            mark(q.ty, &mut todo);
        }
        for q in self.prog.quote_pats.iter() {
            mark(q.ty, &mut todo);
        }
        for &t in self.prog.deferred_tests.values() {
            mark(t, &mut todo);
        }
        for m in &self.deferred_matches {
            mark(m.sty, &mut todo);
            for &(_, t, _) in &m.gadt {
                mark(t, &mut todo);
            }
        }
        for b in &self.deferred_bounds {
            for t in b.types() {
                mark(t, &mut todo);
            }
        }
        for &t in self.captured_locals.values().chain(self.unchecked_variance.local.values()) {
            mark(t, &mut todo);
        }
        for v in self.outer_prefixes.values() {
            for &(_, t) in v {
                mark(t, &mut todo);
            }
        }
        for d in self.quote.deferred.local.values() {
            sig_types(&d.sig, &mut todo, &mut mark);
            for &(_, t) in d.owner_subst.iter().chain(d.subst.iter()) {
                mark(t, &mut todo);
            }
            for t in [Some(d.ret_ty), d.prefix, d.expected].into_iter().flatten() {
                mark(t, &mut todo);
            }
        }
        for pat in self.prog.pats.iter() {
            if let TPat::Test(_, t, _) | TPat::Class(_, t, _, _) = *pat {
                mark(t, &mut todo);
            }
        }
        for i in 0..self.prog.exprs.len() as u32 {
            if let Some(t) = self.prog.expr_types.get(i).filter(|&t| t != crate::tir::NO_TYPE) {
                counts.exprs += 1;
                mark(t, &mut todo);
            }
        }
        let mut kids: Vec<TypeId> = Vec::new();
        while let Some(t) = todo.pop() {
            kids.clear();
            let vars = |v: crate::types::TVarId, kids: &mut Vec<TypeId>| {
                let info = &self.tvars[v];
                kids.extend(info.inst.iter().chain(&info.lower).chain(&info.upper).copied());
            };
            match types.get(t) {
                Type::Class(_, args) | Type::AppParam(_, args) | Type::Alias(_, args) => kids.extend_from_slice(types.items(args)),
                Type::Select(p, _) | Type::Member(p, _) => kids.push(p),
                Type::Lambda(ps, b) | Type::Poly(ps, b) => {
                    kids.extend_from_slice(types.items(ps));
                    kids.push(b);
                }
                Type::Union(a, b) | Type::Inter(a, b) | Type::BoundedWild(a, b) => kids.extend([a, b]),
                Type::AppVar(v, args) => {
                    kids.extend_from_slice(types.items(args));
                    vars(v, &mut kids);
                }
                Type::Var(v) => vars(v, &mut kids),
                Type::AppMember(m, args) => {
                    kids.push(m);
                    kids.extend_from_slice(types.items(args));
                }
                Type::Refined(p, r) => {
                    kids.push(p);
                    match types.refinement(r) {
                        Refinement::Alias(_, x) | Refinement::Val(_, _, x) => kids.push(x),
                        Refinement::Bounds(_, lo, hi) => kids.extend([lo, hi]),
                        Refinement::Term(_, _, l) => kids.extend_from_slice(types.items(l)),
                    }
                }
                Type::Match(s, m) => {
                    kids.push(s);
                    let info = types.match_info(m);
                    for c in info.cases.iter() {
                        kids.extend_from_slice(types.items(c.binders));
                        kids.extend([c.pattern, c.body]);
                    }
                    kids.push(info.bound);
                }
                _ => {}
            }
            for &k in &kids {
                mark(k, &mut todo);
            }
        }
        for i in types_from..at_join.min(n as u32) {
            let made = remade(TypeId(i)).filter(|&m| m.0 != i);
            if marked[i as usize] || made.is_some_and(|m| marked.get(m.0 as usize).copied().unwrap_or(false)) {
                counts.escaping += 1;
            }
        }
        crate::measure::types_counted(counts);
    }
}

impl<'a> Worker<'a> {
    /// The workers joined, before the merge: the routes no longer noted, the overlays' counts
    /// and memory taken, and the store one index per sub-store again (`join_overlays`).
    #[inline(never)]
    pub(super) fn overlays_joined(&mut self) {
        self.types.stop_noting();
        self.profile.sig_hook = self.profile.on;
        self.types.note_memory("the join");
        self.types.note_overlay_counts();
        self.types.join_overlays();
    }

    /// A barrier of the merge (`TEQ_WORKERS_MEMORY=1`): the store's components, the variables'
    /// tables, the merge's own tables and the allocator's account, beside the process's footprint.
    #[cold]
    #[inline(never)]
    pub(super) fn note_merge_memory(&self, barrier: &str, merge_tables: usize) {
        if !crate::measure::memory_wanted() {
            return;
        }
        self.types.note_memory(barrier);
        let b = |n: usize| crate::report::bytes(n as u64);
        let vars = (self.tvars.len() + self.tvars.others_len() + self.tvars.kept_len()) * std::mem::size_of::<super::TVarInfo>();
        let held = crate::alloc::held();
        crate::measure::overlays_measured(vec![crate::measure::OverlayRow {
            label: format!("memory at {}, beside the store", barrier),
            count: None,
            time: None,
            note: format!(
                "variables' tables {} ({} own, {} others', {} kept), the merge's tables {}; allocator reserved {}, free in its centre {}, large {}, mapped {}",
                b(vars),
                self.tvars.len(),
                self.tvars.others_len(),
                self.tvars.kept_len(),
                b(merge_tables),
                b(held.reserved),
                b(held.centre_free),
                b(held.large),
                b(held.mapped)
            ),
        }]);
    }

    /// The merge done (`merge_workers`): with the overlays measured
    /// (`TEQ_WORKERS_TYPES=1`), what its promotion did and the time of each of its three steps,
    /// the promotion with the renumbering, the check and the reclamation of the overlays.
    #[cold]
    #[inline(never)]
    pub(super) fn merge_measured(&self, c: super::merge::PromotionCounts, uncovered: usize, times: [std::time::Duration; 3], checked: u64, swept: bool) {
        if !crate::measure::types_wanted() {
            return;
        }
        let row = |label: &str, count: Option<u64>, time: Option<std::time::Duration>, note: String| crate::measure::OverlayRow { label: label.to_string(), count, time, note };
        crate::measure::overlays_measured(vec![
            row(
                "merge: the promotion and the renumbering",
                Some(c.overlay_types + c.late_remade),
                Some(times[0]),
                format!(
                    "{} overlay types interned into the base ({} found there), {} of the base's types made after the fork made again and {} kept as they are, {} types appended; variables: {} of the merged worker's {} kept, {} of the other workers' {}; {} records made outside every work item",
                    c.overlay_types, c.found, c.late_remade, c.late_kept, c.appended, c.own_kept, c.own_vars, c.vars_kept, c.other_vars, uncovered
                ),
            ),
            row(
                "merge: the check",
                Some(if swept { checked } else { c.overlay_types + c.late_remade + c.late_kept }),
                Some(times[1]),
                if swept { "the sweep: every record walked from the first of each arena, the types made after the fork checked part for part".to_string() } else { format!("the roots: the types the promotion handed the records, checked as they were handed over ({} left)", c.roots_left) },
            ),
            row("merge: the overlays given back", None, Some(times[2]), String::new()),
        ]);
    }
}

/// What a cloned table of the attachment holds: its entries and the bytes of its table (entries
/// and control bytes at the capacity; what a value points to is the clone's time, not counted).
pub(super) trait Footprint {
    fn entries(&self) -> usize;
    fn table_bytes(&self) -> usize;
}

impl<K, V> Footprint for crate::intern::FxMap<K, V> {
    fn entries(&self) -> usize {
        self.len()
    }
    fn table_bytes(&self) -> usize {
        self.capacity() * (std::mem::size_of::<K>() + std::mem::size_of::<V>() + 1)
    }
}

impl<K: Copy + Eq + std::hash::Hash> Footprint for super::exports::ExportTables<K> {
    fn entries(&self) -> usize {
        self.len()
    }
    fn table_bytes(&self) -> usize {
        0
    }
}

impl<T> Footprint for Vec<T> {
    fn entries(&self) -> usize {
        self.len()
    }
    fn table_bytes(&self) -> usize {
        self.capacity() * std::mem::size_of::<T>()
    }
}

/// The attachment's maps at the fork (`TEQ_FORK_INVENTORY=1`): each one's entries, by name, for
/// the join's count of the workers that grew it.
static INVENTORY_AT_FORK: std::sync::Mutex<Vec<(&'static str, usize)>> = std::sync::Mutex::new(Vec::new());

/// The inventory's maps of a worker, by name: entries now.
macro_rules! inventory_maps {
    ($w:expr, $f:ident) => {{
        let w = $w;
        vec![
            $f!(w, soft_exprs),
            $f!(w, soft_syms),
            $f!(w, interpolations),
            $f!(w, conversion_indexes),
            $f!(w, inferred_outcomes),
            $f!(w, inferred_val_outcomes),
            $f!(w, poisoned),
            $f!(w, poisoned_withheld),
            $f!(w, given_indexes),
            $f!(w, value_sigs),
            $f!(w, temp_names),
            $f!(w, file_chains),
            $f!(w, class_exports),
            $f!(w, pkg_exports),
            $f!(w, class_given_indexes),
            $f!(w, given_preferences),
            $f!(w, given_orders),
            $f!(w, implicit_scopes),
            $f!(w, implicit_scope_givens),
            $f!(w, given_fast),
            $f!(w, given_fits),
            $f!(w, head_sigs),
            $f!(w, package_levels),
            $f!(w, derived_aliases),
            $f!(w, sam_classes),
            $f!(w, expr_marks),
            $f!(w, eta_expansions),
            $f!(w, folded_paths),
            $f!(w, captured_locals),
            $f!(w, anon_envs),
            $f!(w, irrefutable_pats),
            $f!(w, tag_pats),
            $f!(w, local_news),
            $f!(w, anon_captures),
            $f!(w, anon_parent_args),
            $f!(w, mixin_supers),
            $f!(w, super_problems_told),
            $f!(w, anon_sites),
            $f!(w, reflect_ctor_params),
            $f!(w, reflect_param_of),
            $f!(w, suffixed_roots),
            $f!(w, roots_memo),
            $f!(w, outer_prefixes),
            $f!(w, sam_arity),
            $f!(w, macro_files),
        ]
    }};
}

impl<'a> Worker<'a> {
    /// The inventory of the attachment (`TEQ_FORK_INVENTORY=1`): every
    /// table `attach` clones or attaches, cloned once more apart and timed, with its entries and
    /// its table's bytes at the prefix. A measurement: the clones are dropped.
    #[cold]
    #[inline(never)]
    pub(super) fn fork_inventory(&self) {
        let mut rows = Vec::new();
        macro_rules! timed {
            ($name:expr, $e:expr, $size:expr) => {{
                let t = std::time::Instant::now();
                let c = std::hint::black_box($e);
                let d = t.elapsed();
                let (n, bytes): (Option<usize>, usize) = $size(&c);
                rows.push(crate::measure::OverlayRow { label: format!("fork inventory: {}", $name), count: n.map(|n| n as u64), time: Some(d), note: if bytes > 0 { format!("table {}", crate::report::bytes(bytes as u64)) } else { String::new() } });
                drop(c);
            }};
        }
        fn none<T>(_: &T) -> (Option<usize>, usize) {
            (None, 0)
        }
        fn map<T: Footprint>(m: &T) -> (Option<usize>, usize) {
            (Some(m.entries()), m.table_bytes())
        }
        macro_rules! cloned {
            ($($f:ident),*) => { $( timed!(stringify!($f), self.$f.clone(), map); )* };
        }
        timed!("syms (Symbols::attach)", self.syms.attach(1), none);
        timed!("prog (Program::attach)", self.prog.attach(1), none);
        timed!("b (Builtins)", self.b.clone(), none);
        timed!("def_syms (FileMaps)", self.def_syms.attach(), none);
        timed!("def_classes (FileMaps)", self.def_classes.attach(), none);
        timed!("def_aliases (FileMaps)", self.def_aliases.attach(), none);
        timed!("file_pkgs (FileVec)", self.file_pkgs.attach(1), none);
        timed!("file_imports (FileVec)", self.file_imports.attach(1), none);
        timed!("file_opaques (FileVec)", self.file_opaques.attach(1), none);
        timed!("tvars (TVars::attach)", super::TVars::attach(&self.tvars, 1, self.tvars_at_fork), none);
        timed!("index (Index::attach)", self.index.as_ref().map(|ix| Box::new(ix.attach())), none);
        timed!("sites (DefSites)", self.sites.clone(), none);
        timed!("walk (Walk)", self.walk.clone(), none);
        timed!("derive (Derivation::attach)", self.derive.attach(), none);
        timed!("quote (QuoteState::attach)", self.quote.attach(), none);
        timed!("inline (InlineState::worker)", super::inline::InlineState::worker(&self.inline), none);
        timed!("the Layered tables' attach (18)", (self.class_imports.attach(), self.fun_of_sym.attach(), self.val_init.attach(), self.class_done.attach(), self.retained_bodies.attach(), self.inline_definitions.attach(), self.derived_opaques.attach(), self.arity_classes.attach(), self.inferred_parent_args.attach(), self.inferred_trait_args.attach(), self.any_members.attach(), self.outer_this.attach(), self.outer_accessors.attach(), self.unchecked_variance.attach(), self.merged_overloads.attach()), none);
        timed!("class_exports (ExportTables::attach)", self.class_exports.attach(), map);
        timed!("pkg_exports (ExportTables::attach)", self.pkg_exports.attach(), map);
        cloned!(soft_exprs, soft_syms, interpolations, conversion_indexes, inferred_outcomes, inferred_val_outcomes, poisoned, poisoned_withheld, given_indexes, value_sigs, temp_names, file_chains, class_given_indexes, given_preferences, given_orders, implicit_scopes, implicit_scope_givens, given_fast, given_fits, derived_aliases, sam_classes, expr_marks, eta_expansions, folded_paths, captured_locals, anon_envs, irrefutable_pats, tag_pats, local_news, anon_captures, anon_parent_args, mixin_supers, super_problems_told, anon_sites, reflect_ctor_params, reflect_param_of, suffixed_roots, roots_memo, outer_prefixes, sam_arity, macro_files, cacheable_state, cacheable_state_classes, head_sigs, package_levels);
        timed!("the Arc handles (types, import_hidden, import_values, js_registry, loaded, std, lock, serial, shared_mixin_supers)", (self.types.clone(), self.import_hidden.clone(), self.import_values.clone(), self.js_registry.clone(), self.loaded.clone(), self.std.clone(), self.lock.clone(), self.serial.clone(), self.shared_mixin_supers.clone()), none);
        let t = std::time::Instant::now();
        let w = std::hint::black_box(self.attach(1));
        let whole = t.elapsed();
        drop(w);
        rows.push(crate::measure::OverlayRow { label: "fork inventory: one whole attach".to_string(), count: None, time: Some(whole), note: String::new() });
        macro_rules! len_of {
            ($w:ident, $f:ident) => {
                (stringify!($f), Footprint::entries(&$w.$f))
            };
        }
        *INVENTORY_AT_FORK.lock().unwrap_or_else(|e| e.into_inner()) = inventory_maps!(self, len_of);
        crate::measure::overlays_measured(rows);
    }

    /// The join's half of the inventory: per map, how many of the workers grew it in the body
    /// phase and by how many entries, summed (a write that replaced an entry is not seen).
    #[cold]
    #[inline(never)]
    pub(super) fn fork_inventory_at_join(&self, others: &[Option<Worker<'a>>]) {
        let at_fork = std::mem::take(&mut *INVENTORY_AT_FORK.lock().unwrap_or_else(|e| e.into_inner()));
        if at_fork.is_empty() {
            return;
        }
        macro_rules! len_of {
            ($w:ident, $f:ident) => {
                (stringify!($f), Footprint::entries(&$w.$f))
            };
        }
        let mut grew: Vec<(usize, usize)> = vec![(0, 0); at_fork.len()];
        for w in std::iter::once(self).chain(others.iter().flatten()) {
            for (i, (_, n)) in inventory_maps!(w, len_of).into_iter().enumerate() {
                if n > at_fork[i].1 {
                    grew[i].0 += 1;
                    grew[i].1 += n - at_fork[i].1;
                }
            }
        }
        let workers = others.len() + 1;
        crate::measure::overlays_measured(
            at_fork
                .iter()
                .zip(grew)
                .map(|(&(name, n), (k, added))| crate::measure::OverlayRow { label: format!("fork inventory at the join: {}", name), count: Some(n as u64), time: None, note: format!("grown by {} of {} workers, {} entries added", k, workers, added) })
                .collect(),
        );
    }
}
