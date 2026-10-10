//! The expansion by substitution: a call of
//! an inline method of the program's files, whose body the definition check stored, is expanded by copying that body with the parameters bound as the retype path binds them
//! (`bind_inline_call`) and walking the copy once in evaluation order, the copy made part by part
//! as the walk demands it (`Worker::begin_demand`, `demand_roots`): a condition or a selector
//! first, then the node reduced and the selected branch alone copied and walked, a class the body
//! makes copied and settled where it is made, a call kept for the expansion resolved at the site once its receiver and arguments
//! are walked (`expand_inline` again, which takes this walk or the retype path for the callee),
//! the scopes of the copied blocks standing around what the walk resolves. What the copy keeps
//! is what the definition resolved: its overloads, its implicit arguments, its inferred type
//! arguments. A call the entry decision leaves to the retype path (`substitution_entry`) takes it
//! whole, before any effect, for a reason `Fallback` names; `TEQ_INLINE_COUNTS` counts both
//! (`counts`).

use super::apply::MethodCall;
use crate::ast::DefKind as DefKindAst;
use super::inline::{InlineArg, InlineBody, Intrinsic};
use super::inline_definition::{Bindings, Demand, Instance, StoredIndex};
use super::{ResolvedImport, Worker};
use crate::intern::FxMap;
use crate::source::Span;
use crate::symbols::*;
use crate::tir::*;
use crate::types::*;
use std::sync::Arc;

/// Why a call of an inline method takes the retype path.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Fallback {
    /// A library's or the std's method, whose body is not checked at its definition.
    Library,
    /// A method of the program's files that has no record after the demand for one.
    Absent,
    /// A local inline method without a record: one defined in a body being expanded or checked,
    /// which scalac rejects, or one whose check at an earlier call failed.
    Local,
    /// The method's body is being checked: a call met on the way.
    InProgress,
    /// The body failed the definition check.
    Failed,
    /// The body holds a form the check does not type (an object, a trait, an enum or a case
    /// class it defines, a `summonFrom` of another pattern or in a quote).
    Held(HeldForm),
    /// The body holds a form the walk does not cover yet, named.
    Lacks(&'static str),
    /// A local method called before its definition across a val an import between the two
    /// reads from: scalac's E039 (`InlineState::forward_refs`).
    ForwardReference,
}

impl Fallback {
    pub fn name(self) -> String {
        match self {
            Fallback::Library => "a library's or the std's body".into(),
            Fallback::Absent => "a record absent".into(),
            Fallback::Local => "a local form".into(),
            Fallback::InProgress => "a record in progress".into(),
            Fallback::Failed => "a record failed".into(),
            Fallback::Held(form) => format!("held: {}", form.name()),
            Fallback::Lacks(what) => format!("lacks: {}", what),
            Fallback::ForwardReference => "a forward reference".into(),
        }
    }
}

/// The names the pattern `p` binds.
fn pattern_names(ast: &crate::ast::Ast, p: crate::ast::PatId, out: &mut Vec<crate::intern::Name>) {
    use crate::ast::Pat;
    match ast.pat(p) {
        Pat::Bind(n, inner) => {
            out.push(n);
            if let Some(i) = inner {
                pattern_names(ast, i, out);
            }
        }
        Pat::Typed(i, _) | Pat::NamedField(_, i) | Pat::Rest(i) => pattern_names(ast, i, out),
        Pat::Ctor(_, l) | Pat::Tuple(l) | Pat::Alt(l) | Pat::Interp(_, _, l) => {
            for &i in ast.pat_list(l) {
                pattern_names(ast, i, out);
            }
        }
        _ => {}
    }
}

/// What `env_at_definition` gives a local method's check: the environment, the locals standing for
/// the bindings the block defines between the call and the definition, each with its definition's
/// span, and the val whose initialiser holds the call.
struct ForwardScope {
    env: super::Env,
    pending: Vec<(SymId, Span)>,
    first: Option<(crate::intern::Name, Span)>,
}

/// Whether the initialiser `e` types without effects on the program: a literal, a path, an
/// application or a `new` of a class over such, none of which makes a class or binds a name.
fn typed_without_effects(ast: &crate::ast::Ast, e: crate::ast::ExprId) -> bool {
    use crate::ast::Expr;
    let all = |l: crate::ast::ListRef| ast.expr_list(l).iter().all(|&x| typed_without_effects(ast, x));
    match ast.expr(e) {
        Expr::IntLit(_) | Expr::LongLit(_) | Expr::DoubleLit(_) | Expr::DecimalLit(_) | Expr::FloatLit(_) | Expr::BoolLit(_) | Expr::CharLit(_) | Expr::StringLit(_) | Expr::UnitLit | Expr::NullLit | Expr::Ident(_) | Expr::This => true,
        Expr::Select(r, _) | Expr::Parens(r) | Expr::Typed(r, _) | Expr::Prefix(_, r) | Expr::NamedArg(_, r) => typed_without_effects(ast, r),
        Expr::Apply(f, args) | Expr::UsingApply(f, args) => typed_without_effects(ast, f) && all(args),
        Expr::TypeApply(f, _) => typed_without_effects(ast, f),
        Expr::Infix(a, _, b) => typed_without_effects(ast, a) && typed_without_effects(ast, b),
        Expr::New(_, args) | Expr::Tuple(args) => all(args),
        _ => false,
    }
}

/// The state of one walk: the copy, made as the walk demands it, the kinds of its nodes the walk
/// reads, and what it counts. A walk's state is kept for the next expansion once it ends
/// (`InlineState::spare_walks`), its lists keeping their room, its maps made anew.
pub(super) struct Walk {
    def: Arc<InlineDefinition>,
    inst: Instance,
    demand: Demand,
    /// How far the walk's tables have taken in the instance's metadata.
    absorbed: Absorbed,
    /// What the walk reads of each node of the copy: an `inline if` or `inline match` (its
    /// source's index the instance's `reducible_at`), a kept call, a cast (`CopyKinds`).
    kinds: CopyKinds,
    /// The imports of the copy's blocks, each with the statement it stands before.
    imports: FxMap<TExprId, Vec<(u32, ResolvedImport)>>,
    /// The names the copy's blocks bind to a value's member, each with the statement it stands
    /// before, as an index into the instance's.
    aliases: FxMap<TExprId, Vec<(u32, usize)>>,
    /// The proxy of each parameter, which an import from the parameter names in its place: for
    /// a body with imports, which alone read it.
    param_proxies: Vec<(SymId, SymId)>,
    /// The copy's fresh variables of the type patterns.
    pattern_vars: Vec<TParamId>,
    /// Where the diagnostics of this expansion begin: an error since stops the resolution of
    /// the calls after it (`Walk::failed`).
    diag_mark: usize,
    failed: bool,
    visited: u64,
    discarded: u64,
    /// The stored nodes of the parts the walk never demanded: branches not taken, cases not
    /// selected, defaults an argument replaced.
    pruned: u64,
    /// The proxies of the by-name parameters, which stand for their argument's code: the
    /// definition's call of the parameter is that code, a reference that is not called a thunk
    /// of it.
    by_name: FxMap<SymId, ()>,
    /// The copy's temporaries the definition hoisted operands into.
    hoisted: FxMap<SymId, ()>,
    /// The copy's `inline val`s whose value the walk gives (`InlineDefinition::inline_vals`).
    inline_vals: FxMap<SymId, ()>,
    /// The copy's vals whose type the walked initialiser gives (`InlineDefinition::inferred_vals`).
    inferred_vals: FxMap<SymId, ()>,
    /// The copies of the classes the stored body makes, each with its stored class.
    class_copies: FxMap<ClassId, ClassId>,
    /// The copies of named local classes, settled or being settled.
    local_copies: FxMap<ClassId, LocalCopy>,
    /// The binders of reduced cases, typed at what their patterns narrow the scrutinee to, whose
    /// references take that type.
    binder_types: FxMap<SymId, TypeId>,
    /// While a class's body is settled, each node the walk put in the place of another (a kept
    /// call's expansion, a substituted argument), with the node it replaced: what orders the
    /// class's captures by the stored nodes (`settle_class_body`).
    settling: u32,
    replaced: Vec<(TExprId, TExprId)>,
}

/// Where the walk stands with the copy of a named local class: its body being settled, with the
/// creations met meanwhile (each with whether it stands in the class and the enclosing objects'
/// `this`), or settled, with what it captures.
enum LocalCopy {
    Settling(Vec<(TExprId, bool, Vec<(SymId, TExprId)>)>),
    Settled(Vec<SymId>),
}

/// A part of the body of a class a copy makes, in the order written.
#[derive(Clone, Copy)]
enum ClassPart {
    Fun(FunId),
    Init(usize, TExprId),
}

/// How far a walk's tables have taken in its instance's metadata lists.
#[derive(Default)]
struct Absorbed {
    reducible: usize,
    deferred: usize,
    imports: usize,
    aliases: usize,
    classes: usize,
}

/// The kinds of the copy's nodes the walk reads, dense from the copy's first node (the nodes the
/// copy makes are the program's from there on), the few below it kept aside.
#[derive(Default)]
struct CopyKinds {
    base: u32,
    dense: Vec<u8>,
    below: Vec<(TExprId, u8)>,
}

impl CopyKinds {
    const REDUCIBLE: u8 = 1;
    const DEFERRED: u8 = 2;

    /// Empty, for a copy whose nodes begin at `base`, of a record of `nodes` nodes, its room kept.
    fn reset(&mut self, base: u32, nodes: usize) {
        self.base = base;
        self.dense.clear();
        self.dense.reserve(2 * nodes);
        self.below.clear();
    }

    fn mark(&mut self, e: TExprId, kind: u8) {
        match e.0.checked_sub(self.base) {
            Some(i) => {
                let i = i as usize;
                if i >= self.dense.len() {
                    self.dense.resize(i + 1, 0);
                }
                self.dense[i] |= kind;
            }
            None => match self.below.iter_mut().find(|(n, _)| *n == e) {
                Some((_, k)) => *k |= kind,
                None => self.below.push((e, kind)),
            },
        }
    }

    #[inline]
    fn is(&self, e: TExprId, kind: u8) -> bool {
        let k = match e.0.checked_sub(self.base) {
            Some(i) => self.dense.get(i as usize).copied().unwrap_or(0),
            None => self.below.iter().find(|(n, _)| *n == e).map_or(0, |&(_, k)| k),
        };
        k & kind != 0
    }
}

impl Walk {
    /// The index of the source of the copy's reducible node `e`, the last the instance
    /// published for it.
    fn reducible_at(&self, e: TExprId) -> Option<usize> {
        if !self.kinds.is(e, CopyKinds::REDUCIBLE) {
            return None;
        }
        self.inst.reducible.iter().rposition(|&r| r == e).map(|i| self.inst.reducible_at[i])
    }
}

impl<'a> Worker<'a> {
    /// The record a call of `sym` is expanded from, or why the call takes the retype path, the
    /// entry decision, made before the expansion has any effect: a library's, the std's or a
    /// body file's method; otherwise the record demanded whatever the typing order, and then a call met
    /// while its body is typed (a cycle through the method), a body the capture of the product
    /// modes holds back (one that makes a class) or a `summonFrom` the check holds back, which
    /// the retype path keeps; a local method called before its definition across a val an
    /// import reads is scalac's E039. A failed record's call is the plain call before this
    /// (`failed_definition_call`).
    pub(super) fn substitution_entry(&mut self, sym: SymId) -> Result<Arc<InlineDefinition>, Fallback> {
        let info = self.syms.sym(sym);
        let (file, local) = (info.file, info.owner == Owner::Local);
        if info.def.is_none() || !self.program_file(file) || self.is_body_file(file) {
            return Err(Fallback::Library);
        }
        self.demand_definition(sym);
        if self.inline.forward_refs.contains_key(&sym) {
            return Err(Fallback::ForwardReference);
        }
        let Some(def) = self.inline_definitions.get(&sym).cloned() else {
            // A method of the program's has its record once demanded; a local one has none where
            // its forward call's imports could not be entered ahead (one that does not resolve).
            debug_assert!(local, "no record of {} after its demand", self.name_str(self.syms.sym(sym).name));
            return Err(if local { Fallback::Local } else { Fallback::Absent });
        };
        match def.state {
            DefinitionState::InProgress => return Err(Fallback::InProgress),
            DefinitionState::Failed => return Err(Fallback::Failed),
            DefinitionState::Held(form) => return Err(Fallback::Held(form)),
            DefinitionState::Checked => {}
        }
        debug_assert!(def.body.is_some(), "a checked record without its body");
        if def.body.is_none() {
            return Err(Fallback::Absent);
        }
        match def.walk_lack {
            None => Ok(def),
            Some(WalkLack::SummonFrom) => Err(Fallback::Held(HeldForm::SummonFrom)),
            Some(WalkLack::Lacks(what)) => Err(Fallback::Lacks(what)),
        }
    }

    /// The record of a call's callee, checked on the call if the walk has not met its
    /// definition yet, so that what a call gives does not depend on the typing order.
    pub(super) fn demand_definition(&mut self, sym: SymId) {
        if self.checks_inline_definition() || !self.is_inline_callee(sym) || self.inline_definitions.contains_key(&sym) {
            return;
        }
        // The check is the definition's whatever becomes of an attempt that asked for it.
        let promoted = self.promote_begin(sym);
        if self.syms.sym(sym).owner == Owner::Local {
            self.demand_local_definition(sym);
        } else {
            self.check_inline_definition(sym);
        }
        self.promote_end(promoted);
    }

    /// A local inline method called before its block reaches its definition, checked on the call
    /// in its definition's scope, as the block checks it on reaching it
    /// (`check_local_inline_definition`): the call's environment down to the frame that names the
    /// method, with the imports in scope there and those the block enters between the call and
    /// the definition (`env_at_definition`), not the scopes around the call (a lambda's
    /// parameter of the call's name is not the definition's). Not in a body being expanded or
    /// checked, whose local methods the check does not take (and which scalac rejects, "nested
    /// inline methods are not supported"). The record it makes is the definition's, which the
    /// block's own check finds made, its diagnostics reported once.
    fn demand_local_definition(&mut self, sym: SymId) {
        if self.inline.depth > 0 || self.inline.checking > 0 || self.quote.level > 0 {
            return;
        }
        let name = self.syms.sym(sym).name;
        let frame = self.env.frames.iter().rposition(|f| matches!(f, super::Frame::Locals { names, .. } if names.iter().any(|&(n, s)| n == name && s == sym)));
        let Some(k) = frame else { return };
        let mut env = self.env.clone();
        env.frames.truncate(k + 1);
        env.imports.retain(|i| i.depth as usize <= k + 1);
        let Some(scope) = self.env_at_definition(sym, k, env) else { return };
        let marks = (self.diags.items.len(), self.deferred_matches.len(), self.deferred_bounds.len());
        self.check_definition_in(sym, scope.env);
        let Some(read) = self.pending_read(sym, &scope.pending) else { return };
        // The checked body reads a binding the block defines between the call and the
        // definition: scalac's E039 at the call. The record and what its check reported at the
        // definition go, and the block's own check of the definition, where it stands, makes
        // them again; what the check reported of other bodies on the way stays.
        self.inline_definitions.retain(|&s, _| s != sym);
        let (file, span) = self.definition_span(sym);
        let within = |f: crate::source::FileId, s: Span| f == file && s.start >= span.start && s.end <= span.end;
        let reported = self.diags.items.split_off(marks.0);
        self.diags.items.extend(reported.into_iter().filter(|d| !within(d.file, d.span)));
        let later = self.deferred_matches.split_off(marks.1);
        self.deferred_matches.extend(later.into_iter().filter(|m| !within(m.file, m.span)));
        let later = self.deferred_bounds.split_off(marks.2);
        self.deferred_bounds.extend(later.into_iter().filter(|b| {
            let (f, start, end) = b.site();
            !within(f, Span { start, end })
        }));
        let text = &self.source(file).text;
        let line = |s: Span| crate::source::locate(text, s.start as usize).0;
        let (read_name, read_span) = (self.syms.sym(read.0).name, read.1);
        let shown = self.name_str(read_name);
        let msg = match scope.first {
            Some((f, fspan)) => format!("forward reference to {} (defined on line {}) extends over the definition of {} (on line {})", shown, line(read_span), self.name_str(f), line(fspan)),
            None => format!("forward reference to {} extends over the definition of {} (on line {})", shown, shown, line(read_span)),
        };
        self.inline.forward_refs.insert(sym, msg);
    }

    /// What the definition of `sym` spans, its header to the end of its body.
    fn definition_span(&self, sym: SymId) -> (crate::source::FileId, Span) {
        let info = self.syms.sym(sym);
        let (file, span) = (info.file, info.span);
        let Some(d) = info.def else { return (file, span) };
        let ast = self.ast(file);
        let def = ast.def(d);
        let body = match &def.kind {
            DefKindAst::Fun(f) => f.body.map(|b| ast.expr_span(b)),
            _ => None,
        };
        (file, body.map_or(def.span, |b| Span { start: def.span.start.min(b.start), end: def.span.end.max(b.end) }))
    }

    /// The first of the bindings `pending` stands for that the checked body of `sym` reads: a
    /// node naming its local, or a type naming its path, in the stored trees (its selections
    /// through an import included, which name the local as the prefix).
    fn pending_read(&mut self, sym: SymId, pending: &[(SymId, Span)]) -> Option<(SymId, Span)> {
        if pending.is_empty() {
            return None;
        }
        let def = self.inline_definitions.get(&sym).cloned()?;
        let roots: Vec<TExprId> = self.stored_roots_of(&def);
        let mut locals: Vec<SymId> = Vec::new();
        for &r in &roots {
            for e in self.prog.descendants(r) {
                if let TExpr::Local(s) = self.prog.expr(e) {
                    locals.push(s);
                }
            }
        }
        pending.iter().copied().find(|&(p, _)| {
            locals.contains(&p) || def.node_types.values().any(|&t| self.mentions_term(t, p))
        })
    }

    /// `env`, the environment of the block whose frame `k` defines the local inline method `sym`
    /// as the block stands at the statement being typed, with the imports of the statements
    /// between that one and the definition entered as the block will enter them, so that the
    /// body is checked where it is written (`import B.k` between `val first = g()` and `inline
    /// def g() = k` is `g`'s `k`), an import from a value the call's scope binds entered as the
    /// block enters it (`import source.k` over a `source` before the call). Where such an import
    /// selects from a binding the block defines between the call and the definition (a val, a
    /// lazy val, a given), that binding stands for itself, a local of its declared type or of
    /// its initialiser's (`pending`), so that the body resolves through the import as the block
    /// will, the scopes' precedence and ambiguity included; a body that then reads it is
    /// scalac's E039 (`demand_local_definition`). None where an import cannot be entered ahead of
    /// its block (one that does not resolve), whose call then waits for the block's own check.
    fn env_at_definition(&mut self, sym: SymId, k: usize, env: super::Env) -> Option<ForwardScope> {
        let (file, def) = {
            let s = self.syms.sym(sym);
            (s.file, s.def)
        };
        let unchanged = |env| Some(ForwardScope { env, pending: Vec::new(), first: None });
        let Some(def) = def else { return unchanged(env) };
        let Some(cursor) = self.inline.blocks.iter().rev().find(|b| b.frame == k && b.file == file).copied() else { return unchanged(env) };
        let ast = self.ast(file);
        let items = ast.stmt_list(cursor.stmts);
        let Some(at) = items.iter().position(|s| matches!(s, crate::ast::Stmt::Def(d) if *d == def)) else { return unchanged(env) };
        let between = items.get(cursor.at + 1..at).unwrap_or(&[]);
        let imports: Vec<u32> = between.iter().filter_map(|s| match *s {
            crate::ast::Stmt::Import(i) => Some(i),
            _ => None,
        }).collect();
        if imports.is_empty() {
            return unchanged(env);
        }
        // The bindings the block defines between the two that it has not entered at the call (its
        // vals, lazy vals and givens; the local methods, classes and objects are named from its
        // start), each with its declared type or the type of an initialiser that types without
        // effects on the program, any other's unknown.
        let mut pending: Vec<(crate::intern::Name, Span, Option<crate::ast::TyExprId>, Option<crate::ast::ExprId>)> = Vec::new();
        for st in between {
            if let crate::ast::Stmt::Def(d) = *st {
                let d = ast.def(d);
                match &d.kind {
                    DefKindAst::Val { pat: Some(p), .. } => {
                        let mut names = Vec::new();
                        pattern_names(ast, *p, &mut names);
                        pending.extend(names.into_iter().map(|n| (n, d.span, None, None)));
                    }
                    DefKindAst::Val { ty, rhs, .. } => pending.push((d.name, d.span, *ty, *rhs)),
                    DefKindAst::Given(g) => pending.push((d.name, d.span, Some(g.ty), None)),
                    _ => {}
                }
            }
        }
        let first = match items.get(cursor.at) {
            Some(crate::ast::Stmt::Def(d)) if matches!(ast.def(*d).kind, DefKindAst::Val { pat: None, .. }) => Some((ast.def(*d).name, ast.def(*d).span)),
            _ => None,
        };
        let index = self.index.take();
        let entered = self.with_env(env, |t| {
            let mut stand_ins = Vec::with_capacity(pending.len());
            for &(name, span, declared, rhs) in &pending {
                let mark = t.diags.items.len();
                let ty = match (declared, rhs) {
                    (Some(ty), _) => t.resolve_type(ty),
                    (None, Some(e)) if typed_without_effects(ast, e) => {
                        let (_, ty) = t.type_expr(e, None);
                        t.solve_inferred(ty)
                    }
                    _ => ERROR,
                };
                t.drop_reported_since(mark);
                let local = t.new_local(name, SymKind::Val, ty, span);
                t.bind_local(name, local);
                stand_ins.push((local, span));
            }
            let mark = t.diags.items.len();
            for i in imports {
                let clauses = ast.import_stmt(i);
                for imp in clauses {
                    // An import from a value of the block binds a local that stands for the
                    // selection (`import v.{m as a}`), as the block's own entering binds it.
                    let Some(&head) = imp.path.first() else { return None };
                    let from_value = t.lookup_term(head).and_then(|r| r.sym()).map_or(false, |s| {
                        let info = t.syms.sym(s);
                        matches!(info.kind, SymKind::Val | SymKind::Param | SymKind::Given) && info.owner == Owner::Local && t.local_module_of_sym(s).is_none()
                    });
                    if from_value && t.enter_value_import(imp).is_some() {
                        continue;
                    }
                    match t.resolve_import(imp, clauses) {
                        Some(r) if t.diags.items.len() == mark => t.env.push_import(cursor.imports_scope, r),
                        _ => {
                            t.drop_reported_since(mark);
                            return None;
                        }
                    }
                }
            }
            Some(ForwardScope { env: t.env.clone(), pending: stand_ins, first })
        });
        self.index = index;
        entered
    }

    /// The first form of the checked body `def` the walk does not take, found once when its
    /// record is made: a `summonFrom` kept as a call (in a quote of the body), and a kept call
    /// without its record, which no check makes. A `super` of the method's own class and a
    /// nested inline method fail the check (scalac's E082 and its restriction).
    pub(super) fn walk_lack(&mut self, def: &InlineDefinition) -> Option<WalkLack> {
        let roots = self.stored_roots_of(def);
        let nodes: Vec<TExprId> = roots.iter().flat_map(|&r| self.prog.descendants(r)).collect();
        let deferred: FxMap<TExprId, ()> = def.deferred.iter().map(|&e| (e, ())).collect();
        for &e in &nodes {
            if deferred.contains_key(&e) {
                let Some(d) = self.quote.deferred.get(&e).cloned() else { return Some(WalkLack::Lacks("a deferred call without its record")) };
                if let Some(Intrinsic::SummonFrom) = self.intrinsic_of(d.sym) {
                    return Some(WalkLack::SummonFrom);
                }
            }
        }
        None
    }

    /// The stored body's trees and its defaults', with the defaults of its local functions.
    fn stored_roots_of(&self, def: &InlineDefinition) -> Vec<TExprId> {
        let mut out: Vec<TExprId> = def.body.into_iter().chain(def.defaults.iter().flatten().copied()).chain(def.aliases.iter().map(|a| a.tree)).collect();
        let mut next = 0;
        while next < out.len() {
            let root = out[next];
            next += 1;
            let mut found = Vec::new();
            for e in self.prog.descendants(root) {
                if let TExpr::Block(stmts, _) = self.prog.expr(e) {
                    for s in self.prog.stmt_list(stmts) {
                        if let TStmt::Fun(f) = *s {
                            found.extend(self.prog.funs[f.idx()].defaults.iter().flatten().copied());
                        }
                    }
                }
            }
            out.extend(found);
        }
        out
    }

    /// The expansion of a call of the inline method `call.sym` from its record `def`: the
    /// arguments bound as the retype path binds them, the body copied with every parameter
    /// standing for its proxy and the receiver for `this`, the defaults of the missing
    /// arguments walked and bound in their places, the copy walked, and the result typed: a
    /// transparent method's the copy's type (a constant's literal type), an inferred one's the
    /// copy's widened, a declared one's the declared instance with the parameters' paths.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn expand_by_substitution(
        &mut self,
        call: &MethodCall,
        sig: &MethodSig,
        subst: &Subst,
        args: &[TExprId],
        body: &InlineBody,
        ret_ty: TypeId,
        span: Span,
        def: &Arc<InlineDefinition>,
    ) -> (TExprId, TypeId) {
        // The classes the walk copies are settled in place.
        self.settling_classes(|t| t.expand_by_substitution_walk(call, sig, subst, args, body, ret_ty, span, def))
    }

    #[allow(clippy::too_many_arguments)]
    fn expand_by_substitution_walk(
        &mut self,
        call: &MethodCall,
        sig: &MethodSig,
        subst: &Subst,
        args: &[TExprId],
        body: &InlineBody,
        ret_ty: TypeId,
        span: Span,
        def: &Arc<InlineDefinition>,
    ) -> (TExprId, TypeId) {
        let mut bound = self.bind_inline_call(call, sig, subst, args, body, ret_ty, span, true);
        let mut targs: Subst = Vec::with_capacity(subst.len() + call.owner_subst.len());
        for &(p, t) in subst {
            let t = self.zonk(t);
            targs.push((p, self.solve_if_var(t)));
        }
        for &(p, t) in &call.owner_subst {
            targs.push((p, self.zonk(t)));
        }
        let index = self.stored_index_of(call.sym, def);
        let mut walk = self.take_walk(def, &index);
        let (mut bargs, mut paths, mut this_paths) = walk.demand.take_lists();
        let bound_from = self.prog.exprs.len() as u32;
        paths.reserve(bound.params.len());
        for i in 0..bound.params.len() {
            let p = bound.params[i];
            bargs.insert(p.sym, self.prog.add(TExpr::Local(p.proxy)));
            let constant = if p.later { None } else { p.constant };
            let path = if p.later { self.types.mk(Type::Term(p.proxy)) } else { self.parameter_path(p.arg, constant, p.proxy) };
            paths.push((p.sym, path));
        }
        let this = bound.this_proxy.map(|p| self.prog.add(TExpr::Local(p)));
        let bound_nodes = (bound_from, self.prog.exprs.len() as u32);
        this_paths.extend_from_slice(&self.inline.this_paths[bound.this_paths_mark..]);
        let (call_subst, call_paths) = (targs.len(), paths.len());
        let bindings = Bindings { args: bargs, subst: targs, paths, this, this_paths, mark_leaves: false };
        let deferred_mark = self.quote.copied_deferred.len();
        let body_root = def.body.expect("a checked record has a body");
        // Under the capture the copy is made whole, the capture's check reading every node of it.
        let on_demand = !self.capturing();
        {
            let w = &mut *walk;
            self.begin_demand_in(def, index, bindings, on_demand, &mut w.demand, &mut w.inst);
            w.demand.names_classes = true;
        }
        // A class the body makes reads the receiver, where it names the method's class's
        // `this`, through the local that stands for it there: the receiver's proxy, as the
        // retype path's `this_ref` gives it.
        if let (Some(this), Owner::Class(k)) = (this, self.syms.sym(call.sym).owner) {
            if let Some(&outer) = self.outer_this.get(&k) {
                walk.demand.stand_for(outer, this);
            }
        }
        walk.demand.bound = bound_nodes;
        self.reset_walk(&mut walk, def);
        if on_demand {
            walk.inst.root = self.demand_in_walk(&mut walk, body_root, None);
        } else {
            let stored: Vec<TExprId> = std::iter::once(body_root).chain(def.defaults.iter().flatten().copied()).collect();
            let copies = self.demand_roots(&mut walk.demand, &walk.def, &mut walk.inst, &stored, None);
            self.absorb_demanded(&mut walk);
            walk.inst.root = copies[0];
            let mut copied_defaults = copies[1..].iter().copied();
            walk.inst.defaults = def.defaults.iter().map(|d| d.and_then(|_| copied_defaults.next())).collect();
        }
        walk.by_name = bound.params.iter().filter(|p| p.by_name).map(|p| (p.proxy, ())).collect();
        if !def.imports.is_empty() {
            walk.param_proxies = bound.params.iter().map(|p| (p.sym, p.proxy)).collect();
        }
        for at in 0..bound.params.len() {
            let stored_default = def.defaults.get(at).copied().flatten();
            if !bound.params[at].later {
                match (on_demand, stored_default) {
                    (true, Some(d)) => walk.pruned += walk.demand.pruned_size(&self.prog, d),
                    (false, Some(_)) => {
                        let d = walk.inst.defaults[at].expect("the copy of a default");
                        self.discard(&mut walk, d);
                    }
                    _ => {}
                }
                continue;
            }
            let d = if on_demand {
                let d = stored_default.expect("the default of a missing argument");
                self.demand_in_walk(&mut walk, d, None)
            } else {
                walk.inst.defaults[at].expect("the default of a missing argument")
            };
            let te = self.walk(&mut walk, d);
            self.bind_parameter(&mut bound, at, te);
        }
        let root = walk.inst.root;
        let te = self.walk(&mut walk, root);
        self.settle_remaining_copies(&mut walk);
        let ty = if walk.failed {
            ERROR
        } else if body.transparent {
            let result = self.result_expr(body.body);
            let ascribed = matches!(self.ast(body.file).expr(result), crate::ast::Expr::Typed(..)) || self.widened_constant(te);
            match self.fold_constant(te).filter(|_| !ascribed) {
                Some(v) => self.types.lit(v),
                None => self.walked_type(&walk, te),
            }
        } else if !body.declared {
            let t = self.walked_type(&walk, te);
            self.solve_inferred(t)
        } else {
            let (subst, paths, this_paths) = walk.demand.bindings();
            self.declared_result(sig, &subst[..call_subst], &paths[..call_paths], this_paths, ret_ty)
        };
        if !body.transparent {
            self.mark_widened_expansion(te, ty);
        }
        self.quote.copied_deferred.truncate(deferred_mark);
        counts::walked(walk.demand.copied, walk.visited, walk.discarded);
        counts::pruned(walk.pruned);
        let te = if walk.failed { self.prog.add(TExpr::Unit) } else { te };
        self.put_walk(walk);
        self.unbind_inline_call(&mut bound, te, ty)
    }

    /// Under `TEQ_INLINE_MEASURE=copy`, the copy a call the walk takes would make, made and
    /// dropped: the parameters bound to fresh locals, the type arguments the call's, the copy's
    /// tests and patterns left out of the output's and its deferred calls unregistered, the nodes
    /// counted as copied.
    pub(super) fn copy_for_measure(&mut self, def: &InlineDefinition, subst: &Subst, owner_subst: &Subst) {
        let mut args: FxMap<SymId, TExprId> = FxMap::default();
        for &p in &def.params {
            let fresh = self.clone_local(p);
            args.insert(p, self.prog.add(TExpr::Local(fresh)));
        }
        let paths = def.params.iter().filter_map(|&p| args.get(&p).and_then(|&a| self.argument_path(a)).map(|path| (p, path))).collect();
        let targs: Subst = subst.iter().chain(owner_subst).map(|&(p, t)| (p, self.zonk(t))).collect();
        let bindings = Bindings { args, subst: targs, paths, this: None, this_paths: Vec::new(), mark_leaves: false };
        let deferred_mark = self.quote.copied_deferred.len();
        let copied_mark = self.prog.exprs.len();
        if let Some(inst) = self.instantiate_definition(def, &bindings) {
            for root in std::iter::once(inst.root).chain(inst.defaults.iter().flatten().copied()) {
                self.store_tests_and_pats(root);
            }
        }
        self.quote.copied_deferred.truncate(deferred_mark);
        counts::walked((self.prog.exprs.len() - copied_mark) as u64, 0, 0);
    }

    /// A declared result type at the call: the call's, unless it names a parameter's path, which
    /// takes the argument's as the copy's types do (`value(3)` of `value(x: Int): x.type` is a
    /// `3`), where the call's has it widened.
    fn declared_result(&mut self, sig: &MethodSig, subst: &[(TParamId, TypeId)], paths: &[(SymId, TypeId)], this_paths: &[(ClassId, TypeId)], ret_ty: TypeId) -> TypeId {
        let names_param = self.types.has_paths(sig.ret) && paths.iter().any(|&(p, _)| self.mentions_term(sig.ret, p));
        if !names_param {
            return ret_ty;
        }
        let mut t = self.types.subst(sig.ret, &subst.to_vec());
        t = self.subst_paths(t, paths);
        for &(c, path) in this_paths.iter().rev() {
            t = self.as_seen_from(t, path, c);
        }
        t
    }

    /// A walk's state for an expansion of `def`: one an earlier expansion ended with, its lists
    /// keeping their room, or a new one; `begin_demand_in` and `reset_walk` set it for the call.
    fn take_walk(&mut self, def: &Arc<InlineDefinition>, index: &Arc<StoredIndex>) -> Box<Walk> {
        if let Some(w) = self.inline.spare_walks.pop() {
            return w;
        }
        Box::new(Walk {
            def: def.clone(),
            inst: Instance::empty(),
            demand: Demand::empty(index.clone(), self.inline.no_subst.clone()),
            absorbed: Absorbed::default(),
            kinds: CopyKinds::default(),
            imports: FxMap::default(),
            aliases: FxMap::default(),
            param_proxies: Vec::new(),
            pattern_vars: Vec::new(),
            diag_mark: 0,
            failed: false,
            visited: 0,
            discarded: 0,
            pruned: 0,
            by_name: FxMap::default(),
            hoisted: FxMap::default(),
            inline_vals: FxMap::default(),
            inferred_vals: FxMap::default(),
            class_copies: FxMap::default(),
            local_copies: FxMap::default(),
            binder_types: FxMap::default(),
            settling: 0,
            replaced: Vec::new(),
        })
    }

    /// The walk's own state set for an expansion of `def`, whose demand and instance are set.
    fn reset_walk(&mut self, w: &mut Walk, def: &Arc<InlineDefinition>) {
        w.def = def.clone();
        w.absorbed = Absorbed::default();
        let room = if def.reducible.is_empty() && def.deferred.is_empty() { 0 } else { def.node_types.len() };
        w.kinds.reset(self.prog.exprs.len() as u32, room);
        w.imports = FxMap::default();
        w.aliases = FxMap::default();
        w.param_proxies.clear();
        w.pattern_vars.clear();
        let inst = &w.inst;
        w.pattern_vars.extend(def.pattern_tparams.iter().filter_map(|p| inst.tparams.iter().find(|(q, _)| q == p).map(|&(_, f)| f)));
        w.diag_mark = self.diags.items.len();
        w.failed = false;
        w.visited = 0;
        w.discarded = 0;
        w.pruned = 0;
        w.by_name = FxMap::default();
        w.hoisted = FxMap::default();
        w.inline_vals = FxMap::default();
        w.inferred_vals = FxMap::default();
        w.class_copies = FxMap::default();
        w.local_copies = FxMap::default();
        w.binder_types = FxMap::default();
        w.settling = 0;
        w.replaced.clear();
    }

    /// A walk's state kept for the next expansion, up to the depth the expansions nest to.
    fn put_walk(&mut self, w: Box<Walk>) {
        if self.inline.spare_walks.len() < 64 {
            self.inline.spare_walks.push(w);
        }
    }

    /// The copies of the stored trees `roots` for the walk (`demand_roots`), with `specialise`
    /// applied, and the record's metadata on them taken into the walk's maps before anything in
    /// them is walked.
    fn demand_in_walk(&mut self, w: &mut Walk, root: TExprId, specialise: Option<&Subst>) -> TExprId {
        let copy = self.demand_root(&mut w.demand, &w.def, &mut w.inst, root, specialise);
        self.absorb_demanded(w);
        copy
    }

    /// The record's metadata a demand published in the instance, taken into the walk's tables.
    fn absorb_demanded(&mut self, w: &mut Walk) {
        let a = &mut w.absorbed;
        let inst = &w.inst;
        for &e in &inst.reducible[a.reducible..] {
            w.kinds.mark(e, CopyKinds::REDUCIBLE);
        }
        a.reducible = inst.reducible.len();
        for &e in &inst.deferred[a.deferred..] {
            w.kinds.mark(e, CopyKinds::DEFERRED);
        }
        a.deferred = inst.deferred.len();
        let mut blocks: Vec<TExprId> = Vec::new();
        for i in &inst.imports[a.imports..] {
            w.imports.entry(i.block).or_default().push((i.at, i.import));
            blocks.push(i.block);
        }
        a.imports = inst.imports.len();
        for b in blocks {
            if let Some(list) = w.imports.get_mut(&b) {
                list.sort_by_key(|&(at, _)| at);
            }
        }
        let mut blocks: Vec<TExprId> = Vec::new();
        for i in a.aliases..inst.aliases.len() {
            let alias = inst.aliases[i];
            w.aliases.entry(alias.block).or_default().push((alias.at, i));
            blocks.push(alias.block);
        }
        a.aliases = inst.aliases.len();
        for b in blocks {
            if let Some(list) = w.aliases.get_mut(&b) {
                list.sort_by_key(|&(at, _)| at);
            }
        }
        for &(stored, copy) in &inst.classes[a.classes..] {
            w.class_copies.insert(copy, stored);
        }
        a.classes = inst.classes.len();
        for h in &w.def.hoisted {
            if let Some(&f) = inst.renames.get(h) {
                w.hoisted.insert(f, ());
            }
        }
        for v in &w.def.inline_vals {
            if let Some(&f) = inst.renames.get(v) {
                w.inline_vals.insert(f, ());
            }
        }
        for v in &w.def.inferred_vals {
            if let Some(&f) = inst.renames.get(v) {
                w.inferred_vals.insert(f, ());
            }
        }
    }

    /// The type of a node of the walk: the copy's, or the one the walk gave a node it made.
    fn walked_type(&self, w: &Walk, e: TExprId) -> TypeId {
        w.inst.types.get(e).or_else(|| self.prog.type_of(e)).unwrap_or(ERROR)
    }

    fn set_walked_type(&mut self, w: &mut Walk, e: TExprId, t: TypeId) {
        w.inst.types.insert(e, t);
        self.prog.set_type(e, t);
    }

    /// The node that stands for `e` once walked: `e` with its children walked in evaluation order
    /// and replaced where they changed, a reduced conditional's selected branch, a kept call's
    /// expansion, a substituted parameter's argument.
    fn walk(&mut self, w: &mut Walk, e: TExprId) -> TExprId {
        // A part the copy left, reached: copied now, the walk never writing into the stored body.
        let e = if w.demand.is_left(e) { self.demand_in_walk(w, e, None) } else { e };
        let walked = self.walk_node(w, e);
        // Whatever the walk puts in a node's place (an argument for a parameter's read, a
        // by-name read's argument, the branch an `inline if` or `inline match` reduces to, a
        // kept call's expansion) stands for it where an ascription widened it, or it is a plain
        // inline call's, each mark by itself (`Program::opaque_bits`).
        if walked != e && (self.prog.is_widened(e) || self.prog.is_opaque(e) || self.prog.is_spread(e)) {
            if self.prog.is_widened(e) {
                self.prog.mark_widened(walked);
            }
            if self.prog.is_opaque(e) {
                self.prog.mark_opaque(walked);
            }
            if self.prog.is_spread(e) {
                self.prog.mark_spread(walked);
            }
        }
        if w.settling > 0 && walked != e {
            w.replaced.push((e, walked));
        }
        walked
    }

    /// A cast of the body, decided now that the copy fills its types in, as dotty's erasure
    /// decides the casts of inlined code (`TypeTestsCasts.interceptTypeApply` after `Inlining`):
    /// from the receiver's type as the copy has it, a local's its declared one (a parameter's
    /// proxy has its argument's own type, as dotty's `paramBindingDef` gives it).
    fn walk_cast(&mut self, w: &mut Walk, e: TExprId, recv: TExprId, op: CastOp, to: TypeId) -> TExprId {
        let walked = self.walk(w, recv);
        if op != CastOp::Written {
            if walked != recv {
                self.prog.exprs[e.idx()] = TExpr::Cast(walked, op, to);
            }
            return e;
        }
        let from = match self.prog.expr(walked) {
            TExpr::Local(s) => self.sig_of(s).ret,
            _ => self.walked_type(w, walked),
        };
        let lowering = self.cast_lowering(from, to, false);
        match self.untested_lowering(walked, lowering) {
            super::prims::CastLowering::Op(op) if !matches!(op, CastOp::Unbox(..)) || !matches!(self.prog.expr(walked), TExpr::Null) => {
                self.prog.exprs[e.idx()] = TExpr::Cast(walked, op, to);
                e
            }
            lowering => {
                let (cast, ty) = self.lower_cast_as(walked, from, to, lowering);
                self.set_walked_type(w, cast, ty);
                cast
            }
        }
    }

    fn walk_node(&mut self, w: &mut Walk, e: TExprId) -> TExprId {
        w.visited += 1;
        match self.prog.expr(e) {
            TExpr::Int(_)
            | TExpr::Long(_)
            | TExpr::Double(_)
            | TExpr::Bool(_)
            | TExpr::Char(_)
            | TExpr::Str(_)
            | TExpr::Unit
            | TExpr::Null
            | TExpr::This
            | TExpr::Super(_)
            | TExpr::Module(_)
            | TExpr::ClassOf(_)
            | TExpr::JsImport(_)
            | TExpr::JsGlobal(..) => self.resolved(w, e),
            TExpr::Static(_) => self.resolved(w, e),
            TExpr::Local(s) => match self.inline_arg(s) {
                // The argument's copy takes what the parameter's node said of the chain of `+`
                // around it: an ascription or a `toString` written on the parameter ends one.
                Some((copied, ty)) => {
                    self.set_walked_type(w, copied, ty);
                    if self.prog.ends_chain(e) {
                        self.prog.mark_chain_end(copied);
                    }
                    copied
                }
                None => {
                    if let Some(&t) = w.binder_types.get(&s) {
                        self.set_walked_type(w, e, t);
                    }
                    e
                }
            },
            TExpr::Field(r, s) => {
                let r2 = self.walk(w, r);
                if r2 != r {
                    self.prog.exprs[e.idx()] = TExpr::Field(r2, s);
                }
                self.resolved(w, e)
            }
            TExpr::CallStatic(_, args) => {
                self.walk_list(w, args);
                self.resolved(w, e)
            }
            TExpr::CallMethod(r, s, args) => {
                let r2 = self.walk(w, r);
                if r2 != r {
                    self.prog.exprs[e.idx()] = TExpr::CallMethod(r2, s, args);
                }
                self.walk_list(w, args);
                self.resolved(w, e)
            }
            TExpr::CallClosure(f, args) if args.len == 0 && matches!(self.prog.expr(f), TExpr::Local(s) if w.by_name.contains_key(&s)) => {
                w.visited += 1;
                self.walk(w, f)
            }
            TExpr::CallClosure(f, args) => {
                let f2 = self.walk(w, f);
                if f2 != f {
                    self.prog.exprs[e.idx()] = TExpr::CallClosure(f2, args);
                }
                self.walk_list(w, args);
                // A lambda argument applied in the body is its body with the parameters
                // replaced, as the retype path and scalac's `InlineTyper` reduce it.
                let items = self.prog.expr_list(args).to_vec();
                match self.beta_reduce(f2, &items) {
                    Some(reduced) => {
                        if let Some(t) = w.inst.types.get(e) {
                            self.set_walked_type(w, reduced, t);
                        }
                        reduced
                    }
                    None => e,
                }
            }
            TExpr::Js(template, args) => {
                self.walk_list(w, args);
                self.register_template(template, args);
                e
            }
            TExpr::New(c, _) if w.class_copies.contains_key(&c) => self.finish_class_copy(w, e, c),
            TExpr::New(_, args) | TExpr::NewVia(_, args) | TExpr::SeqLit(args) | TExpr::ArrayLit(args) | TExpr::ObjLit(args) => {
                self.walk_list(w, args);
                e
            }
            TExpr::StrConcat(args) => {
                self.walk_list(w, args);
                e
            }
            TExpr::Lambda(params, body) => {
                self.push_scope();
                for p in self.prog.sym_list(params).to_vec() {
                    self.bind_walked(p);
                }
                let b = self.walk(w, body);
                self.pop_scope();
                if b != body {
                    self.prog.exprs[e.idx()] = TExpr::Lambda(params, b);
                }
                e
            }
            TExpr::If(..) => self.walk_if(w, e),
            TExpr::Match(..) if w.kinds.is(e, CopyKinds::REDUCIBLE) => match w.def.reducible_sources[w.reducible_at(e).expect("a reducible match's source")] {
                ReducibleSource::SummonFrom { .. } => self.reduce_summon_from(w, e),
                _ => self.reduce_inline_match(w, e),
            },
            TExpr::Match(scrut, cases) => {
                let s2 = self.walk(w, scrut);
                if s2 != scrut {
                    self.prog.exprs[e.idx()] = TExpr::Match(s2, cases);
                }
                let sty = self.walked_type(w, s2);
                if sty != ERROR {
                    for i in cases.range() {
                        let p = self.prog.cases[i].pat;
                        self.settle_test(p, sty);
                    }
                }
                self.walk_cases(w, cases);
                e
            }
            TExpr::While(c, b) => {
                let c2 = self.walk(w, c);
                let b2 = self.walk(w, b);
                if (c2, b2) != (c, b) {
                    self.prog.exprs[e.idx()] = TExpr::While(c2, b2);
                }
                e
            }
            TExpr::Block(..) => self.walk_block(w, e),
            TExpr::Assign(a, b) => {
                let a2 = self.walk(w, a);
                let b2 = self.walk(w, b);
                if (a2, b2) != (a, b) {
                    self.prog.exprs[e.idx()] = TExpr::Assign(a2, b2);
                }
                e
            }
            TExpr::Prim(op, a, b) => {
                let a2 = self.walk(w, a);
                let mark = if op == PrimOp::BoolAnd { self.push_tested(a2) } else { self.tested.len() };
                let b2 = self.walk(w, b);
                self.pop_tested(mark);
                if (a2, b2) != (a, b) {
                    self.prog.exprs[e.idx()] = TExpr::Prim(op, a2, b2);
                }
                e
            }
            TExpr::Unary(op, a) => self.walk_one(w, e, a, |x| TExpr::Unary(op, x)),
            TExpr::ToStr(a, k) => {
                let a2 = self.walk(w, a);
                // A conversion the definition chose on a type the copy makes precise is chosen
                // again, as the retype path chooses it: none for a string.
                if k.kind() == StrKind::Generic {
                    let t = self.walked_type(w, a2);
                    // The program's `toString` on a type that declares one calls that member, as
                    // the retype path's member lookup comes before the universal `toString`.
                    if t != ERROR && !k.is_rendering() {
                        if let Some((member, _)) = self.find_member(t, crate::names::TO_STRING) {
                            self.prog.exprs[e.idx()] = TExpr::CallMethod(a2, member, crate::ast::ListRef::EMPTY);
                            return e;
                        }
                    }
                    if t != ERROR {
                        match self.str_conversion(t) {
                            None if k.is_rendering() => return a2,
                            // The program's `toString` of a string, the call `to_string_call` keeps.
                            None => {
                                let call = self.to_string_call(a2, t);
                                if call == a2 {
                                    return a2;
                                }
                                self.prog.exprs[e.idx()] = self.prog.expr(call);
                                return e;
                            }
                            Some(kind) => {
                                let conv = if k.is_rendering() { StrConv::rendering(kind) } else { StrConv::call(kind) };
                                self.prog.exprs[e.idx()] = TExpr::ToStr(a2, conv);
                                return e;
                            }
                        }
                    }
                }
                if a2 != a {
                    self.prog.exprs[e.idx()] = TExpr::ToStr(a2, k);
                }
                e
            }
            TExpr::TypeTest(a, t) => self.walk_one(w, e, a, |x| TExpr::TypeTest(x, t)),
            TExpr::Cast(a, op, to) => self.walk_cast(w, e, a, op, to),
            TExpr::Index(a, i) => self.walk_one(w, e, a, |x| TExpr::Index(x, i)),
            TExpr::JsSelect(a, n) => self.walk_one(w, e, a, |x| TExpr::JsSelect(x, n)),
            TExpr::Spread(a) => self.walk_one(w, e, a, TExpr::Spread),
            TExpr::Return(a) => self.walk_one(w, e, a, TExpr::Return),
            TExpr::Throw(a, wraps) => self.walk_one(w, e, a, |x| TExpr::Throw(x, wraps)),
            TExpr::Splice(lambda) => self.run_stored_splice(w, e, lambda),
            TExpr::Try(i) => {
                let (body, cases, finalizer) = {
                    let t = &self.prog.tries[i as usize];
                    (t.body, t.cases, t.finalizer)
                };
                let b2 = self.walk(w, body);
                self.walk_cases(w, cases);
                let f2 = finalizer.map(|f| self.walk(w, f));
                let t = &mut self.prog.tries[i as usize];
                t.body = b2;
                t.finalizer = f2;
                e
            }
        }
    }

    /// The statements that run before a parent's arguments are passed, walked in place.
    fn walk_prelude(&mut self, w: &mut Walk, prelude: crate::ast::ListRef) {
        let stmts: Vec<TStmt> = self.prog.stmt_list(prelude).to_vec();
        for (i, st) in stmts.into_iter().enumerate() {
            let walked = match st {
                TStmt::Val(s, x) => TStmt::Val(s, self.walk(w, x)),
                TStmt::Expr(x) => TStmt::Expr(self.walk(w, x)),
                other => other,
            };
            self.prog.stmts[prelude.range().start + i] = walked;
        }
    }

    /// A class the stored body makes, copied for the expansion (`Copier::stored_class`) and
    /// settled where the walk meets its creation, as the retype path's typing of it settles it
    /// there: the arguments to its parent walked where it is created, then its body in its
    /// class's frame, part by part in the order written, each method with its parameters in
    /// scope; what it captures read off the walked body in that order among the locals the
    /// expansion has in scope here, which lead its constructor's parameters (`captured_by`);
    /// and the instance made with those (`anon_instance`).
    fn finish_class_copy(&mut self, w: &mut Walk, e: TExprId, c: ClassId) -> TExprId {
        if self.syms.class(c).kind != ClassKind::Anon {
            return self.finish_local_copy(w, e, c);
        }
        if let Some(call) = self.anon_parent_args.get(&c).copied() {
            self.walk_prelude(w, call.prelude);
            self.walk_list(w, call.args);
        }
        self.settle_superclass_copy(w, c);
        let (index, captures) = self.settle_class_body(w, c);
        self.settle_nested_copies(w, c);
        let tc = &self.prog.classes[index];
        let sam = tc.captures > 0 && tc.captures == tc.ctor_params.len();
        let body = &mut self.prog.classes[index];
        body.ctor_defaults = vec![None; captures.len()];
        body.ctor_params = captures.clone();
        if sam {
            body.captures = captures.len();
        }
        self.anon_captures.insert(c, captures);
        let (instance, ty) = self.anon_instance(c);
        self.set_walked_type(w, instance, ty);
        instance
    }

    /// A creation of the copy of a named local class the stored body makes (`new C(args)`): the
    /// class settled at the first one the walk meets (its body walked, what it captures leading
    /// its constructor's parameters, as `finish_local_class` settles it), each creation passing
    /// what the class captures before its own arguments; one inside the class's own body, met
    /// while the class is settled, gets them once they are known.
    fn finish_local_copy(&mut self, w: &mut Walk, e: TExprId, c: ClassId) -> TExprId {
        let stored = w.class_copies[&c];
        let stored_captures = w.def.classes.iter().find(|tc| tc.id == stored).map_or(0, |tc| tc.captures);
        let TExpr::New(_, args) = self.prog.expr(e) else { unreachable!() };
        let items = self.prog.expr_list(args).to_vec();
        let own: Vec<TExprId> = items[stored_captures.min(items.len())..].iter().map(|&a| self.walk(w, a)).collect();
        let inside = self.env.frames.iter().any(|f| matches!(f, super::Frame::Class(k) if *k == c));
        let selfs = self.enclosing_object_selfs();
        match w.local_copies.get(&c) {
            Some(LocalCopy::Settled(captures)) => {
                let captures = captures.clone();
                return self.local_copy_new(e, c, &captures, inside, &selfs, &own);
            }
            Some(LocalCopy::Settling(_)) => {
                let l = self.prog.list(&own);
                self.prog.exprs[e.idx()] = TExpr::New(c, l);
                if let Some(LocalCopy::Settling(pending)) = w.local_copies.get_mut(&c) {
                    pending.push((e, inside, selfs));
                }
                return e;
            }
            None => {}
        }
        let captures = self.settle_local_copy(w, c);
        self.local_copy_new(e, c, &captures, inside, &selfs, &own)
    }

    /// The copy `c` of a named local class the stored body makes settled (`finish_local_copy`):
    /// the class it extends, a copy too, settled first, as the block types it first; its body
    /// walked, what it captures leading its constructor's parameters; the creations met while it
    /// was settled given those; and the classes nested in it the walk has not met a creation of
    /// settled after it, as its typing types its members. What it captures.
    fn settle_local_copy(&mut self, w: &mut Walk, c: ClassId) -> Vec<SymId> {
        let stored = w.class_copies[&c];
        let stored_captures = w.def.classes.iter().find(|tc| tc.id == stored).map_or(0, |tc| tc.captures);
        w.local_copies.insert(c, LocalCopy::Settling(Vec::new()));
        self.settle_superclass_copy(w, c);
        let (index, captures) = self.settle_class_body(w, c);
        let n = captures.len();
        let body = &mut self.prog.classes[index];
        let own_params: Vec<SymId> = body.ctor_params[stored_captures.min(body.ctor_params.len())..].to_vec();
        let own_defaults: Vec<Option<TExprId>> = body.ctor_defaults[stored_captures.min(body.ctor_defaults.len())..].to_vec();
        body.ctor_params = captures.iter().copied().chain(own_params).collect();
        body.ctor_defaults = std::iter::repeat(None).take(n).chain(own_defaults).collect();
        body.captures = n;
        self.anon_captures.insert(c, captures.clone());
        let Some(LocalCopy::Settling(pending)) = w.local_copies.insert(c, LocalCopy::Settled(captures.clone())) else { unreachable!() };
        for (p, p_inside, p_selfs) in pending {
            let TExpr::New(_, l) = self.prog.expr(p) else { continue };
            let rest = self.prog.expr_list(l).to_vec();
            self.local_copy_new(p, c, &captures, p_inside, &p_selfs, &rest);
        }
        self.settle_nested_copies(w, c);
        captures
    }

    /// The classes the copy `c` extends, its superclass and the traits it mixes in, where they
    /// are copies of classes the body makes not settled yet (ones the body never creates itself),
    /// settled before `c`, farthest first.
    fn settle_superclass_copy(&mut self, w: &mut Walk, c: ClassId) {
        let bases: Vec<ClassId> = self.syms.class(c).base_types.iter().skip(1).map(|&(b, _)| b).collect();
        for b in bases.into_iter().rev() {
            if self.unsettled_copy(w, b) {
                self.settle_local_copy(w, b);
            }
        }
    }

    /// Whether `c` is the copy of a named class the body makes, with a body, that the walk has not
    /// settled.
    fn unsettled_copy(&mut self, w: &Walk, c: ClassId) -> bool {
        w.class_copies.contains_key(&c) && !w.local_copies.contains_key(&c) && self.syms.class(c).kind != ClassKind::Anon && self.prog_index.class(&self.prog, c).is_some()
    }

    /// The copies of named classes the walk met no creation of (a class a pattern tests, an
    /// enum and its cases, the traits and classes a signature names), settled once the body is
    /// walked: those of the block first, in the order written, the ones nested in another class
    /// in their owner's frame.
    fn settle_remaining_copies(&mut self, w: &mut Walk) {
        let mut left: Vec<ClassId> = w.class_copies.keys().copied().filter(|&k| self.unsettled_copy(w, k)).collect();
        left.sort_by_key(|&k| (self.syms.class(k).span.start, k));
        for k in left {
            if !self.unsettled_copy(w, k) {
                continue;
            }
            match self.syms.class(k).owner {
                Owner::Class(o) if w.class_copies.contains_key(&o) => {
                    self.env.frames.push(super::Frame::Class(o));
                    self.settle_local_copy(w, k);
                    self.env.frames.pop();
                }
                _ => {
                    self.settle_local_copy(w, k);
                }
            }
        }
    }

    /// The copies of the classes nested in the copy `c` that the walk has not settled, settled in
    /// `c`'s frame.
    fn settle_nested_copies(&mut self, w: &mut Walk, c: ClassId) {
        let mut nested: Vec<ClassId> = w.class_copies.keys().copied().filter(|&k| self.syms.class(k).owner == Owner::Class(c) && !w.local_copies.contains_key(&k)).collect();
        nested.sort_by_key(|&k| self.syms.class(k).span.start);
        for k in nested {
            if self.syms.class(k).kind == ClassKind::Anon || w.local_copies.contains_key(&k) {
                continue;
            }
            self.env.frames.push(super::Frame::Class(c));
            self.settle_local_copy(w, k);
            self.env.frames.pop();
        }
    }

    /// The node `e` made the creation of the copy `c` of a named local class with the captures
    /// `captures` and its own arguments `own`, as `new_instance` makes one.
    fn local_copy_new(&mut self, e: TExprId, c: ClassId, captures: &[SymId], inside: bool, selfs: &[(SymId, TExprId)], own: &[TExprId]) -> TExprId {
        let mut items = self.capture_args(captures, inside, selfs);
        items.extend_from_slice(own);
        let l = self.prog.list(&items);
        self.prog.exprs[e.idx()] = TExpr::New(c, l);
        e
    }

    /// The body of the copy `c` of a class the stored body makes, walked where the walk meets
    /// its creation, as the retype path's typing of it types it there: in its class's frame,
    /// part by part in the order written, each method with its parameters in scope; and what it
    /// captures, read off the walked body in that order among the locals the expansion has in
    /// scope here (`captured_by`). The index of its body and its captures.
    fn settle_class_body(&mut self, w: &mut Walk, c: ClassId) -> (usize, Vec<SymId>) {
        self.anon_envs.insert(c, Arc::new(self.env.clone()));
        let Some(index) = self.prog_index.class(&self.prog, c) else { unreachable!("the copy's body") };
        let tc = self.prog.classes[index].clone();
        // The parts of the body in the order written, as the typing takes them.
        let mut parts: Vec<(u32, ClassPart)> = Vec::new();
        for &f in tc.methods.iter().chain(&tc.ctors) {
            let at = self.syms.sym(self.prog.funs[f.idx()].sym).span.start;
            parts.push((at, ClassPart::Fun(f)));
        }
        for (i, init) in tc.init.iter().enumerate() {
            if let TInit::Field(_, x) | TInit::Stmt(x) = *init {
                let at = self.prog.span_of(x).map_or(0, |(_, s)| s.start);
                parts.push((at, ClassPart::Init(i, x)));
            }
        }
        parts.sort_by_key(|&(at, _)| at);
        self.env.frames.push(super::Frame::Class(c));
        w.settling += 1;
        let replaced_mark = w.replaced.len();
        // A named class passes its parent's arguments from its constructor (an anonymous one's
        // creation passes them, `finish_class_copy`).
        let own_parent_args = self.syms.class(c).kind != ClassKind::Anon;
        if own_parent_args {
            self.walk_prelude(w, tc.parent_prelude);
            if let Some(args) = tc.parent_args {
                self.walk_list(w, args);
            }
        }
        for (i, d) in tc.ctor_defaults.iter().enumerate() {
            if let Some(d) = *d {
                let walked = self.walk(w, d);
                self.prog.classes[index].ctor_defaults[i] = Some(walked);
            }
        }
        for &(_, part) in &parts {
            match part {
                ClassPart::Fun(f) => self.walk_fun(w, f),
                ClassPart::Init(i, x) => {
                    let walked = self.walk(w, x);
                    self.prog.classes[index].init[i] = match self.prog.classes[index].init[i] {
                        TInit::Field(s, _) => TInit::Field(s, walked),
                        _ => TInit::Stmt(walked),
                    };
                }
            }
        }
        w.settling -= 1;
        self.env.frames.pop();
        // What it captures, in the order the typing meets it: the retype path reads it off the
        // order the typing makes the nodes of the class's body (`captured_by`), which is the
        // order the definition's typing made the stored nodes the walked ones copy, a lambda
        // typed after the arguments beside it as its inference asks; so each name the body
        // reads is ordered by the stored node it stands in, then by evaluation.
        let mut origin: FxMap<TExprId, u32> = w.inst.exprs.iter().map(|(stored, &copy)| (copy, stored.0)).collect();
        // A node the walk put in another's place stands where that one stood, what an expansion
        // in it brings in turn: `pass(first) + second` reads `first` at the call of `pass`.
        for &(from, to) in &w.replaced[replaced_mark..] {
            if let Some(&o) = origin.get(&from) {
                origin.entry(to).or_insert(o);
            }
        }
        if w.settling == 0 {
            w.replaced.clear();
        }
        let mut keyed: Vec<(u32, SymId)> = Vec::new();
        if own_parent_args {
            for &st in self.prog.stmt_list(tc.parent_prelude) {
                if let TStmt::Val(_, x) | TStmt::Expr(x) = st {
                    self.referenced_in_order(x, &origin, u32::MAX, &mut keyed);
                }
            }
            for &a in tc.parent_args.map_or(&[][..], |l| self.prog.expr_list(l)) {
                self.referenced_in_order(a, &origin, u32::MAX, &mut keyed);
            }
        }
        for &(_, part) in &parts {
            match part {
                ClassPart::Fun(f) => {
                    let (defaults, body) = {
                        let fun = &self.prog.funs[f.idx()];
                        (fun.defaults.clone(), fun.body)
                    };
                    for d in defaults.into_iter().flatten() {
                        self.referenced_in_order(d, &origin, u32::MAX, &mut keyed);
                    }
                    if let Some(b) = body {
                        self.referenced_in_order(b, &origin, u32::MAX, &mut keyed);
                    }
                }
                ClassPart::Init(i, _) => {
                    if let TInit::Field(_, x) | TInit::Stmt(x) = self.prog.classes[index].init[i] {
                        self.referenced_in_order(x, &origin, u32::MAX, &mut keyed);
                    }
                }
            }
        }
        keyed.sort_by_key(|&(k, _)| k);
        let mut refs: Vec<SymId> = Vec::with_capacity(keyed.len());
        for (_, s) in keyed {
            if !refs.contains(&s) {
                refs.push(s);
            }
        }
        (index, self.captured_among(c, &refs))
    }

    /// The locals and local functions `e` names, each with the place the typing of the stored
    /// body met it: the stored node the walked one copies (`origin`), else the nearest such
    /// node above it (`key`).
    fn referenced_in_order(&self, e: TExprId, origin: &FxMap<TExprId, u32>, key: u32, out: &mut Vec<(u32, SymId)>) {
        let prog = &self.prog;
        let key = origin.get(&e).copied().unwrap_or(key);
        let list = |t: &Self, l: crate::ast::ListRef, out: &mut Vec<(u32, SymId)>| {
            for &x in prog.expr_list(l) {
                t.referenced_in_order(x, origin, key, out);
            }
        };
        let note = |s: SymId, out: &mut Vec<(u32, SymId)>| out.push((key, s));
        match prog.expr(e) {
            TExpr::Local(s) => note(s, out),
            TExpr::CallStatic(s, args) => {
                note(s, out);
                list(self, args, out);
            }
            TExpr::Field(r, _) | TExpr::Unary(_, r) | TExpr::ToStr(r, _) | TExpr::TypeTest(r, _) | TExpr::Cast(r, ..) | TExpr::Index(r, _) | TExpr::Spread(r) | TExpr::Return(r) | TExpr::Throw(r, _) | TExpr::JsSelect(r, _) | TExpr::Splice(r) | TExpr::Lambda(_, r) => {
                self.referenced_in_order(r, origin, key, out)
            }
            TExpr::New(_, args) | TExpr::NewVia(_, args) | TExpr::StrConcat(args) | TExpr::Js(_, args) | TExpr::SeqLit(args) | TExpr::ArrayLit(args) | TExpr::ObjLit(args) => list(self, args, out),
            TExpr::CallMethod(r, _, args) | TExpr::CallClosure(r, args) => {
                self.referenced_in_order(r, origin, key, out);
                list(self, args, out);
            }
            TExpr::If(c, t, els) => {
                self.referenced_in_order(c, origin, key, out);
                self.referenced_in_order(t, origin, key, out);
                if let Some(x) = els {
                    self.referenced_in_order(x, origin, key, out);
                }
            }
            TExpr::While(a, b) | TExpr::Assign(a, b) | TExpr::Prim(_, a, b) => {
                self.referenced_in_order(a, origin, key, out);
                self.referenced_in_order(b, origin, key, out);
            }
            TExpr::Block(stmts, res) => {
                for st in &prog.stmts[stmts.range()] {
                    match *st {
                        TStmt::Expr(x) | TStmt::Val(_, x) | TStmt::Pat(_, x) => self.referenced_in_order(x, origin, key, out),
                        TStmt::Fun(f) => {
                            let fun = &prog.funs[f.idx()];
                            for &d in fun.defaults.iter().flatten() {
                                self.referenced_in_order(d, origin, key, out);
                            }
                            if let Some(b) = fun.body {
                                self.referenced_in_order(b, origin, key, out);
                            }
                        }
                    }
                }
                self.referenced_in_order(res, origin, key, out);
            }
            TExpr::Match(scrut, cases) => {
                self.referenced_in_order(scrut, origin, key, out);
                for c in &prog.cases[cases.range()] {
                    if let Some(g) = c.guard {
                        self.referenced_in_order(g, origin, key, out);
                    }
                    self.referenced_in_order(c.body, origin, key, out);
                }
            }
            TExpr::Try(i) => {
                let t = &prog.tries[i as usize];
                self.referenced_in_order(t.body, origin, key, out);
                for c in &prog.cases[t.cases.range()] {
                    self.referenced_in_order(c.body, origin, key, out);
                }
                if let Some(f) = t.finalizer {
                    self.referenced_in_order(f, origin, key, out);
                }
            }
            _ => {}
        }
    }

    /// What the lifted class `c` captures of the names `refs` its body reads, in their order:
    /// those of the scopes where it is made (`anon_envs`), of the call site's, and of the
    /// expansions around, as `captured_by` finds them.
    fn captured_among(&mut self, c: ClassId, refs: &[SymId]) -> Vec<SymId> {
        let visible = self.visible_to_class(c);
        refs.iter().copied().filter(|s| visible.contains(s)).collect()
    }

    /// A macro's splice of the copy (`${ (using q: Quotes) => impl(q) }`): its code, the copy of
    /// the stored lambda over `Quotes`, walked with its `Quotes` a given in scope (the inline calls
    /// it keeps resolved where the body expands, as the retype path's typing of the code at the
    /// call expands them), then run as the retype path runs the code it types (`run_macro`), its
    /// quotes' records the copy's own (`Copier::stored_quote`), its tree standing at the splice's
    /// type in the copy's terms.
    fn run_stored_splice(&mut self, w: &mut Walk, e: TExprId, lambda: TExprId) -> TExprId {
        let TExpr::Lambda(binder, code) = self.prog.expr(lambda) else { return e };
        let &[q] = self.prog.sym_list(binder) else { return e };
        let span = self.prog.span_of(e).map_or(Span::default(), |(_, s)| s);
        let mark = self.diags.items.len();
        let t_pre = self.profile.on.then(std::time::Instant::now);
        self.push_scope();
        self.bind_walked(q);
        self.bind_given(q);
        self.quote.macro_depth += 1;
        let code = self.walk(w, code);
        self.quote.macro_depth -= 1;
        self.pop_scope();
        // An error scalac reports after typing (`state::reports_late`) does not stop the walk.
        if w.failed || self.first_error_since(mark).is_some() {
            w.failed = true;
            let unit = self.prog.add(TExpr::Unit);
            self.set_walked_type(w, unit, ERROR);
            return unit;
        }
        let ret = self.walked_type(w, e);
        let (tree, ty) = self.run_macro(super::quoted::MacroRun { quotes: q, code, ret, span, mark, t_pre });
        if self.first_error_since(w.diag_mark).is_some() {
            w.failed = true;
        }
        // A method declared `Unit` takes any `Expr`, the value discarded as the retype path's
        // typing of the body against `Unit` discards it.
        let (tree, ty) = if ret == self.b.t_unit && ty != ERROR && ty != ret { (self.adapt(tree, ty, ret, span), ret) } else { (tree, ty) };
        self.set_walked_type(w, tree, ty);
        tree
    }

    /// What the output registers of a template call of the copy, which the stored body left out
    /// as no part of the output: a member called through a template on a receiver, whose
    /// overrides the program's classes keep (`Program::template_calls`), and a `getClass`, which
    /// keeps every class's qualified name (`Program::get_class_units`), both the unit's that
    /// the expansion stands in.
    fn register_template(&mut self, template: crate::tir::StrRef, args: crate::ast::ListRef) {
        if let Some(&sym) = self.prog.template_syms.get(&template) {
            let info = self.syms.sym(sym);
            if matches!(info.owner, Owner::Class(_)) && !info.is_extension && args.len > 0 {
                let unit = self.typing_unit();
                self.prog.template_calls.push((sym, unit));
            }
        }
        if self.prog.strings[template.idx()] == "$getClass($0)" {
            let unit = self.typing_unit();
            if !self.prog.get_class_units.contains(&unit) {
                self.prog.get_class_units.push(unit);
            }
        }
    }

    fn walk_one(&mut self, w: &mut Walk, e: TExprId, a: TExprId, make: impl FnOnce(TExprId) -> TExpr) -> TExprId {
        let a2 = self.walk(w, a);
        if a2 != a {
            self.prog.exprs[e.idx()] = make(a2);
        }
        e
    }

    fn walk_list(&mut self, w: &mut Walk, l: crate::ast::ListRef) {
        for i in l.range() {
            let x = self.prog.expr_lists[i];
            let y = self.walk(w, x);
            if y != x {
                self.prog.expr_lists[i] = y;
            }
        }
    }

    fn walk_cases(&mut self, w: &mut Walk, cases: crate::ast::ListRef) {
        for i in cases.range() {
            let c = self.prog.cases[i];
            self.walk_pattern(w, c.pat);
            self.push_scope();
            let mut binders = Vec::new();
            self.walked_pattern_binders(c.pat, &mut binders);
            for b in binders {
                self.bind_walked(b);
            }
            let guard = c.guard.map(|g| self.walk(w, g));
            let body = self.walk(w, c.body);
            self.pop_scope();
            self.prog.cases[i] = TCase { pat: c.pat, guard, body };
        }
    }

    /// A type test of a case of a plain match that the scrutinee's type the copy makes precise
    /// passes whatever the value, as the retype path's typing of the pattern finds it: no test.
    fn settle_test(&mut self, p: TPatId, sty: TypeId) {
        let (test, t, inner) = match self.prog.pats[p.idx()] {
            TPat::Test(test, t, inner) => (test, t, inner),
            TPat::Bind(_, Some(inner)) => return self.settle_test(inner, sty),
            _ => return,
        };
        if matches!(self.prog.tests[test.idx()], TypeTest::Always) || self.types.contains_error(t) {
            return;
        }
        let mark = self.snapshot();
        let always = self.is_sub(sty, t) && self.snapshot() == mark && !self.null_conforms(t);
        self.rollback(mark);
        if always {
            let always = self.prog.add_test(TypeTest::Always);
            self.prog.pats[p.idx()] = TPat::Test(always, t, inner);
        }
    }

    /// The expressions a pattern holds: a literal or a stable path it is compared with, an
    /// extractor's call.
    fn walk_pattern(&mut self, w: &mut Walk, p: TPatId) {
        match self.prog.pats[p.idx()] {
            TPat::Wildcard => {}
            TPat::Bind(_, inner) => {
                if let Some(i) = inner {
                    self.walk_pattern(w, i);
                }
            }
            TPat::Test(_, _, inner) => self.walk_pattern(w, inner),
            TPat::Equals(x, strict) => {
                let y = self.walk(w, x);
                if y != x {
                    self.prog.pats[p.idx()] = TPat::Equals(y, strict);
                }
            }
            TPat::Class(_, _, _, subs) | TPat::Alt(subs) => {
                for q in self.prog.pat_list(subs).to_vec() {
                    self.walk_pattern(w, q);
                }
            }
            TPat::Seq(items, rest) => {
                for q in self.prog.pat_list(items).to_vec() {
                    self.walk_pattern(w, q);
                }
                if let Some(r) = rest {
                    self.walk_pattern(w, r);
                }
            }
            TPat::Unapply(s, call, inner) => {
                let c2 = self.walk(w, call);
                if c2 != call {
                    self.prog.pats[p.idx()] = TPat::Unapply(s, c2, inner);
                }
                self.walk_pattern(w, inner);
            }
        }
    }

    /// A block walked statement by statement in a scope of its own, as the retype path types it:
    /// its local methods named from its start, each val from its statement on, the givens among
    /// them in scope for what the walk resolves inside the block (`summon_at_site` reads the
    /// scopes above the expansion's, `body_scopes`), the imports the definition entered
    /// before each statement too, and every local of the block where a class of an expansion
    /// the walk hands to the retype path captures it. A temporary the definition hoisted an
    /// operand into (`hoist`) that the walk made stable, or whose call's receiver it made stable,
    /// is the operand itself in the call, as the retype path hoists nothing then.
    fn walk_block(&mut self, w: &mut Walk, e: TExprId) -> TExprId {
        let TExpr::Block(stmts, res) = self.prog.expr(e) else { unreachable!() };
        let imports = w.imports.get(&e).cloned().unwrap_or_default();
        let aliases = w.aliases.get(&e).cloned().unwrap_or_default();
        let imports_mark = self.inline.body_imports.len();
        let aliases_mark = self.inline.import_aliases.len();
        let mut next = (0, 0);
        let mut inline_vals: Vec<SymId> = Vec::new();
        self.push_scope();
        for i in stmts.range() {
            if let TStmt::Fun(f) = self.prog.stmts[i] {
                let s = self.prog.funs[f.idx()].sym;
                self.bind_walked(s);
            }
        }
        for (at, i) in stmts.range().enumerate() {
            self.enter_walked_imports(w, &imports, &aliases, at as u32, &mut next);
            if w.failed {
                break;
            }
            let st = self.prog.stmts[i];
            let walked = match st {
                TStmt::Expr(x) => TStmt::Expr(self.walk(w, x)),
                TStmt::Val(s, x) => {
                    let x2 = self.walk(w, x);
                    self.infer_walked_val(w, s, x2);
                    self.bind_walked(s);
                    self.stand_for_inline_val(w, s, x2, &mut inline_vals);
                    TStmt::Val(s, x2)
                }
                TStmt::Pat(p, x) => {
                    let x2 = self.walk(w, x);
                    self.walk_pattern(w, p);
                    let mut binders = Vec::new();
                    self.walked_pattern_binders(p, &mut binders);
                    for b in binders {
                        self.bind_walked(b);
                    }
                    TStmt::Pat(p, x2)
                }
                TStmt::Fun(f) => {
                    self.walk_fun(w, f);
                    TStmt::Fun(f)
                }
            };
            self.prog.stmts[i] = walked;
        }
        self.enter_walked_imports(w, &imports, &aliases, stmts.len, &mut next);
        let res2 = if w.failed { res } else { self.walk(w, res) };
        self.pop_scope();
        self.inline.body_imports.truncate(imports_mark);
        for alias in self.inline.import_aliases.drain(aliases_mark..) {
            self.inline.args.remove(&alias);
        }
        for s in inline_vals {
            self.inline.args.remove(&s);
        }
        let (stmts2, res2) = self.unhoist(w, stmts, res2);
        if stmts2.len == 0 {
            return res2;
        }
        if (stmts2, res2) != (stmts, res) {
            self.prog.exprs[e.idx()] = TExpr::Block(stmts2, res2);
        }
        if res2 != res {
            let t = self.walked_type(w, res2);
            self.set_walked_type(w, e, t);
        }
        e
    }

    /// The imports of a walked block the definition entered before its statement `at`, and the
    /// names it bound to a value's member there, from `next` on, in the block's scope: an import
    /// from a parameter or a renamed local names what stands for it in the copy (the proxy, the
    /// fresh local), and a name bound to a value's member stands for the walked copy of the
    /// selection, a given where it is one, until the block ends.
    fn enter_walked_imports(&mut self, w: &mut Walk, imports: &[(u32, ResolvedImport)], aliases: &[(u32, usize)], at: u32, next: &mut (usize, usize)) {
        while next.0 < imports.len() && imports[next.0].0 <= at {
            let frame = self.env.frames.len() - 1;
            let import = self.walked_import(w, imports[next.0].1);
            self.inline.body_imports.push((frame, import));
            next.0 += 1;
        }
        while next.1 < aliases.len() && aliases[next.1].0 <= at {
            let alias = w.inst.aliases[aliases[next.1].1];
            next.1 += 1;
            let tree = self.demand_in_walk(w, alias.tree, None);
            let walked = self.walk(w, tree);
            let ty = self.walked_type(w, walked);
            let local = self.clone_local(alias.local);
            let mut sig = (*self.sig_of(local)).clone();
            sig.ret = ty;
            self.syms.sym_mut(local).sig = Some(Arc::new(sig));
            self.inline.args.insert(local, super::inline::InlineArg { expr: walked, ty, source: None });
            self.inline.import_aliases.push(local);
            self.bind_walked(local);
        }
    }

    /// An import of the stored body in the copy's terms: one that selects on a parameter or on
    /// a local of the body selects on what stands for it (the parameter's proxy, the local's
    /// fresh copy).
    fn walked_import(&mut self, w: &Walk, import: ResolvedImport) -> ResolvedImport {
        use super::ImportTarget;
        let target = match import.target {
            ImportTarget::ValueAll(v) => self.walked_value(w, v).map(ImportTarget::ValueAll),
            ImportTarget::ValueMember(v, n) => self.walked_value(w, v).map(|v| ImportTarget::ValueMember(v, n)),
            ImportTarget::ValueGivens(v) => self.walked_value(w, v).map(ImportTarget::ValueGivens),
            _ => None,
        };
        match target {
            Some(target) => ResolvedImport { target, ..import },
            None => import,
        }
    }

    /// The value an import selects on, in the copy's terms: `None` where it names no parameter
    /// and no local of the body.
    fn walked_value(&mut self, w: &Walk, v: super::ValueImport) -> Option<super::ValueImport> {
        let (s, module, through) = self.import_values[v.0 as usize];
        let moved = match through {
            Some(prev) => (s, module, Some(self.walked_value(w, prev)?)),
            None => (*w.param_proxies.iter().find(|(p, _)| *p == s).map(|(_, x)| x).or_else(|| w.inst.renames.get(&s))?, module, None),
        };
        let at = super::ValueImport(self.import_values.len() as u32);
        self.import_values.push(moved);
        Some(at)
    }

    /// A val whose type the definition inferred, typed again from its walked initialiser as the
    /// retype path types it where the method expands (`List[String]` where the definition had a
    /// type of its type parameter's elements): what reads it, a class that captures it among
    /// them, has the precise type.
    fn infer_walked_val(&mut self, w: &mut Walk, s: SymId, init: TExprId) {
        if !w.inferred_vals.contains_key(&s) {
            return;
        }
        let t = self.walked_type(w, init);
        if t == ERROR {
            return;
        }
        let t = self.solve_inferred(t);
        let t = self.widen_soft(init, t);
        let mut sig = (*self.sig_of(s)).clone();
        if sig.ret != t {
            sig.ret = t;
            self.syms.sym_mut(s).sig = Some(Arc::new(sig));
            w.binder_types.insert(s, t);
        }
    }

    /// An `inline val` of the body whose value the definition left to an inline call
    /// (`inline val size = constValue[Tuple.Size[E]]`): once walked, a constant its uses stand
    /// for, of its literal type, as the retype path's `check_inline_val` gives it, until the
    /// block ends.
    fn stand_for_inline_val(&mut self, w: &mut Walk, s: SymId, init: TExprId, out: &mut Vec<SymId>) {
        if !w.inline_vals.contains_key(&s) || self.inline.args.contains_key(&s) {
            return;
        }
        let walked = self.walked_type(w, init);
        let by_type = match self.fold_constant(init).filter(|_| !self.expanded_widening(init)) {
            Some(v) => Some((v, false)),
            None => self.fold_type(walked).map(|v| (v, true)),
        };
        // A constant of the initialiser's type is the val's where the initialiser is pure, as
        // scalac requires of it.
        if let Some((_, true)) = by_type {
            if !self.is_pure_value(init) {
                let msg = format!("inline value must be pure but was: {}", self.code_of_tree(init));
                if let Some(site) = self.inline.sites.last() {
                    let (file, span) = (site.body_file, site.body_span);
                    self.diags.error(file, span, msg);
                }
                return;
            }
        }
        let Some((v, _)) = by_type else {
            // scalac's check of the value at the expansion, where the definition kept it.
            let t = self.deref(walked);
            let msg = if t == self.b.t_unit {
                "`inline val` of type `Unit` is not supported.\n\nTo inline a `Unit` consider using `inline def`"
            } else if t == self.b.t_string || self.is_primitive(t) {
                "inline value must have a literal constant type"
            } else {
                "inline value must contain a literal constant value.\n\nTo inline more complex types consider using `inline def`"
            };
            // In the body, which the expansion's relocation moves to the call with its note.
            if let Some(site) = self.inline.sites.last() {
                let (file, span) = (site.body_file, site.body_span);
                self.diags.error(file, span, msg);
            }
            return;
        };
        let ty = self.types.lit(v);
        let mut sig = (*self.sig_of(s)).clone();
        sig.ret = ty;
        self.syms.sym_mut(s).sig = Some(Arc::new(sig));
        self.set_walked_type(w, init, ty);
        self.inline.args.insert(s, super::inline::InlineArg { expr: init, ty, source: None });
        out.push(s);
    }

    /// A local the walk has in scope: a name of the frame, a given of it where it is one.
    fn bind_walked(&mut self, s: SymId) {
        let info = self.syms.sym(s);
        let (name, given) = (info.name, info.mods & (crate::ast::mods::GIVEN | crate::ast::mods::IMPLICIT) != 0);
        self.bind_local(name, s);
        if given {
            self.bind_given(s);
        }
    }

    /// The binders of a pattern.
    fn walked_pattern_binders(&self, p: TPatId, out: &mut Vec<SymId>) {
        match self.prog.pats[p.idx()] {
            TPat::Wildcard | TPat::Equals(..) => {}
            TPat::Bind(s, inner) => {
                out.push(s);
                if let Some(i) = inner {
                    self.walked_pattern_binders(i, out);
                }
            }
            TPat::Test(_, _, inner) | TPat::Unapply(_, _, inner) => self.walked_pattern_binders(inner, out),
            TPat::Class(_, _, _, subs) | TPat::Alt(subs) => {
                for &q in self.prog.pat_list(subs) {
                    self.walked_pattern_binders(q, out);
                }
            }
            TPat::Seq(items, rest) => {
                for &q in self.prog.pat_list(items) {
                    self.walked_pattern_binders(q, out);
                }
                if let Some(r) = rest {
                    self.walked_pattern_binders(r, out);
                }
            }
        }
    }

    /// A local function of the copy: its defaults and body walked with its parameters in scope.
    fn walk_fun(&mut self, w: &mut Walk, f: FunId) {
        let (params, defaults, body) = {
            let fun = &self.prog.funs[f.idx()];
            (fun.params.clone(), fun.defaults.clone(), fun.body)
        };
        self.push_scope();
        for &p in &params {
            self.bind_walked(p);
        }
        let defaults: Vec<Option<TExprId>> = defaults.into_iter().map(|d| d.map(|x| self.walk(w, x))).collect();
        let body = body.map(|b| self.walk(w, b));
        self.pop_scope();
        let fun = &mut self.prog.funs[f.idx()];
        fun.defaults = defaults;
        fun.body = body;
    }

    /// The statements of a walked block without the temporaries the definition hoisted operands
    /// into where the retype path hoists none: an operand the walk made stable, or any operand
    /// of a call whose receiver it made stable, which then stands in the call's arguments.
    fn unhoist(&mut self, w: &Walk, stmts: crate::ast::ListRef, res: TExprId) -> (crate::ast::ListRef, TExprId) {
        if w.hoisted.is_empty() || stmts.len == 0 {
            return (stmts, res);
        }
        let TExpr::CallMethod(recv, _, args) = self.prog.expr(res) else { return (stmts, res) };
        let recv_stable = self.is_stable(recv);
        let items: Vec<TStmt> = self.prog.stmts[stmts.range()].to_vec();
        let mut kept = Vec::with_capacity(items.len());
        let mut moved = false;
        for st in items {
            if let TStmt::Val(h, init) = st {
                if w.hoisted.contains_key(&h) && (recv_stable || self.is_stable(init)) {
                    let slot = args.range().find(|&i| matches!(self.prog.expr(self.prog.expr_lists[i]), TExpr::Local(s) if s == h));
                    if let Some(i) = slot {
                        self.prog.expr_lists[i] = init;
                        moved = true;
                        continue;
                    }
                }
            }
            kept.push(st);
        }
        if !moved {
            return (stmts, res);
        }
        (self.prog.stmts.push_slice(&kept), res)
    }

    /// A kept call resolved once its receiver and arguments are walked: expanded at the site
    /// by `expand_inline` (an intrinsic of `scala.compiletime` evaluated, an inline method
    /// expanded by this walk or by the retype path, one frame deeper), the node that replaces it
    /// taking what the call's node said of the chain of `+` around it. Any other node as it is.
    fn resolved(&mut self, w: &mut Walk, e: TExprId) -> TExprId {
        if w.failed {
            return e;
        }
        if !w.kinds.is(e, CopyKinds::DEFERRED) {
            self.retype_member(w, e);
            self.retype_dependent_call(w, e);
            return e;
        }
        let Some(d) = self.quote.deferred.get(&e).cloned() else { return e };
        let (recv, args) = match self.prog.expr(e) {
            TExpr::CallMethod(r, _, args) => (Some(r), self.prog.expr_list(args).to_vec()),
            TExpr::CallStatic(_, args) => (None, self.prog.expr_list(args).to_vec()),
            TExpr::Field(r, _) => (Some(r), Vec::new()),
            TExpr::Static(_) => (None, Vec::new()),
            _ => return e,
        };
        let arg_types: Vec<(TExprId, TypeId)> = args.iter().filter_map(|&a| w.inst.types.get(a).map(|t| (a, t))).collect();
        // The receiver's type as the retype path's typing of the receiver gives it: a type
        // member of a path the argument's type fixes is what it stands for.
        if let Some(r) = recv {
            if let Some(t) = w.inst.types.get(r) {
                let t = self.dealias(t);
                self.inline.recv_types.insert(r, t);
                self.set_walked_type(w, r, t);
            }
        }
        if let Some(r) = recv {
            if let Some((node, ty)) = self.tuple_builtin(w, r, d.sym, &args, d.span) {
                self.prog.copy_chain_marks(e, node);
                self.set_walked_type(w, node, ty);
                return node;
            }
        }
        let dispatched = recv.and_then(|r| self.dispatch_deferred_member(w, r, &d));
        let (sym, sig, owner_subst, subst, ret_ty) = match dispatched {
            Some(found) => found,
            None => (d.sym, d.sig.clone(), d.owner_subst.clone(), d.subst.clone(), d.ret_ty),
        };
        let (sig, owner_subst, ret_ty, prefix) = match (recv, d.prefix) {
            (Some(r), Some(_)) => self.seen_from_walked_receiver(w, r, sym, &args, &subst, (sig, owner_subst, ret_ty)),
            _ => (sig, owner_subst, ret_ty, d.prefix),
        };
        // The type arguments as the retype path resolves them where the body expands: a type
        // member of a parameter's path and a match type over it reduced with the argument's type
        // (`Tuple.Map[m.MirroredElemTypes, F]` of a mirror's proxy is the tuple of its elements).
        let mut subst: Subst = subst.iter().map(|&(p, t)| (p, self.resolved_type_arg(t))).collect();
        // A program's callee the retype path expands (the bridge: a body the capture holds back)
        // types its body from the type arguments alone, where a type member of a path of the
        // walk (`m.MirroredElemTypes` of a summoned mirror) is no path it knows: it takes what
        // the path's type fixes the member to.
        // A deferred inline method is no callee of either path: its call is refused before any
        // entry decision (`expand_inline_call`).
        let deferred = self.syms.sym(sym).def.is_some() && self.is_abstract_member(sym);
        let entry = (self.intrinsic_of(sym).is_none() && self.is_inline_callee(sym) && !deferred).then(|| self.substitution_entry(sym));
        if matches!(entry, Some(Err(e)) if e != Fallback::Library) {
            for (_, t) in subst.iter_mut() {
                if self.types.has_paths(*t) {
                    *t = self.members_of_paths(*t);
                }
            }
        }
        // A checked record the entry decision just found is the one the expansion takes: its
        // call is not decided a second time (`InlineState::entry`).
        if let Some(Ok(def)) = entry {
            self.inline.entry = Some((sym, def));
        }
        let call = MethodCall { recv, sym, owner_subst, ext_recv: None, prefix };
        self.inline.site_at_end = d.at_end;
        self.inline.arg_types = arg_types;
        let expanded = self.expand_inline(&call, &sig, &subst, &args, ret_ty, d.span, d.expected);
        self.inline.entry = None;
        self.inline.site_at_end = false;
        self.inline.arg_types.clear();
        if let Some(r) = recv {
            self.inline.recv_types.remove(&r);
        }
        if self.first_error_since(w.diag_mark).is_some() {
            w.failed = true;
        }
        match expanded {
            Some((node, ty)) => {
                // An expansion may give a node another worker made (a given's reference the
                // search found before the fork, `summonInline`'s): the walk keeps its type in the
                // instance's table alone and writes nothing on it, as a worker records nothing it
                // did not make, the node standing in the tree with the records the expansion
                // gave it as one worker's does.
                w.inst.types.insert(node, ty);
                if self.prog.owns_expr(node) {
                    self.prog.set_type(node, ty);
                    self.prog.copy_chain_marks(e, node);
                }
                if node != e && !self.capturing() {
                    self.quote.deferred.local.remove(&e);
                }
                node
            }
            None => e,
        }
    }

    /// A member the definition selected on a receiver that the copy makes precise, where the
    /// receiver's class overrides it: the member the definition resolved, typed as the override
    /// seen from the receiver, as scalac's `InlineTyper` types it (`member(O)` of `transparent
    /// inline def member(x: B) = x.value` is an `O.type` where `O` overrides `value` with that
    /// type), and named as the override, as scalac's selection on the new prefix names it
    /// (`pick(x.value)` on `O` calls `O$.value()`). A generic member's type arguments are the
    /// definition's, the override's parameters taking them in order.
    fn retype_member(&mut self, w: &mut Walk, e: TExprId) {
        let (r, s) = match self.prog.expr(e) {
            TExpr::Field(r, s) | TExpr::CallMethod(r, s, _) => (r, s),
            _ => return,
        };
        let Owner::Class(owner) = self.syms.sym(s).owner else { return };
        if self.is_inline_callee(s) || self.syms.alternatives(s).is_some() {
            return;
        }
        let arity = self.sig_of(s).tparams.len();
        let targs: Vec<TypeId> = match w.inst.type_args.iter().find(|(c, _)| *c == e) {
            Some((_, ts)) if ts.len() == arity => ts.clone(),
            None if arity == 0 => Vec::new(),
            _ => return,
        };
        let rt = self.walked_type(w, r);
        if rt == ERROR {
            return;
        }
        let mut classes = Vec::new();
        self.own_classes(rt, &mut classes);
        let Some(&c) = classes.first() else { return };
        if c == owner || !self.derives_from(c, owner) {
            self.retype_own_member(w, e, r, rt, s, owner, targs);
            return;
        }
        let bases = self.syms.class(c).base_types.clone();
        let Some(i) = self.implementation_of(c, &bases, s).filter(|&i| i != s) else {
            self.retype_own_member(w, e, r, rt, s, owner, targs);
            return;
        };
        if self.is_inline_callee(i) || self.is_abstract_member(i) || self.syms.alternatives(i).is_some() || self.sig_of(i).tparams.len() != arity {
            return;
        }
        let Owner::Class(io) = self.syms.sym(i).owner else { return };
        let Some(base) = self.base_type(rt, io) else { return };
        let mut subst = self.owner_subst(base);
        let isig = self.sig_arc(i);
        subst.extend(isig.tparams.iter().copied().zip(targs));
        let mut ty = self.types.subst(isig.ret, &subst);
        if self.types.has_paths(ty) {
            let prefix = self.prefix_of(r, rt);
            ty = self.seen_from_prefix(ty, prefix, i);
        }
        // An override names itself, one the receiver's class inherits from further up does not.
        if io != owner && self.derives_from(io, owner) {
            self.prog.exprs[e.idx()] = match self.prog.expr(e) {
                TExpr::Field(r, _) => TExpr::Field(r, i),
                TExpr::CallMethod(r, _, args) => TExpr::CallMethod(r, i, args),
                other => other,
            };
        }
        self.set_walked_type(w, e, ty);
    }

    /// A member the receiver's class does not override, typed as the walked receiver's type
    /// sees it, as the retype path types the selection on the receiver it types (`cases(i)` of a
    /// `List[String]` is a `String` where the definition had its elements' abstract type).
    #[allow(clippy::too_many_arguments)]
    fn retype_own_member(&mut self, w: &mut Walk, e: TExprId, r: TExprId, rt: TypeId, s: SymId, owner: ClassId, targs: Vec<TypeId>) {
        let Some(base) = self.base_type(rt, owner) else { return };
        let mut subst = self.owner_subst(base);
        let sig = self.sig_arc(s);
        subst.extend(sig.tparams.iter().copied().zip(targs));
        let mut ty = self.types.subst(sig.ret, &subst);
        if self.types.has_paths(ty) {
            let prefix = self.prefix_of(r, rt);
            ty = self.seen_from_prefix(ty, prefix, s);
        }
        if ty != ERROR && !self.types.contains_error(ty) && !self.types.has_vars(ty) {
            self.set_walked_type(w, e, ty);
        }
    }

    /// A type argument of a kept call as the retype path resolves it where the body expands: a
    /// match type over a type member of a path reduced, the member being what the path's type
    /// fixes it to (`Tuple.Map[m.MirroredElemTypes, F]` of a mirror's proxy is the tuple of the
    /// elements' `F`s); any other as it is, which what reads it dealiases.
    fn resolved_type_arg(&mut self, t: TypeId) -> TypeId {
        if !self.types.has_paths(t) || !self.types.is_reducible(t) {
            return t;
        }
        let t = self.members_of_paths(t);
        self.normalize(t)
    }

    /// `t` with each type member of a path replaced by the type the path's type fixes it to.
    fn members_of_paths(&mut self, t: TypeId) -> TypeId {
        if !self.types.has_paths(t) {
            return t;
        }
        match self.types.get(t) {
            Type::Member(..) | Type::Select(..) => {
                let d = self.dealias(t);
                if d == t { t } else { self.members_of_paths(d) }
            }
            Type::Class(c, args) => {
                let items: Vec<TypeId> = self.types.items(args).to_vec();
                let mapped: Vec<TypeId> = items.iter().map(|&a| self.members_of_paths(a)).collect();
                if mapped == items { t } else { self.types.class(c, &mapped) }
            }
            Type::Alias(a, args) => {
                let items: Vec<TypeId> = self.types.items(args).to_vec();
                let mapped: Vec<TypeId> = items.iter().map(|&x| self.members_of_paths(x)).collect();
                if mapped == items {
                    t
                } else {
                    let l = self.types.list(&mapped);
                    self.types.mk(Type::Alias(a, l))
                }
            }
            Type::Union(a, b) => {
                let (x, y) = (self.members_of_paths(a), self.members_of_paths(b));
                if (x, y) == (a, b) { t } else { self.types.union(x, y) }
            }
            Type::Inter(a, b) => {
                let (x, y) = (self.members_of_paths(a), self.members_of_paths(b));
                if (x, y) == (a, b) { t } else { self.types.inter(x, y) }
            }
            _ => t,
        }
    }

    /// A call of a method whose result names a parameter's path (`summon[T](using x: T): x.type`)
    /// typed anew with its walked arguments' paths, as scalac's `InlineTyper` types it: the
    /// definition's type widened the path of what stood there.
    fn retype_dependent_call(&mut self, w: &mut Walk, e: TExprId) {
        let (sym, args) = match self.prog.expr(e) {
            TExpr::CallStatic(s, args) | TExpr::CallMethod(_, s, args) => (s, args),
            _ => return,
        };
        // The std's `summon` has its given's own type, as scala-library's `x.type` gives it.
        if self.is_std_summon(sym) {
            if let Some(&given) = self.prog.expr_list(args).first() {
                let t = self.walked_type(w, given);
                if t != ERROR {
                    self.set_walked_type(w, e, t);
                }
            }
            return;
        }
        let sig = self.sig_arc(sym);
        if !self.types.has_paths(sig.ret) {
            return;
        }
        let params: Vec<SymId> = sig.clauses.iter().flat_map(|c| c.params.iter().map(|p| p.sym)).collect();
        if !params.iter().any(|&p| self.mentions_term(sig.ret, p)) {
            return;
        }
        let args: Vec<TExprId> = self.prog.expr_list(args).to_vec();
        let paths: Vec<(SymId, TypeId)> = params.iter().zip(&args).filter_map(|(&p, &a)| self.argument_path(a).map(|path| (p, path))).collect();
        if paths.is_empty() {
            return;
        }
        let targs: Subst = match w.inst.type_args.iter().find(|(c, _)| *c == e) {
            Some((_, ts)) => sig.tparams.iter().copied().zip(ts.iter().copied()).collect(),
            None if sig.tparams.is_empty() => Vec::new(),
            None => return,
        };
        let ret = self.types.subst(sig.ret, &targs);
        let ret = self.subst_paths(ret, &paths);
        self.set_walked_type(w, e, ret);
    }

    /// The signature of a kept call of a member whose signature depends on `this`, seen from the
    /// walked receiver as the retype path sees it from the receiver it types: the member's
    /// owner's type arguments the receiver's, its result in the receiver's terms and the
    /// arguments' paths. What the definition saw where the receiver has no class.
    fn seen_from_walked_receiver(&mut self, w: &Walk, recv: TExprId, sym: SymId, args: &[TExprId], subst: &Subst, stored: (Arc<MethodSig>, Subst, TypeId)) -> (Arc<MethodSig>, Subst, TypeId, Option<TypeId>) {
        let rt = self.walked_type(w, recv);
        let Owner::Class(owner) = self.syms.sym(sym).owner else { return (stored.0, stored.1, stored.2, None) };
        let Some(base) = self.base_type(rt, owner) else { return (stored.0, stored.1, stored.2, None) };
        let prefix = self.prefix_of(recv, rt);
        let owner_subst = self.owner_subst(base);
        let declared = self.sig_arc(sym);
        let sig = self.sig_seen_from(declared, prefix, sym);
        let mut all = owner_subst.clone();
        all.extend(subst.iter().copied());
        let mut ret = self.types.subst(sig.ret, &all);
        if self.types.has_paths(ret) {
            let params = sig.clauses.iter().flat_map(|c| c.params.iter().map(|p| p.sym));
            let paths: Vec<(SymId, TypeId)> = params.zip(args).filter_map(|(p, &a)| self.argument_path(a).map(|path| (p, path))).collect();
            if !paths.is_empty() {
                ret = self.subst_paths(ret, &paths);
            }
        }
        (sig, owner_subst, ret, Some(prefix))
    }

    /// A call of the std's tuple member `sym` (`toList`, `head`, `tail`) on a receiver whose walked
    /// type is a tuple of known elements: the element-wise form the typer gives such a member on
    /// such a receiver (`tuple_member`), as the retype path, typing the call at the argument's
    /// type, gives it; the definition, at a tuple of open length, kept the std's generic inline
    /// method.
    fn tuple_builtin(&mut self, w: &Walk, recv: TExprId, sym: SymId, args: &[TExprId], span: Span) -> Option<(TExprId, TypeId)> {
        let info = self.syms.sym(sym);
        let Owner::Class(c) = info.owner else { return None };
        if !self.b.may_be_tuple_member(info.name) || !self.b.tuple_members.contains(&info.name) || !self.source(self.syms.class(c).file).is_std {
            return None;
        }
        let owner = self.name_str(self.syms.class(c).name);
        if !matches!(owner.as_str(), "Tuple" | "NonEmptyTuple" | "*:") {
            return None;
        }
        let name = info.name;
        let rt = self.walked_type(w, recv);
        // The tuple class of those elements, whose fields the element-wise form reads.
        let elems = self.tuple_elements(rt)?;
        let rt = self.tuple_of(&elems);
        let lists: Vec<super::apply::ArgList> = if args.is_empty() {
            Vec::new()
        } else {
            let typed = args.iter().map(|&a| super::apply::ArgSrc::Typed(a, self.walked_type(w, a))).collect();
            vec![super::apply::ArgList { args: typed, using: false, span }]
        };
        self.tuple_member(recv, rt, name, &lists, span)
    }

    /// A deferred inline member, one the definition selected on an abstract type, dispatched on
    /// the precise type of the walked receiver by override identity, as the retype path
    /// dispatches it on a parameter's proxy (`deferred_implementation`): the member of the
    /// receiver's class that implements it, its owner's type arguments the receiver's, its own
    /// type arguments the call's, its result seen from them. `None` for any other call, and for
    /// a receiver whose class implements nothing, which the expansion reports.
    fn dispatch_deferred_member(&mut self, w: &Walk, recv: TExprId, d: &super::quoted::DeferredInline) -> Option<(SymId, Arc<MethodSig>, Subst, Subst, TypeId)> {
        if !self.is_inline_callee(d.sym) || !self.is_abstract_member(d.sym) {
            return None;
        }
        let own = self.walked_type(w, recv);
        let mut classes = Vec::new();
        self.own_classes(own, &mut classes);
        let implementation = classes.into_iter().find_map(|c| {
            let bases = self.syms.class(c).base_types.clone();
            self.implementation_of(c, &bases, d.sym).filter(|&i| i != d.sym)
        })?;
        let Owner::Class(owner) = self.syms.sym(implementation).owner else { return None };
        let owner_ty = self.base_type(own, owner)?;
        let owner_subst = self.owner_subst(owner_ty);
        let sig = self.sig_arc(implementation);
        let own_targs = d.sig.tparams.iter().map(|p| d.subst.iter().find(|(q, _)| q == p).map_or(ANY, |&(_, t)| t));
        let subst: Subst = sig.tparams.iter().copied().zip(own_targs).collect();
        let mut all = owner_subst.clone();
        all.extend(subst.iter().copied());
        let ret_ty = self.types.subst(sig.ret, &all);
        Some((implementation, sig, owner_subst, subst, ret_ty))
    }

    /// An `if` of the copy: its condition walked, then reduced as scalac's `InlineTyper.typedIf`
    /// and the retype path reduce it. A condition that folds to a constant selects its branch; one
    /// whose type is a constant selects it too, the condition kept before it (for an `inline if`
    /// whatever it is, as the retype path keeps it, for another where it is not idempotent); an
    /// `inline if` without a constant stops with scalac's message; any other keeps both branches,
    /// walked, one of them alone where the interpreter evaluates the condition.
    fn walk_if(&mut self, w: &mut Walk, e: TExprId) -> TExprId {
        let TExpr::If(c, t, els) = self.prog.expr(e) else { unreachable!() };
        let reducible = w.reducible_at(e);
        let c2 = self.walk(w, c);
        // A condition that is a constant, as a tree or by its type (scalac's `ConstantValue`:
        // the type of a path widened to what it stands for, `cond(): C.b.type` over a `final val
        // b: true`), selects its branch; the condition stays before it unless evaluating it
        // twice or not at all is as evaluating it once (`isIdempotentExpr`).
        let reads = self.inline.leaf_reads;
        let constant = match self.fold_constant(c2) {
            Some(LitVal::Bool(taken)) => Some((taken, true)),
            _ => {
                let cond_ty = self.walked_type(w, c2);
                match self.constant_by_type(c2, Some(cond_ty)) {
                    Some(LitVal::Bool(taken)) => Some((taken, false)),
                    _ => None,
                }
            }
        };
        if let Some((taken, by_tree)) = constant {
            let read_leaf = self.inline.leaf_reads != reads;
            let selected = self.select_branch(w, e, taken, t, els);
            if by_tree {
                self.mark_folded_leaf(selected, |_| read_leaf);
            } else if reducible.is_some() {
                self.mark_folded_leaf(selected, |t| t.leaf_near(c2));
            }
            if self.is_idempotent(c2) {
                return selected;
            }
            let stmts = self.prog.stmts.push_slice(&[TStmt::Expr(c2)]);
            let block = self.prog.add(TExpr::Block(stmts, selected));
            let ty = self.walked_type(w, selected);
            self.set_walked_type(w, block, ty);
            return block;
        }
        if let Some(i) = reducible {
            if let ReducibleSource::If { whole, cond } = w.def.reducible_sources[i].clone() {
                let text = self.source_text(self.env.file, cond);
                self.error(whole, format!("Cannot reduce `inline if` because its condition is not a constant value: {}", text));
            }
            w.failed = true;
            let unit = self.prog.add(TExpr::Unit);
            self.set_walked_type(w, unit, ERROR);
            return unit;
        }
        let mark = self.push_tested(c2);
        let t2 = self.walk(w, t);
        self.pop_tested(mark);
        let e2 = els.map(|x| self.walk(w, x));
        if (c2, t2, e2) != (c, t, els) {
            self.prog.exprs[e.idx()] = TExpr::If(c2, t2, e2);
        }
        let ty = self.walked_type(w, e);
        self.mark_taken_branch(c2, e, ty);
        if let Some(LitVal::Bool(taken)) = self.fold_by_eval(c2) {
            let selected = match (taken, e2) {
                (true, _) => t2,
                (false, Some(x)) => x,
                (false, None) => {
                    let unit = self.prog.add(TExpr::Unit);
                    let u = self.b.t_unit;
                    self.set_walked_type(w, unit, u);
                    unit
                }
            };
            self.mark_folded_leaf(selected, |t| t.leaf_near(c2));
            return selected;
        }
        e
    }

    /// A subtree the walk drops unwalked (a branch not taken, a case not selected, a default not
    /// used): counted, and its tests and patterns left out of the output's as a stored body's.
    fn discard(&mut self, w: &mut Walk, root: TExprId) {
        if w.demand.left.remove(root).is_some() {
            let size = w.demand.pruned_size(&self.prog, root);
            debug_assert_eq!(size, self.prog.descendants(root).count() as u64, "the size of the stored tree {} moved", root.0);
            w.pruned += size;
            return;
        }
        w.discarded += self.prog.descendants(root).count() as u64;
        self.store_tests_and_pats(root);
    }

    /// A part of a reduced case, its guard or its body, specialised by what the case's type
    /// variables stand for (`solved`) before anything in it resolves: copied with them where the
    /// copy left it (`demand_roots`), specialised in place where the copy was made whole.
    fn specialised_part(&mut self, w: &mut Walk, part: TExprId, solved: &Subst) -> TExprId {
        if w.demand.is_left(part) {
            return self.demand_in_walk(w, part, Some(solved));
        }
        self.specialise_instance(&mut w.inst, part, solved);
        part
    }

    /// The branch `taken` of the `if` `e` walked, the other discarded unwalked.
    fn select_branch(&mut self, w: &mut Walk, e: TExprId, taken: bool, t: TExprId, els: Option<TExprId>) -> TExprId {
        let (selected, dropped) = if taken { (Some(t), els) } else { (els, Some(t)) };
        if let Some(d) = dropped {
            self.discard(w, d);
        }
        match selected {
            Some(s) => self.walk(w, s),
            None => {
                let unit = self.prog.add(TExpr::Unit);
                let u = self.b.t_unit;
                self.set_walked_type(w, unit, u);
                let _ = e;
                unit
            }
        }
    }

    /// Whether evaluating `e` twice or not at all is as evaluating it once (scalac's
    /// `isIdempotentExpr` over `exprPurity`): a literal, `this`, an object, a reference to a val
    /// (a lazy one included), a parameter or a given, a selection of one on such a receiver; a
    /// primitive or string operation whose application is a constant, which scalac folds to a
    /// literal (`isPureApply`), over such operands; a block of vals and local methods over
    /// those. A conditional, a string conversion of a value (its `toString` runs) and any
    /// other application are not.
    fn is_idempotent(&mut self, e: TExprId) -> bool {
        match self.prog.expr(e) {
            TExpr::Int(_) | TExpr::Long(_) | TExpr::Double(_) | TExpr::Bool(_) | TExpr::Char(_) | TExpr::Str(_) | TExpr::Unit | TExpr::Null | TExpr::This | TExpr::Module(_) => true,
            TExpr::Local(s) | TExpr::Static(s) => self.idempotent_ref(s),
            TExpr::Field(r, s) => self.idempotent_ref(s) && self.is_idempotent(r),
            TExpr::Unary(_, a) | TExpr::ToStr(a, _) => self.fold_constant_as_typed(e).is_some() && self.is_idempotent(a),
            TExpr::Prim(_, a, b) => self.fold_constant_as_typed(e).is_some() && self.is_idempotent(a) && self.is_idempotent(b),
            TExpr::StrConcat(l) => {
                let parts = self.prog.expr_list(l).to_vec();
                self.fold_constant_as_typed(e).is_some() && parts.into_iter().all(|x| self.is_idempotent(x))
            }
            TExpr::Block(stmts, res) => {
                let stmts = self.prog.stmt_list(stmts).to_vec();
                stmts.into_iter().all(|st| match st {
                    TStmt::Val(s, init) => {
                        let info = self.syms.sym(s);
                        info.kind != SymKind::Var && info.mods & crate::ast::mods::MUTABLE == 0 && self.is_idempotent(init)
                    }
                    TStmt::Fun(_) => true,
                    _ => false,
                }) && self.is_idempotent(res)
            }
            _ => false,
        }
    }

    /// Whether `e` is pure in scalac's sense (`exprPurity` at least `Pure`), as an `inline val`
    /// whose constant is its type's has to be (`InlineVals.checkInlineConformant`): as
    /// `is_idempotent`, but for a lazy val's reference, which is idempotent alone; a `final` val
    /// asks for idempotence.
    pub(super) fn is_pure_value(&mut self, e: TExprId) -> bool {
        let lazy = |t: &Self, s: SymId| t.syms.sym(s).mods & crate::ast::mods::LAZY != 0;
        match self.prog.expr(e) {
            TExpr::Int(_) | TExpr::Long(_) | TExpr::Double(_) | TExpr::Bool(_) | TExpr::Char(_) | TExpr::Str(_) | TExpr::Unit | TExpr::Null | TExpr::This | TExpr::Module(_) => true,
            TExpr::Local(s) | TExpr::Static(s) => self.idempotent_ref(s) && !lazy(self, s),
            TExpr::Field(r, s) => self.idempotent_ref(s) && !lazy(self, s) && self.is_pure_value(r),
            TExpr::Unary(_, a) | TExpr::ToStr(a, _) => self.fold_constant_as_typed(e).is_some() && self.is_pure_value(a),
            TExpr::Prim(_, a, b) => self.fold_constant_as_typed(e).is_some() && self.is_pure_value(a) && self.is_pure_value(b),
            TExpr::StrConcat(l) => {
                let parts = self.prog.expr_list(l).to_vec();
                self.fold_constant_as_typed(e).is_some() && parts.into_iter().all(|x| self.is_pure_value(x))
            }
            TExpr::Block(stmts, res) => {
                let stmts = self.prog.stmt_list(stmts).to_vec();
                stmts.into_iter().all(|st| match st {
                    TStmt::Val(s, init) => {
                        let info = self.syms.sym(s);
                        info.kind != SymKind::Var && info.mods & (crate::ast::mods::MUTABLE | crate::ast::mods::LAZY) == 0 && self.is_pure_value(init)
                    }
                    TStmt::Fun(_) => true,
                    _ => false,
                }) && self.is_pure_value(res)
            }
            _ => false,
        }
    }

    /// A reference scalac's `refPurity` finds at least idempotent: a stable member or local.
    fn idempotent_ref(&self, s: SymId) -> bool {
        let info = self.syms.sym(s);
        let stable = matches!(info.kind, SymKind::Val | SymKind::Param | SymKind::Given | SymKind::Object(_) | SymKind::EnumValue(_));
        let inline_param = info.kind == SymKind::Param && info.mods & crate::ast::mods::INLINE != 0;
        stable && info.mods & crate::ast::mods::MUTABLE == 0 && !inline_param && !info.by_name
    }

    /// An `inline match` of the copy: the selector walked, then the cases tried in order on its
    /// static type, a proxy's type being its argument's, never its value (`InlineReducer`): a
    /// pattern the type matches binds its terms (a val over the scrutinee's part, or the constant
    /// it is) and its type variables, its guard folded with the bindings (`true` selects the
    /// case, `false` goes on, no constant stops), the selected case specialised by what its
    /// variables stand for and walked, the others discarded; no case left is scalac's `cannot
    /// reduce inline match`.
    fn reduce_inline_match(&mut self, w: &mut Walk, e: TExprId) -> TExprId {
        let TExpr::Match(scrut, cases) = self.prog.expr(e) else { unreachable!() };
        let source = w.reducible_at(e).expect("a reducible match's source");
        let s2 = self.walk(w, scrut);
        let sty = self.walked_type(w, s2);
        let sty = self.solve_in(sty);
        let constant = self.fold_constant(s2).or_else(|| self.fold_type(sty));
        let erased = self.inline.erased.take() == Some(s2);
        let mut scrutinee = WalkScrutinee { expr: s2, ty: sty, constant, erased, bound: None };
        // The cases are the copy's, which nothing the reduction makes changes.
        let (first, n) = (cases.range().start, cases.len as usize);
        // The patterns and guards are the reduction's, no part of what it gives.
        self.store_case_pats(cases);
        for i in 0..n {
            if let Some(g) = self.prog.cases[first + i].guard.filter(|&g| !w.demand.is_left(g)) {
                self.store_tests_and_pats(g);
            }
        }
        for i in 0..n {
            if w.failed {
                break;
            }
            let c = self.prog.cases[first + i];
            let mut stmts: Vec<TStmt> = Vec::new();
            let mut solved: Subst = Vec::new();
            let mut bound_args: Vec<SymId> = Vec::new();
            scrutinee.bound = None;
            // Each case an attempt: one that does not match goes whole.
            let mark = self.attempt();
            let mut stuck = false;
            let mut matched = self.reduce_pattern(w, c.pat, &mut scrutinee, &mut stmts, &mut solved, &mut bound_args);
            // The case's binders are in scope for its guard and body, a call the walk hands to
            // the retype path included (a class it makes captures them).
            self.push_scope();
            if matched {
                self.bind_case(c.pat, &stmts);
                if let Some(g) = c.guard {
                    let g = self.specialised_part(w, g, &solved);
                    // A guard is the reduction's, no part of what the expansion gives.
                    self.store_tests_and_pats(g);
                    let g2 = self.walk(w, g);
                    // An error of the guard's expansion stops the reduction with its message,
                    // as scalac's typing of the guard reports it: no speculative miss.
                    if w.failed {
                        self.pop_scope();
                        self.close_without_constraints(mark);
                        for s in bound_args {
                            self.inline.args.remove(&s);
                        }
                        for j in i..n {
                            let rest = self.prog.cases[first + j].body;
                            self.discard(w, rest);
                        }
                        break;
                    }
                    // scalac's `ConstantValue` of the guard: its tree, or its type (`yes()` of
                    // `def yes(): true`).
                    let gty = self.walked_type(w, g2);
                    match self.constant_tree_value(g2, Some(gty)) {
                        Some(LitVal::Bool(b)) => matched = b,
                        _ => {
                            stuck = true;
                            matched = false;
                        }
                    }
                }
            }
            if !matched {
                self.pop_scope();
                self.retract(mark);
                for s in bound_args {
                    self.inline.args.remove(&s);
                }
                self.discard(w, c.body);
                if stuck {
                    for j in i + 1..n {
                        let later = self.prog.cases[first + j].body;
                        self.discard(w, later);
                    }
                    break;
                }
                continue;
            }
            self.close(mark);
            for j in i + 1..n {
                let later = self.prog.cases[first + j].body;
                self.discard(w, later);
            }
            let body = self.specialised_part(w, c.body, &solved);
            let body = self.walk(w, body);
            self.pop_scope();
            for s in bound_args {
                self.inline.args.remove(&s);
            }
            let ty = self.walked_type(w, body);
            if stmts.is_empty() {
                return body;
            }
            let l = self.prog.stmts.push_slice(&stmts);
            let block = self.prog.add(TExpr::Block(l, body));
            self.set_walked_type(w, block, ty);
            return block;
        }
        if !w.failed {
            if let ReducibleSource::Match { whole, scrut: at, cases: written } = w.def.reducible_sources[source].clone() {
                let shown = self.show(sty);
                let mut msg = format!("cannot reduce inline match with\n scrutinee:  {} : {}\n patterns :", self.source_text(self.env.file, at), shown);
                for (i, (pat, guard)) in written.iter().enumerate() {
                    msg.push_str(if i == 0 { "  case " } else { "\n             case " });
                    msg.push_str(&self.source_text(self.env.file, *pat));
                    if let Some(g) = guard {
                        msg.push_str(" if ");
                        msg.push_str(&self.source_text(self.env.file, *g));
                    }
                }
                self.error(whole, msg);
            }
        }
        w.failed = true;
        let unit = self.prog.add(TExpr::Unit);
        self.set_walked_type(w, unit, ERROR);
        unit
    }

    /// A `summonFrom` of the copy, reduced as scalac's `InlineReducer` reduces it: a search at
    /// the call site for each case's pattern type in source order (its type in the copy's
    /// terms), the scopes of the expanded bodies around the site; a search that finds nothing
    /// goes on to the next case, an ambiguous one reports and stops the reduction, a found given
    /// is bound once to the case's binder, a given in scope for the case's guard and body, whose
    /// type is the given's own; a wildcard is taken as it stands. A guard reduces as an inline
    /// match's does (`InlineReducer.reduceCase`): a constant `false` gives way to the next case,
    /// one that is no constant stops the reduction. The selected case's body is walked, the
    /// others discarded; no case left is `cannot reduce summonFrom`.
    fn reduce_summon_from(&mut self, w: &mut Walk, e: TExprId) -> TExprId {
        let TExpr::Match(_, cases) = self.prog.expr(e) else { unreachable!() };
        let ReducibleSource::SummonFrom { whole, cases: written } = w.def.reducible_sources[w.reducible_at(e).expect("a summonFrom's source")].clone() else { unreachable!() };
        let clauses: Vec<TCase> = self.prog.case_list(cases).to_vec();
        // The patterns and guards are the reduction's, no part of what it gives.
        self.store_pats_and_tests(clauses.iter().map(|c| c.pat).collect(), Vec::new());
        for c in &clauses {
            if let Some(g) = c.guard.filter(|&g| !w.demand.is_left(g)) {
                self.store_tests_and_pats(g);
            }
        }
        for (i, c) in clauses.iter().enumerate() {
            let (binder, pattern_ty) = match self.prog.pats[c.pat.idx()] {
                TPat::Wildcard => (None, None),
                TPat::Test(_, t, _) => (None, Some(t)),
                TPat::Bind(s, Some(inner)) => match self.prog.pats[inner.idx()] {
                    TPat::Test(_, t, _) => (Some(s), Some(t)),
                    _ => (Some(s), None),
                },
                _ => (None, None),
            };
            let found = match pattern_ty {
                Some(t) => {
                    let outer = std::mem::replace(&mut self.profile.summon, super::profile::Origin::SummonFrom);
                    let found = self.summon_at_site_telling(t, whole);
                    self.profile.summon = outer;
                    match found {
                        Ok(found) => Some(found),
                        Err(ambiguous) => {
                            if ambiguous {
                                let shown = self.show(t);
                                let msg = self.given_ambiguity.take().unwrap_or_else(|| format!("ambiguous given instances for {}", shown));
                                self.error(whole, msg);
                                w.failed = true;
                                for rest in &clauses[i..] {
                                    self.discard(w, rest.body);
                                }
                                let unit = self.prog.add(TExpr::Unit);
                                self.set_walked_type(w, unit, ERROR);
                                return unit;
                            }
                            self.discard(w, c.body);
                            continue;
                        }
                    }
                }
                None => None,
            };
            self.push_scope();
            let mut stmts: Vec<TStmt> = Vec::new();
            if let Some(s) = binder {
                let (given, ty) = found.unwrap_or_else(|| {
                    let unit = self.prog.add(TExpr::Unit);
                    (unit, self.b.t_unit)
                });
                let mut sig = (*self.sig_of(s)).clone();
                sig.ret = ty;
                self.syms.sym_mut(s).sig = Some(Arc::new(sig));
                w.binder_types.insert(s, ty);
                stmts.push(TStmt::Val(s, given));
                self.bind_walked(s);
                self.bind_given(s);
            }
            if let Some(g) = c.guard {
                let g = self.specialised_part(w, g, &Vec::new());
                self.store_tests_and_pats(g);
                let g2 = self.walk(w, g);
                let gty = self.walked_type(w, g2);
                let taken = match self.constant_tree_value(g2, Some(gty)) {
                    Some(LitVal::Bool(b)) if !w.failed => Some(b),
                    _ => None,
                };
                if taken != Some(true) {
                    self.pop_scope();
                    if taken == Some(false) {
                        self.discard(w, c.body);
                        continue;
                    }
                    for rest in &clauses[i..] {
                        self.discard(w, rest.body);
                    }
                    // An error of the guard's expansion stops the reduction with its message, as
                    // scalac's typing of the guard reports it.
                    if w.failed {
                        let unit = self.prog.add(TExpr::Unit);
                        self.set_walked_type(w, unit, ERROR);
                        return unit;
                    }
                    break;
                }
            }
            for later in &clauses[i + 1..] {
                self.discard(w, later.body);
            }
            let body = self.walk(w, c.body);
            self.pop_scope();
            let ty = self.walked_type(w, body);
            if stmts.is_empty() {
                return body;
            }
            let l = self.prog.stmts.push_slice(&stmts);
            let block = self.prog.add(TExpr::Block(l, body));
            self.set_walked_type(w, block, ty);
            return block;
        }
        let mut msg = "cannot reduce summonFrom with\n patterns :".to_string();
        for (i, &(at, guard)) in written.iter().enumerate() {
            msg.push_str(if i == 0 { "  case " } else { "\n             case " });
            msg.push_str(&self.source_text(self.env.file, at));
            if let Some(g) = guard {
                msg.push_str(" if ");
                msg.push_str(&self.source_text(self.env.file, g));
            }
        }
        self.error(whole, msg);
        w.failed = true;
        let unit = self.prog.add(TExpr::Unit);
        self.set_walked_type(w, unit, ERROR);
        unit
    }

    /// The locals a reduced case binds, its pattern's binders and the scrutinee's val among
    /// `stmts`, in the scope of its guard and body.
    fn bind_case(&mut self, pat: TPatId, stmts: &[TStmt]) {
        let mut binders = Vec::new();
        self.walked_pattern_binders(pat, &mut binders);
        for st in stmts {
            if let TStmt::Val(s, _) = *st {
                if !binders.contains(&s) {
                    binders.push(s);
                }
            }
        }
        for b in binders {
            self.bind_walked(b);
        }
    }

    /// The scrutinee's value as a stable reference, bound to a local of `stmts` on first use.
    fn walked_scrutinee_ref(&mut self, s: &mut WalkScrutinee, stmts: &mut Vec<TStmt>, span: Span) -> TExprId {
        if let Some(b) = s.bound {
            return self.copy_expr(b);
        }
        if s.erased {
            return self.prog.add(TExpr::Unit);
        }
        if self.is_stable(s.expr) {
            return self.copy_expr(s.expr);
        }
        let local = self.indexed_local("scrutinee", stmts.len() as u32, s.ty, span);
        stmts.push(TStmt::Val(local, s.expr));
        let r = self.prog.add(TExpr::Local(local));
        s.bound = Some(r);
        r
    }

    /// Whether the pattern `p` of a copied case matches the scrutinee statically, as the retype
    /// path's `inline_pattern` decides it: its binders bound (the constant they stand for, or a
    /// val of `stmts`), its type variables' solutions in `solved`.
    fn reduce_pattern(&mut self, w: &mut Walk, p: TPatId, s: &mut WalkScrutinee, stmts: &mut Vec<TStmt>, solved: &mut Subst, bound_args: &mut Vec<SymId>) -> bool {
        let mark = self.diags.items.len();
        let matched = self.reduce_pattern_now(w, p, s, stmts, solved, bound_args);
        if !matched {
            self.drop_reported_since(mark);
        }
        matched
    }

    fn reduce_pattern_now(&mut self, w: &mut Walk, p: TPatId, s: &mut WalkScrutinee, stmts: &mut Vec<TStmt>, solved: &mut Subst, bound_args: &mut Vec<SymId>) -> bool {
        let span = Span::default();
        match self.prog.pats[p.idx()] {
            TPat::Wildcard => true,
            TPat::Bind(sym, inner) => {
                let ty = match inner {
                    Some(i) => {
                        if !self.reduce_pattern(w, i, s, stmts, solved, bound_args) {
                            return false;
                        }
                        self.walked_pattern_type(w, i, s.ty, solved)
                    }
                    None => s.ty,
                };
                let mut sig = (*self.sig_of(sym)).clone();
                sig.ret = ty;
                self.syms.sym_mut(sym).sig = Some(Arc::new(sig));
                w.binder_types.insert(sym, ty);
                // A binder over a constant stands for the constant, as scalac's reduced
                // projection does, so that a guard on it folds.
                match s.constant {
                    Some(v) => {
                        let (lit, _) = self.constant(v);
                        self.inline.args.insert(sym, InlineArg { expr: lit, ty, source: None });
                        bound_args.push(sym);
                    }
                    None => {
                        let value = self.walked_scrutinee_ref(s, stmts, span);
                        stmts.push(TStmt::Val(sym, value));
                    }
                }
                true
            }
            TPat::Test(_, pt, inner) => {
                // A wildcard under the test reads nothing of the type the scrutinee narrows to.
                let wildcard = matches!(self.prog.pats[inner.idx()], TPat::Wildcard);
                let Some(narrowed) = self.walked_type_pattern(w, pt, s.ty, solved, !wildcard) else { return false };
                let narrowed = if wildcard { narrowed } else { self.narrowed_to(s.ty, narrowed) };
                let mut inner_scrutinee = WalkScrutinee { expr: s.expr, ty: narrowed, constant: s.constant, erased: s.erased, bound: s.bound };
                let ok = self.reduce_pattern(w, inner, &mut inner_scrutinee, stmts, solved, bound_args);
                s.bound = inner_scrutinee.bound;
                ok
            }
            TPat::Equals(x, _) => {
                if matches!(self.prog.expr(x), TExpr::Null) {
                    return matches!(self.prog.expr(s.expr), TExpr::Null);
                }
                match (self.prog.expr(x), self.prog.expr(s.expr)) {
                    (TExpr::Module(a), TExpr::Module(b)) => a == b,
                    (TExpr::Module(_), _) => {
                        let Some(pty) = w.inst.types.get(x) else { return false };
                        let mark = self.snapshot();
                        let ok = self.is_sub(s.ty, pty);
                        self.rollback(mark);
                        ok
                    }
                    _ => match (self.fold_constant(x), s.constant) {
                        (Some(a), Some(b)) => a == b || super::inline::same_number(a, b),
                        _ => false,
                    },
                }
            }
            TPat::Alt(alts) => {
                let alts = self.prog.pat_list(alts).to_vec();
                alts.into_iter().any(|a| self.reduce_pattern(w, a, s, stmts, solved, bound_args))
            }
            TPat::Class(c, _, _, subs) => {
                let subs = self.prog.pat_list(subs).to_vec();
                if self.is_tuple_class(c) {
                    let sty = self.dealias(s.ty);
                    let Type::Class(k, args) = self.types.get(sty) else { return false };
                    if !self.is_tuple_class(k) || self.types.items(args).len() != subs.len() {
                        return false;
                    }
                    return self.reduce_fields(w, k, args, &subs, s, stmts, solved, bound_args, span);
                }
                let (class_ty, _) = self.instantiate_pattern_class(c, s.ty);
                let mark = self.snapshot();
                if !self.is_sub(s.ty, class_ty) {
                    self.rollback(mark);
                    return false;
                }
                let class_ty = self.zonk(class_ty);
                let Type::Class(_, args) = self.types.get(class_ty) else { return false };
                if self.syms.class(c).ctor_syms.first().map_or(0, |f| f.len()) != subs.len() {
                    return false;
                }
                self.reduce_fields(w, c, args, &subs, s, stmts, solved, bound_args, span)
            }
            TPat::Unapply(..) | TPat::Seq(..) => false,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn reduce_fields(&mut self, w: &mut Walk, c: ClassId, args: TList, subs: &[TPatId], s: &mut WalkScrutinee, stmts: &mut Vec<TStmt>, solved: &mut Subst, bound_args: &mut Vec<SymId>, span: Span) -> bool {
        let fields: Vec<SymId> = self.syms.class(c).ctor_syms.concat();
        self.settle_class(c);
        let subst: Subst = self.syms.class(c).tparams.iter().copied().zip(self.types.items(args).iter().copied()).collect();
        for (i, &sub) in subs.iter().enumerate() {
            let field = fields[i];
            let fty = self.sig_of(field).ret;
            let fty = self.types.subst(fty, &subst);
            let constant = match self.prog.expr(s.expr) {
                TExpr::New(k, items) if k == c => {
                    let item = self.prog.expr_list(items)[i];
                    self.fold_constant(item)
                }
                _ => self.fold_type(fty),
            };
            if matches!(self.prog.pats[sub.idx()], TPat::Wildcard) {
                continue;
            }
            let value = self.walked_scrutinee_ref(s, stmts, span);
            let expr = self.prog.add(TExpr::Field(value, field));
            let mut elem = WalkScrutinee { expr, ty: fty, constant, erased: s.erased, bound: None };
            if !self.reduce_pattern(w, sub, &mut elem, stmts, solved, bound_args) {
                return false;
            }
        }
        true
    }

    /// The type a pattern narrows the scrutinee to, as `inline_pattern_type` gives it.
    fn walked_pattern_type(&mut self, w: &mut Walk, p: TPatId, sty: TypeId, solved: &Subst) -> TypeId {
        match self.prog.pats[p.idx()] {
            TPat::Test(_, pt, _) => {
                let pt = self.types.subst(pt, solved);
                let mut scratch = Vec::new();
                match self.walked_type_pattern(w, pt, sty, &mut scratch, true) {
                    Some(t) => self.narrowed_to(sty, t),
                    None => sty,
                }
            }
            TPat::Class(c, _, _, _) if !self.is_tuple_class(c) => {
                let (t, _) = self.instantiate_pattern_class(c, sty);
                self.zonk(t)
            }
            _ => sty,
        }
    }

    /// Whether the scrutinee's type `sty` matches the pattern type `pt`, whose variables are the
    /// copy's fresh ones, as the retype path's `type_pattern_matches` decides it: a variable
    /// stands for what it meets, a cons or a tuple element by element, any other through the
    /// subtyping with the variables as unknowns. The variables' solutions go to `solved`; what
    /// answers is the pattern type with them where the caller reads it (`typed`), the scrutinee's
    /// type otherwise.
    fn walked_type_pattern(&mut self, w: &Walk, pt: TypeId, sty: TypeId, solved: &mut Subst, typed: bool) -> Option<TypeId> {
        let pt = self.types.subst(pt, solved);
        if let Type::Param(p) = self.types.get(pt) {
            if w.pattern_vars.contains(&p) {
                solved.push((p, sty));
                return Some(sty);
            }
        }
        let structural = match self.types.get(pt) {
            Type::Class(c, _) => self.is_tuple_class(c) || Some(c) == self.b.cons_tuple,
            _ => false,
        };
        if structural {
            let head = self.dealias(sty);
            if let Type::Inter(a, b) = self.types.get(head) {
                let mark = solved.len();
                if let Some(t) = self.walked_type_pattern(w, pt, a, solved, typed) {
                    return Some(t);
                }
                solved.truncate(mark);
                return self.walked_type_pattern(w, pt, b, solved, typed);
            }
        }
        if let Type::Class(c, pargs) = self.types.get(pt) {
            let pargs: Vec<TypeId> = self.types.items(pargs).to_vec();
            if Some(c) == self.b.cons_tuple && pargs.len() == 2 {
                let scrutinee = self.dealias(sty);
                let Type::Class(k, targs) = self.types.get(scrutinee) else { return None };
                if !self.is_tuple_class(k) {
                    return None;
                }
                let elems: Vec<TypeId> = self.types.items(targs).to_vec();
                let tail_ty = self.tuple_of(&elems[1..]);
                let head = self.walked_type_pattern(w, pargs[0], elems[0], solved, typed)?;
                let tail = self.walked_type_pattern(w, pargs[1], tail_ty, solved, typed)?;
                if !typed {
                    return Some(sty);
                }
                let mut all = vec![head];
                let tail = self.dealias(tail);
                if let Type::Class(tc, targs) = self.types.get(tail) {
                    if self.is_tuple_class(tc) {
                        all.extend_from_slice(&self.types.items(targs).to_vec());
                    }
                }
                return Some(self.tuple_type(&all));
            }
            if self.is_tuple_class(c) && pargs.len() >= 2 {
                let scrutinee = self.dealias(sty);
                let Type::Class(k, targs) = self.types.get(scrutinee) else { return None };
                if !self.is_tuple_class(k) || self.types.items(targs).len() != pargs.len() {
                    return None;
                }
                let elems: Vec<TypeId> = self.types.items(targs).to_vec();
                let mut matched = Vec::with_capacity(elems.len());
                for (&p, e) in pargs.iter().zip(elems) {
                    matched.push(self.walked_type_pattern(w, p, e, solved, typed)?);
                }
                return Some(if typed { self.tuple_type(&matched) } else { sty });
            }
        }
        let open: Vec<TParamId> = w.pattern_vars.iter().copied().filter(|&p| self.mentions_tparam_of(pt, Some(&[p]))).collect();
        let vars: Subst = open.iter().map(|&p| (p, self.fresh_var())).collect();
        let with_vars = if vars.is_empty() { pt } else { self.types.subst(pt, &vars) };
        let snapshot = self.snapshot();
        let ok = with_vars != ERROR && self.is_sub(sty, with_vars);
        if !ok {
            self.rollback(snapshot);
            return None;
        }
        for &(p, v) in &vars {
            let t = self.solve_if_var(v);
            solved.push((p, t));
        }
        Some(if typed { self.zonk(with_vars) } else { sty })
    }
}

/// The scrutinee of a reduced match as its patterns see it.
struct WalkScrutinee {
    expr: TExprId,
    ty: TypeId,
    constant: Option<LitVal>,
    erased: bool,
    bound: Option<TExprId>,
}

/// `TEQ_INLINE_MEASURE=copy`, a measurement's setting: a call the walk takes is copied as the walk
/// would copy it and then expanded by the retype path, so that what the copying costs is measured
/// apart from what the walk costs (`Worker::copy_for_measure`).
pub fn measures_copy() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var("TEQ_INLINE_MEASURE").is_ok_and(|v| v == "copy"))
}

/// `TEQ_INLINE_COUNTS=1`: the calls of inline methods the expansion by substitution took and
/// those that fell back to the retype path by reason, the nodes the walks copied and visited,
/// the nodes of branches and defaults copied then discarded, and the records the definition check
/// made, printed at the end of the build; `TEQ_INLINE_COUNTS=<file>` appends the same line to the
/// file. Apart from the definition census (`TEQ_INLINE_CENSUS`), whose copies are not counted.
pub mod counts {
    use super::Fallback;
    use std::collections::BTreeMap;
    use std::sync::{Mutex, OnceLock};

    #[derive(Default)]
    struct Counts {
        substituted: u64,
        fallbacks: BTreeMap<Fallback, u64>,
        bridges: u64,
        copied: u64,
        visited: u64,
        discarded: u64,
        records: u64,
        retained: u64,
        superseded: u64,
        pruned: u64,
    }

    static COUNTS: Mutex<Option<Counts>> = Mutex::new(None);

    extern "C" {
        fn atexit(f: extern "C" fn()) -> i32;
    }

    extern "C" fn at_exit() {
        report();
    }

    /// The setting, read once; the report is made as the process exits, whichever way it does.
    fn setting() -> Option<&'static str> {
        static SETTING: OnceLock<Option<String>> = OnceLock::new();
        SETTING
            .get_or_init(|| {
                let v = std::env::var("TEQ_INLINE_COUNTS").ok().filter(|v| !v.is_empty() && v != "0");
                if v.is_some() {
                    // SAFETY: registers a function of this module to run at the process's exit.
                    unsafe { atexit(at_exit) };
                }
                v
            })
            .as_deref()
    }

    #[inline]
    pub fn on() -> bool {
        setting().is_some()
    }

    fn with(f: impl FnOnce(&mut Counts)) {
        if !on() {
            return;
        }
        let mut c = COUNTS.lock().unwrap_or_else(|e| e.into_inner());
        f(c.get_or_insert_with(Counts::default));
    }

    pub fn substituted() {
        with(|c| c.substituted += 1);
    }

    /// A call that took the retype path, `inside` a walk (a bridge) or not.
    pub fn fell_back(reason: Fallback, inside: bool) {
        with(|c| {
            *c.fallbacks.entry(reason).or_insert(0) += 1;
            c.bridges += inside as u64;
        });
    }

    pub fn walked(copied: u64, visited: u64, discarded: u64) {
        with(|c| {
            c.copied += copied;
            c.visited += visited;
            c.discarded += discarded;
        });
    }

    /// Stored nodes of parts a walk never demanded, so never copied.
    pub fn pruned(n: u64) {
        with(|c| c.pruned += n);
    }

    pub fn record_made() {
        with(|c| c.records += 1);
    }

    pub fn retained(canonical: u64, superseded: u64) {
        with(|c| {
            c.retained += canonical;
            c.superseded += superseded;
        });
    }

    /// The line of the build's counts, once.
    fn report() {
        let Some(setting) = setting() else { return };
        let c = COUNTS.lock().unwrap_or_else(|e| e.into_inner()).take().unwrap_or_default();
        let fell: u64 = c.fallbacks.values().sum();
        let mut line = format!(
            "inline counts: substituted {} fell back {} (bridges {}) copied {} visited {} discarded {} records {} retained {} superseded {} pruned {}",
            c.substituted, fell, c.bridges, c.copied, c.visited, c.discarded, c.records, c.retained, c.superseded, c.pruned
        );
        for (reason, n) in &c.fallbacks {
            line.push_str(&format!("; {}: {}", reason.name(), n));
        }
        if setting == "1" {
            eprintln!("{}", line);
        } else if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(setting) {
            use std::io::Write;
            // One write, so that the lines of builds appending at once do not interleave.
            line.push('\n');
            let _ = f.write_all(line.as_bytes());
        }
    }
}

/// The nodes, patterns and binders of the stored inline bodies, whose capture records a build
/// drops uncounted: it checks a body in each worker that calls the
/// method, so that their number would move with the workers' (`compact_capture`).
#[derive(Default)]
pub(super) struct StoredKeys {
    exprs: FxMap<TExprId, ()>,
    pats: FxMap<TPatId, ()>,
    binders: FxMap<SymId, ()>,
    classes: FxMap<ClassId, ()>,
}

impl StoredKeys {
    pub(super) fn holds(&self, k: crate::tir::capture::Key) -> bool {
        use crate::tir::capture::Key;
        match (k, k.expr()) {
            (_, Some(e)) => self.exprs.contains_key(&e),
            (Key::Pat(p), _) => self.pats.contains_key(&p),
            (Key::Local(s), _) => self.binders.contains_key(&s),
            (Key::Class(c), _) => self.classes.contains_key(&c),
            _ => false,
        }
    }
}

impl<'a> Worker<'a> {
    /// The keys `StoredKeys` holds.
    pub(super) fn stored_inline_keys(&self) -> StoredKeys {
        let mut keys = StoredKeys::default();
        for def in self.inline_definitions.local.values().chain(&self.superseded_definitions) {
            for root in self.stored_roots_of(def) {
                keys.exprs.extend(self.prog.descendants(root).map(|e| (e, ())));
            }
            keys.binders.extend(def.binders.iter().map(|&b| (b, ())));
        }
        keys.pats.extend(self.prog.stored_pats.keys().map(|&p| (p, ())));
        for &k in &self.check_capture_keys {
            use crate::tir::capture::Key;
            match (k, k.expr()) {
                (_, Some(e)) => {
                    keys.exprs.insert(e, ());
                }
                (Key::Pat(p), _) => {
                    keys.pats.insert(p, ());
                }
                (Key::Local(s), _) => {
                    keys.binders.insert(s, ());
                }
                (Key::Class(c), _) => {
                    keys.classes.insert(c, ());
                }
                _ => {}
            }
        }
        keys
    }

    /// How many records' indexes the worker holds (`InlineState::stored_indexes`), for a
    /// session's `stats`.
    pub fn stored_index_count(&self) -> usize {
        self.inline.stored_indexes.len()
    }

    /// The records of checked bodies the build holds after the bodies, for `counts`: the
    /// program's, and those a merge replaced that a capture's census still reads.
    pub fn count_retained_records(&self) {
        let canonical = self.inline_definitions.local.values().filter(|d| d.body.is_some()).count();
        let superseded = self.superseded_definitions.iter().filter(|d| d.body.is_some()).count();
        counts::retained(canonical as u64, superseded as u64);
    }
}
