//! The merge's validator: a read of what the merged program keeps that takes no part in the merge.
//!
//! The walk renumbers the records it has a line for, and the check replays the walk
//! (`Worker::check_merge`): a field the walk never names is invisible to both, and a worker's id left
//! in it is read as another record's by whatever asks next. This reader names such fields one by one,
//! each with what the merge owes it: its ids renumbered with the program, its entries of a worker's
//! ids dropped at the boundary, work that does not stand across the boundary, or no id at all. It runs
//! where the check runs, over the merged worker, and changes nothing: an id of a worker's layer it
//! meets fails the check, which names the field.
//!
//! The records it takes apart are matched whole, every field named, and its matches over an enum
//! that can hold an id have no wildcard arm: a field or a variant added to one of them is given its
//! line here before the compiler builds. That is an alarm for what is added, not the proof that a
//! line is right, which the read is.

use super::Remap;
use crate::symbols::Forward;
use crate::tir::capture::BlockStmt;
use crate::typer::resolve::TypeRef;
use crate::tir::TExprId;
use crate::typer::{FunId, ImportTarget, ResolvedImport, Worker};
use crate::types::{AliasId, ClassId, PkgId, SymId, TParamId, TypeId};

/// The validator's reader: the check's own leaf tests (an id of a worker's range, a type that is an
/// overlay's or names a worker's id), and the fields that failed them.
pub(crate) struct Left<'r> {
    check: &'r Remap,
    pub(super) fields: Vec<&'static str>,
}

impl<'r> Left<'r> {
    pub(super) fn new(check: &'r Remap) -> Left<'r> {
        Left { check, fields: Vec::new() }
    }

    /// What the check has refused so far: the ids of a worker's range and the types.
    fn refused(&self) -> u64 {
        let ids: u64 = self.check.ids.all().iter().map(|(_, t)| t.refused() as u64).sum();
        let types = self.check.checking.as_ref().map_or(0, |v| {
            let v = v.borrow();
            v.overlay_types + v.local_types
        });
        ids + types
    }

    /// One field read: `read` hands its ids to the check, and the field is named among the failures
    /// when the check refused one of them.
    pub(crate) fn field(&mut self, name: &'static str, read: impl FnOnce(&Left<'r>)) {
        let before = self.refused();
        read(self);
        if self.refused() != before {
            self.fields.push(name);
        }
    }

    /// A field that holds work under way or one round's state, which does not stand across the
    /// boundary: named among the failures unless it is empty there.
    pub(crate) fn empty(&mut self, name: &'static str, empty: bool) {
        if !empty {
            self.fields.push(name);
        }
    }

    pub(crate) fn sym(&self, s: SymId) {
        self.check.sym(s);
    }

    pub(crate) fn class(&self, c: ClassId) {
        self.check.class(c);
    }

    pub(crate) fn tparam(&self, p: TParamId) {
        self.check.tparam(p);
    }

    pub(crate) fn alias(&self, a: AliasId) {
        self.check.alias(a);
    }

    pub(crate) fn pkg(&self, p: PkgId) {
        self.check.pkg(p);
    }

    pub(crate) fn expr(&self, e: TExprId) {
        self.check.expr(e);
    }

    pub(crate) fn fun(&self, f: FunId) {
        self.check.fun(f);
    }

    pub(crate) fn ty(&self, t: TypeId) {
        self.check.ty(t);
    }

    /// An import as a record keeps it resolved: what it opens and the bound its type was given. Its
    /// hidden names are a list of the program's, the walk's.
    pub(crate) fn import(&self, i: &ResolvedImport) {
        let ResolvedImport { name: _, target, hidden: _, bound, depth: _, stmt: _, unimports_predef: _, sel: _ } = i;
        match *target {
            ImportTarget::PkgMember(p, _) | ImportTarget::PkgAll(p) | ImportTarget::PkgGivens(p) => self.pkg(p),
            ImportTarget::ClassMember(c, _) | ImportTarget::ClassAll(c) | ImportTarget::ClassGivens(c) => self.class(c),
            // A value import is an index of the imports every worker shares.
            ImportTarget::ValueAll(_) | ImportTarget::ValueMember(_, _) | ImportTarget::ValueGivens(_) => {}
            ImportTarget::UnimportPredef | ImportTarget::Unresolved => {}
        }
        if let Some(t) = *bound {
            self.ty(t);
        }
    }

    fn type_ref(&self, r: &TypeRef) {
        match *r {
            TypeRef::Class(c) | TypeRef::Member(c, _) => self.class(c),
            TypeRef::Alias(a) => self.alias(a),
            TypeRef::Param(p) => self.tparam(p),
            TypeRef::ValueMember(_, _) => {}
        }
    }
}

impl<'a> Worker<'a> {
    /// The fields of the merged worker that still hold an id of a worker's layer, or work that should
    /// not stand at the boundary, by name; none when the merge left nothing.
    pub(super) fn validate_merge(&self, check: &Remap) -> Vec<&'static str> {
        let mut v = Left::new(check);
        self.validate_program(&mut v);
        self.validate_symbols(&mut v);
        self.tvars.left_ids(&mut v);
        self.inlining.left_ids(&mut v);
        self.unused.left_ids(&mut v);
        self.validate_tables(&mut v);
        v.fields
    }

    fn validate_program(&self, v: &mut Left) {
        let crate::tir::Program {
            local_enum_names,
            class_tag_module,
            aside_types,
            capture,
            // The arenas of records and the registers the merge's walk renumbers, which the check replays; the
            // switches and counters among them hold no id.
            exprs: _, pats: _, strings: _, expr_lists: _, pat_lists: _, sym_lists: _, stmts: _, cases: _, tries: _,
            tests: _, deferred_tests: _, funs: _, classes: _, top_funs: _, top_vals: _, main: _, main_object: _,
            overrides: _, partial_function: _, throwable: _, js_exception: _, js_imports: _, js_exports: _,
            record_types: _, types_only: _, types_aside: _, template_calls: _, get_class_units: _,
            template_syms: _, class_bodies: _, expr_types: _, expr_spans: _, file_tags: _, quotes: _,
            quote_pats: _, expansions: _, block_classes: _, expansion_bits: _, chain_end_bits: _,
            taken_then_bits: _, taken_else_bits: _, widened_bits: _, opaque_bits: _, spread_bits: _,
            ascribed_bits: _, cast_bits: _, setter_assign_bits: _, tuple_index_bits: _, kept_bits: _, leaf_bits: _,
            leaf_tests: _, stored_tests: _, stored_pats: _, shifts: _, build_start: _, runs: _, lifted_funs: _,
            block_class_traits: _, forward: _,
        } = &self.prog;
        // A local enum's name is kept by its class, a class of a block: renumbered with the program.
        v.field("the local enums' names (Program::local_enum_names)", |v| {
            for &(c, _) in local_enum_names.iter() {
                v.class(c);
            }
        });
        // The std's, by where it is written.
        v.field("Program::class_tag_module", |v| {
            if let Some(c) = *class_tag_module {
                v.class(c);
            }
        });
        // Filled where the program keeps its types aside, after the merges.
        v.field("Program::aside_types", |v| {
            for (&e, &t) in aside_types.iter() {
                v.expr(e);
                v.ty(t);
            }
        });
        // A block's statements as a capture keeps them: an import's bound is a type of the body's.
        v.field("a captured block's statements (Capture::block_stmts)", |v| {
            for stmts in capture.iter().flat_map(|c| c.block_stmts.values()) {
                for (_, _, stmt) in stmts.iter() {
                    match stmt {
                        BlockStmt::Import(i) => v.import(i),
                        BlockStmt::InlineDef(s) => v.sym(*s),
                        BlockStmt::TypeAlias(a) => v.alias(*a),
                    }
                }
            }
        });
    }

    fn validate_symbols(&self, v: &mut Left) {
        let crate::symbols::Symbols {
            classes,
            pkgs,
            opaque_holders,
            product_callee_sources,
            product_synthetics,
            product_class_places,
            // The arenas and layered tables the merge's walk renumbers; the products' places hold files and
            // offsets; the cells are a state by record, placed with the record.
            syms: _, tparams: _, aliases: _, overloads: _, dispatch_names: _, js_export_names: _,
            context_bounds: _, binder_annots: _, file_ranks: _, product_places: _, product_sources: _,
            product_files: _, class_cells: _, sym_cells: _, body_cells: _, alias_cells: _, check_cells: _,
        } = &self.syms;
        // A forwarder's record is kept whole where its target does not move: every id of it is read.
        let forward = |v: &Left, fwds: &[(SymId, std::sync::Arc<Forward>)]| {
            for (s, fwd) in fwds.iter() {
                let Forward { path: _, target, clause: _, alt: _, path_method } = &**fwd;
                v.sym(*s);
                v.sym(*target);
                if let Some(pm) = *path_method {
                    v.sym(pm);
                }
            }
        };
        v.field("a class's value forwarders (ClassInfo::forwarders)", |v| {
            for c in classes.iter() {
                forward(v, &c.forwarders);
            }
        });
        v.field("a package's value forwarders (PkgInfo::forwarders)", |v| {
            for p in pkgs.iter() {
                forward(v, &p.forwarders);
            }
        });
        // Written before the fork or by the products' reader, by where they are written.
        v.field("Symbols::opaque_holders", |v| opaque_holders.values().for_each(|&s| v.sym(s)));
        v.field("Symbols::product_callee_sources", |v| product_callee_sources.keys().for_each(|&s| v.sym(s)));
        v.field("Symbols::product_synthetics", |v| product_synthetics.keys().for_each(|&s| v.sym(s)));
        v.field("Symbols::product_class_places", |v| product_class_places.keys().for_each(|&c| v.class(c)));
    }

    /// The main worker's own tables that no worker hands over and the merge has no line for. A table
    /// that can be made again holds no entry of a worker's id past the boundary; unfinished work does not
    /// stand there at all.
    fn validate_tables(&self, v: &mut Left) {
        v.field("Worker::inherited_named", |v| {
            let inherited = &self.inherited_named;
            for (&(c, _), (_, found)) in inherited.types.iter() {
                v.class(c);
                if let Some(r) = found {
                    v.type_ref(r);
                }
            }
            for (&(c, _), &(_, found)) in inherited.nested.iter() {
                v.class(c);
                if let Some(n) = found {
                    v.class(n);
                }
            }
            inherited.names.keys().for_each(|&c| v.class(c));
        });
        v.field("Worker::conversion_nonvariant", |v| {
            for (&t, params) in self.conversion_nonvariant.iter() {
                v.ty(t);
                params.iter().for_each(|&p| v.tparam(p));
            }
        });
        v.field("Worker::receiver_types", |v| {
            for (&e, &t) in self.receiver_types.iter() {
                v.expr(e);
                v.ty(t);
            }
        });
        v.field("Worker::import_heads", |v| {
            for head in self.import_heads.values() {
                v.expr(head.te);
                v.ty(head.ty);
                for step in head.steps.iter() {
                    match *step {
                        crate::typer::import_check::PathStep::Object(c) | crate::typer::import_check::PathStep::This(c) => v.class(c),
                        crate::typer::import_check::PathStep::Term(s) => v.sym(s),
                    }
                }
            }
        });
        v.field("Worker::stable_realizable", |v| self.stable_realizable.keys().for_each(|&s| v.sym(s)));
        // A class's forwarders are keyed by its number beside no package; a package file's by the two.
        v.field("Worker::forwarders_completion", |v| {
            for &(p, n) in self.forwarders_completion.keys() {
                if p == u32::MAX {
                    v.class(ClassId(n));
                } else {
                    v.pkg(PkgId(p));
                }
            }
        });
        v.empty("Worker::forwarders_completion (a completion under way)", self.forwarders_completion.values().all(|&made| made));
        // Methods queued for a class that is not made yet: appended when it is, in the same typing.
        v.empty("Worker::pending_held", self.pending_held.is_empty());
        // By where they are written: a class of the std, a jar or the loader, or what the phases around the
        // bodies fill.
        v.field("Worker::arity_classes", |v| self.arity_classes.iter().for_each(|(_, &c)| v.class(c)));
        v.field("Worker::reflective_jar_found", |v| self.reflective_jar_found.iter().flatten().for_each(|&c| v.class(c)));
        v.field("Worker::sites.type_test", |v| self.sites.type_test.iter().for_each(|&c| v.class(c)));
        v.field("the collections' classes (Builtins::collection_set, collection_map)", |v| {
            self.b.collection_set.iter().chain(self.b.collection_map.iter()).for_each(|&c| v.class(c));
        });
        v.field("Worker::inline_accessor_syms", |v| {
            for (&(c, s, _), &a) in self.inline_accessor_syms.iter() {
                v.class(c);
                v.sym(s);
                v.sym(a);
            }
        });
        v.field("Worker::inline_results", |v| {
            for (&s, &t) in self.inline_results.iter() {
                v.sym(s);
                v.ty(t);
            }
        });
        v.field("Worker::mixins_bound", |v| self.mixins_bound.1.keys().for_each(|&c| v.class(c)));
    }
}
