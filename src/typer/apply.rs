use super::profile::{About, Kind, Outcome};
use super::resolve::TermRef;
use super::{Worker, Undo};
use crate::ast::{Expr, ExprId, ListRef, Pat, PatId, TyExpr, TyExprId};
use crate::intern::Name;
use crate::names;
use crate::source::{FileId, Span};
use crate::symbols::*;
use crate::tir::*;
use crate::tir::capture::{Form, Wrap};
use crate::types::*;
use std::sync::Arc;

#[derive(Clone, Copy)]
pub enum ForRest {
    Rest { idx: u32, end: u32, body: ExprId, is_yield: bool },
    Guard(ExprId),
    /// Binds the value definitions in `idx..end` and packs every variable in scope.
    Pack { idx: u32, end: u32 },
    /// The `withFilter` predicate of a `case` generator: whether the pattern matches.
    Filter,
}

#[derive(Clone, Copy)]
pub enum ForBinder {
    Pat(PatId),
    /// The pattern of a `case` generator, which may fail; the filter has run before it.
    CasePat(PatId),
    /// Variables restored from `Worker::for_packs`.
    Unpack(u32),
}

#[derive(Clone, Copy)]
pub enum ArgSrc {
    Ast(ExprId),
    /// An argument that is written before the receiver but evaluated after it, such as the left
    /// operand of a right-associative operator: bound to a temporary unless it is stable.
    Hoisted(ExprId),
    Typed(TExprId, TypeId),
    /// A named argument that was typed before the method it goes to was known.
    Named(Name, TExprId, TypeId),
    ForLambda(ForBinder, ForRest),
}

impl ArgSrc {
    pub(super) fn ast(self) -> Option<ExprId> {
        match self {
            ArgSrc::Ast(e) | ArgSrc::Hoisted(e) => Some(e),
            _ => None,
        }
    }
}

pub struct ArgList {
    pub args: Vec<ArgSrc>,
    pub using: bool,
    pub span: Span,
}

impl Clone for ArgList {
    fn clone(&self) -> ArgList {
        ArgList { args: self.args.clone(), using: self.using, span: self.span }
    }

    /// Into the list's own buffer, which a kept copy of a call's lists reuses.
    fn clone_from(&mut self, source: &ArgList) {
        self.args.clone_from(&source.args);
        self.using = source.using;
        self.span = source.span;
    }
}

/// An argument of a member's first list in the test of `Worker::boundary_attempt`: as written
/// (`written`, a named one's `n = v`, and `value`, its `v`) or typed already, the parameter it
/// goes to, whether it is a sequence spliced into it (`xs*`), its name where named, its place in
/// the list and whether it is an operand evaluated ahead of the receiver.
struct BoundaryArg {
    written: Option<ExprId>,
    value: Option<ExprId>,
    pretyped: Option<(TExprId, TypeId)>,
    param: usize,
    splice: bool,
    named: Option<Name>,
    src: usize,
    hoisted: bool,
    /// Written bare between the list's delimiters, nothing but blanks and parentheses around its
    /// expression: no braces or comments, which the parser drops (`{ (e) }` is read as `e`).
    bare: bool,
}

/// The extensions of a name beyond the lexical scope: the givens with one, the context's the
/// first `contextual`, and the implicit scope's objects', whose inherited ones `ext_modules`
/// records from `modules_mark` on.
struct ExtensionsBeyond {
    givens: Vec<(super::implicits::GivenRef, TypeId)>,
    contextual: usize,
    /// The nesting level of each of the first `contextual` givens.
    levels: Vec<u32>,
    companions: Vec<SymId>,
    modules_mark: usize,
    /// The profile's event of the lookup, opened before the gathering (`--profile`).
    prof: Option<usize>,
}

/// The givens that provide an extension, ranked (`Worker::rank_extension_givens`): the one
/// preferred, with its probe where one was made (`Worker::given_extension_for`), or two of one rank.
enum Ranked {
    None,
    One(usize, Option<(MethodCall, super::state::SetAside)>),
    Ambiguous(usize, usize),
}

/// What the lexical scope gives an extension's selection, dotty's `tryExtension` (Typer.scala
/// 4280 to 4311).
enum Lexical {
    /// The candidate found by name, its prefix (`prefix_holds`) not tried yet.
    Found(MethodCall),
    /// The candidate whose prefix held, what the prefix resolved set aside for its application.
    Selected(MethodCall),
    /// No candidate's prefix held: the first that failed, if any, which is applied for its errors
    /// where the implicit scope has nothing either (dotty commits `failures.head`, 4304, and keeps
    /// its error as the search's, 4326 to 4332).
    Failed(Option<MethodCall>),
    /// The prefixes of two imports' candidates held (4306), each with the object it imports from:
    /// no selection, the implicit scope searched, and failing that the ambiguity reported.
    Ambiguous((MethodCall, Option<ClassId>), (MethodCall, Option<ClassId>)),
}

/// What a member's application under `apply_member_or_extension` typed of the arguments it was
/// given, as dotty's `FunProto` caches them (`cacheTypedArg`, ProtoTypes.scala 471): its retry
/// on the qualifier adapts these typings and types none of the arguments again, so that an
/// expansion, a macro's among them, runs once.
pub struct ArgCache {
    file: crate::source::FileId,
    /// The argument lists as written, for the retry, flat: every list's arguments in one buffer,
    /// and per list where its arguments begin, whether it is a using one and its span
    /// (`written`); a call that types keeps the buffers for the next (`Worker::arg_caches`).
    args: Vec<ArgSrc>,
    lists: Vec<(u32, bool, Span)>,
    /// Set while the first argument list that is not a using one is typed (`type_clause_args`),
    /// whose typings alone the retry reads: it is tried as written and as its tupled dual.
    active: bool,
    /// Whether that list has been reached.
    done: bool,
    /// Whether the application stopped at that list, which failed (`apply_method_in_now`): the
    /// lists after it are typed once, by the retry that takes the application over or alone.
    stopped: bool,
    typings: Vec<ArgTyping>,
    /// The type mismatches the application reported, by diagnostic index and span with the
    /// type each required (`Worker::mismatches`), filled where it failed.
    mismatches: Vec<(usize, Span, TypeId)>,
    /// The application's attempt: what a clean typing of an argument
    /// writes of the rarely written journals is kept through its set-aside and the retry's
    /// tries (`state::retained_typing_begin`).
    attempt: Option<super::state::Mark>,
    /// The count of type variables where the application began (`Taken::vars`).
    vars: u32,
    /// The depth of applications (`Worker::app_depth`) of the member's application, which the
    /// first application under the cache claims: an application nested in a later list's
    /// argument types its own later lists for their errors.
    depth: u32,
}

impl ArgCache {
    /// Keeps `lists` as written.
    #[inline]
    fn keep(&mut self, lists: &[ArgList]) {
        self.args.clear();
        self.lists.clear();
        for l in lists {
            self.lists.push((self.args.len() as u32, l.using, l.span));
            self.args.extend_from_slice(&l.args);
        }
    }

    /// The lists as written.
    fn written(&self) -> Vec<ArgList> {
        let mut out = Vec::with_capacity(self.lists.len());
        for (i, &(start, using, span)) in self.lists.iter().enumerate() {
            let end = self.lists.get(i + 1).map_or(self.args.len(), |l| l.0 as usize);
            out.push(ArgList { args: self.args[start as usize..end].to_vec(), using, span });
        }
        out
    }
}

impl Default for ArgCache {
    fn default() -> ArgCache {
        ArgCache { file: crate::source::NO_FILE, args: Vec::new(), lists: Vec::new(), active: false, done: false, stopped: false, typings: Vec::new(), mismatches: Vec::new(), attempt: None, vars: 0, depth: 0 }
    }
}

/// One typing of an `ArgCache`: the marks the application's typing of an argument left, from
/// which the retry reads what it needs where the application failed.
#[derive(Clone, Copy)]
struct ArgTyping {
    e: ExprId,
    /// The formal the member's application typed it against; `None` where it typed it alone,
    /// an argument of no parameter.
    formal: Option<TypeId>,
    /// `typedUnadapted`'s tree, its type as the typing gave it and that type with the
    /// variables solved where the application ended (`retry_on_qualifier`).
    te: TExprId,
    own: TypeId,
    ty: TypeId,
    /// The typing adapted to the formal, as a named argument's is kept (`typedNamedArg`); its
    /// type is the node's where the retry reads it.
    adapted: Option<TExprId>,
    /// The trail's length where the typing began, where it ended and where its adaptation ended.
    trail: (u32, u32, u32),
    /// The type variables' count where the typing began: the variables it made are from it on.
    vars: u32,
    /// The diagnostics' count where the typing began, where it ended and where its adaptation
    /// ended.
    diags: (u32, u32, u32),
    /// Whether the typing bound a temporary in the application's segment.
    hoisted: bool,
    /// The index's records the typing made.
    index: (super::index::Recorded, super::index::Recorded),
    /// A tupled dual (`type_tupled_dual`): whether its elements were typed alone, the formal
    /// being no tuple type of as many.
    dual: Option<bool>,
    /// A function literal the application did not type (`type_arg_recorded`): the formal gave
    /// its parameters no types, so it reported the missing type and typed none of it.
    untyped: bool,
}

impl ArgTyping {
    #[inline]
    fn trail(&self) -> (usize, usize, usize) {
        (self.trail.0 as usize, self.trail.1 as usize, self.trail.2 as usize)
    }

    #[inline]
    fn diags(&self) -> (usize, usize, usize) {
        (self.diags.0 as usize, self.diags.1 as usize, self.diags.2 as usize)
    }
}

/// What a member's retry took out of the member's application: its diagnostics, from the index
/// `diags_at` on, what the index recorded from `index_at` on, the trail's bindings from `mark`
/// on (`undone`, the last first) and the type mismatches it logged.
struct Taken<'d> {
    diags: &'d [crate::source::Diagnostic],
    diags_at: usize,
    index: &'d super::index::Taken,
    index_at: super::index::Recorded,
    undone: &'d [super::Redo],
    mark: usize,
    /// The count of type variables where the member's application began: those made since are
    /// its own, whose instances stay where the retry sets the application aside (the owner rule)
    /// and whose bounds go.
    vars: usize,
    mismatches: &'d [(usize, Span, TypeId)],
}

/// `cached_argument`'s reading of a typing: an error argument, an erroneous typing of an
/// argument typed alone, one dotty would type again, or the typing to adapt and whether the
/// retry may reuse it.
enum Cached {
    ErrorArg,
    Erroneous,
    /// A typing the retry would have to type again: the retry declines.
    Declined,
    /// The tree, its type and, where the retry may reuse it, the bindings to bring back.
    Typed(TExprId, TypeId, Option<Vec<super::Redo>>),
}

/// How a try of a member's retry ends before any extension or conversion is looked for.
enum Boundary {
    /// The member's errors stand for this try, as dotty has it: an argument is an error
    /// argument (`hasErrorArg`), or the member matches the arguments (`isMatchedBy`).
    Stands,
    /// The retry would have to type an argument the member's application typed, or adapt a
    /// typing whose dependencies the rollback undoes: the member's errors stand, and no
    /// further try is made.
    Declines,
    /// The argument lists to apply an extension or a conversion to, the index records of the
    /// typings they reuse, the bindings of their own type variables to bring back and the
    /// warnings of the reused macro expansions that are arguments themselves, which go with a
    /// try that applies (`expansion_warnings`).
    Retry(Vec<ArgList>, Vec<(super::index::Recorded, super::index::Recorded)>, Vec<super::Redo>, Vec<crate::source::Diagnostic>),
}

/// The tries of a member's retry, in dotty's order (Applications.scala 1494): the arguments as
/// written, separately, or tupled into one.
#[derive(Clone, Copy, PartialEq)]
enum RetryTry {
    Separate,
    Tupled,
}

pub enum Callee {
    Method { recv: Option<TExprId>, sym: SymId, owner_subst: Subst, prefix: Option<TypeId> },
    /// An overloaded name with the type of the receiver, which the owners of the alternatives
    /// are seen from.
    Overloaded { recv: Option<TExprId>, recv_ty: Option<TypeId>, set: SymId },
    Ctor(ClassId),
    Value(TExprId, TypeId),
    /// An extension method of a class or an object that the name an application's head writes is
    /// bound to: applied as its selection on that receiver is (`Syntax.tag(x)` for `tag(x)`).
    Selection { recv: TExprId, recv_ty: TypeId, name: Name },
}

/// `tupled` also admits several arguments for a single parameter, which auto-tupling packs.
fn clause_accepts(clause: &ClauseSig, n: usize, tupled: bool) -> bool {
    let repeated = clause.params.iter().any(|p| p.repeated);
    if tupled && n > 1 && clause.params.len() == 1 && !repeated {
        return true;
    }
    let required = clause.params.iter().filter(|p| !p.has_default && !p.repeated).count();
    n >= required && (repeated || n <= clause.params.len())
}

/// The declaration a library body's selection `e` names, out of line: a source file has no
/// reader's tables and never gets here.
#[cold]
#[inline(never)]
/// The type argument list of an application's head, where the head is applied to one.
fn type_list_of(ast: &crate::ast::Ast, e: ExprId) -> Option<ListRef> {
    let mut at = e;
    loop {
        match ast.expr(at) {
            Expr::Apply(f, _) | Expr::UsingApply(f, _) => at = f,
            Expr::TypeApply(_, ta) => return Some(ta),
            _ => return None,
        }
    }
}

pub(super) fn declaration_of(tables: &crate::ast::ReaderTables, e: ExprId) -> Option<crate::ast::DeclRef> {
    tables.decls.get(&e).copied()
}

#[derive(Clone)]
pub struct MethodCall {
    pub recv: Option<TExprId>,
    pub sym: SymId,
    pub owner_subst: Subst,
    pub ext_recv: Option<(TExprId, TypeId)>,
    /// The prefix the member's signature is seen from, where it depends on `this`.
    pub prefix: Option<TypeId>,
}

impl<'a> Worker<'a> {
    #[inline]
    pub fn type_application(&mut self, e: ExprId, expected: Option<TypeId>) -> (TExprId, TypeId) {
        if self.in_converted_body() {
            return self.type_hinted(e, expected, Self::type_application_now);
        }
        self.type_application_now(e, expected)
    }

    /// An application in a program with library bodies that hold inferred types.
    #[cold]
    #[inline(never)]
    pub(super) fn type_hinted(&mut self, e: ExprId, expected: Option<TypeId>, type_it: fn(&mut Self, ExprId, Option<TypeId>) -> (TExprId, TypeId)) -> (TExprId, TypeId) {
        match self.inferred_type_list(e) {
            Some(list) => self.type_with_inferred_args(e, list, expected, type_it),
            None => type_it(self, e, expected),
        }
    }

    /// The receiver a member is selected on. A bare polymorphic method there leaves the type
    /// arguments its result does not fix open for what follows.
    #[inline]
    pub(in crate::typer) fn type_receiver(&mut self, q: ExprId, selected: bool, bare: bool) -> (TExprId, TypeId) {
        let applied = matches!(self.cur_ast().expr(q), Expr::Apply(..) | Expr::UsingApply(..) | Expr::Select(..)).then(|| self.cur_ast().expr_span(q));
        let outer_applied = std::mem::replace(&mut self.open_applied_receiver, applied);
        let outer = (std::mem::replace(&mut self.apply_selected, selected), std::mem::replace(&mut self.open_receiver, bare));
        let (tq, qty) = self.type_expr(q, None);
        (self.apply_selected, self.open_receiver) = outer;
        self.open_applied_receiver = outer_applied;
        let left_open = (bare || applied.is_some()) && std::mem::take(&mut self.receiver_left_open);
        // A variable of the receiver's type that nothing bounds yet stays open through the
        // selection, as dotty selects a member on `List[?X]` without instantiating `?X`: the
        // fresh element variable of an improved lambda parameter (`foldLeft(Nil)((acc, x) =>
        // acc.::(x))`) is settled by the body, not by `acc`'s first selection. A bounded one is
        // solved as before.
        let _ = left_open;
        let qty = self.solve_bounded_in(qty);
        (tq, qty)
    }

    /// The type argument list scalac inferred for the application `e` of a library body, or for
    /// the receiver its member is selected from.
    pub(super) fn inferred_type_list(&self, e: ExprId) -> Option<u32> {
        let ast = self.cur_ast();
        if ast.inferred_type_lists.is_empty() {
            return None;
        }
        let mut head = e;
        let list = loop {
            match ast.expr(head) {
                Expr::Apply(f, _) | Expr::UsingApply(f, _) | Expr::Select(f, _) => head = f,
                Expr::TypeApply(_, ta) => break ta,
                Expr::New(ty, _) => match ast.ty(ty) {
                    TyExpr::Apply(_, ta) => break ta,
                    _ => return None,
                },
                _ => return None,
            }
        };
        ast.inferred_type_lists.binary_search(&list.start).is_ok().then_some(list.start)
    }

    /// A library body's application with the type arguments its jar's compile inferred, which
    /// reflect the definitions that compile saw: where they do not check against the class
    /// path, the application is typed with the arguments inferred again. Where neither
    /// checks, the pickled arguments' errors are the ones reported.
    #[cold]
    #[inline(never)]
    pub(super) fn type_with_inferred_args(
        &mut self,
        e: ExprId,
        list: u32,
        expected: Option<TypeId>,
        type_it: fn(&mut Self, ExprId, Option<TypeId>) -> (TExprId, TypeId),
    ) -> (TExprId, TypeId) {
        let key = (self.env.file, e, expected);
        match self.inferred_outcomes.get(&key) {
            Some(true) => return self.type_dropping(e, list, expected, type_it),
            Some(false) => return type_it(self, e, expected),
            None => {}
        }
        // Each typing that may not check is an attempt of its own.
        let diags = self.diags.items.len();
        let mark = self.attempt();
        let pickled = type_it(self, e, expected);
        if self.application_checks(pickled.1, diags, expected) {
            self.close(mark);
            return pickled;
        }
        self.retract(mark);
        let mark = self.attempt();
        let inferred = self.type_dropping(e, list, expected, type_it);
        let dropped = self.application_checks(inferred.1, diags, expected);
        self.inferred_outcomes.insert(key, dropped);
        if dropped {
            self.close(mark);
            return inferred;
        }
        self.retract(mark);
        type_it(self, e, expected)
    }

    fn type_dropping(
        &mut self,
        e: ExprId,
        list: u32,
        expected: Option<TypeId>,
        type_it: fn(&mut Self, ExprId, Option<TypeId>) -> (TExprId, TypeId),
    ) -> (TExprId, TypeId) {
        self.dropped_type_list = Some(list);
        let alternative = self.cur_ast().inferred_alternatives.iter().find(|&&(n, _)| n == e).map(|&(_, a)| a);
        let typed = match alternative {
            Some(a) => {
                let typed = self.type_application_now(a, expected);
                self.pass_new_outer(e, typed.0);
                typed
            }
            None => type_it(self, e, expected),
        };
        self.dropped_type_list = None;
        typed
    }

    /// Whether `offered` conforms to `declared`, constraining nothing where it does not.
    pub(super) fn admits(&mut self, offered: TypeId, declared: TypeId) -> bool {
        let mark = self.snapshot();
        let conforms = self.is_sub(offered, declared);
        if !conforms {
            self.rollback(mark);
        }
        conforms
    }

    fn application_checks(&mut self, ty: TypeId, diags: usize, expected: Option<TypeId>) -> bool {
        if self.diags.items[diags..].iter().any(|d| !d.is_warning) || ty == ERROR {
            return false;
        }
        match expected {
            Some(exp) if exp != self.b.t_unit && exp != ANY => {
                let mark = self.snapshot();
                let conforms = self.is_sub(ty, exp);
                if !conforms {
                    self.rollback(mark);
                }
                conforms
            }
            _ => true,
        }
    }

    /// A library body's application of a selection, the place of a diagnostic of its typing:
    /// the hook rides on the reader tables' test the
    /// selection makes anyway, so that a source's typing pays nothing for it.
    #[cold]
    #[inline(never)]
    fn type_application_placed(&mut self, e: ExprId, head: ExprId, expected: Option<TypeId>) -> (TExprId, TypeId) {
        let outer = self.body_node.replace(super::BodyNode { file: self.env.file, selection: head, call: e });
        let typed = self.type_application_now(e, expected);
        self.body_node = outer;
        typed
    }

    fn type_application_now(&mut self, e: ExprId, expected: Option<TypeId>) -> (TExprId, TypeId) {
        let ast = self.cur_ast();
        let span = ast.expr_span(e);
        let mut targs: Option<ListRef> = None;
        let mut head = e;
        let mut applies = 0;
        loop {
            match ast.expr(head) {
                Expr::Apply(f, _) | Expr::UsingApply(f, _) => {
                    applies += 1;
                    head = f;
                }
                Expr::TypeApply(f, ta) => {
                    if self.dropped_type_list != Some(ta.start) {
                        targs = Some(ta);
                    } else {
                        self.dropped_type_list = None;
                    }
                    head = f;
                    break;
                }
                _ => break,
            }
        }
        // Collected at their counted length rather than pushed during the walk: pushing kept both
        // vectors' headers in stack memory across the loop, and profile-guided builds after
        // unrelated edits stalled on reloading them.
        let mut app = e;
        let mut raw_lists: Vec<(ExprId, ListRef, bool)> = (0..applies)
            .map(|_| {
                let (f, args, using) = match ast.expr(app) {
                    Expr::Apply(f, args) => (f, args, false),
                    Expr::UsingApply(f, args) => (f, args, true),
                    _ => unreachable!("counted as an application above"),
                };
                let list = (app, args, using);
                app = f;
                list
            })
            .collect();
        raw_lists.reverse();
        let lists: Vec<ArgList> = raw_lists
            .iter()
            .map(|&(app, args, using)| ArgList { args: ast.expr_list(args).iter().map(|&a| ArgSrc::Ast(a)).collect(), using, span: ast.expr_span(app) })
            .collect();
        match ast.expr(head) {
            Expr::Infix(l, op, r) if targs.is_none() => {
                let head_span = ast.expr_span(head);
                if let Some(typed) = self.type_infix_applied(l, op, r, lists.clone(), head_span, expected) {
                    return typed;
                }
                let (te, ty) = self.type_expr(head, None);
                self.apply_callee(Callee::Value(te, ty), targs, lists, span, expected)
            }
            // `new C(a)(b)`: the further argument lists of the constructor.
            Expr::New(ty, first) if targs.is_none() => {
                let mut clauses = vec![(first, false)];
                clauses.extend(raw_lists.iter().map(|&(_, args, using)| (args, using)));
                let first_written = ast.new_empty_first.contains(&head);
                return self.type_new(ty, &clauses, first_written, span, expected);
            }
            Expr::SymRef(s) => {
                let r = self.sym_ref_term(s);
                let name = self.syms.sym(s).name;
                let Some(callee) = self.term_callee(r, name, span) else {
                    self.type_args_for_errors(&lists);
                    return (self.prog.add(TExpr::Unit), ERROR);
                };
                let callee = if matches!(callee, Callee::Overloaded { .. }) { self.referenced_alternative(callee, s, e, span) } else { callee };
                self.apply_named_callee(callee, targs, lists, span, expected)
            }
            Expr::Ident(name) => {
                if let Some(scope) = self.ext_scope {
                    if let Some(receiver) = self.sibling_extension_receiver(scope, name, &lists) {
                        if let Some(Callee::Value(recv, recv_ty)) = self.term_callee(receiver, name, span) {
                            return self.apply_member(recv, recv_ty, name, targs, lists, span, expected);
                        }
                    }
                }
                // `classOf[T]` is `Predef`'s, which the compiler types; scala-library's own is a
                // stub that answers `null`.
                let library_stub = |t: &mut Self| t.lookup_term(name).map_or(true, |r| r.sym().map_or(false, |s| t.in_jar(t.syms.sym(s).file)));
                if name == names::CLASS_OF && lists.is_empty() && library_stub(self) {
                    if let Some(ta) = targs.filter(|ta| ta.len == 1) {
                        let ty = self.resolve_type(self.cur_ast().ty_list(ta)[0]);
                        return self.type_class_of(ty, span);
                    }
                }
                // Synthesized members are reachable without a qualifier too.
                let synthesized = matches!(
                    name,
                    names::COPY | names::ORDINAL | names::TO_STRING | names::HASH_CODE | names::PRODUCT_PREFIX | names::GET_CLASS | names::SYNCHRONIZED
                );
                // So is a case class companion's `apply` inside the companion; another class's
                // constructor proxy is no member to name so (`apply()` in `object C` of `class C()`).
                let companion_apply = |t: &mut Self| {
                    name == names::APPLY
                        && t.this_class().and_then(|c| t.companion_class(c)).map_or(false, |k| t.syms.class(k).mods & crate::ast::mods::CASE != 0)
                };
                if (synthesized || companion_apply(self)) && self.lookup_term(name).is_none() {
                    if let Some(c) = self.this_class() {
                        self.complete_class(c);
                        let self_ty = self.syms.this_type(c);
                        let this = self.prog.add(TExpr::This);
                        return self.apply_member(this, self_ty, name, targs, lists, span, expected);
                    }
                }
                let of_enum_companion = matches!(name, names::VALUES | names::VALUE_OF | names::FROM_ORDINAL);
                if of_enum_companion && self.lookup_term(name).is_none() {
                    if let Some(c) = self.enclosing_enum_companion() {
                        let module_ty = self.types.class(c, &[]);
                        let module = self.prog.add(TExpr::Module(c));
                        return self.apply_member(module, module_ty, name, targs, lists, span, expected);
                    }
                }
                let outer_head = std::mem::replace(&mut self.extension_call_head, !lists.is_empty());
                let callee = self.resolve_ident(name, ast.expr_span(head));
                self.extension_call_head = outer_head;
                let Some(callee) = callee else {
                    self.type_args_for_errors(&lists);
                    return (self.prog.add(TExpr::Unit), ERROR);
                };
                self.apply_named_callee(callee, targs, lists, span, expected)
            }
            Expr::Select(q, name) => {
                if let Expr::Super(parent) = ast.expr(q) {
                    return self.apply_super_member(parent, name, targs, lists, ast.expr_span(q), span, expected);
                }
                if let Some(tables) = ast.reader.as_deref() {
                    if self.body_node.map_or(true, |n| n.file != self.env.file || n.call != e) {
                        // The walk above took a type argument list the caller drops: the walk
                        // again takes it the same way.
                        if targs.is_none() {
                            if let Some(ta) = type_list_of(ast, e) {
                                self.dropped_type_list = Some(ta.start);
                            }
                        }
                        return self.type_application_placed(e, head, expected);
                    }
                    if let Some(d) = declaration_of(tables, head) {
                        return self.apply_declared(d, head, q, name, targs, lists, span, expected);
                    }
                }
                match self.resolve_static_path(q, name) {
                    StaticPath::Callee(callee) => return self.apply_named_callee(callee, targs, lists, span, expected),
                    StaticPath::Missing(pkg) => {
                        let msg = format!("value {} is not a member of {}", self.name_str(name), self.package_display(pkg));
                        self.error(span, msg);
                        self.type_args_for_errors(&lists);
                        return (self.prog.add(TExpr::Unit), ERROR);
                    }
                    StaticPath::NotStatic => {}
                }
                if name == names::APPLY {
                    if let Some((c, class_targs)) = self.ctor_ref(q) {
                        return self.apply_companion_apply(c, targs.or(class_targs), lists, span, expected);
                    }
                }
                // `C.<init>$default$n` of a library body on a class without a companion object
                // (a facade's constructor with a `js.native` default): the missing argument.
                // Only a converted body spells a selection so; a source program's names are
                // not scanned for it.
                if self.loaded.is_some() && self.is_body_file(self.env.file) {
                    if let Some(n) = self.ctor_default_index(name) {
                        if let Some((c, _)) = self.ctor_ref(q) {
                            if let Some(ty) = self.ctor_param_default(c, n) {
                                return (self.prog.add(TExpr::Unit), ty);
                            }
                        }
                    }
                }
                let receiver = ast.expr(q);
                let selected = name == names::APPLY && matches!(receiver, Expr::TypeApply(..));
                let bare = matches!(receiver, Expr::Ident(_) | Expr::Select(..) | Expr::TypeApply(..));
                let (tq, qty) = self.type_receiver(q, selected, bare);
                let typed = self.apply_member(tq, qty, name, targs, lists, span, expected);
                if self.assign_op_target == Some(e) {
                    self.assign_op_receiver = Some((tq, qty));
                }
                typed
            }
            // The self call of a secondary constructor.
            Expr::This if self.excluded_ctor.is_some() => match self.this_class() {
                Some(c) => self.apply_callee(Callee::Ctor(c), targs, lists, span, expected),
                None => (self.prog.add(TExpr::Unit), ERROR),
            },
            _ => {
                let (te, ty) = self.type_expr(head, None);
                self.apply_callee(Callee::Value(te, ty), targs, lists, span, expected)
            }
        }
    }

    /// `super.m(args)` is the first concrete `m` behind the class in its linearisation, and
    /// `super[T].m(args)` the first one in the linearisation of the parent `T`. The call goes
    /// through the superclass chain when the method is found there, and names the class or trait
    /// that defines it otherwise, which is always the case inside a trait.
    #[inline(never)]
    fn apply_super_member(
        &mut self,
        parent: Name,
        name: Name,
        targs: Option<ListRef>,
        lists: Vec<ArgList>,
        super_span: Span,
        span: Span,
        expected: Option<TypeId>,
    ) -> (TExprId, TypeId) {
        let failed = |t: &mut Self, lists: &[ArgList]| {
            t.type_args_for_errors(lists);
            (t.prog.add(TExpr::Unit), ERROR)
        };
        let Some(c) = self.this_class().filter(|&c| self.parent_args_of != Some(c)) else {
            self.error(super_span, "super can be used only in a class, object, or template");
            return failed(self, &lists);
        };
        if let Some(m) = self.inline_under_check().filter(|&m| !self.defined_in_body_of(c, m)) {
            let msg = format!("Super call not allowed in inlineable method {}", self.name_str(self.syms.sym(m).name));
            self.error(super_span, msg);
            return failed(self, &lists);
        }
        self.complete_class(c);
        let mut bases: Vec<(ClassId, TypeId)> = self.syms.class(c).base_types[1..].to_vec();
        let mut named_parent = None;
        if parent != names::EMPTY {
            let parents = self.syms.class(c).parents.clone();
            let classes: Vec<ClassId> = parents.iter().filter_map(|&p| self.class_of(p)).collect();
            let found = classes.into_iter().find(|&k| self.syms.class(k).name == parent);
            let Some(k) = found else {
                let msg = format!("{} does not name a parent of {}", self.name_str(parent), self.class_description(c));
                self.error(super_span, msg);
                return failed(self, &lists);
            };
            bases.retain(|&(b, _)| self.syms.class(k).base_types.iter().any(|&(x, _)| x == b));
            named_parent = Some(k);
        }
        let in_trait = self.syms.class(c).kind == ClassKind::Trait && named_parent.is_none();
        let mut lists = lists;
        let mut target = None;
        let mut deferred = None;
        // Of an overloaded name `super` means the alternative the arguments choose; what follows
        // looks at the members with its parameters alone.
        let chosen = self.super_alternative(c, &bases, name, targs, &mut lists, expected);
        for &(b, bt) in &bases {
            let mut k = 0;
            while let Some(m) = self.own_alternative(b, name, k) {
                k += 1;
                if self.is_private(m) || chosen.map_or(false, |alt| alt != m && !self.same_parameters(c, &bases, m, alt, bt)) {
                    continue;
                }
                if self.is_abstract_member(m) {
                    deferred.get_or_insert((m, b, bt));
                    continue;
                }
                target = Some((m, b, bt));
                break;
            }
            if target.is_some() {
                break;
            }
        }
        // Inside a trait the call is bound by the class that mixes the trait in; a member
        // declared `abstract override` may call what is still abstract here.
        if in_trait && target.is_none() {
            let stackable = crate::ast::mods::ABSTRACT | crate::ast::mods::OVERRIDE;
            let own = self.syms.class(c).members.get(&name).copied();
            target = deferred.filter(|_| own.map_or(false, |o| self.syms.sym(o).mods & stackable == stackable));
        }
        let Some((m, owner, owner_ty)) = target else {
            // `super.equals` and its companions reach `Object`'s definitions, whichever trait
            // declared them abstract on the way.
            if matches!(name, names::TO_STRING | names::HASH_CODE | names::EQUALS | names::CLONE) {
                return self.apply_super_any_member(c, in_trait, name, targs, lists, span, expected);
            }
            let msg = match deferred {
                Some((m, b, _)) => format!(
                    "{} is accessed from super. It may not be abstract unless it is overridden by a member declared `abstract' and `override'",
                    self.member_description(m, Some(b))
                ),
                None => format!("value {} is not a member of the parents of {}", self.name_str(name), self.class_description(c)),
            };
            self.error(span, msg);
            return failed(self, &lists);
        };
        let first_parent = self.syms.class(c).parents.first().copied();
        let direct_superclass = first_parent.and_then(|p| self.class_of(p));
        let in_class = self.syms.class(owner).kind != ClassKind::Trait;
        if in_class && named_parent.map_or(false, |k| Some(k) != direct_superclass) && Some(owner) != direct_superclass {
            let msg = format!(
                "Super call cannot be emitted: the selected {} is declared in {}, which is not the direct superclass of {}",
                self.member_description(m, None),
                self.class_description(owner),
                self.class_description(c)
            );
            self.error(span, msg);
            return failed(self, &lists);
        }
        if !self.is_method_sym(m) {
            let msg = format!("super may be not be used on {}", self.member_description(m, None));
            self.error(span, msg);
            return failed(self, &lists);
        }
        let info = self.syms.class(c);
        let through_chain = info.kind != ClassKind::Trait
            && info.superclass.map_or(false, |s| {
                named_parent.map_or(true, |k| k == s) && self.syms.class(s).base_types.iter().any(|&(x, _)| x == owner)
            });
        let target = if in_trait {
            self.note_mixin_super(c, m);
            SuperTarget::Mixin(c)
        } else if through_chain {
            SuperTarget::Chain
        } else {
            SuperTarget::Class(owner)
        };
        let recv = self.prog.add(TExpr::Super(target));
        let callee = self.member_callee_in(recv, owner_ty, Some(owner_ty), m);
        self.apply_callee(callee, targs, lists, span, expected)
    }

    /// `super.toString`, `super.hashCode` and `super.equals(that)` where no class or trait behind
    /// the caller defines the member: what `Any` does. Inside a trait the class that mixes the
    /// trait in may still have a definition behind it, so the call is bound there.
    #[cold]
    fn apply_super_any_member(
        &mut self,
        c: ClassId,
        in_trait: bool,
        name: Name,
        targs: Option<ListRef>,
        mut lists: Vec<ArgList>,
        span: Span,
        expected: Option<TypeId>,
    ) -> (TExprId, TypeId) {
        if name != names::EQUALS && lists.first().map_or(false, |l| !l.using && l.args.is_empty()) {
            lists.remove(0);
        }
        if in_trait {
            let sym = self.any_member_of(c, name, span);
            self.note_mixin_super(c, sym);
            let recv = self.prog.add(TExpr::Super(SuperTarget::Mixin(c)));
            let callee = Callee::Method { recv: Some(recv), sym, owner_subst: Vec::new(), prefix: None };
            return self.apply_callee(callee, targs, lists, span, expected);
        }
        let this = self.prog.add(TExpr::This);
        let helper = |t: &mut Self, template: &str, ty: TypeId| {
            let s = t.prog.add_str(template);
            let l = t.prog.list(&[this]);
            let te = t.prog.add(TExpr::Js(s, l));
            if t.capturing() {
                t.capture_form(te, Form::SuperOp(name));
            }
            (te, ty)
        };
        match (name, lists.as_slice()) {
            (names::TO_STRING, []) => helper(self, "$anyStr($0)", self.b.t_string),
            (names::HASH_CODE, []) => helper(self, "$identityHash($0)", self.b.t_int),
            (names::CLONE, []) => helper(self, "$cloneObject($0)", self.b.t_any_ref),
            (names::EQUALS, [list]) if list.args.len() == 1 && list.args[0].ast().is_some() => {
                let (that, _) = self.type_expr(list.args[0].ast().unwrap(), None);
                let te = self.prog.add(TExpr::Prim(PrimOp::RefEq, this, that));
                if self.capturing() {
                    self.capture_form(te, Form::SuperOp(name));
                }
                (te, self.b.t_boolean)
            }
            _ => {
                let msg = format!("super.{} does not take these arguments", self.name_str(name));
                self.error(span, msg);
                self.type_args_for_errors(&lists);
                (self.prog.add(TExpr::Unit), ERROR)
            }
        }
    }

    /// The symbol under which the trait `tr` calls the `toString`, `hashCode` or `equals` that
    /// follows it in a linearisation; no ancestor of the trait declares one.
    fn any_member_of(&mut self, tr: ClassId, name: Name, span: Span) -> SymId {
        self.with_loader(|w| w.any_member_of_unlocked(tr, name, span))
    }

    fn any_member_of_unlocked(&mut self, tr: ClassId, name: Name, span: Span) -> SymId {
        if let Some(&s) = self.any_members.get(&(tr, name)) {
            return s;
        }
        let file = self.syms.class(tr).file;
        let sym = self.syms.new_sym(name, SymKind::Def, 0, Owner::Class(tr), file, None, span);
        let (clauses, ret) = match name {
            names::EQUALS => {
                let that = self.syms.new_sym(names::THAT, SymKind::Param, 0, Owner::Local, file, None, span);
                self.syms.sym_mut(that).sig = Some(Arc::new(MethodSig::value(ANY)));
                self.syms.sym_mut(that).state().set(Completion::Done);
                let param =
                    ParamSig { name: names::THAT, ty: ANY, by_name: false, repeated: false, has_default: false, sym: that };
                (vec![ClauseSig { params: vec![param], is_using: false, is_implicit: false }], self.b.t_boolean)
            }
            names::HASH_CODE => (Vec::new(), self.b.t_int),
            names::CLONE => (Vec::new(), self.b.t_any_ref),
            _ => (Vec::new(), self.b.t_string),
        };
        let mut info = self.syms.sym_mut(sym);
        info.sig = Some(Arc::new(MethodSig { tparams: Vec::new(), clauses, ret }));
        info.state().set(Completion::Done);
        self.any_members.insert((tr, name), sym);
        sym
    }

    /// Inside an extension method `g(args)` stands for `x.g(args)` when `g` is a method of the
    /// same `extension (x: T)` clause. A local `g` wins. A term `g` defined next to the extension
    /// is an overload to Scala, which is decided here by the shape of the call alone.
    #[inline(never)]
    fn sibling_extension_receiver(
        &mut self,
        scope: super::ExtScope,
        name: Name,
        lists: &[ArgList],
    ) -> Option<TermRef> {
        let is_sibling = |t: &Self, s: SymId| {
            let info = t.syms.sym(s);
            if info.name != name || info.file != scope.file {
                return false;
            }
            match info.def.map(|d| &t.ast(info.file).def(d).kind) {
                Some(crate::ast::DefKind::Fun(f)) => f.ext_group == scope.group,
                _ => false,
            }
        };
        let siblings: Vec<SymId> = match scope.owner {
            Owner::Package(p) => match self.syms.pkg(p).entries.get(&name) {
                Some(e) => e.extensions.iter().copied().filter(|&s| is_sibling(self, s)).collect(),
                None => return None,
            },
            Owner::Class(c) => {
                self.syms.class(c).extensions.iter().copied().filter(|&s| is_sibling(self, s)).collect()
            }
            Owner::Local => self
                .env
                .frames
                .iter()
                .flat_map(|f| match f {
                    super::Frame::Locals { names, .. } => names.as_slice(),
                    super::Frame::Class(_) => &[],
                })
                .map(|&(_, s)| s)
                .filter(|&s| is_sibling(self, s))
                .collect(),
        };
        if siblings.is_empty() {
            return None;
        }
        let overloaded = match self.lookup_term(name) {
            Some(TermRef::Local(s)) if !siblings.contains(&s) => return None,
            Some(TermRef::Local(_)) => false,
            Some(TermRef::Class(c)) => self.syms.class(c).owner == scope.owner,
            Some(r) => r.sym().map_or(false, |s| self.syms.sym(s).owner == scope.owner),
            None => false,
        };
        if overloaded {
            let applied = lists.iter().any(|l| !l.using);
            let fits = siblings.iter().any(|&s| {
                applied != self.is_parameterless_extension(s) && self.extension_accepts(s, lists, true)
            });
            if !fits {
                return None;
            }
        }
        Some(TermRef::Local(scope.receiver))
    }

    fn enclosing_enum_companion(&self) -> Option<ClassId> {
        self.env.frames.iter().rev().find_map(|f| match f {
            super::Frame::Class(c) => {
                let info = self.syms.class(*c);
                let of_enum = info.kind == ClassKind::Object
                    && info.companion.map_or(false, |e| self.syms.class(e).kind == ClassKind::Enum);
                of_enum.then_some(*c)
            }
            _ => None,
        })
    }

    /// An error about how many arguments a list holds: dependent where the parser's recovery cut
    /// the list (`Ast::cut_args`: its first argument, or for an empty list the application whose
    /// own span the list carries), each list of an application apart.
    fn arity_error(&mut self, list: &ArgList, msg: String) {
        let ast = self.cur_ast();
        let cut = match list.args.first().and_then(|a| a.ast()) {
            Some(head) => ast.cut_args.contains(&head),
            None => ast.cut_args.iter().any(|&e| ast.expr_span(e) == list.span),
        };
        if cut {
            self.dependent_error(list.span, msg);
        } else {
            self.error(list.span, msg);
        }
    }

    pub(super) fn type_args_for_errors(&mut self, lists: &[ArgList]) {
        for l in lists {
            for (i, a) in l.args.iter().enumerate() {
                if let Some(mut e) = a.ast() {
                    if let Expr::NamedArg(_, v) = self.cur_ast().expr(e) {
                        e = v;
                    }
                    // A spread in its place would report itself as misplaced: its sequence alone is typed.
                    if let Expr::Typed(inner, t) = self.cur_ast().expr(e) {
                        if i + 1 == l.args.len() && matches!(self.cur_ast().ty(t), TyExpr::Repeated(_)) {
                            e = inner;
                        }
                    }
                    if !matches!(self.cur_ast().expr(e), Expr::Lambda(..)) {
                        self.type_expr(e, None);
                    }
                }
            }
        }
    }

    /// Resolves `pkg.name` and `pkg.Obj` style paths that are not values by themselves.
    pub(in crate::typer) fn resolve_static_path(&mut self, q: ExprId, name: Name) -> StaticPath {
        let Some(pkg) = self.package_path(q) else { return StaticPath::NotStatic };
        if self.deps.is_some() {
            self.deps_package_path(pkg);
        }
        match self.pkg_term(pkg, name) {
            None => StaticPath::Missing(pkg),
            Some(TermRef::Package(_)) => StaticPath::NotStatic,
            Some(r) => match self.term_callee(r, name, self.cur_ast().expr_span(q)) {
                Some(callee) => StaticPath::Callee(callee),
                None => StaticPath::NotStatic,
            },
        }
    }

    /// A package as scalac names it in a diagnostic: its simple name, `<root>` for the root.
    pub(in crate::typer) fn package_display(&self, p: PkgId) -> String {
        if p == ROOT_PKG { "<root>".to_string() } else { self.name_str(self.syms.pkg(p).name).to_string() }
    }

    /// The class that `q` names in `q.apply`: `C`, `Obj.C` or `pkg.C`, with or without type
    /// arguments. A class with an explicit companion is named by the companion instead.
    pub(in crate::typer) fn ctor_ref(&mut self, q: ExprId) -> Option<(ClassId, Option<ListRef>)> {
        match self.cur_ast().expr(q) {
            Expr::TypeApply(f, targs) => Some((self.ctor_ref(f)?.0, Some(targs))),
            _ => match self.static_ref(q)? {
                TermRef::Class(c) => Some((c, None)),
                _ => None,
            },
        }
    }

    /// The class that an application of `callee` constructs: a class, or a companion object
    /// without an `apply` of its own.
    pub fn constructed_class(&self, callee: TermRef) -> Option<ClassId> {
        if let TermRef::Class(c) = callee {
            return Some(c);
        }
        let SymKind::Object(o) = self.syms.sym(callee.sym()?).kind else { return None };
        let class = self.companion_class(o)?;
        (!self.syms.class(o).members.contains_key(&names::APPLY)).then_some(class)
    }

    /// Whether `f` in `f(args) op= rhs` names a value whose element is updated, rather than a
    /// constructor or a method whose result is the receiver of `op=`.
    pub fn is_indexable(&mut self, f: ExprId) -> bool {
        let ast = self.cur_ast();
        // `VdomAttr[String]("x") := v` applies the value's `apply` to type arguments: the
        // result's operator is called, and no `update` of the value stands for it.
        if matches!(ast.expr(f), Expr::TypeApply(..)) {
            return false;
        }
        let head = f;
        match self.static_ref(head) {
            Some(TermRef::Class(_)) | Some(TermRef::Package(_)) => false,
            Some(r) => match r.sym() {
                Some(s) => !self.is_method_sym(s) && self.constructed_class(r).is_none(),
                None => true,
            },
            None => head == f,
        }
    }

    /// `q.x = rhs`: `Ok` with the call `q.x_=(rhs)` when `x` is not a `var` but has a setter
    /// method, `Err` with the typed `q.x` otherwise, for the plain assignment.
    pub fn select_for_assignment(
        &mut self,
        lhs: ExprId,
        q: ExprId,
        name: Name,
        rhs: ExprId,
        span: Span,
    ) -> Result<(TExprId, TypeId), (TExprId, TypeId)> {
        if self.package_path(q).is_some() {
            return Err(self.type_expr(lhs, None));
        }
        let (tq, qty) = self.type_expr(q, None);
        let qty = self.solve_in(qty);
        let is_var = matches!(self.find_member(qty, name), Some((s, _)) if self.syms.sym(s).kind == SymKind::Var);
        if !is_var && qty != ERROR {
            let setter = self.interner.intern(&format!("{}_=", self.name_ref(name)));
            if self.find_member(qty, setter).is_some() || self.has_lexical_extension_for(qty, setter) {
                let lists = vec![ArgList { args: vec![ArgSrc::Ast(rhs)], using: false, span }];
                let (te, _) = self.apply_member(tq, qty, setter, None, lists, span, None);
                return Ok((te, self.b.t_unit));
            }
        }
        Err(self.apply_member(tq, qty, name, None, Vec::new(), span, None))
    }

    pub(super) fn companion_class(&self, object: ClassId) -> Option<ClassId> {
        let info = self.syms.class(object);
        let is_module = info.kind == ClassKind::Object || info.local_module.is_some() || info.inner_object.is_some();
        is_module.then_some(info.companion?).filter(|&c| self.syms.class(c).kind == ClassKind::Class)
    }

    /// `scala.summon`, whose result has the type of the given it found.
    pub(super) fn is_std_summon(&self, sym: SymId) -> bool {
        let info = self.syms.sym(sym);
        matches!(info.name, names::SUMMON | names::IMPLICITLY) && info.owner == Owner::Package(self.b.scala_pkg)
    }

    /// A def that takes nothing but type arguments and using clauses, like `Ordering.apply[T]`.
    fn is_summoner(&mut self, sym: SymId) -> bool {
        if self.syms.sym(sym).kind != SymKind::Def {
            return false;
        }
        let sig = self.sig_of(sym);
        !sig.clauses.is_empty() && sig.clauses.iter().all(|c| c.is_using)
    }

    /// The arity of the first clause that has to be applied, when it needs arguments.
    fn required_arity(clauses: &[ClauseSig]) -> Option<usize> {
        let first = clauses.iter().find(|c| !c.is_using)?;
        first.params.iter().any(|p| !p.has_default && !p.repeated).then_some(first.params.len())
    }

    /// `Codec[Int]` without an argument list, where the `apply` of `object Codec` (or the
    /// constructor of the case class standing in for it) needs arguments that nothing expects a
    /// function for, is `summon[Codec[Int]]`, as summoners read.
    fn summon_type_application(
        &mut self,
        c: ClassId,
        apply: Option<SymId>,
        targs: ListRef,
        span: Span,
        expected: Option<TypeId>,
    ) -> Option<(TExprId, TypeId)> {
        if self.apply_selected {
            return None;
        }
        self.complete_class(c);
        let arity = match apply {
            Some(sym) => Self::required_arity(&self.sig_of(sym).clauses)?,
            None => Self::required_arity(&self.syms.class(c).ctor)?,
        };
        let arg_ids: Vec<TyExprId> = self.cur_ast().ty_list(targs).to_vec();
        if arg_ids.len() != self.syms.class(c).tparams.len() {
            return None;
        }
        if let Some(exp) = expected {
            if self.expected_function(exp, arity).is_some() {
                return None;
            }
        }
        let tys: Vec<TypeId> = arg_ids.iter().map(|&t| self.resolve_type(t)).collect();
        let ty = self.types.class(c, &tys);
        let te = match self.resolve_given(ty, span) {
            Some(te) => te,
            None => {
                let msg = self
                    .given_ambiguity
                    .take()
                    .unwrap_or_else(|| format!("no given instance of type {} was found", self.show(ty)));
                let msg = msg + &self.given_failure_notes();
                self.given_failure_error(span, msg, ty);
                self.prog.add(TExpr::Unit)
            }
        };
        Some((te, ty))
    }

    pub fn static_ref(&mut self, e: ExprId) -> Option<TermRef> {
        match self.cur_ast().expr(e) {
            Expr::Ident(n) => self.lookup_term(n),
            Expr::SymRef(s) => Some(self.sym_ref_term(s)),
            Expr::Select(q, n) => match self.static_ref(q)? {
                TermRef::Package(p) => self.pkg_term(p, n),
                r => match self.syms.sym(r.sym()?).kind {
                    SymKind::Object(c) => self.module_term(c, n),
                    _ => None,
                },
            },
            _ => None,
        }
    }

    fn package_path(&mut self, e: ExprId) -> Option<PkgId> {
        let ast = self.cur_ast();
        match ast.expr(e) {
            Expr::Ident(n) => match self.lookup_term(n)? {
                TermRef::Package(p) => Some(p),
                _ => None,
            },
            Expr::Select(q, n) => {
                let p = self.package_path(q)?;
                if let Some(sub) = self.demand_pkg(p, n) {
                    return Some(sub);
                }
                // A JDK package is entered when the JDK is first opened, which a program's
                // first `java.time.X` path does here.
                if self.sees_classpath() && self.jdk_rooted(p) && (self.load_pkg_member(p, n) || self.forked) {
                    return self.syms.pkg(p).entries.get(&n).and_then(|e| e.pkg);
                }
                None
            }
            _ => None,
        }
    }

    pub(super) fn is_method_sym(&mut self, s: SymId) -> bool {
        match self.syms.sym(s).kind {
            SymKind::Def | SymKind::Overloaded(_) => true,
            SymKind::Given => {
                let sig = self.sig_of(s);
                !sig.clauses.is_empty() || !sig.tparams.is_empty()
            }
            _ => false,
        }
    }

    fn is_path(&self, e: TExprId) -> bool {
        match self.prog.expr(e) {
            TExpr::Module(_) | TExpr::This | TExpr::Super(_) | TExpr::Local(_) | TExpr::Static(_) => true,
            TExpr::Field(recv, _) => self.is_path(recv),
            _ => false,
        }
    }

    fn global_callee(&mut self, s: SymId) -> Callee {
        if let SymKind::Overloaded(_) = self.syms.sym(s).kind {
            return Callee::Overloaded { recv: None, recv_ty: None, set: s };
        }
        if self.is_method_sym(s) {
            return Callee::Method { recv: None, sym: s, owner_subst: Vec::new(), prefix: None };
        }
        let ty = self.sig_of(s).ret;
        if let Some(c) = self.module_alias(s, ty) {
            let te = self.prog.add(TExpr::Module(c));
            if self.deps.is_some() {
                self.deps_node(te, super::deps::Node::AliasVal(s));
            }
            return Callee::Value(te, ty);
        }
        let info = self.syms.sym(s);
        let te = match (info.kind, info.js_import, info.js_global) {
            (SymKind::Object(c), _, _) => self.prog.add(TExpr::Module(c)),
            (_, Some(import), _) => self.prog.add(TExpr::JsImport(import)),
            (_, _, Some(global)) => self.prog.add(TExpr::JsGlobal(global, false)),
            _ => self.prog.add(TExpr::Static(s)),
        };
        if self.capturing() && matches!(self.prog.expr(te), TExpr::JsImport(_) | TExpr::JsGlobal(..)) {
            self.capture_form(te, Form::Member(s));
        }
        Callee::Value(te, ty)
    }

    /// A library val whose type is that of an object (`val Nil: Nil.type` in `scala`) is the
    /// object, which patterns and the given search read it as.
    pub(super) fn module_alias(&self, s: SymId, ty: TypeId) -> Option<ClassId> {
        let info = self.syms.sym(s);
        if info.kind != SymKind::Val || !self.in_jar(info.file) {
            return None;
        }
        match self.types.get(ty) {
            Type::Class(o, _) if self.syms.class(o).kind == ClassKind::Object => Some(o),
            _ => None,
        }
    }

    fn member_callee(&mut self, recv: TExprId, recv_ty: TypeId, s: SymId) -> Callee {
        let name = self.syms.sym(s).name;
        match self.find_member(recv_ty, name) {
            Some((found, owner_ty)) => {
                // What the lookup by type finds may be the settled entry of what the scope held.
                let s = if self.syms.alternatives(found).is_some() { found } else { s };
                self.member_callee_in(recv, recv_ty, Some(owner_ty), s)
            }
            None => self.member_callee_in(recv, recv_ty, None, s),
        }
    }

    fn is_own_ctor_param(&self, s: SymId) -> bool {
        let Owner::Class(owner) = self.syms.sym(s).owner else { return false };
        self.syms.class(owner).ctor_syms.iter().flatten().any(|&p| p == s)
    }

    /// Private members are reachable from their class and its companion only, a top-level
    /// `private[p]` definition from inside `p`.
    pub fn is_accessible(&self, s: SymId) -> bool {
        let info = self.syms.sym(s);
        let Owner::Class(owner) = info.owner else {
            let qualified = info.mods & crate::ast::mods::PRIVATE != 0 && self.has_access_scope(info.file, info.span.start);
            return !qualified || self.in_access_scope(info.file, info.span.start, info.owner);
        };
        if info.mods & crate::ast::mods::PRIVATE == 0 {
            return self.qualified_reachable(s);
        }
        let companion = self.syms.class(owner).companion.or_else(|| self.inner_companion(owner));
        let inside = self.env.frames.iter().any(|f| match f {
            super::Frame::Class(c) => *c == owner || Some(*c) == companion,
            _ => false,
        });
        inside || self.in_access_scope(info.file, info.span.start, info.owner)
    }

    /// The object of a jar that is the companion of its class `c` nested in a class, an inner
    /// object the loader keeps apart from the class (`object Path` beside `class Path`).
    fn inner_companion(&self, c: ClassId) -> Option<ClassId> {
        let Owner::Class(o) = self.syms.class(c).owner else { return None };
        self.nested_object_of(o, self.syms.class(c).name)
    }

    /// A given is a candidate where it is accessible: a protected one inside its class, the
    /// companion of that, a subclass or a trait whose self type is one of those (kittens keeps
    /// old alternatives `protected` so that a search from outside passes them over).
    pub fn is_given_accessible(&self, s: SymId) -> bool {
        if !self.is_accessible(s) {
            return false;
        }
        let info = self.syms.sym(s);
        let Owner::Class(owner) = info.owner else { return true };
        if info.mods & crate::ast::mods::PROTECTED == 0 {
            return true;
        }
        let companion = self.syms.class(owner).companion;
        self.env.frames.iter().any(|f| match f {
            super::Frame::Class(c) => Some(*c) == companion || self.derives_or_selects(*c, owner),
            _ => false,
        })
    }

    /// A jar's `private[p]` or `protected[p]` member is out of a program's reach outside `p`, a
    /// protected one apart from subclasses; the library's own bodies see it as public.
    pub(super) fn qualified_reachable(&self, s: SymId) -> bool {
        if self.syms.sym(s).mods & crate::ast::mods::QUALIFIED == 0 {
            return true;
        }
        let Some(loaded) = self.loaded.as_ref() else { return true };
        let Some(&(scope, protected)) = loaded.access_within.get(&s) else { return true };
        let file = self.env.file;
        self.in_jar(file) || self.is_body_file(file) || self.source(file).is_std || self.within_scope_of(self.syms.sym(s).owner, scope) || (protected && self.in_subclass_of_owner(s))
    }

    /// Whether the site is inside a subclass of the owner of `s`.
    pub(super) fn in_subclass_of_owner(&self, s: SymId) -> bool {
        let Owner::Class(owner) = self.syms.sym(s).owner else { return false };
        self.env.frames.iter().any(|f| matches!(f, super::Frame::Class(c) if self.derives_or_selects(*c, owner)))
    }

    fn derives_or_selects(&self, c: ClassId, owner: ClassId) -> bool {
        let derives = |k: ClassId| k == owner || self.syms.class(k).base_types.iter().any(|&(b, _)| b == owner);
        if derives(c) {
            return true;
        }
        let Some(self_ty) = self.syms.class(c).declared_self else { return false };
        let mut parts = vec![self_ty];
        while let Some(t) = parts.pop() {
            match self.types.get(t) {
                Type::Class(k, _) if derives(k) => return true,
                Type::Inter(a, b) => parts.extend([a, b]),
                _ => {}
            }
        }
        false
    }

    /// A `private` class is reachable from its enclosing class and that class's companion only.
    pub(super) fn is_class_accessible(&self, c: ClassId) -> bool {
        let info = self.syms.class(c);
        if info.mods & crate::ast::mods::PRIVATE == 0 {
            return true;
        }
        let Owner::Class(owner) = info.owner else { return true };
        let companion = self.syms.class(owner).companion;
        let inside = self.env.frames.iter().any(|f| match f {
            super::Frame::Class(k) => *k == owner || Some(*k) == companion,
            _ => false,
        });
        inside || self.in_access_scope(info.file, info.span.start, info.owner)
    }

    /// A protected member is reached through `this` and `super`, inside its class and the
    /// companion of that, and through a receiver that is an instance of the enclosing class that
    /// inherits the member, a self type counting as inheritance (SLS 5.2).
    #[cold]
    fn check_protected_access(&mut self, s: SymId, recv: TExprId, recv_ty: TypeId, span: Span) {
        let through_this = matches!(self.prog.expr(recv), TExpr::This | TExpr::Super(_));
        let Some(enclosing_subclass) = self.protected_denied(s, through_this, recv_ty) else { return };
        let Some(owner) = self.access_owner(s, Some(recv_ty)) else { return };
        let from = match self.this_class() {
            Some(k) => self.class_description(k),
            None => "here".to_string(),
        };
        let member = self.member_description(s, None);
        let msg = format!(
            "{0} in {1} cannot be accessed as a member of {2} from {3}; protected {0} can only be accessed from {4} or one of its subclasses",
            member,
            self.class_description(owner),
            self.show(recv_ty),
            from,
            self.class_description(enclosing_subclass.unwrap_or(owner)),
        );
        self.error(span, msg);
    }

    /// The rule `check_protected_access` reports by, which reports nothing: `None` where the
    /// protected member `s` is reached from the site through a receiver of type `recv_ty` (`this`
    /// or `super` when `through_this`), else the enclosing subclass, if any, that it could be
    /// reached from.
    pub(super) fn protected_denied(&mut self, s: SymId, through_this: bool, recv_ty: TypeId) -> Option<Option<ClassId>> {
        if through_this {
            return None;
        }
        let (owner, file, filed_at) = {
            let info = self.syms.sym(s);
            let Owner::Class(owner) = info.owner else { return None };
            (owner, info.file, info.span.start)
        };
        let companion = self.syms.class(owner).companion;
        let derives = |t: &Self, sub: ClassId, sup: ClassId| t.syms.class(sub).base_types.iter().any(|&(b, _)| b == sup);
        let mut enclosing_subclass = None;
        let classes: Vec<ClassId> = self.env.frames.iter().rev().filter_map(|f| match f {
            super::Frame::Class(k) => Some(*k),
            _ => None,
        }).collect();
        for k in classes {
            if k == owner || Some(k) == companion {
                return None;
            }
            if enclosing_subclass.is_none() && (derives(self, k, owner) || self.self_type_derives(k, owner)) {
                enclosing_subclass = Some(k);
            }
        }
        if self.in_access_scope(file, filed_at, Owner::Class(owner)) {
            return None;
        }
        // The body of an inline method expands where it is called, with the access rights of
        // the class that declares it.
        if let Some(site) = self.inline.sites.last() {
            if let Owner::Class(k) = self.syms.sym(site.callee).owner {
                if k == owner || derives(self, k, owner) || derives(self, owner, k) {
                    return None;
                }
            }
        }
        if let Some(k) = enclosing_subclass {
            if self.instance_of_class(recv_ty, k) {
                return None;
            }
        }
        Some(enclosing_subclass)
    }

    pub(super) fn check_access(&mut self, s: SymId, recv_ty: Option<TypeId>, span: Span) {
        if !self.is_accessible(s) {
            let Some(owner) = self.access_owner(s, recv_ty) else { return };
            let msg = format!("{} is private to {}", self.name_str(self.syms.sym(s).name), self.name_str(self.syms.class(owner).name));
            self.error(span, msg);
        }
    }

    /// The class an access error names for the member `s`: its owner, or for a deferred given
    /// read on an instance of a class the class holding the implementation the search made.
    fn access_owner(&mut self, s: SymId, recv_ty: Option<TypeId>) -> Option<ClassId> {
        let Owner::Class(owner) = self.syms.sym(s).owner else { return None };
        if self.syms.sym(s).mods & crate::ast::mods::DEFERRED == 0 {
            return Some(owner);
        }
        let implementing = recv_ty.and_then(|t| self.class_of(t)).and_then(|k| self.implementing_class(k, s));
        Some(implementing.unwrap_or(owner))
    }

    /// `private[scope]` also admits whatever is inside the class or package called `scope`.
    pub(super) fn has_access_scope(&self, file: FileId, filed_at: u32) -> bool {
        self.ast(file).access_scopes.iter().any(|&(at, _)| at == filed_at)
    }

    /// Whether the site is inside the class or package that the `private[scope]` filed at
    /// `filed_at` names, for a definition in `from`.
    pub(super) fn in_access_scope(&self, file: FileId, filed_at: u32, from: Owner) -> bool {
        let scopes = &self.ast(file).access_scopes;
        let Some(&(_, scope)) = scopes.iter().find(|&&(at, _)| at == filed_at) else { return false };
        self.within_scope_of(from, scope)
    }

    /// Whether the site is inside what `private[scope]` names for a definition in `from`: the
    /// innermost class or package around it of that name, as scalac resolves the qualifier
    /// (`q.p` is not the `p` of a definition in `p`); by the name alone where none is found.
    pub(super) fn within_scope_of(&self, from: Owner, scope: crate::intern::Name) -> bool {
        let mut at = from;
        loop {
            match at {
                Owner::Class(c) => {
                    if self.syms.class(c).name == scope {
                        let companion = self.syms.class(c).companion;
                        return self.env.frames.iter().any(|f| matches!(f, super::Frame::Class(k) if *k == c || Some(*k) == companion));
                    }
                    at = self.syms.class(c).owner;
                }
                Owner::Package(p) => {
                    let mut q = Some(p);
                    while let Some(pk) = q {
                        if self.syms.pkg(pk).name == scope {
                            let mut site = Some(self.file_pkgs[self.env.file.0 as usize]);
                            while let Some(sp) = site {
                                if sp == pk {
                                    return true;
                                }
                                site = self.syms.pkg(sp).parent;
                            }
                            return false;
                        }
                        q = self.syms.pkg(pk).parent;
                    }
                    return self.within_scope(scope);
                }
                Owner::Local => return self.within_scope(scope),
            }
        }
    }

    /// Whether the site is inside a class or package of the simple name `scope`, what a
    /// `private[scope]` definition is reachable from.
    pub(super) fn within_scope(&self, scope: crate::intern::Name) -> bool {
        let in_class = self.env.frames.iter().any(|f| match f {
            super::Frame::Class(c) => self.syms.class(*c).name == scope,
            _ => false,
        });
        if in_class {
            return true;
        }
        let mut pkg = Some(self.file_pkgs[self.env.file.0 as usize]);
        while let Some(p) = pkg {
            if self.syms.pkg(p).name == scope {
                return true;
            }
            pkg = self.syms.pkg(p).parent;
        }
        false
    }

    /// A private constructor takes the `apply` and `copy` of a case class with it.
    pub(super) fn check_ctor_access(&mut self, c: ClassId, span: Span) {
        if self.ctor_accessible(c) {
            return;
        }
        let name = self.name_str(self.syms.class(c).name);
        let msg = if self.syms.class(c).mods & crate::ast::mods::PRIVATE_CTOR != 0 {
            format!("the constructor of {0} is private to {0}", name)
        } else {
            let from = self.this_class().or(self.parent_args_of).map_or("here".to_string(), |k| self.class_description(k));
            format!("constructor {0} cannot be accessed as a member of {0} from {1}", name, from)
        };
        self.error(span, msg);
    }

    /// A private constructor is reachable inside its class and companion, a protected one in
    /// the subclasses as well, and either one inside its `private[scope]`.
    pub(super) fn ctor_accessible(&mut self, c: ClassId) -> bool {
        let info = self.syms.class(c);
        if self.in_jar(info.file) {
            let file = self.env.file;
            if self.in_jar(file) || self.is_body_file(file) || self.source(file).is_std {
                return true;
            }
        }
        let info = self.syms.class(c);
        let (private, protected) = (info.mods & crate::ast::mods::PRIVATE_CTOR != 0, info.mods & crate::ast::mods::PROTECTED_CTOR != 0);
        if !private && !protected {
            return true;
        }
        let companion = info.companion.or_else(|| self.inner_companion(c));
        let derives = |t: &Self, k: ClassId| protected && t.syms.class(k).base_types.iter().any(|&(b, _)| b == c);
        let inside = self.env.frames.iter().any(|f| match f {
            super::Frame::Class(f) => *f == c || Some(*f) == companion || derives(self, *f),
            _ => false,
        });
        // The arguments of a parent clause stand outside the class being defined.
        let in_subclass = self.parent_args_of.map_or(false, |k| derives(self, k));
        inside || in_subclass || self.in_access_scope(info.file, info.span.end, Owner::Class(c))
    }

    pub(super) fn member_callee_in(&mut self, recv: TExprId, recv_ty: TypeId, owner_ty: Option<TypeId>, s: SymId) -> Callee {
        if let SymKind::Overloaded(_) = self.syms.sym(s).kind {
            return Callee::Overloaded { recv: Some(recv), recv_ty: Some(recv_ty), set: s };
        }
        let owner_subst = match owner_ty {
            Some(t) => self.owner_subst(t),
            None => Vec::new(),
        };
        if self.is_method_sym(s) {
            let prefix = self.dependent_prefix(s, recv, recv_ty);
            if prefix.is_some() {
                self.note_outer_prefix(s, recv);
            }
            if self.is_inline_callee(s) {
                self.inline.recv_types.insert(recv, recv_ty);
            }
            return Callee::Method { recv: Some(recv), sym: s, owner_subst, prefix };
        }
        let ret = self.sig_of(s).ret;
        let mut ty = self.types.subst(ret, &owner_subst);
        if self.types.has_paths(ty) {
            let prefix = self.prefix_of(recv, recv_ty);
            ty = self.seen_from_prefix(ty, prefix, s);
            if self.member_of_inner_class(s) {
                ty = self.seen_from_outer_steps(s, recv, ty);
                ty = self.seen_from_enclosing_this(ty);
            }
        }
        if let Some(c) = self.module_alias(s, ty) {
            let te = self.prog.add(TExpr::Module(c));
            if self.deps.is_some() {
                self.deps_node(te, super::deps::Node::AliasVal(s));
            }
            // The value the val was read on (`q` of `q.reflect`, the reflection API's object).
            if self.capturing() && !matches!(self.prog.expr(recv), TExpr::This | TExpr::Module(_)) {
                self.capture_receiver(te, recv);
            }
            return Callee::Value(te, ty);
        }
        let info = self.syms.sym(s);
        if let (Some(import), false) = (info.js_import, matches!(info.kind, SymKind::Object(_))) {
            let te = self.prog.add(TExpr::JsImport(import));
            if self.capturing() {
                self.capture_form(te, Form::Member(s));
            }
            return Callee::Value(te, ty);
        }
        if let Some(global) = info.js_global {
            let te = self.prog.add(TExpr::JsGlobal(global, false));
            if self.capturing() {
                self.capture_form(te, Form::Member(s));
            }
            return Callee::Value(te, ty);
        }
        let te = match self.syms.sym(s).kind {
            // A prefix that is not a path is evaluated before the object is reached.
            SymKind::Object(c) => {
                let module = self.prog.add(TExpr::Module(c));
                if self.is_path(recv) {
                    // The path the object was reached through (`q.reflect` of the reflection
                    // API's `q.reflect.TypeRepr`), which the pickle selects it on.
                    if self.capturing() && !matches!(self.prog.expr(recv), TExpr::This) {
                        self.capture_receiver(module, recv);
                    }
                    module
                } else {
                    if self.capturing() {
                        self.capture_receiver(module, recv);
                    }
                    let stmts = self.prog.stmts.push_slice(&[TStmt::Expr(recv)]);
                    self.prog.add(TExpr::Block(stmts, module))
                }
            }
            SymKind::EnumValue(_) => self.prog.add(TExpr::Static(s)),
            _ => match self.constant_of(s) {
                // A constant member stands for its value behind a stable path, as in Scala,
                // where its object need not be initialised; behind any other receiver the
                // selection stays, with the receiver's effects and its dereference.
                Some(literal) if self.is_stable_path(recv) => {
                    let te = self.prog.add(literal);
                    self.folded_paths.insert(te, (recv, s));
                    if self.deps.is_some() {
                        self.deps_node(te, super::deps::Node::Constant(s));
                    }
                    if self.capturing() && matches!(self.prog.expr(recv), TExpr::This | TExpr::Module(_)) {
                        self.capture_form(te, crate::tir::capture::Form::Constant(s));
                    }
                    te
                }
                _ => {
                    let field = self.prog.add(TExpr::Field(recv, s));
                    if self.syms.sym(s).by_name {
                        self.prog.add(TExpr::CallClosure(field, ListRef::EMPTY))
                    } else {
                        field
                    }
                }
            },
        };
        Callee::Value(te, ty)
    }

    /// The prefix a member's signature has to be seen from, when it depends on `this`.
    pub(super) fn dependent_prefix(&mut self, s: SymId, recv: TExprId, recv_ty: TypeId) -> Option<TypeId> {
        self.sig_of(s);
        let sig = self.syms.sig(s);
        let depends = self.types.has_paths(sig.ret)
            || sig.clauses.iter().any(|cl| cl.params.iter().any(|p| self.types.has_paths(p.ty)))
            || sig.tparams.iter().any(|&p| self.types.has_paths(self.syms.tparam(p).upper) || self.types.has_paths(self.syms.tparam(p).lower));
        depends.then(|| self.prefix_of(recv, recv_ty))
    }

    /// A member of an inner class whose signature names the enclosing class's `this`
    /// (`WhenRequest.thenRespond(..): Self` for `AbstractBackendStub.this.Self`): the outer
    /// instance is the receiver of the call that gave the inner one (`stub.whenRequestMatches(..)`),
    /// noted by the receiver expression for the application to see the signature through.
    #[cold]
    #[inline(never)]
    pub(super) fn note_outer_prefix(&mut self, s: SymId, recv: TExprId) {
        let Owner::Class(inner) = self.syms.sym(s).owner else { return };
        let sig = self.sig_arc(s);
        let steps = self.outer_steps(inner, recv, |t, outer| {
            t.mentions_this_of(sig.ret, outer) || sig.clauses.iter().any(|cl| cl.params.iter().any(|p| t.mentions_this_of(p.ty, outer)))
        });
        if !steps.is_empty() {
            self.outer_prefixes.insert(recv, steps);
        }
    }

    /// For each class enclosing `inner` whose `this` the member's type names (`mentions`), the
    /// path that stands for it: the nearest value up the receiver chain whose class is that
    /// class, derives from it or has it in its self type, reached through values of inner
    /// classes only (`GatewayCommons.this.PrependDefinitionsTo.prependVal(..)`,
    /// `$this.Type.Implicits.AnyType` for `Types.this.Type[Any]`).
    fn outer_steps(&mut self, inner: ClassId, recv: TExprId, mentions: impl Fn(&mut Self, ClassId) -> bool) -> Vec<(ClassId, TypeId)> {
        let mut steps = Vec::new();
        let mut inner = inner;
        let mut at = recv;
        for _ in 0..8 {
            let Owner::Class(outer) = self.syms.class(inner).owner else { break };
            if self.syms.class(outer).kind == ClassKind::Object {
                inner = outer;
                continue;
            }
            let mut found = None;
            for _ in 0..4 {
                let r = match self.prog.expr(at) {
                    TExpr::CallMethod(r, _, _) | TExpr::Field(r, _) => r,
                    _ => break,
                };
                let Some(rt) = self.prog.type_of(r) else { break };
                let rt = self.deref(rt);
                if self.instance_of_class(rt, outer) {
                    found = Some((r, rt));
                    break;
                }
                let Some(c) = self.class_of(rt) else { break };
                if !matches!(self.syms.class(c).owner, Owner::Class(o) if self.syms.class(o).kind != ClassKind::Object) {
                    break;
                }
                at = r;
            }
            let Some((r, rt)) = found else { break };
            if mentions(self, outer) {
                let path = self.prefix_of(r, rt);
                steps.push((outer, path));
            }
            inner = outer;
            at = r;
        }
        steps
    }

    /// Whether a value of type `t` is an instance of `k`: its class is `k`, derives from it or
    /// has it in its self type, or a part of an intersection is one (`u: SelfDefs & SelfUses`).
    fn instance_of_class(&mut self, t: TypeId, k: ClassId) -> bool {
        let under = if self.types.is_path(t) { self.widen_path(t) } else { t };
        let under = self.dealias(under);
        if let Type::Inter(a, b) = self.types.get(under) {
            return self.instance_of_class(a, k) || self.instance_of_class(b, k);
        }
        match self.class_of(under) {
            Some(c) => c == k || self.syms.class(c).base_types.iter().any(|&(b, _)| b == k) || self.self_type_derives(c, k),
            None => false,
        }
    }

    /// A value member's type with the `this` of the classes enclosing its owner seen from the
    /// receiver chain's paths.
    #[cold]
    #[inline(never)]
    fn seen_from_outer_steps(&mut self, s: SymId, recv: TExprId, ty: TypeId) -> TypeId {
        let Owner::Class(inner) = self.syms.sym(s).owner else { return ty };
        let steps = self.outer_steps(inner, recv, |t, outer| t.mentions_this_of(ty, outer));
        let mut ty = ty;
        for (outer, path) in steps {
            ty = self.as_seen_from(ty, path, outer);
        }
        ty
    }

    pub(super) fn mentions_this_of(&self, t: TypeId, k: ClassId) -> bool {
        if !self.types.has_paths(t) {
            return false;
        }
        match self.types.get(t) {
            Type::This(c) => c == k,
            Type::Select(p, _) | Type::Member(p, _) | Type::Lambda(_, p) | Type::Poly(_, p) => self.mentions_this_of(p, k),
            Type::AppMember(m, args) => self.mentions_this_of(m, k) || self.types.items(args).iter().any(|&a| self.mentions_this_of(a, k)),
            Type::Class(_, args) | Type::AppParam(_, args) | Type::AppVar(_, args) | Type::Alias(_, args) => {
                self.types.items(args).iter().any(|&a| self.mentions_this_of(a, k))
            }
            Type::Union(a, b) | Type::Inter(a, b) => self.mentions_this_of(a, k) || self.mentions_this_of(b, k),
            Type::Refined(parent, r) => {
                self.mentions_this_of(parent, k)
                    || match self.types.refinement(r) {
                        Refinement::Alias(_, x) => self.mentions_this_of(x, k),
                        Refinement::Bounds(_, lo, hi) => self.mentions_this_of(lo, k) || self.mentions_this_of(hi, k),
                        Refinement::Val(_, _, x) => self.mentions_this_of(x, k),
                        Refinement::Term(..) => false,
                    }
            }
            _ => false,
        }
    }

    /// The type of `member` seen from `prefix` of class `c`. A member declared in `c` itself
    /// moves only `c.this`: an ancestor's `this` written in it is an enclosing instance
    /// (`lState: Schedule.this.State` in an anonymous `Schedule`).
    fn member_seen_from(&mut self, t: TypeId, prefix: TypeId, c: ClassId, member: SymId) -> TypeId {
        if self.syms.sym(member).owner == Owner::Class(c) {
            self.own_seen_from(t, prefix, c)
        } else {
            self.as_seen_from(t, prefix, c)
        }
    }

    /// A member type as seen from the prefix of the receiver it is selected from: `C.this.T`
    /// in the member's declaration is the receiver's `T`.
    pub(super) fn seen_from_prefix(&mut self, t: TypeId, prefix: TypeId, member: SymId) -> TypeId {
        let Some(c) = self.class_of(prefix) else { return t };
        let t = self.member_seen_from(t, prefix, c, member);
        if !self.types.has_paths(t) {
            return t;
        }
        match self.intersection_prefix(prefix) {
            Some(whole) => self.seen_from_parts(t, prefix, c, whole),
            None => t,
        }
    }

    /// The intersection a prefix stands for, `C & T` for a trait `T` with self type `C`,
    /// whose parts are each a `this` its members are seen from.
    fn intersection_prefix(&mut self, prefix: TypeId) -> Option<TypeId> {
        let whole = match self.types.get(prefix) {
            Type::Inter(..) => prefix,
            Type::Term(_) | Type::Select(..) => self.widen_path(prefix),
            _ => return None,
        };
        matches!(self.types.get(whole), Type::Inter(..)).then_some(whole)
    }

    fn seen_from_parts(&mut self, t: TypeId, prefix: TypeId, c: ClassId, whole: TypeId) -> TypeId {
        let mut parts = vec![whole];
        let mut t = t;
        while let Some(part) = parts.pop() {
            match self.types.get(part) {
                Type::Inter(a, b) => {
                    parts.push(b);
                    parts.push(a);
                }
                _ => {
                    if let Some(k) = self.class_of(part).filter(|&k| k != c) {
                        t = self.as_seen_from(t, prefix, k);
                    }
                }
            }
        }
        t
    }

    /// Whether `s` is declared in a class nested in another class, directly or through objects
    /// nested in it, whose types may name the
    /// enclosing class's `this`.
    pub(super) fn member_of_inner_class(&self, s: SymId) -> bool {
        let Owner::Class(c) = self.syms.sym(s).owner else { return false };
        self.class_in_class(c)
    }

    /// Whether `c` is nested in a class, directly or through objects nested in it.
    pub(super) fn class_in_class(&self, c: ClassId) -> bool {
        let mut c = c;
        for _ in 0..8 {
            match self.syms.class(c).owner {
                Owner::Class(k) if self.syms.class(k).kind == ClassKind::Object => c = k,
                Owner::Class(_) => return true,
                _ => return false,
            }
        }
        false
    }

    /// `Outer.this` left in a member's type by a prefix that is no `Outer` (a member of an
    /// inner class, `at[T, A].apply(..)`) is the enclosing instance where the selection
    /// stands inside a class deriving from `Outer` (`object Fold extends BinaryPolyFunc`) or having it
    /// in its self type (`e.Underlying` of a cake's existential inside `this: Exs =>`).
    #[cold]
    #[inline(never)]
    pub(super) fn seen_from_enclosing_this(&mut self, t: TypeId) -> TypeId {
        if !self.types.has_paths(t) {
            return t;
        }
        let classes: Vec<ClassId> = self.env.frames.iter().rev().filter_map(|f| match f {
            super::Frame::Class(d) => Some(*d),
            _ => None,
        }).collect();
        let mut t = t;
        for d in classes {
            if self.syms.class(d).base_types.len() > 1 || self.syms.class(d).declared_self.is_some() {
                let prefix = self.this_prefix(d);
                t = self.as_seen_from(t, prefix, d);
            }
        }
        t
    }

    /// The signature of an inner class's member with a leftover `Outer.this` seen from the
    /// enclosing instance, where the call stands in a class deriving from `Outer`.
    #[cold]
    #[inline(never)]
    fn sig_seen_from_enclosing(&mut self, sig: Arc<MethodSig>) -> Arc<MethodSig> {
        let ret = self.seen_from_enclosing_this(sig.ret);
        let clauses: Vec<ClauseSig> = sig
            .clauses
            .iter()
            .map(|cl| ClauseSig {
                params: cl.params.iter().map(|p| ParamSig { ty: self.seen_from_enclosing_this(p.ty), ..p.clone() }).collect(),
                is_using: cl.is_using,
                is_implicit: cl.is_implicit,
            })
            .collect();
        Arc::new(MethodSig { tparams: sig.tparams.clone(), clauses, ret })
    }

    /// The signature with the enclosing class's `this` seen from the outer instance's prefix.
    #[cold]
    #[inline(never)]
    pub(super) fn sig_seen_from_outer(&mut self, sig: Arc<MethodSig>, prefix: TypeId, outer: ClassId) -> Arc<MethodSig> {
        let ret = self.as_seen_from(sig.ret, prefix, outer);
        let clauses: Vec<ClauseSig> = sig
            .clauses
            .iter()
            .map(|cl| ClauseSig {
                params: cl.params.iter().map(|p| ParamSig { ty: self.as_seen_from(p.ty, prefix, outer), ..p.clone() }).collect(),
                is_using: cl.is_using,
                is_implicit: cl.is_implicit,
            })
            .collect();
        Arc::new(MethodSig { tparams: sig.tparams.clone(), clauses, ret })
    }

    /// The signature of a member as seen from the prefix of the call's receiver.
    pub(super) fn sig_seen_from(&mut self, sig: Arc<MethodSig>, prefix: TypeId, member: SymId) -> Arc<MethodSig> {
        let Some(c) = self.class_of(prefix) else { return sig };
        let mut whole: Option<Option<TypeId>> = None;
        let mut seen = |t: &mut Self, ty: TypeId| {
            let ty = t.member_seen_from(ty, prefix, c, member);
            if !t.types.has_paths(ty) {
                return ty;
            }
            let w = *whole.get_or_insert_with(|| t.intersection_prefix(prefix));
            match w {
                Some(w) => t.seen_from_parts(ty, prefix, c, w),
                None => ty,
            }
        };
        let ret = seen(self, sig.ret);
        let clauses: Vec<ClauseSig> = sig
            .clauses
            .iter()
            .map(|cl| ClauseSig {
                params: cl.params.iter().map(|p| ParamSig { ty: seen(self, p.ty), ..p.clone() }).collect(),
                is_using: cl.is_using,
                is_implicit: cl.is_implicit,
            })
            .collect();
        Arc::new(MethodSig { tparams: sig.tparams.clone(), clauses, ret })
    }

    /// `final val x = 1` without a type is a constant in Scala: a use is replaced by the literal
    /// (`constant_value`), another worker's initialiser read through its chunk.
    fn constant_of(&mut self, s: SymId) -> Option<TExpr> {
        let mut s = s;
        for _ in 0..8 {
            if let Some(literal) = self.constant_by_signature(s) {
                return Some(literal);
            }
            match self.constant_step(s)? {
                Ok(literal) => return Some(literal),
                Err(m) => s = m,
            }
        }
        None
    }

    /// A `final` or `inline` val of a literal type (`final val x: 1 = 1`, a lazy or an implicit
    /// one too, and a pickle's `final val x = 1`, which scalac pickles with its constant type) is
    /// a constant by its signature, whichever worker types its initialiser. A given is none.
    fn constant_by_signature(&mut self, s: SymId) -> Option<TExpr> {
        use crate::ast::mods;
        let info = self.syms.sym(s);
        if info.mods & (mods::FINAL | mods::INLINE) == 0 || info.mods & mods::GIVEN != 0 || info.kind != SymKind::Val {
            return None;
        }
        let untyped_source = !self.in_jar(info.file) && info.def.is_some_and(|d| matches!(self.ast(info.file).def(d).kind, crate::ast::DefKind::Val { ty: None, .. }));
        if untyped_source {
            return None;
        }
        self.sig_of(s);
        let crate::types::Type::Lit(l) = self.types.get(self.syms.sig(s).ret) else { return None };
        Some(match self.types.lit_val(l) {
            crate::types::LitVal::Int(v) => TExpr::Int(v),
            crate::types::LitVal::Long(v) => TExpr::Long(v),
            crate::types::LitVal::Double(bits) => TExpr::Double(f64::from_bits(bits)),
            crate::types::LitVal::Char(c) => TExpr::Char(c),
            crate::types::LitVal::Bool(b) => TExpr::Bool(b),
            crate::types::LitVal::Str(n) => TExpr::Str(self.prog.add_str(self.interner.get(n))),
        })
    }

    /// which makes it available before its object is initialised. So is every `inline val`, and
    /// a `final val` defined as such a member of another value (`final val flag = getC().yes`),
    /// whose selection the typer keeps for the receiver's sake.
    pub(crate) fn constant_value(&self, s: SymId) -> Option<TExpr> {
        let mut s = s;
        for _ in 0..8 {
            match self.constant_step(s)? {
                Ok(literal) => return Some(literal),
                Err(m) => s = m,
            }
        }
        None
    }

    /// A constant val's literal, or the member its initialiser selects.
    fn constant_step(&self, s: SymId) -> Option<Result<TExpr, SymId>> {
        let info = self.syms.sym(s);
        if info.mods & (crate::ast::mods::FINAL | crate::ast::mods::INLINE) == 0 || info.kind != SymKind::Val {
            return None;
        }
        let untyped = info.mods & crate::ast::mods::INLINE != 0
            || matches!(self.ast(info.file).def(info.def?).kind, crate::ast::DefKind::Val { ty: None, .. })
            || info.sig.as_ref().is_some_and(|sig| matches!(self.types.get(sig.ret), crate::types::Type::Lit(_)));
        if !untyped {
            return None;
        }
        let at = *self.val_init.get(&s)?;
        super::bundle::entered_at(&self.prog.exprs, at.0, super::bundle::Entry::Val);
        // `final val k = (2: Int)` is an `Int`, no constant, as scalac types it, and so is a
        // plain inline call's.
        if self.prog.is_widened(at) || self.prog.is_opaque(at) {
            return None;
        }
        let init = self.prog.expr(at);
        match init {
            TExpr::Int(_) | TExpr::Long(_) | TExpr::Double(_) | TExpr::Bool(_) | TExpr::Char(_) | TExpr::Str(_) => Some(Ok(init)),
            TExpr::Field(_, m) | TExpr::Static(m) => Some(Err(m)),
            _ => None,
        }
    }

    /// What an application's head names: an extension method of a class or an object the name is
    /// bound to as its selection on that receiver (`Callee::Selection`).
    fn resolve_ident(&mut self, name: Name, span: Span) -> Option<Callee> {
        let r = self.lookup_ident(name, span)?;
        if let Some((recv, recv_ty)) = self.extension_binding_receiver(r, span) {
            // The extension's own name: an import may have renamed it (`import O.{tag as t}`).
            let name = r.sym().map_or(name, |s| self.syms.sym(s).name);
            return Some(Callee::Selection { recv, recv_ty, name });
        }
        self.term_callee(r, name, span)
    }

    /// The binding of a name an application's head writes, its errors reported.
    #[inline]
    fn lookup_ident(&mut self, name: Name, span: Span) -> Option<TermRef> {
        let Some(r) = self.lookup_term_at(name, span) else {
            // The self call of a secondary constructor sees no member of the class yet.
            let hidden_member = self.excluded_ctor.is_some()
                && self.parent_args_of.map_or(false, |c| {
                    let self_ty = self.syms.this_type(c);
                    self.find_member(self_ty, name).is_some()
                });
            if hidden_member {
                let msg = format!("{} is not accessible from constructor arguments", self.name_str(name));
                self.error(span, msg);
            } else {
                let msg = format!("not found: {}", self.name_str(name));
                self.not_found_error(name, span, msg);
            }
            return None;
        };
        if let (Some(case), TermRef::ModuleMember(c, _)) = (self.parent_args_of, r) {
            if self.syms.class(case).owner == Owner::Class(c) && self.syms.class(case).kind == ClassKind::EnumCase {
                let msg = format!(
                    "illegal reference to {0} from an enum case; write {1}.{0}",
                    self.name_str(name),
                    self.name_str(self.syms.class(c).name)
                );
                self.error(span, msg);
            }
        }
        Some(r)
    }

    /// The receiver an extension method of a class or an object that `r` binds is selected on:
    /// the object, the class's instance or the imported value, whose selection of the name
    /// applies the extension with the first argument list as its receiver's.
    #[inline]
    fn extension_binding_receiver(&mut self, r: TermRef, span: Span) -> Option<(TExprId, TypeId)> {
        let (TermRef::ModuleMember(_, s) | TermRef::This(_, s) | TermRef::ValueMember(_, s)) = r else { return None };
        if !self.syms.sym(s).is_extension {
            return None;
        }
        self.extension_receiver_of(r, span)
    }

    #[cold]
    #[inline(never)]
    fn extension_receiver_of(&mut self, r: TermRef, span: Span) -> Option<(TExprId, TypeId)> {
        match r {
            TermRef::ModuleMember(c, _) => Some((self.module_ref(c, span)?, self.types.class(c, &[]))),
            TermRef::This(c, _) => {
                self.complete_class(c);
                let ty = self.inline_this_of(c).unwrap_or_else(|| self.syms.this_type(c));
                Some((self.this_ref(c), ty))
            }
            TermRef::ValueMember(v, _) => self.import_value_ref(v, span),
            _ => None,
        }
    }

    /// The object `c` read where its name is: a local object's lazy val, an object nested in a
    /// class on the enclosing instance's.
    fn module_ref(&mut self, c: ClassId, span: Span) -> Option<TExprId> {
        Some(match (self.syms.class(c).local_module, self.syms.class(c).inner_object) {
            (Some(v), _) => self.prog.add(TExpr::Local(v)),
            (None, Some(v)) => match self.inner_object_ref(v) {
                Some(e) => e,
                None => {
                    let msg = format!("{} is not accessible from here: it belongs to an instance of its enclosing class", self.name_str(self.syms.class(c).name));
                    self.error(span, msg);
                    return None;
                }
            },
            (None, None) => self.prog.add(TExpr::Module(c)),
        })
    }

    pub(super) fn term_callee(&mut self, r: TermRef, name: Name, span: Span) -> Option<Callee> {
        Some(match r {
            TermRef::SelfAlias(c) => {
                self.complete_class(c);
                let (te, ty) = self.type_this_of(c);
                Callee::Value(te, ty)
            }
            TermRef::Local(s) => {
                if let Some((te, ty)) = self.inline_arg(s) {
                    // A constant `inline val`'s read, its value: the capture keeps the reference.
                    if self.capturing() && self.prog.capture.as_deref().and_then(|c| c.local_of(s)).map_or(false, |l| l.mods & crate::ast::mods::INLINE != 0) {
                        self.capture_form(te, crate::tir::capture::Form::Constant(s));
                    }
                    Callee::Value(te, ty)
                } else if self.syms.sym(s).kind == SymKind::Def {
                    Callee::Method { recv: None, sym: s, owner_subst: Vec::new(), prefix: None }
                } else if let Some(c) = self.enclosing_local_object(s) {
                    let ty = self.sig_of(s).ret;
                    Callee::Value(self.this_ref(c), ty)
                } else {
                    let ty = self.sig_of(s).ret;
                    let local = self.prog.add(TExpr::Local(s));
                    let te = if self.syms.sym(s).by_name {
                        self.prog.add(TExpr::CallClosure(local, ListRef::EMPTY))
                    } else {
                        local
                    };
                    Callee::Value(te, ty)
                }
            }
            TermRef::This(c, s) => {
                self.complete_class(c);
                // Inside an inline expansion `this` is the receiver, whose class may
                // implement what the method's own class only declares.
                let (self_ty, s) = match self.inline_this(c) {
                    Some((_, t)) => (t, self.find_member(t, name).map_or(s, |(m, _)| m)),
                    None => (self.syms.this_type(c), s),
                };
                let this = self.this_ref(c);
                self.member_callee(this, self_ty, s)
            }
            TermRef::ModuleMember(c, s) => {
                let module_ty = self.types.class(c, &[]);
                // A member imported from a local object is selected on the object's lazy val,
                // one from an object nested in a class on the enclosing instance's.
                let module = self.module_ref(c, span)?;
                self.member_callee(module, module_ty, s)
            }
            TermRef::Global(s) => self.global_callee(s),
            TermRef::ValueMember(v, m) => {
                let (recv, recv_ty) = self.import_value_ref(v, span)?;
                self.member_callee(recv, recv_ty, m)
            }
            TermRef::Class(c) => Callee::Ctor(c),
            TermRef::Package(_) => {
                let msg = format!("{} is a package, not a value", self.name_str(name));
                self.error(span, msg);
                return None;
            }
        })
    }

    /// The value an import selects members of (`import v.*`, `import o.v.x`): a local, a
    /// member of an object or a package's val, read as its name reads it.
    pub(super) fn import_value_ref(&mut self, value: super::ValueImport, span: Span) -> Option<(TExprId, TypeId)> {
        let (v, module, through) = self.import_values[value.0 as usize];
        if let Some(prev) = through {
            let (recv, recv_ty) = self.import_value_ref(prev, span)?;
            let callee = self.member_callee(recv, recv_ty, v);
            return Some(self.apply_callee(callee, None, Vec::new(), span, None));
        }
        self.stable_value_ref(v, module, span)
    }

    fn stable_value_ref(&mut self, v: SymId, module: Option<ClassId>, span: Span) -> Option<(TExprId, TypeId)> {
        let prefix = match (module, self.syms.sym(v).owner) {
            // A val of an enclosing class is read on that class's instance.
            (Some(c), _) if self.syms.class(c).kind != ClassKind::Object => TermRef::This(c, v),
            (Some(m), _) => TermRef::ModuleMember(m, v),
            (None, Owner::Class(c)) => TermRef::ModuleMember(c, v),
            (None, Owner::Local) => TermRef::Local(v),
            (None, Owner::Package(_)) => TermRef::Global(v),
        };
        let name = self.syms.sym(v).name;
        let callee = self.term_callee(prefix, name, span)?;
        Some(self.apply_callee(callee, None, Vec::new(), span, None))
    }

    /// The singleton type of the value an import reads: `o.R.S.type` through the values it is
    /// selected on.
    pub(super) fn import_value_type(&mut self, value: super::ValueImport) -> TypeId {
        let (v, _, through) = self.import_values[value.0 as usize];
        match through {
            Some(prev) => {
                let head = self.import_value_type(prev);
                self.types.mk(Type::Select(head, v))
            }
            None => self.import_value_prefix(value),
        }
    }

    /// The type of the val an import selects on: its declared one, seen from the instance of
    /// the enclosing class it is read on where it is that class's (an inherited `val value: X`
    /// of a `trait W[A] extends E[(A, A)]` is an `(A, A)`).
    pub(super) fn import_value_ret(&mut self, value: super::ValueImport) -> TypeId {
        let (v, module, through) = self.import_values[value.0 as usize];
        let ret = self.sig_of(v).ret;
        if let (None, Some(c)) = (through, module) {
            if self.syms.class(c).kind != ClassKind::Object {
                let this = self.this_prefix(c);
                let name = self.syms.sym(v).name;
                if let Some((s, owner_ty)) = self.find_member(this, name) {
                    if s == v {
                        let subst = self.owner_subst(owner_ty);
                        return self.types.subst(ret, &subst);
                    }
                }
            }
        }
        ret
    }

    /// The path of the val an import selects on, the prefix of the types it brings: `v.type`,
    /// or `C.this.v` for a val of an enclosing class's instance, as it is read elsewhere.
    pub(super) fn import_value_prefix(&mut self, value: super::ValueImport) -> TypeId {
        let (v, module, through) = self.import_values[value.0 as usize];
        match (through, module) {
            (None, Some(c)) if self.syms.class(c).kind != ClassKind::Object => {
                let this = self.this_prefix(c);
                self.types.mk(Type::Select(this, v))
            }
            _ => self.types.mk(Type::Term(v)),
        }
    }

    /// `callee(args)` of a written name: a case class's name applies its companion's `apply`.
    fn apply_named_callee(
        &mut self,
        callee: Callee,
        targs: Option<ListRef>,
        lists: Vec<ArgList>,
        span: Span,
        expected: Option<TypeId>,
    ) -> (TExprId, TypeId) {
        if self.deps.is_none() {
            return self.apply_callee(callee, targs, lists, span, expected);
        }
        self.apply_named_callee_recorded(callee, targs, lists, span, expected)
    }

    #[cold]
    #[inline(never)]
    fn apply_named_callee_recorded(
        &mut self,
        callee: Callee,
        targs: Option<ListRef>,
        lists: Vec<ArgList>,
        span: Span,
        expected: Option<TypeId>,
    ) -> (TExprId, TypeId) {
        let case_class = match callee {
            Callee::Ctor(c) if self.syms.class(c).mods & crate::ast::mods::CASE != 0 || self.syms.class(c).kind == ClassKind::EnumCase => Some(c),
            _ => None,
        };
        let (te, ty) = self.apply_callee(callee, targs, lists, span, expected);
        if let Some(c) = case_class {
            self.deps_node(te, super::deps::Node::Apply(c));
        }
        (te, ty)
    }

    pub fn apply_callee(
        &mut self,
        callee: Callee,
        targs: Option<ListRef>,
        lists: Vec<ArgList>,
        span: Span,
        expected: Option<TypeId>,
    ) -> (TExprId, TypeId) {
        match callee {
            // A method whose header the parser could not complete takes any arguments: they are
            // typed on their own and the application has the error type.
            Callee::Method { sym, .. } if self.recovered && self.syms.sym(sym).mods & crate::ast::mods::INCOMPLETE != 0 => {
                self.type_args_for_errors(&lists);
                (self.prog.add(TExpr::Unit), ERROR)
            }
            Callee::Method { sym, .. } if self.is_dynamic_literal_with_names(sym, &lists) => {
                self.dynamic_literal(sym, lists, span, expected)
            }
            Callee::Method { recv, sym, owner_subst, prefix } => {
                if self.is_right_assoc_extension(sym) {
                    return self.apply_right_assoc_method(recv, sym, owner_subst, prefix, targs, lists, span, expected);
                }
                let call = MethodCall { recv, sym, owner_subst, ext_recv: None, prefix };
                self.apply_method(call, None, targs, lists, span, expected, false).unwrap()
            }
            Callee::Overloaded { recv, recv_ty, set } => {
                self.apply_overloaded(recv, recv_ty, set, targs, lists, span, expected)
            }
            Callee::Selection { recv, recv_ty, name } => self.apply_member(recv, recv_ty, name, targs, lists, span, expected),
            Callee::Ctor(c) => {
                if let (Some(ta), true) = (targs, lists.is_empty()) {
                    if let Some(r) = self.summon_type_application(c, None, ta, span, expected) {
                        return r;
                    }
                }
                // The constructor of a `new`, taken before its arguments are typed: an application
                // of a class's name among them is one.
                let explicit = std::mem::take(&mut self.explicit_new);
                let (te, ty) = self.apply_ctor(c, targs, lists, span, expected);
                if !explicit && self.capturing() && self.syms.class(c).mods & crate::ast::mods::CASE != 0 && matches!(self.prog.expr(te), TExpr::New(..)) {
                    self.capture_form(te, Form::CaseApply);
                }
                (te, ty)
            }
            Callee::Value(te, ty) => {
                if self.loaded.is_some() {
                    self.report_blocked_type(ty, span);
                }
                if lists.is_empty() && targs.is_none() {
                    return (te, ty);
                }
                // A polymorphic function value applied: its type parameters take the explicit
                // type arguments, or fresh variables the arguments settle.
                let head = self.deref(ty);
                if let Type::Poly(ps, fun) = self.types.get(head) {
                    let ps = self.types.items(ps).to_vec();
                    let explicit: Vec<TypeId> = match targs {
                        Some(l) => {
                            let ids = self.cur_ast().ty_list(l).to_vec();
                            ids.iter().map(|&t| self.resolve_type(t)).collect()
                        }
                        None => Vec::new(),
                    };
                    if !explicit.is_empty() && explicit.len() != ps.len() {
                        self.error(span, "wrong number of type arguments");
                    }
                    let subst: Subst = ps
                        .iter()
                        .enumerate()
                        .filter_map(|(i, &p)| match self.types.get(p) {
                            Type::Param(id) => Some((id, explicit.get(i).copied().unwrap_or_else(|| self.fresh_var()))),
                            _ => None,
                        })
                        .collect();
                    let fun = self.types.subst(fun, &subst);
                    if lists.is_empty() {
                        let targs: Vec<TypeId> = subst.iter().map(|&(_, t)| t).collect();
                        if let Some((ptys, ret)) = self.as_function(fun) {
                            return self.eta_poly_function(te, ty, fun, &ptys, ret, &targs, span);
                        }
                        // A context function's instance is applied to the givens in scope, as scalac
                        // applies `f[Int]` where no context function is expected.
                        if let Some((ptys, ret)) = self.as_context_function(fun) {
                            let mut args = Vec::with_capacity(ptys.len());
                            for &pty in &ptys {
                                let arg = match self.resolve_given(pty, span) {
                                    Some(a) => a,
                                    None => {
                                        let msg = format!("no given instance of type {} was found", self.show(pty));
                                        self.given_failure_error(span, msg, pty);
                                        self.prog.add(TExpr::Unit)
                                    }
                                };
                                args.push(arg);
                            }
                            let l = self.prog.list(&args);
                            let call = self.prog.add(TExpr::CallClosure(te, l));
                            self.prog.set_type(call, ret);
                            if self.capturing() {
                                self.capture_targs(call, &targs);
                            }
                            return (call, ret);
                        }
                    }
                    self.poly_targs = Some(subst.iter().map(|&(_, t)| t).collect());
                    let mark = self.poly_pending.len();
                    let typed = self.apply_callee(Callee::Value(te, fun), None, lists, span, expected);
                    // Inferred type arguments are fixed once every clause is typed, as a method's
                    // are: a curried function's later clauses settle them too.
                    let pending: Vec<(TExprId, Vec<TypeId>)> = self.poly_pending.drain(mark..).collect();
                    for (call, targs) in pending {
                        let targs: Vec<TypeId> = if explicit.is_empty() { targs.iter().map(|&t| self.solve_inferred(t)).collect() } else { targs };
                        if self.capturing() {
                            self.capture_targs(call, &targs);
                        }
                    }
                    return typed;
                }
                // A type argument the value's type leaves open (`Partial[T]` of a `withService[T]`)
                // is settled by the `apply` the arguments go to; a bare variable is solved.
                let head = self.deref(ty);
                let ty = match self.types.get(head) {
                    Type::Var(_) | Type::AppVar(..) => self.solve_in(ty),
                    _ => self.solve_bounded_in(ty),
                };
                let poly_targs = self.poly_targs.take();
                // A context function applied to an explicit `(using ...)` clause, or to the
                // givens in scope when its result takes the arguments.
                if let Some((ptys, ret)) = self.as_context_function(ty) {
                    if targs.is_some() {
                        self.error(span, "this value does not take type arguments");
                        return (te, ty);
                    }
                    let mut lists = lists;
                    // A library body passes the givens of a using clause as a plain list.
                    let body_file = self.is_body_file(self.env.file);
                    let explicit = lists.first().map_or(false, |l| (l.using || body_file) && l.args.len() == ptys.len());
                    let mut args = Vec::with_capacity(ptys.len());
                    let mark = self.hoisted.len();
                    let mut te = te;
                    let head = self.deref_alias(ty);
                    if explicit && self.named_fun_parent(head).is_some() {
                        // Named parameters take their arguments by name too, and a dependent
                        // result reads each argument's own type.
                        let first = lists.remove(0);
                        let clause = self.closure_clause(ty, &ptys);
                        let mut f = [te];
                        if self.type_clause_args(&clause, first, &Vec::new(), &mut args) {
                            self.hoist_before(mark, &mut f, span);
                        }
                        te = f[0];
                    } else if explicit {
                        let first = lists.remove(0);
                        for (arg, &pty) in first.args.into_iter().zip(&ptys) {
                            let ArgSrc::Ast(a) = arg else { continue };
                            args.push(self.check_expr(a, pty));
                        }
                    } else {
                        for &pty in &ptys {
                            let arg = match self.resolve_given(pty, span) {
                                Some(a) => a,
                                None => {
                                    let msg = format!("no given instance of type {} was found", self.show(pty));
                                    self.given_failure_error(span, msg, pty);
                                    self.prog.add(TExpr::Unit)
                                }
                            };
                            args.push(arg);
                        }
                    }
                    let ret = self.applied_named_result(ty, &args, &ptys).unwrap_or(ret);
                    let call = match self.beta_reduce(te, &args) {
                        Some(reduced) => reduced,
                        None => {
                            let l = self.prog.list(&args);
                            let call = self.prog.add(TExpr::CallClosure(te, l));
                            if let Some(targs) = &poly_targs {
                                self.poly_pending.push((call, targs.clone()));
                            }
                            call
                        }
                    };
                    let call = self.wrap_hoisted(mark, call);
                    self.prog.set_type(call, ret);
                    return self.apply_callee(Callee::Value(call, ret), None, lists, span, expected);
                }
                if let Some((ptys, ret)) = self.as_function(ty) {
                    if targs.is_some() {
                        self.error(span, "this value does not take type arguments");
                        return (te, ty);
                    }
                    if lists.is_empty() {
                        return (te, ty);
                    }
                    let mut lists = lists;
                    let first = lists.remove(0);
                    let applied = first.span;
                    let clause = self.closure_clause(ty, &ptys);
                    let mut args = Vec::new();
                    let mark = self.hoisted.len();
                    let mut f = [te];
                    if self.type_clause_args(&clause, first, &Vec::new(), &mut args) {
                        self.hoist_before(mark, &mut f, span);
                    }
                    let ret = self.applied_named_result(ty, &args, &ptys).unwrap_or(ret);
                    let call = match self.beta_reduce(f[0], &args) {
                        Some(reduced) => reduced,
                        None => {
                            let l = self.prog.list(&args);
                            let call = self.prog.add(TExpr::CallClosure(f[0], l));
                            if let Some(targs) = &poly_targs {
                                self.poly_pending.push((call, targs.clone()));
                            }
                            call
                        }
                    };
                    let call = self.wrap_hoisted(mark, call);
                    // The result the next list applies, at the application so far
                    // (`factory()(1)(|)`), for signature help.
                    if self.index.is_some() && !lists.is_empty() {
                        self.index_value(applied, call, ret);
                    }
                    return self.apply_callee(Callee::Value(call, ret), None, lists, span, expected);
                }
                // dotty's `tryInsertApplyOrImplicit`: the `apply` inserted for a value's arguments
                // is marked (`InsertedApply`) and none goes in after it (`isSyntheticApply`), so
                // an `apply` that is a value, or a method without parameters, takes none (E050).
                // Explicit type arguments make it no synthetic one (`Mk[Int](f)` for a
                // `def apply[A]: Mk[A]` applies the result's `apply`).
                if targs.is_none() && lists.first().is_some_and(|l| !l.using) {
                    if let Some(sym) = self.apply_taking_none(ty) {
                        let what = if self.syms.sym(sym).kind == SymKind::Def { "method" } else { "value" };
                        let msg = format!("{} {} does not take parameters", what, self.method_description(sym));
                        self.error(span, msg);
                        return (te, ERROR);
                    }
                }
                self.apply_member(te, ty, names::APPLY, targs, lists, span, expected)
            }
        }
    }

    /// `((x, y) => b)(a, c)` for arguments that are paths or literals is `b` with the parameters
    /// replaced, as scalac's inliner reduces a lambda argument applied inside an inline body
    /// (a macro sees `_.name` where `Focus` applies its context lambda to the keyword context).
    pub(super) fn beta_reduce(&mut self, f: TExprId, args: &[TExprId]) -> Option<TExprId> {
        let TExpr::Lambda(params, body) = self.prog.expr(f) else { return None };
        let params = self.prog.sym_list(params).to_vec();
        if params.len() != args.len() {
            return None;
        }
        let simple = args.iter().all(|&a| {
            matches!(
                self.prog.expr(a),
                TExpr::Local(_) | TExpr::Int(_) | TExpr::Long(_) | TExpr::Double(_) | TExpr::Bool(_) | TExpr::Char(_) | TExpr::Str(_) | TExpr::Unit | TExpr::Module(_) | TExpr::This | TExpr::Null
            )
        });
        if !simple {
            return None;
        }
        let map: crate::intern::FxMap<SymId, TExprId> = params.iter().copied().zip(args.iter().copied()).collect();
        let empty = crate::intern::FxMap::default();
        let renames = crate::intern::FxMap::default();
        // The body is copied: its pending plain calls are expanded first.
        if self.attempts.pending_len() != 0 {
            self.expand_pending_in(&[body]);
        }
        let reduced = self.instantiate_quote(body, &empty, &map, &Subst::default(), &renames);
        if self.capturing() {
            self.capture_builtin_call(reduced, names::APPLY, f, args.to_vec());
        }
        // The lambda an inline method's call expanded to, applied at once (`plain()(5)`): the
        // reduced body stands for that call in the language server's index, as the lambda did.
        if let Some(ix) = self.index.as_mut() {
            if let Some(&callee) = ix.expansions.get(&f) {
                if reduced != body && !args.contains(&reduced) {
                    ix.expansions.entry(reduced).or_insert(callee);
                }
            }
        }
        Some(reduced)
    }

    /// `C.apply`: the constructor as a method of the companion. Named without arguments it is a
    /// function value, and that of an enum case produces the enum as the constructor does.
    pub(super) fn apply_companion_apply(
        &mut self,
        c: ClassId,
        targs: Option<ListRef>,
        lists: Vec<ArgList>,
        span: Span,
        expected: Option<TypeId>,
    ) -> (TExprId, TypeId) {
        if !lists.is_empty() {
            let (te, ty) = self.apply_ctor(c, targs, lists, span, expected);
            if self.deps.is_some() {
                self.deps_node(te, super::deps::Node::Apply(c));
            }
            if self.capturing() && self.syms.class(c).mods & crate::ast::mods::CASE != 0 && matches!(self.prog.expr(te), TExpr::New(..)) {
                self.capture_form(te, Form::CaseApply);
            }
            return (te, ty);
        }
        let (te, ty) = self.construct(c, targs, lists, span, expected, true);
        let Some((params, ret)) = self.as_function(ty) else { return (te, ty) };
        let info = self.syms.class(c);
        let Some(parent) = info.parents.first().copied().filter(|_| info.kind == ClassKind::EnumCase) else {
            return (te, ty);
        };
        // Against an expected `Int => E.A` the function keeps the case type, as scalac's does.
        let case_expected = expected
            .and_then(|t| self.expected_function(t, params.len()))
            .map_or(false, |(_, ret)| self.expects_class(ret, c));
        if case_expected {
            return (te, ty);
        }
        let widened = self.widen_enum_case(ret, parent);
        (te, self.fun_type(&params, widened))
    }

    /// Whether the expected type `exp` asks for the class `c` itself: it is `c`, or an open
    /// variable that the enclosing expected type bounded above by `c` (`Z` of a `mapN[Z]` whose
    /// result `V[Z]` is expected to be a `V[Ch.Email]`).
    fn expects_class(&mut self, exp: TypeId, c: ClassId) -> bool {
        let exp = self.deref(exp);
        match self.types.get(exp) {
            Type::Class(ec, _) => ec == c,
            Type::Var(v) => self.upper_bounds_through_vars(v).into_iter().any(|u| matches!(self.types.get(u), Type::Class(ec, _) if ec == c)),
            _ => false,
        }
    }

    fn widen_enum_case(&mut self, precise: TypeId, parent: TypeId) -> TypeId {
        match self.types.get(precise) {
            Type::Class(_, args) if args != EMPTY_LIST => {
                self.class_of(parent).and_then(|e| self.base_type(precise, e)).unwrap_or(parent)
            }
            _ => parent,
        }
    }

    fn apply_ctor(
        &mut self,
        c: ClassId,
        targs: Option<ListRef>,
        lists: Vec<ArgList>,
        span: Span,
        expected: Option<TypeId>,
    ) -> (TExprId, TypeId) {
        self.construct(c, targs, lists, span, expected, false)
    }

    fn construct(
        &mut self,
        c: ClassId,
        targs: Option<ListRef>,
        lists: Vec<ArgList>,
        span: Span,
        expected: Option<TypeId>,
        as_function: bool,
    ) -> (TExprId, TypeId) {
        self.complete_class(c);
        // A class whose completion is under way has no parents yet: one constructed from an
        // initialiser that its own completion reads (`class C { val a = new D; import a.*;
        // class D }`, whose names `D` looks up through `a`) is a cyclic reference (E046).
        if self.syms.class(c).base_types.is_empty() {
            let msg = format!("cyclic reference involving class {}", self.name_str(self.syms.class(c).name));
            self.error(span, msg);
            self.type_args_for_errors(&lists);
            return (self.prog.add(TExpr::Unit), ERROR);
        }
        if self.loaded.is_some() && self.report_java_inner(c, span) {
            self.type_args_for_errors(&lists);
            return (self.prog.add(TExpr::Unit), ERROR);
        }
        if self.loaded.is_some() {
            self.absorb_java_ctors(c);
        }
        let secondaries: Vec<SymId> =
            self.syms.class(c).ctors.iter().copied().filter(|&s| Some(s) != self.excluded_ctor && self.ctor_callable_here(s)).collect();
        if secondaries.is_empty() {
            self.check_ctor_access(c, span);
        }
        let info = self.syms.class(c);
        let kind = info.kind;
        // The self call of a secondary constructor constructs nothing on its own.
        let is_abstract = info.mods & crate::ast::mods::ABSTRACT != 0 && self.excluded_ctor.is_none();
        // The class of a given with parameters, which its def's body read from another module's
        // products makes (`new pair2[A, B]()`), as a call of the given does.
        let product_given = kind == ClassKind::GivenImpl && self.is_product_class(c);
        if !(matches!(kind, ClassKind::Class | ClassKind::EnumCase) || product_given) || is_abstract {
            let why = match kind {
                ClassKind::Trait => "is a trait; it",
                ClassKind::Class => "is abstract; it",
                _ => "",
            };
            let msg = format!("{} {} cannot be instantiated", self.name_str(info.name), why).replace("  ", " ");
            self.error(span, msg);
            self.type_args_for_errors(&lists);
            return (self.prog.add(TExpr::Unit), ERROR);
        }
        let tparams = info.tparams.clone();
        let self_ty = info.base_types[0].1;
        // A class whose header the parser could not complete constructs from any arguments,
        // typed on their own.
        if info.mods & crate::ast::mods::INCOMPLETE != 0 {
            self.type_args_for_errors(&lists);
            let explicit: Vec<TypeId> = match targs {
                Some(l) => self.cur_ast().ty_list(l).to_vec().into_iter().map(|t| self.resolve_type(t)).collect(),
                None => Vec::new(),
            };
            let subst: Subst = tparams.iter().enumerate().map(|(i, &p)| (p, explicit.get(i).copied().unwrap_or_else(|| self.fresh_var()))).collect();
            let ty = self.types.subst(self_ty, &subst);
            return (self.prog.add(TExpr::New(c, crate::ast::ListRef::EMPTY)), ty);
        }
        let enum_parent = info.parents.first().copied().filter(|_| kind == ClassKind::EnumCase);
        let sig = Arc::new(MethodSig { tparams, clauses: info.ctor.clone(), ret: self_ty });
        let mut lists = lists;
        // A Java class has its constructors as alternatives only, each with a parameter list.
        let has_normal_clause = sig.clauses.iter().any(|c| !c.is_using) || (!secondaries.is_empty() && self.is_java_class(c));
        if lists.is_empty() && has_normal_clause && !as_function {
            lists.push(ArgList { args: Vec::new(), using: false, span });
        }
        // `Foo()` for a class declared without a parameter list.
        if !has_normal_clause && lists.first().map_or(false, |l| !l.using && l.args.is_empty()) {
            lists.remove(0);
        }
        if !secondaries.is_empty() && !as_function {
            let primary = self.syms.class(c).primary_ctor.filter(|_| self.ctor_accessible(c)).map(|s| (s, sig));
            let secondaries = secondaries.into_iter().map(|s| (s, self.sig_arc(s))).collect();
            return self.construct_overloaded(c, primary, secondaries, targs, lists, span, expected);
        }
        let call = MethodCall { recv: None, sym: SymId(u32::MAX), owner_subst: Vec::new(), ext_recv: None, prefix: None };
        let (mut te, precise) = self.apply_method(call, Some((c, sig)), targs, lists, span, expected, false).unwrap();
        if self.syms.class(c).js == JsKind::Native {
            if let TExpr::New(_, args) = self.prog.expr(te) {
                let args = self.native_ctor_args(c, args);
                let native = self.prog.add(TExpr::New(c, args));
                if self.capturing() {
                    self.capture_copy(te, native);
                }
                te = native;
            }
        }
        // An enum case constructor produces the enum type, as in Scala, unless the case type is
        // asked for or a member is selected from the application.
        let Some(parent) = enum_parent.filter(|_| !as_function) else { return (te, precise) };
        let widened = self.widen_enum_case(precise, parent);
        let case_expected = expected.map_or(false, |exp| self.expects_class(exp, c));
        if case_expected || !matches!(self.prog.expr(te), TExpr::New(..)) {
            return (te, if case_expected { precise } else { widened });
        }
        self.enum_case_new = Some((te, precise));
        (te, widened)
    }

    /// Applies a method (or constructor when `ctor` is set). In trial mode an inapplicable
    /// extension receiver yields `None` without reporting anything.
    pub fn apply_method(
        &mut self,
        call: MethodCall,
        ctor: Option<(ClassId, Arc<MethodSig>)>,
        targs: Option<ListRef>,
        lists: Vec<ArgList>,
        span: Span,
        expected: Option<TypeId>,
        trial: bool,
    ) -> Option<(TExprId, TypeId)> {
        let outer_base = std::mem::replace(&mut self.app_base, self.tvars.len() as u32);
        let r = self.apply_method_in(call, ctor, targs, lists, span, expected, trial);
        self.app_base = outer_base;
        r
    }

    fn apply_method_in(
        &mut self,
        call: MethodCall,
        ctor: Option<(ClassId, Arc<MethodSig>)>,
        targs: Option<ListRef>,
        lists: Vec<ArgList>,
        span: Span,
        expected: Option<TypeId>,
        trial: bool,
    ) -> Option<(TExprId, TypeId)> {
        let outer = std::mem::replace(&mut self.annotation_args, self.typing_annotation && ctor.is_some());
        self.app_depth += 1;
        // The member's application under `apply_member_or_extension` is the first under its cache.
        if let Some(c) = self.arg_cache.as_mut() {
            if c.depth == 0 {
                c.depth = self.app_depth;
            }
        }
        let erroneous_mark = (self.erroneous_args.len(), self.erroneous_lambdas.len());
        let applied = self.apply_method_in_now(call, ctor, targs, lists, span, expected, trial);
        self.erroneous_args.truncate(erroneous_mark.0);
        self.erroneous_lambdas.truncate(erroneous_mark.1);
        self.app_depth -= 1;
        self.annotation_args = outer;
        applied
    }

    #[allow(clippy::too_many_arguments)]
    fn apply_method_in_now(
        &mut self,
        call: MethodCall,
        ctor: Option<(ClassId, Arc<MethodSig>)>,
        targs: Option<ListRef>,
        lists: Vec<ArgList>,
        span: Span,
        expected: Option<TypeId>,
        trial: bool,
    ) -> Option<(TExprId, TypeId)> {
        let sig = match &ctor {
            // The constructor of a class nested in a class names the outer class's `this`,
            // the enclosing instance deriving from it where it is constructed or extended.
            Some((c, s)) if self.loaded.is_some() && self.class_in_class(*c) => self.sig_seen_from_enclosing(s.clone()),
            Some((_, s)) => s.clone(),
            None => match call.prefix {
                Some(prefix) => {
                    // The signature is kept through the application, which types the arguments.
                    let sig = self.sig_arc(call.sym);
                    let sig = self.sig_seen_from(sig, prefix, call.sym);
                    if self.member_of_inner_class(call.sym) {
                        let sig = match call.recv.and_then(|r| self.outer_prefixes.get(&r).cloned()) {
                            Some(steps) => steps.into_iter().fold(sig, |sig, (outer, path)| self.sig_seen_from_outer(sig, path, outer)),
                            None => sig,
                        };
                        self.sig_seen_from_enclosing(sig)
                    } else {
                        sig
                    }
                }
                None => self.sig_arc(call.sym),
            },
        };
        if !trial && ctor.is_none() && self.in_jar(self.syms.sym(call.sym).file) && self.report_blocked_sig(call.sym, span) {
            self.type_args_for_errors(&lists);
            return Some((self.prog.add(TExpr::Unit), ERROR));
        }
        // scala-library's `uninitialized` is a plain method that means what the std's inline
        // one means: the zero of the type it initialises.
        if ctor.is_none() && self.scala_library_std() && self.intrinsic_of(call.sym) == Some(super::inline::Intrinsic::Uninitialized) {
            let ret = sig.ret;
            return Some(self.inline_intrinsic(super::inline::Intrinsic::Uninitialized, &sig, &Vec::new(), &[], ret, span, expected));
        }
        // And its `deferred`, compile-time only, is the lean std's.
        if ctor.is_none() && self.scala_library_std() && self.intrinsic_of(call.sym) == Some(super::inline::Intrinsic::Deferred) {
            let ret = sig.ret;
            return Some(self.inline_intrinsic(super::inline::Intrinsic::Deferred, &sig, &Vec::new(), &[], ret, span, expected));
        }
        if ctor.is_none() && self.is_inline_callee(call.sym) {
            if let Some(r) = self.early_intrinsic(call.sym, &lists, span, expected) {
                return Some(r);
            }
        }
        let (ext_tparams, ext_clauses) = match (&ctor, call.ext_recv) {
            (None, Some(_)) => {
                let s = self.syms.sym(call.sym);
                (s.ext_tparams as usize, s.ext_clauses as usize)
            }
            _ => (0, 0),
        };
        let explicit_from = if std::mem::take(&mut self.direct_ext_targs) { 0 } else { ext_tparams };
        let explicit: Vec<TypeId> = match targs {
            Some(l) => {
                let ids = self.cur_ast().ty_list(l).to_vec();
                ids.iter().map(|&t| self.resolve_type_ctor(t)).collect()
            }
            None => match self.macro_targs.take() {
                Some((name, ts)) if name == self.syms.sym(call.sym).name && ts.len() + explicit_from == sig.tparams.len() => ts,
                other => {
                    self.macro_targs = other;
                    Vec::new()
                }
            },
        };
        let mut subst = call.owner_subst.clone();
        let first_var = self.tvars.len();
        let widen_mark = self.pending_widenings.len();
        for (i, &tp) in sig.tparams.iter().enumerate() {
            let t = if i >= explicit_from && i - explicit_from < explicit.len() {
                explicit[i - explicit_from]
            } else {
                let v = self.fresh_var();
                self.tvars.last_mut().unwrap().name = Some(self.syms.tparam(tp).name);
                v
            };
            subst.push((tp, t));
        }
        if explicit.len() > sig.tparams.len().saturating_sub(explicit_from) {
            self.error(span, "too many type arguments");
        }
        for (i, &tp) in sig.tparams.iter().enumerate() {
            let (upper, lower) = (self.syms.tparam(tp).upper, self.syms.tparam(tp).lower);
            // A bound naming the owner's `this` (`Tuple.head[This >: this.type]`) is seen from
            // the receiver, as the signature is.
            let (upper, lower) = match call.prefix {
                Some(prefix) if self.types.has_paths(upper) || self.types.has_paths(lower) => match self.class_of(prefix) {
                    Some(c) => (self.member_seen_from(upper, prefix, c, call.sym), self.member_seen_from(lower, prefix, c, call.sym)),
                    None => (upper, lower),
                },
                _ => (upper, lower),
            };
            let var = self.types.param(tp);
            let var = self.types.subst(var, &subst);
            let mut is_explicit = i >= explicit_from && i - explicit_from < explicit.len();
            if is_explicit {
                let arity = self.param_arity(tp);
                if var != NOTHING && self.type_arity(var) != arity && !self.is_body_file(self.env.file) {
                    let msg = self.kind_mismatch(var, tp, arity);
                    self.error(span, msg);
                }
                is_explicit = !self.types.contains_app_param(var);
            }
            if upper != ANY {
                let u = self.types.subst(upper, &subst);
                if !self.is_sub(var, u) && is_explicit {
                    let msg = format!("type argument {} does not conform to upper bound {}", self.show(var), self.show(u));
                    self.error(span, msg);
                }
            }
            if lower != NOTHING {
                let l = self.types.subst(lower, &subst);
                if !self.is_sub(l, var) && is_explicit {
                    let msg = format!("type argument {} does not conform to lower bound {}", self.show(var), self.show(l));
                    self.error(span, msg);
                }
            }
        }

        let recv_clause = if call.ext_recv.is_some() {
            sig.clauses.iter().take(ext_clauses).position(|c| !c.is_using)
        } else {
            None
        };
        if let (Some(idx), Some((_, recv_ty))) = (recv_clause, call.ext_recv) {
            let pty = self.types.subst(sig.clauses[idx].params[0].ty, &subst);
            let mark = self.snapshot();
            if !self.is_sub(recv_ty, pty) {
                self.rollback(mark);
                if trial {
                    return None;
                }
                let msg = format!(
                    "extension method {} is not applicable to {}",
                    self.name_str(self.syms.sym(call.sym).name),
                    self.show(recv_ty)
                );
                self.error(span, msg);
            }
        }

        let normal_clauses = sig
            .clauses
            .iter()
            .enumerate()
            .filter(|(i, c)| !c.is_using && Some(*i) != recv_clause)
            .count();
        let explicit_lists = lists.iter().filter(|l| !l.using).count();
        let mut ret_ty = self.types.subst(sig.ret, &subst);
        // The std's `summon` of a given standing for a parameter names that parameter for what
        // resolves on the result (`pick(summon[B])` ranks at the declared type).
        let mut summon_names: Option<SymId> = None;
        ret_ty = self.normalize(ret_ty);
        // A result or a later parameter that names an earlier parameter's path (`x.T`) takes
        // the path of the argument, or its type where the argument is no path.
        let dependent = self.names_param_paths(&sig);
        let mut open_expected = None;
        // A dependent result is compared with what is expected once the arguments' paths are in it.
        // The expected type constrains the result once every normal list is supplied, and with
        // several of them only when the last is reached, as scalac constrains the outermost
        // application after the inner ones typed their arguments: `fold("-")(f)` against a
        // `Shown` takes `B` from the first argument, and the whole result converts.
        // A result that names no path is matched with the expected type before the arguments
        // are typed, whatever paths the parameters name (a `using trace: Tracer.instance.Type`).
        let constrain_by_expected = explicit_lists == normal_clauses && !self.types.has_paths(sig.ret);
        if constrain_by_expected && explicit_lists <= 1 {
            open_expected = self.constrain_result_by_expected(ret_ty, expected);
        }

        let mut call = call;
        // dotty types an application clause by clause, the innermost first (`realApply`,
        // Applications.scala 1315): where the first plain clause fails, the clauses after it
        // are no part of the application. They are typed once, after it: by the retry on the
        // qualifier that takes the application over (`retry_on_qualifier`), or alone (1438).
        let mut stopped = false;
        let hoist_mark = self.hoisted.len();
        let paths_mark = self.param_paths.len();
        let inline_callee = ctor.is_none() && self.is_inline_callee(call.sym);
        let outer_recording = std::mem::replace(&mut self.inline.record_args, inline_callee);
        let outer_arg_types = if inline_callee { std::mem::take(&mut self.inline.arg_types) } else { Vec::new() };
        let mut args_out: Vec<TExprId> = Vec::new();
        // Where this application's arguments of an erroneous type start (`note_erroneous_arg`),
        // which an erased tag's search reads.
        let erroneous_mark = self.erroneous_args.len();
        let mut lists = lists.into_iter().peekable();
        let mut eta: Vec<(Vec<SymId>, Vec<TypeId>)> = Vec::new();
        // The method's parameter names per expanded clause, which a dependent expansion shows.
        let mut eta_names: Vec<Vec<Name>> = Vec::new();
        // The result the expected function type asks of the innermost eta expansion.
        let mut eta_ret_expected = None;
        // The trait with a single abstract method the expansion implements, when one is expected.
        let mut eta_sam = None;
        // A parameter or result type of that method the expansion does not conform to: found
        // and required.
        let mut eta_sam_mismatch: Option<(TypeId, TypeId)> = None;
        // Definition-site givens are positioned at the end of the argument lists applied so far.
        let mut given_site = span;
        let mut normal_seen = 0;
        for (ci, clause) in sig.clauses.iter().enumerate() {
            if Some(ci) == recv_clause {
                let (recv, recv_ty) = call.ext_recv.unwrap();
                // A by-name receiver (`extension (n: => Int)`) is passed as its thunk, each use
                // evaluating it.
                args_out.push(if clause.params[0].by_name { self.by_name_thunk(recv) } else { recv });
                // A parameter as the receiver was selected and inferred at its declared type;
                // an inline extension's binding takes the argument's own, as scalac's does.
                if self.inline.record_args {
                    if let Some(own) = self.proxy_own(recv) {
                        self.inline.arg_types.push((recv, own));
                    }
                }
                // `extension (p: Mirror.Product) def fromTuple(t: p.MirroredElemTypes)`: the
                // receiver's path is the parameter's.
                if dependent {
                    let p = clause.params[0].sym;
                    let path = self.arg_path(recv, recv_ty);
                    self.param_paths.push((p, path));
                    ret_ty = self.subst_paths(ret_ty, &[(p, path)]);
                }
                continue;
            }
            if clause.is_using {
                // A library body carries the arguments scalac inferred for a using clause as a
                // plain list.
                // A list the compiler made itself (`ArgSrc::Typed`: an extractor's scrutinee)
                // is never such a pickled list.
                let in_body = self.is_body_file(self.env.file);
                let written = |l: &ArgList| l.args.iter().all(|a| matches!(a, ArgSrc::Ast(_) | ArgSrc::Named(..)));
                if lists.peek().map_or(false, |l| l.using || ((clause.is_implicit || (in_body && written(l))) && !l.args.is_empty())) {
                    let list = lists.next().unwrap();
                    if !list.using && !in_body {
                        self.warn(list.span, "Implicit parameters should be provided with a `using` clause.");
                    }
                    given_site.end = list.span.end;
                    let first_arg = args_out.len();
                    let outer = std::mem::replace(&mut self.note_paths, dependent);
                    if self.type_clause_args(clause, list, &subst, &mut args_out) {
                        self.hoist_receiver(&mut call, hoist_mark, span);
                    }
                    self.note_paths = outer;
                    if dependent {
                        ret_ty = self.subst_param_paths(clause, &args_out[first_arg..], ret_ty, &mut subst);
                    }
                } else {
                    let determined = self.vars_of_applied_clauses(&sig.clauses[..ci], &subst);
                    for p in &clause.params {
                        let target = self.param_type(p.ty, &subst);
                        self.solve_selected_in(target, &determined);
                        let open_tag = self.open_tag(target);
                        // A given the probe of the extension's using clauses resolved is taken,
                        // not resolved again (`prefix_holds`).
                        let found = match self.inferred_hit(call.sym, target, p.sym) {
                            Some(hit) => Some(hit),
                            None => self.resolve_given_typed(target, given_site),
                        };
                        match found {
                            Some((te, given_ty)) => {
                                if ctor.is_none() && self.is_std_summon(call.sym) {
                                    ret_ty = given_ty;
                                    summon_names = self.parameter_named(te);
                                }
                                // An inline callee's proxy for the parameter takes the given's own
                                // type, which for a synthesized mirror says more than the parameter.
                                if self.inline.record_args {
                                    self.inline.arg_types.push((te, given_ty));
                                }
                                if dependent {
                                    let path = self.path_of(te).unwrap_or(given_ty);
                                    self.param_paths.push((p.sym, path));
                                    ret_ty = self.subst_paths(ret_ty, &[(p.sym, path)]);
                                }
                                args_out.push(if p.by_name { self.by_name_thunk(te) } else { te });
                            }
                            // No given: the parameter's default, as scalac falls back to it.
                            None if p.has_default && self.given_ambiguity.is_none() => {
                                self.failed_givens.clear();
                                let omitted = self.default_placeholder();
                                args_out.push(omitted);
                            }
                            None => {
                                let missing_tag = self.missing_tag(p.ty, target, open_tag);
                                let target = self.solve_bounded_in(target);
                                let msg = self.given_ambiguity.take().or(missing_tag).unwrap_or_else(|| {
                                    format!(
                                        "no given instance of type {} was found for parameter {}",
                                        self.show(target),
                                        self.name_str(p.name)
                                    )
                                });
                                let msg = msg + &self.given_failure_notes();
                                // At the end of what is applied so far, as dotty's
                                // `adaptNoArgsImplicitMethod` asks for the argument
                                // (`tree.span.endPos`): the call's end, or that of the lists before
                                // a clause a written list follows; before every written list (an
                                // extension's using clause), the call's start, as before.
                                let at = match lists.peek() {
                                    None => span.end,
                                    Some(_) if given_site.end != span.end => given_site.end,
                                    Some(_) => span.start,
                                };
                                self.given_failure_error(Span::new(at, at), msg, target);
                                args_out.push(self.prog.add(TExpr::Unit));
                            }
                        }
                    }
                }
                continue;
            }
            // A converted library body marks a list of givens as a using clause on the
            // arguments alone: where the clause is a plain one, so are they.
            match lists.next() {
                Some(list) if !list.using || self.is_body_file(self.env.file) => {
                    normal_seen += 1;
                    if constrain_by_expected && explicit_lists > 1 && normal_seen == normal_clauses {
                        open_expected = self.constrain_result_by_expected(ret_ty, expected);
                    }
                    given_site.end = list.span.end;
                    self.last_arg_types.clear();
                    let first_arg = args_out.len();
                    let outer = std::mem::replace(&mut self.note_paths, dependent);
                    let before = self.diags.items.len();
                    if self.type_clause_args(clause, list, &subst, &mut args_out) {
                        self.hoist_receiver(&mut call, hoist_mark, span);
                    }
                    self.note_paths = outer;
                    self.stringify_printed_double(&call, &mut args_out);
                    if dependent {
                        ret_ty = self.subst_param_paths(clause, &args_out[first_arg..], ret_ty, &mut subst);
                    }
                    // The first plain clause failed: the clauses after it are left.
                    if normal_seen == 1 && self.diags.items.len() != before && self.clause_failed(before) {
                        stopped = true;
                        break;
                    }
                }
                Some(list) => {
                    self.error(list.span, "unexpected using clause");
                }
                None => {
                    // Missing argument list: eta-expand over this clause and the ones after it.
                    let outer_arity = eta.first().map_or(clause.params.len(), |(syms, _)| syms.len());
                    let mut exp_fn = expected.and_then(|t| self.expected_function(t, outer_arity));
                    if exp_fn.is_none() && eta.is_empty() && ctor.is_none() {
                        if let Some(sam) = expected.and_then(|t| self.sam_method(t, clause.params.len())) {
                            let (_, _, sam_sig, sam_subst) = &sam;
                            // A by-name parameter of the method stays one, for the expansion to pass
                            // the argument on unevaluated.
                            let ps: Vec<TypeId> = sam_sig.clauses[0]
                                .params
                                .iter()
                                .map(|p| {
                                    let t = self.types.subst(p.ty, sam_subst);
                                    if p.by_name { self.by_name_type(t) } else { t }
                                })
                                .collect();
                            exp_fn = Some((ps, self.types.subst(sam_sig.ret, sam_subst)));
                            eta_sam = Some(sam);
                        }
                    }
                    for _ in 0..eta.len() {
                        exp_fn = exp_fn.and_then(|(_, ret)| self.as_function(ret));
                    }
                    let bare = exp_fn.is_none() && clause.params.is_empty() && eta.is_empty() && explicit_lists == 0;
                    let auto_applied = bare && ctor.is_none() && (self.has_java_parens(call.sym) || self.scala2_library_member(call.sym));
                    // The expansion's function evaluates the receiver and the arguments written
                    // once, as scalac's does: bound to temporaries in front of the lambda.
                    if eta.is_empty() && !auto_applied {
                        if let Some(r) = call.recv {
                            call.recv = Some(self.hoist_typed_at(hoist_mark, r, span));
                        }
                        for i in 0..args_out.len() {
                            let at = self.hoisted.len();
                            args_out[i] = self.hoist_typed_at(at, args_out[i], span);
                        }
                    }
                    let first_index: usize = eta.iter().map(|(syms, _)| syms.len()).sum();
                    eta_ret_expected = exp_fn.as_ref().map(|&(_, r)| r);
                    // Expected to be a function of one tuple of its parameters, the method is
                    // untupled as a lambda would be: `pairs.map(key)` for a `key(a, i)`.
                    if let Some((pt, tc, elems)) = self.untupled_expansion(&exp_fn, clause) {
                        let tuple = self.indexed_local("eta", first_index as u32, pt, span);
                        let fields: Vec<SymId> = self.syms.class(tc).ctor_syms.concat();
                        for (i, p) in clause.params.iter().enumerate() {
                            let pty = self.types.subst(p.ty, &subst);
                            self.is_sub(elems[i], pty);
                            let t = self.prog.add(TExpr::Local(tuple));
                            let mut arg = self.prog.add(TExpr::Field(t, fields[i]));
                            if p.by_name {
                                arg = self.by_name_thunk(arg);
                            }
                            args_out.push(arg);
                        }
                        eta.push((vec![tuple], vec![pt]));
                        eta_names.push(Vec::new());
                        continue;
                    }
                    // Expected with fewer parameters than the clause has, the function takes
                    // the first ones and the defaults fill the rest: `xs.map(LocalDate.parse)`
                    // for a `parse(text: String, format: Formatter = ISO)`.
                    let expanded = match &exp_fn {
                        Some((eps, _))
                            if eps.len() < clause.params.len()
                                && clause.params[eps.len()..].iter().all(|p| p.has_default && !p.repeated) =>
                        {
                            eps.len()
                        }
                        _ => clause.params.len(),
                    };
                    let mut syms = Vec::new();
                    let mut tys = Vec::new();
                    for (i, p) in clause.params.iter().enumerate() {
                        if i >= expanded {
                            let omitted = self.default_placeholder();
                            args_out.push(omitted);
                            continue;
                        }
                        let mut pty = self.param_type(p.ty, &subst);
                        if let (true, Some(seq)) = (p.repeated, self.seq_class()) {
                            pty = self.types.class(seq, &[pty]);
                        }
                        let expected_param = exp_fn.as_ref().filter(|(eps, _)| eps.len() == expanded).map(|(eps, _)| eps[i]);
                        if let Some(inner) = expected_param.filter(|_| self.b.by_name.is_some()).and_then(|e| self.by_name_arg(e)) {
                            let (s, arg, ty) = self.eta_by_name_param(inner, pty, p.by_name, call.sym, (first_index + i) as u32, span);
                            args_out.push(arg);
                            syms.push(s);
                            tys.push(ty);
                            continue;
                        }
                        // Implementing a trait, the expansion takes the parameter types of its
                        // method, which the argument adaptations take to the method's (dotc's
                        // lambda `(x: Int) => m(x)` for an `m(x: Long)`, or `m(convert(x))`).
                        let mut adapted_from = None;
                        if let Some(e) = expected_param {
                            let mark = self.snapshot();
                            if !self.is_sub(e, pty) && eta_sam.is_some() {
                                self.rollback(mark);
                                let view_compat = std::mem::replace(&mut self.view_compat, true);
                                let fits = self.is_compatible(e, pty);
                                self.view_compat = view_compat;
                                self.rollback(mark);
                                if fits {
                                    adapted_from = Some(e);
                                } else {
                                    eta_sam_mismatch.get_or_insert((e, pty));
                                }
                            }
                        }
                        let pty = self.solve_maximized_in(pty);
                        let s = self.indexed_local("eta", (first_index + i) as u32, adapted_from.unwrap_or(pty), span);
                        let mut arg = self.prog.add(TExpr::Local(s));
                        if let Some(from) = adapted_from {
                            arg = self.adapt(arg, from, pty, span);
                        }
                        if p.by_name {
                            arg = self.by_name_thunk(arg);
                        }
                        args_out.push(arg);
                        syms.push(s);
                        tys.push(adapted_from.unwrap_or(pty));
                    }
                    // A method with parameters becomes a function value wherever it is named
                    // without arguments, as in Scala 3; constructors do not, and one declared
                    // with `()` has to be called with them.
                    if exp_fn.is_none() && !clause.params.is_empty() && ctor.is_some() && explicit_lists > 0 {
                        self.error(span, "missing argument list for the constructor");
                    }
                    // A vararg method has no function type to expand to unless one is expected.
                    if exp_fn.is_none() && ctor.is_none() && clause.params.iter().any(|p| p.repeated) {
                        let msg = format!("missing argument list for method {}", self.method_description(call.sym));
                        self.error(span, msg);
                    }
                    if auto_applied {
                        continue;
                    }
                    if bare && ctor.is_none() {
                        self.report_missing_parens(call.sym, span);
                    }
                    // A result that names a parameter names the expansion's own: `depmeth`
                    // for a `depmeth(x: C): x.M` is a `(x: C) => x.M`.
                    if dependent {
                        let terms: Vec<(SymId, TypeId)> = clause.params.iter().zip(&syms).map(|(p, &s)| (p.sym, self.types.mk(Type::Term(s)))).collect();
                        self.param_paths.extend(terms.iter().copied());
                        ret_ty = self.subst_paths(ret_ty, &terms);
                    }
                    eta_names.push(clause.params.iter().take(syms.len()).map(|p| p.name).collect());
                    eta.push((syms, tys));
                }
            }
        }

        let mut evidence = Vec::new();
        if ctor.is_none() && self.syms.sym(call.sym).erased_tags != 0 {
            let written = lists.next_if(|l| l.using);
            if let Some(list) = &written {
                given_site.end = list.span.end;
            }
            let erroneous_arg = self.erroneous_args[erroneous_mark..].contains(&self.app_depth);
            evidence = self.erased_evidence(call.sym, &sig, &subst, written, erroneous_arg, given_site, span);
        }
        self.param_paths.truncate(paths_mark);
        // Type arguments that nothing constrains stay open while the expected type is an open
        // variable: under a branch guide the join of the branches settles them (`if c then
        // Map.empty else m`), otherwise the enclosing application does
        // (`Right(Refined.unsafeApply(t))` against an `Either[String, Refined[T, P]]`).
        let mut open_in_ret = Vec::new();
        let mut held = Vec::new();
        if let Some(v) = open_expected {
            self.conform_to_bounds(ret_ty, v, false);
            let ret_ty = self.zonk(ret_ty);
            self.collect_vars(ret_ty, &mut open_in_ret);
            // Under a branch guide a variable the result holds invariantly and only lower bounds
            // constrain stays open too, as scalac leaves it: the join of the branches equates it
            // with the other branches' (`if c then pure(Left(e)) else pure(Right(r))` is a
            // `Fr[Either[E, R]]`, not the `Product & Serializable` of `Fr[Left]` and `Fr[Right]`).
            if self.tvars[v].guide {
                for (w, sign) in self.var_variances(ret_ty) {
                    if sign != 0 || w.idx() < first_var || self.tvars[w].inst.is_some() {
                        continue;
                    }
                    let uppers = self.tvars[w].upper.clone();
                    if uppers.into_iter().all(|u| { let u = self.deref(u); matches!(self.types.get(u), Type::Var(_)) }) {
                        held.push(w);
                    }
                }
            }
        } else if normal_clauses == 0 && ctor.is_none() && (lists.peek().is_some() || (explicit_lists == 0 && !sig.tparams.is_empty() && std::mem::take(&mut self.open_receiver))) {
            // The arguments go to the result (`withService(f)` for a `def withService[T]: Partial[T]`
            // whose `apply` takes `f`), or a member of the result is selected (`lens.mod(f)` for
            // a `def lens[F[_]]: Lens[Holder[F], ..]`): what follows settles the type arguments
            // the result leaves open, from the expected type and the arguments, as scalac's does.
            let ret_ty = self.zonk(ret_ty);
            self.collect_vars(ret_ty, &mut open_in_ret);
            self.receiver_left_open = !open_in_ret.is_empty();
        } else if !trial && self.open_applied_receiver == Some(span) {
            self.open_applied_receiver = None;
            let ret_ty = self.zonk(ret_ty);
            for (w, sign) in self.var_variances(ret_ty) {
                let info = &self.tvars[w];
                if sign == 0 && info.inst.is_none() && info.lower.is_empty() && info.upper.is_empty() {
                    open_in_ret.push(w);
                }
            }
            self.receiver_left_open = !open_in_ret.is_empty();
        }
        self.solve_application_vars(first_var, widen_mark, ret_ty, &open_in_ret, &held, span);
        let ret_ty = self.zonk(ret_ty);
        let l = self.prog.list(&args_out);
        let mut ty = ret_ty;
        self.inline.record_args = outer_recording;
        let mut te = match &ctor {
            Some((c, _)) => self.new_instance(*c, l, span),
            None if inline_callee && self.keeps_inline_calls() => {
                self.inline.arg_types = outer_arg_types;
                let te = self.build_call(&call, l);
                self.defer_inline(&call, &sig, &subst, te, ret_ty, span, expected);
                te
            }
            // A result type the typer cannot give without the expansion (one inferred as an
            // error, one that depends on a parameter) keeps the call's expansion here.
            None if inline_callee
                && self.defers_plain_inline(call.sym)
                && !self.types.contains_error(ret_ty)
                && !sig.clauses.iter().flat_map(|c| &c.params).any(|p| self.mentions_term(sig.ret, p.sym)) =>
            {
                // A plain inline call waits for the later expansion phase, as dotty expands it
                // in its `Inlining` phase.
                let te = self.build_call(&call, l);
                let arg_types = std::mem::replace(&mut self.inline.arg_types, outer_arg_types);
                // The call's type is its result type as types are read, reduced
                // (`ToString[7]` the singleton of "7"), as scalac's typer leaves it.
                ty = self.normalize(ret_ty);
                self.defer_plain_inline(te, &call, &sig, &subst, &args_out, arg_types, ret_ty, ty, span, expected);
                te
            }
            None if inline_callee => {
                // What the expansion copies or reads of its arguments is expanded first: a plain
                // call among them pending.
                if self.attempts.pending_len() != 0 {
                    let mut roots = args_out.clone();
                    roots.extend(call.recv);
                    roots.extend(call.ext_recv.map(|(r, _)| r));
                    self.expand_pending_in(&roots);
                }
                let expanded = self.expand_inline(&call, &sig, &subst, &args_out, ret_ty, span, expected);
                self.inline.arg_types = outer_arg_types;
                match expanded {
                    Some((e, t)) => {
                        ty = t;
                        e
                    }
                    None => self.build_call(&call, l),
                }
            }
            None => self.build_call(&call, l),
        };
        if self.inline.checking > 0 && ctor.is_none() && !sig.tparams.is_empty() {
            self.note_type_args(te, &subst[call.owner_subst.len()..]);
        }
        if self.capturing() && !sig.tparams.is_empty() && !(inline_callee && !self.keeps_inline_calls()) {
            let targs: Vec<TypeId> = subst[call.owner_subst.len()..].iter().map(|&(_, t)| t).collect();
            self.capture_targs(te, &targs);
        }
        if self.capturing() && !evidence.is_empty() && !(inline_callee && !self.keeps_inline_calls()) {
            self.capture_evidence(te, evidence);
        }
        if let Some(parameter) = summon_names {
            self.inline.copies.push((te, parameter));
        }
        if eta.is_empty() {
            te = self.wrap_hoisted(hoist_mark, te);
        } else {
            (te, ty) = self.widen_lambda_result(te, ty, eta_ret_expected, span);
            if let (Some(ret), true, 1) = (eta_ret_expected, eta_sam.is_some(), eta.len()) {
                if ret != self.b.t_unit && eta_sam_mismatch.is_none() && !self.fits_lambda_result(ty, ret) {
                    eta_sam_mismatch = Some((ty, ret));
                }
            }
        }
        let expanded = !eta.is_empty();
        for ((syms, tys), names) in eta.into_iter().zip(eta_names).rev() {
            let params = self.prog.syms(&syms);
            te = self.prog.add(TExpr::Lambda(params, te));
            ty = self.closure_type_named(false, &names, &syms, &tys, ty, span);
        }
        // dotc types the expansion as a lambda whose parameters are the method's of the trait:
        // a method that does not take them, or whose result is not the trait method's, does
        // not implement it.
        let plain_eta = expanded && eta_sam_mismatch.is_none() && eta_sam.is_none();
        if let Some((found, required)) = eta_sam_mismatch {
            let msg = format!("type mismatch: found {}, required {}", self.show(found), self.show(required));
            self.error_unless_unknown(span, msg, &[found, required]);
            ty = ERROR;
        } else if let Some((trait_ty, method, sam_sig, sam_subst)) = eta_sam {
            (te, ty) = self.sam_from_function(trait_ty, method, sam_sig, sam_subst, te, ty, span);
        }
        if expanded {
            self.prog.set_type(te, ty);
            te = self.wrap_hoisted(hoist_mark, te);
            if plain_eta {
                self.eta_expansions.insert(te, ());
            }
        }
        let rest: Vec<ArgList> = lists.collect();
        if stopped {
            // The lists past the clause that failed: a member's attempt leaves them to its retry
            // (`apply_member_or_extension`), the application a retry's try makes to the next try
            // (the one entered from the retry, `Worker::retry_depth`; one nested in its arguments
            // is its own); any other application types them alone, once, for their own errors
            // (`typedArgs`, 1438).
            let depth = self.app_depth;
            let owned = self.arg_cache.as_ref().is_some_and(|c| c.depth == depth);
            if !owned && self.retry_depth.map_or(true, |d| d + 1 != self.app_depth) {
                self.type_args_for_errors(&rest);
            }
            return Some((te, ERROR));
        }
        if rest.is_empty() {
            return Some((te, ty));
        }
        let empty_parens = rest[0].args.is_empty() && !rest[0].using;
        if normal_clauses == 0 && ctor.is_none() && self.as_function(ty).is_none() {
            // A parameterless `apply` that was inserted for the arguments cannot pass them on
            // to another parameterless `apply` of its result: `(new C)(22)` for a `def apply: C`.
            let next_apply = self.find_member(ty, names::APPLY).map(|(s, _)| s);
            let loops = self.syms.sym(call.sym).name == names::APPLY
                && next_apply.map_or(false, |s| {
                    let alts: Vec<SymId> = self.syms.alternatives(s).map_or_else(|| vec![s], |a| a.to_vec());
                    alts.iter().all(|&a| self.sig_of(a).clauses.iter().all(|c| c.is_using))
                });
            if loops || (empty_parens && next_apply.is_none()) {
                if empty_parens && rest.len() == 1 && self.has_java_parens(call.sym) {
                    return Some((te, ty));
                }
                if loops || !self.has_apply_extension(ty) {
                    self.report_takes_no_parameters(call.sym, span);
                    return Some((te, ty));
                }
            }
        }
        // The result the next list applies, which signature help reads at the span of the
        // application so far (`factory()(|)`), where some list was applied.
        if self.index.is_some() && given_site.end < span.end {
            self.index_value(Span::new(span.start, given_site.end), te, ty);
        }
        let applied = self.apply_callee(Callee::Value(te, ty), None, rest, span, expected);
        let widen_mark = self.pending_widenings.len();
        self.solve_application_vars(first_var, widen_mark, applied.1, &[], &[], span);
        Some(applied)
    }

    /// A parameter type instantiated for a call: the type arguments substituted, and the paths
    /// of the parameters applied so far replaced by those of their arguments.
    #[inline]
    pub(super) fn param_type(&mut self, ty: TypeId, subst: &Subst) -> TypeId {
        let t = self.types.subst(ty, subst);
        if self.param_paths.is_empty() || !self.types.has_paths(t) {
            return t;
        }
        let paths = self.param_paths.clone();
        self.subst_paths(t, &paths)
    }

    /// After a clause is applied, the paths of its parameters in the result type give way to
    /// the paths of the arguments; the substitution of later clauses is extended the same way
    /// through a parameter's own type, which `subst` leaves alone, so the paths are put into
    /// the parameter types of the clauses still to come.
    #[cold]
    fn subst_param_paths(&mut self, clause: &ClauseSig, args: &[TExprId], ret_ty: TypeId, subst: &mut Subst) -> TypeId {
        let mut terms: Vec<(SymId, TypeId)> = Vec::new();
        for (p, &arg) in clause.params.iter().zip(args) {
            let path = match self.param_paths.iter().rev().find(|&&(s, _)| s == p.sym) {
                Some(&(_, path)) => path,
                None => {
                    let pty = match self.arg_types_seen.iter().rev().find(|&&(e, _)| e == arg) {
                        Some(&(_, t)) => t,
                        None => self.types.subst(p.ty, subst),
                    };
                    self.arg_path(arg, pty)
                }
            };
            terms.push((p.sym, path));
        }
        self.param_paths.extend(terms.iter().copied());
        self.subst_paths(ret_ty, &terms)
    }

    /// Whether the result or a parameter of `sig` names a parameter's path. A path that names
    /// none (zio's `trace: Trace`, `Tracer.instance.Type`) leaves the arguments' types open
    /// until the application solves them.
    fn names_param_paths(&mut self, sig: &MethodSig) -> bool {
        let types: Vec<TypeId> = std::iter::once(sig.ret).chain(sig.clauses.iter().flat_map(|c| c.params.iter().map(|p| p.ty))).filter(|&t| self.types.has_paths(t)).collect();
        if types.is_empty() {
            return false;
        }
        let params: Vec<SymId> = sig.clauses.iter().flat_map(|c| c.params.iter().map(|p| p.sym)).collect();
        types.into_iter().any(|t| self.types.names_term(t, &params))
    }

    /// What a parameter's path stands for once an argument is in: the argument's path, the
    /// argument's type where that is a singleton type, its widened type otherwise.
    pub(super) fn arg_path(&mut self, arg: TExprId, pty: TypeId) -> TypeId {
        if let Some(path) = self.path_of(arg) {
            return path;
        }
        let ty = self.prog.expr_types.get(arg.0).filter(|&t| t != crate::tir::NO_TYPE);
        let ty = ty.unwrap_or(pty);
        let ty = self.deref(ty);
        // A constant an ascription or a plain inline call widened has the widened type for a
        // dependent parameter too (`Elem[T, n.type]` of `(1, "x")((g(0): Int))`).
        let ty = if self.widened_constant(arg) { self.widen_lit(ty) } else { ty };
        if self.types.is_path(ty) {
            return ty;
        }
        let ty = self.solve_bounded_in(ty);
        self.widen_path(ty)
    }

    /// Records the path an argument gives its parameter, for the parameters after it in the
    /// same clause (`class Foo(val x: String, val y: Option[x.type])`).
    #[cold]
    fn note_param_path(&mut self, p: &ParamSig, arg: TExprId, pty: TypeId) {
        let typed = self.last_arg_types.first().copied().unwrap_or(pty);
        let path = self.arg_path(arg, typed);
        self.param_paths.push((p.sym, path));
    }

    /// The tuple type, its class and its element types when a clause of several parameters is
    /// expected to be a function of one tuple of them.
    fn untupled_expansion(
        &mut self,
        exp_fn: &Option<(Vec<TypeId>, TypeId)>,
        clause: &ClauseSig,
    ) -> Option<(TypeId, ClassId, Vec<TypeId>)> {
        let (eps, _) = exp_fn.as_ref()?;
        if eps.len() != 1 || clause.params.len() < 2 || clause.params.iter().any(|p| p.repeated) {
            return None;
        }
        let pt = self.solve_if_var(eps[0]);
        let Type::Class(tc, args) = self.types.get(pt) else { return None };
        let elems = self.types.items(args).to_vec();
        (self.is_tuple_class(tc) && elems.len() == clause.params.len()).then_some((pt, tc, elems))
    }

    /// Solves the type variables an application made from `first_var` on, then applies the
    /// numeric widenings its arguments were waiting for. A branch guide and an unconstrained
    /// variable of `open_in_ret` stay open for the enclosing application.
    fn solve_application_vars(
        &mut self,
        first_var: usize,
        widen_mark: usize,
        ret_ty: TypeId,
        open_in_ret: &[TVarId],
        held: &[TVarId],
        span: Span,
    ) {
        // The variables of the result type with their variances, walked when an unconstrained
        // variable first asks and again after a solution that touched one of them (an
        // instantiation lets other variables show through). The variables between `first_var`
        // and the end are mostly what the nested searches and expansions left: a rolled-back
        // candidate's, unconstrained and in no result type.
        let mut in_ret: Option<Vec<(TVarId, i8)>> = None;
        for v in first_var..self.tvars.len() {
            let info = &self.tvars[v];
            let unconstrained = info.inst.is_none() && info.lower.is_empty() && info.upper.is_empty();
            let v = self.tvars.id(v);
            if info.guide || (unconstrained && open_in_ret.contains(&v)) || held.contains(&v) {
                continue;
            }
            // Nothing is known about it: a variable the result uses only contravariantly is
            // `Any`, as scalac maximises it, so that `mk` for `def mk[A]: Con[A]` is a `Con[Any]`.
            if unconstrained {
                if in_ret.is_none() {
                    in_ret = Some(self.var_variances(ret_ty));
                }
                if in_ret.as_ref().map_or(false, |vars| vars.iter().any(|&(w, s)| w == v && s == -1)) {
                    self.instantiate(v, ANY);
                    in_ret = None;
                    continue;
                }
            }
            let mark = self.trail.len();
            if !self.solve_var_directed(v, Some(ret_ty)) {
                let msg = format!("cannot infer type arguments: conflicting bounds for {}", self.show(ret_ty));
                self.error(span, msg);
                break;
            }
            if let Some(vars) = &in_ret {
                let touched = |u: &Undo| {
                    let w = match u {
                        Undo::Inst(w) | Undo::Lower(w) | Undo::Upper(w) => *w,
                    };
                    vars.iter().any(|&(x, _)| x == w)
                };
                if self.trail[mark..].iter().any(touched) {
                    in_ret = None;
                }
            }
        }
        for i in widen_mark..self.pending_widenings.len() {
            let (te, actual, pty) = self.pending_widenings[i];
            let target = self.zonk(pty);
            let ops = match (self.is_numeric(actual), self.is_numeric(target)) {
                (Some(f), Some(t)) if super::prims::widens(f, t) => super::prims::convert_ops(f, t),
                _ => &[],
            };
            if !ops.is_empty() {
                // The argument node is rewritten in place, so whatever refers to it sees the
                // widened value, and typed as the parameter: the backend boxes the widened
                // number as one (an `Int`-typed `IntToLong` was narrowed back and boxed as an
                // `Integer`); journaled under an attempt (`state::rewrite_expr`).
                let original_type = self.prog.type_of(te);
                let mut original = self.prog.add(self.prog.expr(te));
                if let Some(t) = original_type {
                    self.prog.set_type(original, t);
                }
                for &op in &ops[..ops.len() - 1] {
                    original = self.prog.add(TExpr::Unary(op, original));
                }
                self.rewrite_expr(te, TExpr::Unary(ops[ops.len() - 1], original));
                self.retype_expr(te, target);
            }
        }
        self.pending_widenings.truncate(widen_mark);
    }

    /// Relates the result type of an application to what is expected before its arguments are
    /// typed, kept only where it holds; the open variable an expected variable is, for the
    /// bounds to conform to once the arguments are in.
    fn constrain_result_by_expected(&mut self, ret_ty: TypeId, expected: Option<TypeId>) -> Option<TVarId> {
        if let Some(exp) = self.concrete_expected(expected) {
            if exp != self.b.t_unit && exp != ANY {
                let mark = self.snapshot();
                let outer = std::mem::replace(&mut self.necessary_either, true);
                let ok = self.is_sub(ret_ty, exp);
                self.necessary_either = outer;
                if !ok {
                    self.rollback(mark);
                }
            }
            return None;
        }
        let exp = self.deref(expected?);
        if let Type::Var(v) = self.types.get(exp) {
            self.conform_to_bounds(ret_ty, v, true);
            return Some(v);
        }
        if exp != ERROR && self.types.has_vars(ret_ty) {
            // An expected type that holds variables of an enclosing call still relates
            // its type arguments to the result: `List(Column(_.name))` against a
            // `List[Column[?T]]` makes the lambda's parameter a `?T`.
            let mark = self.snapshot();
            if !self.is_sub(ret_ty, exp) {
                self.rollback(mark);
            }
        }
        None
    }

    /// The result of an application whose expected type is the open variable `expected` has to
    /// conform to the upper bounds of that variable. Its lower bounds are what the result will be
    /// joined with, so type arguments that the application leaves open follow them where they
    /// can: `Map.empty` is a `Map[Int, String]` in `opt.getOrElse(Map.empty)`.
    fn conform_to_bounds(&mut self, ret_ty: TypeId, expected: TVarId, upper: bool) {
        let info = &self.tvars[expected];
        let bounds = if upper { info.upper.clone() } else { info.lower.clone() };
        if bounds.is_empty() {
            return;
        }
        let ret_ty = self.zonk(ret_ty);
        if !self.types.has_vars(ret_ty) {
            return;
        }
        for bound in bounds {
            let bound = self.zonk(bound);
            let mark = self.snapshot();
            if !self.is_sub(ret_ty, bound) {
                self.rollback(mark);
            }
        }
    }

    /// The type variables in the parameter types of the argument lists applied so far. As in
    /// scalac they are solved before a using clause is resolved, so `f(Some(1))` looks for a
    /// `TC[Some[Int]]`; a variable that only the using clause mentions is left to the search.
    pub(super) fn vars_of_applied_clauses(&mut self, applied: &[ClauseSig], subst: &Subst) -> Vec<TVarId> {
        let mut vars = Vec::new();
        for clause in applied.iter().filter(|c| !c.is_using) {
            for p in &clause.params {
                let pty = self.types.subst(p.ty, subst);
                self.collect_vars(pty, &mut vars);
            }
        }
        // A variable of an enclosing application (`R1` of a `flatMap[R1 <: R]` whose lambda
        // calls `traverse_`) is left to that application, which knows how its result uses it.
        vars.retain(|v| v.index() as u32 >= self.app_base);
        vars
    }

    /// `println(1.0)` prints `1.0` as on the JVM: a statically known Double is formatted before
    /// it reaches the untyped print functions.
    fn stringify_printed_double(&mut self, call: &MethodCall, args: &mut Vec<TExprId>) {
        if call.sym.0 == u32::MAX || args.len() != 1 || self.last_arg_types.len() != 1 {
            return;
        }
        let info = self.syms.sym(call.sym);
        let is_print = matches!(info.name, names::PRINTLN | names::PRINT)
            && info.owner == Owner::Package(self.b.scala_pkg);
        let ty = self.last_arg_types[0];
        let printed = self.deref(ty);
        if is_print && matches!(self.is_numeric(printed), Some(super::prims::R_DOUBLE | super::prims::R_FLOAT)) {
            args[0] = self.prog.rendering(args[0], StrKind::Double);
        }
    }

    pub(super) fn build_call(&mut self, call: &MethodCall, args: ListRef) -> TExprId {
        let info = self.syms.sym(call.sym);
        if let Some(import) = info.js_import {
            return self.build_import_call(call.sym, TExpr::JsImport(import), args);
        }
        if let Some(global) = info.js_global {
            return self.build_import_call(call.sym, TExpr::JsGlobal(global, false), args);
        }
        // A `@js` template on a member of a native type is what the member does (`jsIterator`
        // as `$0[Symbol.iterator]()`); any other native member is a property or method call.
        if let (Some(r), true, None) = (call.recv, self.syms.js_member(call.sym), &info.intrinsic) {
            return self.build_js_member_call(r, call.sym, args);
        }
        if let Some(template) = info.intrinsic.clone() {
            let mut all: Vec<TExprId> = Vec::new();
            if let (Some(r), false) = (call.recv, info.is_extension) {
                all.push(r);
                // A stored inline body is no part of the output: its expansion's copy is.
                if matches!(info.owner, Owner::Class(_)) && !self.checks_inline_definition() {
                    let unit = self.typing_unit();
                    self.prog.template_calls.push((call.sym, unit));
                }
            }
            all.extend_from_slice(self.prog.expr_list(args));
            let l = self.prog.list(&all);
            let s = self.prog.add_str(&template);
            self.prog.template_syms.insert(s, call.sym);
            if self.forked && !self.prog.template_syms.to_shared {
                self.pending_shared.push(super::PendingShared::Template(s, call.sym));
            }
            let te = self.prog.add(TExpr::Js(s, l));
            // An extension of a class's instance (a `Quotes`'s `x.show`) leaves the instance out.
            if let (Some(r), true, true, Owner::Class(_)) = (call.recv, info.is_extension, self.capturing(), info.owner) {
                self.capture_receiver(te, r);
            }
            return te;
        }
        if info.kind == SymKind::Given {
            if args.is_empty() {
                return match call.recv {
                    Some(r) => self.prog.add(TExpr::Field(r, call.sym)),
                    None => self.prog.add(TExpr::Static(call.sym)),
                };
            }
            if let Some(impl_class) = info.impl_class {
                // The class of a given of a class or trait instance takes that instance first.
                let args = match call.recv {
                    Some(r) if self.outer_class(impl_class).is_some() => {
                        let items: Vec<TExprId> = std::iter::once(r).chain(self.prog.expr_list(args).iter().copied()).collect();
                        self.prog.list(&items)
                    }
                    _ => args,
                };
                let te = self.prog.add(TExpr::New(impl_class, args));
                if self.capturing() {
                    self.capture_form(te, Form::GivenCall(call.sym));
                }
                return te;
            }
        }
        if info.name == names::INIT {
            return self.prog.add(TExpr::NewVia(call.sym, args));
        }
        if let Some(te) = self.companion_widening(call, args) {
            return te;
        }
        match call.recv {
            Some(r) => self.prog.add(TExpr::CallMethod(r, call.sym, args)),
            None => self.prog.add(TExpr::CallStatic(call.sym, args)),
        }
    }

    /// scalac's widening conversion on a primitive's companion (`Int.int2double(i)`, what a
    /// module's product or a library body names for an implicit widening) is the typer's own
    /// widening of the argument in a body read from TASTy: the node a build from source has, so
    /// that a build over products writes the same JavaScript. A source body's own call stays a
    /// call, as a macro sees it and as master wrote it; a receiver that is not the companion
    /// itself (`ints.int2double(x)` for a `def ints: Int.type`) is evaluated as a call's is.
    fn companion_widening(&mut self, call: &MethodCall, args: ListRef) -> Option<TExprId> {
        use super::prims::*;
        if self.cur_ast().reader.is_none() {
            return None;
        }
        if let Some(r) = call.recv {
            if !matches!(self.prog.expr(r), TExpr::Module(_)) {
                return None;
            }
        }
        let info = self.syms.sym(call.sym);
        let Owner::Class(owner) = info.owner else { return None };
        let &[arg] = self.prog.expr_list(args) else { return None };
        let name = self.interner.get(info.name);
        let (from, to) = name.split_once('2')?;
        let rank = |n: &str| match n {
            "byte" => Some(R_BYTE),
            "short" => Some(R_SHORT),
            "char" => Some(R_CHAR),
            "int" => Some(R_INT),
            "long" => Some(R_LONG),
            "float" => Some(R_FLOAT),
            "double" => Some(R_DOUBLE),
            _ => None,
        };
        let (from, to) = (rank(from)?, rank(to)?);
        let (from_ty, to_ty) = (self.rank_type(from), self.rank_type(to));
        let Type::Class(fc, _) = self.types.get(from_ty) else { return None };
        if self.syms.class(fc).companion != Some(owner) {
            return None;
        }
        self.widen_numeric(arg, from_ty, to_ty)
    }

    fn is_function_literal(&self, a: &ArgSrc) -> bool {
        match a {
            ArgSrc::Ast(e) | ArgSrc::Hoisted(e) => {
                let ast = self.cur_ast();
                let inner = match ast.expr(*e) {
                    Expr::NamedArg(_, v) => v,
                    _ => *e,
                };
                matches!(ast.expr(inner), Expr::Lambda(..))
            }
            ArgSrc::Typed(..) | ArgSrc::Named(..) => false,
            ArgSrc::ForLambda(..) => true,
        }
    }

    /// An expression whose evaluation has no effect and observes none, so its place in the
    /// evaluation order is immaterial.
    pub fn is_stable(&self, te: TExprId) -> bool {
        match self.prog.expr(te) {
            TExpr::Int(_)
            | TExpr::Long(_)
            | TExpr::Double(_)
            | TExpr::Bool(_)
            | TExpr::Char(_)
            | TExpr::Str(_)
            | TExpr::Unit
            | TExpr::This
            | TExpr::Super(_)
            | TExpr::Module(_)
            | TExpr::Lambda(..) => true,
            TExpr::Local(s) => self.syms.sym(s).kind != SymKind::Var,
            // A given's reference still pending is the call its expansion makes, not a path.
            TExpr::Static(s) => self.syms.sym(s).kind != SymKind::Var && !self.is_pending_call(te),
            _ => false,
        }
    }

    /// A plain inline call still pending that is about to be bound to a temporary, expanded first
    /// (`expand_pending_kept`, after the calls typed before it): its expansion decides whether it
    /// is bound, as where every call expanded as typed, so that a literal stays in place
    /// (`tw"mt-4"`) and an effect is bound.
    fn expand_before_binding(&mut self, te: TExprId) {
        if self.is_pending_call(te) {
            self.expand_pending_kept(&[te]);
        }
    }

    /// Binds an expression that would otherwise be evaluated twice or out of order to a
    /// temporary of the innermost `hoisted` segment; a stable one is returned as it is.
    pub fn hoist(&mut self, te: TExprId, ty: TypeId, span: Span) -> TExprId {
        self.expand_before_binding(te);
        if self.is_stable(te) {
            return te;
        }
        let tmp = self.indexed_local("h", self.hoisted.len() as u32, ty, span);
        if self.capturing() && ty == ANY {
            if let Some(own) = self.prog.type_of(te) {
                self.capture_local(tmp, |l| l.ty = Some(own));
            }
        }
        if self.inline.checking > 0 {
            self.note_hoisted(tmp);
        }
        self.hoisted.push(TStmt::Val(tmp, te));
        self.prog.add(TExpr::Local(tmp))
    }

    /// Wraps `te` into a block that evaluates the temporaries hoisted since `mark` first.
    pub fn wrap_hoisted(&mut self, mark: usize, te: TExprId) -> TExprId {
        if self.hoisted.len() == mark {
            return te;
        }
        let stmts: Vec<TStmt> = self.hoisted.drain(mark..).collect();
        let l = self.prog.stmts.push_slice(&stmts);
        self.prog.add(TExpr::Block(l, te))
    }

    /// Types the arguments of one clause into `out`. True when named arguments were written out
    /// of parameter order and had to be bound to temporaries to keep their evaluation order;
    /// the caller then hoists what it evaluates before them.
    pub fn type_clause_args(
        &mut self,
        clause: &ClauseSig,
        list: ArgList,
        subst: &Subst,
        out: &mut Vec<TExprId>,
    ) -> bool {
        // A member's application under `apply_member_or_extension` records the typings of the
        // first list that is not a using one, its retry's, and no other list's.
        if let Some(c) = self.arg_cache.as_mut() {
            c.active = !list.using && !c.done;
            c.done |= !list.using;
        }
        let ast = self.cur_ast();
        let n = clause.params.len();
        if n == 1 && list.args.len() > 1 && !clause.params[0].repeated {
            if let Some(elems) = self.positional_args(&list.args) {
                out.push(self.type_tupled_args(&clause.params[0], &elems, subst, list.span));
                return false;
            }
        }
        let plain = list.args.len() == n
            && !clause.params.iter().any(|p| p.repeated || p.by_name)
            && !list.args.iter().any(|a| match a {
                ArgSrc::Named(..) => true,
                _ => a.ast().map_or(false, |e| matches!(ast.expr(e), Expr::NamedArg(..))),
            });
        let dependent = self.note_paths;
        if plain {
            // Positional arguments only: function literals are still typed last.
            let base = out.len();
            out.resize(base + n, TExprId(0));
            for pass in 0..2 {
                for i in 0..n {
                    if (pass == 1) != self.is_function_literal(&list.args[i]) {
                        continue;
                    }
                    let pty = self.param_type(clause.params[i].ty, subst);
                    out[base + i] = self.type_arg(list.args[i], pty, list.span);
                    if dependent {
                        self.note_param_path(&clause.params[i], out[base + i], pty);
                    }
                }
            }
            return false;
        }
        let mut slots: Vec<Vec<ArgSrc>> = vec![Vec::new(); n];
        // The parameter each argument fills, in the order the arguments are written.
        let mut order: Vec<usize> = Vec::with_capacity(list.args.len());
        let mut splice = false;
        // A positional argument after a named one fills the parameter after the named one's,
        // provided every parameter before it has an argument.
        let mut pos = 0usize;
        let mut misplaced = false;
        for a in &list.args {
            let named = match a {
                ArgSrc::Ast(e) => match ast.expr(*e) {
                    Expr::NamedArg(name, v) => Some((name, ArgSrc::Ast(v))),
                    _ => None,
                },
                // A named argument bound ahead of the call keeps its value bound.
                ArgSrc::Hoisted(e) => match ast.expr(*e) {
                    Expr::NamedArg(name, v) => Some((name, ArgSrc::Hoisted(v))),
                    _ => None,
                },
                ArgSrc::Named(name, te, ty) => Some((*name, ArgSrc::Typed(*te, *ty))),
                _ => None,
            };
            match named {
                Some((name, v)) => match clause.params.iter().position(|p| p.name == name) {
                    Some(i) if slots[i].is_empty() => {
                        if let (true, ArgSrc::Ast(e)) = (self.index.is_some(), a) {
                            self.index_named_arg(*e, name, clause.params[i].sym);
                        }
                        slots[i].push(v);
                        order.push(i);
                        pos = pos.max(i + 1);
                    }
                    Some(_) => {
                        let msg = format!("parameter {} is given twice", self.name_str(name));
                        self.error(list.span, msg);
                    }
                    None => {
                        let msg = format!("there is no parameter named {}", self.name_str(name));
                        self.error(list.span, msg);
                        if let ArgSrc::Ast(v) = v {
                            self.type_extra_arg(v);
                        }
                    }
                },
                None => {
                    let is_repeated = clause.params.get(pos.min(n.saturating_sub(1))).map_or(false, |p| p.repeated);
                    let skipped = slots[..pos.min(n)].iter().any(|s| s.is_empty());
                    if skipped || (pos >= n && !is_repeated) {
                        misplaced |= skipped;
                        let msg = if skipped { "positional after named argument".to_string() } else { format!("too many arguments: expected {}", n) };
                        self.arity_error(&list, msg);
                        if let ArgSrc::Ast(e) = a {
                            self.type_extra_arg(*e);
                        }
                        continue;
                    }
                    let i = pos.min(n - 1);
                    if let ArgSrc::Ast(e) = a {
                        if let Expr::Typed(inner, t) = ast.expr(*e) {
                            if matches!(ast.ty(t), TyExpr::Repeated(_)) {
                                splice = true;
                                slots[i].push(ArgSrc::Ast(inner));
                                pos += 1;
                                continue;
                            }
                        }
                    }
                    slots[i].push(*a);
                    order.push(i);
                    if !clause.params[i].repeated {
                        pos += 1;
                    }
                }
            }
        }
        let mut results: Vec<Option<TExprId>> = vec![None; n];
        for pass in 0..2 {
            for i in 0..n {
                let p = &clause.params[i];
                let pty = self.param_type(p.ty, subst);
                if p.repeated {
                    if pass == 1 {
                        continue;
                    }
                    if splice {
                        let seq_ty = match self.seq_class() {
                            Some(seq) => self.types.class(seq, &[pty]),
                            None => ERROR,
                        };
                        self.spread_arg = true;
                        let spliced = self.type_arg(slots[i][0], seq_ty, list.span);
                        self.spread_arg = false;
                        if self.capturing() {
                            self.capture_wrap(spliced, Wrap::Splice(pty));
                        }
                        results[i] = Some(spliced);
                    } else {
                        let open = self.deref(pty);
                        let lower_mark = match self.types.get(open) {
                            Type::Var(v) => self.tvars[v].lower.len(),
                            _ => 0,
                        };
                        let items: Vec<TExprId> =
                            slots[i].clone().iter().map(|&a| self.type_arg(a, pty, list.span)).collect();
                        self.harmonize_varargs(&items, pty, lower_mark);
                        let l = self.prog.list(&items);
                        let seq = self.prog.add(TExpr::SeqLit(l));
                        if self.capturing() {
                            self.capture_form(seq, Form::Repeated(pty));
                        }
                        // The sequence's type says what its elements are, which the JVM boxes
                        // them as: an `Int` written where a `Double` is expected is one.
                        if let Some(seq_class) = self.seq_class() {
                            let seq_ty = self.types.class(seq_class, &[pty]);
                            self.prog.set_type(seq, seq_ty);
                        }
                        results[i] = Some(seq);
                    }
                    if p.by_name {
                        results[i] = results[i].map(|seq| self.by_name_thunk(seq));
                    }
                    continue;
                }
                let Some(&arg) = slots[i].first() else {
                    if pass == 0 {
                        if !p.has_default && !misplaced {
                            let msg = format!("missing argument for parameter {}", self.name_str(p.name));
                            self.arity_error(&list, msg);
                        }
                        results[i] = Some(if p.has_default { self.default_placeholder() } else { self.prog.add(TExpr::Unit) });
                    }
                    continue;
                };
                if (pass == 1) != self.is_function_literal(&arg) {
                    continue;
                }
                let arg = match arg {
                    ArgSrc::Hoisted(e) if p.by_name || (self.b.by_name.is_some() && self.by_name_arg(pty).is_some()) => ArgSrc::Ast(e),
                    a => a,
                };
                let te = self.type_arg(arg, pty, list.span);
                results[i] = Some(if p.by_name {
                    self.by_name_thunk(te)
                } else {
                    te
                });
            }
        }
        let mut results: Vec<TExprId> = results.into_iter().map(|r| r.unwrap()).collect();
        let reordered = self.keep_call_order(&order, &mut results, out, list.span);
        if self.capturing() {
            self.capture_named_args(clause, &list, &results);
        }
        out.extend(results);
        reordered
    }

    /// The name of each argument written with one, on the argument typed for its parameter.
    #[cold]
    #[inline(never)]
    fn capture_named_args(&mut self, clause: &ClauseSig, list: &ArgList, results: &[TExprId]) {
        let ast = self.cur_ast();
        let mut named = vec![false; results.len()];
        for a in &list.args {
            let name = match a {
                ArgSrc::Ast(e) => match ast.expr(*e) {
                    Expr::NamedArg(name, _) => name,
                    _ => continue,
                },
                ArgSrc::Named(name, ..) => *name,
                _ => continue,
            };
            if let Some(i) = clause.params.iter().position(|p| p.name == name) {
                if i < results.len() && !named[i] {
                    named[i] = true;
                    self.capture_wrap(results[i], Wrap::Named(name));
                }
            }
        }
    }

    /// Named arguments are evaluated as written, not in parameter order: when the order differs
    /// and two or more of them have effects, they are bound to temporaries in call order, as
    /// are the arguments of the clauses before them.
    fn keep_call_order(&mut self, order: &[usize], results: &mut [TExprId], earlier: &mut [TExprId], span: Span) -> bool {
        let inverted = order.windows(2).any(|w| w[0] > w[1]);
        if !inverted || self.annotation_args || order.iter().filter(|&&i| !self.is_stable(results[i])).count() < 2 {
            return false;
        }
        // Pending plain calls count by their expansions, as where every call expanded as typed.
        let pending: Vec<TExprId> = results.iter().copied().filter(|&e| self.is_pending_call(e)).collect();
        if !pending.is_empty() {
            self.expand_pending_kept(&pending);
            if order.iter().filter(|&&i| !self.is_stable(results[i])).count() < 2 {
                return false;
            }
        }
        let mark = self.hoisted.len();
        let mut done = vec![false; results.len()];
        for &i in order {
            if !done[i] {
                done[i] = true;
                results[i] = self.hoist(results[i], ANY, span);
            }
        }
        self.hoist_before(mark, earlier, span);
        true
    }

    /// The receiver of a call whose arguments were bound to temporaries is evaluated before them.
    fn hoist_receiver(&mut self, call: &mut MethodCall, mark: usize, span: Span) {
        if let Some(r) = call.recv {
            let mut recv = [r];
            self.hoist_before(mark, &mut recv, span);
            call.recv = Some(recv[0]);
        }
    }

    /// Binds `e`, unless it is stable, to a temporary of its recorded type placed at `at` among
    /// the hoisted temporaries, and returns the reference: a value class's receiver keeps its
    /// type, which an `Any` temporary would box.
    fn hoist_typed_at(&mut self, at: usize, e: TExprId, span: Span) -> TExprId {
        self.expand_before_binding(e);
        if self.is_pure_prefix(e) {
            return e;
        }
        let ty = self.prog.type_of(e).unwrap_or(ANY);
        let tmp = self.indexed_local("h", self.hoisted.len() as u32, ty, span);
        if self.capturing() {
            if let Some(own) = self.prog.type_of(e) {
                self.capture_local(tmp, |l| l.ty = Some(own));
            }
        }
        if self.inline.checking > 0 {
            self.note_hoisted(tmp);
        }
        self.hoisted.insert(at, TStmt::Val(tmp, e));
        self.prog.add(TExpr::Local(tmp))
    }

    /// What scalac's eta-expansion leaves in place of a lifted val: a literal, a lambda or a
    /// stable path (`exprPurity` at `Pure` or above).
    fn is_pure_prefix(&self, e: TExprId) -> bool {
        match self.prog.expr(e) {
            TExpr::Int(_) | TExpr::Long(_) | TExpr::Double(_) | TExpr::Bool(_) | TExpr::Char(_) | TExpr::Str(_) | TExpr::Unit | TExpr::Lambda(..) => true,
            // A pending call is its expansion's, whose effects a path would move.
            _ => self.is_stable_path(e) && !self.is_pending_call(e),
        }
    }

    /// Binds the effectful expressions of `exprs`, which are evaluated before the temporaries
    /// hoisted since `mark`, to temporaries placed in front of them.
    pub fn hoist_before(&mut self, mark: usize, exprs: &mut [TExprId], span: Span) {
        let mut at = mark;
        for e in exprs.iter_mut() {
            self.expand_before_binding(*e);
            if !self.is_stable(*e) {
                let tmp = self.indexed_local("h", self.hoisted.len() as u32, ANY, span);
                if self.capturing() {
                    if let Some(ty) = self.prog.type_of(*e) {
                        self.capture_local(tmp, |l| l.ty = Some(ty));
                    }
                }
                self.hoisted.insert(at, TStmt::Val(tmp, *e));
                *e = self.prog.add(TExpr::Local(tmp));
                at += 1;
            }
        }
    }

    /// None when an argument is named, spliced or already typed, which rules out auto-tupling.
    fn positional_args(&self, args: &[ArgSrc]) -> Option<Vec<ExprId>> {
        let ast = self.cur_ast();
        args.iter()
            .map(|a| match a {
                ArgSrc::Ast(e) | ArgSrc::Hoisted(e) => match ast.expr(*e) {
                    Expr::NamedArg(..) => None,
                    Expr::Typed(_, t) if matches!(ast.ty(t), TyExpr::Repeated(_)) => None,
                    _ => Some(*e),
                },
                ArgSrc::Typed(..) | ArgSrc::Named(..) | ArgSrc::ForLambda(..) => None,
            })
            .collect()
    }

    /// Auto-tupling: several arguments for a single parameter are passed as one tuple.
    fn type_tupled_args(&mut self, param: &ParamSig, elems: &[ExprId], subst: &Subst, span: Span) -> TExprId {
        let pty = self.param_type(param.ty, subst);
        let target = self.deref(pty);
        let takes_tuple = matches!(self.types.get(target), Type::Class(c, _) if self.is_tuple_class(c))
            || self.holds_tuple(pty, elems.len());
        let (te, ty) = self.type_tupled_dual(elems, pty);
        self.last_arg_types.clear();
        self.last_arg_types.push(ty);
        let te = if takes_tuple {
            self.adapt(te, ty, pty, span)
        } else {
            self.error(span, "too many arguments: expected 1");
            te
        };
        if param.by_name {
            self.by_name_thunk(te)
        } else {
            te
        }
    }

    fn type_arg(&mut self, arg: ArgSrc, pty: TypeId, span: Span) -> TExprId {
        if self.b.by_name.is_some() {
            if let Some(inner) = self.by_name_arg(pty) {
                return self.type_by_name_arg(arg, inner, span);
            }
        }
        self.type_arg_as(arg, pty, span)
    }

    /// An argument of a function's by-name parameter `=> A`, passed unevaluated.
    #[cold]
    #[inline(never)]
    fn type_by_name_arg(&mut self, arg: ArgSrc, inner: TypeId, span: Span) -> TExprId {
        let te = self.type_arg_as(arg, inner, span);
        self.by_name_thunk(te)
    }

    /// A polymorphic function instantiated without arguments (`f[Int]`), eta-expanded as scalac
    /// expands it: `(a0: Int) => f.apply[Int](a0)` of the instantiated function type `fun`, an
    /// unstable `f` evaluated once before, the call's type arguments recorded for the pickle.
    #[allow(clippy::too_many_arguments)]
    fn eta_poly_function(&mut self, f: TExprId, poly: TypeId, fun: TypeId, ptys: &[TypeId], ret: TypeId, targs: &[TypeId], span: Span) -> (TExprId, TypeId) {
        let mark = self.hoisted.len();
        let f = self.hoist(f, poly, span);
        let locals: Vec<SymId> = ptys.iter().enumerate().map(|(i, &t)| self.indexed_local("a", i as u32, t, span)).collect();
        let args: Vec<TExprId> = locals.iter().map(|&s| self.prog.add(TExpr::Local(s))).collect();
        for (&a, &t) in args.iter().zip(ptys) {
            self.prog.set_type(a, t);
        }
        let args = self.prog.list(&args);
        let call = self.prog.add(TExpr::CallClosure(f, args));
        self.prog.set_type(call, ret);
        if self.capturing() {
            self.capture_targs(call, targs);
        }
        let ty = fun;
        let ps = self.prog.syms(&locals);
        let lambda = self.prog.add(TExpr::Lambda(ps, call));
        self.prog.set_type(lambda, ty);
        let expanded = self.wrap_hoisted(mark, lambda);
        self.prog.set_type(expanded, ty);
        (expanded, ty)
    }

    /// The parameter of an eta-expansion expected to be by-name (`(=> Int) => Int` for a
    /// `twice(x: => Int)`): a thunk, passed on to a by-name parameter and evaluated for
    /// another; an inline callee substitutes the argument where it reads the parameter, so it
    /// gets the thunk's evaluation, as a by-name argument is written.
    #[cold]
    #[inline(never)]
    fn eta_by_name_param(&mut self, inner: TypeId, pty: TypeId, by_name_param: bool, callee: SymId, index: u32, span: Span) -> (SymId, TExprId, TypeId) {
        self.is_sub(inner, pty);
        let pty = self.solve_maximized_in(pty);
        let s = self.indexed_local("eta", index, pty, span);
        self.syms.sym_mut(s).by_name = true;
        let mut arg = self.prog.add(TExpr::Local(s));
        let inline = self.is_inline_callee(callee);
        if !by_name_param || inline {
            arg = self.prog.add(TExpr::CallClosure(arg, ListRef::EMPTY));
        }
        if by_name_param && inline {
            arg = self.by_name_thunk(arg);
        }
        (s, arg, self.by_name_type(pty))
    }

    #[inline]
    fn type_arg_as(&mut self, arg: ArgSrc, pty: TypeId, span: Span) -> TExprId {
        let spread = std::mem::take(&mut self.spread_arg);
        match arg {
            ArgSrc::Ast(e) | ArgSrc::Hoisted(e) => {
                if self.arg_cache.is_some() && self.watches_arg(e) {
                    return self.type_arg_recorded(arg, e, pty, spread);
                }
                // An argument an earlier attempt over the same application typed is adapted, not
                // typed again (`state::ArgCache`).
                let (te, ty) = match self.cached_arg(e) {
                    Some(typed) => typed,
                    None => {
                        let retained = self.arg_typing_begin(e);
                        let typed = self.type_expr(e, Some(pty));
                        // What the typing of a cached argument wrote is the argument's.
                        if self.cache_arg(e, typed) {
                            self.promote_end(retained);
                        }
                        typed
                    }
                };
                let arg_span = self.cur_ast().expr_span(e);
                let te = match arg {
                    ArgSrc::Hoisted(_) => self.hoist(te, ty, arg_span),
                    _ => te,
                };
                self.mark_spread_array(spread, te, ty);
                self.typed_arg(te, ty, pty, arg_span)
            }
            ArgSrc::Typed(te, ty) | ArgSrc::Named(_, te, ty) => {
                self.mark_spread_array(spread, te, ty);
                self.typed_arg(te, ty, pty, span)
            }
            ArgSrc::ForLambda(pat, rest) => {
                let (te, ty) = self.type_for_lambda(pat, rest, pty, span);
                self.note_erroneous_arg(ty);
                self.adapt(te, ty, pty, span)
            }
        }
    }

    /// An argument whose type holds the error type, noted at the depth of the application it is
    /// an argument of (`Worker::erroneous_args`); a lambda whose body's type does is noted by
    /// `typed_arg` (`Worker::erroneous_lambdas`).
    #[inline]
    fn note_erroneous_arg(&mut self, ty: TypeId) {
        if self.types.contains_error(ty) {
            self.erroneous_args.push(self.app_depth);
        }
    }

    fn typed_arg(&mut self, te: TExprId, ty: TypeId, pty: TypeId, span: Span) -> TExprId {
        if self.erroneous_lambdas.last() == Some(&te) {
            self.erroneous_lambdas.pop();
            self.erroneous_args.push(self.app_depth);
        } else {
            self.note_erroneous_arg(ty);
        }
        // A parameter of the body under expansion is ranked, inferred from and adapted at its
        // declared type, as scalac resolved the body at the definition; the binding of a nested
        // inline call takes the argument's own type, as scalac's binding does.
        let own = ty;
        let ty = self.proxy_declared(te).unwrap_or(ty);
        self.last_arg_types.clear();
        self.last_arg_types.push(ty);
        self.arg_types_seen.push((te, ty));
        if self.inline.record_args {
            self.inline.arg_types.push((te, own));
        }
        // A numeric argument for a still-open type parameter may need widening once the
        // parameter is solved: Math.min(someLong, 1) infers Long and widens the 1.
        if matches!(self.types.get(pty), Type::Var(_)) {
            let open = self.deref(pty);
            if matches!(self.types.get(open), Type::Var(_)) {
                let actual = self.deref(ty);
                if self.is_numeric(actual).is_some() {
                    self.pending_widenings.push((te, actual, pty));
                }
            }
        }
        let ty = self.stable_arg_for_singleton(te, ty, pty);
        self.adapt(te, ty, pty, span)
    }

    /// Whether `e` is an argument the member's application under `apply_member_or_extension`
    /// was given (`ArgCache::watched`).
    #[inline]
    fn watches_arg(&self, _e: ExprId) -> bool {
        self.arg_cache.as_ref().is_some_and(|c| c.active && c.file == self.env.file)
    }

    /// Where a typing starts: the trail's, the diagnostics', the temporaries', the index's and
    /// the type variables' lengths.
    fn typing_marks(&self) -> (usize, usize, usize, super::index::Recorded, usize) {
        (self.trail.len(), self.diags.items.len(), self.hoisted.len(), self.index_mark(), self.tvars.len())
    }

    /// The bindings the trail took under the member's application (`taken.undone`) from the
    /// typing `ty`'s start to `to`, all of variables made by the typing: what the retry that
    /// reuses it brings back. `None` where one is of an older variable, the application's. A
    /// tree of a type without variables, a literal or an instance creation of such, depends on
    /// none.
    fn bindings_in(&self, taken: &Taken, ty: &ArgTyping, to: usize) -> Option<Vec<super::Redo>> {
        use super::Redo;
        if !self.types.has_vars(ty.own) && self.closed_tree(ty.te) {
            return Some(Vec::new());
        }
        // Typed against a formal over variables older than the application, which an enclosing
        // typing had constrained before it: what the typing made of them is undone with the
        // application. The application's own keep their instances (`Taken::vars`).
        if ty.formal.is_some_and(|f| ty.trail().0 > taken.mark && self.mentions_var_before(f, taken.vars)) {
            return None;
        }
        let (from, vars) = (ty.trail().0, ty.vars as usize);
        // The typing's segment lies in the record (the trail grew from the application's mark
        // to its end without a rollback below a typing's end); where it does not, nothing is
        // reused.
        if from < taken.mark || to < from || to - taken.mark > taken.undone.len() {
            return None;
        }
        let mut out = Vec::with_capacity(to - from);
        for k in from..to {
            let r = taken.undone[taken.undone.len() - 1 - (k - taken.mark)];
            let (Redo::Inst(v, _) | Redo::Lower(v, _) | Redo::Upper(v, _)) = r;
            if !self.tvars.made_since(v, vars) {
                // One of the application's variables, made before the typing: its instance stays,
                // its bounds go with the application; neither is the typing's to bring back.
                if self.tvars.made_since(v, taken.vars) {
                    continue;
                }
                return None;
            }
            out.push(r);
        }
        Some(out)
    }

    /// Whether `t` mentions a type variable this worker made before the first `vars`, or another
    /// worker's.
    fn mentions_var_before(&self, t: TypeId, vars: usize) -> bool {
        if !self.types.has_vars(t) {
            return false;
        }
        let mut found = Vec::new();
        self.collect_vars(t, &mut found);
        found.iter().any(|&v| !self.tvars.made_since(v, vars))
    }

    /// Whether no type variable is in the tree `te`: a literal, or an instance creation of a type
    /// with none (a tuple's among them) of such arguments.
    fn closed_tree(&self, te: TExprId) -> bool {
        match self.prog.expr(te) {
            TExpr::Int(_) | TExpr::Long(_) | TExpr::Double(_) | TExpr::Bool(_) | TExpr::Char(_) | TExpr::Str(_) | TExpr::Unit => true,
            // A node's static type is recorded for the class-based targets alone: where it is,
            // it has to be free of variables; on JavaScript the typing's own type stands for it.
            TExpr::New(_, l) => self.prog.type_of(te).is_none_or(|t| !self.types.has_vars(t)) && self.prog.expr_list(l).iter().all(|&a| self.closed_tree(a)),
            _ => false,
        }
    }

    /// Brings back the bindings `bindings_in` read, in their order.
    fn rebind(&mut self, bindings: &[super::Redo]) {
        use super::{Redo, Undo};
        for &r in bindings {
            match r {
                Redo::Inst(v, t) => self.instantiate(v, t),
                Redo::Lower(v, t) => {
                    self.tvars[v].lower.push(t);
                    self.trail.push(Undo::Lower(v));
                }
                Redo::Upper(v, t) => {
                    self.tvars[v].upper.push(t);
                    self.trail.push(Undo::Upper(v));
                }
            }
        }
    }

    /// The record of `e`'s typing begun at `at`, against `formal`, to `te` of type `own`, as
    /// the typing ended: adapted where `adapted`.
    #[inline]
    fn arg_typing(&self, e: ExprId, formal: Option<TypeId>, (te, own): (TExprId, TypeId), at: (usize, usize, usize, super::index::Recorded, usize), typed: (usize, usize), adapted: Option<TExprId>) -> ArgTyping {
        ArgTyping {
            e,
            formal,
            te,
            own,
            ty: own,
            adapted,
            trail: (at.0 as u32, typed.0 as u32, self.trail.len() as u32),
            vars: at.4 as u32,
            diags: (at.1 as u32, typed.1 as u32, self.diags.items.len() as u32),
            hoisted: self.hoisted.len() > at.2,
            index: (at.3, self.index_mark()),
            dual: None,
            untyped: false,
        }
    }

    /// `type_arg_as` of an argument the member's application under `apply_member_or_extension`
    /// was given: its typing, and that adapted to the formal, recorded for the retry.
    #[inline(never)]
    fn type_arg_recorded(&mut self, arg: ArgSrc, e: ExprId, pty: TypeId, spread: bool) -> TExprId {
        let at = self.typing_marks();
        // A function literal whose parameters the formal gives no types: dotty's test leaves it
        // untyped (`functionWithUnknownParamType`, ProtoTypes.scala 475) and its retry types it
        // against the extension's formal. `type_lambda` reports the missing types and types none
        // of it under `Worker::untyped_lambda`, so that the retry's typing is its first.
        let ast = self.cur_ast();
        let literal = match ast.expr(e) {
            crate::ast::Expr::Lambda(..) => true,
            crate::ast::Expr::Parens(inner) => matches!(ast.expr(inner), crate::ast::Expr::Lambda(..)),
            _ => false,
        };
        if literal {
            self.untyped_lambda = Some(false);
        }
        // The cache is out while the argument is typed: what is typed inside it is no argument
        // of the application.
        let cache = self.arg_cache.take();
        let retained = self.retained_typing(&cache);
        let (te, own) = self.type_expr(e, Some(pty));
        self.retained_typing_end(retained, at.1);
        let untyped = literal && self.untyped_lambda.take() == Some(true);
        let typed = (self.trail.len(), self.diags.items.len());
        let arg_span = self.cur_ast().expr_span(e);
        let value = match arg {
            ArgSrc::Hoisted(_) => self.hoist(te, own, arg_span),
            _ => te,
        };
        self.mark_spread_array(spread, value, own);
        let adapted = self.typed_arg(value, own, pty, arg_span);
        let mut typing = self.arg_typing(e, Some(pty), (te, own), at, typed, Some(adapted));
        typing.untyped = untyped;
        self.arg_cache = cache;
        if let Some(c) = self.arg_cache.as_mut() {
            c.typings.push(typing);
        }
        adapted
    }

    /// The typing of an argument the member's application under `cache` records begins: a clean
    /// one's rarely written journals stay through the application's set-aside (dotty's cached
    /// typing, `cacheTypedArg` taking it where nothing failed, ProtoTypes.scala 488).
    fn retained_typing(&self, cache: &Option<Box<ArgCache>>) -> Option<super::state::PromoteMark> {
        let m = cache.as_ref()?.attempt?;
        Some(self.retained_typing_begin(&m))
    }

    fn retained_typing_end(&mut self, retained: Option<super::state::PromoteMark>, diags: usize) {
        if retained.is_some() && !self.diags.items[diags..].iter().any(|d| !d.is_warning) {
            self.promote_end(retained);
        }
    }

    /// An argument of no parameter, typed alone for its errors; recorded for the retry where
    /// the member's application under `apply_member_or_extension` was given it.
    fn type_extra_arg(&mut self, e: ExprId) {
        if !(self.arg_cache.is_some() && self.watches_arg(e)) {
            self.type_expr(e, None);
            return;
        }
        let at = self.typing_marks();
        let cache = self.arg_cache.take();
        let retained = self.retained_typing(&cache);
        let typed = self.type_expr(e, None);
        self.retained_typing_end(retained, at.1);
        let end = (self.trail.len(), self.diags.items.len());
        let typing = self.arg_typing(e, None, typed, at, end, None);
        self.arg_cache = cache;
        if let Some(c) = self.arg_cache.as_mut() {
            c.typings.push(typing);
        }
    }

    /// `type_tuple` of the tupled dual of the arguments `elems` against `pty`, recorded for the
    /// retry where the member's application under `apply_member_or_extension` was given them.
    fn type_tupled_dual(&mut self, elems: &[ExprId], pty: TypeId) -> (TExprId, TypeId) {
        if !(self.arg_cache.is_some() && self.watches_arg(elems[0])) {
            return self.type_tuple(elems, Some(pty));
        }
        let alone = self.tuple_expected_elements(Some(pty), elems.len()).is_none();
        let at = self.typing_marks();
        let cache = self.arg_cache.take();
        let retained = self.retained_typing(&cache);
        let typed = self.type_tuple(elems, Some(pty));
        self.retained_typing_end(retained, at.1);
        let end = (self.trail.len(), self.diags.items.len());
        let mut typing = self.arg_typing(elems[0], Some(pty), typed, at, end, None);
        typing.dual = Some(alone);
        self.arg_cache = cache;
        if let Some(c) = self.arg_cache.as_mut() {
            c.typings.push(typing);
        }
        typed
    }

    /// A stable argument for a parameter bounded by `Singleton` keeps its path type
    /// (`Foo(x)` of a `Foo[T <: Singleton](t: T)` is a `Foo[x.type]`).
    fn stable_arg_for_singleton(&mut self, te: TExprId, ty: TypeId, pty: TypeId) -> TypeId {
        if self.b.t_singleton == ERROR {
            return ty;
        }
        if !self.bounded_by_singleton(pty) {
            return ty;
        }
        if let Some(path) = self.path_of(te) {
            return path;
        }
        match self.prog.expr(te) {
            TExpr::Int(_) | TExpr::Long(_) | TExpr::Double(_) | TExpr::Bool(_) | TExpr::Char(_) | TExpr::Str(_) if !self.widened_constant(te) => match self.fold_constant(te) {
                Some(v) => self.types.lit(v),
                None => ty,
            },
            _ => ty,
        }
    }

    /// Whether `u` is `Singleton`, an intersection with it, or a variable bounded by one.
    pub(super) fn bounded_by_singleton(&mut self, u: TypeId) -> bool {
        self.b.t_singleton != ERROR && self.bounded_by_singleton_in(u, 8)
    }

    /// `depth` bounds the variables followed, which may name each other.
    fn bounded_by_singleton_in(&mut self, u: TypeId, depth: u32) -> bool {
        let u = self.deref(u);
        match self.types.get(u) {
            Type::Inter(a, b) => self.bounded_by_singleton_in(a, depth) || self.bounded_by_singleton_in(b, depth),
            Type::Var(v) if depth > 0 => {
                for i in 0..self.tvars[v].upper.len() {
                    let w = self.tvars[v].upper[i];
                    if self.bounded_by_singleton_in(w, depth - 1) {
                        return true;
                    }
                }
                false
            }
            _ => u == self.b.t_singleton,
        }
    }

    pub(super) fn type_for_lambda(&mut self, binder: ForBinder, rest: ForRest, expected: TypeId, span: Span) -> (TExprId, TypeId) {
        let Some((ptys, ret)) = self.as_function(expected) else {
            self.error(span, "this value cannot be used in a for expression");
            return (self.prog.add(TExpr::Unit), ERROR);
        };
        let open = self.deref(ptys[0]);
        let unknown = matches!(self.types.get(open), Type::Var(v) if self.tvars[v].lower.is_empty() && self.tvars[v].upper.is_empty());
        let elem = self.solve_if_var(ptys[0]);
        if unknown && ptys.len() == 1 && matches!(binder, ForBinder::Pat(_) | ForBinder::CasePat(_)) {
            self.error(span, "cannot infer the element type of this generator");
        }
        let ret_expected = self.concrete_expected(Some(ret));
        self.for_bases.push(self.env.frames.len());
        self.push_scope();
        let ast = self.cur_ast();
        let lambda_span = match binder {
            ForBinder::Pat(p) | ForBinder::CasePat(p) => ast.pat_spans[p.idx()],
            ForBinder::Unpack(_) => span,
        };
        self.sites.owners.push(super::site::SiteOwner::Lambda(lambda_span));
        let result = match binder {
            ForBinder::Unpack(id) => {
                let sym = self.fresh_local("u", ANY, span);
                let vars = self.for_packs[id as usize].clone();
                let tuple = if self.capturing() { self.pack_tuple(&vars) } else { None };
                if let Some(elems) = tuple {
                    let tys: Vec<TypeId> = self.types.items(elems).to_vec();
                    if let Some(c) = self.b.tuples.get(tys.len()).copied().flatten() {
                        let t = self.types.class(c, &tys);
                        self.capture_local(sym, |l| l.ty = Some(t));
                    }
                }
                let mut stmts = Vec::with_capacity(vars.len());
                for (i, (name, ty, given)) in vars.into_iter().enumerate() {
                    let local = self.new_local(name, SymKind::Val, ty, span);
                    self.bind_local(name, local);
                    if given {
                        self.syms.sym_mut(local).mods |= crate::ast::mods::GIVEN;
                        self.bind_given(local);
                    }
                    let packed = self.prog.add(TExpr::Local(sym));
                    stmts.push(TStmt::Val(local, self.prog.add(TExpr::Index(packed, i as u32))));
                }
                let (tb, bty) = self.type_for_rest(rest, span, ret_expected);
                let l = self.prog.stmts.push_slice(&stmts);
                let unpacked = self.prog.add(TExpr::Block(l, tb));
                if let Some(t) = tuple {
                    self.capture_form(unpacked, Form::Unpack(t));
                }
                (sym, unpacked, bty)
            }
            ForBinder::Pat(pat) | ForBinder::CasePat(pat) => match ast.pat(pat) {
                // A generator's identifier is a variable, capitalised or not.
                Pat::Bind(name, None) => {
                    let sym = self.new_local(name, SymKind::Val, elem, span);
                    if self.index.is_some() {
                        self.index_local_at(sym, ast.pat_spans[pat.idx()]);
                    }
                    self.bind_local(name, sym);
                    let (tb, bty) = self.type_for_rest(rest, span, ret_expected);
                    (sym, tb, bty)
                }
                Pat::StableId(path) if matches!(ast.expr(path), Expr::Ident(_)) => {
                    let Expr::Ident(name) = ast.expr(path) else { unreachable!() };
                    let sym = self.new_local(name, SymKind::Val, elem, span);
                    if self.index.is_some() {
                        self.index_local_at(sym, ast.pat_spans[pat.idx()]);
                    }
                    self.bind_local(name, sym);
                    let (tb, bty) = self.type_for_rest(rest, span, ret_expected);
                    (sym, tb, bty)
                }
                Pat::Wildcard => {
                    let sym = self.fresh_local("w", elem, span);
                    let (tb, bty) = self.type_for_rest(rest, span, ret_expected);
                    (sym, tb, bty)
                }
                _ => {
                    let sym = self.fresh_local("p", elem, span);
                    let filter = matches!(rest, ForRest::Filter);
                    let checked = !filter && matches!(binder, ForBinder::Pat(_));
                    let outer = std::mem::replace(&mut self.refutable_is_error, checked);
                    let tp = self.type_pattern(pat, elem);
                    self.refutable_is_error = outer;
                    if checked {
                        self.check_irrefutable(tp, pat, elem, None);
                    }
                    let (tb, bty) = self.type_for_rest(rest, span, ret_expected);
                    let scrut = self.prog.add(TExpr::Local(sym));
                    let mut cases = vec![TCase { pat: tp, guard: None, body: tb }];
                    if filter {
                        let wildcard = self.prog.add_pat(TPat::Wildcard);
                        let no = self.prog.add(TExpr::Bool(false));
                        cases.push(TCase { pat: wildcard, guard: None, body: no });
                    }
                    let cases = self.prog.cases.push_slice(&cases);
                    (sym, self.prog.add(TExpr::Match(scrut, cases)), bty)
                }
            },
        };
        self.sites.owners.pop();
        self.for_bases.pop();
        self.pop_scope();
        let (sym, tb, bty) = result;
        // A yield's value, not typed against the call's result where that is a variable, whose
        // bounds may expect a function of it: what a completion of a name there reads.
        if let (true, None, ForRest::Rest { idx, end, body, is_yield: true }) = (self.index.is_some(), ret_expected, rest) {
            let generators = (idx..end).any(|i| matches!(self.cur_ast().enumerators[i as usize], crate::ast::Enumerator::Gen(..) | crate::ast::Enumerator::CaseGen(..)));
            if !generators && self.expects_function(ret, 4) {
                let mut names = Vec::new();
                super::index::result_names(self.cur_ast(), body, &mut names);
                for name in names {
                    self.index_function_name(name, tb);
                }
            }
        }
        let params = self.prog.syms(&[sym]);
        let ty = self.fun_type(&[elem], bty);
        (self.prog.add(TExpr::Lambda(params, tb)), ty)
    }

    pub fn apply_member(
        &mut self,
        recv: TExprId,
        recv_ty: TypeId,
        name: Name,
        targs: Option<ListRef>,
        lists: Vec<ArgList>,
        span: Span,
        expected: Option<TypeId>,
    ) -> (TExprId, TypeId) {
        let (te, ty) = self.apply_member_of_class(recv, recv_ty, name, targs, lists, span, expected);
        self.narrowed_by_refinement(te, ty, recv, recv_ty, name)
    }

    /// A member of a receiver whose type refines it (`Base { val v: Int }`, `Ops { def m(x:
    /// Int): A }`) has the refinement's result: the class's member is called, the refinement
    /// gives its type.
    fn narrowed_by_refinement(&mut self, te: TExprId, ty: TypeId, recv: TExprId, recv_ty: TypeId, name: Name) -> (TExprId, TypeId) {
        if ty == ERROR || !self.types.term_refinements() || matches!(self.types.get(recv_ty), Type::Class(..) | Type::Lit(_) | Type::Param(_) | Type::Error) {
            return (te, ty);
        }
        let under = self.widen_path(recv_ty);
        let Some(r) = self.refined_term(under, name) else { return (te, ty) };
        let sig = self.refinement_sig(r);
        if !sig.tparams.is_empty() || sig.ret == ty {
            return (te, ty);
        }
        // Only a call of the member the refinement describes: an extension or a conversion the
        // call fell back to has its own result.
        match self.refined_member(under, name, &sig) {
            Some(member) if self.calls_member(te, recv, member) => {}
            _ => return (te, ty),
        }
        let mark = self.trail.len();
        if !self.is_sub(sig.ret, ty) {
            self.rollback(mark);
            return (te, ty);
        }
        self.prog.set_type(te, sig.ret);
        (te, sig.ret)
    }

    /// The member the refinement `sig` of `name` describes: the one member of that name the
    /// class of `under` has, with its parameter types. An overload the refinement does not
    /// describe keeps its own result.
    fn refined_member(&mut self, under: TypeId, name: Name, sig: &MethodSig) -> Option<SymId> {
        let (member, owner_ty) = self.find_member(under, name)?;
        if self.syms.alternatives(member).is_some() {
            return None;
        }
        let theirs = self.sig_of(member);
        let shape = |s: &MethodSig| s.clauses.iter().map(|c| c.params.len()).collect::<Vec<_>>();
        if !theirs.tparams.is_empty() || shape(theirs) != shape(sig) {
            return None;
        }
        let pairs: Vec<(TypeId, TypeId)> =
            theirs.clauses.iter().flat_map(|c| c.params.iter()).zip(sig.clauses.iter().flat_map(|c| c.params.iter())).map(|(p, q)| (p.ty, q.ty)).collect();
        let subst = self.owner_subst(owner_ty);
        let mark = self.trail.len();
        let same = pairs.into_iter().all(|(p, q)| {
            let p = self.types.subst(p, &subst);
            self.is_same(p, q)
        });
        if !same {
            self.rollback(mark);
        }
        same.then_some(member)
    }

    /// Whether the typed application `te` is a call or a read of `member` itself on `recv`, not
    /// on something a conversion made of it.
    fn calls_member(&self, te: TExprId, recv: TExprId, member: SymId) -> bool {
        let mut e = te;
        while let TExpr::Block(_, inner) = self.prog.expr(e) {
            e = inner;
        }
        matches!(self.prog.expr(e), TExpr::Field(r, s) | TExpr::CallMethod(r, s, _) if s == member && r == recv)
    }

    fn apply_member_of_class(
        &mut self,
        recv: TExprId,
        recv_ty: TypeId,
        name: Name,
        targs: Option<ListRef>,
        lists: Vec<ArgList>,
        span: Span,
        expected: Option<TypeId>,
    ) -> (TExprId, TypeId) {
        // A converted receiver takes no second conversion.
        let converted = std::mem::take(&mut self.no_receiver_conversion);
        let recv_ty = match self.enum_case_new {
            Some((te, precise)) if te == recv => precise,
            _ => {
                let solved = self.solve_bounded_in(recv_ty);
                self.capture_wildcards_in(recv, solved)
            }
        };
        if recv_ty == ERROR {
            self.type_args_for_errors(&lists);
            return (self.prog.add(TExpr::Unit), ERROR);
        }
        // `s.+(x)`: the member `+(x$0: Any)` scalac gives `String` (`String_+`), which is the
        // concatenation that `s + x` makes.
        let lists = if name == names::PLUS && self.stands_for_string(recv_ty) {
            let lists = self.string_plus_argument(lists);
            let t_string = self.b.t_string;
            if let Some(r) = self.prim_op_on(recv, t_string, name, targs, &lists, span, expected) {
                return r;
            }
            lists
        } else {
            lists
        };
        // A parameter of the body under expansion takes the extensions and the conversions of
        // its declared type, as scalac chose them at the definition; the receiver keeps the
        // argument's own type for the binding of an inline extension.
        let resolved_ty = self.proxy_declared(recv).unwrap_or(recv_ty);
        let recv_head = self.deref(recv_ty);
        if name == names::APPLY && matches!(self.types.get(recv_head), Type::Poly(..)) {
            return self.apply_callee(Callee::Value(recv, recv_ty), targs, lists, span, expected);
        }
        // A receiver that is a function type naming its parameters only through a path, a type
        // parameter's bound, an intersection or a further refinement (`g: f.type`, `F <: (c:
        // Ctx) => c.T`) is called as that function type, which instantiates the result.
        if name == names::APPLY && !matches!(self.types.get(recv_head), Type::Class(..)) && self.named_fun_parent(recv_head).is_none() {
            if let Some(fun) = self.named_function_within(recv_head) {
                return self.apply_callee(Callee::Value(recv, fun), targs, lists, span, expected);
            }
        }
        if let Some((params, ret)) = self.as_function(recv_ty) {
            if name == names::APPLY {
                return self.apply_callee(Callee::Value(recv, recv_ty), targs, lists, span, expected);
            }
            // A function of more than 22 parameters has neither, as scalac's FunctionXXL.
            if targs.is_none() && matches!(name, names::TUPLED | names::CURRIED) && (2..=22).contains(&params.len()) {
                let (te, ty) = self.function_conversion(recv, &params, ret, name, span);
                return self.apply_callee(Callee::Value(te, ty), None, lists, span, expected);
            }
        } else if targs.is_none() && matches!(name, names::TUPLED | names::CURRIED) && self.find_member(recv_ty, name).is_none() {
            // A class that extends a function type (`object Die extends AbstractFunction2`) has
            // the function's `tupled` and `curried`.
            if let Some((params, ret)) = self.function_base(recv_ty).filter(|(ps, _)| ps.len() >= 2) {
                let (te, ty) = self.function_conversion(recv, &params, ret, name, span);
                return self.apply_callee(Callee::Value(te, ty), None, lists, span, expected);
            }
        }
        // Where an opaque type is transparent, the members of its underlying type win over
        // extensions on the opaque type, as members always do. A path keeps its type: the
        // member is seen from it (`this += x` is `this.type`).
        let underlying = self.dealias(recv_ty);
        if underlying != recv_ty && !self.types.is_path(recv_ty) && self.has_member_like(underlying, name) {
            self.no_receiver_conversion = converted;
            return self.apply_member(recv, underlying, name, targs, lists, span, expected);
        }
        if let Some(r) = self.named_tuple_member(recv, recv_ty, name, targs, &lists, span, expected) {
            return r;
        }
        let dynamic = self.is_js_dynamic(recv_ty);
        if dynamic && matches!(name, names::SELECT_DYNAMIC | names::UPDATE_DYNAMIC | names::APPLY_DYNAMIC) {
            return self.apply_dynamic_member(recv, name, targs, lists, span, expected);
        }
        let mut lists = lists;
        // The tuple cons and the members of `Tuple` are the builtin's, whatever the library
        // declares on `Tuple`.
        // The type arguments of `Tuple`'s signatures, which a jar body passes (`*:[H, This]`,
        // `head[This]`): `This` is the receiver's type, which the builtin reads from the
        // receiver, and `H` the element's.
        let tuple_targs: Option<Vec<crate::ast::TyExprId>> = targs.map(|l| self.cur_ast().ty_list(l).to_vec());
        if name == names::CONS_TUPLE && matches!(tuple_targs.as_deref(), None | Some([_, _])) {
            let mut cons_lists = lists.clone();
            if let (Some([h, _]), [list]) = (tuple_targs.as_deref(), cons_lists.as_mut_slice()) {
                if let [ArgSrc::Ast(e)] = list.args.as_slice() {
                    let hty = self.resolve_type(*h);
                    let te = self.check_expr(*e, hty);
                    list.args[0] = ArgSrc::Typed(te, hty);
                }
            }
            if let Some(r) = self.tuple_cons(recv, recv_ty, &cons_lists, span) {
                return r;
            }
        }
        if matches!(tuple_targs.as_deref(), None | Some([_])) {
            if let Some(r) = self.tuple_member(recv, recv_ty, name, &lists, span) {
                return r;
            }
        }
        // A parameter's member is the one its declared type has, a deferred inline member's
        // implementation the one its own type's class has.
        let mut member = match self.parameter_member(recv, recv_ty, name) {
            Some(m) => Some(m),
            None => self.find_member(recv_ty, name).or_else(|| self.product_member(recv_ty, name)),
        };
        // A library body's selection of an overloaded name calls the alternative its TASTy
        // declares, seen from the receiver's type at the class that declares it.
        let mut declared = false;
        if self.declared_call.is_some() {
            if let Some(taken) = member.and_then(|(found, _)| self.take_declared(recv, recv_ty, name, found)) {
                member = Some(taken);
                declared = true;
            }
        }
        if let Some(helper) = member.and_then(|(sym, _)| self.product_helper(sym)) {
            match lists.first_mut() {
                Some(first) if !first.using => first.args.insert(0, ArgSrc::Typed(recv, recv_ty)),
                _ => lists.insert(0, ArgList { args: vec![ArgSrc::Typed(recv, recv_ty)], using: false, span }),
            }
            let callee = Callee::Method { recv: None, sym: helper, owner_subst: Vec::new(), prefix: None };
            let (te, ty) = self.apply_callee(callee, targs, lists, span, expected);
            if self.capturing() {
                if let Some((sym, _)) = member {
                    self.capture_form(te, Form::Member(sym));
                }
            }
            return (te, ty);
        }
        if member.is_none() {
            if let Some(refined) = self.refined_term(recv_ty, name) {
                let sig = self.refinement_sig(refined);
                return self.apply_structural(recv, recv_ty, sig, name, targs, lists, span, expected);
            }
        }
        if let Some((sym, _)) = member.filter(|&(sym, _)| !declared && self.syms.sym(sym).mods & crate::ast::mods::PRIVATE != 0) {
            // A private constructor parameter is `this`'s alone: on another instance the
            // name means the inherited member (`ChronoField.ERA.ordinal()` next to the enum's
            // `ordinal` parameter).
            let through_other = self.is_own_ctor_param(sym) && !matches!(self.prog.expr(recv), TExpr::This);
            if !self.is_accessible(sym) || through_other {
                member = self.find_inherited_member(recv_ty, name).or(member);
            }
        }
        if let Some((sym, owner_ty)) = member {
            if self.drops_java_parens(sym, &lists) {
                lists.remove(0);
            }
            // `Codec(enc, dec)` next to the summoner `Codec.apply[A](using Codec[A])`: arguments
            // go to the constructor, which the summoner cannot take. Not in a library body, whose
            // `Codec.apply[A](ev)` is the summoner with the given scalac inferred as a plain list
            // (its creator applications are pickled as `new`).
            if name == names::APPLY && lists.iter().any(|l| !l.using) && self.is_summoner(sym) && !self.is_body_file(self.env.file) {
                if let Some(c) = self.class_of(recv_ty).and_then(|o| self.companion_class(o)) {
                    return self.apply_companion_apply(c, targs, lists, span, expected);
                }
            }
            if let (names::APPLY, Some(ta), true) = (name, targs, lists.is_empty()) {
                if let Some(c) = self.class_of(recv_ty).and_then(|o| self.companion_class(o)) {
                    if let Some(r) = self.summon_type_application(c, Some(sym), ta, span, expected) {
                        return r;
                    }
                }
            }
            // An extension with the same name, or a conversion, can take over an inaccessible
            // member, which is as good as absent for them, and a member selected without an
            // argument list where a parameterless extension stands in. A member whose arity does
            // not fit its argument lists is applied, and gives way only as dotty's retry on the
            // qualifier has it (`retry_on_qualifier`): its arguments are typed against its
            // parameters first.
            let inaccessible = !declared && !self.is_accessible(sym);
            let without_lists = lists.iter().all(|l| l.using);
            let unfit = !declared && (inaccessible || (without_lists && !self.member_accepts(sym, &lists, false, expected)));
            // As in scalac, a member that takes the arguments as one tuple wins over extensions.
            if unfit && (inaccessible || !self.member_takes_tuple(sym, owner_ty, &lists)) {
                if inaccessible || !self.lexical_extensions(name).is_empty() {
                    let mut opt = Some(lists);
                    if let Some(r) = self.try_extensions(recv, resolved_ty, resolved_ty, name, targs, &mut opt, span, expected) {
                        return r;
                    }
                    lists = opt.unwrap_or_default();
                }
                if !converted {
                    if let Some(r) = self.apply_through_conversion(recv, resolved_ty, name, targs, &lists, span, expected) {
                        return r;
                    }
                }
            }
            if self.syms.alternatives(sym).is_some() {
                self.no_receiver_conversion = converted;
                return self.apply_overloaded_member(recv, recv_ty, sym, name, targs, lists, span, expected);
            }
            // The `apply` that scalac gives a case class is overloaded with those its companion
            // defines. A plain class with such a companion gets no constructor proxy.
            if name == names::APPLY && !declared && self.syms.sym(sym).kind == SymKind::Def && self.constructor_next_to_apply(recv_ty).is_some() {
                return self.apply_overloaded_member(recv, recv_ty, sym, name, targs, lists, span, expected);
            }
            self.check_access(sym, Some(recv_ty), span);
            if self.syms.sym(sym).mods & crate::ast::mods::PROTECTED != 0 {
                self.check_protected_access(sym, recv, recv_ty, span);
            }
            let callee = self.member_callee_in(recv, recv_ty, Some(owner_ty), sym);
            let fits = || lists.iter().all(|l| l.args.is_empty());
            if declared || self.written_targs(targs) || without_lists || (fits() && self.member_accepts(sym, &lists, false, expected)) {
                return self.apply_callee(callee, targs, lists, span, expected);
            }
            return self.apply_member_or_extension(callee, (sym, owner_ty), recv, resolved_ty, name, targs, lists, span, expected, converted);
        }
        // A right-associative extension's arguments bound ahead of its call are placed here.
        let mark = self.hoisted.len();
        if let Some((te, ty)) = self.apply_extension_of_instance(recv, recv_ty, name, targs, &mut lists, span, expected) {
            return (self.wrap_hoisted(mark, te), ty);
        }
        if name == names::APPLY {
            // `String(bytes, UTF_8)` is `new String(bytes, UTF_8)`, which the platform layer's
            // `newString` implements; the std's `scala.String` object stands for the same statics.
            if let Type::Class(c, _) = self.types.get(recv_ty) {
                let info = self.syms.class(c);
                let scala_string = info.name == names::STRING && info.kind == ClassKind::Object && info.owner == Owner::Package(self.b.scala_pkg);
                let java_string = if info.name == names::STRING { self.java_lang_object("String") } else { None };
                if let Some(j) = java_string.filter(|&j| j == c || scala_string) {
                    let recv = self.prog.add(TExpr::Module(j));
                    let recv_ty = self.types.class(j, &[]);
                    let name = self.interner.intern("newString");
                    return self.apply_member(recv, recv_ty, name, targs, lists, span, expected);
                }
            }
            // `Point(1, 2)` with an explicit companion object that defines no `apply`.
            if let Type::Class(c, _) = self.types.get(recv_ty) {
                let info = self.syms.class(c);
                let is_module = info.kind == ClassKind::Object || info.local_module.is_some() || info.inner_object.is_some();
                if let (true, Some(companion)) = (is_module, info.companion) {
                    if self.syms.class(companion).kind == ClassKind::Class {
                        if let (Some(ta), true) = (targs, lists.is_empty()) {
                            if let Some(r) = self.summon_type_application(companion, None, ta, span, expected) {
                                return r;
                            }
                        }
                        return self.apply_companion_apply(companion, targs, lists, span, expected);
                    }
                }
            }
        }
        if let Some(r) = self.universal_member(recv, recv_ty, name, targs, &lists, span) {
            return r;
        }
        if let Some(r) = self.prim_member(recv, recv_ty, name, &lists) {
            return r;
        }
        // `E.values(i)`, as a jar body writes `E.values.apply(i)`: the array, then indexed.
        if name == names::VALUES && targs.is_none() && lists.len() == 1 && !lists[0].args.is_empty() {
            if let Some((array, array_ty)) = self.universal_member(recv, recv_ty, name, None, &[], span) {
                return self.apply_member(array, array_ty, names::APPLY, None, lists, span, expected);
            }
        }
        if let Type::Class(c, _) = self.types.get(recv_ty) {
            if self.is_module_class(c) {
                if let Some(&nested) = self.syms.class(c).nested.get(&name) {
                    // A class nested in an object nested in a class takes the object, the
                    // receiver, as its outer instance (`Product.Getter(n, f)`).
                    let inner = self.outer_class(nested).is_some();
                    let prefixed = inner || self.new_prefixed;
                    let outer_prefixed = std::mem::replace(&mut self.new_prefixed, prefixed);
                    let (te, ty) = self.apply_callee(Callee::Ctor(nested), targs, lists, span, expected);
                    self.new_prefixed = outer_prefixed;
                    let case = self.syms.class(nested).mods & crate::ast::mods::CASE != 0 || self.syms.class(nested).kind == ClassKind::EnumCase;
                    if self.deps.is_some() && case {
                        self.deps_node(te, super::deps::Node::Apply(nested));
                    }
                    if inner {
                        if let TExpr::New(k, args) = self.prog.expr(te) {
                            if args.len > 0 {
                                let mut items = self.prog.expr_list(args).to_vec();
                                items[0] = recv;
                                let l = self.prog.list(&items);
                                self.prog.exprs[te.idx()] = TExpr::New(k, l);
                            }
                        }
                        return (te, ty);
                    }
                    if self.is_path(recv) {
                        return (te, ty);
                    }
                    let stmts = self.prog.stmts.push_slice(&[TStmt::Expr(recv)]);
                    return (self.prog.add(TExpr::Block(stmts, te)), ty);
                }
            }
            // A name that the class exports refers to the original definition; the receiver
            // only has to be evaluated (an object for the body it may run).
            if let Some(&r) = self.exports_of(c).as_ref().and_then(|e| e.terms.get(&name)) {
                if let Some(callee) = self.term_callee(r, name, span) {
                    let (te, ty) = self.apply_callee(callee, targs, lists, span, expected);
                    if self.deps.is_some() {
                        self.deps_node(te, super::deps::Node::Export(c, name, recv));
                    }
                    let pure = matches!(self.prog.expr(recv), TExpr::This | TExpr::Local(_) | TExpr::Static(_));
                    if pure {
                        return (te, ty);
                    }
                    let stmts = self.prog.stmts.push_slice(&[TStmt::Expr(recv)]);
                    return (self.prog.add(TExpr::Block(stmts, te)), ty);
                }
            }
            if name == names::COPY && self.syms.class(c).mods & crate::ast::mods::CASE != 0 {
                return self.apply_copy(recv, recv_ty, c, targs, lists, span, expected);
            }
        }
        let mut lists = Some(lists);
        if let Some(r) = self.try_extensions_or_conversion(recv, resolved_ty, name, targs, &mut lists, span, expected, converted) {
            return r;
        }
        // Where an opaque type's definition is visible its underlying type's members apply too.
        // An alias kept by name is expanded one step, so that what it stands for gets its turn.
        let underlying = match self.types.get(recv_ty) {
            Type::Alias(..) => self.deref_alias(recv_ty),
            _ => self.dealias(recv_ty),
        };
        if underlying != recv_ty {
            // An abstract type member has an implicit scope of its own (its prefix's), which
            // its upper bound does not carry: a conversion is tried on it before the bound.
            let abstract_member = matches!(self.types.get(recv_ty), Type::Member(..) | Type::AppMember(..) | Type::Decl(_));
            if !converted && abstract_member {
                let l = lists.take().unwrap_or_default();
                if let Some(r) = self.apply_through_conversion(recv, resolved_ty, name, targs, &l, span, expected) {
                    return r;
                }
                lists = Some(l);
            }
            self.no_receiver_conversion = converted;
            return self.apply_member(recv, underlying, name, targs, lists.unwrap_or_default(), span, expected);
        }
        if dynamic {
            return self.apply_dynamic_member(recv, name, targs, lists.unwrap_or_default(), span, expected);
        }
        if !converted {
            let l = lists.take().unwrap_or_default();
            if let Some(r) = self.apply_through_conversion(recv, resolved_ty, name, targs, &l, span, expected) {
                return r;
            }
            lists = Some(l);
        }
        // `x.copy$default$n`, the default of the nth parameter of a case class's `copy`, is the
        // field of that parameter.
        if let Some(field) = self.copy_default_field(recv_ty, name) {
            return self.apply_member(recv, recv_ty, field, None, lists.unwrap_or_default(), span, expected);
        }
        // `x.m$default$n` in a library body, the default of the nth parameter of `x.m`, is what
        // a missing argument is: the callee supplies it.
        if let Some(ty) = self.member_default_type(recv_ty, name) {
            return (self.prog.add(TExpr::Unit), ty);
        }
        let msg = format!("value {} is not a member of {}", self.name_str(name), self.show(recv_ty));
        // What a class whose header, or an ancestor's, the parser could not complete lacks is
        // unknown.
        if self.incomplete_ancestry(recv_ty) {
            self.dependent_error(span, msg);
        } else {
            self.error(span, msg);
        }
        if let Some(l) = &lists {
            self.type_args_for_errors(l);
        }
        (self.prog.add(TExpr::Unit), ERROR)
    }

    /// `f(recv).name(args)` for a conversion `f` whose result has such a member, as dotc
    /// inserts it once the receiver's own members and the extension methods have failed.
    pub(super) fn apply_through_conversion(
        &mut self,
        recv: TExprId,
        recv_ty: TypeId,
        name: Name,
        targs: Option<ListRef>,
        lists: &[ArgList],
        span: Span,
        expected: Option<TypeId>,
    ) -> Option<(TExprId, TypeId)> {
        match self.convert_for_member(recv, recv_ty, name, lists, expected, span)? {
            Ok((converted, ty)) => {
                if let Some(r) = self.prim_op_on(converted, ty, name, targs, lists, span, expected) {
                    return Some(r);
                }
                self.no_receiver_conversion = true;
                Some(self.apply_member(converted, ty, name, targs, lists.to_vec(), span, expected))
            }
            Err(()) => {
                self.type_args_for_errors(lists);
                Some((self.prog.add(TExpr::Unit), ERROR))
            }
        }
    }

    /// The case class whose constructor `C(args)` and `C.apply` mean next to the `apply` methods
    /// of the companion object `recv_ty`.
    fn constructor_next_to_apply(&mut self, recv_ty: TypeId) -> Option<ClassId> {
        let c = self.class_of(recv_ty).and_then(|o| self.companion_class(o))?;
        let mods = self.syms.class(c).mods;
        (mods & crate::ast::mods::CASE != 0 && mods & crate::ast::mods::ABSTRACT == 0).then_some(c)
    }

    /// An overloaded member: an alternative that applies wins, and the extension methods of
    /// that name are tried when none does.
    #[inline(never)]
    fn apply_overloaded_member(
        &mut self,
        recv: TExprId,
        recv_ty: TypeId,
        set: SymId,
        name: Name,
        targs: Option<ListRef>,
        lists: Vec<ArgList>,
        span: Span,
        expected: Option<TypeId>,
    ) -> (TExprId, TypeId) {
        let converted = std::mem::take(&mut self.no_receiver_conversion);
        let resolved_ty = self.proxy_declared(recv).unwrap_or(recv_ty);
        let ctor = (name == names::APPLY).then(|| self.constructor_next_to_apply(recv_ty)).flatten();
        // With an extension method of the name in scope, a lone alternative that fits the shape
        // of the call still has to take the arguments, or the extension is meant.
        let strict = !self.lexical_extensions(name).is_empty();
        let failed = match self.try_overloaded(Some(recv), Some(recv_ty), set, ctor, strict, targs, lists, span, expected) {
            Ok(r) => return r,
            Err(failed) => failed,
        };
        if !failed.none_applicable {
            return self.report_unresolved(failed, span);
        }
        let super::overload::Unresolved { msg, lists, none_applicable, dependent } = failed;

        let mut lists = Some(lists);
        if let Some(r) = self.try_extensions_or_conversion(recv, resolved_ty, name, targs, &mut lists, span, expected, converted) {
            return r;
        }
        if !converted {
            let l = lists.take().unwrap_or_default();
            if let Some(r) = self.apply_through_conversion(recv, resolved_ty, name, targs, &l, span, expected) {
                return r;
            }
            lists = Some(l);
        }
        let failed = super::overload::Unresolved { msg, lists: lists.unwrap_or_default(), none_applicable, dependent };
        self.report_unresolved(failed, span)
    }

    /// A member that only a refinement of the receiver's type declares (`T { def m(x: Int): String }`):
    /// on a `Selectable` it is `recv.applyDynamic("m")(args)`, or `recv.selectDynamic("m")` for a
    /// value, with the type the refinement gives it; anything else is scalac's error.
    #[cold]
    fn apply_structural(
        &mut self,
        recv: TExprId,
        recv_ty: TypeId,
        sig: Arc<MethodSig>,
        name: Name,
        targs: Option<ListRef>,
        lists: Vec<ArgList>,
        span: Span,
        expected: Option<TypeId>,
    ) -> (TExprId, TypeId) {
        let failed = |t: &mut Self, lists: &[ArgList]| {
            t.type_args_for_errors(lists);
            (t.prog.add(TExpr::Unit), ERROR)
        };
        let (parent, _) = self.types.refinements_of(recv_ty);
        let selectable = self.b.selectable.filter(|&s| self.base_type(parent, s).is_some());
        if selectable.is_none() {
            let found = self.path_of(recv).unwrap_or(recv_ty);
            let msg = format!("type mismatch: found {}, required Selectable | Dynamic", self.show(found));
            self.error(span, msg);
            return failed(self, &lists);
        }
        if !sig.tparams.is_empty() || targs.is_some() {
            self.error(span, "not supported yet: a structural member with type parameters");
            return failed(self, &lists);
        }
        let key = self.prog.add_str(&self.name_str(name));
        let key = self.prog.add(TExpr::Str(key));
        let mut dynamic_lists = vec![ArgList { args: vec![ArgSrc::Typed(key, self.b.t_string)], using: false, span }];
        let mut lists = lists.into_iter();
        let mut typed: Vec<ArgSrc> = Vec::new();
        for clause in sig.clauses.iter().filter(|cl| !cl.is_using) {
            let Some(list) = lists.next() else {
                let msg = format!("missing argument list for method {}", self.name_str(name));
                self.error(span, msg);
                return (self.prog.add(TExpr::Unit), ERROR);
            };
            let mut out = Vec::new();
            self.type_clause_args(clause, list, &Vec::new(), &mut out);
            typed.extend(out.into_iter().zip(&clause.params).map(|(te, p)| ArgSrc::Typed(te, p.ty)));
        }
        let dynamic = if sig.clauses.is_empty() {
            names::SELECT_DYNAMIC
        } else {
            dynamic_lists.push(ArgList { args: typed, using: false, span });
            names::APPLY_DYNAMIC
        };
        let (te, _) = self.apply_member(recv, recv_ty, dynamic, None, dynamic_lists, span, None);
        let rest: Vec<ArgList> = lists.collect();
        if rest.is_empty() {
            return (te, sig.ret);
        }
        self.apply_callee(Callee::Value(te, sig.ret), None, rest, span, expected)
    }

    /// Whether the static type is `scala.scalajs.js.Dynamic`, a subtype of it, or an intersection
    /// with one: a receiver whose unknown members are JavaScript properties.
    pub fn is_js_dynamic(&mut self, t: TypeId) -> bool {
        let Some(dynamic) = self.b.js_dynamic else { return false };
        let t = self.deref(t);
        match self.types.get(t) {
            Type::Class(c, _) => c == dynamic || self.base_type(t, dynamic).is_some(),
            Type::Inter(a, b) => self.is_js_dynamic(a) || self.is_js_dynamic(b),
            Type::Param(p) => {
                let upper = self.syms.tparam(p).upper;
                upper != ANY && self.is_js_dynamic(upper)
            }
            _ => false,
        }
    }

    /// A member of a `js.Dynamic` receiver that nothing declares: `e.n` reads the property, `e.n(args)`
    /// calls it, `e(args)` calls the receiver, and `selectDynamic`, `updateDynamic` and
    /// `applyDynamic` name the property with a string. Everything is typed `js.Dynamic`.
    fn apply_dynamic_member(
        &mut self,
        recv: TExprId,
        name: Name,
        targs: Option<ListRef>,
        lists: Vec<ArgList>,
        span: Span,
        expected: Option<TypeId>,
    ) -> (TExprId, TypeId) {
        let dynamic = self.types.class(self.b.js_dynamic.unwrap(), &[]);
        if targs.is_some() {
            self.error(span, "a member of js.Dynamic takes no type arguments");
        }
        let mut lists = lists;
        let (te, ty) = match name {
            names::SELECT_DYNAMIC | names::UPDATE_DYNAMIC | names::APPLY_DYNAMIC => {
                let what = self.name_str(name);
                let Some(key) = self.dynamic_args(&mut lists, &what, span) else { return (recv, ERROR) };
                if key.len() != 1 {
                    self.error(span, format!("{} takes the property name as its one argument", what));
                    return (recv, ERROR);
                }
                let mut all = vec![recv, key[0]];
                let (template, ty) = match name {
                    names::SELECT_DYNAMIC => ("($0)[$1]", dynamic),
                    names::UPDATE_DYNAMIC => {
                        let Some(value) = self.dynamic_args(&mut lists, &what, span) else { return (recv, ERROR) };
                        all.extend(value);
                        ("(($0)[$1] = $2)", self.b.t_unit)
                    }
                    _ => {
                        let Some(args) = self.dynamic_args(&mut lists, &what, span) else { return (recv, ERROR) };
                        let l = self.prog.list(&args);
                        all.push(self.prog.add(TExpr::ArrayLit(l)));
                        ("($0)[$1](...$2)", dynamic)
                    }
                };
                let s = self.prog.add_str(template);
                let l = self.prog.list(&all);
                let te = self.prog.add(TExpr::Js(s, l));
                if self.capturing() {
                    self.capture_form(te, Form::Op(name));
                }
                (te, ty)
            }
            names::APPLY => {
                let Some(args) = self.dynamic_args(&mut lists, "a call of a js.Dynamic value", span) else {
                    return (recv, ERROR);
                };
                let l = self.prog.list(&args);
                let te = self.prog.add(TExpr::CallClosure(recv, l));
                if self.capturing() {
                    self.capture_form(te, Form::Op(name));
                }
                (te, dynamic)
            }
            _ => {
                let sel = self.prog.add(TExpr::JsSelect(recv, name));
                if self.capturing() {
                    self.capture_form(sel, Form::Dynamic(names::SELECT_DYNAMIC));
                }
                if lists.first().map_or(true, |l| l.using) {
                    (sel, dynamic)
                } else {
                    let args = self.dynamic_args(&mut lists, "a call", span).unwrap_or_default();
                    let l = self.prog.list(&args);
                    let te = self.prog.add(TExpr::CallClosure(sel, l));
                    if self.capturing() {
                        self.capture_form(te, Form::Dynamic(names::APPLY_DYNAMIC));
                    }
                    (te, dynamic)
                }
            }
        };
        if lists.is_empty() {
            return (te, ty);
        }
        self.apply_callee(Callee::Value(te, ty), None, lists, span, expected)
    }

    /// The first argument list of a dynamic call, typed against `Any`.
    fn dynamic_args(&mut self, lists: &mut Vec<ArgList>, what: &str, span: Span) -> Option<Vec<TExprId>> {
        if lists.is_empty() || lists[0].using {
            self.error(span, format!("{} needs an argument list", what));
            return None;
        }
        let list = lists.remove(0);
        let mut out = Vec::with_capacity(list.args.len());
        for a in list.args {
            out.push(self.type_arg(a, ANY, list.span));
        }
        Some(out)
    }

    /// `js.Dynamic.literal(a = 1, b = 2)`: the named arguments are the properties of an object
    /// literal, where the tuple form goes through the library.
    fn is_dynamic_literal_with_names(&self, sym: SymId, lists: &[ArgList]) -> bool {
        let info = self.syms.sym(sym);
        let Some(dynamic) = self.b.js_dynamic else { return false };
        let companion = self.syms.class(dynamic).companion.map(Owner::Class);
        // `literal` is a method of `Dynamic`'s companion, or an object in it whose `apply` is
        // called (Scala.js's shape, applied dynamically by a jar's bodies).
        let literal_method = info.name == names::LITERAL && Some(info.owner) == companion;
        let literal_object = info.name == names::APPLY
            && matches!(info.owner, Owner::Class(o) if self.syms.class(o).kind == ClassKind::Object && self.syms.class(o).name == names::LITERAL && Some(self.syms.class(o).owner) == companion);
        if !literal_method && !literal_object {
            return false;
        }
        let ast = self.cur_ast();
        lists.first().map_or(false, |l| {
            !l.using && l.args.iter().any(|a| matches!(a, ArgSrc::Ast(e) if matches!(ast.expr(*e), Expr::NamedArg(..))))
        })
    }

    /// Typed as the std declares `literal`: Scala.js's `js.Object with js.Dynamic`.
    fn dynamic_literal(&mut self, sym: SymId, lists: Vec<ArgList>, span: Span, expected: Option<TypeId>) -> (TExprId, TypeId) {
        let dynamic = self.sig_of(sym).ret;
        let mut lists = lists;
        let list = lists.remove(0);
        let ast = self.cur_ast();
        let mut items = Vec::with_capacity(list.args.len() * 2);
        for a in list.args {
            let (ArgSrc::Ast(e), Expr::NamedArg(n, v)) = (a, a.ast().map(|e| ast.expr(e)).unwrap_or(Expr::Error)) else {
                self.error(list.span, "js.Dynamic.literal takes either named arguments or pairs, not both");
                if let Some(e) = a.ast() {
                    self.type_expr(e, None);
                }
                continue;
            };
            let _ = e;
            let key = self.name_str(n);
            let key = self.prog.add_str(&key);
            items.push(self.prog.add(TExpr::Str(key)));
            items.push(self.type_arg(ArgSrc::Ast(v), ANY, list.span));
        }
        let l = self.prog.list(&items);
        let te = self.prog.add(TExpr::ObjLit(l));
        if self.capturing() {
            self.capture_form(te, Form::DynamicLiteral(sym));
        }
        if lists.is_empty() {
            return (te, dynamic);
        }
        self.apply_callee(Callee::Value(te, dynamic), None, lists, span, expected)
    }

    /// `x.toString()` and `s.length()` on a parameterless member: Java defines them with the
    /// parentheses, so scalac takes both forms.
    fn drops_java_parens(&mut self, sym: SymId, lists: &[ArgList]) -> bool {
        let empty_parens = lists.len() == 1 && lists[0].args.is_empty() && !lists[0].using;
        if !empty_parens || self.syms.sym(sym).kind != SymKind::Def || !self.has_java_parens(sym) {
            return false;
        }
        let ext_clauses = self.syms.sym(sym).ext_clauses as usize;
        let sig = self.sig_of(sym);
        !sig.clauses.iter().skip(ext_clauses).any(|c| !c.is_using)
    }

    pub(super) fn has_java_parens(&self, sym: SymId) -> bool {
        let info = self.syms.sym(sym);
        info.java_defined || matches!(info.name, names::TO_STRING | names::HASH_CODE | names::CLONE) || self.in_java_package(sym) || self.string_member_of_std(sym) || self.scala2_std_member(sym) || self.predef_println(sym)
    }

    /// The std's `println()`, which stands for scala-library's `Predef.println()`: a Scala 2
    /// method, which scalac lets a Scala 3 caller call as `println`.
    fn predef_println(&self, sym: SymId) -> bool {
        let info = self.syms.sym(sym);
        info.name == names::PRINTLN && info.owner == Owner::Package(self.b.scala_pkg) && self.source(info.file).is_std
    }

    /// A member of the std's ports of Scala 2 artifacts (sbt's test interface, Scala.js's test
    /// bridge and JUnit classes), whose `def f()` a Scala 3 caller may call as `f`, as scalac lets
    /// it call the artifacts' own.
    fn scala2_std_member(&self, sym: SymId) -> bool {
        let source = self.source(self.syms.sym(sym).file);
        source.is_std && crate::TESTING_STD_FILES.contains(&source.path.as_str())
    }

    /// An extension of the std on `String` that stands for a member of `java.lang.String`
    /// (`getBytes()`), which takes the Java parentheses rule as the JDK's members do.
    fn string_member_of_std(&self, sym: SymId) -> bool {
        let info = self.syms.sym(sym);
        if !info.is_extension || !self.source(info.file).is_std {
            return false;
        }
        let receiver = info.sig.as_ref().and_then(|sig| sig.clauses.first()).and_then(|c| c.params.first()).map(|p| p.ty);
        receiver == Some(self.b.t_string)
    }

    /// Whether `sym` is a member of a class of a `java.*` package: a JDK class read from its
    /// class file, the std's platform layer or a source stand-in for one, whose members all
    /// take the Java parentheses rule, as scalac takes the JDK's own members.
    fn in_java_package(&self, sym: SymId) -> bool {
        let mut owner = self.syms.sym(sym).owner;
        while let Owner::Class(c) = owner {
            owner = self.syms.class(c).owner;
        }
        let Owner::Package(mut p) = owner else { return false };
        loop {
            let info = self.syms.pkg(p);
            match info.parent {
                Some(parent) if parent == ROOT_PKG => return info.name == names::JAVA,
                Some(parent) => p = parent,
                None => return false,
            }
        }
    }

    /// scalac's E100 for `x.f` where `f` is declared as `def f()`.
    fn report_missing_parens(&mut self, sym: SymId, span: Span) {
        let msg = format!("method {} must be called with () argument", self.method_description(sym));
        self.error(span, msg);
    }

    /// A member of a class of a Scala 2 library (`Predef.println()`, `Iterator.next()` of
    /// scala-library), whose `def f()` scalac's `matchNullaryLoosely` lets a bare `f` call, with
    /// the E100 warning, which teq leaves out.
    fn scala2_library_member(&self, sym: SymId) -> bool {
        matches!(self.syms.sym(sym).owner, Owner::Class(c) if self.is_scala2_class(c))
    }

    /// Whether `sym` is a right-associative extension method, the one kind of method whose
    /// lists a call as a method takes in another order than its signature's.
    #[inline]
    fn is_right_assoc_extension(&self, sym: SymId) -> bool {
        let info = self.syms.sym(sym);
        info.is_extension && self.interner.get(info.name).ends_with(':')
    }

    /// A right-associative extension method called as a method (`Syntax.+:(s)(n)`, an imported
    /// `+:(s)(n)`): its type arguments checked against the first clause, its lists in the
    /// signature's order, and its receiver evaluated before the arguments bound ahead.
    #[cold]
    #[inline(never)]
    #[allow(clippy::too_many_arguments)]
    fn apply_right_assoc_method(
        &mut self,
        recv: Option<TExprId>,
        sym: SymId,
        owner_subst: Subst,
        prefix: Option<TypeId>,
        targs: Option<ListRef>,
        lists: Vec<ArgList>,
        span: Span,
        expected: Option<TypeId>,
    ) -> (TExprId, TypeId) {
        if self.right_assoc_targs_excess(sym, targs, "", span) {
            self.type_args_for_errors(&lists);
            return (self.prog.add(TExpr::Unit), ERROR);
        }
        let mark = self.hoisted.len();
        let (lists, hoisted) = self.right_assoc_written_lists(sym, lists);
        let mut recv = recv;
        if let (Some(r), true) = (recv, hoisted) {
            let mut r = [r];
            self.hoist_before(mark, &mut r, span);
            recv = Some(r[0]);
        }
        let call = MethodCall { recv, sym, owner_subst, ext_recv: None, prefix };
        let (te, ty) = self.apply_method(call, None, targs, lists, span, expected, false).unwrap();
        match hoisted {
            true => (self.wrap_hoisted(mark, te), ty),
            false => (te, ty),
        }
    }

    /// The argument lists of a right-associative extension method called as a method
    /// (`Syntax.+:(s)(n)`), written in the order its declaration's clauses take (dotty's
    /// `Desugar.extMethod`, `rightAssocParams`: the method's first clause before the receiver's),
    /// in the order of its signature, which keeps the receiver's first. Lists that stop before
    /// the receiver's clause are left as they are. The arguments are still evaluated as written:
    /// those of a list that goes after one written later are bound to temporaries first
    /// (`ArgSrc::Hoisted`, which the caller's `wrap_hoisted` places), but a by-name parameter's;
    /// whether any was, after which the receiver the method is selected on is evaluated before
    /// them (dotty's `Applications.liftFun` lifts the function's prefix first).
    fn right_assoc_written_lists(&mut self, sym: SymId, lists: Vec<ArgList>) -> (Vec<ArgList>, bool) {
        if !self.is_right_assoc_extension(sym) {
            return (lists, false);
        }
        let ext_clauses = self.syms.sym(sym).ext_clauses as usize;
        let sig = self.sig_of(sym).clone();
        let Some(order) = sig.right_assoc_order(ext_clauses) else { return (lists, false) };
        // Each written list to its clause, a using clause taking a list written with `using`
        // and passed over otherwise, as the application matches them.
        let mut assigned: Vec<(usize, usize)> = Vec::new();
        let mut next = 0;
        for &c in &order {
            let Some(l) = lists.get(next) else { break };
            if sig.clauses[c].is_using {
                if l.using {
                    assigned.push((c, next));
                    next += 1;
                }
                continue;
            }
            if l.using {
                return (lists, false);
            }
            assigned.push((c, next));
            next += 1;
        }
        let receiver = (0..ext_clauses).find(|&i| !sig.clauses[i].is_using);
        if !assigned.iter().any(|&(c, _)| Some(c) == receiver) {
            return (lists, false);
        }
        let mut hoisted = false;
        assigned.sort_by_key(|&(c, _)| c);
        let mut slots: Vec<Option<ArgList>> = lists.into_iter().map(Some).collect();
        let mut out: Vec<ArgList> = Vec::with_capacity(slots.len());
        for (k, &(c, w)) in assigned.iter().enumerate() {
            let Some(mut list) = slots[w].take() else { continue };
            if assigned[..k].iter().any(|&(_, earlier)| earlier > w) {
                for (j, a) in list.args.iter_mut().enumerate() {
                    let ArgSrc::Ast(e) = *a else { continue };
                    // A named argument's parameter is the one of its name.
                    let param = match self.cur_ast().expr(e) {
                        Expr::NamedArg(n, _) => sig.clauses[c].params.iter().find(|p| p.name == n),
                        _ => sig.clauses[c].params.get(j),
                    };
                    if !param.is_some_and(|p| p.by_name) {
                        *a = ArgSrc::Hoisted(e);
                        hoisted = true;
                    }
                }
            }
            out.push(list);
        }
        out.extend(slots.into_iter().flatten());
        (out, hoisted)
    }

    /// Explicit type arguments of a right-associative extension called as a method, whose
    /// extension and method both take type parameters: dotty declares them as two clauses
    /// (`Desugar.extMethod`, `rightAssocParams`: `[A][B](s)(n)`), and the arguments written fill
    /// the first, the extension's (E023 for more). Reported, with whether it was.
    fn right_assoc_targs_excess(&mut self, sym: SymId, targs: Option<ListRef>, prefix: &str, span: Span) -> bool {
        let Some(l) = targs else { return false };
        if !self.is_right_assoc_extension(sym) {
            return false;
        }
        let info = self.syms.sym(sym);
        let (ext_tparams, ext_clauses, name) = (info.ext_tparams as usize, info.ext_clauses as usize, info.name);
        let sig = self.sig_of(sym).clone();
        let written = self.cur_ast().ty_list(l).to_vec();
        if sig.right_assoc_order(ext_clauses).is_none() || ext_tparams == 0 || sig.tparams.len() <= ext_tparams || written.len() <= ext_tparams {
            return false;
        }
        let expected: Vec<String> = sig.tparams[..ext_tparams].iter().map(|&p| self.name_str(self.syms.tparam(p).name)).collect();
        let actual: Vec<String> = written.iter().map(|&t| {
            let ty = self.resolve_type(t);
            self.show(ty)
        }).collect();
        let msg = format!(
            "Too many type arguments for {}{}[{}]\nexpected: [{}]\nactual:   [{}]",
            prefix,
            self.name_str(name),
            expected.join(", "),
            expected.join(", "),
            actual.join(", ")
        );
        self.error(span, msg);
        true
    }

    /// The `apply` member of `ty` when it is a value or a method without parameter lists, every
    /// alternative of it.
    fn apply_taking_none(&mut self, ty: TypeId) -> Option<SymId> {
        // An `apply` the class itself declares with a parameter list (a companion's) settles it
        // without the lookup through the parents.
        if let Type::Class(c, _) = self.types.get(ty) {
            if let Some(&s) = self.syms.class(c).members.get(&names::APPLY) {
                if self.syms.sym(s).kind == SymKind::Def && self.syms.sym(s).sig.as_ref().is_some_and(|sig| !sig.clauses.is_empty()) {
                    return None;
                }
            }
        }
        let (s, _) = self.find_member(ty, names::APPLY)?;
        let alts: Vec<SymId> = self.syms.alternatives(s).map_or_else(|| vec![s], |a| a.to_vec());
        let none = alts.iter().all(|&a| match self.syms.sym(a).kind {
            SymKind::Def => self.sig_of(a).clauses.is_empty(),
            SymKind::Overloaded(_) => false,
            _ => true,
        });
        none.then(|| alts[0])
    }

    /// `x.f()` where `f` is declared without a parameter list.
    fn report_takes_no_parameters(&mut self, sym: SymId, span: Span) {
        let msg = format!("method {} does not take parameters", self.method_description(sym));
        self.error(span, msg);
    }

    pub(super) fn method_description(&self, sym: SymId) -> String {
        let info = self.syms.sym(sym);
        match info.owner {
            Owner::Class(c) => {
                let kind = match self.syms.class(c).kind {
                    ClassKind::Trait => "trait",
                    ClassKind::Object => "object",
                    _ => "class",
                };
                format!("{} in {} {}", self.name_str(info.name), kind, self.name_str(self.syms.class(c).name))
            }
            _ => self.name_str(info.name),
        }
    }

    /// `F.map(fa)(f)` for an extension method `map` of the type class `Functor[F]`: the first
    /// argument is the receiver of the extension, as in Scala.
    fn apply_extension_of_instance(
        &mut self,
        recv: TExprId,
        recv_ty: TypeId,
        name: Name,
        targs: Option<ListRef>,
        lists: &mut Vec<ArgList>,
        span: Span,
        expected: Option<TypeId>,
    ) -> Option<(TExprId, TypeId)> {
        let c = self.class_of(recv_ty)?;
        self.complete_class(c);
        let owner = self.syms.class(c).base_types.iter().map(|&(b, _)| b).find(|&b| {
            self.syms.class(b).extensions.iter().any(|&s| self.syms.sym(s).name == name)
        })?;
        let mut exts: Vec<SymId> = self.syms.class(owner).extensions.iter().copied().filter(|&s| self.syms.sym(s).name == name).collect();
        // Explicit type arguments of a direct call cover the extension's parameters and its
        // own: among same-named extensions, those with that many (`OrElse.unify[I, K, T](x)`
        // next to `unify[A, B]`).
        if let Some(l) = targs {
            let n = self.cur_ast().ty_list(l).len();
            let fitting: Vec<SymId> = exts.iter().copied().filter(|&e| self.sig_of(e).tparams.len() == n).collect();
            if !fitting.is_empty() {
                exts = fitting;
            }
        }
        // A right-associative one's lists are written in its declaration's order, the method's
        // first clause before the receiver's (`right_assoc_written_lists`).
        let right_assoc = match exts[..] {
            [ext] if self.is_right_assoc_extension(ext) => Some(ext),
            _ => None,
        };
        let reordered = right_assoc.map(|ext| self.right_assoc_written_lists(ext, lists.clone()));
        // A library body's argument list of one given (`OrElse.unify[..](inst)` with a using
        // parameter `inst`) reads as a using clause, which the receiver clause is not.
        let in_body = self.is_body_file(self.env.file);
        let ordered: &[ArgList] = match &reordered {
            Some((o, _)) => o,
            None => lists,
        };
        let first = ordered.first().filter(|l| (!l.using || in_body) && l.args.len() == 1)?;
        let ArgSrc::Ast(arg) = first.args[0] else { return None };
        if matches!(self.cur_ast().expr(arg), Expr::NamedArg(..)) {
            return None;
        }
        let mut recv = recv;
        if let (Some(ext), Some((ordered, hoisted))) = (right_assoc, reordered) {
            let prefix = match self.class_of(recv_ty) {
                Some(c) => format!("{}.", self.name_str(self.syms.class(c).name)),
                None => String::new(),
            };
            if self.right_assoc_targs_excess(ext, targs, &prefix, span) {
                self.type_args_for_errors(lists);
                return Some((self.prog.add(TExpr::Unit), ERROR));
            }
            // The receiver the extension is selected on is evaluated before the arguments bound
            // ahead of the call (the caller's `wrap_hoisted` places both).
            if hoisted {
                let mut r = [recv];
                let at = self.hoisted.len();
                self.hoist_before(at, &mut r, span);
                recv = r[0];
            }
            *lists = ordered;
        }
        let (te, ty) = self.type_expr(arg, None);
        let ty = self.solve_in(ty);
        let owner_ty = self.base_type(recv_ty, owner).unwrap_or(recv_ty);
        let owner_subst = self.owner_subst(owner_ty);
        let mut rest = lists.split_off(1);
        // Same-named extensions are overloads whose first argument list is the receiver: the
        // receiver decides, and the lists after it only between receivers it leaves equally
        // specific, over every alternative the receiver applies to (scalac's
        // `resolveOverloaded`); a receiver more specific than the others wins whatever follows.
        if exts.len() > 1 && targs.is_none() {
            let calls: Vec<MethodCall> = exts
                .iter()
                .map(|&sym| MethodCall { recv: Some(recv), sym, owner_subst: owner_subst.clone(), ext_recv: Some((te, ty)), prefix: None })
                .collect();
            if let Some(i) = self.pick_by_arguments(&calls, te, ty, &mut rest, expected) {
                self.direct_ext_targs = false;
                let call = calls.into_iter().nth(i).unwrap();
                return self.apply_method(call, None, None, rest, span, expected, false);
            }
        }
        // Otherwise (`appliedTo(targ)` and `appliedTo(targs)` with explicit type arguments, or
        // no alternative the receiver decides): the ones whose own clause takes the arguments,
        // and among those the first that applies.
        let fitting: Vec<SymId> = if exts.len() > 1 { exts.iter().copied().filter(|&e| self.extension_accepts(e, &rest, false)).collect() } else { exts.clone() };
        let candidates = if fitting.is_empty() { exts } else { fitting };
        let last = candidates.len() - 1;
        // Each candidate is an attempt of its own over the same arguments, typed once.
        if last > 0 {
            self.arg_cache_open(&rest);
        }
        for (i, &ext) in candidates.iter().enumerate() {
            let call = MethodCall { recv: Some(recv), sym: ext, owner_subst: owner_subst.clone(), ext_recv: Some((te, ty)), prefix: None };
            // The explicit type arguments of a direct call cover the extension's own parameters
            // too, for each candidate tried.
            self.direct_ext_targs = targs.is_some();
            if i == last {
                let r = self.apply_method(call, None, targs, rest, span, expected, false);
                if last > 0 {
                    self.arg_cache_close();
                }
                return r;
            }
            let mark = self.attempt();
            self.arg_cache_attempt(mark);
            let result = self.apply_method(call, None, targs, rest.clone(), span, expected, false);
            if !self.attempt_reported(&mark) {
                self.close(mark);
                self.arg_cache_close();
                return result;
            }
            self.retract(mark);
        }
        None
    }

    /// Arity check that lets same-named extensions differ in their parameter count.
    fn extension_accepts(&mut self, sym: SymId, lists: &[ArgList], tupled: bool) -> bool {
        let ext_clauses = self.syms.sym(sym).ext_clauses as usize;
        let sig = self.sig_of(sym);
        let own_clause = sig.clauses.iter().skip(ext_clauses).find(|c| !c.is_using);
        let list = lists.iter().find(|l| !l.using);
        match (own_clause, list) {
            (Some(clause), Some(list)) => clause_accepts(clause, list.args.len(), tupled),
            _ => true,
        }
    }

    pub(super) fn member_accepts(&mut self, sym: SymId, lists: &[ArgList], tupled: bool, expected: Option<TypeId>) -> bool {
        if self.syms.sym(sym).kind != SymKind::Def {
            return true;
        }
        let sig = self.sig_of(sym);
        let Some(clause) = sig.clauses.iter().find(|c| !c.is_using) else { return true };
        let Some(list) = lists.iter().find(|l| !l.using) else {
            // `xs.mkString`: a parameterless extension stands in for the overload Scala would
            // pick; without one, or with a function expected (`o.flatMap(map.get)`), the member
            // becomes a function value.
            let required = clause.params.iter().filter(|p| !p.has_default && !p.repeated).count();
            let arity = clause.params.len();
            if required == 0 || expected.map_or(false, |t| self.expected_function(t, arity).is_some()) {
                return true;
            }
            let name = self.syms.sym(sym).name;
            return !self.lexical_extensions(name).into_iter().any(|e| self.is_parameterless_extension(e));
        };
        clause_accepts(clause, list.args.len(), tupled)
    }

    /// Whether auto-tupling applies and the member's single parameter can hold such a tuple.
    pub(super) fn member_takes_tuple(&mut self, sym: SymId, owner_ty: TypeId, lists: &[ArgList]) -> bool {
        if !self.member_accepts(sym, lists, true, None) {
            return false;
        }
        let Some(list) = lists.iter().find(|l| !l.using) else { return false };
        if self.positional_args(&list.args).is_none() {
            return false;
        }
        let sig = self.sig_arc(sym);
        let Some(param) = sig.clauses.iter().find(|c| !c.is_using).and_then(|c| c.params.first()) else {
            return false;
        };
        let mut subst = self.owner_subst(owner_ty);
        for &tp in &sig.tparams {
            let v = self.fresh_var();
            subst.push((tp, v));
        }
        let pty = self.types.subst(param.ty, &subst);
        self.holds_tuple(pty, list.args.len())
    }

    /// Tuples are covariant, so a type that rejects the smallest tuple type of a size takes none.
    pub(super) fn holds_tuple(&mut self, ty: TypeId, size: usize) -> bool {
        let smallest = self.tuple_type(&vec![NOTHING; size]);
        let mark = self.snapshot();
        let holds = self.is_sub(smallest, ty);
        self.rollback(mark);
        holds
    }

    fn is_parameterless_extension(&mut self, sym: SymId) -> bool {
        let ext_clauses = self.syms.sym(sym).ext_clauses as usize;
        let sig = self.sig_of(sym);
        !sig.clauses.iter().skip(ext_clauses).any(|c| !c.is_using)
    }

    /// `c.copy(args)` is a `new` of the case class whose missing arguments are the fields of `c`.
    /// As scalac's `copy[T](...)`, it takes the type parameters of the class afresh, so that an
    /// argument may change a type argument, and a kept field has to conform to its new type.
    fn apply_copy(
        &mut self,
        recv: TExprId,
        recv_ty: TypeId,
        c: ClassId,
        targs: Option<ListRef>,
        lists: Vec<ArgList>,
        span: Span,
        expected: Option<TypeId>,
    ) -> (TExprId, TypeId) {
        // The `copy` of a case class whose header the parser could not complete takes any
        // arguments.
        if self.syms.class(c).mods & crate::ast::mods::INCOMPLETE != 0 {
            self.type_args_for_errors(&lists);
            return (recv, recv_ty);
        }
        self.check_ctor_access(c, span);
        let tmp = self.fresh_local("c", recv_ty, span);
        let recv_subst = self.owner_subst(recv_ty);
        let Some(first) = self.syms.class(c).ctor.first().cloned() else {
            return (recv, recv_ty);
        };
        let outer_base = std::mem::replace(&mut self.app_base, self.tvars.len() as u32);
        let first_var = self.tvars.len();
        let widen_mark = self.pending_widenings.len();
        let explicit: Vec<TypeId> = match targs {
            Some(l) => {
                let ids = self.cur_ast().ty_list(l).to_vec();
                ids.iter().map(|&t| self.resolve_type_ctor(t)).collect()
            }
            None => Vec::new(),
        };
        self.settle_class(c);
        let tparams = self.syms.class(c).tparams.clone();
        if explicit.len() > tparams.len() {
            self.error(span, "too many type arguments");
        }
        let mut subst: Subst = Vec::with_capacity(tparams.len());
        for (i, &tp) in tparams.iter().enumerate() {
            let t = match explicit.get(i) {
                Some(&t) => t,
                None => {
                    let v = self.fresh_var();
                    self.tvars.last_mut().unwrap().name = Some(self.syms.tparam(tp).name);
                    v
                }
            };
            subst.push((tp, t));
        }
        let self_ty = self.syms.class(c).base_types[0].1;
        let ret_ty = self.types.subst(self_ty, &subst);
        if let Some(exp) = self.concrete_expected(expected) {
            if exp != self.b.t_unit && exp != ANY {
                let mark = self.snapshot();
                if !self.is_sub(ret_ty, exp) {
                    self.rollback(mark);
                }
            }
        }
        let list = lists.into_iter().next().unwrap_or(ArgList { args: Vec::new(), using: false, span });
        let given = self.filled_params(&first, &list);
        for (i, p) in first.params.iter().enumerate() {
            if given[i] {
                continue;
            }
            let kept = self.types.subst(p.ty, &recv_subst);
            let pty = self.types.subst(p.ty, &subst);
            if !self.is_sub(kept, pty) {
                let msg = format!("type mismatch: found {}, required {}", self.show(kept), self.show(pty));
                self.error_unless_unknown(span, msg, &[kept, pty]);
            }
        }
        let mut clause = first.clone();
        for p in &mut clause.params {
            p.has_default = true;
        }
        let mut args = Vec::new();
        let mark = self.hoisted.len();
        self.type_clause_args(&clause, list, &subst, &mut args);
        for (i, p) in first.params.iter().enumerate() {
            if !given[i] {
                let t = self.prog.add(TExpr::Local(tmp));
                args[i] = self.prog.add(TExpr::Field(t, p.sym));
            }
        }
        // The copy keeps the evidence and the other parameter lists of the original.
        let rest: Vec<SymId> =
            self.syms.class(c).ctor.iter().skip(1).flat_map(|cl| cl.params.iter().map(|p| p.sym)).collect();
        for sym in rest {
            let t = self.prog.add(TExpr::Local(tmp));
            args.push(self.prog.add(TExpr::Field(t, sym)));
        }
        self.solve_application_vars(first_var, widen_mark, ret_ty, &[], &[], span);
        self.app_base = outer_base;
        let ret_ty = self.zonk(ret_ty);
        let l = self.prog.list(&args);
        let new = self.new_instance(c, l, span);
        let mut stmts = vec![TStmt::Val(tmp, recv)];
        stmts.extend(self.hoisted.drain(mark..));
        let stmts = self.prog.stmts.push_slice(&stmts);
        let te = self.prog.add(TExpr::Block(stmts, new));
        if self.capturing() {
            let targs: Vec<TypeId> = subst.iter().map(|&(_, t)| t).collect();
            self.capture_targs(new, &targs);
            self.capture_form(te, Form::CaseCopy);
        }
        (te, ret_ty)
    }

    /// The parameters of `clause` that the arguments of `list` fill, by name or by position.
    fn filled_params(&self, clause: &ClauseSig, list: &ArgList) -> Vec<bool> {
        let ast = self.cur_ast();
        let n = clause.params.len();
        let mut given = vec![false; n];
        let mut pos = 0usize;
        for a in &list.args {
            let named = match a {
                ArgSrc::Named(name, ..) => Some(*name),
                _ => a.ast().and_then(|e| match ast.expr(e) {
                    Expr::NamedArg(name, _) => Some(name),
                    _ => None,
                }),
            };
            match named {
                Some(name) => {
                    if let Some(i) = clause.params.iter().position(|p| p.name == name) {
                        given[i] = true;
                        pos = pos.max(i + 1);
                    }
                }
                None if pos < n => {
                    given[pos] = true;
                    if !clause.params[pos].repeated {
                        pos += 1;
                    }
                }
                None => {}
            }
        }
        given
    }

    fn member_default_type(&mut self, recv_ty: TypeId, name: Name) -> Option<TypeId> {
        let text = self.name_str(name);
        let (member, n) = text.rsplit_once("$default$")?;
        let n: usize = n.parse().ok()?;
        if member == "<init>" || member == "$lessinit$greater" {
            return self.ctor_default_type(recv_ty, n);
        }
        let member = self.interner.intern(member);
        let Some((sym, owner_ty)) = self.find_member(recv_ty, member) else {
            return self.extension_default_type(member, n);
        };
        let alts: Vec<SymId> = match self.syms.alternatives(sym) {
            Some(set) => set.to_vec(),
            None => vec![sym],
        };
        let subst = self.owner_subst(owner_ty);
        for alt in alts {
            let sig = self.sig_of(alt);
            let p = sig.clauses.iter().flat_map(|c| c.params.iter()).nth(n.checked_sub(1)?);
            if let Some(ty) = p.filter(|p| p.has_default).map(|p| p.ty) {
                return Some(self.types.subst(ty, &subst));
            }
        }
        None
    }

    /// The default of the nth parameter of an extension method `name` in scope, where a
    /// library body asks for `x.m$default$n` of a method the std has as an extension
    /// (`indexWhere` on a `String`).
    fn extension_default_type(&mut self, name: Name, n: usize) -> Option<TypeId> {
        for sym in self.lexical_extensions(name) {
            let ext_clauses = self.syms.sym(sym).ext_clauses as usize;
            let sig = self.sig_of(sym);
            let p = sig.clauses.iter().skip(ext_clauses).flat_map(|c| c.params.iter()).nth(n.checked_sub(1)?);
            if let Some(p) = p.filter(|p| p.has_default) {
                return Some(p.ty);
            }
        }
        None
    }

    /// `C.<init>$default$n` on the companion `C`: the default of the nth constructor parameter
    /// of the class, which the constructor supplies for a missing argument.
    fn ctor_default_type(&mut self, recv_ty: TypeId, n: usize) -> Option<TypeId> {
        let o = self.class_of(recv_ty)?;
        let c = self.companion_class(o).unwrap_or(o);
        self.ctor_param_default(c, n)
    }

    pub(in crate::typer) fn ctor_param_default(&mut self, c: ClassId, n: usize) -> Option<TypeId> {
        self.complete_class(c);
        let p = self.syms.class(c).info.ctor.iter().flat_map(|cl| cl.params.iter()).nth(n.checked_sub(1)?)?;
        p.has_default.then_some(p.ty)
    }

    /// `n` of a `<init>$default$n` name.
    pub(in crate::typer) fn ctor_default_index(&self, name: Name) -> Option<usize> {
        let text = self.name_ref(name);
        let (member, n) = text.rsplit_once("$default$")?;
        (member == "<init>" || member == "$lessinit$greater").then(|| n.parse().ok()).flatten()
    }

    fn copy_default_field(&mut self, recv_ty: TypeId, name: Name) -> Option<Name> {
        let text = self.name_str(name);
        let n: usize = text.strip_prefix("copy$default$")?.parse().ok()?;
        let c = self.class_of(recv_ty)?;
        if self.is_tuple_class(c) {
            return Some(self.interner.intern(&format!("_{}", n)));
        }
        if self.syms.class(c).mods & crate::ast::mods::CASE == 0 {
            return None;
        }
        let params: Vec<Name> = self.syms.class(c).ctor.iter().flat_map(|cl| cl.params.iter().map(|p| p.name)).collect();
        params.get(n.checked_sub(1)?).copied()
    }

    /// Std extensions on builtin types play the role that members have in Scala.
    fn is_std_extension(&self, sym: SymId) -> bool {
        self.syms.sym(sym).owner == Owner::Package(self.b.scala_pkg)
    }

    pub(super) fn is_builtin_type(&mut self, t: TypeId) -> bool {
        match self.class_of(t) {
            Some(c) => self.syms.class(c).kind == ClassKind::Builtin,
            None => false,
        }
    }

    /// True if `name` is a declared member of `t`, or a std extension applicable to a builtin `t`.
    pub(super) fn has_member_like(&mut self, t: TypeId, name: Name) -> bool {
        if self.find_member(t, name).is_some() {
            return true;
        }
        if !self.is_builtin_type(t) {
            return false;
        }
        let scala_pkg = self.b.scala_pkg;
        self.demand_std(scala_pkg, name, crate::stdindex::TERM);
        let candidates: Vec<SymId> =
            self.syms.pkg(scala_pkg).entries.get(&name).map(|e| e.extensions.clone()).unwrap_or_default();
        candidates.into_iter().any(|sym| {
            let mark = self.snapshot();
            let call = MethodCall { recv: None, sym, owner_subst: Vec::new(), ext_recv: None, prefix: None };
            let ok = self.extension_applicable(&call, t);
            self.rollback(mark);
            ok
        })
    }

    /// True if `name` is a member of `t`, of the type behind an opaque `t`, or a std extension
    /// standing for a member of a builtin type.
    pub fn has_member_or_std_extension(&mut self, t: TypeId, name: Name) -> bool {
        if self.has_member_like(t, name) {
            return true;
        }
        let underlying = self.dealias(t);
        underlying != t && self.has_member_like(underlying, name)
    }

    /// Applies an extension method to `recv` the way scalac resolves one: a method visible by
    /// name comes first; otherwise one implicit search, where the givens in scope rank by nesting
    /// level and the extensions of the implicit scope (the companions of the classes the type
    /// mentions, of their bases, and the objects those are nested in) sit at the outermost level
    /// next to the givens found there, so that two applicable ones are ambiguous. The implicit
    /// scope is that of `scope_ty`: the receiver's type, except for a right-associative operator,
    /// where it is the type of the right operand.
    pub fn try_extensions(
        &mut self,
        recv: TExprId,
        recv_ty: TypeId,
        scope_ty: TypeId,
        name: Name,
        targs: Option<ListRef>,
        lists: &mut Option<Vec<ArgList>>,
        span: Span,
        expected: Option<TypeId>,
    ) -> Option<(TExprId, TypeId)> {
        let (lexical, imports) = self.lexical_extensions_for(name, recv_ty);
        self.try_extensions_among(lexical, imports, None, recv, recv_ty, scope_ty, name, targs, lists, span, expected)
    }

    /// The extensions called `name` of the nearest scope that has one and of package `scala`,
    /// the std's last on a builtin receiver, and how many imports of that scope brought one.
    fn lexical_extensions_for(&mut self, name: Name, recv_ty: TypeId) -> (Vec<SymId>, u32) {
        let (mut lexical, imports) = self.nearest_extensions_counted(name);
        if self.is_builtin_type(recv_ty) {
            lexical.sort_by_key(|&s| !self.is_std_extension(s));
        }
        (lexical, imports)
    }

    /// `try_extensions` over the lexical candidates `lexical` and, where the caller gathered
    /// them, the extensions beyond the lexical scope.
    #[allow(clippy::too_many_arguments)]
    fn try_extensions_among(
        &mut self,
        lexical: Vec<SymId>,
        imports: u32,
        beyond: Option<ExtensionsBeyond>,
        recv: TExprId,
        recv_ty: TypeId,
        scope_ty: TypeId,
        name: Name,
        targs: Option<ListRef>,
        lists: &mut Option<Vec<ArgList>>,
        span: Span,
        expected: Option<TypeId>,
    ) -> Option<(TExprId, TypeId)> {
        // Imports of one scope that each bring an extension of the name are alternatives tried
        // alone (`import_alternatives`), where two imports brought one. The path of a lexical
        // extension without using clauses and alternatives stays as lean as it was: a lexical
        // extension call is on the hot path of extension-heavy code (`ext_16`).
        let several = imports > 1;
        let lexical_pick = self.pick_extension(lexical, recv, recv_ty, lists, false, expected);
        if let Some(call) = &lexical_pick {
            // An extension without using clauses is selected by its receiver alone, beside
            // whatever the implicit scope holds: dotty's prefix `f(qual)` resolves nothing else.
            if !several && !self.sig_of(call.sym).clauses.iter().any(|c| c.is_using) {
                if self.lexical_selected.is_some() {
                    self.lexical_selected = Some(true);
                }
                return self.apply_extension(lexical_pick.unwrap(), targs, lists, span, expected);
            }
        }
        self.lexical_beyond_pick(several, lexical_pick, beyond, recv, recv_ty, scope_ty, name, targs, lists, span, expected)
    }

    /// `try_extensions_among` past the lexical candidate the receiver picked: the imports'
    /// alternatives, the prefix, the extensions beyond the lexical scope.
    #[allow(clippy::too_many_arguments)]
    #[inline(never)]
    fn lexical_beyond_pick(
        &mut self,
        several: bool,
        lexical_pick: Option<MethodCall>,
        beyond: Option<ExtensionsBeyond>,
        recv: TExprId,
        recv_ty: TypeId,
        scope_ty: TypeId,
        name: Name,
        targs: Option<ListRef>,
        lists: &mut Option<Vec<ArgList>>,
        span: Span,
        expected: Option<TypeId>,
    ) -> Option<(TExprId, TypeId)> {
        let alternatives = if several && lexical_pick.is_some() { self.import_alternatives(name) } else { Vec::new() };
        let lexical = match lexical_pick {
            Some(call) if alternatives.len() > 1 && alternatives.iter().any(|g| g.syms.contains(&call.sym)) => {
                self.lexical_among_imports(alternatives, recv, recv_ty, targs, lists, span, expected)
            }
            Some(call) if !self.sig_of(call.sym).clauses.iter().any(|c| c.is_using) => {
                if self.lexical_selected.is_some() {
                    self.lexical_selected = Some(true);
                }
                return self.apply_extension(call, targs, lists, span, expected);
            }
            Some(call) => Lexical::Found(call),
            None => Lexical::Failed(None),
        };
        let mut beyond = beyond;
        let prof = beyond.as_mut().and_then(|b| b.prof.take()).or_else(|| self.prof(Kind::Extension, span, About::Member(recv_ty, name)));
        let r = self.extensions_beyond_lexical(lexical, beyond, recv, recv_ty, scope_ty, name, targs, lists, span, expected);
        if let Some(p) = prof {
            self.profile.exit(p, if r.is_some() { Outcome::Found } else { Outcome::NotFound });
        }
        r
    }

    /// Whether a body of type `ty`, adapted as a lambda's is, gives the result `ret`.
    fn fits_lambda_result(&mut self, ty: TypeId, ret: TypeId) -> bool {
        let mark = self.snapshot();
        let fits = self.is_sub(ty, ret);
        self.rollback(mark);
        let from = self.deref(ty);
        let from = self.widen_lit(from);
        let to = self.deref(ret);
        fits || self.numeric_widening(from, to)
    }

    /// The selection `q.op(a)` of a right-associative extension whose declaration puts the
    /// method's clause before the receiver's (dotty's `Desugar.rightAssocParams`) is `op(q)(a)`
    /// (`extMethodApply` fills the first clause with the qualifier), as the infix `a op q` is: `a`
    /// its receiver, `q` its method's first argument, evaluated first. Taken where the qualifier
    /// has no member of the name, nor a std extension standing for one (`Array`'s `+:`), and the
    /// program's nearest extensions of the name, or else its implicit scope's, are all such;
    /// where none applies, or its application fails, the qualifier's conversion is tried as for
    /// a failed extension (`inferView`), never the receiver's reading. `None` where the selection
    /// is no such one; otherwise the extension's application, if one applied without errors.
    #[cold]
    #[inline(never)]
    #[allow(clippy::too_many_arguments)]
    fn right_assoc_selection(
        &mut self,
        recv: TExprId,
        recv_ty: TypeId,
        name: Name,
        targs: Option<ListRef>,
        lists: &mut Option<Vec<ArgList>>,
        span: Span,
        expected: Option<TypeId>,
    ) -> Option<Option<(TExprId, TypeId)>> {
        let written = lists.as_ref()?;
        let first = written.first().filter(|l| !l.using && l.args.len() == 1)?;
        let arg = first.args[0];
        if !matches!(arg, ArgSrc::Ast(_) | ArgSrc::Typed(..)) || self.has_member_like(recv_ty, name) {
            return None;
        }
        let candidates = self.selection_swaps(name, recv_ty)?;
        // What the candidates' receivers take, the argument's place: the argument is typed against
        // their parameter type where they share one that is no type parameter's (a `Float`, a
        // function type a lambda's parameters take), as the application types it; one named after
        // their receiver's parameter is that argument (`"x".+:(n = 2)`).
        let mut receiver: Option<(Name, Option<TypeId>)> = None;
        for (i, &c) in candidates.iter().enumerate() {
            let ext_clauses = self.syms.sym(c).ext_clauses as usize;
            let sig = self.sig_arc(c);
            let at = (0..ext_clauses).find(|&k| !sig.clauses[k].is_using)?;
            let param = &sig.clauses[at].params[0];
            let generic_owner = matches!(self.syms.sym(c).owner, Owner::Class(o) if !self.syms.class(o).tparams.is_empty());
            // The extension's own type parameters are fixed by the qualifier first, as `op(q)`
            // applies before the receiver's list (`extension [A](f: A => A) def +:(x: A)`).
            let ty = match () {
                _ if generic_owner || param.by_name => None,
                _ if sig.tparams.is_empty() => Some(param.ty),
                _ => {
                    let mark = self.snapshot();
                    let subst: Subst = sig.tparams.iter().map(|&tp| (tp, self.fresh_var())).collect();
                    let first = self.types.subst(sig.clauses[ext_clauses].params[0].ty, &subst);
                    let fixed = self.is_sub(recv_ty, first);
                    let wanted = self.types.subst(param.ty, &subst);
                    let wanted = self.solve_in(wanted);
                    let ty = (fixed && !self.types.has_vars(wanted)).then_some(wanted);
                    self.rollback(mark);
                    ty
                }
            };
            receiver = match receiver {
                _ if i == 0 => Some((param.name, ty)),
                Some((n, t)) => Some((if n == param.name { n } else { names::EMPTY }, if t == ty { t } else { None })),
                None => None,
            };
        }
        let (receiver_name, expected_arg) = receiver?;
        let arg = match arg {
            ArgSrc::Ast(e) => match self.cur_ast().expr(e) {
                Expr::NamedArg(n, inner) if n == receiver_name && n != names::EMPTY => ArgSrc::Ast(inner),
                Expr::NamedArg(..) => return None,
                _ => arg,
            },
            other => other,
        };
        let written = lists.take().unwrap();
        let mark = self.hoisted.len();
        // An attempt as a whole, as dotty's `tryExtension`, the argument typed inside it: where no
        // extension is selected, or the selected one's prefix `op(q)` does not take the qualifier,
        // it gives way to the conversion, which takes the arguments as written. Once the prefix
        // holds, what fails after it (the receiver's argument, a trailing given) is its error.
        let attempt = self.attempt();
        self.applied_extension = None;
        let tried = self.swapped_application(recv, recv_ty, arg, expected_arg, &written, name, targs, span, expected);
        let selected = self.applied_extension.take();
        if let (Some((te, ty)), Some(sym)) = (tried, selected) {
            if !self.attempt_failed(&attempt) || self.swapped_prefix_fits(sym, recv_ty) {
                // The qualifier is evaluated before the argument where the selected overload takes
                // it by value: bound ahead, the application made again over the binding.
                let ext_clauses = self.syms.sym(sym).ext_clauses as usize;
                let strict = !self.sig_arc(sym).clauses[ext_clauses].params[0].by_name;
                if !strict || self.is_stable(recv) {
                    self.close(attempt);
                    return Some(Some((self.wrap_hoisted(mark, te), ty)));
                }
                self.retract(attempt);
                self.hoisted.truncate(mark);
                let attempt = self.attempt();
                let q = self.hoist(recv, recv_ty, span);
                if let Some((te, ty)) = self.swapped_application(q, recv_ty, arg, expected_arg, &written, name, targs, span, expected) {
                    self.close(attempt);
                    return Some(Some((self.wrap_hoisted(mark, te), ty)));
                }
                self.retract(attempt);
                self.hoisted.truncate(mark);
                *lists = Some(written);
                return Some(None);
            }
        }
        self.retract(attempt);
        self.hoisted.truncate(mark);
        // A candidate whose prefix takes the qualifier is the one dotty selects even where no
        // receiver took the argument: the argument's mismatch with its receiver is the error.
        if let Some(sym) = candidates.iter().copied().find(|&c| self.swapped_prefix_fits(c, recv_ty)) {
            self.type_args_for_errors(&written[1..]);
            self.swapped_receiver_error(sym, recv_ty, arg, span);
            return Some(Some((self.prog.add(TExpr::Unit), ERROR)));
        }
        *lists = Some(written);
        Some(None)
    }

    /// The argument of a right-associative selection typed against the receiver of `sym`, whose
    /// prefix took the qualifier, for the error that it does not fit there.
    fn swapped_receiver_error(&mut self, sym: SymId, q_ty: TypeId, arg: ArgSrc, span: Span) {
        let ext_clauses = self.syms.sym(sym).ext_clauses as usize;
        let sig = self.sig_arc(sym);
        let Some(at) = (0..ext_clauses).find(|&k| !sig.clauses[k].is_using) else { return };
        let subst: Subst = sig.tparams.iter().map(|&tp| (tp, self.fresh_var())).collect();
        let first = self.types.subst(sig.clauses[ext_clauses].params[0].ty, &subst);
        self.is_sub(q_ty, first);
        let receiver = self.types.subst(sig.clauses[at].params[0].ty, &subst);
        let receiver = self.solve_in(receiver);
        let expected = (!self.types.has_vars(receiver)).then_some(receiver);
        let (ta, aty, at_span) = match arg {
            ArgSrc::Ast(e) => {
                let (ta, aty) = self.type_expr(e, expected);
                (ta, aty, self.cur_ast().expr_span(e))
            }
            ArgSrc::Typed(te, ty) => (te, ty, span),
            _ => return,
        };
        if !self.types.contains_error(aty) {
            self.adapt(ta, aty, receiver, at_span);
        }
    }

    /// The swapped application of `right_assoc_selection`: the argument typed (against
    /// `expected_arg`) as the receiver, the qualifier `q` the method's first argument, the lists
    /// after the first as written.
    #[allow(clippy::too_many_arguments)]
    fn swapped_application(
        &mut self,
        q: TExprId,
        recv_ty: TypeId,
        arg: ArgSrc,
        expected_arg: Option<TypeId>,
        written: &[ArgList],
        name: Name,
        targs: Option<ListRef>,
        span: Span,
        expected: Option<TypeId>,
    ) -> Option<(TExprId, TypeId)> {
        let (ta, aty) = match arg {
            ArgSrc::Ast(e) => self.type_expr(e, expected_arg),
            ArgSrc::Typed(te, ty) => (te, ty),
            _ => return None,
        };
        let aty = self.solve_in(aty);
        let mut swapped = vec![ArgList { args: vec![ArgSrc::Typed(q, recv_ty)], using: false, span: written[0].span }];
        swapped.extend(written[1..].iter().cloned());
        let mut swapped = Some(swapped);
        self.try_extensions(ta, aty, recv_ty, name, targs, &mut swapped, span, expected)
    }

    /// Whether the prefix `op(q)` of the right-associative extension `sym` takes the qualifier's
    /// type at its method's first parameter (a number widened as an argument is), its type
    /// parameters inferred afresh.
    fn swapped_prefix_fits(&mut self, sym: SymId, q_ty: TypeId) -> bool {
        let ext_clauses = self.syms.sym(sym).ext_clauses as usize;
        let sig = self.sig_arc(sym);
        let mark = self.snapshot();
        let subst: Subst = sig.tparams.iter().map(|&tp| (tp, self.fresh_var())).collect();
        let first = self.types.subst(sig.clauses[ext_clauses].params[0].ty, &subst);
        let from = self.deref(q_ty);
        let from = self.widen_lit(from);
        let to = self.deref(first);
        let fits = self.is_sub(q_ty, first) || self.numeric_widening(from, to);
        self.rollback(mark);
        fits
    }

    /// The extensions a selection of `name` with an argument list tries, the program's nearest or
    /// else the implicit scope's of `scope_ty`, those taking a list of their own, where they are all
    /// right-associative ones whose method's clause comes before the receiver's.
    fn selection_swaps(&mut self, name: Name, scope_ty: TypeId) -> Option<Vec<SymId>> {
        // One that takes no list of its own is left out: the selection's argument list goes to an
        // overload that takes it, not to its result (`pick_extension`'s rule).
        let (lexical, _) = self.nearest_extensions_counted(name);
        let mut candidates: Vec<SymId> = lexical.into_iter().filter(|&s| !self.is_std_extension(s) && !self.is_parameterless_extension(s)).collect();
        if candidates.is_empty() {
            let mark = self.ext_modules.len();
            candidates = self.implicit_scope_extensions(scope_ty, name);
            self.ext_modules.truncate(mark);
            candidates.retain(|&s| !self.is_parameterless_extension(s));
        }
        let swap = !candidates.is_empty()
            && candidates.iter().all(|&s| {
                let ext_clauses = self.syms.sym(s).ext_clauses as usize;
                self.is_right_assoc_extension(s) && self.sig_of(s).right_assoc_order(ext_clauses).is_some()
            });
        swap.then_some(candidates)
    }

    /// dotc's `tryExtensionOrConversion`: an extension named without arguments whose
    /// application fails, or whose result does not fit the expected type, gives way to an
    /// implicit conversion of the receiver to a type with the member, when that one fits;
    /// otherwise the extension stands with its errors.
    #[allow(clippy::too_many_arguments)]
    fn try_extensions_or_conversion(
        &mut self,
        recv: TExprId,
        recv_ty: TypeId,
        name: Name,
        targs: Option<ListRef>,
        lists: &mut Option<Vec<ArgList>>,
        span: Span,
        expected: Option<TypeId>,
        converted: bool,
    ) -> Option<(TExprId, TypeId)> {
        let without_args = lists.as_ref().map_or(true, |l| l.iter().all(|a| a.using));
        if !without_args && !converted && self.interner.get(name).ends_with(':') {
            if let Some(r) = self.right_assoc_selection(recv, recv_ty, name, targs, lists, span, expected) {
                return r;
            }
        }
        if converted || !without_args {
            return self.try_extensions(recv, recv_ty, recv_ty, name, targs, lists, span, expected);
        }
        // Where no extension has the name, none is tried and nothing is set aside: no attempt (a
        // member reached through a conversion, every site of `gview_1`).
        let (lexical, imports) = self.lexical_extensions_for(name, recv_ty);
        let beyond = if lexical.is_empty() {
            // The profile's event covers the gathering, as where `try_extensions` gathers.
            let prof = self.prof(Kind::Extension, span, About::Member(recv_ty, name));
            let mut b = self.extensions_beyond(name, recv_ty);
            if b.givens.is_empty() && b.companions.is_empty() {
                self.ext_modules.truncate(b.modules_mark);
                if let Some(p) = prof {
                    self.profile.exit(p, Outcome::NotFound);
                }
                return None;
            }
            b.prof = prof;
            Some(b)
        } else {
            None
        };
        let written = lists.clone();
        // The extension is an attempt, set aside while the conversion is tried and restored where
        // the conversion does not fit either: what work done on demand
        // inside it wrote (a body typed for an inferred result) is its definition's and stays.
        let mark = self.attempt();
        let Some(r) = self.try_extensions_among(lexical, imports, beyond, recv, recv_ty, recv_ty, name, targs, lists, span, expected) else {
            self.close(mark);
            return None;
        };
        if !self.attempt_misfits(&mark, r.1, expected) {
            self.close(mark);
            return Some(r);
        }
        let extension = self.set_aside(mark);
        let conversion = self.attempt();
        if let Some(c) = self.apply_through_conversion(recv, recv_ty, name, targs, &written.unwrap_or_default(), span, expected) {
            if !self.attempt_misfits(&conversion, c.1, expected) {
                self.close(conversion);
                return Some(c);
            }
        }
        self.retract(conversion);
        self.restore(extension);
        Some(r)
    }

    /// dotc's `tryInsertImplicitOnQualifier`: a member that is inapplicable to its first argument
    /// list gives way to an extension of its name that applies, or to a conversion of the
    /// receiver to a type whose member does (`settings.map(_.map(_.toLowerCase))` over pairs,
    /// cats' `Functor` syntax or the program's `extension (p: (A, A)) def map`, beside `Tuple`'s
    /// `map` of a polymorphic function). An error past that boundary, a missing given, a later
    /// argument list, a lambda's body, an inline expansion, is the member's
    /// (`boundary_attempt`); where neither takes over, the member's application stands
    /// with its errors. What the application types of the arguments is kept (`ArgCache`): the
    /// retry adapts it and types no argument again.
    #[allow(clippy::too_many_arguments)]
    #[inline]
    fn apply_member_or_extension(
        &mut self,
        callee: Callee,
        (sym, owner_ty): (SymId, TypeId),
        recv: TExprId,
        recv_ty: TypeId,
        name: Name,
        targs: Option<ListRef>,
        lists: Vec<ArgList>,
        span: Span,
        expected: Option<TypeId>,
        converted: bool,
    ) -> (TExprId, TypeId) {
        // The member's application is an attempt, set aside where the retry takes over: what it
        // wrote of the journals the retry does not take out itself goes with it (its pending
        // expansions, a macro's infos, its deferred checks), but what a cached typing of an
        // argument wrote.
        let mark = self.attempt();
        let at = (self.snapshot(), self.diags.items.len(), self.index_mark(), self.hoisted.len());
        // The lists as written, for the retry, into a spare buffer's, which a call that types
        // gives back.
        let mut cache = self.arg_caches.pop().unwrap_or_default();
        cache.keep(&lists);
        self.watch_args(&mut cache);
        cache.attempt = Some(mark);
        cache.vars = self.tvars.len() as u32;
        cache.depth = 0;
        let outer = self.arg_cache.replace(cache);
        // The type mismatches reported are logged for the retry (`Worker::mismatches`), the
        // enclosing application's entries kept before this one's.
        let log_start = self.mismatches.len();
        self.logging += 1;
        let r = self.apply_callee(callee, targs, lists, span, expected);
        let mut cache = std::mem::replace(&mut self.arg_cache, outer).unwrap_or_default();
        self.logging -= 1;
        cache.attempt = None;
        if !self.diags.items[at.1..].iter().any(|d| !d.is_warning) {
            if self.logging == 0 {
                self.mismatches.clear();
            }
            self.close(mark);
            self.arg_caches.push(cache);
            return r;
        }
        cache.mismatches.extend_from_slice(&self.mismatches[log_start..]);
        self.mismatches.truncate(log_start);
        self.retry_on_qualifier(r, at, mark, (sym, owner_ty), recv, recv_ty, name, targs, cache, span, expected, converted)
    }

    /// The typed arguments of `lists`, and the elements of a tuple among them, each with its
    /// node and type as they are: what `adapt_typed` rewrites in place.
    fn cached_nodes(&self, lists: &[ArgList]) -> Vec<(TExprId, TExpr, Option<TypeId>)> {
        let mut out = Vec::new();
        for a in lists.iter().flat_map(|l| &l.args) {
            let (ArgSrc::Typed(te, _) | ArgSrc::Named(_, te, _)) = *a else { continue };
            out.push((te, self.prog.expr(te), self.prog.type_of(te)));
            if let TExpr::New(_, l) = self.prog.expr(te) {
                for &i in self.prog.expr_list(l) {
                    out.push((i, self.prog.expr(i), self.prog.type_of(i)));
                }
            }
        }
        out
    }

    /// Gives the typed arguments of `lists`, and the elements of a tuple among them, the types
    /// their typings gave them.
    fn settle_cached_types(&mut self, lists: &[ArgList]) {
        for a in lists.iter().flat_map(|l| l.args.clone()) {
            let (ArgSrc::Typed(te, ty) | ArgSrc::Named(_, te, ty)) = a else { continue };
            self.prog.set_type(te, ty);
            if let TExpr::New(_, l) = self.prog.expr(te) {
                let items = self.prog.expr_list(l).to_vec();
                if let Some(elems) = self.tuple_elements(ty).filter(|e| e.len() == items.len()) {
                    for (&i, &t) in items.iter().zip(&elems) {
                        self.prog.set_type(i, t);
                    }
                }
            }
        }
    }

    fn restore_nodes(&mut self, nodes: &[(TExprId, TExpr, Option<TypeId>)]) {
        for &(te, ex, ty) in nodes {
            self.prog.exprs[te.idx()] = ex;
            if let Some(ty) = ty {
                self.prog.set_type(te, ty);
            }
        }
    }

    /// Sets `cache` to record the typings of a member's application's arguments: those of the
    /// first list that is not a using one, the retry's (`boundary_attempt`), which
    /// `type_clause_args` marks; until it is reached, the arguments of any other path.
    fn watch_args(&self, cache: &mut ArgCache) {
        cache.file = self.env.file;
        cache.active = true;
        cache.done = false;
        cache.stopped = false;
        cache.typings.clear();
        cache.mismatches.clear();
    }

    /// `apply_member_or_extension` past a member's application that reported an error, begun at
    /// `at`: the retry, or the member's application back. dotty's tries in its order
    /// (`retry_tries`), each ending where the member's errors stand for it (`Boundary::Stands`);
    /// where the retry would have to type an argument the member's application typed, or reuse
    /// a typing whose dependencies the rollback undoes, none is made (`Boundary::Declines`).
    #[cold]
    #[inline(never)]
    #[allow(clippy::too_many_arguments)]
    fn retry_on_qualifier(
        &mut self,
        r: (TExprId, TypeId),
        (mark, diags, recorded, hoisted): (usize, usize, super::index::Recorded, usize),
        attempt: super::state::Mark,
        (sym, owner_ty): (SymId, TypeId),
        recv: TExprId,
        recv_ty: TypeId,
        name: Name,
        targs: Option<ListRef>,
        mut cache: Box<ArgCache>,
        span: Span,
        expected: Option<TypeId>,
        converted: bool,
    ) -> (TExprId, TypeId) {
        // Each typing's type as the application solved it, before the rollback undoes that.
        for i in 0..cache.typings.len() {
            let own = cache.typings[i].own;
            cache.typings[i].ty = self.zonk(own);
        }
        let journals = self.set_aside_journals(attempt);
        // The member's diagnostics leave the list: no promoted range keeps their positions, which
        // the tries' diagnostics take (`state::diags_cut_at`).
        self.diags_cut_at(diags);
        let member_diags = self.diags.items.split_off(diags);
        let member_index = self.index_take(recorded);
        let member_hoisted = self.hoisted.split_off(hoisted);
        // The application's variables keep their instances (dotty's permanent instantiation of a
        // variable by the state that owns it): a typing that read one stays what it was.
        let undone = self.rollback_saving_owned(mark, cache.vars as usize);
        let taken = Taken { diags: &member_diags, diags_at: diags, index: &member_index, index_at: recorded, undone: &undone, mark, vars: cache.vars as usize, mismatches: &cache.mismatches };
        let written = cache.written();
        let outer_depth = self.retry_depth.replace(self.app_depth);
        for t in self.retry_tries(sym, &written).into_iter().flatten() {
            let (lists, records, bindings, mut reused_warnings) = match self.boundary_attempt(sym, owner_ty, &written, t, &cache, &taken) {
                Boundary::Stands => continue,
                Boundary::Declines => break,
                Boundary::Retry(lists, records, bindings, warnings) => (lists, records, bindings, warnings),
            };
            // The typings the retry reuses come with the index's records they made and the
            // bindings of the variables they made.
            for (a, b) in records {
                self.index_put_back_between(&member_index, recorded, a, b);
            }
            self.rebind(&bindings);
            // A cached typing is `typedUnadapted`'s: its nodes get the types the typing gave
            // them back, which the member's adaptation may have left otherwise (a literal typed
            // `Double` by a conversion it tried, `adapt_typed`). What a try then adapts in place
            // is put back where the try fails: the nodes stand for the member's arguments too.
            self.settle_cached_types(&lists);
            let nodes = self.cached_nodes(&lists);
            let (kept_index, kept_hoisted) = (self.index_mark(), self.hoisted.len());
            let mut opt = Some(lists.clone());
            // Either takes over where it types: the expected type is no prototype of the
            // application (`IgnoredProto`, Applications.scala 1309), whose result a misfit of it
            // is reported against once applied (`init` defers it, 693). Each is an attempt.
            let tried = self.attempt();
            let outer = self.lexical_selected.replace(false);
            let ext = self.try_extensions(recv, recv_ty, recv_ty, name, targs, &mut opt, span, expected);
            let selected = std::mem::replace(&mut self.lexical_selected, outer) == Some(true);
            // The extension applied clean, or applied to the first list and failing in a later
            // one: dotty's outer application of that list fails with no retry (the function part
            // is an `Apply`, not a `Select`), and the error is the extension's.
            if let Some(ext) = ext.filter(|_| !self.diags.items[diags..].iter().any(|d| !d.is_warning) || self.later_clause_failure(diags, &written)) {
                self.close(tried);
                self.diags.items.append(&mut reused_warnings);
                self.retry_depth = outer_depth;
                self.retry_typed.clear();
                self.arg_caches.push(cache);
                return ext;
            }
            self.retract(tried);
            self.drop_reported_since(diags);
            self.rollback(mark);
            self.index_drop(kept_index);
            self.hoisted.truncate(kept_hoisted);
            self.restore_nodes(&nodes);
            // A lexical extension selected on the qualifier is the try's whatever its
            // application gives: its failure to apply opens no search for a conversion (dotty's
            // `tryExtensionOrConversion` returns it, Typer.scala 4320).
            if !converted && !selected {
                let tried = self.attempt();
                if let Some(c) = self.apply_through_conversion(recv, recv_ty, name, targs, &lists, span, expected) {
                    if !self.diags.items[diags..].iter().any(|d| !d.is_warning) || self.later_clause_failure(diags, &written) {
                        self.close(tried);
                        self.diags.items.append(&mut reused_warnings);
                        self.retry_depth = outer_depth;
                        self.retry_typed.clear();
                        self.arg_caches.push(cache);
                        return c;
                    }
                }
                self.retract(tried);
            }
            self.retry_typed.clear();
            self.drop_reported_since(diags);
            self.rollback(mark);
            self.index_drop(recorded);
            self.hoisted.truncate(hoisted);
            self.restore_nodes(&nodes);
        }
        self.retry_depth = outer_depth;
        self.retry_typed.clear();
        self.replay(undone);
        self.diags.items.extend(member_diags);
        self.index_put_back(member_index);
        self.hoisted.extend(member_hoisted);
        self.restore_journals(journals);
        // The member's errors stand. Where its attempt stopped at its first plain clause, the
        // lists after it are typed alone now, once (`typedArgs` under an erroneous function
        // part, Applications.scala 1438).
        if cache.stopped {
            let first = written.iter().position(|l| !l.using).unwrap_or(0);
            self.type_args_for_errors(&written[first + 1..]);
        }
        self.arg_caches.push(cache);
        r
    }

    /// Whether an error was reported from the diagnostics' index `before` on: the clause typed
    /// since then failed. Where it did under a member's attempt, the attempt stops there
    /// (`ArgCache::stopped`).
    #[cold]
    #[inline(never)]
    fn clause_failed(&mut self, before: usize) -> bool {
        if !self.diags.items[before..].iter().any(|d| !d.is_warning) {
            return false;
        }
        if let Some(c) = self.arg_cache.as_mut() {
            c.stopped = true;
        }
        true
    }

    /// Whether every error reported from the diagnostics' index `diags` on lies in a list of
    /// `lists` past the first plain one: the application's first clause fit, and a later one
    /// failed.
    fn later_clause_failure(&self, diags: usize, lists: &[ArgList]) -> bool {
        let first = lists.iter().position(|l| !l.using).unwrap_or(0);
        let later = &lists[first + 1..];
        if later.is_empty() {
            return false;
        }
        let mut any = false;
        for d in self.diags.items[diags..].iter().filter(|d| !d.is_warning) {
            any = true;
            if d.file != self.env.file || !later.iter().any(|l| d.span.start >= l.span.start && d.span.end <= l.span.end) {
                return false;
            }
        }
        any
    }

    /// Whether a call names type arguments the source wrote, which dotc's
    /// `tryInsertImplicitOnQualifier` takes for no extension's or conversion's (scalac's own,
    /// in a library body, are inferred ones).
    fn written_targs(&self, targs: Option<ListRef>) -> bool {
        let Some(l) = targs else { return false };
        let ast = self.cur_ast();
        !self.in_converted_body() || ast.ty_list(l).iter().any(|t| ast.inferred_types.binary_search(&t.0).is_err())
    }

    /// dotty's tries of a member's retry, in its order (Applications.scala 1494): the first list
    /// as written, then its tupled dual where the member's application used that one
    /// (`needsTupledDual`, 1671). A unary member given several positional arguments used the
    /// dual, one tuple; one of more parameters given an infix operand's tuple (`a op (b, c)`,
    /// which is one argument as written, `ApplyKind.InfixTuple`) used its elements; a unary
    /// member given that tuple used it as written, and there is no other try.
    fn retry_tries(&mut self, sym: SymId, lists: &[ArgList]) -> [Option<RetryTry>; 2] {
        if self.syms.sym(sym).kind != SymKind::Def {
            return [None, None];
        }
        let sig = self.sig_arc(sym);
        // A member whose first clause is a using one, or given a using list first, is applied to
        // that clause innermost: what fails is an application, not a selection, which
        // `tryInsertImplicitOnQualifier` retries nothing on (Typer.scala 4245, a `Select` or a
        // `TypeApply` of one alone).
        if sig.clauses.first().is_some_and(|c| c.is_using) || lists.first().is_some_and(|l| l.using) {
            return [None, None];
        }
        let (Some(clause), Some(list)) = (sig.clauses.iter().find(|c| !c.is_using), lists.iter().find(|l| !l.using)) else { return [None, None] };
        let unary = clause.params.len() == 1 && !clause.params[0].repeated;
        let ast = self.cur_ast();
        let positional = list.args.len() > 1 && list.args.iter().all(|a| matches!(*a, ArgSrc::Ast(e) | ArgSrc::Hoisted(e) if !matches!(ast.expr(e), crate::ast::Expr::NamedArg(..))));
        let infix = positional && matches!(list.args[0], ArgSrc::Ast(e) | ArgSrc::Hoisted(e) if self.infix_tuples.contains_key(&(self.env.file, e)));
        match (infix, unary) {
            (true, true) => [Some(RetryTry::Tupled), None],
            (true, false) => [Some(RetryTry::Tupled), Some(RetryTry::Separate)],
            (false, true) if positional => [Some(RetryTry::Separate), Some(RetryTry::Tupled)],
            _ => [Some(RetryTry::Separate), None],
        }
    }

    /// One of dotty's `tryWithImplicitOnQualifier` tries (Applications.scala 1373, scalac 3.8.4's
    /// lines) past a member's application that failed. The member's errors stand
    /// for the try where an argument of the first list is an error argument (`FunProto.hasErrorArg`)
    /// or the member matches the list (`SelectionProto.isMatchedBy`); otherwise the argument
    /// lists the retry applies the extension or the conversion to, each argument as the
    /// member's application typed it (`ArgCache`), which dotty's `FunProto` caches and its retry
    /// adapts. The retry declines where it cannot reuse such a typing: one that placed a
    /// diagnostic other than an error argument's, constrained a type variable, bound a
    /// temporary, a sequence spliced, an operand evaluated ahead of the receiver, an argument
    /// dotty types again (a typing that failed short of an error argument, an argument of a
    /// tupled dual tried as written that is no literal its formal left as typed alone). An
    /// argument the member's application did not type is typed here, its only typing.
    fn boundary_attempt(&mut self, sym: SymId, owner_ty: TypeId, lists: &[ArgList], t: RetryTry, cache: &ArgCache, taken: &Taken) -> Boundary {
        let sig = self.sig_arc(sym);
        let (Some(clause), Some(at)) = (sig.clauses.iter().find(|c| !c.is_using), lists.iter().position(|l| !l.using)) else { return Boundary::Stands };
        if t == RetryTry::Tupled {
            return self.tupled_boundary(&sig, clause, owner_ty, lists, at, cache, taken);
        }
        let Some((args, matched, reordered)) = self.boundary_args(clause, &lists[at]) else { return Boundary::Stands };
        // The member's application used the tupled dual: these arguments, typed inside its
        // tuple, are typed alone in dotty's test (`typedArgs`).
        let dual = cache.typings.iter().find(|ty| ty.dual.is_some());
        let positional: Vec<ExprId> = lists[at].args.iter().filter_map(|a| a.ast()).collect();
        let mut used: Vec<Option<(TExprId, TypeId)>> = Vec::with_capacity(args.len());
        let mut types: Vec<Option<TypeId>> = Vec::with_capacity(args.len());
        let mut records = Vec::new();
        let mut bindings = Vec::new();
        let mut warnings = Vec::new();
        let (mut declines, mut erroneous, mut retypes) = (false, false, false);
        for a in &args {
            if let Some(p) = a.pretyped {
                used.push(Some(p));
                types.push(Some(p.1));
                continue;
            }
            let Some(value) = a.value else {
                used.push(None);
                types.push(None);
                continue;
            };
            if let Some(d) = dual {
                let alone = self.dual_element_alone(d, value, taken, &positional);
                if alone.is_some() && d.dual == Some(true) && !records.contains(&d.index) {
                    records.push(d.index);
                    bindings.extend(self.bindings_in(taken, d, d.trail().1).into_iter().flatten());
                }
                declines |= alone.is_none();
                used.push(alone);
                types.push(alone.map(|p| p.1));
                continue;
            }
            // A splice's typing is its sequence's.
            let key = match self.cur_ast().expr(value) {
                crate::ast::Expr::Typed(inner, _) if a.splice => inner,
                _ => value,
            };
            let typing = cache.typings.iter().rfind(|ty| ty.e == key && ty.dual.is_none()).filter(|ty| !ty.untyped);
            match typing {
                Some(ty) => match self.cached_argument(a, ty, taken, reordered) {
                    Cached::ErrorArg => return Boundary::Stands,
                    Cached::Erroneous => {
                        erroneous = true;
                        used.push(None);
                        types.push(None);
                    }
                    Cached::Declined => {
                        retypes = true;
                        used.push(None);
                        types.push(None);
                    }
                    Cached::Typed(te, t, reused) => {
                        declines |= reused.is_none() || a.splice || a.hoisted;
                        if let Some(b) = reused {
                            records.push(ty.index);
                            bindings.extend(b);
                            warnings.extend(self.expansion_warnings(a, ty, taken));
                        }
                        used.push(Some((te, t)));
                        types.push(Some(t));
                    }
                },
                None => {
                    // An argument the member's application did not type, a function literal
                    // with a parameter of no written type being `FunctionN(declared or ?, ?)`
                    // alone (`cacheTypedArg` unforced, ProtoTypes.scala 475).
                    let e = key;
                    if let Some(shape) = self.function_shape(e) {
                        used.push(None);
                        types.push(Some(shape));
                        continue;
                    }
                    declines |= a.splice || a.hoisted;
                    let kept = self.typed_alone_kept(e);
                    erroneous |= kept.is_none();
                    used.push(kept);
                    types.push(kept.map(|p| p.1));
                }
            }
        }
        // An argument dotty types again has no typing here to test the member with.
        if retypes {
            return Boundary::Declines;
        }
        let outer = self.snapshot();
        let diags = self.diags.items.len();
        let subst = self.boundary_vars(&sig, owner_ty);
        let applicable = matched && self.applicable_to_types(clause, &args, &types, &subst);
        self.discard_diagnostics(diags);
        self.rollback(outer);
        if applicable {
            return Boundary::Stands;
        }
        if declines || erroneous || records.iter().any(|&(a, b)| !self.index_holds(taken.index, taken.index_at, a, b)) {
            return Boundary::Declines;
        }
        let mut out = lists.to_vec();
        for (a, u) in args.iter().zip(used) {
            let (Some((te, ty)), None) = (u, a.pretyped) else { continue };
            self.retry_typed.insert(te, ());
            out[at].args[a.src] = match a.named {
                Some(n) => ArgSrc::Named(n, te, ty),
                None => ArgSrc::Typed(te, ty),
            };
        }
        Boundary::Retry(out, records, bindings, warnings)
    }

    /// The member's typing `ty` of the argument `a`, as the retry would reuse it: an error
    /// argument where an error is inside it (`hasInnerErrors`); declined where it failed short
    /// of one, which dotty types again; otherwise its typing (a named one's adapted, as
    /// `typedNamedArg` types one), reusable where it placed no diagnostic, constrained no
    /// variable older than the application and bound no temporary. A named argument's typing whose
    /// adaptation alone failed, and any argument where `reorder` failed (`reordered` false:
    /// dotty typed none), stand for the typing alone dotty makes where they are that typing,
    /// a literal or a tuple of literals the formal left as typed alone (`literal_alone`), and
    /// decline otherwise. One typed alone and erroneous is erroneous.
    fn cached_argument(&mut self, a: &BoundaryArg, ty: &ArgTyping, taken: &Taken, reordered: bool) -> Cached {
        let alone = |w: &mut Self| match w.literal_alone(ty.e, ty.te) {
            Some(t) => Cached::Typed(ty.te, t, Some(Vec::new())),
            None => Cached::Declined,
        };
        if !reordered && ty.formal.is_some() {
            return alone(self);
        }
        let named = a.named.is_some();
        let end = if named { ty.diags().2 } else { ty.diags().1 };
        if ty.diags().0 < taken.diags_at || end - taken.diags_at > taken.diags.len() {
            return Cached::Erroneous;
        }
        let placed = &taken.diags[ty.diags().0 - taken.diags_at..end - taken.diags_at];
        if placed.iter().any(|d| !d.is_warning) {
            let Some(formal) = ty.formal else { return Cached::Erroneous };
            let typing = &taken.diags[ty.diags().0 - taken.diags_at..ty.diags().1 - taken.diags_at];
            if named && !typing.iter().any(|d| !d.is_warning) {
                return alone(self);
            }
            // `hasInnerErrors` judges the diagnostics the typing placed, as they were.
            let from = self.diags.items.len();
            self.diags.items.extend_from_slice(placed);
            // An entry stands for the diagnostic at its index where the spans agree: a
            // speculation inside the typing may have logged one and discarded its diagnostic.
            let logged: Vec<(usize, TypeId)> = taken.mismatches.iter().filter(|&&(i, sp, _)| i >= ty.diags().0 && i < end && taken.diags[i - taken.diags_at].span == sp).map(|&(i, _, r)| (i - ty.diags().0 + from, r)).collect();
            let tree = if named { a.written.unwrap_or(ty.e) } else { ty.e };
            let inner = self.has_inner_errors(tree, named, formal, formal, ty.own, from, &logged);
            self.drop_reported_since(from);
            return if inner { Cached::ErrorArg } else { Cached::Declined };
        }
        let (te, t) = match (named, ty.adapted) {
            (true, Some(adapted)) => (adapted, self.prog.type_of(adapted).unwrap_or(ty.formal.unwrap_or(ty.ty))),
            _ => (ty.te, ty.ty),
        };
        // The typing is reusable where it placed no diagnostic but the typer's own pure-statement
        // warning, bound no temporary and constrained no variable older than the application
        // (`bindings_in`). That warning goes with the application, as dotty's cached typing keeps
        // nothing of the failed state's reporter (`{ 1; 2 }` reused, no warning, as scalac's); a
        // macro's warning is scalac's to report again where its retry expands the argument again:
        // an argument that is the expansion itself (`put(M.next, 3)`, its warnings standing at the
        // argument) is reused with them (`expansion_warnings`), one that holds an expansion (a
        // nested, wrapped or named argument: `id(M.next)`, `{ M.next }`, `(M.next: Int)`, `y =
        // M.next`), which scalac reuses without the warning, declines.
        let to = if named { ty.trail().2 } else { ty.trail().1 };
        let pure_statement = |d: &crate::source::Diagnostic| d.is_warning && d.msg.starts_with("A pure expression does nothing in statement position");
        let own = self.own_expansion_warning(a, ty);
        let bindings = if placed.iter().all(|d| pure_statement(d) || own(d)) && !ty.hoisted { self.bindings_in(taken, ty, to) } else { None };
        Cached::Typed(te, t, bindings)
    }

    /// The warnings the member's typing `ty` of the argument `a` placed where the argument is a
    /// macro's or an inline method's expansion itself, which scalac's retry expands again and
    /// warns of again: kept for the try that reuses the typing, the pure-statement one aside.
    fn expansion_warnings(&self, a: &BoundaryArg, ty: &ArgTyping, taken: &Taken) -> Vec<crate::source::Diagnostic> {
        let own = self.own_expansion_warning(a, ty);
        let (from, to) = (ty.diags().0 - taken.diags_at, ty.diags().1 - taken.diags_at);
        taken.diags[from..to.min(taken.diags.len())].iter().filter(|d| own(d)).cloned().collect()
    }

    /// Whether a warning the member's typing `ty` of the argument `a` placed is the argument's own
    /// expansion's: the argument a plain call written bare in its list (`BoundaryArg::bare`), whose
    /// tree is an expansion, the warning standing at the argument itself, as a macro's stands at
    /// its call (not at a call inside a block, parentheses, an ascription or another inline
    /// method's argument).
    fn own_expansion_warning(&self, a: &BoundaryArg, ty: &ArgTyping) -> impl Fn(&crate::source::Diagnostic) -> bool {
        let call = matches!(self.cur_ast().expr(ty.e), Expr::Ident(_) | Expr::Select(..) | Expr::Apply(..) | Expr::TypeApply(..));
        let (file, at) = (self.env.file, self.cur_ast().expr_span(ty.e));
        let direct = a.named.is_none() && a.bare && call && self.prog.is_expansion(ty.te);
        move |d: &crate::source::Diagnostic| direct && d.is_warning && d.file == file && d.span == at
    }

    /// `tupled_boundary` of the try with the arguments tupled into one (`untpd.Tuple(args)`):
    /// the tuple the member's application typed (`typedTuple`, Typer.scala 3729, its elements
    /// against its parameter's element types: an error in one is inside the tuple, an error
    /// argument), or, where it typed the elements as its arguments (an infix operand's tuple
    /// for a member of more parameters than one), the tuple alone of the literals they are; an
    /// element of another kind declines. Where it typed none, the tuple is typed here.
    #[allow(clippy::too_many_arguments)]
    fn tupled_boundary(&mut self, sig: &MethodSig, clause: &ClauseSig, owner_ty: TypeId, lists: &[ArgList], at: usize, cache: &ArgCache, taken: &Taken) -> Boundary {
        let list = &lists[at];
        let Some(elems) = list.args.iter().map(|a| match *a {
            ArgSrc::Ast(e) => Some(e),
            _ => None,
        }).collect::<Option<Vec<ExprId>>>() else { return Boundary::Declines };
        let mut records = Vec::new();
        let mut bindings = Vec::new();
        let (te, ty, reusable) = match cache.typings.iter().rfind(|t| t.dual.is_some() && t.e == elems[0]) {
            Some(t) => {
                if t.diags().0 < taken.diags_at || t.diags().1 - taken.diags_at > taken.diags.len() {
                    return Boundary::Declines;
                }
                let placed = &taken.diags[t.diags().0 - taken.diags_at..t.diags().1 - taken.diags_at];
                if placed.iter().any(|d| !d.is_warning) {
                    return Boundary::Stands;
                }
                records.push(t.index);
                let own = self.bindings_in(taken, t, t.trail().1);
                let reusable = placed.is_empty() && own.is_some() && !t.hoisted;
                bindings.extend(own.into_iter().flatten());
                (t.te, t.ty, reusable)
            }
            None if elems.iter().all(|&e| !cache.typings.iter().any(|t| t.e == e)) => {
                let (diags, hoisted, recorded) = (self.diags.items.len(), self.hoisted.len(), self.index_mark());
                let (te, ty) = self.type_tuple(&elems, None);
                let failed = self.diags.items[diags..].iter().any(|d| !d.is_warning);
                self.discard_diagnostics(diags);
                if failed {
                    self.hoisted.truncate(hoisted);
                    self.index_drop(recorded);
                    return Boundary::Stands;
                }
                let ty = self.zonk(ty);
                (te, ty, true)
            }
            None => {
                let mut items = Vec::with_capacity(elems.len());
                let mut tys = Vec::with_capacity(elems.len());
                for &e in &elems {
                    let alone = cache.typings.iter().rfind(|t| t.e == e && t.dual.is_none()).and_then(|t| self.literal_alone(e, t.te).map(|ty| (t.te, ty)));
                    let Some((te, ty)) = alone else { return Boundary::Declines };
                    items.push(te);
                    tys.push(ty);
                }
                let ty = self.tuple_of(&tys);
                let te = self.tuple_value(&items);
                self.prog.set_type(te, ty);
                // What `type_tuple` records of a tuple it makes, for the writer and the dependencies.
                if self.capturing() && items.len() <= 22 {
                    self.capture_targs(te, &tys);
                }
                if self.deps.is_some() {
                    let class = self.tuple_class(items.len());
                    self.deps_node(te, super::deps::Node::Apply(class));
                }
                (te, ty, true)
            }
        };
        // The member takes the tuple where its first parameter does and the others have
        // defaults.
        let outer = self.snapshot();
        let subst = self.boundary_vars(sig, owner_ty);
        let formal = self.types.subst(clause.params[0].ty, &subst);
        let formal = self.deref(formal);
        let matches = clause.params[1..].iter().all(|p| p.has_default) && self.fits_boundary(ty, formal);
        self.rollback(outer);
        if matches {
            return Boundary::Stands;
        }
        if !reusable || records.iter().any(|&(a, b)| !self.index_holds(taken.index, taken.index_at, a, b)) {
            return Boundary::Declines;
        }
        self.retry_typed.insert(te, ());
        let mut out = lists.to_vec();
        out[at] = ArgList { args: vec![ArgSrc::Typed(te, ty)], using: false, span: list.span };
        Boundary::Retry(out, records, bindings, Vec::new())
    }

    /// The element `e` of the tupled dual the member's application typed (`d`, of the positional
    /// arguments `elems`), as typed alone for the try of the arguments as written: as the dual
    /// typed it where that typed its elements alone (its formal no tuple type of as many) and
    /// placed no diagnostic, constrained no older variable and bound no temporary; otherwise a
    /// literal its element's type left as it is (`literal_alone`).
    fn dual_element_alone(&mut self, d: &ArgTyping, e: ExprId, taken: &Taken, elems: &[ExprId]) -> Option<(TExprId, TypeId)> {
        let i = elems.iter().position(|&x| x == e)?;
        let TExpr::New(_, l) = self.prog.expr(d.te) else { return None };
        let items = self.prog.expr_list(l);
        if items.len() != elems.len() {
            return None;
        }
        let te = items[i];
        let clean = d.diags().0 >= taken.diags_at && d.diags().1 - taken.diags_at <= taken.diags.len() && d.diags().0 == d.diags().1;
        if d.dual == Some(true) && clean && !d.hoisted && self.bindings_in(taken, d, d.trail().1).is_some() {
            let ty = self.tuple_elements(d.ty)?.get(i).copied()?;
            return Some((te, ty));
        }
        self.literal_alone(e, te).map(|ty| (te, ty))
    }

    /// The type a literal `e` the member's application typed to `te` has typed alone, where
    /// that typing is the one alone: a literal its formal left as it is (an `Int` literal
    /// against an `Int`, not against a `Long`, which dotty's `typedNumber` makes a `Long` one),
    /// or a tuple of such.
    fn literal_alone(&mut self, e: ExprId, te: TExprId) -> Option<TypeId> {
        use crate::ast::Expr;
        let ast = self.cur_ast();
        if let Expr::Parens(inner) = ast.expr(e) {
            return self.literal_alone(inner, te);
        }
        if let (Expr::Tuple(l), TExpr::New(c, items)) = (ast.expr(e), self.prog.expr(te)) {
            let elems = ast.expr_list(l);
            let items = self.prog.expr_list(items).to_vec();
            if !self.is_tuple_class(c) || elems.len() != items.len() || elems.len() > 22 {
                return None;
            }
            let tys = elems.iter().zip(&items).map(|(&e, &te)| self.literal_alone(e, te)).collect::<Option<Vec<TypeId>>>()?;
            let ty = self.tuple_of(&tys);
            self.prog.set_type(te, ty);
            if self.capturing() {
                self.capture_targs(te, &tys);
            }
            return Some(ty);
        }
        let ty = match (ast.expr(e), self.prog.expr(te)) {
            (Expr::IntLit(_), TExpr::Int(_)) => self.b.t_int,
            (Expr::LongLit(_), TExpr::Long(_)) => self.b.t_long,
            (Expr::DoubleLit(_), TExpr::Double(_)) => self.b.t_double,
            (Expr::DecimalLit(_), TExpr::Double(_)) if self.prog.type_of(te) != Some(self.b.t_float) => self.b.t_double,
            (Expr::FloatLit(_), TExpr::Double(_)) => self.b.t_float,
            (Expr::BoolLit(_), TExpr::Bool(_)) => self.b.t_boolean,
            (Expr::CharLit(_), TExpr::Char(_)) => self.b.t_char,
            (Expr::StringLit(_), TExpr::Str(_)) => self.b.t_string,
            _ => return None,
        };
        self.prog.set_type(te, ty);
        Some(ty)
    }

    /// Whether the argument `e` of the list at `list` stands bare in it, as scalac reads it: the
    /// list's opening parenthesis or a comma before it, parentheses and blanks alone between, as
    /// after it (parentheses are no tree for scalac either); not inside braces or after a
    /// comment, which the parser drops too (`{ M.next }`, `{ (M.next) }` are read as `M.next`).
    fn bare_in_list(&self, e: ExprId, list: Span) -> bool {
        let text = &self.source(self.env.file).text;
        let at = self.cur_ast().expr_span(e);
        let (Some(before), Some(after)) = (text.get(list.start as usize..at.start as usize), text.get(at.end as usize..list.end as usize)) else { return false };
        let before = before.trim_end();
        let outer = before.trim_end_matches(|c: char| c == '(' || c.is_whitespace());
        let parens = before[outer.len()..].matches('(').count();
        // The list's own parenthesis follows a name, a type argument list or a list before it.
        let opens = outer.ends_with(',') || parens >= 1 && outer.ends_with(|c: char| c.is_alphanumeric() || matches!(c, '_' | ']' | ')' | '`'));
        let rest = after.trim_start_matches(|c: char| c == ')' || c.is_whitespace());
        opens && !rest.starts_with(['}', '/'])
    }

    /// The first list's arguments with the parameters they go to, as `Application.matchArgs`
    /// pairs them (a named one by its name, `reorder`), and whether the pairing succeeds: an
    /// argument of no parameter, a parameter of no argument and no default, a sequence spliced
    /// where it may not be (`checkNoVarArg`) fail it. `None` where an argument is of a form the
    /// typer typed before the member was known.
    fn boundary_args(&mut self, clause: &ClauseSig, list: &ArgList) -> Option<(Vec<BoundaryArg>, bool, bool)> {
        let ast = self.cur_ast();
        let mut out = Vec::with_capacity(list.args.len());
        let mut matched = true;
        // `reorder` fails a named argument of no parameter, or of one already given, before any
        // argument is typed (`init`: `if success then matchArgs`, Applications.scala 661).
        let mut reordered = true;
        let mut filled = vec![false; clause.params.len()];
        let mut repeated_args = 0;
        for (i, arg) in list.args.iter().enumerate() {
            let (e, pretyped, hoisted) = match *arg {
                ArgSrc::Ast(e) => (Some(e), None, false),
                ArgSrc::Hoisted(e) => (Some(e), None, true),
                ArgSrc::Typed(te, ty) => (None, Some((te, ty)), false),
                _ => return None,
            };
            let named = e.and_then(|e| match ast.expr(e) {
                crate::ast::Expr::NamedArg(n, v) => Some((n, v)),
                _ => None,
            });
            let at = match named {
                Some((n, _)) => clause.params.iter().position(|p| p.name == n),
                None => match clause.params.len() {
                    0 => None,
                    n if i < n => Some(i),
                    n => clause.params[n - 1].repeated.then_some(n - 1),
                },
            };
            let Some(at) = at else {
                matched = false;
                reordered &= named.is_none();
                continue;
            };
            // A parameter named twice, or a positional argument where a named one went.
            if filled[at] && !clause.params[at].repeated {
                matched = false;
                reordered &= named.is_none();
            }
            filled[at] = true;
            let value = named.map(|(_, v)| v).or(e);
            let splice = value.is_some_and(|e| matches!(ast.expr(e), crate::ast::Expr::Typed(_, t) if matches!(ast.ty(t), crate::ast::TyExpr::Repeated(_))));
            let repeated = clause.params[at].repeated;
            if repeated {
                repeated_args += 1;
            }
            if splice && !repeated {
                matched = false;
            }
            let bare = e.is_some_and(|e| self.bare_in_list(e, list.span));
            out.push(BoundaryArg { written: e, value, pretyped, param: at, splice, named: named.map(|(n, _)| n), src: i, hoisted, bare });
        }
        // A splice is the repeated parameter's only argument.
        if out.iter().any(|a| a.splice) && repeated_args > 1 {
            matched = false;
        }
        if clause.params.iter().zip(&filled).any(|(p, &f)| !f && !p.has_default && !p.repeated) {
            matched = false;
        }
        Some((out, matched, reordered))
    }

    /// The method's type parameters as variables within their bounds, as `Application.methType`
    /// instantiates a `PolyType` (`instantiateWithTypeVars`), its owner's as the receiver gives.
    fn boundary_vars(&mut self, sig: &MethodSig, owner_ty: TypeId) -> Subst {
        let mut subst = self.owner_subst(owner_ty);
        for &tp in &sig.tparams {
            let v = self.fresh_var();
            subst.push((tp, v));
        }
        for &tp in &sig.tparams {
            let (upper, lower) = (self.syms.tparam(tp).upper, self.syms.tparam(tp).lower);
            let var = self.types.param(tp);
            let var = self.types.subst(var, &subst);
            if upper != ANY {
                let u = self.types.subst(upper, &subst);
                self.is_sub(var, u);
            }
            if lower != NOTHING {
                let l = self.types.subst(lower, &subst);
                self.is_sub(l, var);
            }
        }
        subst
    }

    /// The formal an argument is typed against: its parameter's type, a repeated one's element
    /// (`matchArgs`' `elemFormal`).
    fn boundary_formal(&mut self, clause: &ClauseSig, a: &BoundaryArg, subst: &Subst) -> TypeId {
        let t = self.types.subst(clause.params[a.param].ty, subst);
        self.deref(t)
    }

    /// dotty's `FunProto.hasInnerErrors` (ProtoTypes.scala 450) of the argument `e`, typed against
    /// `pt` (its formal `formal`, or a splice's sequence) to the type `own`, over the
    /// diagnostics from `diags` on. The tree it judges is the argument's own (a named argument's
    /// whole `n = v`, which it does not look through), through an ascription to what it
    /// ascribes (451), through a function literal, a block of one alone, a polymorphic one and
    /// the context function dotty puts around an argument where one is expected (452,
    /// `closureDef`) to the body, typed against the formal's result (a function's, a SAM's, a
    /// context function's). An error inside that tree counts, not one at the tree itself, nor
    /// any where the tree's own type is the error (a selection on an erroneous qualifier), nor
    /// a type mismatch whose expected type is `formal` (i20335: an `if` whose branch does not
    /// fit). teq reports some errors inside a tree at the tree (an element of `Array(..)`): a
    /// mismatch there of another type than the tree's own expected one, where that is known,
    /// is inside it.
    #[allow(clippy::too_many_arguments)]
    fn has_inner_errors(&mut self, e: ExprId, named: bool, pt: TypeId, formal: TypeId, own: TypeId, diags: usize, logged: &[(usize, TypeId)]) -> bool {
        use crate::ast::Expr;
        let ast = self.cur_ast();
        let (mut t, mut own, mut expected) = (e, own, Some(pt));
        while !named {
            // dotty puts a context function around an argument that is none where one is
            // expected: its body is the argument, typed against the result.
            if let Some((_, ret)) = expected.and_then(|x| self.as_context_function(x)) {
                if !self.is_contextual_closure(t) {
                    expected = Some(ret);
                    own = self.as_context_function(own).map_or(own, |(_, r)| r);
                    continue;
                }
            }
            match ast.expr(t) {
                Expr::Typed(inner, a) if !matches!(ast.ty(a), crate::ast::TyExpr::Repeated(_)) => {
                    t = inner;
                    expected = Some(self.resolve_type(a));
                }
                Expr::Parens(inner) => t = inner,
                Expr::Lambda(ps, body) => {
                    let arity = ps.len as usize;
                    t = body;
                    own = self.function_result(own, arity).unwrap_or(own);
                    expected = expected.and_then(|x| self.function_result(x, arity));
                }
                Expr::PolyLambda(_, lambda) => {
                    t = lambda;
                    let unpoly = |w: &Self, t: TypeId| match w.types.get(t) {
                        Type::Poly(_, body) => body,
                        _ => t,
                    };
                    own = unpoly(self, own);
                    expected = expected.map(|x| unpoly(self, x));
                }
                Expr::Block(l) if matches!(ast.stmt_list(l), &[crate::ast::Stmt::Expr(x)] if matches!(ast.expr(x), Expr::Lambda(..) | Expr::PolyLambda(..))) => {
                    let &[crate::ast::Stmt::Expr(x)] = ast.stmt_list(l) else { unreachable!() };
                    t = x;
                }
                _ => break,
            }
        }
        if own == ERROR {
            return false;
        }
        let at = ast.expr_span(t);
        let formal = self.zonk(formal);
        let expected = expected.map(|x| self.zonk(x));
        let required: Vec<(usize, TypeId)> = logged.iter().map(|&(i, r)| (i, self.zonk(r))).collect();
        self.diags.items[diags..].iter().enumerate().any(|(k, d)| {
            let mismatch = required.iter().find(|&&(i, _)| i == diags + k).map(|&(_, r)| r);
            let inside = d.span != at && d.span.start >= at.start && d.span.end <= at.end;
            let placed_at_tree = d.span == at && mismatch.is_some_and(|r| expected.is_some_and(|x| r != x));
            !d.is_warning && mismatch != Some(formal) && (inside || placed_at_tree)
        })
    }

    /// The result type a function literal of `arity` parameters is typed against where `t` is
    /// expected (`decomposeProtoFunction`): a function type's, a SAM's method's, a context
    /// function's.
    fn function_result(&mut self, t: TypeId, arity: usize) -> Option<TypeId> {
        if let Some((_, ret)) = self.as_function(t) {
            return Some(ret);
        }
        if let Some((_, ret)) = self.as_context_function(t) {
            return Some(ret);
        }
        let (_, _, sig, subst) = self.sam_method(t, arity)?;
        Some(self.types.subst(sig.ret, &subst))
    }

    /// `x ?=> e`, which dotty puts no context function around (`isContextualClosure`).
    fn is_contextual_closure(&self, e: ExprId) -> bool {
        let ast = self.cur_ast();
        match ast.expr(e) {
            crate::ast::Expr::Lambda(ps, _) => ps.len > 0 && ast.lambda_params[ps.range()].iter().all(|p| p.contextual),
            crate::ast::Expr::Parens(inner) => self.is_contextual_closure(inner),
            _ => false,
        }
    }

    /// `isApplicableMethodRef` of the member to the arguments of the types `types` (`FunProto.typedArgs`,
    /// ProtoTypes.scala 508, `ApplicableToTrees`, Applications.scala 1079), under fresh
    /// variables: each fits its formal as `TestApplication.argOK` has it (958), a spliced one
    /// its sequence or its array's elements; an erroneous one, of no type here, by its error type.
    fn applicable_to_types(&mut self, clause: &ClauseSig, args: &[BoundaryArg], types: &[Option<TypeId>], subst: &Subst) -> bool {
        for (a, &ty) in args.iter().zip(types) {
            let Some(ty) = ty else { continue };
            let formal = self.boundary_formal(clause, a, subst);
            let fits = match a.splice {
                true => match self.array_element(ty) {
                    Some(elem) => self.fits_boundary(elem, formal),
                    None => match self.spliced_formal(formal, false) {
                        Some(seq) => self.fits_boundary(ty, seq),
                        None => true,
                    },
                },
                false => self.fits_boundary(ty, formal),
            };
            if !fits {
                return false;
            }
        }
        true
    }

    /// `FunctionN(declared or ?, ?)`, what `cacheTypedArg` unforced makes of a function literal
    /// with a parameter of no written type (`functionWithUnknownParamType`, TreeInfo.scala 446,
    /// through a block of it alone).
    fn function_shape(&mut self, e: ExprId) -> Option<TypeId> {
        use crate::ast::{Expr, Stmt};
        let ast = self.cur_ast();
        match ast.expr(e) {
            Expr::Lambda(ps, _) => {
                let params = ast.lambda_params[ps.range()].to_vec();
                if params.iter().all(|p| p.ty.is_some()) {
                    return None;
                }
                let tys: Vec<TypeId> = params.iter().map(|p| match p.ty {
                    Some(t) => self.resolve_type(t),
                    None => self.fresh_var(),
                }).collect();
                let ret = self.fresh_var();
                Some(self.fun_type(&tys, ret))
            }
            Expr::Block(l) => match ast.stmt_list(l) {
                &[Stmt::Expr(x)] => self.function_shape(x),
                _ => None,
            },
            Expr::Parens(inner) => self.function_shape(inner),
            _ => None,
        }
    }

    /// `e` typed alone, as `FunProto.typedArgs` types it, the typing kept where it is clean, its
    /// temporaries and records too: `None` where it is erroneous.
    fn typed_alone_kept(&mut self, e: ExprId) -> Option<(TExprId, TypeId)> {
        let (diags, recorded, hoisted) = (self.diags.items.len(), self.index_mark(), self.hoisted.len());
        let (te, ty) = self.type_expr(e, None);
        let erroneous = self.diags.items[diags..].iter().any(|d| !d.is_warning) || self.types.contains_error(ty);
        let ty = self.zonk(ty);
        self.discard_diagnostics(diags);
        if erroneous {
            self.index_drop(recorded);
            self.hoisted.truncate(hoisted);
            return None;
        }
        Some((te, ty))
    }

    /// `Seq[T]`, or `Array[T]`, of a repeated parameter's element `T`.
    fn spliced_formal(&mut self, elem: TypeId, array: bool) -> Option<TypeId> {
        let c = if array { self.b.array } else { self.seq_class()? };
        Some(self.types.class(c, &[elem]))
    }

    /// `T` of `Array[T]`.
    /// An array spread as it is written (`f(arr*)`): the sequence the conversion to the
    /// parameter's makes of it is the typer's, which a Java parameter takes as the array itself
    /// (`Program::spread_bits`); a sequence written in the program is no such array.
    fn mark_spread_array(&mut self, spread: bool, te: TExprId, ty: TypeId) {
        if spread && self.array_element(ty).is_some() {
            self.prog.mark_spread(te);
        }
    }

    pub(super) fn array_element(&mut self, ty: TypeId) -> Option<TypeId> {
        let ty = self.deref(ty);
        match self.types.get(ty) {
            Type::Class(c, args) if c == self.b.array => self.types.items(args).first().copied(),
            _ => None,
        }
    }

    /// `TestApplication.argOK` (Applications.scala 958): whether an argument of type `ty` fits
    /// `formal` (`isCompatible`, a view allowed, and `SAMargOK`), the constraints on the method's
    /// variables kept for the arguments after it. A context function formal takes a context
    /// function alone (`ApplicableToTrees.argType`, 1082: no typed tree is wrapped in one).
    fn fits_boundary(&mut self, ty: TypeId, formal: TypeId) -> bool {
        if self.as_context_function(formal).is_some() && self.as_context_function(ty).is_none() {
            return false;
        }
        let view_compat = std::mem::replace(&mut self.view_compat, true);
        let fits = self.is_compatible(ty, formal);
        self.view_compat = view_compat;
        fits
    }

    /// Whether the attempt `m` failed: it reported an error, or its result of type `ty` does not
    /// fit the expected type.
    fn attempt_misfits(&mut self, m: &super::state::Mark, ty: TypeId, expected: Option<TypeId>) -> bool {
        if self.attempt_failed(m) {
            return true;
        }
        let Some(exp) = expected else { return false };
        let exp = self.deref(exp);
        if ty == ERROR || exp == ANY || exp == self.b.t_unit {
            return false;
        }
        let mark = self.snapshot();
        let view_compat = std::mem::replace(&mut self.view_compat, true);
        let fits = self.is_compatible(ty, exp);
        self.view_compat = view_compat;
        self.rollback(mark);
        !fits
    }

    #[allow(clippy::too_many_arguments)]
    fn extensions_beyond_lexical(
        &mut self,
        lexical: Lexical,
        beyond: Option<ExtensionsBeyond>,
        recv: TExprId,
        recv_ty: TypeId,
        scope_ty: TypeId,
        name: Name,
        targs: Option<ListRef>,
        lists: &mut Option<Vec<ArgList>>,
        span: Span,
        expected: Option<TypeId>,
    ) -> Option<(TExprId, TypeId)> {
        let b = match beyond {
            Some(b) => b,
            None => self.extensions_beyond(name, scope_ty),
        };
        let r = self.extensions_of_scopes(lexical, b.givens, b.contextual, b.levels, b.companions, recv, recv_ty, name, targs, lists, span, expected);
        self.ext_modules.truncate(b.modules_mark);
        r
    }

    /// The extensions called `name` beyond the lexical scope of `scope_ty`'s receiver.
    fn extensions_beyond(&mut self, name: Name, scope_ty: TypeId) -> ExtensionsBeyond {
        let (givens, contextual, levels) = self.givens_with_extension_levels(name, scope_ty);
        let modules_mark = self.ext_modules.len();
        let companions = self.implicit_scope_extensions(scope_ty, name);
        ExtensionsBeyond { givens, contextual, levels, companions, modules_mark, prof: None }
    }

    /// dotty's `tryExtensionOrConversion` (Typer.scala 4274 to 4358) over the lexical scope's
    /// outcome: a lexical candidate whose prefix holds is selected (`tryExtension`, 4320 to 4325),
    /// whatever the implicit scope holds beside it; otherwise the implicit scope's candidates are
    /// searched (`inferView`, 4336 to 4355), and where none applies the lexical failure is the
    /// error: the first candidate that failed applied for its errors, or the imports' ambiguity
    /// reported.
    #[allow(clippy::too_many_arguments)]
    fn extensions_of_scopes(
        &mut self,
        lexical: Lexical,
        givens: Vec<(super::implicits::GivenRef, TypeId)>,
        contextual: usize,
        levels: Vec<u32>,
        companions: Vec<SymId>,
        recv: TExprId,
        recv_ty: TypeId,
        name: Name,
        targs: Option<ListRef>,
        lists: &mut Option<Vec<ArgList>>,
        span: Span,
        expected: Option<TypeId>,
    ) -> Option<(TExprId, TypeId)> {
        let lexical = match lexical {
            // The prefix is tried where it decides between the scopes: beside a candidate of the
            // implicit scope, or under a member's retry, whose conversion the extension's
            // selection rules out. Alone, the candidate is applied and its application resolves
            // the givens. Under a member's retry the receiver is taken as it is, as `f(qual)`
            // types it (4285), not through a conversion.
            Lexical::Found(call) => {
                let retry = self.lexical_selected.is_some();
                if retry && !self.receiver_fits(&call, recv_ty) {
                    // Applied for its errors, not selected: the retry tries a conversion after it.
                    return self.apply_extension(call, targs, lists, span, expected);
                }
                let alone = givens.is_empty() && companions.is_empty() && !retry;
                if alone || self.prefix_holds(&call, recv_ty, targs, lists, span) {
                    Lexical::Selected(call)
                } else {
                    Lexical::Failed(Some(call))
                }
            }
            other => other,
        };
        if let Lexical::Selected(call) = lexical {
            if self.lexical_selected.is_some() {
                self.lexical_selected = Some(true);
            }
            // The application of the extension, one deeper, takes what the prefix resolved.
            self.attempts.inferred_for = Some((call.sym, self.app_depth + 1));
            let applied = self.apply_extension(call, targs, lists, span, expected);
            self.attempts.inferred.clear();
            self.attempts.inferred_for = None;
            return applied;
        }
        // The givens in scope first, the implicit scope's where none provides the extension
        // (`bestImplicit`, Implicits.scala 1653 to 1681), each set ranked (`rank`).
        let in_scope: Vec<((super::implicits::GivenRef, TypeId), u32)> = givens[..contextual].iter().copied().zip(levels).collect();
        match self.rank_extension_givens(&in_scope, name, recv, recv_ty, lists, expected, span) {
            Ranked::One(i, probe) => {
                let ((given, given_ty), _) = in_scope[i];
                if let Some(call) = self.given_extension_for(given, given_ty, name, recv, recv_ty, lists, expected, span, probe) {
                    self.attribute(given.0);
                    return self.apply_extension(call, targs, lists, span, expected);
                }
            }
            Ranked::Ambiguous(a, b) => {
                let msg = self.ambiguous_givens_message(in_scope[a].0 .0 .0, in_scope[b].0 .0 .0, name, recv_ty);
                return self.ambiguous_extensions(msg, lists, span);
            }
            Ranked::None => {}
        }
        let from_companion = self.pick_extension(companions, recv, recv_ty, lists, true, expected);
        let beyond: Vec<((super::implicits::GivenRef, TypeId), u32)> = givens[contextual..].iter().map(|&g| (g, 0)).collect();
        let from_given = match self.rank_extension_givens(&beyond, name, recv, recv_ty, lists, expected, span) {
            Ranked::One(i, probe) => Some((beyond[i].0, probe)),
            Ranked::Ambiguous(a, b) => {
                let msg = self.ambiguous_givens_message(beyond[a].0 .0 .0, beyond[b].0 .0 .0, name, recv_ty);
                return self.ambiguous_extensions(msg, lists, span);
            }
            Ranked::None => None,
        };
        // A companion's extension beside a given's: the two extension methods compared, as
        // `disambiguate` compares two extension results (Implicits.scala 1411 to 1438), the more
        // specific receiver preferred; the given tried first in an attempt of its own, set aside.
        let from_given = match (&from_companion, from_given) {
            (Some(companion), Some(((given, given_ty), probe))) => {
                let probe = probe.or_else(|| {
                    let (calls, aside) = self.probe_given_extension(given, given_ty, name, recv, recv_ty, span)?;
                    Some((self.choose_given_extension(calls, recv, recv_ty, lists, expected)?, aside))
                });
                match probe {
                    Some((g, aside)) if self.has_more_specific_receiver(&g, companion) => {
                        self.given_extension_for(given, given_ty, name, recv, recv_ty, lists, expected, span, Some((g, aside))).map(|c| (c, given.0))
                    }
                    Some((g, _)) if !self.has_more_specific_receiver(companion, &g) => {
                        let Owner::Class(module) = self.syms.sym(companion.sym).owner else { unreachable!() };
                        let msg = format!(
                            "ambiguous extension methods: both {} and {} provide {} on {}",
                            self.name_str(self.syms.class(module).name),
                            self.name_str(self.syms.sym(given.0).name),
                            self.name_str(name),
                            self.show(recv_ty)
                        );
                        return self.ambiguous_extensions(msg, lists, span);
                    }
                    _ => None,
                }
            }
            (_, Some(((given, given_ty), probe))) => {
                self.given_extension_for(given, given_ty, name, recv, recv_ty, lists, expected, span, probe).map(|c| (c, given.0))
            }
            (_, None) => None,
        };
        match (from_given, from_companion) {
            (Some((call, given)), _) => {
                self.attribute(given);
                self.apply_extension(call, targs, lists, span, expected)
            }
            (None, Some(call)) => self.apply_extension(call, targs, lists, span, expected),
            (None, None) => match lexical {
                Lexical::Failed(Some(call)) => self.apply_extension(call, targs, lists, span, expected),
                Lexical::Ambiguous((a, a_module), (b, b_module)) => {
                    let msg = format!(
                        "ambiguous extension methods: both {} and {} are possible expansions of {} on {}",
                        self.extension_path(a.sym, a_module),
                        self.extension_path(b.sym, b_module),
                        self.name_str(name),
                        self.show(recv_ty)
                    );
                    self.ambiguous_extensions(msg, lists, span)
                }
                _ => None,
            },
        }
    }

    /// What dotty's `rank` (Implicits.scala 1466 to 1531) makes of the givens of `cands`, each with
    /// its nesting level, that provide an extension called `name` taking the receiver and, where
    /// the given has several, applying to the arguments: none, the one retained, or the two of an
    /// ambiguity nothing healed. The candidates are tried in order, each in an attempt of its own,
    /// set aside, as dotty keeps a success's state (`SearchSuccess.tstate`): a new success against
    /// the one retained is `disambiguate`d (1406 to 1447: `compareAlternatives`, 1369, the deeper
    /// nesting, then `compare_givens`, and where that ties, the two extension methods compared, the
    /// more specific receiver preferred); a candidate strictly worse than the one retained is not
    /// tried; after an ambiguity only a candidate strictly better than both by
    /// `compareAlternatives` is (`healAmbiguous`), so a narrower receiver alone does not heal it.
    /// The retained one's state is returned for its application, so that what its instantiation
    /// ran, a transparent given's macro among it, runs once. A single candidate is the caller's.
    #[allow(clippy::too_many_arguments)]
    fn rank_extension_givens(
        &mut self,
        cands: &[((super::implicits::GivenRef, TypeId), u32)],
        name: Name,
        recv: TExprId,
        recv_ty: TypeId,
        lists: &mut Option<Vec<ArgList>>,
        expected: Option<TypeId>,
        span: Span,
    ) -> Ranked {
        match cands.len() {
            0 => return Ranked::None,
            1 => return Ranked::One(0, None),
            _ => {}
        }
        // `compareAlternatives`: positive where `a` is preferred over `b` by their nesting and the
        // givens themselves.
        let by_givens = |w: &mut Self, a: usize, b: usize| match cands[a].1.cmp(&cands[b].1) {
            std::cmp::Ordering::Greater => 1,
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Equal => w.compare_givens(cands[a].0 .0 .0, cands[b].0 .0 .0),
        };
        let mut found: Option<(usize, MethodCall, super::state::SetAside)> = None;
        // The ambiguities being healed: a candidate has to be strictly better than each of them.
        let mut ambiguous: Vec<(usize, usize)> = Vec::new();
        for (i, &((given, given_ty), _)) in cands.iter().enumerate() {
            if ambiguous.iter().any(|&(x, y)| by_givens(self, i, x) <= 0 || by_givens(self, i, y) <= 0) {
                continue;
            }
            if found.as_ref().is_some_and(|f| by_givens(self, f.0, i) > 0) {
                continue;
            }
            let Some((calls, aside)) = self.probe_given_extension(given, given_ty, name, recv, recv_ty, span) else { continue };
            // The arguments choose among a given's overloads, outside its attempt, as dotty's
            // overload resolution of `f(qual)` against the call's prototype; none chosen, it fails.
            let Some(call) = self.choose_given_extension(calls, recv, recv_ty, lists, expected) else { continue };
            let Some((b, best, best_aside)) = found.take() else {
                found = Some((i, call, aside));
                continue;
            };
            let diff = match by_givens(self, b, i) {
                0 if self.has_more_specific_receiver(&best, &call) => 1,
                0 if self.has_more_specific_receiver(&call, &best) => -1,
                d => d,
            };
            match diff {
                d if d > 0 => found = Some((b, best, best_aside)),
                d if d < 0 => found = Some((i, call, aside)),
                _ => ambiguous.push((b, i)),
            }
        }
        match (found, ambiguous.last()) {
            (Some((b, call, aside)), _) => Ranked::One(b, Some((call, aside))),
            (None, Some(&(x, y))) => Ranked::Ambiguous(x, y),
            (None, None) => Ranked::None,
        }
    }

    /// The calls of the extension `name` that the given provides for the receiver, without the
    /// arguments: an attempt of their own, set aside where there is one, retracted where not.
    #[allow(clippy::too_many_arguments)]
    fn probe_given_extension(
        &mut self,
        given: super::implicits::GivenRef,
        given_ty: TypeId,
        name: Name,
        recv: TExprId,
        recv_ty: TypeId,
        span: Span,
    ) -> Option<(Vec<MethodCall>, super::state::SetAside)> {
        let mark = self.attempt();
        let calls = self.given_extension_calls(given, given_ty, name, recv, recv_ty, span);
        if calls.is_empty() {
            self.retract(mark);
            return None;
        }
        Some((calls, self.set_aside(mark)))
    }

    /// The one of a given's calls of an extension that the arguments choose; none where they
    /// choose none of several.
    fn choose_given_extension(&mut self, mut calls: Vec<MethodCall>, recv: TExprId, recv_ty: TypeId, lists: &mut Option<Vec<ArgList>>, expected: Option<TypeId>) -> Option<MethodCall> {
        if calls.len() > 1 {
            if let Some(l) = lists {
                let chosen = self.pick_by_arguments(&calls, recv, recv_ty, l, expected)?;
                return Some(calls.swap_remove(chosen));
            }
        }
        Some(calls.swap_remove(0))
    }

    /// The call of the extension `name` that the given provides for the arguments `lists`: its
    /// probe's, put back, where one was made, otherwise made now.
    #[allow(clippy::too_many_arguments)]
    fn given_extension_for(
        &mut self,
        given: super::implicits::GivenRef,
        given_ty: TypeId,
        name: Name,
        recv: TExprId,
        recv_ty: TypeId,
        lists: &mut Option<Vec<ArgList>>,
        expected: Option<TypeId>,
        span: Span,
        probe: Option<(MethodCall, super::state::SetAside)>,
    ) -> Option<MethodCall> {
        if let Some((call, aside)) = probe {
            self.restore(aside);
            return Some(call);
        }
        self.given_extension_call(given, given_ty, name, recv, recv_ty, lists, expected, span)
    }

    fn ambiguous_givens_message(&mut self, a: SymId, b: SymId, name: Name, recv_ty: TypeId) -> String {
        format!(
            "ambiguous extension methods: both {} and {} provide {} on {}",
            self.name_str(self.syms.sym(a).name),
            self.name_str(self.syms.sym(b).name),
            self.name_str(name),
            self.show(recv_ty)
        )
    }

    /// Reports an ambiguity among extensions, the arguments typed for their own errors.
    fn ambiguous_extensions(&mut self, msg: String, lists: &mut Option<Vec<ArgList>>, span: Span) -> Option<(TExprId, TypeId)> {
        self.error(span, msg);
        if let Some(l) = lists.take() {
            self.type_args_for_errors(&l);
        }
        Some((self.prog.add(TExpr::Unit), ERROR))
    }

    /// An extension method by the name of the object it is imported from or of its owner,
    /// `B.pick`.
    fn extension_path(&self, sym: SymId, module: Option<ClassId>) -> String {
        match (module, self.syms.sym(sym).owner) {
            (Some(c), _) | (None, Owner::Class(c)) => format!("{}.{}", self.name_str(self.syms.class(c).name), self.name_str(self.syms.sym(sym).name)),
            _ => self.name_str(self.syms.sym(sym).name),
        }
    }

    /// dotty's `tryExtension` over imports of one scope that each bring an extension of the
    /// name (Typer.scala 4286 to 4308): each import's candidate tried alone, by its prefix, an
    /// inherited one on the object it is imported from; one success is selected, two are an
    /// ambiguity, none a failure, the first's. An import whose overloads the arguments do not
    /// decide between fails, as dotty's overload resolution of `f(qual)` against the call fails.
    #[allow(clippy::too_many_arguments)]
    fn lexical_among_imports(
        &mut self,
        alternatives: Vec<super::resolve::ImportAlternative>,
        recv: TExprId,
        recv_ty: TypeId,
        targs: Option<ListRef>,
        lists: &mut Option<Vec<ArgList>>,
        span: Span,
        expected: Option<TypeId>,
    ) -> Lexical {
        let retry = self.lexical_selected.is_some();
        let mut held: Vec<(MethodCall, Option<ClassId>, Vec<super::state::Inferred>)> = Vec::new();
        let mut failed = None;
        for alt in alternatives {
            // An extension the object inherits is called on it, as the implicit scope's are
            // (`extension_module_at`).
            let mark = self.ext_modules.len();
            if let Some(m) = alt.module {
                for &s in &alt.syms {
                    if matches!(self.syms.sym(s).owner, Owner::Class(c) if c != m) {
                        self.ext_modules.push((s, m));
                    }
                }
            }
            self.ext_undecided = false;
            let call = self.pick_extension(alt.syms, recv, recv_ty, lists, alt.module.is_some(), expected);
            self.ext_modules.truncate(mark);
            let undecided = std::mem::take(&mut self.ext_undecided);
            let Some(call) = call else { continue };
            if !undecided && (!retry || self.receiver_fits(&call, recv_ty)) && self.prefix_holds(&call, recv_ty, targs, lists, span) {
                held.push((call, alt.module, std::mem::take(&mut self.attempts.inferred)));
            } else if failed.is_none() {
                failed = Some(call);
            }
        }
        match held.len() {
            0 => Lexical::Failed(failed),
            1 => {
                let (call, _, inferred) = held.pop().unwrap();
                self.attempts.inferred = inferred;
                Lexical::Selected(call)
            }
            _ => {
                let mut held = held.into_iter().map(|(call, module, _)| (call, module));
                let (a, b) = (held.next().unwrap(), held.next().unwrap());
                Lexical::Ambiguous(a, b)
            }
        }
    }

    /// Whether the extension `call`'s receiver clause takes `recv_ty`: what dotty's `tryExtension`
    /// types as `f(qual)` (Typer.scala 4285), which selects the extension under a member's retry.
    /// Nothing else is resolved ahead of the application.
    fn receiver_fits(&mut self, call: &MethodCall, recv_ty: TypeId) -> bool {
        let sig = self.sig_arc(call.sym);
        let mark = self.snapshot();
        let mut subst = call.owner_subst.clone();
        for &tp in &sig.tparams {
            let v = self.fresh_var();
            subst.push((tp, v));
        }
        let ext_clauses = self.syms.sym(call.sym).ext_clauses as usize;
        let fits = match sig.clauses.iter().take(ext_clauses).position(|c| !c.is_using) {
            Some(idx) => {
                let pty = self.types.subst(sig.clauses[idx].params[0].ty, &subst);
                self.is_sub(recv_ty, pty)
            }
            None => true,
        };
        self.rollback(mark);
        fits
    }

    /// dotty's prefix of an extension's application, `extMethodApply` (Applications.scala 2969 to
    /// 2988): `f(qual)` typed against the rest of the call as its prototype, the receiver applied
    /// and the using clauses before the method's own parameters resolved (`adaptNoArgs`, as the
    /// prototype is no `using` application of theirs), none where the call's next list is an
    /// explicit `using` one, and none of the method's own where the call writes type arguments for
    /// its own type parameters. Whether every given among them has an instance, one whose type the
    /// receiver leaves open searched as it stands, as `adaptNoArgs` searches it: the instance fixes
    /// the open variables, which its application then takes, and none or two of them fail the
    /// prefix (`import-generic-prefix-fallback`: scalac takes the other import's). A using clause
    /// after the method's parameters is the application's, resolved after its arguments: its
    /// missing or ambiguous given is the selected extension's error, not a reason to select
    /// another (scalac reports both), where without an explicit list of the method's every using
    /// clause is the prefix's (scalac takes the companion's).
    ///
    /// The prefix is an attempt, retracted: each given it resolves is set aside in an attempt of
    /// its own (`attempts.inferred`) and put back where the application takes it (`inferred_hit`),
    /// so that a given, an inline one's expansion and its warnings among them, is resolved once;
    /// one the application does not take goes.
    fn prefix_holds(&mut self, call: &MethodCall, recv_ty: TypeId, targs: Option<ListRef>, lists: &Option<Vec<ArgList>>, span: Span) -> bool {
        let sig = self.sig_arc(call.sym);
        let info = self.syms.sym(call.sym);
        let (ext_clauses, ext_tparams) = (info.ext_clauses as usize, info.ext_tparams as usize);
        let receiver = sig.clauses.iter().take(ext_clauses).position(|c| !c.is_using);
        let mut leading = match receiver {
            Some(idx) => sig.clauses.iter().skip(idx + 1).position(|c| !c.is_using).map_or(sig.clauses.len(), |k| idx + 1 + k),
            None => sig.clauses.len(),
        };
        // An explicit `using` list of the call is the prototype's application of the clauses
        // after the receiver (`matchingApply`), which the prefix leaves to it; type arguments the
        // call writes for the method's own type parameters are its `PolyProto`, which leaves the
        // method's clauses unapplied (scalac reports the missing given of the selected
        // extension).
        if lists.as_deref().and_then(|l| l.first()).is_some_and(|l| l.using) {
            leading = receiver.map_or(0, |idx| idx);
        } else if targs.is_some() && sig.tparams.len() > ext_tparams {
            leading = leading.min(ext_clauses);
        }
        let searched: Vec<usize> = (0..leading).filter(|&k| sig.clauses[k].is_using).collect();
        if searched.is_empty() {
            return true;
        }
        // The receiver as the candidate takes it: converted where `pick_extension` converted it.
        let recv_ty = call.ext_recv.map_or(recv_ty, |(_, t)| t);
        let mark = self.attempt();
        let mut subst = call.owner_subst.clone();
        for &tp in &sig.tparams {
            let v = self.fresh_var();
            subst.push((tp, v));
        }
        if let Some(idx) = receiver {
            let pty = self.types.subst(sig.clauses[idx].params[0].ty, &subst);
            self.is_sub(recv_ty, pty);
        }
        // A plain inline given resolved here is registered for the later phase with its attempt
        // (`try_given`), not expanded: the set-aside takes the registration, the application that
        // takes the given puts it back, and a candidate given up drops it, so that its macro runs
        // once, for the candidate selected, where scalac's `Inlining` phase runs it; an error of
        // its expansion is the selected extension's, no reason to take another.
        let mut hold = true;
        let mut resolved = Vec::new();
        // The paths of the givens resolved so far, put into the types of the parameters that name
        // them (`using z: Z[x.type]`), as the application puts the arguments' paths
        // (`param_type`), so that the application finds the given it takes under the same target.
        let mut paths: Vec<(SymId, TypeId)> = Vec::new();
        'clauses: for &k in &searched {
            for p in &sig.clauses[k].params {
                let target = self.types.subst(p.ty, &subst);
                let target = if paths.is_empty() || !self.types.has_paths(target) { target } else { self.subst_paths(target, &paths) };
                let target = self.solve_bounded_in(target);
                // An open target's given is the application's by its parameter: the application's
                // target has variables of its own.
                let open = self.types.has_vars(target).then_some(p.sym);
                let one = self.attempt();
                match self.resolve_given_typed(target, span) {
                    Some((te, ty)) => {
                        let zonked = self.zonk(target);
                        paths.push((p.sym, self.path_of(te).unwrap_or(ty)));
                        let aside = self.set_aside(one);
                        // What the search fixed of an open target holds for the clauses after it
                        // (`(using Need[T])(using Closed[T])`: `Closed[Int]`), within the prefix's
                        // attempt, which retracts it; the given's own work stays set aside for the
                        // application.
                        if open.is_some() {
                            self.is_sub(ty, target);
                        }
                        resolved.push((zonked, open, te, ty, aside));
                    }
                    None => {
                        self.retract(one);
                        hold = false;
                        break 'clauses;
                    }
                }
            }
        }
        self.given_ambiguity = None;
        self.retract(mark);
        if hold {
            self.attempts.inferred = resolved;
        }
        hold
    }

    fn apply_extension(
        &mut self,
        call: MethodCall,
        targs: Option<ListRef>,
        lists: &mut Option<Vec<ArgList>>,
        span: Span,
        expected: Option<TypeId>,
    ) -> Option<(TExprId, TypeId)> {
        let mut l = lists.take().unwrap();
        if self.drops_java_parens(call.sym, &l) {
            l.remove(0);
        }
        self.attribute_extension(call.sym);
        let sym = call.sym;
        let applied = self.apply_method(call, None, targs, l, span, expected, false);
        // After the application, so that the outermost one an argument's typing nests is read.
        self.applied_extension = Some(sym);
        applied
    }

    /// The extension methods called `name` of the objects in the implicit scope of `t`. One an
    /// object inherits from a trait is called on that object, which `ext_modules` records
    /// until the caller truncates it.
    pub(super) fn implicit_scope_extensions(&mut self, t: TypeId, name: Name) -> Vec<SymId> {
        let mut out = Vec::new();
        for m in self.implicit_scope_objects(t) {
            let start = out.len();
            self.module_extensions(m, name, &mut out);
            // A private or protected extension is a candidate where its member is reachable.
            let mut k = start;
            while k < out.len() {
                if self.is_given_accessible(out[k]) {
                    k += 1;
                } else {
                    out.remove(k);
                }
            }
            for &s in &out[start..] {
                if let Owner::Class(c) = self.syms.sym(s).owner {
                    if c != m {
                        self.ext_modules.push((s, m));
                    }
                }
            }
        }
        out
    }

    /// The object of the implicit scope the `i`-th of `candidates`, an extension inherited from
    /// a trait, was found in. One method reached through two objects (`Html` and `Svg` of one
    /// trait) is two candidates in the order the search pushed their objects, which are the last
    /// entries for it.
    fn extension_module_at(&self, candidates: &[SymId], i: usize) -> Option<ClassId> {
        let i = i % candidates.len();
        let sym = candidates[i];
        let total = candidates.iter().filter(|&&s| s == sym).count();
        let before = candidates[..i].iter().filter(|&&s| s == sym).count();
        let found: Vec<ClassId> = self.ext_modules.iter().filter(|&&(s, _)| s == sym).map(|&(_, m)| m).collect();
        found.len().checked_sub(total).map(|start| found[start + before])
    }

    /// Among the applicable candidates of the nearest scope that has any, the one with the most
    /// specific receiver, whatever its own clauses take. Where several receivers are equally
    /// specific, the argument lists that follow decide among every applicable candidate, their
    /// arity and auto-tupling included, as dotc deepens the prototype of an ambiguous overload;
    /// failing that, the first. One named without arguments is taken before one that would be
    /// eta-expanded.
    fn pick_extension(
        &mut self,
        mut candidates: Vec<SymId>,
        recv: TExprId,
        recv_ty: TypeId,
        arg_lists: &mut Option<Vec<ArgList>>,
        implicit_scope: bool,
        expected: Option<TypeId>,
    ) -> Option<MethodCall> {
        let lists: &[ArgList] = arg_lists.as_deref().unwrap_or(&[]);
        let without_args = !lists.iter().any(|l| !l.using);
        // The shape of the call ranks the overloads: arguments go to one that takes them
        // before the result of a parameterless one is applied to them.
        if candidates.len() > 1 && without_args {
            candidates.sort_by_key(|&sym| !self.is_parameterless_extension(sym));
        }
        let builtin_recv = self.is_builtin_type(recv_ty);
        // A std extension of a builtin type stands for a member of the class, which only
        // arguments it takes choose, as written before tupled, and which gives way to the
        // extensions when none takes them: those come again, as a group of their own, for
        // several arguments tupled.
        let strict_count = candidates.len();
        let several_args = lists.iter().find(|l| !l.using).map_or(false, |l| l.args.len() > 1);
        if builtin_recv && several_args {
            let members: Vec<SymId> = candidates.iter().copied().filter(|&s| self.is_std_extension(s)).collect();
            candidates.extend(members);
        }
        // The applicable candidates of the group, `best` and `ties` indices into them; the ties
        // are those whose receiver is as specific as that of the best one.
        let mut pool: Vec<MethodCall> = Vec::new();
        let mut best: Option<((bool, bool, usize), usize)> = None;
        let mut ties: Vec<usize> = Vec::new();
        let all = candidates.clone();
        for (i, sym) in candidates.into_iter().enumerate() {
            let scope = match () {
                _ if implicit_scope => 0,
                _ if builtin_recv && self.is_std_extension(sym) => usize::MAX,
                _ => self.extension_scope_level(sym),
            };
            let tupled = i >= strict_count;
            let group = (tupled, without_args && self.is_parameterless_extension(sym), scope);
            if best.map_or(false, |(found, _)| found != group) {
                break;
            }
            let mut owner_subst = Vec::new();
            let mut prefix = None;
            let owner = match self.syms.sym(sym).owner {
                Owner::Class(c) if self.syms.class(c).kind == ClassKind::Object => {
                    Some(self.prog.add(TExpr::Module(c)))
                }
                Owner::Class(c) => {
                    let module = if implicit_scope { self.extension_module_at(&all[..strict_count], i) } else { None };
                    let site = match module {
                        Some(m) => super::exports::TraitMemberSite::Module(m),
                        None => self.trait_member_site(sym, c),
                    };
                    owner_subst = self.trait_member_subst(site, c);
                    prefix = self.inherited_extension_prefix(module);
                    Some(self.trait_member_receiver(site))
                }
                _ => None,
            };
            // The receiver ranks the extensions before the lists after it, whatever their own
            // clauses take: a receiver more specific than the rest wins even where its
            // arguments are then wrong, as under scalac, and the lists decide between equally
            // specific ones below.
            if builtin_recv && self.is_std_extension(sym) && !self.extension_accepts(sym, lists, tupled) {
                continue;
            }
            let mark = self.snapshot();
            let call = MethodCall { recv: owner, sym, owner_subst, ext_recv: Some((recv, recv_ty)), prefix };
            let applicable = self.extension_applicable(&call, recv_ty);
            self.rollback(mark);
            if !applicable {
                continue;
            }
            pool.push(call);
            let k = pool.len() - 1;
            match best {
                // Arguments go to an overload that takes them before the result of a
                // parameterless one is applied to them, where its receiver is as specific and
                // its own clause takes as many.
                Some((_, b))
                    if !without_args
                        && self.is_parameterless_extension(pool[b].sym)
                        && !self.is_parameterless_extension(pool[k].sym)
                        && !self.has_more_specific_receiver(&pool[b], &pool[k])
                        && self.extension_accepts(pool[k].sym, lists, true) =>
                {
                    ties.clear();
                    best = Some((group, k));
                }
                Some((_, b)) if !self.has_more_specific_receiver(&pool[k], &pool[b]) => {
                    // One that takes arguments is no overload of one that takes none: the
                    // first of the two stays, as the shape of the call has chosen it.
                    let alike = pool[b].sym != pool[k].sym
                        && self.is_parameterless_extension(pool[b].sym) == self.is_parameterless_extension(pool[k].sym);
                    if alike && !self.receiver_class_derives(pool[b].sym, pool[k].sym) && !self.has_more_specific_receiver(&pool[b], &pool[k]) {
                        ties.push(k);
                    }
                }
                _ => {
                    ties.clear();
                    best = Some((group, k));
                }
            }
        }
        if let Some((_, b)) = best {
            if ties.is_empty() {
                return Some(pool.swap_remove(b));
            }
            let Some(l) = arg_lists else { return Some(pool.swap_remove(b)) };
            let parameterless = self.is_parameterless_extension(pool[b].sym);
            let first = pool.swap_remove(b);
            let mut calls = vec![first];
            for call in pool {
                if self.is_parameterless_extension(call.sym) == parameterless && !calls.iter().any(|c| c.sym == call.sym) {
                    calls.push(call);
                }
            }
            // Failing a choice by the arguments, the first whose own clauses take them.
            let chosen = match self.pick_by_arguments(&calls, recv, recv_ty, l, expected) {
                Some(i) => i,
                None => {
                    self.ext_undecided = true;
                    (0..calls.len()).find(|&i| self.extension_accepts(calls[i].sym, l, false)).unwrap_or(0)
                }
            };
            return Some(calls.swap_remove(chosen));
        }
        // A number widens to the receiver type of an extension as it would to a parameter
        // (`i.showLong` for an `extension (n: Long)`), once nothing takes it as it is.
        let from = self.deref(recv_ty);
        let from = self.widen_lit(from);
        let wide = [self.b.t_int, self.b.t_long, self.b.t_float, self.b.t_double].into_iter().find(|&w| self.numeric_widening(from, w));
        if let Some(wide) = wide {
            let recorded = self.prog.type_or_aside(recv);
            let widened = self.widen_numeric(recv, from, wide).unwrap_or(recv);
            let picked = self.pick_extension(all, widened, wide, arg_lists, implicit_scope, expected);
            if picked.is_none() {
                self.prog.restore_type(recv, recorded);
            }
            return picked;
        }
        // As the receiver is an argument of the extension, a conversion may take it to the
        // receiver type, as scalac's adaptation of the argument does.
        if self.converting_receiver || self.types.contains_error(recv_ty) {
            return None;
        }
        for (j, &sym) in all.iter().enumerate() {
            let module = if implicit_scope { self.extension_module_at(&all[..strict_count], j) } else { None };
            let prefix = self.inherited_extension_prefix(module);
            let call = MethodCall { recv: None, sym, owner_subst: Vec::new(), ext_recv: None, prefix };
            // The receiver converted for each extension is an attempt of its own.
            let mark = self.attempt();
            let Some(pty) = self.receiver_param_type(&call, true) else {
                self.retract(mark);
                continue;
            };
            // A receiver type over the extension's own parameters (`IterableOnce[A]` for an
            // `Array[String]`) takes what the conversion's result fixes them to.
            let head = self.deref(pty);
            if matches!(self.types.get(head), Type::Var(_) | Type::AppVar(..)) {
                self.retract(mark);
                continue;
            }
            self.converting_receiver = true;
            let converted = self.convert_to(recv, recv_ty, pty, Span::default(), false);
            self.converting_receiver = false;
            if let Some(converted) = converted {
                if converted != recv {
                    let pty = self.solve_bounded_in(pty);
                    if !self.types.has_vars(pty) {
                        if let Some(call) = self.pick_extension(all.clone(), converted, pty, arg_lists, implicit_scope, expected) {
                            self.close(mark);
                            return Some(call);
                        }
                    }
                }
            }
            self.retract(mark);
        }
        None
    }

    /// The call of the extension method `name` that the given provides for `recv`, when it has
    /// one that takes the receiver; the given is instantiated for it.
    #[allow(clippy::too_many_arguments)]
    fn given_extension_call(
        &mut self,
        given: super::implicits::GivenRef,
        given_ty: TypeId,
        name: Name,
        recv: TExprId,
        recv_ty: TypeId,
        lists: &mut Option<Vec<ArgList>>,
        expected: Option<TypeId>,
        span: Span,
    ) -> Option<MethodCall> {
        let mut calls = self.given_extension_calls(given, given_ty, name, recv, recv_ty, span);
        if calls.len() > 1 {
            let chosen = match lists {
                Some(l) => self.pick_by_arguments(&calls, recv, recv_ty, l, expected).unwrap_or(0),
                None => 0,
            };
            return Some(calls.swap_remove(chosen));
        }
        calls.pop()
    }

    /// The calls of the extension methods `name` that the given provides for `recv`: those of the
    /// first of its class's bases that has one taking the receiver, overloads for the arguments to
    /// choose between; the given instantiated for each. An extension whose receiver's class rules
    /// the receiver out is passed over before the given is instantiated, as dotty's search does not
    /// type a candidate its selection rules out: a transparent given's expansion runs no macro for
    /// it.
    fn given_extension_calls(
        &mut self,
        given: super::implicits::GivenRef,
        given_ty: TypeId,
        name: Name,
        recv: TExprId,
        recv_ty: TypeId,
        span: Span,
    ) -> Vec<MethodCall> {
        let Some(gc) = self.class_of(given_ty) else { return Vec::new() };
        self.complete_class(gc);
        let bases: Vec<(ClassId, TypeId)> = self.syms.class(gc).base_types.clone();
        for (b, _) in bases {
            let exts: Vec<SymId> =
                self.syms.class(b).extensions.iter().copied().filter(|&s| self.syms.sym(s).name == name).collect();
            // Several of one class that take the receiver are overloads: the arguments decide.
            let mut calls: Vec<MethodCall> = Vec::new();
            for ext in exts {
                if self.receiver_class_rules_out(ext, recv_ty) {
                    continue;
                }
                // The given instantiated for each extension is an attempt of its own.
                let mark = self.attempt();
                let first_var = self.tvars.len();
                let Some((instance, inst_ty)) = self.instantiate_given_with(given, None, span, false) else {
                    self.retract(mark);
                    continue;
                };
                let owner_subst = match self.base_type(inst_ty, b) {
                    Some(bt) => self.owner_subst(bt),
                    None => Vec::new(),
                };
                let call = MethodCall { recv: Some(instance), sym: ext, owner_subst, ext_recv: Some((recv, recv_ty)), prefix: None };
                if !self.extension_applicable(&call, recv_ty) {
                    self.retract(mark);
                    continue;
                }
                // The receiver fixed the given's own type arguments (`E` in `Either[E, X]`).
                for v in first_var..self.tvars.len() {
                    self.solve_var(self.tvars.id(v));
                }
                self.close(mark);
                let owner_subst: Subst = call.owner_subst.iter().map(|&(p, t)| (p, self.zonk(t))).collect();
                calls.push(MethodCall { owner_subst, ..call });
            }
            if !calls.is_empty() {
                return calls;
            }
        }
        Vec::new()
    }

    /// Whether an extension `apply` in lexical scope or in the implicit scope of `t` takes it
    /// as its receiver, which `f()` applies to the result of a parameterless `f`.
    fn has_apply_extension(&mut self, t: TypeId) -> bool {
        if self.types.contains_error(t) {
            return false;
        }
        let lexical = self.lexical_extensions(names::APPLY);
        let mark = self.ext_modules.len();
        let from_scope = self.implicit_scope_extensions(t, names::APPLY);
        let modules: Vec<Option<ClassId>> = (0..from_scope.len()).map(|i| self.extension_module_at(&from_scope, i)).collect();
        let candidates: Vec<(SymId, Option<ClassId>)> = lexical.into_iter().map(|s| (s, None)).chain(from_scope.into_iter().zip(modules)).collect();
        self.ext_modules.truncate(mark);
        let declared = candidates.into_iter().any(|(sym, module)| {
            let prefix = self.inherited_extension_prefix(module);
            let owner_subst = match (module, self.syms.sym(sym).owner) {
                (Some(m), Owner::Class(c)) => self.trait_member_subst(super::exports::TraitMemberSite::Module(m), c),
                _ => Vec::new(),
            };
            let call = MethodCall { recv: None, sym, owner_subst, ext_recv: None, prefix };
            let mark = self.snapshot();
            let applicable = self.extension_applicable(&call, t);
            self.rollback(mark);
            applicable
        });
        declared || self.given_provides_apply(t)
    }

    /// Whether a given in scope or in the implicit scope of `t` provides an extension `apply`
    /// that takes it (`given Ops[C]` with `extension (t: C) def apply()`).
    fn given_provides_apply(&mut self, t: TypeId) -> bool {
        let (givens, _) = self.givens_with_extension(names::APPLY, t);
        let recv = self.prog.add(TExpr::Unit);
        givens.into_iter().any(|(given, given_ty)| {
            // A probe: what it writes goes with it.
            let mark = self.attempt();
            let found = self.given_extension_call(given, given_ty, names::APPLY, recv, t, &mut None, None, Span::default()).is_some();
            self.retract(mark);
            found
        })
    }

    /// Whether an extension method of that name in lexical scope takes such a receiver.
    pub fn has_lexical_extension_for(&mut self, recv_ty: TypeId, name: Name) -> bool {
        for sym in self.lexical_extensions(name) {
            let call = MethodCall { recv: None, sym, owner_subst: Vec::new(), ext_recv: None, prefix: None };
            let mark = self.snapshot();
            let applicable = self.extension_applicable(&call, recv_ty);
            self.rollback(mark);
            if applicable {
                return true;
            }
        }
        false
    }

    /// Checks whether the receiver conforms to the extension's receiver parameter.
    pub(super) fn extension_applicable(&mut self, call: &MethodCall, recv_ty: TypeId) -> bool {
        if self.receiver_class_rules_out(call.sym, recv_ty) {
            return false;
        }
        match self.receiver_param_type(call, true) {
            Some(pty) => self.is_sub(recv_ty, pty),
            None => false,
        }
    }

    /// A receiver parameter of a class type takes instances of that class only, which settles
    /// most candidates without instantiating them.
    fn receiver_class_rules_out(&mut self, sym: SymId, recv_ty: TypeId) -> bool {
        let ext_clauses = self.syms.sym(sym).ext_clauses as usize;
        let Some(recv_param) = self.sig_of(sym).clauses.iter().take(ext_clauses).find(|c| !c.is_using).map(|c| c.params[0].ty) else { return false };
        let Type::Class(wanted, _) = self.types.get(recv_param) else { return false };
        let recv_ty = self.deref(recv_ty);
        let Type::Class(actual, _) = self.types.get(recv_ty) else { return false };
        let Some(info) = self.syms.class_done(actual) else { return false };
        let opaque = |k: ClassKind| k == ClassKind::Opaque;
        !opaque(info.kind)
            && !opaque(self.syms.class(wanted).kind)
            && !info.base_types.iter().any(|&(b, _)| b == wanted)
    }

    /// Whether the receiver parameters of `a` and `b` are of two classes of which the first
    /// derives from the second, which makes the receiver of `a` the more specific one without
    /// instantiating either.
    fn receiver_class_derives(&mut self, a: SymId, b: SymId) -> bool {
        let class_of_receiver = |t: &mut Self, s: SymId| {
            let ext_clauses = t.syms.sym(s).ext_clauses as usize;
            let recv_param = t.sig_of(s).clauses.iter().take(ext_clauses).find(|c| !c.is_using)?.params[0].ty;
            match t.types.get(recv_param) {
                Type::Class(c, _) => Some(c),
                _ => None,
            }
        };
        let (Some(ca), Some(cb)) = (class_of_receiver(self, a), class_of_receiver(self, b)) else { return false };
        ca != cb && self.syms.class(ca).base_types.iter().any(|&(base, _)| base == cb)
    }

    /// The type of the receiver parameter, with the extension's type parameters either left as
    /// they are or replaced by fresh variables.
    /// The prefix an extension inherited from a trait is seen from, where the object of the
    /// implicit scope it comes from fixes the trait's types in its signature.
    pub(super) fn inherited_extension_prefix(&mut self, module: Option<ClassId>) -> Option<TypeId> {
        module.map(|m| self.types.class(m, &[]))
    }

    fn receiver_param_type(&mut self, call: &MethodCall, fresh: bool) -> Option<TypeId> {
        let sig = self.sig_arc(call.sym);
        let sig = match call.prefix {
            Some(prefix) => self.sig_seen_from(sig, prefix, call.sym),
            None => sig,
        };
        let ext_clauses = self.syms.sym(call.sym).ext_clauses as usize;
        let idx = sig.clauses.iter().take(ext_clauses).position(|c| !c.is_using)?;
        let mut subst = call.owner_subst.clone();
        if fresh {
            for &tp in &sig.tparams {
                let v = self.fresh_var();
                subst.push((tp, v));
            }
        }
        Some(self.types.subst(sig.clauses[idx].params[0].ty, &subst))
    }

    /// Whether every receiver of `a` is a receiver of `b` and not the other way round.
    fn has_more_specific_receiver(&mut self, a: &MethodCall, b: &MethodCall) -> bool {
        let covers = |t: &mut Self, general: &MethodCall, special: &MethodCall| {
            let (Some(wide), Some(narrow)) =
                (t.receiver_param_type(general, true), t.receiver_param_type(special, false))
            else {
                return false;
            };
            let mark = t.snapshot();
            let ok = t.is_sub(narrow, wide);
            t.rollback(mark);
            ok
        };
        covers(self, b, a) && !covers(self, a, b)
    }

    /// How near the scope is that a lexically visible extension comes from: 1 for one that is
    /// imported or defined in a package, more for one that an enclosing class defines. 0 is
    /// left to the extensions of the receiver's implicit scope.
    fn extension_scope_level(&self, sym: SymId) -> usize {
        let frame = match self.syms.sym(sym).owner {
            Owner::Class(owner) => {
                self.env.frames.iter().rposition(|f| matches!(f, super::Frame::Class(c) if *c == owner))
            }
            Owner::Local => self.env.frames.iter().rposition(|f| {
                matches!(f, super::Frame::Locals { names, .. } if names.iter().any(|&(_, s)| s == sym))
            }),
            Owner::Package(_) => None,
        };
        frame.map_or(1, |i| i + 2)
    }
}

pub(in crate::typer) enum StaticPath {
    Callee(Callee),
    Missing(PkgId),
    NotStatic,
}
