//! `export` clauses. They are resolved lazily, once per class, into a table of aliases: a
//! reference to an exported name is typed as a reference to the original symbol, so an export
//! generates no code. The table of an object also lists what the object inherits from its
//! traits, since that is just as reachable through `import Obj.*`.

use super::resolve::{TermRef, TypeRef};
use super::{Env, Frame, ImportTarget, Worker};
use crate::ast::{mods, DefKind, Import, ImportSel, TyExpr, TyExprId};
use crate::intern::{FxMap, Name};
use crate::names;
use crate::source::{FileId, Span};
use crate::symbols::*;
use crate::tir::{TExpr, TExprId};
use crate::types::*;
use std::sync::Arc;

#[derive(Default, Clone)]
pub struct ExportScope {
    pub terms: FxMap<Name, TermRef>,
    pub types: FxMap<Name, TypeRef>,
    pub extensions: FxMap<Name, Vec<SymId>>,
    pub givens: Vec<SymId>,
    /// The object through which an extension method or given defined in a trait is reached.
    pub via: FxMap<SymId, ClassId>,
    /// The object whose table this is: its inherited members, entered first, are not exported
    /// again by a wildcard.
    owner: Option<ClassId>,
    /// Names that two exports brought in for different entities, reported by the clause that
    /// added them.
    conflicts: Vec<Name>,
}

pub enum ExportSlot {
    Unknown,
    InProgress,
    Empty,
    Ready(Arc<ExportScope>),
    /// A table built while the classes listed were incomplete (`Build::awaited`): it answers
    /// the lookups made until one of them completes, and is never read as final.
    Provisional(Option<Arc<ExportScope>>, Vec<ClassId>),
}

impl ExportSlot {
    /// The slot as another worker starts with it: a table under construction is unknown to it.
    pub fn copy(&self) -> ExportSlot {
        match self {
            ExportSlot::Unknown | ExportSlot::InProgress => ExportSlot::Unknown,
            ExportSlot::Empty => ExportSlot::Empty,
            ExportSlot::Ready(s) => ExportSlot::Ready(s.clone()),
            // A table built while classes it awaited were incomplete answers the worker that built
            // it until one completes, and is never published as it stands: another worker builds
            // its own.
            ExportSlot::Provisional(..) => ExportSlot::Unknown,
        }
    }
}

/// The export tables of one kind of owner (a class's, a package's), as a worker holds them
/// (the attachment): the first worker's are its own; an attached
/// worker reads the slots the fork left as another worker starts with them (`ExportSlot::copy`),
/// one copy behind an `Arc` every attached worker shares, under the slots it writes, which are
/// its own.
pub struct ExportTables<K> {
    own: FxMap<K, ExportSlot>,
    /// The first worker's: its slots as the fork left them, seen by another worker.
    snapshot: Option<Arc<FxMap<K, ExportSlot>>>,
    /// An attached worker's: the fork's slots.
    prefix: Option<Arc<FxMap<K, ExportSlot>>>,
}

impl<K: Copy + Eq + std::hash::Hash> ExportTables<K> {
    pub fn new() -> ExportTables<K> {
        ExportTables { own: FxMap::default(), snapshot: None, prefix: None }
    }

    #[inline]
    pub fn get(&self, k: &K) -> Option<&ExportSlot> {
        match self.own.get(k) {
            Some(s) => Some(s),
            None => self.prefix.as_ref()?.get(k),
        }
    }

    /// The slot of `k`, this worker's own to write: the fork's, or an unknown one.
    pub fn slot(&mut self, k: K) -> &mut ExportSlot {
        let prefix = &self.prefix;
        self.own.entry(k).or_insert_with(|| prefix.as_ref().and_then(|p| p.get(&k)).map_or(ExportSlot::Unknown, ExportSlot::copy))
    }

    pub fn insert(&mut self, k: K, v: ExportSlot) {
        self.own.insert(k, v);
    }

    /// Forgets the slot of `k`: a slot the fork left reads unknown from here, as an absent one does.
    pub fn remove(&mut self, k: &K) {
        if self.prefix.as_ref().is_some_and(|p| p.contains_key(k)) {
            self.own.insert(*k, ExportSlot::Unknown);
        } else {
            self.own.remove(k);
        }
    }

    /// The slots as another worker starts with them, kept for the attachments.
    pub fn fork(&mut self) {
        self.snapshot = Some(Arc::new(self.own.iter().map(|(k, v)| (*k, v.copy())).collect()));
    }

    /// Another worker's tables: the fork's slots shared, none of its own yet.
    pub fn attach(&self) -> ExportTables<K> {
        match &self.snapshot {
            Some(s) => ExportTables { own: FxMap::default(), snapshot: None, prefix: Some(s.clone()) },
            None => ExportTables { own: self.own.iter().map(|(k, v)| (*k, v.copy())).collect(), snapshot: None, prefix: None },
        }
    }

    /// The slots this worker wrote, for the merge (`Worker::absorb`).
    pub fn take_own(&mut self) -> FxMap<K, ExportSlot> {
        std::mem::take(&mut self.own)
    }

    /// The fork's snapshot given up: the first worker's tables alone again, after the join.
    pub fn merge_own(&mut self) {
        self.snapshot = None;
    }

    /// The first worker's whole tables, for the merge's renumbering.
    pub fn own_mut(&mut self) -> &mut FxMap<K, ExportSlot> {
        debug_assert!(self.prefix.is_none(), "an attached worker's export tables renumbered");
        &mut self.own
    }

    /// The slots this worker reads, its own and the fork's.
    pub fn len(&self) -> usize {
        self.own.len() + self.prefix.as_ref().map_or(0, |p| p.len())
    }
}

#[derive(Clone, Copy)]
pub enum TraitMemberSite {
    /// `this`, along with the subclass of the trait that `this` is an instance of.
    This(Option<ClassId>),
    Module(ClassId),
    /// A stable value whose members an import brings (`import dsl.*`).
    Value(super::ValueImport),
}

#[derive(Clone, Copy, PartialEq)]
enum Merge {
    /// Everything: a class inherits the exports of its parents as they are.
    Inherit,
    /// `export src.*`: a wildcard leaves givens out.
    Wildcard,
    /// `export src.given`
    Givens,
}

impl ExportScope {
    /// The table under the merge's ids (`merge.rs`): its references' symbols, classes, aliases,
    /// type parameters and packages. A stable value's import (`ValueImport`) indexes the shared
    /// table of them, which keeps its place.
    pub fn remap(&mut self, sym: &dyn Fn(SymId) -> SymId, class: &dyn Fn(ClassId) -> ClassId, alias: &dyn Fn(AliasId) -> AliasId, tparam: &dyn Fn(TParamId) -> TParamId, pkg: &dyn Fn(PkgId) -> PkgId) {
        for t in self.terms.values_mut() {
            *t = match *t {
                TermRef::Local(s) => TermRef::Local(sym(s)),
                TermRef::This(c, s) => TermRef::This(class(c), sym(s)),
                TermRef::ModuleMember(c, s) => TermRef::ModuleMember(class(c), sym(s)),
                TermRef::Global(s) => TermRef::Global(sym(s)),
                TermRef::Class(c) => TermRef::Class(class(c)),
                TermRef::Package(p) => TermRef::Package(pkg(p)),
                TermRef::ValueMember(v, s) => TermRef::ValueMember(v, sym(s)),
                TermRef::SelfAlias(c) => TermRef::SelfAlias(class(c)),
            };
        }
        for t in self.types.values_mut() {
            *t = match *t {
                TypeRef::Class(c) => TypeRef::Class(class(c)),
                TypeRef::Alias(a) => TypeRef::Alias(alias(a)),
                TypeRef::Param(p) => TypeRef::Param(tparam(p)),
                TypeRef::Member(c, n) => TypeRef::Member(class(c), n),
                other @ TypeRef::ValueMember(..) => other,
            };
        }
        for s in self.extensions.values_mut().flatten().chain(self.givens.iter_mut()) {
            *s = sym(*s);
        }
        self.via = std::mem::take(&mut self.via).into_iter().map(|(s, c)| (sym(s), class(c))).collect();
        self.owner = self.owner.map(class);
    }

    /// Every reference of the table, for the merge's check (`merge.rs`).
    pub fn each_ref(&self, sym: &mut dyn FnMut(SymId), class: &mut dyn FnMut(ClassId), alias: &mut dyn FnMut(AliasId), tparam: &mut dyn FnMut(TParamId), pkg: &mut dyn FnMut(PkgId)) {
        for t in self.terms.values() {
            match *t {
                TermRef::Local(s) | TermRef::Global(s) | TermRef::ValueMember(_, s) => sym(s),
                TermRef::This(c, s) | TermRef::ModuleMember(c, s) => {
                    class(c);
                    sym(s);
                }
                TermRef::Class(c) | TermRef::SelfAlias(c) => class(c),
                TermRef::Package(p) => pkg(p),
            }
        }
        for t in self.types.values() {
            match *t {
                TypeRef::Class(c) | TypeRef::Member(c, _) => class(c),
                TypeRef::Alias(a) => alias(a),
                TypeRef::Param(p) => tparam(p),
                TypeRef::ValueMember(..) => {}
            }
        }
        for &s in self.extensions.values().flatten().chain(self.givens.iter()) {
            sym(s);
        }
        for (&s, &c) in &self.via {
            sym(s);
            class(c);
        }
        if let Some(c) = self.owner {
            class(c);
        }
    }

    fn is_empty(&self) -> bool {
        self.terms.is_empty()
            && self.types.is_empty()
            && self.extensions.is_empty()
            && self.givens.is_empty()
    }

    fn add_term(&mut self, name: Name, r: TermRef) {
        match self.terms.get(&name) {
            None => {
                self.terms.insert(name, r);
            }
            Some(&existing) => {
                if !same_term(existing, r) && !self.inherited(existing) {
                    self.conflicts.push(name);
                }
            }
        }
    }

    fn inherited(&self, r: TermRef) -> bool {
        matches!(r, TermRef::ModuleMember(m, _) if Some(m) == self.owner)
    }

    fn add_type(&mut self, name: Name, r: TypeRef) {
        self.types.entry(name).or_insert(r);
    }

    fn add_extension(&mut self, name: Name, s: SymId) {
        let list = self.extensions.entry(name).or_default();
        if !list.contains(&s) {
            list.push(s);
        }
    }

    fn keep_via(&mut self, from: &ExportScope, s: SymId) {
        if let Some(&m) = from.via.get(&s) {
            self.via.entry(s).or_insert(m);
        }
    }

    /// Copies what `from` exports as `name`; returns whether there was anything.
    fn merge_name(&mut self, from: &ExportScope, name: Name, alias: Name, syms: &Symbols) -> bool {
        let mut found = false;
        if let Some(&r) = from.terms.get(&name) {
            found = true;
            self.add_term(alias, r);
            if let Some(g) = r.sym().filter(|&s| syms.is_given(s)) {
                self.givens.push(g);
                self.keep_via(from, g);
            }
        }
        if let Some(&r) = from.types.get(&name) {
            found = true;
            self.add_type(alias, r);
        }
        for &s in from.extensions.get(&name).map_or(&[][..], |list| list) {
            found = true;
            self.add_extension(alias, s);
            self.keep_via(from, s);
        }
        found
    }

    /// What a class inherits from a parent's table, as the program's own classes do.
    pub(crate) fn inherit(&mut self, from: &ExportScope, syms: &Symbols) {
        self.merge(from, Merge::Inherit, &[], syms);
    }

    fn merge(&mut self, from: &ExportScope, mode: Merge, hidden: &[Name], syms: &Symbols) {
        let conflicts = self.conflicts.len();
        self.merge_entries(from, mode, hidden, syms);
        if mode == Merge::Inherit {
            self.conflicts.truncate(conflicts);
        }
    }

    fn merge_entries(&mut self, from: &ExportScope, mode: Merge, hidden: &[Name], syms: &Symbols) {
        if mode == Merge::Givens {
            for &g in &from.givens {
                self.givens.push(g);
                self.keep_via(from, g);
                let name = syms.sym(g).name;
                if let Some(&r) = from.terms.get(&name) {
                    self.add_term(name, r);
                }
            }
            return;
        }
        for (&name, &r) in &from.terms {
            let is_given = r.sym().map_or(false, |s| syms.sym(s).kind == SymKind::Given);
            if hidden.contains(&name) || (mode == Merge::Wildcard && is_given) {
                continue;
            }
            self.add_term(name, r);
        }
        for (&name, &r) in &from.types {
            if !hidden.contains(&name) {
                self.add_type(name, r);
            }
        }
        for (&name, list) in &from.extensions {
            if hidden.contains(&name) {
                continue;
            }
            for &s in list {
                self.add_extension(name, s);
                self.keep_via(from, s);
            }
        }
        if mode == Merge::Inherit {
            for &g in &from.givens {
                self.givens.push(g);
                self.keep_via(from, g);
            }
        }
    }
}

/// The qualifier of an export clause.
#[derive(Clone, Copy)]
pub enum ExportQualifier {
    Object(ClassId),
    Package(PkgId),
}

fn is_private(m: crate::ast::Mods) -> bool {
    m & (mods::PRIVATE | mods::PROTECTED) != 0
}

pub(super) fn same_term(a: TermRef, b: TermRef) -> bool {
    match (a.sym(), b.sym()) {
        (Some(x), Some(y)) => match (a, b) {
            // One member reached through two objects that inherit it is two bindings.
            (TermRef::ModuleMember(m, _), TermRef::ModuleMember(n, _)) => x == y && m == n,
            _ => x == y,
        },
        (None, None) => match (a, b) {
            (TermRef::Class(x), TermRef::Class(y)) => x == y,
            (TermRef::Package(x), TermRef::Package(y)) => x == y,
            _ => false,
        },
        _ => false,
    }
}

const MAX_EXPORT_DEPTH: usize = 64;

/// A table under construction. It is published unless it is `blocked` or `provisional`; either
/// way it answers the lookup at hand, is built again on the next request, and its errors are
/// dropped because the build that is published reports them.
struct Build {
    scope: ExportScope,
    /// The file of the clauses being processed.
    file: FileId,
    errors: Vec<(FileId, Span, String)>,
    /// A lookup made for it met a table that was itself being built. That is no cycle among
    /// exports when a name lookup lies in between: finding the parents of `object Utils extends
    /// Base` may search `import Prelude.*` while `Prelude` exports `Utils.*`. The outermost build
    /// tries the tables in the way on their own and then this one again.
    blocked: bool,
    /// The classes whose parents it read off their syntax because they are not complete and
    /// could not be completed here (`direct_parents`), and those the tables it merged read so.
    /// The table is provisional while any of them is incomplete and is built again once one
    /// completes; the signature phase completes every class of the program before the bodies
    /// read the tables, so it is never published as it stands.
    awaited: Vec<ClassId>,
    /// Clauses whose path starts with a name that was not found, each with whether a table
    /// under construction was in the way. The name may be one that this table exports.
    pending: Vec<(usize, bool)>,
}

impl Build {
    fn error(&mut self, span: Span, msg: String) {
        self.errors.push((self.file, span, msg));
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum ExportOwner {
    Class(ClassId),
    /// A package with top-level export clauses.
    Pkg(PkgId),
}

/// A table being built.
pub struct ExportFrame {
    pub owner: ExportOwner,
    /// Asked for by a clause or a parent of another table, not by a name lookup.
    pub as_source: bool,
    /// The clause or parent whose source the table is asking for at the moment.
    pub at: Option<(FileId, Span)>,
    /// The cycles of export clauses this table stands first in, reported with its errors.
    pub cycles: Vec<(FileId, Span, String)>,
}

#[derive(Clone, Copy, PartialEq)]
enum SourceState {
    /// The source's table is published.
    Complete,
    /// The source's table is `Build::blocked`, or is itself being built with a name lookup in
    /// between.
    Blocked,
    /// The source's table is `Build::provisional`.
    Provisional,
    /// The source is being built, and only export clauses and parents lead from it to here.
    Cycle,
}

impl<'a> Worker<'a> {
    /// What `c` exports and, for an object, inherits. `None` when there is nothing, and also
    /// while the table is being built, which keeps the lookups made for export paths finite.
    #[inline]
    pub fn exports_of(&mut self, c: ClassId) -> Option<Arc<ExportScope>> {
        match self.class_exports.get(&c) {
            Some(ExportSlot::Ready(scope)) => Some(scope.clone()),
            Some(ExportSlot::Empty) => None,
            Some(ExportSlot::InProgress) => self.blocked_lookup(ExportOwner::Class(c)),
            Some(ExportSlot::Provisional(scope, awaited)) => {
                if self.still_awaited(awaited) {
                    let scope = scope.clone();
                    return self.provisional_lookup(scope);
                }
                let p = self.phase(super::profile::Phase::Exports);
                let scope = self.build_for_lookup(ExportOwner::Class(c));
                self.phase_end(p);
                scope
            }
            Some(ExportSlot::Unknown) | None => {
                // Another module's class exports what its pickle's clauses and forwarders say.
                if self.loaded.as_ref().map_or(false, |l| l.is_product_class(c)) {
                    // Read from the pickle, which maps its types into the loader's tables.
                    let scope = self.with_loader(|w| w.product_exports(c)).map(Arc::new);
                    *self.export_slot(ExportOwner::Class(c)) = match &scope {
                        Some(s) => ExportSlot::Ready(s.clone()),
                        None => ExportSlot::Empty,
                    };
                    return scope;
                }
                // Most classes have nothing to export or inherit, which is found out without a
                // build and without the profile's row counting one.
                if self.plainly_no_exports(c) {
                    *self.export_slot(ExportOwner::Class(c)) = ExportSlot::Empty;
                    return None;
                }
                let p = self.phase(super::profile::Phase::Exports);
                let scope = self.build_for_lookup(ExportOwner::Class(c));
                self.phase_end(p);
                scope
            }
        }
    }

    /// The names that top-level export clauses add to the package `p`.
    pub fn pkg_exports_of(&mut self, p: PkgId) -> Option<Arc<ExportScope>> {
        if !self.syms.pkg(p).has_exports() {
            return None;
        }
        match self.pkg_exports.get(&p) {
            Some(ExportSlot::Ready(scope)) => Some(scope.clone()),
            Some(ExportSlot::Empty) => None,
            Some(ExportSlot::InProgress) => self.blocked_lookup(ExportOwner::Pkg(p)),
            Some(ExportSlot::Provisional(scope, awaited)) if self.still_awaited(awaited) => {
                let scope = scope.clone();
                self.provisional_lookup(scope)
            }
            _ => {
                let prof = self.phase(super::profile::Phase::Exports);
                let scope = self.build_for_lookup(ExportOwner::Pkg(p));
                self.phase_end(prof);
                scope
            }
        }
    }

    /// Whether a provisional table still stands: none of the classes it awaits has completed.
    fn still_awaited(&self, awaited: &[ClassId]) -> bool {
        awaited.iter().all(|&k| self.syms.class(k).state() != Completion::Done)
    }

    /// A lookup answered by a provisional table counts as one that met a table being built:
    /// what is built over the answer is not kept.
    #[cold]
    fn provisional_lookup(&mut self, scope: Option<Arc<ExportScope>>) -> Option<Arc<ExportScope>> {
        self.export_blocks += 1;
        scope
    }

    /// A lookup in a table that is being built finds nothing. The table whose construction
    /// made the lookup is told through `export_blocks`, unless it looked into itself.
    #[cold]
    fn blocked_lookup(&mut self, owner: ExportOwner) -> Option<Arc<ExportScope>> {
        if self.export_stack.last().map(|f| f.owner) != Some(owner) {
            self.export_blocks += 1;
        }
        None
    }

    fn build_for_lookup(&mut self, owner: ExportOwner) -> Option<Arc<ExportScope>> {
        let (scope, state) = self.build_exports(owner, false);
        if state != SourceState::Complete {
            self.export_blocks += 1;
        }
        scope
    }

    /// Whether the table of `owner` is published, so that what was built over it can be kept.
    pub fn exports_settled(&self, owner: ExportOwner) -> bool {
        match owner {
            ExportOwner::Class(c) => matches!(self.class_exports.get(&c), Some(ExportSlot::Ready(_) | ExportSlot::Empty)),
            ExportOwner::Pkg(p) => {
                !self.syms.pkg(p).has_exports()
                    || matches!(self.pkg_exports.get(&p), Some(ExportSlot::Ready(_) | ExportSlot::Empty))
            }
        }
    }

    fn export_slot(&mut self, owner: ExportOwner) -> &mut ExportSlot {
        match owner {
            ExportOwner::Class(c) => self.class_exports.slot(c),
            ExportOwner::Pkg(p) => self.pkg_exports.slot(p),
        }
    }

    fn slot_of(&self, owner: ExportOwner) -> Option<&ExportSlot> {
        match owner {
            ExportOwner::Class(c) => self.class_exports.get(&c),
            ExportOwner::Pkg(p) => self.pkg_exports.get(&p),
        }
    }

    /// The table of a class or package that the table under construction is built from.
    fn source_exports(&mut self, owner: ExportOwner) -> (Option<Arc<ExportScope>>, SourceState) {
        if matches!(owner, ExportOwner::Pkg(p) if !self.syms.pkg(p).has_exports()) {
            return (None, SourceState::Complete);
        }
        let provisional = match self.export_slot(owner) {
            ExportSlot::Ready(scope) => return (Some(scope.clone()), SourceState::Complete),
            ExportSlot::Empty => return (None, SourceState::Complete),
            ExportSlot::InProgress => {
                let from = self.export_stack.iter().position(|f| f.owner == owner).unwrap_or(0);
                let only_sources = self.export_stack[from + 1..].iter().all(|f| f.as_source);
                if only_sources {
                    self.note_export_cycle(from);
                    return (None, SourceState::Cycle);
                }
                return (None, SourceState::Blocked);
            }
            ExportSlot::Provisional(scope, awaited) => Some((scope.clone(), awaited.clone())),
            ExportSlot::Unknown => None,
        };
        if let Some((scope, awaited)) = provisional {
            if self.still_awaited(&awaited) {
                return (scope, SourceState::Provisional);
            }
        }
        // Another module's class, a parent of the program's own: its pickle's table.
        if let ExportOwner::Class(c) = owner {
            if self.loaded.as_ref().map_or(false, |l| l.is_product_class(c)) {
                return (self.exports_of(c), SourceState::Complete);
            }
        }
        self.build_exports(owner, true)
    }

    /// `source_exports` for the clause or parent at `span` of `file`, which the frame of the table
    /// under construction records as its edge to the source.
    fn source_exports_at(&mut self, owner: ExportOwner, file: FileId, span: Span) -> (Option<Arc<ExportScope>>, SourceState) {
        if let Some(frame) = self.export_stack.last_mut() {
            frame.at = Some((file, span));
        }
        self.source_exports(owner)
    }

    /// The tables from the frame `from` to the top of the stack ask for each other in a ring,
    /// through export clauses and parents alone. The ring is reported once, at the edge of the
    /// member that comes first in the order of the files' identities and then of positions,
    /// whichever traversal found it, so that the report does not depend on the order in which
    /// the tables were demanded; the message names the members the ring passes on the way back.
    fn note_export_cycle(&mut self, from: usize) {
        let members: Vec<usize> = (from..self.export_stack.len()).collect();
        let rank = |t: &Self, i: usize| -> (String, u32) {
            match t.export_stack[i].owner {
                ExportOwner::Class(c) => {
                    let info = t.syms.class(c);
                    (t.source(info.file).key.clone(), info.span.start)
                }
                ExportOwner::Pkg(_) => match t.export_stack[i].at {
                    Some((file, span)) => (t.source(file).key.clone(), span.start),
                    None => (String::new(), 0),
                },
            }
        };
        let Some(&first) = members.iter().min_by_key(|&&i| rank(self, i)) else { return };
        let name_of = |t: &Self, i: usize| match t.export_stack[i].owner {
            ExportOwner::Class(c) => t.name_str(t.syms.class(c).name).to_string(),
            ExportOwner::Pkg(p) => t.name_str(t.syms.pkg(p).name).to_string(),
        };
        let ring: Vec<usize> = members.iter().copied().cycle().skip_while(|&i| i != first).skip(1).take(members.len()).collect();
        let mut msg = format!("cyclic export of {}", name_of(self, ring[0]));
        for &i in &ring[1..] {
            msg.push_str(&format!(", which exports {}", name_of(self, i)));
        }
        let at = match self.export_stack[first].at {
            Some(at) => at,
            None => match self.export_stack[first].owner {
                ExportOwner::Class(c) => (self.syms.class(c).file, self.syms.class(c).span),
                ExportOwner::Pkg(_) => return,
            },
        };
        self.export_stack[first].cycles.push((at.0, at.1, msg));
    }

    fn build_exports(
        &mut self,
        owner: ExportOwner,
        as_source: bool,
    ) -> (Option<Arc<ExportScope>>, SourceState) {
        // The table of a class made in a block is built again with the block; any other once.
        match owner {
            ExportOwner::Class(c) if self.made_in_block(Owner::Class(c)) => self.build_exports_now(owner, as_source),
            _ => self.once(|t| t.build_exports_now(owner, as_source)),
        }
    }

    fn build_exports_now(
        &mut self,
        owner: ExportOwner,
        as_source: bool,
    ) -> (Option<Arc<ExportScope>>, SourceState) {
        let outermost = self.export_stack.is_empty() && !self.export_retrying;
        if !outermost {
            return self.build_table(owner, as_source, false);
        }
        self.export_retry.clear();
        let first = self.build_table(owner, as_source, false);
        // A provisional table waits for a completion higher up the stack, which no retry ends.
        if first.1 != SourceState::Blocked {
            return first;
        }
        // The tables that a lookup into this one left incomplete are built on their own: then
        // this one is what comes out incomplete, and they get the answer they were missing.
        // With them in place this table is built for good.
        self.export_retrying = true;
        for blocked in std::mem::take(&mut self.export_retry) {
            if blocked != owner && matches!(self.export_slot(blocked), ExportSlot::Unknown) {
                self.build_table(blocked, false, false);
            }
        }
        self.export_retrying = false;
        self.build_table(owner, as_source, true)
    }

    /// `last_attempt` keeps a table that is still blocked, which also reports its errors.
    fn build_table(
        &mut self,
        owner: ExportOwner,
        as_source: bool,
        last_attempt: bool,
    ) -> (Option<Arc<ExportScope>>, SourceState) {
        if self.export_stack.len() >= MAX_EXPORT_DEPTH {
            if let ExportOwner::Class(c) = owner {
                let info = self.syms.class(c);
                self.diags.error(info.file, info.span, "exports are nested too deeply");
            }
            return (None, SourceState::Complete);
        }
        *self.export_slot(owner) = ExportSlot::InProgress;
        self.export_stack.push(ExportFrame { owner, as_source, at: None, cycles: Vec::new() });
        if matches!(owner, ExportOwner::Class(c) if self.has_no_exports(c)) {
            self.export_stack.pop();
            *self.export_slot(owner) = ExportSlot::Empty;
            return (None, SourceState::Complete);
        }
        let mut build = Build {
            scope: ExportScope::default(),
            file: FileId(0),
            errors: Vec::new(),
            blocked: false,
            awaited: Vec::new(),
            pending: Vec::new(),
        };
        match owner {
            ExportOwner::Class(c) => self.fill_class_exports(c, &mut build),
            ExportOwner::Pkg(p) => self.fill_pkg_exports(p, &mut build),
        }
        if let Some(frame) = self.export_stack.pop() {
            build.errors.extend(frame.cycles);
        }
        let Build { mut scope, errors, blocked, mut awaited, .. } = build;
        scope.givens.sort();
        scope.givens.dedup();
        let scope = (!scope.is_empty()).then(|| Arc::new(scope));
        if !awaited.is_empty() {
            awaited.sort();
            awaited.dedup();
            *self.export_slot(owner) = ExportSlot::Provisional(scope.clone(), awaited);
            return (scope, SourceState::Provisional);
        }
        if blocked && !last_attempt {
            *self.export_slot(owner) = ExportSlot::Unknown;
            return (scope, SourceState::Blocked);
        }
        *self.export_slot(owner) = match &scope {
            Some(scope) => ExportSlot::Ready(scope.clone()),
            None => ExportSlot::Empty,
        };
        for (file, span, msg) in errors {
            self.diags.error(file, span, msg);
        }
        (scope, SourceState::Complete)
    }

    fn accept_source(&self, src: ExportOwner, state: SourceState, build: &mut Build) {
        match state {
            SourceState::Complete => {}
            SourceState::Blocked => build.blocked = true,
            SourceState::Provisional => {
                if let Some(ExportSlot::Provisional(_, awaited)) = self.slot_of(src) {
                    build.awaited.extend(awaited.iter().copied());
                }
            }
            // Reported by the table that stands first in the ring (`note_export_cycle`).
            SourceState::Cycle => {}
        }
    }

    fn fill_pkg_exports(&mut self, p: PkgId, build: &mut Build) {
        // Another module's export clauses of the package come with its package objects.
        if self.loaded.is_some() {
            self.enter_pkg_objects(p);
        }
        for file in self.syms.pkg(p).export_files.clone() {
            build.file = file;
            let clauses: &'a [Import] = &self.ast(file).top_exports;
            let targets = self.resolve_export_paths(clauses, Env { file, frames: Vec::new(), imports: Vec::new() }, build);
            self.add_named_exports(clauses, &targets, build);
            self.add_wildcard_exports(clauses, &targets, build);
            self.add_pending_exports(clauses, build);
        }
        for obj in self.syms.pkg(p).export_objects.clone() {
            if let Some(scope) = self.exports_of(obj) {
                build.scope.inherit(&scope, &self.syms);
            }
        }
    }

    /// `has_no_exports` where the answer is in the tables already: the class has no export
    /// clause and no parent, or, not being an object, only parents whose tables are known to
    /// be empty. A parent's table not yet built goes to the build, which builds it.
    fn plainly_no_exports(&self, c: ClassId) -> bool {
        let Some(info) = self.syms.class_done(c) else { return false };
        if info.has_exports {
            return false;
        }
        if info.parents.is_empty() {
            return true;
        }
        if info.kind == ClassKind::Object {
            return false;
        }
        info.parents.iter().all(|&parent| match self.types.get(parent) {
            Type::Class(pc, _) => matches!(self.class_exports.get(&pc), Some(ExportSlot::Empty)),
            _ => true,
        })
    }

    /// Most classes export nothing and inherit no exports, which is found out without the
    /// bookkeeping of a build.
    fn has_no_exports(&mut self, c: ClassId) -> bool {
        if self.syms.class(c).state() == Completion::NotStarted && self.may_complete_for_exports(c) {
            self.complete_class(c);
        }
        let Some(info) = self.syms.class_done(c) else { return false };
        if info.has_exports {
            return false;
        }
        if info.parents.is_empty() {
            return true;
        }
        if info.kind == ClassKind::Object {
            return false;
        }
        let (file, span) = (info.file, info.span);
        for i in 0..self.syms.class(c).parents.len() {
            let parent = self.syms.class(c).parents[i];
            if let Type::Class(pc, _) = self.types.get(parent) {
                if !matches!(self.source_exports_at(ExportOwner::Class(pc), file, span), (None, SourceState::Complete)) {
                    return false;
                }
            }
        }
        true
    }

    fn fill_class_exports(&mut self, c: ClassId, build: &mut Build) {
        let (file, def, kind) = {
            let i = self.syms.class(c);
            (i.file, i.def, i.kind)
        };
        build.file = file;
        build.scope.owner = Some(c);
        let clauses: &'a [Import] = match def.map(|d| &self.ast(file).def(d).kind) {
            Some(DefKind::Class(cls)) => &self.ast(file).exports[cls.exports.range()],
            _ => &[],
        };
        let parents = self.direct_parents(c, build);
        if clauses.is_empty() && parents.is_empty() && !build.blocked && build.awaited.is_empty() {
            return;
        }
        let targets = if clauses.is_empty() {
            Vec::new()
        } else {
            let env = self.env_for(file, Owner::Class(c));
            self.resolve_export_paths(clauses, env, build)
        };
        self.add_named_exports(clauses, &targets, build);
        // Inherited members go before wildcards: a wildcard does not export a name that the
        // class already has.
        if kind == ClassKind::Object {
            self.settle_joined_entries(c);
            let mut ancestors = Vec::new();
            self.collect_ancestors(c, &parents, &mut ancestors, build);
            for a in ancestors {
                self.add_inherited_members(c, a, &mut build.scope);
            }
        }
        self.add_wildcard_exports(clauses, &targets, build);
        let span = self.syms.class(c).span;
        for p in parents {
            let (inherited, state) = self.source_exports_at(ExportOwner::Class(p), file, span);
            self.accept_source(ExportOwner::Class(p), state, build);
            if let Some(inherited) = inherited {
                build.scope.merge(&inherited, Merge::Inherit, &[], &self.syms);
            }
        }
        self.add_pending_exports(clauses, build);
    }

    /// A lookup made for the table under construction met a table being built.
    fn leave_blocked(&mut self, build: &mut Build) {
        build.blocked = true;
        if let Some(frame) = self.export_stack.last() {
            self.export_retry.push(frame.owner);
        }
    }

    /// One target per clause. The selectors of `export a.{x, y}` share a path, resolved once.
    fn resolve_export_paths(
        &mut self,
        clauses: &'a [Import],
        env: Env,
        build: &mut Build,
    ) -> Vec<Option<ImportTarget>> {
        self.with_env(env, |t| {
            let mut targets: Vec<Option<ImportTarget>> = Vec::with_capacity(clauses.len());
            for (i, imp) in clauses.iter().enumerate() {
                let same_path = i.checked_sub(1).filter(|&prev| clauses[prev].path == imp.path);
                let target = match same_path {
                    Some(prev) => {
                        if let Some(&(_, blocked)) = build.pending.last().filter(|&&(p, _)| p == prev) {
                            build.pending.push((i, blocked));
                        }
                        targets[prev]
                    }
                    None => {
                        let blocks = t.export_blocks;
                        let head = imp.path.first().and_then(|&first| t.lookup_term(first));
                        if head.is_none() && !imp.path.is_empty() {
                            build.pending.push((i, t.export_blocks != blocks));
                            None
                        } else {
                            match t.resolve_export_path(head, &imp.path) {
                                Ok(target) => Some(target),
                                Err(msg) => {
                                    build.error(imp.span, msg);
                                    None
                                }
                            }
                        }
                    }
                };
                if let (Some(target), true) = (target, t.index.is_some()) {
                    t.index_import(imp, target);
                }
                targets.push(target);
            }
            targets
        })
    }

    /// The clauses whose path starts with a name that only this table knows, as in
    /// `export facade.Icons` followed by `export Icons.Icon`, or with an inherited export.
    fn add_pending_exports(&mut self, clauses: &[Import], build: &mut Build) {
        if build.pending.is_empty() {
            return;
        }
        let mut targets: Vec<Option<ImportTarget>> = vec![None; clauses.len()];
        for (i, blocked) in std::mem::take(&mut build.pending) {
            let imp = &clauses[i];
            if i > 0 && clauses[i - 1].path == imp.path {
                targets[i] = targets[i - 1];
                continue;
            }
            let head = build.scope.terms.get(&imp.path[0]).copied();
            if head.is_none() && blocked {
                self.leave_blocked(build);
            }
            match self.resolve_export_path(head, &imp.path) {
                Ok(target) => targets[i] = Some(target),
                Err(msg) => build.error(imp.span, msg),
            }
        }
        self.add_named_exports(clauses, &targets, build);
        self.add_wildcard_exports(clauses, &targets, build);
    }

    /// `head` is what the first segment refers to. It is looked up like any identifier inside
    /// the exporting template, which covers nested and sibling objects as well as absolute
    /// package paths.
    fn resolve_export_path(&mut self, head: Option<TermRef>, path: &[Name]) -> Result<ImportTarget, String> {
        let Some((&first, rest)) = path.split_first() else {
            return Err("an export needs a qualifier, as in `export obj.member`".to_string());
        };
        let not_found =
            |t: &Self, seg: Name| format!("cannot resolve export: {} not found", t.name_str(seg));
        let mut target = match head {
            Some(TermRef::Package(p)) => ImportTarget::PkgAll(p),
            Some(r) => match r.sym().map(|s| self.syms.sym(s).kind) {
                Some(SymKind::Object(oc)) => ImportTarget::ClassAll(oc),
                _ => {
                    return Err(format!(
                        "cannot resolve export: {} is not an object or a package",
                        self.name_str(first)
                    ))
                }
            },
            None => return Err(not_found(self, first)),
        };
        for &seg in rest {
            let next = match target {
                ImportTarget::PkgAll(p) => self.import_step_pkg(p, seg),
                ImportTarget::ClassAll(c) => self.import_step_class(c, seg),
                _ => None,
            };
            target = next.ok_or_else(|| not_found(self, seg))?;
        }
        Ok(target)
    }

    /// What an export clause of `c` names as its qualifier: an object or a package, `None` for
    /// a path that does not resolve to either.
    pub fn export_qualifier(&mut self, c: ClassId, path: &[Name]) -> Option<ExportQualifier> {
        let file = self.syms.class(c).file;
        let own = self.exports_of(c);
        self.export_qualifier_at(file, Owner::Class(c), own, path)
    }

    /// The qualifier of a top-level export clause of `file` in package `p`.
    pub fn package_export_qualifier(&mut self, file: FileId, p: PkgId, path: &[Name]) -> Option<ExportQualifier> {
        let own = self.pkg_exports_of(p);
        self.export_qualifier_at(file, Owner::Package(p), own, path)
    }

    fn export_qualifier_at(&mut self, file: FileId, owner: Owner, own: Option<Arc<ExportScope>>, path: &[Name]) -> Option<ExportQualifier> {
        let env = self.env_for(file, owner);
        self.with_env(env, |t| {
            let first = *path.first()?;
            let head = t.lookup_term(first).or_else(|| own.as_ref().and_then(|s| s.terms.get(&first).copied()));
            match t.resolve_export_path(head, path) {
                Ok(ImportTarget::ClassAll(o)) => Some(ExportQualifier::Object(o)),
                Ok(ImportTarget::PkgAll(p)) => Some(ExportQualifier::Package(p)),
                _ => None,
            }
        })
    }

    fn add_named_exports(
        &mut self,
        clauses: &[Import],
        targets: &[Option<ImportTarget>],
        build: &mut Build,
    ) {
        for (imp, &target) in clauses.iter().zip(targets) {
            let (ImportSel::Name(name, alias), Some(target)) = (&imp.sel, target) else { continue };
            if *alias == Some(names::WILDCARD) {
                continue;
            }
            let alias = alias.unwrap_or(*name);
            self.check_export_clash(alias, imp.span, build);
            let (found, owner) = match target {
                ImportTarget::ClassAll(src) => {
                    let found = self.export_class_member(src, *name, alias, imp.span, build);
                    (found, self.syms.class(src).name)
                }
                ImportTarget::PkgAll(p) => {
                    let found = self.export_pkg_member(p, *name, alias, imp.span, build);
                    (found, self.syms.pkg(p).name)
                }
                _ => continue,
            };
            if !found {
                let msg = format!(
                    "cannot export {}: it is not a member of {}",
                    self.name_str(*name),
                    self.name_str(owner)
                );
                build.error(imp.span, msg);
            }
            self.report_export_conflicts(imp.span, build);
        }
    }

    /// An export alias is a member of the class, so it cannot share its name with a member the
    /// class defines or inherits concretely; a wildcard leaves such names out instead.
    fn check_export_clash(&mut self, alias: Name, span: Span, build: &mut Build) {
        let Some(c) = build.scope.owner else { return };
        if self.syms.class(c).members.contains_key(&alias) {
            let msg = format!("{} is exported and defined in {}", self.name_str(alias), self.class_description(c));
            build.error(span, msg);
            return;
        }
        let bases: Vec<ClassId> = self.syms.class(c).base_types.iter().skip(1).map(|&(b, _)| b).collect();
        let inherited = bases.iter().find_map(|&b| self.syms.class(b).members.get(&alias).map(|&s| (b, s)));
        let Some((t, s)) = inherited else { return };
        if !self.is_abstract_member(s) {
            let owner = self.class_description(t);
            let msg = format!(
                "cannot export {}: it would override the concrete member {} of {}",
                self.name_str(alias),
                self.name_str(alias),
                owner
            );
            build.error(span, msg);
        }
    }

    fn report_export_conflicts(&mut self, span: Span, build: &mut Build) {
        let mut names: Vec<Name> = std::mem::take(&mut build.scope.conflicts);
        names.sort();
        names.dedup();
        for name in names {
            let msg = format!("{} is exported twice, for two different definitions", self.name_str(name));
            build.error(span, msg);
        }
    }

    fn add_wildcard_exports(
        &mut self,
        clauses: &[Import],
        targets: &[Option<ImportTarget>],
        build: &mut Build,
    ) {
        for (imp, &target) in clauses.iter().zip(targets) {
            let mode = match imp.sel {
                ImportSel::Wildcard => Merge::Wildcard,
                ImportSel::Given => Merge::Givens,
                ImportSel::Name(..) => continue,
            };
            // `export a.{x as _, y as z, *}`: the wildcard covers neither `x` nor `y`. The
            // selectors of one clause are separate entries whose spans start at the shared path,
            // which tells them from the selectors of another clause with the same path.
            let hidden: Vec<Name> = clauses
                .iter()
                .filter(|other| other.span.start == imp.span.start)
                .filter_map(|other| match other.sel {
                    ImportSel::Name(n, _) => Some(n),
                    _ => None,
                })
                .collect();
            match target {
                Some(ImportTarget::ClassAll(src)) => {
                    self.export_class_members(src, mode, &hidden, imp.span, build)
                }
                Some(ImportTarget::PkgAll(p)) => self.export_pkg_members(p, mode, &hidden, imp.span, build),
                _ => {}
            }
            self.report_export_conflicts(imp.span, build);
        }
    }

    fn export_class_member(
        &mut self,
        src: ClassId,
        name: Name,
        alias: Name,
        span: Span,
        build: &mut Build,
    ) -> bool {
        let (inner, state) = self.source_exports_at(ExportOwner::Class(src), build.file, span);
        let term = self.module_term(src, name).or_else(|| self.inherited_module_member(src, name));
        let scope = &mut build.scope;
        let mut found = false;
        if let Some(r) = term {
            let sym = r.sym().map(|s| (s, self.syms.sym(s)));
            if !sym.map_or(false, |(_, info)| is_private(info.mods)) {
                found = true;
                scope.add_term(alias, r);
                if let Some((g, _)) = sym.filter(|(_, info)| info.kind == SymKind::Given || info.mods & mods::IMPLICIT != 0) {
                    scope.givens.push(g);
                    if let Some(inner) = &inner {
                        scope.keep_via(inner, g);
                    }
                }
            }
        }
        if let Some(r) = self.module_type(src, name) {
            found = true;
            scope.add_type(alias, r);
        }
        let mut extensions = Vec::new();
        self.module_extensions(src, name, &mut extensions);
        for s in extensions {
            if !is_private(self.syms.sym(s).mods) {
                found = true;
                scope.add_extension(alias, s);
                if let Some(inner) = &inner {
                    scope.keep_via(inner, s);
                }
            }
        }
        // The name may be one that `src` exports, which only its complete table can tell.
        if !found && state != SourceState::Complete {
            self.accept_source(ExportOwner::Class(src), state, build);
            return true;
        }
        found
    }

    /// A member a jar's object inherits from its parents, which its own table does not list
    /// (`HtmlTags.main` of `object HtmlTags extends HtmlTags`).
    fn inherited_module_member(&mut self, c: ClassId, name: Name) -> Option<TermRef> {
        if self.syms.class(c).kind != ClassKind::Object || !self.in_jar(self.syms.class(c).file) {
            return None;
        }
        let ty = self.types.class(c, &[]);
        let (s, _) = self.find_member(ty, name)?;
        Some(TermRef::ModuleMember(c, s))
    }

    fn export_class_members(
        &mut self,
        src: ClassId,
        mode: Merge,
        hidden: &[Name],
        span: Span,
        build: &mut Build,
    ) {
        let (inner, state) = self.source_exports_at(ExportOwner::Class(src), build.file, span);
        self.accept_source(ExportOwner::Class(src), state, build);
        // An object's wildcard exports what it inherits as well (`export cats.syntax.all.*`
        // for the syntax traits an object of a jar mixes in), reached through the object.
        let mut sources = vec![src];
        if self.syms.class(src).kind == ClassKind::Object {
            let parents = self.direct_parents(src, build);
            let mut ancestors = Vec::new();
            self.collect_ancestors(src, &parents, &mut ancestors, build);
            sources.extend(ancestors);
        }
        let scope = &mut build.scope;
        for (i, &from) in sources.iter().enumerate() {
            self.export_members_through(src, from, i > 0, mode, hidden, scope);
        }
        if let Some(inner) = inner {
            scope.merge(&inner, mode, hidden, &self.syms);
        }
    }

    /// The members of `from` that a wildcard export of `src` brings, `from` being `src` or
    /// (`inherited`) one of its ancestors: `*` takes the terms, an implicit one among the
    /// givens too, the types and the extensions; `given` takes the givens.
    fn export_members_through(&mut self, src: ClassId, from: ClassId, inherited: bool, mode: Merge, hidden: &[Name], scope: &mut ExportScope) {
        if self.may_complete_for_exports(from) {
            self.complete_class(from);
        }
        let info = self.syms.class(from);
        if mode == Merge::Givens {
            for &g in &info.givens {
                let sym = self.syms.sym(g);
                if !is_private(sym.mods) {
                    scope.givens.push(g);
                    scope.add_term(sym.name, TermRef::ModuleMember(src, g));
                    if inherited {
                        scope.via.entry(g).or_insert(src);
                    }
                }
            }
        } else {
            let own = scope.owner.map(|c| &self.syms.class(c).info.members);
            let src_members = &self.syms.class(src).members;
            for &s in &info.member_order {
                let sym = self.syms.sym(s);
                let defined = own.map_or(false, |m| m.contains_key(&sym.name));
                // What `src` defines under the name stands for what it inherits under it.
                let overridden = inherited && src_members.contains_key(&sym.name);
                if !is_private(sym.mods) && sym.kind != SymKind::Given && !hidden.contains(&sym.name) && !defined && !overridden {
                    // An overloaded name is exported as a whole.
                    let entry = if sym.alternative { info.members[&sym.name] } else { s };
                    scope.add_term(sym.name, TermRef::ModuleMember(src, entry));
                    if sym.mods & mods::IMPLICIT != 0 {
                        scope.givens.push(s);
                        if inherited {
                            scope.via.entry(s).or_insert(src);
                        }
                    }
                }
            }
            for (&name, &nested) in &info.nested {
                if !is_private(self.syms.class(nested).mods) && !hidden.contains(&name) {
                    scope.add_type(name, TypeRef::Class(nested));
                    if !info.members.contains_key(&name) {
                        scope.add_term(name, TermRef::Class(nested));
                    }
                }
            }
            for (&name, &alias) in &info.type_aliases {
                if !hidden.contains(&name) {
                    scope.add_type(name, TypeRef::Alias(alias));
                }
            }
            for &s in &info.extensions {
                let sym = self.syms.sym(s);
                if !is_private(sym.mods) && !hidden.contains(&sym.name) {
                    scope.add_extension(sym.name, s);
                    if inherited {
                        scope.via.entry(s).or_insert(src);
                    }
                }
            }
        }
    }

    fn export_pkg_member(
        &mut self,
        p: PkgId,
        name: Name,
        alias: Name,
        span: Span,
        build: &mut Build,
    ) -> bool {
        // The std file defining the name enters first; a jar's package enters its members as
        // they are named, so this one is named first too.
        self.demand_std(p, name, crate::stdindex::TYPE | crate::stdindex::TERM);
        if self.sees_classpath() {
            let _ = self.pkg_term(p, name);
            let _ = self.pkg_type(p, name);
        }
        let found = match self.syms.pkg(p).entries.get(&name) {
            Some(entry) => self.export_pkg_entry(entry, alias, true, &mut build.scope),
            None => false,
        };
        if found {
            return true;
        }
        let (inner, state) = self.source_exports_at(ExportOwner::Pkg(p), build.file, span);
        if inner.map_or(false, |inner| build.scope.merge_name(&inner, name, alias, &self.syms)) {
            return true;
        }
        if state != SourceState::Complete {
            self.accept_source(ExportOwner::Pkg(p), state, build);
            return true;
        }
        // What the package object inherits from its parents is a member of the package too
        // (`package object react extends ReactEventTypes`'s `ReactEvent`).
        match self.syms.pkg(p).package_object {
            Some(obj) => self.export_class_member(obj, name, alias, span, build),
            None => false,
        }
    }

    fn export_pkg_members(
        &mut self,
        p: PkgId,
        mode: Merge,
        hidden: &[Name],
        span: Span,
        build: &mut Build,
    ) {
        let (inner, state) = self.source_exports_at(ExportOwner::Pkg(p), build.file, span);
        self.accept_source(ExportOwner::Pkg(p), state, build);
        let scope = &mut build.scope;
        let pkg = self.syms.pkg(p);
        if mode == Merge::Givens {
            for &g in &pkg.givens {
                let sym = self.syms.sym(g);
                if !is_private(sym.mods) {
                    scope.givens.push(g);
                    scope.add_term(sym.name, TermRef::Global(g));
                }
            }
        } else {
            for (&name, entry) in &pkg.entries {
                if !hidden.contains(&name) {
                    self.export_pkg_entry(entry, name, false, scope);
                }
            }
        }
        if let Some(inner) = inner {
            scope.merge(&inner, mode, hidden, &self.syms);
        }
    }

    /// Returns whether the entry had anything to export. Givens are only exported by name.
    fn export_pkg_entry(&self, entry: &PkgEntry, alias: Name, named: bool, scope: &mut ExportScope) -> bool {
        let mut found = false;
        let term = entry.term.filter(|&s| !is_private(self.syms.sym(s).mods));
        if let Some(s) = term {
            let is_given = self.syms.is_scala3_given(s);
            if named || !is_given {
                found = true;
                scope.add_term(alias, TermRef::Global(s));
                if self.syms.is_given(s) {
                    scope.givens.push(s);
                }
            }
        }
        if let Some(c) = entry.class.filter(|&c| !is_private(self.syms.class(c).mods)) {
            found = true;
            scope.add_type(alias, TypeRef::Class(c));
            if entry.term.is_none() {
                scope.add_term(alias, TermRef::Class(c));
            }
        }
        if let Some(a) = entry.alias {
            found = true;
            scope.add_type(alias, TypeRef::Alias(a));
        }
        for &s in &entry.extensions {
            if !is_private(self.syms.sym(s).mods) {
                found = true;
                scope.add_extension(alias, s);
            }
        }
        found
    }

    /// The entries of `c` that stand for a name two unrelated ancestors define, settled: the
    /// object's table then holds the alternatives of both, or the one method they are. An
    /// entry that is a method of `c` is settled when it is looked up, which may be under way.
    fn settle_joined_entries(&mut self, c: ClassId) {
        let pending: Vec<SymId> = self
            .syms
            .class(c)
            .members
            .values()
            .copied()
            .filter(|&s| self.syms.sym(s).merge_pending && self.syms.alternatives(s).map_or(false, |a| a.is_empty()))
            .collect();
        for entry in pending {
            self.merge_inherited(c, entry);
        }
    }

    /// What the object `c` gets from its ancestor `a`, reached through `c` when it is used.
    fn add_inherited_members(&mut self, c: ClassId, a: ClassId, scope: &mut ExportScope) {
        let info = self.syms.class(a);
        scope.terms.reserve(info.member_order.len());
        let own = &self.syms.class(c).members;
        for &s in &info.member_order {
            let sym = self.syms.sym(s);
            // What `c` defines under the name stands for the inherited members as well: it
            // overrides them or is overloaded with them.
            if !is_private(sym.mods) && !own.contains_key(&sym.name) {
                let entry = if sym.alternative { info.members[&sym.name] } else { s };
                scope.add_term(sym.name, TermRef::ModuleMember(c, entry));
            }
        }
        for (&name, &entry) in own {
            if self.syms.alternatives(entry).is_some() && !scope.terms.contains_key(&name) {
                scope.add_term(name, TermRef::ModuleMember(c, entry));
            }
        }
        for &s in &info.extensions {
            let sym = self.syms.sym(s);
            if !is_private(sym.mods) {
                scope.add_extension(sym.name, s);
                scope.via.entry(s).or_insert(c);
            }
        }
        for &g in &info.givens {
            if !is_private(self.syms.sym(g).mods) {
                scope.givens.push(g);
                scope.via.entry(g).or_insert(c);
            }
        }
    }

    fn collect_ancestors(&mut self, c: ClassId, parents: &[ClassId], out: &mut Vec<ClassId>, build: &mut Build) {
        for &p in parents {
            if p != c && !out.contains(&p) {
                out.push(p);
                let grandparents = self.direct_parents(p, build);
                self.collect_ancestors(c, &grandparents, out, build);
            }
        }
    }

    /// Whether a table under construction may complete `c` to read its parents and members. A
    /// class read from a jar always: its completion looks nothing up by name, and its parents
    /// and members enter with it. A class of the program only while no completion of a class
    /// is under way on this worker's stack: its completion may look through an import into the
    /// table of an object that extends the class being completed, which scalac accepts
    /// (`import O.*` above `class Y extends Z`, with `object O extends X` and `class X extends
    /// Y` elsewhere), and completing `O` there would report a cycle that is not one.
    fn may_complete_for_exports(&self, c: ClassId) -> bool {
        self.syms.class(c).def.is_none() || self.completing == 0
    }

    /// The classes `c` extends, completing `c` where `may_complete_for_exports` allows. A class
    /// it may not complete has its parents read off the syntax, which leaves the table
    /// provisional: what those parents inherit is complete only once they are.
    fn direct_parents(&mut self, c: ClassId, build: &mut Build) -> Vec<ClassId> {
        if self.syms.class(c).state() == Completion::NotStarted && self.may_complete_for_exports(c) {
            self.complete_class(c);
        }
        if let Some(info) = self.syms.class_done(c) {
            return info
                .parents
                .iter()
                .filter_map(|&t| match self.types.get(t) {
                    Type::Class(pc, _) => Some(pc),
                    _ => None,
                })
                .collect();
        }
        let info = self.syms.class(c);
        let (file, owner, span) = (info.file, info.owner, info.span);
        let Some(def) = info.def else {
            // A class read from a jar whose completion is under way: its parents are being set.
            build.awaited.push(c);
            return Vec::new();
        };
        let parent_tys: Vec<TyExprId> = match &self.ast(file).def(def).kind {
            DefKind::Class(cls) => cls.parents.iter().map(|p| p.ty).collect(),
            DefKind::Given(g) => vec![g.ty],
            _ => Vec::new(),
        };
        if parent_tys.is_empty() {
            return Vec::new();
        }
        build.awaited.push(c);
        let env = self.env_at(file, owner, span.start);
        self.with_env(env, |t| {
            let mut parents = Vec::with_capacity(parent_tys.len());
            for &ty in &parent_tys {
                let blocks = t.export_blocks;
                match t.parent_class(ty) {
                    Some(p) => parents.push(p),
                    None => {
                        if t.export_blocks != blocks {
                            t.leave_blocked(build);
                        }
                    }
                }
            }
            parents
        })
    }

    fn parent_class(&mut self, ty: TyExprId) -> Option<ClassId> {
        let found = match self.cur_ast().ty(ty) {
            TyExpr::Apply(f, _) => return self.parent_class(f),
            TyExpr::Name(n) => self.lookup_type(n)?,
            TyExpr::Select(q, n) => self.lookup_type_in_path(q, n)?,
            _ => return None,
        };
        match found {
            TypeRef::Class(c) => Some(c),
            TypeRef::Member(..) | TypeRef::ValueMember(..) => None,
            TypeRef::Alias(a) => {
                self.complete_alias(a);
                match self.types.get(self.syms.aliases[a.idx()].rhs) {
                    Type::Class(c, _) | Type::Ctor(c) => Some(c),
                    _ => None,
                }
            }
            TypeRef::Param(_) => None,
        }
    }

    /// Where an extension method or given that is defined in the trait `owner` is found when it
    /// is used: on `this` inside the trait and its subclasses, otherwise on the object through
    /// which it was imported or exported.
    pub fn trait_member_site(&mut self, sym: SymId, owner: ClassId) -> TraitMemberSite {
        let mut innermost = true;
        for i in (0..self.env.frames.len()).rev() {
            let Frame::Class(k) = &self.env.frames[i] else { continue };
            let k = *k;
            if k == owner {
                return TraitMemberSite::This(Some(k));
            }
            let info = self.syms.class(k);
            if info.base_types.iter().any(|&(b, _)| b == owner) {
                let module = !innermost && info.kind == ClassKind::Object;
                return if module { TraitMemberSite::Module(k) } else { TraitMemberSite::This(Some(k)) };
            }
            if let Some(&m) = self.exports_of(k).as_ref().and_then(|e| e.via.get(&sym)) {
                return TraitMemberSite::Module(m);
            }
            innermost = false;
        }
        let n_imports = self.import_count();
        for i in 0..n_imports {
            let c = match self.import_at(i).target {
                ImportTarget::ClassAll(c) | ImportTarget::ClassGivens(c) | ImportTarget::ClassMember(c, _) => c,
                ImportTarget::PkgAll(p) | ImportTarget::PkgGivens(p) | ImportTarget::PkgMember(p, _) => {
                    match self.syms.pkg(p).package_object {
                        Some(obj) => obj,
                        None => continue,
                    }
                }
                ImportTarget::ValueAll(v) | ImportTarget::ValueMember(v, _) | ImportTarget::ValueGivens(v) => {
                    let imp = self.import_at(i);
                    let name = self.syms.sym(sym).name;
                    let brings = match imp.target {
                        ImportTarget::ValueMember(_, orig) => orig == name,
                        ImportTarget::ValueGivens(_) => self.syms.is_given(sym),
                        _ => imp.name.is_none() && !self.import_hides(imp, name),
                    };
                    let derives = brings && self.import_value_class(v).map_or(false, |k| self.syms.class(k).base_types.iter().any(|&(b, _)| b == owner));
                    if derives {
                        return TraitMemberSite::Value(v);
                    }
                    continue;
                }
                ImportTarget::UnimportPredef | ImportTarget::Unresolved => continue,
            };
            if let Some(&m) = self.exports_of(c).as_ref().and_then(|e| e.via.get(&sym)) {
                return TraitMemberSite::Module(m);
            }
            // An object of another module's products has no table of what it inherits
            // (`product_exports`): the trait's given an import of the object brings is reached
            // through the object, as a build of its source has it (`add_inherited_members`).
            if self.syms.class(c).kind == ClassKind::Object && c != owner && self.derives_from(c, owner) {
                let imp = self.import_at(i);
                let name = self.syms.sym(sym).name;
                let brings = match imp.target {
                    ImportTarget::ClassMember(_, orig) => orig == name,
                    ImportTarget::ClassGivens(_) => self.syms.is_given(sym),
                    ImportTarget::ClassAll(_) => imp.name.is_none() && !self.import_hides(imp, name),
                    _ => false,
                };
                if brings {
                    return TraitMemberSite::Module(c);
                }
            }
        }
        for p in self.pkg_chain().iter().copied() {
            let Some(obj) = self.syms.pkg(p).package_object else { continue };
            if let Some(&m) = self.exports_of(obj).as_ref().and_then(|e| e.via.get(&sym)) {
                return TraitMemberSite::Module(m);
            }
        }
        TraitMemberSite::This(None)
    }

    /// The type arguments that the class at the site gives to the trait `owner`.
    pub fn trait_member_subst(&mut self, site: TraitMemberSite, owner: ClassId) -> Subst {
        if let TraitMemberSite::Value(v) = site {
            let ty = self.import_value_ret(v);
            let ty = self.zonk(ty);
            return match self.base_type(ty, owner) {
                Some(bt) => self.owner_subst(bt),
                None => Vec::new(),
            };
        }
        let (TraitMemberSite::This(Some(k)) | TraitMemberSite::Module(k)) = site else { return Vec::new() };
        self.complete_class(k);
        match self.syms.class(k).base_types.iter().find(|&&(b, _)| b == owner) {
            Some(&(_, bt)) => self.owner_subst(bt),
            None => Vec::new(),
        }
    }

    pub fn trait_member_receiver(&mut self, site: TraitMemberSite) -> TExprId {
        match site {
            TraitMemberSite::Module(m) => self.prog.add(TExpr::Module(m)),
            TraitMemberSite::This(Some(k)) => self.this_ref(k),
            TraitMemberSite::This(None) => self.prog.add(TExpr::This),
            TraitMemberSite::Value(v) => {
                let span = self.syms.sym(self.import_values[v.0 as usize].0).span;
                match self.import_value_ref(v, span) {
                    Some((te, _)) => te,
                    None => self.prog.add(TExpr::Unit),
                }
            }
        }
    }
}
