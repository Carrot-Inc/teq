use super::exports::ExportOwner;
use super::exports::TraitMemberSite;
use super::profile::{About, Early, Keep, Kind, LevelKind, Memo, Outcome, FitWhy, Step};
use super::{Frame, ImportTarget, ResolvedImport, Worker};
use crate::ast::ListRef;
use crate::intern::{FxMap, Name};
use crate::source::{FileId, Span};
use std::sync::Arc;
use crate::symbols::*;
use crate::tir::*;
use crate::types::*;

/// What a walk of a type's implicit scope has passed, so that a variable bounded through
/// itself or an F-bounded parameter (`T <: Ord[T]`) ends the walk.
#[derive(Default)]
struct ScopeSeen {
    vars: Vec<TVarId>,
    params: Vec<TParamId>,
    /// The abstract type members walked through their bounds (`type T >: Box[T]` names itself).
    members: Vec<TypeId>,
    /// The paths in the scope as references whose members hold givens (dotty's `addPath`): the
    /// prefixes of the inner classes the type names and the companions reached through them.
    paths: Vec<TypeId>,
    /// The inner classes the type names through a path, with that path: their companion is
    /// reached through it, not as an object of its own.
    through: Vec<(ClassId, TypeId)>,
}

const MAX_IMPLICIT_DEPTH: u32 = 32;
/// scalac's `-Ximplicit-search-limit`: candidates tried in one search before it is given up.
const IMPLICIT_SEARCH_LIMIT: u32 = 50000;

/// Givens of one package or class, keyed by the classes their type conforms to and by the names
/// of the extension methods they provide, so that lookups do not scan all of them.
#[derive(Default)]
pub struct GivenIndex {
    pub by_class: FxMap<ClassId, Vec<SymId>>,
    pub by_extension: FxMap<Name, Vec<SymId>>,
    pub unindexed: Vec<SymId>,
    all: Vec<SymId>,
    /// Whether any of them is a Scala 2 implicit, which a wildcard import brings in.
    pub has_scala2: bool,
    /// The implicit conversions among the scope's implicits, from source or from a library:
    /// `implicit def f(a: A): B`, which no search for an instance tries and is only here, and
    /// givens of type `Conversion[A, B]`, which are instances too.
    pub conversions: Vec<SymId>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Wanted {
    Class(ClassId),
    AnyClass,
    Extension(Name),
}

impl Wanted {
    fn key(self) -> (u8, u32) {
        match self {
            Wanted::Class(c) => (0, c.0),
            Wanted::AnyClass => (1, 0),
            Wanted::Extension(n) => (2, n.0),
        }
    }
}

// What the search keeps besides its results (`given_fast`, below), each per worker (a worker
// starts from a copy of the loader's and adds its own), discarded at the merge (`merge.rs`), and
// each keyed by every input it reads, so that an entry answers wherever its key does:
// - `given_fits`: a fit decision per candidate, the object it is reached through and a target
//   without open variables, for a member of an object or a package (`settled_owner`) or of a
//   class or trait reached through a static object of the implicit scope that fixes its site
//   (`module_site_fixed`); kept across a retype, the symbols surviving one. The fit memo's
//   regressions on record (`given_fits_site_*`, `given_fits_trait_receiver`,
//   `given_fits_abstract_member`, `given_fits_site_param_result`) each reach the member
//   lexically, inside an object that derives its trait or through an import, where the site
//   comes from the frames and imports and the key's scope is the same for every site; that
//   reach stays unsettled. Through the implicit scope the key holds the object, and a static
//   object's site (its export's route or itself) and substitution are the same wherever the
//   search runs (`given_fits_module_sites`).
// - `head_sigs`: a candidate's declared result as the class test and the head rejection read it
//   against a class (`HeadSig`), kept once its signature and classes are settled, cleared by a
//   retype.
// - `implicit_scope_givens`: the givens of a ground target's implicit scope per wanted class,
//   before the accessibility filter, with whether one is restricted and whether all are
//   accessible from anywhere; kept across a retype.
// - `package_levels`: a package clause's level per package, file (for the innermost clause, which
//   holds the file's imports) and wanted class, before the accessibility filter
//   (`package_level`); kept where every index it read is settled and no import reads a value,
//   neither read nor kept under a local import at depth 0, cleared by a retype.
// The accessibility filter and the restricted marking run per search over these.

/// A search result kept for the next search of the same target under the same lexical
/// context (`context_key`: the file with its language settings, the frames with their local
/// givens, the local imports, the alias given being defined, the class whose parent arguments
/// are typed, the transparent opaque types): the tree (none where nothing was found) and its
/// type, what every search of its tree wanted (for the by-name refusal), every given opened in
/// the tree (the failed candidates' included: a divergence in a failed branch decides the
/// outcome too, through a `NotGiven`), how deep the tree went below the search, and the
/// attempts it spent. `memo_keep` says when a result is kept and `memo_in_progress_fits` when
/// a search can use one.
pub struct GivenResult {
    expr: Option<TExprId>,
    ty: TypeId,
    consulted: Box<[Wanted]>,
    opened: Box<[SymId]>,
    depth: u32,
    /// Whether the search or a request of its tree was for a by-name parameter.
    byname: bool,
    /// The attempts of the search budget its tree spent, charged to a hit.
    tries: u32,
    /// The candidates the tree took, for the unused-import check (`Unused::given_winners`).
    winners: Box<[GivenRef]>,
}

/// What the searches under way did that a memoised result must not hide: reads of the state
/// of what is in progress (`InProgress`), candidates of an implicit scope whose visibility is
/// restricted (accessible from some scopes only); and the deepest search reached, for the
/// depth a replay would need.
#[derive(Default)]
pub struct GivenTree {
    /// Reads of in-progress state since the outermost search began (`read_in_progress`).
    pub in_progress: u32,
    pub restricted: u32,
    pub deepest: u32,
    /// Requests for a by-name parameter, the trees replayed from the memo counted.
    pub byname_requests: u32,
}

/// The state of what is in progress that a search may read beyond its lexical scope, one
/// variant per reader: the list. A search that reads one marks every search under way
/// (`Worker::read_in_progress`), and no result of a marked search is kept; a memoised result
/// is taken only where none of the readers could read differently than its search did
/// (`memo_in_progress_fits`). A reader added to the search goes here, with its mark, its
/// refusal on a hit, and a case where a program can show it.
///
/// - `Divergence`: the divergence check reads the stack of open givens (`diverges`). Mark:
///   a candidate diverged. Refusal: a given the kept tree opened is open above (the entry's
///   `opened`). Case `given_memo_failed_branch`.
/// - `RecursiveAnswer`: a request below a by-name parameter is answered by an open given's
///   instance (`recursive_ref`). Mark: such an answer. Refusal: an open given reachable through
///   a by-name parameter could answer a request of the kept tree (`knot_possible`, reading
///   every class part of the open target). Cases `given_memo_recursive_answer`,
///   `given_memo_byname_ancestor`, `given_memo_knot_intersection`, `given_memo_knot_refined`.
/// - `DepthLimit`: the depth limit cuts a search (`MAX_IMPLICIT_DEPTH`). Mark: a search cut.
///   Refusal: the kept tree would pass the limit here (the entry's `depth`). No program teq
///   accepts can show it.
/// - `Budget`: the search budget refuses a candidate (`IMPLICIT_SEARCH_LIMIT`), and a hit
///   spends no attempt. Mark: a candidate refused. Refusal: the attempts the kept search
///   spent exceed what is left, and a hit is charged them otherwise (the entry's `tries`; a
///   search whose own tree hit the memo was charged less than a cold run spends, so a
///   program at the limit can pass warm and fail cold). No program of a case's size can show
///   it.
/// - `ConversionGuard`: the recursion guard of the conversion methods (`function_targets`)
///   refuses a conversion for a function type under search through a conversion's own body.
///   Mark: the guard read while a function type is under search. Refusal: one is. Case
///   `given_memo_conversion_guard`.
/// - `InlineArgument`: a using parameter of an inline method under expansion stands for the
///   argument of that expansion (`inline_arg`). Mark: such a parameter instantiated. No
///   refusal: an expansion binds a proxy of its own, so its levels are its own. Case
///   `given_memo_inline_argument`.
/// - `InlineReceiver`: an inline expansion binds `this` to the call's receiver, which a
///   candidate reached through `this` reads, whether it wins or fails to fit under that
///   receiver (`open_given_sig`'s site). Mark: such a candidate opened while an expansion is
///   in progress. No refusal: a search that opened none reads no receiver. Case
///   `given_memo_inline_receiver`.
/// - `InlineExpansion`: an inline given expands while a candidate is instantiated, and the
///   expansion reads the site (a macro's position, the splice owner, a selection typed at
///   the site), so its success, its failure and its type are the site's, whether the
///   candidate wins or loses (a loser that fails at one site succeeds and wins at another).
///   Mark: an expansion in any attempt of the tree. No refusal: the tree is never kept. Cases
///   `given_memo_inline_site` (the winner), `given_memo_inline_loser` (a loser aborting by
///   position).
/// - `ViewLookup`: a conversion for a member selection or an expected type (`conversions.rs`)
///   is looked up while a search is under way, reading the lexical scope with no record in
///   the consulted levels; it happens inside an expansion. Mark: such a lookup. No refusal:
///   the tree is never kept.
/// - `ProvisionalBinding`: a candidate attempt under way has bound an inference variable
///   that a candidate's declared type mentions, or the type of the stable value an imported
///   candidate is reached through (`candidate_type_has_vars`, `receiver_type_has_vars`).
///   Mark: such a candidate met. Refusal: a search under a local given, or a local import
///   from a value, whose type has one takes nothing (`memo_applicable`). Cases
///   `given_memo_var_candidate`, `given_fits_var_candidate`; the imported receiver's shape
///   has no case, since a search may constrain an enclosing application's open variable
///   where scalac does not, cold as warm.
/// - `SignatureInProgress`: a candidate whose signature is being completed (its body is being
///   typed for its type). Mark: such a candidate met. No refusal: the search reports.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum InProgress {
    Divergence,
    RecursiveAnswer,
    DepthLimit,
    Budget,
    ConversionGuard,
    InlineArgument,
    InlineReceiver,
    InlineExpansion,
    ViewLookup,
    ProvisionalBinding,
    SignatureInProgress,
}

impl InProgress {
    pub const ALL: [InProgress; 11] = [InProgress::Divergence, InProgress::RecursiveAnswer, InProgress::DepthLimit, InProgress::Budget, InProgress::ConversionGuard, InProgress::InlineArgument, InProgress::InlineReceiver, InProgress::InlineExpansion, InProgress::ViewLookup, InProgress::ProvisionalBinding, InProgress::SignatureInProgress];

    pub fn name(self) -> &'static str {
        match self {
            InProgress::Divergence => "a candidate diverged",
            InProgress::RecursiveAnswer => "a request answered by an open instance",
            InProgress::DepthLimit => "the depth limit cut a search",
            InProgress::Budget => "the search budget refused a candidate",
            InProgress::ConversionGuard => "the conversion guard read under a function-type search",
            InProgress::InlineArgument => "an inline method's using parameter stood for its argument",
            InProgress::InlineReceiver => "a candidate was reached through an expansion's receiver",
            InProgress::InlineExpansion => "an inline given expanded in an attempt",
            InProgress::ViewLookup => "a conversion was looked up under a search",
            InProgress::ProvisionalBinding => "a candidate's type had a provisionally bound variable",
            InProgress::SignatureInProgress => "a candidate's signature was in progress",
        }
    }
}

/// The state of the searches under way as one of them starts, for what it may keep.
struct TreeMarks {
    consulted: usize,
    opened: usize,
    in_progress: u32,
    restricted: u32,
    deepest: u32,
    byname_requests: u32,
    budget: u32,
    failed: usize,
    diags: usize,
    winners: usize,
}

/// Whether a type argument depends on a prefix, which `head_rejects` leaves to the shape pass.
fn head_dependent(w: &Worker, t: TypeId) -> bool {
    matches!(w.types.get(t), Type::This(_) | Type::Term(_) | Type::Select(..) | Type::Member(..) | Type::AppMember(..) | Type::Decl(_) | Type::Nested(..))
}

/// A given and, when it came out of the index of a class, that class: an object through which
/// a given defined in a trait is reached.
pub type GivenRef = (SymId, GivenScope);

/// Where a given candidate is read: in lexical scope, on an object of the implicit scope that
/// inherits it, on the stable value an import brought it from, or on a path of the implicit
/// scope it is a member of (dotty's `TermRef(o, item)` of `addPath`, `summon[o.Item]`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GivenScope {
    Lexical,
    Module(ClassId),
    Value(super::ValueImport),
    Path(TypeId),
}

impl GivenScope {
    pub fn module(self) -> Option<ClassId> {
        match self {
            GivenScope::Module(m) => Some(m),
            _ => None,
        }
    }

    pub(super) fn memo_code(self) -> u64 {
        match self {
            GivenScope::Lexical => u64::MAX,
            GivenScope::Module(m) => m.0 as u64,
            GivenScope::Value(v) => 1 << 32 | v.0 as u64,
            GivenScope::Path(p) => 2 << 32 | p.0 as u64,
        }
    }

}

impl GivenIndex {
    pub(super) fn select(&self, wanted: Wanted, scope: GivenScope, out: &mut Vec<GivenRef>) {
        let list = match wanted {
            Wanted::Class(c) => self.by_class.get(&c),
            Wanted::Extension(n) => self.by_extension.get(&n),
            Wanted::AnyClass => Some(&self.all),
        };
        out.extend(list.into_iter().flatten().map(|&g| (g, scope)));
        if !matches!(wanted, Wanted::AnyClass) {
            out.extend(self.unindexed.iter().map(|&g| (g, scope)));
        }
    }

    fn select_scala2(&self, wanted: Wanted, syms: &Symbols, out: &mut Vec<GivenRef>) {
        let start = out.len();
        self.select(wanted, GivenScope::Lexical, out);
        let mut kept = start;
        for i in start..out.len() {
            if syms.is_scala2_implicit(out[i].0) {
                out[kept] = out[i];
                kept += 1;
            }
        }
        out.truncate(kept);
    }
}

struct Success {
    given: GivenRef,
    expr: TExprId,
    ty: TypeId,
    /// What this success wrote while another candidate is tried (`state.rs`).
    undone: Option<super::state::SetAside>,
    /// What the nested searches of this success took (`Unused::given_winners`).
    winners: Vec<GivenRef>,
    /// The object the candidate was read on, for the unused-import check.
    receiver: Option<ClassId>,
}

/// The givens of an implicit scope selected for a wanted class, before the accessibility filter,
/// with whether one has a restricted visibility (the search's mark) and whether all are
/// accessible from anywhere (`plainly_accessible`, no filter needed).
pub struct ScopeList {
    givens: Box<[GivenRef]>,
    restricted: bool,
    plain: bool,
}

/// A package level's candidates before the accessibility filter (`Worker::package_levels`):
/// the package's own, then those of each import statement of the file in the order of the
/// source, a context each (`LevelBuf`), and whether every imported one is accessible from
/// anywhere.
pub struct LevelList {
    givens: Box<[GivenRef]>,
    nodes: Box<[(u32, Prec)]>,
    renamed: Box<[(u32, Name)]>,
    plain: bool,
}

/// A context's binding precedence (`Typer.BindingPrec`): a scope's own definitions, an import
/// statement naming what it brings, or one with a wildcard selector, `*` or `given`
/// (`ImportInfo.isWildcardImport`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Prec {
    Wild,
    Named,
    Def,
}

impl Prec {
    /// `BindingPrec.beats`: a definition beats a context inside it, a named import a wildcard.
    fn beats(self, inner: Prec) -> bool {
        self == Prec::Def || self == Prec::Named && inner == Prec::Wild
    }
}

/// The candidates of one level of a search (`level_candidates`), its contexts outermost first
/// (`ContextualImplicits`), each the end of its candidates and its binding precedence, and the
/// name an import renames a candidate to (`implicitName`) where it is not the given's own.
#[derive(Default)]
struct LevelBuf {
    cands: Vec<GivenRef>,
    nodes: Vec<(u32, Prec)>,
    renamed: Vec<(u32, Name)>,
}

impl LevelBuf {
    fn clear(&mut self) {
        self.cands.clear();
        self.nodes.clear();
        self.renamed.clear();
    }

    /// Ends the context whose candidates were pushed since the last one.
    fn close(&mut self, prec: Prec) {
        self.nodes.push((self.cands.len() as u32, prec));
    }
}

pub(super) enum Pick {
    NoMatch,
    Found(TExprId, TypeId),
    Ambiguous,
}

/// The scopes a search walks: the levels among the enclosing frames, the package clauses,
/// `scala.Predef` of a classpath and the implicit scope of the target, which is level 0.
struct Levels {
    frames: FrameLevels,
    packages: Arc<[PkgId]>,
    predef: Option<ClassId>,
    /// The alias given whose right-hand side is being typed, left out above level 0.
    defining: Option<SymId>,
}

/// The levels among the enclosing frames, outermost first (`ContextualImplicits.level`).
enum FrameLevels {
    /// No import inside a frame: each frame a level, its own givens its one context.
    PerFrame(usize),
    /// Each level's contexts, outermost first, `nodes` up to its end in `ends`: a frame's own
    /// givens open a level, an import statement of the same owner as the context outside it
    /// stands at that context's level, one of another owner opens a level. A statement's
    /// selectors are `start..end` of `selectors`, indices in `Env::imports`.
    Composed { nodes: Vec<FrameNode>, ends: Vec<u32>, selectors: Vec<u32> },
}

#[derive(Clone, Copy)]
enum FrameNode {
    Own(u32),
    Stmt(u32, u32, Prec),
}

impl Levels {
    fn n_outer(&self) -> usize {
        1 + self.predef.is_some() as usize
    }

    fn n_frames(&self) -> usize {
        match &self.frames {
            FrameLevels::PerFrame(n) => *n,
            FrameLevels::Composed { ends, .. } => ends.len(),
        }
    }

    fn count(&self) -> usize {
        self.n_frames() + self.packages.len() + self.n_outer()
    }
}

/// A parameterized given whose using clauses are being resolved, with the type it was asked
/// for measured as scalac does for its divergence check: the number of applied types in it,
/// the classes it mentions and the type itself with its open variables blanked out.
pub struct OpenGiven {
    pub given: SymId,
    target: TypeId,
    size: u32,
    classes: u64,
    wild: TypeId,
    /// Whether the request was for a by-name using parameter.
    byname: bool,
    /// The lazy local standing for this instance where a nested request referred back to it.
    rec: Option<SymId>,
}

/// A candidate's declared result as the class test (`may_provide`) and `head_rejects` read it
/// against a target's class, kept per candidate and class (`Worker::head_sigs`). It holds what
/// `class_of` and `dealias` leave as it is without a transparent opaque type or GADT bounds, so
/// it is the same in every context; a result of any other shape is read live.
pub struct HeadSig {
    /// Whether a part's class derives from the class; none where the test is read live.
    provides: Option<bool>,
    head: HeadForm,
}

/// Per part of an intersection, the arguments of its base type at the class, or none where
/// the part does not provide the class.
pub enum HeadForm {
    Live,
    /// A part of another shape: the rejection never holds.
    Never,
    Parts { inter: bool, parts: Box<[Option<Box<[HeadArg]>>]> },
}

#[derive(Clone, Copy)]
pub enum HeadArg {
    /// An argument that depends on a prefix, read through it by the shape pass.
    Dependent,
    /// An argument without a class of its own (a type parameter).
    Open,
    /// A class, with what the comparison asks of it: `special_head`, `tuple_like`,
    /// `nominal_class`.
    Class { c: ClassId, special: bool, tuple: bool, nominal: bool },
}

/// The target of a level's fit decisions as `head_rejects` reads it: its class and arguments,
/// each argument read on first use, and the variances of the class's parameters, read once
/// one is needed.
struct TargetHeads {
    class: Option<(ClassId, TList)>,
    args: Vec<Option<TargetArg>>,
    variances: Option<Box<[i8]>>,
    /// The kept form the class test read last, with its candidate.
    last: Option<(SymId, Arc<HeadSig>)>,
}

#[derive(Clone, Copy)]
enum TargetArg {
    /// An argument that depends on a prefix, or a wildcard.
    Skip,
    Var(TVarId),
    Class { c: ClassId, special: bool, tuple: bool },
    Other,
}

/// One candidate tried against a target.
enum Attempt {
    /// Its type does not fit: not a candidate for this target.
    Mismatch,
    /// Its type fits but a using parameter has no given.
    Incomplete,
    /// Its type fits but a using parameter is ambiguous, which the search only recovers
    /// from with a strictly better candidate.
    Ambiguous,
    Ok(TExprId, TypeId),
}

impl<'a> Worker<'a> {
    /// `TEQ_GIVEN_TRACE`: the searches that stop at a limit or a failed inline candidate, on
    /// stderr.
    fn given_trace(&self) -> bool {
        static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        *ON.get_or_init(|| std::env::var_os("TEQ_GIVEN_TRACE").is_some())
    }

    pub(super) fn given_index(&mut self, p: PkgId) -> Arc<GivenIndex> {
        if let Some(idx) = self.given_indexes.get(&p) {
            return idx.clone();
        }
        let prof = self.phase(super::profile::Phase::GivenIndex);
        let index = self.build_given_index(p);
        self.phase_end(prof);
        index
    }

    fn build_given_index(&mut self, p: PkgId) -> Arc<GivenIndex> {
        self.demand_std_givens(p);
        if self.sees_classpath() {
            self.enter_pkg_objects(p);
        }
        let mut givens = self.syms.pkg(p).givens.clone();
        if let Some(exports) = self.pkg_exports_of(p) {
            givens.extend(exports.givens.iter().copied());
        }
        if let Some(obj) = self.syms.pkg(p).package_object {
            self.complete_class(obj);
            givens.extend(self.syms.class(obj).givens.iter().copied());
            if let Some(exports) = self.exports_of(obj) {
                givens.extend(exports.givens.iter().copied());
            }
        }
        let rc = Arc::new(self.index_givens(givens));
        // An index built over a provisional export table serves the search at hand only.
        let settled = self.exports_settled(ExportOwner::Pkg(p))
            && self.syms.pkg(p).package_object.map_or(true, |obj| self.exports_settled(ExportOwner::Class(obj)));
        if settled {
            self.given_indexes.insert(p, rc.clone());
        }
        rc
    }

    /// The givens that the class `c` defines, exports or, being an object, inherits.
    /// The object a given's tree reads it on (`Field(Module(B), n)`, `B.n(...)`), if it reads it
    /// on one.
    fn given_receiver(&self, e: TExprId, g: SymId) -> Option<ClassId> {
        match self.prog.expr(e) {
            TExpr::Field(r, s) | TExpr::CallMethod(r, s, _) if s == g => match self.prog.expr(r) {
                TExpr::Module(m) => Some(m),
                _ => None,
            },
            TExpr::Block(_, last) => self.given_receiver(last, g),
            _ => None,
        }
    }

    pub(super) fn class_given_index(&mut self, c: ClassId) -> Option<Arc<GivenIndex>> {
        if let Some(idx) = self.class_given_indexes.get(&c) {
            return idx.clone();
        }
        if self.class_without_givens(c) {
            return None;
        }
        let prof = self.phase(super::profile::Phase::GivenIndex);
        let index = self.build_class_given_index(c);
        self.phase_end(prof);
        index
    }

    /// Whether `c` has no given in reach, so that its index would be empty: none of its own,
    /// none exported and, for a class or trait without a self type, none in its ancestors.
    /// Most classes have none; the answer is read off the completed hierarchy and kept
    /// nowhere, since every read gives the same. A class not yet complete goes to the build,
    /// which completes it (the evidence of its context bounds comes with completion).
    fn class_without_givens(&mut self, c: ClassId) -> bool {
        let Some(info) = self.syms.class_done(c) else { return false };
        if !info.givens.is_empty() {
            return false;
        }
        if self.exports_of(c).map_or(false, |e| !e.givens.is_empty()) {
            return false;
        }
        let info = self.syms.class(c);
        if info.kind == ClassKind::Object && !self.is_product_class(c) {
            return true;
        }
        // A base not yet complete (a library trait) has not entered its givens.
        info.declared_self.is_none() && info.base_types.iter().skip(1).all(|&(b, _)| self.syms.class_done(b).map_or(false, |b| b.givens.is_empty()))
    }

    fn build_class_given_index(&mut self, c: ClassId) -> Option<Arc<GivenIndex>> {
        self.complete_class(c);
        let mut givens = self.syms.class(c).givens.clone();
        if let Some(exports) = self.exports_of(c) {
            givens.extend(exports.givens.iter().copied());
        }
        // What a class inherits is in scope in its body; an object's table lists it already,
        // but for another module's object, read from its pickle with its own members alone.
        // So is what its self type declares (`_: FocusBase =>` with `given Quotes`).
        if self.syms.class(c).kind != ClassKind::Object || self.is_product_class(c) {
            let mut bases: Vec<ClassId> = self.syms.class(c).base_types.iter().skip(1).map(|&(b, _)| b).collect();
            for k in self.self_type_classes(c) {
                self.complete_class(k);
                for &(b, _) in &self.syms.class(k).base_types {
                    if b != c && !bases.contains(&b) {
                        bases.push(b);
                    }
                }
            }
            for b in bases {
                self.complete_class(b);
                let inherited: Vec<SymId> =
                    self.syms.class(b).givens.iter().copied().filter(|&g| self.syms.sym(g).mods & crate::ast::mods::PRIVATE == 0).collect();
                givens.extend(inherited);
            }
        }
        let index = if givens.is_empty() { None } else { Some(Arc::new(self.index_givens(givens))) };
        // Completing a class adds the evidence of its context bounds; an index built over a
        // provisional export table serves the search at hand only.
        if self.syms.class(c).state() == Completion::Done && self.exports_settled(ExportOwner::Class(c)) {
            self.class_given_indexes.insert(c, index.clone());
        }
        index
    }

    pub(super) fn given_index_of(&mut self, p: PkgId) -> Arc<GivenIndex> {
        self.given_index(p)
    }

    pub(super) fn class_given_index_of(&mut self, c: ClassId) -> Option<Arc<GivenIndex>> {
        self.class_given_index(c)
    }

    fn index_givens(&mut self, mut givens: Vec<SymId>) -> GivenIndex {
        givens.sort();
        givens.dedup();
        let mut index = GivenIndex::default();
        // A conversion method is also an implicit value of the function type it converts along,
        // at its own scope, as scalac eta-expands it for an `implicit f: A => B`.
        let function1 = self.function_class(1);
        givens.retain(|&g| {
            // A jar's conversion is filed without its signature, which the search reads only
            // for a function type's candidates.
            if self.loaded.as_ref().map_or(false, |l| l.is_conversion(g)) || self.is_conversion_def(g) {
                index.conversions.push(g);
                index.by_class.entry(function1).or_default().push(g);
                return false;
            }
            true
        });
        for &g in &givens {
            let ret = self.sig_of(g).ret;
            if self.is_conversion_given(g) {
                index.conversions.push(g);
            }
            // A given of an intersection type (`Order[A] & Hash[A] & Show[A]`) provides every
            // part.
            let mut parts = Vec::new();
            let mut stack = vec![self.deref(ret)];
            while let Some(t) = stack.pop() {
                match self.types.get(t) {
                    Type::Inter(a, b) => {
                        stack.push(b);
                        stack.push(a);
                    }
                    _ => parts.extend(self.class_of(t)),
                }
            }
            if parts.is_empty() {
                index.unindexed.push(g);
                continue;
            }
            let mut bases: Vec<ClassId> = Vec::new();
            for c in parts {
                // A Java class nothing has read yet has no base types to index by.
                if !self.is_java_placeholder(c) {
                    self.complete_class(c);
                }
                for &(b, _) in &self.syms.class(c).base_types {
                    if !bases.contains(&b) {
                        bases.push(b);
                    }
                }
            }
            for b in bases {
                index.by_class.entry(b).or_default().push(g);
                for &ext in &self.syms.class(b).extensions {
                    let list = index.by_extension.entry(self.syms.sym(ext).name).or_default();
                    if !list.contains(&g) {
                        list.push(g);
                    }
                }
            }
        }
        index.has_scala2 = givens.iter().chain(&index.conversions).any(|&g| self.syms.is_scala2_implicit(g));
        index.all = givens;
        index
    }

    /// The levels among the enclosing frames (`ContextualImplicits.level`, `Contexts.implicits`),
    /// outermost first. A frame with givens (a block's or parameter list's, a class's in reach) is
    /// a context of its own scope and opens a level; each import statement inside a frame that
    /// brings an implicit is a context, standing at the level of the context outside it where
    /// both have one owner and opening a level where not. An import that brings none is no
    /// context. Without an import inside a frame, each frame is a level.
    fn frame_levels(&mut self) -> FrameLevels {
        let n = self.env.frames.len();
        // The imports go outermost scope first, so the last is the deepest.
        debug_assert!(self.env.imports.windows(2).all(|w| w[0].depth <= w[1].depth), "imports go outermost scope first");
        if self.env.imports.last().map_or(true, |imp| imp.depth == 0) {
            return FrameLevels::PerFrame(n);
        }
        self.composed_frame_levels(n)
    }

    #[cold]
    #[inline(never)]
    fn composed_frame_levels(&mut self, n: usize) -> FrameLevels {
        // A scope's imports by statement, in the order of the source.
        let mut order: Vec<u32> = (0..self.env.imports.len() as u32).filter(|&i| self.env.imports[i as usize].depth > 0).collect();
        order.sort_by_key(|&i| {
            let imp = &self.env.imports[i as usize];
            (imp.depth, imp.stmt)
        });
        let (mut nodes, mut ends, mut selectors) = (Vec::new(), Vec::new(), Vec::new());
        // The owner of the last context: a class's own, or the owner depth of a frame's scope.
        let mut last: Option<(bool, u32)> = None;
        let mut k = 0;
        for f in 0..n {
            let owner = match &self.env.frames[f] {
                Frame::Class(_) => (true, f as u32),
                Frame::Locals { owner, .. } => (false, *owner),
            };
            if self.frame_has_givens(f) {
                if !nodes.is_empty() {
                    ends.push(nodes.len() as u32);
                }
                nodes.push(FrameNode::Own(f as u32));
                last = Some(owner);
            }
            let depth = f as u32 + 1;
            while k < order.len() && self.env.imports[order[k] as usize].depth == depth {
                let stmt = self.env.imports[order[k] as usize].stmt;
                let start = selectors.len();
                let (mut prec, mut brings) = (Prec::Named, false);
                while k < order.len() && self.env.imports[order[k] as usize].depth == depth && self.env.imports[order[k] as usize].stmt == stmt {
                    let imp = self.env.imports[order[k] as usize];
                    if imp.name.is_none() {
                        prec = Prec::Wild;
                    }
                    brings = brings || self.import_brings_implicits(imp);
                    selectors.push(order[k]);
                    k += 1;
                }
                if !brings {
                    selectors.truncate(start);
                    continue;
                }
                if last != Some(owner) && !nodes.is_empty() {
                    ends.push(nodes.len() as u32);
                }
                nodes.push(FrameNode::Stmt(start as u32, selectors.len() as u32, prec));
                last = Some(owner);
            }
        }
        if !nodes.is_empty() {
            ends.push(nodes.len() as u32);
        }
        FrameLevels::Composed { nodes, ends, selectors }
    }

    /// Whether frame `frame` is a context of the search (`Contexts.implicits`: its scope's or its
    /// class's implicits are not empty): a scope with givens, a class with a given in reach, the
    /// class whose deferred given's implementation is typed, with its using parameters.
    fn frame_has_givens(&mut self, frame: usize) -> bool {
        match &self.env.frames[frame] {
            Frame::Locals { givens, .. } => !givens.is_empty(),
            Frame::Class(c) if self.parent_args_of == Some(*c) => self.deferred_impl_of == Some(*c),
            Frame::Class(c) => {
                let c = *c;
                self.class_given_index(c).is_some()
            }
        }
    }

    /// Whether an import brings an implicit (`ImportInfo.importedImplicits` not empty), which
    /// makes it a context: a `given` selector any given of its prefix, `*` a Scala 2 implicit, a
    /// name the given or implicit it names.
    fn import_brings_implicits(&mut self, imp: ResolvedImport) -> bool {
        // A `given T` selector's bound and the clause's exclusions first (`importedImplicits`):
        // an import they leave nothing is no context, whatever its prefix holds.
        if !imp.hidden.is_empty() || imp.bound.is_some() {
            let mut found = Vec::new();
            self.import_givens_unfiltered(imp, Wanted::AnyClass, &mut found);
            let hidden: Vec<Name> = self.import_hidden.as_slice()[imp.hidden.range()].to_vec();
            return found.into_iter().any(|given| !hidden.contains(&self.syms.sym(given.0).name) && imp.bound.map_or(true, |b| self.matches_import_bound(self.imported_given(imp, given), b)));
        }
        let any = |i: &GivenIndex| !i.all.is_empty() || !i.conversions.is_empty();
        match imp.target {
            ImportTarget::PkgGivens(p) => any(&self.given_index(p)),
            ImportTarget::ClassGivens(c) => self.class_given_index(c).is_some_and(|i| any(&i)),
            ImportTarget::PkgAll(p) => self.given_index(p).has_scala2,
            ImportTarget::ClassAll(c) => self.class_given_index(c).is_some_and(|i| i.has_scala2),
            ImportTarget::UnimportPredef | ImportTarget::Unresolved => false,
            _ => {
                let mut found = Vec::new();
                self.import_givens_unfiltered(imp, Wanted::AnyClass, &mut found);
                !found.is_empty()
            }
        }
    }

    /// The givens of one lexical scope's own context: a block's or parameter list's, or an
    /// enclosing class's in reach.
    fn frame_own_givens(&mut self, frame: usize, wanted: Wanted, out: &mut Vec<GivenRef>) {
        match &self.env.frames[frame] {
            Frame::Locals { givens, .. } if !givens.iter().any(|&g| self.syms.is_scala2_implicit(g)) => {
                out.extend(givens.iter().rev().map(|&g| (g, GivenScope::Lexical)))
            }
            // A local implicit def with a plain parameter is a conversion, no given.
            Frame::Locals { givens, .. } => {
                let mut locals: Vec<SymId> = givens.iter().rev().copied().collect();
                locals.retain(|&g| !self.is_conversion_def(g));
                out.extend(locals.into_iter().map(|g| (g, GivenScope::Lexical)))
            }
            // The arguments of a parent constructor see the constructor parameters alone, and
            // the implementation of a deferred given the class's using parameters.
            Frame::Class(c) if self.parent_args_of == Some(*c) => {
                if self.deferred_impl_of == Some(*c) {
                    // Its `using` parameters, flagged `Given` as dotty's `implementDeferredGivens`
                    // takes them, the evidence of its context bounds among them: an old-style
                    // `implicit` clause is not.
                    let info = self.syms.class(*c);
                    out.extend(info.ctor.iter().filter(|cl| cl.is_using && !cl.is_implicit).flat_map(|cl| cl.params.iter().rev()).map(|p| (p.sym, GivenScope::Lexical)));
                }
            }
            Frame::Class(c) => {
                let c = *c;
                let start = out.len();
                if let Some(index) = self.class_given_index(c) {
                    index.select(wanted, GivenScope::Lexical, out);
                }
                // An inherited given the class overrides with a member that is no given is no
                // implicit member of its `this` (dotty's `implicitMembers` reads the member the
                // class has under the name: `val Underlying: Type[A]` over an `implicit val`).
                let mut k = start;
                while k < out.len() {
                    let g = out[k].0;
                    let overridden = self.syms.sym(g).owner != Owner::Class(c)
                        && self.syms.class(c).members.get(&self.syms.sym(g).name).is_some_and(|&m| {
                            let info = self.syms.sym(m);
                            m != g && info.kind != SymKind::Given && info.mods & (crate::ast::mods::GIVEN | crate::ast::mods::IMPLICIT) == 0
                        });
                    if overridden {
                        out.remove(k);
                    } else {
                        k += 1;
                    }
                }
            }
        }
    }

    /// The givens an import statement brings (`importedImplicits`) into a level, as a context of
    /// its own: each once (`import P.{*, given}`), the inaccessible left out, with the name a
    /// selector renames one to (`implicitName`). A given another context of the level brings too
    /// stays: which of the two the level keeps is the composition's (`compose_level`).
    /// Unfiltered for a package level's kept list, whose accessibility each search filters
    /// (`package_level`).
    fn stmt_givens(&mut self, stmt: &[ResolvedImport], wanted: Wanted, buf: &mut LevelBuf, filtered: bool) {
        let start = buf.cands.len();
        let mut found = Vec::new();
        for &imp in stmt {
            found.clear();
            self.import_givens(imp, wanted, &mut found);
            for &(g, via) in &found {
                if (!filtered || self.is_given_accessible(g)) && !buf.cands[start..].iter().any(|&(known, _)| known == g) {
                    if let Some(alias) = imp.name.filter(|&n| n != self.syms.sym(g).name) {
                        buf.renamed.push((buf.cands.len() as u32, alias));
                    }
                    buf.cands.push((g, via));
                }
            }
        }
    }

    /// The givens an import from a value brings: the value's class's, through its given selector,
    /// the one named, or its Scala 2 implicits through its wildcard; each read on the value.
    #[cold]
    #[inline(never)]
    fn value_import_givens(&mut self, imp: ResolvedImport, wanted: Wanted, out: &mut Vec<GivenRef>) {
        match imp.target {
            ImportTarget::ValueGivens(v) => {
                if let Some(index) = self.import_value_class(v).and_then(|c| self.class_given_index(c)) {
                    let from = out.len();
                    index.select(wanted, GivenScope::Lexical, out);
                    for given in &mut out[from..] {
                        given.1 = GivenScope::Value(v);
                    }
                }
            }
            ImportTarget::ValueMember(v, n) => {
                let ty = self.import_value_ret(v);
                let ty = self.zonk(ty);
                if let Some((s, _)) = self.find_member(ty, n) {
                    if self.syms.is_given(s) {
                        out.push((s, GivenScope::Value(v)));
                    }
                }
            }
            // A wildcard brings the value's Scala 2 implicits in, as an object's.
            ImportTarget::ValueAll(v) => {
                if let Some(index) = self.import_value_class(v).and_then(|c| self.class_given_index(c)) {
                    if index.has_scala2 {
                        let from = out.len();
                        index.select_scala2(wanted, &self.syms, out);
                        for given in &mut out[from..] {
                            given.1 = GivenScope::Value(v);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    pub(super) fn import_givens(&mut self, imp: ResolvedImport, wanted: Wanted, out: &mut Vec<GivenRef>) {
        let from = out.len();
        self.import_givens_unfiltered(imp, wanted, out);
        if !imp.hidden.is_empty() || imp.bound.is_some() {
            self.filter_wildcard_imported(imp, from, out);
        }
    }

    /// What an import brings, `out[from..]`, as `ImportInfo.importedImplicits` has it: through
    /// a wildcard no name another selector of its clause renames or hides (`excluded`), through
    /// a `given T` only the givens whose type conforms to `T` (`givenBound`, `matchesImportBound`),
    /// and through a name renamed beside a wildcard nothing (its bound `Nothing`).
    #[cold]
    #[inline(never)]
    fn filter_wildcard_imported(&mut self, imp: ResolvedImport, from: usize, out: &mut Vec<GivenRef>) {
        let hidden: Vec<Name> = self.import_hidden.as_slice()[imp.hidden.range()].to_vec();
        let mut i = from;
        while i < out.len() {
            let given = out[i];
            let keep = !hidden.contains(&self.syms.sym(given.0).name) && imp.bound.map_or(true, |b| self.matches_import_bound(self.imported_given(imp, given), b));
            if keep {
                i += 1;
            } else {
                out.remove(i);
            }
        }
    }

    /// The given `g` as the import `imp` reaches it: a member of the object it imports from is
    /// seen from that object (`Strings.box` of a `trait Lib[A]` that `Strings` extends as
    /// `Lib[String]` is a `Box[String]`), as scalac's import reads its members from the
    /// qualifier's type.
    pub(super) fn imported_given(&self, imp: ResolvedImport, g: GivenRef) -> GivenRef {
        match imp.target {
            ImportTarget::ClassGivens(c) | ImportTarget::ClassMember(c, _) | ImportTarget::ClassAll(c) if self.syms.class(c).kind == ClassKind::Object => (g.0, GivenScope::Module(c)),
            ImportTarget::ValueGivens(v) | ImportTarget::ValueMember(v, _) | ImportTarget::ValueAll(v) => (g.0, GivenScope::Value(v)),
            _ => g,
        }
    }

    /// Whether the given matches an import's bound (dotty's `Denotations.matchesImportBound`):
    /// `NoViewsAllowed.normalizedCompatible` of its signature as it is reached (`open_given_sig`:
    /// the trait's parameters as the object instantiates them, its own fresh within their
    /// bounds), without keeping a constraint. Its type is normalized as `ProtoTypes.normalize`
    /// does: a using clause gives way to its result, another clause makes a function type of its
    /// parameters to the rest.
    pub(super) fn matches_import_bound(&mut self, given: GivenRef, bound: TypeId) -> bool {
        if bound == ANY || bound == NOTHING {
            return bound == ANY;
        }
        let mark = self.attempt();
        let fits = match self.open_given_sig(given) {
            Some((_, subst, sig)) => {
                let mut normalized = self.types.subst(sig.ret, &subst);
                for clause in sig.clauses.iter().rev() {
                    if clause.is_using || clause.is_implicit || clause.params.is_empty() {
                        continue;
                    }
                    let mut args: Vec<TypeId> = clause.params.iter().map(|p| self.types.subst(p.ty, &subst)).collect();
                    args.push(normalized);
                    let f = self.function_class(clause.params.len());
                    normalized = self.types.class(f, &args);
                }
                self.is_sub(normalized, bound)
            }
            None => false,
        };
        self.retract(mark);
        fits
    }

    fn import_givens_unfiltered(&mut self, imp: ResolvedImport, wanted: Wanted, out: &mut Vec<GivenRef>) {
        match imp.target {
            ImportTarget::ClassGivens(c) => {
                if let Some(index) = self.class_given_index(c) {
                    index.select(wanted, GivenScope::Lexical, out);
                }
            }
            ImportTarget::PkgGivens(p) => self.given_index(p).select(wanted, GivenScope::Lexical, out),
            ImportTarget::ClassMember(c, n) => {
                if let Some(s) = self.module_term(c, n).and_then(|r| r.sym()) {
                    if self.syms.is_given(s) {
                        out.push((s, GivenScope::Lexical));
                    }
                }
            }
            ImportTarget::ValueGivens(_) | ImportTarget::ValueMember(..) | ImportTarget::ValueAll(_) => self.value_import_givens(imp, wanted, out),
            ImportTarget::UnimportPredef | ImportTarget::Unresolved => {}
            ImportTarget::PkgMember(p, n) => {
                if let Some(s) = self.pkg_term(p, n).and_then(|r| r.sym()) {
                    if self.syms.is_given(s) {
                        out.push((s, GivenScope::Lexical));
                    }
                }
            }
            // A wildcard import brings Scala 2 implicits in, as scalac's does, and no given.
            ImportTarget::PkgAll(p) => {
                let index = self.given_index(p);
                if index.has_scala2 {
                    index.select_scala2(wanted, &self.syms, out);
                }
            }
            ImportTarget::ClassAll(c) => {
                if let Some(index) = self.class_given_index(c) {
                    if index.has_scala2 {
                        index.select_scala2(wanted, &self.syms, out);
                    }
                }
            }
        }
    }

    /// The givens of one package clause of the file, a context of its own, and for the innermost
    /// clause each import statement at the top of the file, a context at the clause's level (one
    /// owner, `ContextualImplicits.level`) in the order of the source. Each clause is a level of
    /// its own, as in scalac.
    fn package_givens(&mut self, p: PkgId, with_imports: bool, wanted: Wanted, buf: &mut LevelBuf) {
        let (list, _) = self.package_level_list(p, with_imports, wanted);
        self.level_list_into(&list, buf);
    }

    /// `package_givens` for a level of its own, from `package_levels` where its inputs are
    /// settled: the package's index, the file's imports when they stand at the level, and the
    /// wanted class. The accessibility filter runs per search. A local import standing at depth
    /// 0 is an input the key does not name, so with one the level is neither looked up nor kept:
    /// the test precedes the lookup.
    fn package_level(&mut self, p: PkgId, with_imports: bool, wanted: Wanted, buf: &mut LevelBuf) {
        debug_assert!(buf.cands.is_empty(), "a level's candidates start empty");
        if with_imports && self.env.imports.iter().any(|imp| imp.depth == 0) {
            return self.package_givens(p, with_imports, wanted, buf);
        }
        let (tag, id) = wanted.key();
        let file = if with_imports { self.env.file.0 } else { u32::MAX };
        let key = (p, file, tag, id);
        let list = match self.package_levels.get(&key) {
            Some(list) => list.clone(),
            None => {
                let (list, settled) = self.package_level_list(p, with_imports, wanted);
                let list = Arc::new(list);
                if settled {
                    self.package_levels.insert(key, list.clone());
                }
                list
            }
        };
        self.level_list_into(&list, buf);
    }

    /// A package level's kept list into a search's level, the inaccessible imported givens left
    /// out.
    fn level_list_into(&mut self, list: &LevelList, buf: &mut LevelBuf) {
        if list.givens.is_empty() {
            return;
        }
        if list.plain {
            buf.cands.extend_from_slice(&list.givens);
            buf.nodes.extend_from_slice(&list.nodes);
            buf.renamed.extend_from_slice(&list.renamed);
            return;
        }
        let mut from = 0;
        for (n, &(end, prec)) in list.nodes.iter().enumerate() {
            for i in from..end as usize {
                let g = list.givens[i];
                // The package's own givens are each accessible in its clause.
                if n == 0 || self.is_given_accessible(g.0) {
                    if let Some(&(_, alias)) = list.renamed.iter().find(|&&(at, _)| at as usize == i) {
                        buf.renamed.push((buf.cands.len() as u32, alias));
                    }
                    buf.cands.push(g);
                }
            }
            buf.close(prec);
            from = end as usize;
        }
    }

    /// A package level's candidates before the accessibility filter of the imported ones, as
    /// `package_givens` has them after it, and whether every input is settled: the indexes read
    /// are kept (built over settled export tables) and no import reads a value, whose type a
    /// search may still be inferring.
    fn package_level_list(&mut self, p: PkgId, with_imports: bool, wanted: Wanted) -> (LevelList, bool) {
        let mut buf = LevelBuf::default();
        self.given_index(p).select(wanted, GivenScope::Lexical, &mut buf.cands);
        buf.cands.sort_by_key(|&(g, _)| g);
        buf.cands.dedup_by_key(|&mut (g, _)| g);
        buf.close(Prec::Def);
        let mut settled = self.given_indexes.contains_key(&p);
        if with_imports {
            // The file's imports by statement, in the order of the source.
            let n_imports = self.import_count();
            let mut imports: Vec<ResolvedImport> = (0..n_imports).map(|i| self.import_at(i)).filter(|imp| imp.depth == 0).collect();
            imports.sort_by_key(|imp| imp.stmt);
            let mut k = 0;
            while k < imports.len() {
                let stmt = imports[k].stmt;
                let end = imports[k..].iter().position(|imp| imp.stmt != stmt).map_or(imports.len(), |n| k + n);
                let clause = imports[k..end].to_vec();
                k = end;
                for &imp in &clause {
                    settled &= match imp.target {
                        ImportTarget::ClassMember(c, _) => self.exports_settled(ExportOwner::Class(c)),
                        ImportTarget::PkgMember(q, _) => self.exports_settled(ExportOwner::Pkg(q)),
                        ImportTarget::ValueGivens(_) | ImportTarget::ValueMember(..) | ImportTarget::ValueAll(_) => false,
                        _ => true,
                    };
                }
                if !clause.iter().any(|&imp| self.import_brings_implicits(imp)) {
                    continue;
                }
                self.stmt_givens(&clause, wanted, &mut buf, false);
                buf.close(if clause.iter().any(|imp| imp.name.is_none()) { Prec::Wild } else { Prec::Named });
                for &imp in &clause {
                    settled &= match imp.target {
                        ImportTarget::PkgGivens(q) | ImportTarget::PkgAll(q) => self.given_indexes.contains_key(&q),
                        ImportTarget::ClassGivens(c) | ImportTarget::ClassAll(c) => {
                            self.class_given_indexes.contains_key(&c) || (self.exports_settled(ExportOwner::Class(c)) && self.class_without_givens(c))
                        }
                        _ => true,
                    };
                }
            }
        }
        let own_end = buf.nodes[0].0 as usize;
        let plain = buf.cands[own_end..].iter().all(|&(g, _)| self.plainly_accessible(g));
        let list = LevelList { givens: buf.cands.into_boxed_slice(), nodes: buf.nodes.into_boxed_slice(), renamed: buf.renamed.into_boxed_slice(), plain };
        (list, settled)
    }

    /// The name a candidate of a level is known by (`implicitName`): the one an import renames it
    /// to, or its own.
    fn level_name(&self, buf: &LevelBuf, i: u32) -> Name {
        match buf.renamed.iter().find(|&&(at, _)| at == i) {
            Some(&(_, alias)) => alias,
            None => self.syms.sym(buf.cands[i as usize].0).name,
        }
    }

    /// The eligible candidates of a level (`ContextualImplicits.eligible`), indices of
    /// `buf.cands`: of the matching ones `fits` (in order), each context composed with the ones
    /// outside it, outermost first (`combineEligibles`): an inner context's candidates hide the
    /// same-named ones outside it, except where the outer context's precedence beats the inner's
    /// (`BindingPrec.beats`), which keeps the outer ones and drops the inner same-named. Inner
    /// first in the result. A context with no matching candidate still sets the precedence the
    /// next one is compared with.
    fn compose_level(&self, buf: &LevelBuf, fits: Vec<u32>) -> Vec<u32> {
        if buf.nodes.len() <= 1 || fits.is_empty() {
            return fits;
        }
        let mut acc: Vec<u32> = Vec::new();
        let mut outer: Option<Prec> = None;
        let mut k = 0;
        for &(end, prec) in &buf.nodes {
            let mut own: Vec<u32> = Vec::new();
            while k < fits.len() && fits[k] < end {
                own.push(fits[k]);
                k += 1;
            }
            if !own.is_empty() && !acc.is_empty() {
                if outer.is_some_and(|o| o.beats(prec)) {
                    own.retain(|&i| {
                        let n = self.level_name(buf, i);
                        !acc.iter().any(|&j| self.level_name(buf, j) == n)
                    });
                } else {
                    let names: Vec<Name> = own.iter().map(|&i| self.level_name(buf, i)).collect();
                    acc.retain(|&j| !names.contains(&self.level_name(buf, j)));
                }
            }
            own.extend_from_slice(&acc);
            acc = own;
            outer = Some(prec);
        }
        // A given two contexts bring under two names is one candidate (`disambiguate`: refs `=:=`).
        let mut seen: Vec<SymId> = Vec::with_capacity(acc.len());
        acc.retain(|&i| {
            let g = buf.cands[i as usize].0;
            !seen.contains(&g) && {
                seen.push(g);
                true
            }
        });
        acc
    }

    /// Givens of the implicit scope of `t`: the companions of every class the type mentions and
    /// of their ancestors, and the objects those classes are nested in. An open type variable
    /// stands for its bounds, so `Eq[?A]` with `?A >: Id` reaches the companion of `Id`. For a
    /// type without variables the selection is made once per wanted class and kept
    /// (`implicit_scope_givens`); only the accessibility filter runs per search, since a private
    /// given is accessible from inside its owner alone.
    fn implicit_scope_givens(&mut self, t: TypeId, wanted: Wanted, out: &mut Vec<GivenRef>) {
        let ground = !self.types.has_vars(t);
        let (tag, id) = wanted.key();
        if ground {
            if let Some(list) = self.implicit_scope_givens.get(&(t, tag, id)) {
                let list = list.clone();
                self.given_tree.restricted += list.restricted as u32;
                if list.plain {
                    out.extend_from_slice(&list.givens);
                } else {
                    out.extend(list.givens.iter().copied().filter(|&(g, scope)| self.is_given_accessible(g) && (!matches!(scope, GivenScope::Path(_)) || self.scope_accessible(scope))));
                }
                return;
            }
        }
        let mut selected = Vec::new();
        for scope in self.implicit_scope_objects(t) {
            let class = match scope {
                GivenScope::Module(m) => Some(m),
                GivenScope::Path(p) => {
                    let under = self.path_underlying(p);
                    self.class_of(under)
                }
                _ => None,
            };
            if let Some(index) = class.and_then(|c| self.class_given_index(c)) {
                index.select(wanted, scope, &mut selected);
            }
        }
        // A given is one candidate per path it is reached through, dotty's `TermRefSet` (a
        // symbol under each prefix): `o.item` and `o2.item` are two; one an object of the scope
        // inherits stays one, read on the first object.
        if selected.iter().any(|&(_, scope)| matches!(scope, GivenScope::Path(_))) {
            let key = |&(g, scope): &GivenRef| (g, if let GivenScope::Path(p) = scope { p.0 + 1 } else { 0 });
            selected.sort_by_key(key);
            selected.dedup_by_key(|r| key(r));
        } else {
            selected.sort_by_key(|&(g, _)| g);
            selected.dedup_by_key(|&mut (g, _)| g);
        }
        self.given_tree.restricted += selected.iter().any(|&(g, _)| self.restricted_visibility(g)) as u32;
        out.extend(selected.iter().copied().filter(|&(g, scope)| self.is_given_accessible(g) && (!matches!(scope, GivenScope::Path(_)) || self.scope_accessible(scope))));
        if ground {
            let list = ScopeList {
                restricted: selected.iter().any(|&(g, _)| self.restricted_visibility(g)),
                plain: selected.iter().all(|&(g, scope)| self.plainly_accessible(g) && self.scope_plainly_accessible(scope)),
                givens: selected.into_boxed_slice(),
            };
            self.implicit_scope_givens.insert((t, tag, id), Arc::new(list));
        }
    }

    /// Whether a given is accessible from any search (`is_given_accessible` reads nothing of
    /// the context for it): neither private nor protected, and no qualified access.
    /// Whether the path a candidate is read on is accessible where the search stands: each value
    /// it selects (a private companion through `i.C`, scalac's access check of the reference
    /// the candidate's tree makes, `SymDenotations.isAccessibleFrom`).
    pub(super) fn scope_accessible(&mut self, scope: GivenScope) -> bool {
        let GivenScope::Path(mut p) = scope else { return true };
        loop {
            match self.types.get(p) {
                Type::Select(q, s) => {
                    if !self.is_accessible(s) {
                        return false;
                    }
                    // A protected member through an instance of the class the access stands in
                    // (`Test.E` inside `Test`), not through another (`t.E` of a `t: T`).
                    if self.syms.sym(s).mods & crate::ast::mods::PROTECTED != 0 && !self.protected_through(q) {
                        return false;
                    }
                    p = q;
                }
                Type::Term(s) => return self.is_accessible(s),
                _ => return true,
            }
        }
    }

    /// Whether `q` is an instance of a class the search stands in: its `this`, the object itself,
    /// or a value of a type deriving from such a class.
    fn protected_through(&mut self, q: TypeId) -> bool {
        let k = match self.types.get(q) {
            Type::This(k) | Type::Class(k, _) => Some(k),
            _ => {
                let under = self.widen_path(q);
                self.class_of(under)
            }
        };
        let Some(k) = k else { return false };
        let classes: Vec<ClassId> = self.env.frames.iter().filter_map(|f| match f {
            super::Frame::Class(x) => Some(*x),
            _ => None,
        }).collect();
        classes.into_iter().any(|x| x == k || self.derives_from(k, x))
    }

    /// Whether the path a candidate is read on selects no value of restricted visibility, so
    /// that its accessibility reads nothing of the search's site.
    fn scope_plainly_accessible(&self, scope: GivenScope) -> bool {
        let GivenScope::Path(mut p) = scope else { return true };
        loop {
            match self.types.get(p) {
                Type::Select(q, s) => {
                    if !self.plainly_accessible(s) {
                        return false;
                    }
                    p = q;
                }
                Type::Term(s) => return self.plainly_accessible(s),
                _ => return true,
            }
        }
    }

    fn plainly_accessible(&self, g: SymId) -> bool {
        self.syms.sym(g).mods & (crate::ast::mods::PRIVATE | crate::ast::mods::PROTECTED | crate::ast::mods::QUALIFIED) == 0
    }

    fn restricted_visibility(&self, g: SymId) -> bool {
        self.syms.sym(g).mods & (crate::ast::mods::PRIVATE | crate::ast::mods::PROTECTED) != 0
    }

    /// The objects of the implicit scope of `t` (`implicit_scope_objects`), where the extensions
    /// and conversions of the scope are looked up.
    pub fn implicit_scope_modules(&mut self, t: TypeId) -> Vec<ClassId> {
        self.implicit_scope_objects(t).into_iter().filter_map(GivenScope::module).collect()
    }

    /// The objects and paths whose members make up the implicit scope of `t`, computed once for a type
    /// without variables: the classes it mentions are completed on the way, so their bases and
    /// companions do not change afterwards.
    pub fn implicit_scope_objects(&mut self, t: TypeId) -> Vec<GivenScope> {
        let ground = !self.types.has_vars(t);
        if ground {
            if let Some(scopes) = self.implicit_scopes.get(&t) {
                return scopes.to_vec();
            }
        }
        let scopes = self.implicit_scope_objects_now(t);
        if ground {
            self.implicit_scopes.insert(t, Arc::from(scopes.as_slice()));
        }
        scopes
    }

    fn implicit_scope_objects_now(&mut self, t: TypeId) -> Vec<GivenScope> {
        let mut classes = Vec::new();
        let mut seen = ScopeSeen::default();
        self.classes_in(t, &mut classes, &mut seen);
        let mut scopes: Vec<ClassId> = Vec::new();
        let mut paths = seen.paths;
        for c in classes {
            // A Java class holds no givens: one nothing has read yet stays unread.
            if !self.is_java_placeholder(c) {
                self.complete_class(c);
            }
            // The companion of an inner class the type names through a path is reached through
            // that path (dotty's `addCompanion(pre, companion)`): one per enclosing instance.
            let through: Vec<TypeId> = seen.through.iter().filter(|&&(k, _)| k == c).map(|&(_, p)| p).collect();
            if let Some(val) = self.syms.class(c).companion.and_then(|m| self.syms.class(m).inner_object).filter(|_| !through.is_empty()) {
                for p in through {
                    let path = self.types.mk(Type::Select(p, val));
                    if !paths.contains(&path) {
                        paths.push(path);
                    }
                }
            }
            let n_bases = self.syms.class(c).base_types.len();
            for i in 0..n_bases {
                let mut b = self.syms.class(c).base_types[i].0;
                loop {
                    let info = self.syms.class(b);
                    let module = if self.is_module_class(b) { Some(b) } else { info.companion };
                    let module = module.filter(|&m| !(b == c && seen.through.iter().any(|&(k, _)| k == c) && self.syms.class(m).inner_object.is_some()));
                    if let Some(m) = module.filter(|&m| self.is_module_class(m)) {
                        if !scopes.contains(&m) {
                            scopes.push(m);
                        }
                    }
                    match self.syms.class(b).owner {
                        Owner::Class(o) if self.is_module_class(o) => b = o,
                        _ => break,
                    }
                }
            }
            // As Scala.js has it, `UnitOps` holds the conversion of `A | Unit` to
            // `js.UndefOrOps[A]`, which the companion of `js.|` held before unions.
            if c == self.b.unit && !self.jvm {
                if let Some(m) = self.unit_ops_object().filter(|m| !scopes.contains(m)) {
                    scopes.push(m);
                }
            }
        }
        scopes.into_iter().map(GivenScope::Module).chain(paths.into_iter().map(GivenScope::Path)).collect()
    }

    /// The path `p` and the paths it is selected on as references of the implicit scope, dotty's
    /// `addPath`: a val declared with a singleton type is the path it names, an object's is its
    /// object's scope (the classes it mentions give it), any other val the path itself.
    fn add_scope_path(&mut self, p: TypeId, out: &mut Vec<ClassId>, seen: &mut ScopeSeen) {
        let mut p = p;
        for _ in 0..8 {
            let (Type::Term(_) | Type::Select(..)) = self.types.get(p) else { return };
            let under = self.path_underlying(p);
            if self.types.is_path(under) {
                p = under;
                continue;
            }
            match self.class_of(under) {
                Some(c) if self.syms.class(c).kind == ClassKind::Object => {
                    if !out.contains(&c) {
                        out.push(c);
                    }
                }
                _ => {
                    if !seen.paths.contains(&p) {
                        seen.paths.push(p);
                    }
                }
            }
            match self.types.get(p) {
                Type::Select(q, _) => p = q,
                _ => return,
            }
        }
    }

    /// An object, top level or nested in a class or trait: what holds givens of the implicit
    /// scope and stands for the class it is the companion of.
    pub(super) fn is_module_class(&self, c: ClassId) -> bool {
        let info = self.syms.class(c);
        info.kind == ClassKind::Object || info.inner_object.is_some()
    }

    fn unit_ops_object(&mut self) -> Option<ClassId> {
        if let Some(found) = self.unit_ops {
            return found;
        }
        let found = self.class_at(&["scala", "scalajs", "js", "internal", "UnitOps"]);
        self.unit_ops = Some(found);
        found
    }

    fn classes_in(&mut self, t: TypeId, out: &mut Vec<ClassId>, seen: &mut ScopeSeen) {
        let t = self.deref(t);
        match self.types.get(t) {
            // A type parameter is no anchor: its scope is the scope of its bounds (dotty's
            // `liftToAnchors`: the upper bound alone where the lower is bottom, else both).
            Type::Param(p) => self.classes_in_param_bounds(p, out, seen),
            Type::Class(c, args) => {
                if !out.contains(&c) {
                    out.push(c);
                }
                for a in self.types.items(args).to_vec() {
                    self.classes_in(a, out, seen);
                }
            }
            Type::Ctor(c) => {
                if !out.contains(&c) {
                    out.push(c);
                }
            }
            Type::AppParam(p, args) => {
                self.classes_in_param_bounds(p, out, seen);
                for a in self.types.items(args).to_vec() {
                    self.classes_in(a, out, seen);
                }
            }
            Type::Var(v) => self.classes_in_bounds(v, out, seen),
            Type::AppVar(v, args) => {
                self.classes_in_bounds(v, out, seen);
                for a in self.types.items(args).to_vec() {
                    self.classes_in(a, out, seen);
                }
            }
            Type::Lambda(_, b) => self.classes_in(b, out, seen),
            Type::Union(a, b) | Type::Inter(a, b) => {
                self.classes_in(a, out, seen);
                self.classes_in(b, out, seen);
            }
            // `p.C`: the class and its prefix's parts (dotty's `collectParts` traverses a class
            // reference's prefix); a path prefix is itself a reference of the scope (`addPath`),
            // and the class's companion is reached through it.
            Type::Nested(p, class) => {
                self.classes_in(class, out, seen);
                // An object is a prefix the class's companion is reached through too (`Test.E`
                // of a trait's `E` in `object Test extends T`).
                if matches!(self.types.get(p), Type::Class(k, _) if self.syms.class(k).kind == ClassKind::Object) {
                    if let Some(c) = self.class_of(class) {
                        if !seen.through.contains(&(c, p)) {
                            seen.through.push((c, p));
                        }
                    }
                    self.classes_in(p, out, seen);
                } else if self.types.is_path(p) {
                    if let Some(c) = self.class_of(class) {
                        if !seen.through.contains(&(c, p)) {
                            seen.through.push((c, p));
                        }
                    }
                    self.add_scope_path(p, out, seen);
                    let under = self.path_underlying(p);
                    if under != p {
                        self.classes_in(under, out, seen);
                    }
                } else {
                    self.classes_in(p, out, seen);
                }
            }
            Type::Refined(parent, r) => {
                self.classes_in(parent, out, seen);
                match self.types.refinement(r) {
                    Refinement::Alias(_, rhs) => self.classes_in(rhs, out, seen),
                    Refinement::Bounds(_, lo, hi) => {
                        self.classes_in(lo, out, seen);
                        self.classes_in(hi, out, seen);
                    }
                    Refinement::Val(_, _, ty) => self.classes_in(ty, out, seen),
                    Refinement::Term(..) => {}
                }
            }
            Type::This(_) | Type::Term(_) | Type::Select(..) => {
                let under = self.path_underlying(t);
                if under != t {
                    self.classes_in(under, out, seen);
                }
            }
            // An abstract type member's scope is its prefix's and its bounds' (dotty's
            // `addCompanions` for an abstract type: the scopes of its lower and upper bound).
            Type::Decl(a) => {
                if seen.members.contains(&t) {
                    return;
                }
                seen.members.push(t);
                if let Owner::Class(o) = self.syms.aliases[a.idx()].owner {
                    if !out.contains(&o) {
                        out.push(o);
                    }
                }
                let (lo, hi) = self.member_bounds(t);
                if lo != NOTHING {
                    self.classes_in(lo, out, seen);
                }
                self.classes_in(hi, out, seen);
            }
            Type::Member(prefix, _) => {
                if seen.members.contains(&t) {
                    return;
                }
                seen.members.push(t);
                let under = if self.types.is_path(prefix) { self.path_underlying(prefix) } else { prefix };
                match self.types.get(under) {
                    Type::This(c) => {
                        if !out.contains(&c) {
                            out.push(c);
                        }
                    }
                    _ => self.classes_in(under, out, seen),
                }
                let (lo, hi) = self.member_bounds(t);
                if lo != NOTHING {
                    self.classes_in(lo, out, seen);
                }
                self.classes_in(hi, out, seen);
            }
            Type::AppMember(m, args) => {
                self.classes_in(m, out, seen);
                for a in self.types.items(args).to_vec() {
                    self.classes_in(a, out, seen);
                }
            }
            Type::Alias(..) | Type::Match(..) => {
                if let Some(reduced) = self.dependent_underlying(t) {
                    self.classes_in(reduced, out, seen);
                }
            }
            _ => {}
        }
    }

    fn classes_in_bounds(&mut self, v: TVarId, out: &mut Vec<ClassId>, seen: &mut ScopeSeen) {
        if seen.vars.contains(&v) {
            return;
        }
        seen.vars.push(v);
        let info = &self.tvars[v];
        let bounds: Vec<TypeId> = info.lower.iter().chain(&info.upper).copied().collect();
        for b in bounds {
            self.classes_in(b, out, seen);
        }
    }

    /// The classes of a type parameter's bounds, each parameter walked once (an F-bound
    /// `T <: Ord[T]` names the parameter again).
    fn classes_in_param_bounds(&mut self, p: TParamId, out: &mut Vec<ClassId>, seen: &mut ScopeSeen) {
        if seen.params.contains(&p) {
            return;
        }
        seen.params.push(p);
        let info = self.syms.tparam(p);
        let (lo, hi) = (info.lower, info.upper);
        if lo != NOTHING {
            self.classes_in(lo, out, seen);
        }
        if hi != ANY {
            self.classes_in(hi, out, seen);
        }
    }

    /// Givens that may provide an extension method called `name` for a receiver of type
    /// `recv_ty`: those in scope, innermost first, then those in the implicit scope of the
    /// receiver, which start at the returned index.
    pub fn givens_with_extension(&mut self, name: Name, recv_ty: TypeId) -> (Vec<(GivenRef, TypeId)>, usize) {
        let (givens, contextual, _) = self.givens_with_extension_levels(name, recv_ty);
        (givens, contextual)
    }

    /// `givens_with_extension`, with the nesting level of each given in scope: its frame's,
    /// counted from the outermost, the imports and packages around them all at 0.
    pub fn givens_with_extension_levels(&mut self, name: Name, recv_ty: TypeId) -> (Vec<(GivenRef, TypeId)>, usize, Vec<u32>) {
        let step = self.step(Step::ExtCollect);
        let found = self.givens_with_extension_now(name, recv_ty);
        self.step_end(step, Step::ExtCollect);
        found
    }

    fn givens_with_extension_now(&mut self, name: Name, recv_ty: TypeId) -> (Vec<(GivenRef, TypeId)>, usize, Vec<u32>) {
        let wanted = Wanted::Extension(name);
        let mut refs: Vec<GivenRef> = Vec::new();
        let mut levels = Vec::new();
        // The levels of the search (`given_levels`), the implicit scope's apart: each level's
        // eligible candidates as `ContextualImplicits.eligible` composes them, those a nearer
        // level's of their name hides left out.
        let given_levels = self.given_levels();
        let mut buf = LevelBuf::default();
        let mut hidden: Vec<Name> = Vec::new();
        for level in (1..given_levels.count()).rev() {
            buf.clear();
            self.level_candidates(&given_levels, level, recv_ty, wanted, &mut buf);
            if buf.cands.is_empty() {
                continue;
            }
            let mut fits: Vec<u32> = Vec::new();
            for i in 0..buf.cands.len() as u32 {
                let g = buf.cands[i as usize].0;
                if refs.iter().any(|&(known, _)| known == g) || !hidden.is_empty() && hidden.contains(&self.level_name(&buf, i)) {
                    continue;
                }
                fits.push(i);
            }
            for &i in &fits {
                let n = self.level_name(&buf, i);
                if !hidden.contains(&n) {
                    hidden.push(n);
                }
            }
            for i in self.compose_level(&buf, fits) {
                refs.push(buf.cands[i as usize]);
                levels.push(level as u32);
            }
        }
        let contextual = refs.len();
        let mut outer = Vec::new();
        self.implicit_scope_givens(recv_ty, wanted, &mut outer);
        outer.retain(|&(g, _)| !refs.iter().any(|&(known, _)| known == g));
        refs.append(&mut outer);
        (refs.into_iter().map(|r| (r, self.sig_of(r.0).ret)).collect(), contextual, levels)
    }

    pub fn resolve_given(&mut self, target: TypeId, span: Span) -> Option<TExprId> {
        self.resolve_given_typed(target, span).map(|(te, _)| te)
    }

    /// The given for `target` and its own type, which is what `summon` returns.
    pub fn resolve_given_typed(&mut self, target: TypeId, span: Span) -> Option<(TExprId, TypeId)> {
        self.resolve_given_telling(target, span).ok()
    }

    /// `resolve_given_typed`, telling a search that found nothing (`Err(false)`) from an
    /// ambiguous one (`Err(true)`).
    pub fn resolve_given_telling(&mut self, target: TypeId, span: Span) -> Result<(TExprId, TypeId), bool> {
        let pick = self.resolve_given_pick(target, span, false);
        // The outermost search is committed: what its tree took uses the imports bringing it.
        if self.implicit_depth == 0 && !self.unused.given_winners.is_empty() {
            self.settle_given_winners(matches!(pick, Pick::Found(..)));
        }
        match pick {
            Pick::Found(te, ty) => {
                let te = if self.capturing() { self.splice_quotes(te, target) } else { te };
                if self.inline.depth > 0 {
                    self.mark_leaf(te);
                }
                Ok((te, ty))
            }
            Pick::Ambiguous => Err(true),
            Pick::NoMatch => Err(false),
        }
    }

    /// The given for `target` by the search alone, without the synthesis a failed search falls
    /// back to: dotty's `inferImplicit` (Implicits.scala 1113), which `tryWithTypeTest` calls for
    /// a type pattern's `TypeTest` and `ClassTag` (Typer.scala 1386), where `inferImplicitArg`
    /// adds `Synthesizer.tryAll` (947 to 958). Nested requests, a candidate's using clause,
    /// synthesize as they do under scalac. A search that fails or is ambiguous leaves nothing
    /// behind: no constraint, no diagnostic, no ambiguity message.
    pub fn resolve_given_search_only(&mut self, target: TypeId, span: Span) -> Option<(TExprId, TypeId)> {
        let mark = self.snapshot();
        let diags = self.diags.items.len();
        let outer = self.search_only_depth.replace(self.implicit_depth);
        let found = self.resolve_given_telling(target, span).ok();
        self.search_only_depth = outer;
        if found.is_none() {
            self.rollback(mark);
            self.drop_reported_since(diags);
            self.given_ambiguity = None;
        }
        found
    }

    /// The given for `target` found for the capture alone, which leaves the executable IR as it
    /// is: no leaf of an expansion marked.
    pub(super) fn resolve_given_for_capture(&mut self, target: TypeId, span: Span) -> Option<TExprId> {
        let pick = self.resolve_given_pick(target, span, false);
        if self.implicit_depth == 0 && !self.unused.given_winners.is_empty() {
            self.settle_given_winners(false);
        }
        match pick {
            Pick::Found(te, _) => Some(te),
            Pick::Ambiguous | Pick::NoMatch => None,
        }
    }

    /// `X.type` of an implicit object or a stable given `X`: the path itself is the one value of
    /// the type, reached through the enclosing class that has it as a member.
    /// The value of the stable path `s` (a local, a top-level val, a member of an enclosing
    /// class or of an object) as an expression where the typer stands; none for a member of a
    /// class that is no enclosing one.
    pub(super) fn stable_path_expr(&mut self, s: SymId) -> Option<TExprId> {
        Some(match self.syms.sym(s).owner {
            Owner::Class(o) => {
                let classes: Vec<ClassId> = self.env.frames.iter().rev().filter_map(|f| match f {
                    Frame::Class(k) => Some(*k),
                    _ => None,
                }).collect();
                match classes.into_iter().find(|&k| k == o || self.derives_from(k, o)) {
                    Some(through) => {
                        let (this, _) = self.type_this_of(through);
                        self.prog.add(TExpr::Field(this, s))
                    }
                    // An object's member (`Registry.token`): through the object itself.
                    None if self.syms.class(o).kind == ClassKind::Object && !self.class_in_class(o) => {
                        let module = self.prog.add(TExpr::Module(o));
                        self.prog.add(TExpr::Field(module, s))
                    }
                    None => return None,
                }
            }
            Owner::Package(_) => self.prog.add(TExpr::Static(s)),
            Owner::Local => self.prog.add(TExpr::Local(s)),
        })
    }

    fn singleton_given(&mut self, target: TypeId) -> Option<(TExprId, TypeId)> {
        let t = self.deref(target);
        let s = match self.types.get(t) {
            Type::Term(s) | Type::Select(_, s) => s,
            _ => return None,
        };
        let info = self.syms.sym(s);
        if info.mods & (crate::ast::mods::IMPLICIT | crate::ast::mods::GIVEN) == 0 || !matches!(info.kind, SymKind::Val | SymKind::Object(_)) {
            return None;
        }
        let te = self.stable_path_expr(s)?;
        self.prog.set_type(te, t);
        Some((te, t))
    }

    /// The givens the compiler supplies where no definition provides one, as scalac synthesizes
    /// them.
    #[cold]
    #[inline(never)]
    fn synthesized_given(&mut self, c: ClassId, target: TypeId, span: Span, byname: bool) -> Option<TExprId> {
        let te = self.evidence_given(c, target, span)
            .or_else(|| self.class_tag_given(c, target))
            .or_else(|| self.type_test_given(c, target, span))
            .or_else(|| self.value_of_given(c, target))
            .or_else(|| self.quoted_type_given(c, target, span))
            .or_else(|| self.synthesized_can_equal(c, target, span))
            .or_else(|| self.not_given(c, target, span, byname))?;
        if self.capturing() {
            self.capture_form(te, crate::tir::capture::Form::Evidence(target));
        }
        Some(te)
    }

    /// `NotGiven[T]` when the search for `T` finds nothing, as scalac provides it: the companion's
    /// cached `value` (dotty's `negateIfNot` references `NotGiven_value`, Implicits.scala:1537-1538,
    /// scala-library's `cachedValue`), with the request kept open in the search history while `T`
    /// is searched, as the candidate attempt of dotty's `amb1[T](using ev: T)` keeps `NotGiven[T]`
    /// in the `OpenSearch` chain (`recursiveRef`, Implicits.scala:1908-1944): a by-name request for
    /// `NotGiven[T]` below it ties to this instance, so `given foo(using n: => NotGiven[Foo]): Foo`
    /// provides a `Foo` under a by-name `NotGiven[Foo]` request and the negation fails, where the
    /// nested `absent[NotGiven[Foo]]` ties its inner request and succeeds. A request that diverges
    /// from an open one is a failed candidate under scalac, which the negation makes a success.
    pub fn not_given(&mut self, class: ClassId, target: TypeId, span: Span, byname: bool) -> Option<TExprId> {
        if !self.is_not_given_class(class) {
            return None;
        }
        let target = self.zonk(target);
        let Type::Class(_, args) = self.types.get(target) else { return None };
        let &[negated] = self.types.items(args) else { return None };
        let (companion, value) = self.not_given_value(class)?;
        let mut open = self.open_given(value, target);
        open.byname = byname;
        let (found, knot) = if self.diverges(&open) {
            (false, None)
        } else {
            self.given_stack.push(open);
            self.given_opened.push(value);
            self.implicit_depth += 1;
            let mark = self.attempt();
            let found = matches!(self.resolve_given_pick(negated, span, false), Pick::Found(..));
            self.retract(mark);
            self.implicit_depth -= 1;
            (found, self.given_stack.pop().and_then(|open| open.rec))
        };
        if found {
            return None;
        }
        let recv = self.prog.add(TExpr::Module(companion));
        let mut te = self.prog.add(TExpr::CallMethod(recv, value, ListRef::EMPTY));
        self.prog.set_type(te, target);
        // A nested by-name request referred to this instance: it becomes a lazy local that the
        // thunks inside read once the instance exists, as a candidate's instance does.
        if let Some(rec) = knot {
            let stmts = self.prog.stmts.push_slice(&[TStmt::Val(rec, te)]);
            let result = self.prog.add(TExpr::Local(rec));
            te = self.prog.add(TExpr::Block(stmts, result));
        }
        Some(te)
    }

    /// `NotGiven.value` where a candidate of the search for `target`, a `NotGiven`, fails
    /// (`rank`'s `negateIfNot(tryImplicit(..))`, Implicits.scala 1493 and 1533): the candidates
    /// each level finds eligible, composed and hidden as `search_given` has them, the context's
    /// and then the implicit scope's (`searchImplicit`, 1700 to 1712: a companion's of the negated
    /// type among them), are tried in its order, a failure negated into the success, a success into
    /// a failure that leaves the search to the next candidate. NotGiven's companion's own and its
    /// parents' (`amb1`, `amb2`, `default`) are what the synthesis answers for (`not_given`), after.
    fn negated_not_given(&mut self, class: ClassId, target: TypeId, span: Span) -> Option<TExprId> {
        let (companion, value) = self.not_given_value(class)?;
        let library = |t: &mut Self, g: SymId| match t.syms.sym(g).owner {
            Owner::Class(o) => o == companion || t.derives_from(companion, o),
            _ => false,
        };
        let levels = self.given_levels();
        let wanted = Wanted::Class(class);
        let memoised = self.transparent.is_empty() && !self.types.has_vars(target) && self.gadt.is_empty();
        let mut heads = None;
        let mut hidden: Vec<Name> = Vec::new();
        let mut buf = LevelBuf::default();
        let mut failed = false;
        'levels: for level in (0..levels.count()).rev() {
            buf.clear();
            self.level_candidates(&levels, level, target, wanted, &mut buf);
            if buf.cands.is_empty() {
                continue;
            }
            let heads = heads.get_or_insert_with(|| self.target_heads(target));
            let eligible = self.level_eligible(&levels, level, target, Some(class), &buf, &mut hidden, memoised, heads);
            let mut order: Vec<usize> = eligible.into_iter().map(|i| i as usize).collect();
            if order.len() > 1 {
                self.preference_order(&buf.cands, &mut order);
            }
            for i in order {
                let given = buf.cands[i];
                if library(self, given.0) {
                    continue;
                }
                let mark = self.attempt();
                let attempt = self.try_given(given, Some(target), span, true);
                self.retract(mark);
                if !matches!(attempt, Attempt::Ok(..)) {
                    failed = true;
                    break 'levels;
                }
            }
        }
        if !failed {
            return None;
        }
        let recv = self.prog.add(TExpr::Module(companion));
        let te = self.prog.add(TExpr::CallMethod(recv, value, ListRef::EMPTY));
        self.prog.set_type(te, target);
        Some(te)
    }

    /// `NotGiven`'s companion object and its `value` member.
    fn not_given_value(&mut self, class: ClassId) -> Option<(ClassId, SymId)> {
        let companion = self.syms.class(class).companion?;
        let obj = self.types.class(companion, &[]);
        let name = self.interner.intern("value");
        let (sym, _) = self.find_member(obj, name)?;
        Some((companion, sym))
    }

    /// `byname` says that the request is for a by-name using parameter, through which a
    /// recursive instance may refer back to itself.
    pub(super) fn resolve_given_pick(&mut self, target: TypeId, span: Span, byname: bool) -> Pick {
        let pre = self.given_entry();
        if self.implicit_depth >= MAX_IMPLICIT_DEPTH {
            self.read_in_progress(InProgress::DepthLimit);
            if self.given_trace() {
                let t = self.show(target);
                eprintln!("depth limit: {}", t);
            }
            self.given_early(pre, Early::DepthLimit);
            return Pick::NoMatch;
        }
        let target = self.zonk(target);
        // A target that is an alias application or a match type is searched as what its head
        // reduces to (`(Eq |: Derived)[Int]` as `OrElse[Eq[Int], Derived[Eq[Int]]]`); an
        // application inside an argument stays, since a candidate's `F[T]` unifies with it.
        let target = match self.types.head_reducible(self.types.get(target)) {
            true => self.reduce_head(target).unwrap_or(target),
            false => target,
        };
        // The unresolved type has been reported already; searching for it would only produce
        // follow-up errors, and generic candidates make that search exponential.
        if self.types.contains_error(target) {
            self.given_early(pre, Early::ErrorTarget);
            return Pick::Found(self.prog.add(TExpr::Unit), target);
        }
        if let Some((te, ty)) = self.singleton_given(target) {
            self.given_early(pre, Early::Singleton);
            return Pick::Found(te, ty);
        }
        if byname || self.given_stack.iter().any(|open| open.byname) {
            if let Some((te, ty)) = self.recursive_ref(target, byname, span) {
                self.read_in_progress(InProgress::RecursiveAnswer);
                self.given_early(pre, Early::RecursiveRef);
                return Pick::Found(te, ty);
            }
        }
        if self.implicit_depth == 0 {
            self.given_ambiguity = None;
            self.failed_givens.clear();
            self.implicit_budget = IMPLICIT_SEARCH_LIMIT;
        }
        // A type nothing constrains yet (`implicitly` without an expected type) has no given,
        // as in scalac, although a generic one such as `conforms[A]: A => A` would fit it.
        let bare = self.deref(target);
        if let Type::Var(v) = self.types.get(bare) {
            let info = &self.tvars[v];
            if info.lower.is_empty() && info.upper.is_empty() {
                self.given_early(pre, Early::BareVar);
                return Pick::NoMatch;
            }
        }
        let target_class = self.class_of(target);
        // A `NotGiven[T]` target: scalac negates every candidate's result (`Implicits.negateIfNot`),
        // a failure into `NotGiven.value` and a success into a failure. The companion's `default`,
        // `amb1` and `amb2` of scala-library negate the search for `T`, which the synthesis makes;
        // a program's own candidate that fits and fails is a success as well.
        if let Some(c) = target_class.filter(|&c| self.is_not_given_class(c)) {
            self.given_early(pre, Early::NotGiven);
            // The contextual candidates first, each negated; the companion's fallback where none
            // fails (`searchImplicit`'s implicit scope after the context).
            return match self.negated_not_given(c, target, span).or_else(|| self.synthesized_given(c, target, span, byname)) {
                Some(te) => Pick::Found(te, target),
                None => Pick::NoMatch,
            };
        }
        if self.given_trace() {
            let t = self.show(target);
            eprintln!("search {} {} #{}", self.implicit_depth, t, target.0);
            self.trace_starts.push(std::time::Instant::now());
        }
        self.given_reached(pre);
        let prof = self.prof(Kind::Given, span, About::Type(target));
        if let Some(p) = prof {
            let origin = self.given_origin();
            self.profile.set_origin(p, origin);
            let ground = !self.types.has_vars(target);
            self.profile.set_request(p, byname, ground);
        }
        let depth = self.implicit_depth;
        let step = self.step(Step::Applicable);
        let memo = self.memo_applicable(target);
        self.step_end(step, Step::Applicable);
        // A by-name request marks the givens opened below it, so that a nested request may
        // refer back to their instances: a kept tree is not what such a search builds.
        let hit = match memo {
            Memo::Hit if !byname => {
                let step = self.step(Step::Lookup);
                let hit = self.memo_lookup(target, depth, prof);
                self.step_end(step, Step::Lookup);
                hit
            }
            Memo::Hit => {
                if let Some(p) = prof {
                    self.profile.set_memo(p, Memo::ByNameRequest);
                }
                None
            }
            _ => {
                if let Some(p) = prof {
                    self.profile.set_memo(p, memo);
                }
                None
            }
        };
        if let Some(Pick::Found(expr, ty)) = hit {
            let step = self.step(Step::Replay);
            let copied = self.copy_expr(expr);
            self.prog.set_span(copied, self.env.file, span);
            self.step_end(step, Step::Replay);
            if let Some(p) = prof {
                self.profile.given.cached += 1;
                self.profile.exit(p, Outcome::Found);
            }
            return Pick::Found(copied, ty);
        }
        self.given_tree.byname_requests += byname as u32;
        let marks = self.tree_marks(depth);
        self.implicit_depth += 1;
        let outer_byname = std::mem::replace(&mut self.search_byname, byname);
        let result = match hit {
            Some(pick) => {
                self.profile.given.cached += self.profile.on as u64;
                if let Some(p) = prof {
                    self.profile.set_hit_nomatch(p);
                }
                pick
            }
            None => {
                let result = self.search_given(target, target_class, span);
                // At the site's span, as a memo hit's copy is: a site's result is the same
                // whichever search a worker made first.
                if let Pick::Found(e, _) = result {
                    self.prog.set_span(e, self.env.file, span);
                }
                if memo == Memo::Hit {
                    let step = self.step(Step::Keep);
                    let keep = self.memo_keep(target, &result, &marks, byname);
                    self.step_end(step, Step::Keep);
                    if let Some(p) = prof {
                        self.profile.set_keep(p, keep);
                    }
                }
                result
            }
        };
        self.given_tree.deepest = marks.deepest.max(self.given_tree.deepest);
        let synthesis = if matches!(result, Pick::NoMatch) { self.step(Step::Synthesis) } else { super::profile::StepTok::NONE };
        let result = match result {
            Pick::NoMatch if target_class.map_or(false, |c| self.derive_mirror_class(c)) => {
                match target_class.and_then(|c| self.scala_mirror_given(c, target)) {
                    // The mirror of the class the target names, as long as it has the element
                    // types and labels the target asks for.
                    Some((te, ty)) if !self.mirror_target_fixes_members(target) || self.conforms_or_rollback(ty, target) => {
                        if self.capturing() {
                            self.capture_form(te, crate::tir::capture::Form::Evidence(target));
                        }
                        Pick::Found(te, ty)
                    }
                    _ => Pick::NoMatch,
                }
            }
            // A search alone (`resolve_given_search_only`) ends with what the candidates gave.
            Pick::NoMatch if self.search_only_depth == Some(depth) => Pick::NoMatch,
            Pick::NoMatch => match target_class.and_then(|c| self.synthesized_given(c, target, span, byname)) {
                Some(te) => Pick::Found(te, target),
                None => Pick::NoMatch,
            },
            pick => pick,
        };
        self.step_end(synthesis, Step::Synthesis);
        self.search_byname = outer_byname;
        self.implicit_depth -= 1;
        if self.implicit_depth == 0 {
            self.given_consulted.clear();
            self.given_opened.clear();
        }
        if self.given_trace() {
            let t = self.show(target);
            let outcome = match &result {
                Pick::Found(..) => "found",
                Pick::NoMatch => "nomatch",
                Pick::Ambiguous => "ambiguous",
            };
            let us = self.trace_starts.pop().map_or(0, |s| s.elapsed().as_micros());
            let memo = prof.map(|p| self.profile.memo_of(p)).unwrap_or_default();
            eprintln!("end {} {} {} {}us {}", self.implicit_depth, t, outcome, us, memo);
        }
        if let Some(p) = prof {
            let outcome = match &result {
                Pick::Found(..) => Outcome::Found,
                Pick::NoMatch => Outcome::NotFound,
                Pick::Ambiguous => Outcome::Ambiguous,
            };
            self.profile.exit(p, outcome);
        }
        if self.implicit_depth == 0 && self.implicit_budget == 0 {
            let shown = self.show(target);
            let msg = format!(
                "Implicit search problem too large. an implicit search was terminated with failure after trying {} expressions. The root candidate for the search was: {}",
                IMPLICIT_SEARCH_LIMIT, shown
            );
            self.error(span, msg);
            return Pick::Found(self.prog.add(TExpr::Unit), target);
        }
        result
    }

    /// Whether `g` is filed as a conversion method: a jar's by the flag the loader read, one of
    /// the source by its signature.
    fn is_conversion_candidate(&mut self, g: SymId) -> bool {
        match self.loaded.as_ref() {
            Some(l) if l.syms.contains_key(&g) => l.is_conversion(g),
            _ => self.is_conversion_def(g),
        }
    }

    /// Whether the type has an open variable nothing bounds, which stays open where
    /// `try_conversion_method` solves the target (`B` of a `flatten`'s `A => IterableOnce[B]`).
    fn has_unbounded_var(&mut self, t: TypeId) -> bool {
        if !self.types.has_vars(t) {
            return false;
        }
        let mut vars = Vec::new();
        self.collect_vars(t, &mut vars);
        vars.iter().any(|v| {
            let info = &self.tvars[*v];
            info.inst.is_none() && info.lower.is_empty() && info.upper.is_empty()
        })
    }

    /// A conversion method as the implicit value of a function type `A => B`, eta-expanded
    /// (`implicit f: A => VdomNode` filled by an `implicit def f(a: A): VdomNode`). A target
    /// the arguments applied so far leave open is no match; a conversion whose own implicit
    /// clause asks for the target again is not tried inside itself.
    fn try_conversion_method(&mut self, given: GivenRef, target: Option<TypeId>, span: Span) -> Attempt {
        let Some(target) = target else { return Attempt::Mismatch };
        let t = self.deref(target);
        let Type::Class(c, _) = self.types.get(t) else { return Attempt::Mismatch };
        if c != self.function_class(1) {
            return Attempt::Mismatch;
        }
        let t = self.solve_bounded_in(t);
        let Type::Class(_, args) = self.types.get(t) else { return Attempt::Mismatch };
        if !self.function_targets.is_empty() {
            self.read_in_progress(InProgress::ConversionGuard);
        }
        if self.types.has_vars(t) || self.function_targets.contains(&t) {
            return Attempt::Mismatch;
        }
        let (from, to) = (self.types.items(args)[0], self.types.items(args)[1]);
        if self.conversion_result_excluded(given.0, to) {
            return Attempt::Mismatch;
        }
        let param = self.fresh_local("x", from, span);
        let arg = self.prog.add(TExpr::Local(param));
        self.function_targets.push(t);
        let body = self.conversion_method_body(given, arg, from, to, span);
        self.function_targets.pop();
        let Some(body) = body else { return Attempt::Mismatch };
        let params = self.prog.syms(&[param]);
        let te = self.prog.add(TExpr::Lambda(params, body));
        self.prog.set_type(te, t);
        Attempt::Ok(te, t)
    }

    /// Whether a conversion method's result cannot conform to a union target by the classes
    /// alone, so that its attempt fails at its result check (`try_conversion_in`'s `accept`,
    /// before any using clause is searched) and is left out. Its declared result is a nominal
    /// class, no tuple, whose ancestors are all read, built of classes, refinements, literals and
    /// parameters, of a method whose owner has no type parameters: the attempt's substitution
    /// then binds only the method's own parameters, keeps the result a subclass of the same
    /// classes and brings nothing blocked in (whose `blocked_conversion` note the attempt
    /// would leave). Every member of the union is a class only its subclasses conform to
    /// (`exclusive_member`), and none is among the result class's base classes.
    fn conversion_result_excluded(&mut self, g: SymId, to: TypeId) -> bool {
        let mut members = Vec::new();
        if !self.exclusive_union_members(to, true, &mut members) {
            return false;
        }
        if let Owner::Class(c) = self.syms.sym(g).owner {
            if !self.syms.class(c).tparams.is_empty() {
                return false;
            }
        }
        let Some(shape) = self.shape_of(g) else { return false };
        if !self.closed_result(shape.to) {
            return false;
        }
        let mut head = self.deref(shape.to);
        while let Type::Refined(parent, _) = self.types.get(head) {
            head = self.deref(parent);
        }
        let Type::Class(a, _) = self.types.get(head) else { return false };
        if !self.nominal_class(a) || self.tuple_like(a) {
            return false;
        }
        self.complete_class(a);
        let n = self.syms.class(a).base_types.len();
        for i in 0..n {
            let k = self.syms.class(a).base_types[i].0;
            if self.is_java_placeholder(k) || members.contains(&k) {
                return false;
            }
        }
        !members.contains(&a)
    }

    /// The members of a union, each a class only its subclasses conform to; false when `t` is
    /// no union (`top`) or a member is of another kind.
    fn exclusive_union_members(&mut self, t: TypeId, top: bool, out: &mut Vec<ClassId>) -> bool {
        let mut t = self.deref(t);
        while let Type::Refined(parent, _) = self.types.get(t) {
            t = self.deref(parent);
        }
        match self.types.get(t) {
            Type::Union(a, b) => self.exclusive_union_members(a, false, out) && self.exclusive_union_members(b, false, out),
            Type::Class(c, _) if !top && self.exclusive_member(c) => {
                out.push(c);
                true
            }
            _ => false,
        }
    }

    /// A class that a class conforms to only by deriving from it: none of the classes with a
    /// rule of their own in `is_sub` on the right (the tops, `Null`, `Product`, `Equals`,
    /// `Singleton`, `reflect.Enum`, `Matchable`, `AnyKind`, an opaque type, a type-level
    /// operation, the tuple classes and the `Tuple` and `NonEmptyTuple` traits, which tuples
    /// conform to by their shape), and no Java class nothing has read yet.
    fn exclusive_member(&self, c: ClassId) -> bool {
        let special_type = |t: TypeId| matches!(self.types.get(t), Type::Class(k, _) if k == c);
        !(self.special_head(c)
            || self.tuple_like(c)
            || Some(c) == self.b.tuple_trait
            || Some(c) == self.b.non_empty_tuple
            || self.is_java_placeholder(c)
            || special_type(self.b.t_singleton)
            || special_type(self.b.t_equals)
            || special_type(self.b.t_product)
            || special_type(self.b.t_enum)
            || self.is_scala_class(c, "Matchable")
            || self.is_scala_class(c, "AnyKind"))
    }

    /// A declared type built of classes (no type-level operation), refinements of them,
    /// literals and parameters only: no path, alias, match type or abstract member, which a
    /// site could read otherwise.
    fn closed_result(&self, t: TypeId) -> bool {
        match self.types.get(t) {
            Type::Class(c, args) => !(args != EMPTY_LIST && self.types.is_op_class(c)) && self.types.items(args).iter().all(|&a| self.closed_result(a)),
            Type::Refined(parent, r) => {
                self.closed_result(parent)
                    && match self.types.refinement(r) {
                        Refinement::Alias(_, rhs) => self.closed_result(rhs),
                        Refinement::Bounds(_, lo, hi) => self.closed_result(lo) && self.closed_result(hi),
                        Refinement::Val(_, _, ty) => self.closed_result(ty),
                        Refinement::Term(..) => false,
                    }
            }
            Type::Param(_) | Type::Lit(_) => true,
            Type::Union(a, b) | Type::Inter(a, b) => self.closed_result(a) && self.closed_result(b),
            _ => false,
        }
    }

    /// Whether a search of `target` may read or keep a memoised result, or why not: the
    /// target has open variables (a variable stands for its bounds, which change), a local
    /// given in scope or the value a local import reads from has one in its type (a candidate
    /// attempt under way may have bound it for the moment), a quote is open (an inline given is then expanded when the quote
    /// ends), or a pattern's GADT bounds are in force (a candidate's type is read through
    /// them). Local givens and imports are candidates of the lexical levels like any other,
    /// and a tree that names one is not kept.
    fn memo_applicable(&mut self, target: TypeId) -> Memo {
        if self.types.has_vars(target) {
            return Memo::Vars;
        }
        for i in 0..self.env.frames.len() {
            let Frame::Locals { givens, .. } = &self.env.frames[i] else { continue };
            for j in 0..givens.len() {
                let g = match &self.env.frames[i] {
                    Frame::Locals { givens, .. } => givens[j],
                    _ => unreachable!(),
                };
                if self.candidate_type_has_vars(g) {
                    return Memo::VarsLocal;
                }
            }
        }
        for i in 0..self.env.imports.len() {
            let value = match self.env.imports[i].target {
                ImportTarget::ValueAll(v) | ImportTarget::ValueMember(v, _) | ImportTarget::ValueGivens(v) => v,
                _ => continue,
            };
            if self.receiver_type_has_vars(GivenScope::Value(value)) {
                return Memo::VarsImport;
            }
        }
        if self.quote.level > 0 {
            Memo::Quote
        } else if !self.gadt.is_empty() {
            Memo::Gadt
        } else {
            Memo::Hit
        }
    }

    /// Whether the stable value an imported candidate is reached through has an inference
    /// variable in its type (a lambda's parameter while the enclosing application is
    /// inferred), which a candidate attempt under way may have bound for the moment; the
    /// candidate's type is read through that value's.
    fn receiver_type_has_vars(&mut self, scope: GivenScope) -> bool {
        let GivenScope::Value(v) = scope else { return false };
        let ty = self.import_value_ret(v);
        let ty = self.zonk(ty);
        self.types.has_vars(ty)
    }

    /// Whether a candidate's declared type mentions an inference variable, which a candidate
    /// attempt under way may have bound: a local's type can (a context function's parameter
    /// while the enclosing application is inferred), a jar's never.
    fn candidate_type_has_vars(&mut self, g: SymId) -> bool {
        let info = self.syms.sym(g);
        if !matches!(info.owner, Owner::Local) && info.def.is_none() {
            return false;
        }
        let sig = self.sig_arc(g);
        self.types.has_vars(sig.ret) || sig.clauses.iter().any(|cl| cl.params.iter().any(|p| self.types.has_vars(p.ty)))
    }

    /// The lexical context a search runs in, as words left in `given_context`: the frames
    /// with their local givens, the local imports, the alias given being defined, the class
    /// whose parent arguments are typed, the transparent opaque types and the file's language
    /// settings, each record tagged and its lists length-prefixed, so that two contexts read
    /// alike only when they are alike. Two searches of one target in one file
    /// with the same words read the same candidates, so a result kept under them answers the
    /// next search without the walk of the lexical levels (`given_fast`, keyed by a hash of
    /// the words and compared in full on a hit).
    fn context_key(&mut self) -> (FileId, u64) {
        let mut words = std::mem::take(&mut self.given_context);
        words.clear();
        for f in &self.env.frames {
            match f {
                Frame::Class(c) => words.extend([1, c.0]),
                Frame::Locals { givens, owner, .. } => {
                    words.extend([2, *owner, givens.len() as u32]);
                    words.extend(givens.iter().map(|g| g.0));
                }
            }
        }
        for imp in &self.env.imports {
            let (tag, id, n) = match imp.target {
                ImportTarget::PkgMember(p, n) => (0, p.0, n.0),
                ImportTarget::PkgAll(p) => (1, p.0, 0),
                ImportTarget::ClassMember(c, n) => (2, c.0, n.0),
                ImportTarget::ClassAll(c) => (3, c.0, 0),
                ImportTarget::PkgGivens(p) => (4, p.0, 0),
                ImportTarget::ClassGivens(c) => (5, c.0, 0),
                ImportTarget::ValueAll(v) => (6, v.0, 0),
                ImportTarget::ValueMember(v, n) => (7, v.0, n.0),
                ImportTarget::ValueGivens(v) => (8, v.0, 0),
                ImportTarget::UnimportPredef => (9, 0, 0),
                ImportTarget::Unresolved => (10, 0, 0),
            };
            words.extend([3, imp.depth, imp.stmt, imp.name.map_or(u32::MAX, |n| n.0), imp.hidden.start, imp.hidden.len, imp.bound.map_or(u32::MAX, |t| t.0), imp.unimports_predef.unwrap_or(u32::MAX), tag, id, n]);
        }
        words.extend([4, self.defining.map_or(u32::MAX, |d| d.0), self.parent_args_of.map_or(u32::MAX, |c| c.0), self.transparent.len() as u32]);
        words.extend(self.transparent.iter().map(|c| c.0));
        // The file's language settings, which the synthesis of a `CanEqual` reads.
        let (strict, future) = (self.strict_equality(), self.cur_ast().source_future);
        words.extend([5, strict as u32, future as u32]);
        let h = words.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, &w| (h ^ w as u64).wrapping_mul(0x0100_0000_01b3));
        self.given_context = words;
        (self.env.file, h)
    }

    /// Whether a request of the kept tree could be answered by an open given's instance
    /// instead of a search (`recursive_ref`): the open givens it walks are those above the
    /// innermost one opened for a by-name request, and every open given when the tree holds
    /// a by-name request itself; one of them answers when its target's class derives from a
    /// class a search of the tree wanted. The tree kept from a search where that happened
    /// names the knot's local, so it was not kept.
    fn knot_possible(&mut self, entry: &GivenResult) -> bool {
        let n = self.given_stack.len();
        let boundary = (0..n).rev().find(|&i| self.given_stack[i].byname);
        let reachable = match (entry.byname, boundary) {
            (true, _) => 0..n,
            (false, Some(k)) => 0..k,
            (false, None) => return false,
        };
        // An open target answers through any class part of its intersections and
        // refinements; one with a part of any other shape (a variable, a union, an alias)
        // is taken to answer any request.
        let mut classes = Vec::new();
        for i in reachable {
            classes.clear();
            let open_target = self.given_stack[i].target;
            if !self.knot_parts(open_target, &mut classes) {
                return true;
            }
            for c in entry.consulted.iter() {
                let Wanted::Class(w) = *c else { return true };
                for k in 0..classes.len() {
                    let oc = classes[k];
                    if oc == w || self.derives_from(oc, w) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// The class parts of an open target, through its intersections and refinements; false
    /// when a part has another shape.
    fn knot_parts(&mut self, t: TypeId, out: &mut Vec<ClassId>) -> bool {
        let t = self.deref(t);
        match self.types.get(t) {
            Type::Inter(a, b) => self.knot_parts(a, out) && self.knot_parts(b, out),
            Type::Refined(parent, _) => self.knot_parts(parent, out),
            Type::Class(c, _) => {
                if !out.contains(&c) {
                    out.push(c);
                }
                true
            }
            _ => false,
        }
    }

    fn tree_marks(&mut self, depth: u32) -> TreeMarks {
        let deepest = std::mem::replace(&mut self.given_tree.deepest, depth);
        TreeMarks {
            consulted: self.given_consulted.len(),
            opened: self.given_opened.len(),
            in_progress: self.given_tree.in_progress,
            restricted: self.given_tree.restricted,
            deepest,
            byname_requests: self.given_tree.byname_requests,
            budget: self.implicit_budget,
            failed: self.failed_givens.len(),
            diags: self.diags.items.len(),
            winners: self.unused.given_winners.len(),
        }
    }

    /// A read of in-progress state by the search under way (`InProgress`): every search
    /// under way is marked, and keeps nothing.
    pub(super) fn read_in_progress(&mut self, reader: InProgress) {
        self.given_tree.in_progress += 1;
        if self.profile.on {
            self.profile.given.in_progress[reader as usize] += 1;
        }
    }

    /// The memoised result of `target` a search at `depth` can use, if there is one: the
    /// result kept under this lexical context, where none of the readers of in-progress state
    /// could read differently than its search did. A use records what the kept search wanted
    /// and opened, and charges the attempts it spent, as if the search had run.
    fn memo_lookup(&mut self, target: TypeId, depth: u32, prof: Option<usize>) -> Option<Pick> {
        let (file, context) = self.context_key();
        if let Some(p) = prof {
            self.profile.set_ctx(p, (file, context));
        }
        let entry = self.given_fast.get(&(target, file, context)).filter(|(_, words)| **words == *self.given_context).map(|(e, _)| e.clone());
        let Some(entry) = entry else {
            if let Some(p) = prof {
                self.profile.set_memo(p, Memo::Miss);
            }
            return None;
        };
        if let Err(reason) = self.memo_in_progress_fits(&entry, depth) {
            if let Some(p) = prof {
                self.profile.set_memo(p, reason);
            }
            return None;
        }
        self.implicit_budget -= entry.tries;
        // An enclosing search records what the tree wanted and opened as if it had run.
        if depth > 0 {
            self.given_consulted.extend(entry.consulted.iter().copied());
            self.given_opened.extend(entry.opened.iter().copied());
            self.given_tree.deepest = self.given_tree.deepest.max(depth + entry.depth);
            self.given_tree.byname_requests += entry.byname as u32;
        }
        if let Some(p) = prof {
            self.profile.set_memo(p, Memo::Hit);
        }
        // What the kept tree took, taken again.
        if !entry.winners.is_empty() {
            self.unused.given_winners.extend(entry.winners.iter().copied());
        }
        Some(match entry.expr {
            Some(expr) => Pick::Found(expr, entry.ty),
            None => Pick::NoMatch,
        })
    }

    /// The part of `memo_fits` that reads the state of what is in progress, which a context
    /// key does not cover: the refusals of the `InProgress` readers.
    fn memo_in_progress_fits(&mut self, entry: &GivenResult, depth: u32) -> Result<(), Memo> {
        if depth + entry.depth >= MAX_IMPLICIT_DEPTH {
            return Err(Memo::Depth);
        }
        if entry.tries > self.implicit_budget {
            return Err(Memo::Budget);
        }
        if !self.function_targets.is_empty() {
            return Err(Memo::ConversionGuard);
        }
        for &g in entry.opened.iter() {
            for i in 0..self.given_stack.len() {
                let open = self.given_stack[i].given;
                if open == g || self.same_signature(g, open) {
                    return Err(Memo::Opened);
                }
            }
        }
        if self.knot_possible(entry) {
            return Err(Memo::ByName);
        }
        Ok(())
    }

    /// Keeps a search's result when a later search of the target can stand on it, and says
    /// why not otherwise: no search of the tree read the state of what is in progress
    /// (`InProgress`, the list), no candidate of an implicit scope has a restricted
    /// visibility, nothing was reported, the result is not ambiguous, a search that found
    /// nothing added no failure note, and a tree found has no open variables in its type and
    /// binds no local, so that a copy can stand for it elsewhere.
    fn memo_keep(&mut self, target: TypeId, result: &Pick, marks: &TreeMarks, byname: bool) -> Keep {
        let tree = &self.given_tree;
        let keep = if tree.in_progress != marks.in_progress {
            Keep::InProgress
        } else if tree.restricted != marks.restricted {
            Keep::Restricted
        } else if self.diags.items.len() != marks.diags {
            Keep::Reported
        } else {
            match result {
                Pick::Ambiguous => Keep::Ambiguous,
                Pick::NoMatch if self.failed_givens.len() != marks.failed => Keep::FailureNotes,
                Pick::NoMatch => Keep::KeptNoMatch,
                Pick::Found(_, ty) if self.types.has_vars(*ty) => Keep::VarsInResult,
                Pick::Found(expr, _) if !self.shareable_tree(*expr) => Keep::NotShareable,
                Pick::Found(..) => Keep::Kept,
            }
        };
        if !matches!(keep, Keep::Kept | Keep::KeptNoMatch) {
            return keep;
        }
        let (expr, ty) = match result {
            Pick::Found(expr, ty) => (Some(*expr), *ty),
            _ => (None, target),
        };
        let depth = self.given_tree.deepest - (self.implicit_depth - 1);
        let consulted = self.given_consulted[marks.consulted..].into();
        let opened = self.given_opened[marks.opened..].into();
        let byname = byname || self.given_tree.byname_requests != marks.byname_requests;
        let tries = marks.budget.saturating_sub(self.implicit_budget);
        let winners = match self.unused.on() {
            true => self.unused.given_winners.get(marks.winners..).unwrap_or(&[]).into(),
            false => Box::default(),
        };
        let entry = Arc::new(GivenResult { expr, ty, consulted, opened, depth, byname, tries, winners });
        let (file, context) = self.context_key();
        let words = self.given_context.clone().into_boxed_slice();
        self.given_fast.insert((target, file, context), (entry, words));
        keep
    }

    /// What asked for the search under way, for the profile.
    fn given_origin(&self) -> super::profile::Origin {
        use super::profile::Origin;
        if self.profile.summon != Origin::Plain {
            self.profile.summon
        } else if !self.inline.sites.is_empty() {
            Origin::Inline
        } else {
            Origin::Plain
        }
    }

    /// A tree a copy can stand for elsewhere: references, selections, applications and
    /// constructions over them, with no local bound or named in it.
    fn shareable_tree(&self, e: TExprId) -> bool {
        let list = |l: ListRef| self.prog.expr_list(l).iter().all(|&x| self.shareable_tree(x));
        match self.prog.expr(e) {
            TExpr::Int(_) | TExpr::Long(_) | TExpr::Double(_) | TExpr::Bool(_) | TExpr::Char(_) | TExpr::Str(_) | TExpr::Unit | TExpr::Null | TExpr::Static(_) | TExpr::Module(_) | TExpr::ClassOf(_) => true,
            TExpr::Field(r, _) | TExpr::TypeTest(r, _) | TExpr::Cast(r, ..) => self.shareable_tree(r),
            TExpr::CallStatic(_, args) | TExpr::New(_, args) | TExpr::NewVia(_, args) | TExpr::SeqLit(args) | TExpr::ArrayLit(args) => list(args),
            TExpr::CallMethod(r, _, args) => self.shareable_tree(r) && list(args),
            _ => false,
        }
    }

    /// Nearer scopes first: the enclosing blocks and classes from the inside out, then the
    /// file's imports with the innermost package clause, the outer clauses, and last the
    /// implicit scope of the type. The first of them with a matching given decides. As in
    /// scalac, an alias given is not in scope in its own right-hand side, though the implicit
    /// scope of the type still has it.
    fn search_given(&mut self, target: TypeId, target_class: Option<ClassId>, span: Span) -> Pick {
        let wanted = target_class.map_or(Wanted::AnyClass, Wanted::Class);
        let mut buf = LevelBuf::default();
        // The names of the matching candidates of the nearer levels, which hide those of their
        // name further out (`combineEligibles`); not the implicit scope's.
        let mut hidden: Vec<Name> = Vec::new();
        let step = self.step(Step::Levels);
        let levels = self.given_levels();
        self.step_end(step, Step::Levels);
        // Nothing is settled under a pattern's GADT bounds either: the head rejection reads
        // a type argument through them.
        let memoised = self.transparent.is_empty() && !self.types.has_vars(target) && self.gadt.is_empty();
        let mut heads = None;
        for level in (0..levels.count()).rev() {
            buf.clear();
            let step = self.step(Step::Levels);
            self.level_candidates(&levels, level, target, wanted, &mut buf);
            let ns = self.step_end(step, Step::Levels);
            if self.profile.on {
                let kind = if level == 0 {
                    LevelKind::ImplicitScope
                } else if level < levels.n_outer() {
                    LevelKind::Predef
                } else if level < levels.packages.len() + levels.n_outer() {
                    LevelKind::Package
                } else {
                    LevelKind::Frame
                };
                self.profile.given.parts.levels[kind as usize].add(ns);
            }
            self.profile.given_found(buf.cands.len());
            if buf.cands.is_empty() {
                continue;
            }
            let heads = heads.get_or_insert_with(|| self.target_heads(target));
            let eligible = self.level_eligible(&levels, level, target, target_class, &buf, &mut hidden, memoised, heads);
            if eligible.is_empty() {
                continue;
            }
            match self.pick_given(&buf.cands, eligible, target, span) {
                Pick::NoMatch => {}
                pick => {
                    self.given_consulted.push(wanted);
                    return pick;
                }
            }
        }
        self.given_consulted.push(wanted);
        Pick::NoMatch
    }

    /// A level's eligible candidates for `target` (`ContextualImplicits.eligible`), indices of
    /// `buf.cands`: those that match it (`fit_decision`, scalac's `filterMatching`), but the
    /// alias given being defined and those of a name a matching candidate of a nearer level has
    /// (`hidden`), composed (`compose_level`). The matching ones' names join `hidden` for the
    /// levels further out, the implicit scope's apart.
    #[allow(clippy::too_many_arguments)]
    fn level_eligible(
        &mut self,
        levels: &Levels,
        level: usize,
        target: TypeId,
        target_class: Option<ClassId>,
        buf: &LevelBuf,
        hidden: &mut Vec<Name>,
        memoised: bool,
        heads: &mut TargetHeads,
    ) -> Vec<u32> {
        let defining = levels.defining.filter(|_| level > 0);
        // The given being defined is not in scope in its own right-hand side, nor what it
        // overrides: a trait's given its class implements, which the class's `this` reaches as
        // the definition itself.
        // The implicit scope's candidates are its own eligible list, which no name of the
        // context's hides (`searchImplicit`'s `implicitScope(wildProto).eligible`).
        let hide = level > 0 && !hidden.is_empty();
        let mut fits: Vec<u32> = Vec::new();
        for i in 0..buf.cands.len() as u32 {
            let given = buf.cands[i as usize];
            if defining.is_some_and(|d| self.defined_here(d, given.0)) || hide && hidden.contains(&self.level_name(buf, i)) {
                continue;
            }
            if self.fit_decision(given, target, target_class, memoised, heads) {
                fits.push(i);
            }
        }
        self.profile.given.fits += fits.len() as u64;
        if fits.is_empty() {
            return fits;
        }
        if level > 0 {
            for &i in &fits {
                let n = self.level_name(buf, i);
                if !hidden.contains(&n) {
                    hidden.push(n);
                }
            }
        }
        self.compose_level(buf, fits)
    }

    /// Whether `g` is the given `d` being defined or one `d` overrides: a member of a base class
    /// of `d`'s class under its name.
    fn defined_here(&self, d: SymId, g: SymId) -> bool {
        if g == d {
            return true;
        }
        let (name, owner) = (self.syms.sym(d).name, self.syms.sym(d).owner);
        let info = self.syms.sym(g);
        info.name == name && matches!((owner, info.owner), (Owner::Class(k), Owner::Class(b)) if k != b && self.syms.class(k).base_types.iter().any(|&(x, _)| x == b))
    }

    /// The levels a search walks from the innermost, as `search_given` documents them.
    fn given_levels(&mut self) -> Levels {
        let packages = self.pkg_chain();
        // `scala.Predef` of a classpath is a level of its own outside the package clauses.
        let predef = self.loaded.as_ref().and_then(|l| l.predef);
        Levels { frames: self.frame_levels(), packages, predef, defining: self.defining }
    }

    /// The candidates of one level, its contexts outermost first: level 0 is the implicit scope
    /// of the target.
    #[inline(always)]
    fn level_candidates(&mut self, levels: &Levels, level: usize, target: TypeId, wanted: Wanted, buf: &mut LevelBuf) {
        let n_packages = levels.packages.len();
        let n_outer = levels.n_outer();
        if level == 0 {
            self.implicit_scope_givens(target, wanted, &mut buf.cands);
            buf.close(Prec::Def);
        } else if level < n_outer {
            if let Some(index) = self.class_given_index(levels.predef.unwrap()) {
                index.select(wanted, GivenScope::Lexical, &mut buf.cands);
            }
            buf.close(Prec::Def);
        } else if level < n_packages + n_outer {
            let i = n_packages + n_outer - 1 - level;
            self.package_level(levels.packages[i], i == 0, wanted, buf);
        } else {
            let k = level - n_packages - n_outer;
            match &levels.frames {
                FrameLevels::PerFrame(_) => {
                    self.frame_own_givens(k, wanted, &mut buf.cands);
                    buf.close(Prec::Def);
                }
                FrameLevels::Composed { nodes, ends, selectors } => {
                    let start = if k == 0 { 0 } else { ends[k - 1] as usize };
                    for &node in &nodes[start..ends[k] as usize] {
                        match node {
                            FrameNode::Own(f) => {
                                self.frame_own_givens(f as usize, wanted, &mut buf.cands);
                                buf.close(Prec::Def);
                            }
                            FrameNode::Stmt(from, to, prec) => {
                                let stmt: Vec<ResolvedImport> = selectors[from as usize..to as usize].iter().map(|&i| self.env.imports[i as usize]).collect();
                                self.stmt_givens(&stmt, wanted, buf, true);
                                buf.close(prec);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Whether the candidate's declared result has a class that derives from the target's.
    /// A part of the result with no class of its own (a type parameter, an abstract type
    /// member, a stuck alias) can be any class under the site a class or trait's member is
    /// reached through (`given g: A` of `G[A]` under `G[TC]` provides `TC`), so such a
    /// candidate passes to the shape pass.
    fn may_provide(&mut self, g: SymId, target_class: Option<ClassId>) -> bool {
        let Some(tc) = target_class else { return true };
        let ret = self.sig_of(g).ret;
        let mut classes = Vec::new();
        let unknown = !self.inter_classes_known(ret, &mut classes);
        if unknown && !self.settled_owner(g) {
            return true;
        }
        classes.into_iter().any(|gc| {
            self.complete_class(gc);
            self.syms.class(gc).base_types.iter().any(|&(b, _)| b == tc)
        })
    }

    /// The classes of the parts of an intersection, or the class of the type itself, and whether
    /// every part of the type had a class. An erroneous declaration's type is known to provide
    /// nothing.
    fn inter_classes_known(&mut self, t: TypeId, out: &mut Vec<ClassId>) -> bool {
        let t = self.deref(t);
        if t == ERROR {
            return true;
        }
        match self.types.get(t) {
            Type::Inter(a, b) => {
                let a_known = self.inter_classes_known(a, out);
                self.inter_classes_known(b, out) && a_known
            }
            _ => match self.class_of(t) {
                Some(c) => {
                    if !out.contains(&c) {
                        out.push(c);
                    }
                    true
                }
                None => false,
            },
        }
    }

    /// Whether a member of a class or trait reached through an object of the implicit scope is
    /// read at a site that object alone fixes, as `open_given_sig` finds it: the object is
    /// static (in packages and objects only, so one class id is one object), its export table
    /// is settled, and it exports the member or derives the trait. The site's substitution
    /// and its paths are then the object's own wherever the search runs.
    fn module_site_fixed(&mut self, (g, scope): GivenRef) -> bool {
        let GivenScope::Module(m) = scope else { return false };
        let Owner::Class(c) = self.syms.sym(g).owner else { return false };
        if !self.static_object(m) || !self.exports_settled(ExportOwner::Class(m)) {
            return false;
        }
        self.exports_of(m).map_or(false, |e| e.via.contains_key(&g)) || self.derives_from(m, c)
    }

    /// An object whose enclosing classes are all objects, up to a package.
    fn static_object(&self, m: ClassId) -> bool {
        let mut c = m;
        loop {
            let info = self.syms.class(c);
            if info.kind != ClassKind::Object || info.inner_object.is_some() || info.local_module.is_some() {
                return false;
            }
            match info.owner {
                Owner::Package(_) => return true,
                Owner::Class(o) => c = o,
                Owner::Local => return false,
            }
        }
    }

    /// Whether a candidate's fit against a target can be settled wherever the two meet: for a
    /// member of an object or of a package alone. A member of a class or trait is reached
    /// through an object, class or value that instantiates the owner's type parameters and
    /// fixes its type members (`G.this.A`), and its declared type reaches them through shapes
    /// no syntactic walk covers in full (an application's arguments, an alias kept by name, a
    /// match type, a refinement, a bound), so under another site it is another type; a
    /// local's type is its scope's.
    fn settled_owner(&self, g: SymId) -> bool {
        match self.syms.sym(g).owner {
            Owner::Package(_) => true,
            Owner::Class(c) => self.syms.class(c).kind == ClassKind::Object,
            Owner::Local => false,
        }
    }

    /// Whether a candidate fits the target by type. The class test, the head-class rejection
    /// and the shape pass all read the candidate's signature and the target alone, so the
    /// answer is memoised per candidate and target without open variables while no opaque
    /// type is transparent and no GADT bounds are in force (`given_fits`), unless the
    /// candidate's type reads the scope
    /// (`fits_target` says so), mentions an inference variable, or belongs to a member of a
    /// class or trait (`settled_owner`: the three tests read the declared type, which the
    /// site the member is reached through makes another type) reached otherwise than through
    /// an object of the implicit scope that fixes its site (`module_site_fixed`: the key holds
    /// that object).
    fn fit_decision(&mut self, given: GivenRef, target: TypeId, target_class: Option<ClassId>, memoised: bool, heads: &mut TargetHeads) -> bool {
        let step = self.step(Step::FitShapeYes);
        let (fits, how, why) = self.fit_decision_how(given, target, target_class, memoised, heads);
        if step.taken() {
            self.profile.fit_decided(step, how, why);
        }
        fits
    }

    /// `fit_decision`, with the part of the profile the decision's time goes to and, for a
    /// shape check run, why the fit memo did not answer it.
    fn fit_decision_how(&mut self, given: GivenRef, target: TypeId, target_class: Option<ClassId>, memoised: bool, heads: &mut TargetHeads) -> (bool, Step, Option<FitWhy>) {
        let vars = self.candidate_type_has_vars(given.0) || self.receiver_type_has_vars(given.1);
        if vars {
            self.read_in_progress(InProgress::ProvisionalBinding);
        }
        if self.syms.sym(given.0).state() == Completion::InProgress {
            self.read_in_progress(InProgress::SignatureInProgress);
        }
        let memo_key = (memoised && !vars && (self.settled_owner(given.0) || self.module_site_fixed(given))).then(|| (given.0, given.1.memo_code(), target));
        if let Some(&fits) = memo_key.and_then(|k| self.given_fits.get(&k)) {
            self.profile.given.shape_cached += self.profile.on as u64;
            debug_assert!(
                self.settled_owner(given.0) || self.fit_computed(given, target, target_class, heads).0 == fits,
                "the kept fit of {} reached through its implicit scope's object",
                self.sym_path(given.0)
            );
            return (fits, Step::FitMemo, None);
        }
        let (fits, context_free, how) = self.fit_computed(given, target, target_class, heads);
        if let (Some(k), true) = (memo_key, context_free) {
            self.given_fits.insert(k, fits);
        }
        let why = self.profile.on.then(|| {
            if !memoised {
                FitWhy::NotMemoised
            } else if vars {
                FitWhy::VarCandidate
            } else if memo_key.is_none() {
                FitWhy::UnsettledOwner
            } else if context_free {
                FitWhy::FirstKept
            } else {
                FitWhy::FirstUnkept
            }
        });
        (fits, how, why)
    }

    /// The fit decision itself, with whether its answer is context-free and the part it was
    /// made by: the conversion pre-filter, the class test, the head rejection or the shape pass.
    fn fit_computed(&mut self, given: GivenRef, target: TypeId, target_class: Option<ClassId>, heads: &mut TargetHeads) -> (bool, bool, Step) {
        let mut how = Step::FitConversion;
        let (fits, context_free) = if self.syms.sym(given.0).kind == SymKind::Def && self.is_conversion_candidate(given.0) {
            // The target is looked at before the method's signature, which a jar decodes.
            self.profile.try_start(given.0);
            let fits = target_class == Some(self.function_class(1))
                && !self.has_unbounded_var(target)
                && self.is_conversion_def(given.0)
                && self.conversion_method_may_fit(given.0, target);
            self.profile.try_end(!fits, true);
            (fits, !self.types.has_vars(target))
        } else if !self.may_provide_with(given.0, target_class, heads) {
            how = Step::FitClass;
            (false, true)
        } else if self.head_rejects_with(given.0, target, heads) {
            self.profile.given.head_rejected += 1;
            how = Step::FitHead;
            (false, true)
        } else {
            let mark = self.snapshot();
            if self.given_trace() && self.types.has_vars(target) {
                let mut vars = Vec::new();
                self.collect_vars(target, &mut vars);
                let bounds: Vec<String> = vars.iter().map(|&v| {
                    let (lower, upper) = (self.tvars[v].lower.clone(), self.tvars[v].upper.clone());
                    let l: Vec<String> = lower.iter().map(|&t| self.show(t)).collect();
                    let u: Vec<String> = upper.iter().map(|&t| self.show(t)).collect();
                    format!("?{} >: [{}] <: [{}]", v.0, l.join(", "), u.join(", "))
                }).collect();
                let (n, t) = (self.name_str(self.syms.sym(given.0).name), self.show(target));
                eprintln!("shape {} vs {} with {}", n, t, bounds.join("; "));
            }
            self.profile.try_start(given.0);
            let (fits, context_free) = self.fits_target(given, target);
            self.profile.try_end(!fits, true);
            self.profile.given.shape_unkept += (self.profile.on && !context_free) as u64;
            self.rollback(mark);
            how = if fits { Step::FitShapeYes } else { Step::FitShapeNo };
            (fits, context_free)
        };
        (fits, context_free, how)
    }

    /// Instantiates the candidates that provide `target`, in two passes. The shape pass opens
    /// every candidate's signature and matches its type against the target without its using
    /// clauses; the candidates that fit are then instantiated in full, the preferred ones first
    /// (`preference_order`), and a candidate that a success found before it beats outright is
    /// left untried, as in scalac's `rank`. That changes no pick: `most_specific_given` would
    /// have made such a candidate lose to that success, and the winner beats every success, so
    /// it beats the untried candidates through them; a level with imported candidates keeps
    /// trying them all, since there a same-named candidate hides those of lower precedence
    /// even when it fails. The bindings of a success stay in place until another candidate has
    /// to be tried, so the usual case of a single candidate is instantiated once. A candidate
    /// whose own using parameter was ambiguous makes the level ambiguous too unless the best
    /// candidate beats it outright. In a contextual level (`shadowed` given) a candidate that
    /// fits the target hides the same-named candidates of lower binding precedence, and of
    /// the outer levels if this one fails.
    fn pick_given(&mut self, candidates: &[GivenRef], eligible: Vec<u32>, target: TypeId, span: Span) -> Pick {
        let mut fitting: Vec<usize> = eligible.into_iter().map(|i| i as usize).collect();
        if fitting.len() > 1 {
            let step = self.step(Step::Order);
            self.preference_order(candidates, &mut fitting);
            self.step_end(step, Step::Order);
        }
        // dotc's `isCoherent`: a `CanEqual` search takes the first candidate that applies, in
        // order, unless a candidate before it failed with a nested ambiguity it does not beat
        // (the companion's `canEqualOption` and `canEqualOptions` both fit an `Option[T]`
        // against a `Some[T]`; the first is taken).
        let coherent = self.b.can_equal.is_some() && self.class_of(target) == self.b.can_equal;
        let mut found: Vec<Success> = Vec::new();
        let mut best_so_far: Option<usize> = None;
        let mut ambiguous: Vec<SymId> = Vec::new();
        let mut in_place: Option<super::state::Mark> = None;
        for &i in &fitting {
            let given = candidates[i];
            if let Some(b) = best_so_far {
                if self.compare_givens(found[b].given.0, given.0) > 0 {
                    self.profile.given.pruned += 1;
                    continue;
                }
            }
            let step = self.step(Step::TryOk);
            if let (Some(mark), Some(last)) = (in_place.take(), found.last_mut()) {
                last.undone = Some(self.set_aside(mark));
            }
            // Each candidate is an attempt of its own: what it
            // wrote goes with it where it fails, and stays aside where another wins.
            let mark = self.attempt();
            let captured = self.capture_mark();
            let winners_mark = self.unused.given_winners.len();
            self.profile.try_start(given.0);
            let attempt = self.try_given(given, Some(target), span, true);
            // What a candidate's nested searches took goes with it: kept by a success, dropped
            // by a failure.
            let winners = if self.unused.on() || self.unused.defs_on() { self.unused.given_winners.split_off(winners_mark) } else { Vec::new() };
            let receiver = self.unused.receiver.take();
            self.profile.try_end(!matches!(attempt, Attempt::Ok(..)), false);
            if !matches!(attempt, Attempt::Ok(..)) && captured.is_some() {
                self.capture_drop_since(captured);
            }
            let how = match (step.taken(), &attempt) {
                (false, _) | (true, Attempt::Ok(..)) => Step::TryOk,
                (true, Attempt::Mismatch) => Step::TryMismatch,
                (true, Attempt::Incomplete) => Step::TryIncomplete,
                (true, Attempt::Ambiguous) => Step::TryAmbiguous,
            };
            match attempt {
                Attempt::Ok(expr, ty) => {
                    if coherent {
                        let earlier: Vec<SymId> = ambiguous.clone();
                        if !earlier.iter().all(|&a| self.compare_givens(given.0, a) > 0) {
                            self.retract(mark);
                            self.capture_drop_since(captured);
                            self.step_end(step, how);
                            continue;
                        }
                    }
                    found.push(Success { given, expr, ty, undone: None, winners, receiver });
                    // Open until another candidate sets it aside, or it wins or loses.
                    in_place = Some(mark);
                    let s = found.len() - 1;
                    best_so_far = match best_so_far {
                        Some(b) if self.compare_givens(given.0, found[b].given.0) <= 0 => Some(b),
                        _ => Some(s),
                    };
                    if coherent {
                        self.step_end(step, how);
                        break;
                    }
                }
                Attempt::Ambiguous => {
                    self.retract(mark);
                    ambiguous.push(given.0);
                }
                Attempt::Incomplete | Attempt::Mismatch => self.retract(mark),
            }
            self.step_end(step, how);
        }
        if found.is_empty() {
            if ambiguous.is_empty() {
                return Pick::NoMatch;
            }
            return Pick::Ambiguous;
        }
        let choose = self.step(Step::Choose);
        let mut best = if found.len() == 1 { Some(0) } else { self.most_specific_given(&found) };
        if best.is_none() && !self.transparent.is_empty() {
            if let (Some(mark), Some(last)) = (in_place.take(), found.last_mut()) {
                last.undone = Some(self.set_aside(mark));
            }
            best = self.given_for_opaque_type(&found, target, span);
        }
        if let Some(b) = best {
            let winner = found[b].given.0;
            if !ambiguous.iter().all(|&a| self.compare_givens(winner, a) > 0) {
                best = None;
            }
        }
        let Some(best) = best else {
            if let Some(mark) = in_place {
                self.retract(mark);
            }
            if self.given_ambiguity.is_none() {
                let names: Vec<String> =
                    found.iter().map(|s| self.name_str(self.syms.sym(s.given.0).name)).collect();
                let wanted = self.target_shape(target, &mut Vec::new());
                self.given_ambiguity = Some(format!(
                    "ambiguous given instances for {}: {}",
                    self.show(wanted),
                    names.join(", ")
                ));
            }
            self.step_end(choose, Step::Choose);
            return Pick::Ambiguous;
        };
        let winner = found.swap_remove(best);
        if let Some(undone) = winner.undone {
            if let Some(mark) = in_place {
                self.retract(mark);
            }
            self.restore(undone);
        } else if let Some(mark) = in_place {
            self.close(mark);
        }
        if self.unused.on() || self.unused.defs_on() {
            self.unused.given_winners.extend(winner.winners);
            // An inherited given read on an imported object (`B.n` of a trait's `n`) is that
            // object's, as CheckUnused's prefix test credits it.
            let given = match (winner.given.1, winner.receiver) {
                (GivenScope::Lexical, Some(m)) => (winner.given.0, GivenScope::Module(m)),
                _ => winner.given,
            };
            self.unused.given_winners.push(given);
        }
        self.step_end(choose, Step::Choose);
        Pick::Found(winner.expr, winner.ty)
    }

    /// Whether the given cannot fit the target because a type argument's head class differs
    /// where the target's class is invariant: `TC[Option[A]]` asked for `TC[String]`. Only two
    /// plain classes are compared, for which `is_sub`'s invariant rule (`is_same`) is false
    /// when they differ: not `Null`, `AnyRef`, `AnyVal`, `Product` or `Enum`, which `is_sub`
    /// treats specially, not two tuple types (a tuple class, the tuple cons, `EmptyTuple`),
    /// which conform to each other, not an opaque type or a type-level operation, and not while
    /// an opaque type's definition is visible. A Java class nothing has read yet is a class of its own like any other: a
    /// distinct class is no `is_same` whatever its parents. Anything else is left to the shape
    /// pass.
    fn head_rejects(&mut self, g: SymId, target: TypeId) -> bool {
        if !self.transparent.is_empty() {
            return false;
        }
        let target = self.deref(target);
        let Type::Class(tc, targs) = self.types.get(target) else { return false };
        let ret = self.sig_of(g).ret;
        let ret = self.deref(ret);
        let inter = matches!(self.types.get(ret), Type::Inter(..));
        let mut providing = 0;
        self.head_rejects_parts(ret, tc, targs, inter, &mut providing) && providing > 0
    }

    /// The target as `head_rejects` reads it, before any argument is.
    fn target_heads(&mut self, target: TypeId) -> TargetHeads {
        let t = self.deref(target);
        let class = match self.types.get(t) {
            Type::Class(tc, targs) => Some((tc, targs)),
            _ => None,
        };
        TargetHeads { class, args: Vec::new(), variances: None, last: None }
    }

    /// `head_rejects`, with the candidate's side read from `head_sigs` and the target's from
    /// `heads`, both what `head_rejects_at` computes for them; the answer is the same.
    fn head_rejects_with(&mut self, g: SymId, target: TypeId, heads: &mut TargetHeads) -> bool {
        if !self.transparent.is_empty() {
            return false;
        }
        if !self.gadt.is_empty() {
            return self.head_rejects(g, target);
        }
        let Some((tc, _)) = heads.class else { return false };
        let sig = match heads.last.take() {
            Some((last, sig)) if last == g => sig,
            _ => self.head_sig(g, tc),
        };
        let rejects = match &sig.head {
            HeadForm::Live => return self.head_rejects(g, target),
            HeadForm::Never => false,
            HeadForm::Parts { inter, parts } => {
                let mut providing = 0;
                let mut all = true;
                for args in parts.iter().flatten() {
                    providing += 1;
                    if !self.head_args_reject(args, heads, *inter) {
                        all = false;
                        break;
                    }
                }
                all && providing > 0
            }
        };
        debug_assert_eq!(rejects, self.head_rejects(g, target), "the head rejection of {} read from its kept form", self.sym_path(g));
        rejects
    }

    /// `head_rejects_at` over a kept part's arguments.
    fn head_args_reject(&mut self, args: &[HeadArg], heads: &mut TargetHeads, inter: bool) -> bool {
        let Some((tc, targs)) = heads.class else { return false };
        let n = self.types.items(targs).len().min(args.len());
        for (i, &arg) in args.iter().enumerate().take(n) {
            let HeadArg::Class { c: cy, special: special_y, tuple: tuple_y, nominal: nominal_y } = arg else { continue };
            let x = self.target_arg(heads, i);
            if matches!(x, TargetArg::Skip) {
                continue;
            }
            let variance = self.target_variance(heads, tc, i);
            match x {
                TargetArg::Var(v) => {
                    if (variance == 0 || !inter) && nominal_y && self.bounds_reject_nominal(v, cy, variance) {
                        return true;
                    }
                }
                TargetArg::Class { c: cx, special: special_x, tuple: tuple_x } if variance == 0 => {
                    if cx != cy && !special_x && !special_y && !(tuple_x && tuple_y) {
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }

    /// The target's argument `i` as `head_rejects_at` compares it, read on first use.
    fn target_arg(&mut self, heads: &mut TargetHeads, i: usize) -> TargetArg {
        if let Some(Some(arg)) = heads.args.get(i) {
            return *arg;
        }
        let Some((_, targs)) = heads.class else { return TargetArg::Skip };
        let x = self.types.items(targs)[i];
        let arg = if head_dependent(self, x) || self.types.is_wild(x) {
            TargetArg::Skip
        } else {
            let x = self.dealias(x);
            match self.types.get(x) {
                Type::Var(v) => TargetArg::Var(v),
                Type::Class(c, _) => TargetArg::Class { c, special: self.special_head(c), tuple: self.tuple_like(c) },
                _ => TargetArg::Other,
            }
        };
        if heads.args.len() <= i {
            heads.args.resize(i + 1, None);
        }
        heads.args[i] = Some(arg);
        arg
    }

    fn target_variance(&mut self, heads: &mut TargetHeads, tc: ClassId, i: usize) -> i8 {
        if heads.variances.is_none() {
            self.settle_class(tc);
            let variances: Box<[i8]> = self.syms.class(tc).tparams.iter().map(|&tp| self.syms.tparam(tp).variance).collect();
            heads.variances = Some(variances);
        }
        heads.variances.as_ref().expect("read above")[i]
    }

    /// `may_provide`, with the answer read from `head_sigs` where it is kept; the entry read is
    /// left in `heads` for `head_rejects_with` when the classes are the same.
    fn may_provide_with(&mut self, g: SymId, target_class: Option<ClassId>, heads: &mut TargetHeads) -> bool {
        let Some(tc) = target_class else { return true };
        if !self.transparent.is_empty() || !self.gadt.is_empty() {
            return self.may_provide(g, target_class);
        }
        let sig = self.head_sig(g, tc);
        let provides = sig.provides;
        if heads.class.map_or(false, |(c, _)| c == tc) {
            heads.last = Some((g, sig));
        }
        let Some(provides) = provides else { return self.may_provide(g, target_class) };
        debug_assert_eq!(provides, self.may_provide(g, target_class), "the class test of {} read from its kept form", self.sym_path(g));
        provides
    }

    /// The candidate's side of `may_provide` and `head_rejects` against the class `tc`, kept when it is settled:
    /// the candidate's signature complete, its result without inference variables, every
    /// class of it complete and none a Java class nothing has read yet.
    fn head_sig(&mut self, g: SymId, tc: ClassId) -> Arc<HeadSig> {
        if let Some(sig) = self.head_sigs.get(&(g, tc)) {
            return sig.clone();
        }
        let (sig, settled) = self.head_sig_now(g, tc);
        let sig = Arc::new(sig);
        if settled {
            self.head_sigs.insert((g, tc), sig.clone());
        }
        sig
    }

    fn head_sig_now(&mut self, g: SymId, tc: ClassId) -> (HeadSig, bool) {
        let live = HeadSig { provides: None, head: HeadForm::Live };
        if self.syms.sym(g).state() == Completion::InProgress {
            return (live, false);
        }
        let ret = self.sig_of(g).ret;
        let ret = self.deref(ret);
        if self.types.has_vars(ret) {
            return (live, false);
        }
        let inter = matches!(self.types.get(ret), Type::Inter(..));
        let mut parts = Vec::new();
        // The class test's answer, unless a part has no plain class; the head's form, which a
        // part of another shape makes `Never` and an argument `dealias` reads makes `Live`.
        let (mut provides, mut provides_live) = (false, false);
        let (mut never, mut head_live) = (false, false);
        let mut settled = true;
        let mut stack = vec![ret];
        while let Some(t) = stack.pop() {
            let t = self.deref(t);
            if t == ERROR {
                never = true;
                continue;
            }
            match self.types.get(t) {
                Type::Inter(a, b) => {
                    stack.push(b);
                    stack.push(a);
                }
                Type::Class(gc, gcargs) => {
                    if self.is_java_placeholder(gc) {
                        return (live, false);
                    }
                    if gcargs != EMPTY_LIST && self.types.is_op_class(gc) {
                        provides_live = true;
                    }
                    let base = if gc == tc { Some(t) } else { self.base_type(t, tc) };
                    self.complete_class(gc);
                    settled &= self.syms.class_done(gc).is_some();
                    provides |= self.syms.class(gc).base_types.iter().any(|&(b, _)| b == tc);
                    let Some(base) = base else {
                        parts.push(None);
                        continue;
                    };
                    let Type::Class(_, gargs) = self.types.get(base) else {
                        never = true;
                        continue;
                    };
                    let mut args = Vec::with_capacity(self.types.items(gargs).len());
                    for k in 0..self.types.items(gargs).len() {
                        let y = self.types.items(gargs)[k];
                        let y = if matches!(self.types.get(y), Type::Lit(_)) { self.widen_lit(y) } else { y };
                        let arg = match self.types.get(y) {
                            _ if head_dependent(self, y) => HeadArg::Dependent,
                            Type::Param(_) => HeadArg::Open,
                            Type::Class(c, cargs) if !(cargs != EMPTY_LIST && self.types.is_op_class(c)) => {
                                if self.is_java_placeholder(c) {
                                    return (live, false);
                                }
                                HeadArg::Class { c, special: self.special_head(c), tuple: self.tuple_like(c), nominal: self.nominal_class(c) }
                            }
                            _ => {
                                head_live = true;
                                HeadArg::Open
                            }
                        };
                        args.push(arg);
                    }
                    parts.push(Some(args.into_boxed_slice()));
                }
                _ => {
                    never = true;
                    provides_live = true;
                }
            }
        }
        let head = if never {
            HeadForm::Never
        } else if head_live {
            HeadForm::Live
        } else {
            HeadForm::Parts { inter, parts: parts.into_boxed_slice() }
        };
        (HeadSig { provides: (!provides_live).then_some(provides), head }, settled)
    }

    /// Whether every part of the candidate's result that provides the target's class is
    /// rejected; a given of an intersection type provides every part.
    fn head_rejects_parts(&mut self, ret: TypeId, tc: ClassId, targs: TList, inter: bool, providing: &mut u32) -> bool {
        let ret = self.deref(ret);
        match self.types.get(ret) {
            Type::Inter(a, b) => self.head_rejects_parts(a, tc, targs, inter, providing) && self.head_rejects_parts(b, tc, targs, inter, providing),
            Type::Class(gc, _) => {
                let ret = if gc == tc { Some(ret) } else { self.base_type(ret, tc) };
                match ret {
                    Some(ret) => {
                        *providing += 1;
                        self.head_rejects_at(ret, tc, targs, inter)
                    }
                    None => true,
                }
            }
            _ => false,
        }
    }

    /// `head_rejects` for a candidate's result `ret`, an application of the target's class.
    /// The parts of an intersection (`inter`) combine at a variant position (`TC[A] & TC[B]`
    /// is a `TC[A & B]` for a covariant `TC`, a `TC[A | B]` for a contravariant one), so a
    /// variable's bounds reject a part at an invariant position only, where the whole fits
    /// through one part.
    fn head_rejects_at(&mut self, ret: TypeId, tc: ClassId, targs: TList, inter: bool) -> bool {
        let Type::Class(_, gargs) = self.types.get(ret) else { return false };
        if targs == gargs {
            return false;
        }
        let n = self.types.items(targs).len().min(self.types.items(gargs).len());
        for i in 0..n {
            self.settle_class(tc);
            let tp = self.syms.class(tc).tparams[i];
            let variance = self.syms.tparam(tp).variance;
            let (x, y) = (self.types.items(targs)[i], self.types.items(gargs)[i]);
            // An argument that depends on a prefix (`Kind.this` of a given reached through an
            // object that extends `Kind`) is read through that prefix by the shape pass.
            if head_dependent(self, x) || head_dependent(self, y) || self.types.is_wild(x) {
                continue;
            }
            let (x, y) = (self.dealias(x), self.dealias(y));
            let Type::Class(cy, _) = self.types.get(y) else { continue };
            match self.types.get(x) {
                Type::Var(v) => {
                    if (variance == 0 || !inter) && self.bounds_reject(v, cy, variance) {
                        return true;
                    }
                }
                Type::Class(cx, _) if variance == 0 => {
                    if cx != cy && !self.special_head(cx) && !self.special_head(cy) && !(self.tuple_like(cx) && self.tuple_like(cy)) {
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }

    /// Whether a candidate's argument of class `cy` cannot meet the open variable `v` at a
    /// position of the given variance: a lower bound of the variable has to conform to the
    /// argument where the position is invariant or contravariant, the argument to an upper
    /// bound where it is invariant or covariant, and a class conforms to another only by
    /// deriving from it where both are nominal (`nominal_class`); a bound of any other shape
    /// is left to the shape pass.
    fn bounds_reject(&mut self, v: TVarId, cy: ClassId, variance: i8) -> bool {
        self.nominal_class(cy) && self.bounds_reject_nominal(v, cy, variance)
    }

    /// `bounds_reject` for a nominal class `cy`.
    fn bounds_reject_nominal(&mut self, v: TVarId, cy: ClassId, variance: i8) -> bool {
        let plain = |t: &mut Self, b: TypeId| -> Option<ClassId> {
            let b = t.dealias(b);
            let Type::Class(cb, _) = t.types.get(b) else { return None };
            if !t.nominal_class(cb) || (t.tuple_like(cb) && t.tuple_like(cy)) {
                return None;
            }
            Some(cb)
        };
        if variance <= 0 {
            for i in 0..self.tvars[v].lower.len() {
                let l = self.tvars[v].lower[i];
                if let Some(cl) = plain(self, l) {
                    if cl != cy && !self.derives_from(cl, cy) {
                        return true;
                    }
                }
            }
        }
        if variance >= 0 {
            for i in 0..self.tvars[v].upper.len() {
                let u = self.tvars[v].upper[i];
                if let Some(cu) = plain(self, u) {
                    if cu != cy && !self.derives_from(cy, cu) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Whether conformance to or from the class is decided by its base types alone, which
    /// is what a bound rejection reads. Every class with a rule of its own in `is_sub` is
    /// left to the shape pass: `Null`, `AnyRef`, `AnyVal`, `Singleton`, `Product`, `Equals`,
    /// `reflect.Enum`, `Matchable` and `AnyKind` (the tops and the rights conferred by a
    /// shape), an opaque type, a type-level operation, a Java class nothing has read, and
    /// the classes whose conformance `base_type` decides beyond their base types (`String`
    /// and `Array` towards their Java interfaces, a function class towards `js.Function`).
    fn nominal_class(&self, c: ClassId) -> bool {
        let special_type = |t: TypeId| matches!(self.types.get(t), Type::Class(k, _) if k == c);
        !(self.special_head(c)
            || self.is_java_placeholder(c)
            || special_type(self.b.t_singleton)
            || special_type(self.b.t_equals)
            || special_type(self.b.t_product)
            || special_type(self.b.t_enum)
            || self.is_scala_class(c, "Matchable")
            || self.is_scala_class(c, "AnyKind")
            || c == self.b.string
            || c == self.b.array
            || self.is_function_class(c))
    }

    pub(super) fn is_scala_class(&self, c: ClassId, name: &str) -> bool {
        let info = self.syms.class(c);
        info.owner == Owner::Package(self.b.scala_pkg) && self.interner.get(info.name) == name
    }

    fn special_head(&self, c: ClassId) -> bool {
        c == self.b.null
            || c == self.b.any_ref
            || c == self.b.any_val
            || Some(c) == self.b.product
            || Some(c) == self.b.reflect_enum
            || self.syms.class(c).kind == ClassKind::Opaque
            || self.types.is_op_class(c)
    }

    fn tuple_like(&self, c: ClassId) -> bool {
        Some(c) == self.b.cons_tuple || Some(c) == self.b.empty_tuple || self.is_tuple_class(c)
    }

    /// Whether the given's type can fit the target: its signature opened within its bounds and
    /// its result type matched against the target, which is the part of `try_given` before the
    /// using clauses. The bindings this makes are the caller's to roll back.
    /// Whether the candidate's type fits the target, and whether that answer holds wherever the
    /// same candidate meets the same target without open variables: it does unless the
    /// candidate's type or bounds mention a path or a type parameter of an enclosing class,
    /// which the scope of the search would read (`fit_decision` rules the members of generic
    /// owners out before asking).
    fn fits_target(&mut self, given: GivenRef, target: TypeId) -> (bool, bool) {
        let Some((_, subst, sig)) = self.open_given_sig(given) else { return (false, false) };
        let mut ret = self.types.subst(sig.ret, &subst);
        let mut context_free = !self.types.has_paths(ret) && !self.mentions_tparam_of(ret, None);
        if self.types.has_paths(ret) {
            let params: Vec<SymId> = sig.clauses.iter().flat_map(|cl| cl.params.iter().map(|p| p.sym)).collect();
            ret = self.approx_paths(ret, &params, &mut Vec::new());
        }
        for &tp in sig.tparams.iter() {
            if !context_free {
                break;
            }
            let info = self.syms.tparam(tp);
            for bound in [info.upper, info.lower] {
                if bound != ANY && bound != NOTHING {
                    let bound = self.types.subst(bound, &subst);
                    if self.types.has_paths(bound) || self.mentions_tparam_of(bound, None) {
                        context_free = false;
                    }
                }
            }
        }
        let fits = self.is_sub(ret, target) && self.hk_bounds_hold(&sig, &subst);
        (fits, context_free)
    }

    /// Whether the bound of each higher-kinded type parameter the target instantiated holds:
    /// `C[X] <: Set[X]` rules `List` out for `C`, which `open_given_sig` cannot tell while
    /// `C` is still open.
    fn hk_bounds_hold(&mut self, sig: &MethodSig, subst: &Subst) -> bool {
        for &tp in &sig.tparams {
            let info = self.syms.tparam(tp);
            let (arity, upper) = (info.arity, info.upper);
            if arity == 0 || upper == ANY {
                continue;
            }
            let var = self.types.param(tp);
            let var = self.types.subst(var, subst);
            let var = self.deref(var);
            if matches!(self.types.get(var), Type::Var(_) | Type::AppVar(..)) || var == ERROR {
                continue;
            }
            let args: Vec<TypeId> = (0..arity).map(|_| {
                let p = self.syms.new_tparam(crate::names::WILDCARD, 0);
                self.types.param(p)
            }).collect();
            let applied = self.types.apply_ctor(var, &args);
            let bound = self.types.subst(upper, subst);
            let bound = self.types.apply_ctor(bound, &args);
            if applied == ERROR || bound == ERROR {
                continue;
            }
            let mark = self.snapshot();
            let holds = self.is_sub(applied, bound);
            self.rollback(mark);
            if !holds {
                return false;
            }
        }
        true
    }

    /// Orders the fitting candidates so that the ones `compare_givens` prefers come first: each
    /// by how many of the others it beats, then by their position. A count is a total order,
    /// so the comparison need not be one.
    /// The order of a long list is kept per list of symbols (`given_orders`) under the
    /// conditions `compare_givens` keeps a comparison, since the order is fixed by the
    /// symbols where the comparisons are.
    fn preference_order(&mut self, candidates: &[GivenRef], fitting: &mut Vec<usize>) {
        let keep = fitting.len() > 8
            && self.transparent.is_empty()
            && self.gadt.is_empty()
            && fitting.iter().all(|&i| !self.defined_in_body(candidates[i].0));
        let key: Option<Box<[SymId]>> = keep.then(|| fitting.iter().map(|&i| candidates[i].0).collect());
        if let Some(order) = key.as_ref().and_then(|k| self.given_orders.get(k)) {
            let ordered: Vec<usize> = order.iter().map(|&k| fitting[k as usize]).collect();
            *fitting = ordered;
            return;
        }
        let mut scored: Vec<(usize, usize)> = Vec::with_capacity(fitting.len());
        for (k, &i) in fitting.iter().enumerate() {
            let mut beats = 0;
            for &j in fitting.iter() {
                if i != j && self.compare_givens(candidates[i].0, candidates[j].0) > 0 {
                    beats += 1;
                }
            }
            scored.push((beats, k));
        }
        scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        if let Some(key) = key {
            self.given_orders.insert(key, scored.iter().map(|&(_, k)| k as u32).collect());
        }
        let ordered: Vec<usize> = scored.into_iter().map(|(_, k)| fitting[k]).collect();
        *fitting = ordered;
    }

    /// The success that is preferred over every other one, if there is one.
    fn most_specific_given(&mut self, found: &[Success]) -> Option<usize> {
        let mut best = 0;
        for i in 1..found.len() {
            if self.compare_givens(found[i].given.0, found[best].given.0) > 0 {
                best = i;
            }
        }
        let beats_all =
            (0..found.len()).all(|i| i == best || self.compare_givens(found[best].given.0, found[i].given.0) > 0);
        beats_all.then_some(best)
    }

    /// Where an opaque type is transparent the givens of its underlying type apply to it as
    /// well. The one that applies without looking through the opaque type is meant.
    fn given_for_opaque_type(&mut self, found: &[Success], target: TypeId, span: Span) -> Option<usize> {
        let transparent = std::mem::take(&mut self.transparent);
        let mut only = None;
        let mut count = 0;
        for (i, success) in found.iter().enumerate() {
            // A probe of each success under the definition: what it writes goes with it.
            let mark = self.attempt();
            if self.instantiate_given_with(success.given, Some(target), span, true).is_some() {
                only = Some(i);
                count += 1;
            }
            self.retract(mark);
        }
        self.transparent = transparent;
        only.filter(|_| count == 1)
    }

    /// Scala 3.7's preference between two givens that both apply: positive when `a` is
    /// preferred. The given whose owner derives from the other's owner wins whatever their
    /// types; between unrelated owners the given whose type is as good as the other's wins.
    /// After a draw the given without using parameters wins, and two givens with using
    /// parameters are compared by those.
    pub(super) fn compare_givens(&mut self, a: SymId, b: SymId) -> i32 {
        if a == b {
            return 0;
        }
        // The answer depends on the two signatures and owners alone, except through an opaque
        // type whose definition is visible here, or a given defined inside a body, whose
        // signature may name a type parameter of the enclosing method and read it under a
        // pattern's GADT bounds; those are compared afresh.
        let keep = self.transparent.is_empty()
            && self.gadt.is_empty()
            && !self.defined_in_body(a)
            && !self.defined_in_body(b);
        if keep {
            if let Some(&c) = self.given_preferences.get(&(a, b)) {
                return c as i32;
            }
        }
        let c = self.compare_givens_uncached(a, b);
        if keep {
            self.given_preferences.insert((a, b), c as i8);
            self.given_preferences.insert((b, a), -c as i8);
        }
        c
    }

    /// Whether the given is defined inside a body: a local, or a member of a class nested,
    /// at any depth, in a block.
    fn defined_in_body(&self, g: SymId) -> bool {
        let mut owner = self.syms.sym(g).owner;
        loop {
            match owner {
                Owner::Local => return true,
                Owner::Package(_) => return false,
                Owner::Class(c) => owner = self.syms.class(c).owner,
            }
        }
    }

    fn compare_givens_uncached(&mut self, a: SymId, b: SymId) -> i32 {
        let owners = self.compare_given_owners(a, b);
        // Two Scala 2 implicits are compared as under Scala 3.0 to 3.6 (dotc's intermediate
        // scheme): the more specific type wins, and an owner's win holds only where the type
        // comparison does not go the other way. Between givens the owner decides outright.
        let old_style = self.syms.is_scala2_implicit(a) && self.syms.is_scala2_implicit(b);
        if owners != 0 && !old_style {
            return owners;
        }
        let (wins_a, wins_b) = (self.is_as_good_given(a, b, old_style), self.is_as_good_given(b, a, old_style));
        let by_type = match owners {
            1 if wins_a || !wins_b => 1,
            -1 if wins_b || !wins_a => -1,
            0 => wins_a as i32 - wins_b as i32,
            _ => 0,
        };
        if by_type != 0 {
            return by_type;
        }
        let (using_a, using_b) = (!self.sig_of(a).clauses.is_empty(), !self.sig_of(b).clauses.is_empty());
        match (using_a, using_b) {
            (false, false) => 0,
            (false, true) => 1,
            (true, false) => -1,
            (true, true) => self.preference(|t| t.takes_using_params_of(b, a), |t| t.takes_using_params_of(a, b)),
        }
    }

    fn preference(
        &mut self,
        a_as_good: impl FnOnce(&mut Self) -> bool,
        b_as_good: impl FnOnce(&mut Self) -> bool,
    ) -> i32 {
        a_as_good(self) as i32 - b_as_good(self) as i32
    }

    pub(super) fn compare_given_owners(&mut self, a: SymId, b: SymId) -> i32 {
        match (self.syms.sym(a).owner, self.syms.sym(b).owner) {
            (Owner::Class(ca), Owner::Class(cb)) => self.compare_owners(ca, cb),
            _ => 0,
        }
    }

    /// An object also stands for the class it is the companion of.
    fn compare_owners(&mut self, a: ClassId, b: ClassId) -> i32 {
        if a == b {
            return 0;
        }
        self.complete_class(a);
        self.complete_class(b);
        if self.derives_from(a, b) {
            return 1;
        }
        if self.derives_from(b, a) {
            return -1;
        }
        let companion_class = |t: &Self, c: ClassId| {
            let info = t.syms.class(c);
            info.companion.filter(|_| t.is_module_class(c))
        };
        if let Some(class_a) = companion_class(self, a) {
            return self.compare_owners(class_a, b);
        }
        if let Some(class_b) = companion_class(self, b) {
            return self.compare_owners(a, class_b);
        }
        0
    }

    pub(super) fn derives_from(&mut self, c: ClassId, base: ClassId) -> bool {
        self.complete_class(c);
        self.syms.class(c).base_types.iter().any(|&(b, _)| b == base)
    }

    fn fresh_tparam_subst(&mut self, tparams: &[TParamId]) -> Subst {
        tparams.iter().map(|&tp| (tp, self.fresh_var())).collect()
    }

    /// Scala 3.7 prefers the more general of two givens: `a` is as good as `b` when the type
    /// of `b`, for some choice of its type parameters, conforms to the type of `a`. For the
    /// usual invariant type class that makes `TC[List[A]]` as good as `TC[A]` and not the
    /// other way round.
    /// Scala 3.7's preference between givens: `a` is as good as `b` when `b`'s type, its
    /// parameters open, fits `a`'s (the more general wins). Between two Scala 2 implicits
    /// (`old_style`) the relation is the other way round, as in Scala 3.0 to 3.6: `a`'s type
    /// fits `b`'s with `b`'s parameters open (the more specific wins), the arguments at
    /// contravariant parameters flipped so that the more specific argument wins there too
    /// (`Ord[Int]` over `Ord[Any]` for an `Ord[-T]`).
    fn is_as_good_given(&mut self, a: SymId, b: SymId, old_style: bool) -> bool {
        let (sig_a, sig_b) = (self.sig_arc(a), self.sig_arc(b));
        let mark = self.snapshot();
        let subst = self.fresh_tparam_subst(&sig_b.tparams);
        let ret_b = self.types.subst(sig_b.ret, &subst);
        let ok = if old_style {
            let (ret_a, ret_b) = (self.flip_contravariant(sig_a.ret), self.flip_contravariant(ret_b));
            self.is_sub(ret_a, ret_b)
        } else {
            self.is_sub(ret_b, sig_a.ret)
        };
        // `b`'s type parameters, instantiated to `a`'s, keep their bounds: a `C[X] <: Set[X]`
        // takes no `C[X] <: Iterable[X]` of the other.
        let ok = ok && self.hk_bounds_hold(&sig_b, &subst);
        self.rollback(mark);
        ok
    }

    /// `t` with every argument at a contravariant parameter replaced by `arg => Unit`, as
    /// scalac's intermediate scheme prepares the types it compares.
    fn flip_contravariant(&mut self, t: TypeId) -> TypeId {
        let t = self.deref(t);
        let Type::Class(c, args) = self.types.get(t) else { return t };
        let items: Vec<TypeId> = self.types.items(args).to_vec();
        if items.is_empty() {
            return t;
        }
        self.complete_class(c);
        let tparams = self.syms.class(c).tparams.clone();
        let function1 = self.function_class(1);
        let unit = self.b.t_unit;
        let mapped: Vec<TypeId> = items
            .iter()
            .enumerate()
            .map(|(i, &arg)| {
                let arg = self.flip_contravariant(arg);
                match tparams.get(i).map(|&p| self.syms.tparam(p).variance) {
                    Some(v) if v < 0 => self.types.class(function1, &[arg, unit]),
                    _ => arg,
                }
            })
            .collect();
        self.types.class(c, &mapped)
    }

    /// Whether `g` could be applied to the using parameters of `other`.
    fn takes_using_params_of(&mut self, g: SymId, other: SymId) -> bool {
        let params = |sig: &MethodSig| -> Vec<TypeId> {
            sig.clauses.iter().flat_map(|c| c.params.iter().map(|p| p.ty)).collect()
        };
        let sig = self.sig_arc(g);
        let own = params(&sig);
        let given = params(self.sig_of(other));
        if own.len() != given.len() {
            return false;
        }
        let mark = self.snapshot();
        let subst = self.fresh_tparam_subst(&sig.tparams);
        let ok = own.iter().zip(&given).all(|(&p, &arg)| {
            let p = self.types.subst(p, &subst);
            self.is_sub(arg, p)
        });
        self.rollback(mark);
        ok
    }

    /// The candidates of one search share its target, so the measure is taken once per target.
    fn open_given(&mut self, g: SymId, target: TypeId) -> OpenGiven {
        let (size, classes, wild) = match self.measured_target {
            Some((t, size, classes, wild)) if t == target => (size, classes, wild),
            _ => {
                let (mut size, mut classes) = (0, 0);
                let wild = self.wild_approx(target, &mut size, &mut classes, 0);
                self.measured_target = Some((target, size, classes, wild));
                (size, classes, wild)
            }
        };
        OpenGiven { given: g, target, size, classes, wild, byname: self.search_byname, rec: None }
    }

    /// The type with its open variables blanked out, counting on the way the applied types it
    /// is made of and the classes it mentions, bounds of the variables included.
    fn wild_approx(&mut self, t: TypeId, size: &mut u32, classes: &mut u64, depth: u32) -> TypeId {
        let t = self.deref(t);
        match self.types.get(t) {
            Type::Class(c, args) => {
                *classes |= 1 << (c.idx() % 64);
                let items = self.types.items(args);
                if items.is_empty() {
                    return t;
                }
                *size += 1;
                let items = items.to_vec();
                let approx: Vec<TypeId> = items.iter().map(|&a| self.wild_approx(a, size, classes, depth)).collect();
                self.types.class(c, &approx)
            }
            Type::Ctor(c) => {
                *classes |= 1 << (c.idx() % 64);
                t
            }
            Type::Var(v) => {
                if depth < 8 {
                    let info = &self.tvars[v];
                    let bounds: Vec<TypeId> = info.lower.iter().chain(&info.upper).copied().collect();
                    for b in bounds {
                        self.wild_approx(b, size, classes, depth + 1);
                    }
                }
                WILD
            }
            Type::AppVar(_, args) => {
                *size += 1;
                for a in self.types.items(args).to_vec() {
                    self.wild_approx(a, size, classes, depth);
                }
                WILD
            }
            Type::Param(_) | Type::AppParam(..) => WILD,
            Type::Lambda(ps, body) => {
                let b = self.wild_approx(body, size, classes, depth);
                let blank = vec![WILD; self.types.items(ps).len()];
                let ps = self.types.list(&blank);
                self.types.mk(Type::Lambda(ps, b))
            }
            Type::Union(a, b) => {
                let (a, b) = (self.wild_approx(a, size, classes, depth), self.wild_approx(b, size, classes, depth));
                self.types.union(a, b)
            }
            Type::Inter(a, b) => {
                let (a, b) = (self.wild_approx(a, size, classes, depth), self.wild_approx(b, size, classes, depth));
                self.types.inter(a, b)
            }
            _ => t,
        }
    }

    /// scalac's divergence rule: the request diverges when the same given, or a sibling of the
    /// same signature, is already open for a type that is no smaller and made of the same
    /// classes, unless that type is smaller than or unlike the one requested now.
    pub(super) fn diverges(&mut self, open: &OpenGiven) -> bool {
        for i in (0..self.given_stack.len()).rev() {
            let prev = &self.given_stack[i];
            let (given, size, classes, wild, prev_target) = (prev.given, prev.size, prev.classes, prev.wild, prev.target);
            if given != open.given && !self.same_signature(given, open.given) {
                continue;
            }
            if size > open.size || classes != open.classes {
                continue;
            }
            let r = size < open.size || wild == open.wild;
            if r && self.given_trace() {
                let (t, w) = (self.show(open.target), self.show(prev_target));
                eprintln!("diverges: {} under {} ({})", t, w, self.name_str(self.syms.sym(open.given).name));
            }
            return r;
        }
        false
    }

    /// Two givens of one owner that declare the same types, which ask for the same using
    /// parameters and so make the same progress.
    fn same_signature(&mut self, a: SymId, b: SymId) -> bool {
        if self.syms.sym(a).owner != self.syms.sym(b).owner {
            return false;
        }
        let (sa, sb) = (self.sig_arc(a), self.sig_arc(b));
        sa.tparams.len() == sb.tparams.len()
            && sa.clauses.len() == sb.clauses.len()
            && self.declared_types(&sa) == self.declared_types(&sb)
    }

    /// The result and parameter types with every type parameter blanked out.
    fn declared_types(&mut self, sig: &MethodSig) -> Vec<TypeId> {
        let (mut size, mut classes) = (0, 0);
        let mut out = vec![self.wild_approx(sig.ret, &mut size, &mut classes, 0)];
        for clause in &sig.clauses {
            for p in &clause.params {
                out.push(self.wild_approx(p.ty, &mut size, &mut classes, 0));
            }
        }
        out
    }

    /// The target with every open type variable replaced by one of its bounds, or blanked out
    /// when it has none, for reporting.
    fn target_shape(&mut self, t: TypeId, seen: &mut Vec<TVarId>) -> TypeId {
        if !self.types.has_vars(t) {
            return t;
        }
        let t = self.deref(t);
        match self.types.get(t) {
            Type::Var(v) => {
                if seen.contains(&v) {
                    return ERROR;
                }
                seen.push(v);
                let info = &self.tvars[v];
                let bounds: Vec<TypeId> = info.lower.iter().chain(&info.upper).copied().collect();
                let shape = bounds.into_iter().map(|b| self.target_shape(b, seen)).find(|&b| b != ERROR);
                seen.pop();
                shape.unwrap_or(ERROR)
            }
            Type::Class(c, args) => {
                let args = self.types.items(args).to_vec();
                let shapes: Vec<TypeId> = args.into_iter().map(|a| self.target_shape(a, seen)).collect();
                self.types.class(c, &shapes)
            }
            _ => t,
        }
    }

    /// Builds a reference to the given, resolving its own using clauses recursively. With
    /// `solve` unset the given's type variables stay open for the caller to constrain.
    pub fn instantiate_given_with(
        &mut self,
        given: GivenRef,
        target: Option<TypeId>,
        span: Span,
        solve: bool,
    ) -> Option<(TExprId, TypeId)> {
        if self.implicit_depth == 0 {
            self.implicit_budget = IMPLICIT_SEARCH_LIMIT;
        }
        match self.try_given(given, target, span, solve) {
            Attempt::Ok(te, ty) => Some((te, ty)),
            _ => None,
        }
    }

    /// The given's signature opened for one use: fresh variables for its type parameters, within
    /// their bounds, and the trait's as the object it is reached through instantiates them.
    /// `None` when the bounds cannot hold.
    fn open_given_sig(&mut self, given: GivenRef) -> Option<(Option<(TraitMemberSite, ClassId)>, Subst, Arc<MethodSig>)> {
        let (g, scope) = given;
        // Kept until the candidate is instantiated below, past the site's lookup.
        let sig = self.sig_arc(g);
        // A given defined in a trait and reached through an object sees the trait's type
        // parameters as that object instantiates them.
        let trait_owner = match self.syms.sym(g).owner {
            Owner::Class(c) if self.syms.class(c).kind != ClassKind::Object => Some(c),
            _ => None,
        };
        let site = trait_owner.map(|c| {
            if let GivenScope::Value(v) = scope {
                return (TraitMemberSite::Value(v), c);
            }
            if let GivenScope::Path(p) = scope {
                return (TraitMemberSite::Path(p), c);
            }
            let scope = scope.module();
            let through = scope.and_then(|m| self.exports_of(m)).and_then(|e| e.via.get(&g).copied());
            // An object of the implicit scope that extends the trait (`K0 extends Kind[..]`
            // for `Kind.mkProductInstances`) is the module the given is reached through.
            let inherited = scope.filter(|&m| self.syms.class(m).kind == ClassKind::Object && self.derives_from(m, c));
            match through.or(inherited) {
                Some(m) => (TraitMemberSite::Module(m), c),
                None => (self.trait_member_site(g, c), c),
            }
        });
        if let Some((TraitMemberSite::This(_), _)) = site {
            if self.inline.depth > 0 {
                self.read_in_progress(InProgress::InlineReceiver);
            }
        }
        let mut subst: Subst = match site {
            Some((site, c)) => self.trait_member_subst(site, c),
            None => Vec::with_capacity(sig.tparams.len()),
        };
        // `Kind.this` in the signature is the module or path the given is reached through.
        let sig = match site {
            // A given a class in scope inherits sees its owner's `this` as that class's (dotty's
            // `TermRef(Sub.this, given)` seen from `Sub.this`).
            Some((site @ (TraitMemberSite::Module(_) | TraitMemberSite::Path(_) | TraitMemberSite::This(Some(_))), c))
                if !matches!(site, TraitMemberSite::This(Some(k)) if k == c)
                    && (self.types.has_paths(sig.ret) || sig.clauses.iter().any(|cl| cl.params.iter().any(|p| self.types.has_paths(p.ty)))) =>
            {
                let (prefix, c) = match site {
                    TraitMemberSite::Path(p) => (p, c),
                    TraitMemberSite::Module(m) => (self.types.class(m, &[]), c),
                    TraitMemberSite::This(Some(k)) => (self.this_prefix(k), k),
                    _ => unreachable!(),
                };
                let mut seen = (*sig).clone();
                seen.ret = self.as_seen_from(seen.ret, prefix, c);
                for cl in seen.clauses.iter_mut() {
                    for p in cl.params.iter_mut() {
                        p.ty = self.as_seen_from(p.ty, prefix, c);
                    }
                }
                Arc::new(seen)
            }
            _ => sig,
        };
        for &tp in &sig.tparams {
            let v = self.fresh_var();
            subst.push((tp, v));
        }
        for &tp in &sig.tparams {
            let info = self.syms.tparam(tp);
            let (upper, lower) = (info.upper, info.lower);
            if upper != ANY || lower != NOTHING {
                let var = self.types.param(tp);
                let var = self.types.subst(var, &subst);
                let (upper, lower) = (self.types.subst(upper, &subst), self.types.subst(lower, &subst));
                if !(self.is_sub(var, upper) && self.is_sub(lower, var)) {
                    return None;
                }
            }
        }
        Some((site, subst, sig))
    }

    /// Instantiates the variables made from `first_var` on whose lower and upper bounds name
    /// one type without variables.
    fn solve_pinned_vars(&mut self, first_var: usize) {
        for v in first_var..self.tvars.len() {
            let info = &self.tvars[v];
            if info.inst.is_some() || info.lower.len() != 1 || info.upper.len() != 1 {
                continue;
            }
            let (l, u) = (info.lower[0], info.upper[0]);
            let (l, u) = (self.zonk(l), self.zonk(u));
            if l == u && !self.types.has_vars(l) && l != ERROR {
                self.instantiate(self.tvars.id(v), l);
            }
        }
    }

    fn try_given(&mut self, given: GivenRef, target: Option<TypeId>, span: Span, solve: bool) -> Attempt {
        let (g, scope) = given;
        if self.implicit_budget == 0 {
            self.read_in_progress(InProgress::Budget);
            return Attempt::Incomplete;
        }
        self.implicit_budget -= 1;
        self.profile.tries += self.profile.on as u32;
        let first_var = self.tvars.len();
        if self.is_conversion_def(g) {
            let step = self.step(Step::Conversion);
            let attempt = self.try_conversion_method(given, target, span);
            self.step_end(step, Step::Conversion);
            return attempt;
        }
        let Some((site, subst, sig)) = self.open_given_sig(given) else { return Attempt::Mismatch };
        // Measured before the target's variables are bound to parts of this given's type.
        let open = (!sig.clauses.is_empty()).then(|| self.open_given(g, target.unwrap_or(sig.ret)));
        let mut ret = self.types.subst(sig.ret, &subst);
        // A result naming a using parameter's path (`fold.Out`) is matched with that member
        // open, as scalac approximates a dependent result by wildcards, and settled once the
        // argument is found.
        let dependent = self.types.has_paths(sig.ret) || sig.clauses.iter().any(|cl| cl.params.iter().any(|p| self.types.has_paths(p.ty)));
        let mut approx: Vec<(TypeId, TypeId)> = Vec::new();
        if let Some(t) = target {
            let check = if dependent {
                let params: Vec<SymId> = sig.clauses.iter().flat_map(|cl| cl.params.iter().map(|p| p.sym)).collect();
                self.approx_paths(ret, &params, &mut approx)
            } else {
                ret
            };
            if !self.is_sub(check, t) || !self.hk_bounds_hold(&sig, &subst) {
                if self.given_trace() {
                    let (a, b) = (self.show(check), self.show(t));
                    eprintln!("mismatch {} : {} vs {}", self.name_str(self.syms.sym(g).name), a, b);
                }
                return Attempt::Mismatch;
            }
            if self.given_trace() {
                let (a, b) = (self.show(check), self.show(t));
                eprintln!("fits {} : {} vs {}", self.name_str(self.syms.sym(g).name), a, b);
            }
        }
        // A variable the target pinned from both sides (`T` of `F[T]` against `Eq[Slot]`
        // at an invariant position) is that type for the using clauses, which a mirror
        // synthesis reads as a class.
        self.solve_pinned_vars(first_var);
        let mut args = Vec::new();
        let mut knot = None;
        if let Some(open) = open {
            let step = self.step(Step::Divergence);
            let diverges = self.diverges(&open);
            self.step_end(step, Step::Divergence);
            if diverges {
                self.read_in_progress(InProgress::Divergence);
                return Attempt::Incomplete;
            }
            self.given_stack.push(open);
            self.given_opened.push(g);
            let mut failed = None;
            let inline_callee = self.is_inline_callee(g);
            // The arguments of an inline given expanded where it is found (a transparent one) are
            // resolved with the plain inline givens among them expanded where they are found, as
            // the expansion reads them (`Attempts::given_args`).
            let expands_now = inline_callee && !self.defers_plain_inline(g);
            self.attempts.given_args += expands_now as u32;
            let mut paths: Vec<(SymId, TypeId)> = Vec::new();
            'clauses: for clause in &sig.clauses {
                for p in &clause.params {
                    let pty = self.types.subst(p.ty, &subst);
                    let pty = if dependent { self.subst_paths(pty, &paths) } else { pty };
                    match self.resolve_given_pick(pty, span, p.by_name) {
                        Pick::Found(arg, arg_ty) => {
                            let arg = if self.capturing() { self.splice_quotes(arg, pty) } else { arg };
                            // An inline given's proxy for the parameter takes the argument's
                            // own type (a synthesized mirror's `MirroredElemTypes`).
                            if inline_callee {
                                self.inline.arg_types.push((arg, arg_ty));
                            }
                            if dependent {
                                let path = self.path_of(arg).unwrap_or(arg_ty);
                                paths.push((p.sym, path));
                            }
                            args.push(if p.by_name { self.by_name_thunk(arg) } else { arg });
                        }
                        // No given for a parameter with a default: the default, as at a call
                        // (http4s' `UrlForm.entityEncoder(implicit charset: Charset = UTF-8)`).
                        Pick::NoMatch if p.has_default => {
                            let omitted = self.default_placeholder();
                            args.push(omitted);
                        }
                        Pick::NoMatch => {
                            if self.given_trace() {
                                let a = self.show(pty);
                                eprintln!("incomplete {} : {}", self.name_str(self.syms.sym(g).name), a);
                            }
                            failed = Some(Attempt::Incomplete);
                            break 'clauses;
                        }
                        Pick::Ambiguous => {
                            if self.given_trace() {
                                let a = self.show(pty);
                                eprintln!("ambiguous {} : {}", self.name_str(self.syms.sym(g).name), a);
                            }
                            failed = Some(Attempt::Ambiguous);
                            break 'clauses;
                        }
                    }
                }
            }
            self.attempts.given_args -= expands_now as u32;
            knot = self.given_stack.pop().and_then(|open| open.rec);
            if let Some(failed) = failed {
                return failed;
            }
            if dependent {
                for &(member, var) in &approx {
                    let settled = self.subst_paths(member, &paths);
                    if settled != member && !(self.is_sub(settled, var) && self.is_sub(var, settled)) {
                        if self.given_trace() {
                            let (a, b) = (self.show(settled), self.show(var));
                            eprintln!("approx mismatch {} : {} vs {}", self.name_str(self.syms.sym(g).name), a, b);
                        }
                        return Attempt::Mismatch;
                    }
                }
                ret = self.subst_paths(ret, &paths);
                if let Some(t) = target {
                    if !self.is_sub(ret, t) {
                        if self.given_trace() {
                            let (a, b) = (self.show(ret), self.show(t));
                            eprintln!("settled mismatch {} : {} vs {}", self.name_str(self.syms.sym(g).name), a, b);
                        }
                        return Attempt::Mismatch;
                    }
                }
            }
        }
        if solve {
            for v in first_var..self.tvars.len() {
                self.solve_var(self.tvars.id(v));
            }
        }
        let (owner, impl_class) = {
            let s = self.syms.sym(g);
            (s.owner, s.impl_class)
        };
        let l = self.prog.list(&args);
        let kind = self.syms.sym(g).kind;
        // A Scala 2 implicit def is called even without parameters, as a given with type
        // parameters is (dotty's `Parsers.givenDef` makes it a def); an implicit object is the
        // module itself.
        let has_params = !sig.clauses.is_empty() || !sig.tparams.is_empty() || kind == SymKind::Def;
        let mut te = match owner {
            // A given object found inside its own class is that class's `this` (`tpd.ref`).
            _ if !has_params && self.enclosing_module_class(g).is_some() => {
                let k = self.enclosing_module_class(g).unwrap();
                self.this_ref(k)
            }
            _ if matches!(kind, SymKind::Object(_)) => {
                let SymKind::Object(c) = kind else { unreachable!() };
                self.prog.add(TExpr::Module(c))
            }
            Owner::Local if has_params => self.prog.add(TExpr::CallStatic(g, l)),
            // The using parameter of an inline method under expansion stands for its argument,
            // eligible at the parameter's declared type and found with the argument's own.
            Owner::Local => match self.inline_arg(g) {
                Some((arg, own)) => {
                    self.read_in_progress(InProgress::InlineArgument);
                    ret = own;
                    arg
                }
                None => self.prog.add(TExpr::Local(g)),
            },
            // `this` does not exist yet where the constructor of a superclass is called.
            Owner::Class(c) if self.parent_args_of == Some(c) && self.syms.class(c).superclass.is_some() => {
                self.prog.add(TExpr::Local(g))
            }
            Owner::Package(_) => match (impl_class, has_params) {
                (Some(c), true) => self.prog.add(TExpr::New(c, l)),
                (None, true) => self.prog.add(TExpr::CallStatic(g, l)),
                _ => self.prog.add(TExpr::Static(g)),
            },
            Owner::Class(c) => {
                let imported = match scope {
                    GivenScope::Value(v) if self.syms.class(c).inner_object.is_some() && self.import_value_class(v) == Some(c) => Some(v),
                    _ => None,
                };
                let path = match scope {
                    GivenScope::Path(p) => Some(p),
                    _ => None,
                };
                let recv = if let Some(p) = path {
                    // A given of the implicit scope's path is read on the path's value.
                    match self.prefix_value(p) {
                        Some(e) => e,
                        None => return Attempt::Mismatch,
                    }
                } else if let Some(v) = imported {
                    // A given imported from an object nested in a class (`import a.R.given`)
                    // is read on the imported value.
                    match self.import_value_ref(v, span) {
                        Some((e, _)) => e,
                        None => return Attempt::Mismatch,
                    }
                } else if let Some(v) = self.syms.class(c).local_module {
                    // A given imported from a local object lives on its lazy val.
                    self.prog.add(TExpr::Local(v))
                } else if let Some(v) = self.syms.class(c).inner_object {
                    // One of an object nested in a class is read from the enclosing instance.
                    match self.inner_object_ref(v) {
                        Some(e) => e,
                        None => return Attempt::Mismatch,
                    }
                } else if self.syms.class(c).kind == ClassKind::Object {
                    self.prog.add(TExpr::Module(c))
                } else {
                    match site {
                        Some((site, _)) => self.trait_member_receiver(site),
                        None => self.prog.add(TExpr::This),
                    }
                };
                match (impl_class, has_params) {
                    // The class of a given of a class or trait instance takes that instance first.
                    (Some(ic), true) if self.outer_class(ic).is_some() => {
                        let items: Vec<TExprId> = std::iter::once(recv).chain(args.iter().copied()).collect();
                        let l = self.prog.list(&items);
                        self.prog.add(TExpr::New(ic, l))
                    }
                    (Some(ic), true) => self.prog.add(TExpr::New(ic, l)),
                    (None, true) => self.prog.add(TExpr::CallMethod(recv, g, l)),
                    // A by-name using parameter of a class is forced, as a read of it is.
                    _ if self.syms.sym(g).by_name => {
                        let field = self.prog.add(TExpr::Field(recv, g));
                        self.prog.add(TExpr::CallClosure(field, ListRef::EMPTY))
                    }
                    _ => self.prog.add(TExpr::Field(recv, g)),
                }
            }
        };
        if self.capturing() {
            if let (Some(ic), TExpr::New(c, _)) = (impl_class, self.prog.expr(te)) {
                if c == ic {
                    self.capture_form(te, crate::tir::capture::Form::GivenCall(g));
                }
            }
        }
        // The object the candidate is read on, before an inline expansion takes the reference's
        // place, for the unused-import check (`Unused::receiver`).
        let receiver = if self.unused.on() { self.given_receiver(te, g) } else { None };
        // An `inline given` and an `implicit inline def` expand where they are used, like an
        // inline method; a macro among them runs here. Quoted code keeps the call of a member
        // for the site the quote is spliced at, and the definition check keeps every such call.
        let kept = self.is_inline_callee(g)
            && match self.prog.expr(te) {
                TExpr::CallMethod(..) | TExpr::Field(..) => self.keeps_inline_calls(),
                _ => self.checks_inline_definition(),
            };
        if kept {
            let recv = match self.prog.expr(te) {
                TExpr::CallMethod(r, _, _) | TExpr::Field(r, _) => Some(r),
                _ => None,
            };
            let call = super::apply::MethodCall { recv, sym: g, owner_subst: Vec::new(), ext_recv: None, prefix: None };
            let ret_ty = self.zonk(ret);
            self.read_in_progress(InProgress::InlineExpansion);
            self.defer_inline(&call, &sig, &subst, te, ret_ty, span, Some(ret_ty));
            self.quote.deferred_at_end(te);
        } else if self.is_inline_callee(g) && self.defers_plain_inline(g) && !self.types.contains_error(ret) {
            // A plain inline given is the candidate as a reference: dotty expands it after
            // typing, so its expansion's errors are no reason to try the next candidate.
            // Its expansion is the candidate's attempt's.
            let recv = match self.prog.expr(te) {
                TExpr::CallMethod(r, _, _) | TExpr::Field(r, _) => Some(r),
                _ => None,
            };
            let call = super::apply::MethodCall { recv, sym: g, owner_subst: Vec::new(), ext_recv: None, prefix: None };
            let ret_ty = self.zonk(ret);
            self.read_in_progress(InProgress::InlineExpansion);
            // The using arguments' own types go with the call, as the expansion below takes them
            // (a synthesized mirror's refinement, which a macro's quoted pattern reads).
            let arg_types = std::mem::take(&mut self.inline.arg_types);
            self.inline.site_at_end = true;
            self.defer_plain_inline(te, &call, &sig, &subst, &args, arg_types, ret_ty, ret_ty, span, Some(ret_ty));
            self.inline.site_at_end = false;
            // Found for an argument of a transparent given expanded where it is found, it is
            // expanded at once, after the pending calls typed before it, as the given's expansion
            // reads it; one whose expansion fails is no instance of its own search.
            if self.attempts.given_args > 0 {
                if let Some(first) = self.expand_pending_in(&[te]).filter(|_| self.implicit_depth > 0) {
                    self.drop_held(te);
                    if self.given_trace() {
                        let n = self.name_str(self.syms.sym(g).name);
                        eprintln!("candidate {} failed: {}", n, first.lines().next().unwrap_or(""));
                    }
                    if !self.failed_givens.iter().any(|(s, _)| *s == g) {
                        self.failed_givens.push((g, first));
                    }
                    return Attempt::Incomplete;
                }
            }
        } else if self.is_inline_callee(g) {
            let recv = match self.prog.expr(te) {
                TExpr::CallMethod(r, _, _) | TExpr::Field(r, _) => Some(r),
                _ => None,
            };
            let call = super::apply::MethodCall { recv, sym: g, owner_subst: Vec::new(), ext_recv: None, prefix: None };
            let ret_ty = self.zonk(ret);
            // A pending plain call among the arguments is expanded first, outside the
            // candidate's failure.
            if self.attempts.pending_len() != 0 {
                let mut roots = args.clone();
                roots.extend(recv);
                self.expand_pending_in(&roots);
            }
            let diag_mark = self.diags.items.len();
            self.read_in_progress(InProgress::InlineExpansion);
            self.inline.site_at_end = true;
            let expanded = self.expand_inline(&call, &sig, &subst, &args, ret_ty, span, Some(ret_ty));
            self.inline.site_at_end = false;
            if let Some((e, t)) = expanded {
                // A candidate whose expansion fails inside a search is no candidate, as in
                // scalac; the failure is kept for the report when nothing else is found.
                if self.implicit_depth > 0 {
                    // An error scalac reports after typing is no failure of the candidate (a
                    // `compiletime.error` the expansion holds, `state::reports_late`), but for a
                    // plain given found for an argument of a transparent one (`Attempts::given_args`).
                    let own = self.attempts.given_args > 0 && self.syms.sym(g).mods & crate::ast::mods::TRANSPARENT == 0;
                    let failure = match own {
                        true => self.diags.items[diag_mark..].iter().find(|d| !d.is_warning),
                        false => self.first_error_since(diag_mark),
                    };
                    if let Some(first) = failure.map(|d| d.msg.clone()) {
                        if self.given_trace() {
                            let n = self.name_str(self.syms.sym(g).name);
                            eprintln!("candidate {} failed: {}", n, first.lines().next().unwrap_or(""));
                        }
                        self.drop_reported_since(diag_mark);
                        if !self.failed_givens.iter().any(|(s, _)| *s == g) {
                            self.failed_givens.push((g, first));
                        }
                        return Attempt::Incomplete;
                    }
                }
                te = e;
                if self.syms.sym(g).mods & crate::ast::mods::TRANSPARENT != 0 {
                    ret = t;
                }
            }
        }
        if self.inline.checking > 0 && !sig.tparams.is_empty() {
            let own: Vec<(TParamId, TypeId)> = subst.iter().filter(|(p, _)| sig.tparams.contains(p)).copied().collect();
            self.note_type_args(te, &own);
        }
        if self.capturing() && !sig.tparams.is_empty() {
            let own: Vec<TypeId> = subst.iter().filter(|(p, _)| sig.tparams.contains(p)).map(|&(_, t)| t).collect();
            self.capture_call_targs(te, &own);
        }
        // A nested by-name request referred to this instance: it becomes a lazy local that the
        // thunks inside read once the instance exists.
        if let Some(rec) = knot {
            let stmts = self.prog.stmts.push_slice(&[TStmt::Val(rec, te)]);
            let result = self.prog.add(TExpr::Local(rec));
            te = self.prog.add(TExpr::Block(stmts, result));
        }
        // A given object is its module: the instance is of its class, as a reference to it by its
        // name is (`given_object_type`), whichever of the two a body read from a pickle holds;
        // through the path of the implicit scope it is reached on, the class through that path
        // (dotty's `TypeRef(p, C$)` under the module's `TermRef(p, given)`).
        let ty = match self.given_object_type(g) {
            Some(class) => match site {
                Some((TraitMemberSite::Path(p), _)) => self.nested_type(p, class),
                _ => class,
            },
            None => self.zonk(ret),
        };
        if self.given_trace() {
            let a = self.show(ty);
            eprintln!("ok {} : {}", self.name_str(self.syms.sym(g).name), a);
        }
        // The instance's type is kept on the expression for a macro that reads the argument.
        if self.prog.type_of(te).is_none() {
            self.prog.set_type(te, ty);
        }
        self.unused.receiver = receiver;
        Attempt::Ok(te, ty)
    }

    /// What to add to a "no given instance" report: the candidates whose expansion failed.
    pub fn given_failure_notes(&mut self) -> String {
        let failed = std::mem::take(&mut self.failed_givens);
        let mut out = String::new();
        for (g, msg) in failed.iter().take(3) {
            let first = msg.lines().next().unwrap_or("");
            out.push_str(&format!("\n  candidate {} failed: {}", self.method_description(*g), first));
        }
        out
    }

    /// scalac's recursive reference: a request that stands below a by-name parameter of an open
    /// given whose type conforms to it is answered by that given's own instance, which ties the
    /// knot of a recursive instance such as `Show[Tree]` through `Show[List[Tree]]`.
    fn recursive_ref(&mut self, target: TypeId, byname: bool, span: Span) -> Option<(TExprId, TypeId)> {
        let mut below_byname = byname;
        for i in (0..self.given_stack.len()).rev() {
            let (open_target, open_byname) = (self.given_stack[i].target, self.given_stack[i].byname);
            if below_byname {
                let mark = self.snapshot();
                if self.is_sub(open_target, target) {
                    let rec = match self.given_stack[i].rec {
                        Some(rec) => rec,
                        None => {
                            let rec = self.indexed_local("rec", i as u32, open_target, span);
                            self.syms.sym_mut(rec).mods |= crate::ast::mods::LAZY;
                            self.given_stack[i].rec = Some(rec);
                            rec
                        }
                    };
                    return Some((self.prog.add(TExpr::Local(rec)), open_target));
                }
                self.rollback(mark);
            }
            below_byname |= open_byname;
        }
        None
    }
}
