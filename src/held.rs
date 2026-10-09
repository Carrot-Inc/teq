//! What a store holds on the heap, for a session's `stats` (src/watch.rs): the capacity of its
//! arrays and tables and the texts and lists its records own. An estimate by the stores' own
//! shapes, not the allocator's account, which has no names for what it holds: it tells which
//! store a growth is in. A store whose fields are private gives its size by a `held` of its
//! own (the type store, the interner, the emitter's caches, the index).
//!
//! `Program::held` names every field of the program: a field added to it does not compile
//! until its size is counted here. The others count the arrays that make their size.

use crate::ast::Ast;
use crate::intern::FxMap;
use crate::symbols::Symbols;
use crate::tir::Program;
use std::mem::size_of;

pub fn array<T>(v: &Vec<T>) -> usize {
    v.capacity() * size_of::<T>()
}

pub fn table<K, V>(m: &FxMap<K, V>) -> usize {
    // A bucket and its control byte.
    m.capacity() * (size_of::<(K, V)>() + 1)
}

pub fn texts(v: &Vec<String>) -> usize {
    array(v) + v.iter().map(String::capacity).sum::<usize>()
}

/// An array and what each of its records owns.
pub fn owners<T>(v: &Vec<T>, owned: impl Fn(&T) -> usize) -> usize {
    array(v) + v.iter().map(owned).sum::<usize>()
}

impl Program {
    /// The bytes the typed program's stores hold: the records of every build since the last
    /// full one, the ones a retype left dead among them.
    pub fn held(&self) -> usize {
        let Program {
            exprs,
            pats,
            strings,
            expr_lists,
            pat_lists,
            sym_lists,
            stmts,
            cases,
            tries,
            tests,
            deferred_tests,
            funs,
            classes,
            top_funs,
            top_vals,
            main: _,
            main_object: _,
            overrides,
            partial_function: _,
            throwable: _,
            js_exception: _,
            js_imports,
            js_exports,
            record_types: _,
            types_only: _,
            capture,
            types_aside: _,
            aside_types,
            template_calls,
            get_class_units,
            template_syms,
            class_bodies,
            expr_types,
            expr_spans,
            file_tags,
            quotes,
            quote_pats,
            expansions,
            expansion_bits,
            leaf_bits,
            leaf_tests,
            stored_tests,
            stored_pats,
            chain_end_bits,
            taken_then_bits,
            taken_else_bits,
            widened_bits,
            opaque_bits,
            spread_bits,
            shifts: _,
        } = self;
        exprs.held()
            + pats.held()
            + strings.held()
            + strings.own().iter().map(String::capacity).sum::<usize>()
            + expr_lists.held()
            + pat_lists.held()
            + sym_lists.held()
            + stmts.held()
            + cases.held()
            + tries.held()
            + tests.held()
            + table(deferred_tests)
            + funs.held()
            + funs.own().iter().map(|f| array(&f.params) + array(&f.defaults)).sum::<usize>()
            + classes.held()
            + classes
                .own()
                .iter()
                .map(|c| array(&c.ctor_params) + array(&c.ctor_defaults) + array(&c.init) + array(&c.methods) + array(&c.ctors) + array(&c.forwarders) + array(&c.super_accessors) + array(&c.bridges) + array(&c.deferred_givens))
                .sum::<usize>()
            + top_funs.held()
            + top_vals.held()
            + overrides.held()
            + overrides.local.values().map(array).sum::<usize>()
            + array(js_imports)
            + array(js_exports)
            + array(template_calls)
            + array(get_class_units)
            + template_syms.held()
            + class_bodies.held()
            + expr_types.held()
            + expr_spans.held()
            + file_tags.held()
            + quotes.held()
            + quotes.own().iter().map(|q| array(&q.holes) + array(&q.types) + array(&q.binders)).sum::<usize>()
            + quote_pats.held()
            + quote_pats.own().iter().map(|q| array(&q.holes) + array(&q.type_params) + array(&q.types)).sum::<usize>()
            + table(expansions)
            + table(aside_types)
            + expansion_bits.held()
            + leaf_bits.held()
            + table(leaf_tests)
            + table(stored_tests)
            + table(stored_pats)
            + chain_end_bits.held()
            + taken_then_bits.held()
            + taken_else_bits.held()
            + widened_bits.held()
            + opaque_bits.held()
            + spread_bits.held()
            + capture.as_ref().map_or(0, |c| c.held())
    }

    /// The records of the stores that a retype appends to.
    pub fn records(&self) -> usize {
        self.exprs.len() + self.pats.len() + self.stmts.len() + self.cases.len() + self.funs.len() + self.classes.len()
    }
}

impl Symbols {
    /// The symbol table's arrays, and the lists and tables of its classes and packages.
    pub fn held(&self) -> usize {
        self.classes.held()
            + self
                .classes
                .own()
                .iter()
                .map(|c| {
                    array(&c.tparams)
                        + array(&c.parents)
                        + array(&c.subclasses)
                        + table(&c.members)
                        + array(&c.member_order)
                        + table(&c.nested)
                        + table(&c.type_aliases)
                        + array(&c.children)
                        + array(&c.ctor)
                        + owners(&c.ctor_syms, array)
                        + array(&c.base_types)
                        + array(&c.extensions)
                        + array(&c.givens)
                })
                .sum::<usize>()
            + self.syms.held()
            + self.tparams.held()
            + self.aliases.held()
            + self.pkgs.held()
            + self.pkgs.own().iter().map(|p| table(&p.entries) + array(&p.givens) + array(&p.export_files)).sum::<usize>()
            + self.overloads.held()
            + self.overloads.own().iter().map(|(_, alternatives)| array(alternatives)).sum::<usize>()
            + self.dispatch_names.held()
            + self.class_cells.held()
            + self.sym_cells.held()
            + self.body_cells.held()
            + self.alias_cells.held()
            + self.check_cells.held()
    }
}

impl Ast {
    /// The arrays of a file's syntax tree that make its size.
    pub fn held(&self) -> usize {
        array(&self.exprs)
            + array(&self.expr_spans)
            + array(&self.pats)
            + array(&self.pat_spans)
            + array(&self.tys)
            + array(&self.ty_spans)
            + owners(&self.defs, |d| array(&d.annots))
            + texts(&self.strings)
            + array(&self.expr_lists)
            + array(&self.pat_lists)
            + array(&self.ty_lists)
            + array(&self.def_lists)
            + array(&self.str_lists)
            + array(&self.str_list_spans)
            + array(&self.name_lists)
            + array(&self.stmts)
            + array(&self.cases)
            + array(&self.enumerators)
            + array(&self.tries)
            + array(&self.imports)
            + array(&self.local_imports)
            + array(&self.def_ranges)
            + array(&self.name_spans)
            + array(&self.ty_name_spans)
            + array(&self.import_ranges)
            + array(&self.package_ranges)
            + array(&self.recoveries)
            + array(&self.inline_annots)
            + array(&self.cut_args)
            + array(&self.broken_cases)
    }
}
