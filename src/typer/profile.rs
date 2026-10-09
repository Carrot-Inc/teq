//! `--profile`: the time of the type phase attributed to the constructs that search (given
//! searches, overload resolutions, conversion searches, inline expansions, extension lookups
//! that reach the implicit scope, arguments typed twice, loader completions) and to the sites
//! that use them. A search calls `Worker::prof` on entry and `Profile::exit` on exit; off, each
//! is one branch on `Profile::on`, and nothing is allocated and no clock read.

use super::Worker;
use crate::dialect;
use crate::intern::{FxMap, Name};
use crate::source::{FileId, Span};
use crate::symbols::*;
use crate::interp::profile::{ticks, Allocs, InterpProf, ProfKey, ProfRow};
use crate::types::*;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub enum Kind {
    Given,
    Overload,
    Conversion,
    Inline,
    /// An extension method looked up beyond the lexical scope: through the givens in scope and
    /// the implicit scope of the receiver.
    Extension,
    /// A function literal typed ahead of an overload resolution, which the chosen alternative
    /// then types again, and the second round of a resolution with conversions allowed.
    Retry,
    /// The reduction of a match type; the reducer calls `prof(Kind::MatchType, ..)` around it.
    MatchType,
    /// A class of the classpath entered from its TASTy or class file.
    Completion,
    /// A class or method of the classpath compiled from its TASTy body for JavaScript.
    LibraryBody,
    /// A macro expanded by the interpreter, with the steps it ran.
    Macro,
}

impl Kind {
    const ALL: [Kind; 10] =
        [Kind::Given, Kind::Overload, Kind::Conversion, Kind::Inline, Kind::Extension, Kind::Retry, Kind::MatchType, Kind::Completion, Kind::LibraryBody, Kind::Macro];

    pub fn name(self) -> &'static str {
        match self {
            Kind::Given => "given search",
            Kind::Overload => "overload resolution",
            Kind::Conversion => "conversion search",
            Kind::Inline => "inline expansion",
            Kind::Extension => "extension lookup in the implicit scope",
            Kind::Retry => "argument typed twice",
            Kind::MatchType => "match type reduction",
            Kind::Completion => "class completion from the classpath",
            Kind::LibraryBody => "library body typed from the classpath",
            Kind::Macro => "macro expansion",
        }
    }

    fn short(self) -> &'static str {
        match self {
            Kind::Given => "given",
            Kind::Overload => "overload",
            Kind::Conversion => "conversion",
            Kind::Inline => "inline",
            Kind::Extension => "extension",
            Kind::Retry => "retry",
            Kind::MatchType => "match-type",
            Kind::Completion => "completion",
            Kind::LibraryBody => "library-body",
            Kind::Macro => "macro",
        }
    }
}

/// What a search was about.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum About {
    /// The target of a given search or of a conversion to an expected type.
    Type(TypeId),
    /// The overloaded name, the inline callee, the extension method's name as a symbol.
    Sym(SymId),
    /// A member selected on a receiver that lacks it: a conversion or an extension lookup.
    Member(TypeId, Name),
    Class(ClassId),
    None,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Found,
    NotFound,
    Ambiguous,
    /// A function literal typed ahead and passed on as typed.
    Kept,
}

struct Event {
    kind: Kind,
    file: FileId,
    pos: u32,
    about: About,
    start: Instant,
    incl: u64,
    child: u64,
    /// How many events of the same kind were open when this one started.
    nested: u16,
    tries_at_entry: u32,
    tried: u32,
    applicable: u32,
    /// The interpreter steps of a macro expansion.
    steps: u64,
    outcome: Outcome,
    /// A given search: the candidates its levels yielded, those it instantiated itself and
    /// those of them that failed, and what asked for it.
    cands: u32,
    tried_here: u32,
    failed_here: u32,
    origin: Origin,
    /// A given search: what the result memo did for it and whether its result was kept.
    memo: Memo,
    keep: Keep,
    /// A nested given search: the candidate whose using clause asked for it.
    asked_by: Option<SymId>,
    /// An inline expansion: a hash of its type arguments.
    targs: u64,
    /// The processor's counter at the start, and the ticks of the events nested in it, for
    /// the parts' clock (`StepClock`).
    start_ticks: u64,
    child_ticks: u64,
    /// A given search or an extension lookup: its own time by part (`Step`).
    parts: [u64; N_STEPS],
    /// A given search: whether its request was for a by-name parameter, whether its target
    /// had no open variable, whether an extension lookup was open around it, whether the memo
    /// answered it with nothing found, and the context its lookup keyed (the file and the
    /// hash of `context_key`).
    byname: bool,
    ground: bool,
    under_ext: bool,
    hit_nomatch: bool,
    ctx: Option<(FileId, u64)>,
    /// The innermost given search open around it, `u32::MAX` for none, and the worker whose
    /// profile it was recorded in (0 for the build's own).
    parent: u32,
    worker: u16,
}

/// What the result memo did for a given search.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum Memo {
    #[default]
    None,
    Hit,
    /// Not applicable: open variables in the target, in a local given's type or in the type of
    /// the value a local import reads from, an open quote, GADT bounds in force, a request for
    /// a by-name parameter.
    Vars,
    VarsLocal,
    VarsImport,
    Quote,
    Gadt,
    ByNameRequest,
    /// No entry for the target.
    Miss,
    /// An entry that does not fit: the tree too deep here, the search budget out, a
    /// function-type search through a conversion in progress, a given of the path open above,
    /// a by-name given open above could answer a request of the tree.
    Depth,
    Budget,
    ConversionGuard,
    Opened,
    ByName,
}

impl Memo {
    const ALL: [Memo; 14] = [Memo::Hit, Memo::Vars, Memo::VarsLocal, Memo::VarsImport, Memo::Quote, Memo::Gadt, Memo::ByNameRequest, Memo::Miss, Memo::Depth, Memo::Budget, Memo::ConversionGuard, Memo::Opened, Memo::ByName, Memo::None];

    fn name(self) -> &'static str {
        match self {
            Memo::None => "no memo",
            Memo::Hit => "answered from the memo",
            Memo::Vars => "not applicable: open type variables in the target",
            Memo::VarsLocal => "not applicable: an inference variable in a local given's type",
            Memo::VarsImport => "not applicable: an inference variable in a local import's value",
            Memo::Quote => "not applicable: inside a quote",
            Memo::Gadt => "not applicable: under GADT bounds",
            Memo::ByNameRequest => "not applicable: a request for a by-name parameter",
            Memo::Miss => "miss: no entry for the target under this context",
            Memo::Depth => "miss: the kept tree too deep here",
            Memo::Budget => "miss: the kept search's attempts exceed the budget left",
            Memo::ConversionGuard => "miss: a function type is under search through a conversion",
            Memo::Opened => "miss: a given of the kept path is open above",
            Memo::ByName => "miss: a by-name given open above could answer a request of the kept tree",
        }
    }
}

/// Whether a search's result was kept in the memo, and why not.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum Keep {
    #[default]
    None,
    Kept,
    KeptNoMatch,
    Ambiguous,
    Reported,
    VarsInResult,
    NotShareable,
    /// A search of the tree read the state of what is in progress (`implicits::InProgress`).
    InProgress,
    Restricted,
    FailureNotes,
}

impl Keep {
    const ALL: [Keep; 9] = [Keep::Kept, Keep::KeptNoMatch, Keep::Ambiguous, Keep::Reported, Keep::VarsInResult, Keep::NotShareable, Keep::InProgress, Keep::Restricted, Keep::FailureNotes];

    fn name(self) -> &'static str {
        match self {
            Keep::None => "",
            Keep::Kept => "kept",
            Keep::KeptNoMatch => "kept: nothing found",
            Keep::Ambiguous => "not kept: ambiguous",
            Keep::Reported => "not kept: a diagnostic reported",
            Keep::VarsInResult => "not kept: open variables in the result type",
            Keep::NotShareable => "not kept: the tree binds locals",
            Keep::InProgress => "not kept: a search of the tree read in-progress state",
            Keep::Restricted => "not kept: an implicit scope's given has restricted visibility",
            Keep::FailureNotes => "not kept: failure notes",
        }
    }
}

/// A candidate's part in the searches: shape checks, full instantiations and their failures,
/// and the time of both without the searches nested in them.
#[derive(Default, Clone, Copy)]
pub struct CandidateRow {
    pub shape: u32,
    pub tries: u32,
    pub fails: u32,
    pub self_ns: u64,
}

/// What asked for a given search: the top-level request, inherited by the nested searches.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum Origin {
    /// A using argument or `summon` in ordinary code.
    #[default]
    Plain,
    /// A using argument inside an inline expansion (a library's inline bodies).
    Inline,
    SummonInline,
    SummonFrom,
}

impl Origin {
    const ALL: [Origin; 4] = [Origin::SummonInline, Origin::Inline, Origin::SummonFrom, Origin::Plain];

    fn name(self) -> &'static str {
        match self {
            Origin::Plain => "using arguments and summon in plain code",
            Origin::Inline => "using arguments inside inline expansions",
            Origin::SummonInline => "summonInline and summonAll",
            Origin::SummonFrom => "summonFrom",
        }
    }
}

/// The parts of a given search's own time, and of an extension lookup's: a part is entered by
/// `Worker::step` and left by `step_end`, which names the part its time goes to (a fit decision
/// is told by how it was decided only once it is); the time between, less the searches,
/// expansions and parts nested in it, is the part's, and what no part claims is the residual
/// (`Rest`). So the parts of an event sum to its own time, read off one clock.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Step {
    Rest,
    /// The checks of a nested search before its event starts, its early returns included.
    PreEvent,
    Applicable,
    Lookup,
    Replay,
    Levels,
    FitMemo,
    FitClass,
    FitHead,
    FitShapeYes,
    FitShapeNo,
    FitConversion,
    Order,
    TryOk,
    TryMismatch,
    TryIncomplete,
    TryAmbiguous,
    /// A conversion method instantiated as the value of a function type.
    Conversion,
    Divergence,
    Choose,
    Keep,
    Synthesis,
    /// Completions, signatures, bodies and indexes entered from inside (`Phase`).
    Lazy,
    /// An extension lookup's collection of the givens that may provide the extension.
    ExtCollect,
}

pub const N_STEPS: usize = 24;

impl Step {
    const ALL: [Step; N_STEPS] = [
        Step::PreEvent,
        Step::Applicable,
        Step::Lookup,
        Step::Replay,
        Step::Levels,
        Step::FitMemo,
        Step::FitClass,
        Step::FitHead,
        Step::FitShapeYes,
        Step::FitShapeNo,
        Step::FitConversion,
        Step::Order,
        Step::TryOk,
        Step::TryMismatch,
        Step::TryIncomplete,
        Step::TryAmbiguous,
        Step::Conversion,
        Step::Divergence,
        Step::Choose,
        Step::Keep,
        Step::Synthesis,
        Step::Lazy,
        Step::ExtCollect,
        Step::Rest,
    ];

    fn name(self) -> &'static str {
        match self {
            Step::Rest => "residual (no part claims it)",
            Step::PreEvent => "nested searches' checks before their event",
            Step::Applicable => "memo applicability",
            Step::Lookup => "memo lookup",
            Step::Replay => "memo hit replayed (tree copied)",
            Step::Levels => "levels' candidates collected",
            Step::FitMemo => "fit decided by the fit memo",
            Step::FitClass => "fit rejected by the class test",
            Step::FitHead => "fit rejected by a head class",
            Step::FitShapeYes => "shape pass run: fits",
            Step::FitShapeNo => "shape pass run: no fit",
            Step::FitConversion => "conversion method's fit",
            Step::Order => "preference order",
            Step::TryOk => "instantiated: success",
            Step::TryMismatch => "instantiated: mismatch",
            Step::TryIncomplete => "instantiated: a using parameter unresolved",
            Step::TryAmbiguous => "instantiated: a using parameter ambiguous",
            Step::Conversion => "conversion method instantiated",
            Step::Divergence => "divergence checks",
            Step::Choose => "choice among successes",
            Step::Keep => "memo keep",
            Step::Synthesis => "synthesis after nothing found",
            Step::Lazy => "lazy work entered from inside (completions, signatures, bodies, indexes)",
            Step::ExtCollect => "extension lookup's collection of givens",
        }
    }
}

/// The ways a given search ends before its event starts.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Early {
    DepthLimit,
    ErrorTarget,
    Singleton,
    RecursiveRef,
    BareVar,
    NotGiven,
}

impl Early {
    const ALL: [Early; 6] =
        [Early::DepthLimit, Early::ErrorTarget, Early::Singleton, Early::RecursiveRef, Early::BareVar, Early::NotGiven];

    fn name(self) -> &'static str {
        match self {
            Early::DepthLimit => "the depth limit",
            Early::ErrorTarget => "an erroneous target",
            Early::Singleton => "a singleton given's path",
            Early::RecursiveRef => "a recursive reference to an open instance",
            Early::BareVar => "an unconstrained type variable",
            Early::NotGiven => "a NotGiven target, answered by the negated search",
        }
    }
}

/// Why a fit decision was made rather than answered from the fit memo.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FitWhy {
    /// A key the memo had no answer for yet; the answer was kept when context-free.
    FirstKept,
    /// A key the memo had no answer for, whose answer read the scope and was not kept.
    FirstUnkept,
    /// The target has open variables, an opaque type is transparent or GADT bounds hold.
    NotMemoised,
    /// The candidate's or its imported receiver's type has an inference variable.
    VarCandidate,
    /// A member of a class or trait, or a local: its fit reads the site.
    UnsettledOwner,
}

impl FitWhy {
    const ALL: [FitWhy; 5] = [FitWhy::FirstKept, FitWhy::FirstUnkept, FitWhy::NotMemoised, FitWhy::VarCandidate, FitWhy::UnsettledOwner];

    fn name(self) -> &'static str {
        match self {
            FitWhy::FirstKept => "first decision of its key, kept",
            FitWhy::FirstUnkept => "first decision of its key, not kept as it read the scope",
            FitWhy::NotMemoised => "no memo: open variables in the target, a transparent opaque type or GADT bounds",
            FitWhy::VarCandidate => "no memo: the candidate's or its receiver's type has an inference variable",
            FitWhy::UnsettledOwner => "no memo: a member of a class or trait, or a local (the site decides)",
        }
    }
}

/// The fit decisions the fit memo may answer, as they are told apart when it does not.
const FIT_HOWS: [Step; 5] = [Step::FitClass, Step::FitHead, Step::FitShapeYes, Step::FitShapeNo, Step::FitConversion];

/// The kinds of level a search collects candidates from.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LevelKind {
    ImplicitScope,
    Predef,
    Package,
    Frame,
}

impl LevelKind {
    const ALL: [LevelKind; 4] = [LevelKind::Frame, LevelKind::Package, LevelKind::Predef, LevelKind::ImplicitScope];

    fn name(self) -> &'static str {
        match self {
            LevelKind::ImplicitScope => "the target's implicit scope",
            LevelKind::Predef => "scala.Predef",
            LevelKind::Package => "package clauses with the file's imports",
            LevelKind::Frame => "enclosing blocks and classes with their imports",
        }
    }
}

/// A count and a time.
#[derive(Default, Clone, Copy)]
pub struct Tally {
    pub n: u64,
    pub ns: u64,
}

impl Tally {
    pub fn add(&mut self, ns: u64) {
        self.n += 1;
        self.ns += ns;
    }

    fn absorb(&mut self, o: Tally) {
        self.n += o.n;
        self.ns += o.ns;
    }
}

/// A part entered and not yet left, on the clock of the innermost event that keeps one.
struct StepFrame {
    step: Step,
    acc: u64,
}

/// The clock of an open given search or extension lookup: the frames of its parts start at
/// `base` in `Profile::frames`, and its time since `last` goes to the innermost of them, less
/// what its nested events took since then (`child_at_last` against the event's
/// `child_ticks`). It counts the ticks of the processor's counter (`ticks`), cheaper to read
/// than the clock, and the event's close scales its parts to the event's own time.
struct StepClock {
    event: usize,
    base: usize,
    last: u64,
    child_at_last: u64,
}

/// A candidate's attempt under way: the given search it belongs to, when it began, and the
/// ticks of the given searches nested in it so far.
struct Trying {
    given: SymId,
    owner: Option<usize>,
    start: u64,
    nested: u64,
}

/// A part entered, for its exit: the index of its frame, none when no clock took it.
#[derive(Clone, Copy)]
pub struct StepTok(Option<u32>);

impl StepTok {
    pub const NONE: StepTok = StepTok(None);

    #[inline(always)]
    pub fn taken(self) -> bool {
        self.0.is_some()
    }
}

/// A nested search's checks before its event: when it began, and its part of the given search
/// it stands in.
#[derive(Clone, Copy)]
pub struct PreEvent {
    start: Option<u64>,
    tok: StepTok,
}

/// What the given searches did: the candidates found, the instantiations and their failures;
/// where their time went is the parts' (`Step`).
#[derive(Default)]
pub struct GivenSplit {
    pub cands: u64,
    pub tries: u64,
    pub fails: u64,
    /// Candidates whose type fit the target in the shape pass.
    pub fits: u64,
    /// Candidates left untried because a success beat them outright.
    pub pruned: u64,
    /// Candidates rejected by the head class of a type argument.
    pub head_rejected: u64,
    /// Searches answered from the result cache.
    pub cached: u64,
    /// Shape checks answered from the memo.
    pub shape_cached: u64,
    /// Shape checks run, and those whose answer could not be kept because the candidate's type
    /// reads the scope of the search.
    pub shape_checks: u64,
    pub shape_unkept: u64,
    /// Reads of in-progress state, by reader (`implicits::InProgress`).
    pub in_progress: [u32; 11],
    /// The candidates under instantiation, innermost last.
    trying: Vec<Trying>,
    pub by_candidate: FxMap<SymId, CandidateRow>,
    /// The parts' tables, out of line: the profile is a field of every `Worker`.
    pub parts: Box<PartTables>,
}

/// What the parts of the given searches (`Step`) count beyond each event's own split, and the
/// clocks that split it.
#[derive(Default)]
pub struct PartTables {
    /// How often each part was left, over the given searches and extension lookups.
    pub step_count: [u64; N_STEPS],
    /// The lazy work inside the searches by part of the type phase.
    pub lazy_by_phase: [Tally; 24],
    /// The searches that ended before their event, by how, and the checks of those that
    /// reached it; each in all and inside a given search's own time.
    pub early: [(Tally, Tally); 6],
    pub pre_event: (Tally, Tally),
    /// The fit decisions the fit memo did not answer, by how they were decided (`FIT_HOWS`)
    /// and why the memo did not answer them.
    pub fit_why: [[Tally; 5]; 5],
    /// The collection of each level's candidates, by kind of level.
    pub levels: [Tally; 4],
    /// The clocks of the open given searches and extension lookups, innermost last, and the
    /// frames of their parts.
    clocks: Vec<StepClock>,
    frames: Vec<StepFrame>,
    /// The workers' profiles absorbed so far, which number their events.
    absorbed: u16,
    /// The own time of the events with parts, in nanoseconds and in ticks: the scale of the
    /// tables above, which count ticks.
    own_ns: u64,
    own_ticks: u64,
    /// The clock and the counter as the profile was turned on, the scale where no event with
    /// parts closed (a run whose searches all end before their event).
    calibration: Option<(Instant, u64)>,
}

impl PartTables {
    /// Nanoseconds of the ticks the tables count.
    fn ns(&self, counted: u64) -> u64 {
        if self.own_ticks != 0 {
            return (counted as u128 * self.own_ns as u128 / self.own_ticks as u128) as u64;
        }
        let Some((at, start)) = self.calibration else { return 0 };
        let (ns, since) = (at.elapsed().as_nanos(), ticks().wrapping_sub(start));
        if since == 0 { 0 } else { (counted as u128 * ns / since as u128) as u64 }
    }
}

/// What an inline expansion spends its own time on, for the split `--profile` prints: a part
/// calls `Worker::part` on entry and `part_end` on exit, and its self time excludes the parts
/// and the searches nested in it. The parts of the body's constructs (`Apply`, `Closure`,
/// `Copy`) are entered only inside an expansion.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Part {
    /// The type arguments substituted and the definition's scope entered.
    TypeArgs,
    /// The parameters bound to their arguments, the folding aside.
    Bind,
    /// Arguments folded to constants, by the folder and by the interpreter.
    Fold,
    /// The patterns of an `inline match` tested against the scrutinee.
    Match,
    /// A member of `scala.compiletime` evaluated, its given search aside.
    Intrinsic,
    /// A function literal of the body typed.
    Closure,
    /// A selection or application of the body typed, its overload resolution aside.
    Apply,
    /// An argument expression copied into a use of its parameter.
    Copy,
    /// The body's diagnostics moved to the call site.
    Relocate,
}

impl Part {
    const ALL: [Part; 9] = [Part::TypeArgs, Part::Bind, Part::Fold, Part::Match, Part::Intrinsic, Part::Closure, Part::Apply, Part::Copy, Part::Relocate];

    fn name(self) -> &'static str {
        match self {
            Part::TypeArgs => "type arguments substituted, scope entered",
            Part::Bind => "arguments bound to the parameters",
            Part::Fold => "arguments folded (constant folder and interpreter)",
            Part::Match => "inline match patterns tested",
            Part::Intrinsic => "compiletime intrinsics evaluated (given searches excluded)",
            Part::Closure => "function literals typed",
            Part::Apply => "selections and applications typed (resolutions excluded)",
            Part::Copy => "argument expressions copied into their uses",
            Part::Relocate => "diagnostics relocated",
        }
    }

    fn short(self) -> &'static str {
        match self {
            Part::TypeArgs => "targs",
            Part::Bind => "bind",
            Part::Fold => "fold",
            Part::Match => "match",
            Part::Intrinsic => "intrinsics",
            Part::Closure => "closures",
            Part::Apply => "applications",
            Part::Copy => "copies",
            Part::Relocate => "relocation",
        }
    }
}

struct PartFrame {
    part: Part,
    callee: Option<SymId>,
    start: Instant,
    child: u64,
    /// How many events were open when the part started: an event that exits at this depth was
    /// directly inside the part, and its time is the part's child time.
    event_depth: usize,
}

#[derive(Default, Clone, Copy)]
pub struct PartRow {
    pub count: u32,
    pub self_ns: u64,
}

/// Where the self time of the inline expansions went, by part and by callee, with the counts
/// the parts report: interpreter runs and steps of the folding, rollbacks of the type
/// variable trail inside expansions, nodes copied.
#[derive(Default)]
pub struct InlineSplit {
    rows: [PartRow; 9],
    by_callee: FxMap<SymId, [PartRow; 9]>,
    open: Vec<PartFrame>,
    pub fold_runs: u64,
    pub fold_steps: u64,
    /// The time of the interpreter runs themselves, their setup and teardown included.
    pub fold_interp_ns: u64,
    pub fold_literals: u64,
    pub rollbacks: u64,
    pub rollback_entries: u64,
    pub copied_nodes: u64,
}

/// The parts of the type phase, for the split `--profile` prints: what runs before any body is
/// typed (entering, the eager completions), the bodies, what a body completes lazily on its
/// way (a class, a signature, an index), and what runs after the bodies. A part calls
/// `Worker::phase` on entry and `phase_end` on exit; off, each is one branch on `Profile::on`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Classpath,
    Enter,
    StdClasses,
    Complete,
    Alias,
    Exports,
    Signature,
    Body,
    GivenIndex,
    ConversionIndex,
    /// The signature phase's pass over the program's scopes (`build_scope_tables`): its own
    /// time, the tables it builds being rows of their own.
    Tables,
    Macro,
    ClassCheck,
    Final,
    /// A std or library body the reach pass or the interpreter asked for.
    Deferred,
    /// A std file entered after `enter_all`, parsed and checked on the spot.
    StdEnter,
    TastyRead,
    LibraryComplete,
    LibrarySignature,
    TermDecode,
    Convert,
    LibraryClassCheck,
    /// The reach pass's own walk, the typer's work it asks for excluded.
    ReachWalk,
    /// The merge after the body phase: the chunk's records renumbered (`merge.rs`).
    Merge,
}

impl Phase {
    const ALL: [Phase; 24] = [
        Phase::Classpath,
        Phase::Enter,
        Phase::StdClasses,
        Phase::Complete,
        Phase::Alias,
        Phase::Exports,
        Phase::Signature,
        Phase::Body,
        Phase::GivenIndex,
        Phase::ConversionIndex,
        Phase::Tables,
        Phase::Macro,
        Phase::ClassCheck,
        Phase::Final,
        Phase::Deferred,
        Phase::StdEnter,
        Phase::TastyRead,
        Phase::LibraryComplete,
        Phase::LibrarySignature,
        Phase::TermDecode,
        Phase::Convert,
        Phase::LibraryClassCheck,
        Phase::ReachWalk,
        Phase::Merge,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Phase::Classpath => "classpath opened",
            Phase::Enter => "definitions entered",
            Phase::StdClasses => "std classes found",
            Phase::Complete => "classes completed",
            Phase::Alias => "aliases completed",
            Phase::Exports => "export tables built",
            Phase::Signature => "signatures completed",
            Phase::Body => "bodies typed",
            Phase::GivenIndex => "given indexes built",
            Phase::ConversionIndex => "conversion indexes built",
            Phase::Tables => "scope tables pass (the walk over the program's scopes)",
            Phase::Macro => "macros expanded",
            Phase::ClassCheck => "classes checked (parents, overrides, variances)",
            Phase::Final => "passes after the bodies",
            Phase::Deferred => "std and library bodies asked for",
            Phase::StdEnter => "std files entered while typing",
            Phase::TastyRead => "TASTy files inflated and indexed",
            Phase::LibraryComplete => "library classes completed",
            Phase::LibrarySignature => "library signatures decoded",
            Phase::TermDecode => "library bodies decoded from TASTy",
            Phase::Convert => "library classes converted to ASTs",
            Phase::LibraryClassCheck => "library classes checked",
            Phase::ReachWalk => "the reach walk",
            Phase::Merge => "chunks merged after the bodies",
        }
    }

    /// Whether the part is the threads' in the parallel typer's split: the bodies with what they
    /// trigger, the macro expansions, the class checks
    /// (a class's check belongs to its work item with its member bodies) and the std and
    /// library bodies asked for. The rest is the sequential part: entering, the completions,
    /// the tables of the signature phase, the final passes, the loader's work before the
    /// walk; an entry of it made from inside a body counts with the threads whatever its
    /// part (`lazy`). The merge's walk is neither: it runs under `--profile` only.
    pub fn on_threads(self) -> bool {
        matches!(self, Phase::Body | Phase::Macro | Phase::ClassCheck | Phase::LibraryClassCheck | Phase::Deferred)
    }

    fn short(self) -> &'static str {
        match self {
            Phase::Classpath => "classpath",
            Phase::Enter => "enter",
            Phase::StdClasses => "std-classes",
            Phase::Complete => "complete",
            Phase::Alias => "alias",
            Phase::Exports => "exports",
            Phase::Signature => "signature",
            Phase::Body => "body",
            Phase::GivenIndex => "given-index",
            Phase::ConversionIndex => "conversion-index",
            Phase::Tables => "tables",
            Phase::Macro => "macro",
            Phase::ClassCheck => "class-check",
            Phase::Final => "final",
            Phase::Deferred => "deferred",
            Phase::StdEnter => "std-enter",
            Phase::TastyRead => "tasty-read",
            Phase::LibraryComplete => "library-complete",
            Phase::LibrarySignature => "library-signature",
            Phase::TermDecode => "term-decode",
            Phase::Convert => "convert",
            Phase::LibraryClassCheck => "library-class-check",
            Phase::ReachWalk => "reach-walk",
            Phase::Merge => "merge",
        }
    }
}

struct OpenPhase {
    phase: Phase,
    start: Instant,
    child: u64,
    /// The row in `Profile::bodies` of a `Body` phase.
    body: usize,
    /// Its part of the given search or extension lookup it was entered from inside.
    step: StepTok,
}

#[derive(Default, Clone, Copy)]
struct PhaseRow {
    count: u32,
    self_ns: u64,
    /// The entries made from inside a body.
    lazy_count: u32,
    lazy_self_ns: u64,
    /// The share made under a body typed before the walk, for a table of the signature phase:
    /// the sequential part's, whatever the row.
    early_self_ns: u64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BodyWhere {
    Program,
    Std,
    Library,
}

/// One member's body typed: a member of a package or of a class that stands in no body; the
/// bodies of locals and of the members of local and anonymous classes belong to the body they
/// stand in.
pub struct BodyRow {
    pub sym: SymId,
    pub file: FileId,
    pub where_: BodyWhere,
    /// Typed for the result type its definition leaves to inference.
    pub inferred: bool,
    /// Typed from inside another body, which needed it.
    pub nested: bool,
    /// Typed for the reach pass or a macro, after the walk over the files.
    pub reach: bool,
    /// Typed before the walk, for a table of the signature phase (a conversion's shape).
    pub early: bool,
    /// How many bodies typed for an inferred result type enclose it, itself included when it
    /// is one.
    pub chain: u16,
    pub incl_ns: u64,
    pub self_ns: u64,
    /// Whether it named a definition, other than itself, whose result type is inferred.
    pub reads_inferred: bool,
    /// Whether one of those had no body typed yet, so that this body typed it.
    pub blocked: bool,
    /// Whether one of those stands in another file.
    pub reads_other_file: bool,
    /// The bodies typed from inside it for an inferred result type.
    pub triggered: u32,
}

#[derive(Default)]
pub struct Profile {
    pub on: bool,
    /// Whether a signature read is noted: `on`, or the overlays' measurement
    /// (`TypeStore::note_signature`); one test where a read asks.
    pub sig_hook: bool,
    /// What the merge after the body phase walked (`merge.rs`).
    pub merge: super::merge::MergeStats,
    events: Vec<Event>,
    open: Vec<usize>,
    phase_rows: [PhaseRow; 24],
    /// The rows as the reach pass began, which a build with a classpath types bodies in.
    rows_before_reach: Option<[PhaseRow; 24]>,
    phase_open: Vec<OpenPhase>,
    /// The rows in `bodies` of the bodies being typed, outermost first.
    open_bodies: Vec<usize>,
    pub bodies: Vec<BodyRow>,
    /// Per definition whose result type is inferred, how often a body other than its own asked
    /// for its signature, and how often before its body was typed.
    inferred_reads: FxMap<SymId, (u32, u32)>,
    /// Candidates instantiated so far: `try_given` and `try_conversion` count themselves here.
    pub tries: u32,
    pub given: GivenSplit,
    pub inline: InlineSplit,
    /// What the given searches under way were asked by (`Origin`), set by the summon intrinsics.
    pub summon: Origin,
    /// The counts an overload resolution reports: alternatives that fit the shape, applicable ones.
    pub fitting: u32,
    pub applicable: u32,
    /// The function literals typed ahead of an overload resolution, by their typed expression.
    ahead: FxMap<crate::tir::TExprId, usize>,
    /// The interpreter's histogram over every macro expansion, and per callee.
    macro_all: MacroRows,
    macro_by_callee: FxMap<SymId, MacroRows>,
    /// The signature reads (`sig_of` and `sig_arc`, through `completed_sig`), the `Arc` clones
    /// `sig_arc` handed out, and, a heuristic, the reads at which the last clone's allocation
    /// had fewer owners than when it was handed out: a clone released before the next read
    /// counts, and so does another owner's release meanwhile, so the count is an upper bound
    /// on the short-lived clones, not their number. Completion's own clones and the direct
    /// ones are not counted.
    /// The waits on cells claimed by other threads and the time spent in them, and the
    /// bodies typed on this thread for a signature another thread's item owns.
    pub waited: u64,
    pub wait_ns: u64,
    pub stolen: u64,
    sig_reads: u64,
    sig_clones: u64,
    sig_dropped: u64,
    last_sig: Option<(std::sync::Weak<MethodSig>, usize)>,
}

/// Where the time of a set of macro expansions went: the typing of the splices, the interpreter
/// runs by interpreted function and builtin, the allocations.
#[derive(Default)]
pub struct MacroRows {
    pub count: u32,
    pub pre_ns: u64,
    pub run_ns: u64,
    pub run_ticks: u64,
    pub rows: FxMap<ProfKey, ProfRow>,
    pub allocs: Allocs,
}

impl MacroRows {
    fn absorb(&mut self, other: MacroRows) {
        self.count += other.count;
        self.pre_ns += other.pre_ns;
        self.run_ns += other.run_ns;
        self.run_ticks += other.run_ticks;
        for (key, row) in other.rows {
            self.rows.entry(key).or_default().add(&row);
        }
        self.allocs.add(&other.allocs);
    }

    fn remap(&mut self, key: impl Fn(ProfKey) -> ProfKey) {
        let old = std::mem::take(&mut self.rows);
        self.rows.extend(old.into_iter().map(|(k, row)| (key(k), row)));
    }

    fn add(&mut self, pre_ns: u64, run: (u64, u64), prof: &InterpProf) {
        self.count += 1;
        self.pre_ns += pre_ns;
        self.run_ns += run.0;
        self.run_ticks += run.1;
        for (key, row) in prof.rows() {
            self.rows.entry(key.clone()).or_default().add(row);
        }
        self.allocs.add(&prof.allocs);
    }

    /// Nanoseconds per tick of the counter the interpreter's hooks read, from the runs timed
    /// both ways.
    fn ns_per_tick(&self) -> f64 {
        if self.run_ticks == 0 { 0.0 } else { self.run_ns as f64 / self.run_ticks as f64 }
    }
}

/// A run of the interpreter timed both ways: the clock's nanoseconds and the counter's ticks.
pub struct RunClock {
    start: Instant,
    ticks: u64,
}

impl RunClock {
    pub fn start() -> RunClock {
        RunClock { start: Instant::now(), ticks: ticks() }
    }

    pub fn stop(&self) -> (u64, u64) {
        (self.start.elapsed().as_nanos() as u64, ticks().wrapping_sub(self.ticks))
    }
}

/// The groups the histogram sums the interpreter's self time into.
#[derive(Clone, Copy, PartialEq, Eq)]
enum RowKind {
    Own,
    Std,
    Reflect,
    Native,
    Quote,
}

impl RowKind {
    const ALL: [RowKind; 5] = [RowKind::Own, RowKind::Std, RowKind::Reflect, RowKind::Native, RowKind::Quote];

    fn name(self) -> &'static str {
        match self {
            RowKind::Own => "the macros' own code",
            RowKind::Std => "std and library code",
            RowKind::Reflect => "reflect builtins",
            RowKind::Native => "native builtins",
            RowKind::Quote => "quote instantiation and matching",
        }
    }

    fn short(self) -> &'static str {
        match self {
            RowKind::Own => "own",
            RowKind::Std => "std",
            RowKind::Reflect => "reflect",
            RowKind::Native => "native",
            RowKind::Quote => "quote",
        }
    }
}

impl Profile {
    /// The profile on or off, and the signature reads' hook with it.
    pub fn set_on(&mut self, on: bool) {
        self.on = on;
        self.sig_hook = on;
        if on && self.given.parts.calibration.is_none() {
            self.given.parts.calibration = Some((Instant::now(), ticks()));
        }
    }

    /// A signature read: settles whether the clone the previous read handed out is gone.
    /// A worker's profile, on when the build's is, its signature reads hooked when the
    /// build's are (the profile, the routes' notes, the overlays).
    pub fn worker(on: bool, sig_hook: bool) -> Profile {
        Profile { on, sig_hook, ..Default::default() }
    }

    /// Adds what another worker's profile counted: the rows summed, the bodies and events
    /// appended, so that the report reads as one worker's over every thread's work.
    pub fn absorb(&mut self, other: Profile) {
        if !self.on {
            return;
        }
        for (mine, theirs) in self.phase_rows.iter_mut().zip(other.phase_rows.iter()) {
            mine.count += theirs.count;
            mine.self_ns += theirs.self_ns;
            mine.lazy_count += theirs.lazy_count;
            mine.lazy_self_ns += theirs.lazy_self_ns;
            mine.early_self_ns += theirs.early_self_ns;
        }
        let base = self.events.len() as u32;
        self.given.parts.absorbed += 1;
        let worker = self.given.parts.absorbed;
        self.events.extend(other.events.into_iter().map(|mut e| {
            if e.parent != u32::MAX {
                e.parent += base;
            }
            e.worker = worker;
            e
        }));
        self.bodies.extend(other.bodies);
        for (s, (reads, before)) in other.inferred_reads {
            let e = self.inferred_reads.entry(s).or_default();
            e.0 += reads;
            e.1 += before;
        }
        self.tries += other.tries;
        self.fitting += other.fitting;
        self.applicable += other.applicable;
        let g = &mut self.given;
        let o = other.given;
        g.cands += o.cands;
        g.tries += o.tries;
        g.fails += o.fails;
        g.fits += o.fits;
        g.pruned += o.pruned;
        g.head_rejected += o.head_rejected;
        g.cached += o.cached;
        g.shape_cached += o.shape_cached;
        g.shape_checks += o.shape_checks;
        g.shape_unkept += o.shape_unkept;
        for (a, b) in g.in_progress.iter_mut().zip(o.in_progress.iter()) {
            *a += b;
        }
        let (gp, op) = (&mut g.parts, &o.parts);
        for (a, b) in gp.step_count.iter_mut().zip(op.step_count.iter()) {
            *a += b;
        }
        for (a, b) in gp.lazy_by_phase.iter_mut().zip(op.lazy_by_phase.iter()) {
            a.absorb(*b);
        }
        for (a, b) in gp.early.iter_mut().zip(op.early.iter()) {
            a.0.absorb(b.0);
            a.1.absorb(b.1);
        }
        gp.own_ns += op.own_ns;
        gp.own_ticks += op.own_ticks;
        gp.pre_event.0.absorb(op.pre_event.0);
        gp.pre_event.1.absorb(op.pre_event.1);
        for (a, b) in gp.fit_why.iter_mut().flatten().zip(op.fit_why.iter().flatten()) {
            a.absorb(*b);
        }
        for (a, b) in gp.levels.iter_mut().zip(op.levels.iter()) {
            a.absorb(*b);
        }
        for (s, row) in o.by_candidate {
            let e = g.by_candidate.entry(s).or_default();
            e.shape += row.shape;
            e.tries += row.tries;
            e.fails += row.fails;
            e.self_ns += row.self_ns;
        }
        let i = &mut self.inline;
        let o = other.inline;
        for (a, b) in i.rows.iter_mut().zip(o.rows.iter()) {
            a.count += b.count;
            a.self_ns += b.self_ns;
        }
        for (s, rows) in o.by_callee {
            let e = i.by_callee.entry(s).or_default();
            for (a, b) in e.iter_mut().zip(rows.iter()) {
                a.count += b.count;
                a.self_ns += b.self_ns;
            }
        }
        i.fold_runs += o.fold_runs;
        i.fold_steps += o.fold_steps;
        i.fold_interp_ns += o.fold_interp_ns;
        i.fold_literals += o.fold_literals;
        i.rollbacks += o.rollbacks;
        i.rollback_entries += o.rollback_entries;
        i.copied_nodes += o.copied_nodes;
        self.macro_all.absorb(other.macro_all);
        for (s, rows) in other.macro_by_callee {
            match self.macro_by_callee.entry(s) {
                std::collections::hash_map::Entry::Occupied(mut e) => e.get_mut().absorb(rows),
                std::collections::hash_map::Entry::Vacant(e) => {
                    e.insert(rows);
                }
            }
        }
        self.waited += other.waited;
        self.wait_ns += other.wait_ns;
        self.stolen += other.stolen;
        self.sig_reads += other.sig_reads;
        self.sig_clones += other.sig_clones;
        self.sig_dropped += other.sig_dropped;
    }

    /// The records under the merge's ids (`merge.rs`): what they hold of the body phase's
    /// symbols, classes, types and expressions.
    pub fn remap(&mut self, sym: impl Fn(SymId) -> SymId, class: impl Fn(ClassId) -> ClassId, ty: impl Fn(TypeId) -> TypeId, expr: impl Fn(crate::tir::TExprId) -> crate::tir::TExprId) {
        if !self.on {
            return;
        }
        for e in &mut self.events {
            e.about = match e.about {
                About::Type(t) => About::Type(ty(t)),
                About::Sym(s) => About::Sym(sym(s)),
                About::Member(t, n) => About::Member(ty(t), n),
                About::Class(c) => About::Class(class(c)),
                About::None => About::None,
            };
            e.asked_by = e.asked_by.map(&sym);
        }
        for b in &mut self.bodies {
            b.sym = sym(b.sym);
        }
        for table in [&mut self.inferred_reads] {
            let old = std::mem::take(table);
            table.extend(old.into_iter().map(|(s, v)| (sym(s), v)));
        }
        let old = std::mem::take(&mut self.given.by_candidate);
        self.given.by_candidate.extend(old.into_iter().map(|(s, v)| (sym(s), v)));
        let old = std::mem::take(&mut self.inline.by_callee);
        self.inline.by_callee.extend(old.into_iter().map(|(s, v)| (sym(s), v)));
        let old = std::mem::take(&mut self.ahead);
        self.ahead.extend(old.into_iter().map(|(e, i)| (expr(e), i)));
        let key = |k: ProfKey| match k {
            ProfKey::Fun(s) => ProfKey::Fun(sym(s)),
            ProfKey::Lambda(e) => ProfKey::Lambda(expr(e)),
            ProfKey::Ctor(c) => ProfKey::Ctor(class(c)),
            builtin @ ProfKey::Builtin(_) => builtin,
        };
        self.macro_all.remap(&key);
        let old = std::mem::take(&mut self.macro_by_callee);
        self.macro_by_callee.extend(old.into_iter().map(|(s, mut v)| {
            v.remap(&key);
            (sym(s), v)
        }));
    }

    pub fn sig_read(&mut self) {
        self.sig_reads += 1;
        if let Some((last, count)) = self.last_sig.take() {
            self.sig_dropped += (last.strong_count() < count) as u64;
        }
    }

    pub fn sig_cloned(&mut self, sig: &std::sync::Arc<MethodSig>) {
        self.sig_clones += 1;
        self.last_sig = Some((std::sync::Arc::downgrade(sig), std::sync::Arc::strong_count(sig)));
    }

    pub fn enter(&mut self, kind: Kind, file: FileId, pos: u32, about: About) -> usize {
        let nested = self.open.iter().filter(|&&i| self.events[i].kind == kind).count() as u16;
        let i = self.events.len();
        self.events.push(Event {
            kind,
            file,
            pos,
            about,
            start: Instant::now(),
            start_ticks: ticks(),
            incl: 0,
            child: 0,
            child_ticks: 0,
            nested,
            tries_at_entry: self.tries,
            tried: 0,
            applicable: 0,
            steps: 0,
            outcome: Outcome::NotFound,
            cands: 0,
            tried_here: 0,
            failed_here: 0,
            origin: Origin::Plain,
            memo: Memo::None,
            keep: Keep::None,
            asked_by: if kind == Kind::Given { self.given.trying.last().map(|t| t.given) } else { None },
            targs: 0,
            parts: [0; N_STEPS],
            byname: false,
            ground: false,
            under_ext: kind == Kind::Given && self.open.iter().any(|&o| self.events[o].kind == Kind::Extension),
            hit_nomatch: false,
            ctx: None,
            parent: if kind == Kind::Given { self.open_given().map_or(u32::MAX, |p| p as u32) } else { u32::MAX },
            worker: 0,
        });
        self.open.push(i);
        if matches!(kind, Kind::Given | Kind::Extension) {
            let start = self.events[i].start_ticks;
            self.given.parts.clocks.push(StepClock { event: i, base: self.given.parts.frames.len(), last: start, child_at_last: 0 });
            self.given.parts.frames.push(StepFrame { step: Step::Rest, acc: 0 });
        }
        i
    }

    pub fn set_request(&mut self, i: usize, byname: bool, ground: bool) {
        self.events[i].byname = byname;
        self.events[i].ground = ground;
    }

    pub fn set_hit_nomatch(&mut self, i: usize) {
        self.events[i].hit_nomatch = true;
    }

    pub fn set_ctx(&mut self, i: usize, ctx: (FileId, u64)) {
        self.events[i].ctx = Some(ctx);
    }


    /// The clock of the innermost open event, when that event keeps one.
    fn innermost_clock(&self) -> Option<usize> {
        let c = self.given.parts.clocks.last()?;
        (self.open.last() == Some(&c.event)).then_some(self.given.parts.clocks.len() - 1)
    }

    /// Gives the time since the clock's last reading to its innermost part.
    fn credit(&mut self, c: usize, now: u64) {
        let clock = &mut self.given.parts.clocks[c];
        let child = self.events[clock.event].child_ticks;
        let seg = now.saturating_sub(clock.last).saturating_sub(child - clock.child_at_last);
        clock.last = now;
        clock.child_at_last = child;
        self.given.parts.frames.last_mut().expect("a clock has its residual frame").acc += seg;
    }

    /// Enters a part of the innermost event's own time, when that event keeps a clock: the
    /// part's frame, for `step_pop`.
    #[cold]
    #[inline(never)]
    pub fn step_push(&mut self, step: Step) -> StepTok {
        let Some(c) = self.innermost_clock() else { return StepTok(None) };
        self.credit(c, ticks());
        self.given.parts.frames.push(StepFrame { step, acc: 0 });
        StepTok(Some(self.given.parts.frames.len() as u32 - 1))
    }

    /// Leaves the part `step_push` entered, its time going to `step`; the time is returned. A
    /// part left out of order (an event or a part opened inside it and not closed) is left to
    /// the event's close, which credits it to the part it was entered as.
    #[cold]
    #[inline(never)]
    pub fn step_pop(&mut self, tok: StepTok, step: Step) -> u64 {
        let Some(frame) = tok.0 else { return 0 };
        let in_order = self.innermost_clock().map_or(false, |c| self.given.parts.clocks[c].base < frame as usize) && self.given.parts.frames.len() == frame as usize + 1;
        debug_assert!(in_order, "a part ends in the event it began in, after the parts inside it");
        if !in_order {
            return 0;
        }
        let c = self.given.parts.clocks.len() - 1;
        self.credit(c, ticks());
        let frame = self.given.parts.frames.pop().expect("a part is open");
        self.events[self.given.parts.clocks[c].event].parts[step as usize] += frame.acc;
        self.given.parts.step_count[step as usize] += 1;
        frame.acc
    }

    /// A fit decision made, its time going to the part `how` says and, for a shape check run,
    /// to why the fit memo did not answer it.
    #[cold]
    #[inline(never)]
    pub fn fit_decided(&mut self, tok: StepTok, how: Step, why: Option<FitWhy>) {
        let ticks = self.step_pop(tok, how);
        if let (Some(why), Some(k)) = (why, FIT_HOWS.iter().position(|&h| h == how)) {
            self.given.parts.fit_why[k][why as usize].add(ticks);
        }
    }

    #[cold]
    #[inline(never)]
    fn pre_event_enter(&mut self) -> PreEvent {
        PreEvent { start: Some(ticks()), tok: self.step_push(Step::PreEvent) }
    }

    /// The end of a search's checks before its event, at an early return (`how`) or as its event
    /// starts.
    #[cold]
    #[inline(never)]
    fn pre_event_exit(&mut self, pre: PreEvent, how: Option<Early>) {
        let Some(start) = pre.start else { return };
        let ns = ticks().wrapping_sub(start);
        // The checks are a given search's own time when the clock that took them is a given
        // search's, not an extension lookup's.
        let inside = pre.tok.0.is_some() && self.given.parts.clocks.last().map_or(false, |c| self.events[c.event].kind == Kind::Given);
        let row = match how {
            Some(how) => &mut self.given.parts.early[how as usize],
            None => &mut self.given.parts.pre_event,
        };
        row.0.add(ns);
        if inside {
            row.1.add(ns);
        }
        self.step_pop(pre.tok, Step::PreEvent);
    }

    /// Closes the clock of the event `i` at `now`: every part still open is credited to itself,
    /// and the parts, counted in ticks, become shares of the event's own time `own_ns`, the
    /// rounding left to the residual, so that they sum to it.
    fn close_clock(&mut self, i: usize, now: u64, own_ns: u64) {
        let c = self.given.parts.clocks.len() - 1;
        debug_assert_eq!(self.given.parts.clocks[c].event, i, "the innermost clock is the closing event's");
        self.credit(c, now);
        let base = self.given.parts.clocks[c].base;
        debug_assert_eq!(self.given.parts.frames.len(), base + 1, "every part of the event was left");
        for frame in self.given.parts.frames.drain(base.min(self.given.parts.frames.len())..) {
            self.events[i].parts[frame.step as usize] += frame.acc;
        }
        self.given.parts.clocks.pop();
        let parts = &mut self.events[i].parts;
        let own_ticks: u64 = parts.iter().sum();
        let mut given_out = 0u64;
        for p in parts.iter_mut() {
            *p = if own_ticks == 0 { 0 } else { (*p as u128 * own_ns as u128 / own_ticks as u128) as u64 };
            given_out += *p;
        }
        parts[Step::Rest as usize] += own_ns.saturating_sub(given_out);
        self.given.parts.own_ns += own_ns;
        self.given.parts.own_ticks += own_ticks;
    }

    pub fn set_origin(&mut self, i: usize, origin: Origin) {
        self.events[i].origin = origin;
    }

    pub fn set_memo(&mut self, i: usize, memo: Memo) {
        self.events[i].memo = memo;
    }

    pub fn set_keep(&mut self, i: usize, keep: Keep) {
        self.events[i].keep = keep;
    }

    pub fn set_targs(&mut self, i: usize, targs: u64) {
        self.events[i].targs = targs;
    }

    /// What the result memo did for a search and what became of its result, for the trace.
    pub fn memo_of(&self, i: usize) -> String {
        format!("memo={} keep={}", self.events[i].memo.name(), self.events[i].keep.name())
    }

    /// The innermost given search under way.
    fn open_given(&self) -> Option<usize> {
        self.open.iter().rev().copied().find(|&i| self.events[i].kind == Kind::Given)
    }

    #[inline(always)]
    pub fn tick(&self) -> Option<Instant> {
        if self.on { Some(Instant::now()) } else { None }
    }

    /// The candidates one level of a search yielded.
    #[inline(always)]
    pub fn given_found(&mut self, n: usize) {
        if !self.on {
            return;
        }
        self.given.cands += n as u64;
        if let Some(i) = self.open_given() {
            self.events[i].cands += n as u32;
        }
    }

    /// The start of a shape check or an instantiation of the candidate `g`, which the searches
    /// nested in it are asked by.
    #[inline(always)]
    pub fn try_start(&mut self, g: SymId) {
        if self.on {
            let owner = self.open_given();
            self.given.trying.push(Trying { given: g, owner, start: ticks(), nested: 0 });
        }
    }

    /// The end of an instantiation, or of a shape check (`shape`): the candidate's time is the
    /// whole attempt's less the given searches nested in it, with the conversion, the inline
    /// expansions and the lazy work it ran, apart from the exclusive parts (in ticks).
    #[inline(always)]
    pub fn try_end(&mut self, failed: bool, shape: bool) {
        if !self.on {
            return;
        }
        if let Some(t) = self.given.trying.pop() {
            let row = self.given.by_candidate.entry(t.given).or_default();
            row.self_ns += ticks().wrapping_sub(t.start).saturating_sub(t.nested);
            row.shape += shape as u32;
            row.tries += !shape as u32;
            row.fails += (!shape && failed) as u32;
        }
        if shape {
            self.given.shape_checks += 1;
            return;
        }
        self.given.tries += 1;
        self.given.fails += failed as u64;
        if let Some(i) = self.open_given() {
            self.events[i].tried_here += 1;
            self.events[i].failed_here += failed as u32;
        }
    }

    pub fn exit(&mut self, i: usize, outcome: Outcome) {
        let now = ticks();
        let incl = self.events[i].start.elapsed().as_nanos() as u64;
        let incl_ticks = now.wrapping_sub(self.events[i].start_ticks);
        if matches!(self.events[i].kind, Kind::Given | Kind::Extension) {
            let own_ns = incl.saturating_sub(self.events[i].child);
            self.close_clock(i, now, own_ns);
            debug_assert!(
                {
                    let e = &self.events[i];
                    let parts: u64 = e.parts.iter().sum();
                    parts.abs_diff(incl.saturating_sub(e.child)) <= N_STEPS as u64
                },
                "the parts of an event sum to its own time"
            );
        }
        let e = &mut self.events[i];
        e.incl = incl;
        e.outcome = outcome;
        e.tried = self.tries - e.tries_at_entry;
        self.open.pop();
        if let Some(&parent) = self.open.last() {
            self.events[parent].child += incl;
            self.events[parent].child_ticks += incl_ticks;
        }
        // A given search nested in a candidate's attempt is not the candidate's time.
        if self.events[i].kind == Kind::Given {
            let parent = self.open_given();
            if let Some(t) = self.given.trying.last_mut().filter(|t| t.owner == parent) {
                t.nested += incl_ticks;
            }
        }
        if let Some(top) = self.inline.open.last_mut() {
            if top.event_depth == self.open.len() {
                top.child += incl;
            }
        }
    }

    /// A part counts when the innermost search or expansion under way is an inline expansion,
    /// whose self time it then splits; inside a nested search (an argument typed under an
    /// overload resolution) it is that search's time and stays out of the split.
    pub fn part_enter(&mut self, part: Part) {
        let callee = self.open.last().and_then(|&i| match (self.events[i].kind, self.events[i].about) {
            (Kind::Inline, About::Sym(s)) => Some(s),
            _ => None,
        });
        self.inline.open.push(PartFrame { part, callee, start: Instant::now(), child: 0, event_depth: self.open.len() });
    }

    /// The exit of a part: its own time is what its nested parts and events left. A nested
    /// part with an event between it and this one is inside that event's time already.
    pub fn part_exit(&mut self) {
        let Some(f) = self.inline.open.pop() else { return };
        let incl = f.start.elapsed().as_nanos() as u64;
        let self_ns = incl.saturating_sub(f.child);
        if let Some(c) = f.callee {
            let row = &mut self.inline.rows[f.part as usize];
            row.count += 1;
            row.self_ns += self_ns;
            let row = &mut self.inline.by_callee.entry(c).or_default()[f.part as usize];
            row.count += 1;
            row.self_ns += self_ns;
        }
        if let Some(parent) = self.inline.open.last_mut() {
            if parent.event_depth == f.event_depth {
                parent.child += incl;
            }
        }
    }

    pub fn note_steps(&mut self, i: usize, steps: u64) {
        self.events[i].steps = steps;
    }

    /// The histogram of one expansion: the time its splice took to type, the run, and what the
    /// interpreter counted.
    pub fn note_macro(&mut self, callee: SymId, pre_ns: u64, run: (u64, u64), prof: &InterpProf) {
        self.macro_all.add(pre_ns, run, prof);
        self.macro_by_callee.entry(callee).or_default().add(pre_ns, run, prof);
    }

    /// The exit of an overload resolution, which counted its alternatives into `fitting` and
    /// `applicable`.
    pub fn exit_counted(&mut self, i: usize, outcome: Outcome) {
        self.exit(i, outcome);
        let (fitting, applicable) = (self.fitting, self.applicable);
        let e = &mut self.events[i];
        e.tried = fitting;
        e.applicable = applicable;
        self.fitting = 0;
        self.applicable = 0;
    }

    /// A function literal typed ahead: kept unless `retried` says otherwise.
    pub fn typed_ahead(&mut self, i: usize, te: crate::tir::TExprId) {
        self.exit(i, Outcome::Kept);
        self.ahead.insert(te, i);
    }

    pub fn retried(&mut self, te: crate::tir::TExprId) {
        if let Some(&i) = self.ahead.get(&te) {
            self.events[i].outcome = Outcome::Found;
        }
    }

    fn in_phase(&self, phase: Phase) -> bool {
        self.phase_open.iter().any(|o| o.phase == phase)
    }

    pub fn in_body(&self) -> bool {
        !self.open_bodies.is_empty()
    }

    fn in_early_body(&self) -> bool {
        self.open_bodies.first().map_or(false, |&b| self.bodies[b].early)
    }

    pub fn phase_enter(&mut self, phase: Phase) -> usize {
        if phase == Phase::ReachWalk {
            self.rows_before_reach = Some(self.phase_rows);
        }
        let step = self.step_push(Step::Lazy);
        self.phase_open.push(OpenPhase { phase, start: Instant::now(), child: 0, body: usize::MAX, step });
        self.phase_open.len() - 1
    }

    pub fn phase_exit(&mut self, i: usize) {
        debug_assert_eq!(self.phase_open.len(), i + 1);
        let o = self.phase_open.pop().expect("a phase is open");
        let incl = o.start.elapsed().as_nanos() as u64;
        if o.step.0.is_some() {
            let lazy = self.step_pop(o.step, Step::Lazy);
            self.given.parts.lazy_by_phase[o.phase as usize].add(lazy);
        }
        let self_ns = incl.saturating_sub(o.child);
        if let Some(parent) = self.phase_open.last_mut() {
            parent.child += incl;
        }
        // A body's demands are the threads', and so are a class check's (the check is its
        // class's work item); a body typed before the walk, for a
        // table of the signature phase, is the sequential part's with everything under it.
        let early = if o.body != usize::MAX { self.bodies[o.body].early } else { self.in_early_body() };
        let lazy = !early && (self.in_body() || self.in_phase(Phase::ClassCheck));
        let row = &mut self.phase_rows[o.phase as usize];
        row.count += 1;
        row.self_ns += self_ns;
        if lazy {
            row.lazy_count += 1;
            row.lazy_self_ns += self_ns;
        }
        if early {
            row.early_self_ns += self_ns;
        }
        if o.body != usize::MAX {
            let b = &mut self.bodies[o.body];
            b.incl_ns = incl;
            b.self_ns = self_ns;
            self.open_bodies.pop();
        }
    }

    /// The entry of a member's body, which is a `Body` phase with a row of its own.
    pub fn body_enter(&mut self, sym: SymId, file: FileId, where_: BodyWhere, inferred: bool, before_walk: bool) -> usize {
        let nested = self.in_body();
        let reach = !nested && self.in_phase(Phase::Deferred);
        let early = !reach && before_walk;
        let enclosing = self.open_bodies.iter().filter(|&&b| self.bodies[b].inferred).count() as u16;
        if inferred {
            if let Some(&requester) = self.open_bodies.last() {
                self.bodies[requester].triggered += 1;
            }
        }
        let b = self.bodies.len();
        self.bodies.push(BodyRow {
            sym,
            file,
            where_,
            inferred,
            nested,
            reach,
            early,
            chain: enclosing + inferred as u16,
            incl_ns: 0,
            self_ns: 0,
            reads_inferred: false,
            blocked: false,
            reads_other_file: false,
            triggered: 0,
        });
        self.open_bodies.push(b);
        let i = self.phase_enter(Phase::Body);
        self.phase_open[i].body = b;
        i
    }

    /// The body being typed named a definition of `file` whose result type is inferred; `open`
    /// when that definition's body was not typed yet.
    pub fn note_inferred_read(&mut self, sym: SymId, file: FileId, open: bool) {
        let Some(&b) = self.open_bodies.last() else { return };
        let row = &mut self.bodies[b];
        row.reads_inferred = true;
        row.blocked |= open;
        row.reads_other_file |= row.file != file;
        let e = self.inferred_reads.entry(sym).or_insert((0, 0));
        e.0 += 1;
        e.1 += open as u32;
    }
}

struct BodyStats {
    count: u32,
    walk: u32,
    early: u32,
    inferred_nested: u32,
    demand: u32,
    reach: u32,
    program: u32,
    std: u32,
    library: u32,
    library_self_ns: u64,
    /// The bodies' own time by origin and by how each was reached (macro expansions and
    /// completions excluded).
    program_self_ns: u64,
    std_self_ns: u64,
    walk_self_ns: u64,
    early_self_ns: u64,
    inferred_nested_self_ns: u64,
    demand_self_ns: u64,
    reach_self_ns: u64,
    /// Over the bodies typed outside every other body, which hold the nested ones.
    incl_ns: u64,
    self_ns: u64,
    p50: u64,
    p90: u64,
    p99: u64,
    max_ns: u64,
    max_name: String,
    inferred_defs: u32,
    inferred_read: u32,
    inferred_read_open: u32,
    readers: u32,
    readers_other_file: u32,
    blocked: u32,
    /// Per depth, the bodies typed from inside another for an inferred result type.
    chains: Vec<u32>,
    file_max_ns: u64,
    file_max_name: String,
    file_p50: u64,
    top4_share: f64,
    /// Per file with bodies, largest first: path, bodies, inclusive time.
    files: Vec<(String, u32, u64)>,
    body_ns: Vec<u64>,
}

/// The percentile `p` (0 to 1) of sorted values, 0 without any.
fn percentile(sorted: &[u64], p: f64) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    sorted[((sorted.len() - 1) as f64 * p) as usize]
}

struct SiteRow {
    kind: Kind,
    file: FileId,
    pos: u32,
    about: About,
    count: u32,
    top_level: u32,
    self_ns: u64,
    incl_ns: u64,
    tried: u64,
    applicable: u64,
    depth: u16,
    found: u32,
}

struct GroupRow {
    about: About,
    count: u32,
    self_ns: u64,
    incl_ns: u64,
    tried: u64,
    applicable: u64,
    depth: u16,
    found: u32,
    steps: u64,
}

fn ms(ns: u64) -> String {
    let v = ns as f64 / 1e6;
    if v >= 100.0 {
        format!("{:.0} ms", v)
    } else if v >= 10.0 {
        format!("{:.1} ms", v)
    } else if v >= 0.01 {
        format!("{:.2} ms", v)
    } else {
        format!("{} ns", ns)
    }
}

impl<'a> Worker<'a> {
    /// The entry hook of a search: its event index while profiling, nothing otherwise.
    #[inline(always)]
    pub fn prof(&mut self, kind: Kind, span: Span, about: About) -> Option<usize> {
        if self.profile.on {
            Some(self.profile.enter(kind, self.env.file, span.start, about))
        } else {
            None
        }
    }

    /// The entry hook of a part of a given search's or an extension lookup's own time, paired
    /// with `step_end`, which names the part.
    #[inline(always)]
    pub fn step(&mut self, step: Step) -> StepTok {
        if self.profile.on {
            self.profile.step_push(step)
        } else {
            StepTok(None)
        }
    }

    #[inline(always)]
    pub fn step_end(&mut self, tok: StepTok, step: Step) -> u64 {
        if tok.0.is_some() {
            self.profile.step_pop(tok, step)
        } else {
            0
        }
    }

    /// The entry of a given search, before its event: the checks until the event starts are
    /// the enclosing search's part `PreEvent`.
    #[inline(always)]
    pub fn given_entry(&mut self) -> PreEvent {
        if self.profile.on {
            self.profile.pre_event_enter()
        } else {
            PreEvent { start: None, tok: StepTok(None) }
        }
    }

    /// A given search that ends before its event.
    #[inline(always)]
    pub fn given_early(&mut self, pre: PreEvent, how: Early) {
        if pre.start.is_some() {
            self.profile.pre_event_exit(pre, Some(how));
        }
    }

    /// A given search whose event starts now.
    #[inline(always)]
    pub fn given_reached(&mut self, pre: PreEvent) {
        if pre.start.is_some() {
            self.profile.pre_event_exit(pre, None);
        }
    }

    /// The entry hook of a part of an inline expansion, paired with `part_end`.
    #[inline(always)]
    pub fn part(&mut self, part: Part) -> bool {
        if self.profile.on {
            self.profile.part_enter(part);
        }
        self.profile.on
    }

    /// The same for a construct of the body, which is a part only inside an expansion.
    #[inline(always)]
    pub fn body_part(&mut self, part: Part) -> bool {
        if self.profile.on && self.inline.depth > 0 {
            self.profile.part_enter(part);
            return true;
        }
        false
    }

    #[inline(always)]
    pub fn part_end(&mut self, on: bool) {
        if on {
            self.profile.part_exit();
        }
    }

    /// The entry hook of a part of the type phase, paired with `phase_end`.
    #[inline(always)]
    pub fn phase(&mut self, phase: Phase) -> Option<usize> {
        if self.profile.on {
            Some(self.profile.phase_enter(phase))
        } else {
            None
        }
    }

    /// `phase`, with the library's own part of it apart.
    #[inline(always)]
    pub fn phase_split(&mut self, phase: Phase, library_phase: Phase, library: impl FnOnce(&Self) -> bool) -> Option<usize> {
        if self.profile.on {
            let p = if library(self) { library_phase } else { phase };
            Some(self.profile.phase_enter(p))
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn phase_end(&mut self, i: Option<usize>) {
        if let Some(i) = i {
            self.profile.phase_exit(i);
        }
    }

    /// The entry hook of a body, paired with `phase_end`: a member's body gets a row, the body
    /// of a local or of a member of a local class belongs to the body it stands in.
    pub(super) fn body_prof(&mut self, sym: SymId) -> Option<usize> {
        if !self.profile.on || !self.is_member_body(sym) {
            return None;
        }
        let (file, state) = {
            let s = self.syms.sym(sym);
            (s.file, s.state().get())
        };
        let where_ = if self.is_body_file(file) {
            BodyWhere::Library
        } else if self.source(file).is_std {
            BodyWhere::Std
        } else {
            BodyWhere::Program
        };
        let before_walk = !self.walk.started;
        Some(self.profile.body_enter(sym, file, where_, state == Completion::InProgress, before_walk))
    }

    fn is_member_body(&self, sym: SymId) -> bool {
        let mut owner = self.syms.sym(sym).owner;
        loop {
            match owner {
                Owner::Package(_) => return true,
                Owner::Local => return false,
                Owner::Class(c) => owner = self.syms.class(c).owner,
            }
        }
    }

    /// `sig_of` from inside a body: a definition whose result type is inferred is noted, with
    /// whether its body was typed already.
    pub(super) fn note_sig_read(&mut self, sym: SymId) {
        if !self.profile.in_body() || self.bodies_in_progress.last() == Some(&sym) {
            return;
        }
        let (kind, file, def, state, mods) = {
            let s = self.syms.sym(sym);
            (s.kind, s.file, s.def, s.state().get(), s.mods)
        };
        if !matches!(kind, SymKind::Val | SymKind::Var | SymKind::Def) || mods & crate::ast::mods::INLINE != 0 || !self.is_member_body(sym) {
            return;
        }
        let Some(def) = def else { return };
        let inferred = match &self.ast(file).def(def).kind {
            crate::ast::DefKind::Val { ty, rhs, .. } => ty.is_none() && rhs.is_some(),
            crate::ast::DefKind::Fun(f) => f.ret.is_none() && f.body.is_some(),
            _ => false,
        };
        if inferred {
            self.profile.note_inferred_read(sym, file, state != Completion::Done);
        }
    }

    /// The qualified name of a definition: its packages and classes joined by dots, the bare
    /// name for a local.
    pub fn sym_path(&self, s: SymId) -> String {
        let info = self.syms.sym(s);
        let name = self.name_str(info.name);
        match info.owner {
            Owner::Package(p) if p != ROOT_PKG => format!("{}.{}", self.class_path_prefix(Owner::Package(p)), name),
            Owner::Class(c) => format!("{}.{}", self.class_path(c), name),
            _ => name,
        }
    }

    pub fn class_path_prefix(&self, owner: Owner) -> String {
        match owner {
            Owner::Package(p) => {
                let info = self.syms.pkg(p);
                match info.parent {
                    Some(parent) if parent != ROOT_PKG => format!("{}.{}", self.class_path_prefix(Owner::Package(parent)), self.name_str(info.name)),
                    _ => self.name_str(info.name),
                }
            }
            Owner::Class(c) => self.class_path(c),
            Owner::Local => String::new(),
        }
    }

    fn about_text(&mut self, about: About) -> String {
        match about {
            About::Type(t) => self.show(t),
            About::Sym(s) => self.sym_path(s),
            About::Member(t, n) => format!("{}.{}", self.show(t), self.name_str(n)),
            About::Class(c) => self.class_path(c),
            About::None => String::new(),
        }
    }

    fn hint(&mut self, kind: Kind, about: About, found: bool) -> String {
        match kind {
            Kind::Given => match about {
                About::Type(t) if found => {
                    format!("a given written where it is used (given {} = ...) needs no search", self.show(t))
                }
                _ => "the search failed; a given written where it is used ends it".to_string(),
            },
            Kind::Overload => format!("alternatives under distinct names, or the dialect flag {}", dialect::NO_OVERLOADING),
            Kind::Conversion => match about {
                About::Member(t, n) if found => format!(
                    "an extension method {} on {} is found by name; the dialect flag {} removes conversions",
                    self.name_str(n),
                    self.show(t),
                    dialect::NO_IMPLICIT_CONVERSIONS
                ),
                About::Member(..) => format!("searched in vain: the receiver has no such member; {} skips the search", dialect::NO_IMPLICIT_CONVERSIONS),
                _ if found => format!("a value of the expected type, or an extension method; {} removes conversions", dialect::NO_IMPLICIT_CONVERSIONS),
                _ => format!("searched in vain before the type mismatch was reported; {} skips the search", dialect::NO_IMPLICIT_CONVERSIONS),
            },
            Kind::Inline => match about {
                About::Sym(s) => format!("a plain def in place of inline def {}, or the dialect flag {}", self.name_str(self.syms.sym(s).name), dialect::NO_INLINE),
                _ => format!("a plain def, or the dialect flag {}", dialect::NO_INLINE),
            },
            Kind::Extension => "an import of the extension method by name finds it in the lexical scope".to_string(),
            Kind::Retry => "a type ascription on the parameters of the function literal ends the second typing".to_string(),
            Kind::MatchType => "the reduced type written out".to_string(),
            Kind::Completion => "read from the classpath on first use".to_string(),
            Kind::LibraryBody => "compiled from the classpath because the program reaches it".to_string(),
            Kind::Macro => "the histogram at the end shows where the expansions' interpreter time goes; a std method high in it wants a native (src/interp/natives.rs), the macro's own code less work per site".to_string(),
        }
    }

    fn line_col(&self, starts: &mut FxMap<FileId, Vec<u32>>, file: FileId, pos: u32) -> (usize, usize) {
        let text = &self.source(file).text;
        let lines = starts.entry(file).or_insert_with(|| {
            let mut v = vec![0u32];
            v.extend(text.bytes().enumerate().filter(|&(_, b)| b == b'\n').map(|(i, _)| i as u32 + 1));
            v
        });
        let line = lines.partition_point(|&s| s <= pos);
        (line, (pos - lines[line - 1]) as usize + 1)
    }

    fn site_rows(&self) -> Vec<SiteRow> {
        let mut index: FxMap<(Kind, FileId, u32, About), usize> = FxMap::default();
        let mut rows: Vec<SiteRow> = Vec::new();
        for e in &self.profile.events {
            let i = *index.entry((e.kind, e.file, e.pos, e.about)).or_insert_with(|| {
                rows.push(SiteRow {
                    kind: e.kind,
                    file: e.file,
                    pos: e.pos,
                    about: e.about,
                    count: 0,
                    top_level: 0,
                    self_ns: 0,
                    incl_ns: 0,
                    tried: 0,
                    applicable: 0,
                    depth: 0,
                    found: 0,
                });
                rows.len() - 1
            });
            let r = &mut rows[i];
            r.count += 1;
            r.self_ns += e.incl - e.child;
            r.incl_ns += e.incl;
            if e.nested == 0 {
                r.top_level += 1;
                r.about = e.about;
            }
            r.tried += e.tried as u64;
            r.applicable += e.applicable as u64;
            r.depth = r.depth.max(e.nested + 1);
            r.found += (e.outcome == Outcome::Found) as u32;
        }
        rows
    }

    fn group_rows(&mut self, kind: Kind, key: impl Fn(&mut Self, About) -> About) -> Vec<GroupRow> {
        let mut index: FxMap<About, usize> = FxMap::default();
        let mut rows: Vec<GroupRow> = Vec::new();
        let n = self.profile.events.len();
        for i in 0..n {
            let e = &self.profile.events[i];
            if e.kind != kind {
                continue;
            }
            let (about, incl, child, nested, tried, applicable, outcome, steps) = (e.about, e.incl, e.child, e.nested, e.tried, e.applicable, e.outcome, e.steps);
            let k = key(self, about);
            let j = *index.entry(k).or_insert_with(|| {
                rows.push(GroupRow { about: k, count: 0, self_ns: 0, incl_ns: 0, tried: 0, applicable: 0, depth: 0, found: 0, steps: 0 });
                rows.len() - 1
            });
            let r = &mut rows[j];
            r.count += 1;
            r.steps += steps;
            r.self_ns += incl - child;
            r.incl_ns += incl;
            r.tried += tried as u64;
            r.applicable += applicable as u64;
            r.depth = r.depth.max(nested + 1);
            r.found += (outcome == Outcome::Found) as u32;
        }
        rows.sort_by(|a, b| b.self_ns.cmp(&a.self_ns));
        rows
    }

    fn given_class(&mut self, about: About) -> About {
        match about {
            About::Type(t) => match self.class_of(t) {
                Some(c) => About::Class(c),
                None => About::Type(t),
            },
            other => other,
        }
    }

    /// The report on stderr: totals per kind against the type phase, the top sites and the
    /// top groups per kind, each line with the hint that would remove its cost.
    pub fn profile_report(&mut self, type_phase: Duration, top: usize) -> String {
        use std::fmt::Write;
        let mut out = String::new();
        let type_ns = type_phase.as_nanos() as u64;
        let share = |ns: u64| if type_ns == 0 { 0.0 } else { ns as f64 * 100.0 / type_ns as f64 };
        let attributed: u64 = self.profile.events.iter().map(|e| e.incl - e.child).sum();
        let _ = writeln!(
            out,
            "profile: type phase {}, of which {} ({:.1}%) in {} searches and expansions",
            ms(type_ns),
            ms(attributed),
            share(attributed),
            self.profile.events.len()
        );
        let _ = writeln!(out, "  {:<42} {:>8} {:>10} {:>7}", "kind", "count", "self", "share");
        for kind in Kind::ALL {
            let (mut count, mut self_ns) = (0u32, 0u64);
            for e in &self.profile.events {
                if e.kind == kind {
                    count += 1;
                    self_ns += e.incl - e.child;
                }
            }
            if count == 0 {
                continue;
            }
            let _ = writeln!(out, "  {:<42} {:>8} {:>10} {:>6.1}%", kind.name(), count, ms(self_ns), share(self_ns));
        }
        self.given_section(&mut out, type_ns, top);
        self.given_parts_section(&mut out, type_ns);
        self.phase_section(&mut out, type_ns);
        let mut rows = self.site_rows();
        rows.retain(|r| !matches!(r.kind, Kind::Completion | Kind::LibraryBody));
        rows.sort_by(|a, b| b.self_ns.cmp(&a.self_ns).then(a.file.cmp(&b.file)).then(a.pos.cmp(&b.pos)));
        if !rows.is_empty() {
            let _ = writeln!(out, "top {} sites by self time (incl counts what nested inside):", top.min(rows.len()));
            let mut starts = FxMap::default();
            for r in rows.iter().take(top) {
                let (line, col) = self.line_col(&mut starts, r.file, r.pos);
                let about = self.about_text(r.about);
                let hint = self.hint(r.kind, r.about, r.found > 0);
                let path = self.source(r.file).path.clone();
                let mut detail = format!("{} x", r.count);
                if r.depth > 1 {
                    let _ = write!(detail, ", depth {}", r.depth);
                }
                if r.tried > 0 {
                    let _ = write!(detail, ", {} candidates", r.tried);
                }
                if r.kind == Kind::Overload {
                    let _ = write!(detail, ", {} applicable", r.applicable);
                }
                if r.kind == Kind::Conversion || r.kind == Kind::Given {
                    let _ = write!(detail, ", {} found", r.found);
                }
                let _ = writeln!(
                    out,
                    "  {}:{}:{}: {} {} ({}): self {}, incl {}, {:.1}%\n    hint: {}",
                    path,
                    line,
                    col,
                    r.kind.short(),
                    about,
                    detail,
                    ms(r.self_ns),
                    ms(r.incl_ns),
                    share(r.self_ns),
                    hint
                );
            }
        }
        let groups: [(Kind, &str); 9] = [
            (Kind::Given, "given searches by type class"),
            (Kind::Overload, "overload resolutions by method"),
            (Kind::Conversion, "conversion searches by receiver and member"),
            (Kind::Inline, "inline expansions by callee"),
            (Kind::Extension, "extension lookups by receiver and name"),
            (Kind::MatchType, "match type reductions"),
            (Kind::Completion, "classes completed from the classpath"),
            (Kind::LibraryBody, "library bodies compiled from the classpath"),
            (Kind::Macro, "macro expansions by callee"),
        ];
        for (kind, title) in groups {
            let rows = self.group_rows(kind, |t, a| if kind == Kind::Given { t.given_class(a) } else { a });
            if rows.is_empty() {
                continue;
            }
            let _ = writeln!(out, "{}:", title);
            for r in rows.iter().take(top) {
                let about = self.about_text(r.about);
                let mut detail = format!("{} x", r.count);
                match kind {
                    Kind::Given | Kind::Conversion => {
                        let _ = write!(detail, ", {} candidates tried, {} found", r.tried, r.found);
                        if r.depth > 1 {
                            let _ = write!(detail, ", nested {} deep", r.depth);
                        }
                    }
                    Kind::Overload => {
                        let _ = write!(detail, ", {:.1} by shape, {:.1} applicable on average", r.tried as f64 / r.count as f64, r.applicable as f64 / r.count as f64);
                    }
                    Kind::Inline => {
                        let _ = write!(detail, ", depth {}", r.depth);
                    }
                    Kind::Macro => {
                        let _ = write!(detail, ", {} interpreter steps", r.steps);
                    }
                    _ => {}
                }
                let _ = writeln!(out, "  {} ({}): self {}, incl {}, {:.1}%", about, detail, ms(r.self_ns), ms(r.incl_ns), share(r.self_ns));
            }
        }
        self.inline_section(&mut out, type_ns, top);
        self.inline_repeats_section(&mut out, type_ns, top);
        self.macro_histogram(&mut out, top);
        out
    }

    /// The expansions of one callee over the same type arguments: how many of each callee's
    /// expansions repeat an earlier one's type arguments, and the time the repeats took.
    fn inline_repeats_section(&mut self, out: &mut String, type_ns: u64, top: usize) {
        use std::fmt::Write;
        let share = |ns: u64| if type_ns == 0 { 0.0 } else { ns as f64 * 100.0 / type_ns as f64 };
        let mut seen: FxMap<(SymId, u64), ()> = FxMap::default();
        let mut by_callee: FxMap<SymId, (u32, u32, u64, u64, u64)> = FxMap::default();
        for e in self.profile.events.iter().filter(|e| e.kind == Kind::Inline) {
            let About::Sym(s) = e.about else { continue };
            let r = by_callee.entry(s).or_default();
            r.0 += 1;
            r.3 += e.incl;
            if seen.insert((s, e.targs), ()).is_some() {
                r.1 += 1;
                r.2 += e.incl;
                r.4 += e.incl - e.child;
            }
        }
        if by_callee.is_empty() {
            return;
        }
        let (all, repeats, repeat_ns): (u32, u32, u64) = by_callee.values().fold((0, 0, 0), |a, r| (a.0 + r.0, a.1 + r.1, a.2 + r.2));
        let _ = writeln!(out, "inline expansions repeating a callee's type arguments: {} of {} ({} inclusive, nested expansions counted in each); by callee (repeats of expansions, inclusive and self time of the repeats):", repeats, all, ms(repeat_ns));
        let mut rows: Vec<(SymId, (u32, u32, u64, u64, u64))> = by_callee.into_iter().filter(|r| r.1 .1 > 0).collect();
        rows.sort_by(|a, b| b.1 .2.cmp(&a.1 .2).then(a.0.cmp(&b.0)));
        for (s, (n, rep, incl, all_incl, self_ns)) in rows.iter().take(top) {
            let shown = self.about_text(About::Sym(*s));
            let _ = writeln!(out, "  {} ({} of {} x; {} of {} incl, {:.1}%; {} self)", shown, rep, n, ms(*incl), ms(*all_incl), share(*incl), ms(*self_ns));
        }
    }

    /// Where the self time of the inline expansions went: by part over all expansions, then
    /// per callee with what the parts leave (the body's other constructs: blocks, literals,
    /// ascriptions, the bookkeeping around them).
    fn inline_section(&mut self, out: &mut String, type_ns: u64, top: usize) {
        use std::fmt::Write;
        let events: Vec<(SymId, u64)> = self
            .profile
            .events
            .iter()
            .filter(|e| e.kind == Kind::Inline)
            .filter_map(|e| match e.about {
                About::Sym(s) => Some((s, e.incl - e.child)),
                _ => None,
            })
            .collect();
        if events.is_empty() {
            return;
        }
        let share = |ns: u64| if type_ns == 0 { 0.0 } else { ns as f64 * 100.0 / type_ns as f64 };
        let total_self: u64 = events.iter().map(|e| e.1).sum();
        let parts_self: u64 = self.profile.inline.rows.iter().map(|r| r.self_ns).sum();
        let _ = writeln!(out, "inline expansion split ({} expansions, {} self; the self time of each part, nested parts, searches and expansions excluded):", events.len(), ms(total_self));
        let _ = writeln!(out, "  {:<62} {:>8} {:>10} {:>7}", "part", "count", "self", "share");
        for part in Part::ALL {
            let r = self.profile.inline.rows[part as usize];
            if r.count == 0 {
                continue;
            }
            let _ = writeln!(out, "  {:<62} {:>8} {:>10} {:>6.1}%", part.name(), r.count, ms(r.self_ns), share(r.self_ns));
        }
        let _ = writeln!(out, "  {:<62} {:>8} {:>10} {:>6.1}%", "the rest of the bodies (blocks, literals, ascriptions, bookkeeping)", "", ms(total_self.saturating_sub(parts_self)), share(total_self.saturating_sub(parts_self)));
        let s = &self.profile.inline;
        let _ = writeln!(
            out,
            "  counts: {} interpreter runs for the folding ({} in all, {} steps, {} gave a literal); {} rollbacks of the type variable trail inside expansions ({} entries); {} nodes copied",
            s.fold_runs, ms(s.fold_interp_ns), s.fold_steps, s.fold_literals, s.rollbacks, s.rollback_entries, s.copied_nodes
        );
        let mut by_callee: FxMap<SymId, (u32, u64)> = FxMap::default();
        for &(s, self_ns) in &events {
            let r = by_callee.entry(s).or_insert((0, 0));
            r.0 += 1;
            r.1 += self_ns;
        }
        let mut callees: Vec<(SymId, u32, u64)> = by_callee.into_iter().map(|(s, (n, ns))| (s, n, ns)).collect();
        callees.sort_by(|a, b| b.2.cmp(&a.2).then(a.0.cmp(&b.0)));
        let _ = writeln!(out, "  by callee (count, self time, self time per expansion; then each part's self time per expansion):");
        for (sym, count, self_ns) in callees.into_iter().take(top.min(8)) {
            let rows = self.profile.inline.by_callee.get(&sym).copied().unwrap_or_default();
            let parts_ns: u64 = rows.iter().map(|r| r.self_ns).sum();
            let per = |ns: u64| ns as f64 / count as f64 / 1000.0;
            let mut detail = String::new();
            for part in Part::ALL {
                let r = rows[part as usize];
                if r.count == 0 {
                    continue;
                }
                let _ = write!(detail, "{} {:.1} µs ({} x), ", part.short(), per(r.self_ns), r.count);
            }
            let _ = write!(detail, "rest {:.1} µs", per(self_ns.saturating_sub(parts_ns)));
            let _ = writeln!(out, "    {} ({} x, {}, {:.1} µs each): {}", self.sym_path(sym), count, ms(self_ns), per(self_ns), detail);
        }
    }

    /// What the given searches did and where their time went: nesting, repeats of a target,
    /// candidates found against instantiated and failed, the split by part of the search and by
    /// what asked for it, and the targets searched most often.
    fn given_section(&mut self, out: &mut String, type_ns: u64, top: usize) {
        use std::fmt::Write;
        let events: Vec<(About, u16, u64, u64, Outcome, u32, u32, u32, Origin)> = self
            .profile
            .events
            .iter()
            .filter(|e| e.kind == Kind::Given)
            .map(|e| (e.about, e.nested, e.incl, e.child, e.outcome, e.cands, e.tried_here, e.failed_here, e.origin))
            .collect();
        if events.is_empty() {
            return;
        }
        let share = |ns: u64| if type_ns == 0 { 0.0 } else { ns as f64 * 100.0 / type_ns as f64 };
        let top_level = events.iter().filter(|e| e.1 == 0).count();
        let max_depth = events.iter().map(|e| e.1).max().unwrap_or(0) as usize + 1;
        let mut by_depth = vec![(0u32, 0u64); max_depth];
        for e in &events {
            by_depth[e.1 as usize].0 += 1;
            by_depth[e.1 as usize].1 += e.2 - e.3;
        }
        let depths: Vec<String> = by_depth.iter().enumerate().map(|(d, &(n, ns))| format!("{}: {} ({})", d + 1, n, ms(ns))).collect();
        let _ = writeln!(
            out,
            "given searches: {} in all, {} top level, {} nested; by depth {}",
            events.len(),
            top_level,
            events.len() - top_level,
            depths.join(", ")
        );
        let mut targets: FxMap<TypeId, (u32, u32, u64, u64, u64, u32)> = FxMap::default();
        let mut with_vars = 0;
        for e in &events {
            let About::Type(t) = e.0 else { continue };
            if self.types.has_vars(t) {
                with_vars += 1;
            }
            let r = targets.entry(t).or_insert((0, 0, 0, 0, 0, 0));
            r.0 += 1;
            r.1 += (e.4 == Outcome::Found) as u32;
            r.2 += e.2 - e.3;
            r.3 += e.5 as u64;
            r.4 += e.6 as u64;
            r.5 += (e.1 == 0) as u32;
        }
        let distinct = targets.len();
        let g = &self.profile.given;
        let _ = writeln!(
            out,
            "  targets: {} distinct, {} repeats (the same target type searched again, {} of the searches had open type variables); candidates {} yielded by the levels, {} instantiated in full ({} failed, {} fit by type in the shape pass), {} left untried as beaten, {} rejected by an argument's head class, {} shape checks answered from the memo, {} searches answered from the cache",
            distinct,
            events.len() - distinct,
            with_vars,
            g.cands,
            g.tries,
            g.fails,
            g.fits,
            g.pruned,
            g.head_rejected,
            g.shape_cached,
            g.cached
        );
        let mut parts = [0u64; N_STEPS];
        for e in self.profile.events.iter().filter(|e| e.kind == Kind::Given) {
            for (a, &b) in parts.iter_mut().zip(e.parts.iter()) {
                *a += b;
            }
        }
        let sum = |steps: &[Step]| steps.iter().map(|&s| parts[s as usize]).sum::<u64>();
        let find = sum(&[Step::Levels]);
        let shape = sum(&[Step::FitShapeYes, Step::FitShapeNo, Step::FitConversion]);
        let tried = sum(&[Step::TryOk, Step::TryMismatch, Step::TryIncomplete, Step::TryAmbiguous, Step::Conversion]);
        let choose = sum(&[Step::Choose]);
        let diverge = sum(&[Step::Divergence]);
        let _ = writeln!(
            out,
            "  time: finding candidates {} ({:.1}%), shape checks {} ({:.1}%, {} run, {} not kept as the candidate reads the scope), instantiating them {} ({:.1}%, nested searches, expansions and divergence checks excluded), choosing {} ({:.1}%), divergence check {} ({:.1}%)",
            ms(find),
            share(find),
            ms(shape),
            share(shape),
            g.shape_checks,
            g.shape_unkept,
            ms(tried),
            share(tried),
            ms(choose),
            share(choose),
            ms(diverge),
            share(diverge)
        );
        let _ = writeln!(out, "  result memo, by what it did for a search (count, self time of the searches, inclusive):");
        let all: Vec<(About, u16, u64, u64, Memo, Keep)> = self.profile.events.iter().filter(|e| e.kind == Kind::Given).map(|e| (e.about, e.nested, e.incl, e.child, e.memo, e.keep)).collect();
        for m in Memo::ALL {
            let (mut n, mut self_ns, mut incl) = (0u32, 0u64, 0u64);
            for e in &all {
                if e.4 == m {
                    n += 1;
                    self_ns += e.2 - e.3;
                    incl += e.2;
                }
            }
            if n > 0 {
                let _ = writeln!(out, "    {}: {} ({} self, {} incl)", m.name(), n, ms(self_ns), ms(incl));
            }
        }
        let _ = writeln!(out, "  results of the searches that ran where the memo applied (count, self time, inclusive):");
        for k in Keep::ALL {
            let (mut n, mut self_ns, mut incl) = (0u32, 0u64, 0u64);
            for e in &all {
                if e.5 == k {
                    n += 1;
                    self_ns += e.2 - e.3;
                    incl += e.2;
                }
            }
            if n > 0 {
                let _ = writeln!(out, "    {}: {} ({} self, {} incl)", k.name(), n, ms(self_ns), ms(incl));
            }
        }
        let mut asked: FxMap<SymId, (u32, u64, u64)> = FxMap::default();
        for e in self.profile.events.iter().filter(|e| e.kind == Kind::Given) {
            if let Some(g) = e.asked_by {
                let r = asked.entry(g).or_default();
                r.0 += 1;
                r.1 += e.incl - e.child;
                r.2 += e.incl;
            }
        }
        let mut asked: Vec<(SymId, (u32, u64, u64))> = asked.into_iter().collect();
        asked.sort_by(|a, b| b.1 .2.cmp(&a.1 .2).then(a.0.cmp(&b.0)));
        let _ = writeln!(out, "  reads of in-progress state, by reader (count; every search under way keeps nothing):");
        for r in super::implicits::InProgress::ALL {
            let n = self.profile.given.in_progress[r as usize];
            if n > 0 {
                let _ = writeln!(out, "    {}: {}", r.name(), n);
            }
        }
        let _ = writeln!(out, "  nested searches by the candidate whose using clause asked (count, self time, inclusive):");
        for (g, (n, self_ns, incl)) in asked.iter().take(top) {
            let shown = self.about_text(About::Sym(*g));
            let _ = writeln!(out, "    {} ({} x, {} self, {} incl)", shown, n, ms(*self_ns), ms(*incl));
        }
        let scale = |t: u64| self.profile.given.parts.ns(t);
        let mut cands: Vec<(SymId, CandidateRow)> = self.profile.given.by_candidate.iter().map(|(&g, &r)| (g, CandidateRow { self_ns: scale(r.self_ns), ..r })).collect();
        cands.sort_by(|a, b| b.1.self_ns.cmp(&a.1.self_ns).then(a.0.cmp(&b.0)));
        let _ = writeln!(out, "  costliest candidates (shape checks, instantiations, failed; own time, nested searches excluded):");
        for (g, r) in cands.iter().take(top) {
            let shown = self.about_text(About::Sym(*g));
            let _ = writeln!(out, "    {} ({} shape checks, {} instantiated, {} failed; {})", shown, r.shape, r.tries, r.fails, ms(r.self_ns));
        }
        let mut origins = String::new();
        for o in Origin::ALL {
            let (mut n, mut ns, mut tops) = (0u32, 0u64, 0u32);
            for e in &events {
                if e.8 == o {
                    n += 1;
                    ns += e.2 - e.3;
                    tops += (e.1 == 0) as u32;
                }
            }
            if n > 0 {
                let _ = write!(origins, " {}: {} ({} top level, {} self, {:.1}%);", o.name(), n, tops, ms(ns), share(ns));
            }
        }
        let _ = writeln!(out, "  by what asked:{}", origins);
        let mut rows: Vec<(TypeId, (u32, u32, u64, u64, u64, u32))> = targets.into_iter().collect();
        rows.sort_by(|a, b| b.1 .2.cmp(&a.1 .2).then(b.1 .0.cmp(&a.1 .0)).then(a.0.cmp(&b.0)));
        let _ = writeln!(out, "  costliest targets (count, top level, found, self time; candidates yielded and instantiated per search):");
        for (t, (n, found, ns, cands, tried, tops)) in rows.iter().take(top) {
            let shown = self.show(*t);
            let _ = writeln!(
                out,
                "    {} ({} x, {} top level, {} found, {}; {:.1} yielded, {:.1} instantiated)",
                shown,
                n,
                tops,
                found,
                ms(*ns),
                *cands as f64 / *n as f64,
                *tried as f64 / *n as f64
            );
        }
    }

    /// The given searches' own time split exclusively by part, summed to the row's self time
    /// with the residual, and read along the dimensions that decide what a cut can reach: what
    /// the memo did (for a miss, what the previous search of the target did), the target's
    /// shape, the request's mode, what asked, the nesting (a nested search counted once,
    /// under its outer, in the inclusive columns), the early returns, the lazy work entered
    /// from inside, and the hundred costliest targets.
    fn given_parts_section(&mut self, out: &mut String, type_ns: u64) {
        use std::fmt::Write;
        let events: Vec<usize> = (0..self.profile.events.len()).filter(|&i| self.profile.events[i].kind == Kind::Given).collect();
        if events.is_empty() {
            let g = &self.profile.given.parts;
            let early = g.early.iter().any(|(all, _)| all.n > 0);
            if early || self.profile.events.iter().any(|e| e.kind == Kind::Extension) {
                let _ = writeln!(out, "given searches: none reached its event");
                self.given_apart_tables(out, &events);
            }
            return;
        }
        let share = |ns: u64| if type_ns == 0 { 0.0 } else { ns as f64 * 100.0 / type_ns as f64 };
        let ev = |t: &Self, i: usize| -> (u64, [u64; N_STEPS]) {
            let e = &t.profile.events[i];
            let mut parts = [0u64; N_STEPS];
            for (p, &n) in parts.iter_mut().zip(e.parts.iter()) {
                *p = n;
            }
            (e.incl - e.child, parts)
        };
        let mut row_self = 0u64;
        let mut parts_sum = [0u64; N_STEPS];
        for &i in &events {
            let (own, parts) = ev(self, i);
            row_self += own;
            for (a, b) in parts_sum.iter_mut().zip(parts.iter()) {
                *a += b;
            }
        }
        let parts_total: u64 = parts_sum.iter().sum();
        let row_share = |ns: u64| if row_self == 0 { 0.0 } else { ns as f64 * 100.0 / row_self as f64 };
        let _ = writeln!(
            out,
            "given searches' own time by part (exclusive: the parts sum to the row's {} with the residual; {} in the parts, difference {} ns):",
            ms(row_self),
            ms(parts_total),
            parts_total as i64 - row_self as i64
        );
        let _ = writeln!(out, "  {:<72} {:>9} {:>10} {:>6} {:>6}", "part", "entries", "self", "row", "phase");
        for st in Step::ALL {
            let ns = parts_sum[st as usize];
            let n = self.profile.given.parts.step_count[st as usize];
            if (ns == 0 && n == 0) || st == Step::ExtCollect {
                continue;
            }
            let _ = writeln!(out, "  {:<72} {:>9} {:>10} {:>5.1}% {:>5.1}%", st.name(), n, ms(ns), row_share(ns), share(ns));
        }
        // The groups of parts the outcome rows show.
        const GROUPS: [(&str, &[Step]); 9] = [
            ("memo", &[Step::Applicable, Step::Lookup, Step::Replay, Step::Keep]),
            ("collect", &[Step::Levels]),
            ("fits", &[Step::FitMemo, Step::FitClass, Step::FitHead, Step::FitShapeYes, Step::FitShapeNo, Step::FitConversion]),
            ("tries", &[Step::TryOk, Step::TryMismatch, Step::TryIncomplete, Step::TryAmbiguous, Step::Conversion, Step::Divergence]),
            ("order+choose", &[Step::Order, Step::Choose]),
            ("synth", &[Step::Synthesis]),
            ("lazy", &[Step::Lazy]),
            ("pre", &[Step::PreEvent]),
            ("rest", &[Step::Rest, Step::ExtCollect]),
        ];
        let grouped = |parts: &[u64; N_STEPS]| -> String {
            GROUPS.iter().map(|(name, steps)| format!("{} {}", name, ms(steps.iter().map(|&s| parts[s as usize]).sum()))).collect::<Vec<_>>().join(", ")
        };
        // The class of each search by what the memo did, a miss by what the previous memo lookup
        // of its target did in the same worker (a search the memo does not apply to, under GADT
        // bounds or for a target with variables, makes no lookup and is no previous one).
        let mut class: Vec<String> = vec![String::new(); self.profile.events.len()];
        let mut last_in_ctx: FxMap<(u16, TypeId, FileId, u64), usize> = FxMap::default();
        let mut seen_file: FxMap<(u16, TypeId, FileId), ()> = FxMap::default();
        let mut seen_worker: FxMap<(u16, TypeId), ()> = FxMap::default();
        let mut kept_worker: FxMap<(u16, TypeId), ()> = FxMap::default();
        let mut seen_any: FxMap<TypeId, ()> = FxMap::default();
        for &i in &events {
            let e = &self.profile.events[i];
            let About::Type(t) = e.about else { continue };
            let c = match e.memo {
                Memo::Hit if e.hit_nomatch => "hit: nothing found (then the synthesis)".to_string(),
                Memo::Hit => "hit: found".to_string(),
                Memo::Miss => {
                    let (file, h) = e.ctx.unwrap_or((FileId(u32::MAX), 0));
                    if let Some(&prev) = last_in_ctx.get(&(e.worker, t, file, h)) {
                        let p = &self.profile.events[prev];
                        if p.memo == Memo::Hit {
                            "miss: the previous lookup in its context was a hit".to_string()
                        } else {
                            format!("miss: repeats its context's previous lookup, {}", p.keep.name())
                        }
                    } else if seen_file.contains_key(&(e.worker, t, file)) {
                        let kept = if kept_worker.contains_key(&(e.worker, t)) { "a result kept" } else { "none kept" };
                        format!("miss: looked up before in another context of the file, {}", kept)
                    } else if seen_worker.contains_key(&(e.worker, t)) {
                        let kept = if kept_worker.contains_key(&(e.worker, t)) { "a result kept" } else { "none kept" };
                        format!("miss: looked up before in another file, {}", kept)
                    } else if seen_any.contains_key(&t) {
                        "miss: looked up before by another worker only".to_string()
                    } else {
                        "miss: the target's first memo lookup".to_string()
                    }
                }
                m => m.name().to_string(),
            };
            if let Some((file, h)) = e.ctx {
                last_in_ctx.insert((e.worker, t, file, h), i);
                seen_file.insert((e.worker, t, file), ());
                seen_worker.insert((e.worker, t), ());
                seen_any.insert(t, ());
                if matches!(e.keep, Keep::Kept | Keep::KeptNoMatch) || e.memo == Memo::Hit {
                    kept_worker.insert((e.worker, t), ());
                }
            }
            class[i] = c;
        }
        // The inclusive time of a class's searches, each counted once: those with no ancestor
        // of the same class.
        let outermost = |t: &Self, i: usize, key: &dyn Fn(usize) -> bool| -> bool {
            let mut p = t.profile.events[i].parent;
            while p != u32::MAX {
                if key(p as usize) {
                    return false;
                }
                p = t.profile.events[p as usize].parent;
            }
            true
        };
        let mut rows: Vec<(String, u32, u64, u64, [u64; N_STEPS])> = Vec::new();
        let mut index: FxMap<String, usize> = FxMap::default();
        for &i in &events {
            let (own, parts) = ev(self, i);
            let k = *index.entry(class[i].clone()).or_insert_with(|| {
                rows.push((class[i].clone(), 0, 0, 0, [0; N_STEPS]));
                rows.len() - 1
            });
            let once = outermost(self, i, &|p| class[p] == class[i]);
            let r = &mut rows[k];
            r.1 += 1;
            r.2 += own;
            if once {
                r.3 += self.profile.events[i].incl;
            }
            for (a, b) in r.4.iter_mut().zip(parts.iter()) {
                *a += b;
            }
        }
        rows.sort_by(|a, b| b.2.cmp(&a.2).then(a.0.cmp(&b.0)));
        let _ = writeln!(out, "  by what the memo did (count, self, inclusive with each search counted once; the self time by group of parts):");
        for (name, n, own, incl, parts) in &rows {
            let _ = writeln!(out, "    {}: {} ({} self, {:.1}% of the row, {} incl; {})", name, n, ms(*own), row_share(*own), ms(*incl), grouped(parts));
        }
        let _ = writeln!(out, "  by the result's keep, of the searches that ran where the memo applied (count, self, inclusive counted once):");
        for k in Keep::ALL {
            let (mut n, mut own, mut incl) = (0u32, 0u64, 0u64);
            for &i in &events {
                let e = &self.profile.events[i];
                if e.keep == k {
                    n += 1;
                    own += e.incl - e.child;
                    if outermost(self, i, &|p| self.profile.events[p].kind == Kind::Given && self.profile.events[p].keep == k) {
                        incl += e.incl;
                    }
                }
            }
            if n > 0 {
                let _ = writeln!(out, "    {}: {} ({} self, {} incl)", k.name(), n, ms(own), ms(incl));
            }
        }
        // The dimensions, each on its own.
        let mut dims: Vec<(String, u32, u64, u64)> = Vec::new();
        let mut dim = |t: &Self, name: &str, keep: &dyn Fn(&Event) -> bool| {
            let (mut n, mut own, mut incl) = (0u32, 0u64, 0u64);
            for &i in &events {
                let e = &t.profile.events[i];
                if keep(e) {
                    n += 1;
                    own += e.incl - e.child;
                    if outermost(t, i, &|p| keep(&t.profile.events[p])) {
                        incl += e.incl;
                    }
                }
            }
            dims.push((name.to_string(), n, own, incl));
        };
        dim(self, "top level", &|e| e.parent == u32::MAX);
        dim(self, "nested (under another given search)", &|e| e.parent != u32::MAX);
        dim(self, "target without open variables", &|e| e.ground);
        dim(self, "target with open variables", &|e| !e.ground);
        dim(self, "request for a by-name parameter", &|e| e.byname);
        dim(self, "request for a plain parameter", &|e| !e.byname);
        dim(self, "under an extension lookup", &|e| e.under_ext);
        for o in Origin::ALL {
            dim(self, &format!("origin: {}", o.name()), &|e| e.origin == o);
        }
        let _ = writeln!(out, "  by dimension, each on its own (count, self, inclusive counted once):");
        for (name, n, own, incl) in &dims {
            let _ = writeln!(out, "    {}: {} ({} self, {:.1}% of the row, {} incl)", name, n, ms(*own), row_share(*own), ms(*incl));
        }
        self.given_apart_tables(out, &events);
        let g = &self.profile.given.parts;
        let _ = writeln!(out, "  fit decisions the fit memo did not answer, by how they were decided and why (count, time):");
        for (k, how) in FIT_HOWS.iter().enumerate() {
            for why in FitWhy::ALL {
                let t = g.fit_why[k][why as usize];
                if t.n > 0 {
                    let _ = writeln!(out, "    {}, {}: {} ({})", how.name(), why.name(), t.n, ms(g.ns(t.ns)));
                }
            }
        }
        let _ = writeln!(out, "  candidates collected, by kind of level (levels walked, time):");
        for kind in LevelKind::ALL {
            let t = g.levels[kind as usize];
            if t.n > 0 {
                let _ = writeln!(out, "    {}: {} ({})", kind.name(), t.n, ms(g.ns(t.ns)));
            }
        }
        let _ = writeln!(out, "  lazy work entered from inside the given searches and extension lookups, by part of the type phase (entries, own time):");
        let mut lazy: Vec<(Phase, Tally)> = Phase::ALL.iter().map(|&p| (p, g.lazy_by_phase[p as usize])).filter(|(_, t)| t.n > 0).collect();
        lazy.sort_by(|a, b| b.1.ns.cmp(&a.1.ns));
        for (p, t) in lazy {
            let _ = writeln!(out, "    {}: {} ({})", p.name(), t.n, ms(g.ns(t.ns)));
        }
        // The hundred costliest targets, by their shown type (the searches of a type with open
        // variables each have a target of their own).
        let mut by_shown: FxMap<String, usize> = FxMap::default();
        let mut targets: Vec<(String, u32, u32, u32, u64, u64, [u32; 3], [u64; N_STEPS])> = Vec::new();
        let mut shown_of: Vec<u32> = vec![u32::MAX; self.profile.events.len()];
        for &i in &events {
            let About::Type(t) = self.profile.events[i].about else { continue };
            let shown = self.show(t);
            let k = *by_shown.entry(shown.clone()).or_insert_with(|| {
                targets.push((shown, 0, 0, 0, 0, 0, [0; 3], [0; N_STEPS]));
                targets.len() - 1
            });
            shown_of[i] = k as u32;
        }
        for &i in &events {
            let k = shown_of[i];
            if k == u32::MAX {
                continue;
            }
            let (own, parts) = ev(self, i);
            let e = &self.profile.events[i];
            let once = outermost(self, i, &|p| shown_of[p] == k);
            let r = &mut targets[k as usize];
            r.1 += 1;
            r.2 += (e.parent == u32::MAX) as u32;
            r.3 += (e.outcome == Outcome::Found) as u32;
            r.4 += own;
            if once {
                r.5 += e.incl;
            }
            let c = match e.memo {
                Memo::Hit => 0,
                Memo::Miss | Memo::Depth | Memo::Budget | Memo::ConversionGuard | Memo::Opened | Memo::ByName => 1,
                _ => 2,
            };
            r.6[c] += 1;
            for (a, b) in r.7.iter_mut().zip(parts.iter()) {
                *a += b;
            }
        }
        targets.sort_by(|a, b| b.4.cmp(&a.4).then(a.0.cmp(&b.0)));
        let _ = writeln!(out, "  the hundred costliest targets by self time (count, top level, found; self, inclusive counted once; hits, misses, not served; the self time by group of parts):");
        for (shown, n, tops, found, own, incl, classes, parts) in targets.iter().take(100) {
            let _ = writeln!(
                out,
                "    {} ({} x, {} top level, {} found; {} self, {} incl; {} hits, {} misses, {} not served; {})",
                shown, n, tops, found, ms(*own), ms(*incl), classes[0], classes[1], classes[2], grouped(parts)
            );
        }
    }

    /// The tables of the work around the given searches' events, printed whether or not any
    /// search reached its event: the extension lookups apart, with the given searches under
    /// them, and the searches that ended before their event.
    fn given_apart_tables(&mut self, out: &mut String, events: &[usize]) {
        use std::fmt::Write;
        // The extension lookups, apart: their collection and the given searches nested in them.
        let ext: Vec<usize> = (0..self.profile.events.len()).filter(|&i| self.profile.events[i].kind == Kind::Extension).collect();
        if !ext.is_empty() {
            let (mut own, mut collect, mut incl) = (0u64, 0u64, 0u64);
            for &i in &ext {
                let e = &self.profile.events[i];
                own += e.incl - e.child;
                collect += e.parts[Step::ExtCollect as usize];
                incl += e.incl;
            }
            let (mut gn, mut gown) = (0u32, 0u64);
            for &i in events {
                let e = &self.profile.events[i];
                if e.under_ext {
                    gn += 1;
                    gown += e.incl - e.child;
                }
            }
            let _ = writeln!(
                out,
                "  extension lookups in the implicit scope, apart: {} ({} self, of which the collection of givens {}; {} incl, nested lookups counted again), with {} given searches under them ({} self, in the row)",
                ext.len(),
                ms(own),
                ms(collect),
                ms(incl),
                gn,
                ms(gown)
            );
        }
        let g = &self.profile.given.parts;
        let _ = writeln!(out, "  searches ended before their event (count, time; inside a given search's own time, the rest outside the row):");
        for how in Early::ALL {
            let (all, inside) = g.early[how as usize];
            if all.n > 0 {
                let _ = writeln!(out, "    {}: {} ({}; inside {} ({}))", how.name(), all.n, ms(g.ns(all.ns)), inside.n, ms(g.ns(inside.ns)));
            }
        }
        let _ = writeln!(out, "    the checks of the searches that reached their event: {} ({}; inside {} ({}))", g.pre_event.0.n, ms(g.ns(g.pre_event.0.ns)), g.pre_event.1.n, ms(g.ns(g.pre_event.1.ns)));
    }

    /// The split of the type phase into its parts and the numbers on the bodies, after the
    /// kinds table.
    fn phase_section(&mut self, out: &mut String, type_ns: u64) {
        use std::fmt::Write;
        let rows = self.profile.phase_rows;
        if rows.iter().all(|r| r.count == 0) {
            return;
        }
        let share = |ns: u64| if type_ns == 0 { 0.0 } else { ns as f64 * 100.0 / type_ns as f64 };
        let before = self.profile.rows_before_reach;
        let _ = writeln!(out, "split of the type phase (self time of each part; lazy: entered from inside a body{}):", if before.is_some() { "; in reach: the part of the self time spent in the reach pass" } else { "" });
        let reach_column = if before.is_some() { format!(" {:>10}", "in reach") } else { String::new() };
        let _ = writeln!(out, "  {:<48} {:>8} {:>10} {:>7}{} {:>8} {:>10}", "part", "count", "self", "share", reach_column, "lazy", "lazy self");
        let mut attributed = 0u64;
        for p in Phase::ALL {
            let r = rows[p as usize];
            if r.count == 0 {
                continue;
            }
            attributed += r.self_ns;
            let lazy = if r.lazy_count > 0 { format!("{:>8} {:>10}", r.lazy_count, ms(r.lazy_self_ns)) } else { String::new() };
            let in_reach = match before {
                Some(b) => format!(" {:>10}", ms(r.self_ns - b[p as usize].self_ns)),
                None => String::new(),
            };
            let _ = writeln!(out, "  {:<48} {:>8} {:>10} {:>6.1}%{} {}", p.name(), r.count, ms(r.self_ns), share(r.self_ns), in_reach, lazy);
        }
        let rest = type_ns.saturating_sub(attributed);
        let _ = writeln!(out, "  {:<48} {:>8} {:>10} {:>6.1}%", "unattributed (setup, the walk over the files)", "", ms(rest), share(rest));
        // The split of the parallel typer: what stays on the main thread against what the
        // work items do, the lazy entries with the items and the merge's walk with neither.
        let (mut sequential, mut threads) = (0u64, 0u64);
        for p in Phase::ALL {
            let r = rows[p as usize];
            if p == Phase::Merge {
                continue;
            }
            if p.on_threads() {
                threads += r.self_ns - r.early_self_ns;
                sequential += r.early_self_ns;
            } else {
                sequential += r.self_ns - r.lazy_self_ns;
                threads += r.lazy_self_ns;
            }
        }
        let _ = writeln!(
            out,
            "  sequential {} ({:.1}%, the unattributed part included), on the threads {} ({:.1}%)",
            ms(sequential + rest),
            share(sequential + rest),
            ms(threads),
            share(threads)
        );
        let m = self.profile.merge;
        if m.records > 0 {
            let _ = writeln!(
                out,
                "merge: {} records walked; the chunk: {} symbols, {} classes, {} expressions, {} functions, {} class bodies, {} names; {} types made in the body phase, {} of them held by the chunk's records",
                m.records, m.syms, m.classes, m.exprs, m.funs, m.tclasses, m.names, m.types_made, m.types_held
            );
        }
        let p = &self.profile;
        let _ = writeln!(
            out,
            "signatures: {} reads (sig_of and sig_arc), {} Arc clones handed out by sig_arc, at most {} of them released before the next read",
            p.sig_reads, p.sig_clones, p.sig_dropped
        );
        let _ = writeln!(out, "threads: waited {} times on another thread's cell ({} ms), stolen {} signatures of files other threads took", p.waited, ms(p.wait_ns), p.stolen);
        let Some(stats) = self.body_stats() else { return };
        let _ = writeln!(
            out,
            "bodies: {} members typed: {} in the walk over the files, {} before it for the signature phase's tables, {} from inside another body for an inferred result type, {} from inside another body on demand, {} for the reach pass or a macro; {} of the program, {} of the std, {} of libraries ({} self)",
            stats.count, stats.walk, stats.early, stats.inferred_nested, stats.demand, stats.reach, stats.program, stats.std, stats.library, ms(stats.library_self_ns)
        );
        let _ = writeln!(
            out,
            "  time: {} inclusive ({:.1}% of the type phase), {} in the bodies themselves; per body median {}, p90 {}, p99 {}, largest {} ({})",
            ms(stats.incl_ns),
            share(stats.incl_ns),
            ms(stats.self_ns),
            ms(stats.p50),
            ms(stats.p90),
            ms(stats.p99),
            ms(stats.max_ns),
            stats.max_name
        );
        let chains: Vec<String> = stats.chains.iter().enumerate().skip(1).filter(|(_, &n)| n > 0).map(|(d, n)| format!("{} deep: {}", d, n)).collect();
        let _ = writeln!(
            out,
            "  inferred result types: {} definitions typed for one; {} of them read from another body ({} before their body was typed); {} of the {} bodies read one ({:.1}%), {} one of another file, {} had to type one first; chains {}",
            stats.inferred_defs,
            stats.inferred_read,
            stats.inferred_read_open,
            stats.readers,
            stats.count,
            stats.readers as f64 * 100.0 / stats.count.max(1) as f64,
            stats.readers_other_file,
            stats.blocked,
            if chains.is_empty() { "none".to_string() } else { chains.join(", ") }
        );
        let _ = writeln!(
            out,
            "  per file: {} files with bodies, largest {} ({}), median {}; the largest four hold {:.1}% of the bodies' time",
            stats.files.len(),
            ms(stats.file_max_ns),
            stats.file_max_name,
            ms(stats.file_p50),
            stats.top4_share * 100.0
        );
    }

    /// The numbers behind the bodies line of the report and the JSON.
    fn body_stats(&self) -> Option<BodyStats> {
        let bodies = &self.profile.bodies;
        if bodies.is_empty() {
            return None;
        }
        let count = |f: &dyn Fn(&BodyRow) -> bool| bodies.iter().filter(|b| f(b)).count() as u32;
        let mut top: Vec<u64> = bodies.iter().filter(|b| !b.nested).map(|b| b.incl_ns).collect();
        top.sort_unstable();
        let (max_ns, max_sym) = bodies.iter().filter(|b| !b.nested).map(|b| (b.incl_ns, b.sym)).max_by_key(|&(ns, _)| ns).unwrap();
        let mut chains = vec![0u32; 1 + bodies.iter().map(|b| b.chain as usize).max().unwrap_or(0)];
        for b in bodies.iter().filter(|b| b.inferred && b.nested) {
            chains[b.chain as usize] += 1;
        }
        let mut per_file: FxMap<FileId, (u32, u64)> = FxMap::default();
        for b in bodies.iter().filter(|b| !b.nested) {
            let e = per_file.entry(b.file).or_insert((0, 0));
            e.0 += 1;
            e.1 += b.incl_ns;
        }
        let mut files: Vec<(FileId, u32, u64)> = per_file.into_iter().map(|(f, (n, ns))| (f, n, ns)).collect();
        files.sort_by(|a, b| b.2.cmp(&a.2).then(a.0.cmp(&b.0)));
        let file_total: u64 = files.iter().map(|f| f.2).sum();
        let top4: u64 = files.iter().take(4).map(|f| f.2).sum();
        let mut file_sizes: Vec<u64> = files.iter().map(|f| f.2).collect();
        file_sizes.sort_unstable();
        let body_ns: Vec<u64> = bodies.iter().filter(|b| !b.nested).map(|b| b.incl_ns).collect();
        Some(BodyStats {
            count: bodies.len() as u32,
            walk: count(&|b| !b.nested && !b.reach && !b.early),
            early: count(&|b| b.early),
            inferred_nested: count(&|b| b.nested && b.inferred && !b.early),
            demand: count(&|b| b.nested && !b.inferred && !b.early),
            reach: count(&|b| b.reach),
            program: count(&|b| b.where_ == BodyWhere::Program),
            std: count(&|b| b.where_ == BodyWhere::Std),
            library: count(&|b| b.where_ == BodyWhere::Library),
            library_self_ns: bodies.iter().filter(|b| b.where_ == BodyWhere::Library).map(|b| b.self_ns).sum(),
            program_self_ns: bodies.iter().filter(|b| b.where_ == BodyWhere::Program).map(|b| b.self_ns).sum(),
            std_self_ns: bodies.iter().filter(|b| b.where_ == BodyWhere::Std).map(|b| b.self_ns).sum(),
            walk_self_ns: bodies.iter().filter(|b| !b.nested && !b.reach && !b.early).map(|b| b.self_ns).sum(),
            early_self_ns: bodies.iter().filter(|b| b.early).map(|b| b.self_ns).sum(),
            inferred_nested_self_ns: bodies.iter().filter(|b| b.nested && b.inferred && !b.early).map(|b| b.self_ns).sum(),
            demand_self_ns: bodies.iter().filter(|b| b.nested && !b.inferred && !b.early).map(|b| b.self_ns).sum(),
            reach_self_ns: bodies.iter().filter(|b| b.reach).map(|b| b.self_ns).sum(),
            incl_ns: top.iter().sum(),
            self_ns: bodies.iter().map(|b| b.self_ns).sum(),
            p50: percentile(&top, 0.5),
            p90: percentile(&top, 0.9),
            p99: percentile(&top, 0.99),
            max_ns,
            max_name: self.sym_path(max_sym),
            inferred_defs: count(&|b| b.inferred),
            inferred_read: self.profile.inferred_reads.len() as u32,
            inferred_read_open: self.profile.inferred_reads.values().filter(|r| r.1 > 0).count() as u32,
            readers: count(&|b| b.reads_inferred),
            readers_other_file: count(&|b| b.reads_other_file),
            blocked: count(&|b| b.blocked),
            chains,
            file_max_ns: files.first().map_or(0, |f| f.2),
            file_max_name: files.first().map_or(String::new(), |f| self.source(f.0).path.clone()),
            file_p50: percentile(&file_sizes, 0.5),
            top4_share: if file_total == 0 { 0.0 } else { top4 as f64 / file_total as f64 },
            files: files.into_iter().map(|(f, n, ns)| (self.source(f).path.clone(), n, ns)).collect(),
            body_ns,
        })
    }

    fn phase_json(&mut self, out: &mut String) {
        use crate::watch::json_string as js;
        out.push_str(",\"phases\":[");
        let mut first = true;
        for p in Phase::ALL {
            let r = self.profile.phase_rows[p as usize];
            if r.count == 0 {
                continue;
            }
            if !first {
                out.push(',');
            }
            first = false;
            out.push_str(&format!(
                "{{\"phase\":\"{}\",\"count\":{},\"self_ns\":{},\"lazy_count\":{},\"lazy_self_ns\":{},\"early_self_ns\":{},\"threads\":{}}}",
                p.short(),
                r.count,
                r.self_ns,
                r.lazy_count,
                r.lazy_self_ns,
                r.early_self_ns,
                p.on_threads()
            ));
        }
        out.push(']');
        let m = self.profile.merge;
        out.push_str(&format!(
            ",\"merge\":{{\"records\":{},\"syms\":{},\"classes\":{},\"exprs\":{},\"funs\":{},\"tclasses\":{},\"names\":{},\"types_made\":{},\"types_held\":{}}}",
            m.records, m.syms, m.classes, m.exprs, m.funs, m.tclasses, m.names, m.types_made, m.types_held
        ));
        out.push_str(&format!(",\"waited\":{},\"wait_ns\":{},\"stolen\":{}", self.profile.waited, self.profile.wait_ns, self.profile.stolen));
        let Some(s) = self.body_stats() else { return };
        out.push_str(&format!(
            ",\"bodies\":{{\"count\":{},\"walk\":{},\"early\":{},\"inferred_nested\":{},\"demand\":{},\"reach\":{},\"program\":{},\"std\":{},\"library\":{},\"incl_ns\":{},\"self_ns\":{},\"p50_ns\":{},\"p90_ns\":{},\"p99_ns\":{},\"max_ns\":{},\"max_name\":",
            s.count, s.walk, s.early, s.inferred_nested, s.demand, s.reach, s.program, s.std, s.library, s.incl_ns, s.self_ns, s.p50, s.p90, s.p99, s.max_ns
        ));
        js(&s.max_name, out);
        out.push_str(&format!(
            ",\"program_self_ns\":{},\"std_self_ns\":{},\"library_self_ns\":{},\"walk_self_ns\":{},\"early_self_ns\":{},\"inferred_nested_self_ns\":{},\"demand_self_ns\":{},\"reach_self_ns\":{}",
            s.program_self_ns, s.std_self_ns, s.library_self_ns, s.walk_self_ns, s.early_self_ns, s.inferred_nested_self_ns, s.demand_self_ns, s.reach_self_ns
        ));
        out.push_str(&format!(
            ",\"inferred_defs\":{},\"inferred_read\":{},\"inferred_read_open\":{},\"readers\":{},\"readers_other_file\":{},\"blocked\":{},\"chains\":[{}],\"per_file\":[",
            s.inferred_defs,
            s.inferred_read,
            s.inferred_read_open,
            s.readers,
            s.readers_other_file,
            s.blocked,
            s.chains.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(",")
        ));
        for (i, (path, n, ns)) in s.files.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push_str("{\"file\":");
            js(path, out);
            out.push_str(&format!(",\"count\":{},\"ns\":{}}}", n, ns));
        }
        out.push_str("],\"body_ns\":[");
        out.push_str(&s.body_ns.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(","));
        out.push_str("]}");
        let (types, lists) = self.types.len();
        out.push_str(&format!(
            ",\"arenas\":{{\"syms\":{},\"classes\":{},\"tparams\":{},\"aliases\":{},\"types\":{},\"type_lists\":{},\"tvars\":{},\"exprs\":{},\"funs\":{},\"tclasses\":{},\"names\":{}}}",
            self.syms.syms.len(),
            self.syms.classes.len(),
            self.syms.tparams.len(),
            self.syms.aliases.len(),
            types,
            lists,
            self.tvars.len(),
            self.prog.exprs.len(),
            self.prog.funs.len(),
            self.prog.classes.len(),
            self.interner.len()
        ));
    }

    fn row_kind(&self, key: &ProfKey) -> RowKind {
        let of_file = |t: &Self, f: FileId| if t.source(f).is_std { RowKind::Std } else { RowKind::Own };
        match key {
            ProfKey::Fun(s) => of_file(self, self.syms.sym(*s).file),
            ProfKey::Lambda(e) => self.first_span(*e).map_or(RowKind::Own, |(f, _)| of_file(self, f)),
            ProfKey::Ctor(c) => of_file(self, self.syms.class(*c).file),
            ProfKey::Builtin(name) => {
                if name.starts_with("scala.quoted.") {
                    RowKind::Reflect
                } else if name.starts_with("$quote") {
                    RowKind::Quote
                } else {
                    RowKind::Native
                }
            }
        }
    }

    /// The span of an expression, or of the first of its parts that has one: the body of a
    /// by-name argument or of a `{ case ... }` literal is the typer's, its cases are the source's.
    fn first_span(&self, e: crate::tir::TExprId) -> Option<(FileId, Span)> {
        use crate::tir::TExpr;
        let mut at = e;
        for _ in 0..8 {
            if let Some(found) = self.prog.span_of(at) {
                return Some(found);
            }
            at = match self.prog.expr(at) {
                TExpr::Block(_, res) => res,
                TExpr::Match(_, cases) => self.prog.cases[cases.start as usize].body,
                TExpr::If(c, _, _) => c,
                TExpr::CallMethod(r, _, _) => r,
                TExpr::CallStatic(_, args) | TExpr::CallClosure(_, args) if !args.is_empty() => self.prog.expr_lists[args.start as usize],
                _ => return None,
            };
        }
        None
    }

    /// The path of a function, an extension method with the simple name of its receiver's
    /// class before its own, as the interpreter keys its builtins.
    fn fun_path(&self, s: SymId) -> String {
        let info = self.syms.sym(s);
        if !info.is_extension {
            return self.sym_path(s);
        }
        let receiver = info.sig.as_ref().and_then(|sig| sig.clauses.iter().take(info.ext_clauses.max(1) as usize).find(|c| !c.is_using)).and_then(|c| c.params.first()).map(|p| p.ty);
        let class_name = match receiver.map(|t| self.types.get(t)) {
            Some(Type::Class(c, _)) => self.name_str(self.syms.class(c).name),
            _ => "Any".to_string(),
        };
        let path = self.sym_path(s);
        match path.rfind('.') {
            Some(i) => format!("{}.{}.{}", &path[..i], class_name, &path[i + 1..]),
            None => format!("{}.{}", class_name, path),
        }
    }

    fn row_name(&self, starts: &mut FxMap<FileId, Vec<u32>>, key: &ProfKey) -> String {
        match key {
            ProfKey::Fun(s) => self.fun_path(*s),
            ProfKey::Lambda(e) => match self.first_span(*e) {
                Some((f, span)) => {
                    let (line, _) = self.line_col(starts, f, span.start);
                    let path = &self.source(f).path;
                    format!("<lambda> {}:{}", path.rsplit(std::path::is_separator).next().unwrap_or(path), line)
                }
                None => "<lambda>".to_string(),
            },
            ProfKey::Ctor(c) => format!("new {}", self.class_path(*c)),
            ProfKey::Builtin(name) => name.to_string(),
        }
    }

    fn macro_rows_section(&mut self, out: &mut String, rows: &MacroRows, top: usize, indent: &str, share_of: u64) {
        use std::fmt::Write;
        let share = |ns: u64| if share_of == 0 { 0.0 } else { ns as f64 * 100.0 / share_of as f64 };
        let ratio = rows.ns_per_tick();
        let ns = |t: u64| (t as f64 * ratio) as u64;
        let mut by_kind = [0u64; 5];
        let mut steps_by_kind = [0u64; 5];
        let mut table: Vec<(&ProfKey, &ProfRow, RowKind)> = Vec::with_capacity(rows.rows.len());
        for (key, row) in &rows.rows {
            let kind = self.row_kind(key);
            let i = RowKind::ALL.iter().position(|&k| k == kind).unwrap();
            by_kind[i] += ns(row.self_ticks);
            steps_by_kind[i] += row.self_steps;
            table.push((key, row, kind));
        }
        let _ = write!(out, "{}interpreter self time by kind:", indent);
        for (i, kind) in RowKind::ALL.iter().enumerate() {
            if by_kind[i] > 0 {
                let _ = write!(out, " {} {} ({:.1}%, {} steps);", kind.name(), ms(by_kind[i]), share(by_kind[i]), steps_by_kind[i]);
            }
        }
        let _ = writeln!(out);
        let a = &rows.allocs;
        let per = |n: u64| if rows.count == 0 { 0 } else { n / rows.count as u64 };
        let _ = writeln!(
            out,
            "{}allocations: {} frames, {} objects, {} closures, {} strings ({} / {} / {} / {} per expansion)",
            indent,
            a.frames,
            a.objects,
            a.closures,
            a.strings,
            per(a.frames),
            per(a.objects),
            per(a.closures),
            per(a.strings)
        );
        let mut starts = FxMap::default();
        for (title, by_steps) in [("by self time", false), ("by self steps", true)] {
            if by_steps {
                table.sort_by(|a, b| b.1.self_steps.cmp(&a.1.self_steps).then(b.1.self_ticks.cmp(&a.1.self_ticks)));
            } else {
                table.sort_by(|a, b| b.1.self_ticks.cmp(&a.1.self_ticks).then(b.1.self_steps.cmp(&a.1.self_steps)));
            }
            let n = top.min(table.len());
            let _ = writeln!(out, "{}top {} {} ({:>9} {:>11} {:>9} {:>9}  {:<7} name):", indent, n, title, "self", "steps", "calls", "incl", "kind");
            for (key, row, kind) in table.iter().take(n) {
                let name = self.row_name(&mut starts, key);
                let _ = writeln!(
                    out,
                    "{}  {:>11} {:>11} {:>9} {:>11}  {:<7} {}",
                    indent,
                    ms(ns(row.self_ticks)),
                    row.self_steps,
                    row.count,
                    ms(ns(row.incl_ticks)),
                    kind.short(),
                    name
                );
            }
        }
    }

    /// The histogram of the macro expansions: where the interpreter's time and steps went, for
    /// all of them and for the top callees.
    fn macro_histogram(&mut self, out: &mut String, top: usize) {
        use std::fmt::Write;
        let all = std::mem::take(&mut self.profile.macro_all);
        if all.count == 0 {
            return;
        }
        let macro_ns: u64 = self.profile.events.iter().filter(|e| e.kind == Kind::Macro).map(|e| e.incl - e.child).sum();
        let _ = writeln!(
            out,
            "macro expansion histogram ({} expansions, {} self): typing the splices {}, interpreter runs {}, the rest {}",
            all.count,
            ms(macro_ns),
            ms(all.pre_ns),
            ms(all.run_ns),
            ms(macro_ns.saturating_sub(all.run_ns))
        );
        self.macro_rows_section(out, &all, top, "  ", all.run_ns);
        let mut callees: Vec<(SymId, MacroRows)> = std::mem::take(&mut self.profile.macro_by_callee).into_iter().collect();
        callees.sort_by(|a, b| b.1.run_ns.cmp(&a.1.run_ns));
        for (callee, rows) in callees.iter().take(3) {
            let name = self.sym_path(*callee);
            let _ = writeln!(out, "  {} ({} x): typing the splices {}, interpreter runs {} ({} per expansion)", name, rows.count, ms(rows.pre_ns), ms(rows.run_ns), ms(rows.run_ns / rows.count.max(1) as u64));
            self.macro_rows_section(out, rows, top / 2, "    ", rows.run_ns);
        }
        self.profile.macro_all = all;
        self.profile.macro_by_callee = callees.into_iter().collect();
    }

    /// The whole table as JSON: totals per kind, every site and every group.
    pub fn profile_json(&mut self, type_phase: Duration) -> String {
        use crate::watch::json_string as js;
        let mut out = String::new();
        let type_ns = type_phase.as_nanos() as u64;
        out.push_str(&format!("{{\"type_ns\":{},\"kinds\":[", type_ns));
        let mut first = true;
        for kind in Kind::ALL {
            let (mut count, mut self_ns, mut incl_ns) = (0u32, 0u64, 0u64);
            for e in &self.profile.events {
                if e.kind == kind {
                    count += 1;
                    self_ns += e.incl - e.child;
                    if e.nested == 0 {
                        incl_ns += e.incl;
                    }
                }
            }
            if count == 0 {
                continue;
            }
            if !first {
                out.push(',');
            }
            first = false;
            out.push_str(&format!("{{\"kind\":\"{}\",\"count\":{},\"self_ns\":{},\"incl_ns\":{}}}", kind.short(), count, self_ns, incl_ns));
        }
        out.push(']');
        self.phase_json(&mut out);
        out.push_str(",\"sites\":[");
        let mut rows = self.site_rows();
        rows.sort_by(|a, b| b.self_ns.cmp(&a.self_ns).then(a.file.cmp(&b.file)).then(a.pos.cmp(&b.pos)));
        let mut starts = FxMap::default();
        for (i, r) in rows.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            let (line, col) = if matches!(r.kind, Kind::Completion | Kind::LibraryBody) { (0, 0) } else { self.line_col(&mut starts, r.file, r.pos) };
            let about = self.about_text(r.about);
            let hint = self.hint(r.kind, r.about, r.found > 0);
            out.push_str("{\"file\":");
            js(&self.source(r.file).path, &mut out);
            out.push_str(&format!(",\"line\":{},\"col\":{},\"kind\":\"{}\",\"about\":", line, col, r.kind.short()));
            js(&about, &mut out);
            out.push_str(&format!(
                ",\"count\":{},\"top_level\":{},\"self_ns\":{},\"incl_ns\":{},\"tried\":{},\"applicable\":{},\"depth\":{},\"found\":{},\"hint\":",
                r.count, r.top_level, r.self_ns, r.incl_ns, r.tried, r.applicable, r.depth, r.found
            ));
            js(&hint, &mut out);
            out.push('}');
        }
        out.push_str("],\"groups\":[");
        first = true;
        for kind in Kind::ALL {
            let rows = self.group_rows(kind, |t, a| if kind == Kind::Given { t.given_class(a) } else { a });
            for r in rows {
                if !first {
                    out.push(',');
                }
                first = false;
                let about = self.about_text(r.about);
                out.push_str(&format!("{{\"kind\":\"{}\",\"about\":", kind.short()));
                js(&about, &mut out);
                out.push_str(&format!(
                    ",\"count\":{},\"self_ns\":{},\"incl_ns\":{},\"tried\":{},\"applicable\":{},\"depth\":{},\"found\":{}}}",
                    r.count, r.self_ns, r.incl_ns, r.tried, r.applicable, r.depth, r.found
                ));
            }
        }
        out.push_str("],\"macro_histogram\":[");
        let all = std::mem::take(&mut self.profile.macro_all);
        let mut starts = FxMap::default();
        first = true;
        for (key, r) in &all.rows {
            if !first {
                out.push(',');
            }
            first = false;
            let (name, kind) = (self.row_name(&mut starts, key), self.row_kind(key));
            let ratio = all.ns_per_tick();
            out.push_str("{\"name\":");
            js(&name, &mut out);
            out.push_str(&format!(
                ",\"kind\":\"{}\",\"count\":{},\"self_ns\":{},\"incl_ns\":{},\"self_steps\":{},\"incl_steps\":{}}}",
                kind.short(),
                r.count,
                (r.self_ticks as f64 * ratio) as u64,
                (r.incl_ticks as f64 * ratio) as u64,
                r.self_steps,
                r.incl_steps
            ));
        }
        self.profile.macro_all = all;
        out.push_str("]}\n");
        out
    }
}
