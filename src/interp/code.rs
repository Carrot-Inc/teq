//! The interpreter's execution units and the layouts of their frames.
//!
//! An execution unit is what one activation runs from its start, over the frame it is given: a
//! function's body under `call_fun` (its parameters bound in order into a fresh frame whose parent
//! is the scope the function reads its captures from) and, inside it, the bodies of its lambdas
//! (`call_closure`: the lambda's parameters in a frame whose parent is the frame the lambda was made
//! in) and of its local defs (`call_fun` again, the parent the block frame that declares the def).
//! A unit's layout says, for each read or assignment of a local inside it, which frame the walker
//! will find the local in, counted from the reading site, and at which slot: the frames are the
//! walker's own (`eval.rs`), opened by the same rules, so the layout is a replay of those rules over
//! the unit's tree and nothing else.
//!
//! The frames a unit opens, as the walker opens them:
//!
//! - the activation's: the parameters in order (a local trait's member, whose absent proxies are
//!   left unbound, `bind_params`, binds a dynamic prefix: its slots are not exact);
//! - a block's, when one of its statements declares something (`block_frame`): its local defs
//!   first, in statement order, then its vals and the binders of its pattern definitions in
//!   statement order; a block of the tuple-match shape (`tuple_match_shape`) opens none of its own;
//! - a case's, when its pattern binds (`eval_match`, `eval_tuple_match`): the binders in the order
//!   `match_pat` binds them (a binder before its inner pattern, an `unapply`'s scrutinee local before
//!   its call, fields and sequence items in order); an alternative that binds makes the rest of the
//!   frame inexact, since a failed alternative leaves its binders behind;
//! - a catch case's, always (`eval_try`, `Frame::new`), its binders as a case's;
//! - a lambda's and a local def's, which are units of their own nested in this one.
//!
//! What the layout does not cover is read by symbol, as the walker reads every local today
//! (`lookup`, its hint checked against the symbol): a local bound outside the unit (the frames of an
//! anonymous or local class's scope, `o.env`, which differ with the site that made the instance), a
//! constructor parameter read through `this` (`local`), the expressions inside patterns (an
//! `Equals` pattern's operand is evaluated in the match's frame by `quick_reject` and in the case's
//! by `match_pat`), defaults (run under `bind_params`' partial frame or under `eval_default`'s own),
//! lazy initialisers and splice roots (`Env`, `quoted.rs`). An address is trusted without the symbol
//! only by code that opened the frames it counts; the walker keeps the check.
//!
//! A unit is promoted at its second call (`TEQ_INTERP_LOWER=<n>`, `0` for the walker alone): its
//! layout's addresses become the walker's hints, and its body is lowered into `Code`, a flat sequence
//! of instructions over an operand stack of the activation's, which `exec` runs over the frame the
//! call gives it; a loop of a body the walker runs is promoted after `LOOP_PROMOTION` turns and runs
//! lowered from the head of its next turn, so that a body called once (a macro's) loops at the
//! lowered speed. The code keeps the walker's every effect in its order and its frames: blocks open
//! and leave theirs as `block_frame` and `leave_block` do, a case its frame of binders, a catch case
//! its own; a `return` of a function's own code is its exit, a lambda's travels as
//! `Control::Return`; a self call in tail position hands `call_fun` the next activation's arguments
//! (`Control::Tail`), which binds them in a fresh frame as the walker's loop does; a `try` runs its
//! protected range, its cases and its finaliser each as a run of its own, with `eval_try`'s exits.
//! The steps are the walker's: each instruction charges the nodes whose evaluation starts with it
//! before it runs, so that the budget, the trace's count and the profile's steps read the same.
//!
//! The capability matrix: each `TExpr` variant, what the layout replays of it and how the lowered
//! form takes it. "lowered" is an instruction of its own; "declined" is a node the preflight finds
//! outside the matrix, which declines its whole unit to the walker before any code is installed (the
//! lowering is the preflight: its first such node gives the unit's reason, `Walked`, and nothing of
//! the unit runs lowered); "refused" is the walker's own failure, kept. Inside lowered code the
//! walker's semantic helpers still evaluate the expressions they own: a pattern's extractor call and
//! literal operand (`match_pat`), a type test's path, the arguments of a class with captures
//! (`anon_args`, which reads a captured local as itself and passes a lazy one's thunk), the other
//! arguments of a local trait's member called with its proxies (`call_super_proxied`), a reflective
//! template's arguments (`call_by_signature`), a default's own expression.
//!
//! | variant | frames and locals | lowered form |
//! |---|---|---|
//! | `Int` `Long` `Double` `Bool` `Char` `Str` `Unit` `Null` | none | lowered (`int_literal`'s narrowing, a `Float` literal by its type) |
//! | `Local` | an address when a frame of the unit binds it, else by symbol | lowered (checked address, or `local`) |
//! | `This` `Super` | none | lowered (`this`) |
//! | `Static` `Module` `ClassOf` | none | lowered |
//! | `Field` | its receiver | lowered (the slot memo of `field_slots`) |
//! | `CallStatic` | its arguments; a local def's call reads the def by symbol or address | lowered (file initialiser before the arguments) |
//! | `CallMethod` | receiver, arguments | lowered (the site memo); a `super` call lowered without its receiver (`call_super`; `super[C]`'s member resolved before the arguments as `call_super_class` resolves it, and a local trait's member it names called with its proxies run whole by `call_super_proxied`); declined: a local trait's lifted member |
//! | `CallClosure` | callee, arguments | lowered (`apply_value`) |
//! | `New` | arguments | lowered (companion and file before the arguments); a class with captures by `anon_args` |
//! | `NewVia` | arguments | lowered |
//! | `Lambda` | a unit of its own: its parameters' frame | lowered (the closure; its body a unit) |
//! | `If` `While` | children | lowered (jumps, the condition checked `Boolean`) |
//! | `Block` | a frame when a statement declares; local defs (units) bound first | lowered; the tuple-match shape (`tuple_match_shape`) as `eval_tuple_match` runs it: the components without the tuple, each case over them (`tuple_match_fits` admits the shape only where every case is a tuple pattern of the components' arity) |
//! | `Assign` | a local target by address or symbol | lowered (a static's file first, the value before a field's receiver); declined: any other target |
//! | `Match` | per binding case a frame | lowered (cases as tests and jumps over `quick_reject`, `match_pat`, the guard) |
//! | `Prim` `Unary` | operands | lowered (`&&`, `||` as jumps) |
//! | `StrConcat` | operands, a chain head's block frame | lowered (the operands in order, each `ToStr` rendering with its own step, rendered as `concat` renders them: a value at once, an object kept to render once every operand ran, `render_late`; a head that goes on in a chain or a block, as `concat_head` and `Program::chain_head` say, into the same text) |
//! | `ToStr` | operand | lowered |
//! | `Js` | arguments | lowered (resolved before its arguments; a builtin's call after them, any other template's whole by `template`'s own path; `$quote`'s binders renamed before its holes and the renames left after the call, or on the run's exit by a failure) |
//! | `TypeTest` `Cast` | operand (the test's own expressions by symbol) | lowered |
//! | `SeqLit` `ArrayLit` `Index` | items, operand | lowered |
//! | `Return` | operand | lowered (the unit's exit in a function's own code; a lambda's travels as `Control::Return`) |
//! | `Throw` | operand | lowered |
//! | `Try` | body; per catch case a frame | lowered (the protected range a run of its own, the finaliser after every exit but a failure) |
//! | `Default` | the receiver and earlier arguments | lowered (the receiver and earlier arguments, then `default_apply`) |
//! | `JsImport` `JsGlobal` `JsSelect` `ObjLit` `Spread` | none | declined (the walker refuses JavaScript interop) |
//! | `Splice` | not entered | declined (the walker refuses it) |
//!
//! The patterns (`TPat`, all eight forms) are matched by `match_pat` in the lowered form as in the
//! walker, after `quick_reject`'s literal tests; their binders are the case frame's layout.
//!
//! Folded instructions, each two of the above with nothing between them (no jump lands on the
//! second, no node starts there): `this.f` (`ThisField`), `this.f = v` (`StoreThisField`), a primitive
//! comparison and its branch (`PrimJumpIfFalse`), and a pure value nobody reads, which vanishes with
//! its `Pop` (its steps charged by the next instruction). A widening of a value whose operation
//! makes the kind already (an `Int` addition typed `Int`) is left out.

use super::value::*;
use super::*;
use crate::ast::ListRef;
use super::eval::{push_text, CatchCase, TryPart};
use crate::tir::*;

/// One frame of a unit as the layout replays it: the symbols it binds in their order, and how
/// many of them, from the first, sit at known slots.
struct Scope {
    syms: Vec<SymId>,
    exact: usize,
    /// Whether the frame may not be there at all: a thunk a capture of a local class takes as a
    /// lazy val's (`anon_args`), which runs in the frame it was made in, where an argument of a
    /// by-name parameter runs in one of its own. Nothing beyond it has a known depth.
    opaque: bool,
    /// What opens the frame, the key of its descriptor once the walk closes it.
    opener: Option<Opener>,
}

impl Scope {
    fn new() -> Scope {
        Scope { syms: Vec::new(), exact: 0, opaque: false, opener: None }
    }

    fn opened(opener: Opener) -> Scope {
        Scope { opener: Some(opener), ..Scope::new() }
    }

    fn bind(&mut self, s: SymId) {
        if self.exact == self.syms.len() {
            self.exact += 1;
        }
        self.syms.push(s);
    }

    /// The rest of the frame's slots are not known: what a binding alternative or a proxy leaves.
    fn inexact(&mut self) {
        self.exact = self.exact.min(self.syms.len());
        self.syms.push(SymId(u32::MAX));
    }
}

/// What opens a frame of a unit: a function's activation, a lambda's, a block (by its statements'
/// place, which no other block shares) or a case of a `match`, a tuple match or a `catch` (by its
/// place among the program's cases).
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum Opener {
    Activation(FunId),
    Lambda(TExprId),
    Block(u32),
    Case(u32),
}

/// A frame a unit opens, as the layout walk closed it: the symbols its bindings take in their
/// order, the first `exact` of them at known slots. Immutable once made and shared: the unit's code
/// keeps it, the instruction that opens the frame names it, its slots are the addresses the code's
/// checked reads and stores take, and the assertion-enabled build checks the frame's bindings
/// against it where the frame is left.
pub(super) struct ScopeDesc {
    syms: Box<[SymId]>,
    exact: u32,
}

impl ScopeDesc {
    /// The slot of `sym` in a frame of this descriptor, where a reading site's hint says (`slot`)
    /// when the descriptor binds it there: one of the known slots, or none when the frame binds
    /// `sym` past them or not at all (the adapter's check of a hint against the frame it reaches).
    #[inline]
    fn slot_of(&self, sym: SymId, slot: usize) -> Option<usize> {
        let known = &self.syms[..self.exact as usize];
        if known.get(slot) == Some(&sym) {
            return Some(slot);
        }
        known.iter().rposition(|&s| s == sym)
    }

    /// Whether the frame `f` binds what the descriptor says, in its order, as far as it knows it.
    #[allow(dead_code)]
    pub(super) fn binds(&self, f: &Frame) -> bool {
        let vals = f.vals.borrow();
        vals.len() >= self.exact as usize && self.syms.iter().take(self.exact as usize).zip(vals.iter()).all(|(s, (b, _))| s == b)
    }
}

/// The descriptors of the code an interpreter's caches keep, by id: what a frame a lowered unit
/// makes carries (`Frame::desc`) and the adapter finds its descriptor by. An id is the table's
/// generation (its high byte, never 0) and the descriptor's place; the table goes with the code
/// (`forget_units`) and the next generation's ids are others, so a frame older code made finds
/// nothing and is read as the walker's frames are, by symbol.
pub(super) struct Descs {
    generation: u32,
    all: Vec<Rc<ScopeDesc>>,
}

/// The places of one generation's descriptors (`Descs`): an id's low 24 bits.
const DESC_PLACES: usize = (1 << 24) - 1;

thread_local! {
    /// The generation the next table of this thread starts (`Descs::new`, `Descs::reset`).
    static DESC_GENERATION: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

impl Descs {
    pub(super) fn new() -> Descs {
        Descs { generation: Self::next_generation(), all: Vec::new() }
    }

    fn next_generation() -> u32 {
        DESC_GENERATION.with(|g| {
            let n = g.get() % 255 + 1;
            g.set(n);
            n
        })
    }

    /// The descriptors of a unit's code entered under consecutive ids, the first returned. Where
    /// the generation's places run out a new one starts: the codes of the last keep their ids,
    /// which their frames carry and the adapter no longer finds (it reads those by symbol).
    fn register(&mut self, scopes: &[Rc<ScopeDesc>]) -> u32 {
        if self.all.len() + scopes.len() > DESC_PLACES {
            self.reset();
        }
        let first = (self.generation << 24) | (self.all.len() as u32 + 1);
        self.all.extend(scopes.iter().cloned());
        first
    }

    /// The descriptor of id `id`, when this generation made it.
    #[inline]
    fn get(&self, id: u32) -> Option<&ScopeDesc> {
        if id >> 24 != self.generation {
            return None;
        }
        self.all.get(((id & 0xff_ffff) as usize).wrapping_sub(1)).map(|d| &**d)
    }

    /// A new generation, the descriptors let go with the code that made them.
    fn reset(&mut self) {
        self.all.clear();
        self.generation = Self::next_generation();
    }
}

/// The addresses of a unit's locals: per reading or assigning expression, the depth of the frame
/// that binds the local, counted from the site's frame, and the slot in it; and the descriptors of
/// the frames it opens.
#[derive(Default)]
pub(super) struct Layout {
    /// The address of each reading or assigning site and what opened the frame that binds it.
    pub addrs: Vec<(TExprId, u16, u16, Opener)>,
    /// The descriptors of the frames the walk closed, and what opened each.
    scopes: Vec<Rc<ScopeDesc>>,
    openers: FxMap<Opener, u32>,
    /// The local defs nested in the unit, whose bodies the layout covered with it.
    pub local_defs: Vec<FunId>,
    /// The expressions the walk visited.
    pub nodes: u32,
}

struct Walk {
    scopes: Vec<Scope>,
    layout: Layout,
    /// The lazy initialisers and local def bodies of the blocks under way, walked once their
    /// block's frame is complete (each runs in or under that frame, after any of its statements).
    deferred: Vec<Deferred>,
}

enum Deferred {
    Lazy(TExprId),
    Def(FunId),
}

impl Walk {
    /// The address of `s` at the site, when a frame of the unit binds it at a known slot, and what
    /// opened that frame (every frame an address counts has an opener: the opaque one binds nothing).
    fn find(&self, s: SymId) -> Option<(u16, u16, Opener)> {
        for (depth, scope) in self.scopes.iter().rev().enumerate() {
            if let Some(slot) = scope.syms.iter().rposition(|&x| x == s) {
                let opener = scope.opener?;
                return (slot < scope.exact && depth < 0x7fff && slot < 0xffff).then_some((depth as u16, slot as u16, opener));
            }
            if scope.opaque {
                return None;
            }
        }
        None
    }

    fn note(&mut self, e: TExprId, s: SymId) {
        if let Some((depth, slot, opener)) = self.find(s) {
            self.layout.addrs.push((e, depth, slot, opener));
        }
    }

    fn top(&mut self) -> &mut Scope {
        self.scopes.last_mut().expect("a frame of the unit")
    }

    /// Closes the innermost frame, its descriptor kept under what opened it.
    fn close(&mut self) {
        let scope = self.scopes.pop().expect("a frame of the unit");
        if let Some(opener) = scope.opener {
            let desc = ScopeDesc { syms: scope.syms.into_boxed_slice(), exact: scope.exact as u32 };
            self.layout.scopes.push(Rc::new(desc));
            self.layout.openers.insert(opener, (self.layout.scopes.len() - 1) as u32);
        }
    }
}

impl<'a, 't> Interp<'a, 't> {
    /// The executable body of `f`: what `call_fun` runs and `tail_info` reads, the one accessor the
    /// interpreter takes a function's body through (`call_fun` takes a unit's code, lowered from the
    /// body this gave, without it). The reflection reads the typed view
    /// (`TreeRef::Fun`, `ensure_body`, `val_init_of`), never this.
    #[inline]
    pub(super) fn exec_body(&mut self, f: FunId) -> Option<TExprId> {
        // Its plain inline calls expanded first, where the inlining phase keeps some still
        // (`Worker::prepare_fun`); a unit the run's unit waits for stops the run.
        if !self.typer.prepare_fun(f) {
            self.note_withheld();
        }
        self.prog().funs[f.idx()].body
    }

    /// `exec_body`'s preparation for a call that finds no unit's code (`call_fun`), off the path
    /// of the calls that find it.
    #[cold]
    #[inline(never)]
    pub(super) fn prepare_unlowered(&mut self, f: FunId) {
        if !self.typer.prepare_fun(f) {
            self.note_withheld();
        }
    }

    /// The layout of the unit of function `f`'s body, its lambdas and local defs included, for the
    /// incoming layout `absent`: the proxies (of a local trait's member) its call leaves unbound, a
    /// bit each (`absent_proxies`).
    pub(super) fn fun_layout(&mut self, f: FunId, absent: u64) -> Layout {
        let mut w = Walk { scopes: Vec::new(), layout: Layout::default(), deferred: Vec::new() };
        self.def_layout(&mut w, f, absent);
        w.layout
    }

    /// A function's activation frame and its body, under the frames of `w`: the parameters bound
    /// in order, each proxy the call leaves absent skipped as `bind_params` skips it.
    fn def_layout(&mut self, w: &mut Walk, f: FunId, absent: u64) {
        let Some(body) = self.exec_body(f) else { return };
        let (params, proxies) = {
            let fun = &self.prog().funs[f.idx()];
            (fun.params.clone(), fun.captures as usize)
        };
        let mut scope = Scope::opened(Opener::Activation(f));
        for (i, p) in params.into_iter().enumerate() {
            if i < proxies && (i >= 64 || absent & (1 << i) != 0) {
                continue;
            }
            scope.bind(p);
        }
        w.scopes.push(scope);
        self.expr_layout(w, body);
        w.close();
    }

    fn list_layout(&mut self, w: &mut Walk, items: ListRef) {
        for i in items.range() {
            let x = self.prog().expr_lists[i];
            self.expr_layout(w, x);
        }
    }

    fn expr_layout(&mut self, w: &mut Walk, e: TExprId) {
        w.layout.nodes += 1;
        match self.prog().expr(e) {
            TExpr::Local(s) => w.note(e, s),
            TExpr::Field(r, _) | TExpr::Unary(_, r) | TExpr::ToStr(r, _) | TExpr::TypeTest(r, _) | TExpr::Cast(r, _, _) | TExpr::Index(r, _) | TExpr::Return(r) | TExpr::Throw(r, _) => self.expr_layout(w, r),
            TExpr::CallStatic(s, args) => {
                if self.syms().sym(s).owner == Owner::Local {
                    w.note(e, s);
                }
                self.list_layout(w, args);
            }
            TExpr::CallMethod(r, _, args) => {
                self.expr_layout(w, r);
                self.list_layout(w, args);
            }
            TExpr::CallClosure(f, args) => {
                self.expr_layout(w, f);
                self.list_layout(w, args);
            }
            TExpr::New(_, args) => {
                for i in args.range() {
                    let x = self.prog().expr_lists[i];
                    match self.prog().expr(x) {
                        TExpr::Lambda(params, body) if params.len == 0 => {
                            w.scopes.push(Scope { opaque: true, ..Scope::new() });
                            self.expr_layout(w, body);
                            w.close();
                        }
                        _ => self.expr_layout(w, x),
                    }
                }
            }
            TExpr::Default(_, _, args) | TExpr::NewVia(_, args) | TExpr::StrConcat(args) | TExpr::Js(_, args) | TExpr::SeqLit(args) | TExpr::ArrayLit(args) => self.list_layout(w, args),
            TExpr::Lambda(params, body) => {
                let mut scope = Scope::opened(Opener::Lambda(body));
                for i in params.range() {
                    scope.bind(self.prog().sym_lists[i]);
                }
                w.scopes.push(scope);
                self.expr_layout(w, body);
                w.close();
            }
            TExpr::If(c, t, els) => {
                self.expr_layout(w, c);
                self.expr_layout(w, t);
                if let Some(x) = els {
                    self.expr_layout(w, x);
                }
            }
            TExpr::While(a, b) | TExpr::Prim(_, a, b) => {
                self.expr_layout(w, a);
                self.expr_layout(w, b);
            }
            TExpr::Assign(target, value) => {
                self.expr_layout(w, value);
                match self.prog().expr(target) {
                    TExpr::Local(s) => w.note(target, s),
                    TExpr::Field(r, _) => self.expr_layout(w, r),
                    _ => {}
                }
            }
            TExpr::Block(stmts, res) => self.block_layout(w, e, stmts, res),
            TExpr::Match(scrut, cases) => {
                self.expr_layout(w, scrut);
                for i in cases.range() {
                    let case = self.prog().cases[i];
                    let binds = self.pat_binds_cached(case.pat);
                    self.case_layout(w, i as u32, case, binds);
                }
            }
            TExpr::Try(i) => {
                let (body, cases, finalizer) = {
                    let t = &self.prog().tries[i as usize];
                    (t.body, t.cases, t.finalizer)
                };
                self.expr_layout(w, body);
                for k in cases.range() {
                    let case = self.prog().cases[k];
                    self.case_layout(w, k as u32, case, true);
                }
                if let Some(f) = finalizer {
                    self.expr_layout(w, f);
                }
            }
            TExpr::Int(_) | TExpr::Long(_) | TExpr::Double(_) | TExpr::Bool(_) | TExpr::Char(_) | TExpr::Str(_) | TExpr::Unit | TExpr::Null | TExpr::This | TExpr::Super(_) | TExpr::Static(_) | TExpr::Module(_) | TExpr::ClassOf(..) => {}
            TExpr::JsImport(_) | TExpr::JsGlobal(..) | TExpr::JsSelect(..) | TExpr::ObjLit(_) | TExpr::Spread(_) | TExpr::Splice(_) => {}
        }
    }

    /// A case's guard and body, in a frame of the pattern's binders when it binds (a catch case's
    /// always opens one).
    fn case_layout(&mut self, w: &mut Walk, k: u32, case: TCase, frame: bool) {
        if frame {
            let mut scope = Scope::opened(Opener::Case(k));
            self.pat_binders(case.pat, &mut scope);
            w.scopes.push(scope);
        }
        if let Some(g) = case.guard {
            self.expr_layout(w, g);
        }
        self.expr_layout(w, case.body);
        if frame {
            w.close();
        }
    }

    /// The locals `match_pat` binds for `pat`, in its order.
    fn pat_binders(&self, pat: TPatId, scope: &mut Scope) {
        match self.prog().pats[pat.idx()] {
            TPat::Wildcard | TPat::Equals(..) => {}
            TPat::Bind(s, inner) => {
                scope.bind(s);
                if let Some(i) = inner {
                    self.pat_binders(i, scope);
                }
            }
            TPat::Test(_, _, inner) => self.pat_binders(inner, scope),
            TPat::Class(_, _, _, subs) => {
                for i in subs.range() {
                    self.pat_binders(self.prog().pat_lists[i], scope);
                }
            }
            TPat::Alt(alts) => {
                if self.prog().pat_lists[alts.range()].iter().any(|&a| self.pat_binds(a)) {
                    scope.inexact();
                }
            }
            TPat::Unapply(s, _, inner) => {
                scope.bind(s);
                self.pat_binders(inner, scope);
            }
            TPat::Seq(items, rest) => {
                for i in items.range() {
                    self.pat_binders(self.prog().pat_lists[i], scope);
                }
                if let Some(r) = rest {
                    self.pat_binders(r, scope);
                }
            }
        }
    }

    /// A block as `eval` runs it: the tuple-match shape without a frame of its own, a block that
    /// declares nothing in its enclosing frame, any other in a frame of its own whose local defs are
    /// bound first.
    fn block_layout(&mut self, w: &mut Walk, e: TExprId, stmts: ListRef, res: TExprId) {
        if stmts.is_empty() {
            return self.expr_layout(w, res);
        }
        if let Some((_, args, cases)) = self.tuple_match_shape(e, stmts, res) {
            self.list_layout(w, args);
            for i in cases.range() {
                let case = self.prog().cases[i];
                let binds = self.pat_binds_cached(case.pat);
                self.case_layout(w, i as u32, case, binds);
            }
            return;
        }
        let declares = self.prog().stmts[stmts.range()].iter().any(|s| !matches!(s, TStmt::Expr(_)));
        if !declares {
            for i in stmts.range() {
                if let TStmt::Expr(x) = self.prog().stmts[i] {
                    self.expr_layout(w, x);
                }
            }
            return self.expr_layout(w, res);
        }
        let mark = w.deferred.len();
        w.scopes.push(Scope::opened(Opener::Block(stmts.start)));
        for i in stmts.range() {
            if let TStmt::Fun(f) = self.prog().stmts[i] {
                let sym = self.prog().funs[f.idx()].sym;
                w.top().bind(sym);
                w.deferred.push(Deferred::Def(f));
            }
        }
        for i in stmts.range() {
            match self.prog().stmts[i] {
                TStmt::Expr(x) => self.expr_layout(w, x),
                TStmt::Fun(_) => {}
                TStmt::Val(sym, init) => {
                    if self.syms().sym(sym).mods & crate::ast::mods::LAZY != 0 {
                        w.deferred.push(Deferred::Lazy(init));
                    } else {
                        self.expr_layout(w, init);
                    }
                    w.top().bind(sym);
                }
                TStmt::Pat(pat, init) => {
                    self.expr_layout(w, init);
                    let mut scope = std::mem::replace(w.top(), Scope::new());
                    self.pat_binders(pat, &mut scope);
                    *w.top() = scope;
                }
            }
        }
        self.expr_layout(w, res);
        // The local defs and lazy initialisers run in (under) the block's frame, complete by then.
        while w.deferred.len() > mark {
            match w.deferred.pop().expect("a deferred body") {
                Deferred::Lazy(init) => self.expr_layout(w, init),
                Deferred::Def(f) => {
                    w.layout.local_defs.push(f);
                    self.def_layout(w, f, 0);
                }
            }
        }
        w.close();
    }
}

// ---- promotion ----

/// A unit's state (`Interp::units`, `lambda_units`, `loop_units`): its calls in the low half,
/// counted on after its promotion (saturating), and in the high half what became of it: nothing
/// yet, `READY` with the index of its code in `Interp::codes`, `DECLINED` with the reason the
/// preflight gave (a node outside the capability matrix: the walker runs the whole unit), or
/// `KEPT`, laid out without code (`TEQ_INTERP_CODE=0`).
const READY: u32 = 1 << 31;
const DECLINED: u32 = 1 << 30;
const KEPT: u32 = 1 << 29;

#[inline]
fn fate(state: u64) -> u32 {
    (state >> 32) as u32
}

/// `state` with one more call counted.
#[inline]
fn called(state: u64) -> u64 {
    (state & !0xffff_ffff) | (state as u32).saturating_add(1) as u64
}

/// The call of a unit at which it is promoted: its layout applied and its body lowered.
/// `TEQ_INTERP_LOWER=<n>`: at the n-th call, `0` never (the walker alone).
fn promotion() -> u32 {
    static AT: std::sync::OnceLock<u32> = std::sync::OnceLock::new();
    *AT.get_or_init(|| std::env::var("TEQ_INTERP_LOWER").ok().and_then(|v| v.parse().ok()).unwrap_or(2))
}

/// The turns of a loop of a walked body after which the rest of the loop runs lowered: a body
/// called once may loop for the whole run (a macro's scan).
const LOOP_PROMOTION: u32 = 16;

/// Whether a promoted unit is lowered: `TEQ_INTERP_CODE=0` keeps its layout's addresses alone,
/// as hints of the walker's.
fn lowering() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var("TEQ_INTERP_CODE").map_or(true, |v| v != "0"))
}

/// The incoming layout of a call that leaves absent more proxies than a key holds: the walker's.
pub(super) const NO_LAYOUT: u64 = u64::MAX;

/// The incoming layout of a call of a local trait's member with `proxies` proxies on `args`: the
/// proxies the caller leaves absent, which `bind_params` leaves unbound, a bit each (`0` for a call
/// that passes every proxy, and for any call of a function without proxies).
pub(super) fn absent_proxies(args: &[Value], proxies: usize) -> u64 {
    if proxies >= 64 {
        return NO_LAYOUT;
    }
    (0..proxies).filter(|&i| matches!(args.get(i), None | Some(Value::Absent))).fold(0, |m, i| m | (1 << i))
}

/// The kinds of unit, as the statistics count them.
const FUNCTION: u8 = 0;
const LAMBDA: u8 = 1;
const LOOP: u8 = 2;

impl<'a, 't> Interp<'a, 't> {
    /// The state of function `f`'s unit for the incoming layout `absent`: the dense table's for the
    /// layout every call of a function without proxies makes, a map's for another.
    #[inline(always)]
    fn fun_state(&self, f: FunId, absent: u64) -> u64 {
        if absent == 0 {
            self.units[f.idx()]
        } else {
            self.proxy_state(f, absent)
        }
    }

    #[inline(always)]
    fn set_fun_state(&mut self, f: FunId, absent: u64, state: u64) {
        if absent == 0 {
            *self.units.at(f.idx()) = state;
        } else {
            self.set_proxy_state(f, absent, state);
        }
    }

    #[cold]
    #[inline(never)]
    fn proxy_state(&self, f: FunId, absent: u64) -> u64 {
        self.proxy_units.get(&(f, absent)).copied().unwrap_or(0)
    }

    #[cold]
    #[inline(never)]
    fn set_proxy_state(&mut self, f: FunId, absent: u64, state: u64) {
        self.proxy_units.insert((f, absent), state);
    }

    /// The code of function `f`'s body `body` for a call passing every proxy (the layout of every
    /// call of a function without proxies), once lowered; the call counted. The state is the dense
    /// table's and is read and counted in place, a unit with code having its entry; code made for
    /// another body or another generation of the program is not this unit's, which is promoted
    /// again. A call of a unit with proxies whose arguments leave some absent is another unit's,
    /// left to `unit_miss`.
    #[inline(always)]
    pub(super) fn unit_code(&mut self, f: FunId, body: TExprId, args: &[Value]) -> Option<Rc<Code>> {
        let state = self.units.get_mut(f.idx())?;
        if fate(*state) & READY == 0 {
            return None;
        }
        let code = &self.codes[(fate(*state) & !READY) as usize];
        if code.body != body || code.shifts != self.typer.prog.shifts {
            *state &= 0xffff_ffff;
            return None;
        }
        debug_assert!(code.absent == 0, "a unit's code taken for another incoming layout");
        if code.proxies != 0 && absent_proxies(args, code.proxies as usize) != 0 {
            return None;
        }
        *state = called(*state);
        if self.code_stats {
            count(&COUNTERS.hits, 1);
        }
        Some(code.clone())
    }

    /// A call of function `f` that `unit_code` found no code for: its incoming layout read off the
    /// function's proxies (`absent_proxies`), the code of that layout's unit when it has some, else
    /// the call counted towards that unit's promotion (`unit_call`); with the def's tail-call facts,
    /// the layout, and whether the function has proxies, whose self calls in tail position may
    /// change the layout (`relayout`).
    #[inline(never)]
    pub(super) fn unit_miss(&mut self, f: FunId, body: TExprId, args: &[Value]) -> (Option<Rc<Code>>, Option<(SymId, u32)>, u64, bool) {
        let proxies = self.prog().funs[f.idx()].captures as usize;
        let absent = if proxies > 0 { absent_proxies(args, proxies) } else { 0 };
        if absent != 0 && absent != NO_LAYOUT {
            if let Some(c) = self.proxied_code(f, body, absent) {
                let tail = c.tail;
                return (Some(c), tail, absent, true);
            }
        }
        let tail = self.tail_info(f);
        (self.unit_call(f, tail, absent), tail, absent, proxies > 0)
    }

    /// The code of function `f`'s body for the incoming layout `absent`, which leaves proxies
    /// absent, kept by layout in `proxy_units`; the call counted.
    #[cold]
    #[inline(never)]
    fn proxied_code(&mut self, f: FunId, body: TExprId, absent: u64) -> Option<Rc<Code>> {
        let state = self.proxy_state(f, absent);
        if fate(state) & READY == 0 {
            return None;
        }
        let code = self.codes[(fate(state) & !READY) as usize].clone();
        debug_assert!(code.absent == absent, "a unit's code taken for another incoming layout");
        if code.body != body || code.shifts != self.prog().shifts {
            self.set_proxy_state(f, absent, state & 0xffff_ffff);
            return None;
        }
        self.set_proxy_state(f, absent, called(state));
        if self.code_stats {
            count(&COUNTERS.hits, 1);
        }
        Some(code)
    }

    /// A self call in tail position of a function with proxies (`call_fun`'s loop), whose
    /// arguments may leave other proxies absent than the activation before it: the new incoming
    /// layout and its unit's code, when the layout changed.
    #[cold]
    #[inline(never)]
    pub(super) fn relayout(&mut self, f: FunId, body: TExprId, tail: Option<(SymId, u32)>, args: &[Value], absent: u64) -> Option<(Option<Rc<Code>>, u64)> {
        let next = absent_proxies(args, self.prog().funs[f.idx()].captures as usize);
        if next == absent {
            return None;
        }
        let code = match next {
            0 => self.unit_code(f, body, args),
            NO_LAYOUT => None,
            _ => self.proxied_code(f, body, next),
        };
        Some((code.or_else(|| self.unit_call(f, tail, next)), next))
    }

    /// A call of function `f`'s body (`call_fun`) for the incoming layout `absent` that has no
    /// code, counted towards its unit's promotion: its code once lowered. A layout of more proxies
    /// than a key holds (`NO_LAYOUT`) stays with the walker.
    #[inline]
    pub(super) fn unit_call(&mut self, f: FunId, tail: Option<(SymId, u32)>, absent: u64) -> Option<Rc<Code>> {
        if absent == NO_LAYOUT {
            return None;
        }
        let state = called(self.fun_state(f, absent));
        self.set_fun_state(f, absent, state);
        if fate(state) != 0 {
            self.declined_call(fate(state));
            return None;
        }
        if promotion() == 0 || (state as u32) < promotion() {
            return None;
        }
        self.promote_fun(f, tail, absent)
    }

    /// A call of the lambda whose body is `body` (`call_closure`), counted as a function's.
    #[inline]
    pub(super) fn lambda_call(&mut self, params: ListRef, body: TExprId) -> Option<Rc<Code>> {
        let state = called(self.lambda_units[body.idx()]);
        *self.lambda_units.at(body.idx()) = state;
        let fate = fate(state);
        if fate & READY != 0 {
            let code = &self.codes[(fate & !READY) as usize];
            if code.shifts == self.prog().shifts {
                if self.code_stats {
                    count(&COUNTERS.hits, 1);
                }
                return Some(code.clone());
            }
            *self.lambda_units.at(body.idx()) = state & 0xffff_ffff;
            return None;
        }
        if fate != 0 {
            self.declined_call(fate);
            return None;
        }
        if promotion() == 0 || (state as u32) < promotion() {
            return None;
        }
        self.promote_lambda(params, body)
    }

    /// A call of a unit the preflight declined, for the fallback's cost.
    #[inline]
    fn declined_call(&self, fate: u32) {
        if self.code_stats && fate & DECLINED != 0 {
            count(&COUNTERS.declined_calls[(fate & !DECLINED) as usize], 1);
        }
    }

    /// Promotes the unit of `f`: its layout's addresses become the walker's hints for the reads
    /// and assignments of its locals (checked against the symbol as every hint is), the bodies
    /// of its lambdas and local defs among them, and its body is lowered unless the preflight
    /// declines it.
    #[cold]
    #[inline(never)]
    fn promote_fun(&mut self, f: FunId, tail: Option<(SymId, u32)>, absent: u64) -> Option<Rc<Code>> {
        let layout = self.fun_layout(f, absent);
        self.apply_layout(&layout);
        let code = match self.exec_body(f) {
            Some(body) if lowering() => self.lower_unit(body, layout, tail, true, FUNCTION, absent, self.prog().funs[f.idx()].captures, Opener::Activation(f)),
            _ => Err(None),
        };
        let state = self.fun_state(f, absent);
        let (code, fate) = self.keep_code(code);
        self.set_fun_state(f, absent, (state & 0xffff_ffff) | ((fate as u64) << 32));
        code
    }

    /// A loop the walker runs, at the head of its `n`-th iteration: once it has turned
    /// `LOOP_PROMOTION` times, its code from the next head on (`lower_loop`), where the walker
    /// goes on.
    #[inline]
    pub(super) fn loop_turn(&mut self, e: TExprId, n: u32) -> Option<Rc<Code>> {
        if n != LOOP_PROMOTION || promotion() == 0 || !lowering() {
            return None;
        }
        self.promote_loop(e)
    }

    #[cold]
    #[inline(never)]
    fn promote_loop(&mut self, e: TExprId) -> Option<Rc<Code>> {
        let state = called(self.loop_units[e.idx()]);
        *self.loop_units.at(e.idx()) = state;
        let fate = fate(state);
        if fate & READY != 0 {
            let code = &self.codes[(fate & !READY) as usize];
            if code.shifts == self.prog().shifts {
                if self.code_stats {
                    count(&COUNTERS.hits, 1);
                }
                return Some(code.clone());
            }
        } else if fate != 0 {
            self.declined_call(fate);
            return None;
        }
        let TExpr::While(c, body) = self.prog().expr(e) else { return None };
        // The loop runs in the frame the walker gives it: a local bound outside it is read by
        // symbol, one of a block inside it at the layout's address. Those addresses count only the
        // frames the loop opens, the same from whatever frame the walker reaches the loop in, so
        // they are the walker's hints too (`apply_layout`), checked against the symbol as every hint.
        let mut w = Walk { scopes: Vec::new(), layout: Layout::default(), deferred: Vec::new() };
        self.expr_layout(&mut w, e);
        let layout = w.layout;
        self.apply_layout(&layout);
        let code = self.lower_loop(e, c, body, layout);
        let (code, fate) = self.keep_code(code);
        *self.loop_units.at(e.idx()) = (state & 0xffff_ffff) | ((fate as u64) << 32);
        code
    }

    #[cold]
    #[inline(never)]
    fn promote_lambda(&mut self, params: ListRef, body: TExprId) -> Option<Rc<Code>> {
        let layout = self.lambda_layout(params, body);
        self.apply_layout(&layout);
        let code = if lowering() { self.lower_unit(body, layout, None, false, LAMBDA, 0, 0, Opener::Lambda(body)) } else { Err(None) };
        let state = self.lambda_units[body.idx()];
        let (code, fate) = self.keep_code(code);
        *self.lambda_units.at(body.idx()) = (state & 0xffff_ffff) | ((fate as u64) << 32);
        code
    }

    fn apply_layout(&mut self, layout: &Layout) {
        for &(e, depth, slot, _) in &layout.addrs {
            self.local_slots[e.idx()] = ((depth as u32 + 1) << 16) | slot as u32;
        }
        if self.code_stats {
            count(&COUNTERS.layouts, 1);
            count(&COUNTERS.layout_nodes, layout.nodes as u64);
        }
    }

    /// What became of a promoted unit, and its code: kept, declined by the preflight with its
    /// reason, or not lowered.
    fn keep_code(&mut self, code: Result<Rc<Code>, Option<Walked>>) -> (Option<Rc<Code>>, u32) {
        match code {
            Ok(c) if self.codes.len() < KEPT as usize => {
                self.code_bytes += c.bytes();
                self.codes.push(c.clone());
                (Some(c), READY | (self.codes.len() - 1) as u32)
            }
            Err(Some(why)) => {
                // The execution that promoted the unit runs on the walker too: the first of the
                // declined calls, which every later one adds to (`declined_call`).
                if self.code_stats {
                    count(&COUNTERS.declined[why as usize], 1);
                    count(&COUNTERS.declined_calls[why as usize], 1);
                }
                (None, DECLINED | why as u32)
            }
            _ => (None, KEPT),
        }
    }

    /// After a retype: the units are counted, laid out and lowered again, since a function's
    /// record may stand for another body (`incremental.rs`).
    pub(super) fn forget_units(&mut self) {
        self.units.clear();
        self.proxy_units.clear();
        self.lambda_units.clear();
        self.loop_units.clear();
        self.codes.clear();
        self.descs.reset();
        self.code_bytes = 0;
    }

    /// The layout of a lambda's body taken as a unit of its own, outside its enclosing one.
    fn lambda_layout(&mut self, params: ListRef, body: TExprId) -> Layout {
        let mut w = Walk { scopes: Vec::new(), layout: Layout::default(), deferred: Vec::new() };
        let mut scope = Scope::opened(Opener::Lambda(body));
        for i in params.range() {
            scope.bind(self.prog().sym_lists[i]);
        }
        w.scopes.push(scope);
        self.expr_layout(&mut w, body);
        w.close();
        w.layout
    }
}

// ---- the lowered form ----

/// One instruction of a unit's code and the steps it charges before it runs: those of the nodes
/// whose evaluation starts with it, which the walker charges one by one (`step`, at each `eval`
/// and at the nodes `eval_tail` and `concat` take themselves), so that the budget runs out at the
/// same effect as the walker's.
#[derive(Clone, Copy)]
pub(super) struct Ins {
    op: Op,
    charge: u32,
}

// An instruction stays 24 bytes: the widest operations (`Local`, `Case`) fill the operation's 16.
const _: () = assert!(std::mem::size_of::<Ins>() == 24);

/// What an instruction does; the operands are on the interpreter's stack (`Interp::stack`), the
/// last on top, and its result goes there.
#[derive(Clone, Copy)]
enum Op {
    Int(i32),
    Long(i64),
    Double(f64),
    Float(f32),
    Byte(i8),
    Short(i16),
    Char(u16),
    Bool(bool),
    Str(StrRef),
    Unit,
    Null,
    This,
    /// A local a frame of the unit binds: at `depth` frames up, at `slot`, the frame there carrying
    /// the descriptor `Code::scopes[scope]` (the one the unit opened it with); else as the walker
    /// reads it (a lazy local's thunk, or a binding still to come).
    Local { e: TExprId, sym: SymId, depth: u16, slot: u16, kind: u8, scope: u16 },
    /// A local bound outside the unit's frames (a capture, a default's or a splice root's binding):
    /// where the walker's hint says, checked against the descriptor of the frame there or by the
    /// symbol in a frame without one (`hinted`), else as the walker reads it (`local`).
    LocalSym { e: TExprId, sym: SymId },
    Static(SymId),
    Module(ClassId),
    ClassOf(ClassId, u8),
    Field { e: TExprId, sym: SymId },
    /// `this.f`: `This` then `Field`.
    ThisField { e: TExprId, sym: SymId },
    /// The initialiser of the file of a top-level definition (`init_file_of`), before its call's
    /// arguments or the value assigned to it.
    InitFile(SymId),
    CallStatic { e: TExprId, sym: SymId, argc: u32 },
    CallMethod { e: TExprId, sym: SymId, argc: u32 },
    /// A call on `super` of the chain's or a trait's (`call_super`), the target read off the call
    /// at `e`.
    CallSuper { e: TExprId, sym: SymId, argc: u32 },
    /// `super[C].s(args)` at `e` before its arguments (`call_super_class`): its member resolved, and a
    /// local trait's member called with its proxies run whole (`call_super_proxied`), the arguments
    /// skipped.
    SuperClassPrep { e: TExprId, skip: u32 },
    CallSuperClass { e: TExprId, argc: u32 },
    /// A default getter's application on its receiver and earlier arguments (`default_apply`),
    /// the getter read off the node at `e`.
    DefaultArg { e: TExprId, argc: u32 },
    /// A function value and its arguments (`apply_value`).
    Apply { argc: u32 },
    /// The file and companion of a class before its creation's arguments; a class with captures
    /// is created whole (`anon_args`), its arguments skipped.
    NewPrep { e: TExprId, class: ClassId, skip: u32 },
    New { class: ClassId, argc: u32 },
    NewVia { sym: SymId, argc: u32 },
    Lambda { params: ListRef, body: TExprId },
    /// A template resolved before its arguments: a builtin's go on to `JsCall`, any other runs as
    /// the walker runs it, its arguments skipped.
    JsPrep { e: TExprId, s: StrRef, skip: u32, quote: bool },
    /// A builtin template's call; a quote's (`$quote`) leaves the renames of its binders after it.
    JsCall { s: StrRef, argc: u32, quote: bool },
    Jump(u32),
    JumpIfFalse(u32),
    JumpIfTrue(u32),
    /// The value on top is a `Boolean`, as a condition's has to be.
    CheckBool,
    Pop,
    /// Charges its steps alone: before a loop's head.
    Charge,
    /// A block's frame opened, its descriptor `Code::scopes[scope]`, made for the `cap` bindings
    /// the descriptor holds.
    PushFrame { scope: u32, cap: u16 },
    BindDef(FunId),
    Bind(SymId),
    BindLazy { sym: SymId, init: TExprId },
    BindPat(TPatId),
    PopFrame { scope: u32 },
    /// An assignment of a local at its address, checked as `Local` reads it.
    StoreLocal { e: TExprId, sym: SymId, depth: u16, slot: u16, scope: u16 },
    /// An assignment of a local outside the unit's frames, at its hint checked as `LocalSym`'s.
    StoreSym { e: TExprId, sym: SymId },
    StoreField(SymId),
    /// `this.f = v`: `This` then `StoreField`.
    StoreThisField(SymId),
    StoreStatic(SymId),
    Prim(PrimOp),
    /// `Prim` then `JumpIfFalse`.
    PrimJumpIfFalse(PrimOp, u32),
    Unary(UnOp),
    ToStr(StrConv),
    TypeTest(TestId),
    Cast(CastOp),
    SeqLit(u32),
    ArrayLit(u32),
    Index(u32),
    Return,
    Throw,
    /// A self call in tail position: the next activation's arguments, and its receiver.
    TailCall { argc: u32, recv: bool },
    Try(u32),
    /// A case of a match over the scrutinee on top: on to `next` when its pattern does not match,
    /// in a frame of its binders when it binds (its descriptor's, of `cap` bindings).
    Case { pat: TPatId, binds: bool, next: u32, scope: u32, cap: u16 },
    /// A case's guard on top: on to `next` when false.
    Guard { binds: bool, next: u32 },
    /// A case's body done: its frame left, the scrutinee under the result dropped.
    CaseEnd { binds: bool, scope: u32 },
    NoCase,
    /// A case of a case lambda's match over the tuple of its `n` parameters, the components on top
    /// (`eval_tuple_match`): on to `next` when its pattern does not take them.
    TupleCase { case: u32, binds: bool, next: u32 },
    /// The guard of such a case on top, the components under it: on to `next` when false.
    TupleGuard { binds: bool, next: u32 },
    /// Such a case taken: its components dropped before its body runs.
    TupleBody(u32),
    /// Such a case's body done: its frame left.
    TupleEnd { binds: bool },
    /// No case took the components: the tuple they make is the `MatchError`'s.
    NoTupleCase { class: ClassId, n: u32 },
    /// A chain of `+`'s text begun (`eval_concat`), on the interpreter's stack of chains.
    ConcatBegin,
    /// An operand of a `ToStr` rendering on top, rendered into the chain's text, or kept with its
    /// place to render once every operand ran (`concat`).
    ConcatRender(StrConv),
    /// A string operand on top, its text appended; another value rendered now.
    ConcatPlain,
    /// The chain's text, its kept operands rendered in their places.
    ConcatEnd,
    Adapt(u8),
    /// The numeric widening of the node, read from its type when it first runs.
    AdaptLazy(TExprId),
    End,
}

/// A `try` of a unit's code: the protected range, its catch cases and its finaliser, each a run of
/// its own ending in `End` (`Interp::run_try`), and where the code goes on after it.
struct TryPlan {
    body: u32,
    cases: Box<[CasePlan]>,
    finalizer: Option<u32>,
    wraps: bool,
    end: u32,
}

struct CasePlan {
    pat: TPatId,
    /// The descriptor of the case's frame.
    scope: u32,
    guard: Option<u32>,
    body: u32,
}

/// A case of a case lambda's tuple match, its pattern's components told apart once
/// (`eval_tuple_match`'s reading of the sub-patterns).
struct TupleCasePlan {
    comps: Box<[Comp]>,
    /// The descriptor of the case's frame of binders, when it binds, and how many it binds.
    scope: u32,
    cap: u16,
}

#[derive(Clone, Copy)]
enum Comp {
    Wildcard,
    /// A binder of the component alone.
    Bind(SymId),
    /// A literal the component is compared with (`TPat::Equals`), its sub-pattern.
    Literal(TPatId),
    /// Any other sub-pattern.
    Other(TPatId),
}

/// A unit's lowered code.
pub(super) struct Code {
    ins: Box<[Ins]>,
    tries: Box<[TryPlan]>,
    tuple_cases: Box<[TupleCasePlan]>,
    /// The descriptors of the frames the unit opens (`Layout`), which the instructions that open
    /// them name.
    scopes: Box<[Rc<ScopeDesc>]>,
    /// The key the code was made for, checked where it is taken: the body (a function's
    /// executable body, a lambda's, a loop's `while`), the program's generation (`Program::shifts`)
    /// and the incoming layout (`absent_proxies`).
    body: TExprId,
    shifts: u32,
    absent: u64,
    /// The proxies of the local trait's member the unit is (`TFun::captures`), `0` for any other
    /// unit: a call of it reads its incoming layout off its arguments.
    pub(super) proxies: u32,
    /// The id of `scopes`' first descriptor (`Descs`), the others' following it.
    first_id: u32,
    /// The descriptor id of the frame the unit is entered with: a function's activation for the
    /// incoming layout, which `call_fun` binds its arguments in, or a lambda's parameters
    /// (`call_closure`); `0` for a loop, which runs in the walker's frame.
    pub(super) entry_id: u32,
    /// Whether a `return` that leaves the code's own run is the unit's exit: a function's body,
    /// whose `call_fun` takes its own `Control::Return` (a lambda's travels to its def).
    exits: bool,
    /// The def whose self calls in tail position the code hands back as `Control::Tail`: a
    /// function's `tail_info`, which its `Ctx` carries.
    pub(super) tail: Option<(SymId, u32)>,
    /// What kind of unit it is (`FUNCTION`, `LAMBDA`, `LOOP`), for the statistics.
    kind: u8,
}

impl Drop for Code {
    fn drop(&mut self) {
        if stats_on() {
            COUNTERS.live.fetch_sub(self.bytes() as i64, std::sync::atomic::Ordering::Relaxed);
        }
    }
}

/// A loop's code, entered in the walker's frame: no descriptor of its own for that frame (the
/// `entry` `made` takes).
const NO_ENTRY: u32 = u32::MAX;

impl Code {
    /// The descriptor id a frame the unit opens for scope `scope` carries.
    #[inline]
    fn frame_id(&self, scope: u32) -> u32 {
        self.first_id + scope
    }

    fn bytes(&self) -> usize {
        std::mem::size_of::<Code>() + self.scopes.iter().map(|d| std::mem::size_of::<ScopeDesc>() + d.syms.len() * 4).sum::<usize>() + self.ins.len() * std::mem::size_of::<Ins>() + self.tries.iter().map(|t| std::mem::size_of::<TryPlan>() + t.cases.len() * std::mem::size_of::<CasePlan>()).sum::<usize>() + self.tuple_cases.iter().map(|t| std::mem::size_of::<TupleCasePlan>() + t.comps.len() * std::mem::size_of::<Comp>()).sum::<usize>()
    }
}

/// The lowering of one unit under way.
struct Lowering {
    ins: Vec<Ins>,
    tries: Vec<TryPlan>,
    tuple_cases: Vec<TupleCasePlan>,
    /// The layout's descriptors and what opened each.
    scopes: Vec<Rc<ScopeDesc>>,
    openers: FxMap<Opener, u32>,
    /// The steps of the nodes started since the last instruction, which the next one charges.
    pending: u32,
    /// Each addressed site's depth, slot and the descriptor of the frame that binds it (a unit of
    /// more frames than an instruction names reads the others as the adapter does).
    addrs: FxMap<TExprId, (u16, u16, u16)>,
    /// The def whose self calls in tail position loop (`Ctx::tail`), when the unit's does.
    tail: Option<(SymId, u32)>,
    /// Whether the type store's views are off, so that a node's numeric kind is read without
    /// importing its type into a worker's view (`compute_coerce`).
    kinds_now: bool,
    /// The place of the last jump target taken: no instruction there or after it is folded into
    /// another (`Lowering::fold`).
    target: usize,
    /// The first node the capability matrix leaves to the walker, which declines the whole unit.
    declined: Option<Walked>,
    nodes: u32,
}

impl Lowering {
    /// The descriptor of the frame `opener` opens.
    fn scope(&self, opener: Opener) -> u32 {
        *self.openers.get(&opener).expect("the layout's frame")
    }

    /// The bindings of descriptor `scope`'s frame, the capacity the frame is made with.
    fn cap(&self, scope: u32) -> u16 {
        self.scopes.get(scope as usize).map_or(0, |d| d.syms.len().min(u16::MAX as usize) as u16)
    }

    fn emit(&mut self, op: Op) -> usize {
        let charge = std::mem::take(&mut self.pending);
        self.ins.push(Ins { op, charge });
        self.ins.len() - 1
    }

    /// The last instruction, when nothing jumps between it and the next one and no node starts
    /// there: what the next one may fold.
    fn foldable(&self) -> Option<Op> {
        if self.pending != 0 || self.target == self.ins.len() {
            return None;
        }
        self.ins.last().map(|i| i.op)
    }

    /// Takes back the last instruction, its steps pending again for the next one.
    fn unemit(&mut self) {
        let last = self.ins.pop().expect("an instruction to take back");
        self.pending += last.charge;
    }

    /// The widening to kind `k` of the value the last instructions left: none where the last one
    /// makes a value of that kind, else an instruction of its own.
    fn adapt(&mut self, k: u8) {
        if self.target != self.ins.len() && self.pending == 0 {
            if let Some(i) = self.ins.last_mut() {
                if made_kind(i.op) == Some(k) {
                    return;
                }
                // A local read widens what it read itself.
                if let Op::Local { kind: kind @ 0, .. } = &mut i.op {
                    *kind = k;
                    return;
                }
            }
        }
        self.emit(Op::Adapt(k));
    }

    /// Emits `op`, folded with the instruction before it where a superinstruction does both:
    /// `This` and `Field` (`ThisField`), `This` and `StoreField`, `Prim` and `JumpIfFalse`, and
    /// a value nothing reads (a pure push and `Pop`, which vanish).
    fn emit_folding(&mut self, op: Op) -> usize {
        if let Some(last) = self.foldable() {
            let folded = match (last, op) {
                (Op::This, Op::Field { e, sym }) => Some(Op::ThisField { e, sym }),
                (Op::This, Op::StoreField(sym)) => Some(Op::StoreThisField(sym)),
                (Op::Prim(p), Op::JumpIfFalse(t)) => Some(Op::PrimJumpIfFalse(p, t)),
                (Op::Unit | Op::Null | Op::This | Op::Int(_) | Op::Long(_) | Op::Double(_) | Op::Float(_) | Op::Byte(_) | Op::Short(_) | Op::Char(_) | Op::Bool(_) | Op::Str(_), Op::Pop) => {
                    self.unemit();
                    return usize::MAX;
                }
                _ => None,
            };
            if let Some(f) = folded {
                self.unemit();
                return self.emit(f);
            }
        }
        self.emit(op)
    }

    /// The place of the next instruction, a forward jump's target: nothing pending there, since a
    /// node's steps go with its first instruction.
    fn here(&mut self) -> u32 {
        debug_assert!(self.pending == 0, "a jump's target with steps pending");
        self.target = self.ins.len();
        self.ins.len() as u32
    }

    /// A loop's head, which the jump back reaches: the steps pending charged once before it.
    fn head(&mut self) -> u32 {
        if self.pending > 0 {
            self.emit(Op::Charge);
        }
        self.target = self.ins.len();
        self.ins.len() as u32
    }

    fn patch(&mut self, at: usize, to: u32) {
        match &mut self.ins[at].op {
            Op::Jump(t) | Op::JumpIfFalse(t) | Op::JumpIfTrue(t) | Op::PrimJumpIfFalse(_, t) => *t = to,
            Op::Case { next, .. } | Op::Guard { next, .. } | Op::TupleCase { next, .. } | Op::TupleGuard { next, .. } => *next = to,
            Op::NewPrep { skip, .. } | Op::JsPrep { skip, .. } | Op::SuperClassPrep { skip, .. } => *skip = to,
            _ => unreachable!("a patch of an instruction that does not jump"),
        }
    }
}

/// The numeric kind of the value an instruction makes, where its operation decides it.
fn made_kind(op: Op) -> Option<u8> {
    use PrimOp::*;
    Some(match op {
        Op::Int(_) => K_INT,
        Op::Long(_) => K_LONG,
        Op::Double(_) => K_DOUBLE,
        Op::Float(_) => K_FLOAT,
        Op::Byte(_) => K_BYTE,
        Op::Short(_) => K_SHORT,
        Op::Char(_) => K_CHAR,
        Op::Prim(IntAdd | IntSub | IntMul | IntDiv | IntRem | IntAnd | IntOr | IntXor | IntShl | IntShr | IntUshr) => K_INT,
        Op::Prim(LongAdd | LongSub | LongMul | LongDiv | LongRem | LongAnd | LongOr | LongXor | LongShl | LongShr | LongUshr) => K_LONG,
        Op::Prim(DoubleAdd | DoubleSub | DoubleMul | DoubleDiv | DoubleRem) => K_DOUBLE,
        Op::Prim(FloatAdd | FloatSub | FloatMul | FloatDiv | FloatRem) => K_FLOAT,
        _ => return None,
    })
}

impl<'a, 't> Interp<'a, 't> {
    /// Lowers a unit's body, or declines it with the reason of the first node the capability
    /// matrix leaves to the walker: the preflight, decided before any code is installed, a unit
    /// declined running whole on the walker.
    #[allow(clippy::too_many_arguments)]
    fn lower_unit(&mut self, body: TExprId, layout: Layout, tail: Option<(SymId, u32)>, exits: bool, kind: u8, absent: u64, proxies: u32, entry: Opener) -> Result<Rc<Code>, Option<Walked>> {
        let entry = *layout.openers.get(&entry).expect("the unit's entry frame");
        let mut b = self.lowering(layout, tail);
        if tail.is_some() {
            self.lower_tail(&mut b, body);
        } else {
            self.lower(&mut b, body);
        }
        b.emit(Op::End);
        self.made(b, exits, tail, kind, body, (absent, proxies, entry))
    }

    /// A lowering over `layout`'s addresses and descriptors.
    fn lowering(&self, layout: Layout, tail: Option<(SymId, u32)>) -> Lowering {
        let addrs = layout.addrs.iter().filter_map(|&(e, d, s, o)| Some((e, (d, s, u16::try_from(*layout.openers.get(&o)?).ok()?)))).collect();
        let kinds_now = !self.typer.types.overlays_on();
        Lowering { ins: Vec::new(), tries: Vec::new(), tuple_cases: Vec::new(), scopes: layout.scopes, openers: layout.openers, pending: 0, addrs, tail, kinds_now, target: 0, declined: None, nodes: 0 }
    }

    /// The code of a lowering, unless the preflight declined it.
    #[allow(clippy::too_many_arguments)]
    fn made(&mut self, b: Lowering, exits: bool, tail: Option<(SymId, u32)>, kind: u8, body: TExprId, (absent, proxies, entry): (u64, u32, u32)) -> Result<Rc<Code>, Option<Walked>> {
        if let Some(why) = b.declined {
            return Err(Some(why));
        }
        let shifts = self.prog().shifts;
        let first_id = self.descs.register(&b.scopes);
        let entry_id = if entry == NO_ENTRY { 0 } else { first_id + entry };
        let code = Code { ins: b.ins.into_boxed_slice(), tries: b.tries.into_boxed_slice(), tuple_cases: b.tuple_cases.into_boxed_slice(), scopes: b.scopes.into_boxed_slice(), body, shifts, absent, proxies, first_id, entry_id, exits, tail, kind };
        if self.code_stats {
            count(&COUNTERS.lowered[kind as usize], 1);
            count(&COUNTERS.nodes, b.nodes as u64);
            count(&COUNTERS.instructions, code.ins.len() as u64);
            count(&COUNTERS.made, code.bytes() as u64);
            let live = COUNTERS.live.fetch_add(code.bytes() as i64, std::sync::atomic::Ordering::Relaxed) + code.bytes() as i64;
            COUNTERS.peak.fetch_max(live, std::sync::atomic::Ordering::Relaxed);
        }
        Ok(Rc::new(code))
    }

    /// The code of a loop from the head of an iteration on (`While`, past its node's step): the
    /// condition, the body, and the loop's `Unit` at its end.
    fn lower_loop(&mut self, e: TExprId, c: TExprId, body: TExprId, layout: Layout) -> Result<Rc<Code>, Option<Walked>> {
        let mut b = self.lowering(layout, None);
        self.lower_while(&mut b, c, body);
        b.emit(Op::End);
        self.made(b, false, None, LOOP, e, (0, 0, NO_ENTRY))
    }

    /// Why the walker has to evaluate `node`, where the capability matrix says so: the reason the
    /// preflight declines the node's unit with.
    fn walked(&self, node: TExpr) -> Option<Walked> {
        Some(match node {
            TExpr::Splice(_) | TExpr::JsImport(_) | TExpr::JsGlobal(..) | TExpr::JsSelect(..) | TExpr::ObjLit(_) | TExpr::Spread(_) => Walked::Refused,
            // A call the walker runs as a local trait's lifted member (`invoke_lifted`), told apart
            // by its own classification: the member's proxies and the call's arguments for them.
            // `super` calls never are (`call_super_class`, `call_super_proxied`).
            TExpr::CallMethod(r, s, args) if !matches!(self.prog().expr(r), TExpr::Super(_)) && self.lifted_arity(s, args).is_some() => Walked::Lifted,
            TExpr::Assign(target, _) if !matches!(self.prog().expr(target), TExpr::Local(_) | TExpr::Field(..) | TExpr::Static(_)) => Walked::Assign,
            _ => return None,
        })
    }

    fn lower_list(&mut self, b: &mut Lowering, items: ListRef) {
        for i in items.range() {
            let x = self.prog().expr_lists[i];
            self.lower(b, x);
        }
    }

    /// The read of local `s` at expression `e`: at its layout's address, else by symbol.
    fn lower_local(&mut self, b: &mut Lowering, e: TExprId, s: SymId) {
        match b.addrs.get(&e) {
            Some(&(depth, slot, scope)) => b.emit(Op::Local { e, sym: s, depth, slot, kind: 0, scope }),
            None => b.emit(Op::LocalSym { e, sym: s }),
        };
    }

    /// What `eval` does with `e`: its step, the node, and its numeric widening.
    fn lower(&mut self, b: &mut Lowering, e: TExprId) {
        let node = self.prog().expr(e);
        if let Some(why) = self.walked(node) {
            // The unit is declined; what follows is lowered for nothing and dropped.
            b.declined.get_or_insert(why);
            b.emit(Op::Unit);
            return;
        }
        b.nodes += 1;
        b.pending += 1;
        match node {
            TExpr::Int(v) => {
                let op = match self.int_literal(e, v) {
                    Value::Byte(x) => Op::Byte(x),
                    Value::Short(x) => Op::Short(x),
                    Value::Char(x) => Op::Char(x),
                    _ => Op::Int(v),
                };
                b.emit(op);
            }
            TExpr::Long(v) => _ = b.emit(Op::Long(v)),
            TExpr::Double(v) => {
                let float = self.prog().type_of(e).is_some_and(|t| self.typer.types.canonical(self.typer.widen_lit(t)) == self.typer.b.t_float);
                b.emit(if float { Op::Float(v as f32) } else { Op::Double(v) });
            }
            TExpr::Bool(v) => _ = b.emit(Op::Bool(v)),
            TExpr::Char(c) => _ = b.emit(Op::Char(c)),
            TExpr::Str(s) => _ = b.emit(Op::Str(s)),
            TExpr::Unit => _ = b.emit(Op::Unit),
            TExpr::Null => _ = b.emit(Op::Null),
            TExpr::Local(s) => self.lower_local(b, e, s),
            TExpr::This | TExpr::Super(_) => _ = b.emit(Op::This),
            TExpr::Static(s) => _ = b.emit(Op::Static(s)),
            TExpr::Module(c) => _ = b.emit(Op::Module(c)),
            TExpr::ClassOf(c, dims) => _ = b.emit(Op::ClassOf(c, dims)),
            TExpr::Field(r, s) => {
                self.lower(b, r);
                b.emit_folding(Op::Field { e, sym: s });
            }
            TExpr::CallStatic(s, args) => {
                if self.syms().sym(s).owner == Owner::Local {
                    self.lower_local(b, e, s);
                    self.lower_list(b, args);
                    b.emit(Op::Apply { argc: args.len });
                } else {
                    b.emit(Op::InitFile(s));
                    self.lower_list(b, args);
                    b.emit(Op::CallStatic { e, sym: s, argc: args.len });
                }
            }
            TExpr::CallMethod(r, s, args) => match self.prog().expr(r) {
                // `super`'s calls evaluate no receiver (`eval_node`'s `CallMethod`).
                TExpr::Super(SuperTarget::Class(_)) => {
                    let prep = b.emit(Op::SuperClassPrep { e, skip: 0 });
                    self.lower_list(b, args);
                    b.emit(Op::CallSuperClass { e, argc: args.len });
                    let to = b.here();
                    b.patch(prep, to);
                }
                TExpr::Super(_) => {
                    self.lower_list(b, args);
                    b.emit(Op::CallSuper { e, sym: s, argc: args.len });
                }
                _ => {
                    self.lower(b, r);
                    self.lower_list(b, args);
                    b.emit(Op::CallMethod { e, sym: s, argc: args.len });
                }
            },
            TExpr::Default(_, _, args) => {
                self.lower_list(b, args);
                b.emit(Op::DefaultArg { e, argc: args.len });
            }
            TExpr::CallClosure(f, args) => {
                self.lower(b, f);
                self.lower_list(b, args);
                b.emit(Op::Apply { argc: args.len });
            }
            TExpr::New(c, args) => {
                let prep = b.emit(Op::NewPrep { e, class: c, skip: 0 });
                self.lower_list(b, args);
                b.emit(Op::New { class: c, argc: args.len });
                let to = b.here();
                b.patch(prep, to);
            }
            TExpr::NewVia(s, args) => {
                self.lower_list(b, args);
                b.emit(Op::NewVia { sym: s, argc: args.len });
            }
            TExpr::Lambda(params, body) => _ = b.emit(Op::Lambda { params, body }),
            TExpr::If(c, t, els) => self.lower_if(b, c, t, els, false),
            TExpr::While(c, body) => self.lower_while(b, c, body),
            TExpr::Block(stmts, res) => match self.tuple_match_shape(e, stmts, res).filter(|_| !stmts.is_empty()) {
                Some((c, args, cases)) => self.lower_tuple_match(b, c, args, cases, false),
                None => self.lower_block(b, stmts, res, false),
            },
            TExpr::Assign(target, value) => {
                match self.prog().expr(target) {
                    TExpr::Local(s) => {
                        self.lower(b, value);
                        match b.addrs.get(&target) {
                            Some(&(depth, slot, scope)) => b.emit(Op::StoreLocal { e: target, sym: s, depth, slot, scope }),
                            None => b.emit(Op::StoreSym { e: target, sym: s }),
                        };
                    }
                    TExpr::Field(r, s) => {
                        self.lower(b, value);
                        self.lower(b, r);
                        b.emit_folding(Op::StoreField(s));
                    }
                    TExpr::Static(s) => {
                        b.emit(Op::InitFile(s));
                        self.lower(b, value);
                        b.emit(Op::StoreStatic(s));
                    }
                    // Any other target the walker takes whole (`walked`, before this match).
                    _ => unreachable!("an assignment the walker takes"),
                }
                b.emit(Op::Unit);
            }
            TExpr::Match(scrut, cases) => {
                self.lower(b, scrut);
                self.lower_cases(b, cases, false);
            }
            TExpr::Prim(op @ (PrimOp::BoolAnd | PrimOp::BoolOr), x, y) => self.lower_logic(b, op, x, y, false),
            TExpr::Prim(op, x, y) => {
                self.lower(b, x);
                self.lower(b, y);
                b.emit(Op::Prim(op));
            }
            TExpr::Unary(op, x) => {
                self.lower(b, x);
                b.emit(Op::Unary(op));
            }
            TExpr::ToStr(inner, conv) => {
                self.lower(b, inner);
                b.emit(Op::ToStr(conv));
            }
            TExpr::StrConcat(items) => {
                b.emit(Op::ConcatBegin);
                self.lower_concat(b, items);
                b.emit(Op::ConcatEnd);
            }
            TExpr::Js(s, args) => {
                let quote = &*self.prog().strings[s.idx()] == "$quote";
                let prep = b.emit(Op::JsPrep { e, s, skip: 0, quote });
                self.lower_list(b, args);
                b.emit(Op::JsCall { s, argc: args.len, quote });
                let to = b.here();
                b.patch(prep, to);
            }
            TExpr::TypeTest(inner, test) => {
                self.lower(b, inner);
                b.emit(Op::TypeTest(test));
            }
            TExpr::Cast(inner, op, _) => {
                self.lower(b, inner);
                b.emit(Op::Cast(op));
            }
            TExpr::SeqLit(items) => {
                self.lower_list(b, items);
                b.emit(Op::SeqLit(items.len));
            }
            TExpr::ArrayLit(items) => {
                self.lower_list(b, items);
                b.emit(Op::ArrayLit(items.len));
            }
            TExpr::Index(r, i) => {
                self.lower(b, r);
                b.emit(Op::Index(i));
            }
            TExpr::Return(v) => {
                self.lower(b, v);
                b.emit(Op::Return);
            }
            TExpr::Throw(inner, _) => {
                self.lower(b, inner);
                b.emit(Op::Throw);
            }
            TExpr::Try(i) => self.lower_try(b, i, false),
            TExpr::Splice(_) | TExpr::JsImport(_) | TExpr::JsGlobal(..) | TExpr::JsSelect(..) | TExpr::ObjLit(_) | TExpr::Spread(_) => unreachable!("a node the walker takes"),
        }
        self.lower_adapt(b, e);
    }

    /// The numeric widening `eval` gives node `e` on its way out.
    fn lower_adapt(&mut self, b: &mut Lowering, e: TExprId) {
        let known = match self.coerce.get(e.idx()) {
            Some(&k) if k != value::K_UNKNOWN => Some(k),
            _ if b.kinds_now => Some(self.coerce_kind(e)),
            _ => None,
        };
        match known {
            Some(0) => {}
            Some(k) => b.adapt(k),
            None => _ = b.emit(Op::AdaptLazy(e)),
        }
    }

    /// What `eval_tail` does with `e`: the nodes it takes itself without a widening, a self call in
    /// tail position a jump of the def's loop, any other node as `eval` takes it.
    fn lower_tail(&mut self, b: &mut Lowering, e: TExprId) {
        let Some((sym, arity)) = b.tail else { return self.lower(b, e) };
        let node = self.prog().expr(e);
        match node {
            TExpr::Block(stmts, res) => {
                b.nodes += 1;
                b.pending += 1;
                match self.tuple_match_shape(e, stmts, res).filter(|_| !stmts.is_empty()) {
                    Some((c, args, cases)) => self.lower_tuple_match(b, c, args, cases, true),
                    None => self.lower_block(b, stmts, res, true),
                }
            }
            TExpr::If(c, t, els) => {
                b.nodes += 1;
                b.pending += 1;
                self.lower_if(b, c, t, els, true);
            }
            TExpr::Match(scrut, cases) => {
                b.nodes += 1;
                b.pending += 1;
                self.lower(b, scrut);
                self.lower_cases(b, cases, true);
            }
            TExpr::Try(i) if self.prog().tries[i as usize].finalizer.is_none() => {
                b.nodes += 1;
                b.pending += 1;
                self.lower_try(b, i, true);
            }
            TExpr::Prim(op @ (PrimOp::BoolAnd | PrimOp::BoolOr), x, y) => {
                b.nodes += 1;
                b.pending += 1;
                self.lower_logic(b, op, x, y, true);
            }
            TExpr::Return(v) => {
                b.nodes += 1;
                b.pending += 1;
                self.lower_tail(b, v);
            }
            TExpr::CallStatic(s, _) | TExpr::CallMethod(_, s, _) if s == sym => match self.prog().self_call(sym, arity as usize, e) {
                Some((recv, args)) => {
                    b.nodes += 1;
                    b.pending += 1;
                    if let Some(r) = recv {
                        self.lower(b, r);
                    }
                    self.lower_list(b, args);
                    b.emit(Op::TailCall { argc: args.len, recv: recv.is_some() });
                }
                None => self.lower(b, e),
            },
            _ => self.lower(b, e),
        }
    }

    /// A `while` loop (`eval_node`'s), its node's step charged before its head.
    fn lower_while(&mut self, b: &mut Lowering, c: TExprId, body: TExprId) {
        let head = b.head();
        self.lower(b, c);
        let exit = b.emit_folding(Op::JumpIfFalse(0));
        self.lower(b, body);
        b.emit_folding(Op::Pop);
        b.emit(Op::Jump(head));
        let to = b.here();
        b.patch(exit, to);
        b.emit(Op::Unit);
    }

    fn lower_if(&mut self, b: &mut Lowering, c: TExprId, t: TExprId, els: Option<TExprId>, tail: bool) {
        self.lower(b, c);
        let skip = b.emit_folding(Op::JumpIfFalse(0));
        self.lower_in(b, t, tail);
        let done = b.emit(Op::Jump(0));
        let to = b.here();
        b.patch(skip, to);
        match els {
            Some(x) => self.lower_in(b, x, tail),
            None => _ = b.emit(Op::Unit),
        }
        let to = b.here();
        b.patch(done, to);
    }

    fn lower_in(&mut self, b: &mut Lowering, e: TExprId, tail: bool) {
        if tail {
            self.lower_tail(b, e)
        } else {
            self.lower(b, e)
        }
    }

    /// `&&` and `||`, short-circuiting: the left operand's value decides, the right one's is the
    /// result, checked a `Boolean` outside a tail position (`eval_prim`, `eval_tail`).
    fn lower_logic(&mut self, b: &mut Lowering, op: PrimOp, x: TExprId, y: TExprId, tail: bool) {
        self.lower(b, x);
        let and = op == PrimOp::BoolAnd;
        let short = b.emit(if and { Op::JumpIfFalse(0) } else { Op::JumpIfTrue(0) });
        self.lower_in(b, y, tail);
        if !tail {
            b.emit(Op::CheckBool);
        }
        let done = b.emit(Op::Jump(0));
        let to = b.here();
        b.patch(short, to);
        b.emit(Op::Bool(!and));
        let to = b.here();
        b.patch(done, to);
    }

    /// A block as `eval` runs it (`block_frame`, `leave_block`), its value in tail position when
    /// `tail`.
    fn lower_block(&mut self, b: &mut Lowering, stmts: ListRef, res: TExprId, tail: bool) {
        if stmts.is_empty() {
            return self.lower_in(b, res, tail);
        }
        let declares = self.lower_stmts(b, stmts);
        self.lower_in(b, res, tail);
        if declares {
            let scope = b.scope(Opener::Block(stmts.start));
            b.emit(Op::PopFrame { scope });
        }
    }

    /// A block's statements as `block_frame` runs them, in a frame of their own (whose leaving is
    /// the caller's, when this says it declares) when one of them declares something.
    fn lower_stmts(&mut self, b: &mut Lowering, stmts: ListRef) -> bool {
        let declares = self.prog().stmts[stmts.range()].iter().any(|s| !matches!(s, TStmt::Expr(_)));
        if declares {
            let scope = b.scope(Opener::Block(stmts.start));
            b.emit(Op::PushFrame { scope, cap: b.cap(scope) });
            for i in stmts.range() {
                if let TStmt::Fun(f) = self.prog().stmts[i] {
                    b.emit(Op::BindDef(f));
                }
            }
        }
        for i in stmts.range() {
            match self.prog().stmts[i] {
                TStmt::Expr(x) => {
                    self.lower(b, x);
                    b.emit_folding(Op::Pop);
                }
                TStmt::Fun(_) => {}
                TStmt::Val(sym, init) => {
                    if self.syms().sym(sym).mods & crate::ast::mods::LAZY != 0 {
                        b.emit(Op::BindLazy { sym, init });
                    } else {
                        self.lower(b, init);
                        b.emit(Op::Bind(sym));
                    }
                }
                TStmt::Pat(pat, init) => {
                    self.lower(b, init);
                    b.emit(Op::BindPat(pat));
                }
            }
        }
        declares
    }

    /// The operands of a chain of `+` into the chain's text, as `concat` takes them: a rendering's
    /// own step and its operand, a head that goes on in a chain (its step, then its operands into
    /// the same text) or in a block (its step, the block's statements in their frame, then the head
    /// its result is), any other operand evaluated and its text appended. Where a head goes is the
    /// program's to say (`Program::chain_head`).
    fn lower_concat(&mut self, b: &mut Lowering, items: ListRef) {
        for i in items.range() {
            let item = self.prog().expr_lists[i];
            match self.prog().expr(item) {
                TExpr::ToStr(inner, conv) if conv.is_rendering() => {
                    b.pending += 1;
                    self.lower(b, inner);
                    b.emit(Op::ConcatRender(conv));
                }
                TExpr::StrConcat(_) | TExpr::Block(..) | TExpr::If(..) if i == items.start as usize && self.lower_concat_head(b, item) => {}
                _ => {
                    self.lower(b, item);
                    b.emit(Op::ConcatPlain);
                }
            }
        }
    }

    /// A chain's head that goes on in a chain or a block (`concat_head`): `false` for an operand.
    fn lower_concat_head(&mut self, b: &mut Lowering, head: TExprId) -> bool {
        match self.prog().chain_head(head) {
            ChainHead::Operand => false,
            ChainHead::Chain(items) => {
                b.pending += 1;
                self.lower_concat(b, items);
                true
            }
            ChainHead::Block(stmts, res) => {
                b.pending += 1;
                let declares = !stmts.is_empty() && self.lower_stmts(b, stmts);
                let went_on = self.lower_concat_head(b, res);
                debug_assert!(went_on, "a block heading a chain ends in its head");
                if declares {
                    let scope = b.scope(Opener::Block(stmts.start));
                    b.emit(Op::PopFrame { scope });
                }
                true
            }
        }
    }

    /// The cases of a match over the scrutinee on the stack (`eval_match`).
    fn lower_cases(&mut self, b: &mut Lowering, cases: ListRef, tail: bool) {
        let mut misses: Vec<usize> = Vec::new();
        let mut done: Vec<usize> = Vec::new();
        for i in cases.range() {
            let case = self.prog().cases[i];
            let to = b.here();
            for m in misses.drain(..) {
                b.patch(m, to);
            }
            let binds = self.pat_binds_cached(case.pat);
            let scope = if binds { b.scope(Opener::Case(i as u32)) } else { u32::MAX };
            misses.push(b.emit(Op::Case { pat: case.pat, binds, next: 0, scope, cap: b.cap(scope) }));
            if let Some(g) = case.guard {
                self.lower(b, g);
                misses.push(b.emit(Op::Guard { binds, next: 0 }));
            }
            self.lower_in(b, case.body, tail);
            b.emit(Op::CaseEnd { binds, scope });
            done.push(b.emit(Op::Jump(0)));
        }
        let to = b.here();
        for m in misses {
            b.patch(m, to);
        }
        b.emit(Op::NoCase);
        let to = b.here();
        for d in done {
            b.patch(d, to);
        }
    }

    /// A case lambda's match over the tuple of its parameters (`eval_tuple_match`): the components
    /// evaluated without the tuple, the steps of the block's node alone (the tuple's creation, the
    /// match and its scrutinee are no nodes the walker evaluates), each case over the components.
    fn lower_tuple_match(&mut self, b: &mut Lowering, c: ClassId, args: ListRef, cases: ListRef, tail: bool) {
        self.lower_list(b, args);
        let n = args.len;
        let mut misses: Vec<usize> = Vec::new();
        let mut done: Vec<usize> = Vec::new();
        for i in cases.range() {
            let case = self.prog().cases[i];
            let to = b.here();
            for m in misses.drain(..) {
                b.patch(m, to);
            }
            let binds = self.pat_binds_cached(case.pat);
            let comps: Box<[Comp]> = match self.prog().pats[case.pat.idx()] {
                TPat::Class(_, _, _, subs) => subs
                    .range()
                    .map(|i| {
                        let sub = self.prog().pat_lists[i];
                        match self.prog().pats[sub.idx()] {
                            TPat::Wildcard => Comp::Wildcard,
                            TPat::Bind(sym, None) => Comp::Bind(sym),
                            TPat::Equals(..) => Comp::Literal(sub),
                            _ => Comp::Other(sub),
                        }
                    })
                    .collect(),
                // `tuple_match_fits` gave the shape only for cases that are all tuple patterns of the
                // components' arity, where the walker's `eval_tuple_match` stops at another.
                _ => unreachable!("a tuple match's case of no tuple pattern"),
            };
            debug_assert_eq!(comps.len(), n as usize);
            let scope = if binds { b.scope(Opener::Case(i as u32)) } else { u32::MAX };
            b.tuple_cases.push(TupleCasePlan { comps, scope, cap: b.cap(scope) });
            let k = (b.tuple_cases.len() - 1) as u32;
            misses.push(b.emit(Op::TupleCase { case: k, binds, next: 0 }));
            if let Some(g) = case.guard {
                self.lower(b, g);
                misses.push(b.emit(Op::TupleGuard { binds, next: 0 }));
            }
            b.emit(Op::TupleBody(n));
            self.lower_in(b, case.body, tail);
            b.emit(Op::TupleEnd { binds });
            done.push(b.emit(Op::Jump(0)));
        }
        let to = b.here();
        for m in misses {
            b.patch(m, to);
        }
        b.emit(Op::NoTupleCase { class: c, n });
        let to = b.here();
        for d in done {
            b.patch(d, to);
        }
    }

    /// A `try` (`eval_try`): the instruction that runs it, then its ranges, each ending in `End`.
    fn lower_try(&mut self, b: &mut Lowering, i: u32, tail: bool) {
        let (body, cases, finalizer, wraps) = {
            let t = &self.prog().tries[i as usize];
            (t.body, t.cases, t.finalizer, t.wraps)
        };
        let k = b.tries.len();
        b.tries.push(TryPlan { body: 0, cases: Box::new([]), finalizer: None, wraps, end: 0 });
        b.emit(Op::Try(k as u32));
        let body_at = b.here();
        self.lower(b, body);
        b.emit(Op::End);
        let mut plans = Vec::with_capacity(cases.len as usize);
        for c in cases.range() {
            let case = self.prog().cases[c];
            let guard = case.guard.map(|g| {
                let at = b.here();
                self.lower(b, g);
                b.emit(Op::End);
                at
            });
            let at = b.here();
            self.lower_in(b, case.body, tail);
            b.emit(Op::End);
            plans.push(CasePlan { pat: case.pat, scope: b.scope(Opener::Case(c as u32)), guard, body: at });
        }
        let finalizer = finalizer.map(|f| {
            let at = b.here();
            self.lower(b, f);
            b.emit(Op::End);
            at
        });
        let end = b.here();
        b.tries[k] = TryPlan { body: body_at, cases: plans.into_boxed_slice(), finalizer, wraps, end };
    }

    // ---- the dispatcher ----

    /// Runs `code` from `start` over the frame `fr`: a unit's body when `top`, else a range of a
    /// `try`. Its operands live on a stack of its own; the frames the code opened and the values it
    /// left are let go on every exit.
    pub(super) fn exec(&mut self, code: &Code, start: u32, fr: &Rc<Frame>, cx: &Ctx, top: bool) -> R {
        if self.code_stats && start == 0 {
            count(&COUNTERS.runs[code.kind as usize], 1);
        }
        // The incoming layout: the frame a function's or a lambda's code is entered with carries the
        // descriptor of its activation for that layout (`call_fun`, `call_closure`).
        debug_assert!(!(start == 0 && top) || code.entry_id == 0 || fr.desc == code.entry_id, "a unit entered in a frame of another layout");
        let mut st = self.take_vec(4);
        let mut cur = fr.clone();
        let (chains, quotes) = (self.exec.concats.len(), self.exec.quote_renames.len());
        let r = self.run(code, start, &mut cur, &mut st, cx, top);
        // What an exit by a throw or a return left goes before the frames it left open: the operands
        // abandoned on the stack, the chains of `+` under way with the operands they keep, and the
        // quotes' renames, as the walker's evaluations drop theirs on the way out (the exit itself is
        // `r`'s). An abandoned operand (a closure over a block's frame) would otherwise keep
        // `leave_block` from cutting the cycle of that frame's local defs.
        self.recycle_vec(st);
        if self.exec.concats.len() > chains {
            self.exec.concats.truncate(chains);
        }
        if self.exec.quote_renames.len() > quotes {
            self.exec.quote_renames.truncate(quotes);
        }
        if !Rc::ptr_eq(&cur, fr) {
            self.unwind_frames(cur, fr);
        }
        r
    }

    /// Leaves the frames above `root` that an exit left open, innermost first, as the walker's
    /// evaluations leave theirs on their way out.
    #[cold]
    #[inline(never)]
    fn unwind_frames(&mut self, mut cur: Rc<Frame>, root: &Rc<Frame>) {
        while !Rc::ptr_eq(&cur, root) {
            let Some(outer) = cur.parent.clone() else { break };
            self.leave_block(cur, &outer);
            cur = outer;
        }
    }

    #[inline]
    fn charge(&mut self, n: u32) -> R<()> {
        if self.exec.steps >= n as u64 {
            self.exec.steps -= n as u64;
            return Ok(());
        }
        self.charge_slowly(n)
    }

    /// The steps one by one, as the walker charges them, across the budget's slices.
    #[cold]
    #[inline(never)]
    fn charge_slowly(&mut self, n: u32) -> R<()> {
        for _ in 0..n {
            self.step()?;
        }
        Ok(())
    }

    /// The top `n` operands, in their order, as an argument vector.
    #[inline]
    fn args_of(&mut self, st: &mut Vec<Value>, n: u32) -> Vec<Value> {
        let mut out = self.take_vec(n as usize);
        match n {
            0 => {}
            1 => out.push(st.pop().expect("an operand")),
            2 => {
                let y = st.pop().expect("an operand");
                out.push(st.pop().expect("an operand"));
                out.push(y);
            }
            _ => {
                let at = st.len() - n as usize;
                out.extend(st.drain(at..));
            }
        }
        out
    }

    #[inline]
    fn bool_of(&mut self, v: Value) -> R<bool> {
        match v {
            Value::Bool(b) => Ok(b),
            other => self.no_boolean(other),
        }
    }

    #[cold]
    #[inline(never)]
    fn no_boolean<T>(&mut self, v: Value) -> R<T> {
        let shown = self.to_str(&v)?;
        self.unsupported(format!("a condition that is no Boolean: {}", shown))
    }

    /// Leaves a case's frame of binders.
    #[inline]
    fn leave_case(&mut self, cur: &mut Rc<Frame>) {
        let outer = cur.parent.clone().expect("a case's frame has a parent");
        let inner = std::mem::replace(cur, outer);
        self.release_frame(inner);
    }

    fn run(&mut self, code: &Code, start: u32, cur: &mut Rc<Frame>, st: &mut Vec<Value>, cx: &Ctx, top: bool) -> R {
        let mut pc = start as usize;
        loop {
            let ins = &code.ins[pc];
            pc += 1;
            if ins.charge != 0 {
                self.charge(ins.charge)?;
            }
            match ins.op {
                Op::Int(v) => st.push(Value::Int(v)),
                Op::Long(v) => st.push(Value::Long(v)),
                Op::Double(v) => st.push(Value::Double(v)),
                Op::Float(v) => st.push(Value::Float(v)),
                Op::Byte(v) => st.push(Value::Byte(v)),
                Op::Short(v) => st.push(Value::Short(v)),
                Op::Char(v) => st.push(Value::Char(v)),
                Op::Bool(v) => st.push(Value::Bool(v)),
                Op::Str(s) => st.push(Value::Str(self.string_lit(s))),
                Op::Unit => st.push(Value::Unit),
                Op::Null => st.push(Value::Null),
                Op::This => st.push(cx.this.clone()),
                Op::Local { e, sym, depth, slot, kind, scope } => {
                    // A frame the unit opened, with the descriptor that gave the address (the
                    // entry frame checked where the run starts, `exec`); the binding there is
                    // the symbol's once made.
                    let found = match frame_at(cur, depth) {
                        Some(f) => {
                            debug_assert!(f.desc == code.frame_id(scope as u32), "a lowered read in a frame its unit did not open");
                            let vals = f.vals.borrow();
                            match vals.get(slot as usize) {
                                Some((s, v)) if *s == sym && !matches!(v, Value::Fun(c) if matches!(c.kind, ClosureKind::Lazy(_))) => Some(v.clone()),
                                // A lazy local's thunk, forced by `local`, or a binding to come.
                                _ => None,
                            }
                        }
                        None => None,
                    };
                    let v = match found {
                        Some(v) => v,
                        None => self.local(e, sym, cur, cx)?,
                    };
                    st.push(if kind != 0 { Self::adapt(v, kind) } else { v });
                }
                Op::LocalSym { e, sym } => {
                    let v = match self.hinted(e, sym, cur) {
                        Some(v) => v,
                        None => self.local(e, sym, cur, cx)?,
                    };
                    st.push(v);
                }
                Op::Static(s) => st.push(self.static_value(s)?),
                Op::Module(c) => st.push(self.module(c)?),
                Op::ClassOf(c, dims) => st.push(self.class_literal(c, dims)),
                Op::Field { e, sym } => {
                    let recv = st.pop().expect("an operand");
                    st.push(self.field_at(e, recv, sym)?);
                }
                Op::ThisField { e, sym } => st.push(self.field_at(e, cx.this.clone(), sym)?),
                Op::InitFile(s) => {
                    let info = self.syms().sym(s);
                    let (owner, file) = (info.owner, info.file);
                    self.init_file_of(s, owner, file)?;
                }
                Op::CallStatic { e, sym, argc } => {
                    let args = self.args_of(st, argc);
                    // An object's member through its module (`call_static_at`).
                    let v = match self.syms().sym(sym).owner {
                        Owner::Class(c) => {
                            let module = self.module(c)?;
                            match self.site_fun(e, &module) {
                                Some((f, owner, parent)) => self.call_fun(f, module, args, parent, Some(owner))?,
                                None => self.invoke_at(e, module, sym, args)?,
                            }
                        }
                        _ => self.call_static(sym, args)?,
                    };
                    st.push(v);
                }
                Op::CallMethod { e, sym, argc } => {
                    let args = self.args_of(st, argc);
                    let recv = st.pop().expect("an operand");
                    let v = match self.site_fun(e, &recv) {
                        Some((f, owner, parent)) => self.call_fun(f, recv, args, parent, Some(owner))?,
                        None => self.invoke_at(e, recv, sym, args)?,
                    };
                    st.push(v);
                }
                Op::CallSuper { e, sym, argc } => {
                    let args = self.args_of(st, argc);
                    st.push(self.call_super_at(e, sym, args, cx)?);
                }
                Op::SuperClassPrep { e, skip } => {
                    if let Some(v) = self.super_class_prep(e, cur, cx)? {
                        st.push(v);
                        pc = skip as usize;
                    }
                }
                Op::CallSuperClass { e, argc } => {
                    let args = self.args_of(st, argc);
                    st.push(self.call_super_class_with(e, args, cx)?);
                }
                Op::DefaultArg { e, argc } => {
                    let vals = self.args_of(st, argc);
                    st.push(self.default_at(e, vals, cur, cx)?);
                }
                Op::Apply { argc } => {
                    let args = self.args_of(st, argc);
                    let f = st.pop().expect("an operand");
                    st.push(self.apply_value(f, args)?);
                }
                Op::NewPrep { e, class, skip } => {
                    if let Some(v) = self.new_prep(e, class, cur, cx)? {
                        st.push(v);
                        pc = skip as usize;
                    }
                }
                Op::New { class, argc } => {
                    let args = self.args_of(st, argc);
                    st.push(self.construct_new(class, args, cur)?);
                }
                Op::NewVia { sym, argc } => {
                    let args = self.args_of(st, argc);
                    st.push(self.construct_via_new(sym, args)?);
                }
                Op::Lambda { params, body } => {
                    self.prof_alloc(|a| a.closures += 1);
                    self.captured(cur, &cx.this);
                    st.push(Value::Fun(Rc::new(Closure { kind: ClosureKind::Lambda(params, body), env: cur.clone(), this: cx.this.clone(), def_key: cx.def_key, class: cx.class, hash: Cell::new(0) })));
                }
                Op::JsPrep { e, s, skip, quote } => {
                    if let Some(v) = self.js_prep(e, s, quote, cur, cx)? {
                        st.push(v);
                        pc = skip as usize;
                    }
                }
                Op::JsCall { s, argc, quote } => {
                    let args = self.args_of(st, argc);
                    let r = self.js_call(s, args);
                    if quote {
                        self.exec.quote_renames.pop();
                    }
                    st.push(r?);
                }
                Op::Jump(t) => pc = t as usize,
                Op::JumpIfFalse(t) => {
                    let v = st.pop().expect("an operand");
                    if !self.bool_of(v)? {
                        pc = t as usize;
                    }
                }
                Op::JumpIfTrue(t) => {
                    let v = st.pop().expect("an operand");
                    if self.bool_of(v)? {
                        pc = t as usize;
                    }
                }
                Op::CheckBool => {
                    if !matches!(st.last(), Some(Value::Bool(_))) {
                        let v = st.pop().expect("an operand");
                        return self.no_boolean(v);
                    }
                }
                Op::Pop => drop(st.pop()),
                Op::Charge => {}
                Op::PushFrame { scope, cap } => {
                    debug_assert!(code.scopes[scope as usize].syms.len().min(u16::MAX as usize) == cap as usize, "a block's frame sized off its descriptor");
                    self.prof_alloc(|a| a.frames += 1);
                    let inner = self.take_frame_as(Some(cur.clone()), cap as usize, code.frame_id(scope));
                    *cur = inner;
                }
                Op::BindDef(f) => {
                    let sym = self.prog().funs[f.idx()].sym;
                    self.captured(cur, &cx.this);
                    let v = Value::Fun(Rc::new(Closure { kind: ClosureKind::LocalDef(f), env: cur.clone(), this: cx.this.clone(), def_key: cx.def_key, class: cx.class, hash: Cell::new(0) }));
                    cur.bind(sym, v);
                }
                Op::Bind(sym) => cur.bind(sym, st.pop().expect("an operand")),
                Op::BindLazy { sym, init } => {
                    self.captured(cur, &cx.this);
                    let v = Value::Fun(Rc::new(Closure { kind: ClosureKind::Lazy(init), env: cur.clone(), this: cx.this.clone(), def_key: cx.def_key, class: cx.class, hash: Cell::new(0) }));
                    cur.bind(sym, v);
                }
                Op::BindPat(pat) => {
                    let v = st.pop().expect("an operand");
                    if !self.match_pat(pat, &v, cur, cx)? {
                        return self.match_error(v);
                    }
                }
                Op::PopFrame { scope } => {
                    debug_assert!(code.scopes[scope as usize].binds(cur), "a block's frame bound off its descriptor");
                    let outer = cur.parent.clone().expect("a block's frame has a parent");
                    let inner = std::mem::replace(cur, outer);
                    self.leave_block(inner, cur);
                }
                Op::StoreLocal { e, sym, depth, slot, scope } => {
                    let v = st.pop().expect("an operand");
                    if let Some(v) = self.store_at(cur, sym, depth, slot, Some(code.frame_id(scope as u32)), v) {
                        self.assign_local(e, sym, v, cur, cx)?;
                    }
                }
                Op::StoreSym { e, sym } => {
                    let v = st.pop().expect("an operand");
                    let hint = self.local_slots[e.idx()];
                    let v = if hint != 0 { self.store_at(cur, sym, (hint >> 16) as u16 - 1, (hint & 0xffff) as u16, None, v) } else { Some(v) };
                    if let Some(v) = v {
                        self.assign_local(e, sym, v, cur, cx)?;
                    }
                }
                Op::StoreField(s) => {
                    let recv = st.pop().expect("an operand");
                    let v = st.pop().expect("an operand");
                    self.set_field(recv, s, v)?;
                }
                Op::StoreThisField(s) => {
                    let v = st.pop().expect("an operand");
                    self.set_field(cx.this.clone(), s, v)?;
                }
                Op::StoreStatic(s) => {
                    let v = st.pop().expect("an operand");
                    self.set_static(s, v)?;
                }
                Op::PrimJumpIfFalse(op, t) => {
                    let y = st.pop().expect("an operand");
                    let x = st.pop().expect("an operand");
                    let v = match quick_prim(op, &x, &y) {
                        Some(v) => v,
                        None => self.prim(op, x, y)?,
                    };
                    if !self.bool_of(v)? {
                        pc = t as usize;
                    }
                }
                Op::Prim(op) => {
                    let y = st.pop().expect("an operand");
                    let x = st.pop().expect("an operand");
                    let v = match quick_prim(op, &x, &y) {
                        Some(v) => v,
                        None => self.prim(op, x, y)?,
                    };
                    st.push(v);
                }
                Op::Unary(op) => {
                    let x = st.pop().expect("an operand");
                    st.push(self.eval_unary(op, x)?);
                }
                Op::ToStr(conv) => {
                    let x = st.pop().expect("an operand");
                    st.push(self.to_str_conv(x, conv)?);
                }
                Op::TypeTest(test) => {
                    let x = st.pop().expect("an operand");
                    let b = self.type_test(&x, test, cur, cx)?;
                    st.push(Value::Bool(b));
                }
                Op::Cast(op) => {
                    let x = st.pop().expect("an operand");
                    st.push(self.cast(x, op, cur, cx)?);
                }
                Op::SeqLit(n) => {
                    let items = self.args_of(st, n);
                    st.push(self.array_seq_of(Value::array(items))?);
                }
                Op::ArrayLit(n) => {
                    let items = self.args_of(st, n);
                    st.push(Value::array(items));
                }
                Op::Index(i) => {
                    let x = st.pop().expect("an operand");
                    st.push(self.index_of(x, i)?);
                }
                Op::Return => {
                    let v = st.pop().expect("an operand");
                    if top && code.exits {
                        return Ok(v);
                    }
                    return Err(Control::Return(cx.def_key, v));
                }
                Op::Throw => {
                    let v = st.pop().expect("an operand");
                    return self.throw_value(v);
                }
                Op::TailCall { argc, recv } => {
                    let args = self.args_of(st, argc);
                    let recv = if recv { st.pop() } else { None };
                    return Err(Control::Tail(args, recv));
                }
                Op::Try(k) => {
                    st.push(self.run_try(code, k, cur, cx)?);
                    pc = code.tries[k as usize].end as usize;
                }
                Op::Case { pat, binds, next, scope, cap } => {
                    debug_assert!(!binds || code.scopes[scope as usize].syms.len().min(u16::MAX as usize) == cap as usize, "a case's frame sized off its descriptor");
                    let v = st.last().expect("the scrutinee");
                    if self.quick_reject(pat, v, cur) {
                        pc = next as usize;
                        continue;
                    }
                    if binds {
                        self.prof_alloc(|a| a.frames += 1);
                        let inner = self.take_frame_as(Some(cur.clone()), cap as usize, code.frame_id(scope));
                        *cur = inner;
                    }
                    let v = st.last().expect("the scrutinee");
                    if !self.match_pat(pat, v, cur, cx)? {
                        if binds {
                            self.leave_case(cur);
                        }
                        pc = next as usize;
                    }
                }
                Op::Guard { binds, next } => {
                    let v = st.pop().expect("an operand");
                    if !self.bool_of(v)? {
                        if binds {
                            self.leave_case(cur);
                        }
                        pc = next as usize;
                    }
                }
                Op::CaseEnd { binds, scope } => {
                    if binds {
                        debug_assert!(code.scopes[scope as usize].binds(cur), "a case's frame bound off its descriptor");
                        self.leave_case(cur);
                    }
                    let at = st.len() - 2;
                    st.swap_remove(at);
                }
                Op::NoCase => {
                    let v = st.pop().expect("the scrutinee");
                    return self.match_error(v);
                }
                Op::TupleCase { case, binds, next } => {
                    if !self.tuple_case(code, &code.tuple_cases[case as usize], binds, st, cur, cx)? {
                        pc = next as usize;
                    }
                }
                Op::TupleGuard { binds, next } => {
                    let v = st.pop().expect("an operand");
                    if !self.bool_of(v)? {
                        if binds {
                            self.leave_case(cur);
                        }
                        pc = next as usize;
                    }
                }
                Op::TupleBody(n) => {
                    let at = st.len() - n as usize;
                    st.truncate(at);
                }
                Op::TupleEnd { binds } => {
                    if binds {
                        self.leave_case(cur);
                    }
                }
                Op::NoTupleCase { class, n } => {
                    let vals = self.args_of(st, n);
                    let v = self.construct_new(class, vals, cur)?;
                    return self.match_error(v);
                }
                Op::ConcatBegin => {
                    self.prof_alloc(|a| a.strings += 1);
                    self.exec.concats.push((String::new(), Vec::new()));
                }
                Op::ConcatRender(conv) => {
                    let v = st.pop().expect("an operand");
                    let (mut out, mut late) = self.exec.concats.pop().expect("a chain");
                    let r = if v.renders_by_value() {
                        self.render(conv.kind(), &v, &mut out, &late)
                    } else {
                        late.push((out.len(), v));
                        Ok(())
                    };
                    self.exec.concats.push((out, late));
                    r?;
                }
                Op::ConcatPlain => {
                    let v = st.pop().expect("an operand");
                    let (mut out, late) = self.exec.concats.pop().expect("a chain");
                    let r = match &v {
                        Value::Str(s) => {
                            push_text(&mut out, &late, s);
                            Ok(())
                        }
                        v => self.render(StrKind::Generic, v, &mut out, &late),
                    };
                    self.exec.concats.push((out, late));
                    r?;
                }
                Op::ConcatEnd => {
                    let (out, late) = self.exec.concats.pop().expect("a chain");
                    let out = if late.is_empty() { out } else { self.render_late(&out, late)? };
                    st.push(Value::string(out));
                }
                Op::Adapt(k) => Self::adapt_top(st, k),
                Op::AdaptLazy(e) => {
                    let k = self.coerce_kind(e);
                    if k != 0 {
                        Self::adapt_top(st, k);
                    }
                }
                Op::End => return Ok(st.pop().expect("a result")),
            }
        }
    }

    /// A local bound outside the unit's frames where the walker last found it (`lookup`'s hint): the
    /// adapter. The frame the hint reaches is checked by its descriptor when a lowered unit made it
    /// (the descriptor's slot of the symbol, the hint's or another known one), by the symbol at the
    /// hint's slot when the walker, a splice root or a default's evaluation made it; a lazy local's
    /// thunk and a miss are the walker's (`local`).
    #[inline]
    fn hinted(&self, e: TExprId, sym: SymId, cur: &Rc<Frame>) -> Option<Value> {
        let hint = self.local_slots[e.idx()];
        if hint == 0 {
            return None;
        }
        let f = frame_at(cur, (hint >> 16) as u16 - 1)?;
        let slot = self.adapted_slot(f, sym, (hint & 0xffff) as usize)?;
        let vals = f.vals.borrow();
        match vals.get(slot) {
            Some((s, v)) if *s == sym && !matches!(v, Value::Fun(c) if matches!(c.kind, ClosureKind::Lazy(_))) => Some(v.clone()),
            _ => None,
        }
    }

    /// The slot a hint names in frame `f`, as the adapter checks it: against the frame's descriptor
    /// when lowered code this table holds made the frame (`ScopeDesc::slot_of`), the hint's own
    /// otherwise, which its reader checks by the symbol bound there.
    #[inline]
    fn adapted_slot(&self, f: &Frame, sym: SymId, slot: usize) -> Option<usize> {
        if f.desc != 0 {
            if let Some(d) = self.descs.get(f.desc) {
                return d.slot_of(sym, slot);
            }
        }
        Some(slot)
    }

    /// Whether frame `f` binds what its descriptor says, when it has one this table holds.
    #[allow(dead_code)]
    pub(super) fn binds_its_desc(&self, f: &Frame) -> bool {
        self.descs.get(f.desc).is_none_or(|d| d.binds(f))
    }

    #[inline]
    fn adapt_top(st: &mut [Value], k: u8) {
        if let Some(top) = st.last_mut() {
            let v = std::mem::replace(top, Value::Unit);
            *top = Self::adapt(v, k);
        }
    }

    /// The function a method call at `e` runs on `recv`, when the site's memo has it for the
    /// receiver's class (`invoke_at`), with the scope it reads its captures from and its class
    /// (`invoke_member`'s `Member::Fun`).
    #[inline]
    fn site_fun(&self, e: TExprId, recv: &Value) -> Option<(FunId, ClassId, Option<Rc<Frame>>)> {
        let Value::Obj(o) = recv else { return None };
        let slot = *self.call_sites.get(e.idx())?;
        let (c, _, m) = self.site_entries.get(slot as usize)?;
        let Member::Fun(f, owner) = *m else { return None };
        if *c != o.class {
            return None;
        }
        let parent = if owner == o.class {
            o.env.clone()
        } else {
            match self.syms().class(owner).owner {
                Owner::Local => o.env.clone(),
                Owner::Class(_) if self.prog().funs[f.idx()].captures > 0 => o.env.clone(),
                _ => None,
            }
        };
        Some((f, owner, parent))
    }

    /// Whether the case of pattern `pat` takes the `n` components on top (`eval_tuple_match`): its
    /// literal sub-patterns rejecting first, then the others, then each component matched in its
    /// frame of binders, opened when it binds and left again when it does not take them.
    #[inline(never)]
    fn tuple_case(&mut self, code: &Code, plan: &TupleCasePlan, binds: bool, st: &mut [Value], cur: &mut Rc<Frame>, cx: &Ctx) -> R<bool> {
        let vals = &st[st.len() - plan.comps.len()..];
        // The literal components first, then the others (a wildcard's and a binder's reject nothing).
        for (comp, v) in plan.comps.iter().zip(vals) {
            if let Comp::Literal(sub) = *comp {
                if self.quick_reject(sub, v, cur) {
                    return Ok(false);
                }
            }
        }
        for (comp, v) in plan.comps.iter().zip(vals) {
            if let Comp::Other(sub) = *comp {
                if self.quick_reject(sub, v, cur) {
                    return Ok(false);
                }
            }
        }
        if binds {
            self.prof_alloc(|a| a.frames += 1);
            let inner = self.take_frame_as(Some(cur.clone()), plan.cap as usize, code.frame_id(plan.scope));
            *cur = inner;
        }
        for (comp, v) in plan.comps.iter().zip(vals) {
            match *comp {
                Comp::Wildcard => {}
                Comp::Bind(sym) => cur.bind(sym, v.clone()),
                Comp::Literal(sub) | Comp::Other(sub) => {
                    if !self.match_pat(sub, v, cur, cx)? {
                        if binds {
                            self.leave_case(cur);
                        }
                        return Ok(false);
                    }
                }
            }
        }
        debug_assert!(!binds || code.scopes[plan.scope as usize].binds(cur), "a tuple case's frame bound off its descriptor");
        Ok(true)
    }

    /// A call on `super` of the chain or of a trait at `e` (`call_super`).
    #[inline(never)]
    fn call_super_at(&mut self, e: TExprId, sym: SymId, args: Vec<Value>, cx: &Ctx) -> R {
        let TExpr::CallMethod(r, _, _) = self.prog().expr(e) else { unreachable!("a super call") };
        let TExpr::Super(target) = self.prog().expr(r) else { unreachable!("a super call") };
        self.call_super(target, sym, args, cx)
    }

    /// The parts of `super[C].s(args)` at `e`.
    fn super_class_call(&self, e: TExprId) -> (ClassId, SymId, ListRef) {
        let TExpr::CallMethod(r, s, args) = self.prog().expr(e) else { unreachable!("a super call") };
        let TExpr::Super(SuperTarget::Class(c)) = self.prog().expr(r) else { unreachable!("a super call") };
        (c, s, args)
    }

    /// `super[C].s(args)` at `e` before its arguments: run whole when it passes proxies.
    #[inline(never)]
    fn super_class_prep(&mut self, e: TExprId, cur: &Rc<Frame>, cx: &Ctx) -> R<Option<Value>> {
        let (c, s, args) = self.super_class_call(e);
        self.call_super_proxied(c, s, args, cur, cx)
    }

    /// `super[C].s(args)` at `e` whose arguments pass no proxies (`call_super_class`).
    #[inline(never)]
    fn call_super_class_with(&mut self, e: TExprId, args: Vec<Value>, cx: &Ctx) -> R {
        let (c, s, _) = self.super_class_call(e);
        let (member, name) = self.super_class_member(c, s);
        self.invoke_member(cx.this.clone(), member, name, args)
    }

    /// The default getter's application at `e` on its evaluated receiver and earlier arguments.
    #[inline(never)]
    fn default_at(&mut self, e: TExprId, mut vals: Vec<Value>, cur: &Rc<Frame>, cx: &Ctx) -> R {
        let TExpr::Default(of, at, args) = self.prog().expr(e) else { unreachable!("a default argument") };
        let recv = (at & DEFAULT_RECEIVER != 0).then(|| vals.remove(0));
        self.default_apply(of, at, args, recv, vals, cur, cx)
    }

    /// A store at a layout's address when the symbol there is the local's; the value back where
    /// it is not, for the walker's assignment.
    #[inline]
    fn store_at(&mut self, cur: &Rc<Frame>, sym: SymId, depth: u16, slot: u16, scope: Option<u32>, v: Value) -> Option<Value> {
        let Some(f) = frame_at(cur, depth) else { return Some(v) };
        let slot = match scope {
            // An address of the unit: a frame it opened with that descriptor.
            Some(id) => {
                debug_assert!(f.desc == id, "a lowered store in a frame its unit did not open");
                slot as usize
            }
            // A hint: checked against the frame's descriptor, or by symbol in a frame without one.
            None => match self.adapted_slot(f, sym, slot as usize) {
                Some(slot) => slot,
                None => return Some(v),
            },
        };
        let mut vals = f.vals.borrow_mut();
        match vals.get_mut(slot) {
            Some((s, place)) if *s == sym => {
                super::written(f.born.get(), f.made.get(), || Some(super::Node::Frame(f.clone())), |other| format!("the local {} of a frame{}{}", self.name(self.syms().sym(sym).name), if other { " another run opened" } else { "" }, super::made_by(f.born.get())));
                *place = v;
                None
            }
            _ => Some(v),
        }
    }

    /// The file and companion of class `c` before its creation's arguments (`TExpr::New`); a class
    /// with captures created whole, its instance given.
    #[inline(never)]
    fn new_prep(&mut self, e: TExprId, c: ClassId, cur: &Rc<Frame>, cx: &Ctx) -> R<Option<Value>> {
        self.init_file_of_class(c)?;
        self.touch_companion(c)?;
        if self.capture_count(c) == 0 {
            return Ok(None);
        }
        let TExpr::New(_, args) = self.prog().expr(e) else { unreachable!("a creation's preparation") };
        let (env, vals) = self.anon_args(c, args, cur, cx)?;
        Ok(Some(self.construct_new(c, vals, &env)?))
    }

    /// A template call's resolution before its arguments (`TExpr::Js`, `template`): `None` for a
    /// builtin, whose arguments come next; any other run whole as the walker runs it.
    #[inline(never)]
    fn js_prep(&mut self, e: TExprId, s: StrRef, quote: bool, cur: &Rc<Frame>, cx: &Ctx) -> R<Option<Value>> {
        if !self.typer.withheld_notes.is_empty() {
            self.note_withheld_body(e);
        }
        let resolved = self.resolve_template(s);
        self.halt_on_withheld()?;
        if let Template::Builtin(_) = resolved {
            if self.trace && self.exec.depth < self.trace_depth {
                self.trace_template(s);
            }
            // A quote renames the locals of its body before its holes are evaluated.
            if quote {
                let TExpr::Js(_, args) = self.prog().expr(e) else { unreachable!("a template's preparation") };
                self.enter_quote(args)?;
            }
            return Ok(None);
        }
        let TExpr::Js(_, args) = self.prog().expr(e) else { unreachable!("a template's preparation") };
        Ok(Some(self.template_resolved(s, resolved, args, cur, cx)?))
    }

    /// A builtin template's call on its arguments, as `template` makes it.
    #[inline]
    fn js_call(&mut self, s: StrRef, vals: Vec<Value>) -> R {
        let Some(Template::Builtin(b)) = self.template_cache[s.idx()].clone() else { unreachable!("a builtin template resolved") };
        let prof = self.prof_template(s);
        let r = b(self, &vals);
        self.prof_exit(prof);
        self.recycle_vec(vals);
        match r {
            Ok(v) => super::process::check_signal(self).map(|()| v),
            e => e,
        }
    }

    /// A `try` of `code` in the protocol both engines share (`try_protocol`): the protected range,
    /// the catch cases' guards and bodies and the finaliser, each a run of its own.
    #[inline(never)]
    fn run_try(&mut self, code: &Code, k: u32, cur: &Rc<Frame>, cx: &Ctx) -> R {
        let plan = &code.tries[k as usize];
        // Each catch case's frame carries its descriptor, as the frames the code opens do.
        let pats: Vec<CatchCase> = plan.cases.iter().map(|c| (c.pat, c.guard.is_some(), code.frame_id(c.scope))).collect();
        self.try_protocol(&pats, plan.wraps, plan.finalizer.is_some(), cur, cx, &mut |it, part, scope| {
            let at = match part {
                TryPart::Body => plan.body,
                TryPart::Guard(i) => plan.cases[i].guard.expect("a guard"),
                TryPart::Case(i) => plan.cases[i].body,
                TryPart::Finalizer => plan.finalizer.expect("a finaliser"),
            };
            it.exec(code, at, scope, cx, false)
        })
    }
}

/// The frame `depth` frames out from `cur`.
#[inline]
fn frame_at(cur: &Rc<Frame>, depth: u16) -> Option<&Rc<Frame>> {
    let mut f = cur;
    for _ in 0..depth {
        f = f.parent.as_ref()?;
    }
    Some(f)
}

/// `prim` of two operands whose kinds make it a primitive's operation, without its call: two
/// `Int`s, or a reference comparison; `None` for the rest, which `prim` takes.
#[inline]
fn quick_prim(op: PrimOp, x: &Value, y: &Value) -> Option<Value> {
    use PrimOp::*;
    match (op, x, y) {
        (RefEq | Same, _, _) => Some(Value::Bool(x.same(y))),
        (RefNe | NotSame, _, _) => Some(Value::Bool(!x.same(y))),
        (_, Value::Int(a), Value::Int(b)) => {
            let (a, b) = (*a, *b);
            Some(match op {
                IntAdd => Value::Int(a.wrapping_add(b)),
                IntSub => Value::Int(a.wrapping_sub(b)),
                IntMul => Value::Int(a.wrapping_mul(b)),
                IntAnd => Value::Int(a & b),
                IntOr => Value::Int(a | b),
                IntXor => Value::Int(a ^ b),
                Lt => Value::Bool(a < b),
                Le => Value::Bool(a <= b),
                Gt => Value::Bool(a > b),
                Ge => Value::Bool(a >= b),
                _ => return None,
            })
        }
        _ => None,
    }
}

// ---- instrumentation ----

/// Why the preflight declines a unit to the walker (`Interp::walked`).
#[derive(Clone, Copy)]
pub(super) enum Walked {
    Refused,
    Lifted,
    Assign,
}

const WALKED: [&str; 3] = ["refused", "lifted member", "assignment"];
const KINDS: [&str; 3] = ["functions", "lambdas", "loops"];

/// What the lowering did in the process, for `TEQ_INTERP_CODE_STATS=1` (`Interp::code_stats`),
/// printed when it exits (`code_stats_report`): the layouts, the units lowered by kind (each a
/// distinct body, lowered once a generation), their nodes and instructions, the bytes of code made
/// and the most alive at once (made less what the caches let go, `Drop for Code`), the runs of lowered code by kind, the calls
/// that found their unit's code, and per reason the units the preflight declined and the calls of
/// theirs the walker ran.
struct Counters {
    layouts: AtomicU64,
    layout_nodes: AtomicU64,
    lowered: [AtomicU64; 3],
    nodes: AtomicU64,
    instructions: AtomicU64,
    made: AtomicU64,
    live: std::sync::atomic::AtomicI64,
    peak: std::sync::atomic::AtomicI64,
    runs: [AtomicU64; 3],
    hits: AtomicU64,
    declined: [AtomicU64; 3],
    declined_calls: [AtomicU64; 3],
}

use std::sync::atomic::AtomicU64;

#[allow(clippy::declare_interior_mutable_const)]
const ZERO: AtomicU64 = AtomicU64::new(0);

static COUNTERS: Counters = Counters {
    layouts: ZERO,
    layout_nodes: ZERO,
    lowered: [ZERO; 3],
    nodes: ZERO,
    instructions: ZERO,
    made: ZERO,
    live: std::sync::atomic::AtomicI64::new(0),
    peak: std::sync::atomic::AtomicI64::new(0),
    runs: [ZERO; 3],
    hits: ZERO,
    declined: [ZERO; 3],
    declined_calls: [ZERO; 3],
};

#[inline]
fn count(c: &AtomicU64, n: u64) {
    c.fetch_add(n, std::sync::atomic::Ordering::Relaxed);
}

pub(super) fn stats_on() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("TEQ_INTERP_CODE_STATS").is_some())
}

/// Prints what the lowering did, under `TEQ_INTERP_CODE_STATS=1`.
pub fn code_stats_report() {
    if !stats_on() {
        return;
    }
    let get = |c: &AtomicU64| c.load(std::sync::atomic::Ordering::Relaxed);
    let c = &COUNTERS;
    let lowered: Vec<String> = KINDS.iter().enumerate().map(|(i, k)| format!("{} {}", get(&c.lowered[i]), k)).collect();
    let runs: Vec<String> = KINDS.iter().enumerate().map(|(i, k)| format!("{} {}", get(&c.runs[i]), k)).collect();
    eprintln!(
        "interp code: {} layouts over {} nodes; lowered {} ({} nodes, {} instructions, {} bytes made, {} alive at most); runs {}; {} calls found their code",
        get(&c.layouts),
        get(&c.layout_nodes),
        lowered.join(", "),
        get(&c.nodes),
        get(&c.instructions),
        get(&c.made),
        c.peak.load(std::sync::atomic::Ordering::Relaxed),
        runs.join(", "),
        get(&c.hits)
    );
    let declined: Vec<String> = WALKED.iter().enumerate().filter(|&(i, _)| get(&c.declined[i]) > 0).map(|(i, w)| format!("{} {} ({} calls)", w, get(&c.declined[i]), get(&c.declined_calls[i]))).collect();
    if !declined.is_empty() {
        eprintln!("interp code: declined to the walker: {}", declined.join(", "));
    }
}
