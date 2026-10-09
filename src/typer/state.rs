//! The state of an attempt: a typing that may be abandoned
//! begins with `attempt`, which takes its position in every journal the typing writes, and ends
//! with `close` (the attempt stands: what it wrote is the enclosing attempt's), `retract` (what it
//! wrote goes, but for what is not its own) or `set_aside` (what it wrote is taken out, for
//! `restore` to put back after another attempt was tried). Attempts nest as marks on a stack do:
//! the one that ends is the innermost open one.
//!
//! The journals most attempts write (the trail, the diagnostics, the index's records and the
//! unused-import marks) are marked when the attempt begins; the ones written rarely (`Rare`) at
//! their first write inside it (`note_rare`), so that an attempt that writes none of them costs
//! nothing there.
//!
//! What is not the attempt's own survives a retraction:
//! - the instance the attempt gave a type variable made inside it, as dotty makes an instance
//!   permanent where the variable's owning state made it (`TypeVar.instantiateWith`), so that a
//!   tree the attempt typed and something keeps names no variable the rollback reopened; the
//!   bounds the attempt put on such a variable go, as they lived in dotty's dropped constraint;
//! - what work done on demand for a definition made before the attempt began wrote (a body typed
//!   for an inferred signature, a signature, an inline definition's check): that definition's, as
//!   dotty completes a symbol in its creation context, whatever becomes of the attempt that asked.

use super::index::{Recorded, Taken};
use super::resolve::DeferredBound;
use super::space::DeferredMatch;
use super::{InfoMessage, Redo, Undo, Worker};
use crate::source::Diagnostic;

/// The journals an attempt marks lazily.
#[derive(Clone, Copy)]
pub(super) enum Rare {
    Infos = 0,
    DeferredMatches = 1,
    DeferredBounds = 2,
    /// The nodes rewritten in place, each with its prior value.
    Rewrites = 3,
    /// The plain inline calls typed as calls, for the later expansion phase.
    Pending = 4,
}

const RARE: usize = 5;
const UNTAKEN: u32 = u32::MAX;

/// A given an extension's prefix resolved (`apply::prefix_holds`): its zonked target type, the
/// parameter it was resolved for where that target was open, its tree and type, and its attempt set
/// aside for the application to take.
pub(super) type Inferred = (crate::types::TypeId, Option<crate::types::SymId>, crate::tir::TExprId, crate::types::TypeId, SetAside);

/// The attempts under way, and the journal ranges work done on demand inside them wrote.
#[derive(Default)]
pub struct Attempts {
    /// How many attempts began: the last one's sequence number.
    seq: u32,
    /// The sequence number of the innermost open attempt, 0 outside every attempt.
    open: u32,
    /// The open attempts, innermost last, with their positions in the rarely written journals.
    stack: Vec<Open>,
    /// The ranges of the journals written by work done on demand inside an open attempt, each
    /// with the symbol count bound of its definition: kept by the retraction of an attempt that
    /// began after the definition was made.
    promoted: Vec<Promoted>,
    /// The nodes rewritten or retyped in place inside an open attempt, each with what it was
    /// before: a retraction puts it back, since the node may be one something retains (an
    /// argument an attempt cached, `ArgCache`).
    rewrites: Vec<Rewrite>,
    /// The plain inline calls typed as calls whose expansion waits for the later phase,
    /// in the order they were typed: arguments before calls.
    pending: Vec<PendingInline>,
    /// How many flushes of `pending` are under way: the later phase, whose errors are no
    /// attempt's verdict.
    flushing: u32,
    /// The givens a probe of an extension's using clauses resolved, by their zonked target type,
    /// for the application that follows to take instead of resolving them again (dotty resolves
    /// a candidate's inferred arguments once): `Worker::inferred_hit`.
    pub(super) inferred: Vec<Inferred>,
    /// The application `inferred` is for: the extension's symbol and the depth of applications
    /// it is applied at (`Worker::app_depth`), so that no nested application takes them.
    pub inferred_for: Option<(crate::types::SymId, u32)>,
    /// The `if`s, by their condition, whose taken branch waits for their conditions' calls
    /// (`Worker::mark_taken_branch`).
    pub fold_later: Vec<(crate::tir::TExprId, crate::tir::TExprId)>,
    /// The tuple indexes over pending calls of constant types, by the index, the block that
    /// evaluates it before the element it reads and the element's read: the index dropped once
    /// the expansions leave it a constant (`Worker::fold_indexes_later`).
    pub index_later: Vec<(crate::tir::TExprId, crate::tir::TExprId, crate::tir::TExprId)>,
    /// Set while a constant is read off a tree whose pending calls stand for their types
    /// (`Worker::fold_by_pending_types`).
    pub by_pending_types: bool,
    /// How many inline givens expanded where a search finds them (transparent ones) have their
    /// using arguments resolved now (`Worker::try_given`): a plain inline given found for one is
    /// expanded at once, after the pending calls typed before it, and one whose expansion fails,
    /// late or not, is no instance of its own search, as where every given expanded as typed.
    pub given_args: u32,
    /// Whether a copy of a typed tree is under way (`Worker::copy_expr`), which expanded the
    /// tree's pending calls when it began.
    pub copying: bool,
    /// Whether the flush records the expansions it moved into their calls' nodes (`moved`),
    /// for the tail-call check, which counts calls node by node.
    pub tracking_moved: u32,
    /// The expansions' roots the flush moved into the calls' nodes, left behind unreached.
    pub moved: Vec<crate::tir::TExprId>,
    /// How many times work done on demand began inside an attempt (`promote_begin`).
    demanded: u32,
    /// The positions of the diagnostics an open attempt holds that scalac reports only in its
    /// `Inlining` phase (a `compiletime.error` inside a transparent expansion typed at typing
    /// time): reported, but no reason for the attempt to fail.
    late: Vec<u32>,
}

/// Where plain inline calls are expanded: at the end of the item, dotty's order within it
/// (`item`, the default); `TEQ_PLAIN_EXPANSION` names the
/// others for measurement: where they are typed, as before (`typing`), or at the outermost
/// attempt's end (`commit`).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum PlainExpansion {
    Typing,
    Commit,
    Item,
}

fn flush_trace() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("TEQ_FLUSH_TRACE").is_some())
}

pub(super) fn plain_expansion() -> PlainExpansion {
    static AT: std::sync::OnceLock<PlainExpansion> = std::sync::OnceLock::new();
    *AT.get_or_init(|| match std::env::var("TEQ_PLAIN_EXPANSION").as_deref() {
        Ok("commit") => PlainExpansion::Commit,
        Ok("typing") => PlainExpansion::Typing,
        _ => PlainExpansion::Item,
    })
}

/// A plain inline call typed as a call, with what its expansion needs of the call site; the
/// call taken out once an expansion that reads it as an argument expanded it first
/// (`expand_pending_in`).
pub(super) struct PendingInline {
    node: crate::tir::TExprId,
    /// The call's type as the typing gave it.
    typed_as: crate::types::TypeId,
    call: Option<PendingCall>,
    /// The last attempt's sequence number when the call was typed: an attempt begun after it
    /// keeps the call's expansion ahead of the later phase (`Worker::expand_pending_in`).
    seq: u32,
    /// What that expansion reported while typing: reported at the flush of the call's unit, where
    /// the later phase reports it (`Worker::release_held`), so that no typing in between takes it
    /// as its own (an attempt's verdict, an argument the retry on the qualifier reuses, a quiet
    /// typing that drops what it reported); a retraction that puts the call back pending takes it.
    held: Vec<crate::source::Diagnostic>,
}

/// What the expansion of a pending call needs of its site.
#[derive(Clone)]
struct PendingCall {
    call: super::apply::MethodCall,
    sig: std::sync::Arc<crate::symbols::MethodSig>,
    subst: crate::types::Subst,
    args: Vec<crate::tir::TExprId>,
    arg_types: Vec<(crate::tir::TExprId, crate::types::TypeId)>,
    ret_ty: crate::types::TypeId,
    span: crate::source::Span,
    expected: Option<crate::types::TypeId>,
    env: std::sync::Arc<super::Env>,
    gadt: Vec<(crate::types::TParamId, crate::types::TypeId, i8)>,
    nowarn: u32,
    body_node: Option<super::BodyNode>,
    of_bodies: bool,
    /// scalac's owner chain at the call, which a macro's `Symbol.spliceOwner` reads.
    owners: Vec<super::site::SiteOwner>,
    bodies_in_progress: Vec<crate::types::SymId>,
    /// Whether the expansion stands at the end of the call it completes (an inline given found
    /// by a search, `InlineState::site_at_end`).
    at_end: bool,
}

/// A node changed in place, with its value or its recorded type as it was; or the pending call
/// at a node expanded ahead of the later phase (`expand_pending_in`), with the call to put back,
/// and, set aside, what its expansion reported for `restore`.
enum Rewrite {
    Expr(crate::tir::TExprId, crate::tir::TExpr),
    Type(crate::tir::TExprId, Option<crate::types::TypeId>),
    Consumed(crate::tir::TExprId, Option<Box<PendingCall>>, Vec<crate::source::Diagnostic>),
}

impl Attempts {
    /// How many times work done on demand began inside an attempt.
    #[inline]
    pub fn demanded(&self) -> u32 {
        self.demanded
    }

    /// How many plain inline calls wait for their expansion.
    #[inline]
    pub fn pending_len(&self) -> usize {
        self.pending.len()
    }

    /// Whether pending plain inline calls wait and no attempt is open: an item's end flushes them.
    #[inline]
    pub fn flushes_at_item_end(&self) -> bool {
        self.open == 0 && !self.pending.is_empty()
    }
}

struct Open {
    seq: u32,
    /// The length of each rarely written journal at the first write inside the attempt, or
    /// `UNTAKEN`.
    at: [u32; RARE],
}

#[derive(Clone, Copy)]
struct Promoted {
    keep: Keep,
    diags: (u32, u32),
    records: (u32, u32),
    rare: [(u32, u32); RARE],
}

/// Which attempts a promoted range is not the work of.
#[derive(Clone, Copy)]
enum Keep {
    /// Work for a definition, one past the index of its symbol: not the work of an attempt
    /// that began with at least this many symbols.
    Made(u32),
    /// The typing of an argument an `ArgCache` retains: not the work of an attempt that began
    /// after the cache opened, this being the last sequence number then.
    Since(u32),
}

impl Keep {
    #[inline]
    fn by(self, m: &Mark) -> bool {
        match self {
            Keep::Made(made) => made <= m.syms,
            Keep::Since(seq) => m.seq > seq,
        }
    }
}

/// Where an attempt began in the journals it marks at once: what lies past it is the
/// attempt's.
#[derive(Clone, Copy)]
pub(super) struct Mark {
    seq: u32,
    outer: u32,
    trail: u32,
    vars: u32,
    syms: u32,
    diags: u32,
    error_nodes: u32,
    index: Recorded,
}

/// What a set-aside attempt wrote, for `restore`.
pub(super) struct SetAside {
    redo: Vec<Redo>,
    diags: Vec<Diagnostic>,
    /// The positions among `diags` of the late ones.
    late: Vec<u32>,
    index: Taken,
    error_nodes: u32,
    infos: Vec<InfoMessage>,
    matches: Vec<DeferredMatch>,
    bounds: Vec<DeferredBound>,
    /// The changed nodes with what the attempt gave them.
    rewrites: Vec<Rewrite>,
    pending: Vec<PendingInline>,
}

/// What `set_aside_journals` took out of an attempt whose trail, diagnostics and index records
/// its caller takes out and puts back itself (a member's retry on the qualifier, `apply.rs`): the
/// rarely written journals, the late errors' positions and the error nodes counted.
pub(super) struct JournalsAside {
    diags_at: u32,
    late: Vec<u32>,
    error_nodes: u32,
    infos: Vec<InfoMessage>,
    matches: Vec<DeferredMatch>,
    bounds: Vec<DeferredBound>,
    rewrites: Vec<Rewrite>,
    pending: Vec<PendingInline>,
}

/// The journal positions work done on demand began at (`Worker::promote_begin`).
pub(super) struct PromoteMark {
    keep: Keep,
    /// Whether its diagnostics are kept: a definition's are, a cached argument's go with the
    /// attempt that typed it, as dotty's failed state's reporter does.
    diags_kept: bool,
    diags: u32,
    records: u32,
    marks: usize,
    rare: [u32; RARE],
}

/// The typed arguments of an application that several attempts apply, dotty's `FunProtoState`
/// (`ProtoTypes` 368 to 498): each argument typed against the first attempt's formal and
/// cached unadapted while that attempt has reported no error, then adapted by every attempt to
/// its own formal, so that nothing the typing made (a macro's run, an anonymous class) is made
/// again.
pub(super) struct ArgCache {
    /// The last attempt's sequence number when the cache opened.
    seq: u32,
    file: crate::source::FileId,
    args: crate::intern::FxMap<crate::ast::ExprId, Option<(crate::tir::TExprId, crate::types::TypeId)>>,
    /// The attempt under way over the arguments.
    attempt: Option<Mark>,
}

/// Takes the entries of `v` from `from` on out, but those at the positions `kept` (ascending),
/// which stay in their order.
fn split_except<T>(v: &mut Vec<T>, from: usize, kept: &[usize]) -> Vec<T> {
    let from = from.min(v.len());
    if kept.is_empty() {
        return v.split_off(from);
    }
    let tail = v.split_off(from);
    let mut out = Vec::with_capacity(tail.len());
    for (i, x) in tail.into_iter().enumerate() {
        if kept.binary_search(&(from + i)).is_ok() {
            v.push(x);
        } else {
            out.push(x);
        }
    }
    out
}

/// Where the position `i` of a journal stands once its entries from `from` on but those at the
/// positions `kept` are gone.
fn moved(i: u32, from: u32, kept: &[usize]) -> u32 {
    if i <= from {
        return i;
    }
    from + kept.partition_point(|&k| (k as u32) < i) as u32
}

impl<'a> Worker<'a> {
    /// Opens the cache of the source arguments of `lists`, for the attempts that apply them.
    pub(super) fn arg_cache_open(&mut self, lists: &[super::apply::ArgList]) {
        let mut args = crate::intern::FxMap::default();
        for l in lists {
            for a in &l.args {
                if let super::apply::ArgSrc::Ast(e) = *a {
                    args.insert(e, None);
                }
            }
        }
        self.attempt_caches.push(ArgCache { seq: self.attempts.seq, file: self.env.file, args, attempt: None });
    }

    /// The attempt `m` over the cached arguments begins.
    pub(super) fn arg_cache_attempt(&mut self, m: Mark) {
        if let Some(c) = self.attempt_caches.last_mut() {
            c.attempt = Some(m);
        }
    }

    pub(super) fn arg_cache_close(&mut self) {
        self.attempt_caches.pop();
    }

    /// The typing of the argument `e` an earlier attempt cached, if any.
    #[inline]
    pub(super) fn cached_arg(&self, e: crate::ast::ExprId) -> Option<(crate::tir::TExprId, crate::types::TypeId)> {
        if self.attempt_caches.is_empty() {
            return None;
        }
        self.cached_arg_slow(e)
    }

    #[inline(never)]
    fn cached_arg_slow(&self, e: crate::ast::ExprId) -> Option<(crate::tir::TExprId, crate::types::TypeId)> {
        let file = self.env.file;
        self.attempt_caches.iter().rev().find_map(|c| if c.file == file { c.args.get(&e).copied().flatten() } else { None })
    }

    /// The typing of the argument `e` begins: where a cache holds it, what the typing writes is
    /// the retained argument's, not the attempt's (`promote_end`).
    #[inline]
    pub(super) fn arg_typing_begin(&self, e: crate::ast::ExprId) -> Option<PromoteMark> {
        if self.attempt_caches.is_empty() || self.attempts.open == 0 {
            return None;
        }
        let file = self.env.file;
        let c = self.attempt_caches.iter().rev().find(|c| c.file == file && c.args.contains_key(&e))?;
        Some(self.promote_mark(Keep::Since(c.seq), false))
    }

    /// Caches the typing of the argument `e` where a cache holds it and the attempt under way
    /// has reported no error (`ProtoTypes` 488: whatever reported it).
    /// Whether it cached it.
    #[inline]
    pub(super) fn cache_arg(&mut self, e: crate::ast::ExprId, typed: (crate::tir::TExprId, crate::types::TypeId)) -> bool {
        if self.attempt_caches.is_empty() {
            return false;
        }
        self.cache_arg_slow(e, typed)
    }

    #[inline(never)]
    fn cache_arg_slow(&mut self, e: crate::ast::ExprId, typed: (crate::tir::TExprId, crate::types::TypeId)) -> bool {
        let file = self.env.file;
        let Some(k) = self.attempt_caches.iter().rposition(|c| c.file == file && c.args.contains_key(&e)) else { return false };
        if self.literal_arg(e) {
            return false;
        }
        // A late error is one all the same (scalac reports it from the kept tree, which a cached
        // typing would keep without it: a retraction drops a cached argument's diagnostics).
        let clean = match self.attempt_caches[k].attempt {
            Some(m) => !self.attempt_failed(&m) && !self.attempts.late.iter().any(|&i| i >= m.diags),
            None => true,
        };
        if clean {
            self.attempt_caches[k].args.insert(e, Some(typed));
        }
        clean
    }

    /// Whether the argument `e` is a literal, or a tuple of literals, in parentheses or not: each
    /// attempt types it against its own formal and none caches it. Its typing is the formal's
    /// (dotty's `typedNumber` makes `1` a `Double` literal against a `Double`), which a later
    /// candidate's formal may not take where the literal typed alone fits it; where dotty applies
    /// several alternatives to one argument it resolves the overload on the arguments typed alone
    /// (`typedArgs`), so that the alternative it applies adapts `1` to its own formal. Typing a
    /// literal again makes nothing an attempt would make twice.
    fn literal_arg(&self, e: crate::ast::ExprId) -> bool {
        use crate::ast::Expr;
        let ast = self.cur_ast();
        match ast.expr(e) {
            Expr::IntLit(_) | Expr::LongLit(_) | Expr::DoubleLit(_) | Expr::DecimalLit(_) | Expr::FloatLit(_) | Expr::BoolLit(_) | Expr::CharLit(_) | Expr::StringLit(_) => true,
            Expr::Parens(inner) => self.literal_arg(inner),
            Expr::Tuple(l) => ast.expr_list(l).iter().all(|&x| self.literal_arg(x)),
            _ => false,
        }
    }

    /// Whether a plain inline call of `sym` typed now waits for the later expansion phase:
    /// not a transparent method's, not one typed inside an expansion
    /// or a retained inline body, the method inlined into itself (whose nested calls teq still
    /// expands as it walks; dotty's `ForceInline` positions, an `inline if`'s condition, are
    /// among them), and only under an attempt or an item, which flush it.
    pub(super) fn defers_plain_inline(&mut self, sym: crate::types::SymId) -> bool {
        let at = plain_expansion();
        if at == PlainExpansion::Typing
            || self.inline.depth > 0
            || self.inline.retained > 0
            || self.inline.checking > 0
            || self.quote.level > 0
            || self.quote.macro_depth > 0
        {
            return false;
        }
        let open = self.attempts.open != 0 || (at == PlainExpansion::Item && !self.var_frames.is_empty());
        // A local inline method's call keeps its expansion here: what it checks at the call (a
        // forward reference, scalac's E039 of the typer) is the typer's.
        open && self.syms.sym(sym).mods & crate::ast::mods::TRANSPARENT == 0
            && self.syms.sym(sym).owner != crate::symbols::Owner::Local
            && self.intrinsic_of(sym).is_none()
    }

    /// Whether an error an inline intrinsic reports now is one scalac reports only in its
    /// `Inlining` phase: inside the expansion of a transparent method at typing time, under an
    /// attempt, with the later phase on.
    pub(super) fn reports_late(&self) -> bool {
        plain_expansion() != PlainExpansion::Typing
            && self.attempts.open != 0
            && self.attempts.flushing == 0
            && self.inline.sites.iter().any(|s| self.syms.sym(s.callee).mods & crate::ast::mods::TRANSPARENT != 0)
    }

    /// The diagnostic just reported is late (`reports_late`).
    pub(super) fn note_late(&mut self) {
        if let Some(i) = self.diags.items.len().checked_sub(1) {
            self.attempts.late.push(i as u32);
        }
    }

    /// The first error from `from` on that is no late one.
    pub(super) fn first_error_since(&self, from: usize) -> Option<&crate::source::Diagnostic> {
        self.diags.items.iter().enumerate().skip(from).find(|(i, d)| !d.is_warning && !self.attempts.late.contains(&(*i as u32))).map(|(_, d)| d)
    }

    /// What the expansion of the pending call at `node` reported ahead of the later phase, dropped:
    /// a candidate given up for it, whose expansion is no part of the program.
    pub(super) fn drop_held(&mut self, node: crate::tir::TExprId) {
        if let Some(p) = self.attempts.pending.iter_mut().rev().find(|p| p.node == node) {
            p.held.clear();
        }
    }

    /// The diagnostics from `from` on dropped with the late marks among them: a candidate given up.
    pub(super) fn drop_reported_since(&mut self, from: usize) {
        self.diags_cut_at(from);
        self.diags.items.truncate(from);
    }

    /// The diagnostics from `from` on about to leave the list (dropped, or held for a flush): the
    /// late marks among them go, and no promoted range keeps a position past `from`, which the
    /// next diagnostic would take.
    pub(super) fn diags_cut_at(&mut self, from: usize) {
        if !self.attempts.late.is_empty() {
            self.attempts.late.retain(|&i| (i as usize) < from);
        }
        let from = from as u32;
        for p in self.attempts.promoted.iter_mut() {
            p.diags = (p.diags.0.min(from), p.diags.1.min(from));
        }
    }

    /// `f` typed as a unit of its own, a body typed on demand: not in the flush or the given's
    /// arguments that demanded it, so that its early expansions keep their order and hold
    /// what they report as at the walk.
    pub(super) fn as_own_unit<T>(&mut self, f: impl FnOnce(&mut Self) -> T) -> T {
        let flushing = std::mem::replace(&mut self.attempts.flushing, 0);
        let given_args = std::mem::replace(&mut self.attempts.given_args, 0);
        let result = f(self);
        self.attempts.flushing = flushing;
        self.attempts.given_args = given_args;
        result
    }

    /// Registers the call `node`, typed as a call, for its expansion in the later phase.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn defer_plain_inline(
        &mut self,
        node: crate::tir::TExprId,
        call: &super::apply::MethodCall,
        sig: &std::sync::Arc<crate::symbols::MethodSig>,
        subst: &crate::types::Subst,
        args: &[crate::tir::TExprId],
        arg_types: Vec<(crate::tir::TExprId, crate::types::TypeId)>,
        ret_ty: crate::types::TypeId,
        typed_as: crate::types::TypeId,
        span: crate::source::Span,
        expected: Option<crate::types::TypeId>,
    ) {
        // The types as they are solved now: a later rollback of a probe that forced this typing
        // may reopen a variable they name.
        let owner_subst = call.owner_subst.iter().map(|&(p, t)| (p, self.zonk(t))).collect();
        let call = super::apply::MethodCall { recv: call.recv, sym: call.sym, owner_subst, ext_recv: call.ext_recv, prefix: call.prefix };
        let subst = subst.iter().map(|&(p, t)| (p, self.zonk(t))).collect();
        let arg_types = arg_types.into_iter().map(|(e, t)| (e, self.zonk(t))).collect();
        let ret_ty = self.zonk(ret_ty);
        let expected = expected.map(|t| self.zonk(t));
        let pending = PendingCall {
            call,
            sig: sig.clone(),
            subst,
            args: args.to_vec(),
            arg_types,
            ret_ty,
            span,
            expected,
            env: std::sync::Arc::new(self.env.clone()),
            gadt: self.gadt.clone(),
            nowarn: self.nowarn,
            body_node: self.body_node,
            of_bodies: self.diags.of_bodies,
            owners: self.sites.owners.clone(),
            bodies_in_progress: self.bodies_in_progress.clone(),
            at_end: self.inline.site_at_end,
        };
        let len = self.attempts.pending.len();
        self.note_rare(Rare::Pending, len);
        self.attempts.pending.push(PendingInline { node, typed_as, call: Some(pending), seq: self.attempts.seq, held: Vec::new() });
    }

    /// The later expansion phase: each pending call expanded at its site, in the order the calls
    /// were typed, its node replaced in place by its expansion (as `expand_deferred_inlines`
    /// replaces a quote's call); what an expansion leaves pending is expanded after it.
    pub(super) fn flush_pending_inline(&mut self) {
        self.flush_pending_from(0);
    }

    /// The end of a unit the parallel typer publishes once it is made (a member's body, a class's
    /// `TClass`), which the interpreter may also run at once: the calls it registered from `start`
    /// on expanded first, since a published record is not changed before the merge, at one
    /// worker as at several.
    pub(super) fn flush_pending_since(&mut self, start: usize) {
        if self.attempts.pending.len() > start {
            self.flush_pending_from(start);
        }
    }

    /// The pending calls registered from position `start` on, expanded.
    pub(super) fn flush_pending_from(&mut self, start: usize) {
        self.release_held(start);
        self.attempts.flushing += 1;
        self.flush_pending_now(start);
        self.attempts.flushing -= 1;
        self.fold_conditions_later();
        self.fold_indexes_later();
    }

    fn flush_pending_now(&mut self, start: usize) {
        while self.attempts.pending.len() > start {
            let pending = self.attempts.pending.split_off(start);
            self.pending_cut_at(start);
            for p in pending {
                if let Some(call) = p.call {
                    self.expand_pending(p.node, p.typed_as, call);
                }
            }
        }
    }

    /// The entries of the pending journal from `start` on taken by a unit's flush under an open
    /// attempt: the positions in it that the open attempts and the promoted ranges hold past
    /// `start` come back to it, so that what is registered next is the innermost attempt's and no
    /// range keeps an entry that took a flushed one's place.
    fn pending_cut_at(&mut self, start: usize) {
        let start = start as u32;
        for open in self.attempts.stack.iter_mut() {
            let at = &mut open.at[Rare::Pending as usize];
            if *at != UNTAKEN && *at > start {
                *at = start;
            }
        }
        for p in self.attempts.promoted.iter_mut() {
            let (a, b) = &mut p.rare[Rare::Pending as usize];
            *a = (*a).min(start);
            *b = (*b).min(start);
        }
    }

    /// The pending calls inside the trees of `roots`, the arguments of an expansion run now (a
    /// transparent call's, an intrinsic's such as `requireConst`, a searched inline given's, a
    /// constant narrowing's), expanded first in the order they were typed, after the pending
    /// calls typed before them: the expansion copies an inline parameter's argument and reads its
    /// constant. dotty leaves them inside the copies for its `Inlining`
    /// phase; teq expands each once, before, as it did where every call expanded as typed. What
    /// the expansion reports is held for the flush of the call's unit, where the later phase
    /// reports it (`PendingInline::held`), so that it fails no attempt, as scalac reports it after
    /// typing; in a flush, it is reported in place, late under an attempt. The first error the
    /// expansion of a call in `roots` reported while typing.
    pub(super) fn expand_pending_in(&mut self, roots: &[crate::tir::TExprId]) -> Option<String> {
        self.expand_pending_in_keeping(roots, false)
    }

    /// `expand_pending_in` for a call about to be bound to a temporary (`expand_before_binding`),
    /// whose expansion decides the binding once: where the call was typed before the innermost
    /// open attempt, its expansion and what it reports are that typing's and stay when the
    /// attempt is retracted, as those of the calls typed before it do, so that an overload's
    /// alternative given up leaves it expanded for the next, which neither expands it again nor
    /// loses its error.
    pub(super) fn expand_pending_kept(&mut self, roots: &[crate::tir::TExprId]) {
        self.expand_pending_in_keeping(roots, true);
    }

    fn expand_pending_in_keeping(&mut self, roots: &[crate::tir::TExprId], keep_roots: bool) -> Option<String> {
        let mut live: Vec<crate::tir::TExprId> = self.attempts.pending.iter().filter(|p| p.call.is_some()).map(|p| p.node).collect();
        if live.is_empty() {
            return None;
        }
        live.sort_unstable();
        let mut found: Vec<crate::tir::TExprId> = Vec::new();
        'walk: for &r in roots {
            for e in self.prog.descendants(r) {
                if live.binary_search(&e).is_ok() && !found.contains(&e) {
                    found.push(e);
                    if found.len() == live.len() {
                        break 'walk;
                    }
                }
            }
        }
        if found.is_empty() {
            return None;
        }
        // The calls typed before them that still wait are expanded first, in their order, so that
        // the expansions keep the order the calls were typed in, dotty's within a body (a macro
        // sharing state sees it). Such an expansion is its call's typing's: an attempt begun
        // after the call was typed keeps it when retracted (`Keep::Since`), as it keeps work done
        // on demand. Not in a flush, which expands its unit's calls in their order.
        let mut failed = None;
        let last = if self.attempts.flushing == 0 { self.attempts.pending.iter().rposition(|p| p.call.is_some() && found.contains(&p.node)) } else { None };
        for i in 0..self.attempts.pending.len() {
            let Some(p) = self.attempts.pending.get_mut(i) else { break };
            if p.call.is_none() {
                continue;
            }
            let before = last.is_some_and(|l| i < l) && !found.contains(&p.node);
            if !before && !found.contains(&p.node) {
                continue;
            }
            let (node, typed_as, seq) = (p.node, p.typed_as, p.seq);
            let promote = ((before || keep_roots) && self.attempts.open > seq).then(|| self.promote_mark(Keep::Since(seq), true));
            let Some(call) = self.consume_pending(node) else { continue };
            let diags = self.diags.items.len();
            let typing = self.attempts.flushing == 0;
            self.attempts.flushing += 1;
            self.expand_pending(node, typed_as, call);
            self.attempts.flushing -= 1;
            if typing {
                // What it reports waits for the flush of the call's unit (`PendingInline::held`).
                if self.diags.items.len() > diags {
                    self.diags_cut_at(diags);
                    let held = self.diags.items.split_off(diags);
                    if failed.is_none() && !before {
                        failed = held.iter().find(|d| !d.is_warning).map(|d| d.msg.clone());
                    }
                    if let Some(p) = self.attempts.pending.iter_mut().rev().find(|p| p.node == node) {
                        p.held = held;
                    }
                }
            } else if self.attempts.open != 0 {
                for j in diags..self.diags.items.len() {
                    if !self.diags.items[j].is_warning {
                        self.attempts.late.push(j as u32);
                    }
                }
            }
            self.promote_end(promote);
        }
        // An index they leave a constant is dropped before the expansion copies it.
        self.fold_indexes_later();
        failed
    }

    /// Whether `te` is a call still pending.
    pub(super) fn is_pending_call(&self, te: crate::tir::TExprId) -> bool {
        !self.attempts.pending.is_empty() && self.attempts.pending.iter().rev().any(|p| p.node == te && p.call.is_some())
    }

    /// The type the typing gave `te`, a call still pending.
    pub(super) fn pending_type(&self, te: crate::tir::TExprId) -> Option<crate::types::TypeId> {
        if self.attempts.pending.is_empty() {
            return None;
        }
        self.attempts.pending.iter().rev().find(|p| p.node == te && p.call.is_some()).map(|p| p.typed_as)
    }

    /// Whether the tree of `te` holds a call still pending.
    pub(super) fn holds_pending_call(&self, te: crate::tir::TExprId) -> bool {
        if self.attempts.pending.iter().all(|p| p.call.is_none()) {
            return false;
        }
        self.prog.descendants(te).any(|e| self.is_pending_call(e))
    }

    /// The `if`s whose conditions held pending calls (`mark_taken_branch`), folded once their
    /// calls are expanded: the branch a constant condition takes marked for the concatenation.
    fn fold_conditions_later(&mut self) {
        if self.attempts.fold_later.is_empty() {
            return;
        }
        let later = std::mem::take(&mut self.attempts.fold_later);
        for (cond, te) in later {
            if self.holds_pending_call(cond) {
                self.attempts.fold_later.push((cond, te));
            } else if let Some(first) = self.folded_condition(cond) {
                self.prog.mark_taken(te, first);
            }
        }
    }

    /// What the expansions ahead of the later phase of the calls registered from `start` on
    /// reported while typing (`PendingInline::held`), reported at their unit's flush, late under
    /// an attempt open.
    fn release_held(&mut self, start: usize) {
        for i in start..self.attempts.pending.len() {
            if self.attempts.pending[i].held.is_empty() || self.attempts.pending[i].call.is_some() {
                continue;
            }
            for d in std::mem::take(&mut self.attempts.pending[i].held) {
                if self.attempts.open != 0 && !d.is_warning {
                    self.attempts.late.push(self.diags.items.len() as u32);
                }
                self.diags.items.push(d);
            }
        }
    }

    /// The tuple indexes over pending calls (`index_later`) whose calls are expanded: one the
    /// expansions leave a constant is dropped, the block left the element's read, as an index
    /// typed a constant is; one they leave an effect to is evaluated before the read.
    fn fold_indexes_later(&mut self) {
        if self.attempts.index_later.is_empty() {
            return;
        }
        let later = std::mem::take(&mut self.attempts.index_later);
        for (index, te, elem) in later {
            if self.holds_pending_call(index) {
                self.attempts.index_later.push((index, te, elem));
            } else if self.fold_constant_as_typed(index).is_some() {
                let value = self.prog.expr(elem);
                self.rewrite_expr(te, value);
            }
        }
    }

    /// Takes the pending call at `node` out of the queue for its expansion ahead of the later
    /// phase, journaled while an attempt is open: a retraction puts the call back and the node's
    /// value with it (a cached argument's call, which the next attempt finds pending again).
    fn consume_pending(&mut self, node: crate::tir::TExprId) -> Option<PendingCall> {
        let call = self.attempts.pending.iter_mut().rev().find(|p| p.node == node)?.call.take()?;
        if self.attempts.open != 0 {
            let len = self.attempts.rewrites.len();
            self.note_rare(Rare::Rewrites, len);
            self.attempts.rewrites.push(Rewrite::Consumed(node, Some(Box::new(call.clone())), Vec::new()));
        }
        Some(call)
    }

    /// The pending call at `node` expanded, with what it captured of its site restored, the node
    /// replaced in place by the expansion.
    fn expand_pending(&mut self, node: crate::tir::TExprId, typed_as: crate::types::TypeId, p: PendingCall) {
        let gadt = std::mem::replace(&mut self.gadt, p.gadt);
        let nowarn = std::mem::replace(&mut self.nowarn, p.nowarn);
        let body_node = std::mem::replace(&mut self.body_node, p.body_node);
        let of_bodies = std::mem::replace(&mut self.diags.of_bodies, p.of_bodies);
        let owners = std::mem::replace(&mut self.sites.owners, p.owners);
        let bodies = std::mem::replace(&mut self.bodies_in_progress, p.bodies_in_progress);
        let arg_types: Vec<_> = p.arg_types.into_iter().map(|(e, t)| (e, self.zonk(t))).collect();
        let arg_types = std::mem::replace(&mut self.inline.arg_types, arg_types);
        let (mut call, sig, args, span) = (p.call, p.sig, p.args, p.span);
        call.owner_subst = call.owner_subst.iter().map(|&(q, t)| (q, self.zonk(t))).collect();
        let subst: crate::types::Subst = p.subst.iter().map(|&(q, t)| (q, self.zonk(t))).collect();
        let ret_ty = self.zonk(p.ret_ty);
        let expected = p.expected.map(|t| self.zonk(t));
        let env = std::sync::Arc::try_unwrap(p.env).unwrap_or_else(|shared| (*shared).clone());
        let at_end = std::mem::replace(&mut self.inline.site_at_end, p.at_end);
        let expanded = self.with_env(env, |t| t.expand_inline(&call, &sig, &subst, &args, ret_ty, span, expected));
        self.inline.site_at_end = at_end;
        self.inline.arg_types = arg_types;
        self.gadt = gadt;
        self.nowarn = nowarn;
        self.body_node = body_node;
        self.diags.of_bodies = of_bodies;
        self.sites.owners = owners;
        self.bodies_in_progress = bodies;
        if flush_trace() {
            let name = self.name_str(self.syms.sym(call.sym).name);
            eprintln!("flush {} at {:?}: {:?}", name, span, expanded.map(|(e, _)| self.prog.expr(e)));
        }
        if let Some((expanded, ty)) = expanded {
            if expanded != node {
                // A type the typing gave the call's node after it was typed (an `Int` widened to
                // `Double` in place, `widen_numeric`) stays, as it stays on an expansion's node.
                // A numeric widening is the node retyped (an `Int` node typed `Double`); another
                // type the typing gave it (an ascription to `Any`, which boxes the value as its own
                // class) stands over the expansion, which keeps its own, as over a narrowed literal.
                let changed = match self.prog.type_of(node) {
                    Some(t) if t != typed_as && self.zonk(t) != self.zonk(typed_as) => Some(t),
                    _ => None,
                };
                match changed {
                    Some(t) if self.is_numeric(t).is_none() || self.is_numeric(ty).is_none() => {
                        self.rewrite_expr(node, crate::tir::TExpr::Block(crate::ast::ListRef::EMPTY, expanded));
                        self.retype_expr(node, t);
                    }
                    _ => {
                        let value = self.prog.expr(expanded);
                        self.rewrite_expr(node, value);
                        self.retype_expr(node, changed.unwrap_or(ty));
                        self.move_expansion_marks(expanded, node);
                        if self.attempts.tracking_moved != 0 {
                            self.attempts.moved.push(expanded);
                        }
                    }
                }
                self.prog.copy_chain_marks(expanded, node);
                if self.capturing() {
                    self.capture_moved(expanded, node);
                }
            }
        }
    }

    /// What the expansion's root `from` is recorded as, given to the call's node `to` that takes its
    /// value, as `copy_expr` gives a copy: the expansion's record (by which the JavaScript emitter
    /// shares one function among the expansions of one shape, docs/TARGETS.md), its leaf, widening
    /// and opacity marks, its evaluation and the language server's record of it.
    fn move_expansion_marks(&mut self, from: crate::tir::TExprId, to: crate::tir::TExprId) {
        if self.prog.is_expansion(from) {
            match self.prog.expansions.get(&from) {
                Some(&x) => self.prog.note_expansion(to, x),
                None => self.prog.mark_expansion(to),
            }
        }
        if self.prog.is_leaf(from) {
            self.prog.note_leaf(to);
        }
        if self.prog.is_widened(from) {
            self.prog.mark_widened(to);
        }
        if self.prog.is_opaque(from) {
            self.prog.mark_opaque(to);
        }
        if self.inline.evaluated.contains_key(&from) {
            self.inline.evaluated.insert(to, ());
        }
        if let Some(ix) = self.index.as_mut() {
            if let Some(&sym) = ix.expansions.get(&from) {
                ix.expansions.insert(to, sym);
            }
            if ix.conversions.contains_key(&from) {
                ix.conversions.insert(to, ());
            }
        }
    }

    /// Drops the diagnostics from `from` on: through `discard_diagnostics` where one counted in
    /// the loader's census of unsupported bodies, by a truncation otherwise.
    fn drop_diagnostics(&mut self, from: usize) {
        if self.loaded.is_some() && self.diags.items[from..].iter().any(|d| d.msg.starts_with("not supported yet: ")) {
            self.discard_counted(from);
        } else {
            self.diags.items.truncate(from);
        }
    }

    /// Begins an attempt.
    #[inline]
    pub(super) fn attempt(&mut self) -> Mark {
        self.attempts.seq += 1;
        let seq = self.attempts.seq;
        let m = Mark {
            seq,
            outer: self.attempts.open,
            trail: self.trail.len() as u32,
            vars: self.tvars.len() as u32,
            syms: self.syms.syms.len() as u32,
            diags: self.diags.items.len() as u32,
            error_nodes: self.error_nodes,
            index: self.index_mark(),
        };
        self.attempts.open = seq;
        self.attempts.stack.push(Open { seq, at: [UNTAKEN; RARE] });
        m
    }

    /// The attempt stands: what it wrote is the enclosing attempt's.
    #[inline]
    pub(super) fn close(&mut self, m: Mark) {
        self.end_attempt(&m);
    }

    /// A write to a rarely written journal of length `len` is about to be made: the open
    /// attempts that have not seen one take their position there.
    #[inline]
    pub(super) fn note_rare(&mut self, j: Rare, len: usize) {
        for open in self.attempts.stack.iter_mut().rev() {
            if open.at[j as usize] != UNTAKEN {
                break;
            }
            open.at[j as usize] = len as u32;
        }
    }

    /// A macro's info message is about to be held (`Worker::infos`).
    pub fn note_info(&mut self) {
        if let Some(len) = self.infos.as_ref().map(|i| i.len()) {
            self.note_rare(Rare::Infos, len);
        }
    }

    /// Whether the attempt reported anything, a warning included, past `m` (what work done on
    /// demand for an older definition reported aside).
    pub(super) fn attempt_reported(&self, m: &Mark) -> bool {
        let from = m.diags as usize;
        if self.diags.items.len() <= from {
            return false;
        }
        if self.attempts.promoted.is_empty() {
            return true;
        }
        let kept = self.promoted_positions(m, |p| p.diags, m.diags);
        (from..self.diags.items.len()).any(|i| kept.binary_search(&i).is_err())
    }

    /// Whether the attempt reported an error, or typed an error node, past `m`: what work done
    /// on demand for an older definition reported is that definition's, not the attempt's.
    pub(super) fn attempt_failed(&self, m: &Mark) -> bool {
        self.error_nodes > m.error_nodes || self.attempt_reported_error(m)
    }

    /// Whether the attempt reported an error of its own past `m`, the error nodes it typed aside:
    /// the test of a library val's typing against scalac's type (`type_inferred_val`), as on
    /// master, a type of the body the reader could not convert being an error node that reports
    /// nothing (`TyExpr::Error`).
    pub(super) fn attempt_reported_error(&self, m: &Mark) -> bool {
        let from = m.diags as usize;
        if self.attempts.promoted.is_empty() && self.attempts.late.is_empty() {
            return self.diags.items.get(from..).is_some_and(|items| items.iter().any(|d| !d.is_warning));
        }
        let kept = self.promoted_positions(m, |p| p.diags, m.diags);
        self.diags.items.iter().enumerate().skip(from).any(|(i, d)| !d.is_warning && kept.binary_search(&i).is_err() && !self.attempts.late.contains(&(i as u32)))
    }

    /// The attempt goes: what it wrote is taken back, but for what is not its own.
    pub(super) fn retract(&mut self, m: Mark) {
        self.retract_trail(m.trail as usize, m.vars as usize);
        self.error_nodes = m.error_nodes;
        let at = self.attempts.stack.last().map_or([UNTAKEN; RARE], |o| o.at);
        if self.keeps_promoted(&m, &at) {
            self.retract_keeping_promoted(&m, &at);
        } else {
            if !self.attempts.late.is_empty() {
                self.attempts.late.retain(|&i| i < m.diags);
            }
            if self.diags.items.len() > m.diags as usize {
                self.drop_diagnostics(m.diags as usize);
            }
            self.index_drop(m.index);
            if at.iter().any(|&a| a != UNTAKEN) {
                let none: &[usize] = &[];
                self.cut_rare(&at, &[none, none, none, none, none]);
            }
            self.drop_promoted_past(&m, &at);
        }
        self.end_attempt(&m);
    }

    /// The attempt ends with what it reported kept and its constraints undone: a failure that is
    /// reported where it happened and stops what was being tried (an inline match's guard whose
    /// expansion failed, `reduce_inline_match`).
    pub(super) fn close_without_constraints(&mut self, m: Mark) {
        self.retract_trail(m.trail as usize, m.vars as usize);
        self.end_attempt(&m);
    }

    /// The attempt is taken out whole, for `restore` to put back: dotty's failed state kept
    /// aside while another attempt is tried, then committed after all.
    pub(super) fn set_aside(&mut self, m: Mark) -> SetAside {
        let redo = self.set_aside_trail(m.trail as usize, m.vars as usize);
        let error_nodes = self.error_nodes - m.error_nodes;
        self.error_nodes = m.error_nodes;
        let at = self.attempts.stack.last().map_or([UNTAKEN; RARE], |o| o.at);
        let keeps = self.keeps_promoted(&m, &at);
        let kept_diags = if keeps { self.promoted_positions(&m, |p| p.diags, m.diags) } else { Vec::new() };
        let kept_records = if keeps { self.promoted_positions(&m, |p| p.records, m.index.records() as u32) } else { Vec::new() };
        let kept_rare = if keeps { self.promoted_rare(&m, &at) } else { Default::default() };
        let late = self.split_late(m.diags, &kept_diags);
        let diags = split_except(&mut self.diags.items, m.diags as usize, &kept_diags);
        let index = self.index_take_except(m.index, &kept_records);
        let (infos, matches, bounds, rewrites, pending) = self.cut_rare(&at, &[kept_rare[0].as_slice(), kept_rare[1].as_slice(), kept_rare[2].as_slice(), kept_rare[3].as_slice(), kept_rare[4].as_slice()]);
        if keeps {
            self.relocate_promoted(&m, &at, &kept_diags, &kept_records, &kept_rare);
        } else {
            self.drop_promoted_past(&m, &at);
        }
        self.end_attempt(&m);
        SetAside { redo, diags, late, index, error_nodes, infos, matches, bounds, rewrites, pending }
    }

    /// The late diagnostics (`reports_late`) once those from `from` on but the ones at the
    /// positions `kept` are taken out: a kept one's position where it stands then, and the
    /// positions of the taken ones among the taken, which `restore` puts back late.
    fn split_late(&mut self, from: u32, kept: &[usize]) -> Vec<u32> {
        if self.attempts.late.is_empty() {
            return Vec::new();
        }
        let mut aside = Vec::new();
        let late = std::mem::take(&mut self.attempts.late);
        for i in late {
            if i < from {
                self.attempts.late.push(i);
            } else if kept.binary_search(&(i as usize)).is_ok() {
                self.attempts.late.push(moved(i, from, kept));
            } else {
                aside.push(i - from - kept.partition_point(|&k| (k as u32) < i) as u32);
            }
        }
        aside
    }

    /// `set_aside` of the journals the caller does not take out itself: the trail, the
    /// diagnostics and the index records stay where they are, for the caller (a member's retry
    /// on the qualifier, which reads its typings' positions in them), and the rarely written
    /// journals, the late errors' positions (relative to the attempt's first diagnostic, where the
    /// caller puts its diagnostics back) and the error nodes go aside. What promoted work kept
    /// (a cached argument's typing, `retained_typing_begin`) stays.
    pub(super) fn set_aside_journals(&mut self, m: Mark) -> JournalsAside {
        let error_nodes = self.error_nodes - m.error_nodes;
        self.error_nodes = m.error_nodes;
        let at = self.attempts.stack.last().map_or([UNTAKEN; RARE], |o| o.at);
        let keeps = self.keeps_promoted(&m, &at);
        let kept_rare = if keeps { self.promoted_rare(&m, &at) } else { Default::default() };
        let late = self.split_late(m.diags, &[]);
        let (infos, matches, bounds, rewrites, pending) = self.cut_rare(&at, &[kept_rare[0].as_slice(), kept_rare[1].as_slice(), kept_rare[2].as_slice(), kept_rare[3].as_slice(), kept_rare[4].as_slice()]);
        if keeps {
            self.relocate_promoted(&m, &at, &[], &[], &kept_rare);
        } else {
            self.drop_promoted_past(&m, &at);
        }
        self.end_attempt(&m);
        JournalsAside { diags_at: m.diags, late, error_nodes, infos, matches, bounds, rewrites, pending }
    }

    /// Puts back what `set_aside_journals` took out, the caller having put the attempt's
    /// diagnostics back where they began.
    pub(super) fn restore_journals(&mut self, s: JournalsAside) {
        self.attempts.late.extend(s.late.iter().map(|&i| s.diags_at + i));
        self.error_nodes += s.error_nodes;
        self.restore_rare(s.infos, s.matches, s.bounds, s.rewrites, s.pending);
    }

    /// The given the prefix resolved for the zonked `target`, or for the parameter `param` where its
    /// target was open (`Attempts::inferred`), taken by the application of `sym` it is for.
    pub(super) fn inferred_hit(&mut self, sym: crate::types::SymId, target: crate::types::TypeId, param: crate::types::SymId) -> Option<(crate::tir::TExprId, crate::types::TypeId)> {
        if self.attempts.inferred.is_empty() || self.attempts.inferred_for != Some((sym, self.app_depth)) {
            return None;
        }
        let target = self.zonk(target);
        // A given the prefix found for an open target is its parameter's: the application's target
        // takes the variables' instances the search fixed.
        let i = match self.attempts.inferred.iter().position(|&(t, _, _, _, _)| t == target) {
            Some(i) => i,
            None => self.attempts.inferred.iter().position(|&(_, p, _, _, _)| p == Some(param))?,
        };
        let (_, open, te, ty, aside) = self.attempts.inferred.remove(i);
        self.restore(aside);
        if open.is_some() {
            self.is_sub(ty, target);
        }
        Some((te, ty))
    }

    /// The typing of an argument a member's application under the attempt `m` caches for its
    /// retry begins: what it writes of the rarely written journals is kept through the set-aside
    /// of `m` and the retractions of the retry's tries (`promote_end` where it is clean).
    pub(super) fn retained_typing_begin(&self, m: &Mark) -> PromoteMark {
        self.promote_mark(Keep::Since(m.seq - 1), false)
    }


    /// Puts back what `set_aside` took out, into the attempt open now.
    pub(super) fn restore(&mut self, s: SetAside) {
        self.replay(s.redo);
        let base = self.diags.items.len() as u32;
        self.attempts.late.extend(s.late.iter().map(|&i| base + i));
        self.diags.items.extend(s.diags);
        self.error_nodes += s.error_nodes;
        self.index_put_back(s.index);
        self.restore_rare(s.infos, s.matches, s.bounds, s.rewrites, s.pending);
    }

    fn restore_rare(&mut self, infos: Vec<InfoMessage>, matches: Vec<DeferredMatch>, bounds: Vec<DeferredBound>, rewrites: Vec<Rewrite>, pending: Vec<PendingInline>) {
        if !infos.is_empty() {
            if let Some(len) = self.infos.as_ref().map(|i| i.len()) {
                self.note_rare(Rare::Infos, len);
                self.infos.as_mut().unwrap().extend(infos);
            }
        }
        if !matches.is_empty() {
            self.note_rare(Rare::DeferredMatches, self.deferred_matches.len());
            self.deferred_matches.extend(matches);
        }
        if !bounds.is_empty() {
            self.note_rare(Rare::DeferredBounds, self.deferred_bounds.len());
            self.deferred_bounds.extend(bounds);
        }
        if !pending.is_empty() {
            self.note_rare(Rare::Pending, self.attempts.pending.len());
            self.attempts.pending.extend(pending);
        }
        for r in rewrites {
            match r {
                Rewrite::Expr(te, value) => self.rewrite_expr(te, value),
                Rewrite::Type(te, ty) => {
                    self.journal_type(te);
                    self.prog.restore_type(te, ty);
                }
                Rewrite::Consumed(node, _, held) => {
                    self.consume_pending(node);
                    if let Some(p) = self.attempts.pending.iter_mut().rev().find(|p| p.node == node) {
                        p.held = held;
                    }
                }
            }
        }
    }

    /// Rewrites the node `te` in place, its prior value journaled while an attempt is open.
    pub(super) fn rewrite_expr(&mut self, te: crate::tir::TExprId, value: crate::tir::TExpr) {
        if self.attempts.open != 0 {
            let len = self.attempts.rewrites.len();
            self.note_rare(Rare::Rewrites, len);
            let prior = self.prog.expr(te);
            self.attempts.rewrites.push(Rewrite::Expr(te, prior));
        }
        self.prog.exprs[te.idx()] = value;
    }

    /// Records the type of an existing node `te` as `ty` in place, its prior type journaled while
    /// an attempt is open (a conversion candidate's numeric widening of its receiver).
    pub(super) fn retype_expr(&mut self, te: crate::tir::TExprId, ty: crate::types::TypeId) {
        self.journal_type(te);
        self.prog.set_type(te, ty);
    }

    fn journal_type(&mut self, te: crate::tir::TExprId) {
        if self.attempts.open != 0 && self.prog.record_types {
            let len = self.attempts.rewrites.len();
            self.note_rare(Rare::Rewrites, len);
            let prior = self.prog.type_or_aside(te);
            self.attempts.rewrites.push(Rewrite::Type(te, prior));
        }
    }

    /// Work done on demand for `def` begins: what it writes is the definition's. A library
    /// body's diagnostics (a definition read from TASTy, which scalac typed) are teq's own
    /// limits, not the program's: they stay the attempt's that demanded the body, as on master,
    /// whose retries dropped them with what they gave up.
    #[inline]
    pub(super) fn promote_begin(&mut self, def: crate::types::SymId) -> Option<PromoteMark> {
        if self.attempts.open == 0 {
            return None;
        }
        self.attempts.demanded = self.attempts.demanded.wrapping_add(1);
        // A definition a library body makes as it is typed (a member of its anonymous class) is
        // the library's as well: what its typing reports stays the attempt's, as a library
        // member's does, where scalac typed the body (cats' `catsStdInstancesForStream`).
        let library = self.is_library_member(def) || self.in_jar(self.syms.sym(def).file);
        let diags_kept = !library;
        // A definition of the std or a library entered on demand inside an attempt is no code
        // of the attempt's, and outlives it: its work is kept by every attempt (a macro's run
        // that types `ArrayBuffer.empty` and fails keeps its expanded `emptyBuffer`).
        let made = if library || self.source(self.syms.sym(def).file).is_std { 0 } else { def.0 + 1 };
        Some(self.promote_mark(Keep::Made(made), diags_kept))
    }

    fn promote_mark(&self, keep: Keep, diags_kept: bool) -> PromoteMark {
        PromoteMark {
            keep,
            diags_kept,
            diags: self.diags.items.len() as u32,
            records: self.index_mark().records() as u32,
            marks: self.unused.journal_len(),
            rare: self.rare_lens(),
        }
    }

    /// Work done on demand ends: its entries are kept by the retraction of every open attempt
    /// that began after its definition was made.
    #[inline]
    pub(super) fn promote_end(&mut self, p: Option<PromoteMark>) {
        let Some(p) = p else { return };
        // Its unused-import marks are settled, out of every attempt's reach (as a body typed on
        // demand settles its own, `check.rs`).
        if self.unused.on() {
            self.unused.settle_since(p.marks);
        }
        let lens = self.rare_lens();
        let diags_end = if p.diags_kept { self.diags.items.len() as u32 } else { p.diags };
        let promoted = Promoted {
            keep: p.keep,
            diags: (p.diags, diags_end),
            records: (p.records, self.index_mark().records() as u32),
            rare: std::array::from_fn(|j| (p.rare[j], lens[j])),
        };
        let wrote = promoted.diags.0 < promoted.diags.1
            || promoted.records.0 < promoted.records.1
            || promoted.rare.iter().any(|&(a, b)| a < b);
        if wrote {
            self.attempts.promoted.push(promoted);
        }
    }

    fn rare_lens(&self) -> [u32; RARE] {
        [
            self.infos.as_ref().map_or(0, |i| i.len() as u32),
            self.deferred_matches.len() as u32,
            self.deferred_bounds.len() as u32,
            self.attempts.rewrites.len() as u32,
            self.attempts.pending.len() as u32,
        ]
    }

    /// The attempt `m` is over: it leaves the stack, and with the outermost one the promoted
    /// ranges are no attempt's to keep any more.
    #[inline]
    fn end_attempt(&mut self, m: &Mark) {
        debug_assert!(self.attempts.stack.last().map(|o| o.seq) == Some(m.seq), "an attempt ends that is not the innermost open one");
        self.attempts.stack.pop();
        self.attempts.open = m.outer;
        if m.outer == 0 {
            if !self.attempts.promoted.is_empty() {
                self.attempts.promoted.clear();
            }
            if !self.attempts.late.is_empty() {
                self.attempts.late.clear();
            }
            if !self.attempts.rewrites.is_empty() {
                self.attempts.rewrites.clear();
            }
            if !self.attempts.pending.is_empty() && (plain_expansion() == PlainExpansion::Commit || self.var_frames.is_empty()) {
                self.flush_pending_inline();
            }
        }
    }

    /// Pops the trail to `mark`, undoing each entry but the instances of the variables made past
    /// `vars`: an instance the attempt gave its own variable is permanent, as dotty's owning
    /// state makes it, while a bound the attempt put on one goes with the attempt's constraint
    /// (dotty keeps it in the dropped state alone), so that a variable the attempt left open is
    /// open and unbounded for whatever scans the variables made since (`solve_application_vars`).
    fn retract_trail(&mut self, mark: usize, vars: usize) {
        while self.trail.len() > mark {
            match self.trail.pop().unwrap() {
                Undo::Inst(v) => {
                    if !self.tvars.made_since(v, vars) {
                        self.tvars[v].inst = None;
                    }
                }
                Undo::Lower(v) => {
                    self.tvars[v].lower.pop();
                }
                Undo::Upper(v) => {
                    self.tvars[v].upper.pop();
                }
            }
        }
    }

    /// `retract_trail` that returns what it undid, for `replay`.
    fn set_aside_trail(&mut self, mark: usize, vars: usize) -> Vec<Redo> {
        let mut undone = Vec::with_capacity(self.trail.len().saturating_sub(mark));
        while self.trail.len() > mark {
            undone.push(match self.trail.pop().unwrap() {
                Undo::Inst(v) => {
                    if self.tvars.made_since(v, vars) {
                        continue;
                    }
                    Redo::Inst(v, self.tvars[v].inst.take().unwrap())
                }
                Undo::Lower(v) => Redo::Lower(v, self.tvars[v].lower.pop().unwrap()),
                Undo::Upper(v) => Redo::Upper(v, self.tvars[v].upper.pop().unwrap()),
            });
        }
        undone
    }

    /// Takes each rarely written journal from the attempt's position in it on out, but the
    /// positions `kept`: what it took, by journal.
    #[allow(clippy::type_complexity)]
    fn cut_rare(
        &mut self,
        at: &[u32; RARE],
        kept: &[&[usize]; RARE],
    ) -> (Vec<InfoMessage>, Vec<DeferredMatch>, Vec<DeferredBound>, Vec<Rewrite>, Vec<PendingInline>) {
        let infos = match (at[Rare::Infos as usize], self.infos.as_mut()) {
            (a, Some(infos)) if a != UNTAKEN => split_except(infos, a as usize, kept[0]),
            _ => Vec::new(),
        };
        let a = at[Rare::DeferredMatches as usize];
        let matches = if a != UNTAKEN { split_except(&mut self.deferred_matches, a as usize, kept[1]) } else { Vec::new() };
        let a = at[Rare::DeferredBounds as usize];
        let bounds = if a != UNTAKEN { split_except(&mut self.deferred_bounds, a as usize, kept[2]) } else { Vec::new() };
        // A rewritten node gets its prior value back, newest first; what it was given is kept for
        // `restore`.
        let a = at[Rare::Rewrites as usize];
        let mut rewrites = Vec::new();
        if a != UNTAKEN {
            let undone = split_except(&mut self.attempts.rewrites, a as usize, kept[3]);
            for r in undone.into_iter().rev() {
                match r {
                    Rewrite::Expr(te, prior) => {
                        rewrites.push(Rewrite::Expr(te, self.prog.expr(te)));
                        self.prog.exprs[te.idx()] = prior;
                    }
                    Rewrite::Type(te, prior) => {
                        rewrites.push(Rewrite::Type(te, self.prog.type_or_aside(te)));
                        self.prog.restore_type(te, prior);
                    }
                    Rewrite::Consumed(node, call, _) => {
                        let mut held = Vec::new();
                        if let Some(p) = self.attempts.pending.iter_mut().rev().find(|p| p.node == node) {
                            if p.call.is_none() {
                                p.call = call.map(|c| *c);
                                held = std::mem::take(&mut p.held);
                            }
                        }
                        rewrites.push(Rewrite::Consumed(node, None, held));
                    }
                }
            }
            rewrites.reverse();
        }
        let a = at[Rare::Pending as usize];
        let pending = if a != UNTAKEN { split_except(&mut self.attempts.pending, a as usize, kept[4]) } else { Vec::new() };
        (infos, matches, bounds, rewrites, pending)
    }

    /// Whether a promoted range lies past the attempt for a definition made before it began:
    /// one its retraction keeps.
    #[inline]
    fn keeps_promoted(&self, m: &Mark, at: &[u32; RARE]) -> bool {
        !self.attempts.promoted.is_empty() && self.attempts.promoted.iter().any(|p| Self::kept_by(p, m, at))
    }

    fn kept_by(p: &Promoted, m: &Mark, at: &[u32; RARE]) -> bool {
        p.keep.by(m)
            && (p.diags.1 > m.diags
                || p.records.1 > m.index.records() as u32
                || (0..RARE).any(|j| at[j] != UNTAKEN && p.rare[j].1 > at[j]))
    }

    /// The positions from `from` on of a journal that the ranges `range` of the promoted work
    /// kept by `m` hold, ascending.
    fn promoted_positions(&self, m: &Mark, range: impl Fn(&Promoted) -> (u32, u32), from: u32) -> Vec<usize> {
        let mut kept = Vec::new();
        for p in self.attempts.promoted.iter().filter(|p| p.keep.by(m)) {
            let (a, b) = range(p);
            kept.extend((a.max(from) as usize)..b as usize);
        }
        kept.sort_unstable();
        kept.dedup();
        kept
    }

    fn promoted_rare(&self, m: &Mark, at: &[u32; RARE]) -> [Vec<usize>; RARE] {
        std::array::from_fn(|j| if at[j] == UNTAKEN { Vec::new() } else { self.promoted_positions(m, |p| p.rare[j], at[j]) })
    }

    /// Forgets the promoted ranges past `m`: their entries went with the attempt.
    fn drop_promoted_past(&mut self, m: &Mark, at: &[u32; RARE]) {
        if !self.attempts.promoted.is_empty() {
            let records = m.index.records() as u32;
            self.attempts.promoted.retain(|p| {
                p.diags.1 <= m.diags && p.records.1 <= records && (0..RARE).all(|j| at[j] == UNTAKEN || p.rare[j].1 <= at[j])
            });
        }
    }

    /// The promoted ranges kept by `m`, moved to where their entries stand once the attempt's own
    /// are gone; the others past `m` are forgotten.
    fn relocate_promoted(&mut self, m: &Mark, at: &[u32; RARE], kept_diags: &[usize], kept_records: &[usize], kept_rare: &[Vec<usize>; RARE]) {
        let records = m.index.records() as u32;
        let mut out = Vec::with_capacity(self.attempts.promoted.len());
        for p in self.attempts.promoted.iter() {
            let past = p.diags.1 > m.diags || p.records.1 > records || (0..RARE).any(|j| at[j] != UNTAKEN && p.rare[j].1 > at[j]);
            if !past {
                out.push(*p);
            } else if p.keep.by(m) {
                let mv = |(a, b): (u32, u32), from: u32, kept: &[usize]| (moved(a, from, kept), moved(b, from, kept));
                out.push(Promoted {
                    keep: p.keep,
                    diags: mv(p.diags, m.diags, kept_diags),
                    records: mv(p.records, records, kept_records),
                    rare: std::array::from_fn(|j| if at[j] == UNTAKEN { p.rare[j] } else { mv(p.rare[j], at[j], &kept_rare[j]) }),
                });
            }
        }
        self.attempts.promoted = out;
    }

    /// `retract` where work done on demand for older definitions wrote past `m`: its entries
    /// stay, in their order, and the attempt's own go.
    #[cold]
    #[inline(never)]
    fn retract_keeping_promoted(&mut self, m: &Mark, at: &[u32; RARE]) {
        let kept_diags = self.promoted_positions(m, |p| p.diags, m.diags);
        let kept_records = self.promoted_positions(m, |p| p.records, m.index.records() as u32);
        let kept_rare = self.promoted_rare(m, at);
        self.split_late(m.diags, &kept_diags);
        let dropped = split_except(&mut self.diags.items, m.diags as usize, &kept_diags);
        // The census of what the dropped ones counted, as `discard_diagnostics` takes it back.
        let len = self.diags.items.len();
        self.diags.items.extend(dropped);
        self.discard_counted(len);
        self.index_drop_except(m.index, &kept_records);
        self.cut_rare(at, &[kept_rare[0].as_slice(), kept_rare[1].as_slice(), kept_rare[2].as_slice(), kept_rare[3].as_slice(), kept_rare[4].as_slice()]);
        self.relocate_promoted(m, at, &kept_diags, &kept_records, &kept_rare);
    }
}
