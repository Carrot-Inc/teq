//! The program's publications off the loader's lock:
//! a body or a class check typed into a worker's own chunk is made every worker's without the lock,
//! its records first, then its side tables, then its entry points in the order a reader entering
//! at each needs, each stored with release. The assertion-enabled builds seal exactly the records
//! a publication's roots reach as its entry is stored, by either carrier (a bundle's store, a
//! hold's release), and count what the bundles' checks met, by entry-point class, for the checks'
//! log (`summary_line`).

use super::{PendingShared, Worker};
#[cfg(debug_assertions)]
use crate::tir::{FunId, TExpr, TInit, TPat, TStmt, TypeTest};
use crate::types::ClassId;

/// What a publication makes every worker's, read by the entry point a reader enters at.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Entry {
    /// A body's function (`fun_of_sym`).
    Fun,
    /// A val's initialiser (`val_init`).
    Val,
    /// A class's `TClass` (`Program::class_bodies`).
    Class,
    /// A class checked (`class_done`).
    Done,
    /// A `Js` template's definition (`Program::template_syms`).
    Template,
}

const ENTRIES: usize = 5;
const ENTRY_NAMES: [&str; ENTRIES] = ["fun", "val", "class", "done", "template"];

/// A peer's record read past an entry point: by an id inside a record read before (`Direct`), or
/// from what a thread's interpreter cached (`Cache`). The assertion-enabled build counts them.
#[cfg(debug_assertions)]
#[derive(Clone, Copy)]
pub enum Read {
    Direct,
    Cache,
}

/// What a publication goes by: a bundle stored outside the loader's lock, or the release of a hold,
/// whose publications the assertion-enabled build seals and counts alone (`seal_released`).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Carrier {
    Bundle,
    #[cfg(debug_assertions)]
    Release,
}

#[cfg(debug_assertions)]
mod counts {
    use std::sync::atomic::{AtomicU64, Ordering};
    pub static PUBLISHED: [[AtomicU64; super::ENTRIES]; 2] = [const { [const { AtomicU64::new(0) }; super::ENTRIES] }; 2];
    pub static ENTERED: [AtomicU64; super::ENTRIES] = [const { AtomicU64::new(0) }; super::ENTRIES];
    pub static READ: [AtomicU64; 2] = [const { AtomicU64::new(0) }; 2];
    pub static SEALED: AtomicU64 = AtomicU64::new(0);
    /// The publications the lock carries, by why: made inside a hold, or with a type to export.
    pub static HELD: AtomicU64 = AtomicU64::new(0);
    pub static EXPORTED: AtomicU64 = AtomicU64::new(0);
    /// The waits for a cell whose publication `TEQ_BUNDLE_PAUSE` was withholding.
    pub static WITHHELD_WAITS: AtomicU64 = AtomicU64::new(0);

    pub fn take(c: &AtomicU64) -> u64 {
        c.swap(0, Ordering::Relaxed)
    }
}

/// A publication's entry of class `e` stored by `carrier`, its checks made.
#[inline]
pub fn published(e: Entry, carrier: Carrier) {
    #[cfg(debug_assertions)]
    counts::PUBLISHED[(carrier == Carrier::Release) as usize][e as usize].fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let _ = (e, carrier);
}

/// A peer's publication entered at an entry point of class `e`, its checks made.
#[cfg(debug_assertions)]
#[inline]
pub fn entered(e: Entry) {
    counts::ENTERED[e as usize].fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}

/// A reader entered a publication at an entry point of class `e`, which gave it the record `i` of
/// `arena`: where the record is a peer's, the publication sealed it before storing the entry.
#[inline]
#[cfg_attr(debug_assertions, track_caller)]
pub fn entered_at<T>(arena: &crate::arena::Arena<T>, i: u32, e: Entry) {
    #[cfg(debug_assertions)]
    if arena.is_peer(i) {
        assert!(arena.sealed(i), "a peer's publication entered at its {} entry before its records were sealed (record {})", ENTRY_NAMES[e as usize], i);
        entered(e);
    }
    let _ = (arena, i, e);
}

/// A thread's cache answered with a peer's record `i` of `arena`, sealed since it was cached.
#[inline]
#[cfg_attr(debug_assertions, track_caller)]
pub fn cached_at<T>(arena: &crate::arena::Arena<T>, i: u32) {
    #[cfg(debug_assertions)]
    if arena.is_peer(i) {
        assert!(arena.sealed(i), "a cache answered with a peer's record {} its publication never sealed", i);
        count(Read::Cache);
    }
    let _ = (arena, i);
}

/// A peer's record read past an entry point, checked sealed.
#[cfg(debug_assertions)]
#[inline]
pub fn count(r: Read) {
    counts::READ[r as usize].fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}

/// What the bundles' checks met since the last fork's join, for its line in the checks' log; the
/// counts start again.
#[cfg(debug_assertions)]
pub fn summary_line() -> String {
    use counts::*;
    let list = |row: &[std::sync::atomic::AtomicU64; ENTRIES]| ENTRY_NAMES.iter().zip(row.iter()).map(|(n, c)| format!("{} {}", n, take(c))).collect::<Vec<_>>().join(", ");
    format!(
        "bundles: published {}; released {}; entered {}; read direct {}, cache {}; sealed {}; by the lock inside a hold {}, for an export {}; waited while withheld {}",
        list(&PUBLISHED[0]),
        list(&PUBLISHED[1]),
        list(&ENTERED),
        take(&READ[0]),
        take(&READ[1]),
        take(&SEALED),
        take(&HELD),
        take(&EXPORTED),
        take(&WITHHELD_WAITS)
    )
}

/// A record a publication's roots reach, by its kind and id.
#[cfg(debug_assertions)]
#[derive(Clone, Copy)]
pub(super) enum Node {
    Expr(u32),
    Exprs(crate::ast::ListRef),
    Syms(crate::ast::ListRef),
    Pats(crate::ast::ListRef),
    Stmts(crate::ast::ListRef),
    Cases(crate::ast::ListRef),
    Pat(u32),
    Try(u32),
    Test(u32),
    Fun(u32),
    TClass(u32),
    Quote(u32),
    QuotePat(u32),
    Str(u32),
    Sym(u32),
    Class(u32),
}

impl<'a> Worker<'a> {
    /// Publishes, outside the loader's lock, the body of `sym` this worker typed into its own chunk
    /// (`publish_body`'s bundle): the classes it made and the templates it named, then its function
    /// or initialiser. Under a hold, or with a variance registration whose type the base has to take
    /// (an export, the holder's), the hold's release publishes it.
    pub(super) fn publish_body_bundle(&mut self, sym: crate::types::SymId) {
        let fun = self.fun_of_sym.local.get(&sym).copied();
        let init = self.val_init.local.get(&sym).copied();
        let outside = crate::shared::lock_depth() == 0;
        if outside {
            crate::shake::point(crate::shake::Point::Computed);
        }
        self.settle_registered_merges();
        if !outside || self.pending_exports() {
            #[cfg(debug_assertions)]
            (if outside { &counts::EXPORTED } else { &counts::HELD }).fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            return self.with_loader_for(crate::measure::Hold::Publication, |w| {
                if let Some(f) = fun {
                    w.fun_of_sym.insert_shared(sym, f);
                }
                if let Some(e) = init {
                    w.val_init.insert_shared(sym, e);
                }
                w.publish_class_bodies();
            });
        }
        #[cfg(debug_assertions)]
        {
            let mut roots = self.bundle_class_roots();
            roots.extend(fun.map(|f| Node::Fun(f.0)));
            roots.extend(init.map(|e| Node::Expr(e.0)));
            self.seal_bundle(roots);
        }
        self.publish_side_tables();
        pause_entry(crate::shared::CellKey::new(super::check::CELL_BODY, sym.0), if fun.is_some() { Entry::Fun } else { Entry::Val });
        self.publish_bundle_classes();
        if let Some(f) = fun {
            self.fun_of_sym.publish(sym, f);
            published(Entry::Fun, Carrier::Bundle);
        }
        if let Some(e) = init {
            self.val_init.publish(sym, e);
            published(Entry::Val, Carrier::Bundle);
        }
        #[cfg(debug_assertions)]
        self.fault_after_entry(fun.and_then(|f| self.prog.funs.raw(f.0).body).or(init));
    }

    /// Publishes, outside the loader's lock, the check of the shared class `c` this worker made
    /// into its own chunk: its `TClass` and the classes made since the last publication, then the
    /// check done. Under a hold, or with a variance registration to export, the hold's release does.
    pub(super) fn publish_check_bundle(&mut self, c: ClassId) {
        crate::shake::point(crate::shake::Point::Computed);
        self.settle_registered_merges();
        let outside = crate::shared::lock_depth() == 0;
        if !outside || self.pending_exports() {
            #[cfg(debug_assertions)]
            (if outside { &counts::EXPORTED } else { &counts::HELD }).fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            return self.with_loader_for(crate::measure::Hold::Publication, |w| {
                w.class_done.insert_shared(c, ());
                w.publish_class_bodies();
            });
        }
        #[cfg(debug_assertions)]
        {
            let roots = self.bundle_class_roots();
            self.seal_bundle(roots);
        }
        self.publish_side_tables();
        pause_entry(crate::shared::CellKey::new(super::check::CELL_CHECK, c.0), Entry::Done);
        self.publish_bundle_classes();
        if self.class_done.publish(c, ()) {
            published(Entry::Done, Carrier::Bundle);
        }
        #[cfg(debug_assertions)]
        self.fault_after_entry(None);
    }

    /// A reader found `c` checked (`class_done`) and returns without its cell: where the check is
    /// a peer's publication, its `TClass` was registered and sealed before the entry was stored.
    #[inline]
    pub(super) fn entered_done(&self, c: ClassId) {
        #[cfg(debug_assertions)]
        if self.forked && !self.class_done.local.contains_key(&c) {
            if let Some(&i) = self.prog.class_bodies.get(&c) {
                entered_at(&self.prog.classes, i, Entry::Done);
            }
        }
        let _ = c;
    }

    /// Whether a registration waiting for the next publication names a type the base has to take
    /// first (a variance registration's, exported by the loader's lock holder alone).
    fn pending_exports(&self) -> bool {
        self.pending_shared.iter().any(|p| match p {
            PendingShared::Variance(_, t) => self.types.translate(self.types.view_here(), *t) != *t,
            PendingShared::Template(..) => false,
        })
    }

    /// The registrations waiting for the next publication, every worker's from here: a template's
    /// definition, a variance registration of a type the base holds.
    fn publish_side_tables(&mut self) {
        for p in std::mem::take(&mut self.pending_shared) {
            match p {
                PendingShared::Variance(k, t) => {
                    self.unchecked_variance.publish(k, t);
                }
                PendingShared::Template(s, sym) => {
                    self.prog.template_syms.publish(s, sym);
                    published(Entry::Template, Carrier::Bundle);
                }
            }
        }
    }

    /// The `TClass`es this worker made since its last publication, every worker's from here, with
    /// the check of each it checked: what a reader of a body or of a class may enter by.
    fn publish_bundle_classes(&mut self) {
        let base = self.prog.classes.own_base();
        for k in self.tclasses_to_register() {
            let c = self.prog.classes.own()[k].id;
            self.prog.class_bodies.publish(c, base + k as u32);
            published(Entry::Class, Carrier::Bundle);
            if self.class_done.local.contains_key(&c) && self.class_done.publish(c, ()) {
                published(Entry::Done, Carrier::Bundle);
            }
            self.tclass_registered(k);
        }
    }

    /// The merges of inherited alternatives still pending in the worker's own classes the next
    /// publication registers, settled first (`merge_inherited`): a published class's members are
    /// never changed before the merge, where a lookup would otherwise settle them after it.
    /// Outside the loader's lock alone, where a signature another worker holds is waited for
    /// (`merge_inherited`); a publication inside a hold registers what is settled by then. A
    /// publication made inside a merge (a signature it completed, a body that signature typed)
    /// settles the classes made since as well, before its seal reaches them, and leaves a class
    /// with an entry under merge, every entry of it and its registration, to the merge that holds
    /// it: another entry's comparison could read the signature that merge is completing.
    pub(super) fn settle_registered_merges(&mut self) {
        if !self.forked || crate::shared::lock_depth() > 0 {
            return;
        }
        for k in self.tclasses_to_register_pending() {
            let c = self.prog.classes.own()[k].id;
            if self.shared_class(c) || self.under_merge(c) {
                continue;
            }
            let pending: Vec<crate::types::SymId> = self.syms.class(c).members.values().copied().filter(|&m| self.syms.sym(m).merge_pending).collect();
            for entry in pending {
                if self.syms.sym(entry).merge_pending && !self.under_merge(c) {
                    self.settle_own_merge(c, entry);
                }
            }
        }
    }

    /// Whether an entry of `c` is being merged in an enclosing frame (`merging`).
    fn under_merge(&self, c: ClassId) -> bool {
        self.merging.iter().any(|&m| self.syms.class(c).members.get(&self.syms.sym(m).name) == Some(&m))
    }

    /// `merge_inherited` for the worker's own class `c`: the set made outside the loader's lock
    /// unless it marks a shared alternative, whose record only the lock's holder changes.
    fn settle_own_merge(&mut self, c: ClassId, entry: crate::types::SymId) {
        self.merging.push(entry);
        let all = self.merged_alternatives(c, entry);
        self.merging.pop();
        if all.1.iter().any(|&a| self.shared_sym(a) && !self.syms.sym(a).alternative) {
            self.with_loader(|w| {
                w.settle_merged(c, entry, all);
                w.syms.sym_mut(entry).merge_pending = false;
            });
        } else {
            self.settle_merged(c, entry, all);
            self.syms.sym_mut(entry).merge_pending = false;
        }
    }

    /// Runs `f`, a maker that fixes up in place the classes a copier makes for it: the copies it
    /// makes are registered by no publication before `f` ends (`tclasses_settling`).
    pub(super) fn settling_classes<R>(&mut self, f: impl FnOnce(&mut Self) -> R) -> R {
        let mark = self.tclasses_settling.len();
        self.tclass_settling_scopes += 1;
        let out = f(self);
        self.tclass_settling_scopes -= 1;
        self.tclasses_settling.truncate(mark);
        out
    }

    /// The worker's own `TClass`es the next publication registers, by their index among its own:
    /// the ones made since the last, but those their maker still fixes up in place
    /// (`tclasses_settling`), those made in a class check still under way
    /// (`note_nested_in_check`) and those with merges pending.
    pub(super) fn tclasses_to_register(&self) -> Vec<usize> {
        self.tclasses_to_register_pending().into_iter().filter(|&k| !self.merges_pending(self.prog.classes.own()[k].id)).collect()
    }

    /// `tclasses_to_register` with the classes whose merges are pending, which
    /// `settle_registered_merges` settles first.
    fn tclasses_to_register_pending(&self) -> Vec<usize> {
        let base = self.prog.classes.own_base();
        (self.tclasses_published..self.prog.classes.own().len())
            .filter(|k| !self.tclasses_registered.contains_key(k) && !self.tclasses_settling.contains(&(base + *k as u32)))
            .filter(|&k| !self.in_unfinished_check(base + k as u32))
            .collect()
    }

    /// The `TClass` `id` this worker just pushed, made inside the check of an own class of its
    /// (a class nested in it, or one a member's body defines), whose body may read that class's
    /// members: no publication registers it before the outermost such check ends, which finishes
    /// those members' records (`mark_accessors`) after the nested classes are made.
    pub(super) fn note_nested_in_check(&mut self, id: u32) {
        if !self.forked || !self.prog.classes.is_own(id) {
            return;
        }
        let enclosing = self.env.frames.iter().find_map(|f| match f {
            super::Frame::Class(k) if !self.shared_class(*k) && self.prog.classes.own()[(id - self.prog.classes.own_base()) as usize].id != *k && self.syms.check_cells.get(k.0) == crate::symbols::Completion::InProgress && self.syms.check_cells.mine(k.0) => Some(*k),
            _ => None,
        });
        if let Some(k) = enclosing {
            self.tclasses_in_checks.push((id, k));
        }
    }

    /// Whether the `TClass` `id` waits for the check of a class it was made in (`note_nested_in_check`).
    fn in_unfinished_check(&self, id: u32) -> bool {
        self.tclasses_in_checks.iter().any(|&(t, k)| t == id && self.syms.check_cells.get(k.0) != crate::symbols::Completion::Done)
    }

    /// Whether a worker's own class has members whose inherited alternatives no lookup merged
    /// yet: its registration waits for a publication that settles them (`settle_registered_merges`,
    /// outside the loader's lock and outside another settle).
    fn merges_pending(&self, c: ClassId) -> bool {
        !self.shared_class(c) && self.syms.class(c).members.values().any(|&m| self.syms.sym(m).merge_pending)
    }

    /// The worker's own `TClass` at `k` is registered: the watermark moves past every registered
    /// one before the first that is not.
    pub(super) fn tclass_registered(&mut self, k: usize) {
        self.tclasses_registered.insert(k, ());
        while self.tclasses_registered.remove(&self.tclasses_published).is_some() {
            self.tclasses_published += 1;
        }
    }

    /// Whether the worker's own `TClass` at `i` is registered (`tclass_registered`).
    pub(super) fn tclass_is_registered(&self, i: usize) -> bool {
        let k = i - self.prog.classes.own_base() as usize;
        k < self.tclasses_published || self.tclasses_registered.contains_key(&k)
    }

    /// The roots of the `TClass`es the next publication makes every worker's.
    #[cfg(debug_assertions)]
    fn bundle_class_roots(&self) -> Vec<Node> {
        let base = self.prog.classes.own_base();
        self.tclasses_to_register().into_iter().map(|k| Node::TClass(base + k as u32)).collect()
    }

    /// What a publication's entry makes every worker's, marked: escaped (the watermark, a prefix
    /// that may hold an enclosing record still being made) and sealed (exactly the records the
    /// roots reach), before the entry is stored.
    #[cfg(debug_assertions)]
    fn seal_bundle(&mut self, roots: Vec<Node>) {
        self.syms.escape_own();
        self.prog.escape_own();
        if fault() == Fault::Unsealed {
            // The test's fault: a bundle whose entries are stored with its records never sealed.
            return;
        }
        self.seal_reached(roots, true);
    }

    /// `TEQ_BUNDLE_FAULT=write`, after a bundle's entries are stored: a record of the bundle
    /// written, which the seal refuses.
    #[cfg(debug_assertions)]
    fn fault_after_entry(&mut self, root: Option<crate::tir::TExprId>) {
        match fault() {
            Fault::Write => {
                if let Some(e) = root.filter(|e| self.prog.exprs.is_own(e.0)) {
                    let span = self.prog.span_of(e).unwrap_or((crate::source::FileId(0), crate::source::Span::default()));
                    self.prog.set_span(e, span.0, span.1);
                }
            }
            Fault::None | Fault::Unsealed => {}
        }
    }

    /// The release's publications, sealed: the worker's own records the entries it applies name
    /// (`lock_released`), which the escape watermark marked already.
    #[cfg(debug_assertions)]
    pub(super) fn seal_released(&self) {
        let own = |x: u32| x >= crate::arena::LOCAL_BASE;
        let mut roots: Vec<Node> = Vec::new();
        for (_, &f) in self.fun_of_sym.pending() {
            if own(f.0) {
                roots.push(Node::Fun(f.0));
                published(Entry::Fun, Carrier::Release);
            }
        }
        for (_, &e) in self.val_init.pending() {
            if own(e.0) {
                roots.push(Node::Expr(e.0));
                published(Entry::Val, Carrier::Release);
            }
        }
        for (_, &i) in self.prog.class_bodies.pending() {
            if own(i) {
                roots.push(Node::TClass(i));
                published(Entry::Class, Carrier::Release);
            }
        }
        self.seal_reached(roots, false);
    }

    /// Seals every record of this worker's own the roots reach through the records' ids (the
    /// program's records and the symbols they name, not the types: a published signature naming a
    /// worker's class leaves it under the escape watermark), each once.
    #[cfg(debug_assertions)]
    fn seal_reached(&self, mut work: Vec<Node>, settled: bool) {
        let p = &self.prog;
        let s = &self.syms;
        let mut sealed = 0u64;
        let list = |work: &mut Vec<Node>, l: crate::ast::ListRef, f: fn(crate::ast::ListRef) -> Node| {
            if l.len > 0 {
                work.push(f(l));
            }
        };
        while let Some(n) = work.pop() {
            match n {
                Node::Expr(i) => {
                    if !p.exprs.seal(i) {
                        continue;
                    }
                    match *p.exprs.raw(i) {
                        TExpr::Str(t) => work.push(Node::Str(t.0)),
                        TExpr::Local(x) | TExpr::Static(x) => work.push(Node::Sym(x.0)),
                        TExpr::Super(crate::tir::SuperTarget::Class(c) | crate::tir::SuperTarget::Mixin(c)) | TExpr::Module(c) | TExpr::ClassOf(c) => work.push(Node::Class(c.0)),
                        TExpr::Field(e, x) => {
                            work.push(Node::Expr(e.0));
                            work.push(Node::Sym(x.0));
                        }
                        TExpr::CallStatic(x, l) | TExpr::NewVia(x, l) => {
                            work.push(Node::Sym(x.0));
                            list(&mut work, l, Node::Exprs);
                        }
                        TExpr::CallMethod(e, x, l) => {
                            work.push(Node::Expr(e.0));
                            work.push(Node::Sym(x.0));
                            list(&mut work, l, Node::Exprs);
                        }
                        TExpr::CallClosure(e, l) => {
                            work.push(Node::Expr(e.0));
                            list(&mut work, l, Node::Exprs);
                        }
                        TExpr::New(c, l) => {
                            work.push(Node::Class(c.0));
                            list(&mut work, l, Node::Exprs);
                        }
                        TExpr::Lambda(l, e) => {
                            list(&mut work, l, Node::Syms);
                            work.push(Node::Expr(e.0));
                        }
                        TExpr::If(a, b, c) => {
                            work.push(Node::Expr(a.0));
                            work.push(Node::Expr(b.0));
                            work.extend(c.map(|c| Node::Expr(c.0)));
                        }
                        TExpr::While(a, b) | TExpr::Assign(a, b) | TExpr::Prim(_, a, b) => {
                            work.push(Node::Expr(a.0));
                            work.push(Node::Expr(b.0));
                        }
                        TExpr::Block(l, e) => {
                            list(&mut work, l, Node::Stmts);
                            work.push(Node::Expr(e.0));
                        }
                        TExpr::Match(e, l) => {
                            work.push(Node::Expr(e.0));
                            list(&mut work, l, Node::Cases);
                        }
                        TExpr::Unary(_, e) | TExpr::ToStr(e, _) | TExpr::Index(e, _) | TExpr::JsSelect(e, _) | TExpr::Spread(e) | TExpr::Return(e) | TExpr::Throw(e, _) | TExpr::Splice(e) => work.push(Node::Expr(e.0)),
                        TExpr::StrConcat(l) | TExpr::SeqLit(l) | TExpr::ArrayLit(l) | TExpr::ObjLit(l) => list(&mut work, l, Node::Exprs),
                        TExpr::Js(t, l) => {
                            work.push(Node::Str(t.0));
                            list(&mut work, l, Node::Exprs);
                            let at = match p.strings.raw(t.0).as_str() {
                                "$quote" => Some(0),
                                "$quoteMatch" => Some(1),
                                _ => None,
                            };
                            if let Some(&arg) = at.and_then(|at| p.expr_list(l).get(at)) {
                                if let TExpr::Int(q) = *p.exprs.raw(arg.0) {
                                    work.push(if at == Some(0) { Node::Quote(q as u32) } else { Node::QuotePat(q as u32) });
                                }
                            }
                        }
                        TExpr::TypeTest(e, t) | TExpr::Cast(e, crate::tir::CastOp::Check(t, _) | crate::tir::CastOp::Unbox(t, _), _) => {
                            work.push(Node::Expr(e.0));
                            work.push(Node::Test(t.0));
                        }
                        TExpr::Cast(e, ..) => work.push(Node::Expr(e.0)),
                        TExpr::Try(i) => work.push(Node::Try(i)),
                        TExpr::Int(_) | TExpr::Long(_) | TExpr::Double(_) | TExpr::Bool(_) | TExpr::Char(_) | TExpr::Unit | TExpr::This | TExpr::Super(crate::tir::SuperTarget::Chain) | TExpr::JsImport(_) | TExpr::JsGlobal(..) | TExpr::Null => {}
                    }
                }
                Node::Exprs(l) => {
                    for k in l.start..l.start + l.len {
                        if p.expr_lists.seal(k) {
                            work.push(Node::Expr(p.expr_lists.raw(k).0));
                        }
                    }
                }
                Node::Syms(l) => {
                    for k in l.start..l.start + l.len {
                        if p.sym_lists.seal(k) {
                            work.push(Node::Sym(p.sym_lists.raw(k).0));
                        }
                    }
                }
                Node::Pats(l) => {
                    for k in l.start..l.start + l.len {
                        if p.pat_lists.seal(k) {
                            work.push(Node::Pat(p.pat_lists.raw(k).0));
                        }
                    }
                }
                Node::Stmts(l) => {
                    for k in l.start..l.start + l.len {
                        if !p.stmts.seal(k) {
                            continue;
                        }
                        match *p.stmts.raw(k) {
                            TStmt::Expr(e) => work.push(Node::Expr(e.0)),
                            TStmt::Val(x, e) => {
                                work.push(Node::Sym(x.0));
                                work.push(Node::Expr(e.0));
                            }
                            TStmt::Fun(f) => work.push(Node::Fun(f.0)),
                            TStmt::Pat(q, e) => {
                                work.push(Node::Pat(q.0));
                                work.push(Node::Expr(e.0));
                            }
                        }
                    }
                }
                Node::Cases(l) => {
                    for k in l.start..l.start + l.len {
                        if !p.cases.seal(k) {
                            continue;
                        }
                        let c = *p.cases.raw(k);
                        work.push(Node::Pat(c.pat.0));
                        work.extend(c.guard.map(|g| Node::Expr(g.0)));
                        work.push(Node::Expr(c.body.0));
                    }
                }
                Node::Pat(i) => {
                    if !p.pats.seal(i) {
                        continue;
                    }
                    match *p.pats.raw(i) {
                        TPat::Wildcard => {}
                        TPat::Bind(x, q) => {
                            work.push(Node::Sym(x.0));
                            work.extend(q.map(|q| Node::Pat(q.0)));
                        }
                        TPat::Test(t, _, q) => {
                            work.push(Node::Test(t.0));
                            work.push(Node::Pat(q.0));
                        }
                        TPat::Equals(e, _) => work.push(Node::Expr(e.0)),
                        TPat::Class(c, _, fields, pats) => {
                            work.push(Node::Class(c.0));
                            list(&mut work, fields, Node::Syms);
                            list(&mut work, pats, Node::Pats);
                        }
                        TPat::Alt(l) => list(&mut work, l, Node::Pats),
                        TPat::Seq(l, q) => {
                            list(&mut work, l, Node::Pats);
                            work.extend(q.map(|q| Node::Pat(q.0)));
                        }
                        TPat::Unapply(x, e, q) => {
                            work.push(Node::Sym(x.0));
                            work.push(Node::Expr(e.0));
                            work.push(Node::Pat(q.0));
                        }
                    }
                }
                Node::Try(i) => {
                    if !p.tries.seal(i) {
                        continue;
                    }
                    let t = p.tries.raw(i);
                    work.push(Node::Expr(t.body.0));
                    list(&mut work, t.cases, Node::Cases);
                    work.extend(t.finalizer.map(|f| Node::Expr(f.0)));
                }
                Node::Test(i) => {
                    if !p.tests.seal(i) {
                        continue;
                    }
                    match *p.tests.raw(i) {
                        TypeTest::Value(e) => work.push(Node::Expr(e.0)),
                        TypeTest::Or(a, b) | TypeTest::And(a, b) => {
                            work.push(Node::Test(a.0));
                            work.push(Node::Test(b.0));
                        }
                        TypeTest::Class(c) | TypeTest::Trait(c) => work.push(Node::Class(c.0)),
                        TypeTest::Outer(_, inner) => work.push(Node::Test(inner.0)),
                        _ => {}
                    }
                }
                Node::Fun(i) => {
                    if !p.funs.seal(i) {
                        continue;
                    }
                    let f = p.funs.raw(i);
                    work.push(Node::Sym(f.sym.0));
                    work.extend(f.params.iter().map(|x| Node::Sym(x.0)));
                    work.extend(f.defaults.iter().flatten().map(|e| Node::Expr(e.0)));
                    work.extend(f.body.map(|e| Node::Expr(e.0)));
                }
                Node::TClass(i) => {
                    if !p.classes.seal(i) {
                        continue;
                    }
                    let tc = p.classes.raw(i);
                    work.push(Node::Class(tc.id.0));
                    work.extend(tc.ctor_params.iter().map(|x| Node::Sym(x.0)));
                    work.extend(tc.ctor_defaults.iter().flatten().map(|e| Node::Expr(e.0)));
                    if let Some(l) = tc.parent_args {
                        list(&mut work, l, Node::Exprs);
                    }
                    work.extend(tc.parent_via.map(|x| Node::Sym(x.0)));
                    list(&mut work, tc.parent_prelude, Node::Stmts);
                    for init in &tc.init {
                        match *init {
                            TInit::Field(x, e) => {
                                work.push(Node::Sym(x.0));
                                work.push(Node::Expr(e.0));
                            }
                            TInit::Stmt(e) => work.push(Node::Expr(e.0)),
                            TInit::Parent(c, pc) => {
                                work.push(Node::Class(c.0));
                                list(&mut work, pc.prelude, Node::Stmts);
                                list(&mut work, pc.args, Node::Exprs);
                                work.extend(pc.via.map(|x| Node::Sym(x.0)));
                            }
                        }
                    }
                    let funs: Vec<FunId> = tc.methods.iter().chain(tc.ctors.iter()).copied().collect();
                    work.extend(funs.iter().map(|f| Node::Fun(f.0)));
                    for &(a, b) in tc.forwarders.iter().chain(tc.bridges.iter()).chain(tc.deferred_givens.iter()) {
                        work.push(Node::Sym(a.0));
                        work.push(Node::Sym(b.0));
                    }
                    for sa in &tc.super_accessors {
                        work.push(Node::Class(sa.of_trait.0));
                        work.push(Node::Sym(sa.member.0));
                        work.extend(sa.target.map(|x| Node::Sym(x.0)));
                    }
                }
                Node::Quote(i) => {
                    if !p.quotes.seal(i) {
                        continue;
                    }
                    let q = p.quotes.raw(i);
                    work.extend(q.body.map(|e| Node::Expr(e.0)));
                    for &(x, e) in &q.holes {
                        work.push(Node::Sym(x.0));
                        work.push(Node::Expr(e.0));
                    }
                    work.extend(q.types.iter().map(|&(_, e)| Node::Expr(e.0)));
                    work.extend(q.binders.iter().map(|x| Node::Sym(x.0)));
                    work.extend(q.quotes.map(|e| Node::Expr(e.0)));
                }
                Node::QuotePat(i) => {
                    if !p.quote_pats.seal(i) {
                        continue;
                    }
                    let q = p.quote_pats.raw(i);
                    work.extend(q.body.map(|e| Node::Expr(e.0)));
                    work.extend(q.holes.iter().map(|x| Node::Sym(x.0)));
                    work.extend(q.types.iter().map(|&(_, e)| Node::Expr(e.0)));
                    work.extend(q.quotes.map(|e| Node::Expr(e.0)));
                }
                Node::Str(i) => {
                    if !p.strings.seal(i) {
                        continue;
                    }
                }
                Node::Sym(i) => {
                    if !s.syms.seal(i) {
                        continue;
                    }
                }
                Node::Class(i) => {
                    if !s.classes.seal(i) {
                        continue;
                    }
                    if settled && s.class(ClassId(i)).members.values().any(|&m| s.sym(m).merge_pending) {
                        panic!("worker {} sealed its class {} with inherited alternatives not merged: a published class's members are settled before its seal", self.worker, self.name_str(s.class(ClassId(i)).name));
                    }
                }
            }
            sealed += 1;
        }
        counts::SEALED.fetch_add(sealed, std::sync::atomic::Ordering::Relaxed);
    }
}

/// `TEQ_BUNDLE_PAUSE=[<entry>:]<ms>`, a test's: a bundle's entry store (of a function, `fun`, an
/// initialiser, `val`, or a class checked, `done`; every bundle's without the class) is withheld,
/// its records and side tables published, until a reader has waited for its cell or for `ms`, so
/// that a reader comes for it while it is not whole (`waited_for`).
fn pause_entry(key: crate::shared::CellKey, entry: Entry) {
    static PAUSE: std::sync::OnceLock<Option<(Option<Entry>, u64)>> = std::sync::OnceLock::new();
    let pause = *PAUSE.get_or_init(|| {
        let v = std::env::var("TEQ_BUNDLE_PAUSE").ok()?;
        let (of, ms) = match v.split_once(':') {
            Some((of, ms)) => (ENTRY_NAMES.iter().position(|&n| n == of).map(|i| [Entry::Fun, Entry::Val, Entry::Class, Entry::Done, Entry::Template][i]), ms),
            None => (None, v.as_str()),
        };
        Some((of, ms.parse().ok()?))
    });
    let Some((of, ms)) = pause else { return };
    if of.is_some_and(|of| of != entry) {
        return;
    }
    PAUSING.store(true, std::sync::atomic::Ordering::Relaxed);
    WITHHELD.lock().unwrap_or_else(|e| e.into_inner()).push((key.0, false));
    let start = std::time::Instant::now();
    while start.elapsed().as_millis() < ms as u128 {
        let met = WITHHELD.lock().unwrap_or_else(|e| e.into_inner()).iter().any(|&(k, met)| k == key.0 && met);
        if met {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    let mut held = WITHHELD.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(i) = held.iter().position(|&(k, _)| k == key.0) {
        held.swap_remove(i);
    }
}

/// The cells whose bundles' entries `pause_entry` withholds now, and whether a reader waited for
/// each.
static WITHHELD: std::sync::Mutex<Vec<(u64, bool)>> = std::sync::Mutex::new(Vec::new());
static PAUSING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// A reader waits for the cell `key`: counted where its bundle's entries are being withheld, the
/// evidence that a reader came for a publication while it was not yet whole.
#[inline]
pub fn waited_for(key: crate::shared::CellKey) {
    if !PAUSING.load(std::sync::atomic::Ordering::Relaxed) {
        return;
    }
    let mut held = WITHHELD.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(w) = held.iter_mut().find(|w| w.0 == key.0) {
        w.1 = true;
        #[cfg(debug_assertions)]
        counts::WITHHELD_WAITS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
}

/// `TEQ_BUNDLE_FAULT`, a test's, in the assertion-enabled builds: `unsealed` stores a bundle's
/// entries with its records never sealed, an incomplete publication, which a peer's read refuses;
/// `write` writes a record of a bundle after its entries, which the seal refuses.
#[cfg(debug_assertions)]
#[derive(Clone, Copy, PartialEq, Eq)]
enum Fault {
    None,
    Unsealed,
    Write,
}

#[cfg(debug_assertions)]
fn fault() -> Fault {
    static FAULT: std::sync::OnceLock<Fault> = std::sync::OnceLock::new();
    *FAULT.get_or_init(|| match std::env::var("TEQ_BUNDLE_FAULT").as_deref() {
        Ok("unsealed") => Fault::Unsealed,
        Ok("write") => Fault::Write,
        _ => Fault::None,
    })
}
