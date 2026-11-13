//! What the code at a place stands in beyond its environment, and the one list of the Worker's fields by what they
//! are to a definition typed where it was made: dotty's completer runs in the context it was created in
//! (`Namer.Completer.creationContext`), whatever asked for the definition.

use super::Worker;
use crate::symbols::*;
use crate::tir::TExprId;
use crate::types::*;

/// The Worker's fields, each under its class. A field added to the Worker and not named here does not compile
/// (`Worker::every_field_classed`), and neither does a name here that is no field. The first of each pair of classes
/// is the list a definition's place puts up (`Worker::where_made`); the second waits: recorded, told by the doors'
/// dump, and left as the asking code has it until a program shows it at a door.
macro_rules! worker_fields {
    (
        place { $($p:ident : $pt:ty),* $(,)? }
        place_waiting { $($pw:ident : $pwt:ty),* $(,)? }
        held { $($h:ident : $ht:ty),* $(,)? }
        stacks { $($s:ident),* $(,)? }
        stacks_waiting { $($sw:ident),* $(,)? }
        own { $($o:ident is $on:ident),* $(,)? }
        own_waiting { $($ow:ident is $own:ident),* $(,)? }
        parts { $($part:ident),* $(,)? }
        own_way { $($w:ident),* $(,)? }
        store { $($st:ident),* $(,)? }
        thread { $($th:ident),* $(,)? }
        unread { $($u:ident),* $(,)? }
    ) => {
        /// The settings of the place: a value each, the one to type under there.
        #[derive(Clone, Copy, Debug, Default, PartialEq)]
        pub struct Settings {
            $(pub $p: $pt,)*
            $(pub $pw: $pwt,)*
            $(pub $h: $ht,)*
        }

        /// The stacks of the place: how long each stood there.
        #[derive(Clone, Copy, Debug, Default, PartialEq)]
        pub struct Lengths {
            $(pub $s: u32,)*
            $(pub $sw: u32,)*
        }

        /// For the doors' dump alone: a mark of each stack's content where the record was taken, which tells an
        /// entry below the recorded length changed in place since.
        #[derive(Clone, Copy, Debug, Default, PartialEq)]
        pub struct Contents {
            $(pub $s: u64,)*
            $(pub $sw: u64,)*
        }

        impl<'a> Worker<'a> {
            /// What the code being typed stands in here.
            pub(super) fn standing_here(&self) -> Standing {
                Standing {
                    settings: Settings { $($p: self.$p,)* $($pw: self.$pw,)* $($h: self.$h,)* },
                    lengths: Lengths { $($s: self.$s.len() as u32,)* $($sw: self.$sw.len() as u32,)* },
                    contents: if dump_on() { Contents { $($s: content(&self.$s),)* $($sw: content(&self.$sw),)* } } else { Contents::default() },
                    parts: self.parts_here(),
                }
            }

            /// Runs `f` with the list put up as `made` has it and gives the asking code's back after: a setting takes
            /// the place's value; a stack is held to the length it had at the place, what stands past it set aside
            /// and what `f` leaves past it dropped; a value that is the asking code's own is taken, so that `f` starts
            /// with none.
            fn in_standing<R>(&mut self, made: &Standing, f: impl FnOnce(&mut Self) -> R) -> R {
                $(let $p = std::mem::replace(&mut self.$p, made.settings.$p);)*
                $(
                    let $s = {
                        let n = (made.lengths.$s as usize).min(self.$s.len());
                        (n, self.$s.split_off(n))
                    };
                )*
                $(let $o = std::mem::take(&mut self.$o);)*
                let r = f(self);
                $(self.$o = $o;)*
                $(
                    self.$s.truncate($s.0);
                    self.$s.extend($s.1);
                )*
                $(self.$p = $p;)*
                r
            }

            /// The fields of the list in which the state here is not `made`'s: what `in_standing` changes.
            fn list_differs(&self, made: &Standing) -> Vec<&'static str> {
                let mut out: Vec<&'static str> = Vec::new();
                $(if self.$p != made.settings.$p { out.push(stringify!($p)); })*
                $(if self.$s.len() as u32 != made.lengths.$s { out.push(stringify!($s)); })*
                $(if !neutral::$on(&self.$o) { out.push(stringify!($o)); })*
                out
            }

            /// The stacks whose entries below `made`'s length are not what they were where `made` was taken, or that
            /// stand shorter than there.
            fn stacks_changed_below(&self, made: &Standing) -> Vec<&'static str> {
                let mut out: Vec<&'static str> = Vec::new();
                $(
                    let n = made.lengths.$s as usize;
                    if self.$s.len() < n {
                        out.push(concat!(stringify!($s), " (shorter than at the place)"));
                    } else if n > 0 && content(&self.$s[..n]) != made.contents.$s {
                        out.push(concat!(stringify!($s), " (changed below the place's length)"));
                    }
                )*
                $(
                    let n = made.lengths.$sw as usize;
                    if self.$sw.len() < n {
                        out.push(concat!(stringify!($sw), " (shorter than at the place)"));
                    } else if n > 0 && content(&self.$sw[..n]) != made.contents.$sw {
                        out.push(concat!(stringify!($sw), " (changed below the place's length)"));
                    }
                )*
                out
            }

            /// The fields in which the state here is not `made`'s, the waiting ones too: a setting of another value,
            /// a stack of another length, a value of the asking code's own that is up.
            pub(super) fn standing_differs(&self, made: &Standing) -> Vec<&'static str> {
                let mut out = self.list_differs(made);
                $(if self.$pw != made.settings.$pw { out.push(stringify!($pw)); })*
                $(if self.$h != made.settings.$h { out.push(stringify!($h)); })*
                $(if self.$sw.len() as u32 != made.lengths.$sw { out.push(stringify!($sw)); })*
                $(if !neutral::$own(&self.$ow) { out.push(stringify!($ow)); })*
                self.parts_here().differs(&made.parts, &mut out);
                out
            }

            /// Names every field of the Worker under its class: the settings and the stacks a place records, the
            /// values that are the asking code's own, the parts of a field with a function of its own, and what is no
            /// part of a place (the stores, the thread's bookkeeping, what no body reads).
            #[allow(dead_code)]
            fn every_field_classed(&self) {
                let Worker {
                    // A setting: the value at the place is the value to type under.
                    $($p: _,)*
                    // A setting that waits for its program: left as the asking code has it.
                    $($pw: _,)*
                    // A setting whose class is not settled (a depth compared with the thread's).
                    $($h: _,)*
                    // A stack: the place holds it up to the length it had there.
                    $($s: _,)*
                    // A stack that waits for its program.
                    $($sw: _,)*
                    // The asking code's own: keyed by its depth, a span, a tree or a type, or the state of a
                    // comparison of its own; the definition starts with none.
                    $($o: _,)*
                    // The asking code's own that waits for its program.
                    $($ow: _,)*
                    // A field that holds a part of the place among other things (`Parts`).
                    $($part: _,)*
                    // Set aside by a function of its own: the environment's, a body's own entry, the index's, the
                    // dependencies'.
                    $($w: _,)*
                    // A store: what is known of the program, whoever asks.
                    $($st: _,)*
                    // The thread's bookkeeping: what it has begun and not ended, whichever place asks.
                    $($th: _,)*
                    // Read by no body.
                    $($u: _,)*
                } = self;
            }
        }
    };
}

/// The tests of a value that is the asking code's own being down.
mod neutral {
    pub fn none<T>(o: &Option<T>) -> bool {
        o.is_none()
    }
    pub fn no(b: &bool) -> bool {
        !*b
    }
    pub fn max(n: &u32) -> bool {
        *n == u32::MAX
    }
    pub fn empty<T>(v: &[T]) -> bool {
        v.is_empty()
    }
}

worker_fields! {
    place {
        gadt_open: bool,
        defining: Option<SymId>,
        excluded_ctor: Option<SymId>,
        ctor_outer: Option<(ClassId, SymId)>,
        creation_outer: Option<(ClassId, TExprId)>,
        lexical_selected: Option<bool>,
        view_compat: bool,
        skip_proxies: bool,
        parent_args_of: Option<ClassId>,
        deferred_impl_of: Option<ClassId>
    }
    place_waiting {
        typing_annotation: bool,
        annotation_args: bool,
        inline_val_rhs: bool,
        inline_getters_only: bool,
        search_only_depth: Option<u32>,
        new_prefixed: bool,
        explicit_new: bool,
        apply_selected: bool,
        extension_call_head: bool,
        bare_lookup: bool,
        cycle_at_use: Option<bool>,
        java_varargs: bool,
        note_paths: bool,
        creator_receiver: Option<(ClassId, ClassId)>,
        search_byname: bool,
        converting_receiver: bool,
        implicit_depth: u32,
        implicit_budget: u32
    }
    held {
        inferring_depth: u32
    }
    stacks {
        gadt, bound_path, local_blocks,
    }
    stacks_waiting {
        param_paths, binder_completing, case_binders, lub_in_progress, hoisted, given_stack, given_context,
        given_consulted, given_opened, trace_starts, failed_givens, function_targets,
    }
    own {
        gadt_in_use is empty,
        join_wild is no,
        soft_lub is no,
        interp_call is none,
        interp_parts is none,
        macro_targs is none,
        sames is none,
        simplifying_normal is no
    }
    own_waiting {
        retry_depth is none,
        retry_applied is none,
        member_call is none,
        any_function is none,
        infix_watch is max,
        assign_op_target is none,
        poly_literal_fun is none,
        measured_target is none,
        blocked_conversion is none,
        unsimplified is no,
        given_ambiguity is none,
        unentered_impls is empty,
        necessary_either is no
    }
    parts {
        given_tree, sites, quote, inline,
    }
    own_way {
        env, transparent, ext_scope, refine_scopes, type_def_frames, var_frames, return_to, returns, body_depth,
        unused, index, deps,
    }
    store {
        asts, interner, syms, types, diags, prog, b, def_syms, def_classes, def_aliases, file_pkgs, file_imports,
        import_hidden, import_sels, class_imports, tvars, cache_buffers, fun_of_sym, getters, stable_realizable,
        checked_imports, import_heads, interpolations, val_init, class_done, binder_nowarns, written_purity,
        lone_abstract_defs, path_sigs, unbox_candidates, erased_results, context_function_classes,
        inner_chain_members, plain_name_members, roots_told, java_members_exhausted, conversion_indexes,
        retry_typed, member_calls, infix_tuples, retracted_unapplies, inferred_outcomes, inferred_val_outcomes,
        recovered, erased_values, file_opaques, file_modules, retained_bodies, inline_definitions,
        superseded_definitions, check_capture_keys, conversion_nonvariant, opened, simplified_memo,
        class_base_memo, std_binding_keys, std_binding_names, poisoned, poisoned_withheld, withheld_notes,
        reflective_jar_found, renamed_roots, for_packs, given_indexes, value_sigs, lit_types, temp_names,
        file_chains, class_exports, pkg_exports, class_given_indexes, given_preferences, given_orders,
        implicit_scopes, implicit_scope_givens, package_levels, given_fast, given_fits, head_sigs, base_type_cache,
        target_name_files, rule_member_names, derived_aliases, derived_opaques, import_values, export_blocks,
        js_import_ids, js_registry, js_imports_at_bodies, js_exported, worker, arity_classes, items, diag_ranges,
        diag_region, history_reads, told, meetings, claims, inherited, tclasses_published, tclasses_registered,
        walk_by_size, bodies_started, std_seen, arity_seen, lists_at_fork, tvars_at_fork, files,
        ascription_constants, kind_clauses, inferred_parent_args, inferred_trait_args, sam_classes,
        std_package_misses, inline_accessor_syms, inline_results, expr_marks, eta_expansions, folded_paths,
        folded_deprecated, wildcards_used, captured_locals, spaces, deferred_matches, late_methods, override_pairs,
        derive, anon_envs, irrefutable_pats, tag_pats, arg_types_seen, local_news, lifted_members, lifted_lazy,
        accessor_captures, held_proxies, held_proxy_of, pending_held, anon_captures, anon_parent_args,
        mixin_supers, shared_mixin_supers, super_problems_told, mixins_bound, inference_failed, any_members,
        anon_sites, outer_this, ctor_outer_params, outer_accessors, reflect_ctor_params, reflect_param_of,
        entry_points, deferred_bounds, unchecked_variance, variance_checked, main_name, choose_entry, open_world,
        jvm_volatile, jvm_thread_unsafe, writes_products, emits_bodies, withheld_met, dialect, jvm, unit_ops,
        java_serializable, interp, semantic, sem_records, form_passes, unsettled_std_entries, dispatch_pending,
        named_like, vals_by_accessor, suffixed_roots, prog_index, roots_memo, inherited_named, subclasses,
        alternatives_named, inherited_pairs, loaded, std, entering_done, array_seq_asked, refinement_sigs,
        poly_rebound, outer_prefixes, receiver_types, sam_arity, member_cycles, merged_overloads, views_on,
        profile, other_view_memos, macro_files, cacheable_state, cacheable_state_classes, known_caches,
        macro_state_per_worker, inline_deps, lock, forked, value_forwarders_made, forwarders_completion,
        forward_names, forwarders_on_completion, deferred_lambdas, lambda_results, merge_failed, types_at_fork,
        serial,
    }
    thread {
        trail, mark_floor, dropped_deps, attempts, inlining, attempt_caches, bodies_in_progress, mismatches,
        mismatch_renders, logging, app_depth, error_nodes, pending_widenings, open_vars, conversion_open_vars,
        inline_unboxes, completing, pending_library_names, naming_library, export_stack, export_retry,
        export_retrying, diag_range_open, diag_rank, diag_range_from, tclasses_settling, tclass_settling_scopes,
        tclasses_in_checks, pending_shared, merging, infos, std_entered_pending, loader_done_pending, kind_walk,
        deferred_classes, declaring, declared_views, sealed_std_files, pending_mirror_parents, late_naming_from,
        dep_depth, dep_budget, dep_root, dep_exhausted, singleton_depth, lookups, matches, same_level,
        blocked_clauses, importing_files,
    }
    unread {
        app_base, proto_vars, plain_member, kind_only_bound, binop_rhs, ref_exported, naming_late_batch,
        spread_arg, spread_was_array, last_unapply, infix_called, infix_passed, infix_universal, denotation_marks,
        unadapted_call, unadapted_node, ext_undecided, no_receiver_conversion, declared_call, declared_took,
        infix_declared, direct_ext_targs, ctor_prefix, sig_bound_names, copy_application, case_scrutinee,
        apply_seen_from, private_alias_prefix, dropped_type_list, assign_op_receiver, written_qualifier, joint_sig,
        dependent_checks, last_arg_types, eager_inline_conversion, unboxing_after, tested, alias_bound_as_path,
        for_bases, ext_modules, enum_case_new, ascribed_constant, selections, selection_typed, dead_case,
        partial_match, erroneous_args, erroneous_lambdas, wildcard_results, lambda_levels, body_node,
        variance_skip, branch_types, for_interpreter, erased_tags, declared_lost, declared_alias_hit,
        semantic_alias_ctor, poly_targs, poly_pending, walk, shadowed_object, show_depth, show_qualified,
        show_recs, path_approx, overload_arg_spans, match_pt, last_typed_pattern, pattern_alternatives,
        refutable_is_error, unchecked_typed_pats, for_map_watch, applied_extension, probing_forward_names,
        making_value_forwarders, mapping_signature, predecoded,
    }
}

/// The parts of a place that stand in a field with other things: the owners' chain (`Sites::owners`), the given
/// search's counts (`GivenTree`), the quote's level, quotes, holes and pattern (`QuoteState`), and how deep in
/// expansions and in definition checks the place is (`InlineState`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Parts {
    pub owners: u32,
    pub given_tree: (u32, u32, u32, u32),
    pub quote: (u32, u32, u32, bool),
    pub inline_depth: u32,
    pub inline_checking: u32,
}

impl Parts {
    fn differs(&self, made: &Parts, out: &mut Vec<&'static str>) {
        if self.owners != made.owners {
            out.push("sites.owners");
        }
        if self.given_tree != made.given_tree {
            out.push("given_tree");
        }
        if self.quote != made.quote {
            out.push("quote");
        }
        if self.inline_depth != made.inline_depth {
            out.push("inline.depth");
        }
        if self.inline_checking != made.inline_checking {
            out.push("inline.checking");
        }
    }
}

/// What the code at a place stands in: its settings, its stacks' lengths and the parts.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Standing {
    pub settings: Settings,
    pub lengths: Lengths,
    pub contents: Contents,
    pub parts: Parts,
}

/// What of a stack's entry the dump's content mark reads.
trait Entry {
    fn feed(&self, h: &mut std::collections::hash_map::DefaultHasher);
}

fn content<T: Entry>(entries: &[T]) -> u64 {
    use std::hash::Hasher;
    let mut h = std::collections::hash_map::DefaultHasher::new();
    for e in entries {
        e.feed(&mut h);
    }
    h.finish()
}

macro_rules! entry_by_hash {
    ($($t:ty),* $(,)?) => {
        $(impl Entry for $t {
            fn feed(&self, h: &mut std::collections::hash_map::DefaultHasher) {
                std::hash::Hash::hash(self, h);
            }
        })*
    };
}

entry_by_hash! {
    (TParamId, TypeId, i8), (SymId, TypeId), (crate::intern::Name, TypeId), TParamId, (TypeId, TypeId), u32, SymId,
    std::time::Instant, (SymId, String), TypeId,
}

impl Entry for super::ClauseCompletion {
    fn feed(&self, h: &mut std::collections::hash_map::DefaultHasher) {
        use std::hash::Hash;
        (self.file, &self.ids, &self.own).hash(h);
        for s in &self.states {
            (*s as u8).hash(h);
        }
    }
}

impl Entry for crate::tir::TStmt {
    fn feed(&self, h: &mut std::collections::hash_map::DefaultHasher) {
        std::hash::Hash::hash(&format!("{:?}", self), h);
    }
}

impl Entry for super::implicits::OpenGiven {
    fn feed(&self, h: &mut std::collections::hash_map::DefaultHasher) {
        std::hash::Hash::hash(&self.given, h);
    }
}

impl Entry for super::implicits::Wanted {
    fn feed(&self, h: &mut std::collections::hash_map::DefaultHasher) {
        let key = match *self {
            super::implicits::Wanted::Class(c) => (0u8, c.0),
            super::implicits::Wanted::AnyClass => (1, 0),
            super::implicits::Wanted::Extension(n) => (2, n.0),
        };
        std::hash::Hash::hash(&key, h);
    }
}

impl Entry for super::resolve::LocalBlock {
    fn feed(&self, h: &mut std::collections::hash_map::DefaultHasher) {
        std::hash::Hash::hash(&(self.file, self.frame_depth, self.imports_scope, self.next), h);
    }
}

/// The mark a comparison leaves in the bound of `Worker::gadt` it is using (`is_sub_by_gadt_bound`), in the variance's
/// place.
pub(super) const GADT_IN_USE: i8 = 2;

/// What a door enters: a definition, a class or a type alias.
#[derive(Clone, Copy)]
pub(super) enum Entered {
    Def(SymId),
    Class(ClassId),
    Alias(AliasId),
}

/// Which kind of place a definition's is.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum PlaceKind {
    /// Outside every block: nothing stands around it.
    Top,
    /// A class made in a block, or what such a class owns.
    BlockClass,
    /// A block's own definition.
    Block,
}

impl<'a> Worker<'a> {
    fn parts_here(&self) -> Parts {
        let tree = &self.given_tree;
        Parts {
            owners: self.sites.owners.len() as u32,
            given_tree: (tree.in_progress, tree.restricted, tree.deepest, tree.byname_requests),
            quote: self.quote.standing(),
            inline_depth: self.inline.depth,
            inline_checking: self.inline.checking,
        }
    }

    /// What the place of the definition `sym` held where it was made, with the place's kind: nothing for a
    /// definition outside every block; for a class made in a block and what it owns, what stood where the class
    /// was entered (`InlineState::standing_classes`); for a block's own definition, what stood where the block
    /// opened (`LocalBlock::standing`), found by the frame that names it. `None` where no record tells: a local
    /// symbol that no frame names (a refinement's member, a structural type's), which is no block's definition.
    pub(super) fn standing_of(&self, sym: SymId) -> (PlaceKind, Option<&Standing>) {
        let info = self.syms.sym(sym);
        match info.owner {
            Owner::Local => {
                let (name, file) = (info.name, info.file);
                let frame = self.env.frames.iter().rposition(|f| matches!(f, super::Frame::Locals { names, .. } if names.iter().any(|&(n, s)| n == name && s == sym)));
                let block = frame.and_then(|k| self.local_blocks.iter().rev().find(|b| b.frame_depth == k + 1 && b.file == file));
                let made = block.and_then(|b| b.standing.as_deref());
                // A method or a given a block defines, named by the block's frame, has the block's record.
                debug_assert!(
                    made.is_some() || frame.is_none() || !info.def.is_some_and(|d| matches!(self.ast(file).def(d).kind, crate::ast::DefKind::Fun(_) | crate::ast::DefKind::Given(_))),
                    "a block's definition without its block's record: {}",
                    self.name_str(name)
                );
                (PlaceKind::Block, made)
            }
            Owner::Class(c) => self.standing_of_class(c),
            Owner::Package(_) => (PlaceKind::Top, Some(nothing_standing())),
        }
    }

    /// `standing_of` for the class `c` and what it owns.
    pub(super) fn standing_of_class(&self, mut c: ClassId) -> (PlaceKind, Option<&Standing>) {
        loop {
            if let Some(made) = self.inline.standing_classes.get(&c) {
                return (PlaceKind::BlockClass, Some(made));
            }
            match self.syms.class(c).owner {
                Owner::Class(outer) => c = outer,
                Owner::Local => return (PlaceKind::BlockClass, None),
                Owner::Package(_) => return (PlaceKind::Top, Some(nothing_standing())),
            }
        }
    }

    /// Runs `f`, the typing of the definition `entered` that the door `door` starts, in what the definition's place
    /// holds of the list, whatever asked for it: dotty's completer runs in the context it was created in
    /// (`Namer.Completer.completeInCreationContext`). The place of a definition outside every block holds nothing; a
    /// class made in a block, and what it owns, what stood where the class was entered; a block's own definition,
    /// what stood where its block opened. What the door sets aside itself stays the door's, and what it sets for
    /// the definition it sets inside `f`.
    ///
    /// A comparison of the asking code's that is using a bound (`gadt_in_use`) has its mark lifted from the bounds
    /// the place holds while `f` runs, and set again after: for the definition none is in use. The quote's level,
    /// its quotes, holes and pattern are the place's together (`QuoteState::put_up`). And where the place stood
    /// outside every definition check and one is under way, the whole of the check's state is set aside
    /// (`outside_inline`): the definition is typed as the walk types it, its inline calls expanded.
    ///
    /// Left as the asking code has them: the definitions of the std and of the libraries, which keep what their
    /// doors set aside (the part that puts their places up proves it by a sweep of every program, not by a
    /// program a field); and a local symbol no frame names (a refinement's member, a structural type's), which has
    /// no block.
    pub(super) fn where_made<R>(&mut self, door: &'static str, entered: Entered, f: impl FnOnce(&mut Self) -> R) -> R {
        let file = match entered {
            Entered::Def(s) => self.syms.sym(s).file,
            Entered::Class(c) => self.syms.class(c).file,
            Entered::Alias(a) => self.syms.alias(a).file,
        };
        if !self.program_file(file) {
            return f(self);
        }
        let (kind, made) = match entered {
            Entered::Def(s) => self.standing_of(s),
            Entered::Class(c) => self.standing_of_class(c),
            Entered::Alias(a) => self.standing_of_alias(a),
        };
        let Some(made) = made else {
            // A class made in a block has its record, and so has what it owns (`standing_of` asserts the same of a
            // block's own method or given).
            debug_assert!(kind != PlaceKind::BlockClass, "a class of a block without its record at the door {}", door);
            if dump_on() {
                eprintln!("PUTUP\t{}\t{}\t\t\t", door, if kind == PlaceKind::BlockClass { "NORECORD" } else { "NOFRAME" });
            }
            return f(self);
        };
        // The record for the time of the typing: the one of a place outside every block is nobody's to change and is
        // read where it is; a block's or a class's is copied, once, since the typing may move what holds it.
        let copy;
        let made: &Standing = if kind == PlaceKind::Top {
            nothing_standing()
        } else {
            copy = *made;
            &copy
        };
        if dump_on() {
            self.dump_put_up(door, entered, made);
        }
        // The bounds the place holds that a comparison is using: for the definition none is.
        let lifted: Vec<(u32, i8)> = self.gadt_in_use.iter().copied().filter(|&(i, _)| i < made.lengths.gadt && (i as usize) < self.gadt.len()).collect();
        for &(i, variance) in &lifted {
            self.gadt[i as usize].2 = variance;
        }
        let quote = self.quote.put_up(made.parts.quote);
        let checked = made.parts.inline_checking == 0 && self.inline.checking > 0 && self.inline.depth == made.parts.inline_depth;
        let r = self.in_standing(made, |t| {
            let r = if checked { t.outside_inline(f) } else { f(t) };
            debug_assert!(t.gadt_in_use.is_empty(), "a definition's typing left a bound in use");
            r
        });
        self.quote.give_back(quote);
        for &(i, variance) in &lifted {
            debug_assert!(self.gadt[i as usize].2 == variance, "a definition's typing changed a bound its place holds");
            self.gadt[i as usize].2 = GADT_IN_USE;
        }
        r
    }

    /// Runs `f`, which enters the block's own definition `sym`, in the frames of its block: the environment down to
    /// the frame that names it, where the asking code stands deeper (a nested scope's names, the parameters of the
    /// method that calls it, a class of the block are none of the definition's). The imports are those of the
    /// frames kept, as the block has entered them so far. A block of the program's: a body of the std or of a library
    /// is entered in what is at hand, as `where_made` leaves it.
    pub(super) fn in_block_frames<R>(&mut self, sym: SymId, f: impl FnOnce(&mut Self) -> R) -> R {
        let (name, file) = {
            let info = self.syms.sym(sym);
            (info.name, info.file)
        };
        if !self.program_file(file) {
            return f(self);
        }
        let frame = self.env.frames.iter().rposition(|fr| matches!(fr, super::Frame::Locals { names, .. } if names.iter().any(|&(n, s)| n == name && s == sym)));
        let Some(k) = frame.filter(|&k| k + 1 < self.env.frames.len()) else { return f(self) };
        let mut env = self.env.clone();
        env.frames.truncate(k + 1);
        env.imports.retain(|i| i.depth as usize <= k + 1);
        let saved = std::mem::replace(&mut self.env, env);
        let saved_transparent = std::mem::take(&mut self.transparent);
        self.compute_transparent();
        let r = f(self);
        self.env = saved;
        self.transparent = saved_transparent;
        r
    }

    /// Under `TEQ_CONTEXT_DUMP`, a line for each field of the list that `where_made` changes at this entry.
    #[cold]
    #[inline(never)]
    fn dump_put_up(&self, door: &'static str, entered: Entered, made: &Standing) {
        let (what, file) = match entered {
            Entered::Def(s) => (self.sym_path(s), self.syms.sym(s).file),
            Entered::Class(c) => (self.name_str(self.syms.class(c).name), self.syms.class(c).file),
            Entered::Alias(a) => (self.name_str(self.syms.alias(a).name), self.syms.alias(a).file),
        };
        let path = &self.source(file).path;
        let asker = self.bodies_in_progress.last().map_or_else(String::new, |&b| self.sym_path(b));
        let mut fields = self.list_differs(made);
        if self.quote.differs(made.parts.quote) {
            fields.push("quote");
        }
        if made.parts.inline_checking == 0 && self.inline.checking > 0 && self.inline.depth == made.parts.inline_depth {
            fields.push("inline.checking");
        }
        if self.gadt_in_use.iter().any(|&(i, _)| i < made.lengths.gadt) {
            fields.push("gadt (a bound in use)");
        }
        for field in fields {
            eprintln!("PUTUP\t{}\t{}\t{}\t{}\t{}", door, field, what, path, asker);
        }
    }

    /// Under `TEQ_CONTEXT_DUMP`, a line for each field, the waiting ones too, in which the state at the door `door`
    /// of the definition `entered` is not its place's: what a definition entered there is typed under that is the
    /// asking code's.
    #[inline]
    pub(super) fn dump_door(&self, door: &'static str, entered: Entered) {
        if dump_on() {
            self.dump_door_now(door, entered);
        }
    }

    /// `standing_of` for the type alias `a`: its class's place, or for a block's alias its block's, found by the
    /// frame that names it.
    fn standing_of_alias(&self, a: AliasId) -> (PlaceKind, Option<&Standing>) {
        match self.syms.alias(a).owner {
            Owner::Local => {
                let frame = self.env.frames.iter().rposition(|f| matches!(f, super::Frame::Locals { aliases, .. } if aliases.iter().any(|&(_, x)| x == a)));
                let block = frame.and_then(|k| self.local_blocks.iter().rev().find(|b| b.frame_depth == k + 1));
                (PlaceKind::Block, block.and_then(|b| b.standing.as_deref()))
            }
            Owner::Class(c) => self.standing_of_class(c),
            Owner::Package(_) => (PlaceKind::Top, Some(nothing_standing())),
        }
    }

    #[cold]
    #[inline(never)]
    fn dump_door_now(&self, door: &'static str, entered: Entered) {
        let (kind, made) = match entered {
            Entered::Def(s) => self.standing_of(s),
            Entered::Class(c) => self.standing_of_class(c),
            Entered::Alias(a) => self.standing_of_alias(a),
        };
        let made = made.copied();
        let kind = match kind {
            PlaceKind::Top => "top",
            PlaceKind::BlockClass => "block-class",
            PlaceKind::Block => "block",
        };
        let (what, file, at) = match entered {
            Entered::Def(s) => (self.sym_path(s), self.syms.sym(s).file, self.syms.sym(s).span.start),
            Entered::Class(c) => (self.name_str(self.syms.class(c).name), self.syms.class(c).file, self.syms.class(c).span.start),
            Entered::Alias(a) => (self.name_str(self.syms.alias(a).name), self.syms.alias(a).file, 0),
        };
        let path = &self.source(file).path;
        let whose = if self.program_file(file) { "program" } else { "other" };
        let asker = self.bodies_in_progress.last().map_or_else(String::new, |&b| self.sym_path(b));
        let Some(mut made) = made else {
            // No record: a class of a block entered without one (another worker's), or a local symbol that no
            // frame names (a refinement's member, a body being expanded), told apart.
            let named = match entered {
                Entered::Def(s) => {
                    let name = self.syms.sym(s).name;
                    self.env.frames.iter().any(|f| matches!(f, super::Frame::Locals { names, .. } if names.iter().any(|&(n, x)| n == name && x == s)))
                }
                _ => true,
            };
            eprintln!("CONTEXT\t{}\t{}\t{}\t{}\t{}:{}\t{}\t{}", door, kind, if named { "NORECORD" } else { "NOFRAME" }, what, path, at, asker, whose);
            return;
        };
        // The owners' chain is the place's for a block's own definition alone: a member's readers take it from
        // the member down.
        if kind != "block" {
            made.parts.owners = self.sites.owners.len() as u32;
        }
        for field in self.standing_differs(&made).into_iter().chain(self.stacks_changed_below(&made)) {
            eprintln!("CONTEXT\t{}\t{}\t{}\t{}\t{}:{}\t{}\t{}", door, kind, field, what, path, at, asker, whose);
        }
    }
}

/// What a place outside every block holds: nothing. One record, read where it is.
#[inline]
fn nothing_standing() -> &'static Standing {
    static NOTHING: std::sync::OnceLock<Standing> = std::sync::OnceLock::new();
    NOTHING.get_or_init(Standing::default)
}

/// Whether the doors' dump is asked for (`TEQ_CONTEXT_DUMP`), read once.
#[inline]
pub(super) fn dump_on() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("TEQ_CONTEXT_DUMP").is_some())
}
