//! An interpreter over the typed IR: the third consumer of `tir::Program`, next to the two
//! emitters, and the engine that runs macro implementations at compile time. It walks the IR
//! as the typer produced it, typing the bodies of the standard library when it meets them, so
//! it needs the `Worker` rather than a finished program, and it imports nothing of the emitters.

mod archive;
mod bignum;
mod builtins;
mod char_tables;
mod class;
mod eval;
mod fast;
mod files;
mod format;
pub mod identity;
mod jdk;
mod maps;
mod natives;
mod net;
pub mod process;
pub mod profile;
pub mod quoted;
pub(crate) mod regex;
pub mod registry;
mod runtime;
pub mod value;

use crate::intern::{FxMap, Name};
use crate::source::FileId;
use crate::symbols::{Owner, SymKind, Symbols};
use crate::tir::{FunId, Program, StrRef, TExprId};
use crate::typer::Worker;
use crate::types::{ClassId, LitVal, SymId};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
pub use quoted::MacroCtx;
pub use value::{Object, TreeRef, Value};

/// One scope of locals. A def call, a block that declares something, a case with bindings and
/// a lambda call each open one; closures keep the frame they were made in.
pub struct Frame {
    pub parent: Option<Rc<Frame>>,
    pub vals: RefCell<Vec<(SymId, Value)>>,
    /// The frame's entry in the registry of a resident (`registry.rs`), or zero.
    pub(super) slot: Cell<u32>,
    /// The epoch that stamped the frame, 0 while it is its epoch's alone (`watch`).
    pub(super) born: Cell<u32>,
    /// When the frame was made (`Object::made`).
    pub(super) made: Cell<u32>,
}

impl Frame {
    pub fn new(parent: Option<Rc<Frame>>) -> Rc<Frame> {
        Rc::new(Frame { parent, vals: RefCell::new(Vec::new()), slot: Cell::new(0), born: Cell::new(0), made: Cell::new(made_now()) })
    }

    pub fn with(parent: Option<Rc<Frame>>, vals: Vec<(SymId, Value)>) -> Rc<Frame> {
        Rc::new(Frame { parent, vals: RefCell::new(vals), slot: Cell::new(0), born: Cell::new(0), made: Cell::new(made_now()) })
    }

    pub fn bind(self: &Rc<Self>, sym: SymId, v: Value) {
        written(self.born.get(), self.made.get(), || Some(Node::Frame(self.clone())), |other| format!("a local of a frame{}", if other { " another run opened" } else { "" }));
        self.vals.borrow_mut().push((sym, v));
    }
}

impl Drop for Frame {
    fn drop(&mut self) {
        if self.slot.get() != 0 {
            registry::vacate(self.slot.get());
        }
    }
}

/// What does not change within one body: the receiver, the def a `return` leaves, the class the
/// body belongs to (what `super` starts from) and the def whose tail calls loop.
#[derive(Clone)]
pub struct Ctx {
    pub this: Value,
    pub def_key: u64,
    pub class: Option<ClassId>,
    pub tail: Option<(SymId, u32)>,
}

impl Ctx {
    pub fn plain() -> Ctx {
        Ctx { this: Value::Unit, def_key: 0, class: None, tail: None }
    }
}

/// The scope an expression is evaluated in, for the API.
pub struct Env {
    pub frame: Rc<Frame>,
    pub ctx: Ctx,
}

impl Env {
    pub fn empty() -> Env {
        Env { frame: Frame::new(None), ctx: Ctx::plain() }
    }

    #[allow(dead_code)]
    pub fn bind(&self, sym: SymId, v: Value) {
        self.frame.bind(sym, v);
    }
}

/// Why an evaluation stopped: a thrown value, a `return` on its way to its def, a tail call the
/// def's loop takes, or a failure of the interpreter itself.
pub enum Control {
    Throw(Value),
    Return(u64, Value),
    Tail(Vec<Value>, Option<Value>),
    Fail(Failure),
}

pub type R<T = Value> = Result<T, Control>;

pub enum Failure {
    /// The step budget ran out.
    Budget,
    /// The call depth passed the limit and no `StackOverflowError` could be made.
    Depth,
    /// An exception left the program.
    Thrown(Value),
    /// An IR shape or a builtin the interpreter lacks, or a program the typer could not type.
    Unsupported(String),
    /// A record that is gone was used: a tree or a method type of an earlier build, or a
    /// symbol of another expansion's site (`Records`).
    Stale(String),
    /// The typing of what the run reached met a body withheld from its products, an error the
    /// diagnostics name; the run stops before anything runs on past it.
    Withheld,
    /// The program called `System.exit` (`sys.exit`) with the status: it stops there, no `finally`
    /// running, as the JVM halts; `teq interp` exits with the status.
    Exit(i32),
}

impl From<Failure> for Control {
    fn from(f: Failure) -> Control {
        Control::Fail(f)
    }
}

pub struct Limits {
    pub steps: u64,
    pub depth: u32,
}

impl Limits {
    pub fn unlimited() -> Limits {
        Limits { steps: u64::MAX, depth: 100_000 }
    }
}

/// A builtin behind a `@js` template or a member the macro package supplies: it receives the
/// interpreter and the arguments, the receiver first for a member.
pub type Builtin = Rc<dyn Fn(&mut Interp<'_, '_>, &[Value]) -> R>;

/// What a template call resolved to, cached per template string.
#[derive(Clone)]
enum Template {
    Builtin(Builtin),
    /// A member with a builtin of an abstract class, or without a body (`Number.intValue`,
    /// `Number.byteValue`): the builtin answers a primitive receiver, an object answers through
    /// its own implementation.
    AbstractBuiltin(Builtin, SymId),
    /// The def has a Scala body, which runs in place of the template; the flag says whether the
    /// first argument is a receiver.
    Body(SymId, bool),
    Missing(Rc<str>),
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct FieldKey(pub Name, pub u32);

/// What a member name resolves to on a class, in the order the JavaScript prototype chain
/// answers it.
#[derive(Clone)]
pub enum Member {
    Fun(FunId, ClassId),
    /// An extension or private method outside a class, which runs as named.
    Ext(FunId),
    /// A templated member without a body: the builtin under its qualified name, and the name.
    Builtin(Builtin, Rc<str>),
    Field(FieldKey),
    /// The setter `x_=` of a var `x` that implements an abstract setter: it writes the field.
    SetField(FieldKey),
    Lazy(FieldKey, TExprId, ClassId),
    Module(ClassId),
    Static(SymId),
    CaseToString,
    CaseEquals,
    CaseHash,
    ValueEquals(FieldKey),
    ValueHash(FieldKey),
    AnyToString,
    AnyEquals,
    AnyHash,
    /// The JDK's `String.valueOf`, whose overload the call selected: `true` for the one that
    /// takes a `char[]`.
    StringValueOf(bool),
    Missing,
}

/// The shape a class registers with the runtime: what its `toString`, `equals` and `hashCode`
/// by rule cover.
#[derive(Clone)]
pub enum Shape {
    Plain,
    /// A case class: its fields, and the name it prints and hashes under (empty for a tuple).
    Case(Rc<[FieldKey]>, Rc<str>),
    CaseObject(Rc<str>),
    EnumValue,
}

thread_local! {
    /// What the interpreter cached over the macro expansions of this thread so far
    /// (`InterpCaches`), taken for a run and put back after it. Per thread, as the builtins
    /// table is: the module instances are `Rc` values, and each thread that expands macros
    /// builds its own (the typing thread, `typer/thread.rs`, keeps them across the builds and
    /// retypes of a session); dropped by `Typer::run` for a new program. Two sets while a worker
    /// types with the type store's views on (`partition_by_view`): the worker's, outside the
    /// loader's lock, and the holder's, under it, each holding its view's types; one otherwise.
    static CACHES: RefCell<[Option<Box<InterpCaches>>; 2]> = const { RefCell::new([None, None]) };
    /// Whether this thread's caches are kept apart by view (`partition_by_view`).
    static PARTITIONED: Cell<bool> = const { Cell::new(false) };
    /// What `Interp::keep_cacheable_state` has to say, for the build to tell once its own
    /// diagnostics are settled: a run's may be dropped with the run (a fold's, a speculative
    /// typing's).
    static CACHEABLE_WARNINGS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    /// The objects with cacheable state dropped once with a warning, by name: told once a session,
    /// whatever becomes of the caches (a full build, a run out of its budget).
    static WARNED_CACHEABLE: RefCell<FxMap<String, ()>> = RefCell::new(FxMap::default());
}

/// Whether the macros' runs are watched for the state they share (`watch`): while several
/// workers type the bodies, or for the census (`TEQ_MACRO_CENSUS`).
static WATCHING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

thread_local! {
    /// The epoch under way on this thread: a macro's run, or a module's, an enum value's, a
    /// file's or a lazy val's initialiser inside one; 0 outside every run.
    static EPOCH: Cell<u32> = const { Cell::new(0) };
    static NEXT_EPOCH: Cell<u32> = const { Cell::new(1) };
    /// The first state shared with other runs that the run under way touched.
    static TOUCHED: RefCell<Option<Touched>> = const { RefCell::new(None) };
    /// What each epoch is the initialiser or the run of, where the census or the watch names it:
    /// what a change of a value the epoch made names.
    static EPOCH_OWNERS: RefCell<FxMap<u32, (Rc<str>, EpochOwner)>> = RefCell::new(FxMap::default());
    /// The stamped containers an epoch changed without the change being reported (its own, or
    /// a cacheable state's), each with that epoch, whose new contents the epoch's end stamps
    /// with it (`leave_epoch`); an epoch nested in another leaves the outer one's entries.
    static DIRTY: RefCell<Vec<(u32, u32, Node)>> = const { RefCell::new(Vec::new()) };
    /// The containers made on this thread while the watch is on, the last one's number: a
    /// container's `made` (`made_now`), which the stamps do not change.
    static MADE: Cell<u32> = const { Cell::new(1) };
    /// The initialisers under way (`enter_init`), innermost last: the count of `MADE` when each
    /// began and what it initialises.
    static INITS: RefCell<Vec<(u32, Rc<str>)>> = const { RefCell::new(Vec::new()) };
    /// The innermost initialiser's count of `MADE` when it began, 0 outside every initialiser:
    /// a container made up to it existed before the initialiser.
    static INIT_START: Cell<u32> = const { Cell::new(0) };
    /// The count of `MADE` when the run under way began (`enter_run`), 0 outside every run: an
    /// unstamped container made after it is the run's own.
    static RUN_START: Cell<u32> = const { Cell::new(0) };
    /// The identity hashes the run under way asked and the ones it computed, for the census.
    static RUN_HASHES: Cell<(u64, u64)> = const { Cell::new((0, 0)) };
}

/// The identity hashes the run under way asked and computed so far (`enter_run`).
pub fn run_hashes() -> (u64, u64) {
    RUN_HASHES.with(|h| h.get())
}

/// Counts an identity hash the run under way asked, computed when `computed`.
pub(super) fn hash_asked(computed: bool) {
    RUN_HASHES.with(|h| {
        let (a, c) = h.get();
        h.set((a + 1, c + computed as u64));
    });
}

/// A macro's run or a constant's folding under way (`enter_run`), left when the guard drops.
pub struct RunGuard {
    outer: u32,
    hashes: (u64, u64),
    hashed: Option<FxMap<(u64, i32), Vec<identity::Held>>>,
}

impl Drop for RunGuard {
    fn drop(&mut self) {
        RUN_START.with(|s| s.set(self.outer));
        RUN_HASHES.with(|h| h.set(self.hashes));
        if let Some(h) = self.hashed.take() {
            identity::give_hashed(h);
        }
    }
}

/// Begins a macro's run or a constant's folding, for what its initialisers may change: a
/// container the run made and no initialiser's end stamped is the run's own, which no other run
/// reaches, since a value becomes another run's only through a change of stamped state (itself
/// reported) or through a cacheable state, which the run's end stamps (`watch`).
pub fn enter_run() -> Option<RunGuard> {
    if !watching() {
        return None;
    }
    let start = MADE.with(|m| m.get());
    let hashed = crate::measure::on().then(identity::take_hashed);
    Some(RunGuard { outer: RUN_START.with(|s| s.replace(start)), hashes: RUN_HASHES.with(|h| h.replace((0, 0))), hashed })
}

/// The number of a container made now, 0 while nothing is watched: what an initialiser tells
/// its own values from the ones that existed before it began by (`enter_init`). The count stops
/// at its largest value, after which every container counts as existing before every
/// initialiser, which gives the build away at an initialiser's first write.
#[inline]
pub(super) fn made_now() -> u32 {
    if !watching() {
        return 0;
    }
    MADE.with(|m| {
        let n = m.get().saturating_add(1);
        m.set(n);
        n
    })
}

/// An initialiser under way (a module's, a file's, an enum value's, a lazy val's or a lazy
/// local's, and a top-level val's read on its own): the context the watch judges its writes
/// in, left when the guard drops, on every exit.
pub(super) struct InitGuard {
    outer: u32,
}

impl Drop for InitGuard {
    fn drop(&mut self) {
        INITS.with(|i| i.borrow_mut().pop());
        INIT_START.with(|s| s.set(self.outer));
    }
}

/// Begins an initialiser, named by `name` for the build's give-way. Its one destination is
/// the slot it initialises, which the caller writes as a
/// publication (`publish`); it may make values and change them, and a change of any container
/// made before it began, whether or not its value reaches the container, is a change of state
/// other runs share, which gives the build to one worker: a module's or another value's state
/// that an earlier run may have initialised on this worker's heap and not on another's. Two
/// kinds of container are no such state: the run's own (`enter_run`) and a cacheable state's.
/// Initialisers nest; the innermost is the one a write is judged in, since what the innermost
/// may change the outer ones may too.
pub(super) fn enter_init<N: Into<Rc<str>>>(name: impl FnOnce() -> N) -> Option<InitGuard> {
    if !watching() {
        return None;
    }
    let start = MADE.with(|m| m.get());
    INITS.with(|i| i.borrow_mut().push((start, name().into())));
    Some(InitGuard { outer: INIT_START.with(|s| s.replace(start)) })
}

/// Runs `f`, an initialiser's publication of its value in its slot (a lazy val's field, a lazy
/// local's place in its frame), outside every initialiser's judgement: the one write the
/// initialisers around it may not make, and the one this initialiser exists for.
pub(super) fn publish<T>(f: impl FnOnce() -> T) -> T {
    let start = INIT_START.with(|s| s.replace(0));
    let r = f();
    INIT_START.with(|s| s.set(start));
    r
}

/// Records what the epoch under way is, named `name`: a module's initialiser, whose module holds
/// cacheable state when its `Cache` says so (`--cacheable-state`, nothing but a cache, whose
/// changes are no state another run reads differently), or another initialiser or a run.
pub(super) fn own_epoch(owner: EpochOwner, name: impl FnOnce() -> String) {
    if watching() {
        let e = EPOCH.with(|e| e.get());
        EPOCH_OWNERS.with(|o| o.borrow_mut().insert(e, (Rc::from(name()), owner)));
    }
}

/// Whether an initialiser's module holds nothing but a cache: declared so (`--cacheable-state`),
/// or one of the libraries' objects whose code teq read (`typer::KNOWN_CACHES`), whose reachable
/// state is the cache's whole.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Cache {
    None,
    Declared,
    Known,
}

/// What an epoch is, as the watch reads it.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum EpochOwner {
    /// A module's initialiser, with what `--cacheable-state` and the known caches say of its state.
    Module(Cache, ClassId),
    /// A macro's run, or a file's, a top-level val's or an enum value's initialiser.
    Other,
}

/// Names the epoch under way for the census, a macro's run: what a change of a value it left
/// stamped names (`made_by`).
pub fn name_epoch(name: impl FnOnce() -> String) {
    own_epoch(EpochOwner::Other, name);
}

/// Where a value of epoch `born` was made, for the census: " of <module>'s initialiser".
pub(super) fn made_by(born: u32) -> String {
    EPOCH_OWNERS.with(|o| o.borrow().get(&born).map_or(String::new(), |(n, _)| if n.starts_with("the run of ") { format!(" left by {}", n) } else { format!(" made by the initialiser of {}", n) }))
}

/// Watches the macros' runs:
/// every worker's interpreter has a heap of its own, so a run that changes what another run
/// made (a value an earlier run made, a module's state after its initialiser) changes what in
/// one worker's order, scalac's, other runs read and write. A module's, an enum value's, a
/// file's and a lazy val's initialiser run in an epoch of their own, the value's: what an
/// initialiser makes and writes is the value's making, the same in every worker's heap as long
/// as no run changes it after.
///
/// The values stay unstamped (epoch 0) while they are their epoch's alone, and the end of an
/// initialiser stamps what it made reachable from the value with its epoch (`stamp`): a value
/// a run makes becomes another run's only through a change of stamped state, which is itself
/// the change the watch reports, or through a cacheable state's containers, whose new contents
/// the end of the run stamps. A change of a stamped value of another epoch, and an identity hash
/// given to one, is state the runs share. So is a change an initialiser makes of a container
/// made before it began (`enter_init`), which the stamps cannot tell: they say which epoch made a
/// value, not whether it existed when the initialiser began, and a lazy val's initialiser goes on
/// in its object's epoch. Without the watch nothing is stamped or counted, and a change costs
/// the test of its value's stamp and count.
pub fn watch(on: bool) {
    WATCHING.store(on, std::sync::atomic::Ordering::Release);
}

pub fn watching() -> bool {
    WATCHING.load(std::sync::atomic::Ordering::Relaxed)
}

/// The epoch under way: 0 while nothing is watched.
pub fn epoch() -> u32 {
    if !watching() {
        return 0;
    }
    EPOCH.with(|e| e.get())
}

/// Begins a macro's run or an initialiser in a fresh epoch; `leave_epoch` with what it
/// returns ends it.
pub fn enter_epoch() -> u32 {
    if !watching() {
        return 0;
    }
    let fresh = NEXT_EPOCH.with(|n| {
        let e = n.get();
        n.set(e.wrapping_add(1).max(1));
        e
    });
    EPOCH.with(|e| e.replace(fresh))
}

/// Resumes the epoch `born`, a stamped value's, for the rest of its initialisation (a lazy
/// val's); an unstamped value's is the epoch under way, which goes on.
pub(super) fn resume_epoch(born: u32) -> u32 {
    if !watching() {
        return 0;
    }
    EPOCH.with(|e| if born == 0 { e.get() } else { e.replace(born) })
}

/// Ends the epoch under way, whose unreported changes of stamped containers leave their new
/// contents stamped with it, and goes back to `outer`.
pub fn leave_epoch(outer: u32) {
    if watching() {
        let now = EPOCH.with(|e| e.get());
        loop {
            let last = DIRTY.with(|d| {
                let mut d = d.borrow_mut();
                if d.last().is_some_and(|&(e, ..)| e == now) { d.pop() } else { None }
            });
            let Some((_, stamp, container)) = last else { break };
            stamp_contents(&container, stamp);
        }
        EPOCH.with(|e| e.set(outer));
    }
}

/// A change of a value stamped `born`, 0 for one no epoch stamped: state other runs share
/// when the value is another epoch's and the change is in a run. `node` is the value or frame
/// changed, whose new contents the epoch's end stamps where the change is not reported: a
/// change in the epoch that stamped it (a lazy val's initialiser writing its object's other
/// fields), or of a cacheable state.
///
/// `made` is when the container was made (`made_now`): a change inside an initialiser of a
/// container made before it began is state other runs share (`enter_init`), a cacheable
/// state's aside.
#[inline]
/// `what` names the container, as another run's when its argument is true.
pub(super) fn written(born: u32, made: u32, node: impl FnOnce() -> Option<Node>, what: impl FnOnce(bool) -> String) {
    if born | made != 0 {
        written_watched(born, made, node, what);
    }
}

#[inline(never)]
fn written_watched(born: u32, made: u32, node: impl FnOnce() -> Option<Node>, what: impl FnOnce(bool) -> String) {
    if !watching() {
        return;
    }
    let start = INIT_START.with(|s| s.get());
    if start != 0 && made <= start && !run_own(born, made) && !(born != 0 && cacheable(born)) {
        let name = INITS.with(|i| i.borrow().last().map_or_else(|| Rc::from(""), |(_, n)| n.clone()));
        touched(born, || format!("{} in the initialiser of {}, made before the initialiser began", what(false), name));
        return;
    }
    if born == 0 {
        return;
    }
    let now = EPOCH.with(|e| e.get());
    if now == 0 {
        return;
    }
    if born == now || cacheable(born) {
        // What a run puts in a known cache's container becomes the cache's, stamped with its
        // epoch: a pooled buffer a later run takes and fills is still the pool's. A declared
        // cache vouches for its own containers alone, and what a run puts there stays the run's.
        if let Some(n) = node() {
            let stamp = if born != now && known_cache(born) { born } else { now };
            DIRTY.with(|d| d.borrow_mut().push((now, stamp, n)));
        }
        return;
    }
    touched(born, || what(true));
}

/// Whether a container stamped `born` and made at `made` is the run's under way alone
/// (`enter_run`).
fn run_own(born: u32, made: u32) -> bool {
    let start = RUN_START.with(|s| s.get());
    born == 0 && start != 0 && made > start
}

/// Whether values of epoch `born` are a cacheable state's, which its module's initialiser made.
fn cacheable(born: u32) -> bool {
    EPOCH_OWNERS.with(|o| o.borrow().get(&born).is_some_and(|&(_, w)| matches!(w, EpochOwner::Module(Cache::Declared | Cache::Known, _))))
}

/// Whether values of epoch `born` are a known cache's (`Cache::Known`).
fn known_cache(born: u32) -> bool {
    EPOCH_OWNERS.with(|o| o.borrow().get(&born).is_some_and(|&(_, w)| matches!(w, EpochOwner::Module(Cache::Known, _))))
}

/// The module whose initialiser made the values of epoch `born`, by name and class, where it is
/// neither declared a cache nor a known one.
fn undeclared_module(born: u32) -> Option<(Rc<str>, ClassId)> {
    EPOCH_OWNERS.with(|o| match o.borrow().get(&born) {
        Some((name, EpochOwner::Module(Cache::None, c))) => Some((name.clone(), *c)),
        _ => None,
    })
}

#[cold]
#[inline(never)]
fn touched(born: u32, what: impl FnOnce() -> String) {
    TOUCHED.with(|t| {
        let mut t = t.borrow_mut();
        if t.is_none() {
            *t = Some(Touched { what: what(), module: undeclared_module(born) });
        }
    });
}

/// The first state shared with other runs that a run touched: its description, and the undeclared
/// module whose initialiser made it, by name and class.
pub struct Touched {
    pub what: String,
    pub module: Option<(Rc<str>, ClassId)>,
}

/// What the runs since the last call touched of the state they share, the first thing.
pub fn take_touched() -> Option<Touched> {
    TOUCHED.with(|t| t.borrow_mut().take())
}

/// What a stamp walks: a value or a frame.
pub(super) enum Node {
    Value(Value),
    Frame(Rc<Frame>),
}

/// Stamps what `v` reaches that no epoch stamped yet with the epoch `e`: the end of the
/// initialiser that made it (`watch`). A value stamped already is another epoch's, or this
/// one's, and what it reaches was stamped with it.
pub(super) fn stamp(v: &Value, e: u32) {
    if e == 0 || !watching() {
        return;
    }
    stamp_from(vec![Node::Value(v.clone())], e);
}

/// Stamps what the stamped container `n` holds and no epoch stamped yet with `e`.
fn stamp_contents(n: &Node, e: u32) {
    if e == 0 {
        return;
    }
    let mut todo = Vec::new();
    match n {
        Node::Value(v) => children(v, &mut todo),
        Node::Frame(f) => frame_children(f, &mut todo),
    }
    stamp_from(todo, e);
}

/// A frame's locals and its parent.
fn frame_children(f: &Rc<Frame>, todo: &mut Vec<Node>) {
    if let Ok(vals) = f.vals.try_borrow() {
        todo.extend(vals.iter().map(|(_, v)| Node::Value(v.clone())));
    }
    if let Some(p) = &f.parent {
        todo.push(Node::Frame(p.clone()));
    }
}

fn stamp_from(mut todo: Vec<Node>, e: u32) {
    while let Some(n) = todo.pop() {
        match n {
            Node::Frame(f) => {
                if f.born.get() == 0 {
                    f.born.set(e);
                    frame_children(&f, &mut todo);
                }
            }
            Node::Value(v) => {
                let born = match &v {
                    Value::Obj(o) => Some(&o.born),
                    Value::Array(a) => Some(&a.born),
                    Value::Map(m) => Some(&m.born),
                    _ => None,
                };
                match born {
                    Some(b) if b.get() != 0 => {}
                    Some(b) => {
                        b.set(e);
                        children(&v, &mut todo);
                    }
                    None => children(&v, &mut todo),
                }
            }
        }
    }
}

/// What a value holds that a change can reach: an object's fields and scope, an array's
/// items, a map's keys and values, a closure's scope and receiver, a trie node's content.
fn children(v: &Value, todo: &mut Vec<Node>) {
    match v {
        Value::Obj(o) => {
            if let Ok(fields) = o.fields.try_borrow() {
                todo.extend(fields.iter().map(|f| Node::Value(f.clone())));
            }
            if let Some(env) = &o.env {
                todo.push(Node::Frame(env.clone()));
            }
        }
        Value::Array(a) => {
            if let Ok(items) = a.try_borrow() {
                todo.extend(items.iter().map(|i| Node::Value(i.clone())));
            }
        }
        Value::Map(m) => {
            if let Ok(map) = m.try_borrow() {
                for (k, v) in map.entries.iter().flatten() {
                    todo.push(Node::Value(k.clone()));
                    todo.push(Node::Value(v.clone()));
                }
            }
        }
        Value::Fun(c) => {
            todo.push(Node::Frame(c.env.clone()));
            todo.push(Node::Value(c.this.clone()));
            if let value::ClosureKind::Partial(a, b) = &c.kind {
                todo.push(Node::Value(a.clone()));
                todo.push(Node::Value(b.clone()));
            }
        }
        Value::Trie(t) => todo.extend(t.content.iter().map(|c| Node::Value(c.clone()))),
        _ => {}
    }
}

/// The set of caches a run on this thread takes and keeps: the holder's under the loader's lock
/// while they are kept apart by view, the worker's (the only one otherwise) elsewhere.
fn cache_set() -> usize {
    (PARTITIONED.with(|p| p.get()) && crate::shared::lock_depth() > 0) as usize
}

/// The caches of this thread's macro expansions so far in the view the run is in, if a run
/// left any.
pub fn take_caches() -> Option<Box<InterpCaches>> {
    let set = cache_set();
    #[cfg(debug_assertions)]
    if set == 1 {
        crate::types::view::crossed(crate::types::view::Crossing::HolderRun, 1);
    }
    CACHES.with(|c| c.borrow_mut()[set].take())
}

/// Keeps `caches` for the next run on this thread in the view the run is in; `None` drops what
/// was kept there.
pub fn keep_caches(caches: Option<Box<InterpCaches>>) {
    let set = cache_set();
    CACHES.with(|c| c.borrow_mut()[set] = caches);
}

/// Drops every cache this thread kept, both views': where the program's ids change (the
/// workers' merge) or the program is replaced.
pub fn drop_caches() {
    CACHES.with(|c| *c.borrow_mut() = [None, None]);
}

/// Keeps this thread's caches apart by view from here, or no more (a worker's work with the
/// type store's views on, `Worker::work`): a run under the loader's lock takes and keeps the
/// holder's, whose types are the base's, and one outside it the worker's.
pub fn partition_by_view(on: bool) {
    PARTITIONED.with(|p| p.set(on));
}

/// Drops what the interpreter kept on this thread and frees the cycles its runs left
/// (`registry.rs`): where a program is replaced, where a session's full build ends, and before a
/// thread that ran macros ends. Nothing a run stamped outlives it, so the watch's numbering
/// starts again: a session's typing thread would otherwise number on through every forked build,
/// its count of containers saturating, after which every write of a run reads as a change of
/// shared state and every attempt gives way.
pub fn dispose() {
    drop_caches();
    registry::sweep();
    debug_assert!(EPOCH.with(|e| e.get()) == 0 && DIRTY.with(|d| d.borrow().is_empty()) && INITS.with(|i| i.borrow().is_empty()), "the interpreter disposed of in a run");
    EPOCH_OWNERS.with(|o| *o.borrow_mut() = FxMap::default());
    NEXT_EPOCH.with(|n| n.set(1));
    MADE.with(|m| m.set(1));
}

/// What the interpreter keeps on this thread, by name, for `TEQ_SESSION_INVENTORY`.
pub fn inventory() -> Vec<(&'static str, usize)> {
    let registry = registry::stats();
    vec![
        ("interp caches", CACHES.with(|c| c.borrow().iter().filter(|s| s.is_some()).count())),
        ("interp partitioned", PARTITIONED.with(|p| p.get() as usize)),
        ("cacheable warnings pending", CACHEABLE_WARNINGS.with(|w| w.borrow().len())),
        ("cacheable modules told", WARNED_CACHEABLE.with(|w| w.borrow().len())),
        ("watching", watching() as usize),
        ("epoch", EPOCH.with(|e| e.get() as usize)),
        ("next epoch", NEXT_EPOCH.with(|e| e.get() as usize)),
        ("touched", TOUCHED.with(|t| t.borrow().is_some() as usize)),
        ("epoch owners", EPOCH_OWNERS.with(|o| o.borrow().len())),
        ("dirty containers", DIRTY.with(|d| d.borrow().len())),
        ("containers made", MADE.with(|m| m.get() as usize)),
        ("initialisers under way", INITS.with(|i| i.borrow().len())),
        ("run start", RUN_START.with(|r| r.get() as usize)),
        ("macro files", files::count()),
        ("identity hashes", identity::hashed()),
        ("registry live", registry.registered),
        ("registry frames", registry.frames),
        ("registry objects", registry.objects),
    ]
}

/// The warnings the builds' runs left about the objects with cacheable state, drained.
pub fn take_cacheable_warnings() -> Vec<String> {
    CACHEABLE_WARNINGS.with(|w| std::mem::take(&mut *w.borrow_mut()))
}

/// Drops the files the macros of the last build read (`files`), for a build that starts.
pub fn forget_files() {
    files::forget();
}

/// What running code made, apart from what is read off the typed program: the instances of
/// the objects, the values of the top-level vals and enum cases, and the files whose vals were
/// initialised. The tables of `InterpCaches` beside it say what the program is; this says what
/// happened, a macro's own state among it, which lasts a build (`Interp::forget_made`).
pub struct Made {
    modules: crate::arena::Dense<Option<Rc<Object>>>,
    /// Per module the run that made it (`Records::runs`), for what asks whether an object is
    /// the run's own or an earlier one's.
    module_runs: crate::arena::Dense<u32>,
    statics: FxMap<SymId, Value>,
    static_runs: FxMap<SymId, u32>,
    /// Per static the epoch of the initialiser that made its value (`watch`).
    static_born: FxMap<SymId, u32>,
    /// Per static when its slot was made, while the watch is on (`made_now`): a file's
    /// initialiser may change the slots it made, and no other.
    static_made: FxMap<SymId, u32>,
    file_done: Vec<bool>,
    /// Per file: whether an access of its definitions runs nothing of its initialiser's but
    /// the reads of its vals, as a std file's (`init_file_of`).
    file_quiet: Vec<bool>,
}

impl Made {
    fn module(&self, c: ClassId) -> Option<&Rc<Object>> {
        self.modules.get(c.idx()).and_then(|m| m.as_ref())
    }

    fn set_module(&mut self, c: ClassId, obj: Rc<Object>, run: u32, _classes: usize) {
        *self.modules.at(c.idx()) = Some(obj);
        *self.module_runs.at(c.idx()) = run;
    }

    fn clear_module(&mut self, c: ClassId) {
        if let Some(m) = self.modules.get_mut(c.idx()) {
            *m = None;
        }
    }

    fn insert_static(&mut self, s: SymId, v: Value, run: u32) {
        self.statics.insert(s, v);
        self.static_runs.insert(s, run);
        if watching() {
            self.static_made.entry(s).or_insert_with(made_now);
        }
    }

    /// Stamps the static `s` and what its value reaches with the epoch `e` of its initialiser
    /// (`watch`).
    fn stamp_static(&mut self, s: SymId, e: u32) {
        if e == 0 {
            return;
        }
        self.static_born.insert(s, e);
        if let Some(v) = self.statics.get(&s) {
            stamp(v, e);
        }
    }

    /// The run that made the instance of object `c`, `None` while none is made: for the
    /// parallel typer's workers, which expand macros each in a run of their own.
    #[allow(dead_code)]
    pub fn made_in_run(&self, c: ClassId) -> Option<u32> {
        self.module(c).map(|_| self.module_runs[c.idx()])
    }

    /// The run that made the value of the top-level val or enum value `s`.
    #[allow(dead_code)]
    pub fn static_made_in_run(&self, s: SymId) -> Option<u32> {
        self.statics.get(&s).and_then(|_| self.static_runs.get(&s).copied())
    }
}

/// Whether a value is a holder of values that a walk marked (`walk_reach`).
fn holder_marked(v: &Value, marked: &FxMap<usize, ()>) -> bool {
    let at = match v {
        Value::Obj(o) => Rc::as_ptr(o) as usize,
        Value::Fun(c) => Rc::as_ptr(c) as usize,
        Value::Array(a) => Rc::as_ptr(a) as usize,
        Value::Map(m) => Rc::as_ptr(m) as usize,
        Value::Trie(t) => Rc::as_ptr(t) as usize,
        _ => return false,
    };
    marked.contains_key(&at)
}

/// Marks in `seen`, by the address of its holder, everything `root` reaches: the fields of
/// objects, the frames closures and anonymous classes capture, arrays, maps and trie nodes,
/// each looked at once; and says whether a quoted value (a tree, a type, a symbol, a position,
/// a source file) is among what is reached.
fn walk_reach(root: &Value, seen: &mut FxMap<usize, ()>) -> bool {
    enum Holder {
        Obj(Rc<Object>),
        Fun(Rc<value::Closure>),
        Array(Rc<value::ArrayCell>),
        Map(Rc<value::MapCell>),
        Trie(Rc<value::Trie>),
        Frame(Rc<Frame>),
    }
    struct Walk<'s> {
        stack: Vec<Holder>,
        seen: &'s mut FxMap<usize, ()>,
        quoted: bool,
    }
    impl<'s> Walk<'s> {
        #[inline]
        fn look(&mut self, v: &Value) {
            match v {
                Value::Tree(_) | Value::Type(_) | Value::Sym(_) | Value::Pos(..) | Value::Src(_) => self.quoted = true,
                Value::Obj(o) => {
                    if self.seen.insert(Rc::as_ptr(o) as usize, ()).is_none() {
                        self.stack.push(Holder::Obj(o.clone()));
                    }
                }
                Value::Fun(c) => {
                    if self.seen.insert(Rc::as_ptr(c) as usize, ()).is_none() {
                        self.stack.push(Holder::Fun(c.clone()));
                    }
                }
                Value::Array(a) => {
                    if self.seen.insert(Rc::as_ptr(a) as usize, ()).is_none() {
                        self.stack.push(Holder::Array(a.clone()));
                    }
                }
                Value::Map(m) => {
                    if self.seen.insert(Rc::as_ptr(m) as usize, ()).is_none() {
                        self.stack.push(Holder::Map(m.clone()));
                    }
                }
                Value::Trie(t) => {
                    if self.seen.insert(Rc::as_ptr(t) as usize, ()).is_none() {
                        self.stack.push(Holder::Trie(t.clone()));
                    }
                }
                _ => {}
            }
        }

        fn frame(&mut self, f: &Rc<Frame>) {
            if self.seen.insert(Rc::as_ptr(f) as usize, ()).is_none() {
                self.stack.push(Holder::Frame(f.clone()));
            }
        }
    }
    let mut w = Walk { stack: Vec::new(), seen, quoted: false };
    w.look(root);
    while let Some(h) = w.stack.pop() {
        match h {
            Holder::Obj(o) => {
                for v in o.fields.borrow().iter() {
                    w.look(v);
                }
                if let Some(env) = &o.env {
                    w.frame(env);
                }
            }
            Holder::Fun(c) => {
                w.look(&c.this);
                w.frame(&c.env);
                if let value::ClosureKind::Partial(a, b) = &c.kind {
                    w.look(a);
                    w.look(b);
                }
            }
            Holder::Array(a) => {
                for v in a.borrow().iter() {
                    w.look(v);
                }
            }
            Holder::Map(m) => {
                for (k, v) in m.borrow().entries.iter().flatten() {
                    w.look(k);
                    w.look(v);
                }
            }
            Holder::Trie(t) => {
                for v in t.content.iter() {
                    w.look(v);
                }
            }
            Holder::Frame(f) => {
                for (_, v) in f.vals.borrow().iter() {
                    w.look(v);
                }
                if let Some(p) = &f.parent {
                    w.frame(p);
                }
            }
        }
    }
    w.quoted
}

/// The entries of one build, numbered on from those of the builds before: an id names an
/// entry of this build or of one that is gone, never one of another build's at the same
/// place. The ids of a session wrap after four billion entries.
pub struct Store<T> {
    first: u32,
    items: Vec<T>,
}

impl<T> Default for Store<T> {
    fn default() -> Store<T> {
        Store { first: 0, items: Vec::new() }
    }
}

impl<T> Store<T> {
    fn push(&mut self, item: T) -> u32 {
        self.items.push(item);
        self.first.wrapping_add(self.items.len() as u32 - 1)
    }

    fn get(&self, id: u32) -> Option<&T> {
        self.items.get(id.wrapping_sub(self.first) as usize)
    }

    fn next_build(&mut self) {
        self.first = self.first.wrapping_add(self.items.len() as u32);
        self.items.clear();
    }
}

/// What the quoted values a macro makes refer to, apart from the typed program: the records
/// of a build, which a value made in one build and used in the next names no more
/// (`Interp::stale`). The program's arenas, which the other quoted values index (an
/// expression, a symbol, a class, a type), only grow within a session and hold their records
/// for the session; a full build drops the interpreter's caches whole. The owner chain of an
/// expansion (`MacroCtx::owners`) is the run's: the symbols of its site carry the run's
/// number and are stale in any other run.
#[derive(Default)]
pub struct Records {
    /// Selections and references that await their arguments (`TreeRef::Pending`).
    pending: Store<quoted::Pending>,
    /// The method types the reflect API hands out (`TypeRepr.memberType` of a constructor):
    /// parameter names and types, and the result, of the type `<method>N` stands for.
    method_types: Store<quoted::MethodTypeRepr>,
    /// Each method type of `method_types` by its parts, which it is made for once, so that
    /// equal method types are one type (`r.info =:= r.info`).
    method_type_ids: FxMap<quoted::MethodTypeRepr, crate::types::TypeId>,
    /// The polymorphic method types of the selections of generic methods, of the type
    /// `<poly>N` stands for.
    poly_types: Store<quoted::PolyTypeRepr>,
    /// Named arguments built by a macro (`TreeRef::NamedArg`): the parameter's name and the
    /// argument.
    named_args: Store<(Name, TreeRef)>,
    /// The inner applications of calls with several parameter clauses (`TreeRef::Partial`):
    /// the callee tree and the arguments of one clause.
    partials: Store<(TreeRef, Vec<TreeRef>)>,
    /// How many expansions the session ran so far: the number of the one under way.
    runs: u32,
}

impl Records {
    fn next_build(&mut self) {
        self.pending.next_build();
        self.method_types.next_build();
        self.method_type_ids.clear();
        self.poly_types.next_build();
        self.named_args.next_build();
        self.partials.next_build();
    }
}

impl Default for Made {
    fn default() -> Made {
        Made { modules: crate::arena::Dense::new(None), module_runs: crate::arena::Dense::new(0), statics: FxMap::default(), static_runs: FxMap::default(), static_born: FxMap::default(), static_made: FxMap::default(), file_done: Vec::new(), file_quiet: Vec::new() }
    }
}

/// The caches of an interpreter that outlive one run: the module instances, the layouts and
/// the indexes over the program, kept from one macro expansion to the next so that the quoted
/// std's objects are constructed once per build.
pub struct InterpCaches {
    coerce: crate::arena::Dense<u8>,
    local_slots: crate::arena::Dense<u32>,
    field_slots: crate::arena::Dense<(u32, u32)>,
    str_cache: crate::arena::Dense<Option<Rc<str>>>,
    made: Made,
    records: Records,
    file_vals: FxMap<FileId, Vec<(SymId, TExprId)>>,
    top_val_of: FxMap<SymId, TExprId>,
    /// The shared and the own top-level values read so far (`refresh_top_vals`).
    top_vals_seen: (usize, usize),
    tclass_of: crate::arena::Dense<u32>,
    layouts: crate::arena::Dense<Option<Box<FxMap<FieldKey, u32>>>>,
    /// Per class, the keys of its layout that are lazy vals' slots (`mark_lazy_slot`), which an
    /// identity hash leaves out (`identity.rs`).
    lazy_slots: crate::arena::Dense<Option<Box<FxMap<FieldKey, ()>>>>,
    field_keys: FxMap<SymId, FieldKey>,
    members: FxMap<(ClassId, Name), Member>,
    alt_members: FxMap<(ClassId, SymId), Member>,
    method_index: crate::arena::Dense<Option<Box<FxMap<Name, FunId>>>>,
    dispatch_syms: crate::arena::Dense<Option<Box<FxMap<Name, SymId>>>>,
    ext_names: FxMap<SymId, Name>,
    methods_seen: crate::arena::Dense<u32>,
    lazy_inits: FxMap<SymId, (ClassId, TExprId)>,
    lazy_seen: (usize, usize),
    shapes: crate::arena::Dense<Option<Shape>>,
    has_body: crate::arena::Dense<Option<bool>>,
    tail_of: crate::arena::Dense<u8>,
    template_cache: crate::arena::Dense<Option<Template>>,
    qnames: FxMap<SymId, Rc<str>>,
    class_values: FxMap<Rc<str>, Rc<ClassValueRef>>,
    known_classes: FxMap<&'static str, Option<ClassId>>,
    regex_cache: FxMap<String, Rc<regex::Regex>>,
    reflect_classes: FxMap<&'static str, Option<ClassId>>,
    path_classes: FxMap<String, Option<ClassId>>,
    call_sites: crate::arena::Dense<u32>,
    site_entries: Vec<(ClassId, Name, Member)>,
    pat_facts: crate::arena::Dense<u8>,
    pat_slots: crate::arena::Dense<(u32, u32)>,
    block_facts: crate::arena::Dense<u8>,
    ctor_plans: crate::arena::Dense<Option<Option<Rc<[u32]>>>>,
    macro_seen: crate::arena::Dense<bool>,
    macro_classes: crate::arena::Dense<bool>,
    frame_pool: Vec<Rc<Frame>>,
    vec_pool: Vec<Vec<Value>>,
    miss_closure: Option<Value>,
    partial_miss: Option<Value>,
    fun_natives: crate::arena::Dense<u32>,
    list_shape: Option<Option<Rc<natives::ListShape>>>,
    option_shape: Option<Option<Rc<natives::OptionShape>>>,
    plain_classes: FxMap<&'static str, Option<(ClassId, Rc<[u32]>)>>,
    named_slots: FxMap<(ClassId, &'static str), u32>,
    names: FxMap<&'static str, Name>,
    lambda_variances: FxMap<crate::types::TParamId, u64>,
    lambda_of: FxMap<crate::types::TParamId, crate::types::TypeId>,
    prof_state: Option<Box<profile::ProfState>>,
    /// `Program::shifts` when the caches were put away.
    shifts: u32,
    /// How many holders the last walk of the cacheable state marked: the size the next
    /// walk's table starts at.
    walk_hint: usize,
}

impl InterpCaches {
    /// Every value the interpreter holds from one build to the next, the roots of a sweep of
    /// the session's memory. The caches are taken apart field by field, so that a table added
    /// later has to say whether it holds values. Those left out hold none: the tables by
    /// expression, class, symbol and function hold positions, ids, names, slots and builtins;
    /// `records` holds trees and types by id; the frames of `frame_pool` and the vectors of
    /// `vec_pool` were emptied when they were pooled; `class_values` holds names. What a sweep at
    /// a retype would have to keep: `registry::sweep` runs only where a full build has dropped
    /// the caches.
    #[allow(dead_code)]
    pub fn roots(&self) -> Vec<Value> {
        let InterpCaches {
            coerce: _,
            local_slots: _,
            field_slots: _,
            str_cache: _,
            made,
            records: _,
            file_vals: _,
            top_val_of: _,
            top_vals_seen: _,
            tclass_of: _,
            layouts: _,
            lazy_slots: _,
            field_keys: _,
            members: _,
            alt_members: _,
            method_index: _,
            dispatch_syms: _,
            ext_names: _,
            methods_seen: _,
            lazy_inits: _,
            lazy_seen: _,
            shapes: _,
            has_body: _,
            tail_of: _,
            template_cache: _,
            qnames: _,
            class_values: _,
            known_classes: _,
            regex_cache: _,
            reflect_classes: _,
            path_classes: _,
            call_sites: _,
            site_entries: _,
            pat_facts: _,
            pat_slots: _,
            block_facts: _,
            ctor_plans: _,
            macro_seen: _,
            macro_classes: _,
            frame_pool: _,
            vec_pool: _,
            miss_closure,
            partial_miss,
            fun_natives: _,
            list_shape,
            option_shape,
            plain_classes: _,
            named_slots: _,
            names: _,
            lambda_variances: _,
            lambda_of: _,
            prof_state: _,
            shifts: _,
            walk_hint: _,
        } = self;
        let mut out: Vec<Value> = made.modules.iter().flatten().map(|o| Value::Obj(o.clone())).collect();
        out.extend(made.statics.values().cloned());
        out.extend(miss_closure.iter().chain(partial_miss.iter()).cloned());
        if let Some(Some(list)) = list_shape {
            out.push(Value::Obj(list.nil.clone()));
        }
        if let Some(Some(option)) = option_shape {
            out.push(Value::Obj(option.none.clone()));
        }
        out
    }
}

/// The steps a run takes between two looks for a signal (`Interp::watch_signals`).
const SIGNAL_SLICE: u64 = 1 << 20;

pub struct Interp<'a, 't> {
    pub typer: &'a mut Worker<'t>,
    /// `registry::on()`, read once.
    resident: bool,
    steps: u64,
    /// The budget past `steps`, granted a slice at a time when the run watches for signals.
    reserve: u64,
    /// The typer's count of withheld bodies met when the run last looked (`note_withheld`), and
    /// whether one stopped the run.
    withheld_seen: u32,
    halted: bool,
    depth: u32,
    max_depth: u32,
    next_key: u64,
    /// How many names `Symbol.freshName` made in this run.
    pub(super) fresh_names: u32,
    /// Per expression the numeric kind its static type asks for, where it differs from what the
    /// IR produces: `0` for none.
    coerce: crate::arena::Dense<u8>,
    /// Per `Local` expression the frame depth and slot it was last found at, plus one.
    local_slots: crate::arena::Dense<u32>,
    /// Per `Field` expression the class of the last receiver and the slot of the field in it.
    field_slots: crate::arena::Dense<(u32, u32)>,
    str_cache: crate::arena::Dense<Option<Rc<str>>>,
    made: Made,
    records: Records,
    file_vals: FxMap<FileId, Vec<(SymId, TExprId)>>,
    top_val_of: FxMap<SymId, TExprId>,
    /// The shared and the own top-level values read so far (`refresh_top_vals`).
    top_vals_seen: (usize, usize),
    /// Per class the index of its `TClass` in `Program::classes`, `u32::MAX` for none.
    tclass_of: crate::arena::Dense<u32>,
    layouts: crate::arena::Dense<Option<Box<FxMap<FieldKey, u32>>>>,
    lazy_slots: crate::arena::Dense<Option<Box<FxMap<FieldKey, ()>>>>,
    field_keys: FxMap<SymId, FieldKey>,
    members: FxMap<(ClassId, Name), Member>,
    alt_members: FxMap<(ClassId, SymId), Member>,
    method_index: crate::arena::Dense<Option<Box<FxMap<Name, FunId>>>>,
    dispatch_syms: crate::arena::Dense<Option<Box<FxMap<Name, SymId>>>>,
    ext_names: FxMap<SymId, Name>,
    methods_seen: crate::arena::Dense<u32>,
    lazy_inits: FxMap<SymId, (ClassId, TExprId)>,
    lazy_seen: (usize, usize),
    shapes: crate::arena::Dense<Option<Shape>>,
    has_body: crate::arena::Dense<Option<bool>>,
    tail_of: crate::arena::Dense<u8>,
    builtins: Rc<FxMap<String, Builtin>>,
    template_cache: crate::arena::Dense<Option<Template>>,
    qnames: FxMap<SymId, Rc<str>>,
    class_values: FxMap<Rc<str>, Rc<ClassValueRef>>,
    known_classes: FxMap<&'static str, Option<ClassId>>,
    array_seq: Option<ClassId>,
    regex_cache: FxMap<String, Rc<regex::Regex>>,
    /// The classes and objects registered for Scala.js's reflective instantiation, by name, each
    /// an array of its class, its constructors or accessor, and the wrapper a lookup made; made
    /// when a lookup first asks.
    reflective: Option<Box<[FxMap<Rc<str>, Value>; 2]>>,
    pub out: String,
    /// Whether output goes to the process's stdout as it is made, or only to `out`.
    pub stream: bool,
    /// Whether an effect (output, a mutation of shared state, a random number) is a failure:
    /// what a compile-time evaluation asks for.
    pub pure: bool,
    /// Whether identity hashes are their values' contents, as every compile-time run's are, or
    /// the numbers of a sequence the interpreter gives out, distinct as the JVM's, as the program's
    /// own run under `teq interp` asks (`identity.rs`).
    pub content_hashes: bool,
    next_hash: i32,
    /// The macro expansion this interpreter runs, which `scala.quoted` reads the call site from.
    pub macro_ctx: Option<Rc<MacroCtx>>,
    /// The binders of the quotes being run, innermost last, each with the fresh local of that
    /// run: what a quote of a reference to one of them (`'y`) resolves to.
    pub(crate) quote_renames: Vec<FxMap<SymId, SymId>>,
    /// The classes of `scala.quoted.Reflect` by name, once looked up.
    reflect_classes: FxMap<&'static str, Option<ClassId>>,
    /// The classes the quoted layer names by their path (`java.nio.file.Path`), once looked up.
    path_classes: FxMap<String, Option<ClassId>>,
    /// The histogram of the profiled run under way (`profile.rs`); `None` outside `--profile`.
    prof: Option<Box<profile::ProfState>>,
    /// The histogram's state between runs.
    prof_state: Option<Box<profile::ProfState>>,
    /// Per call expression the entry of `site_entries` it resolved to last (`fast.rs`).
    call_sites: crate::arena::Dense<u32>,
    site_entries: Vec<(ClassId, Name, Member)>,
    /// Per pattern whether it binds: `0` unknown, `1` no, `2` yes.
    pat_facts: crate::arena::Dense<u8>,
    /// Per sub-pattern of a class pattern the class of the last object and the slot of its field.
    pat_slots: crate::arena::Dense<(u32, u32)>,
    /// Per block whether it is a case lambda's match over the tuple of its parameters
    /// (`tuple_match_shape`): `0` unknown, `1` no, `2` yes.
    block_facts: crate::arena::Dense<u8>,
    /// Per class the slots of a plain constructor, `Some(None)` where the constructor runs code.
    ctor_plans: crate::arena::Dense<Option<Option<Rc<[u32]>>>>,
    /// Per function whether a macro's run reached it, for `Worker::macro_files`.
    macro_seen: crate::arena::Dense<bool>,
    macro_classes: crate::arena::Dense<bool>,
    frame_pool: Vec<Rc<Frame>>,
    vec_pool: Vec<Vec<Value>>,
    miss_closure: Option<Value>,
    partial_miss: Option<Value>,
    /// Per function the native that runs in place of its body (`natives.rs`), once looked up.
    fun_natives: crate::arena::Dense<u32>,
    list_shape: Option<Option<Rc<natives::ListShape>>>,
    option_shape: Option<Option<Rc<natives::OptionShape>>>,
    store_shapes: Option<Option<Rc<natives::StoreShapes>>>,
    plain_classes: FxMap<&'static str, Option<(ClassId, Rc<[u32]>)>>,
    named_slots: FxMap<(ClassId, &'static str), u32>,
    /// The structural variance of each parameter of a type lambda whose `typeParams` a macro
    /// read, as scalac's `LambdaParam.paramVariance` gives it.
    lambda_variances: FxMap<crate::types::TParamId, u64>,
    /// The lambda each parameter a macro met belongs to: such a parameter is a `ParamRef`.
    lambda_of: FxMap<crate::types::TParamId, crate::types::TypeId>,
    /// The `Symbol.typeRef` of each val it was asked of: scalac's `TypeRef` to a term, whose
    /// `typeSymbol` is the term, where the `TermRef` of the same val has its class.
    val_type_refs: FxMap<crate::types::SymId, crate::types::TypeId>,
    /// The names the natives call members by, interned once.
    names: FxMap<&'static str, Name>,
    walk_hint: usize,
    trace: bool,
    /// The call depth the trace prints down to: `TEQ_INTERP_TRACE=<depth>`, 40 otherwise.
    trace_depth: u32,
    seed: u64,
}

type ClassValueRef = value::ClassValue;

impl<'a, 't> Interp<'a, 't> {
    pub fn new(typer: &'a mut Worker<'t>, limits: Limits) -> Interp<'a, 't> {
        let array_seq = typer.array_seq_class();
        let withheld_seen = typer.withheld_met;
        Interp {
            typer,
            resident: registry::on(),
            steps: limits.steps,
            reserve: 0,
            withheld_seen,
            halted: false,
            depth: 0,
            max_depth: limits.depth,
            next_key: 1,
            fresh_names: 0,
            coerce: crate::arena::Dense::new(value::K_UNKNOWN),
            local_slots: crate::arena::Dense::new(0),
            field_slots: crate::arena::Dense::new((u32::MAX, 0)),
            str_cache: crate::arena::Dense::new(None),
            made: Made::default(),
            file_vals: FxMap::default(),
            top_val_of: FxMap::default(),
            top_vals_seen: (0, 0),
            tclass_of: crate::arena::Dense::new(u32::MAX),
            layouts: crate::arena::Dense::new(None),
            lazy_slots: crate::arena::Dense::new(None),
            field_keys: FxMap::default(),
            members: FxMap::default(),
            alt_members: FxMap::default(),
            method_index: crate::arena::Dense::new(None),
            dispatch_syms: crate::arena::Dense::new(None),
            ext_names: FxMap::default(),
            methods_seen: crate::arena::Dense::new(0),
            lazy_inits: FxMap::default(),
            lazy_seen: (0, 0),
            shapes: crate::arena::Dense::new(None),
            has_body: crate::arena::Dense::new(None),
            tail_of: crate::arena::Dense::new(0),
            builtins: builtins::table(),
            template_cache: crate::arena::Dense::new(None),
            qnames: FxMap::default(),
            class_values: FxMap::default(),
            known_classes: FxMap::default(),
            array_seq,
            regex_cache: FxMap::default(),
            reflective: None,
            out: String::new(),
            stream: false,
            pure: false,
            content_hashes: true,
            next_hash: 1,
            macro_ctx: None,
            records: Records::default(),
            quote_renames: Vec::new(),
            reflect_classes: FxMap::default(),
            path_classes: FxMap::default(),
            prof: None,
            prof_state: None,
            call_sites: crate::arena::Dense::new(u32::MAX),
            site_entries: Vec::new(),
            pat_facts: crate::arena::Dense::new(0),
            pat_slots: crate::arena::Dense::new((u32::MAX, 0)),
            block_facts: crate::arena::Dense::new(0),
            ctor_plans: crate::arena::Dense::new(None),
            macro_seen: crate::arena::Dense::new(false),
            macro_classes: crate::arena::Dense::new(false),
            frame_pool: Vec::new(),
            vec_pool: Vec::new(),
            miss_closure: None,
            partial_miss: None,
            fun_natives: crate::arena::Dense::new(u32::MAX),
            list_shape: None,
            option_shape: None,
            store_shapes: None,
            plain_classes: FxMap::default(),
            named_slots: FxMap::default(),
            lambda_variances: FxMap::default(),
            lambda_of: FxMap::default(),
            val_type_refs: FxMap::default(),
            names: FxMap::default(),
            walk_hint: 0,
            trace: std::env::var_os("TEQ_INTERP_TRACE").is_some(),
            trace_depth: std::env::var("TEQ_INTERP_TRACE").ok().and_then(|v| v.parse().ok()).unwrap_or(40),
            seed: 0x2545F4914F6CDD1D,
        }
    }

    pub fn into_caches(self) -> InterpCaches {
        InterpCaches {
            coerce: self.coerce,
            local_slots: self.local_slots,
            field_slots: self.field_slots,
            str_cache: self.str_cache,
            made: self.made,
            records: self.records,
            file_vals: self.file_vals,
            top_val_of: self.top_val_of,
            top_vals_seen: self.top_vals_seen,
            tclass_of: self.tclass_of,
            layouts: self.layouts,
            lazy_slots: self.lazy_slots,
            field_keys: self.field_keys,
            members: self.members,
            alt_members: self.alt_members,
            method_index: self.method_index,
            dispatch_syms: self.dispatch_syms,
            ext_names: self.ext_names,
            methods_seen: self.methods_seen,
            lazy_inits: self.lazy_inits,
            lazy_seen: self.lazy_seen,
            shapes: self.shapes,
            has_body: self.has_body,
            tail_of: self.tail_of,
            template_cache: self.template_cache,
            qnames: self.qnames,
            class_values: self.class_values,
            known_classes: self.known_classes,
            regex_cache: self.regex_cache,
            reflect_classes: self.reflect_classes,
            path_classes: self.path_classes,
            call_sites: self.call_sites,
            site_entries: self.site_entries,
            pat_facts: self.pat_facts,
            pat_slots: self.pat_slots,
            block_facts: self.block_facts,
            ctor_plans: self.ctor_plans,
            macro_seen: self.macro_seen,
            macro_classes: self.macro_classes,
            frame_pool: self.frame_pool,
            vec_pool: self.vec_pool,
            miss_closure: self.miss_closure,
            partial_miss: self.partial_miss,
            fun_natives: self.fun_natives,
            list_shape: self.list_shape,
            option_shape: self.option_shape,
            plain_classes: self.plain_classes,
            named_slots: self.named_slots,
            lambda_variances: self.lambda_variances,
            lambda_of: self.lambda_of,
            names: self.names,
            prof_state: self.prof_state,
            shifts: self.typer.prog.shifts,
            walk_hint: self.walk_hint,
        }
    }

    pub fn restore(&mut self, c: InterpCaches) {
        self.coerce = c.coerce;
        self.local_slots = c.local_slots;
        self.field_slots = c.field_slots;
        self.str_cache = c.str_cache;
        self.made = c.made;
        self.records = c.records;
        self.file_vals = c.file_vals;
        self.top_val_of = c.top_val_of;
        self.top_vals_seen = c.top_vals_seen;
        self.tclass_of = c.tclass_of;
        self.layouts = c.layouts;
        self.lazy_slots = c.lazy_slots;
        self.field_keys = c.field_keys;
        self.members = c.members;
        self.alt_members = c.alt_members;
        self.method_index = c.method_index;
        self.dispatch_syms = c.dispatch_syms;
        self.ext_names = c.ext_names;
        self.methods_seen = c.methods_seen;
        self.lazy_inits = c.lazy_inits;
        self.lazy_seen = c.lazy_seen;
        self.shapes = c.shapes;
        self.has_body = c.has_body;
        self.tail_of = c.tail_of;
        self.template_cache = c.template_cache;
        self.qnames = c.qnames;
        self.class_values = c.class_values;
        self.known_classes = c.known_classes;
        self.regex_cache = c.regex_cache;
        self.reflect_classes = c.reflect_classes;
        self.path_classes = c.path_classes;
        self.call_sites = c.call_sites;
        self.site_entries = c.site_entries;
        self.pat_facts = c.pat_facts;
        self.pat_slots = c.pat_slots;
        self.block_facts = c.block_facts;
        self.ctor_plans = c.ctor_plans;
        self.macro_seen = c.macro_seen;
        self.macro_classes = c.macro_classes;
        self.frame_pool = c.frame_pool;
        self.vec_pool = c.vec_pool;
        self.miss_closure = c.miss_closure;
        self.partial_miss = c.partial_miss;
        self.fun_natives = c.fun_natives;
        self.list_shape = c.list_shape;
        self.option_shape = c.option_shape;
        self.plain_classes = c.plain_classes;
        self.named_slots = c.named_slots;
        self.lambda_variances = c.lambda_variances;
        self.lambda_of = c.lambda_of;
        self.names = c.names;
        self.prof_state = c.prof_state;
        self.walk_hint = c.walk_hint;
        if c.shifts != self.typer.prog.shifts {
            self.forget_positions();
            self.records.next_build();
            self.forget_made();
        }
    }

    /// The walk of the cacheable state: the objects declared to hold cacheable state
    /// (`Worker::cacheable_state_classes`) that are made and reach no record of a build that is
    /// gone, with everything each reaches marked by the address of its holder, which a build keeps
    /// with them. One that reaches a quoted value is dropped with the rest, told once a session as
    /// a warning (`take_cacheable_warnings`): what it holds would be a diagnostic at its next use
    /// (`Records`), and making it again is what the declaration allows.
    fn keep_cacheable_state(&mut self) -> FxMap<usize, ()> {
        let mut kept: FxMap<usize, ()> = FxMap::default();
        for c in self.typer.cacheable_state_classes.clone() {
            let Some(root) = self.made.module(c).cloned() else { continue };
            let mut reach: FxMap<usize, ()> = FxMap::default();
            reach.reserve(self.walk_hint);
            let quoted = walk_reach(&Value::Obj(root), &mut reach);
            self.walk_hint = self.walk_hint.max(reach.len());
            if quoted {
                let path = self.class_path(c);
                if WARNED_CACHEABLE.with(|w| w.borrow_mut().insert(path.clone(), ()).is_none()) {
                    let msg = format!("--cacheable-state {}: made anew, since it holds a tree, a type or a symbol of an earlier build", path);
                    CACHEABLE_WARNINGS.with(|w| w.borrow_mut().push(msg));
                }
                continue;
            }
            if kept.is_empty() {
                kept = reach;
            } else {
                kept.extend(reach);
            }
        }
        kept
    }

    /// At the first run of a retype, which is a build of its own: the objects and top-level values
    /// of the program's own files, and of every jar but the standard library's, are not made yet,
    /// as under scalac, whose macro class loader is made anew for every compiler run, a changed
    /// file's alone included, and finds only the compiler's own classes in its parent loader. A
    /// macro's state (a counter of its expansions, a cache) starts as a fresh build's does; what
    /// connects the expansions of several files is what the files typed in this build make of it.
    /// What the std's and scala-library's code made stays. So does what an object with cacheable
    /// state reaches (`keep_cacheable_state`), the instances of other objects and the values of
    /// top-level vals among it, so that a kept instance stays the one instance of its object; a
    /// file's other vals are made again by its initialiser, which passes over the kept ones.
    fn forget_made(&mut self) {
        let kept = self.keep_cacheable_state();
        let typer = &*self.typer;
        let own = |f: FileId| typer.is_program_file(f) || (typer.in_jar(f) && !typer.from_compiler_library(f));
        let made = &mut self.made;
        for (c, module) in made.modules.iter_mut_indexed() {
            let Some(o) = module else { continue };
            if own(typer.syms.class(ClassId(c as u32)).file) && !kept.contains_key(&(Rc::as_ptr(o) as usize)) {
                *module = None;
            }
        }
        made.statics.retain(|&s, v| !own(typer.syms.sym(s).file) || holder_marked(v, &kept));
        let statics = &made.statics;
        made.static_runs.retain(|s, _| statics.contains_key(s));
        made.static_made.retain(|s, _| statics.contains_key(s));
        for (f, done) in made.file_done.iter_mut().enumerate() {
            if own(FileId(f as u32)) {
                *done = false;
            }
        }
    }

    /// After a retype: the tables that hold positions in the program's lists, or were filled
    /// from a walk over them up to a count, are built again from the lists as they stand. The
    /// walks enter every class and top-level val of the program, read or not, so the entries of
    /// the file typed again are among what goes.
    fn forget_positions(&mut self) {
        self.tclass_of.clear();
        self.lazy_inits.clear();
        self.lazy_seen = (0, 0);
        self.top_val_of.clear();
        self.top_vals_seen = (0, 0);
        self.file_vals.clear();
    }

    #[inline]
    pub fn prog(&self) -> &Program {
        &self.typer.prog
    }

    #[inline]
    pub fn syms(&self) -> &Symbols {
        &self.typer.syms
    }

    pub fn name(&self, n: Name) -> &str {
        self.typer.interner.get(n)
    }

    #[allow(dead_code)]
    pub fn set_limits(&mut self, limits: Limits) {
        self.steps = limits.steps;
        self.max_depth = limits.depth;
    }

    /// Whether a run's failure is the call depth's limit: `Failure::Depth`, or an uncaught
    /// `StackOverflowError`, which the limit throws when the class is in the program (one the
    /// macro threw itself counts too).
    pub fn ended_on_depth(&mut self, f: &Failure) -> bool {
        match f {
            Failure::Depth => true,
            Failure::Thrown(Value::Obj(o)) => self.known_class("StackOverflowError") == Some(o.class),
            _ => false,
        }
    }

    pub fn steps_left(&self) -> u64 {
        self.steps + self.reserve
    }

    /// Has the run see a signal to `teq interp` (`process::handle_signals`) between slices of its
    /// budget, so that a loop that calls no native stops too.
    pub fn watch_signals(&mut self) {
        let total = self.steps + self.reserve;
        self.steps = total.min(SIGNAL_SLICE);
        self.reserve = total - self.steps;
    }

    /// The end of `teq interp`'s run, whatever ended it: the shutdown hooks the program registered
    /// (`Runtime.addShutdownHook`) run in the order they were registered, as the JVM's would at its
    /// exit, an exception in one printed as the JVM's thread prints it; a hook that calls
    /// `System.exit` ends the run there, with its status.
    pub fn shut_down(&mut self) -> Option<i32> {
        let hooks = process::take_hooks();
        for hook in hooks {
            match self.call_by_name(hook.clone(), "run", Vec::new()) {
                Ok(_) => {}
                Err(Control::Fail(Failure::Exit(status))) => return Some(status),
                Err(Control::Throw(e)) => {
                    let name = match self.call_by_name(hook, "getName", Vec::new()) {
                        Ok(Value::Str(n)) => n.to_string(),
                        _ => "Thread-0".to_string(),
                    };
                    let text = self.to_str(&e).unwrap_or_else(|_| "<unprintable exception>".to_string());
                    self.flush();
                    eprintln!("Exception in thread \"{}\" {}", name, text);
                }
                Err(Control::Fail(f)) => {
                    let message = self.describe(&f);
                    self.flush();
                    eprintln!("{}", message);
                }
                Err(_) => {}
            }
        }
        None
    }

    /// Registers a builtin under the qualified name of the def it stands for
    /// (`scala.Predef.println`, `scala.String.length`, `java.lang.Math.max`); an extension
    /// method carries the simple name of its receiver's class before its own.
    #[allow(dead_code)]
    pub fn register(&mut self, qualified: &str, f: Builtin) {
        Rc::make_mut(&mut self.builtins).insert(qualified.to_string(), f);
        self.template_cache.clear();
    }

    pub(super) fn impure<T>(&self, what: &str) -> R<T> {
        Err(Control::Fail(Failure::Unsupported(format!("{} is an effect, which a compile-time evaluation cannot have", what))))
    }

    /// Evaluates an expression in the scope, an empty one when none is given.
    pub fn eval_expr(&mut self, e: TExprId, env: Option<&Env>) -> Result<Value, Failure> {
        let owned;
        let env = match env {
            Some(env) => env,
            None => {
                owned = Env::empty();
                &owned
            }
        };
        let (frame, ctx) = (env.frame.clone(), env.ctx.clone());
        let r = self.eval_in(e, &frame, &ctx);
        self.finish(r)
    }

    fn eval_in(&mut self, e: TExprId, frame: &Rc<Frame>, ctx: &Ctx) -> R {
        self.depth = 0;
        self.eval(e, frame, ctx)
    }

    /// Calls a def, a method (its receiver first among the arguments) or a val by its symbol.
    #[allow(dead_code)]
    pub fn call(&mut self, sym: SymId, mut args: Vec<Value>) -> Result<Value, Failure> {
        self.depth = 0;
        let info = self.syms().sym(sym);
        let r = match (info.kind, info.owner) {
            (SymKind::Def | SymKind::Given, Owner::Class(_)) if !args.is_empty() && !info.is_extension => {
                let recv = args.remove(0);
                self.invoke(recv, sym, args)
            }
            (SymKind::Def | SymKind::Given, _) => self.call_static(sym, args),
            _ => self.static_value(sym),
        };
        self.finish(r)
    }

    /// The instance of an object, created on first use.
    pub fn module_value(&mut self, c: ClassId) -> Result<Value, Failure> {
        self.depth = 0;
        let r = self.module(c);
        self.finish(r)
    }

    /// A value as a message shows it.
    pub fn describe_value(&mut self, v: &Value) -> String {
        self.to_str(v).unwrap_or_else(|_| "<unprintable value>".to_string())
    }

    #[allow(dead_code)]
    pub fn call_method(&mut self, recv: Value, sym: SymId, args: Vec<Value>) -> Result<Value, Failure> {
        self.depth = 0;
        let r = self.invoke(recv, sym, args);
        self.finish(r)
    }

    fn finish(&mut self, r: R) -> Result<Value, Failure> {
        match r {
            Ok(v) => Ok(v),
            Err(Control::Throw(v)) => Err(Failure::Thrown(v)),
            Err(Control::Return(..)) => Err(Failure::Unsupported("a return outside its method".to_string())),
            Err(Control::Tail(..)) => Err(Failure::Unsupported("a tail call outside its method".to_string())),
            Err(Control::Fail(f)) => Err(f),
        }
    }

    /// Runs the program's entry point with the given arguments.
    pub fn run_main(&mut self, args: &[String]) -> Result<(), Failure> {
        let Some(main) = self.prog().main else {
            return Err(Failure::Unsupported("the program has no entry point".to_string()));
        };
        let info = self.syms().sym(main);
        let owner = match self.prog().main_object {
            Some(c) => Owner::Class(c),
            None => info.owner,
        };
        let file = info.file;
        let param = info.sig.as_ref().and_then(|sig| sig.clauses.iter().flat_map(|c| c.params.iter()).next()).map(|p| p.repeated);
        let r = (|| -> R {
            let strings: Vec<Value> = args.iter().map(|a| Value::str(a)).collect();
            let array = Value::array(strings);
            let call_args = match param {
                None => Vec::new(),
                Some(true) => vec![self.array_seq_of(array)?],
                Some(false) => vec![array],
            };
            match owner {
                Owner::Class(c) => {
                    let module = self.module(c)?;
                    self.invoke(module, main, call_args)
                }
                _ => {
                    self.run_file_init(file)?;
                    self.call_static(main, call_args)
                }
            }
        })();
        self.finish(r).map(|_| ())
    }

    /// The message of a failure as the JVM prints it for an uncaught exception.
    pub fn describe(&mut self, f: &Failure) -> String {
        match f {
            Failure::Budget => "the interpreter's step budget ran out".to_string(),
            Failure::Depth => "the interpreter's call depth ran out".to_string(),
            Failure::Unsupported(s) | Failure::Stale(s) => s.clone(),
            Failure::Withheld => "the run needs a body withheld from its products and stops (the error below)".to_string(),
            Failure::Exit(status) => format!("the program called System.exit({})", status),
            Failure::Thrown(v) => {
                let text = match self.to_str(v) {
                    Ok(s) => s,
                    Err(_) => "<unprintable exception>".to_string(),
                };
                format!("Exception in thread \"main\" {}", text)
            }
        }
    }

    #[allow(dead_code)]
    pub fn take_output(&mut self) -> String {
        std::mem::take(&mut self.out)
    }

    pub fn flush(&mut self) {
        if !self.out.is_empty() {
            use std::io::Write;
            let mut stdout = std::io::stdout().lock();
            let _ = stdout.write_all(self.out.as_bytes());
            let _ = stdout.flush();
            self.out.clear();
        }
    }

    /// What `println` and `print` show of a value: the unit as Scala.js prints it, `undefined`,
    /// as the JavaScript runtime's `$printed`; a concatenation and `toString` render it `()`.
    pub(super) fn printed(&mut self, v: &Value) -> R<String> {
        match v {
            Value::Unit => Ok("undefined".to_string()),
            _ => self.to_str(v),
        }
    }

    pub(super) fn print(&mut self, s: &str) {
        self.out.push_str(s);
        if self.stream && self.out.len() > 1 << 16 {
            self.flush();
        }
    }

    pub(super) fn fresh_key(&mut self) -> u64 {
        self.next_key += 1;
        self.next_key
    }

    pub(super) fn unsupported<T>(&self, msg: impl Into<String>) -> R<T> {
        let msg = msg.into();
        if self.trace {
            eprintln!("  unsupported: {}", msg);
        }
        Err(Control::Fail(Failure::Unsupported(msg)))
    }

    pub(super) fn stale<T>(&self, msg: impl Into<String>) -> R<T> {
        let msg = msg.into();
        if self.trace {
            eprintln!("  stale: {}", msg);
        }
        Err(Control::Fail(Failure::Stale(msg)))
    }

    /// The number of the expansion that starts now, which the symbols of its site carry.
    pub fn next_run(&mut self) -> u32 {
        self.records.runs = self.records.runs.wrapping_add(1);
        self.records.runs
    }

    /// The interned string of a literal, shared between its evaluations.
    pub(super) fn string_lit(&mut self, s: StrRef) -> Rc<str> {
        if let Some(rc) = &self.str_cache[s.idx()] {
            return rc.clone();
        }
        let rc: Rc<str> = Rc::from(self.prog().strings[s.idx()].as_str());
        self.str_cache[s.idx()] = Some(rc.clone());
        rc
    }

    #[allow(dead_code)]
    pub fn literal(&self, v: LitVal) -> Value {
        Value::from_lit(v, self.typer.interner)
    }

    pub fn as_literal(&mut self, v: &Value) -> Option<LitVal> {
        v.to_lit(self.typer.interner)
    }
}

#[cfg(test)]
mod tests {
    /// The watch's numbering on a session's typing thread, as forked full builds leave it, starts
    /// again at every full build's end and start (`dispose`), so that a session's thousandth full
    /// build numbers as its first did: `MADE` never reaches the bound past which a run's own
    /// containers read as older than the run.
    #[test]
    fn a_full_build_starts_the_watch_numbering_again() {
        std::thread::spawn(|| {
            for build in 0..3 {
                assert_eq!(super::NEXT_EPOCH.with(|n| n.get()), 1, "build {build}");
                assert_eq!(super::MADE.with(|m| m.get()), 1, "build {build}");
                assert!(super::EPOCH_OWNERS.with(|o| o.borrow().is_empty()), "build {build}");
                super::NEXT_EPOCH.with(|n| n.set(6_762));
                super::MADE.with(|m| m.set(u32::MAX - build));
                super::EPOCH_OWNERS.with(|o| o.borrow_mut().insert(6_761, (std::rc::Rc::from("the run of m"), super::EpochOwner::Other)));
                super::dispose();
            }
        })
        .join()
        .unwrap();
    }

    /// A thread's caches kept apart by view: with the partition
    /// on, a run under the loader's lock takes and keeps the holder's set, nested holds
    /// included, and a run outside it the worker's; with it off, one set whatever the lock.
    #[test]
    fn a_run_under_the_lock_takes_the_holders_caches() {
        let lock = crate::shared::ReentrantLock::new();
        super::partition_by_view(true);
        assert_eq!(super::cache_set(), 0);
        lock.lock();
        assert_eq!(super::cache_set(), 1);
        lock.lock();
        assert_eq!(super::cache_set(), 1);
        lock.unlock();
        lock.unlock();
        assert_eq!(super::cache_set(), 0);
        super::partition_by_view(false);
        lock.lock();
        assert_eq!(super::cache_set(), 0);
        lock.unlock();
    }
}
