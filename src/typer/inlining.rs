//! The inlining phase, dotty's `transform/Inlining.scala`: the plain inline calls the typer left in
//! the typed trees (`Inlines.needsInlining` holds in the typer only for a transparent method or
//! under `Mode.ForceInline`), each kept with what its expansion needs of its site, expanded once
//! the whole program is typed without an error (`Phase.isRunnable`: no phase runs after a typer
//! that reported one), in the order of the units' trees, an argument's calls before the call
//! (`InliningTreeMap.transform` expands a call's children first).
//!
//! A unit's calls are kept where its typing ends (`Worker::end_unit`), past which the parallel
//! typer publishes its records unchanged; the merge carries them to the main worker under the
//! canonical ids, and the phase runs there before the passes that read expansions
//! (`run_deferred_matches`, the contextual parameters' numbering, the final passes). What the
//! interpreter runs before the phase reaches a call (a macro's implementation, an object it reads)
//! has its calls expanded first, its executable form (`Worker::prepare_executable`), as dotty
//! compiles a macro's unit through `Inlining` before the unit that expands the macro.
//!
//! The typed form stays what reflection reads: a member's definition keeps no right-hand side
//! (`FromSymbol`), and an expansion is the tree the phase makes of the call, the call's arguments
//! as typed.

use super::state::{PendingCall, PendingInline};
use super::Worker;
use crate::intern::FxMap;
use crate::source::{DiagOwner, Diagnostic, FileId, Span};
use crate::tir::FunId;
use crate::types::{ClassId, SymId};
use crate::tir::TExprId;
use crate::types::TypeId;

/// Where a kept call stands in the walk of the program's trees: its file, the member whose tree
/// holds it by the member's place (the definitions in the order of the source, as
/// `InliningTreeMap` walks them, the file's top-level vals and defs after its other statements as
/// in its `<file>$package`, `Worker::walk_group`), then the order it was typed in within that member, which is the
/// tree's: a call's prefix and arguments, and the givens they take, before the call (the
/// transform's children first). A member's body typed on demand inside another's keeps its own
/// place, so the order is the source's whichever worker typed what first.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub(super) struct WalkOrder {
    pub(super) file: u32,
    /// The member or the class statement whose tree holds the call, by its place: the unit a
    /// worker of the phase expands whole (`Worker::phase_groups`), whose calls the walk of its
    /// typed tree orders (`Worker::order_unit`).
    pub(super) group: u32,
    typed: u32,
    pub(super) unit: Unit,
}

/// The typed tree a kept call stands in, which the phase walks as `InliningTreeMap` does: a
/// member's (its body and its defaults, or a val's initialiser), a class's statements and header
/// (the class's template, of which the statement is one part), or none the walk knows (a
/// top-level item's), whose calls keep the order they were typed in.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub(super) enum Unit {
    Member(SymId),
    Template(ClassId),
    Item,
}

/// A plain inline call kept for the phase, or what an expansion made ahead of it reported
/// (`PendingInline::held`), reported where the phase reaches the call.
pub(super) struct Kept {
    pub(super) node: TExprId,
    pub(super) typed_as: TypeId,
    pub(super) call: Option<PendingCall>,
    pub(super) held: Vec<Diagnostic>,
    pub(super) at: WalkOrder,
}

/// What a node rewritten in place while a file's round is recorded held before (`Inlining::rewrite_log`).
pub(super) enum Prior {
    Expr(TExprId, crate::tir::TExpr),
    Type(TExprId, Option<TypeId>),
}

/// A file's round in the phase recorded from where the phase enters it, so that a suspension can
/// give the whole round up (`Worker::roll_back`), as dotty's `Phase.runOn` drops a suspended
/// unit's transform and resets its state, the trees other units' macros read the typer's again:
/// the lengths of what its expansions append to.
struct Checkpoint {
    range: std::ops::Range<usize>,
    pending: usize,
    inline_unboxes: usize,
    erased_values: usize,
    lists: [usize; 7],
}

/// Where a class statement stands, read off its tree when a call of it is kept (`walk_place`):
/// the class's check records which statement it types, not its place, which a program keeping no
/// call never reads.
#[derive(Clone, Copy)]
pub(super) enum StatementPlace {
    At(u32),
    Expr(FileId, crate::ast::ExprId),
    Def(FileId, crate::ast::DefId),
}

/// A local trait whose members' trees hold calls kept for the phase, or read such a trait's
/// members: lifted after the phase from its expanded trees (`Worker::lift_deferred_traits`), as
/// dotty's `LambdaLift` takes its free variables after `Inlining`; with the member whose tree
/// defines it.
pub(super) struct DeferredTrait {
    pub(super) class: ClassId,
    pub(super) member: Option<SymId>,
    pub(super) file: FileId,
}

/// A class of a block (a local, an anonymous or a SAM's class, or one an expansion makes) whose
/// body held calls kept for the phase, with the locals its scopes give it (`visible_to_class`)
/// and the member whose tree creates it: its captures, read off its typed nodes where its typing
/// ends, are completed after the phase (`Worker::settle_lifted_captures`).
pub(super) struct Lifted {
    pub(super) class: ClassId,
    pub(super) visible: Vec<SymId>,
    pub(super) member: Option<SymId>,
    pub(super) file: FileId,
}

/// The walk of a unit's typed tree (`Worker::order_unit`): the unit's calls by node with their
/// places in the unit, the ranks met so far, the classes of blocks of the unit's file by place and
/// those walked.
struct RankWalk<'c> {
    /// The unit's calls by node, sorted, each with its place in the unit.
    index: Vec<(TExprId, usize)>,
    ranks: Vec<u32>,
    next: u32,
    classes: &'c [(u32, u32, ClassId)],
    visited: Vec<ClassId>,
    /// A template's walk: every kept call met ranked, by its node (`Inlining::template_ranks`).
    all: Option<FxMap<TExprId, u32>>,
}

/// The kept calls that pay for the phase's fork and merge: below, one worker expands them.
const PHASE_FORK_CALLS: usize = 256;

/// The units of the phase's fork, in the walk's order, each taken whole by one worker
/// (`Worker::expand_in_parallel`).
struct PhaseQueue {
    units: Vec<std::sync::Mutex<Option<Vec<Kept>>>>,
    next: std::sync::atomic::AtomicUsize,
    tables: PhaseTables,
    classes: FxMap<u32, Vec<(u32, u32, ClassId)>>,
    /// The main worker's capture records of the units' kept calls, which the worker that takes a
    /// unit adopts (`Capture::adopt`): the transparent expansion a plain call stands in (its
    /// `INLINED`) is the call's provenance whichever worker expands it, as dotty pickles before
    /// `Inlining`.
    captures: FxMap<TExprId, crate::tir::capture::NodeRecords>,
    /// Whether the round may suspend a file (`Inlining::suspendable`), whether a macro's splice
    /// names its run (`Inlining::splices_name_run`), and the files of its run
    /// (`Inlining::current_run`).
    suspendable: bool,
    splices_name_run: bool,
    current_run: Option<std::sync::Arc<FxMap<u32, ()>>>,
    /// The reflection answers the workers share (`crate::interp::ReflectMemo::shared`).
    reflect: std::sync::Arc<crate::interp::SharedReflectMemo>,
    /// By file id, the first unit, in the walk's order, a worker gave up (`Inlining::suspended`),
    /// `u32::MAX` for none: the file's round is given up from that unit on, and no worker expands
    /// a unit of it after that one.
    given_up: Vec<std::sync::atomic::AtomicU32>,
}

/// The main worker's tables an expansion reads that `attach` leaves a worker of the body phase
/// without (its own bodies' entries alone): a worker of the phase expands any unit's calls, so it
/// starts with the main worker's entries, and hands the merge only those it added. The journal
/// among them: the typed records of the nodes the phase changed before the fork (a unit of an
/// earlier segment), which reflection reads on every worker as on one (`PostTyper`'s `defTree`).
#[derive(Clone, Default)]
struct PhaseTables {
    deferred_tests: FxMap<crate::tir::TestId, TypeId>,
    expansions: FxMap<TExprId, crate::tir::Expansion>,
    leaf_tests: FxMap<crate::tir::TestId, ()>,
    stored_tests: FxMap<crate::tir::TestId, ()>,
    stored_pats: FxMap<crate::tir::TPatId, ()>,
    deferred_lambdas: FxMap<(u32, u32, u32), TExprId>,
    lambda_results: FxMap<(u32, u32, u32), TypeId>,
    journal: FxMap<TExprId, (crate::tir::TExpr, Option<TypeId>)>,
    /// What the body phase's expansions kept past their item (`InlineState::evaluated`, the
    /// literals an interpreted constant made; `InlineState::args`, the locals an `inline val` or a
    /// reduced match's binder stands for): a kept call's expansion on any worker reads them as the
    /// main worker's does.
    evaluated: FxMap<TExprId, ()>,
    inline_args: FxMap<SymId, super::inline::InlineArg>,
}

impl PhaseTables {
    fn of(w: &Worker) -> PhaseTables {
        let p = &w.prog;
        PhaseTables {
            deferred_tests: p.deferred_tests.clone(),
            expansions: p.expansions.clone(),
            leaf_tests: p.leaf_tests.clone(),
            stored_tests: p.stored_tests.clone(),
            stored_pats: p.stored_pats.clone(),
            deferred_lambdas: w.deferred_lambdas.clone(),
            lambda_results: w.lambda_results.clone(),
            journal: w.inlining.journal.clone(),
            evaluated: w.inline.evaluated.clone(),
            inline_args: w.inline.args.clone(),
        }
    }

    fn give(&self, w: &mut Worker) {
        let p = &mut w.prog;
        p.deferred_tests = self.deferred_tests.clone();
        p.expansions = self.expansions.clone();
        p.leaf_tests = self.leaf_tests.clone();
        p.stored_tests = self.stored_tests.clone();
        p.stored_pats = self.stored_pats.clone();
        w.deferred_lambdas = self.deferred_lambdas.clone();
        w.lambda_results = self.lambda_results.clone();
        w.inlining.journal = self.journal.clone();
        w.inline.evaluated = self.evaluated.clone();
        w.inline.args = self.inline_args.clone();
    }

    /// The entries `w` holds that it was given, dropped: the main worker holds them.
    fn take_back(&self, w: &mut Worker) {
        let p = &mut w.prog;
        p.deferred_tests.retain(|k, v| self.deferred_tests.get(k) != Some(v));
        p.expansions.retain(|k, v| self.expansions.get(k).map_or(true, |x| x.callee != v.callee));
        p.leaf_tests.retain(|k, _| !self.leaf_tests.contains_key(k));
        p.stored_tests.retain(|k, _| !self.stored_tests.contains_key(k));
        p.stored_pats.retain(|k, _| !self.stored_pats.contains_key(k));
        w.deferred_lambdas.retain(|k, v| self.deferred_lambdas.get(k) != Some(v));
        w.lambda_results.retain(|k, v| self.lambda_results.get(k) != Some(v));
        w.inlining.journal.retain(|k, _| !self.journal.contains_key(k));
        w.inline.evaluated.retain(|k, _| !self.evaluated.contains_key(k));
        w.inline.args.retain(|k, v| self.inline_args.get(k).map_or(true, |x| x.expr != v.expr || x.ty != v.ty));
    }
}

/// The phase's records: what a worker's units kept, carried through the merge (`Remap::inlining`).
/// Where a unit's typing was cut by its suspension on one worker (`Worker::cut_typing`): the
/// unit, the key of its item the cut is in (`Worker::desugared_key`), and of the definitions in
/// progress there, the unit's own, what they had reported and demanded by then. dotty's
/// `SuspendException` leaves them at that point (`TyperPhase.typeCheck` catches it, the
/// reporter keeping what they reported).
#[derive(Clone)]
pub(super) struct TypingCut {
    pub(super) file: u32,
    pub(super) key: u32,
    /// The records its definitions in progress had reported by then, by identity
    /// (`source::Ordinal`): an equal record reported after, as the item unwinds, is not of them.
    pub(super) reported: Vec<u64>,
    pub(super) demanded: Vec<DiagOwner>,
    /// The rank of the item it was made in (`Worker::diag_rank`), none outside one: a history read
    /// at a later item's reads the cut's standing, one at this item's or before it does not, as one
    /// worker's walk meets the cut there (`Worker::history`, `history_mismatch`).
    pub(super) rank: Option<(u32, u32)>,
}

impl TypingCut {
    /// Whether one worker's walk has met the cut by a read at the item ranked `at`.
    pub(super) fn before(&self, at: (u32, u32)) -> bool {
        self.rank.map_or(true, |r| r < at)
    }
}

/// A file no cut suspended (`Standing::cut_at`).
const NOT_SUSPENDED: u32 = u32::MAX;

/// What stands of what the typing reported where units suspended at typing
/// (`Worker::typing_standing`): by file the key of the item its typing was cut in, the completions
/// those before kept whole and the in-progress definitions at the cuts demanded, and what those had
/// reported by the cut.
pub(super) struct Standing {
    cut_at: Vec<u32>,
    item_of: FxMap<(u32, u32), u32>,
    demanded: FxMap<(u32, u32), ()>,
    at_cut: FxMap<u64, ()>,
}

impl Standing {
    /// Whether `o` is a definition of a unit that suspended.
    fn suspended(&self, o: DiagOwner) -> bool {
        !o.is_none() && self.cut_at.get(o.file as usize).is_some_and(|&k| k != NOT_SUSPENDED)
    }

    /// Whether the traversal keeps `o` whole: of a unit that did not suspend, or of an item before
    /// its unit's cut (each owner's item by its key, worked out once).
    fn whole(&mut self, w: &Worker<'_>, o: DiagOwner) -> bool {
        if !self.suspended(o) {
            return true;
        }
        let key = *self.item_of.entry(o.key()).or_insert_with(|| w.desugared_key(FileId(o.file), o.place));
        key < self.cut_at[o.file as usize]
    }

    /// Whether the record `d` stands: of no owner, of a definition kept whole or demanded, or one a
    /// definition in progress at a cut had reported by then.
    pub(super) fn stands(&mut self, w: &Worker<'_>, d: &Diagnostic) -> bool {
        if d.owner.is_none() || !self.suspended(d.owner) || self.demanded.contains_key(&d.owner.key()) || self.whole(w, d.owner) {
            return true;
        }
        self.at_cut.contains_key(&d.ordinal.0)
    }
}

#[derive(Default)]
pub struct Inlining {
    pub(super) kept: Vec<Kept>,
    /// The kept calls not expanded yet, by node, at their place in `kept`.
    pub(super) index: FxMap<TExprId, u32>,
    /// The order calls were deferred in, by this worker.
    typed: u32,
    /// The place of the item the walk types (`Worker::check_item`), the member of a call no
    /// member's body holds, and the item as the owner of what its typing reports outside its
    /// members' bodies (`Diagnostic::owner`).
    pub(super) item_place: u32,
    /// The files a macro of the run met at typing time suspended (`Worker::typing_suspends`), typed
    /// again from their trees for the phase's second round.
    pub(super) typing_suspended: Vec<u32>,
    /// Whether the typing records which definitions' completions each definition demanded
    /// (`demands`): where a file of the program quotes or splices, whose macro may suspend a unit
    /// at typing (`Worker::begin_demands`).
    pub(super) track_demands: bool,
    /// Whether the typing records where each completion that reported is met (`Worker::meet`): in
    /// a build that may fork (`walk_by_size`), whose merge places a completion's reports at its
    /// first demand in one worker's walk or gives the build to one worker (`events_displaced`).
    pub(super) track_meetings: bool,
    /// `track_demands || track_meetings`, read where every signature is (`notes_demands`).
    pub(super) notes: bool,
    /// Where `diags.items` stood as each completion of `completing` began (`Worker::enter_owner`).
    pub(super) completing_marks: Vec<usize>,
    /// The demands this worker's typing made, each by the definition being typed and the one it
    /// demanded (`Worker::note_demand`): a body it typed or found typed, an inferred signature it
    /// read, an inline method's body an expansion needed, as dotty's completers type them when
    /// read (`Namer.Completer.complete`, `addInlineInfo`'s lazy body). One made again may be
    /// listed again; the closure reads each once (`keep_typing_reports`).
    pub(super) demands: Vec<(DiagOwner, DiagOwner)>,
    /// Where the item being typed began in `demands` (`Worker::check_item`): what its definitions
    /// demanded is listed from there (`Worker::cut_typing`).
    pub(super) item_demands: usize,
    last_demand: ((u32, u32), u32),
    /// By the id of a shared symbol, whether a read of its signature is a demand of its
    /// completion (`note_read`): 0 not asked yet, 1 it is (a program member's completion that
    /// recorded a demand or a report, `SymInfo::completion_noted`), 2 it is not.
    noted: Vec<u8>,
    /// The completions under way on this worker, outermost first, each the owner of what it
    /// reports (`Worker::enter_owner`): the item the walk types, a signature, a body, an alias, a
    /// default's getter, as dotty's completers nest (`Namer.Completer.complete`). Where a unit
    /// suspends, its entries are the definitions in progress (`Worker::cut_typing`).
    pub(super) completing: Vec<DiagOwner>,
    /// Where each unit this worker suspended at typing was cut (`Worker::cut_typing`).
    pub(super) typing_cuts: Vec<TypingCut>,
    /// How many times typing met a macro of the run or read a signature suspended by one: a
    /// signature inferred across either is published `Completion::Suspended` (`Worker::complete_sig`).
    pub(super) typing_suspensions: u32,
    /// The signatures being completed, innermost last, each with the count above where its
    /// inference began (none for a declared one): the state `publish_sig` gives it.
    pub(super) sig_marks: Vec<Option<u32>>,
    /// The items being typed (`Worker::check_item`), and whether a unit typed suspended at
    /// typing inside one: the rest of that unit's code in the item reads as an error value, and
    /// a completion that reaches it is suspended (`Worker::unwinding_in`), as dotty's
    /// `SuspendException` unwinds the unit's typer up to `TyperPhase.typeCheck`, through every
    /// completion under way; another definition's typing on the way, a library body's or a
    /// declared signature's, is its own.
    pub(super) in_item: u32,
    pub(super) unwinding: bool,
    /// The unit a macro's run under way suspends where its execution reaches a definition of a
    /// unit its round has not completed (`Worker::run_suspends`), set by the outermost run that
    /// may suspend; and whether one did, which stops the run (`Failure::Suspended`).
    pub(super) run_consumer: Option<crate::source::FileId>,
    pub(crate) run_suspended: bool,
    /// The place of the class statement, or of the class header, the walk types (a parent's
    /// arguments, a constructor parameter's default), the unit of a call no member's body holds.
    pub(super) statement_place: Option<(ClassId, StatementPlace)>,
    /// The workers the inlining phase may take: the body phase's count where it forked, one
    /// otherwise (`Worker::phase_workers`).
    pub workers: usize,
    /// The first variables' tag the phase's next fork takes: the body phase's workers took those
    /// below `workers`, each fork of the phase the ones after.
    next_tag: usize,
    /// The `if`s whose taken branch waits for their conditions' calls (`Attempts::fold_later`).
    pub(super) fold_later: Vec<(TExprId, TExprId)>,
    /// The tuple indexes over kept calls of constant types (`Attempts::index_later`).
    pub(super) index_later: Vec<(TExprId, TExprId, TExprId)>,
    /// The `erasedValue` calls a unit's trees held when its typing ended, checked once the phase's
    /// expansions are done (`Erasure.checkNotErased` runs after `Inlining`).
    pub(super) erased: Vec<(TExprId, FileId, Span, Option<(FileId, Span)>)>,
    /// The kept calls whose expansions are consumed as a primitive (`Worker::inline_unboxes`),
    /// unboxed once the phase placed them.
    pub(super) unboxes: Vec<(TExprId, TypeId)>,
    /// The `@tailrec` methods whose bodies hold kept calls, checked after the phase (scalac's
    /// `TailRec` runs after `Inlining`).
    pub(super) tailrec: Vec<(SymId, FunId, FileId, Span)>,
    /// The classes of blocks whose bodies held kept calls, in the order they settled (a class
    /// before the class around it), their captures completed after the phase.
    pub(super) lifted: Vec<Lifted>,
    /// The local traits lifted after the phase, in the order they were checked.
    pub(super) deferred_traits: Vec<DeferredTrait>,
    /// Whether the call being expanded may be suspended: one of a file whose round is recorded
    /// (`Checkpoint`), expanded at the top.
    pub(super) suspending: bool,
    /// Whether the call being expanded met a macro of this run, whose run is not made: the
    /// expansion is given up and its file waits for the next round (`expand_groups`).
    pub(super) suspended_hit: bool,
    /// The files whose round an expansion gave up (`expand_groups`), typed again from their trees
    /// for the next round (`Worker::retype_suspended`).
    pub(super) suspended: Vec<u32>,
    /// Whether an expansion of the round may suspend its file: a macro of the round's run is
    /// defined (`defines_macro_of_run`), which an expansion may meet, or, in a later round, the
    /// program defines a macro, whose run may reach the round's files (`run_suspends`). Each
    /// file's round is recorded then (`Checkpoint`).
    suspendable: bool,
    /// Whether a macro's splice names a definition of the round's run (`defines_macro_of_run`):
    /// only then does an expansion's splice, which its definition's is, ask
    /// `splice_names_program`.
    pub(super) splices_name_run: bool,
    /// The files of the round's run, whose definitions are its macros' dependencies: every file
    /// of the program in the first round, the files the round before suspended in each later one,
    /// as dotty's later run compiles the suspended units alone.
    pub(super) current_run: Option<std::sync::Arc<FxMap<u32, ()>>>,
    /// Whether a run inside the file's round expanded another unit's call (`prepare_executable`):
    /// the file is not given up then, which would take back what that one made, and its macros
    /// run in place.
    pub(super) reached_other: bool,
    /// While a file's round is recorded (`Checkpoint`), the anonymous classes' places its
    /// expansions numbered (`anon_name`), counted back where it is given up.
    pub(super) anon_log: Option<Vec<(FileId, u32, Option<(FileId, u32)>)>>,
    /// While a file's round is recorded, what each node its expansions rewrote in place held
    /// before (`Worker::rewrite_expr`, `retype_expr`), put back where it is given up.
    pub(super) rewrite_log: Option<Vec<Prior>>,
    /// What the suspension of a file reported (a macro's dependency in the file's own source),
    /// which outlives the file's round given up.
    pub(super) suspension_reports: Vec<Diagnostic>,
    /// Where the diagnostics of the file's round recorded (`Checkpoint`) begin, and what the
    /// round had reported when its expansion met the macro of the run (`Worker::hit_suspension`):
    /// what stands of the round once it is given up.
    round_diags: usize,
    suspension_prefix: Option<FxMap<u64, ()>>,
    /// On a worker of a forked round that may suspend a file, what each unit reported, with the
    /// unit's place in the walk and its file: what the merge keeps of a file given up
    /// (`give_up_suspended`).
    pub(super) unit_reports: Vec<(u32, u32, u64)>,
    /// The typed records of the nodes the phase changed in place (a kept call's, the index of a
    /// tuple its expansion folded), with their recorded types: the typed view reflection reads
    /// (`Worker::typed_expr`), as dotty's `defTree`, set by `PostTyper` before `Inlining`, keeps
    /// the call (ruling 3's frozen typed publication). A session keeps them with the trees.
    pub(super) journal: FxMap<TExprId, (crate::tir::TExpr, Option<TypeId>)>,
    /// The typed view's copies of definitions' trees made for reflection, by root, for the unit
    /// the phase expands (`Worker::typed_body`), and whether a copy is being made.
    typed_copies: FxMap<TExprId, TExprId>,
    pub(super) typed_view: bool,
    /// Reflection's answers while the phase runs (`crate::interp::ReflectMemo`), made at its
    /// first query.
    reflect: Option<Box<crate::interp::ReflectMemo>>,
    /// While the forked phase's workers run, the answers they share.
    shared_reflect: Option<std::sync::Arc<crate::interp::SharedReflectMemo>>,
    /// Whether the phase is under way.
    pub(super) running: bool,
    /// The functions and the classes whose trees the interpreter has run since calls were kept,
    /// their kept calls expanded (`Worker::prepare_fun`, `prepare_class`): by index under the
    /// canonical ids of the phase, by id while the workers type.
    checked_funs: Vec<bool>,
    checked_funs_typing: FxMap<FunId, ()>,
    checked_classes: Vec<bool>,
    checked_classes_typing: FxMap<usize, ()>,
    /// The kept calls whose expansion is under way, innermost last.
    expanding: Vec<TExprId>,
    /// A class's template walked once in the phase, every kept call in it by its rank
    /// (`Worker::order_unit`): the class's statements are units of their own over one template.
    template_ranks: FxMap<ClassId, FxMap<TExprId, u32>>,
    /// The environment the last kept call took, which the next one of the same scopes shares
    /// (`Worker::shared_site_env`): a block's calls hold one record of its scopes.
    pub(super) last_env: Option<std::sync::Arc<super::Env>>,
    /// The top-level definition the last kept call's place was found in (`Worker::walk_group`): its
    /// file's tree, by address, its extent and whether dotty's desugaring wraps it.
    walk_top: (usize, u32, u32, bool),
    /// The environment the phase's walk holds as the worker's own (`Worker::env`) while it
    /// expands the calls that share it, cloned once for them (`Worker::expand_pending`).
    pub(super) installed: Option<std::sync::Arc<super::Env>>,
}

/// The bit of a top-level definition's key (`Worker::desugared_key`) that places it in the file's
/// `<file>$package` object, after the file's other statements.
const WRAPPED: u32 = 1 << 31;

/// Whether the top-level definition `d` is one dotty's desugaring wraps in the file's
/// `<file>$package` object, which `Desugar.packageDef` places after the file's other statements
/// (`topStats :+ grouped`): `isTopLevelDef` takes a val, a def, a type alias, a given and an
/// implicit class or object, and an object named as such a type (an opaque type's companion)
/// goes with it. A `package object`'s members, read as a package block, are the object's own,
/// in their order.
pub(super) fn wrapped_top_level(ast: &crate::ast::Ast, d: crate::ast::DefId) -> bool {
    wrapped_def(ast, d, |name| ast.top_level.iter().any(|&t| ast.def(t).name == name && wraps_type(ast.def(t))))
}

/// Whether the definition is a type the package object wraps, whose companion object it takes
/// along: a type alias, an implicit class.
fn wraps_type(o: &crate::ast::Def) -> bool {
    use crate::ast::{ClassKind, DefKind};
    match &o.kind {
        DefKind::TypeAlias { .. } => true,
        DefKind::Class(c) => c.kind != ClassKind::Object && o.mods & (crate::ast::mods::GIVEN | crate::ast::mods::IMPLICIT) != 0,
        _ => false,
    }
}

/// `wrapped_top_level` with the question whether a wrapped type is named as an object answered
/// by `wrapped_type_named`.
fn wrapped_def(ast: &crate::ast::Ast, d: crate::ast::DefId, wrapped_type_named: impl Fn(crate::intern::Name) -> bool) -> bool {
    use crate::ast::{ClassKind, DefKind};
    if ast.package_object {
        return false;
    }
    let def = ast.def(d);
    match &def.kind {
        DefKind::Val { .. } | DefKind::Fun(_) | DefKind::TypeAlias { .. } | DefKind::Given(_) => true,
        DefKind::Class(_) if def.mods & (crate::ast::mods::GIVEN | crate::ast::mods::IMPLICIT) != 0 => true,
        DefKind::Class(c) => c.kind == ClassKind::Object && wrapped_type_named(def.name),
    }
}

/// The key of the top-level definition `d` in its file's desugared order: its start, after every
/// other statement of the file where the package object wraps it (`wrapped_top_level`).
pub(super) fn top_level_key(ast: &crate::ast::Ast, d: crate::ast::DefId) -> u32 {
    key_of(ast, d, wrapped_top_level(ast, d))
}

fn key_of(ast: &crate::ast::Ast, d: crate::ast::DefId, wrapped: bool) -> u32 {
    let start = ast.def_range(d).start;
    if wrapped {
        start | WRAPPED
    } else {
        start
    }
}

/// The file's top-level definitions, by their index, in the order dotty's typer meets them:
/// the file's classes and objects, then the package object's members, each in their order
/// (`Desugar.packageDef`'s `topStats :+ grouped`). `None` where that is the order they are
/// written in.
pub(super) fn desugared_order(ast: &crate::ast::Ast) -> Option<Vec<u32>> {
    if ast.package_object || ast.top_level.len() < 2 {
        return None;
    }
    let types: FxMap<crate::intern::Name, ()> = ast.top_level.iter().map(|&t| ast.def(t)).filter(|t| wraps_type(t)).map(|t| (t.name, ())).collect();
    let wrapped: Vec<bool> = ast.top_level.iter().map(|&d| wrapped_def(ast, d, |name| types.contains_key(&name))).collect();
    // Written in that order already: no wrapped definition before one that is not.
    if !wrapped.windows(2).any(|w| w[0] && !w[1]) {
        return None;
    }
    let (own, grouped): (Vec<u32>, Vec<u32>) = (0..wrapped.len() as u32).partition(|&i| !wrapped[i as usize]);
    Some(own.into_iter().chain(grouped).collect())
}

impl Inlining {
    /// Whether nothing waits for the phase.
    pub fn is_empty(&self) -> bool {
        self.kept.is_empty() && self.fold_later.is_empty() && self.index_later.is_empty() && self.erased.is_empty() && self.unboxes.is_empty() && self.tailrec.is_empty() && self.deferred_traits.is_empty()
    }

    /// What another worker kept, appended: the merge renumbers it with the rest
    /// (`Remap::inlining`) and indexes it again.
    pub(super) fn absorb(&mut self, o: &mut Inlining) {
        self.kept.append(&mut o.kept);
        self.fold_later.append(&mut o.fold_later);
        self.index_later.append(&mut o.index_later);
        self.erased.append(&mut o.erased);
        self.unboxes.append(&mut o.unboxes);
        self.tailrec.append(&mut o.tailrec);
        self.lifted.append(&mut o.lifted);
        self.deferred_traits.append(&mut o.deferred_traits);
        self.suspended.append(&mut o.suspended);
        for f in o.typing_suspended.drain(..) {
            if !self.typing_suspended.contains(&f) {
                self.typing_suspended.push(f);
            }
        }
        // The suspended units' reports are placed with the diagnostics (`merge_diagnostics`).
        debug_assert!(o.suspension_reports.is_empty(), "a worker's suspension reports left to the merge");
        self.demands.append(&mut o.demands);
        self.typing_cuts.append(&mut o.typing_cuts);
        self.unit_reports.append(&mut o.unit_reports);
        self.journal.extend(std::mem::take(&mut o.journal));
        self.index.clear();
    }

    /// Whether the typing notes what a read demands: for the units' suspensions (`track_demands`)
    /// or for the merge's placement of what completions reported (`track_meetings`).
    #[inline]
    pub(super) fn notes_demands(&self) -> bool {
        self.notes
    }

    /// The index of the calls still to expand, made again (after a merge or a sort).
    pub(super) fn reindex(&mut self) {
        self.index.clear();
        for (i, k) in self.kept.iter().enumerate() {
            if k.call.is_some() {
                self.index.insert(k.node, i as u32);
            }
        }
    }

    /// Whether `e` is a kept call not expanded yet.
    #[inline]
    pub(super) fn holds(&self, e: TExprId) -> bool {
        !self.index.is_empty() && self.index.contains_key(&e)
    }

    /// The type the typing gave the kept call `e`.
    pub(super) fn typed_as(&self, e: TExprId) -> Option<TypeId> {
        self.index.get(&e).map(|&i| self.kept[i as usize].typed_as)
    }

    /// What a watch session's retype of `file` drops: the calls its typing kept, which the
    /// retype keeps again, and what waits for their expansions (an earlier build that reported an
    /// error ran no phase).
    pub(super) fn forget_file(&mut self, file: FileId, prog: &crate::tir::Program) {
        self.walk_top = Default::default();
        if self.kept.iter().any(|k| k.at.file == file.0) {
            self.kept.retain(|k| k.at.file != file.0);
            self.reindex();
        }
        self.erased.retain(|e| e.1 != file);
        self.tailrec.retain(|t| t.2 != file);
        self.lifted.retain(|l| l.file != file);
        self.deferred_traits.retain(|d| d.file != file);
        let of_file = |e: TExprId| prog.span_of(e).is_some_and(|(f, _)| f == file);
        self.fold_later.retain(|&(c, _)| !of_file(c));
        self.journal.retain(|&e, _| !of_file(e));
        self.index_later.retain(|&(i, _, _)| !of_file(i));
        self.unboxes.retain(|&(e, _)| !of_file(e));
    }
}

impl<'a> Worker<'a> {
    /// The phase over `workers` workers, the calls of `inlining.kept` in `units` (runs of it,
    /// in the walk's order): the program's records shared again (`fork`), each unit expanded whole
    /// by the worker that takes it, its trees that worker's to change (`Arena::owning`: the copies
    /// of the records it changes are its own until the merge writes them over the shared ones,
    /// outside any hold of the loader's lock), and the workers' records merged in the units'
    /// order, which is the walk's (`ItemRange`'s key): what one worker's walk makes. A worker
    /// whose run reaches a tree another unit holds a call in gives the build to one worker
    /// (`prepare_executable`).
    /// The first unit each file a worker gave a unit of up was given up at, by file
    /// (`PhaseQueue::given_up`).
    fn expand_in_parallel(&mut self, workers: usize, units: Vec<std::ops::Range<usize>>, classes: FxMap<u32, Vec<(u32, u32, ClassId)>>) -> FxMap<u32, u32> {
        let mut kept = std::mem::take(&mut self.inlining.kept);
        let mut taken: Vec<Vec<Kept>> = units.iter().rev().map(|u| kept.split_off(u.start)).collect();
        taken.reverse();
        let mut captures: FxMap<TExprId, crate::tir::capture::NodeRecords> = FxMap::default();
        if let Some(c) = self.prog.capture.as_deref() {
            for k in taken.iter().flatten() {
                let r = c.own_records_of(k.node);
                if !r.is_empty() {
                    captures.insert(k.node, r);
                }
            }
        }
        let queue = PhaseQueue { units: taken.into_iter().map(|u| std::sync::Mutex::new(Some(u))).collect(), next: std::sync::atomic::AtomicUsize::new(0), tables: PhaseTables::of(self), classes, captures, suspendable: self.inlining.suspendable, splices_name_run: self.inlining.splices_name_run, current_run: self.inlining.current_run.clone(), reflect: Default::default(), given_up: (0..self.files.len()).map(|_| std::sync::atomic::AtomicU32::new(u32::MAX)).collect() };
        super::substitution::counts::phase_forked(queue.units.len());
        let prefix = super::merge::Prefix::of(self);
        self.diag_ranges.clear();
        self.history_reads.clear();
        self.told.clear();
        self.meetings.clear();
        self.diag_region = (self.diags.items.len(), self.inlining.suspension_reports.len());
        // The body phase's workers took the variables' tags below its count, an earlier fork of
        // the phase the ones after.
        self.tvars.into_phase(self.inlining.next_tag as u32);
        self.inlining.next_tag += workers;
        self.fork(true);
        let overlays = crate::types::overlays_wanted() && self.types.refork_overlays(workers);
        crate::types::view::forked(overlays);
        crate::measure::phase_done("inlining, the fork");
        let others: Vec<Worker<'a>> = (1..workers).map(|k| self.attach(k)).collect();
        let others: Vec<Option<Worker<'a>>> = std::thread::scope(|s| {
            let mut handles = Vec::with_capacity(others.len());
            let mut refused = false;
            for (k, mut w) in others.into_iter().enumerate() {
                // A worker the system gives no thread is left unstarted with every one after it, and the
                // build gives way (`need_serial`), as a body phase's worker refused its thread does.
                if refused {
                    continue;
                }
                let q = &queue;
                let handle = std::thread::Builder::new()
                    .name(format!("teq-inlining-{}", k + 1))
                    .stack_size(1 << 30)
                    .spawn_scoped(s, move || {
                        crate::alloc::enter();
                        w.phase_work(q);
                        // A session outlives the build: the cycles this worker's macro runs left
                        // go with its thread (`interp::dispose`), as a body-phase worker's do.
                        if crate::alloc::is_resident() {
                            crate::interp::dispose();
                        }
                        w
                    });
                match handle {
                    Ok(handle) => handles.push(handle),
                    Err(e) => {
                        refused = true;
                        let why = format!("the system gave a worker of the inlining phase no thread ({})", e);
                        crate::types::note_worker_refused(why.clone());
                        self.need_serial(|| why);
                    }
                }
            }
            self.phase_work(&queue);
            handles.into_iter().map(|h| Some(h.join().unwrap_or_else(|_| std::process::exit(101)))).collect()
        });
        crate::measure::phase_done("inlining, the workers");
        let written = self.tvars.inherited_written() + others.iter().flatten().map(|w| w.tvars.inherited_written()).sum::<usize>();
        super::substitution::counts::inherited_written(written);
        if let Some(why) = self.diagnostics_unplaced(&others).or_else(|| self.history_mismatch(&others)).or_else(|| self.events_displaced(&others)) {
            self.need_serial_for_diagnostics(|| why);
        }
        // The build is typed again by one worker (`need_serial`): what the workers made goes.
        if self.serial.is_needed() {
            return FxMap::default();
        }
        self.merge_workers(others, &prefix);
        self.tvars.out_of_phase();
        self.forget_local_memos();
        crate::measure::phase_done("inlining, the merge");
        let given_up = queue.given_up.iter().map(|g| g.load(std::sync::atomic::Ordering::Acquire));
        given_up.enumerate().filter(|&(_, u)| u != u32::MAX).map(|(f, u)| (f as u32, u)).collect()
    }

    /// The units of `q` this worker takes, each expanded whole as the walk would, the records it
    /// makes recorded as the unit's (`ItemRange`, keyed by the unit's place in the walk).
    fn phase_work(&mut self, q: &PhaseQueue) {
        if self.worker != 0 {
            q.tables.give(self);
            self.inlining.suspendable = q.suspendable;
            self.inlining.splices_name_run = q.splices_name_run;
            self.inlining.current_run = q.current_run.clone();
        }
        self.inlining.shared_reflect = Some(q.reflect.clone());
        if let Some(m) = self.inlining.reflect.as_mut() {
            m.shared = Some(q.reflect.clone());
        }
        self.begin_work();
        let measured = crate::measure::phase_work_begin(self.worker);
        self.prog.exprs.owning = true;
        self.prog.expr_types.owning = true;
        self.inlining.running = true;
        let outer_env = self.env.clone();
        let erased = self.erased_values.len();
        // `TEQ_IDLE_WORKERS` keeps a worker from the units as from the items (`check::idle_worker`).
        let idle = super::check::idle_worker(self.worker);
        loop {
            if self.serial.is_needed() || idle {
                break;
            }
            crate::shake::point(crate::shake::Point::Item);
            let u = q.next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let Some(slot) = q.units.get(u) else { break };
            let Some(unit) = slot.lock().unwrap_or_else(|e| e.into_inner()).take() else { continue };
            if self.worker != 0 && !q.captures.is_empty() {
                if let Some(c) = self.prog.capture.as_deref_mut() {
                    for k in &unit {
                        if let Some(r) = q.captures.get(&k.node) {
                            c.adopt(k.node, r.clone());
                        }
                    }
                }
            }
            // A file's unit after one a worker gave up waits for the next round, which its walk
            // does not reach; one before it is expanded, as the walk reaches it first.
            let file = unit.first().map_or(0, |k| k.at.file);
            if q.given_up.get(file as usize).is_some_and(|g| u as u32 > g.load(std::sync::atomic::Ordering::Acquire)) {
                continue;
            }
            let start = self.own_marks();
            let marks = self.begin_diag_range((0, u as u32));
            let given_up = self.inlining.suspended.len();
            let reported = self.diags.items.len();
            self.inlining.kept = unit;
            let groups = self.phase_groups();
            self.expand_groups(groups, &q.classes);
            for &f in &self.inlining.suspended[given_up..] {
                if let Some(g) = q.given_up.get(f as usize) {
                    g.fetch_min(u as u32, std::sync::atomic::Ordering::AcqRel);
                }
            }
            if q.suspendable {
                let reports = self.diags.items.get(reported..).unwrap_or_default();
                self.inlining.unit_reports.extend(reports.iter().map(|d| (u as u32, file, d.ordinal.0)));
            }
            self.inlining.kept.clear();
            self.inlining.index.clear();
            self.publish_made_classes();
            let end = self.own_marks();
            self.items.push(super::merge::ItemRange { key: (0, u as u32), start, end });
            self.end_diag_range((0, u as u32), marks);
            if self.worker != 0 {
                super::substitution::counts::attached_unit();
            }
        }
        self.inlining.shared_reflect = None;
        if let Some(m) = self.inlining.reflect.as_mut() {
            m.shared = None;
        }
        // The erased values the expansions made go through the merge with the units' records.
        let made = self.erased_values.split_off(erased);
        self.inlining.erased.extend(made);
        if self.worker != 0 {
            q.tables.take_back(self);
            self.inlining.template_ranks.clear();
            if let Some(memo) = self.inlining.reflect.take() {
                super::substitution::counts::reflected(memo.asked, memo.answered);
            }
        }
        self.env = outer_env;
        crate::measure::phase_work_end(self.worker, measured);
        self.end_work();
    }

    /// The pending call `p` kept for the phase: its site's context without the typer's state
    /// (the GADT bounds, the bodies under way), as dotty's phase has only the tree's own context
    /// (`TreeMapWithPreciseStatContexts`).
    pub(super) fn keep_for_phase(&mut self, p: PendingInline) {
        let PendingInline { node, typed_as, call, held, place, .. } = p;
        let at = place;
        if super::substitution::counts::on() {
            // An environment shared with the calls before counts once.
            let frames: usize = call.as_ref().filter(|c| std::sync::Arc::strong_count(&c.env) <= 2).map_or(0, |c| {
                c.env.frames.iter().map(|f| match f {
                    super::Frame::Locals { names, tparams, givens, classes, aliases, .. } => {
                        names.len() * 8 + tparams.len() * 8 + givens.len() * 4 + classes.len() * 8 + aliases.len() * 8 + 64
                    }
                    super::Frame::Class(_) => 16,
                }).sum::<usize>() + c.env.imports.len() * std::mem::size_of::<super::ResolvedImport>() + c.args.len() * 4 + c.arg_types.len() * 8
            });
            super::substitution::counts::kept((std::mem::size_of::<Kept>() + frames) as u64);
        }
        let call = call.map(|mut c| {
            c.gadt = Vec::new();
            c.bodies_in_progress = Vec::new();
            c.of_bodies = true;
            c
        });
        if call.is_some() {
            self.inlining.index.insert(node, self.inlining.kept.len() as u32);
        }
        self.inlining.kept.push(super::inlining::Kept { node, typed_as, call, held, at });
    }

    /// The typed tree of the definition whose executable tree is `root`, which reflection reads
    /// (`Symbol.tree`, a member's right-hand side): `root` itself while the phase changed nothing,
    /// else a copy made of its typed records (`Inlining::journal`), once for the unit the phase
    /// expands, whichever worker expands it.
    pub(crate) fn typed_body(&mut self, root: TExprId) -> TExprId {
        if self.inlining.journal.is_empty() {
            return root;
        }
        if let Some(&copy) = self.inlining.typed_copies.get(&root) {
            return copy;
        }
        let was = std::mem::replace(&mut self.inlining.typed_view, true);
        let copy = self.copy_expr(root);
        self.inlining.typed_view = was;
        self.inlining.typed_copies.insert(root, copy);
        copy
    }

    /// The memo of reflection's queries, while the phase runs and the typed program is frozen.
    pub(crate) fn reflect_memo(&mut self) -> Option<&mut crate::interp::ReflectMemo> {
        if !self.inlining.running {
            return None;
        }
        let l = &mut self.inlining;
        Some(l.reflect.get_or_insert_with(|| {
            let mut m = Box::<crate::interp::ReflectMemo>::default();
            m.shared = l.shared_reflect.clone();
            m
        }))
    }

    /// The node `e`'s typed record and type kept before the phase changes it in place
    /// (`Inlining::journal`), its first one.
    pub(super) fn journal_typed(&mut self, e: TExprId) {
        if !self.inlining.journal.contains_key(&e) {
            let record = (self.prog.expr(e), self.prog.type_of(e));
            self.inlining.journal.insert(e, record);
        }
    }

    /// The typed record of `e`, which reflection reads: the one the phase changed, else the
    /// node's.
    #[inline]
    pub(crate) fn typed_expr(&self, e: TExprId) -> crate::tir::TExpr {
        if self.inlining.journal.is_empty() {
            return self.prog.expr(e);
        }
        match self.inlining.journal.get(&e) {
            Some(&(record, _)) => record,
            None => self.prog.expr(e),
        }
    }

    /// The recorded type of `e` in the typed view (`typed_expr`).
    #[inline]
    pub(crate) fn typed_type_of(&self, e: TExprId) -> Option<TypeId> {
        if self.inlining.journal.is_empty() {
            return self.prog.type_of(e);
        }
        match self.inlining.journal.get(&e) {
            Some(&(_, t)) => t,
            None => self.prog.type_of(e),
        }
    }

    /// The class of a block `c`, settled: noted with the locals `visible` to it where its trees
    /// hold calls kept for the phase (`Program::kept_bits`: a call the copier of a stored body made
    /// before the class settled among them), whose expansions may read a local the calls do not
    /// name (`settle_lifted_captures`).
    pub(super) fn note_lifted(&mut self, c: ClassId, visible: Vec<SymId>) {
        let holds = match self.checked_class(c).cloned() {
            Some(tc) => self.class_roots(std::slice::from_ref(&tc)).into_iter().any(|r| self.prog.descendants(r).any(|e| self.prog.is_kept(e))),
            None => false,
        };
        if !holds {
            return;
        }
        let mut member = None;
        for &s in self.bodies_in_progress.iter().rev() {
            let info = self.syms.sym(s);
            if info.owner != crate::symbols::Owner::Local && !self.in_lifted_class(info.owner) {
                member = Some(s);
                break;
            }
        }
        let file = self.syms.class(c).file;
        self.inlining.lifted.push(Lifted { class: c, visible, member, file });
    }

    /// Whether the local trait `t`, just checked, is lifted after the phase (`DeferredTrait`): its
    /// members' trees, or those of the classes nested in it, hold calls kept for the phase, whose
    /// expansions may read a value of the block they do not name, or it reads or extends a trait
    /// lifted after the phase, whose members' proxies are not known yet.
    pub(super) fn defer_trait_lifting(&mut self, t: ClassId) -> bool {
        if self.is_stored_class(t) || !self.dotty_local_trait(t) {
            return false;
        }
        let deferred = |w: &Self, c: ClassId| w.inlining.deferred_traits.iter().any(|d| d.class == c);
        let mut classes = vec![t];
        let mut k = 0;
        while k < classes.len() {
            let nested: Vec<ClassId> = self.syms.class(classes[k]).nested.values().copied().collect();
            for n in nested {
                if !classes.contains(&n) {
                    classes.push(n);
                }
            }
            k += 1;
        }
        let bodies: Vec<crate::tir::TClass> = classes.iter().filter_map(|&k| self.checked_class(k).cloned()).collect();
        let reads_deferred = |w: &Self, s: SymId| matches!(w.syms.sym(s).owner, crate::symbols::Owner::Class(o) if deferred(w, o));
        let holds = self.syms.class(t).base_types.iter().skip(1).any(|&(b, _)| deferred(self, b))
            || self.class_roots(&bodies).into_iter().any(|r| {
                self.prog.descendants(r).any(|e| {
                    self.prog.is_kept(e)
                        || match self.prog.expr(e) {
                            crate::tir::TExpr::CallMethod(_, s, _) | crate::tir::TExpr::Field(_, s) => reads_deferred(self, s),
                            _ => false,
                        }
                })
            });
        if !holds {
            return false;
        }
        let member = self.bodies_in_progress.iter().rev().copied().find(|&s| {
            let info = self.syms.sym(s);
            info.owner != crate::symbols::Owner::Local && !self.in_lifted_class(info.owner)
        });
        let file = self.syms.class(t).file;
        self.inlining.deferred_traits.push(DeferredTrait { class: t, member, file });
        true
    }

    /// The local traits lifted after the phase (`defer_trait_lifting`), from their expanded trees,
    /// as dotty's `LambdaLift` runs after `Inlining`: each trait's members take their proxies
    /// (`lift_trait_proxies`), each class that mixes one in takes its forwarders and passes the
    /// proxies to its initialiser, the calls of the members in the trees of the members that define
    /// the traits and in their classes pass them, and those classes capture what they now read
    /// (`settle_lifted_captures`, which runs next).
    /// Between the phase's rounds, `waiting` holds the files of the suspended calls, whose traits
    /// wait for their expansions.
    fn lift_deferred_traits(&mut self, waiting: Option<&FxMap<u32, ()>>) {
        let deferred = match waiting {
            None => std::mem::take(&mut self.inlining.deferred_traits),
            Some(w) => {
                let (wait, now) = std::mem::take(&mut self.inlining.deferred_traits).into_iter().partition(|d| w.contains_key(&d.file.0));
                self.inlining.deferred_traits = wait;
                now
            }
        };
        if deferred.is_empty() {
            return;
        }
        let traits: Vec<ClassId> = deferred.iter().map(|d| d.class).collect();
        for &t in &traits {
            self.lift_trait_proxies(t);
        }
        let mixers: Vec<ClassId> = self
            .prog
            .classes
            .iter()
            .map(|tc| tc.id)
            .filter(|&k| self.syms.class(k).kind != crate::symbols::ClassKind::Trait && self.syms.class(k).base_types.iter().skip(1).any(|&(b, _)| traits.contains(&b)))
            .collect();
        for &k in &mixers {
            self.add_trait_forwarders_of(k, Some(&traits));
        }
        let mut done: Vec<(Option<SymId>, FileId)> = Vec::new();
        for d in &deferred {
            if done.contains(&(d.member, d.file)) {
                continue;
            }
            done.push((d.member, d.file));
            let (roots, classes) = self.member_trees(d.member, d.file);
            self.adapt_block_calls_in(&roots);
            // A trait's own calls are adapted where it is lifted, its lazy vals' reads of their
            // initialisers left to the source signature.
            for &k in &classes {
                if self.syms.class(k).kind == crate::symbols::ClassKind::Trait {
                    continue;
                }
                if let Some(tc) = self.checked_class(k).cloned() {
                    let roots = self.lifted_roots_of(&tc);
                    self.adapt_lifted_calls(&roots);
                }
            }
            for k in classes {
                if self.syms.class(k).kind == crate::symbols::ClassKind::Trait || self.inlining.lifted.iter().any(|l| l.class == k) || !self.anon_envs.contains_key(&k) {
                    continue;
                }
                let visible = self.visible_to_class(k);
                self.inlining.lifted.push(Lifted { class: k, visible, member: d.member, file: d.file });
            }
        }
    }

    /// The trees of the member `member` of `file` (with none, the file's functions'), and the
    /// classes of blocks it holds, by their places: its extent, from its name to the end of its
    /// trees.
    fn member_trees(&self, member: Option<SymId>, file: FileId) -> (Vec<TExprId>, Vec<ClassId>) {
        let mut roots: Vec<TExprId> = Vec::new();
        match member {
            Some(m) => {
                if let Some(&f) = self.fun_of_sym.get(&m) {
                    let fun = &self.prog.funs[f.idx()];
                    roots.extend(fun.body.iter().chain(fun.defaults.iter().flatten()));
                }
                if let Some(&v) = self.val_init.get(&m) {
                    roots.push(v);
                }
            }
            None => {
                for f in self.prog.funs.iter() {
                    if self.syms.sym(f.sym).file == file {
                        roots.extend(f.body.iter().chain(f.defaults.iter().flatten()));
                    }
                }
            }
        }
        let span = member.map(|m| self.member_extent(m));
        let classes = self
            .prog
            .classes
            .iter()
            .filter(|tc| {
                let info = self.syms.class(tc.id);
                info.file == file && (info.owner == crate::symbols::Owner::Local || self.in_lifted_class(info.owner)) && span.map_or(true, |s| s.start <= info.span.start && info.span.end <= s.end)
            })
            .map(|tc| tc.id)
            .collect();
        (roots, classes)
    }

    /// The captures of the classes of blocks whose calls the phase expanded, completed as dotty's
    /// `LambdaLift` takes them after `Inlining`, from the expanded trees: a local of the class's
    /// scopes an expansion reads and the class's calls did not name (a local given `summonInline`
    /// finds at the site) joins its captures, after those it had, its nested classes read it
    /// through it (`capture_through_nested`), and each creation of it, and each subclass's call
    /// of its constructor, passes it; a class of a block whose tree such a creation stands in
    /// captures it in turn.
    /// Between the phase's rounds, `waiting` holds the files of the suspended calls, whose classes
    /// wait for their expansions.
    fn settle_lifted_captures(&mut self, waiting: Option<&FxMap<u32, ()>>) {
        let lifted = match waiting {
            None => std::mem::take(&mut self.inlining.lifted),
            Some(w) => {
                let (wait, now) = std::mem::take(&mut self.inlining.lifted).into_iter().partition(|l| w.contains_key(&l.file.0));
                self.inlining.lifted = wait;
                now
            }
        };
        let mut todo: Vec<(ClassId, Vec<SymId>, Option<SymId>, FileId)> = lifted.into_iter().map(|l| (l.class, l.visible, l.member, l.file)).collect();
        let mut i = 0;
        while i < todo.len() {
            let (c, visible, member, file) = todo[i].clone();
            i += 1;
            let Some(tc) = self.checked_class(c).cloned() else { continue };
            let had: Vec<SymId> = match self.anon_captures.get(&c) {
                Some(v) => v.clone(),
                None => tc.ctor_params[..tc.captures.min(tc.ctor_params.len())].to_vec(),
            };
            let mut added: Vec<SymId> = Vec::new();
            let mut classes = vec![c];
            let mut k = 0;
            while k < classes.len() {
                let nested: Vec<ClassId> = self.syms.class(classes[k]).nested.values().copied().collect();
                for n in nested {
                    if !classes.contains(&n) {
                        classes.push(n);
                    }
                }
                k += 1;
            }
            let bodies: Vec<crate::tir::TClass> = classes.iter().filter_map(|&k| self.checked_class(k).cloned()).collect();
            // The parents' arguments with the rest: the constructor reads them.
            let roots: Vec<TExprId> = bodies.iter().flat_map(|tc| self.lifted_roots_of(tc)).collect();
            for root in roots {
                for e in self.prog.descendants(root) {
                    if let crate::tir::TExpr::Local(s) | crate::tir::TExpr::CallStatic(s, _) = self.prog.expr(e) {
                        if visible.contains(&s) && !had.contains(&s) && !added.contains(&s) {
                            added.push(s);
                        }
                    }
                }
            }
            if added.is_empty() {
                continue;
            }
            let n = had.len();
            if let Some(tc) = self.checked_class_mut(c) {
                let at = n.min(tc.ctor_params.len());
                tc.ctor_params.splice(at..at, added.iter().copied());
                tc.ctor_defaults.splice(at..at, std::iter::repeat(None).take(added.len()));
                if tc.captures == n {
                    tc.captures += added.len();
                }
            }
            self.anon_captures.entry(c).or_default().splice(n..n, added.iter().copied());
            // The classes nested in `c` read them through it, from parameters of their own
            // (`capture_through_nested`, which passes them where `c`'s trees make them); a
            // creation of one elsewhere passes them too.
            let before: Vec<(ClassId, Vec<SymId>)> = classes[1..].iter().filter_map(|&k| self.checked_class(k).map(|tc| (k, tc.ctor_params.clone()))).collect();
            self.capture_through_nested(c, &added);
            for (k, was) in before {
                let Some(now) = self.checked_class(k).map(|tc| tc.ctor_params.clone()) else { continue };
                if now.len() <= was.len() {
                    continue;
                }
                let at = (0..was.len()).find(|&i| now[i] != was[i]).unwrap_or(was.len());
                let passed: Vec<SymId> = now[at..at + now.len() - was.len()].to_vec();
                self.pass_captures(k, at, &passed, member, file, &classes);
            }
            for k in self.pass_captures(c, n, &added, member, file, &[]) {
                if !todo[i..].iter().any(|t| t.0 == k) {
                    let visible = self.visible_to_class(k);
                    let file = self.syms.class(k).file;
                    todo.push((k, visible, member, file));
                }
            }
        }
    }

    /// The extent of the definition of `m` in its source: from its name to the end of its body or
    /// initialiser (a symbol's own span is its name's).
    fn member_extent(&self, m: SymId) -> Span {
        let info = self.syms.sym(m);
        let at = info.span;
        let file = info.file.0 as usize;
        let Some(d) = info.def.filter(|_| file < self.asts.len()) else { return at };
        let ast = &self.asts[file];
        let end = match &ast.def(d).kind {
            crate::ast::DefKind::Fun(f) => f.body.map(|b| ast.expr_span(b).end),
            crate::ast::DefKind::Val { rhs, .. } => rhs.map(|r| ast.expr_span(r).end),
            _ => None,
        };
        Span { start: at.start, end: end.map_or(at.end, |e| e.max(at.end)) }
    }

    /// The new captures `added` of the class `c`, which had `n`, passed by each creation of it in
    /// the trees of `member` (its body, its defaults, and the classes of blocks inside it) and of
    /// the file's members where no member holds it, and by each subclass's call of its
    /// constructor: their places after the `n` it passed. The classes of blocks whose trees took
    /// such an argument.
    fn pass_captures(&mut self, c: ClassId, n: usize, added: &[SymId], member: Option<SymId>, file: FileId, skip: &[ClassId]) -> Vec<ClassId> {
        use crate::tir::TExpr;
        let mut roots: Vec<(Option<ClassId>, TExprId)> = Vec::new();
        match member {
            Some(m) => {
                if let Some(&f) = self.fun_of_sym.get(&m) {
                    let fun = &self.prog.funs[f.idx()];
                    roots.extend(fun.body.iter().chain(fun.defaults.iter().flatten()).map(|&r| (None, r)));
                }
                if let Some(&v) = self.val_init.get(&m) {
                    roots.push((None, v));
                }
            }
            None => {
                for f in self.prog.funs.iter() {
                    if self.syms.sym(f.sym).file == file {
                        roots.extend(f.body.iter().chain(f.defaults.iter().flatten()).map(|&r| (None, r)));
                    }
                }
            }
        }
        // The classes of blocks the member holds, by their places (the member's extent: its name to
        // the end of its trees); with no member, the file's.
        let span = member.map(|m| self.member_extent(m));
        let classes: Vec<crate::tir::TClass> = self
            .prog
            .classes
            .iter()
            .filter(|tc| {
                let info = self.syms.class(tc.id);
                !skip.contains(&tc.id) && info.file == file && (info.owner == crate::symbols::Owner::Local || self.in_lifted_class(info.owner)) && span.map_or(true, |s| s.start <= info.span.start && info.span.end <= s.end)
            })
            .cloned()
            .collect();
        for tc in &classes {
            roots.extend(self.class_roots(std::slice::from_ref(tc)).into_iter().map(|r| (Some(tc.id), r)));
        }
        let mut changed: Vec<ClassId> = Vec::new();
        let insert = |t: &mut Self, l: crate::ast::ListRef| -> crate::ast::ListRef {
            let mut items = t.prog.expr_list(l).to_vec();
            let at = n.min(items.len());
            let args: Vec<TExprId> = added.iter().map(|&s| t.prog.add(TExpr::Local(s))).collect();
            items.splice(at..at, args);
            t.prog.list(&items)
        };
        for (owner, root) in roots {
            let found: Vec<TExprId> = self.prog.descendants(root).collect();
            for e in found {
                let rewritten = match self.prog.expr(e) {
                    TExpr::New(k, args) if k == c => TExpr::New(k, insert(self, args)),
                    TExpr::NewVia(s, args) if self.syms.sym(s).owner == crate::symbols::Owner::Class(c) => TExpr::NewVia(s, insert(self, args)),
                    _ => continue,
                };
                self.prog.exprs[e.idx()] = rewritten;
                if let Some(k) = owner.filter(|&k| k != c && !changed.contains(&k)) {
                    changed.push(k);
                }
            }
        }
        // A subclass's call of the class's constructor, its implicit `super()` among them, which
        // passes the captures from the subclass's own (the free variables of a superclass
        // constructor call are the subclass's, `Dependencies` in dotty's `LambdaLift`).
        for tc in &classes {
            if self.syms.class(tc.id).superclass != Some(c) || tc.id == c {
                continue;
            }
            let args = match tc.parent_args {
                Some(args) => args,
                None => self.prog.list(&[]),
            };
            let l = insert(self, args);
            if let Some(t) = self.checked_class_mut(tc.id) {
                t.parent_args = Some(l);
            }
            if !changed.contains(&tc.id) {
                changed.push(tc.id);
            }
        }
        changed
    }

    /// The place of a call deferred now in the phase's walk (`WalkOrder`): the innermost member
    /// whose body is typed (a local definition is its member's tree), else the item's.
    pub(super) fn walk_place(&mut self) -> WalkOrder {
        let typed = self.inlining.typed;
        self.inlining.typed += 1;
        // The innermost member typing whose tree is its own: a local definition's is its block's,
        // and so is a member's of a class of a block, which `LambdaLift` lifts after the phase.
        let mut member = None;
        for &s in self.bodies_in_progress.iter().rev() {
            let info = self.syms.sym(s);
            if info.owner != crate::symbols::Owner::Local && !self.in_lifted_class(info.owner) {
                member = Some((s, info.file, info.span.start));
                break;
            }
        }
        // Inside an expansion the walk's place is the outermost call's.
        let file = self.inline.sites.first().map_or(self.env.file, |s| s.file);
        match (member, self.inlining.statement_place) {
            (Some((s, file, place)), _) => WalkOrder { file: file.0, group: self.walk_group(file, place), typed, unit: Unit::Member(s) },
            (None, Some((c, place))) => {
                let place = match place {
                    StatementPlace::At(p) => p,
                    StatementPlace::Expr(f, e) => self.ast(f).expr_span(e).start,
                    StatementPlace::Def(f, d) => self.ast(f).def(d).span.start,
                };
                WalkOrder { file: file.0, group: self.walk_group(file, place), typed, unit: Unit::Template(c) }
            }
            (None, None) => WalkOrder { file: file.0, group: self.walk_group(file, self.inlining.item_place), typed, unit: Unit::Item },
        }
    }

    /// The place in its file's walk of the unit at `place` (`WalkOrder::group`): the offset, after
    /// every other statement of the file where the top-level definition that holds it is one
    /// dotty's desugaring wraps in the file's `<file>$package` object, which `Desugar.packageDef`
    /// places after the others (`topStats :+ grouped`). `isTopLevelDef` takes a val, a def, a
    /// type alias, a given and an implicit class or object; an object named as such a type (an
    /// opaque type's companion) goes with it.
    fn walk_group(&mut self, file: FileId, place: u32) -> u32 {
        let ast = self.ast(file);
        let key = ast as *const crate::ast::Ast as usize;
        let (k, start, end, wrapped) = self.inlining.walk_top;
        if k == key && start <= place && place < end {
            return if wrapped { place | WRAPPED } else { place };
        }
        let holds = |d: &&crate::ast::DefId| {
            let r = ast.def_range(**d);
            r.start <= place && place < r.end
        };
        let Some(&d) = ast.top_level.iter().find(holds) else { return place };
        let wrapped = wrapped_top_level(ast, d);
        let r = ast.def_range(d);
        self.inlining.walk_top = (key, r.start, r.end, wrapped);
        if wrapped {
            place | WRAPPED
        } else {
            place
        }
    }

    /// Whether a definition owned by `owner` is in a class of a block, which is lifted with what
    /// it captures (`Worker::end_unit`).
    pub(super) fn in_lifted_class(&self, owner: crate::symbols::Owner) -> bool {
        // The owner chain ends at a package or a block, as dotty's walk of a symbol's owners does.
        let mut owner = owner;
        while let crate::symbols::Owner::Class(c) = owner {
            let info = self.syms.class(c);
            if info.owner == crate::symbols::Owner::Local {
                return true;
            }
            owner = info.owner;
        }
        false
    }

    /// The worker's environment as the kept calls hold it (`shared_site_env`), compared with the
    /// last one's record before a copy is made: the calls of a block share it.
    pub(super) fn shared_worker_env(&mut self) -> std::sync::Arc<super::Env> {
        if let Some(last) = &self.inlining.last_env {
            if **last == self.env {
                return last.clone();
            }
        }
        let env = std::sync::Arc::new(self.env.clone());
        self.inlining.last_env = Some(env.clone());
        env
    }

    /// `env` as the kept calls hold it: the last one's record where its scopes are the same.
    pub(super) fn shared_site_env(&mut self, env: super::Env) -> std::sync::Arc<super::Env> {
        match &self.inlining.last_env {
            Some(last) if **last == env => last.clone(),
            _ => {
                let env = std::sync::Arc::new(env);
                self.inlining.last_env = Some(env.clone());
                env
            }
        }
    }

    /// Whether the typing reported an error, after which no phase runs (dotty's `Run` runs a
    /// phase only while the reporter has no error); an error of a later phase scalac reports
    /// (`Diagnostic::late`) is none.
    fn typing_failed(&self) -> bool {
        !self.diags.syntax.is_empty() || self.diags.items.iter().any(|d| !d.is_warning && !d.late)
    }

    /// The inlining phase: every kept call expanded at its site, in the walk's order, its node
    /// replaced in place by its expansion; then what reads the expansions: the folds the calls
    /// held back, the erased values' check, the `@tailrec` checks.
    pub fn inlining_phase(&mut self) {
        // What is still pending outside every unit, no attempt being open, is kept too.
        for p in self.take_pending_from(0) {
            self.keep_for_phase(p);
        }
        self.keep_folds_for_phase();
        // The files a macro of the run suspended at typing wait for the round after the first, typed
        // again: what their typing reported and kept goes with it, as dotty's `TyperPhase` drops a
        // suspended unit's tree.
        self.take_typing_suspended();
        if self.inlining.is_empty() && self.inlining.suspended.is_empty() {
            return;
        }
        if self.typing_failed() {
            // The calls depend on their callees' bodies, whether a later build expands them or
            // an edit of a callee's file is what makes it one.
            for k in &self.inlining.kept {
                let Some(c) = &k.call else { continue };
                let def_file = self.inline_forwarded_file(c.call.sym).unwrap_or(self.syms.sym(c.call.sym).file);
                if def_file != c.env.file {
                    self.inline_deps.entry(def_file).or_default().insert(c.env.file, ());
                }
            }
            return;
        }
        self.inlining.running = true;
        let errors = self.diags.items.iter().filter(|d| !d.is_warning).count();
        let erased_mark = self.erased_values.len();
        // The order of dotty's runs: every file in the walk's order, as `Phase.runOn` takes the
        // units; an expansion that meets a macro of the run (its splice names a definition of a
        // file of the run, `Inliner.expandMacro`'s `macroDependencies`) suspends its file's unit
        // (`CompilationUnit.suspend`), whose round is given up whole (`Phase.runOn` resets its
        // state), as the typer suspended the files whose typing met one (`typing_suspends`); the
        // suspended files are typed again from their trees and go through the phase in a run of
        // their own (`Run.compileSuspendedUnits`, repeated by `Driver.finish` while units
        // suspend), once the phases after `Inlining` are done with the files no suspended call is
        // in: the trees the macro's run reaches complete. A run no file passes is dotty's cyclic
        // dependency (`Run.checkSuspendedUnits`).
        let mut kept = std::mem::take(&mut self.inlining.kept);
        if !kept.windows(2).all(|p| p[0].at <= p[1].at) {
            kept.sort_by_cached_key(|k| k.at.clone());
        }
        self.inlining.kept = kept;
        let outer_env = self.env.clone();
        let mut classes = self.local_classes();
        self.inlining.next_tag = self.inlining.workers;
        self.inlining.current_run = None;
        self.inlining.suspendable = self.defines_macro_of_run();
        self.inlining.splices_name_run = self.inlining.suspendable;
        let defines_macro = self.inlining.suspendable || self.defines_program_macro();
        // The macros' runs are watched for the state they share from the phase's start where a
        // round may fork (a later round's units are among the first's), as the body phase's are
        // from the build's: each worker's interpreter has a heap of its own, and a run that changes
        // what another made gives the build to one worker (`interp::watch`).
        let mut groups = self.phase_groups();
        let watched = self.phase_workers(groups.len(), self.inlining.kept.len()) > 1;
        if watched {
            crate::interp::watch(true);
        }
        let mut cyclic: Vec<Kept> = Vec::new();
        loop {
            self.expand_round(groups, &classes);
            // A worker of the phase gave the build to one worker (`need_serial`), which types it again.
            if self.serial.is_needed() {
                self.inlining.kept.clear();
                self.inlining.suspended.clear();
                self.inlining.suspendable = false;
                self.inlining.splices_name_run = false;
                self.inlining.current_run = None;
                self.inlining.running = false;
                return;
            }
            if self.inlining.suspended.is_empty() {
                break;
            }
            self.inlining.installed = None;
            self.env = outer_env.clone();
            let waiting: FxMap<u32, ()> = self.inlining.suspended.iter().map(|&f| (f, ())).collect();
            // No further run where an error was reported (`Driver.finish`).
            let errored = self.diags.items.iter().filter(|d| !d.is_warning).count() > errors;
            let passed = match self.inlining.current_run.as_deref() {
                Some(run) => run.keys().any(|f| !waiting.contains_key(f)),
                None => (0..self.files.len()).any(|f| self.program_file(FileId(f as u32)) && !waiting.contains_key(&(f as u32))),
            };
            if errored || !passed {
                if !errored {
                    self.cyclic_macro_dependencies(&waiting);
                }
                break;
            }
            self.lift_deferred_traits(Some(&waiting));
            self.settle_lifted_captures(Some(&waiting));
            self.inlining.current_run = Some(std::sync::Arc::new(waiting));
            self.prog.runs += 1;
            // The files given up typed again from their trees, their transparent macros run again
            // where the run's other files are done (`Run.compileSuspendedUnits`), their calls kept.
            self.inlining.kept.clear();
            let mut suspended: Vec<FileId> = std::mem::take(&mut self.inlining.suspended).into_iter().map(FileId).collect();
            if !suspended.is_empty() {
                suspended.sort_unstable();
                super::substitution::counts::suspended_files(0, suspended.len());
                self.inlining.running = false;
                self.retype_suspended(&suspended);
                self.inlining.running = true;
                // The walk of the next round takes the retyped trees, their classes of blocks and
                // their templates' nodes, as dotty's `InliningTreeMap` walks the unit's tree its
                // later run typed again.
                classes = self.local_classes();
                self.inlining.template_ranks.clear();
                // What a macro's run made executable is made again from the retyped bodies.
                self.inlining.checked_funs.clear();
                self.inlining.checked_classes.clear();
                self.inlining.checked_funs_typing.clear();
                self.inlining.checked_classes_typing.clear();
                if let Some(memo) = self.inlining.reflect.take() {
                    super::substitution::counts::reflected(memo.asked, memo.answered);
                }
                for p in self.take_pending_from(0) {
                    self.keep_for_phase(p);
                }
                self.keep_folds_for_phase();
                self.env = outer_env.clone();
                // A file that meets a macro of this run again at typing waits for the next round.
                self.take_typing_suspended();
            }
            // The walk's order of the units, each put in its walk's order again as the next round
            // takes it (`expand_groups`), as dotty's later run walks the suspended unit again.
            let mut later = std::mem::take(&mut self.inlining.kept);
            if !later.windows(2).all(|w| w[0].at <= w[1].at) {
                later.sort_by_cached_key(|k| k.at.clone());
            }
            // The later run passes the phases' gates as the first did (`Phase.isRunnable`): an error
            // its typing reported runs no expansion, the calls kept as a build with an error keeps them.
            if self.typing_failed() {
                cyclic = later;
                break;
            }
            self.inlining.kept = later;
            // A later run's expansion also suspends where a macro's run reaches a file of the
            // run (`run_suspends`), which a macro of the program's may do whatever its splice
            // names, as dotty's interpreter suspends on any class of the current run.
            self.inlining.splices_name_run = self.defines_macro_of_run();
            self.inlining.suspendable = defines_macro || self.inlining.splices_name_run;
            groups = self.phase_groups();
        }
        self.inlining.suspended.clear();
        self.inlining.suspendable = false;
        self.inlining.splices_name_run = false;
        self.inlining.current_run = None;
        if watched {
            crate::interp::watch(std::env::var_os("TEQ_MACRO_CENSUS").is_some());
        }
        self.inlining.installed = None;
        self.env = outer_env;
        self.lift_deferred_traits(None);
        self.settle_lifted_captures(None);
        self.inlining.kept.clear();
        self.inlining.index.clear();
        // The calls of the files left suspended (a cyclic dependency, an error) stay kept, as a build
        // with an error keeps its calls for a session's next build.
        if !cyclic.is_empty() {
            for k in &cyclic {
                let Some(c) = &k.call else { continue };
                let def_file = self.inline_forwarded_file(c.call.sym).unwrap_or(self.syms.sym(c.call.sym).file);
                if def_file != c.env.file {
                    self.inline_deps.entry(def_file).or_default().insert(c.env.file, ());
                }
            }
            self.inlining.kept = cyclic;
        }
        self.inlining.last_env = None;
        self.inlining.checked_funs = Vec::new();
        self.inlining.checked_funs_typing = FxMap::default();
        self.inlining.checked_classes = Vec::new();
        self.inlining.checked_classes_typing = FxMap::default();
        self.fold_kept_folds();
        let unboxes = std::mem::take(&mut self.inlining.unboxes);
        self.inline_unboxes.extend(unboxes);
        if !self.inline_unboxes.is_empty() {
            self.apply_inline_unboxes();
        }
        // The phases after `Inlining` run where it reported no error: `Erasure`'s check of the
        // erased values, `TailRec`'s.
        if self.diags.items.iter().filter(|d| !d.is_warning).count() == errors {
            self.check_erased_after_phase(erased_mark);
            for (sym, f, file, span) in std::mem::take(&mut self.inlining.tailrec) {
                let at = std::mem::replace(&mut self.env.file, file);
                self.check_tailrec(sym, f, span);
                self.env.file = at;
            }
        }
        self.inlining.erased.clear();
        self.inlining.tailrec.clear();
        self.inlining.template_ranks.clear();
        self.inlining.walk_top = Default::default();
        if let Some(memo) = self.inlining.reflect.take() {
            super::substitution::counts::reflected(memo.asked, memo.answered);
        }
        self.inlining.running = false;
    }

    /// The files a macro of the run suspended at typing, waiting for the next round: what their typing
    /// kept goes with it, as dotty's `TyperPhase` drops a suspended unit's tree. What stands of what
    /// the typing reported is what dotty's reporter holds when `TyperPhase.typeCheck` catches the
    /// `SuspendException` (`keep_typing_reports`). An error among it keeps the phase from running
    /// (`typing_failed`).
    fn take_typing_suspended(&mut self) {
        let files = std::mem::take(&mut self.inlining.typing_suspended);
        let cuts = std::mem::take(&mut self.inlining.typing_cuts);
        let demands = std::mem::take(&mut self.inlining.demands);
        self.inlining.item_demands = 0;
        self.inlining.last_demand = Default::default();
        if files.is_empty() {
            return;
        }
        super::substitution::counts::suspended_files(files.len(), 0);
        self.keep_typing_reports(&files, &cuts, &demands);
        // The checks of the matches the files' typing deferred go with it: `Phase.runOn` keeps a
        // unit that suspended out of the phases after the typer, `PatternMatcher`'s
        // `SpaceEngine.checkMatch` among them, and the later run's typing defers them again.
        self.deferred_matches.retain(|m| !files.contains(&m.file.0));
        let reported = std::mem::take(&mut self.inlining.suspension_reports);
        self.diags.extend(reported);
        if self.inlining.kept.iter().any(|k| files.contains(&k.at.file)) {
            self.inlining.kept.retain(|k| !files.contains(&k.at.file));
        }
        for f in files {
            if !self.inlining.suspended.contains(&f) {
                self.inlining.suspended.push(f);
            }
        }
    }

    /// Which records stand of what the typing reported, the units `files` suspended at typing at
    /// `cuts` with the completions `demands` lists (`keep_typing_reports`'s rule, `Standing::stands`):
    /// what the reports' dropping keeps, and what the history a decision reads counts of a record
    /// made before it (`merge::Worker::history_mismatch`), as dotty's reporter holds it.
    pub(super) fn typing_standing(&self, files: &[u32], cuts: &[TypingCut], demands: &[(DiagOwner, DiagOwner)]) -> Standing {
        // By file: not suspended, or the key of the item its typing was cut in, the earliest of its
        // cuts; a unit no cut names (none of its definitions was in progress) keeps no item whole.
        let mut cut_at = vec![NOT_SUSPENDED; self.asts.len()];
        for &f in files {
            cut_at[f as usize] = cuts.iter().filter(|c| c.file == f).map(|c| c.key).min().unwrap_or(0);
        }
        let mut standing = Standing { cut_at, item_of: FxMap::default(), demanded: FxMap::default(), at_cut: FxMap::default() };
        // The completions those demanded, and the ones the definitions in progress at a cut in
        // the cut item had demanded by then, closed under the demands.
        let mut onward: FxMap<(u32, u32), Vec<DiagOwner>> = FxMap::default();
        let mut reached: Vec<DiagOwner> = Vec::new();
        for &(from, to) in demands {
            if !standing.suspended(to) {
                continue;
            }
            if standing.whole(self, from) {
                reached.push(to);
            } else {
                onward.entry(from.key()).or_default().push(to);
            }
        }
        let first_cuts: Vec<&TypingCut> = cuts.iter().filter(|c| standing.cut_at.get(c.file as usize) == Some(&c.key)).collect();
        reached.extend(first_cuts.iter().flat_map(|c| c.demanded.iter().copied()).filter(|&o| standing.suspended(o)));
        while let Some(o) = reached.pop() {
            if standing.whole(self, o) || standing.demanded.insert(o.key(), ()).is_some() {
                continue;
            }
            if let Some(next) = onward.get(&o.key()) {
                reached.extend(next.iter().copied());
            }
        }
        // What the definitions in progress at the cuts had reported by then, by identity.
        standing.at_cut = first_cuts.iter().flat_map(|c| c.reported.iter()).map(|&o| (o, ())).collect();
        standing
    }

    /// Of what the typing reported, what dotty's reporter holds when `TyperPhase.typeCheck` catches
    /// the units' `SuspendException`s, as its traversal decides it. A body diagnostic stands where
    /// its owner (`Diagnostic::owner`, the definition whose typing reported it) is:
    /// - a definition of a unit that did not suspend;
    /// - of a suspended unit, one of the items before the one its typing was cut in, in the order
    ///   dotty's typer meets them (`desugared_key`: `Desugar.packageDef` places the file's package
    ///   object after its classes and objects), typed whole before the cut;
    /// - a completion one of those demanded, or one the definitions in progress at the cut had
    ///   demanded by then, transitively, wherever it lives (`Inlining::demands`): Namer's
    ///   completers type a definition when it is read (`Namer.Completer.complete`) and nothing
    ///   retracts what they report.
    ///
    /// Of the definitions in progress at the cut, what they had reported by then stands
    /// (`TypingCut`). Any other owner's reports go: a typing dotty's traversal never reaches, which
    /// a worker did unaware of the suspension.
    fn keep_typing_reports(&mut self, files: &[u32], cuts: &[TypingCut], demands: &[(DiagOwner, DiagOwner)]) {
        let mut standing = self.typing_standing(files, cuts, demands);
        let stands: Vec<bool> = (0..self.diags.items.len()).map(|i| standing.stands(self, &self.diags.items[i])).collect();
        // `TEQ_UNPRESENTED=1`: what goes, for a trace of what the history reads counted before.
        if std::env::var_os("TEQ_UNPRESENTED").is_some_and(|v| v == "1") {
            for (d, _) in self.diags.items.iter().zip(&stands).filter(|(_, &s)| !s) {
                let why = if d.unwound.0 { "made while its item unwound" } else { "a typing dotty's traversal never reaches" };
                eprintln!("teq: dropped at its unit's suspension ({}): {} at file {} {}..{}: {}", why, if d.is_warning { "warning" } else { "error" }, d.file.0, d.span.start, d.span.end, d.msg.lines().next().unwrap_or(""));
            }
        }
        let mut i = 0;
        self.diags.items.retain(|_| {
            i += 1;
            stands[i - 1]
        });
        // A bounds check deferred until every class is complete reports as its owner would have
        // (`DeferredBound::owner`): one owned by a completion the traversal never reached goes
        // with it.
        let deferred = std::mem::take(&mut self.deferred_bounds);
        let kept: Vec<_> = deferred.into_iter().filter(|b| b.owner.is_none() || standing.demanded.contains_key(&b.owner.key()) || standing.whole(self, b.owner)).collect();
        self.deferred_bounds = kept;
    }

    /// The key of the top-level definition of `file` that holds `place`, in the order dotty's typer
    /// meets the file's statements (`top_level_key`); a place no top-level definition holds is
    /// before them all.
    pub(super) fn desugared_key(&self, file: FileId, place: u32) -> u32 {
        let place = place & !DiagOwner::SIG;
        let ast = self.ast(file);
        let holds = |d: &&crate::ast::DefId| {
            let r = ast.def_range(**d);
            r.start <= place && place < r.end
        };
        if let Some(&d) = ast.top_level.iter().find(holds) {
            return top_level_key(ast, d);
        }
        // A top-level export clause is a member of the file's package object
        // (`Desugar.isTopLevelDef`: `Export`), placed after the file's classes and objects.
        if ast.top_exports.iter().any(|e| e.span.start <= place && place < e.span.end) {
            return place | WRAPPED;
        }
        0
    }

    /// The owner of a top-level export clause of `file` at `place`: the clause is a member of
    /// the file's package object (`Desugar.isTopLevelDef`), whose completer makes its forwarders
    /// (`Namer.processExports`), owned at the clause.
    pub(super) fn top_export_owner(&self, file: FileId, place: u32) -> Option<DiagOwner> {
        let ast = self.ast(file);
        ast.top_exports.iter().find(|e| e.span.start <= place && place < e.span.end).map(|e| DiagOwner::sig(file, e.span.start))
    }

    /// The definitions whose typing owns what it reports (`Diagnostic::owner`): a member's body,
    /// a block's local definition and a lifted class's member being their enclosing member's.
    pub(super) fn body_owner(&self, sym: SymId) -> Option<DiagOwner> {
        let info = self.syms.sym(sym);
        if info.owner == crate::symbols::Owner::Local || self.in_lifted_class(info.owner) {
            return None;
        }
        Some(DiagOwner::new(info.file, info.span.start))
    }

    /// The completion `owner` begins on this worker (`Inlining::completing`): what it reports is
    /// its own until `leave_owner` with what this returned.
    pub(super) fn enter_owner(&mut self, owner: DiagOwner) -> DiagOwner {
        self.inlining.completing.push(owner);
        self.inlining.completing_marks.push(self.diags.items.len());
        std::mem::replace(&mut self.diags.owner, owner)
    }

    /// The completion `enter_owner` began is over: what is reported is the enclosing one's again,
    /// which demanded it where `demanded` (`Inlining::demands`).
    pub(super) fn leave_owner(&mut self, outer: DiagOwner, demanded: bool) {
        let done = std::mem::replace(&mut self.diags.owner, outer);
        self.inlining.completing.pop();
        // A completion that reported is told where it was made (`Worker::tell`).
        let mark = self.inlining.completing_marks.pop().unwrap_or(usize::MAX);
        if self.inlining.track_meetings && self.diags.items.len() > mark && !done.is_none() {
            self.tell(super::merge::Event::Completion(done.key()));
        }
        if demanded {
            self.note_demand(outer, done);
        }
    }

    /// The typing switched to the body of `sym` (`body_owner`): what it reports is that body's,
    /// until `left_body` with what this returned.
    pub(super) fn entered_body(&mut self, sym: SymId) -> Option<DiagOwner> {
        let owner = self.body_owner(sym)?;
        Some(self.enter_owner(owner))
    }

    /// The typing of a body `entered_body` switched to over: what is reported is the enclosing
    /// completion's again, which demanded the body (`Inlining::demands`).
    pub(super) fn left_body(&mut self, outer: Option<DiagOwner>) {
        if let Some(outer) = outer {
            self.leave_owner(outer, true);
        }
    }

    /// The completion of the signature of `sym` (`DiagOwner::sig`): a program member's, whose
    /// body has an owner of its own (`body_owner`).
    pub(super) fn sig_owner(&self, sym: SymId) -> Option<DiagOwner> {
        let info = self.syms.sym(sym);
        // A default's getter, at its default (`super::getters`).
        if info.def.is_none() && self.getters.link.contains_key(&sym) {
            return Some(DiagOwner::sig(info.file, info.span.start));
        }
        // An export's forwarder: made by the completer of the class or the package object whose
        // clause it is (`Namer.processExports`), which a read of it demands.
        if info.def.is_none() {
            if let Some(f) = self.syms.forward_of(sym) {
                return match info.owner {
                    crate::symbols::Owner::Class(c) => self.class_owner(c),
                    crate::symbols::Owner::Package(_) => self.top_export_owner(info.file, f.clause.start),
                    crate::symbols::Owner::Local => None,
                };
            }
        }
        if info.owner == crate::symbols::Owner::Local || info.def.is_none() || !self.program_file(info.file) || self.in_lifted_class(info.owner) {
            return None;
        }
        Some(DiagOwner::sig(info.file, info.span.start))
    }

    /// The walk reached the body of `sym` and found it typed, or an expansion needed it: a demand
    /// of the definition being typed (`note_demand`).
    pub(super) fn note_reached(&mut self, sym: SymId) {
        if !self.inlining.notes_demands() {
            return;
        }
        if let Some(to) = self.body_owner(sym) {
            self.note_demand(self.diags.owner, to);
        }
    }

    /// A read of the signature of `sym`, at `completed_sig`'s every return: where its completion
    /// recorded a demand or a report, the completion in progress demanded it, whichever worker
    /// completed it first, as a reader of a completed denotation depends on what its completer
    /// demanded (`Denotation.info`). Most reads name a definition whose completion recorded
    /// nothing (a library's, a declared one), known once asked (`Inlining::noted`).
    #[inline]
    pub(super) fn demand_sig_read(&mut self, sym: SymId) {
        if self.inlining.noted.get(sym.0 as usize) == Some(&2) {
            return;
        }
        self.note_read(sym);
    }

    #[inline(never)]
    fn note_read(&mut self, sym: SymId) {
        let from = self.diags.owner;
        if from.is_none() || self.inlining.last_demand == (from.key(), sym.0) {
            return;
        }
        // A worker's own symbol is a local definition's, which demands nothing of its own.
        if sym.0 as usize >= crate::arena::LOCAL_BASE as usize {
            return;
        }
        let i = sym.0 as usize;
        if self.inlining.noted.len() <= i {
            self.inlining.noted.resize(i + 1, 0);
        }
        if self.inlining.noted[i] == 0 {
            // Known once the completion is over: a completion under way may record more.
            if !self.syms.sym(sym).state().get().is_complete() {
                return self.note_read_owner(from, sym);
            }
            let noted = self.syms.sym(sym).completion_noted && self.sig_owner(sym).is_some();
            self.inlining.noted[i] = if noted { 1 } else { 2 };
        }
        if self.inlining.noted[i] != 1 {
            return;
        }
        self.note_read_owner(from, sym);
    }

    fn note_read_owner(&mut self, from: DiagOwner, sym: SymId) {
        let Some(to) = self.sig_owner(sym) else { return };
        self.inlining.last_demand = (from.key(), sym.0);
        self.note_demand(from, to);
    }

    /// `from`'s typing demanded `to`'s (`Inlining::demands`).
    pub(super) fn note_demand(&mut self, from: DiagOwner, to: DiagOwner) {
        if self.inlining.track_meetings {
            self.meet(super::merge::Event::Completion(to.key()));
        }
        if !self.inlining.track_demands || from.is_none() || from.key() == to.key() {
            return;
        }
        self.inlining.demands.push((from, to));
    }

    /// The typing of a build or a session's retype begins: the demands are recorded where a file
    /// of the program quotes or splices, whose macro may suspend a unit at typing.
    pub(crate) fn begin_demands(&mut self) {
        self.inlining.demands.clear();
        self.inlining.item_demands = 0;
        self.inlining.typing_cuts.clear();
        self.inlining.last_demand = Default::default();
        self.inlining.noted.clear();
        self.inlining.track_demands = (0..self.asts.len() as u32).map(FileId).any(|f| self.ast(f).has_quotes && self.program_file(f));
        self.inlining.notes = self.inlining.track_demands || self.inlining.track_meetings;
    }

    /// A run whose every file suspended again: dotty's `Run.checkSuspendedUnits` ("no further
    /// progress can be made"), its files in the walk's order. scalac's last line names its option
    /// `-Xprint-suspension`, which teq has not.
    fn cyclic_macro_dependencies(&mut self, waiting: &FxMap<u32, ()>) {
        let mut files: Vec<u32> = waiting.keys().copied().collect();
        files.sort_unstable();
        let paths: Vec<String> = files.iter().map(|&f| self.source(FileId(f)).path.clone()).collect();
        let place = match paths.as_slice() {
            [one] => format!("in {}.", one),
            _ => format!("among\n\n  {}\n", paths.join(", ")),
        };
        let msg = format!("Cyclic macro dependencies {}\nCompilation stopped since no further progress can be made.\n\nTo fix this, place macros in one set of files and their callers in another.", place);
        self.diags.error(crate::source::NO_FILE, Span::default(), msg);
    }

    /// The classes of blocks of the program by file, each with its place, which the walk of a
    /// unit's tree takes where they stand in a block (`order_unit`).
    fn local_classes(&self) -> FxMap<u32, Vec<(u32, u32, ClassId)>> {
        let mut by_file: FxMap<u32, Vec<(u32, u32, ClassId)>> = FxMap::default();
        for tc in self.prog.classes.iter() {
            let info = self.syms.class(tc.id);
            if info.owner == crate::symbols::Owner::Local {
                by_file.entry(info.file.0).or_default().push((info.span.start, info.span.end, tc.id));
            }
        }
        for v in by_file.values_mut() {
            v.sort_unstable();
        }
        by_file
    }

    /// The calls of `inlining.kept[range]`, one unit's, in the order the walk of the unit's typed
    /// tree meets them, as `InliningTreeMap.transform` does: a node's children first, in the
    /// order of the tree (a call's receiver, its arguments in their clauses' order, a given of a
    /// clause before the arguments of the clause after it), a block's statements in turn with a
    /// class of the block where it stands among them, an anonymous class where it is made, a
    /// template's parts in their order. A call the walk does not meet keeps its typing's place
    /// after those it meets.
    fn order_unit(&mut self, range: std::ops::Range<usize>, classes: &FxMap<u32, Vec<(u32, u32, ClassId)>>) -> bool {
        if range.len() < 2 {
            return false;
        }
        let unit = self.inlining.kept[range.start].at.unit;
        let file = self.inlining.kept[range.start].at.file;
        let classes = classes.get(&file).map_or(&[][..], |v| v.as_slice());
        let ranks: Vec<u32> = match unit {
            // A class's statements, each a unit, share the template's walk: made once, at the
            // first statement's, before any of the template's calls expands.
            Unit::Template(c) => {
                if !self.inlining.template_ranks.contains_key(&c) {
                    let roots = self.unit_trees(unit);
                    let mut walk = RankWalk { index: Vec::new(), ranks: Vec::new(), next: 0, classes, visited: Vec::new(), all: Some(FxMap::default()) };
                    for root in roots {
                        self.rank_expr(&mut walk, root);
                    }
                    self.inlining.template_ranks.insert(c, walk.all.unwrap_or_default());
                }
                let ranked = &self.inlining.template_ranks[&c];
                self.inlining.kept[range.clone()].iter().map(|k| ranked.get(&k.node).copied().unwrap_or(u32::MAX)).collect()
            }
            _ => {
                let roots = self.unit_trees(unit);
                if roots.is_empty() {
                    return false;
                }
                let mut index: Vec<(TExprId, usize)> = self.inlining.kept[range.clone()].iter().enumerate().map(|(i, k)| (k.node, i)).collect();
                index.sort_unstable();
                let mut walk = RankWalk {
                    index,
                    ranks: vec![u32::MAX; range.len()],
                    next: 0,
                    classes,
                    visited: Vec::new(),
                    all: None,
                };
                for root in roots {
                    self.rank_expr(&mut walk, root);
                }
                walk.ranks
            }
        };
        // The unit's calls reordered where they stand, the calls after them left in place; whether
        // one moved.
        let unit = &mut self.inlining.kept[range];
        let mut order: Vec<usize> = (0..unit.len()).collect();
        order.sort_by(|&a, &b| ranks[a].cmp(&ranks[b]).then_with(|| unit[a].at.cmp(&unit[b].at)));
        if order.iter().enumerate().all(|(i, &o)| i == o) {
            return false;
        }
        for i in 0..order.len() {
            let mut at = i;
            while order[at] != usize::MAX {
                let from = std::mem::replace(&mut order[at], usize::MAX);
                if from == i {
                    break;
                }
                unit.swap(at, from);
                at = from;
            }
        }
        true
    }

    /// The roots of `unit`'s typed tree in the walk's order.
    fn unit_trees(&mut self, unit: Unit) -> Vec<TExprId> {
        match unit {
            Unit::Member(s) => {
                let mut roots = Vec::new();
                if let Some(&f) = self.fun_of_sym.get(&s) {
                    let fun = &self.prog.funs[f.idx()];
                    roots.extend(fun.body);
                    roots.extend(fun.defaults.iter().flatten().copied());
                }
                if let Some(&v) = self.val_init.get(&s) {
                    roots.push(v);
                }
                roots
            }
            Unit::Template(c) => self.template_roots(c, false),
            Unit::Item => Vec::new(),
        }
    }

    /// The trees of the class `c`'s template in their order: its header (the parents' arguments
    /// and what they evaluate first, its parameters' defaults), then its statements and, with
    /// `members`, its methods' bodies by their places.
    fn template_roots(&mut self, c: ClassId, members: bool) -> Vec<TExprId> {
        use crate::tir::{TInit, TStmt};
        let Some(tc) = self.checked_class(c).cloned() else { return Vec::new() };
        let mut roots = Vec::new();
        for &st in self.prog.stmt_list(tc.parent_prelude) {
            if let TStmt::Val(_, x) | TStmt::Expr(x) | TStmt::Pat(_, x) = st {
                roots.push(x);
            }
        }
        roots.extend(tc.parent_args.map_or(&[][..], |l| self.prog.expr_list(l)).iter().copied());
        let mut parts: Vec<(u32, TExprId)> = Vec::new();
        for init in &tc.init {
            match *init {
                TInit::Parent(_, call) => roots.extend(self.prog.expr_list(call.args).iter().copied()),
                TInit::Field(s, x) => parts.push((self.syms.sym(s).span.start, x)),
                TInit::Stmt(x) => parts.push((self.prog.span_of(x).map_or(0, |(_, sp)| sp.start), x)),
            }
        }
        roots.extend(tc.ctor_defaults.iter().flatten().copied());
        if members {
            for &f in tc.methods.iter().chain(&tc.ctors) {
                let fun = &self.prog.funs[f.idx()];
                let at = self.syms.sym(fun.sym).span.start;
                parts.extend(fun.body.iter().chain(fun.defaults.iter().flatten()).map(|&r| (at, r)));
            }
        }
        parts.sort_by_key(|p| p.0);
        roots.extend(parts.into_iter().map(|p| p.1));
        roots
    }

    /// The local classes of the walk's file that stand before `at` in the block from `from`, not
    /// walked yet, walked now: their templates, members and all.
    fn rank_classes_before(&mut self, walk: &mut RankWalk<'_>, from: u32, at: u32) {
        let (lo, hi) = (walk.classes.partition_point(|x| x.0 < from), walk.classes.partition_point(|x| x.0 < at));
        let due: Vec<ClassId> = walk.classes[lo..hi.max(lo)].iter().filter(|&&(_, _, c)| !walk.visited.contains(&c)).map(|&(_, _, c)| c).collect();
        for c in due {
            if walk.visited.contains(&c) {
                continue;
            }
            walk.visited.push(c);
            for root in self.template_roots(c, true) {
                self.rank_expr(walk, root);
            }
        }
    }

    /// The walk of the tree `e` in post-order, ranking the unit's calls it meets.
    fn rank_expr(&mut self, walk: &mut RankWalk<'_>, e: TExprId) {
        use crate::tir::{TExpr, TStmt};
        // Every call of the unit ranked: what the walk meets after them orders nothing.
        if walk.all.is_none() && walk.next as usize == walk.index.len() {
            return;
        }
        let list = |t: &mut Self, walk: &mut RankWalk<'_>, l: crate::ast::ListRef| {
            for i in 0..t.prog.expr_list(l).len() {
                let x = t.prog.expr_list(l)[i];
                t.rank_expr(walk, x);
            }
        };
        match self.prog.expr(e) {
            TExpr::Field(r, _) | TExpr::Unary(_, r) | TExpr::ToStr(r, _) | TExpr::TypeTest(r, _) | TExpr::Cast(r, ..) | TExpr::Index(r, _) | TExpr::Spread(r) | TExpr::Return(r) | TExpr::Throw(r, _) | TExpr::JsSelect(r, _) | TExpr::Splice(r) | TExpr::Lambda(_, r) => self.rank_expr(walk, r),
            TExpr::CallStatic(_, args) | TExpr::NewVia(_, args) | TExpr::StrConcat(args) | TExpr::Js(_, args) | TExpr::SeqLit(args) | TExpr::ArrayLit(args) | TExpr::ObjLit(args) | TExpr::Default(_, _, args) => list(self, walk, args),
            TExpr::New(c, args) => {
                // An anonymous class stands where it is made, its template before its creation.
                if self.syms.class(c).kind == crate::symbols::ClassKind::Anon && !walk.visited.contains(&c) {
                    walk.visited.push(c);
                    for root in self.template_roots(c, true) {
                        self.rank_expr(walk, root);
                    }
                }
                list(self, walk, args)
            }
            TExpr::CallMethod(r, _, args) | TExpr::CallClosure(r, args) => {
                self.rank_expr(walk, r);
                list(self, walk, args)
            }
            TExpr::If(c, t, f) => {
                self.rank_expr(walk, c);
                self.rank_expr(walk, t);
                if let Some(f) = f {
                    self.rank_expr(walk, f);
                }
            }
            TExpr::While(a, b) | TExpr::Assign(a, b) | TExpr::Prim(_, a, b) => {
                self.rank_expr(walk, a);
                self.rank_expr(walk, b);
            }
            TExpr::Block(stmts, v) => {
                let from = self.prog.span_of(e).map_or(0, |(_, s)| s.start);
                let stmts = self.prog.stmt_list(stmts).to_vec();
                for st in stmts {
                    let at = match st {
                        TStmt::Expr(x) | TStmt::Val(_, x) | TStmt::Pat(_, x) => self.prog.span_of(x).map(|(_, s)| s.start),
                        TStmt::Fun(f) => Some(self.syms.sym(self.prog.funs[f.idx()].sym).span.start),
                    };
                    if let Some(at) = at {
                        self.rank_classes_before(walk, from, at);
                    }
                    match st {
                        TStmt::Expr(x) | TStmt::Val(_, x) => self.rank_expr(walk, x),
                        TStmt::Pat(p, x) => {
                            self.rank_expr(walk, x);
                            self.rank_pat(walk, p);
                        }
                        TStmt::Fun(f) => {
                            let fun = &self.prog.funs[f.idx()];
                            let roots: Vec<TExprId> = fun.body.iter().chain(fun.defaults.iter().flatten()).copied().collect();
                            for r in roots {
                                self.rank_expr(walk, r);
                            }
                        }
                    }
                }
                if let Some(at) = self.prog.span_of(v).map(|(_, s)| s.start) {
                    self.rank_classes_before(walk, from, at);
                }
                self.rank_expr(walk, v);
            }
            TExpr::Match(scrutinee, cases) => {
                self.rank_expr(walk, scrutinee);
                self.rank_cases(walk, cases);
            }
            TExpr::Try(t) => {
                let t = self.prog.tries[t as usize].clone();
                self.rank_expr(walk, t.body);
                self.rank_cases(walk, t.cases);
                if let Some(f) = t.finalizer {
                    self.rank_expr(walk, f);
                }
            }
            _ => {}
        }
        if let Some(all) = walk.all.as_mut() {
            if self.prog.is_kept(e) && !all.contains_key(&e) {
                all.insert(e, walk.next);
                walk.next += 1;
            }
        } else if let Some(i) = self.prog.is_kept(e).then(|| walk.index.binary_search_by_key(&e, |&(n, _)| n).ok().map(|k| walk.index[k].1)).flatten() {
            if walk.ranks[i] == u32::MAX {
                walk.ranks[i] = walk.next;
                walk.next += 1;
            }
        }
    }

    fn rank_cases(&mut self, walk: &mut RankWalk<'_>, cases: crate::ast::ListRef) {
        let cases: Vec<crate::tir::TCase> = self.prog.cases[cases.range()].to_vec();
        for c in cases {
            self.rank_pat(walk, c.pat);
            if let Some(g) = c.guard {
                self.rank_expr(walk, g);
            }
            self.rank_expr(walk, c.body);
        }
    }

    fn rank_pat(&mut self, walk: &mut RankWalk<'_>, p: crate::tir::TPatId) {
        use crate::tir::TPat;
        match self.prog.pats[p.idx()] {
            TPat::Equals(e, _) => self.rank_expr(walk, e),
            TPat::Unapply(_, call, inner) => {
                self.rank_expr(walk, call);
                self.rank_pat(walk, inner);
            }
            TPat::Bind(_, Some(inner)) | TPat::Test(_, _, inner) => self.rank_pat(walk, inner),
            TPat::Class(_, _, _, pats) | TPat::Alt(pats) => {
                let pats = self.prog.pat_list(pats).to_vec();
                for q in pats {
                    self.rank_pat(walk, q);
                }
            }
            TPat::Seq(items, rest) => {
                let items = self.prog.pat_list(items).to_vec();
                for q in items {
                    self.rank_pat(walk, q);
                }
                if let Some(r) = rest {
                    self.rank_pat(walk, r);
                }
            }
            _ => {}
        }
    }

    /// A round of the phase: the units `groups` of `inlining.kept` over the workers where they pay
    /// for a fork, in the walk's order on one. Where the round forks, a file with a call that may
    /// run a macro of its run as its callee is written is one unit of the fork, whose worker
    /// expands its calls as one worker does (`whole_reaching_files`): the file's round is given up
    /// whole where an expansion meets such a macro, and only there, the same at every count of
    /// workers (dotty's `Inliner.expandMacro` decides at the splice it reaches).
    fn expand_round(&mut self, groups: Vec<std::ops::Range<usize>>, classes: &FxMap<u32, Vec<(u32, u32, ClassId)>>) {
        let mut workers = self.phase_workers(groups.len(), self.inlining.kept.len());
        let mut units = None;
        if workers > 1 && self.inlining.splices_name_run {
            let whole = self.whole_reaching_files(&groups);
            workers = self.phase_workers(whole.len(), self.inlining.kept.len());
            units = Some(whole);
        }
        if workers > 1 {
            let mark = self.diags.items.len();
            let given_up = self.expand_in_parallel(workers, units.unwrap_or(groups), classes.clone());
            if !self.serial.is_needed() && !given_up.is_empty() {
                self.give_up_suspended(mark, &given_up);
            }
            self.inlining.unit_reports.clear();
        } else {
            self.expand_groups(groups, classes);
        }
        self.inlining.installed = None;
    }

    /// After a forked round's merge, the round of each file a worker gave a unit of up given up
    /// whole, as one worker gives it up at the expansion that meets a macro of the run: what the
    /// file's units after the first one given up (`given_up`, by file) reported in the round
    /// (from `mark`) goes, as the walk of one worker does not reach them; what the units before
    /// it and that one up to its suspension reported stays (`roll_back`); and the file's
    /// expansions go with its typing at the next round's retype. Dotty suspends the whole unit
    /// at the splice its expansion reaches (`Inliner.expandMacro`), and its reporter keeps what
    /// the unit reported before.
    fn give_up_suspended(&mut self, mark: usize, given_up: &FxMap<u32, u32>) {
        // The records of the units that follow their file's first unit given up, by identity
        // (`source::Ordinal`): an equal record of a unit before it stays.
        let beyond: FxMap<u64, ()> = std::mem::take(&mut self.inlining.unit_reports)
            .into_iter()
            .filter(|&(u, file, _)| given_up.get(&file).is_some_and(|&first| u > first))
            .map(|(_, _, ordinal)| (ordinal, ()))
            .collect();
        let round = self.diags.items.split_off(mark);
        self.diags.extend(round.into_iter().filter(|d| !beyond.contains_key(&d.ordinal.0)));
    }

    /// The units `groups` of `inlining.kept` expanded in their order, each by the walk of its
    /// tree. Where the round's run defines a macro, each file's round is recorded from where it
    /// begins (`open_checkpoint`), and an expansion that meets a macro of the run gives the file's
    /// round up whole (`roll_back`), the file typed again for the next round
    /// (`Inlining::suspended`), as dotty's `Phase.runOn` resets a suspended unit and its later run
    /// compiles the whole unit again from its untyped tree. On one worker the file is its calls in
    /// `kept`; on a worker of the phase, whose `kept` is its unit, the unit: the whole file where a
    /// call of it may run a macro of the run as written (`whole_reaching_files`), else one member's
    /// or statement's calls, whose file's other units keep their round until the file is typed
    /// again, which changes the order of no expansion but those of a macro with state, whose run
    /// gives the build to one worker (`interp::watch`).
    fn expand_groups(&mut self, groups: Vec<std::ops::Range<usize>>, classes: &FxMap<u32, Vec<(u32, u32, ClassId)>>) {
        // Every call not expanded yet indexed: a run that reaches another unit's tree expands its
        // calls first (`prepare_executable`). A unit is put in its walk's order as its turn comes.
        self.inlining.reindex();
        let n = self.inlining.kept.len();
        let mut waiting = 0;
        let mut open: Option<Checkpoint> = None;
        for g in groups {
            if g.start < waiting {
                continue;
            }
            if self.inlining.suspendable && !open.as_ref().is_some_and(|c| c.range.contains(&g.start)) {
                if open.take().is_some() {
                    self.close_checkpoint();
                }
                let file = self.inlining.kept[g.start].at.file;
                let end = (g.start..n).find(|&j| self.inlining.kept[j].at.file != file).unwrap_or(n);
                open = Some(self.open_checkpoint(g.start..end));
            }
            if self.order_unit(g.clone(), classes) {
                for j in g.clone() {
                    let k = &self.inlining.kept[j];
                    if k.call.is_some() {
                        self.inlining.index.insert(k.node, j as u32);
                    }
                }
            }
            if self.expand_range(g).is_none() {
                continue;
            }
            let c = open.take().expect("a suspension in a recorded round");
            let range = c.range.clone();
            self.roll_back(c);
            // Its typing again makes its calls and what was held for them.
            for k in &mut self.inlining.kept[range.clone()] {
                if k.call.take().is_some() {
                    self.inlining.index.remove(&k.node);
                }
                k.held.clear();
            }
            let file = self.inlining.kept[range.start].at.file;
            if !self.inlining.suspended.contains(&file) {
                self.inlining.suspended.push(file);
            }
            waiting = range.end;
        }
        if open.is_some() {
            self.close_checkpoint();
        }
        self.inlining.installed = None;
    }

    /// The round of the file whose calls are `range` of `kept` recorded from here (`Checkpoint`).
    fn open_checkpoint(&mut self, range: std::ops::Range<usize>) -> Checkpoint {
        self.inlining.round_diags = self.diags.items.len();
        self.inlining.suspension_prefix = None;
        self.inlining.anon_log = Some(Vec::new());
        self.inlining.rewrite_log = Some(Vec::new());
        self.inlining.reached_other = false;
        let l = &self.inlining;
        let lists = [l.unboxes.len(), l.tailrec.len(), l.lifted.len(), l.deferred_traits.len(), l.erased.len(), l.fold_later.len(), l.index_later.len()];
        Checkpoint { range, pending: self.attempts.pending_len(), inline_unboxes: self.inline_unboxes.len(), erased_values: self.erased_values.len(), lists }
    }

    /// The file's round stands: its record goes.
    fn close_checkpoint(&mut self) {
        self.inlining.anon_log = None;
        self.inlining.rewrite_log = None;
        self.inlining.reached_other = false;
    }

    /// The file's round recorded in `c` given up: the nodes its expansions rewrote in place put
    /// back, newest first, what they left pending and listed for after the phase taken back, and
    /// the anonymous classes' places they numbered counted back. What they reported before the
    /// expansion met the macro of the run stands, as dotty's reporter keeps it through the unit's
    /// reset (`Phase.runOn` resets the typer state alone, `TyperState.resetTo`): an error stops the
    /// runs after this one (`Driver.finish`), and a retype drops a warning the next round reports
    /// again. What they reported after it goes, as the `SuspendException` leaves the unit there.
    /// The classes and nodes its expansions made stay, met by nothing.
    fn roll_back(&mut self, c: Checkpoint) {
        for prior in self.inlining.rewrite_log.take().unwrap_or_default().into_iter().rev() {
            match prior {
                Prior::Expr(te, value) => self.prog.exprs[te.idx()] = value,
                Prior::Type(te, ty) => self.prog.restore_type(te, ty),
            }
        }
        if let Some(prefix) = self.inlining.suspension_prefix.take() {
            self.keep_suspension_prefix(prefix);
        }
        let reported = std::mem::take(&mut self.inlining.suspension_reports);
        self.diags.extend(reported);
        drop(self.take_pending_from(c.pending));
        self.inline_unboxes.truncate(c.inline_unboxes);
        self.erased_values.truncate(c.erased_values);
        let l = &mut self.inlining;
        l.unboxes.truncate(c.lists[0]);
        l.tailrec.truncate(c.lists[1]);
        l.lifted.truncate(c.lists[2]);
        l.deferred_traits.truncate(c.lists[3]);
        l.erased.truncate(c.lists[4]);
        l.fold_later.truncate(c.lists[5]);
        l.index_later.truncate(c.lists[6]);
        for key in self.inlining.anon_log.take().unwrap_or_default().into_iter().rev() {
            if let Some(n) = self.anon_sites.get_mut(&key) {
                *n -= 1;
                if *n == 0 {
                    self.anon_sites.remove(&key);
                }
            }
        }
        // Each call's node is the typed call again, as reflection's view of it reads it.
        #[cfg(debug_assertions)]
        for j in c.range.clone() {
            let node = self.inlining.kept[j].node;
            if let Some((value, ty)) = self.inlining.journal.get(&node) {
                assert_eq!(format!("{:?}", self.prog.expr(node)), format!("{:?}", value), "a kept call's node not put back");
                assert_eq!(self.prog.type_or_aside(node), *ty, "a kept call's type not put back");
            }
        }
        self.inlining.reached_other = false;
        self.inlining.installed = None;
    }

    /// The expansion under way met a macro of the round's run (`suspended_hit`): what the file's
    /// recorded round reported up to here is what stands of it once it is given up (`roll_back`),
    /// as dotty's `SuspendException` leaves the unit at the splice (`Inliner.expandMacro`) or at
    /// the interpreter's missing class (`suspendOnMissing`).
    pub(super) fn hit_suspension(&mut self) {
        if !self.inlining.suspended_hit && self.inlining.rewrite_log.is_some() {
            let from = self.inlining.round_diags.min(self.diags.items.len());
            self.inlining.suspension_prefix = Some(self.diags.items[from..].iter().map(|d| (d.ordinal.0, ())).collect());
        }
        self.inlining.suspended_hit = true;
    }

    /// The round's diagnostics cut back to `prefix`, what it had reported at its suspension (by
    /// identity, `source::Ordinal`): those
    /// reported after it dropped (through the census of unsupported bodies), and those a retraction
    /// took back since staying out.
    fn keep_suspension_prefix(&mut self, prefix: FxMap<u64, ()>) {
        let from = self.inlining.round_diags.min(self.diags.items.len());
        let after = self.diags.items.split_off(from);
        // By identity (`source::Ordinal`): an equal record reported after the suspension is not
        // of the prefix.
        let (keep, dropped): (Vec<Diagnostic>, Vec<Diagnostic>) = after.into_iter().partition(|d| prefix.contains_key(&d.ordinal.0));
        self.diags.extend(dropped);
        self.drop_diagnostics_from(from);
        self.diags.extend(keep);
    }

    /// The kept calls of the unit `range` expanded in their order (`expand_kept`), up to one whose
    /// expansion met a macro of the round's run: its place then.
    fn expand_range(&mut self, range: std::ops::Range<usize>) -> Option<usize> {
        let start = range.start;
        let mut i = range.start;
        while i < range.end {
            // Each unit's reflection reads its own copies of the typed trees (`typed_body`).
            if i == start || (self.inlining.kept[i].at.file, self.inlining.kept[i].at.group) != (self.inlining.kept[i - 1].at.file, self.inlining.kept[i - 1].at.group) {
                self.inlining.typed_copies.clear();
            }
            if let Some(env) = self.inlining.kept[i].call.as_ref().map(|c| c.env.clone()) {
                if !self.inlining.installed.as_ref().is_some_and(|e| std::sync::Arc::ptr_eq(e, &env)) {
                    self.env = (*env).clone();
                    self.inlining.installed = Some(env);
                }
            }
            if self.expand_suspending(i) {
                self.inlining.installed = None;
                return Some(i);
            }
            // An expansion's erased result is unboxed once placed, as a unit's flush unboxes its
            // own (`apply_inline_unboxes`): the list stays the expansion's.
            if !self.inline_unboxes.is_empty() {
                self.apply_inline_unboxes();
            }
            i += 1;
        }
        None
    }

    /// Whether the program defines a macro of the round's run: an inline method of its own with a
    /// splice whose code names a definition of a file of the run outside its quotes (dotty's
    /// `Inliner.macroDependencies`: the terms referred to at level -1 and defined in the current
    /// run). Only then may an expansion meet one, however it reaches it (an inline call of its
    /// body, a given its search finds), and suspend its file.
    fn defines_macro_of_run(&self) -> bool {
        self.inline_definitions.iter().any(|(&sym, d)| {
            self.program_file(self.syms.sym(sym).file)
                && d.body.into_iter().chain(d.defaults.iter().flatten().copied()).chain(self.class_roots(&d.classes)).any(|root| {
                    self.prog.descendants(root).any(|e| matches!(self.prog.expr(e), crate::tir::TExpr::Splice(lambda) if matches!(self.prog.expr(lambda), crate::tir::TExpr::Lambda(_, code) if self.splice_names_program(code, self.syms.sym(sym).file))))
                })
        })
    }

    /// Whether the program defines a macro: an inline method of its own with a splice, whose run
    /// executes the program's code, which a later run's file may hold (`run_suspends`).
    fn defines_program_macro(&self) -> bool {
        self.inline_definitions.iter().any(|(&sym, d)| {
            self.program_file(self.syms.sym(sym).file)
                && d.body.into_iter().chain(d.defaults.iter().flatten().copied()).chain(self.class_roots(&d.classes)).any(|root| self.prog.descendants(root).any(|e| matches!(self.prog.expr(e), crate::tir::TExpr::Splice(_))))
        })
    }

    /// Whether an expansion of `sym` may run a macro of the round's run as its body is written: its
    /// body, the classes it makes, or an inline method of the program they call has a splice naming
    /// a definition of a file of the run (`whole_reaching_files`).
    fn runs_macro_of_run(&self, sym: SymId, memo: &mut FxMap<SymId, bool>) -> bool {
        if let Some(&b) = memo.get(&sym) {
            return b;
        }
        memo.insert(sym, false);
        // A library's or the std's inline method names no definition of the program.
        if !self.program_file(self.syms.sym(sym).file) {
            return false;
        }
        let Some(d) = self.inline_definitions.get(&sym).cloned() else { return false };
        let roots: Vec<TExprId> = d.body.into_iter().chain(d.defaults.iter().flatten().copied()).chain(self.class_roots(&d.classes)).collect();
        let found = roots.into_iter().any(|root| {
            self.prog.descendants(root).any(|e| match self.prog.expr(e) {
                crate::tir::TExpr::Splice(lambda) => matches!(self.prog.expr(lambda), crate::tir::TExpr::Lambda(_, code) if self.splice_names_program(code, self.syms.sym(sym).file)),
                crate::tir::TExpr::CallStatic(s, _) | crate::tir::TExpr::CallMethod(_, s, _) | crate::tir::TExpr::Field(_, s) => s != sym && self.syms.sym(s).mods & crate::ast::mods::INLINE != 0 && self.runs_macro_of_run(s, memo),
                _ => false,
            })
        });
        memo.insert(sym, found);
        found
    }

    /// The units of a forked round: `groups`, those of each file with a call that may run a
    /// macro of the round's run as its callee is written (`runs_macro_of_run`) joined into one, a
    /// file's calls standing together in the walk's order. The worker that takes such a file
    /// expands it as one worker does, its round recorded from its start and given up whole at the
    /// first expansion that meets a macro of the run (`expand_groups`), so that no unit of it after
    /// that call is expanded in the round; another file's unit that meets one otherwise (a given
    /// an implicit search finds) gives up its unit.
    fn whole_reaching_files(&self, groups: &[std::ops::Range<usize>]) -> Vec<std::ops::Range<usize>> {
        let mut memo: FxMap<SymId, bool> = FxMap::default();
        let mut files: FxMap<u32, ()> = FxMap::default();
        for k in &self.inlining.kept {
            if !files.contains_key(&k.at.file) && k.call.as_ref().is_some_and(|c| self.runs_macro_of_run(c.call.sym, &mut memo)) {
                files.insert(k.at.file, ());
            }
        }
        let mut units: Vec<std::ops::Range<usize>> = Vec::with_capacity(groups.len());
        for g in groups {
            let file = self.inlining.kept[g.start].at.file;
            match units.last_mut() {
                Some(u) if u.end == g.start && files.contains_key(&file) && self.inlining.kept[u.start].at.file == file => u.end = g.end,
                _ => units.push(g.clone()),
            }
        }
        units
    }

    /// Whether `f` is a file of the round's run (`Inlining::current_run`).
    fn in_current_run(&self, f: FileId) -> bool {
        self.program_file(f) && self.inlining.current_run.as_ref().map_or(true, |run| run.contains_key(&f.0))
    }

    /// Whether a splice's code names a definition of the round's run outside its quotes: a member,
    /// a module, the `this` of the class of the macro defined in `this_file` (dotty's
    /// `macroDependencies`: the terms at level -1 defined in the current run).
    pub(super) fn splice_names_program(&self, code: TExprId, this_file: FileId) -> bool {
        let member = |s: SymId| self.syms.sym(s).owner != crate::symbols::Owner::Local && self.in_current_run(self.syms.sym(s).file);
        let names = |e: TExprId| match self.prog.expr(e) {
            crate::tir::TExpr::CallStatic(s, _) | crate::tir::TExpr::CallMethod(_, s, _) | crate::tir::TExpr::Field(_, s) | crate::tir::TExpr::Static(s) => member(s),
            crate::tir::TExpr::Module(c) => self.in_current_run(self.syms.class(c).file),
            crate::tir::TExpr::This => self.in_current_run(this_file),
            _ => false,
        };
        // The splice's references to the run's definitions; where it has some, those inside a
        // quote set apart (level 0, the expansion's own code).
        let named: Vec<TExprId> = self.prog.descendants(code).filter(|&e| names(e)).collect();
        if named.is_empty() {
            return false;
        }
        let quote = |e: TExprId| matches!(self.prog.expr(e), crate::tir::TExpr::Js(t, _) if self.prog.strings[t.0 as usize] == "$quote");
        let mut quoted: FxMap<TExprId, ()> = FxMap::default();
        for e in self.prog.descendants(code) {
            if quote(e) {
                quoted.extend(self.prog.descendants(e).skip(1).map(|q| (q, ())));
            }
        }
        named.iter().any(|e| !quoted.contains_key(e))
    }

    /// The units of the sorted kept calls, each a run of the calls of one member or class
    /// statement (`WalkOrder::group`): the trees the phase's workers expand apart, an expansion
    /// reading its own unit's tree alone. A unit whose calls stand apart in the order (a place
    /// the walk does not tell from another's) takes every run between, so that each unit is one
    /// run and the runs keep the walk's order.
    pub(super) fn phase_groups(&self) -> Vec<std::ops::Range<usize>> {
        let kept = &self.inlining.kept;
        let mut groups: Vec<std::ops::Range<usize>> = Vec::new();
        let mut first_run: FxMap<(u32, u32), usize> = FxMap::default();
        for (i, k) in kept.iter().enumerate() {
            let unit = (k.at.file, k.at.group);
            match groups.last_mut() {
                Some(g) if (kept[g.start].at.file, kept[g.start].at.group) == unit => g.end = i + 1,
                _ => match first_run.get(&unit) {
                    // The unit came before: the runs from its first on are one.
                    Some(&j) => {
                        let start = groups[j].start;
                        groups.truncate(j);
                        groups.push(start..i + 1);
                        for (g, run) in groups.iter().enumerate().skip(j) {
                            for k in &kept[run.clone()] {
                                first_run.insert((k.at.file, k.at.group), g);
                            }
                        }
                    }
                    None => {
                        first_run.insert(unit, groups.len());
                        groups.push(i..i + 1);
                    }
                },
            }
        }
        groups
    }

    /// The workers the phase takes for `groups` units: the body phase's where it forked, the
    /// round's units hold enough calls to pay for a fork and a merge, and there is more than one unit;
    /// `TEQ_PHASE_FORK=1` forks whatever the count (the tests'), `=0` never.
    fn phase_workers(&self, groups: usize, calls: usize) -> usize {
        let wanted = std::env::var("TEQ_PHASE_FORK").ok();
        if wanted.as_deref() == Some("0") || self.inlining.workers < 2 || groups < 2 {
            return 1;
        }
        if wanted.as_deref() != Some("1") && calls < PHASE_FORK_CALLS {
            return 1;
        }
        // The variables' tags: the body phase's workers took those below its count, the rounds of
        // the phase forked before this one the ones after, up to `next_tag`.
        let tags = (u8::MAX as usize + 1).saturating_sub(self.inlining.next_tag);
        let workers = self.inlining.workers.min(tags).min(groups);
        if workers < 2 {
            return 1;
        }
        workers
    }

    /// `expand_kept` in a file whose round is recorded (`Checkpoint`), which suspends the call
    /// where its expansion meets a macro of the round's run: dotty's `Inliner.expandMacro` checks
    /// the macro's dependencies as it expands the splice and suspends the unit
    /// (`CompilationUnit.suspend`); the macro is not run, and every walk under way stops there.
    /// Whether it did.
    fn expand_suspending(&mut self, i: usize) -> bool {
        if self.inlining.rewrite_log.is_none() || self.inlining.kept[i].call.is_none() {
            self.expand_kept(i);
            return false;
        }
        self.inlining.suspending = true;
        self.expand_kept(i);
        self.inlining.suspending = false;
        std::mem::take(&mut self.inlining.suspended_hit)
    }

    /// The kept call at place `i` expanded, what was held for it reported first; what its expansion
    /// left pending (a copy of a call among a transparent call's arguments) expanded after it.
    fn expand_kept(&mut self, i: usize) {
        let (node, typed_as, call, held) = {
            let k = &mut self.inlining.kept[i];
            (k.node, k.typed_as, k.call.take(), std::mem::take(&mut k.held))
        };
        self.diags.extend(held);
        let Some(call) = call else { return };
        // A call another expansion's run reaches (`prepare_executable`) is expanded whole.
        let suspending = self.inlining.suspending;
        self.inlining.suspending = suspending && self.inlining.expanding.is_empty();
        self.inlining.index.remove(&node);
        super::substitution::counts::phase_expanded(!self.inlining.running || self.inlining.expanding.len() > 0);
        self.inlining.expanding.push(node);
        self.attempts.flushing_begin();
        // The call's file is the unit the expansion's records are its (the capture's, by
        // `typing_unit`), around the expansion as at its rewrite of the call's node after it.
        let file = std::mem::replace(&mut self.env.file, call.env.file);
        self.expand_pending(node, typed_as, call);
        self.env.file = file;
        self.attempts.flushing_end();
        self.inlining.expanding.pop();
        self.inlining.suspending = suspending;
        if self.attempts.pending_len() != 0 && !self.inlining.suspended_hit {
            self.flush_pending_from(0);
        }
    }

    /// The executable form of the tree `root`, which the interpreter is about to run: the kept
    /// calls inside it expanded now, in the walk's order, as dotty compiles a macro's unit through
    /// its `Inlining` before running it. Where the records are published to the workers that type
    /// beside this one, nothing may change them: the build is typed again by one worker
    /// (`need_serial`).
    /// Whether the tree `root` is ready to run: false where it holds kept calls this worker may not
    /// expand.
    pub(crate) fn prepare_executable(&mut self, root: TExprId) -> bool {
        if !self.may_hold_kept() {
            return true;
        }
        // Another worker's kept call is told by its mark (`Program::kept_bits`) on a node still the
        // call: an expansion replaces the node's value.
        let mut found: Vec<u32> = Vec::new();
        let mut foreign = false;
        for e in self.prog.descendants(root) {
            match self.inlining.index.get(&e) {
                Some(&i) => found.push(i),
                None => foreign |= self.forked && self.prog.is_kept(e) && self.is_unexpanded_call(e),
            }
        }
        if found.is_empty() && !foreign {
            return true;
        }
        // A record the workers published is changed by no one before the merge, at one forked
        // worker as at several (`PARALLEL-TYPER.md`'s publication rule).
        if self.forked {
            super::substitution::counts::gave_way();
            self.need_serial_for_preparation();
            return false;
        }
        found.sort_unstable_by(|&a, &b| self.inlining.kept[a as usize].at.cmp(&self.inlining.kept[b as usize].at));
        self.inlining.reached_other |= self.inlining.rewrite_log.is_some();
        // Each a root of its own, outside the expansion and the search under way (a macro's run
        // reaching the tree), as a body typed on demand is (`type_member_body`).
        self.as_own_unit(|t| {
            t.outside_search(|t| {
                t.outside_inline(|t| {
                    t.outside_quotes(|t| {
                        for i in found {
                            t.expand_kept(i as usize);
                        }
                    })
                })
            })
        });
        true
    }

    /// Whether a tree the interpreter runs may hold a kept call: this worker kept some, or the
    /// build is forked, where the workers' kept calls only their marks tell.
    #[inline]
    fn may_hold_kept(&self) -> bool {
        !self.inlining.index.is_empty() || self.forked || self.inlining.run_consumer.is_some()
    }

    /// Whether the definition of the file `file` that a macro's run is about to run belongs to a
    /// unit the round has not completed (the round's run, `in_current_run`), where the run's
    /// unit may suspend (`Inlining::run_consumer`): it does, and the run stops
    /// (`Inlining::run_suspended`), as dotty's interpreter meets a class of the current run
    /// missing from its class loader (`Interpreter.MissingClassValidInCurrentRun`) and suspends
    /// the unit it expands in (`suspendOnMissing`); a later run finds the unit through its
    /// `Inlining` and `LambdaLift`.
    pub(crate) fn run_suspends(&mut self, file: crate::source::FileId) -> bool {
        if self.inlining.run_consumer.is_some() && self.in_current_run(file) {
            self.inlining.run_suspended = true;
            return true;
        }
        false
    }

    /// The build given to one worker, whose walk prepares the tree in place: a forked build of one
    /// worker too (`TEQ_FORK=1`), which `need_serial` leaves on its worker.
    fn need_serial_for_preparation(&self) {
        let why = || "a macro's run reached a body whose inline calls wait for the inlining phase".to_string();
        if self.walk_by_size {
            return self.need_serial(why);
        }
        if self.bodies_started && !self.serial.is_needed() {
            let mut reason = self.serial.reason.lock().unwrap_or_else(|e| e.into_inner());
            if reason.is_none() {
                *reason = Some((why(), None));
                crate::measure::gave_way();
            }
            self.serial.needed.store(true, std::sync::atomic::Ordering::Release);
        }
    }

    /// Whether the node `e` is still a call of an inline method (no expansion replaced it).
    fn is_unexpanded_call(&self, e: TExprId) -> bool {
        match self.prog.expr(e) {
            crate::tir::TExpr::CallStatic(s, _) | crate::tir::TExpr::CallMethod(_, s, _) | crate::tir::TExpr::Field(_, s) => self.syms.sym(s).mods & crate::ast::mods::INLINE != 0,
            _ => false,
        }
    }

    /// The body of the function `f`, which the interpreter is about to run, made executable once
    /// (`prepare_executable`); false where the run stops instead, its unit suspended
    /// (`run_suspends`).
    #[inline]
    pub(crate) fn prepare_fun(&mut self, f: FunId) -> bool {
        if self.may_hold_kept() && !(!self.forked && self.inlining.checked_funs.get(f.idx()).copied().unwrap_or(false)) {
            self.prepare_fun_now(f);
            return !self.inlining.run_suspended;
        }
        true
    }

    #[inline(never)]
    fn prepare_fun_now(&mut self, f: FunId) {
        let i = f.idx();
        // The ids are canonical where no workers fork (one worker's build, the phase after the
        // merge): a function's check is a vector's entry; while workers type, a map's.
        let dense = !self.forked;
        if !dense && self.inlining.checked_funs_typing.contains_key(&f) {
            return;
        }
        // A call is kept in a program's file alone (`Worker::end_unit`).
        let file = self.syms.sym(self.prog.funs[i].sym).file;
        let program = self.program_file(file);
        if program && self.run_suspends(file) {
            return;
        }
        let ready = !program || {
            let (body, defaults) = {
                let fun = &self.prog.funs[i];
                (fun.body, fun.defaults.iter().flatten().copied().collect::<Vec<_>>())
            };
            body.into_iter().chain(defaults).all(|r| self.prepare_executable(r))
        };
        if !ready {
            return;
        }
        if dense {
            if self.inlining.checked_funs.len() <= i {
                self.inlining.checked_funs.resize(i + 1, false);
            }
            self.inlining.checked_funs[i] = true;
        } else {
            self.inlining.checked_funs_typing.insert(f, ());
        }
    }

    /// The statements and parents' arguments of the class body at `ti`, which a construction is
    /// about to run, made executable once; false where they hold a call whose expansion is under
    /// way (a macro's run makes the module of its inline method's owner, whose body would read
    /// the expansion it makes): the module is made without running its body, as while its class
    /// is checked (`Interp::construct`).
    #[inline]
    pub(crate) fn prepare_class(&mut self, ti: usize) -> bool {
        if !self.may_hold_kept() && self.inlining.expanding.is_empty() {
            return true;
        }
        self.prepare_class_now(ti)
    }

    #[inline(never)]
    fn prepare_class_now(&mut self, ti: usize) -> bool {
        let file = self.syms.class(self.prog.classes[ti].id).file;
        if !self.program_file(file) {
            return true;
        }
        if self.run_suspends(file) {
            return false;
        }
        let checked = if self.inlining.running {
            self.inlining.checked_classes.get(ti).copied().unwrap_or(false)
        } else {
            self.inlining.checked_classes_typing.contains_key(&ti)
        };
        if checked {
            return true;
        }
        let tc = &self.prog.classes[ti];
        let mut roots: Vec<TExprId> = tc
            .init
            .iter()
            .filter_map(|i| match *i {
                crate::tir::TInit::Field(_, e) | crate::tir::TInit::Stmt(e) => Some(e),
                crate::tir::TInit::Parent(..) => None,
            })
            .collect();
        if let Some(l) = tc.parent_args {
            roots.extend(self.prog.expr_list(l).iter().copied());
        }
        if !self.inlining.expanding.is_empty() && roots.iter().any(|&r| self.prog.descendants(r).any(|e| self.inlining.expanding.contains(&e))) {
            return false;
        }
        if !roots.into_iter().all(|r| self.prepare_executable(r)) {
            return false;
        }
        if self.inlining.running {
            if self.inlining.checked_classes.len() <= ti {
                self.inlining.checked_classes.resize(ti + 1, false);
            }
            self.inlining.checked_classes[ti] = true;
        } else {
            self.inlining.checked_classes_typing.insert(ti, ());
        }
        true
    }

    /// The erased values a unit's typing made from `mark` on, which its trees `roots` may hold: a
    /// program's checked once the inlining phase is done, a library's or the std's body's now.
    pub(super) fn unit_erased_values(&mut self, mark: usize, roots: &[TExprId], file: FileId) {
        if !self.program_file(file) {
            return self.report_erased_values(mark, roots);
        }
        let made = self.erased_values.split_off(mark);
        self.inlining.erased.extend(made);
    }

    /// The erased values the units held and the phase made, reported where the program's trees
    /// still hold them once every expansion is done (`Erasure.checkNotErased`): a value an
    /// expansion dropped (an unused inline parameter's argument) is none.
    fn check_erased_after_phase(&mut self, mark: usize) {
        let kept = std::mem::take(&mut self.inlining.erased);
        self.erased_values.extend(kept);
        if self.erased_values.len() <= mark {
            return;
        }
        let roots = self.program_roots();
        self.report_erased_values(mark, &roots);
    }

    /// Every tree the program's output holds: the functions' bodies and defaults, the classes'
    /// statements and parents' arguments, the vals' initialisers.
    fn program_roots(&self) -> Vec<TExprId> {
        let mut roots = Vec::new();
        for f in self.prog.funs.iter() {
            roots.extend(f.body);
            roots.extend(f.defaults.iter().flatten().copied());
        }
        for c in self.prog.classes.iter() {
            for init in &c.init {
                match *init {
                    crate::tir::TInit::Field(_, e) | crate::tir::TInit::Stmt(e) => roots.push(e),
                    crate::tir::TInit::Parent(..) => {}
                }
            }
            if let Some(l) = c.parent_args {
                roots.extend(self.prog.expr_list(l).iter().copied());
            }
        }
        roots.extend(self.val_init.iter().map(|(_, &e)| e));
        roots
    }
}
