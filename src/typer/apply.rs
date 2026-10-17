use super::profile::{About, Kind, Outcome};
use super::resolve::TermRef;
use super::implicits::GivenScope;
use super::state::{ArgSlot, Cache, Dual};
use super::Worker;
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


/// dotty's `SelectionProto` of a qualifier's typing (`Typer.typedSelect`'s `shallowSelectionProto`,
/// Typer.scala 1081; an infix operation's left operand is one through `typedInfixOp`'s
/// `Desugar.binop`, 3707): the member `name` selected on the qualifier written at `span`, applied
/// to `args`. An overloaded reference at that span of that file without arguments is resolved
/// by it (`resolve_selected`, `resolveOverloaded1`).
#[derive(Clone, Copy)]
pub struct Selection {
    pub span: Span,
    pub name: Name,
    pub args: SelectionArgs,
    pub result: SelectionResult,
}

/// What a qualifier is typed under, its `SelectionProto` (`Worker::selections`): the values at
/// hand where the member is selected, from which a reader of the selection, the resolution of an
/// unapplied overloaded reference or a deeper round of it, derives the selection itself
/// (`Worker::selection_at`). Most qualifiers' selections are never read, and a frame costs their
/// typing no reading of the tree.
#[derive(Clone, Copy)]
pub struct SelectionFrame {
    pub file: FileId,
    /// The qualifier as written, its parentheses and braces read where the selection is.
    pub q: ExprId,
    pub name: Name,
    /// The application whose member is selected (`Member`), or the operation's other operand.
    pub at: ExprId,
    /// The span of the application or operation, which a selection on its result names.
    pub app: Span,
    pub expected: Option<TypeId>,
    pub kind: FrameKind,
}

#[derive(Clone, Copy, PartialEq)]
pub enum FrameKind {
    /// A member selected on `q`, applied to the lists of the application `at`.
    Member,
    /// An operator on `q`, its operand `at`; `more` where further plain lists apply it.
    Operand { more: bool },
    /// A member selected for its declared type, nothing expected of it.
    Bare,
}

/// What is expected of the selected member applied to its first argument list, or of the member
/// itself without one: dotty's `IgnoredProto(pt)` inside the member prototype, revealed by a later
/// round of the resolution (`deepenProto`).
/// One alternative of a member a deeper selection tests that has no symbol of its own
/// (`Worker::sig_alt`).
pub struct SigAlt {
    params: Option<Vec<Option<TypeId>>>,
    repeated: bool,
    result: Option<TypeId>,
}

#[derive(Clone, Copy)]
pub enum SelectionResult {
    /// Nothing, or what a second argument list applies: dotty's result prototype there tells no
    /// alternatives apart (`FunProto`'s `resultType` is no value type).
    Ignored,
    /// The expected type of the member's application.
    Type(TypeId),
    /// A selection on the member's result: the selection beneath this one on `Worker::selections`.
    Selected,
}

/// The arguments of the member a qualifier is selected for, which the resolution's deeper pass
/// reads (`deepenProto` of the `SelectionProto`'s `FunProto`).
#[derive(Clone, Copy)]
pub enum SelectionArgs {
    None,
    /// An infix operation's right operand.
    Operand(ExprId),
    /// The member's first list of plain arguments as written.
    List(ListRef),
}

/// An operand of a selected member that the deeper selection typed alone, as `FunProto.typedArgs`
/// types its arguments (ProtoTypes.scala 508, `typer.typed`): its typing, `None` where the typing
/// reported an error (`errorArgs`, not cached). Every candidate's test reads it, and the member's
/// application takes it from its argument cache (`arg_cache_open_member`) as `typedArg` takes a
/// cached typing (556), adapted to its formal: the operand is typed once.
#[derive(Clone, Copy)]
pub struct SelectionTyped {
    pub file: FileId,
    /// The span of the qualifier whose selection's operand it is (`Selection::span`), whose
    /// typing drops it once the member is applied (`drop_selection_typed`).
    pub owner: Span,
    pub expr: ExprId,
    pub typed: Option<(TExprId, TypeId)>,
    /// The literal type of a literal written as the operand, which the candidates' test reads
    /// (`pretype_value`).
    pub literal: Option<TypeId>,
}

/// How a bare `f` of a `def f()` is taken (`auto_applied`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum AutoApply {
    No,
    Silent,
    Warned,
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
/// (`value`, a named one's `v`) or typed already, the parameter it goes to and whether it is a
/// sequence spliced into it (`xs*`).
struct BoundaryArg {
    value: Option<ExprId>,
    pretyped: Option<(TExprId, TypeId)>,
    param: usize,
    splice: bool,
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

/// What a member's application under `apply_member_or_extension` keeps for its retry on the
/// qualifier: its argument lists as written, flat (every list's arguments in one buffer, and per
/// list where its arguments begin, whether it is a using one and its span); a call that types
/// keeps the buffers for the next (`Worker::member_calls`). The typings of its first plain
/// list's arguments are in the attempts' cache (`state::ArgCache`, dotty's `FunProto`).
pub struct MemberCall {
    args: Vec<ArgSrc>,
    lists: Vec<(u32, bool, Span)>,
    /// The depth of applications (`Worker::app_depth`) of the member's application, which the
    /// first application under it claims: an application nested in a later list's argument types
    /// its own later lists for their errors.
    depth: u32,
    /// Whether the application stopped at its first plain list, which failed
    /// (`apply_method_in_now`): the lists after it are typed once, by the retry that takes the
    /// application over or alone.
    stopped: bool,
}

impl MemberCall {
    /// Keeps `lists` as written.
    #[inline]
    fn keep(&mut self, lists: &[ArgList]) {
        self.args.clear();
        self.lists.clear();
        for l in lists {
            self.lists.push((self.args.len() as u32, l.using, l.span));
            self.args.extend_from_slice(&l.args);
        }
        self.depth = 0;
        self.stopped = false;
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

impl Default for MemberCall {
    fn default() -> MemberCall {
        MemberCall { args: Vec::new(), lists: Vec::new(), depth: 0, stopped: false }
    }
}

/// How a try of a member's retry ends before any extension or conversion is looked for.
enum Boundary {
    /// The member's errors stand for this try, as dotty has it: an argument is an error
    /// argument (`hasErrorArg`), or the member matches the arguments (`isMatchedBy`).
    Stands,
    /// The argument lists to apply an extension or a conversion to.
    Retry(Vec<ArgList>),
}

/// The tries of a member's retry, in dotty's order (Applications.scala 1494): the arguments as
/// written, separately, or tupled into one.
#[derive(Clone, Copy, PartialEq)]
enum RetryTry {
    Separate,
    Tupled,
}

pub enum Callee {
    /// `refined`: the member as the receiver's refinement declares it, which the call is typed
    /// by (`MethodCall::refined`).
    Method { recv: Option<TExprId>, sym: SymId, owner_subst: Subst, prefix: Option<TypeId>, refined: Option<Arc<MethodSig>> },
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
    /// The member's signature as a refinement of the receiver's type declares it, seen from the
    /// receiver, where it conforms to the class's member: the call is typed by it, its
    /// parameters' names and its result, and the class's member called (dotty's
    /// `Types.findMember` `goRefined`, the joint denotation with the refinement's info).
    pub refined: Option<Arc<MethodSig>>,
}

impl<'a> Worker<'a> {
    #[inline]
    pub fn type_application(&mut self, e: ExprId, expected: Option<TypeId>) -> (TExprId, TypeId) {
        if self.in_converted_body() {
            return self.type_hinted(e, expected, Self::type_application_now);
        }
        self.type_application_now(e, expected)
    }

    /// Where an opaque type's definition is visible, the type of a reference to a member names
    /// the opaque type externally (dotty's `A.T`, which stays opaque for the implicit scope) where
    /// the member's declared type names it and either the member is declared outside the
    /// definition's scope, where only `A.T` can name it, or the reference reaches the member
    /// through a path whose root is no enclosing `this` (`asSeenFrom` of the prefix `A.T` maps
    /// `A.this` to `A`): `A.T.of(1)`, an import's `of(1)`, and a top-level member read from the
    /// top level, whose prefix dotty makes the package's. `T.of(1)` and `mk(1)` of the enclosing
    /// object read through its `this` keep the reference transparent.
    #[cold]
    #[inline(never)]
    fn external_opaque_result(&mut self, e: ExprId, te: TExprId, ty: TypeId) -> TypeId {
        let Some(s) = self.referenced_member(te) else { return ty };
        let ret = match self.syms.sym(s).sig.as_ref() {
            Some(sig) => sig.ret,
            None => return ty,
        };
        let declared: Vec<ClassId> = self.transparent.clone().into_iter().filter(|&c| self.type_names_class(ret, c, 0)).collect();
        if declared.is_empty() {
            return ty;
        }
        let owner = self.syms.sym(s).owner;
        let file = self.syms.sym(s).file;
        let through = self.reached_externally(e);
        let marks: Vec<ClassId> = declared.into_iter().filter(|&c| through || !self.opaque_visible_in(c, owner, file)).collect();
        if marks.is_empty() {
            return ty;
        }
        self.mark_external_opaques(ty, &marks, 0)
    }

    /// The member a typed reference or call reads.
    fn referenced_member(&self, te: TExprId) -> Option<SymId> {
        match self.prog.expr(te) {
            TExpr::CallMethod(_, s, _) | TExpr::CallStatic(s, _) | TExpr::Field(_, s) | TExpr::Static(s) => Some(s),
            TExpr::Block(_, r) => self.referenced_member(r),
            _ => None,
        }
    }

    /// Whether `t` names the class `c` in its classes' arguments, unions and intersections.
    fn type_names_class(&self, t: TypeId, c: ClassId, depth: u32) -> bool {
        if depth > 16 {
            return false;
        }
        match self.types.get(t) {
            Type::Class(k, args) => k == c || self.types.items(args).iter().any(|&a| self.type_names_class(a, c, depth + 1)),
            Type::Union(a, b, _) | Type::Inter(a, b) => self.type_names_class(a, c, depth + 1) || self.type_names_class(b, c, depth + 1),
            _ => false,
        }
    }

    /// Whether the application's function is reached through a path whose root is no enclosing
    /// class's `this`: a qualified selection on such a root, or a name bound at the top level or
    /// by an import (dotty's prefix: a package's, an import's path).
    fn reached_externally(&mut self, e: ExprId) -> bool {
        let ast = self.cur_ast();
        let mut f = e;
        loop {
            match ast.expr(f) {
                Expr::Apply(g, _) | Expr::UsingApply(g, _) | Expr::TypeApply(g, _) => f = g,
                _ => break,
            }
        }
        match ast.expr(f) {
            Expr::Select(q, _) => self.path_root_external(q, 0),
            Expr::Ident(n) => self.name_bound_externally(n),
            _ => false,
        }
    }

    fn path_root_external(&mut self, q: ExprId, depth: u32) -> bool {
        if depth > 32 {
            return false;
        }
        let ast = self.cur_ast();
        match ast.expr(q) {
            Expr::Select(_, names::THIS) | Expr::This => false,
            Expr::Select(inner, _) => self.path_root_external(inner, depth + 1),
            Expr::Ident(n) => self.name_bound_externally(n),
            _ => false,
        }
    }

    /// Whether `n` is bound here otherwise than as a local or a member of an enclosing class.
    fn name_bound_externally(&mut self, n: Name) -> bool {
        let exported = self.ref_exported;
        let r = self.lookup_term(n);
        self.ref_exported = exported;
        matches!(r, Some(TermRef::Global(_) | TermRef::ModuleMember(..) | TermRef::ValueMember(..) | TermRef::Class(_) | TermRef::Package(_)))
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

    /// The receiver a member is selected on, typed under its selection.
    #[inline(always)]
    pub(in crate::typer) fn type_receiver(&mut self, q: ExprId, selected: bool, selection: SelectionFrame) -> (TExprId, TypeId) {
        let outer = std::mem::replace(&mut self.apply_selected, selected);
        let (tq, qty) = self.type_selected(q, selection);
        self.apply_selected = outer;
        (tq, self.receiver_vars(qty))
    }

    /// A variable of the receiver's type stays open through the selection where the receiver's
    /// interpolation left it so (its type holds it nonvariantly, or a polymorphic method named
    /// without a parameter list made it), as dotty selects a member on `Box[?T]` without
    /// instantiating `?T`: `foo(1).zero` against a `1` makes `?T` a `1`, and `::` on the
    /// `List[Item[?T]]` of `xs.map(c => Item(c, Some(c)))` leaves `?T` to its expected type. A
    /// receiver that is a variable itself is instantiated, as the trees around would have a
    /// covariant one.
    #[inline]
    pub(in crate::typer) fn receiver_vars(&mut self, qty: TypeId) -> TypeId {
        // A head that is no variable (most): the type as it is.
        if !matches!(self.types.get(qty), Type::Var(_) | Type::AppVar(..) | Type::Annotated(_)) {
            return qty;
        }
        self.receiver_vars_slow(qty)
    }

    #[inline(never)]
    fn receiver_vars_slow(&mut self, qty: TypeId) -> TypeId {
        let head = self.deref(qty);
        match self.types.get(head) {
            Type::Var(_) => self.solve_bounded_in(qty),
            // `?G[Unit]`, whose members are its variable's: dotty's selection instantiates the
            // constructor bounded below and selects again (`couldInstantiateTypeVar` of an
            // `AppliedType`), so `fk.foo.bla` over `foo[G[x] >: M[x]]: G[Unit]` selects on `M[Unit]`.
            Type::AppVar(..) if self.could_instantiate_var(qty, false) => self.zonk(qty),
            _ => qty,
        }
    }

    /// `q` typed as the qualifier of the selection, with no expected type but the selection's.
    #[inline(always)]
    pub(in crate::typer) fn type_selected(&mut self, q: ExprId, selection: SelectionFrame) -> (TExprId, TypeId) {
        self.selections.push(selection);
        let typed = self.type_expr(q, None);
        self.selections.pop();
        typed
    }

    /// The span an overloaded reference resolved by a selection on the qualifier `q` has: the
    /// qualifier's own, inside any parentheses, which its typing passes through. None where a
    /// block holds it: dotty types a block's expression with `pt.dropIfProto` (`Typer.typedBlock`),
    /// a single expression in braces included.
    pub(in crate::typer) fn qualifier_span(&self, q: ExprId) -> Option<Span> {
        let ast = self.cur_ast();
        let mut q = q;
        loop {
            if ast.is_braced(q) {
                return None;
            }
            match ast.expr(q) {
                Expr::Parens(inner) => q = inner,
                _ => break,
            }
        }
        Some(ast.expr_span(q))
    }

    /// The selection the frame at `i` of `Worker::selections` stands for, derived where it is read:
    /// the qualifier's span (`qualifier_span`), the member's first list of plain arguments or the
    /// operand, and what is expected of the member: nothing past a second plain list, the selection
    /// beneath where the member's application is itself its qualifier, else the expected type.
    /// None for a frame of another file or a qualifier in braces.
    pub(in crate::typer) fn selection_at(&self, i: usize) -> Option<Selection> {
        let f = self.selections[i];
        if f.file != self.env.file {
            return None;
        }
        let span = self.qualifier_span(f.q)?;
        let (args, plain) = match f.kind {
            FrameKind::Member => {
                let ast = self.cur_ast();
                let (mut first, mut plain, mut app) = (None, 0, f.at);
                loop {
                    match ast.expr(app) {
                        Expr::Apply(g, l) => {
                            first = Some(l);
                            plain += 1;
                            app = g;
                        }
                        Expr::UsingApply(g, _) => app = g,
                        _ => break,
                    }
                }
                (first.map_or(SelectionArgs::None, SelectionArgs::List), plain)
            }
            FrameKind::Operand { more } => (SelectionArgs::Operand(f.at), 1 + usize::from(more)),
            FrameKind::Bare => {
                return Some(Selection { span, name: f.name, args: SelectionArgs::None, result: SelectionResult::Ignored });
            }
        };
        let result = if plain > 1 {
            SelectionResult::Ignored
        } else if i > 0 && self.selection_at(i - 1).is_some_and(|s| s.span == f.app) {
            SelectionResult::Selected
        } else {
            match f.expected {
                Some(t) if t != ANY && t != self.b.t_unit && !matches!(self.types.get(t), Type::Var(_)) => SelectionResult::Type(t),
                _ => SelectionResult::Ignored,
            }
        };
        Some(Selection { span, name: f.name, args, result })
    }

    /// The selection at `i`, which a deeper round reads where the resolution found it
    /// (`selection_at`); a frame no selection derives from reads as one revealing nothing.
    pub(super) fn sel(&self, i: usize) -> Selection {
        self.selection_at(i).unwrap_or(Selection { span: Span::new(0, 0), name: Name(0), args: SelectionArgs::None, result: SelectionResult::Ignored })
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
                self.pass_new_outer(e, typed.0, typed.1);
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

    /// An application, its type naming an opaque type externally where its definition is
    /// visible and the reference is external (`external_opaque_result`). A function of its own,
    /// its body inlined into it, as master's was, so that the typing of an expression, which
    /// inlines `type_application`, is compiled as before.
    #[inline(never)]
    fn type_application_now(&mut self, e: ExprId, expected: Option<TypeId>) -> (TExprId, TypeId) {
        let (te, ty) = self.type_application_body(e, expected);
        if self.transparent.is_empty() {
            return (te, ty);
        }
        (te, self.external_opaque_result(e, te, ty))
    }

    #[inline(always)]
    fn type_application_body(&mut self, e: ExprId, expected: Option<TypeId>) -> (TExprId, TypeId) {
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
        // The parts of an interpolator's call whose arguments `-Wtostring-interpolated` lints
        // (`StringInterpolatorOpt`'s lint runs under the setting alone).
        if applies == 1 && self.diags.policy.tostring_interpolated {
            self.note_interpolation_parts(e, head);
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
            // `new C(a)(b)`: the further argument lists of the constructor, a class nested in a
            // class made with the enclosing instance its prefix gives, as with one list.
            Expr::New(ty, first) if targs.is_none() => {
                let mut clauses = vec![(first, false, ast.expr_span(head))];
                clauses.extend(raw_lists.iter().map(|&(app, args, using)| (args, using, ast.expr_span(app))));
                let first_written = ast.new_empty_first.contains(&head);
                // An explicit `new`, its constructor's signature its own (`type_new_expr`).
                let prefixed = ast.new_outers.contains_key(&head);
                let outer_prefixed = std::mem::replace(&mut self.new_prefixed, prefixed);
                let outer_new = std::mem::replace(&mut self.explicit_new, true);
                let typed = self.type_new(ty, &clauses, first_written, span, expected);
                self.explicit_new = outer_new;
                self.new_prefixed = outer_prefixed;
                self.new_enclosing_instance(head, typed, prefixed);
                return typed;
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
                // Where the class has the member they shadow what scopes outside it define (dotty
                // synthesizes them as members, `Desugar.classDef`): only a local or the class's
                // own member of the name comes before them.
                let outside = |t: &mut Self| match t.lookup_term(name) {
                    None => true,
                    Some(super::resolve::TermRef::Local(_)) => false,
                    Some(super::resolve::TermRef::This(k, _)) => Some(k) != t.this_class(),
                    Some(_) => t.this_class().is_some_and(|c| t.has_synthesized_member(c, name)),
                };
                if (synthesized || companion_apply(self)) && outside(self) {
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
                        if let Some(module) = self.module_ref(c, span) {
                            return self.apply_member(module, module_ty, name, targs, lists, span, expected);
                        }
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
                match self.resolve_static_path(q, name, !lists.is_empty()) {
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
                                return self.getter_application(DefaultOf::Ctor(c), n - 1, None, &[], targs, ty, lists, span);
                            }
                        }
                    }
                }
                let receiver = ast.expr(q);
                let selected = name == names::APPLY && matches!(receiver, Expr::TypeApply(..));
                let frame = SelectionFrame { file: self.env.file, q, name, at: e, app: span, expected, kind: FrameKind::Member };
                let (tq, qty) = self.type_receiver(q, selected, frame);
                // The qualifier's type as the written tree gives it (`written_prefix`), the prefix
                // its members are selected through: read once, here, and only of a tree whose type
                // `TypeAssigner` gives otherwise than teq's typing does (an `if`, a match, a block
                // ending in one); any other qualifier's is the type it was typed as.
                let branching = matches!(receiver, Expr::If(..) | Expr::Match(..) | Expr::Block(_));
                let pre = if branching { self.written_prefix(q, tq, qty) } else { qty };
                let typed = if pre != qty {
                    let outer = std::mem::replace(&mut self.written_qualifier, Some((tq, pre)));
                    let typed = self.apply_member(tq, qty, name, targs, lists, span, expected);
                    self.written_qualifier = outer;
                    typed
                } else {
                    self.apply_member(tq, qty, name, targs, lists, span, expected)
                };
                if !self.selection_typed.is_empty() {
                    let owner = self.qualifier_span(q);
                    self.drop_selection_typed(owner);
                }
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
        let mut bases: Vec<(ClassId, TypeId)> = self.met_base_types(c)[1..].to_vec();
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
        // A product's getter selected on `super` (`super.m$default$N`, in a trait through its
        // accessor) is applied as the source's call has it: the getter's application on
        // `super` (`Applications.findDefaultGetter`).
        if self.loaded.as_ref().is_some_and(|l| !l.product_synthetics.is_empty() && l.product_synthetics.contains_key(&m)) {
            let target = if in_trait {
                SuperTarget::Mixin(c)
            } else if through_chain {
                SuperTarget::Chain
            } else {
                SuperTarget::Class(owner)
            };
            let recv = self.prog.add(TExpr::Super(target));
            if let Some(r) = self.default_getter_selection(recv, owner_ty, name, targs, &lists, span) {
                return r;
            }
        }
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
            let callee = Callee::Method { recv: Some(recv), sym, owner_subst: Vec::new(), prefix: None, refined: None };
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
                let te = self.prog.add(TExpr::Prim(PrimOp::Same, this, that));
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
    /// `Any`'s `toString()` or `hashCode()` beside a member of the name that does not match it
    /// (`Denotation.matches`: `@targetName("s") def toString[A]: String`, `def toString(x: Int)`)
    /// is an alternative of it (`computeMembersNamed`): `()`, which that member takes no list
    /// for or needs arguments in, applies `Any`'s (`resolveOverloaded` keeps the applicable one).
    #[cold]
    #[allow(clippy::too_many_arguments)]
    fn any_member_beside(
        &mut self,
        recv: TExprId,
        recv_ty: TypeId,
        sym: SymId,
        name: Name,
        targs: Option<ListRef>,
        lists: &[ArgList],
        span: Span,
    ) -> Option<(TExprId, TypeId)> {
        if targs.is_some() || lists.len() != 1 || !lists[0].args.is_empty() || lists[0].using || self.syms.sym(sym).kind != SymKind::Def {
            return None;
        }
        if self.any_member_agreement(sym) != super::check::Agreement::Params {
            return None;
        }
        let sig = self.sig_of(sym);
        let takes_none = match sig.clauses.iter().find(|c| !c.is_using) {
            None => true,
            Some(c) => c.params.iter().any(|p| !p.has_default && !p.repeated),
        };
        if !takes_none {
            return None;
        }
        self.universal_member(recv, recv_ty, name, None, lists, span)
    }

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
        info.sig = Some(Arc::new(MethodSig { tparams: Vec::new(), clauses, ret, declared: crate::tir::NO_TYPE }));
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
            super::Frame::Class(c) => self.is_enum_companion(*c).then_some(*c),
            _ => None,
        })
    }

    /// Whether the class `c` is an enum's companion: an object, or the class of a local object
    /// or of an object nested in a class, which an enum of a block or a class has.
    pub(super) fn is_enum_companion(&self, c: ClassId) -> bool {
        let info = self.syms.class(c);
        (info.kind == ClassKind::Object || info.local_module.is_some() || info.inner_object.is_some())
            && info.companion.map_or(false, |e| self.syms.class(e).kind == ClassKind::Enum)
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
    pub(in crate::typer) fn resolve_static_path(&mut self, q: ExprId, name: Name, applied: bool) -> StaticPath {
        let Some(pkg) = self.package_path(q) else { return StaticPath::NotStatic };
        if self.deps.is_some() {
            self.deps_package_path(pkg);
        }
        match self.pkg_term(pkg, name) {
            None => StaticPath::Missing(pkg),
            Some(TermRef::Package(sub)) => match self.package_apply(sub).filter(|_| applied) {
                Some(r) => match self.term_callee(r, names::APPLY, self.cur_ast().expr_span(q)) {
                    Some(callee) => StaticPath::Callee(callee),
                    None => StaticPath::NotStatic,
                },
                None => StaticPath::NotStatic,
            },
            Some(r) => {
                if let Some(msg) = self.package_member_denied(pkg, r) {
                    let span = self.cur_ast().expr_span(q);
                    self.error(span, msg);
                }
                let outer_head = std::mem::replace(&mut self.extension_call_head, applied);
                let callee = self.term_callee(r, name, self.cur_ast().expr_span(q));
                self.extension_call_head = outer_head;
                match callee {
                    Some(callee) => StaticPath::Callee(callee),
                    None => StaticPath::NotStatic,
                }
            }
        }
    }

    /// The top-level `apply` of a package applied as a function (`views.activity(u)` for a
    /// `package views.activity` holding a `def apply`): dotty's `checkValue` leaves an applied
    /// package unchecked (a `FunOrPolyProto`) and `tryInsertApplyOrImplicit` selects its `apply`.
    fn package_apply(&mut self, p: PkgId) -> Option<TermRef> {
        self.pkg_term(p, names::APPLY).filter(|r| !matches!(r, TermRef::Package(_)))
    }

    /// A package as scalac names it in a diagnostic: its simple name, `<root>` for the root.
    pub(in crate::typer) fn package_display(&self, p: PkgId) -> String {
        if p == ROOT_PKG { "<root>".to_string() } else { self.name_str(self.syms.pkg(p).name).to_string() }
    }

    /// dotty's `userDefinedImplicitNotFoundTypeMessage` (messages.scala, `MissingImplicitArgument`):
    /// the `@implicitNotFound` text of the type `t` a given was searched for; one that starts
    /// with `explain=` explains and is no message (`userDefinedImplicitNotFoundMessage`).
    pub(super) fn implicit_not_found_message(&mut self, t: TypeId) -> Option<String> {
        let text = self.type_not_found_text(t, t, 0)?;
        (!text.starts_with("explain=")).then_some(text)
    }

    /// The `recur` of `userDefinedImplicitNotFoundTypeMessage`: a class's own text with its
    /// arguments (`AppliedType`, `TypeRef`), else the first of its base classes', their
    /// parameters as seen from it (`ClassInfo`); an alias, an abstract type, a parameter, a path
    /// or a refinement through what it stands for (`TypeProxy`); the first side of an
    /// intersection that has one, else the second (`AndType`). `pt` is the type searched for, which
    /// the class's type members are seen from.
    fn type_not_found_text(&mut self, pt: TypeId, t: TypeId, depth: u32) -> Option<String> {
        if depth > 16 {
            return None;
        }
        let t = self.deref(t);
        let d = depth + 1;
        match self.types.get(t) {
            Type::Class(c, args) => {
                let args = self.types.items(args).to_vec();
                if let Some(m) = self.class_not_found_text(c, &args, pt, None) {
                    return Some(m);
                }
                self.complete_class(c);
                let bases: Vec<ClassId> = self.syms.class(c).base_types.iter().skip(1).map(|&(b, _)| b).collect();
                bases.into_iter().find_map(|b| {
                    // The base class's parameters as seen from the type (`asSeenFrom(pt, sym)`).
                    let args: Vec<TypeId> = match self.base_type(t, b).map(|bt| self.types.get(bt)) {
                        Some(Type::Class(_, l)) => self.types.items(l).to_vec(),
                        _ => Vec::new(),
                    };
                    self.class_not_found_text(b, &args, pt, None)
                })
            }
            // A class through its enclosing instance (`o.E[Char]`, dotty's `TypeRef(o, E)`): the enclosing
            // classes' parameters and members are seen from the prefix.
            Type::Nested(prefix, inner) => {
                if let Type::Class(c, args) = self.types.get(inner) {
                    let args = self.types.items(args).to_vec();
                    if let Some(m) = self.class_not_found_text(c, &args, pt, Some(prefix)) {
                        return Some(m);
                    }
                }
                self.type_not_found_text(pt, inner, d)
            }
            Type::Inter(a, b) => self.type_not_found_text(pt, a, d).or_else(|| self.type_not_found_text(pt, b, d)),
            Type::Param(p) => {
                let upper = self.syms.tparam(p).upper;
                self.type_not_found_text(pt, upper, d)
            }
            Type::AppParam(p, args) => {
                let upper = self.app_param_upper(p, args)?;
                self.type_not_found_text(pt, upper, d)
            }
            Type::Lit(_) => {
                let class = self.widen_lit(t);
                self.type_not_found_text(pt, class, d)
            }
            Type::This(_) | Type::Term(_) | Type::Select(..) | Type::Member(..) | Type::AppMember(..) | Type::Decl(_) | Type::Refined(..) | Type::Alias(..) | Type::Match(..) => {
                let under = self.dependent_underlying(t)?;
                self.type_not_found_text(pt, under, d)
            }
            _ => None,
        }
    }

    /// dotty's `userDefinedImplicitNotFoundParamMessage`: the `@implicitNotFound` text of the
    /// using parameter `param` (named `name`) of `method`, or of the constructor of the class
    /// `ctor`, which comes before the type's, formatted in dotty's common context
    /// (`formatAnnotationMessage`): its `${T}` the type parameters of the method and of the
    /// classes enclosing it (`substitutableTypeSymbolsInScope`), as the call's `subst` gives
    /// them, the receiver's arguments for its class's (`asSeenFrom(methodOwnerType, ..)`).
    fn param_not_found_message(&mut self, method: SymId, ctor: Option<ClassId>, param: SymId, name: Name, subst: &Subst) -> Option<String> {
        let (file, def, mut tparams, enclosing) = match ctor {
            Some(c) => {
                let info = self.syms.class(c);
                (info.file, info.def, info.tparams.clone(), info.owner)
            }
            None => {
                let (file, def, owner) = {
                    let info = self.syms.sym(method);
                    (info.file, info.def, info.owner)
                };
                (file, def, self.sig_arc(method).tparams.clone(), owner)
            }
        };
        tparams.extend(self.enclosing_tparams(enclosing));
        let raw = match def {
            Some(d) => {
                let ast = self.ast(file);
                let clauses = match &ast.def(d).kind {
                    crate::ast::DefKind::Fun(f) => &f.clauses,
                    crate::ast::DefKind::Class(k) => &k.clauses,
                    _ => return None,
                };
                let p = clauses.iter().flat_map(|c| c.params.iter()).find(|p| p.name == name)?;
                let annot = ast.param_annots(p).iter().find(|a| self.name_ref(a.name) == "implicitNotFound")?;
                ast.str(*ast.str_lists[annot.args.range()].first()?).to_string()
            }
            // A jar's method or class, whose parameter's annotation its pickle carries.
            None => self.loaded.as_ref()?.param_not_found.get(&param)?.clone(),
        };
        if raw.starts_with("explain=") {
            return None;
        }
        let names: Vec<String> = tparams.iter().map(|&p| self.name_ref(self.syms.tparam(p).name).to_string()).collect();
        let args: Vec<TypeId> = tparams
            .iter()
            .map(|&p| {
                let t = self.types.param(p);
                let t = self.types.subst(t, subst);
                self.solve_in(t)
            })
            .collect();
        Some(self.format_not_found(&raw, &names, &args))
    }

    /// The `@implicitNotFound` text a class declares, its type parameters' `${A}` replaced by
    /// `args` and its type members' by what they are seen from `pt` (`asSeenFrom(pt, sym)`).
    fn class_not_found_text(&mut self, c: ClassId, args: &[TypeId], pt: TypeId, prefix: Option<TypeId>) -> Option<String> {
        let (file, def) = {
            let info = self.syms.class(c);
            (info.file, info.def)
        };
        let raw = match def {
            Some(d) => {
                let ast = self.ast(file);
                let annot = ast.def(d).annots.iter().find(|a| self.name_ref(a.name) == "implicitNotFound")?;
                ast.str(*ast.str_lists[annot.args.range()].first()?).to_string()
            }
            // A jar's class, whose annotation its pickle carries.
            None => self.loaded.as_ref()?.implicit_not_found.get(&c)?.clone(),
        };
        let mut names: Vec<String> = self.syms.class(c).tparams.iter().map(|&p| self.name_ref(self.syms.tparam(p).name).to_string()).collect();
        // The classes enclosing it (`substitutableTypeSymbolsInScope`): their parameters as seen from the
        // prefix where the type has one, as they stand otherwise.
        let mut owner = self.syms.class(c).owner;
        let mut args = args.to_vec();
        args.truncate(names.len());
        let under = prefix.map(|q| self.path_underlying(q));
        for _ in 0..32 {
            let Owner::Class(k) = owner else { break };
            let tparams = self.syms.class(k).tparams.clone();
            let seen: Vec<TypeId> = match under.and_then(|u| self.base_type(u, k)).map(|bt| self.types.get(bt)) {
                Some(Type::Class(_, l)) => self.types.items(l).to_vec(),
                _ => Vec::new(),
            };
            for (i, &p) in tparams.iter().enumerate() {
                names.push(self.name_ref(self.syms.tparam(p).name).to_string());
                args.push(seen.get(i).copied().unwrap_or_else(|| self.types.param(p)));
            }
            owner = self.syms.class(k).owner;
        }
        // The type members of the class, inherited ones too, and of the enclosing classes where the type
        // has a prefix (`typeRef.nonClassTypeMembers`), each named in the text and not a parameter: the
        // member selected on the searched type, or on the prefix.
        for name in Self::not_found_names(&raw) {
            if names.iter().any(|n| *n == name) {
                continue;
            }
            let n = self.interner.intern(&name);
            let seen = if self.class_has_type_member(c, n) {
                self.member_type(pt, n)
            } else {
                prefix.and_then(|q| self.member_type(q, n))
            };
            if let Some(t) = seen {
                names.push(name);
                args.push(t);
            }
        }
        Some(self.format_not_found(&raw, &names, &args))
    }

    /// The type parameters of the classes from `owner` out to its package, innermost first
    /// (the classes' part of dotty's `substitutableTypeSymbolsInScope`).
    fn enclosing_tparams(&mut self, mut owner: Owner) -> Vec<TParamId> {
        let mut out = Vec::new();
        for _ in 0..32 {
            let Owner::Class(k) = owner else { break };
            let info = self.syms.class(k);
            out.extend(info.tparams.iter().copied());
            owner = info.owner;
        }
        out
    }

    /// Whether the class declares or inherits the type member `name`.
    fn class_has_type_member(&mut self, c: ClassId, name: Name) -> bool {
        self.complete_class(c);
        let bases: Vec<ClassId> = self.syms.class(c).base_types.iter().map(|&(b, _)| b).collect();
        bases.iter().any(|&b| self.syms.class(b).type_aliases.contains_key(&name))
    }

    /// The names a text's `${A}` placeholders give, in order.
    fn not_found_names(raw: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut rest = raw;
        while let Some(at) = rest.find("${") {
            let after = &rest[at + 2..];
            let Some(end) = after.find('}') else { break };
            out.push(after[..end].trim().to_string());
            rest = &after[end + 1..];
        }
        out
    }

    /// dotty's `formatAnnotationMessage` (`userDefinedErrorString`): `${A}` of the parameter
    /// named `A` among `names` replaced by its argument among `args`, `?A` for one without.
    fn format_not_found(&mut self, raw: &str, names: &[String], args: &[TypeId]) -> String {
        let mut out = String::with_capacity(raw.len());
        let mut rest = raw;
        while let Some(at) = rest.find("${") {
            out.push_str(&rest[..at]);
            let after = &rest[at + 2..];
            let Some(end) = after.find('}') else {
                out.push_str(&rest[at..]);
                rest = "";
                break;
            };
            let name = after[..end].trim();
            match names.iter().position(|n| n == name).and_then(|i| args.get(i)) {
                Some(&arg) => out.push_str(&self.show(arg)),
                None => {
                    out.push('?');
                    out.push_str(name);
                }
            }
            rest = &after[end + 1..];
        }
        out.push_str(rest);
        out
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

    /// The `New` a constructor proxy's application makes has a class type whose prefix is to be
    /// a path, as dotty's `PostTyper` checks it (`checkClassType` with `stablePrefixReq`, E083):
    /// `outer.Inner(2)` of a `def outer` is refused.
    fn check_class_prefix(&mut self, recv: TExprId, recv_ty: TypeId, span: Span) {
        if recv_ty == ERROR || self.is_class_prefix_path(recv) {
            return;
        }
        let t = self.widen_path(recv_ty);
        let msg = format!("{} is not a valid class prefix, since it is not an immutable path", self.show(t));
        self.error(span, msg);
    }

    /// Whether the constructor proxy `apply` of the explicit companion `module` of the class `c`
    /// is accessible here: dotty gives it the constructor's access, `protected` where the class is
    /// protected, and the class's qualifier (`NamerOps.addConstructorApplies`), so a private or
    /// protected one is reached from the companion or the class alone, a qualified one from the
    /// qualifier's scope.
    fn proxy_apply_accessible(&self, c: ClassId, module: ClassId) -> bool {
        let info = self.syms.class(c);
        let inside = self.env.frames.iter().any(|f| matches!(f, super::Frame::Class(k) if *k == module || *k == c));
        // The proxy takes the constructor's access, not the class's own `private` (which the
        // class's name keeps), a private constructor's being the application's own check
        // (`check_ctor_access`); a case class's `apply` is its own synthetic method.
        // Protected where the class is and its companion is not (a protected companion's
        // public `apply` is as reachable as the companion is).
        let case = info.mods & crate::ast::mods::CASE != 0;
        if !case && info.mods & crate::ast::mods::PROTECTED != 0 && self.syms.class(module).mods & crate::ast::mods::PROTECTED == 0 {
            return inside;
        }
        let qualified = info.mods & crate::ast::mods::PRIVATE != 0 && self.has_access_scope(info.file, info.span.start);
        !qualified || inside || self.in_access_scope(info.file, info.span.start, info.owner)
    }

    /// A path as a class prefix: a stable path, a parameterless val's read through one included
    /// (its getter's call), no def's call.
    fn is_class_prefix_path(&self, e: TExprId) -> bool {
        match self.prog.expr(e) {
            TExpr::CallMethod(r, s, args) => args.is_empty() && self.syms.sym(s).kind == SymKind::Val && self.is_class_prefix_path(r),
            TExpr::CallStatic(s, args) => args.is_empty() && self.syms.sym(s).kind == SymKind::Val,
            TExpr::Field(r, s) => self.syms.sym(s).kind != SymKind::Var && self.is_class_prefix_path(r),
            _ => self.is_stable_path(e),
        }
    }

    /// The class that an application of `callee` constructs: a class, or a companion object
    /// without an `apply` of its own.
    pub fn constructed_class(&self, callee: TermRef) -> Option<ClassId> {
        if let TermRef::Class(c) | TermRef::ValueClass(_, c) = callee {
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
            Some(TermRef::Class(_) | TermRef::ValueClass(..)) | Some(TermRef::Package(_)) => false,
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
            // The setter a class has or exports (an export of a var forwards its setter too,
            // dotty's `typedAssign` selecting `pre.member(setterName)`).
            let exported = |w: &mut Self| match w.class_of(qty) {
                Some(c) => w.exports_of(c).is_some_and(|e| e.terms.contains_key(&setter)),
                None => false,
            };
            if self.find_member(qty, setter).is_some() || exported(self) || self.has_lexical_extension_for(qty, setter) {
                let lists = vec![ArgList { args: vec![ArgSrc::Ast(rhs)], using: false, span }];
                let (te, _) = self.apply_member(tq, qty, setter, None, lists, span, None);
                return Ok((te, self.b.t_unit));
            }
        }
        Err(self.apply_member(tq, qty, name, None, Vec::new(), span, None))
    }

    /// The application of the synthesized `apply` of `module`, a companion nested in a class, of the
    /// class `companion`, which constructs inside it (`Desugar.classDef`): its enclosing instance
    /// is the one the companion is selected on, `o` of `o.C(1)`, the prefix `newExpr` keeps. The
    /// constructor stands for the member, whichever way the application reached it, an overload
    /// resolved to it included (`try_overloaded`), so that every one passes the prefix
    /// (`pass_outer_applied`).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn nested_companion_apply(
        &mut self,
        recv: TExprId,
        recv_ty: TypeId,
        module: ClassId,
        companion: ClassId,
        proxy: bool,
        targs: Option<ListRef>,
        lists: Vec<ArgList>,
        span: Span,
        expected: Option<TypeId>,
    ) -> (TExprId, TypeId) {
        let Some(written_outer) = self.module_prefix(recv, module) else {
            let msg = format!("not supported yet: the companion of {}, a class nested in a class, applied through no path", self.name_str(self.syms.class(companion).name));
            self.error(span, msg);
            return (self.prog.add(TExpr::Unit), ERROR);
        };
        // The class is constructed through the companion's prefix, which is an immutable path
        // (dotty's `newExpr`, E083).
        if proxy && !self.is_stable_path(recv) {
            let prefix = match (self.types.get(recv_ty), self.prog.expr(recv)) {
                (Type::Nested(p, _), _) => Some(p),
                (_, TExpr::Field(q, _) | TExpr::CallMethod(q, _, _)) => self.prog.type_of(q),
                _ => self.prog.type_of(written_outer),
            };
            let msg = format!("{} is not a valid class prefix, since it is not an immutable path", self.show(prefix.unwrap_or(recv_ty)));
            self.error(span, msg);
        }
        // The companion is the receiver of its `apply` (`EtaExpansion.liftApp`): bound before the
        // arguments where it is not idempotent (`Lifter.liftPrefix`), else evaluated first, as the
        // call's qualifier is, which runs a companion's initialiser before the arguments; the
        // enclosing instance the creation takes is the saved companion's.
        let (recv, before) = self.companion_receiver(recv, module, span);
        let outer = if before.iter().any(|st| matches!(st, TStmt::Val(..))) {
            self.module_prefix(recv, module).unwrap_or(written_outer)
        } else {
            written_outer
        };
        let outer_prefixed = std::mem::replace(&mut self.new_prefixed, true);
        // The receiver's class, which the class's members are seen from (a private alias of a
        // base is none of a subclass's): the companion's prefix's (`d` of `d.Cmd(..)`), its node
        // typed or its type's prefix.
        let outer_ty = self.prog.type_of(outer).or(match self.types.get(recv_ty) {
            Type::Nested(p, _) => Some(p),
            _ => None,
        });
        let receiver_class = outer_ty.and_then(|t| self.class_of(t));
        let outer_creator = std::mem::replace(&mut self.creator_receiver, receiver_class.map(|r| (companion, r)));
        // The constructor seen from the companion's prefix (`g.I(7, "a")` of a `g: O[Int]` takes
        // an `Int`), as a creation through it is.
        let prefix = match self.types.get(recv_ty) {
            Type::Nested(p, _) => Some(p),
            _ => None,
        };
        let prefix = prefix.filter(|_| self.is_inner_class(companion));
        let outer_ctor_prefix = std::mem::replace(&mut self.ctor_prefix, prefix);
        let outer_creation = std::mem::replace(&mut self.creation_outer, Some((companion, outer)));
        let r = self.apply_companion_apply(companion, targs, lists, span, expected);
        self.creation_outer = outer_creation;
        self.ctor_prefix = outer_ctor_prefix;
        self.creator_receiver = outer_creator;
        self.new_prefixed = outer_prefixed;
        self.pass_outer_applied(r.0, outer);
        let r = (self.bind_prefix(r.0, before), r.1);
        // The instance's type is the class through the companion's prefix (`o.C`), its `apply`'s
        // result seen from `o.C.type`; an enum's class case widened to its enum, the enum's base
        // type of that (`o.E` of `o.E.K(1)`, the case's companion through the enum's).
        let ty = match (self.types.get(recv_ty), self.types.get(r.1)) {
            (Type::Nested(p, _), Type::Class(k, _)) if k != companion && self.syms.class(companion).kind == ClassKind::EnumCase => {
                let case_ty = self.prog.type_of(r.0).filter(|&t| self.class_of(t) == Some(companion)).unwrap_or_else(|| self.types.class(companion, &[]));
                let precise = self.nested_type(p, case_ty);
                self.base_type(precise, k).unwrap_or(r.1)
            }
            (Type::Nested(p, _), Type::Class(..)) => self.nested_type(p, r.1),
            _ => r.1,
        };
        (r.0, ty)
    }

    /// The receiver of a companion's `apply`, the companion `module`, and the statements that
    /// evaluate it before the arguments: one that is not idempotent bound to a temporary of the
    /// companion's type (`Lifter.liftPrefix`, `EtaExpansion.liftApp`); an idempotent one kept, and
    /// evaluated first where the companion has initialisation (dotty's `NoInits`, `no_inits`), the
    /// one case its evaluation as the call's qualifier is told apart.
    fn companion_receiver(&mut self, recv: TExprId, module: ClassId, span: Span) -> (TExprId, Vec<TStmt>) {
        use super::elaborate::Purity;
        if self.expr_purity(recv).at_least(Purity::IDEMPOTENT) {
            if self.no_inits(module) {
                return (recv, Vec::new());
            }
            let first = self.prior_copy(recv);
            return (recv, vec![TStmt::Expr(first)]);
        }
        self.bound_once(recv, span)
    }

    /// `e` bound to a fresh temporary before what follows, of its type (an object nested in a
    /// class selected on its prefix, of its module's), and the reference to it.
    fn bound_once(&mut self, e: TExprId, span: Span) -> (TExprId, Vec<TStmt>) {
        let own = self.prog.type_of(e).or_else(|| match self.prog.expr(e) {
            TExpr::Field(_, s) | TExpr::CallMethod(_, s, _) => self.syms.sym(s).sig.as_ref().map(|sig| sig.ret),
            _ => None,
        });
        let ty = own.unwrap_or(ANY);
        let tmp = self.fresh_local("prefix", ty, span);
        let r = self.prog.add(TExpr::Local(tmp));
        self.prog.set_type(r, ty);
        (r, vec![TStmt::Val(tmp, e)])
    }

    /// The creation `te` under the statements `companion_receiver` made, which come before the
    /// temporaries of the arguments.
    fn bind_prefix(&mut self, te: TExprId, before: Vec<TStmt>) -> TExprId {
        if before.is_empty() {
            return te;
        }
        let mut stmts = before;
        let value = match self.prog.expr(te) {
            TExpr::Block(inner, v) => {
                stmts.extend_from_slice(self.prog.stmt_list(inner));
                v
            }
            _ => te,
        };
        let ty = self.prog.type_of(te);
        let stmts = self.prog.stmts.push_slice(&stmts);
        let b = self.prog.add(TExpr::Block(stmts, value));
        if let Some(t) = ty {
            self.prog.set_type(b, t);
        }
        b
    }

    /// The enclosing instance of the companion object `module` nested in a class, which its
    /// synthesized `apply` constructs with (`Desugar.classDef`): the instance the companion is
    /// selected on (`o` of `o.C`, its member found through that instance's class, `findRef`);
    /// inside the companion itself (`C.this`, as a pickle writes `C(..)` there) its own
    /// enclosing instance; on another value holding the companion, that value's outer instance.
    pub(super) fn module_prefix(&mut self, recv: TExprId, module: ClassId) -> Option<TExprId> {
        let own = self.syms.class(module).inner_object;
        match self.prog.expr(recv) {
            TExpr::Field(q, s) if Some(s) == own => return Some(q),
            TExpr::CallMethod(q, s, args) if args.len == 0 && Some(s) == own => return Some(q),
            TExpr::This if self.this_class() == Some(module) => {
                let outer = self.enclosing_instance_class(module)?;
                return Some(self.this_ref(outer));
            }
            _ => {}
        }
        self.enclosing_instance_class(module)?;
        let accessor = self.outer_accessor(module);
        let call = self.prog.add(TExpr::CallMethod(recv, accessor, crate::ast::ListRef::EMPTY));
        let ty = self.sig_of(accessor).ret;
        self.prog.set_type(call, ty);
        Some(call)
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
                let msg = match self.given_ambiguity.take().or_else(|| self.implicit_not_found_message(ty)) {
                    Some(m) => m,
                    None => format!("no given instance of type {} was found", self.show(ty)),
                };
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
                r => {
                    let c = self.forwarded_object(r.sym()?)?;
                    self.module_term(c, n)
                }
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

    pub(crate) fn is_method_sym(&mut self, s: SymId) -> bool {
        match self.syms.sym(s).kind {
            SymKind::Def | SymKind::Overloaded(_) => true,
            SymKind::Given => self.is_method_given(s),
            _ => false,
        }
    }

    /// A given of parameters, or an inline given, an inline method of no parameters too
    /// (dotty's `Desugar` makes `inline given g: T = e` an `inline def`), which a selection
    /// expands.
    #[inline(never)]
    fn is_method_given(&mut self, s: SymId) -> bool {
        if self.syms.sym(s).mods & crate::ast::mods::INLINE != 0 {
            return true;
        }
        let sig = self.sig_of(s);
        !sig.clauses.is_empty() || !sig.tparams.is_empty()
    }

    /// The type a reference to a parameterless structural given has: dotty makes the given a
    /// module (`Parsers.givenDef`'s `ModuleDef`), whose reference is of its class, every member
    /// of the body visible (`t.self`), while the search compares it by the type it was declared
    /// with (`Applications.widenGiven`), which is its signature's.
    #[inline]
    pub(super) fn given_object_type(&mut self, s: SymId) -> Option<TypeId> {
        let info = self.syms.sym(s);
        let c = info.impl_class.filter(|_| info.kind == SymKind::Given)?;
        self.given_object_type_of(s, c)
    }

    fn given_object_type_of(&mut self, s: SymId, c: ClassId) -> Option<TypeId> {
        let sig = self.sig_of(s);
        (sig.tparams.is_empty() && sig.clauses.is_empty() && self.syms.class(c).tparams.is_empty()).then(|| self.types.class(c, &[]))
    }

    /// Whether `s` may name a module read inside its own class (`enclosing_module_class`): a
    /// given or a lazy val, which every other read of a name is not.
    #[inline(always)]
    fn may_read_module(&self, s: SymId) -> bool {
        let info = self.syms.sym(s);
        info.kind == SymKind::Given || info.kind == SymKind::Val && info.mods & crate::ast::mods::LAZY != 0
    }

    /// A module's name read inside its own class: that class's `this`.
    #[cold]
    #[inline(never)]
    fn module_read_callee(&mut self, s: SymId) -> Option<Callee> {
        let k = self.enclosing_module_class(s)?;
        let ty = self.module_ref_type(s, k);
        Some(Callee::Value(self.this_ref(k), ty))
    }

    /// A local object's lazy val read inside its class: that class's `this`.
    #[cold]
    #[inline(never)]
    fn local_module_read_callee(&mut self, s: SymId) -> Option<Callee> {
        let c = self.enclosing_module_class(s)?;
        let ty = self.sig_of(s).ret;
        Some(Callee::Value(self.this_ref(c), ty))
    }

    /// The type of a module's name read inside its class `k`: its class's, or a local or inner
    /// object's own.
    fn module_ref_type(&mut self, s: SymId, k: ClassId) -> TypeId {
        match self.given_object_type(s) {
            Some(t) => t,
            None if self.syms.class(k).tparams.is_empty() => self.types.class(k, &[]),
            None => self.sig_of(s).ret,
        }
    }

    pub(super) fn is_path(&self, e: TExprId) -> bool {
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
            return Callee::Method { recv: None, sym: s, owner_subst: Vec::new(), prefix: None, refined: None };
        }
        let ty = self.sig_of(s).ret;
        let ty = self.given_object_type(s).unwrap_or(ty);
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

    /// The member `name` a selection on `recv` (of type `recv_ty`) finds: its class's, or, on an
    /// intersection, its parts' met against the qualifier's type as written (`written_prefix`),
    /// the joint information a meet makes left for the member's application (`joint_sig`).
    #[inline]
    fn selection_member(&mut self, recv: TExprId, recv_ty: TypeId, name: Name) -> Option<(SymId, TypeId)> {
        let pre = match self.written_qualifier {
            Some((r, p)) if r == recv => p,
            _ => recv_ty,
        };
        let mut joint = None;
        let found = self.find_member_joint(pre, recv_ty, name, &mut joint);
        if let (Some(info), Some((s, _))) = (joint, found) {
            self.joint_sig = Some((Some(recv), s, info));
        }
        found
    }

    pub(super) fn member_callee(&mut self, recv: TExprId, recv_ty: TypeId, s: SymId) -> Callee {
        let name = self.syms.sym(s).name;
        // A member of `this` whose self type is an intersection carries the joint information
        // the meet gives it, as a selection's does (`selection_member`).
        let mut joint = None;
        let found = self.find_member_joint(recv_ty, recv_ty, name, &mut joint);
        if let (Some(info), Some((found, _))) = (joint, found) {
            self.joint_sig = Some((Some(recv), found, info));
        }
        match found {
            Some((found, owner_ty)) => {
                // What the lookup by type finds may be the settled entry of what the scope held,
                // or the member the receiver's class has under the name where the scope held one
                // of a parent it overrides (`Util.tag` imported through an object whose `Cap`
                // gives its own): the type's, whose owner the owner type is, as dotty selects a
                // member anew on the prefix, where it matches the member named.
                // A given alternative a given selector brought (`export O.given`, `import O.given`)
                // stands alone, the other alternatives of its name not brought with it (dotty's
                // `addWildcardForwarders` forwards the given member by itself).
                let take_found = self.syms.alternatives(found).is_some() || found != s && self.syms.alternatives(s).is_none() && self.found_matches(recv_ty, s, found, owner_ty);
                let s = if take_found && !self.given_alternative(s) { found } else { s };
                self.member_callee_in(recv, recv_ty, Some(owner_ty), s)
            }
            None => self.member_callee_in(recv, recv_ty, None, s),
        }
    }

    /// Whether `found`, the member the receiver has under the name of `s`, takes the place of
    /// `s`: dotty puts one denotation of a name in another's place only where the two match
    /// (`Denotations.meet`'s `mergeDenot`, `SingleDenotation.matches`), their parameters, as the
    /// receiver sees them, erasing alike (`Signature.matchDegree`'s `consistentParams`). A
    /// module matches no method of its name that takes parameters: an implicit class's
    /// conversion beside the companion that holds its constructor's defaults.
    #[cold]
    #[inline(never)]
    fn found_matches(&mut self, recv_ty: TypeId, s: SymId, found: SymId, found_owner_ty: TypeId) -> bool {
        use super::loader::declared::PickledSigParam;
        let shape = |sig: &MethodSig| (sig.tparams.len(), sig.clauses.iter().map(|c| c.params.len()).sum::<usize>());
        if shape(&self.sig_arc(s)) != shape(&self.sig_arc(found)) {
            return false;
        }
        let owner_ty = match self.syms.sym(s).owner {
            Owner::Class(c) => {
                let t = self.widen_path(recv_ty);
                self.base_type(t, c)
            }
            _ => None,
        };
        let mut seen = |m: SymId, owner_ty: Option<TypeId>| {
            let subst = owner_ty.map_or_else(Vec::new, |t| self.owner_subst(t));
            let (params, _) = self.pickled_signature_seen(m, "", &subst)?;
            Some(params.into_iter().map(|p| match p {
                PickledSigParam::Types(n) => format!("#{n}"),
                PickledSigParam::Term(t) => t,
            }).collect::<Vec<_>>())
        };
        match (seen(s, owner_ty), seen(found, Some(found_owner_ty))) {
            (Some(a), Some(b)) => a == b,
            // A type whose erasure teq cannot tell is consistent with any (dotty's
            // `tpnme.Uninstantiated`).
            _ => true,
        }
    }

    fn is_own_ctor_param(&self, s: SymId) -> bool {
        let Owner::Class(owner) = self.syms.sym(s).owner else { return false };
        self.syms.class(owner).ctor_syms.iter().flatten().any(|&p| p == s)
    }

    /// An alternative of a name that is a given, which a given selector brings alone.
    #[cold]
    #[inline(never)]
    fn given_alternative(&self, s: SymId) -> bool {
        let i = self.syms.sym(s);
        i.alternative && (i.kind == SymKind::Given || i.mods & crate::ast::mods::GIVEN != 0)
    }

    /// Private members are reachable from their class and its companion only, a top-level
    /// `private[p]` definition from inside `p`.
    pub fn is_accessible(&self, s: SymId) -> bool {
        let mut info = self.syms.sym(s);
        // A deferred given's implementation has its declaration's access (`dcl.copy`).
        let mut s = s;
        if info.mods & crate::ast::mods::OVERRIDE != 0 && info.def.is_none() {
            s = self.access_declaration_of(s);
            info = self.syms.sym(s);
        }
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

    /// The package a member of `owner` is a member of: the package itself, or the one around a
    /// package object (dotty's `skipWeakOwner`).
    fn effective_package(&self, owner: Owner) -> Option<PkgId> {
        match owner {
            Owner::Package(p) => Some(p),
            Owner::Class(c) if self.is_package_object(c) => match self.syms.class(c).owner {
                Owner::Package(p) => Some(p),
                _ => None,
            },
            _ => None,
        }
    }

    /// Whether the site is inside the package `p` (`ctx.owner.isContainedIn(boundary)`).
    fn site_in_package(&self, p: PkgId) -> bool {
        let mut site = Some(self.file_pkgs[self.env.file.0 as usize]);
        while let Some(sp) = site {
            if sp == p {
                return true;
            }
            site = self.syms.pkg(sp).parent;
        }
        false
    }

    /// dotty's `CannotBeAccessed` for a private member of the package `pre` selected through it
    /// from outside its boundary: a `private` definition of a package (or of a package object)
    /// is `private[p]` of that package (`Namer`: "widen private to private[p]"), a `private[q]`
    /// one is reachable from inside `q` (`isAccessibleFrom`'s `accessWithin`). A member a jar
    /// holds goes by `qualified_reachable`; library bodies and the std see every member.
    pub(super) fn package_member_denied(&mut self, pre: PkgId, r: crate::typer::resolve::TermRef) -> Option<String> {
        use crate::typer::resolve::TermRef;
        let (mods, file, start, owner, word, name) = match r {
            TermRef::Global(s) | TermRef::ModuleMember(_, s) => {
                let info = self.syms.sym(s);
                let word = match info.kind {
                    SymKind::Object(_) => "object",
                    SymKind::Def => "method",
                    SymKind::Var => "variable",
                    _ => "value",
                };
                (info.mods, info.file, info.span.start, info.owner, word, info.name)
            }
            TermRef::Class(c) => {
                let info = self.syms.class(c);
                let word = match info.kind {
                    ClassKind::Trait => "trait",
                    ClassKind::Object => "object",
                    _ => "class",
                };
                (info.mods, info.file, info.span.start, info.owner, word, info.name)
            }
            _ => return None,
        };
        if mods & crate::ast::mods::PRIVATE == 0 || self.in_jar(file) {
            return None;
        }
        let site = self.env.file;
        if self.in_jar(site) || self.is_body_file(site) || self.source(site).is_std {
            return None;
        }
        let home = self.effective_package(owner)?;
        let scoped = self.has_access_scope(file, start);
        if if scoped { self.in_access_scope(file, start, owner) } else { self.site_in_package(home) } {
            return None;
        }
        let boundary = if scoped {
            self.ast(file).access_scopes.iter().find(|&&(at, _)| at == start).map(|&(_, n)| self.name_str(n).to_string()).unwrap_or_default()
        } else {
            self.name_str(self.syms.pkg(home).name).to_string()
        };
        let shown = format!("{} {}", word, self.name_str(name));
        // The member is shown with its place where its owner is no package: a package object's,
        // as every top-level term but an object is in dotty (`<file>$package`).
        let package_member = matches!(owner, Owner::Package(_)) && matches!(word, "object" | "class" | "trait");
        let what = if package_member { shown.clone() } else { format!("{} in package {}", shown, self.package_full_name(home)) };
        let from = self.this_class().map(|k| format!(" from {}", self.class_description(k))).unwrap_or_default();
        Some(format!(
            "{} cannot be accessed as a member of {}.type{}.\n  private[{}] {} can only be accessed from package {}.",
            what,
            self.package_full_name(pre),
            from,
            boundary,
            shown,
            if scoped { boundary.clone() } else { self.package_full_name(home) },
        ))
    }

    /// A package's full name as scalac shows it (`a.b`).
    fn package_full_name(&self, p: PkgId) -> String {
        let mut parts = Vec::new();
        let mut at = Some(p);
        while let Some(q) = at {
            if q == ROOT_PKG {
                break;
            }
            parts.push(self.name_str(self.syms.pkg(q).name).to_string());
            at = self.syms.pkg(q).parent;
        }
        parts.reverse();
        parts.join(".")
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

    pub(super) fn derives_or_selects(&self, c: ClassId, owner: ClassId) -> bool {
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
        let s = self.access_declaration_of(s);
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
        // Only along a chain to an inner class's member: a member of one, or a member whose
        // result is an instance of one, which is decided once per member.
        let inner_chain = self.on_inner_chain(s);
        if inner_chain {
            self.note_inner_receiver(recv, recv_ty);
        }
        if self.is_method_sym(s) {
            let prefix = self.dependent_prefix(s, recv, recv_ty);
            if prefix.is_some() {
                self.note_outer_prefix(s, recv);
            } else if inner_chain {
                self.note_outer_owner_args(s, recv, &owner_subst);
            }
            if self.is_inline_callee(s) {
                self.inline.recv_types.insert(recv, recv_ty);
            }
            return Callee::Method { recv: Some(recv), sym: s, owner_subst, prefix, refined: None };
        }
        // An intersection's joint member's result (`joint_sig`), else its symbol's.
        let ret = match self.take_joint_sig(Some(recv), s) {
            Some(info) => info.ret,
            None => self.given_object_type(s).unwrap_or_else(|| self.sig_of(s).ret),
        };
        let mut ty = self.types.subst(ret, &owner_subst);
        if self.types.has_paths(ty) {
            let prefix = self.prefix_of(recv, recv_ty);
            ty = self.seen_from_prefix(ty, prefix, s);
            if self.member_of_inner_class(s) {
                ty = self.seen_from_outer_steps(s, recv, ty);
                let owner = match self.syms.sym(s).owner {
                    Owner::Class(o) => Some(o),
                    _ => None,
                };
                ty = self.seen_from_enclosing_this_of(ty, owner);
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
            // A value of an enum whose companion is a local object or an object nested in a class
            // is a field of the companion's instance.
            SymKind::EnumValue(_) if !self.instance_enum_value(s) => self.prog.add(TExpr::Static(s)),
            _ => match self.constant_of(s) {
                // A constant member stands for its value behind a stable path, as in Scala,
                // where its object need not be initialised; behind any other receiver the
                // selection stays, with the receiver's effects and its dereference.
                Some(literal) if self.is_stable_path(recv) => {
                    let te = self.prog.add(literal);
                    self.folded_paths.insert(te, (recv, s));
                    self.folded_deprecated |= self.syms.sym(s).info.mods & crate::ast::mods::DEPRECATED != 0;
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

    /// The owner's type arguments of a member of an inner class seen from the receiver chain's
    /// outer instances, as its signature is; apart from the application's path, which most calls
    /// take with no outer prefix noted.
    #[cold]
    #[inline(never)]
    fn owner_subst_from_outer(&mut self, call: &MethodCall, subst: &mut Subst) {
        if !self.member_of_inner_class(call.sym) {
            return;
        }
        if let Some(steps) = call.recv.and_then(|r| self.outer_prefixes.get(&r).cloned()) {
            for (_, t) in subst.iter_mut() {
                *t = steps.iter().fold(*t, |t, &(outer, path)| self.as_seen_from(t, path, outer));
            }
        }
    }

    /// The prefix a member's signature has to be seen from, when it depends on `this`.
    pub(super) fn dependent_prefix(&mut self, s: SymId, recv: TExprId, recv_ty: TypeId) -> Option<TypeId> {
        self.sig_names_paths(s).then(|| self.prefix_of(recv, recv_ty))
    }

    /// Whether `c` is a context function class, decided once per class (`context_function_classes`)
    /// where the arity's slot names none yet: the test reads the class's name.
    fn is_context_function_class_memo(&mut self, c: ClassId) -> bool {
        let k = SymId(c.0);
        if let Some(v) = self.context_function_classes.get(k) {
            return v;
        }
        let v = self.is_context_function_class(c);
        self.context_function_classes.set(k, v);
        v
    }

    /// Whether the signature of `s` names a path, which its prefix may move: decided once its
    /// signature is complete and kept per symbol (`path_sigs`), as dotty decides a member's
    /// denotation once rather than at each of its applications.
    #[inline]
    pub(super) fn sig_names_paths(&mut self, s: SymId) -> bool {
        match self.path_sigs.get(s) {
            Some(v) => v,
            None => self.sig_names_paths_now(s),
        }
    }

    #[cold]
    #[inline(never)]
    fn sig_names_paths_now(&mut self, s: SymId) -> bool {
        self.sig_of(s);
        let sig = self.syms.sig(s);
        let v = self.types.has_paths(sig.ret)
            || sig.clauses.iter().any(|cl| cl.params.iter().any(|p| self.types.has_paths(p.ty)))
            || sig.tparams.iter().any(|&p| self.types.has_paths(self.syms.tparam(p).upper) || self.types.has_paths(self.syms.tparam(p).lower));
        if self.syms.sym_cells.get(s.0) == crate::symbols::Completion::Done {
            self.path_sigs.set(s, v);
        }
        v
    }

    /// The receiver of a call on a chain to an inner class's member, noted by its expression.
    #[cold]
    #[inline(never)]
    fn note_inner_receiver(&mut self, recv: TExprId, recv_ty: TypeId) {
        if !self.prog.record_types {
            self.receiver_types.insert(recv, recv_ty);
        }
    }

    /// The owner's type arguments may name an outer instance too (`Limit.apply` of an object
    /// whose parents' arguments name the class it is nested in), for a member of a class nested
    /// in a class.
    #[cold]
    #[inline(never)]
    fn note_outer_owner_args(&mut self, s: SymId, recv: TExprId, owner_subst: &Subst) {
        if owner_subst.iter().any(|&(_, t)| self.types.has_paths(t)) {
            self.note_outer_prefix(s, recv);
        }
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
        // The receivers met on the way, whose types may name outer instances of objects.
        let mut met: Vec<TypeId> = Vec::new();
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
                let Some(rt) = self.prog.type_of(r).or_else(|| self.receiver_types.get(&r).copied()).or_else(|| self.unrecorded_receiver_type(r)) else { break };
                let rt = self.deref(rt);
                met.push(rt);
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
        // A receiver whose type is the singleton of an object nested in a class (`framework:
        // coll.AggregationFramework.type`) gives that class's `this` as the object's prefix (`coll`),
        // as dotty's prefix types carry it: what a member of the object names of the class.
        let mut starts: Vec<TypeId> = steps.iter().map(|&(_, p)| p).collect();
        starts.extend(self.prog.type_of(recv).or_else(|| self.receiver_types.get(&recv).copied()).or_else(|| self.unrecorded_receiver_type(recv)));
        starts.extend(met);
        for start in starts {
            // Not asked whether the signature mentions the class: its `this` may come in with
            // the steps before (the object's parents' arguments name it).
            for (k, q) in self.object_prefix_steps(start) {
                if !steps.iter().any(|&(c, _)| c == k) {
                    steps.push((k, q));
                }
            }
        }
        steps
    }

    /// The outer instances a value of type `t` gives where it is an object nested in a class
    /// (`framework: coll.AggregationFramework.type`): that class's `this` is the object's prefix
    /// (`coll`), and so on outwards, as dotty's prefix types carry them.
    pub(super) fn object_prefix_steps(&mut self, t: TypeId) -> Vec<(ClassId, TypeId)> {
        let mut steps = Vec::new();
        let mut path = Some(t);
        for _ in 0..4 {
            let Some(p) = path else { break };
            let root = self.object_path_root(p);
            let Type::Select(q, o) = self.types.get(root) else { break };
            // An object, or a jar's object nested in a class, whose instance the class holds in
            // a lazy val of its name (`ClassInfo::inner_object`).
            let object = matches!(self.syms.sym(o).kind, SymKind::Object(_)) || {
                let ret = self.sig_of(o).ret;
                self.class_of(ret).is_some_and(|c| self.syms.class(c).inner_object == Some(o))
            };
            let Owner::Class(k) = self.syms.sym(o).owner else { break };
            if !object {
                break;
            }
            if self.syms.class(k).kind != ClassKind::Object {
                steps.push((k, q));
            }
            path = Some(q);
        }
        steps
    }

    /// The path a value of type `t` is, through vals of singleton types and aliases of them
    /// (`framework: coll.AggregationFramework` whose alias is `coll.AggregationFramework.type`).
    fn object_path_root(&mut self, t: TypeId) -> TypeId {
        let mut t = self.stable_root(t);
        for _ in 0..4 {
            let under = if self.types.is_path(t) { self.path_underlying(t) } else { t };
            let under = self.dealias(under);
            if !self.types.is_path(under) || under == t {
                return t;
            }
            t = self.stable_root(under);
        }
        t
    }

    /// Whether a call of `s` is on a chain to an inner class's member, its receiver's type wanted
    /// there (`receiver_types`): a member of an inner class or one whose result is an instance of
    /// one, read once per member.
    #[inline]
    fn on_inner_chain(&mut self, s: SymId) -> bool {
        match self.inner_chain_members.get(s) {
            Some(on) => on,
            None => self.on_inner_chain_now(s),
        }
    }

    #[cold]
    #[inline(never)]
    fn on_inner_chain_now(&mut self, s: SymId) -> bool {
        let on = self.member_of_inner_class(s) || self.result_of_inner_class(s);
        self.inner_chain_members.set(s, on);
        on
    }

    /// Whether `s`'s result is an instance of a class nested in a class.
    fn result_of_inner_class(&mut self, s: SymId) -> bool {
        let ret = self.sig_of(s).ret;
        // A class type, the most results, read directly.
        match self.types.get(ret) {
            Type::Class(c, _) => self.class_in_class(c),
            Type::Nested(..) => true,
            _ => self.class_of(ret).is_some_and(|c| self.class_in_class(c)),
        }
    }

    /// The type of a receiver whose type the program does not record (a build that records no
    /// expression's type, JavaScript's): a val or a parameter as its path, an object, a static
    /// val; `None` for any other expression.
    fn unrecorded_receiver_type(&mut self, r: TExprId) -> Option<TypeId> {
        match self.prog.expr(r) {
            TExpr::Local(s) | TExpr::Static(s) => Some(match self.syms.sym(s).kind {
                SymKind::Val | SymKind::Param => self.types.mk(Type::Term(s)),
                _ => self.sig_of(s).ret,
            }),
            TExpr::Module(o) => Some(self.types.class(o, &[])),
            _ => None,
        }
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
    pub(super) fn seen_from_outer_steps(&mut self, s: SymId, recv: TExprId, ty: TypeId) -> TypeId {
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
            Type::Select(p, _) | Type::Member(p, _) | Type::Lambda(_, p) => self.mentions_this_of(p, k),
            Type::Poly(ps, p) => self.mentions_this_of(p, k) || self.types.poly_bounds(ps).to_vec().into_iter().any(|b| self.mentions_this_of(b, k)),
            Type::AppMember(m, args) => self.mentions_this_of(m, k) || self.types.items(args).iter().any(|&a| self.mentions_this_of(a, k)),
            Type::Class(_, args) | Type::AppParam(_, args) | Type::AppVar(_, args) | Type::Alias(_, args) => {
                self.types.items(args).iter().any(|&a| self.mentions_this_of(a, k))
            }
            Type::Union(a, b, _) | Type::Inter(a, b) => self.mentions_this_of(a, k) || self.mentions_this_of(b, k),
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
    pub(super) fn member_seen_from(&mut self, t: TypeId, prefix: TypeId, c: ClassId, member: SymId) -> TypeId {
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
        self.seen_from_enclosing_this_of(t, None)
    }

    /// `seen_from_enclosing_this` for a member of the class `owner`: an enclosing class that
    /// is the owner or encloses it sees an outer class's `this` of the member as that outer
    /// instance, whatever the class derives from (dotty's `toPrefix` from the member's owner:
    /// `A.this.C` of `x` in `class B(x: C) extends A` nested in `A`, read inside `B`).
    pub(super) fn seen_from_enclosing_this_of(&mut self, t: TypeId, owner: Option<ClassId>) -> TypeId {
        if !self.types.has_paths(t) {
            return t;
        }
        let classes: Vec<ClassId> = self.env.frames.iter().rev().filter_map(|f| match f {
            super::Frame::Class(d) => Some(*d),
            _ => None,
        }).collect();
        let encloses_owner = |w: &Self, d: ClassId| {
            let mut at = owner.map(Owner::Class);
            while let Some(Owner::Class(k)) = at {
                if k == d {
                    return true;
                }
                at = Some(w.syms.class(k).owner);
            }
            false
        };
        let mut t = t;
        for d in classes {
            if encloses_owner(self, d) {
                continue;
            }
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
    pub(super) fn sig_seen_from_enclosing(&mut self, sig: Arc<MethodSig>) -> Arc<MethodSig> {
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
        Arc::new(MethodSig { tparams: sig.tparams.clone(), clauses, ret, declared: crate::tir::NO_TYPE })
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
        Arc::new(MethodSig { tparams: sig.tparams.clone(), clauses, ret, declared: crate::tir::NO_TYPE })
    }

    /// The signature of a member as seen from the prefix of the call's receiver.
    pub(super) fn sig_seen_from(&mut self, sig: Arc<MethodSig>, prefix: TypeId, member: SymId) -> Arc<MethodSig> {
        self.sig_seen_as(sig, prefix, prefix, member)
    }

    /// The signature seen from `prefix`, with `inserted` in the place of the `this` it moves:
    /// the prefix itself, or the stand-in of `prefix_stand_in`.
    pub(super) fn sig_seen_as(&mut self, sig: Arc<MethodSig>, prefix: TypeId, inserted: TypeId, member: SymId) -> Arc<MethodSig> {
        let Some(c) = self.class_of(prefix) else { return sig };
        let mut whole: Option<Option<TypeId>> = None;
        let mut seen = |t: &mut Self, ty: TypeId| {
            let ty = t.member_seen_from(ty, inserted, c, member);
            if !t.types.has_paths(ty) {
                return ty;
            }
            let w = *whole.get_or_insert_with(|| t.intersection_prefix(prefix));
            match w {
                Some(w) => t.seen_from_parts(ty, inserted, c, w),
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
        Arc::new(MethodSig { tparams: sig.tparams.clone(), clauses, ret, declared: crate::tir::NO_TYPE })
    }

    /// A signature seen from a prefix and then given the owner's type arguments by `owner_subst`
    /// takes the two as one map, as dotty's `AsSeenFromMap` replaces `C.this` by the prefix and the
    /// class's parameters by its arguments in one pass: a prefix that is no path and names those
    /// parameters itself (the receiver `Box[(T, U)]` of `self: this.type` inside `Box[T]`) would
    /// have them replaced a second time. It goes in as a fresh parameter, bounded by it, which the
    /// substitution replaces by the prefix with the owner's arguments: the parameter and the
    /// substitution's entry, none where the prefix passes through the substitution unchanged.
    pub(super) fn prefix_stand_in(&mut self, prefix: TypeId, owner_subst: &Subst) -> Option<(TypeId, (TParamId, TypeId))> {
        if owner_subst.is_empty() || self.types.is_path(prefix) || self.types.subst(prefix, owner_subst) == prefix {
            return None;
        }
        let name = self.interner.intern("this");
        let q = self.syms.new_tparam(name, 0);
        self.syms.tparams[q.idx()].upper = prefix;
        Some((self.types.param(q), (q, prefix)))
    }

    /// `final val x = 1` without a type is a constant in Scala: a use is replaced by the literal
    /// (`constant_value`), another worker's initialiser read through its chunk.
    pub(super) fn constant_of(&mut self, s: SymId) -> Option<TExpr> {
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
            crate::types::LitVal::Float(bits) => TExpr::Double(f32::from_bits(bits) as f64),
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
        if let (Some(case), TermRef::ModuleMember(c, s)) = (self.parent_args_of, r) {
            if self.syms.class(case).owner == Owner::Class(c) && self.syms.class(case).kind == ClassKind::EnumCase && !self.enum_case_ref_allowed(c, s, name) {
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

    /// Whether an enum case's parent may name `s`, a member of the enum's companion `c`, as dotty's
    /// `Checking.checkEnumCaseRefsLegal` allows it: where an identifier just outside the enum, with the
    /// companion's cases imported, gives the same symbol: another case, or a member the scope around
    /// the enum reaches itself.
    fn enum_case_ref_allowed(&mut self, c: ClassId, s: SymId, name: Name) -> bool {
        let Some(e) = self.syms.class(c).companion else { return false };
        let children = self.syms.class(e).children.clone();
        let case = children.iter().any(|&k| {
            self.syms.class(k).singleton == Some(s) || self.syms.class(k).companion.is_some_and(|m| self.syms.class(m).singleton == Some(s))
        });
        if case {
            return true;
        }
        let (file, owner, at) = {
            let info = self.syms.class(e);
            (info.file, info.owner, info.span.start)
        };
        let env = self.env_at(file, owner, at);
        let outside = self.with_env(env, |t| t.quietly(|t| t.lookup_term(name)));
        outside.and_then(|r| r.sym()) == Some(s)
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

    /// Whether the enum value `s` is a field of its companion's instance: its companion is a local
    /// object or an object nested in a class, as dotty's companion of such an enum is.
    pub(super) fn instance_enum_value(&self, s: SymId) -> bool {
        let Owner::Class(m) = self.syms.sym(s).owner else { return false };
        let info = self.syms.class(m);
        info.local_module.is_some() || info.inner_object.is_some()
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

    /// A member's reference written as a name keeps the purity its writing gives (`exprPurity`
    /// of an `Ident`, `Worker::note_written_ref`): its node selects it on its owner.
    #[inline(always)]
    fn written_member(&mut self, callee: Callee) -> Callee {
        if let Callee::Value(te, _) = callee {
            let exported = std::mem::take(&mut self.ref_exported);
            self.note_written_ref(te, exported);
        }
        callee
    }

    pub(super) fn term_callee(&mut self, r: TermRef, name: Name, span: Span) -> Option<Callee> {
        if let TermRef::This(_, s) | TermRef::ModuleMember(_, s) | TermRef::Global(s) = r {
            if self.may_read_module(s) {
                if let Some(callee) = self.module_read_callee(s) {
                    return Some(self.written_member(callee));
                }
            }
        }
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
                } else {
                    let (kind, lazy, by_name) = {
                        let info = self.syms.sym(s);
                        (info.kind, info.mods & crate::ast::mods::LAZY != 0, info.by_name)
                    };
                    match (kind, lazy) {
                        (SymKind::Def, _) => Callee::Method { recv: None, sym: s, owner_subst: Vec::new(), prefix: None, refined: None },
                        // A local object's lazy val read inside its class.
                        (_, true) if self.enclosing_module_class(s).is_some() => self.local_module_read_callee(s)?,
                        _ => {
                            let ty = self.sig_of(s).ret;
                            // Read across a trait a class owns: the proxy the class of the block
                            // holds, through the enclosing instances (`proxies.rs`).
                            if let Some(r) = self.held_holder(s) {
                                let te = self.held_read(r, s);
                                return Some(Callee::Value(te, ty));
                            }
                            let local = self.prog.add(TExpr::Local(s));
                            let te = if by_name { self.prog.add(TExpr::CallClosure(local, ListRef::EMPTY)) } else { local };
                            Callee::Value(te, ty)
                        }
                    }
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
                let callee = self.member_callee(this, self_ty, s);
                self.written_member(callee)
            }
            TermRef::ModuleMember(c, s) => {
                let module_ty = self.types.class(c, &[]);
                // A member imported from a local object is selected on the object's lazy val,
                // one from an object nested in a class on the enclosing instance's.
                let module = self.module_ref(c, span)?;
                let callee = self.member_callee(module, module_ty, s);
                self.written_member(callee)
            }
            TermRef::Global(s) => {
                let callee = self.global_callee(s);
                self.written_member(callee)
            }
            TermRef::ValueMember(v, m) => {
                let (recv, recv_ty) = self.import_value_ref(v, span)?;
                let callee = self.member_callee(recv, recv_ty, m);
                self.written_member(callee)
            }
            // A top-level case class's name as a value: its companion (`materialized_companion`).
            TermRef::Class(c) if !self.extension_call_head && self.syms.class(c).mods & crate::ast::mods::CASE != 0 && matches!(self.syms.class(c).owner, Owner::Package(_)) => {
                match self.materialized_companion(c).and_then(|m| self.syms.class(m).module_sym) {
                    Some(s) => return self.term_callee(TermRef::Global(s), name, span),
                    None => Callee::Ctor(c),
                }
            }
            TermRef::Class(c) => Callee::Ctor(c),
            // The imported proxy applies as its selection on the value: `o.Inner(args)`.
            TermRef::ValueClass(v, c) => {
                let (recv, recv_ty) = self.import_value_ref(v, span)?;
                Callee::Selection { recv, recv_ty, name: self.syms.class(c).name }
            }
            TermRef::ModuleClass(m, c) => {
                let recv = self.module_ref(m, span)?;
                let recv_ty = self.types.class(m, &[]);
                Callee::Selection { recv, recv_ty, name: self.syms.class(c).name }
            }
            TermRef::Package(p) => {
                // Applied, a package is its top-level `apply` (`activity(u)` inside `views`).
                if self.extension_call_head {
                    if let Some(r) = self.package_apply(p) {
                        return self.term_callee(r, names::APPLY, span);
                    }
                }
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
        let entry = self.import_values[value.0 as usize];
        self.import_entry_ret(entry)
    }

    /// `import_value_ret` of an entry: a val selected on the value before it is seen from that
    /// value's type (`env.api` of an `Env[A]`), as a member selected on a path is.
    pub(super) fn import_entry_ret(&mut self, entry: (SymId, Option<ClassId>, Option<super::ValueImport>)) -> TypeId {
        let (v, module, through) = entry;
        let ret = self.sig_of(v).ret;
        if let Some(prev) = through {
            if !matches!(self.syms.sym(v).kind, SymKind::Object(_)) {
                let pre = self.import_value_ret(prev);
                let name = self.syms.sym(v).name;
                if let Some((s, owner_ty)) = self.find_member(pre, name) {
                    if s == v {
                        let subst = self.owner_subst(owner_ty);
                        return self.types.subst(ret, &subst);
                    }
                }
            }
            return ret;
        }
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
    /// `C.this.v` for a val of an enclosing class's instance, or `O.v` for one the object `O`
    /// inherits, as it is read elsewhere.
    pub(super) fn import_value_prefix(&mut self, value: super::ValueImport) -> TypeId {
        let (v, module, through) = self.import_values[value.0 as usize];
        match (through, module) {
            // An object nested in a class, read on the value it is selected on (`o.R`).
            (Some(_), _) => self.import_value_type(value),
            (None, Some(c)) if self.syms.class(c).kind != ClassKind::Object => {
                let this = self.this_prefix(c);
                self.types.mk(Type::Select(this, v))
            }
            (None, Some(o)) => self.object_member_path(o, v),
            (None, None) => self.types.mk(Type::Term(v)),
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

    /// A case class's companion where a function is expected is its `apply`, with scalac's
    /// warning (dotty's `adaptNoArgsOther`, `caseCompanion`): `val g: Int => F = F`, `xs.map(F)`,
    /// `Scope.F` of an object of source or of products. `apply_callee` asks it of the resolved
    /// term applied to nothing where a type is expected, named or selected: the module here,
    /// reached through a block where its prefix is evaluated first, or the constructor's proxy
    /// (`inserted_proxy_apply`), which stands for the companion a case class without one has.
    #[inline(never)]
    fn inserted_module_apply(&mut self, te: TExprId, ty: TypeId, span: Span, expected: Option<TypeId>) -> Option<(TExprId, TypeId)> {
        let module = match self.prog.expr(te) {
            TExpr::Block(_, inner) => inner,
            _ => te,
        };
        let TExpr::Module(o) = self.prog.expr(module) else { return None };
        let k = self.companion_class(o).filter(|&k| self.syms.class(k).mods & crate::ast::mods::CASE != 0)?;
        if !self.inserts_apply(expected) || self.function_base(self.types.class(o, &[])).is_some() {
            return None;
        }
        self.warn_inserted_apply(k, span);
        Some(self.apply_member(te, ty, names::APPLY, None, Vec::new(), span, expected))
    }

    /// `inserted_module_apply` for the constructor proxy of the case class `c`.
    #[inline(never)]
    fn inserted_proxy_apply(&mut self, c: ClassId, span: Span, expected: Option<TypeId>) -> Option<(TExprId, TypeId)> {
        let info = self.syms.class(c);
        if info.mods & crate::ast::mods::CASE == 0 || info.kind != ClassKind::Class || !self.inserts_apply(expected) {
            return None;
        }
        self.warn_inserted_apply(c, span);
        Some(self.apply_companion_apply(c, None, Vec::new(), span, expected))
    }

    /// Whether the expected type asks for a function (dotty's `functionExpected`), outside a
    /// library body, whose trees scalac typed already.
    fn inserts_apply(&mut self, expected: Option<TypeId>) -> bool {
        expected.map_or(false, |t| self.as_function(t).is_some()) && !self.is_body_file(self.env.file)
    }

    /// The warning at the name the selection or identifier `span` ends in, with the tree of the
    /// companion of the case class `k` as scalac shows it: its static path, or as written where
    /// an instance reaches it.
    fn warn_inserted_apply(&mut self, k: ClassId, span: Span) {
        let written = self.source(self.env.file).text.get(span.start as usize..span.end as usize).unwrap_or("").to_string();
        let name_len = written.rsplit('.').next().map_or(0, |n| n.len()) as u32;
        let at = Span { start: span.end - name_len.min(span.end - span.start), end: span.end };
        let text = self.static_path_text(k).unwrap_or(written);
        self.warn(at, format!("The method `apply` is inserted. The auto insertion will be deprecated, please write `{}.apply` explicitly.", text));
    }

    /// The class `k`'s companion as scalac prints a tree that reaches it statically (dotty's
    /// `RefinedPrinter.toTextPrefixOf`): its packages and enclosing objects, without the root,
    /// `scala` and `java.lang` packages (`isOmittablePrefix`). None for one an instance owns.
    fn static_path_text(&mut self, k: ClassId) -> Option<String> {
        let info = self.syms.class(k);
        let name = self.interner.get(info.name).to_string();
        match info.owner {
            Owner::Package(p) if p == crate::symbols::ROOT_PKG || p == self.b.scala_pkg || Some(p) == self.java_lang_pkg() => Some(name),
            Owner::Package(p) => Some(format!("{}.{}", self.pkg_path(p), name)),
            Owner::Class(o) if self.syms.class(o).kind == ClassKind::Object => Some(format!("{}.{}", self.static_path_text(o)?, name)),
            _ => None,
        }
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
            Callee::Method { recv, sym, owner_subst, prefix, refined } => {
                if self.is_right_assoc_extension(sym) {
                    return self.apply_right_assoc_method(recv, sym, owner_subst, prefix, targs, lists, span, expected);
                }
                let call = MethodCall { recv, sym, owner_subst, ext_recv: None, prefix, refined };
                self.apply_method(call, None, targs, lists, span, expected, false).unwrap()
            }
            Callee::Overloaded { recv, recv_ty, set } => {
                self.apply_overloaded(recv, recv_ty, set, targs, lists, span, expected)
            }
            Callee::Selection { recv, recv_ty, name } => self.apply_member(recv, recv_ty, name, targs, lists, span, expected),
            Callee::Ctor(c) => {
                // A proxy is the companion's unless a `new` names the class, or it is reached
                // through an outer instance, which the proxy's application takes in place.
                if lists.is_empty() && targs.is_none() && expected.is_some() && !self.explicit_new && !self.new_prefixed {
                    if let Some(typed) = self.inserted_proxy_apply(c, span, expected) {
                        return typed;
                    }
                }
                // `Make[String]` of a case class: the proxy's `apply` with the type arguments,
                // expanded as any method that needs arguments is (dotty's
                // `tryInsertApplyOrImplicit` for a term applied to types, then eta-expansion),
                // without the warning of an `apply` inserted on the companion itself.
                if let (Some(ta), true) = (targs, lists.is_empty()) {
                    let proxy = !self.explicit_new && !self.new_prefixed && self.syms.class(c).kind == ClassKind::Class && self.syms.class(c).mods & crate::ast::mods::CASE != 0;
                    if proxy && Self::required_arity(&self.syms.class(c).ctor).is_some() {
                        return self.apply_companion_apply(c, targs, Vec::new(), span, expected);
                    }
                    if let Some(r) = self.summon_type_application(c, None, ta, span, expected) {
                        return r;
                    }
                }
                // The constructor of a `new`, taken before its arguments are typed: an application
                // of a class's name among them is one.
                let explicit = std::mem::take(&mut self.explicit_new);
                let outer_seen = self.apply_seen_from;
                self.apply_seen_from = if explicit { None } else { self.companion_apply_receiver(c).map(|r| (c, r)) };
                let (te, ty) = self.apply_ctor(c, targs, lists, span, expected);
                self.apply_seen_from = outer_seen;
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
                    if expected.is_some() && matches!(self.prog.expr(te), TExpr::Module(_) | TExpr::Block(..)) {
                        if let Some(typed) = self.inserted_module_apply(te, ty, span, expected) {
                            return typed;
                        }
                    }
                    return (te, ty);
                }
                // A polymorphic function value applied: its type parameters take the explicit
                // type arguments, or fresh variables the arguments settle.
                let head = self.deref(ty);
                if let Some((ids, fun)) = self.poly_binders(head) {
                    let ps: Vec<TypeId> = ids.iter().map(|&p| self.types.param(p)).collect();
                    // A type argument of a higher-kinded parameter is a type constructor (`f[List]`),
                    // as a method's is.
                    let explicit: Vec<TypeId> = match targs {
                        Some(l) => {
                            let ids = self.cur_ast().ty_list(l).to_vec();
                            ids.iter().map(|&t| self.resolve_type_ctor(t)).collect()
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
                        if self.type_clause_args(&clause, first, &Vec::new(), &mut args, false) {
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
                    if self.type_clause_args(&clause, first, &Vec::new(), &mut args, false) {
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

    /// dotty's `BetaReduce.apply` over an application a copy makes, `f(args)` where `f` is a
    /// closure, possibly under blocks of pure bindings (`isPureBinding`), which the result keeps.
    /// `reduceApplication` binds each argument to its parameter:
    /// - a pure local path (`isPurePath`) is put for the parameter itself;
    /// - any other argument is a val named after the parameter, typed at the argument's constant
    ///   type or its widened one, in the order of the arguments, so that its effects run once and
    ///   first; a pure argument of a constant type (or of `Unit`) needs no val;
    /// - a parameter of a constant type stands for its literal in the copy of the closure's body
    ///   (`expansion1`), where a pure constant the copy comes to is folded to its literal.
    ///
    /// The vals precede the copy (`seq`). A by-name parameter, which dotty binds as a local
    /// method, leaves the application unreduced, as an arity the closure does not take does.
    pub(super) fn beta_reduce_binding(&mut self, f: TExprId, args: &[TExprId]) -> Option<TExprId> {
        use super::elaborate::Purity;
        match self.prog.expr(f) {
            TExpr::Lambda(params, body) => {
                let params = self.prog.sym_list(params).to_vec();
                if params.len() != args.len() || params.iter().any(|&p| self.syms.sym(p).by_name) {
                    return None;
                }
                let mut map: crate::intern::FxMap<SymId, TExprId> = crate::intern::FxMap::default();
                let mut bindings: Vec<TStmt> = Vec::new();
                for (&p, &a) in params.iter().zip(args) {
                    let purity = self.expr_purity(a);
                    if matches!(self.prog.expr(a), TExpr::Local(_)) && purity.at_least(Purity::PURE_PATH) {
                        map.insert(p, a);
                        continue;
                    }
                    let constant = self.argument_constant(a);
                    let ty = self.prog.type_of(a);
                    let unit = constant.is_none() && ty.map_or(false, |t| self.deref(t) == self.b.t_unit);
                    if !((constant.is_some() || unit) && purity.at_least(Purity::PURE)) {
                        let bound_ty = match constant {
                            Some(v) => self.types.lit(v),
                            None => ty.map_or(ANY, |t| self.widen_lit(t)),
                        };
                        let span = self.prog.span_of(a).map_or(Span::default(), |(_, s)| s);
                        let b = self.new_local(self.syms.sym(p).name, SymKind::Val, bound_ty, span);
                        bindings.push(TStmt::Val(b, a));
                        if constant.is_none() && !unit {
                            let r = self.prog.add(TExpr::Local(b));
                            self.prog.set_type(r, bound_ty);
                            map.insert(p, r);
                        }
                    }
                    if let Some(v) = constant {
                        let lit = self.literal_expr(v);
                        self.prog.copy_span(a, lit);
                        map.insert(p, lit);
                    } else if unit {
                        let u = self.prog.add(TExpr::Unit);
                        self.prog.set_type(u, self.b.t_unit);
                        map.insert(p, u);
                    }
                }
                let empty = crate::intern::FxMap::default();
                let renames = crate::intern::FxMap::default();
                if self.attempts.pending_len() != 0 {
                    self.expand_pending_in(&[body]);
                }
                let expansion = self.instantiate_quote(body, &empty, &map, &Subst::default(), &renames);
                let expansion = self.literal_of_pure_constant(expansion);
                if bindings.is_empty() {
                    return Some(expansion);
                }
                let stmts = self.prog.stmts.push_slice(&bindings);
                let block = self.prog.add(TExpr::Block(stmts, expansion));
                if let Some(t) = self.prog.type_of(expansion) {
                    self.prog.set_type(block, t);
                }
                Some(block)
            }
            TExpr::Block(stmts, res) => {
                let pure = self.prog.stmt_list(stmts).to_vec().into_iter().all(|st| self.stat_purity(st).at_least(Purity::PURE));
                if !pure {
                    return None;
                }
                let reduced = self.beta_reduce_binding(res, args)?;
                let block = self.prog.add(TExpr::Block(stmts, reduced));
                if let Some(t) = self.prog.type_of(reduced) {
                    self.prog.set_type(block, t);
                }
                Some(block)
            }
            _ => None,
        }
    }

    /// The constant of an argument of a constant type, as dotty's typer gives one (a literal, a
    /// constant operation, a block ending in one), which a path is not, its type being its
    /// singleton, nor an ascribed constant or a plain inline call's (`widened_constant`).
    pub(super) fn argument_constant(&mut self, a: TExprId) -> Option<LitVal> {
        let path = |t: &Self, x: TExprId| matches!(t.prog.expr(x), TExpr::Local(_) | TExpr::Static(_) | TExpr::Field(..));
        let mut res = a;
        while let TExpr::Block(_, r) = self.prog.expr(res) {
            res = r;
        }
        if path(self, res) || self.widened_constant(a) {
            return None;
        }
        self.fold_constant(res)
    }

    /// `e` with a pure constant it comes to as its literal (`BetaReduce`'s `expansion1`), at its
    /// place: the whole, else a block's result.
    pub(super) fn literal_of_pure_constant(&mut self, e: TExprId) -> TExprId {
        use super::elaborate::Purity;
        if !matches!(self.prog.expr(e), TExpr::Int(_) | TExpr::Long(_) | TExpr::Double(_) | TExpr::Bool(_) | TExpr::Char(_) | TExpr::Str(_)) {
            if let Some(v) = self.fold_constant(e).filter(|_| self.expr_purity(e).at_least(Purity::PURE)) {
                let lit = self.literal_expr(v);
                self.prog.copy_span(e, lit);
                return lit;
            }
        }
        if let TExpr::Block(stmts, res) = self.prog.expr(e) {
            let r = self.literal_of_pure_constant(res);
            if r != res {
                self.prog.exprs[e.idx()] = TExpr::Block(stmts, r);
            }
        }
        e
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
            // The companion's synthesized `apply`, its signature seen from the receiver
            // (`companion_apply_sig`).
            let outer_seen = self.apply_seen_from;
            self.apply_seen_from = self.companion_apply_receiver(c).map(|r| (c, r));
            let (te, ty) = self.apply_ctor(c, targs, lists, span, expected);
            self.apply_seen_from = outer_seen;
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
            // `p.C` asks for `C` whatever its prefix (`isRef`).
            Type::Nested(_, class) => matches!(self.types.get(class), Type::Class(ec, _) if ec == c),
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
        let prefix = self.ctor_prefix.take();
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
        // An enum's secondary constructor calls another of its own.
        let enum_self_call = kind == ClassKind::Enum && self.excluded_ctor.is_some();
        if !(matches!(kind, ClassKind::Class | ClassKind::EnumCase) || product_given || enum_self_call) || is_abstract {
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
        let mut sig = Arc::new(MethodSig { tparams, clauses: info.ctor.clone(), ret: self_ty, declared: crate::tir::NO_TYPE });
        // A companion's `apply` takes the written types seen from the instance it is selected on
        // (`companion_apply_sig`), and a construction through a prefix sees them through it.
        if let Some((_, r)) = self.apply_seen_from.filter(|&(k, _)| k == c) {
            self.apply_seen_from = None;
            sig = self.companion_apply_sig(c, r, sig);
        }
        let sig = match prefix {
            Some(p) => self.ctor_sig_through(sig, p, c),
            None => sig,
        };
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
            let secondaries = secondaries
                .into_iter()
                .map(|s| {
                    let sig = self.sig_arc(s);
                    (s, match prefix {
                        Some(p) => self.ctor_sig_through(sig, p, c),
                        None => sig,
                    })
                })
                .collect();
            return self.construct_overloaded(c, primary, secondaries, targs, lists, span, expected);
        }
        let call = MethodCall { recv: None, sym: SymId(u32::MAX), owner_subst: Vec::new(), ext_recv: None, prefix: None, refined: None };
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

    /// A constructor's signature for an instance of the inner class `c` made through `prefix`
    /// (`new o.I(a)`): its parameters and its result seen from the prefix, the constructor of
    /// dotty's `o.I` (`outer_seen_from`).
    pub(super) fn ctor_sig_through(&mut self, sig: Arc<MethodSig>, prefix: TypeId, c: ClassId) -> Arc<MethodSig> {
        let mut seen = (*sig).clone();
        for p in seen.clauses.iter_mut().flat_map(|cl| cl.params.iter_mut()) {
            p.ty = self.outer_seen_from(p.ty, prefix, c);
        }
        seen.ret = self.outer_seen_from(seen.ret, prefix, c);
        Arc::new(seen)
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
        if self.app_depth == 0 && !self.mismatch_renders.is_empty() {
            self.render_mismatches(0);
        }
        r
    }

    /// Type arguments of a member that takes none (an export's forwarder of an object, `final def
    /// O: p.O.type`) are its result's `apply`'s: `O[T]` is `O.apply[T]`, as dotty's
    /// `typedTypeApply` inserts the `apply` of a value that is no polymorphic method
    /// (`tryInsertApplyOrImplicit`).
    /// None where the member is no method or is an extension's.
    #[cold]
    #[inline(never)]
    #[allow(clippy::too_many_arguments)]
    fn apply_result_targs(&mut self, call: &MethodCall, targs: Option<ListRef>, lists: &[ArgList], span: Span, expected: Option<TypeId>, trial: bool) -> Option<Option<(TExprId, TypeId)>> {
        if call.ext_recv.is_some() || targs.is_none() || !self.is_method_sym(call.sym) {
            return None;
        }
        let Some((te, ty)) = self.apply_method(call.clone(), None, None, Vec::new(), span, None, trial) else { return Some(None) };
        Some(Some(self.apply_member(te, ty, names::APPLY, targs, lists.to_vec(), span, expected)))
    }

    /// The interpolation the call `sym` on `recv` with `lists` is, where the parts of one were
    /// noted (`interpolation_call`).
    #[cold]
    #[inline(never)]
    fn interpolation_of(&mut self, sym: crate::types::SymId, recv: Option<TExprId>, span: Span, lists: &[ArgList]) -> Option<super::tostring::InterpCall> {
        let args = lists.first().map_or(0, |l| l.args.len());
        self.interpolation_call(sym, recv, span, args)
    }

    /// `apply_method_in_now` of a standard interpolator's call: its arguments are linted as the
    /// interpolation's (`interp_call`, which the arguments at the call's depth alone read).
    #[cold]
    #[inline(never)]
    #[allow(clippy::too_many_arguments)]
    fn apply_interpolation(&mut self, interp: super::tostring::InterpCall, call: MethodCall, targs: Option<ListRef>, lists: Vec<ArgList>, span: Span, expected: Option<TypeId>, trial: bool) -> Option<(TExprId, TypeId)> {
        let outer = self.interp_call.replace(interp);
        let applied = self.apply_method_in_now(call, None, targs, lists, span, expected, trial);
        self.interp_call = outer;
        applied
    }

    /// The joint signature left for the application of `sym` on `recv` (`joint_sig`), taken.
    #[inline]
    pub(super) fn take_joint_sig(&mut self, recv: Option<TExprId>, sym: SymId) -> Option<Arc<MethodSig>> {
        match &self.joint_sig {
            Some((r, s, _)) if *r == recv && *s == sym => self.joint_sig.take().map(|(_, _, info)| info),
            _ => None,
        }
    }

    /// The type mismatches reported under the application at `depth` or deeper, its arguments
    /// typed, rendered from the types their variables were instantiated to, what is still open by
    /// `reported_mismatch`: dotty renders a `TypeMismatch` lazily with the constraint of the
    /// attempt it was reported in (`TypeMismatch.msg`'s `mapCtx`), which the application's
    /// interpolation, made after the attempt is committed, does not reach (`simpleApply` under
    /// `tryEither`). What an application left (one that returned before solving) is rendered once
    /// the outermost is typed.
    fn render_mismatches(&mut self, depth: u32) {
        let renders = std::mem::take(&mut self.mismatch_renders);
        for (at, old, found, expected, note_len, at_depth) in renders {
            if at_depth < depth {
                self.mismatch_renders.push((at, old, found, expected, note_len, at_depth));
                continue;
            }
            if self.diags.items.get(at).map_or(true, |d| d.msg != old) {
                continue;
            }
            let (found, expected) = (self.zonk(found), self.zonk(expected));
            // A variable the failure left at the error type keeps the message as reported.
            if self.types.contains_error(found) || self.types.contains_error(expected) {
                continue;
            }
            let (found, expected, _) = self.reported_mismatch(found, expected);
            // What the message adds after the types (a match type's note) stays.
            let note = &old[old.len() - note_len..];
            let msg = format!("type mismatch: found {}, required {}{}", self.show(found), self.show(expected), note);
            self.diags.items[at].msg = msg;
        }
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
        if self.retry_depth.is_some_and(|d| d + 1 == self.app_depth) {
            self.retry_applied = None;
        }
        // The member's application under `apply_member_or_extension` is the first under it, and
        // the arguments of an argument cache's application are typed at its depth.
        if let Some(c) = self.member_call.as_mut() {
            if c.depth == 0 {
                c.depth = self.app_depth;
            }
        }
        self.arg_cache_claim(self.app_depth);
        let erroneous_mark = (self.erroneous_args.len(), self.erroneous_lambdas.len());
        let applied = self.apply_method_in_now(call, ctor, targs, lists, span, expected, trial);
        self.erroneous_args.truncate(erroneous_mark.0);
        self.erroneous_lambdas.truncate(erroneous_mark.1);
        self.app_depth -= 1;
        self.annotation_args = outer;
        applied
    }

    #[allow(clippy::too_many_arguments)]
    /// The plan of an application of `sym`, or of the constructor `ctor` names.
    #[cold]
    #[inline(never)]
    fn call_plan(&self, ctor: &Option<(ClassId, Arc<MethodSig>)>, sym: SymId, sig: &Arc<MethodSig>) -> super::elaborate::CallPlan {
        let of = match ctor {
            Some((c, _)) => DefaultOf::Ctor(*c),
            None => DefaultOf::Method(sym),
        };
        super::elaborate::CallPlan::new(of, sig.clone(), self.annotation_args)
    }

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
        let copy = std::mem::take(&mut self.copy_application);
        // A standard interpolator's call over literal parts, whose arguments are linted, where
        // the parts were noted (`note_interpolation_parts`): its parts taken, it comes back here.
        if self.interp_parts.is_some() && ctor.is_none() && call.sym.0 != u32::MAX {
            if let Some(interp) = self.interpolation_of(call.sym, call.recv, span, &lists) {
                return self.apply_interpolation(interp, call, targs, lists, span, expected, trial);
            }
        }
        let mut stand_in: Option<(TParamId, TypeId)> = None;
        let sig = match &ctor {
            // The constructor of a class nested in a class names the outer class's `this`,
            // the enclosing instance deriving from it where it is constructed or extended.
            Some((c, s)) if self.loaded.is_some() && self.class_in_class(*c) => self.sig_seen_from_enclosing(s.clone()),
            Some((_, s)) => s.clone(),
            None if call.refined.is_some() => call.refined.clone().expect("the refined signature"),
            None => match call.prefix {
                Some(prefix) => {
                    // The signature is kept through the application, which types the arguments.
                    let sig = match self.take_joint_sig(call.recv, call.sym) {
                        Some(info) => info,
                        None => self.sig_arc(call.sym),
                    };
                    let inserted = match self.prefix_stand_in(prefix, &call.owner_subst) {
                        Some((param, entry)) => {
                            stand_in = Some(entry);
                            param
                        }
                        None => prefix,
                    };
                    let sig = self.sig_seen_as(sig, prefix, inserted, call.sym);
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
                // An intersection's joint member's signature (`joint_sig`).
                None => match self.take_joint_sig(call.recv, call.sym) {
                    Some(info) => info,
                    None => self.sig_arc(call.sym),
                },
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
            // A macro's `copy[..](..)` or companion `apply[..](..)` applies the constructor's
            // signature (`ctor`), whose type parameters the class's are, through no symbol.
            None => match self.macro_targs.take() {
                Some((name, ts)) if (ctor.is_some() || name == self.syms.sym(call.sym).name) && ts.len() + explicit_from == sig.tparams.len() => ts,
                other => {
                    self.macro_targs = other;
                    Vec::new()
                }
            },
        };
        // The owner's substitution, empty for a receiver of no type parameters, which is not copied.
        let mut subst = if call.owner_subst.is_empty() { Subst::new() } else { call.owner_subst.clone() };
        // The owner's type arguments are seen from the receiver chain's outer instances as the
        // signature is (`Match$[AggregationOps.this.pack.type]` of an object nested in a class).
        if ctor.is_none() && !self.outer_prefixes.is_empty() {
            self.owner_subst_from_outer(&call, &mut subst);
        }
        if let Some(entry) = stand_in {
            subst.push(entry);
        }
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
            // A member that takes no parameters at all applies its result's `apply`.
            if ctor.is_none() && sig.tparams.is_empty() && sig.clauses.is_empty() {
                if let Some(r) = self.apply_result_targs(&call, targs, &lists, span, expected, trial) {
                    return r;
                }
            }
            self.error(span, "too many type arguments");
        }
        // A bound of a clause after the extension's receiver may name it (`Elems <:
        // p.MirroredElemTypes`, `Mirror.fromProductTyped`): the receiver's path is in it, as dotty
        // instantiates the clause once the receiver is applied.
        let recv_path = match call.ext_recv {
            Some((recv, recv_ty)) if !sig.tparams.is_empty() => self.receiver_bound_path(&sig, ext_clauses, recv, recv_ty),
            _ => None,
        };
        for (i, &tp) in sig.tparams.iter().enumerate() {
            let (upper, lower) = (self.syms.tparam(tp).upper, self.syms.tparam(tp).lower);
            let (upper, lower) = match recv_path {
                Some(path) => (self.subst_paths(upper, &[path]), self.subst_paths(lower, &[path])),
                None => (upper, lower),
            };
            // A bound naming the owner's `this` (`Tuple.head[This >: this.type]`) is seen from
            // the receiver, as the signature is.
            let (upper, lower) = match call.prefix {
                Some(prefix) if self.types.has_paths(upper) || self.types.has_paths(lower) => match self.class_of(prefix) {
                    Some(c) => {
                        let inserted = stand_in.map_or(prefix, |(q, _)| self.types.param(q));
                        (self.member_seen_from(upper, inserted, c, call.sym), self.member_seen_from(lower, inserted, c, call.sym))
                    }
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
                    let msg = self.kind_mismatch(var, tp);
                    self.error(span, msg);
                }
                is_explicit = !self.types.contains_app_param(var);
            }
            // The variable keeps the bounds its parameter declares, which `instDirection` tells
            // the constraint's apart from.
            if let Type::Var(v) = self.types.get(var) {
                if upper != ANY || lower != NOTHING {
                    let u = if upper != ANY { self.types.subst(upper, &subst) } else { ANY };
                    let l = if lower != NOTHING { self.types.subst(lower, &subst) } else { NOTHING };
                    self.tvars[v].set_declared(l, u);
                }
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
            // Through a leading using parameter the receiver's type is open until that given is
            // found (`receiver_param_type`).
            let leading: Vec<SymId> = sig.clauses[..idx].iter().flat_map(|c| c.params.iter().map(|p| p.sym)).collect();
            let pty = self.approx_paths(pty, &leading, &mut Vec::new());
            let mark = self.snapshot();
            if !self.is_sub(recv_ty, pty) {
                self.rollback(mark);
                if trial {
                    return None;
                }
                let msg = format!(
                    "extension method {} is not applicable to {}",
                    self.name_str(self.syms.sym(call.sym).name),
                    self.show(self.widen_lit(recv_ty))
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
        // An inline method declared without a result type has the one its body infers at the
        // definition, before any call (dotty's `Namer.inferredResultType`): what is expected
        // constrains the call's type arguments by it (`NoOp.prepare[E]` given for a
        // `Shader[Env, Unit]`), where the expansion types the call itself.
        if ret_ty == ERROR && ctor.is_none() {
            if let Some(t) = self.inline_inferred_result(call.sym) {
                let mut with_owner = subst.clone();
                with_owner.extend(call.owner_subst.iter().copied());
                ret_ty = self.types.subst(t, &with_owner);
            }
        }
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
        // A result that names no parameter is matched with the expected type before the
        // arguments are typed, whatever paths the parameters name (a `using trace:
        // Tracer.instance.Type`) and whatever other paths it names (an inner class's `Test.this`
        // in `Tree[A]`), as dotty constrains a result that is not dependent (`isResultDependent`).
        let names_param = self.types.has_paths(sig.ret) && {
            let params: Vec<SymId> = sig.clauses.iter().flat_map(|c| c.params.iter().map(|p| p.sym)).collect();
            self.types.names_term(sig.ret, &params)
        };
        let constrain_by_expected = explicit_lists == normal_clauses && !names_param;
        if constrain_by_expected && explicit_lists <= 1 {
            open_expected = self.constrain_result_by_expected(&mut ret_ty, expected);
        }
        // A partial application, whose clauses after the written ones eta-expansion makes a
        // function of, relates that function type to what is expected once its last written clause
        // is reached, as dotty's `constrainResult` compares a result that is a method type
        // `normalize`d to a function type (`ProtoTypes.normalize`): `standardErrorsF { (a, b) => .. }`
        // against a `(Try[P], Try[T]) => ..` fixes `P` and `T` before the lambda is typed.
        let partial = explicit_lists >= 1 && explicit_lists < normal_clauses && !names_param && !dependent;
        if partial && explicit_lists == 1 {
            self.constrain_partial_by_expected(&sig, &subst, recv_clause, explicit_lists, ret_ty, expected);
        }

        let mut call = call;
        // dotty types an application clause by clause, the innermost first (`realApply`,
        // Applications.scala 1315): where the first plain clause fails, the clauses after it
        // are no part of the application. They are typed once, after it: by the retry on the
        // qualifier that takes the application over (`retry_on_qualifier`), or alone (1438).
        let mut stopped = false;
        let hoist_mark = self.hoisted.len();
        // The application's plan, made where a clause leaves an argument out or reorders its
        // arguments (`typer::elaborate`).
        let mut plan: Option<super::elaborate::CallPlan> = None;
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
        // The context function an expansion's result is expected to be, when its using clauses
        // are left: its parameters, givens to them, and their types.
        let mut eta_context: Option<(Vec<SymId>, Vec<TypeId>)> = None;
        // Definition-site givens are positioned at the end of the argument lists applied so far.
        let mut given_site = span;
        let mut normal_seen = 0;
        for (ci, clause) in sig.clauses.iter().enumerate() {
            if Some(ci) == recv_clause {
                let (recv, recv_ty) = call.ext_recv.unwrap();
                // Through a leading using parameter the receiver was taken at an approximation
                // of its type; with the givens found it is checked at the type their paths give
                // it, as dotty's `extMethodApply` types the receiver as the next argument of the
                // method applied to them (Applications.scala 2969, `tryApplyingExtensionMethod`).
                if ci > 0 && self.param_paths.len() > paths_mark {
                    let leading: Vec<SymId> = sig.clauses[..ci].iter().flat_map(|c| c.params.iter().map(|p| p.sym)).collect();
                    let pty = self.param_type(clause.params[0].ty, &subst);
                    if self.types.has_paths(pty) && !self.types.names_term(pty, &leading) {
                        let mark = self.snapshot();
                        if !self.is_sub(recv_ty, pty) {
                            self.rollback(mark);
                            let msg = format!(
                                "value {} is not a member of {}: the extension method takes {}",
                                self.name_str(self.syms.sym(call.sym).name),
                                self.show(recv_ty),
                                self.show(pty)
                            );
                            self.error(span, msg);
                        }
                    }
                }
                // A receiver of a primitive parameter's type (`extension (i: Int)`) is the primitive.
                let recv = if !self.jvm && self.is_primitive_type_id(clause.params[0].ty) { self.unbox_erased(recv, recv_ty) } else { recv };
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
                    if self.index.is_some() && ctor.is_none() {
                        self.index_applied(&list, call.sym);
                    }
                    let first_arg = args_out.len();
                    let outer = std::mem::replace(&mut self.note_paths, dependent);
                    let is_java = self.is_java_method(call.sym);
                    let java = std::mem::replace(&mut self.java_varargs, is_java);
                    let mut written = super::elaborate::ClauseArgs { inherit: ctor.is_none().then_some((call.sym, first_arg)), ..Default::default() };
                    self.type_clause_args_planned(clause, list, &subst, &mut args_out, Some(&mut written), false);
                    self.java_varargs = java;
                    if written.planned() {
                        let p = plan.get_or_insert_with(|| self.call_plan(&ctor, call.sym, &sig));
                        self.elaborate_written(p, &mut call.recv, &mut args_out, first_arg, &written, clause, &subst, span);
                    }
                    self.note_paths = outer;
                    if dependent {
                        ret_ty = self.subst_param_paths(clause, &args_out[first_arg..], ret_ty, &mut subst);
                    }
                } else {
                    // An expansion whose result is expected to be a context function takes the
                    // using clauses left as one: dotty types the expansion's body against it,
                    // and `typedUnadapted` makes the body that closure (`makeContextualFunction`),
                    // whose parameters the clauses' search finds (`(x: A) => (c: C) ?=> m(x)(using
                    // c)` for an `m(x: A)(using C)` where an `A => C ?=> R` is expected).
                    if !eta.is_empty() && eta_context.is_none() && lists.peek().is_none() && sig.clauses[ci..].iter().all(|c| c.is_using) {
                        let expected_ret = eta_ret_expected.unwrap_or(ERROR);
                        if let Some((ptys, ret)) = eta_ret_expected.and_then(|r| self.as_context_function(r)) {
                            self.push_scope();
                            let syms: Vec<SymId> = ptys
                                .iter()
                                .enumerate()
                                .map(|(i, &t)| {
                                    let sym = self.indexed_local("contextual", i as u32 + 1, t, span);
                                    self.bind_given(sym);
                                    sym
                                })
                                .collect();
                            eta_ret_expected = Some(self.context_result(expected_ret, &syms, ret));
                            eta_context = Some((syms, ptys));
                        }
                    }
                    // An expansion's result is the expected function's before its using clause is
                    // searched, as dotty constrains a result by its prototype ahead of the implicit
                    // arguments (`Application.init`'s `constrainResult`, `adaptNoArgsImplicitMethod`'s
                    // deepened prototype): tyrian's `Sub.emit[F[_]: Sync, Msg]` expanded where an
                    // `A => Sub[IO, A]` is expected searches `Sync[IO]`.
                    if !eta.is_empty() && eta_context.is_none() {
                        if let Some(r) = eta_ret_expected.filter(|&r| r != ERROR && !self.types.contains_error(r)) {
                            let mark = self.snapshot();
                            if !self.is_sub(ret_ty, r) {
                                self.rollback(mark);
                            }
                        }
                    }
                    let clause_first = args_out.len();
                    let mut determined = self.vars_of_applied_clauses(&sig.clauses[..ci], &subst);
                    // The receiver's conversion's open variables too, those the using clause names
                    // (`EncoderOps[?A](xs).toJson(using JsonEncoder[?A])` takes `xs`'s type), as
                    // dotty's `tvarsInParams` selects a prefix's variables; one it does not name stays
                    // open for what follows (cats' `pure[F]`).
                    if !self.conversion_open_vars.is_empty() {
                        let open: Vec<TVarId> = self.conversion_open_vars.iter().copied().filter(|&v| self.tvars[v].inst.is_none()).collect();
                        determined.extend(open);
                    }
                    let mut defaulted: Vec<usize> = Vec::new();
                    for (k, p) in clause.params.iter().enumerate() {
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
                            // No given: the parameter's default, as scalac falls back to it, the
                            // clause applied again with the givens found as named arguments and
                            // this one left out (`Typer.adaptNoArgsImplicitMethod`), which the
                            // clause's plan completes below.
                            // A default of its own or one a member it overrides declares
                            // (`SymDenotations.hasDefaultParams`).
                            None if (p.has_default || (ctor.is_none() && self.default_declarer(call.sym, clause_first + k).is_some())) && self.given_ambiguity.is_none() => {
                                self.failed_givens.clear();
                                defaulted.push(k);
                                args_out.push(self.prog.add(TExpr::Unit));
                            }
                            // The tag of a std member's `@jvmEvidence` that the std's own bodies and a
                            // jar's find none of is taken on trust: `null`, the standard allocation
                            // (`std/buffer.scala`'s `evidenced`); for a member a class outside the std
                            // may override (zio's `Chunk.toArray`), which asks the tag itself, the tag
                            // of `AnyRef`, the array an erased element makes.
                            None if ctor.is_none() && self.syms.sym(call.sym).jvm_evidence && self.syms.sym(call.sym).erased_tags == 0 && self.is_class_tag(target) && self.evidence_on_trust() => {
                                self.failed_givens.clear();
                                self.given_ambiguity = None;
                                let any_ref = match self.sites.class_tag.filter(|_| self.overridable_member(call.sym)) {
                                    Some(tag_class) => self.any_ref_class_tag(tag_class),
                                    None => None,
                                };
                                let tag = any_ref.unwrap_or_else(|| self.prog.add(TExpr::Null));
                                self.prog.set_type(tag, target);
                                args_out.push(tag);
                            }
                            None => {
                                let missing_tag = self.missing_tag(p.ty, target, open_tag);
                                // A `@jvmEvidence` member's tag that no search finds is scalac's E172 of
                                // the array's construction, at the call where it stands.
                                let tag_at_call = missing_tag.is_some() && ctor.is_none() && self.syms.sym(call.sym).jvm_evidence;
                                let target = self.solve_bounded_in(target);
                                let msg = self.given_ambiguity.take().or(missing_tag);
                                let msg = msg.or_else(|| self.param_not_found_message(call.sym, ctor.as_ref().map(|&(c, _)| c), p.sym, p.name, &subst));
                                let msg = match msg.or_else(|| self.implicit_not_found_message(target)) {
                                    Some(m) => m,
                                    None => format!("no given instance of type {} was found for parameter {}", self.show(target), self.name_str(p.name)),
                                };
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
                                let at = if tag_at_call { span } else { Span::new(at, at) };
                                self.given_failure_error(at, msg, target);
                                args_out.push(self.prog.add(TExpr::Unit));
                            }
                        }
                    }
                    if !defaulted.is_empty() {
                        let order: Vec<usize> = (0..clause.params.len()).filter(|k| !defaulted.contains(k)).collect();
                        let written = super::elaborate::ClauseArgs { inherit: None, omitted: defaulted, order };
                        let p = plan.get_or_insert_with(|| self.call_plan(&ctor, call.sym, &sig));
                        self.elaborate_written(p, &mut call.recv, &mut args_out, clause_first, &written, clause, &subst, span);
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
                        open_expected = self.constrain_result_by_expected(&mut ret_ty, expected);
                    }
                    if partial && explicit_lists > 1 && normal_seen == explicit_lists {
                        self.constrain_partial_by_expected(&sig, &subst, recv_clause, explicit_lists, ret_ty, expected);
                    }
                    given_site.end = list.span.end;
                    if self.index.is_some() && ctor.is_none() {
                        self.index_applied(&list, call.sym);
                    }
                    self.last_arg_types.clear();
                    let first_arg = args_out.len();
                    let outer = std::mem::replace(&mut self.note_paths, dependent);
                    let before = self.diags.items.len();
                    let is_java = self.is_java_method(call.sym);
                    let java = std::mem::replace(&mut self.java_varargs, is_java);
                    let mut written = super::elaborate::ClauseArgs { inherit: ctor.is_none().then_some((call.sym, first_arg)), ..Default::default() };
                    self.type_clause_args_planned(clause, list, &subst, &mut args_out, Some(&mut written), false);
                    self.java_varargs = java;
                    if written.planned() {
                        let p = plan.get_or_insert_with(|| self.call_plan(&ctor, call.sym, &sig));
                        self.elaborate_written(p, &mut call.recv, &mut args_out, first_arg, &written, clause, &subst, span);
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
                    // Applied, it ends what a try of a member's retry decides on (`retry_applied`).
                    if normal_seen == 1 && self.retry_depth.is_some_and(|d| d + 1 == self.app_depth) {
                        self.retry_applied = Some((self.diags.items.len() as u32, self.error_nodes));
                    }
                }
                Some(list) => {
                    self.error(list.span, "unexpected using clause");
                }
                None => {
                    // Missing argument list: eta-expand over this clause and the ones after it.
                    let outer_arity = eta.first().map_or(clause.params.len(), |(syms, _)| syms.len());
                    let mut exp_fn = expected.and_then(|t| self.expected_function(t, outer_arity));
                    // Against a type that is no function type (a SAM), a method of no parameters
                    // or of one repeated is not expanded: it is applied or is an error (dotty's
                    // `adaptNoArgs`, its arity -1). So a method with `()` is expanded to a function
                    // type alone, never to a trait or class with one method: named bare where one is
                    // expected, a Java method is called (`new InputStreamReader(p.getInputStream)`,
                    // `InputStream` having one abstract method) and a Scala one is reported.
                    let n = clause.params.len();
                    let sam_arity = n > 1 || (n == 1 && !clause.params[0].repeated);
                    if exp_fn.is_none() && eta.is_empty() && ctor.is_none() && sam_arity {
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
                    let auto = if bare && ctor.is_none() { self.auto_applied(call.sym) } else { AutoApply::No };
                    let auto_applied = auto != AutoApply::No;
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
                    // The expected function's parameter types are the expansion's where it is
                    // fully defined, or the method's types are not (dotty's `adapt1`, the arity
                    // `etaExpand` is given): `xs.map(m)` for an `m(x: Long)` of an `Int` list,
                    // its result open, keeps `Long`.
                    let from_expected = match &exp_fn {
                        Some((eps, r)) => {
                            let tys: Vec<TypeId> = eps.iter().copied().chain(std::iter::once(*r)).collect();
                            let defined = tys.into_iter().all(|t| {
                                let z = self.zonk(t);
                                !self.types.has_vars(z)
                            });
                            defined || clause.params.iter().any(|p| {
                                let t = self.types.subst(p.ty, &subst);
                                self.types.has_vars(t)
                            })
                        }
                        None => false,
                    };
                    let mut syms = Vec::new();
                    let mut tys = Vec::new();
                    let clause_first = args_out.len();
                    for (i, p) in clause.params.iter().enumerate() {
                        if i >= expanded {
                            let ty = self.param_type(p.ty, &subst);
                            let earlier = args_out[..clause_first].to_vec();
                            let pl = plan.get_or_insert_with(|| self.call_plan(&ctor, call.sym, &sig));
                            let d = self.default_application(pl, call.recv, &earlier, clause_first + i, ty);
                            args_out.push(if p.by_name { self.by_name_thunk(d) } else { d });
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
                        // Without an expected parameter type a by-name parameter stays one, as
                        // `EtaExpansion.etaExpand` declares the method's own parameter types:
                        // `twice` is a `(=> Int) => Int`.
                        if p.by_name && expected_param.is_none() {
                            let (s, arg, ty) = self.eta_by_name_param(pty, pty, true, call.sym, (first_index + i) as u32, span);
                            args_out.push(arg);
                            syms.push(s);
                            tys.push(ty);
                            continue;
                        }
                        // Implementing a trait, the expansion takes the parameter types of its
                        // method, which the argument adaptations take to the method's (dotc's
                        // lambda `(x: Int) => m(x)` for an `m(x: Long)`, or `m(convert(x))`); so
                        // does an expected function, whose parameter types the expansion's
                        // parameters take (`etaExpand` leaves them to be inferred): lila's
                        // `me.exists(of(f))` of an `Option[Me]` for an `of(f)(user: User)`, a
                        // `Conversion[Me, User]` in scope.
                        let mut adapted_from = None;
                        if let Some(e) = expected_param {
                            let mark = self.snapshot();
                            if !self.is_sub(e, pty) && (eta_sam.is_some() || from_expected) {
                                self.rollback(mark);
                                let view_compat = std::mem::replace(&mut self.view_compat, true);
                                let fits = self.is_compatible(e, pty);
                                self.view_compat = view_compat;
                                self.rollback(mark);
                                if fits {
                                    adapted_from = Some(e);
                                } else if eta_sam.is_some() {
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
                    if exp_fn.is_none() && !clause.params.is_empty() && ctor.is_some() && explicit_lists > 0 && !copy {
                        self.error(span, "missing argument list for the constructor");
                    }
                    // A vararg method has no function type to expand to unless one is expected.
                    if exp_fn.is_none() && ctor.is_none() && clause.params.iter().any(|p| p.repeated) {
                        let msg = format!("missing argument list for method {}", self.method_description(call.sym));
                        self.error(span, msg);
                    }
                    if auto_applied {
                        if auto == AutoApply::Warned {
                            let msg = format!("method {} must be called with () argument", self.name_str(self.syms.sym(call.sym).name));
                            self.warn(span, msg);
                        }
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
        if eta_context.is_some() {
            self.pop_scope();
        }

        let mut evidence = Vec::new();
        if ctor.is_none() && self.syms.sym(call.sym).erased_tags != 0 {
            let written = lists.next_if(|l| l.using);
            if let Some(list) = &written {
                given_site.end = list.span.end;
                if self.index.is_some() {
                    self.index_applied(&list, call.sym);
                }
            }
            let erroneous_arg = self.erroneous_args[erroneous_mark..].contains(&self.app_depth);
            evidence = self.erased_evidence(call.sym, &sig, &subst, written, erroneous_arg, given_site, span);
        } else if stopped && ctor.is_none() && self.syms.sym(call.sym).jvm_evidence {
            // A first clause that failed leaves the using clause unsearched, but for the array's
            // `ClassTag`, which scalac reports beside the argument's error (its E172 of the array's
            // construction), as the erased evidence's search does.
            let erroneous_arg = self.erroneous_args[erroneous_mark..].contains(&self.app_depth);
            self.unapplied_tags(&sig, &subst, erroneous_arg, given_site, span);
        }
        self.param_paths.truncate(paths_mark);
        if let Some(v) = open_expected {
            self.conform_to_bounds(ret_ty, v, false);
        }
        // A polymorphic method named without a parameter list (`List.empty`, `Map.empty`) is
        // adapted to its type arguments as variables without its tree's interpolation (`Typer.adapt`
        // `readapt(tree.appliedToTypeTrees(typeArgs))`, no `simplify`): the trees around interpolate
        // them, so `foldLeft(List.empty)` leaves the element open for the lambda (a `List[Any]`).
        // An inline method's are instantiated before it expands (`Inliner`'s `isFullyDefined` of
        // the call's type arguments): `p.tail` of scala-library's `tail[This >: this.type]`.
        // Named so, an inline one has no interpolation of its own either: its type arguments are
        // defined fully under `ForceDegree.flipBottom` (`Inliner`), one nothing bounds below taking
        // its upper bound (`meth[R <: Int]: R` expected an `Int` is `meth[Int]`, i7798).
        let poly_ref = normal_clauses == 0 && ctor.is_none() && sig.clauses.is_empty() && !sig.tparams.is_empty();
        if poly_ref && inline_callee {
            self.maximize_bottom_from(first_var);
        } else if poly_ref {
            self.leave_vars_open(first_var, ret_ty);
            self.unsimplified = true;
        }
        self.solve_application_vars(first_var, widen_mark, ret_ty, expected, span);
        let ret_ty = self.zonk(ret_ty);
        let known_dims = ctor.is_none() && self.of_dim_standard_tag(call.sym, &sig, &subst, &mut args_out);
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
            // A transparent call that is a member's argument itself, typed for its cache, is
            // `typedUnadapted`'s call (Typer.scala 3800): its expansion is the adaptation's
            // (`Typer.adapt`, 4819), which each attempt over the argument makes
            // (`Worker::expand_unadapted`), a result type that depends on a parameter included
            // (`Inlines.needsInlining`, 66, asks nothing of it). One inferred as an error keeps
            // the call's expansion here.
            None if inline_callee
                && self.unadapted_call.is_some()
                && self.takes_unadapted(call.sym, span)
                && !self.types.contains_error(ret_ty) =>
            {
                let te = self.build_call(&call, l);
                let arg_types = std::mem::replace(&mut self.inline.arg_types, outer_arg_types);
                ty = self.normalize(ret_ty);
                self.defer_plain_inline(te, &call, &sig, &subst, &args_out, arg_types, ret_ty, ty, span, expected);
                self.unadapted_node = Some(te);
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
                // call among them pending. A transparent method's keeps such a call pending in each
                // copy of its argument (`pend_copy`), which the later phase expands where it
                // survives, as dotty's `Inlining` expands each copy.
                let mut kept_pending = Vec::new();
                if self.attempts.pending_len() != 0 {
                    let mut roots = args_out.clone();
                    roots.extend(call.recv);
                    roots.extend(call.ext_recv.map(|(r, _)| r));
                    let transparent = self.syms.sym(call.sym).mods & crate::ast::mods::TRANSPARENT != 0;
                    match self.copyable_pending_in(&roots).filter(|_| transparent) {
                        Some(nodes) => kept_pending = nodes,
                        None => {
                            self.expand_pending_in(&roots);
                        }
                    }
                }
                let expanded = self.expand_inline(&call, &sig, &subst, &args_out, ret_ty, span, expected);
                self.inline.arg_types = outer_arg_types;
                match expanded {
                    Some((e, t)) => {
                        if !kept_pending.is_empty() {
                            self.drop_pending_copied(&kept_pending, e);
                        }
                        ty = t;
                        e
                    }
                    None => self.build_call(&call, l),
                }
            }
            None => {
                let te = self.build_call(&call, l);
                // scala-library's `erasedValue` is a plain method: a call the trees keep is used.
                if self.syms.sym(call.sym).name == crate::names::ERASED_VALUE && self.intrinsic_of(call.sym) == Some(super::inline::Intrinsic::ErasedValue) {
                    self.note_erased_value(te, span);
                }
                te
            }
        };
        if (self.inline.checking > 0 || self.inline.counting > 0 && !inline_callee) && ctor.is_none() && !sig.tparams.is_empty() {
            self.note_type_args(te, &subst[call.owner_subst.len()..]);
            // A library body's list its pickle inferred is none the source wrote.
            let pickled = targs.is_some_and(|l| self.in_converted_body() && self.cur_ast().inferred_type_lists.binary_search(&l.start).is_ok());
            if self.written_targs(targs) && !pickled {
                self.note_written_type_args(te);
            }
        }
        if self.capturing() && !sig.tparams.is_empty() && !(inline_callee && !self.keeps_inline_calls()) {
            let targs: Vec<TypeId> = subst[call.owner_subst.len()..].iter().map(|&(_, t)| t).collect();
            self.capture_targs(te, &targs);
            for &d in plan.as_ref().map_or(&[][..], |p| &p.defaults[..]) {
                self.capture_targs(d, &targs);
            }
        }
        if self.capturing() && !evidence.is_empty() && !(inline_callee && !self.keeps_inline_calls()) {
            self.capture_evidence(te, evidence);
        }
        if known_dims {
            self.of_dim_known(te, call.sym);
        }
        if let Some(parameter) = summon_names {
            self.inline.copies.push((te, parameter));
        }
        if eta.is_empty() {
            te = self.wrap_hoisted(hoist_mark, te);
            ty = self.skolemized_annotated_result(call.sym, ty);
        } else {
            (te, ty) = self.widen_lambda_result(te, ty, eta_ret_expected, span);
            if let (Some(ret), true, 1) = (eta_ret_expected, eta_sam.is_some(), eta.len()) {
                if ret != self.b.t_unit && eta_sam_mismatch.is_none() && !self.fits_lambda_result(ty, ret) {
                    eta_sam_mismatch = Some((ty, ret));
                }
            }
            if let Some((syms, ptys)) = eta_context {
                let l = self.prog.syms(&syms);
                te = self.prog.add(TExpr::Lambda(l, te));
                ty = self.closure_type(true, &syms, &ptys, ty, span);
                self.prog.set_type(te, ty);
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
            let (found_text, required_text) = self.show_pair(found, required);
            let msg = format!("type mismatch: found {}, required {}", found_text, required_text);
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
            let owned = self.member_call.as_ref().is_some_and(|c| c.depth == depth);
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
            if loops || (empty_parens && next_apply.is_none() && !self.has_companion_apply(ty, call.sym)) {
                if empty_parens && rest.len() == 1 && self.takes_java_parens(call.sym) {
                    return Some((te, ty));
                }
                if loops || !self.has_apply_extension(ty) {
                    self.report_takes_no_parameters(call.sym, span);
                    return Some((te, ty));
                }
            }
        }
        let applied = self.apply_callee(Callee::Value(te, ty), None, rest, span, expected);
        let widen_mark = self.pending_widenings.len();
        self.solve_application_vars(first_var, widen_mark, applied.1, expected, span);
        // The result the next list applies, which signature help reads at the span of the
        // application so far (`factory()(|)`), where some list was applied: its type as the
        // application's variables are solved (`new C(1)(|)` applies a `C[Int]`).
        if self.index.is_some() && given_site.end < span.end {
            let ty = self.zonk(ty);
            self.index_value(Span::new(span.start, given_site.end), te, ty);
        }
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

    /// The extension receiver's parameter with the receiver's path, where a type parameter's
    /// bound names a path (`Elems <: p.MirroredElemTypes`).
    #[cold]
    #[inline(never)]
    fn receiver_bound_path(&mut self, sig: &MethodSig, ext_clauses: usize, recv: TExprId, recv_ty: TypeId) -> Option<(SymId, TypeId)> {
        if !sig.tparams.iter().any(|&tp| self.types.has_paths(self.syms.tparam(tp).upper) || self.types.has_paths(self.syms.tparam(tp).lower)) {
            return None;
        }
        sig.clauses.iter().take(ext_clauses).find(|c| !c.is_using).map(|c| (c.params[0].sym, self.arg_path(recv, recv_ty)))
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

    /// An application's result `x.type @RuntimeChecked` whose argument is no path: the annotated
    /// singleton of a skolem of the argument's type, as dotty's `SkolemType` stands for an unstable
    /// argument in a dependent result, so that two such applications are distinct singletons (an
    /// `if` of `Option(7).runtimeChecked` and `Option(8).runtimeChecked` is their union).
    #[inline]
    pub(super) fn skolemized_annotated_result(&mut self, callee: SymId, ty: TypeId) -> TypeId {
        match self.types.get(ty) {
            Type::Annotated(inner) => self.skolemized_annotated(callee, ty, inner),
            _ => ty,
        }
    }

    #[inline(never)]
    fn skolemized_annotated(&mut self, callee: SymId, ty: TypeId, inner: TypeId) -> TypeId {
        if callee.0 == u32::MAX || self.types.is_path(inner) {
            return ty;
        }
        let ret = self.sig_of(callee).ret;
        let singleton = match self.types.get(ret) {
            Type::Annotated(r) => matches!(self.types.get(r), Type::Term(_)),
            _ => false,
        };
        if !singleton {
            return ty;
        }
        let skolem = self.rec_anchor(inner);
        self.types.mk(Type::Annotated(skolem))
    }

    /// What a parameter's path stands for once an argument is in: the argument's path, the
    /// argument's type where that is a singleton type, its widened type otherwise.
    pub(crate) fn arg_path(&mut self, arg: TExprId, pty: TypeId) -> TypeId {
        if let Some(path) = self.path_of(arg) {
            return path;
        }
        let ty = self.prog.expr_types.get(arg.0).filter(|&t| t != crate::tir::NO_TYPE);
        let ty = ty.unwrap_or(pty);
        let ty = self.deref(ty);
        // A constant an ascription or a plain inline call widened has the widened type for a
        // dependent parameter too (`Elem[T, n.type]` of `(1, "x")((g(0): Int))`).
        let widened = self.widened_constant(arg);
        let ty = if widened { self.widen_lit(ty) } else { ty };
        if self.types.is_path(ty) {
            return ty;
        }
        // A constant's type is its literal type, a stable one that dotty's `safeSubstParam`
        // substitutes as it is: `h(1)` of `def h(a: Int): a.type` is a `1`.
        if !widened {
            if let Some(v) = self.fold_constant_as_typed(arg) {
                return self.types.lit(v);
            }
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

    /// Interpolates the type variables an application made from `first_var` on, as dotty's
    /// `interpolateTypeVars` does at the application's tree (`instantiate_vars` over the result
    /// type and the expected one: a variable the result holds nonvariantly stays open for the trees
    /// around), then applies the numeric widenings its arguments were waiting for. A branch guide
    /// stays open for the join of the branches.
    fn solve_application_vars(&mut self, first_var: usize, widen_mark: usize, ret_ty: TypeId, expected: Option<TypeId>, span: Span) {
        // What was reported under the application is rendered as its typing leaves it.
        if !self.mismatch_renders.is_empty() {
            self.render_mismatches(self.app_depth);
        }
        if first_var < self.tvars.len() {
            self.interpolate_application(first_var, ret_ty, expected, span);
        }
        if widen_mark < self.pending_widenings.len() {
            self.apply_pending_widenings(widen_mark);
        }
    }

    /// `solve_application_vars`'s interpolation, of an application that made variables.
    fn interpolate_application(&mut self, first_var: usize, ret_ty: TypeId, expected: Option<TypeId>, span: Span) {
        let mut qualifying = Vec::new();
        // The variables an inner tree left open, when any did.
        let left: crate::intern::FxMap<TVarId, ()> = if self.open_vars.last().is_some_and(|&w| self.tvars.made_since(w, first_var)) {
            let left_open = self.open_vars.iter().rposition(|&w| !self.tvars.made_since(w, first_var)).map_or(0, |i| i + 1);
            self.open_vars[left_open..].iter().map(|&w| (w, ())).collect()
        } else {
            Default::default()
        };
        for v in first_var..self.tvars.len() {
            let info = &self.tvars[v];
            if info.inst.is_some() {
                continue;
            }
            // Left for the trees around (`leave_vars_open`), or by an inner interpolation.
            if !left.is_empty() && left.contains_key(&self.tvars.id(v)) {
                continue;
            }
            if info.guide {
                continue;
            }
            qualifying.push(self.tvars.id(v));
        }
        if !qualifying.is_empty() && !self.instantiate_vars(&qualifying, first_var, ret_ty, expected) {
            let msg = format!("cannot infer type arguments: conflicting bounds for {}", self.show(ret_ty));
            self.error(span, msg);
        }
    }

    /// The numeric widenings the arguments of an application were waiting for, from `widen_mark` on.
    fn apply_pending_widenings(&mut self, widen_mark: usize) {
        for i in widen_mark..self.pending_widenings.len() {
            let (te, actual, pty) = self.pending_widenings[i];
            let target = self.zonk(pty);
            let ops = match (self.is_numeric(actual), self.is_numeric(target)) {
                (Some(f), Some(t)) if super::prims::widens(f, t) => super::prims::convert_ops(f, t),
                _ => &[],
            };
            if ops.is_empty() && !self.jvm && self.expr_marks.get(&te).is_some_and(|m| m & super::MARK_ERASED_SOLVED != 0) && self.deref(actual) == self.deref(target) {
                // An erased result the parameter is solved to as it stands (`List(m.get(k))`, a
                // lambda's `x => m.get(x)`): unboxed in place, as `unbox_erased` unboxes it where
                // the primitive is expected as written.
                if self.unbox_erased_in_place(te, target) {
                    self.expr_marks.entry(te).and_modify(|m| *m &= !super::MARK_ERASED_SOLVED);
                }
            }
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

    /// The clauses of `sig` after its first `written` plain ones (the receiver's apart) as the curried
    /// function type eta-expansion makes of them over `ret`, related to a function type expected
    /// where that holds: dotty's `normalize` of a method type against a function prototype, which
    /// skips an implicit method's clause for its result (`normalize(resultTypeApprox(mt))`). None
    /// where a by-name or repeated parameter is among them, which eta-expansion treats otherwise.
    fn constrain_partial_by_expected(&mut self, sig: &MethodSig, subst: &Subst, recv_clause: Option<usize>, written: usize, ret: TypeId, expected: Option<TypeId>) {
        let Some(exp) = self.concrete_expected(expected) else { return };
        if self.as_function(exp).is_none() {
            return;
        }
        let plain: Vec<usize> = (0..sig.clauses.len()).filter(|&i| !sig.clauses[i].is_using && Some(i) != recv_clause).collect();
        let Some(&last_written) = plain.get(written - 1) else { return };
        let rest: Vec<&_> = sig.clauses[last_written + 1..].iter().filter(|c| !c.is_using).collect();
        if rest.is_empty() || rest.iter().any(|c| c.params.iter().any(|p| p.by_name || p.repeated)) {
            return;
        }
        let mut fun = ret;
        for c in rest.iter().rev() {
            let params: Vec<TypeId> = c.params.iter().map(|p| self.types.subst(p.ty, subst)).collect();
            fun = self.fun_type(&params, fun);
        }
        let mark = self.snapshot();
        if !self.is_sub(fun, exp) {
            self.rollback(mark);
        }
    }

    /// Relates the result type of an application to what is expected before its arguments are
    /// typed, kept only where it holds; the open variable an expected variable is, for the
    /// bounds to conform to once the arguments are in. A result that holds only with its
    /// wildcards captured becomes the captured type.
    fn constrain_result_by_expected(&mut self, ret: &mut TypeId, expected: Option<TypeId>) -> Option<TVarId> {
        let ret_ty = *ret;
        if let Some(exp) = self.concrete_expected(expected) {
            if exp != self.b.t_unit && exp != ANY {
                let mark = self.snapshot();
                let outer = std::mem::replace(&mut self.necessary_either, true);
                let mut ok = self.is_sub(ret_ty, exp);
                if !ok && self.types.has_vars(exp) {
                    // A wildcard argument of the result (`Collector[T, ?, List[T]]` against an
                    // enclosing call's `Collector[? >: String, A, R]`) relates captured, as
                    // dotty's argument check captures it (`Applications.captureWildcardsCompat`).
                    self.rollback(mark);
                    let captured = self.capture_wildcards(ret_ty);
                    ok = captured != ret_ty && self.is_sub(captured, exp);
                    if ok {
                        *ret = captured;
                    }
                }
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
            // A bound holding a variable an interpolation left open (an earlier element's
            // `Rule[?P]` of a `List(..)`) is the enclosing tree's to settle, as dotty leaves it:
            // relating each result to every such bound made a list of n quadratic.
            if self.mentions_left_open(bound) {
                continue;
            }
            let mark = self.snapshot();
            if !self.is_sub(ret_ty, bound) {
                self.rollback(mark);
            }
        }
    }

    /// Whether `t` names a variable an interpolation left open (`open_vars`).
    fn mentions_left_open(&mut self, t: TypeId) -> bool {
        if self.open_vars.is_empty() || !self.types.has_vars(t) {
            return false;
        }
        let mut vars = Vec::new();
        self.collect_vars(t, &mut vars);
        vars.iter().any(|v| self.open_vars.contains(v))
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

    /// `Array.ofDim[T](dims)` of a `T` whose erasure the type says is an array of that class, its tag
    /// unasked: dotty's `ArrayConstructors` makes it so and drops the evidence passed, unevaluated. The
    /// standard tag of `T` takes the evidence's place, which allocates as that array (`Program::
    /// is_standard_class_tag`).
    /// Whether the call is `Array.ofDim` of an element type whose erasure is known (`of_dim_known`).
    fn of_dim_standard_tag(&mut self, sym: SymId, sig: &MethodSig, subst: &Subst, args: &mut [TExprId]) -> bool {
        if sig.tparams.len() != 1 || args.is_empty() {
            return false;
        }
        let info = self.syms.sym(sym);
        if self.interner.get(info.name) != "ofDim" {
            return false;
        }
        let Owner::Class(owner) = info.owner else { return false };
        if self.syms.class(owner).name != names::ARRAY || !matches!(self.syms.class(owner).owner, Owner::Package(p) if p == self.b.scala_pkg) {
            return false;
        }
        let Some(&tp) = sig.tparams.first() else { return false };
        let Some(elem) = subst.iter().find(|&&(p, _)| p == tp).map(|&(_, t)| self.zonk(t)) else { return false };
        if self.stable_erasure(elem).is_none() {
            return false;
        }
        // A standard tag found by the search (`ClassTag.Int`, `ClassTag(classOf[C])`) stands as it is,
        // the tree scalac pickles before `ArrayConstructors` runs.
        if self.prog.is_standard_class_tag(*args.last().unwrap(), &self.syms, self.interner) {
            return true;
        }
        let Some(tag_class) = self.sites.class_tag else { return true };
        let target = self.types.class(tag_class, &[elem]);
        if let Some(tag) = self.class_tag_given(tag_class, target) {
            *args.last_mut().unwrap() = tag;
        }
        true
    }

    /// dotty's `ArrayConstructors` over `Array.ofDim` of two dimensions or more of an element type
    /// whose erasure is known: `Array.newInstance` over every dimension, a negative one refused before
    /// any array is made. The std's `ofDimKnown` of as many dimensions takes the call's place, the
    /// receiver its first argument; the pickle writes the call as written (`Form::Member`).
    fn of_dim_known(&mut self, te: TExprId, of_dim: SymId) {
        let (recv, args) = match self.prog.expr(te) {
            TExpr::CallMethod(r, _, args) => (r, self.prog.expr_list(args).to_vec()),
            TExpr::CallStatic(_, args) => {
                let Owner::Class(c) = self.syms.sym(of_dim).owner else { return };
                let module = self.prog.add(TExpr::Module(c));
                (module, self.prog.expr_list(args).to_vec())
            }
            _ => return,
        };
        if args.len() < 3 {
            return;
        }
        let name = self.interner.intern("ofDimKnown");
        let scala = self.b.scala_pkg;
        self.demand_std(scala, name, crate::stdindex::TERM);
        let Some(first) = self.syms.pkg(scala).entries.get(&name).and_then(|e| e.term) else { return };
        let alternatives = self.syms.alternatives(first).map_or(vec![first], |a| a.to_vec());
        let Some(helper) = alternatives.into_iter().find(|&a| self.sig_of(a).clauses.iter().map(|c| c.params.len()).sum::<usize>() == args.len() + 1) else { return };
        let mut items = vec![recv];
        items.extend(args);
        let l = self.prog.list(&items);
        self.rewrite_expr(te, TExpr::CallStatic(helper, l));
        if self.capturing() {
            self.capture_form(te, crate::tir::capture::Form::Member(of_dim));
        }
    }

    /// Whether `sym` is a member a class outside the std may override: a class's or a trait's, neither
    /// final, no extension (`Program::evidence_args`'s rule).
    fn overridable_member(&self, sym: SymId) -> bool {
        let info = self.syms.sym(sym);
        !info.is_extension
            && info.mods & crate::ast::mods::FINAL == 0
            && match info.owner {
                Owner::Class(c) => {
                    let class = self.syms.class(c);
                    class.kind != ClassKind::Object && class.mods & crate::ast::mods::FINAL == 0
                }
                _ => false,
            }
    }

    /// Whether `te` is a call or a field read whose declared result erases to a reference
    /// (`erases_to_reference`): of a primitive type, its value may be `null`.
    pub(super) fn is_erased_result(&mut self, te: TExprId) -> bool {
        self.call_sym(te).is_some_and(|s| self.erases_to_reference(s))
    }

    /// The method or val a call or a field read `te` names.
    #[inline]
    pub(super) fn call_sym(&self, te: TExprId) -> Option<SymId> {
        match self.prog.expr(te) {
            TExpr::CallMethod(_, s, _) | TExpr::CallStatic(s, _) | TExpr::Field(_, s) => Some(s),
            TExpr::Js(t, _) => self.prog.template_syms.get(&t).copied(),
            _ => None,
        }
    }

    /// Whether the method's declared result is a type parameter or an abstract type member, which
    /// dotty's `TypeErasure` erases to `Object` whatever a call instantiates it to; an array's own
    /// `apply` erases with its array, a primitive one's to the primitive (`Array[Int]` is `int[]`).
    pub(super) fn erases_to_reference(&mut self, sym: SymId) -> bool {
        let ret = self.sig_of(sym).ret;
        // A type parameter or an abstract type member erases as its upper bound (`TypeErasure`):
        // to the primitive where the bound erases to one, else to `Object`.
        let upper = match self.types.get(ret) {
            Type::Param(p) => self.syms.tparam(p).upper,
            Type::Member(..) => self.member_bounds(ret).1,
            _ => return false,
        };
        if self.erased_primitive(upper).is_some() {
            return false;
        }
        let info = self.syms.sym(sym);
        // The std's `unsafeCast`, a template that is its argument itself: the value as it is,
        // reinterpreted, no call whose result erases; an inline method's call, which is its
        // expansion before any erasure (`constValue` the constant it folds to), its own calls
        // erasing as they are.
        if info.intrinsic.as_deref() == Some("$0") || info.mods & crate::ast::mods::INLINE != 0 {
            return false;
        }
        // A deferred given is called through the implementation the class below gets, of the type
        // that class gives it (`Typer.implementDeferredGivens`), which a pickle names and a lookup
        // of the trait's declaration here does not.
        let array_apply = info.is_extension && info.name == names::APPLY && matches!(info.owner, Owner::Package(p) if p == self.b.scala_pkg);
        !array_apply && !self.is_deferred_given(sym)
    }

    /// The primitive class a type erases to, if one (`TypeErasure`): a primitive class's type or a
    /// constant of one, a type parameter's bound's, and a union's where both sides erase to the same
    /// primitive; a union of two (`Int | Long`) erases to `Object`, the erased lub of its sides
    /// (`TypeComparer.orType` with `isErased`, `TypeErasure.erasedLub`).
    pub(super) fn erased_primitive(&mut self, t: TypeId) -> Option<TypeId> {
        let t = self.dealias(t);
        match self.types.get(t) {
            Type::Union(a, b, _) => {
                let (x, y) = (self.erased_primitive(a), self.erased_primitive(b));
                x.filter(|_| x == y)
            }
            Type::Lit(_) => self.erased_primitive(self.widen_lit(t)),
            Type::Param(p) => {
                let upper = self.syms.tparam(p).upper;
                self.erased_primitive(upper)
            }
            _ => self.is_primitive_type_id(t).then_some(t),
        }
    }

    pub(super) fn build_call(&mut self, call: &MethodCall, args: ListRef) -> TExprId {
        if super::setters::is_concrete_setter(&self.syms, call.sym) {
            if let Some(te) = self.setter_assignment(call, args) {
                return te;
            }
        }
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
            // A given with type or term parameters is a method, one without a lazy val (dotty's
            // `Parsers.givenDef`): only the latter is read as a value.
            let impl_class = info.impl_class;
            if args.is_empty() && !self.is_method_sym(call.sym) {
                return match call.recv {
                    Some(r) => self.prog.add(TExpr::Field(r, call.sym)),
                    None => self.prog.add(TExpr::Static(call.sym)),
                };
            }
            if let Some(impl_class) = impl_class {
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
        if self.syms.sym(call.sym).name == names::INIT {
            // A native class's secondary constructor is JavaScript's constructor called with its
            // arguments, as Scala.js writes it (`new Int16Array(length)`): the class has no body
            // of its own that a factory would run.
            if let Owner::Class(c) = self.syms.sym(call.sym).owner {
                if self.syms.class(c).js == JsKind::Native {
                    return self.prog.add(TExpr::New(c, args));
                }
            }
            // A secondary constructor of a class nested in a class takes the enclosing instance
            // first, as the primary one does (`new_instance`).
            let args = match self.syms.sym(call.sym).owner {
                Owner::Class(c) => match self.outer_class(c) {
                    Some(o) => {
                        let at = self.syms.sym(call.sym).span;
                        let outer = self.new_outer_arg(c, o, at);
                        // A default the creation leaves to the constructor's getter takes the
                        // enclosing instance, its receiver, as the primary's do (`new_instance`).
                        let of = DefaultOf::Method(call.sym);
                        self.retarget_defaults(args, of, of, |w| w.prior_copy(outer));
                        let items: Vec<TExprId> = std::iter::once(outer).chain(self.prog.expr_list(args).iter().copied()).collect();
                        self.prog.list(&items)
                    }
                    None => args,
                },
                _ => args,
            };
            return self.prog.add(TExpr::NewVia(call.sym, args));
        }
        // Only a library body spells a widening through the companion, told before the call.
        if self.cur_ast().reader.is_some() {
            if let Some(te) = self.companion_widening(call, args) {
                return te;
            }
        }
        match call.recv {
            Some(r) => self.prog.add(TExpr::CallMethod(r, call.sym, args)),
            // A local def of the block called across a trait a class owns: through the proxy the
            // class of the block holds (`proxies.rs`).
            None if self.prog.has_block_class_traits() && self.syms.sym(call.sym).owner == Owner::Local && self.syms.sym(call.sym).kind == SymKind::Def => match self.held_holder(call.sym) {
                Some(r) => self.held_call(r, call.sym, args),
                None => self.prog.add(TExpr::CallStatic(call.sym, args)),
            },
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
            // An object's val read through its module, as a product's package member is: the
            // path `Static` gives the same member (`o.v`, dotty's stable `TermRef`).
            TExpr::Field(r, s) => {
                let info = self.syms.sym(s);
                matches!(self.prog.expr(r), TExpr::Module(_)) && info.kind == SymKind::Val && info.mods & (crate::ast::mods::MUTABLE | crate::ast::mods::LAZY) == 0
            }
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
    /// the caller then hoists what it evaluates before them. `defaults`: whether the method
    /// called has default parameters (dotty's `hasDefaultParams`), which picks the lifter.
    pub fn type_clause_args(
        &mut self,
        clause: &ClauseSig,
        list: ArgList,
        subst: &Subst,
        out: &mut Vec<TExprId>,
        defaults: bool,
    ) -> bool {
        self.type_clause_args_planned(clause, list, subst, out, None, defaults)
    }

    /// `type_clause_args` for an application's plan (`Worker::elaborate_clause`): what the
    /// arguments leave to the defaults and the order they were written in go to `plan`, where an
    /// omitted argument holds its place with `()`, and nothing is bound here. A parameter without
    /// a default of its own takes one a member it overrides declares (`plan.inherit`). Without a
    /// plan, `defaults` picks the lifter of reordered arguments (`keep_call_order`).
    pub fn type_clause_args_planned(
        &mut self,
        clause: &ClauseSig,
        list: ArgList,
        subst: &Subst,
        out: &mut Vec<TExprId>,
        mut plan: Option<&mut super::elaborate::ClauseArgs>,
        defaults: bool,
    ) -> bool {
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
            // Positional arguments only, typed in their order, a function literal among them, as
            // `matchArgs` types them (Applications.scala 816): an attempt over the arguments caches
            // each before a later one fails (`cacheTypedArg`).
            let base = out.len();
            out.resize(base + n, TExprId(0));
            for i in 0..n {
                let pty = self.param_type(clause.params[i].ty, subst);
                out[base + i] = self.type_arg(list.args[i], pty, list.span);
                if dependent {
                    self.note_param_path(&clause.params[i], out[base + i], pty);
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
        // The arguments past the parameters: reported, and typed alone for their errors, once the
        // arguments before them are typed, where dotty's `matchArgs` reaches them
        // (Applications.scala, `TooManyArguments`), so that an attempt over the arguments caches
        // those as it does.
        let mut extra: Vec<Option<ExprId>> = Vec::new();
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
                    if skipped {
                        misplaced = true;
                        self.arity_error(&list, "positional after named argument".to_string());
                        if let ArgSrc::Ast(e) = a {
                            self.type_extra_arg(*e);
                        }
                        continue;
                    }
                    if pos >= n && !is_repeated {
                        extra.push(a.ast().filter(|_| matches!(a, ArgSrc::Ast(_))));
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
        // The parameters' arguments typed in the parameters' order, as `matchArgs` types the
        // reordered arguments (Applications.scala 661, 816).
        let mut results: Vec<Option<TExprId>> = vec![None; n];
        for i in 0..n {
            let p = &clause.params[i];
            let pty = self.param_type(p.ty, subst);
            if p.repeated {
                if splice {
                    let seq_ty = match self.seq_class() {
                        Some(seq) => self.types.class(seq, &[pty]),
                        None => ERROR,
                    };
                    self.spread_arg = true;
                    self.spread_was_array = false;
                    let mut spliced = self.type_arg(slots[i][0], seq_ty, list.span);
                    self.spread_arg = false;
                    // The JVM's backend passes a Java method an array of its own.
                    if self.java_varargs && !self.spread_was_array && !self.jvm {
                        spliced = self.java_varargs_copy(spliced, pty);
                    }
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
                    if !self.jvm {
                        for &item in &items {
                            if self.is_erased_result(item) {
                                *self.expr_marks.entry(item).or_default() |= super::MARK_ERASED_SOLVED;
                            }
                        }
                    }
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
                let has_default = p.has_default
                    || match plan.as_deref() {
                        Some(super::elaborate::ClauseArgs { inherit: Some((m, from)), .. }) => {
                            let (m, from) = (*m, *from);
                            self.default_declarer(m, from + i).is_some()
                        }
                        _ => false,
                    };
                if !has_default && !misplaced {
                    let msg = format!("missing argument for parameter {}", self.name_str(p.name));
                    self.arity_error(&list, msg);
                }
                if has_default {
                    // The getter the call applies is used (`CheckUnused`'s default arguments):
                    // the parameter's own, or that of the overridden member that declares it.
                    let declaring = match plan.as_deref() {
                        _ if p.has_default => Some(p.sym),
                        Some(super::elaborate::ClauseArgs { inherit: Some((m, from)), .. }) => {
                            let (m, from) = (*m, *from);
                            self.default_declarer(m, from + i).and_then(|d| self.sig_of(d).clauses.iter().flat_map(|c| c.params.iter()).nth(from + i).map(|q| q.sym))
                        }
                        _ => None,
                    };
                    if let Some(q) = declaring {
                        self.use_default(q);
                    }
                    if let Some(pl) = plan.as_deref_mut() {
                        pl.omitted.push(i);
                    }
                }
                results[i] = Some(self.prog.add(TExpr::Unit));
                continue;
            };
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
        for e in extra {
            self.arity_error(&list, format!("too many arguments: expected {}", n));
            if let Some(e) = e {
                self.type_extra_arg(e);
            }
        }
        let mut results: Vec<TExprId> = results.into_iter().map(|r| r.unwrap()).collect();
        let reordered = match plan {
            Some(pl) => {
                pl.order = order;
                false
            }
            None => self.keep_call_order(&order, &mut results, out, list.span, defaults),
        };
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
                // One the source writes, typed before: a copy's argument for a parameter the call
                // leaves out (`apply_copy`, the receiver's field) is no written argument and has no
                // place.
                ArgSrc::Named(name, te, _) if self.prog.span_of(*te).is_some() => *name,
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

    /// Named arguments are evaluated as written, not in parameter order, as dotty's `Application`
    /// keeps them (Applications.scala 1219): where they are reordered and not every one is safe
    /// (`isSafeArg`: a pure expression, the default getter's value a copy reads off its receiver
    /// included), each is bound to a temporary in call order unless the lifter leaves it in place:
    /// `LiftComplex` for a method with default parameters, which leaves a pure path, else
    /// `LiftImpure`, which leaves a pure expression (EtaExpansion.scala 177-188). The arguments
    /// of the clauses before them are bound before them.
    fn keep_call_order(&mut self, order: &[usize], results: &mut [TExprId], earlier: &mut [TExprId], span: Span, defaults: bool) -> bool {
        let inverted = order.windows(2).any(|w| w[0] > w[1]);
        if !inverted || self.annotation_args {
            return false;
        }
        // Pending plain calls count by their expansions, as where every call expanded as typed.
        let pending: Vec<TExprId> = results.iter().copied().filter(|&e| self.is_pending_call(e)).collect();
        if !pending.is_empty() {
            self.expand_pending_kept(&pending);
        }
        if order.iter().all(|&i| self.is_pure_expr(results[i])) {
            return false;
        }
        let mark = self.hoisted.len();
        let mut done = vec![false; results.len()];
        for &i in order {
            if done[i] {
                continue;
            }
            done[i] = true;
            let kept = if defaults { self.is_pure_path(results[i]) } else { self.is_pure_expr(results[i]) };
            if !kept {
                results[i] = self.hoist(results[i], ANY, span);
            }
        }
        self.hoist_before(mark, earlier, span);
        true
    }

    /// dotty's `isPurePath` (`TreeInfo.exprPurity` at `PurePath`): a literal, `this`, an object,
    /// a reference to a val or a parameter (no `var`, `lazy val` or by-name one), or a val selected
    /// on such a path.
    fn is_pure_path(&self, e: TExprId) -> bool {
        let pure_ref = |s: SymId| {
            let info = self.syms.sym(s);
            info.kind != SymKind::Var && !info.by_name && info.mods & crate::ast::mods::LAZY == 0
        };
        if self.is_pending_call(e) {
            return false;
        }
        match self.prog.expr(e) {
            TExpr::Int(_) | TExpr::Long(_) | TExpr::Double(_) | TExpr::Bool(_) | TExpr::Char(_) | TExpr::Str(_) | TExpr::Unit => true,
            TExpr::This | TExpr::Super(_) | TExpr::Module(_) => true,
            TExpr::Local(s) | TExpr::Static(s) => pure_ref(s),
            TExpr::Field(q, s) => pure_ref(s) && self.is_pure_path(q),
            _ => false,
        }
    }

    /// dotty's `isPureExpr` (`exprPurity` at `Pure` or above): a pure path, a lambda, or a
    /// construction of pure arguments that `isPureApply` takes as pure, a class's whose instance
    /// runs no initialisation (a case class's synthetic `apply` included, `isNoInitsRealClass`).
    fn is_pure_expr(&self, e: TExprId) -> bool {
        if self.is_pure_path(e) {
            return true;
        }
        match self.prog.expr(e) {
            TExpr::Lambda(..) => true,
            TExpr::New(c, args) => self.no_inits_class(c) && self.prog.expr_list(args).iter().all(|&a| self.is_pure_expr(a)),
            _ => false,
        }
    }

    /// Whether constructing `c` runs no initialisation of its own: a class (no trait, no local or
    /// anonymous one) whose template holds only definitions and constructor fields, dotty's
    /// `NoInits` (`Namer`'s `isNoInitMember`: a def, a type, a lazy val, a val without a
    /// right-hand side), read from the source or, for a library's, from its members.
    fn no_inits_class(&self, c: ClassId) -> bool {
        let info = self.syms.class(c);
        if info.kind != ClassKind::Class || info.owner == Owner::Local || info.value_class {
            return false;
        }
        match info.def {
            Some(d) => {
                let ast = self.ast(info.file);
                let crate::ast::DefKind::Class(cls) = &ast.def(d).kind else { return false };
                cls.body.iter().all(|st| match *st {
                    crate::ast::Stmt::Import(_) => true,
                    crate::ast::Stmt::Expr(_) => false,
                    crate::ast::Stmt::Def(m) => match &ast.def(m).kind {
                        crate::ast::DefKind::Fun(_) | crate::ast::DefKind::TypeAlias { .. } => true,
                        // A nested class or trait is a type; an object a val it initialises.
                        crate::ast::DefKind::Class(k) => matches!(k.kind, crate::ast::ClassKind::Class | crate::ast::ClassKind::Trait),
                        crate::ast::DefKind::Val { rhs, .. } => rhs.is_none() || ast.def(m).mods & crate::ast::mods::LAZY != 0,
                        _ => false,
                    },
                })
            }
            None => {
                let ctor: Vec<SymId> = info.ctor_syms.iter().flatten().copied().collect();
                info.members.values().all(|&m| {
                    let s = self.syms.sym(m);
                    !matches!(s.kind, SymKind::Val | SymKind::Var) || s.mods & crate::ast::mods::LAZY != 0 || ctor.contains(&m)
                })
            }
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
        let pty = self.captured_lower_bound(pty);
        if self.b.by_name.is_some() {
            if let Some(inner) = self.by_name_arg(pty) {
                return self.type_by_name_arg(arg, inner, span);
            }
        }
        self.type_arg_as(arg, pty, span)
    }

    /// A parameter's type that is a wildcard argument of the receiver's type, captured
    /// (`capture_wildcards`) with a lower bound (`add` of a `JList[? >: Integer]`): that bound, as
    /// dotty's `asSeenFrom` approximates a wildcard argument in a parameter's contravariant position
    /// (`AsSeenFromMap`'s ranges, their lower ends), so that the argument is typed, and converted,
    /// against it (`add(1)`, `Int` converted to `Integer`), what conforms to it conforming to the
    /// captured type too.
    fn captured_lower_bound(&mut self, pty: TypeId) -> TypeId {
        let t = self.deref(pty);
        let Type::Param(p) = self.types.get(t) else { return pty };
        let info = self.syms.tparam(p);
        if info.name == crate::names::WILDCARD && info.lower != NOTHING { info.lower } else { pty }
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
        // The closure over fresh parameters is of the plain function type, its `apply`'s names
        // and a result naming them left with the instance, as scalac's eta-expansion types it
        // (`f[String]` of a `[A] => (a: A) => a.type` is a `String => String`).
        let _ = fun;
        let ty = self.fun_type(ptys, ret);
        let ps = self.prog.syms(&locals);
        let lambda = self.prog.add(TExpr::Lambda(ps, call));
        self.prog.set_type(lambda, ty);
        let expanded = self.wrap_hoisted(mark, lambda);
        self.prog.set_type(expanded, ty);
        (expanded, ty)
    }

    /// A builtin operator selected with no argument where a function of one parameter is
    /// expected, eta-expanded as dotc expands the method it is a member of: `0 <` under
    /// postfixOps, or `'-'.!=`, is `(a0: Int) => 0 < a0`, an unstable receiver evaluated once
    /// before (Typer.adaptNoArgsUnappliedMethod). None where the operator is no builtin one of
    /// the receiver's type for the parameter's.
    fn eta_builtin_operator(&mut self, recv: TExprId, recv_ty: TypeId, op: Name, span: Span, expected: Option<TypeId>) -> Option<(TExprId, TypeId)> {
        let (ptys, _) = self.as_function(expected?)?;
        let [pty] = ptys[..] else { return None };
        let pty = self.solve_in(pty);
        if matches!(self.types.get(pty), Type::Var(_)) {
            return None;
        }
        let lty = self.widen_path(recv_ty);
        let lty = self.widen_lit(lty);
        let equality = op == names::EQEQ || op == names::NEQ;
        let mark = self.hoisted.len();
        let tl = self.hoist(recv, lty, span);
        let local = self.indexed_local("a", 0, pty, span);
        let tr = self.prog.add(TExpr::Local(local));
        self.prog.set_type(tr, pty);
        let (body, ret) = if equality {
            self.typed_equality(tl, lty, tr, pty, op == names::NEQ, span)
        } else {
            match self.typed_binop(op, tl, lty, tr, pty, span) {
                Some(done) => done,
                None => {
                    self.hoisted.truncate(mark);
                    return None;
                }
            }
        };
        let ty = self.fun_type(&[pty], ret);
        let ps = self.prog.syms(&[local]);
        let lambda = self.prog.add(TExpr::Lambda(ps, body));
        self.prog.set_type(lambda, ty);
        let expanded = self.wrap_hoisted(mark, lambda);
        self.prog.set_type(expanded, ty);
        Some((expanded, ty))
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
            ArgSrc::Ast(e) | ArgSrc::Hoisted(e) if spread => {
                // A spliced sequence is typed and adapted against a sequence or an array of the
                // elements (`typedWildcardStarArgExpr`, Typer.scala 1326: `Seq[T] | Array[? <: T]`),
                // then adapted to the sequence. That typing is the wildcard-star argument's: a
                // transparent call there expands in it and a mismatch with either is its error, so
                // a member's cache keeps the sequence expanded, and not at all where it did not fit.
                let tpt = self.splice_typing_type(pty);
                let (te, ty) = match self.arg_slot(e) {
                    ArgSlot::None => self.type_spliced(e, tpt),
                    ArgSlot::Cached(te, ty) => {
                        self.retry_typed.insert(te, ());
                        (te, ty)
                    }
                    ArgSlot::Held { written, .. } => self.type_spliced_cached(e, written, tpt),
                };
                let arg_span = self.cur_ast().expr_span(e);
                let te = match arg {
                    ArgSrc::Hoisted(_) => self.hoist(te, ty, arg_span),
                    _ => te,
                };
                self.mark_spread_array(spread, te, ty);
                self.typed_arg(te, ty, pty, arg_span)
            }
            ArgSrc::Ast(e) | ArgSrc::Hoisted(e) => {
                // An argument an earlier attempt over the same application typed is adapted, not
                // typed again (`state::ArgCache`, dotty's `typedArg`, ProtoTypes.scala 556).
                // A transparent call a member's cache holds unexpanded is expanded as it is
                // adapted (`expand_unadapted`).
                let (te, ty) = match self.arg_slot(e) {
                    ArgSlot::None => self.type_expr(e, Some(pty)),
                    ArgSlot::Cached(te, ty) => {
                        self.retry_typed.insert(te, ());
                        (te, self.expand_unadapted(te, pty).unwrap_or(ty))
                    }
                    ArgSlot::Held { named: true, written, .. } if matches!(arg, ArgSrc::Ast(_)) => return self.type_named_arg_cached(e, written, pty, spread),
                    ArgSlot::Held { written, .. } => match self.type_arg_cached(e, written, pty) {
                        (te, ty, true) => (te, self.expand_unadapted(te, pty).unwrap_or(ty)),
                        (te, ty, false) => (te, ty),
                    },
                };
                let arg_span = self.cur_ast().expr_span(e);
                if self.interp_call.is_some() {
                    self.lint_interpolation_arg(ty, arg_span, spread);
                }
                let te = match arg {
                    ArgSrc::Hoisted(_) => self.hoist(te, ty, arg_span),
                    _ => te,
                };
                self.mark_spread_array(spread, te, ty);
                self.typed_arg(te, ty, pty, arg_span)
            }
            ArgSrc::Typed(te, ty) | ArgSrc::Named(_, te, ty) => {
                self.mark_spread_array(spread, te, ty);
                // One overload resolution typed ahead is adapted where it was written.
                let at = self.overload_arg_spans.iter().rev().find(|&&(e, _)| e == te).map_or(span, |&(_, at)| at);
                self.typed_arg(te, ty, pty, at)
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
        // parameter is solved: Math.min(someLong, 1) infers Long and widens the 1; a varargs
        // element of an erased result, unboxing (`MARK_ERASED_SOLVED`).
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

    /// The spliced sequence `e` typed against `tpt`, `splice_typing_type`'s, and adapted to it
    /// (`typedWildcardStarArgExpr`'s `typedExpr`, Typer.scala 1326 to 1346, whose result decides
    /// the sequence or array form): kept as it stands where it conforms, converted where it does
    /// not (the conversion's tree and the type it gives, an array's or a sequence's), and
    /// erroneous where no conversion takes it to either member or several do, the mismatch or the
    /// ambiguity reported here; the adaptation to the sequence after it (`typed_arg`, an array
    /// marked spread) adapts that.
    fn type_spliced(&mut self, e: ExprId, tpt: TypeId) -> (TExprId, TypeId) {
        let (te, ty) = self.type_expr(e, Some(tpt));
        if self.types.contains_error(ty) {
            return (te, ty);
        }
        let span = self.cur_ast().expr_span(e);
        let mark = self.snapshot();
        if self.is_sub(ty, tpt) {
            self.rollback(mark);
            return (te, ty);
        }
        self.rollback(mark);
        match self.convert_to_typed(te, ty, tpt, span, true) {
            Some((converted, cty)) if cty != ERROR => (converted, cty),
            Some((te, _)) => (te, ERROR),
            None => {
                let (te, _) = self.adapt_typed(te, ty, tpt, span);
                (te, ERROR)
            }
        }
    }

    /// `type_arg_cached` of a spliced sequence `e`, written `written` (`type_spliced`).
    #[inline(never)]
    fn type_spliced_cached(&mut self, e: ExprId, written: ExprId, tpt: TypeId) -> (TExprId, TypeId) {
        let retained = self.arg_typing_begin(e);
        let diags = self.diags.items.len();
        let typed = self.type_spliced(e, tpt);
        match self.cache_arg(e, typed) {
            Cache::Kept => self.promote_end(retained),
            Cache::Failed => self.note_error_arg(e, written, false, tpt, typed.1, diags),
            Cache::Not => {}
        }
        typed
    }

    /// `Seq[T] | Array[? <: T]` of a splice's sequence type `Seq[T]` (`translateFromRepeated` to
    /// either, Typer.scala's `typedWildcardStarArgExpr` 1315): what its expression is typed
    /// against; `Seq[T]` alone in a quote pattern, which takes no array (dotty's #8680).
    fn splice_typing_type(&mut self, seq: TypeId) -> TypeId {
        if self.in_quote_pattern() {
            return seq;
        }
        let Type::Class(_, args) = self.types.get(seq) else { return seq };
        let &[elem] = self.types.items(args) else { return seq };
        let wild = self.types.bounded_wild(NOTHING, elem);
        let array = self.types.class(self.b.array, &[wild]);
        self.types.union(seq, array)
    }

    /// `cacheTypedArg` (ProtoTypes.scala 471) of the argument `e`, written `written`, that a cache
    /// holds and no attempt cached: typed against `pty` unadapted, cached where the attempt has
    /// reported no error (what the typing wrote is then the argument's, kept through the
    /// attempt's set-aside or retraction), an error argument where it has not and an error lies
    /// inside the typing (`hasInnerErrors`, 450).
    /// Whether the typing left its transparent call pending (`takes_unadapted`), the third.
    #[inline(never)]
    fn type_arg_cached(&mut self, e: ExprId, written: ExprId, pty: TypeId) -> (TExprId, TypeId, bool) {
        let retained = self.arg_typing_begin(e);
        let diags = self.diags.items.len();
        // A transparent call that is the argument, through parentheses (dotty's `Parens` is no
        // tree), is left unexpanded (`takes_unadapted`); one in braces is a block's expression,
        // adapted inside it (`typedBlock`).
        let ast = self.cur_ast();
        let mut at = e;
        let mut braced = ast.is_braced(at);
        while let Expr::Parens(inner) = ast.expr(at) {
            at = inner;
            braced |= ast.is_braced(at);
        }
        let call = (!braced).then(|| (self.env.file, ast.expr_span(at)));
        let outer = std::mem::replace(&mut self.unadapted_call, call);
        let outer_node = self.unadapted_node.take();
        let typed = self.type_expr(e, Some(pty));
        self.unadapted_call = outer;
        let deferred = std::mem::replace(&mut self.unadapted_node, outer_node) == Some(typed.0);
        match self.cache_arg(e, typed) {
            Cache::Kept => self.promote_end(retained),
            Cache::Failed => self.note_error_arg(e, written, false, pty, typed.1, diags),
            Cache::Not => {}
        }
        (typed.0, typed.1, deferred)
    }

    /// `type_arg_cached` of a named argument's value `e`: dotty's typing of the argument
    /// (`typedNamedArg`, Typer.scala 1409) adapts the value to the formal, and the cache keeps
    /// that, which a later attempt adapts again.
    #[inline(never)]
    fn type_named_arg_cached(&mut self, e: ExprId, written: ExprId, pty: TypeId, spread: bool) -> TExprId {
        let retained = self.arg_typing_begin(e);
        let diags = self.diags.items.len();
        let (te, ty) = self.type_expr(e, Some(pty));
        let arg_span = self.cur_ast().expr_span(e);
        self.mark_spread_array(spread, te, ty);
        let adapted = self.typed_arg(te, ty, pty, arg_span);
        let adapted_ty = self.prog.type_of(adapted).unwrap_or(pty);
        match self.cache_arg(e, (adapted, adapted_ty)) {
            Cache::Kept => self.promote_end(retained),
            Cache::Failed => self.note_error_arg(e, written, true, pty, ty, diags),
            Cache::Not => {}
        }
        adapted
    }

    /// The argument `e`, written `written`, typed to the type `own` against `pty` where the
    /// attempt had reported an error, is an error argument where `hasInnerErrors` finds an error
    /// inside it among the diagnostics from `diags` on (`cacheTypedArg`, ProtoTypes.scala 489).
    #[cold]
    #[inline(never)]
    fn note_error_arg(&mut self, e: ExprId, written: ExprId, named: bool, pty: TypeId, own: TypeId, diags: usize) {
        if !self.diags.items[diags..].iter().any(|d| !d.is_warning) {
            return;
        }
        // An entry stands for the diagnostic at its index where the spans agree: a speculation
        // inside the typing may have logged one and discarded its diagnostic.
        let end = self.diags.items.len();
        let logged: Vec<(usize, TypeId)> = self.mismatches.iter().filter(|&&(i, sp, _)| i >= diags && i < end && self.diags.items[i].span == sp).map(|&(i, _, r)| (i, r)).collect();
        let tree = if named { written } else { e };
        if self.has_inner_errors(tree, named, pty, pty, own, diags, &logged) {
            self.arg_cache_error(e);
        }
    }

    /// An argument of no parameter, typed alone for its errors: where a cache holds it, as
    /// `typedArgs` types it (ProtoTypes.scala 508), cached where clean.
    fn type_extra_arg(&mut self, e: ExprId) {
        match self.arg_slot(e) {
            ArgSlot::None => {
                self.type_expr(e, None);
            }
            ArgSlot::Cached(..) => {}
            ArgSlot::Held { .. } => {
                let retained = self.arg_typing_begin(e);
                let typed = self.type_expr(e, None);
                if self.cache_arg(e, typed) == Cache::Kept {
                    self.promote_end(retained);
                }
            }
        }
    }

    /// `type_tuple` of the tupled dual of the arguments `elems` against `pty`: where the
    /// application's cache holds their dual (`FunProto.tupledDual`'s own state), the typing an
    /// earlier attempt cached, or one cached where the attempt has reported no error, an error
    /// argument where it reported one.
    fn type_tupled_dual(&mut self, elems: &[ExprId], pty: TypeId) -> (TExprId, TypeId) {
        match self.dual_slot() {
            None => self.type_tuple(elems, Some(pty)),
            Some(Dual::Typed(te, ty)) => {
                self.retry_typed.insert(te, ());
                (te, ty)
            }
            Some(_) => {
                let retained = self.dual_typing_begin();
                let diags = self.diags.items.len();
                let typed = self.type_tuple(elems, Some(pty));
                if self.cache_dual(typed, diags) {
                    self.promote_end(retained);
                }
                typed
            }
        }
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
                Some(v) => {
                    let v = self.folded_as(v, ty);
                    self.types.lit(v)
                }
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
        // A tuple pattern of as many elements as the function expected takes parameters (dotty's
        // `typedFunctionValue` turns the generator's `x$1 => x$1 match case (a1, .., aN) => e` into
        // a function of N parameters): `for (a, b) <- xs.lazyZip(ys)` over `LazyZip2.foreach`. The
        // pattern matches the tuple of the parameters, as `makeCaseLambda` with N parameters does.
        let untupled: Option<Vec<TypeId>> = match binder {
            ForBinder::Pat(p) | ForBinder::CasePat(p) if ptys.len() > 1 => match self.cur_ast().pat(p) {
                Pat::Tuple(elems) if elems.len as usize == ptys.len() => Some(ptys.iter().map(|&t| self.solve_if_var(t)).collect()),
                _ => None,
            },
            _ => None,
        };
        let elem = match &untupled {
            Some(tys) => self.tuple_type(tys),
            None => self.solve_if_var(ptys[0]),
        };
        if unknown && ptys.len() == 1 && matches!(binder, ForBinder::Pat(_) | ForBinder::CasePat(_)) {
            self.error(span, "cannot infer the element type of this generator");
        }
        // The rest is typed as a written lambda's body is (`type_lambda`): against the result its
        // expected function type gives, an open variable included, and adapted to that variable
        // before the closure is made (`lambda_body_closed`), as dotty types the closure
        // `Desugar.makeFor` writes: `yield raw` under `map[B]` with `B <: ToRaw` takes the view.
        let ret_expected = self.concrete_expected(Some(ret));
        let rest_expected = ret_expected.or(Some(ret));
        let body_mark = self.tvars.len();
        let ast = self.cur_ast();
        let lambda_span = match binder {
            ForBinder::Pat(p) | ForBinder::CasePat(p) => ast.pat_spans[p.idx()],
            ForBinder::Unpack(_) => span,
        };
        if let Some(r) = ret_expected {
            self.lambda_results.insert((self.env.file.0, lambda_span.start, lambda_span.end), r);
        }
        // The closure owns its parameters (`Desugar.makeFor`'s lambda, its `$anonfun`): the
        // scope opens below its owner, as a written lambda's does.
        self.sites.owners.push(super::site::SiteOwner::Lambda(lambda_span));
        let pending = self.attempts.pending_len();
        self.for_bases.push(self.env.frames.len());
        self.push_scope();
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
                for (i, (name, ty, given, original)) in vars.into_iter().enumerate() {
                    let local = self.new_local(name, SymKind::Val, ty, span);
                    // The variable restored stands for the one the comprehension packed.
                    self.note_stand_in(local, original);
                    self.bind_local(name, local);
                    if given {
                        self.syms.sym_mut(local).mods |= crate::ast::mods::GIVEN;
                        self.bind_given(local);
                    }
                    let packed = self.prog.add(TExpr::Local(sym));
                    stmts.push(TStmt::Val(local, self.prog.add(TExpr::Index(packed, i as u32))));
                }
                let (tb, bty) = self.type_for_rest(rest, span, rest_expected);
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
                    self.note_def_at(sym, ast.pat_spans[pat.idx()].start);
                    self.bind_local(name, sym);
                    let (tb, bty) = self.type_for_rest(rest, span, rest_expected);
                    (sym, tb, bty)
                }
                Pat::StableId(path) if matches!(ast.expr(path), Expr::Ident(_)) => {
                    let Expr::Ident(name) = ast.expr(path) else { unreachable!() };
                    let sym = self.new_local(name, SymKind::Val, elem, span);
                    if self.index.is_some() {
                        self.index_local_at(sym, ast.pat_spans[pat.idx()]);
                    }
                    self.note_def_at(sym, ast.pat_spans[pat.idx()].start);
                    self.bind_local(name, sym);
                    let (tb, bty) = self.type_for_rest(rest, span, rest_expected);
                    (sym, tb, bty)
                }
                Pat::Wildcard => {
                    let sym = self.fresh_local("w", elem, span);
                    let (tb, bty) = self.type_for_rest(rest, span, rest_expected);
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
                    let (tb, bty) = self.type_for_rest(rest, span, rest_expected);
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
        let (sym, tb, bty) = result;
        let (tb, bty) = match ret_expected {
            Some(_) => (tb, bty),
            None => self.adapt_open_body(tb, bty, Some(ret), lambda_span),
        };
        let bty = self.lambda_body_closed(body_mark, bty, None, Some(ret), &[sym], lambda_span);
        self.for_bases.pop();
        self.pop_scope();
        self.sites.owners.pop();
        // A yield's value, not typed against the call's result where that is a variable, whose
        // bounds may expect a function of it: what a completion of a name there reads.
        if let (true, None, ForRest::Rest { idx, end, body, is_yield: true }) = (self.index.is_some(), ret_expected, rest) {
            let generators = (idx..end).any(|i| matches!(self.cur_ast().enumerators[i as usize], crate::ast::Enumerator::Gen(..) | crate::ast::Enumerator::CaseGen(..)));
            if !generators {
                self.index_yield_functions(body, tb, ret);
            }
        }
        if let Some(tys) = untupled {
            let xs: Vec<SymId> = tys.iter().enumerate().map(|(i, &t)| self.indexed_local("x", i as u32 + 1, t, span)).collect();
            let refs: Vec<TExprId> = xs.iter().map(|&x| self.prog.add(TExpr::Local(x))).collect();
            let l = self.prog.list(&refs);
            let class = self.tuple_class(tys.len());
            let tuple = self.prog.add(TExpr::New(class, l));
            self.prog.set_type(tuple, elem);
            let stmts = self.prog.stmts.push_slice(&[TStmt::Val(sym, tuple)]);
            let body = self.prog.add(TExpr::Block(stmts, tb));
            let params = self.prog.syms(&xs);
            let ty = self.fun_type(&tys, bty);
            return (self.prog.add(TExpr::Lambda(params, body)), ty);
        }
        let params = self.prog.syms(&[sym]);
        let ty = self.fun_type(&[elem], bty);
        let lambda = self.prog.add(TExpr::Lambda(params, tb));
        // A macro its body defers reads the generator's closure as typed (`deferred_lambdas`).
        if self.attempts.pending_len() > pending {
            self.deferred_lambdas.insert((self.env.file.0, lambda_span.start, lambda_span.end), lambda);
        }
        (lambda, ty)
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
        // A member of a union is its join's (dotty's `findMember` of an `OrType`, `goOr`): a
        // union of one primitive class's constants (`1 | 2`, a join of branches) has the class's.
        let recv_ty = match self.types.get(recv_ty) {
            Type::Union(..) => self.primitive_side(recv_ty).unwrap_or(recv_ty),
            _ => recv_ty,
        };
        self.apply_member_of_class(recv, recv_ty, name, targs, lists, span, expected)
    }

    /// The member `sym` of the receiver's class (seen as `owner_ty`) as a refinement of the
    /// receiver's type declares it, opened at the receiver: dotty's `Types.findMember`
    /// `goRefined`, whose joint denotation keeps the refinement's info where it conforms to the
    /// member's (`rinfo <:< joint.info`) or is the `apply` of a function type's refinement, for
    /// the parameters' names and the result, and the two infos met elsewhere (`Denotations.meet`,
    /// `infoMeet`: the member's parameters, the results intersected, `A & Z`). The call is typed
    /// by it; the class's member is the one called, its defaults its own. None for an overloaded
    /// member and for a refinement of another signature (an overload of the name); `Err` with
    /// the alternatives where the parts' refinements make an overload (`refined_alternatives`).
    fn refined_denotation(&mut self, recv: TExprId, recv_ty: TypeId, name: Name, sym: SymId, owner_ty: TypeId) -> Option<Result<(Refinement, Arc<MethodSig>), Vec<Refinement>>> {
        if !self.types.term_refinements() || matches!(self.types.get(recv_ty), Type::Class(..) | Type::Lit(_) | Type::Error) {
            return None;
        }
        if self.syms.alternatives(sym).is_some() {
            return None;
        }
        // A type parameter's member is its bound's (`findMember` through a `TypeParamRef`'s info).
        let under = match self.types.get(recv_ty) {
            Type::Param(p) if self.syms.tparam(p).upper != ANY => self.syms.tparam(p).upper,
            Type::Param(_) => return None,
            _ => self.widen_path(recv_ty),
        };
        let r = match self.refined_alternatives_on(under, name, recv, under) {
            Ok(r) => r?,
            Err(alts) => return Some(Err(alts)),
        };
        let rsig = self.refinement_sig(r);
        let theirs = self.sig_arc(sym);
        let theirs = match self.dependent_prefix(sym, recv, recv_ty) {
            Some(p) => self.sig_seen_from(theirs, p, sym),
            None => theirs,
        };
        let function_apply = name == names::APPLY && self.class_of(under).is_some_and(|c| self.is_function_class(c) || self.is_context_function_class(c));
        let (r, rsig) = if function_apply || self.refined_info_conforms(&theirs, owner_ty, &rsig) {
            (r, rsig)
        } else {
            let met = self.met_denotation(&theirs, owner_ty, &rsig)?;
            let r = match r {
                // A generic meet's types stand over the member's type parameters, which the
                // refinement's symbol does not bind: the refinement as declared.
                Refinement::Term(..) if !met.tparams.is_empty() => r,
                Refinement::Term(n, s, _) => Refinement::Term(n, s, self.sig_types(&met)),
                Refinement::Val(n, s, _) => Refinement::Val(n, s, met.ret),
                other => other,
            };
            (r, Arc::new(met))
        };
        // The defaults are the member's: scalac finds the getters by the symbol the joint
        // denotation keeps.
        let defaults: Vec<bool> = theirs.clauses.iter().flat_map(|c| c.params.iter().map(|p| p.has_default)).collect();
        let rsig = if defaults.iter().any(|&d| d) {
            let mut own = (*rsig).clone();
            for (p, &d) in own.clauses.iter_mut().flat_map(|c| c.params.iter_mut()).zip(&defaults) {
                p.has_default = d;
            }
            Arc::new(own)
        } else {
            rsig
        };
        Some(Ok((r, rsig)))
    }

    /// The arguments `lists` as they fit the refinement alternative `alt`, typed against its
    /// parameters and taken back, each shown; None where they do not fit. The first parameter
    /// list, positional arguments.
    fn refinement_alternative_fits(&mut self, alt: Refinement, lists: &[ArgList]) -> Option<Vec<String>> {
        let sig = self.refinement_sig(alt);
        let (Some(clause), Some(list)) = (sig.clauses.iter().find(|c| !c.is_using), lists.iter().find(|l| !l.using)) else {
            return Some(Vec::new());
        };
        if !clause_accepts(clause, list.args.len(), false) || clause.params.iter().any(|p| p.repeated) {
            return None;
        }
        let vars: Subst = sig.tparams.iter().map(|&p| (p, self.fresh_var())).collect();
        let mark = self.attempt();
        let mut shown = Some(Vec::new());
        for (arg, p) in list.args.iter().zip(&clause.params) {
            let pty = self.types.subst(p.ty, &vars);
            let ty = match arg {
                ArgSrc::Ast(e) | ArgSrc::Hoisted(e) => self.type_expr(*e, Some(pty)).1,
                ArgSrc::Typed(_, ty) => *ty,
                _ => {
                    shown = None;
                    break;
                }
            };
            if self.attempt_failed(&mark) || !self.is_sub(ty, pty) {
                shown = None;
                break;
            }
            let t = self.solve_bounded_in(ty);
            let text = self.show(t);
            if let Some(v) = shown.as_mut() {
                v.push(text);
            }
        }
        self.retract(mark);
        shown
    }

    /// scalac's E051 for a call among the alternatives that a refinement overload makes:
    /// dotty builds each as `TermRef(prefix, name, denot)`, interned by the member's symbol
    /// (`designatorFor`), so they are one reference holding the last one's denotation
    /// (`withDenot`), the rightmost part's. Arguments that fit it fit every alternative, which
    /// `adaptOverloaded` reports ambiguous; others leave the call to the member's info.
    fn refinement_overload_ambiguity(&mut self, sym: SymId, owner_ty: TypeId, alts: &[Refinement], lists: &[ArgList], span: Span) -> bool {
        let Some(&last) = alts.last() else { return false };
        let Some(args) = self.refinement_alternative_fits(last, lists) else { return false };
        let theirs = self.sig_arc(sym);
        let subst = self.owner_subst(owner_ty);
        let mut seen = (*theirs).clone();
        for p in seen.clauses.iter_mut().flat_map(|c| c.params.iter_mut()) {
            p.ty = self.types.subst(p.ty, &subst);
        }
        seen.ret = self.types.subst(seen.ret, &subst);
        let mut out = format!("Ambiguous overload. The overloaded alternatives of {} with types\n {}", self.method_description_of(sym), self.sig_text(&seen));
        for &a in alts {
            let sig = self.refinement_sig(a);
            out.push_str("\n ");
            out.push_str(&self.sig_text(&sig));
        }
        out.push_str(&format!("\nall match arguments ({})\n\nNote: Overloaded definitions introduced by refinements cannot be resolved", args.join(", ")));
        self.error(span, out);
        true
    }

    /// The member's info `theirs` (its owner seen as `owner_ty`) met with the refinement's `rsig`
    /// of one signature (`Denotations.meet` of the two, `infoMeet`): the member's parameters
    /// and type parameters, the refinement's read as them, the bounds met, its result `&` the
    /// refinement's (`A & Z`). Type parameters whose bounds the meet changes are fresh ones of
    /// those bounds. None where the two do not match (an overload).
    fn met_denotation(&mut self, theirs: &MethodSig, owner_ty: TypeId, rsig: &MethodSig) -> Option<MethodSig> {
        let subst = self.owner_subst(owner_ty);
        let mut seen = theirs.clone();
        for p in seen.clauses.iter_mut().flat_map(|c| c.params.iter_mut()) {
            p.ty = self.types.subst(p.ty, &subst);
        }
        seen.ret = self.types.subst(seen.ret, &subst);
        let (bounds, params, ret) = self.meet_method_infos(&seen, rsig, &theirs.tparams)?;
        let bounds: Vec<TypeId> = bounds.into_iter().map(|b| self.types.subst(b, &subst)).collect();
        let (tparams, rebound) = match self.rebind_tparams(&theirs.tparams, &bounds) {
            Some((fresh, renaming)) => (fresh, renaming),
            None => (theirs.tparams.clone(), Vec::new()),
        };
        let mut met = seen;
        met.tparams = tparams;
        for (p, t) in met.clauses.iter_mut().flat_map(|c| c.params.iter_mut()).zip(params) {
            p.ty = self.types.subst(t, &rebound);
        }
        met.ret = self.types.subst(ret, &rebound);
        Some(met)
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
                // A variable the receiver's interpolation left open in its type stays open
                // through the member (`type_receiver`); a receiver that is one is instantiated.
                let head = self.deref(recv_ty);
                let solved = if matches!(self.types.get(head), Type::Var(_)) { self.solve_bounded_in(recv_ty) } else { recv_ty };
                self.capture_wildcards_in(recv, solved)
            }
        };
        if recv_ty == ERROR {
            self.type_args_for_errors(&lists);
            return (self.prog.add(TExpr::Unit), ERROR);
        }
        // A receiver of a context function type is applied to the givens in scope before a
        // member other than its `apply` is selected (dotty's `adaptNoArgsOther` against a
        // selection): `players.contains(p).option(x)` of a `contains(a: A): Eq[A] ?=> Boolean`.
        // Most receivers have a class's type that is no context function, told apart first.
        let plain_class = match self.types.get(recv_ty) {
            // The context function class of that many parameters, where the table holds one, is
            // the only one there is.
            Type::Class(c, args) => args == EMPTY_LIST || match self.b.context_functions.get(self.syms.class(c).tparams.len().saturating_sub(1)) {
                Some(&Some(cf)) => cf != c,
                _ => !self.is_context_function_class_memo(c),
            },
            _ => false,
        };
        let lists = if name != names::APPLY && !plain_class {
            match self.member_of_context_function(recv, recv_ty, name, targs, lists, span, expected, converted) {
                Ok(r) => return r,
                Err(lists) => lists,
            }
        } else {
            lists
        };
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
                let applied = self.apply_callee(Callee::Value(recv, recv_ty), targs, lists, span, expected);
                // The function class's `apply` the written name selects, which a lambda applied
                // at once leaves no call of (its body reduced in place).
                if self.index.is_some() {
                    self.index_selected(applied.0, recv_ty, name);
                }
                return applied;
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
            // A `*:` chain's member its tuple class has is selected from the chain cast to that
            // class (`Typer.trySmallGenericTuple`), which its erasure checks where the chain
            // erases to `Product` (a chain ending in the alias `EmptyTuple`).
            let head = self.deref(recv_ty);
            let recv = match self.types.get(head) {
                Type::Class(c, _) if Some(c) == self.b.cons_tuple => self.lower_cast(recv, recv_ty, underlying, span).0,
                _ => recv,
            };
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
                if self.index.is_some() {
                    self.index_selected(r.0, recv_ty, name);
                }
                return r;
            }
        }
        // A parameter's member is the one its declared type has, a deferred inline member's
        // implementation the one its own type's class has.
        let mut member = match self.parameter_member(recv, recv_ty, name) {
            Some(m) => Some(m),
            None => self.selection_member(recv, recv_ty, name).or_else(|| self.product_member(recv_ty, name)),
        };
        // The `apply` a pickle's synthesized companion of a case class declares is the class's
        // creation, as the program's own companion's (which declares none) is typed: its
        // defaults the constructor's getters (`Applications.findDefaultGetter`); so is the
        // application of a companion the typer made, named on an instance (`d.Cmd(2)(..)`).
        if member.is_some() && (name == names::APPLY || !lists.is_empty()) && self.made_companion_member(member, name, !lists.is_empty()) {
            member = None;
        }
        // A product's getter, a member its class has where the whole program's has not, is
        // applied as the source's call has it.
        if member.is_some_and(|(sym, _)| self.loaded.as_ref().is_some_and(|l| !l.product_synthetics.is_empty() && l.product_synthetics.contains_key(&sym))) {
            if let Some(r) = self.default_getter_selection(recv, recv_ty, name, targs, &lists, span) {
                return r;
            }
        }
        // A library body's selection of an overloaded name calls the alternative its TASTy
        // declares, seen from the receiver's type at the class that declares it.
        let mut declared = false;
        if self.declared_call.is_some() {
            if let Some(taken) = member.and_then(|(found, _)| self.take_declared(recv, recv_ty, name, found)) {
                member = Some(taken);
                declared = true;
            }
        }
        if member.is_none() {
            // The receiver's path is taken where a recursive refinement is opened at it alone.
            if let Some(refined) = self.refined_alternatives_on(recv_ty, name, recv, recv_ty).ok().flatten() {
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
            if matches!(name, names::TO_STRING | names::HASH_CODE) {
                if let Some(r) = self.any_member_beside(recv, recv_ty, sym, name, targs, &lists, span) {
                    return r;
                }
            }
            self.check_access(sym, Some(recv_ty), span);
            let sym_mods = self.syms.sym(sym).mods;
            if sym_mods & crate::ast::mods::PRIVATE != 0 {
                self.note_private_selection(sym, recv, recv_ty, false);
            }
            if sym_mods & crate::ast::mods::PROTECTED != 0 {
                self.check_protected_access(sym, recv, recv_ty, span);
            }
            let mut callee = self.member_callee_in(recv, recv_ty, Some(owner_ty), sym);
            let refined = match self.refined_denotation(recv, recv_ty, name, sym, owner_ty) {
                Some(Err(alts)) => {
                    if self.refinement_overload_ambiguity(sym, owner_ty, &alts, &lists, span) {
                        self.type_args_for_errors(&lists);
                        return (self.prog.add(TExpr::Unit), ERROR);
                    }
                    None
                }
                Some(Ok(r)) => Some(r),
                None => None,
            };
            match (&mut callee, &refined) {
                (Callee::Method { refined: slot, .. }, Some((_, sig))) => *slot = Some(sig.clone()),
                // A value's type is the refinement's (`Base { val v: Int }`).
                (Callee::Value(te, ty), Some((_, sig))) if sig.clauses.is_empty() && sig.tparams.is_empty() => {
                    self.prog.set_type(*te, sig.ret);
                    *ty = sig.ret;
                }
                _ => {}
            }
            let fits = || lists.iter().all(|l| l.args.is_empty());
            let (te, ty) = if declared || self.written_targs(targs) || without_lists || (fits() && self.member_accepts(sym, &lists, false, expected)) {
                self.apply_callee(callee, targs, lists, span, expected)
            } else {
                self.apply_member_or_extension(callee, (sym, owner_ty), recv, resolved_ty, name, targs, lists, span, expected, converted)
            };
            if let (Some((r, _)), true) = (refined, self.capturing()) {
                self.capture_refined_selection(te, recv, sym, r);
            }
            return (te, ty);
        }
        // A right-associative extension's arguments bound ahead of its call are placed here.
        let mark = self.hoisted.len();
        // The class among the receiver's that holds an extension of the name, looked for once.
        if let Some(owner) = self.instance_extension_owner(recv_ty, name) {
            if let Some((te, ty)) = self.apply_extension_of_instance_in(owner, recv, recv_ty, name, targs, &mut lists, span, expected) {
                return (self.wrap_hoisted(mark, te), ty);
            }
            if lists.is_empty() && targs.is_none() {
                if let Some(r) = self.eta_extension_of_instance(owner, recv, recv_ty, name, span, expected) {
                    return r;
                }
            }
        }
        let lists = if name == names::APPLY {
            match self.apply_on_object(recv, recv_ty, targs, lists, span, expected) {
                Ok(r) => return r,
                Err(lists) => lists,
            }
        } else {
            lists
        };
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
        // Of an intersection, the part whose class has the class nested (dotty's member lookup
        // of an `AndType`): `Parser.this.Cfg(..)` under `this: Base =>`, `this` a `Parser & Base`.
        let receiver_class = match self.types.named_class(recv_ty) {
            Some(c) => Some(c),
            None => self.intersection_part_nesting(recv_ty, name),
        };
        if let Some(c) = receiver_class {
            let module = self.is_module_class(c) || self.syms.class(c).local_module.is_some();
            // A class nested in the receiver's class or in one it derives from applies through
            // its constructor proxy, a member of the class that declares it
            // (`NamerOps.addConstructorProxies`): on an object, or on an instance whose class it is
            // nested in, which it takes as its outer one (`o.Inner(1)` is `new o.Inner(1)`).
            let nested = match self.syms.class(c).nested.get(&name) {
                Some(&n) => Some(n),
                None => self.inherited_nested_class(c, name),
            };
            // An export's type forwarder of a class applies as the class's companion, a member of
            // the class (`NamerOps.typeConstructorCompanion`, `exported_alias_class`): the class's
            // own creator, the receiver only evaluated.
            if let Some(k) = nested.is_none().then(|| self.class_exported_alias_term(c, name)).flatten() {
                let (te, ty) = self.apply_callee(Callee::Ctor(k), targs, lists, span, expected);
                let case = self.syms.class(k).mods & crate::ast::mods::CASE != 0 || self.syms.class(k).kind == ClassKind::EnumCase;
                if self.deps.is_some() && case {
                    self.deps_node(te, super::deps::Node::Apply(k));
                }
                if self.is_path(recv) {
                    return (te, ty);
                }
                let stmts = self.prog.stmts.push_slice(&[TStmt::Expr(recv)]);
                return (self.prog.add(TExpr::Block(stmts, te)), ty);
            }
            if module || nested.is_some_and(|n| self.outer_class(n).is_some() && self.needs_constructor_proxy(n)) {
                if let Some(nested) = nested {
                    // A class nested in an object nested in a class or in a local object takes the
                    // object, the receiver, as its outer instance (`Product.Getter(n, f)`, a local
                    // enum's class case `E.B(1)`).
                    let inner = self.outer_class(nested).is_some();
                    // The prefix of a creator application is an immutable path, as dotty's
                    // `newExpr` asks of a `new`'s class type (E083); a case class's companion
                    // `apply` makes its own.
                    if inner && self.syms.class(nested).mods & crate::ast::mods::CASE == 0 {
                        self.check_class_prefix(recv, recv_ty, span);
                    }
                    let prefixed = inner || self.new_prefixed;
                    let outer_prefixed = std::mem::replace(&mut self.new_prefixed, prefixed);
                    if inner && self.is_inner_class(nested) {
                        self.ctor_prefix = Some(self.path_of(recv).unwrap_or(recv_ty));
                    }
                    let outer_creator = std::mem::replace(&mut self.creator_receiver, Some((nested, c)));
                    // The enclosing instance, the receiver, known before the arguments: bound
                    // first where it is not idempotent (`Lifter.liftPrefix`).
                    let (outer, before) = if !inner || self.expr_purity(recv).at_least(super::elaborate::Purity::IDEMPOTENT) {
                        (recv, Vec::new())
                    } else {
                        self.bound_once(recv, span)
                    };
                    let outer_creation = std::mem::replace(&mut self.creation_outer, inner.then_some((nested, outer)));
                    let (te, ty) = self.apply_callee(Callee::Ctor(nested), targs, lists, span, expected);
                    self.creation_outer = outer_creation;
                    self.ctor_prefix = None;
                    self.new_prefixed = outer_prefixed;
                    self.creator_receiver = outer_creator;
                    let case = self.syms.class(nested).mods & crate::ast::mods::CASE != 0 || self.syms.class(nested).kind == ClassKind::EnumCase;
                    if self.deps.is_some() && case {
                        self.deps_node(te, super::deps::Node::Apply(nested));
                    }
                    // The instance's type is the class through the receiver, as dotty's
                    // `applyProxyResultType` gives the proxy's `new` the prefix's type.
                    if inner {
                        self.pass_outer(te, outer);
                        let te = self.bind_prefix(te, before);
                        // The class itself is seen from the receiver, and so is an enum's case
                        // widened to its enum, the enum through the companion's enclosing instance
                        // (`a.E` of `a.E.K(1)`).
                        let ty = match self.types.get(ty) {
                            Type::Class(k, _) if k == nested && self.is_inner_class(nested) => {
                                let prefix = self.path_of(recv).unwrap_or(recv_ty);
                                self.nested_type(prefix, ty)
                            }
                            _ => self.as_seen_from(ty, recv_ty, c),
                        };
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
                    let reference = lists.is_empty() && targs.is_none();
                    let (te, ty) = self.apply_callee(callee, targs, lists, span, expected);
                    // The forwarder selected on the exporting object (`Select` of a stable member).
                    if reference {
                        let on = self.expr_purity(recv);
                        self.note_written_ref(te, true);
                        if let Some(p) = self.written_purity.get(&te).copied() {
                            self.written_purity.insert(te, p.min(on));
                        }
                    }
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
            // The `copy` of a case class is a member of what extends it too (scalachess's
            // `Glyph.MoveAssessment.good`, a `MoveAssessment` trait extending the case class
            // `Glyph`), the case class's own.
            if name == names::COPY {
                let case_base = self.syms.class(c).base_types.iter().map(|&(b, _)| b).find(|&b| b != c && self.syms.class(b).mods & crate::ast::mods::CASE != 0);
                if let Some(b) = case_base {
                    let base_ty = self.base_type(recv_ty, b).unwrap_or(recv_ty);
                    return self.apply_copy(recv, base_ty, b, targs, lists, span, expected);
                }
            }
        }
        // A builtin operator is a member of the primitive class, found before an extension or a
        // conversion is (`any2stringadd`'s `+`): selected with no argument where a function is
        // expected, it is eta-expanded.
        // Only where a function is expected, told before the receiver's primitive is.
        if lists.is_empty() && targs.is_none() && expected.is_some_and(|e| self.as_function(e).is_some()) {
            let prim = self.widen_path(recv_ty);
            let prim = self.widen_lit(prim);
            let prim = self.primitive_side(prim).unwrap_or(prim);
            if name == names::EQEQ || name == names::NEQ || self.prim_op_applies(prim, name) {
                if let Some(r) = self.eta_builtin_operator(recv, recv_ty, name, span, expected) {
                    return r;
                }
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
        // `x.m$default$n` in a library body: the application of the getter of the nth
        // parameter of `x.m`, as the plan of a source's call has it (`typer::elaborate`).
        let l = lists.take().unwrap_or_default();
        if let Some(r) = self.default_getter_selection(recv, recv_ty, name, targs, &l, span) {
            return r;
        }
        lists = Some(l);
        // The receiver shown widened (`NotAMember`'s `site.widen`): `Int` for a `0`.
        let msg = format!("value {} is not a member of {}", self.name_str(name), self.show(self.widen_lit(recv_ty)));
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
        let open_from = self.conversion_open_vars.len();
        match self.convert_for_member(recv, recv_ty, name, lists, expected, span)? {
            Ok((converted, ty)) => {
                if let Some(r) = self.prim_op_on(converted, ty, name, targs, lists, span, expected) {
                    self.interpolate_conversion_vars(open_from, r.1);
                    return Some(r);
                }
                self.no_receiver_conversion = true;
                let r = self.apply_member(converted, ty, name, targs, lists.to_vec(), span, expected);
                self.interpolate_conversion_vars(open_from, r.1);
                Some(r)
            }
            Err(()) => {
                self.type_args_for_errors(lists);
                Some((self.prog.add(TExpr::Unit), ERROR))
            }
        }
    }

    /// dotty's `interpolateTypeVars` at the selection a converted receiver serves, the tree that
    /// encloses the conversion: a variable the conversion left open, its result holding it
    /// nonvariantly (`catsSyntaxOptionId(a): OptionIdOps[?A]`), is instantiated where the member's
    /// type holds it covariantly (`.some: Option[?A]`, from below), contravariantly or not at all
    /// (`instantiateTypeVars`), and stays open where that type still holds it nonvariantly (cats'
    /// `pure[F]`: `F[?A]`). The variables are `conversion_open_vars` from `from` on, this
    /// conversion's.
    fn interpolate_conversion_vars(&mut self, from: usize, ty: TypeId) {
        if self.conversion_open_vars.len() <= from {
            return;
        }
        let ty = self.zonk(ty);
        let occurring = if self.types.has_vars(ty) { self.var_variances(ty) } else { Vec::new() };
        // The ones it instantiates leave the list, the others keep their places.
        let mut kept = from;
        for i in from..self.conversion_open_vars.len() {
            let v = self.conversion_open_vars[i];
            if self.tvars[v].inst.is_none() && !occurring.iter().any(|&(w, s)| w == v && s == 0) {
                self.solve_var_directed(v, Some(ty));
            }
            if self.tvars[v].inst.is_none() {
                self.conversion_open_vars[kept] = v;
                kept += 1;
            }
        }
        self.conversion_open_vars.truncate(kept);
    }

    /// The case class whose constructor `C(args)` and `C.apply` mean next to the `apply` methods
    /// of the companion object `recv_ty`.
    /// The companion of the case class `k`: the written one, the one the namer or a pickle made,
    /// or, for a top-level case class without one, the object `Desugar.classDef`'s `companionDefs`
    /// gives it, made the first time a value names it (`ClassInfo::made_companion`): a program of
    /// many case classes names few of their companions as values, its creations applying the class.
    pub(super) fn materialized_companion(&mut self, k: ClassId) -> Option<ClassId> {
        let info = self.syms.class(k);
        if let Some(m) = info.companion {
            return Some(m);
        }
        if info.kind != ClassKind::Class || info.mods & crate::ast::mods::CASE == 0 || !matches!(info.owner, Owner::Package(_)) || self.is_library_class(k) {
            return None;
        }
        if self.shared_class(k) {
            self.with_loader(|w| w.materialize_companion(k))
        } else {
            self.materialize_companion(k)
        }
    }

    fn materialize_companion(&mut self, k: ClassId) -> Option<ClassId> {
        if let Some(m) = self.syms.class(k).companion {
            return Some(m);
        }
        let (name, owner, file, span) = {
            let info = self.syms.class(k);
            (info.name, info.owner, info.file, info.span)
        };
        let cid = self.syms.new_class(name, ClassKind::Object, crate::ast::mods::FINAL, owner, file, None, span);
        let sym = self.syms.new_sym(name, SymKind::Object(cid), 0, owner, file, None, span);
        let ty = self.types.class(cid, &[]);
        {
            let mut s = self.syms.sym_mut(sym);
            s.sig = Some(std::sync::Arc::new(MethodSig::value(ty)));
            s.state().set(crate::symbols::Completion::Done);
        }
        let mut info = self.syms.class_mut(cid);
        info.module_sym = Some(sym);
        info.companion = Some(k);
        info.made_companion = true;
        // Complete as a class without a definition completes (`complete_class_inner`).
        info.base_types.push((cid, ty));
        info.state().set(crate::symbols::Completion::Done);
        self.syms.class_mut(k).companion = Some(cid);
        self.made_companion_to_string(cid);
        Some(cid)
    }

    /// `apply` on an object: `String(bytes, UTF_8)` as `new String`, and a companion object's
    /// application as its class's creation, apart from `apply_member_of_class`'s path, which most
    /// selections take without it. The argument lists back where neither applies.
    #[inline(never)]
    fn apply_on_object(
        &mut self,
        recv: TExprId,
        recv_ty: TypeId,
        targs: Option<ListRef>,
        lists: Vec<ArgList>,
        span: Span,
        expected: Option<TypeId>,
    ) -> Result<(TExprId, TypeId), Vec<ArgList>> {
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
                return Ok(self.apply_member(recv, recv_ty, name, targs, lists, span, expected));
            }
        }
        // `Point(1, 2)` with an explicit companion object that defines no `apply`, selected
        // through a prefix too (`o.Cmd`, the object's class through `o`).
        if let Type::Class(c, _) = self.types.get(self.types.strip_nested(recv_ty)) {
            let info = self.syms.class(c);
            let is_module = info.kind == ClassKind::Object || info.local_module.is_some() || info.inner_object.is_some();
            let inner = info.inner_object;
            if let (true, Some(companion)) = (is_module, info.companion) {
                // A case class's companion, or an enum's class case's (`enter_enum_case_companion`).
                if matches!(self.syms.class(companion).kind, ClassKind::Class | ClassKind::EnumCase) {
                    if let (Some(ta), true) = (targs, lists.is_empty()) {
                        // `G[Int]` of a case class's companion: its `apply` with the type
                        // arguments, expanded as a method that needs arguments is, as the
                        // constructor's proxy is (`Callee::Ctor`).
                        // A written companion's is a summoner's (`summon_type_application`).
                        let cmods = self.syms.class(companion).mods;
                        // The companion `Desugar.classDef` made: the namer's or the parser's.
                        let made = self.syms.class(c).made_companion || self.syms.class(c).synthetic_companion;
                        let case = cmods & crate::ast::mods::CASE != 0 && cmods & crate::ast::mods::ABSTRACT == 0 && made;
                        if case && inner.is_none() && Self::required_arity(&self.syms.class(companion).ctor).is_some() {
                            return Ok(self.apply_companion_apply(companion, targs, Vec::new(), span, expected));
                        }
                        if let Some(r) = self.summon_type_application(companion, None, ta, span, expected) {
                            return Ok(r);
                        }
                    }
                    // The proxy `apply` a companion without one gets has the constructor's
                    // access, protected for a protected class whose companion is not protected
                    // itself, and the class's qualifier (`NamerOps.addConstructorApplies`): out of
                    // reach, but inside the companion and its class, the object takes no
                    // parameters (E050). A case class has no proxy: its companion's `apply` is the
                    // synthesized member (`NamerOps.needsConstructorProxies`), an enum's class case's
                    // too, a case class as `DesugarEnums` writes it.
                    let proxy = self.syms.class(companion).mods & crate::ast::mods::CASE == 0 && self.syms.class(companion).kind != ClassKind::EnumCase;
                    if proxy && (!self.proxy_apply_accessible(companion, c) || !self.is_class_accessible(companion)) {
                        let owner = match self.syms.class(c).owner {
                            Owner::Class(o) => format!(" in {}", self.class_description(o)),
                            Owner::Package(p) if p != crate::symbols::ROOT_PKG && self.syms.pkg(p).name != names::EMPTY => format!(" in package {}", self.name_str(self.syms.pkg(p).name)),
                            _ => String::new(),
                        };
                        let msg = format!("object {}{} does not take parameters", self.name_str(self.syms.class(companion).name), owner);
                        self.error(span, msg);
                        self.type_args_for_errors(&lists);
                        return Ok((self.prog.add(TExpr::Unit), ERROR));
                    }
                    // The synthesized `apply` of a companion nested in a class constructs
                    // inside it (`Desugar.classDef`).
                    if inner.is_some() && self.outer_class(companion).is_some() {
                        return Ok(self.nested_companion_apply(recv, recv_ty, c, companion, proxy, targs, lists, span, expected));
                    }
                    return Ok(self.apply_companion_apply(companion, targs, lists, span, expected));
                }
            }
        }
        Err(lists)
    }

    /// Whether `member`, applied, is a companion's creation rather than itself: the `apply` a
    /// pickle's synthesized companion declares, or the val of a companion the typer made.
    #[cold]
    #[inline(never)]
    fn made_companion_member(&mut self, member: Option<(SymId, TypeId)>, name: Name, applied: bool) -> bool {
        let Some((sym, _)) = member else { return false };
        (name == names::APPLY && (self.is_made_companion_apply(sym) || self.is_synthesized_case_apply(sym))) || (applied && self.is_made_companion_val(sym))
    }

    /// Whether `s` is the synthesized `apply` a pickle declares in a case class's companion, a
    /// product's or a library's alike (`Desugar.classDef`), which the constructor stands for; a
    /// jar's companion nested in a class has its member called (`jar_nested_companion`).
    pub(super) fn is_synthesized_case_apply(&self, s: SymId) -> bool {
        let Owner::Class(m) = self.syms.sym(s).owner else { return false };
        !self.jar_nested_companion(m)
            && self.loaded.as_ref().is_some_and(|l| l.synthetic_members.contains_key(&s))
            && self.syms.class(m).companion.is_some_and(|c| {
                let ci = self.syms.class(c);
                ci.mods & crate::ast::mods::CASE != 0 && ci.mods & crate::ast::mods::ABSTRACT == 0 && (ci.kind == ClassKind::Class || ci.kind == ClassKind::EnumCase && ci.singleton.is_none())
            })
    }

    /// The class of a part of the intersection `t` that has a class `name` nested, its own or
    /// a base's.
    #[cold]
    #[inline(never)]
    fn intersection_part_nesting(&mut self, t: TypeId, name: Name) -> Option<ClassId> {
        let t = self.deref(t);
        let Type::Inter(a, b) = self.types.get(t) else { return None };
        for part in [a, b] {
            let found = match self.types.named_class(part) {
                Some(c) if self.syms.class(c).nested.contains_key(&name) || self.inherited_nested_class(c, name).is_some() => Some(c),
                Some(_) => None,
                None => self.intersection_part_nesting(part, name),
            };
            if found.is_some() {
                return found;
            }
        }
        None
    }

    /// Whether `s` is the val of a companion the typer made for a case class nested in a class
    /// (`ClassInfo::made_companion`), which an application takes as the class's creation.
    fn is_made_companion_val(&self, s: SymId) -> bool {
        let info = self.syms.sym(s);
        if info.kind != SymKind::Val || info.def.is_some() || info.mods & crate::ast::mods::LAZY == 0 {
            return false;
        }
        crate::tir::is_made_companion_val(&self.syms, s)
    }

    /// Whether `s` is the synthesized `apply` of a case class's companion a pickle declares
    /// (`ClassInfo::made_companion`).
    fn is_made_companion_apply(&self, s: SymId) -> bool {
        let Owner::Class(m) = self.syms.sym(s).owner else { return false };
        self.syms.class(m).made_companion && self.loaded.as_ref().is_some_and(|l| l.synthetic_members.contains_key(&s))
    }

    /// Whether the object `o` is a jar's companion nested in a class, whose synthesized `apply` the
    /// typer calls as the member it is (`constructor_next_to_apply`).
    fn jar_nested_companion(&self, o: ClassId) -> bool {
        self.syms.class(o).inner_object.is_some() && self.is_library_class(o) && !self.is_product_class(o)
    }

    /// The case class whose constructor stands for its companion's synthesized `apply`, the
    /// companion the program's or a product's, nested or not, a static library's, an enum's class
    /// case's too: every application of it is a creation through the one path
    /// (`nested_companion_apply`, `apply_companion_apply`), the companion its receiver. A jar's
    /// companion nested in a class keeps its member called, as dotty calls it, its defaults the
    /// constructor's getters (`synthetic_apply_class`): the interpreter runs a jar's bodies, whose
    /// nested classes it constructs only as their own companions do (chimney's `TargetPath`).
    pub(super) fn constructor_next_to_apply(&mut self, recv_ty: TypeId) -> Option<ClassId> {
        let o = self.class_of(recv_ty)?;
        if self.jar_nested_companion(o) {
            return None;
        }
        let info = self.syms.class(o);
        let is_module = info.kind == ClassKind::Object || info.local_module.is_some() || info.inner_object.is_some();
        let c = info.companion.filter(|_| is_module)?;
        let ci = self.syms.class(c);
        // A class case of an enum is a case class as `DesugarEnums` writes it, its companion an
        // object of the enum's (`enum Layer { case Content(..) }` beside `object Content`).
        let case = match ci.kind {
            ClassKind::Class => ci.mods & crate::ast::mods::CASE != 0,
            ClassKind::EnumCase => ci.singleton.is_none(),
            _ => false,
        };
        (case && ci.mods & crate::ast::mods::ABSTRACT == 0).then_some(c)
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
        // With an extension method of the name in scope that may take the receiver, a lone
        // alternative that fits the shape of the call still has to take the arguments, or the
        // extension is meant (dotty's `tryWithImplicitOnQualifier`, which retries the application
        // with an extension whose receiver takes the qualifier): the std's `Array` extension
        // `apply` is no reason to type the arguments of a case class's companion's `apply` ahead
        // of its parameters.
        let lexical = self.lexical_extensions(name);
        let strict = lexical.into_iter().any(|e| self.extension_may_take(e, recv, recv_ty));
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
        let (mut recv, mut recv_ty) = (recv, recv_ty);
        if selectable.is_none() {
            // The qualifier adapted to `Selectable | Dynamic` first, where an imported conversion
            // (`reflectiveSelectable`) makes it one (dotty's `Dynamic.handleStructural`).
            let target = self.b.selectable.map(|s| self.types.class(s, &[]));
            match target.and_then(|t| self.convert_to_typed(recv, recv_ty, t, span, false)) {
                Some((te, ty)) if ty != ERROR => (recv, recv_ty) = (te, ty),
                _ => {
                    let found = self.path_of(recv).unwrap_or(recv_ty);
                    let msg = format!("type mismatch: found {}, required Selectable | Dynamic", self.show(found));
                    self.error(span, msg);
                    return failed(self, &lists);
                }
            }
        }
        // `Dynamic.handleStructural`: a polymorphic member, or a method whose parameter or result
        // types name its parameters, is no structural call.
        if !sig.tparams.is_empty() {
            let msg = format!("Structural access not allowed on method {} because it is polymorphic", self.name_str(name));
            self.error(span, msg);
            return failed(self, &lists);
        }
        if targs.is_some() {
            let msg = format!("method {} does not take type parameters", self.name_str(name));
            self.error(span, msg);
            return failed(self, &lists);
        }
        let params: Vec<SymId> = sig.clauses.iter().flat_map(|c| c.params.iter().map(|p| p.sym)).collect();
        if !params.is_empty() && (sig.clauses.iter().flat_map(|c| c.params.iter()).any(|p| self.types.names_term(p.ty, &params)) || self.types.names_term(sig.ret, &params)) {
            let msg = format!("Structural access not allowed on method {} because it has a method type with inter-parameter dependencies", self.name_str(name));
            self.error(span, msg);
            return failed(self, &lists);
        }
        let key = self.prog.add_str(&self.name_str(name));
        let key = self.prog.add(TExpr::Str(key));
        let mut dynamic_lists = vec![ArgList { args: vec![ArgSrc::Typed(key, self.b.t_string)], using: false, span }];
        let mut lists = lists.into_iter().peekable();
        let mut typed: Vec<ArgSrc> = Vec::new();
        // Every list's arguments, a contextual one's resolved where the call gives none, as dotty's
        // `handleStructural` passes the call's arguments (`vargss.flatten`).
        for clause in sig.clauses.iter() {
            if clause.is_using && !lists.peek().is_some_and(|l| l.using) {
                for p in &clause.params {
                    match self.resolve_given_typed(p.ty, span) {
                        Some((te, _)) => typed.push(ArgSrc::Typed(te, p.ty)),
                        None => {
                            let msg = format!("no given instance of type {} was found for parameter {}", self.show(p.ty), self.name_str(p.name));
                            self.error(Span::new(span.end, span.end), msg);
                            return failed(self, &[]);
                        }
                    }
                }
                continue;
            }
            let Some(list) = lists.next() else {
                let msg = format!("missing argument list for method {}", self.name_str(name));
                self.error(span, msg);
                return (self.prog.add(TExpr::Unit), ERROR);
            };
            let mut out = Vec::new();
            self.type_clause_args(clause, list, &Vec::new(), &mut out, sig.clauses.iter().any(|c| c.params.iter().any(|p| p.has_default)));
            typed.extend(out.into_iter().zip(&clause.params).map(|(te, p)| ArgSrc::Typed(te, p.ty)));
        }
        let mut selected: Option<(SymId, Subst)> = None;
        let dynamic = if sig.clauses.is_empty() {
            names::SELECT_DYNAMIC
        } else {
            // dotty's `structuralCall`: the call `applyDynamic(name)(args)` is typed first, and
            // where the method it selected has a second parameter `Class[?]*` (scala-library's
            // reflective one), `addClassOfs` adds the classes of the parameters' erasures, which
            // the parameter types alone must fix (`handleStructural`'s
            // `TypeErasure.hasStableErasure` and `erasure`); the selected alternative is applied
            // with them.
            let class_class = self.java_lang_class("Class");
            let mut takes_classes = false;
            if let Some((set, _)) = self.find_member(recv_ty, names::APPLY_DYNAMIC) {
                let mut call_lists = dynamic_lists.clone();
                call_lists.push(ArgList { args: typed.clone(), using: false, span });
                if let Some((sym, owner_subst)) = self.selected_alternative(set, recv_ty, &mut call_lists) {
                    let classes_param = match &self.sig_of(sym).clauses[..] {
                        [first, ..] if first.params.len() == 2 && first.params[1].repeated => Some(first.params[1].ty),
                        _ => None,
                    };
                    if let Some(element) = classes_param {
                        let element = self.types.subst(element, &owner_subst);
                        let element = self.dealias(element);
                        takes_classes = matches!(self.types.get(element), Type::Class(c, _) if Some(c) == class_class);
                    }
                    if self.syms.alternatives(set).is_some() {
                        selected = Some((sym, owner_subst));
                    }
                }
            }
            if takes_classes {
                let Some(erased) = self.erased_parameter_classes(&sig) else {
                    let msg = format!(
                        "Structural access not allowed on method {} because it has a parameter type with an unstable erasure",
                        self.name_str(name)
                    );
                    self.error(span, msg);
                    return (self.prog.add(TExpr::Unit), ERROR);
                };
                let class_ty = match class_class {
                    Some(k) => self.types.class(k, &[WILD]),
                    None => ANY,
                };
                for (c, dims) in erased {
                    let class_of = self.prog.add(TExpr::ClassOf(c, dims));
                    self.prog.set_type(class_of, class_ty);
                    dynamic_lists[0].args.push(ArgSrc::Typed(class_of, class_ty));
                }
            }
            dynamic_lists.push(ArgList { args: typed, using: false, span });
            names::APPLY_DYNAMIC
        };
        let (te, _) = match selected {
            // The overloaded `applyDynamic`'s selected alternative, which dotty's tree keeps when
            // the classes join its arguments.
            Some((sym, owner_subst)) => {
                self.check_access(sym, None, span);
                let call = MethodCall { recv: Some(recv), sym, owner_subst, ext_recv: None, prefix: None, refined: None };
                self.apply_method(call, None, None, dynamic_lists, span, None, false).unwrap()
            }
            None => self.apply_member(recv, recv_ty, dynamic, None, dynamic_lists, span, None),
        };
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
        if !empty_parens || self.syms.sym(sym).kind != SymKind::Def || !self.takes_java_parens(sym) {
            return false;
        }
        let ext_clauses = self.syms.sym(sym).ext_clauses as usize;
        let sig = self.sig_of(sym);
        !sig.clauses.iter().skip(ext_clauses).any(|c| !c.is_using)
    }

    /// Whether `f()` calls a parameterless `f`: the method or one it overrides is Java's, `Any`'s
    /// or `Object.clone` (`matchNullaryLoosely`'s test, `nullary_provenance`, a Scala 2 class's
    /// member left out, which takes no `()`: `List(1).length()` is refused): scala-library's
    /// `StringBuilder.length` over `CharSequence.length()`, called either way; a Java method of
    /// other parameters (`charAt(int)` beside a nullary `charAt`) is none it overrides.
    pub(super) fn takes_java_parens(&mut self, sym: SymId) -> bool {
        if sym.0 == u32::MAX {
            return false;
        }
        let mut chain = vec![sym];
        if matches!(self.syms.sym(sym).owner, Owner::Class(_)) {
            self.overridden_chain(sym, 0, &mut chain);
        }
        chain.into_iter().any(|s| self.nullary_provenance(s) == super::check::Nullary::Java)
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
    pub(super) fn scala2_std_member(&self, sym: SymId) -> bool {
        let source = self.source(self.syms.sym(sym).file);
        source.is_std && crate::TESTING_STD_FILES.contains(&source.path.as_str())
    }

    /// An extension of the std on `String` that stands for a member of `java.lang.String`
    /// (`getBytes()`), which takes the Java parentheses rule as the JDK's members do.
    pub(super) fn string_member_of_std(&self, sym: SymId) -> bool {
        let info = self.syms.sym(sym);
        if !info.is_extension || !self.source(info.file).is_std {
            return false;
        }
        let receiver = info.sig.as_ref().and_then(|sig| sig.clauses.first()).and_then(|c| c.params.first()).map(|p| p.ty);
        receiver == Some(self.b.t_string)
    }

    /// A method of Java's, read from a class file or standing for one in a `java.*` package of the
    /// std: dotty's `JavaDefined`, whose varargs take a spread sequence as an array.
    fn is_java_method(&self, sym: SymId) -> bool {
        sym.0 != u32::MAX && (self.syms.sym(sym).java_defined || self.in_java_package(sym))
    }

    /// `javaVarargs(xs)`: the copy of a sequence spread into a Java method's varargs.
    fn java_varargs_copy(&mut self, spliced: TExprId, elem: TypeId) -> TExprId {
        let Some(copy) = self.std_def("javaVarargs") else { return spliced };
        let seq_ty = self.prog.type_of(spliced);
        let args = self.prog.list(&[spliced]);
        let subst = self.syms.sym(copy).sig.as_ref().map_or(Vec::new(), |sig| sig.tparams.iter().map(|&t| (t, elem)).collect());
        let call = MethodCall { recv: None, sym: copy, owner_subst: subst, ext_recv: None, prefix: None, refined: None };
        let te = self.build_call(&call, args);
        if let Some(t) = seq_ty {
            self.prog.set_type(te, t);
        }
        te
    }

    /// Whether `sym` is a member of a class of a `java.*` package of the std, its platform layer
    /// standing for the JDK, whose members take Java's rules (`JavaDefined`), as the JDK's own
    /// read from their class files do; a program's source in such a package is Scala's.
    pub(super) fn in_java_package(&self, sym: SymId) -> bool {
        self.source(self.syms.sym(sym).file).is_std && self.under_java_package(sym)
    }

    /// Whether the top-level package of `sym`'s owner is `java`.
    pub(super) fn under_java_package(&self, sym: SymId) -> bool {
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

    /// Whether a bare `f` of a `def f()` calls it, as dotty's `isAutoApplied` decides
    /// (`Typer.adaptNoArgsUnappliedMethod`): when `matchNullaryLoosely` holds of the method or of
    /// one it overrides, a method of Java's, of `Any` or of a Scala 2 class; with the E100 warning
    /// (`warnScala2`) where one of them is Scala 2's and none it overrides is Java's or `Any`'s.
    fn auto_applied(&mut self, sym: SymId) -> AutoApply {
        let mut overridden = Vec::new();
        if matches!(self.syms.sym(sym).owner, Owner::Class(_)) {
            self.overridden_chain(sym, 0, &mut overridden);
        }
        // A member the std marks Java's (`@javaDefined`, `Math.random()` of the std's `Math`) is
        // no Scala 2 class's, and `clone()`, `toString()` and `hashCode()` override `Object`'s.
        let own = self.nullary_provenance(sym);
        let own_java = own == super::check::Nullary::Java;
        let mut scala2 = own == super::check::Nullary::Scala2;
        let mut java = matches!(self.syms.sym(sym).name, names::TO_STRING | names::HASH_CODE | names::CLONE);
        for &r in &overridden {
            match self.nullary_provenance(r) {
                super::check::Nullary::Java => java = true,
                super::check::Nullary::Scala2 => scala2 = true,
                super::check::Nullary::Strict => {}
            }
        }
        if scala2 && !java {
            AutoApply::Warned
        } else if scala2 || java || own_java {
            AutoApply::Silent
        } else {
            AutoApply::No
        }
    }

    /// A member of a Scala 2 class, which dotty flags `Scala2x`: of a Scala 2 library's
    /// (`Iterator.next()` of scala-library), or of the lean std's standing for one, `Predef`'s
    /// `println()` among them.
    pub(super) fn scala2_member(&self, sym: SymId) -> bool {
        match self.syms.sym(sym).owner {
            Owner::Class(c) => self.is_scala2_class(c) || self.is_std_scala2_class(c),
            _ => self.predef_println(sym),
        }
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
        let call = MethodCall { recv, sym, owner_subst, ext_recv: None, prefix, refined: None };
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

    /// Whether the object of type `ty`, the result of `f`, has the `apply` its companion class gives
    /// it, which an application types as the class's creation (`apply_on_object`) rather than as a
    /// member, and which dotty's `tryInsertApplyOrImplicit` inserts after `f`: a case class's
    /// synthesized one (`Desugar.classDef`'s `companionDefs`) after any `f`, an export's forwarder of
    /// the object (`PgnDump.WithFlags()`) among them; a constructor proxy
    /// (`NamerOps.addConstructorApplies`) only where `f` names the object, its export's forwarder
    /// (dotty's pos `i12299`), not after a method's result (neg `i16095`).
    fn has_companion_apply(&self, ty: TypeId, f: SymId) -> bool {
        let Type::Class(c, _) = self.types.get(self.types.strip_nested(ty)) else { return false };
        let info = self.syms.class(c);
        let is_module = info.kind == ClassKind::Object || info.local_module.is_some() || info.inner_object.is_some();
        let Some(k) = info.companion.filter(|&k| is_module && self.syms.class(k).kind == ClassKind::Class) else { return false };
        let m = self.syms.class(k).mods;
        let case = m & crate::ast::mods::CASE != 0 && m & crate::ast::mods::ABSTRACT == 0;
        case || self.syms.is_forwarder(f) || self.is_pickled_forwarder(f)
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

    /// `Ops.twice` for an extension method `twice` of the object `Ops` named with no argument: the
    /// method its receiver's clause is the first of, eta-expanded (dotc's `etaExpand` of the
    /// extension's method), `(e: String) => Ops.twice(e)`; the receiver's type is the expected
    /// function's parameter where one is expected.
    fn eta_extension_of_instance(&mut self, owner: ClassId, recv: TExprId, recv_ty: TypeId, name: Name, span: Span, expected: Option<TypeId>) -> Option<(TExprId, TypeId)> {
        let exts: Vec<SymId> = self.syms.class(owner).extensions.iter().copied().filter(|&s| self.syms.sym(s).name == name).collect();
        let (param, result) = match expected.and_then(|e| self.as_function(e)) {
            Some((ps, r)) if ps.len() == 1 => (Some(ps[0]), Some(r)),
            _ => (None, None),
        };
        let param = param.map(|p| self.solve_in(p)).filter(|&p| !matches!(self.types.get(p), Type::Var(_)));
        let rty = match (param, &exts[..]) {
            (Some(p), _) => p,
            (None, [ext]) => {
                let ext_clauses = self.syms.sym(*ext).ext_clauses as usize;
                let sig = self.sig_arc(*ext);
                let clause = sig.clauses.iter().take(ext_clauses).find(|cl| !cl.is_using)?;
                let [p] = &clause.params[..] else { return None };
                let owner_ty = self.base_type(recv_ty, owner).unwrap_or(recv_ty);
                let mut subst = self.owner_subst(owner_ty);
                for &tp in sig.tparams.iter() {
                    let v = self.fresh_var();
                    subst.push((tp, v));
                }
                self.types.subst(p.ty, &subst)
            }
            _ => return None,
        };
        let local = self.indexed_local("e", 0, rty, span);
        let arg = self.prog.add(TExpr::Local(local));
        self.prog.set_type(arg, rty);
        let mut lists = vec![ArgList { args: vec![ArgSrc::Typed(arg, rty)], using: false, span }];
        let mark = self.attempt();
        let applied = self.apply_extension_of_instance(recv, recv_ty, name, None, &mut lists, span, result);
        if self.attempt_failed(&mark) {
            self.retract(mark);
            return None;
        }
        self.close(mark);
        let (body, ret) = applied?;
        let rty = self.solve_in(rty);
        let ty = self.fun_type(&[rty], ret);
        let ps = self.prog.syms(&[local]);
        let lambda = self.prog.add(TExpr::Lambda(ps, body));
        self.prog.set_type(lambda, ty);
        Some((lambda, ty))
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
        let owner = self.instance_extension_owner(recv_ty, name)?;
        self.apply_extension_of_instance_in(owner, recv, recv_ty, name, targs, lists, span, expected)
    }

    /// The class among the base classes of the receiver's type that defines an extension method
    /// named `name`.
    #[inline]
    fn instance_extension_owner(&mut self, recv_ty: TypeId, name: Name) -> Option<ClassId> {
        let c = match self.types.get(recv_ty) {
            Type::Class(c, _) => c,
            _ => self.class_of(recv_ty)?,
        };
        self.complete_class(c);
        // A class none of whose ancestors declares an extension, as its completion found, and
        // that declares none itself, has none to look for: read on the class's record, the walk
        // apart from the member call's path.
        let info = self.syms.class(c);
        if info.no_inherited_extensions && info.extensions.is_empty() {
            return None;
        }
        self.extension_owner_among_bases(c, name)
    }

    #[cold]
    #[inline(never)]
    fn extension_owner_among_bases(&mut self, c: ClassId, name: Name) -> Option<ClassId> {
        self.syms.class(c).base_types.iter().map(|&(b, _)| b).find(|&b| {
            self.syms.class(b).extensions.iter().any(|&s| self.syms.sym(s).name == name)
        })
    }

    /// The member of a receiver that is a context function, selected on its result once its
    /// givens are applied (dotty's `adaptNoArgsOther`); the lists back where it is none. Apart
    /// from the member call's path, which most receivers, of a plain class, do not take.
    #[cold]
    #[inline(never)]
    #[allow(clippy::too_many_arguments)]
    fn member_of_context_function(
        &mut self,
        recv: TExprId,
        recv_ty: TypeId,
        name: Name,
        targs: Option<ListRef>,
        lists: Vec<ArgList>,
        span: Span,
        expected: Option<TypeId>,
        converted: bool,
    ) -> Result<(TExprId, TypeId), Vec<ArgList>> {
        let Some((ptys, ret)) = self.as_context_function(recv_ty) else { return Err(lists) };
        let Some((te, ty)) = self.apply_context_function_to(recv, recv_ty, ANY, ptys, ret, span) else { return Err(lists) };
        self.no_receiver_conversion = converted;
        Ok(self.apply_member_of_class(te, ty, name, targs, lists, span, expected))
    }

    #[allow(clippy::too_many_arguments)]
    fn apply_extension_of_instance_in(
        &mut self,
        owner: ClassId,
        recv: TExprId,
        recv_ty: TypeId,
        name: Name,
        targs: Option<ListRef>,
        lists: &mut Vec<ArgList>,
        span: Span,
        expected: Option<TypeId>,
    ) -> Option<(TExprId, TypeId)> {
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
        // The receiver as written, or the parameter of an eta-expansion (`eta_extension_of_instance`).
        let (arg, typed_recv) = match first.args[0] {
            ArgSrc::Ast(arg) => (Some(arg), None),
            ArgSrc::Typed(te, ty) if right_assoc.is_none() => (None, Some((te, ty))),
            _ => return None,
        };
        if arg.map_or(false, |arg| matches!(self.cur_ast().expr(arg), Expr::NamedArg(..))) {
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
        let (te, ty) = match (arg, typed_recv) {
            (Some(arg), _) => self.type_expr(arg, None),
            (None, Some(typed)) => typed,
            (None, None) => return None,
        };
        let ty = self.solve_in(ty);
        let owner_ty = self.base_type(recv_ty, owner).unwrap_or(recv_ty);
        let owner_subst = self.owner_subst(owner_ty);
        let mut rest = lists.split_off(1);
        // Same-named extensions are overloads whose first argument list is the receiver: the
        // receiver decides, and the lists after it only between receivers it leaves equally
        // specific, over every alternative the receiver applies to, on the arguments typed alone
        // (scalac's `resolveOverloaded`, `pretypeArgs`); a receiver more specific than the others
        // wins whatever follows. None applying, or several, is scalac's E134 or E051.
        let ext = if exts.len() > 1 {
            let calls: Vec<MethodCall> = exts
                .iter()
                .map(|&sym| {
                    let prefix = self.dependent_prefix(sym, recv, recv_ty);
                    MethodCall { recv: Some(recv), sym, owner_subst: owner_subst.clone(), ext_recv: Some((te, ty)), prefix, refined: None }
                })
                .collect();
            match self.resolve_extension_overload(&calls, targs, te, ty, &mut rest, expected) {
                Ok(i) => exts[i],
                Err(msg) => {
                    self.error(span, msg);
                    return Some((self.prog.add(TExpr::Unit), ERROR));
                }
            }
        } else {
            exts[0]
        };
        // Seen from the instance it is selected on (`dsl.in("f")(2)` of an `in` that `dsl`'s trait
        // declares), as dotty's `TermRef(dsl.type, in)` is.
        let prefix = self.dependent_prefix(ext, recv, recv_ty);
        let call = MethodCall { recv: Some(recv), sym: ext, owner_subst, ext_recv: Some((te, ty)), prefix, refined: None };
        // The explicit type arguments of a direct call cover the extension's own parameters too.
        self.direct_ext_targs = targs.is_some();
        self.apply_method(call, None, targs, rest, span, expected, false)
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
        self.settle_class(c);
        let info = self.syms.class(c);
        let Some(first) = info.ctor.first().cloned() else {
            return (recv, recv_ty);
        };
        // dotty's `copyMeths` (Desugar.scala): `copy[Ts](p1: T1 = this.p1, ..)(later clauses): C[Ts]`,
        // the defaults on the first clause alone, the later clauses (using ones included) applied
        // as any method's are.
        let mut clauses = info.ctor.clone();
        for (i, cl) in clauses.iter_mut().enumerate() {
            for p in &mut cl.params {
                p.has_default = i == 0;
            }
        }
        let sig = Arc::new(MethodSig { tparams: info.tparams.clone(), clauses, ret: info.base_types[0].1, declared: crate::tir::NO_TYPE });
        // A parameter the first list leaves out takes the receiver's field, the default getter's
        // value, typed as an argument is.
        let mut lists = lists;
        if let Some(list) = lists.first_mut().filter(|l| !l.using) {
            let given = self.filled_params(&first, list);
            for (i, p) in first.params.iter().enumerate() {
                if given[i] {
                    continue;
                }
                let kept = self.types.subst(p.ty, &recv_subst);
                let t = self.prog.add(TExpr::Local(tmp));
                let field = self.prog.add(TExpr::Field(t, p.sym));
                list.args.push(ArgSrc::Named(p.name, field, kept));
            }
        }
        let call = MethodCall { recv: None, sym: SymId(u32::MAX), owner_subst: Vec::new(), ext_recv: None, prefix: None, refined: None };
        // `copyMeths`' `creatorExpr` constructs inside the class: a class nested in a class is
        // made with the receiver's own enclosing instance, wherever the copy is written, the
        // construction under the eta-expansion of a copy applied in part included.
        let inner = self.outer_class(c).is_some();
        let outer_prefixed = self.new_prefixed;
        self.new_prefixed |= inner;
        self.copy_application = true;
        let applied = self.apply_method(call, Some((c, sig)), targs, lists, span, expected, false);
        self.copy_application = false;
        self.new_prefixed = outer_prefixed;
        let (te, ty) = applied.unwrap();
        if inner {
            let accessor = self.outer_accessor(c);
            let t = self.prog.add(TExpr::Local(tmp));
            let outer = self.prog.add(TExpr::CallMethod(t, accessor, ListRef::EMPTY));
            let mut at = te;
            while let TExpr::Lambda(_, body) = self.prog.expr(at) {
                at = body;
            }
            self.pass_outer(at, outer);
        }
        // The receiver first, then the temporaries of the arguments, around the construction.
        let mut stmts = vec![TStmt::Val(tmp, recv)];
        let value = match self.prog.expr(te) {
            TExpr::Block(inner, v) if matches!(self.prog.expr(v), TExpr::New(..)) => {
                stmts.extend_from_slice(self.prog.stmt_list(inner));
                v
            }
            _ => te,
        };
        let copied = matches!(self.prog.expr(value), TExpr::New(..));
        let stmts = self.prog.stmts.push_slice(&stmts);
        let te = self.prog.add(TExpr::Block(stmts, value));
        if self.capturing() && copied {
            self.capture_form(te, Form::CaseCopy);
        }
        (te, ty)
    }

    /// The class whose instance the creator application of the case class `c` selects its
    /// companion's `apply` on, where that is a proper subclass `R` of the class `K` declaring `c`
    /// and `K` has private aliases: the receiver of the application (`r.C(..)`), else the
    /// innermost enclosing class that has `c` as a member.
    fn companion_apply_receiver(&mut self, c: ClassId) -> Option<ClassId> {
        let info = self.syms.class(c);
        let Owner::Class(k) = info.owner else { return None };
        if info.mods & crate::ast::mods::CASE == 0 || self.syms.class(k).kind == ClassKind::Object {
            return None;
        }
        let aliases: Vec<AliasId> = self.syms.class(k).type_aliases.values().copied().collect();
        if !aliases.iter().any(|&a| self.private_alias(a)) {
            return None;
        }
        let derives = |t: &Self, r: ClassId| t.syms.class(r).base_types.iter().any(|&(b, _)| b == k);
        let r = match self.creator_receiver.filter(|&(n, _)| n == c) {
            Some((_, r)) => r,
            None => self.env.frames.iter().rev().find_map(|f| match f {
                super::Frame::Class(f) if *f == k || derives(self, *f) => Some(*f),
                _ => None,
            })?,
        };
        (r != k && derives(self, r)).then_some(r)
    }

    /// The constructor clauses of the program's case class `c` as declared, for the products: a
    /// private alias of the class `k` it is nested in kept as `k`'s member (`k.this.Event`), as
    /// scalac's pickle has the written type, so that a downstream sees the class's and its
    /// companion's `apply` from its own receiver (`companion_apply_sig`); `None` where there is
    /// none to keep.
    pub(crate) fn declared_ctor(&mut self, c: ClassId) -> Option<Vec<ClauseSig>> {
        let info = self.syms.class(c);
        let Owner::Class(k) = info.owner else { return None };
        if info.def.is_none() || info.mods & crate::ast::mods::CASE == 0 || self.syms.class(k).kind == ClassKind::Object {
            return None;
        }
        let aliases: Vec<AliasId> = self.syms.class(k).type_aliases.values().copied().collect();
        if !aliases.iter().any(|&a| self.private_alias(a)) {
            return None;
        }
        let ctor = self.syms.class(c).ctor.clone();
        let sig = Arc::new(MethodSig { tparams: self.syms.class(c).tparams.clone(), clauses: ctor, ret: ANY, declared: crate::tir::NO_TYPE });
        let declared = self.companion_apply_sig(c, k, sig.clone());
        (!Arc::ptr_eq(&declared, &sig)).then(|| declared.clauses.clone())
    }

    /// The constructor's signature as the companion's synthetic `apply` of the case class `c`
    /// states it, selected on an instance of `r`: dotty's `Desugar.classDef` gives that `apply`
    /// the class's whole declared parameter-list structure, its types as written, which seen
    /// from the receiver (`TypeOps.asSeenFrom`) name a private alias of the base class `k` as a
    /// member of the receiver it is not (`MissingType`, "a private member in a base class"). Every
    /// clause's types are read again with `k`'s private aliases seen from `r` (resolved anew from
    /// the source, or mapped anew from a library's pickle); a parameter whose type names none of
    /// them keeps its own.
    fn companion_apply_sig(&mut self, c: ClassId, r: ClassId, sig: Arc<MethodSig>) -> Arc<MethodSig> {
        let info = self.syms.class(c);
        let (file, def, owner) = (info.file, info.def, info.owner);
        let Owner::Class(k) = owner else { return sig };
        let prefix = self.this_prefix(r);
        let seen: Vec<Vec<TypeId>> = match def {
            Some(d) => {
                let ast = self.ast(file);
                let crate::ast::DefKind::Class(cls) = &ast.def(d).kind else { return sig };
                let written: Vec<Vec<TyExprId>> = cls.clauses.iter().map(|cl| cl.params.iter().map(|p| p.ty).collect()).collect();
                let tparams: Vec<(Name, TParamId)> = cls.tparams.iter().map(|tp| tp.name).zip(sig.tparams.iter().copied()).collect();
                let env = self.env_at(file, Owner::Class(k), ast.def(d).span.start);
                let reported = self.diags.items.len();
                let seen = self.with_env(env, |t| {
                    t.env.frames.push(super::Frame::Locals { names: Vec::new(), tparams, givens: Vec::new(), classes: Vec::new(), aliases: Vec::new(), owner: t.sites.owners.len() as u32 });
                    let outer = std::mem::replace(&mut t.private_alias_prefix, Some((k, prefix)));
                    let seen = written
                        .iter()
                        .map(|clause| {
                            clause
                                .iter()
                                .map(|&ty| {
                                    let ty = match t.cur_ast().ty(ty) {
                                        TyExpr::Repeated(inner) | TyExpr::ByName(inner) => inner,
                                        _ => ty,
                                    };
                                    t.resolve_type(ty)
                                })
                                .collect()
                        })
                        .collect();
                    t.private_alias_prefix = outer;
                    t.env.frames.pop();
                    seen
                });
                self.drop_reported_since(reported);
                seen
            }
            None => match self.loaded_ctor_types_seen_from(c, k, prefix) {
                Some(seen) => seen,
                None => return sig,
            },
        };
        if seen.len() != sig.clauses.len() || seen.iter().zip(&sig.clauses).any(|(s, cl)| s.len() != cl.params.len()) {
            return sig;
        }
        let mut out = (*sig).clone();
        for (clause, types) in out.clauses.iter_mut().zip(seen) {
            for (param, ty) in clause.params.iter_mut().zip(types) {
                if ty != param.ty && !self.types.contains_error(ty) && self.types.mentions_member_of(ty, prefix) {
                    param.ty = ty;
                }
            }
        }
        Arc::new(out)
    }

    /// Whether the class `c` has the member `name` that the compiler gives classes (`copy` a case
    /// class, `ordinal` an enum's, `productPrefix` a product, `toString` and the rest every class).
    fn has_synthesized_member(&self, c: ClassId, name: Name) -> bool {
        let info = self.syms.class(c);
        let case = info.mods & crate::ast::mods::CASE != 0;
        match name {
            names::COPY => case && info.mods & crate::ast::mods::ABSTRACT == 0,
            names::ORDINAL => matches!(info.kind, ClassKind::Enum | ClassKind::EnumCase),
            names::PRODUCT_PREFIX => case || info.kind == ClassKind::EnumCase,
            names::APPLY => false,
            _ => true,
        }
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

    /// The getter `name`, `m$default$n`, selected on a receiver of type `recv_ty` in a library
    /// body or a product's: whose default it is, the parameter's place among the callee's, its
    /// type, and whether the receiver is the callee's. A constructor's (`$lessinit$greater$default$n`
    /// on the companion); a member's, an extension the receiver's class defines among them;
    /// and one of a method the std has as an extension (`indexWhere` on a `String`), whose
    /// clauses after its receiver's are scala-library's member's, the receiver its argument.
    fn member_default_getter(&mut self, recv_ty: TypeId, name: Name) -> Option<(DefaultOf, usize, TypeId, bool)> {
        let text = self.name_str(name);
        let (member, n) = text.rsplit_once("$default$")?;
        let n: usize = n.parse().ok()?;
        let i = n.checked_sub(1)?;
        // A constructor's, on the companion of its class or trait.
        if member == "<init>" || member == "$lessinit$greater" {
            let o = self.class_of(recv_ty)?;
            let c = match self.companion_class(o) {
                Some(c) => c,
                // A trait's, and an enum's class case's (`enter_enum_case_companion`).
                None => self.syms.class(o).companion.filter(|&k| matches!(self.syms.class(k).kind, ClassKind::Trait | ClassKind::EnumCase)).unwrap_or(o),
            };
            return Some((DefaultOf::Ctor(c), i, self.ctor_param_default(c, n)?, false));
        }
        let member = self.interner.intern(member);
        let found = match self.find_member(recv_ty, member) {
            Some(f) => Some(f),
            None => self.class_extension(recv_ty, member),
        };
        let Some((sym, owner_ty)) = found else {
            for sym in self.lexical_extensions(member) {
                let ext_clauses = self.syms.sym(sym).ext_clauses as usize;
                let sig = self.sig_of(sym);
                let skipped: usize = sig.clauses.iter().take(ext_clauses).map(|c| c.params.len()).sum();
                let p = sig.clauses.iter().skip(ext_clauses).flat_map(|c| c.params.iter()).nth(i);
                if let Some(p) = p.filter(|p| p.has_default) {
                    return Some((DefaultOf::Method(sym), skipped + i, p.ty, false));
                }
            }
            return None;
        };
        let alts: Vec<SymId> = match self.syms.alternatives(sym) {
            Some(set) => set.to_vec(),
            None => vec![sym],
        };
        let subst = self.owner_subst(owner_ty);
        for alt in alts {
            let j = self.signature_param_index(alt, i);
            if self.default_declarer(alt, j).is_none() {
                continue;
            }
            let sig = self.sig_of(alt);
            if let Some(ty) = sig.clauses.iter().flat_map(|c| c.params.iter()).nth(j).map(|p| p.ty) {
                return Some((DefaultOf::Method(alt), j, self.types.subst(ty, &subst), true));
            }
        }
        None
    }

    /// The extension `name` the class of `recv_ty`, or a base class, defines, seen from it.
    fn class_extension(&mut self, recv_ty: TypeId, name: Name) -> Option<(SymId, TypeId)> {
        let c = self.class_of(recv_ty)?;
        self.complete_class(c);
        let bases = self.met_base_types(c);
        bases.into_iter().find_map(|(b, bt)| self.syms.class(b).extensions.iter().copied().find(|&x| self.syms.sym(x).name == name).map(|x| (x, bt)))
    }

    /// The place, among the parameters of `m` as its signature has them, of the `i`th of its
    /// declaration's: a right-associative extension's first clause is declared before its
    /// receiver's (`Desugar.extMethod`), which the signature keeps first.
    fn signature_param_index(&mut self, m: SymId, i: usize) -> usize {
        match self.right_assoc_declared_order(m) {
            Some((order, sizes)) => {
                let mut k = i;
                for c in order {
                    if k < sizes[c] {
                        return sizes[..c].iter().sum::<usize>() + k;
                    }
                    k -= sizes[c];
                }
                i
            }
            None => i,
        }
    }

    /// The clauses of a right-associative extension `m` in its declaration's order, with each
    /// clause's size.
    fn right_assoc_declared_order(&mut self, m: SymId) -> Option<(Vec<usize>, Vec<usize>)> {
        if !self.is_right_assoc_extension(m) {
            return None;
        }
        let ext_clauses = self.syms.sym(m).ext_clauses as usize;
        let sig = self.sig_of(m);
        let order = sig.right_assoc_order(ext_clauses)?;
        Some((order, sig.clauses.iter().map(|c| c.params.len()).collect()))
    }

    /// `recv.m$default$n[T..](args)..` of a pickled body: the getter's application to the
    /// receiver, an extension's receiver argument first.
    fn default_getter_selection(&mut self, recv: TExprId, recv_ty: TypeId, name: Name, targs: Option<ListRef>, lists: &[ArgList], span: Span) -> Option<(TExprId, TypeId)> {
        let (of, index, ty, on_receiver) = self.member_default_getter(recv_ty, name)?;
        let (on, leading) = match of {
            DefaultOf::Method(_) if on_receiver => (Some(recv), Vec::new()),
            DefaultOf::Method(_) => (None, vec![recv]),
            DefaultOf::Ctor(_) => (None, Vec::new()),
        };
        Some(self.getter_application(of, index, on, &leading, targs, ty, lists.to_vec(), span))
    }

    /// A getter's application a pickled body spells (`x.m$default$n[T..](args)`): the plan's
    /// node, the receiver and `leading` (an extension's receiver argument) before the arguments
    /// the getter is applied to, typed against the callee's parameters before its own; the type
    /// arguments the callee's type parameters, or its class's for a constructor's, which the
    /// getter repeats.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::typer) fn getter_application(&mut self, of: DefaultOf, index: usize, recv: Option<TExprId>, leading: &[TExprId], targs: Option<ListRef>, ty: TypeId, lists: Vec<ArgList>, span: Span) -> (TExprId, TypeId) {
        let (params, tparams): (Vec<ParamSig>, Vec<TParamId>) = match of {
            DefaultOf::Method(m) => {
                let sig = self.sig_of(m);
                (sig.clauses.iter().flat_map(|c| c.params.iter().cloned()).collect(), sig.tparams.clone())
            }
            DefaultOf::Ctor(c) => (
                self.syms.class(c).ctor.iter().flat_map(|cl| cl.params.iter().cloned()).collect(),
                self.syms.class(c).tparams.to_vec(),
            ),
        };
        let targs: Vec<TypeId> = match targs {
            Some(l) => self.cur_ast().ty_list(l).to_vec().into_iter().map(|t| self.resolve_type(t)).collect(),
            None => Vec::new(),
        };
        let subst: crate::types::Subst = match targs.len() == tparams.len() {
            true => tparams.iter().copied().zip(targs.iter().copied()).collect(),
            false => Vec::new(),
        };
        let ty = self.types.subst(ty, &subst);
        let params: Vec<ParamSig> = params.into_iter().map(|p| ParamSig { ty: self.types.subst(p.ty, &subst), ..p }).collect();
        // A right-associative extension's getter takes the clauses in its declaration's order.
        let lists = match of {
            DefaultOf::Method(m) => match self.right_assoc_declared_order(m) {
                Some((order, _)) => {
                    let mut placed: Vec<(usize, ArgList)> = lists.into_iter().enumerate().map(|(j, l)| (order.get(j).copied().unwrap_or(j), l)).collect();
                    placed.sort_by_key(|&(c, _)| c);
                    placed.into_iter().map(|(_, l)| l).collect()
                }
                None => lists,
            },
            DefaultOf::Ctor(_) => lists,
        };
        let mut items: Vec<TExprId> = Vec::new();
        let mut at = index as u32;
        if let Some(r) = recv {
            items.push(r);
            at |= DEFAULT_RECEIVER;
        }
        items.extend_from_slice(leading);
        let mut k = leading.len();
        for list in lists {
            for arg in list.args {
                let p = params.get(k).cloned();
                let te = match p {
                    // A repeated parameter's sequence, passed as `xs*`.
                    Some(p) if p.repeated => {
                        let seq = match self.seq_class() {
                            Some(seq) => self.types.class(seq, &[p.ty]),
                            None => ANY,
                        };
                        let ast = self.cur_ast();
                        let arg = match arg {
                            ArgSrc::Ast(e) => match ast.expr(e) {
                                Expr::Typed(inner, t) if matches!(ast.ty(t), TyExpr::Repeated(_)) => ArgSrc::Ast(inner),
                                _ => arg,
                            },
                            _ => arg,
                        };
                        self.type_arg(arg, seq, list.span)
                    }
                    Some(p) => {
                        let te = self.type_arg(arg, p.ty, list.span);
                        if p.by_name { self.by_name_thunk(te) } else { te }
                    }
                    None => self.type_arg(arg, ANY, list.span),
                };
                items.push(te);
                k += 1;
            }
        }
        self.note_super_getter(of, recv, index);
        let l = self.prog.list(&items);
        let d = self.prog.add(TExpr::Default(of, at, l));
        self.prog.set_type(d, ty);
        if self.capturing() && !subst.is_empty() {
            self.capture_targs(d, &targs);
        }
        let _ = span;
        (d, ty)
    }

    pub(in crate::typer) fn ctor_param_default(&mut self, c: ClassId, n: usize) -> Option<TypeId> {
        self.complete_class(c);
        let primary = self.syms.class(c).info.ctor.iter().flat_map(|cl| cl.params.iter()).nth(n.checked_sub(1)?).filter(|p| p.has_default).map(|p| p.ty);
        if primary.is_some() || self.syms.class(c).js != JsKind::Native {
            return primary;
        }
        // A native class's secondary constructor that declares defaults (Scala.js's facades, a
        // typed array's `byteOffset` and `length`): its getters are `<init>$default$n` too, the
        // one alternative of the constructors that may have them.
        for k in self.syms.class(c).ctors.clone() {
            let sig = self.sig_arc(k);
            if let Some(p) = sig.clauses.iter().flat_map(|cl| cl.params.iter()).nth(n - 1).filter(|p| p.has_default) {
                return Some(p.ty);
            }
        }
        None
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

    /// Whether the builtin type `t` has the member `name` as dotty's `member` finds it on the class
    /// the builtin stands for: of a `String`, a method the JDK's `java.lang.String` declares
    /// (`jdk_string_alternatives`); else a std extension that is the JDK's (`@javaDefined`) or one
    /// of std/core.scala's (the builtin classes' own members no TASTy file defines), of an `Array`
    /// those Array.scala declares
    /// (`names::ARRAY_MEMBERS`, not the JavaScript array's beside them). The std's other
    /// extensions of a builtin stand for Predef's views (`augmentString`, `intWrapper`), which a
    /// selection meets through a view.
    pub(super) fn builtin_member_like(&mut self, t: TypeId, name: Name) -> bool {
        if self.stands_for_string(t) {
            return self.jdk_string_alternatives(name).is_some();
        }
        let candidates = self.builtin_member_candidates(t, name);
        candidates.into_iter().any(|sym| self.is_builtin_member(sym, t))
    }

    /// The std's extensions named `name` that may be the builtin `t`'s own member
    /// (`builtin_member_like`), before their receivers are tested.
    fn builtin_member_candidates(&mut self, t: TypeId, name: Name) -> Vec<SymId> {
        // A function type is a builtin too: its `andThen` and `compose` are std/core.scala's.
        if !self.is_builtin_type(t) && self.as_function(t).is_none() {
            return Vec::new();
        }
        if self.class_of(t) == Some(self.b.array) && !crate::names::ARRAY_MEMBERS.contains(&self.interner.get(name)) {
            return Vec::new();
        }
        let scala_pkg = self.b.scala_pkg;
        self.demand_std(scala_pkg, name, crate::stdindex::TERM);
        self.syms.pkg(scala_pkg).entries.get(&name).map(|e| e.extensions.clone()).unwrap_or_default()
    }

    /// Whether the std extension `sym` is the builtin `t`'s own member: the JDK's or one of
    /// std/core.scala's, taking `t` as its receiver.
    fn is_builtin_member(&mut self, sym: SymId, t: TypeId) -> bool {
        let info = self.syms.sym(sym);
        let own = info.java_defined || self.source(info.file).path.ends_with("core.scala");
        if !own {
            return false;
        }
        let mark = self.snapshot();
        let call = MethodCall { recv: None, sym, owner_subst: Vec::new(), ext_recv: None, prefix: None, refined: None };
        let ok = self.extension_applicable(&call, t);
        self.rollback(mark);
        ok
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
            let call = MethodCall { recv: None, sym, owner_subst: Vec::new(), ext_recv: None, prefix: None, refined: None };
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
        // dotty's `tryExtension` tries every import of the scope that brings the name
        // (`altImports`), the nearest's failure included: one object's inherited extension is
        // another's seen from its own object (`TotalWrapper`'s `value` through `A.*` and `B.*`).
        let alternatives = if several { self.import_alternatives(name) } else { Vec::new() };
        let lexical = match lexical_pick {
            Some(call) if alternatives.len() > 1 && alternatives.iter().any(|g| g.syms.contains(&call.sym)) => {
                self.lexical_among_imports(alternatives, recv, recv_ty, targs, lists, span, expected)
            }
            None if alternatives.len() > 1 => self.lexical_among_imports(alternatives, recv, recv_ty, targs, lists, span, expected),
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
    /// `retry`: the member's retry on its qualifier (`retry_on_qualifier`), where the qualifier's
    /// member of the name failed, the infix `a op q`'s operand `a` its argument (`Hoisted`).
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
        retry: bool,
    ) -> Option<Option<(TExprId, TypeId)>> {
        let written = lists.as_ref()?;
        let first = written.first().filter(|l| !l.using && l.args.len() == 1)?;
        let arg = first.args[0];
        if !matches!(arg, ArgSrc::Ast(_) | ArgSrc::Typed(..) | ArgSrc::Hoisted(_)) || !retry && (matches!(arg, ArgSrc::Hoisted(_)) || self.has_member_like(recv_ty, name)) {
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
            ArgSrc::Hoisted(e) if matches!(self.cur_ast().expr(e), Expr::NamedArg(..)) => return None,
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
                // it by value: bound ahead, the application made again over the binding. The
                // infix `a op q`'s operand `a` is evaluated before the qualifier where the
                // selected overload's receiver takes it by value, as `typedInfixOp` lifts an
                // impure argument of a parameter that is no by-name one (`LiftImpure.liftArgs`).
                let ext_clauses = self.syms.sym(sym).ext_clauses as usize;
                let sig = self.sig_arc(sym);
                let hoisted_arg = matches!(arg, ArgSrc::Hoisted(_));
                let strict = if hoisted_arg {
                    let at = (0..ext_clauses).find(|&k| !sig.clauses[k].is_using);
                    !at.is_some_and(|k| sig.clauses[k].params[0].by_name)
                } else {
                    !sig.clauses[ext_clauses].params[0].by_name
                };
                // The operand's binding is made over again (`hoist` keeps a stable one as it is).
                let stable = !hoisted_arg && self.is_stable(recv);
                if !strict || stable {
                    self.close(attempt);
                    return Some(Some((self.wrap_hoisted(mark, te), ty)));
                }
                self.retract(attempt);
                self.hoisted.truncate(mark);
                let attempt = self.attempt();
                let (q, arg) = match arg {
                    ArgSrc::Hoisted(e) => {
                        let (ta, aty) = self.type_expr(e, expected_arg);
                        let a = self.hoist(ta, aty, self.cur_ast().expr_span(e));
                        (recv, ArgSrc::Typed(a, aty))
                    }
                    _ => (self.hoist(recv, recv_ty, span), arg),
                };
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
            ArgSrc::Ast(e) | ArgSrc::Hoisted(e) => self.type_expr(e, expected_arg),
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
            if let Some(r) = self.right_assoc_selection(recv, recv_ty, name, targs, lists, span, expected, false) {
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
    /// argument list, an inline expansion, an error inside an argument, is the member's
    /// (`boundary_attempt`); where neither takes over, the member's application stands
    /// with its errors. The member's application is an attempt (`tryEither`, Typer.scala 134)
    /// over its first plain list's arguments cached as dotty's `FunProto` caches them
    /// (`state::ArgCache`): the retry's tries adapt what it cached and type the rest.
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
        // The lists as written, for the retry, into a spare buffer's, which a call that types
        // gives back.
        let mut call = self.member_calls.pop().unwrap_or_default();
        call.keep(&lists);
        self.arg_cache_open_member(lists.iter().find(|l| !l.using));
        let mark = self.attempt();
        self.arg_cache_attempt(mark);
        let hoisted = self.hoisted.len();
        let outer = self.member_call.replace(call);
        // The type mismatches reported are logged, by the type each required, for the test of
        // an error argument (`Worker::mismatches`).
        self.logging += 1;
        let r = self.apply_callee(callee, targs, lists, span, expected);
        self.logging -= 1;
        if self.logging == 0 {
            self.mismatches.clear();
        }
        let call = std::mem::replace(&mut self.member_call, outer).unwrap_or_default();
        if !self.attempt_failed(&mark) {
            self.close(mark);
            self.arg_cache_close();
            self.member_calls.push(call);
            return r;
        }
        self.retry_on_qualifier(r, mark, hoisted, (sym, owner_ty), recv, recv_ty, name, targs, call, span, expected, converted)
    }

    /// `apply_member_or_extension` past a member's application, the attempt `mark`, that
    /// reported an error: the retry, or the member's application back. The application is set
    /// aside whole, as dotty keeps its failed state for its error (Applications.scala 1477): what
    /// work done on demand for an older definition wrote stays (its completion's diagnostics,
    /// `Namer.Completer.complete`, Namer.scala 862), and so do what its cached typings wrote and
    /// the instances of its own variables (the owner rule, `TypeVar.instantiateWith`). dotty's
    /// tries in its order (`retry_tries`), each ending where the member's errors stand for it
    /// (`Boundary::Stands`), each an attempt over the cached arguments.
    #[cold]
    #[inline(never)]
    #[allow(clippy::too_many_arguments)]
    fn retry_on_qualifier(
        &mut self,
        r: (TExprId, TypeId),
        mark: super::state::Mark,
        hoisted: usize,
        (sym, owner_ty): (SymId, TypeId),
        recv: TExprId,
        recv_ty: TypeId,
        name: Name,
        targs: Option<ListRef>,
        call: Box<MemberCall>,
        span: Span,
        expected: Option<TypeId>,
        converted: bool,
    ) -> (TExprId, TypeId) {
        let member_hoisted = self.hoisted.split_off(hoisted);
        let member = self.set_aside(mark);
        // The retry is an attempt of its own, closed as it ends: what its test typed alone and the
        // calls left pending in its typings wait for the tree it gives.
        let retry = self.attempt();
        let written = call.written();
        let outer_depth = (self.retry_depth.replace(self.app_depth), self.retry_applied.take());
        for t in self.retry_tries(sym, &written).into_iter().flatten() {
            let Boundary::Retry(lists) = self.boundary_attempt(sym, owner_ty, &written, t) else { continue };
            // What the test typed alone stays for the tries (`typedArgs`' typings, cached).
            let kept_hoisted = self.hoisted.len();
            // Either takes over where it applies the list: the expected type is no prototype of
            // the application (`IgnoredProto`, Applications.scala 1309), whose result a misfit of
            // it is reported against once applied (`init` defers it, 693). Each is an attempt.
            let tried = self.attempt();
            self.arg_cache_attempt(tried);
            let outer = self.lexical_selected.replace(false);
            let mut opt = Some(lists.clone());
            // A right-associative extension whose method's clause comes before the receiver's
            // takes the qualifier there (`op(q)(a)`, `Desugar.rightAssocParams`), as the infix
            // `a op q` whose member failed is retried (`Typer.typedInfixOp`).
            let ext = match self.interner.get(name).ends_with(':') {
                true => match self.right_assoc_selection(recv, recv_ty, name, targs, &mut opt, span, expected, true) {
                    Some(r) => r,
                    None => self.try_extensions(recv, recv_ty, recv_ty, name, targs, &mut opt, span, expected),
                },
                false => self.try_extensions(recv, recv_ty, recv_ty, name, targs, &mut opt, span, expected),
            };
            let selected = std::mem::replace(&mut self.lexical_selected, outer) == Some(true);
            // The extension applied to the list, whatever its using clauses and later lists give
            // after it (`retry_applied`): dotty's try is the list's application (`simpleApply` in
            // `tryWithImplicitOnQualifier`), and the error is the extension's.
            if let Some(ext) = ext.filter(|_| self.try_applied(&tried)) {
                self.close(tried);
                return self.retry_taken(ext, retry, outer_depth, call);
            }
            self.retract(tried);
            self.hoisted.truncate(kept_hoisted);
            // A lexical extension selected on the qualifier is the try's whatever its
            // application gives: its failure to apply opens no search for a conversion (dotty's
            // `tryExtensionOrConversion` returns it, Typer.scala 4320).
            if !converted && !selected {
                let tried = self.attempt();
                self.arg_cache_attempt(tried);
                if let Some(c) = self.apply_through_conversion(recv, recv_ty, name, targs, &lists, span, expected) {
                    if self.try_applied(&tried) {
                        self.close(tried);
                        return self.retry_taken(c, retry, outer_depth, call);
                    }
                }
                self.retract(tried);
                self.hoisted.truncate(kept_hoisted);
            }
        }
        (self.retry_depth, self.retry_applied) = outer_depth;
        self.restore(member);
        self.hoisted.extend(member_hoisted);
        self.drop_unheld_typings(r.0);
        self.arg_cache_close();
        self.close(retry);
        // The member's errors stand. Where its attempt stopped at its first plain clause, the
        // lists after it are typed alone now, once (`typedArgs` under an erroneous function
        // part, Applications.scala 1438).
        if call.stopped {
            let first = written.iter().position(|l| !l.using).unwrap_or(0);
            self.type_args_for_errors(&written[first + 1..]);
        }
        self.member_calls.push(call);
        r
    }

    /// A try of the member's retry, inside the retry's attempt `retry`, took the application
    /// over, giving `r`: the member's application, set aside, goes.
    fn retry_taken(&mut self, r: (TExprId, TypeId), retry: super::state::Mark, outer_depth: (Option<u32>, Option<(u32, u32)>), call: Box<MemberCall>) -> (TExprId, TypeId) {
        (self.retry_depth, self.retry_applied) = outer_depth;
        self.drop_unheld_typings(r.0);
        self.arg_cache_close();
        self.close(retry);
        self.member_calls.push(call);
        r
    }

    /// The calls left pending for the later expansion phase in the member's cached typings that
    /// the application `app` the retry gives does not hold go: dotty's `Inlining` phase expands
    /// the calls of the tree typing kept (Inlining.scala 135), and a typing the retry gave up is
    /// none of it (the member's tupled dual where a try took the arguments as written, what the
    /// test typed alone where the member's errors stand).
    #[cold]
    #[inline(never)]
    fn drop_unheld_typings(&mut self, app: TExprId) {
        let mut roots: Vec<TExprId> = self.member_args().iter().filter_map(|a| a.typed.map(|t| t.0)).collect();
        if let Dual::Typed(te, _) = self.member_dual() {
            roots.push(te);
        }
        roots.retain(|&te| self.holds_pending_call(te));
        if roots.is_empty() {
            return;
        }
        let held: crate::intern::FxMap<TExprId, ()> = self.prog.descendants(app).map(|e| (e, ())).collect();
        roots.retain(|te| !held.contains_key(te));
        if !roots.is_empty() {
            self.drop_pending_in(&roots);
        }
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
        let depth = self.app_depth;
        if let Some(c) = self.member_call.as_mut().filter(|c| c.depth == depth) {
            c.stopped = true;
        }
        true
    }

    /// Whether the try `tried` of a member's retry applied the member's list: it reported no error,
    /// or none before its application applied its first plain list (`retry_applied`), what follows
    /// being the outer application's.
    fn try_applied(&self, tried: &super::state::Mark) -> bool {
        if !self.attempt_failed(tried) {
            return true;
        }
        let Some((diags, error_nodes)) = self.retry_applied else { return false };
        !self.attempt_failed_before(tried, diags as usize, error_nodes)
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
        // A value, or a def without a parameter list, whose value's application failed: the
        // lists as written are the try's (dotty retries any selection's application,
        // `tryWithImplicitOnQualifier`: `e.width(2)` past a `width: Double` is an extension's).
        if self.is_value_member(sym) {
            return if lists.first().is_some_and(|l| !l.using) { [Some(RetryTry::Separate), None] } else { [None, None] };
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

    /// A member that is a value when selected: a val, a var, an object, a def without a parameter
    /// list.
    fn is_value_member(&mut self, sym: SymId) -> bool {
        if self.syms.sym(sym).kind != SymKind::Def {
            return true;
        }
        self.sig_arc(sym).clauses.iter().all(|c| c.is_using)
    }

    /// One of dotty's `tryWithImplicitOnQualifier` tries (Applications.scala 1373, scalac 3.8.4's
    /// lines) past a member's application that failed. The member's errors stand for the try
    /// where an argument of the first list is an error argument (`FunProto.hasErrorArg`) or the
    /// member matches the list (`SelectionProto.isMatchedBy`, Typer.scala 4253), which tests it
    /// on the arguments as the cache has them, a function literal of a parameter without a type
    /// as its shape and the others typed alone into the cache (`FunProto.typedArgs`,
    /// ProtoTypes.scala 508); otherwise the lists the retry applies an extension or a conversion
    /// to, as written: their applications adapt what the cache holds and type the rest.
    fn boundary_attempt(&mut self, sym: SymId, owner_ty: TypeId, lists: &[ArgList], t: RetryTry) -> Boundary {
        if self.is_value_member(sym) {
            return if self.member_args().iter().any(|a| a.error) { Boundary::Stands } else { Boundary::Retry(lists.to_vec()) };
        }
        let sig = self.sig_arc(sym);
        let (Some(clause), Some(at)) = (sig.clauses.iter().find(|c| !c.is_using), lists.iter().position(|l| !l.using)) else { return Boundary::Stands };
        if t == RetryTry::Tupled {
            return self.tupled_boundary(&sig, clause, owner_ty, lists, at);
        }
        let Some((args, matched, reordered)) = self.boundary_args(clause, &lists[at]) else { return Boundary::Stands };
        // Where `reorder` failed, dotty's application typed none of the arguments (`init`,
        // Applications.scala 661): none is an error argument, and the test types them alone.
        if !reordered {
            self.member_args_forget();
        }
        if self.member_args().iter().any(|a| a.error) {
            return Boundary::Stands;
        }
        let mut types: Vec<Option<TypeId>> = Vec::with_capacity(args.len());
        for a in &args {
            if let Some(p) = a.pretyped {
                self.retry_typed.insert(p.0, ());
                types.push(Some(p.1));
                continue;
            }
            let Some(value) = a.value else {
                types.push(None);
                continue;
            };
            // A splice's typing is its sequence's.
            let key = match self.cur_ast().expr(value) {
                crate::ast::Expr::Typed(inner, _) if a.splice => inner,
                _ => value,
            };
            let cached = self.member_args().iter().find(|c| c.key == key).and_then(|c| c.typed);
            let ty = match cached {
                Some((_, ty)) => ty,
                // `FunctionN(declared or ?, ?)`, what `cacheTypedArg` unforced makes of a
                // function literal with a parameter of no written type (ProtoTypes.scala 475).
                None => match self.function_shape(key) {
                    Some(shape) => shape,
                    None => match self.typed_alone_cached(key) {
                        Some(ty) => ty,
                        None => return Boundary::Stands,
                    },
                },
            };
            types.push(Some(ty));
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
        Boundary::Retry(lists.to_vec())
    }

    /// `boundary_attempt` of the try with the arguments tupled into one (`untpd.Tuple(args)`), a
    /// prototype of its own (`FunProto.tupledDual`): the tuple the member's application typed
    /// where it used this dual (`typedTuple`, Typer.scala 3729, its elements against its
    /// parameter's element types: an error in one is inside the tuple, an error argument), or
    /// the tuple typed alone (`typedArgs`), cached where clean.
    fn tupled_boundary(&mut self, sig: &MethodSig, clause: &ClauseSig, owner_ty: TypeId, lists: &[ArgList], at: usize) -> Boundary {
        let list = &lists[at];
        let Some(elems) = list.args.iter().map(|a| match *a {
            ArgSrc::Ast(e) => Some(e),
            _ => None,
        }).collect::<Option<Vec<ExprId>>>() else { return Boundary::Stands };
        let (te, ty) = match self.member_dual() {
            Dual::Error => return Boundary::Stands,
            Dual::Typed(te, ty) => (te, ty),
            Dual::None => {
                let hoisted = self.hoisted.len();
                let m = self.attempt();
                let (te, ty) = self.type_tuple(&elems, None);
                if self.attempt_failed(&m) {
                    self.retract(m);
                    self.hoisted.truncate(hoisted);
                    return Boundary::Stands;
                }
                self.close(m);
                let ty = self.zonk(ty);
                self.set_member_dual(Dual::Typed(te, ty));
                (te, ty)
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
        self.retry_typed.insert(te, ());
        let mut out = lists.to_vec();
        out[at] = ArgList { args: vec![ArgSrc::Typed(te, ty)], using: false, span: list.span };
        Boundary::Retry(out)
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
            let (e, pretyped) = match *arg {
                ArgSrc::Ast(e) | ArgSrc::Hoisted(e) => (Some(e), None),
                ArgSrc::Typed(te, ty) => (None, Some((te, ty))),
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
            out.push(BoundaryArg { value, pretyped, param: at, splice });
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
        // teq types a tree erroneous wherever an error is in it, an application whose argument
        // failed among them, where dotty gives that one its result type and lets an error type
        // stand for the tree only where it is the one inside (a selection on an erroneous
        // qualifier, `t.typeOpt != t1.typeOpt`): in an erroneous tree only a type mismatch counts,
        // which dotty's adaptation places on the argument it adapts, never on the tree around it.
        let erroneous = own == ERROR;
        let at = ast.expr_span(t);
        let formal = self.zonk(formal);
        let expected = expected.map(|x| self.zonk(x));
        let required: Vec<(usize, TypeId)> = logged.iter().map(|&(i, r)| (i, self.zonk(r))).collect();
        self.diags.items[diags..].iter().enumerate().any(|(k, d)| {
            let mismatch = required.iter().find(|&&(i, _)| i == diags + k).map(|&(_, r)| r);
            let inside = d.span != at && d.span.start >= at.start && d.span.end <= at.end;
            let placed_at_tree = d.span == at && mismatch.is_some_and(|r| expected.is_some_and(|x| r != x));
            !d.is_warning && mismatch != Some(formal) && (inside || placed_at_tree) && (!erroneous || mismatch.is_some())
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

    /// `e` typed alone, as `FunProto.typedArgs` types an argument its cache lacks
    /// (ProtoTypes.scala 508), into the member's cache where clean: its type, `None` where it is
    /// erroneous.
    fn typed_alone_cached(&mut self, e: ExprId) -> Option<TypeId> {
        let hoisted = self.hoisted.len();
        let m = self.attempt();
        let (te, ty) = self.type_expr(e, None);
        if self.attempt_failed(&m) || self.types.contains_error(ty) {
            self.retract(m);
            self.hoisted.truncate(hoisted);
            return None;
        }
        self.close(m);
        self.member_arg_put(e, (te, ty));
        Some(ty)
    }

    /// `SelectionProto.isMatchedBy` of the member `name` under an ignored member prototype
    /// (ProtoTypes.scala 257), for an overloaded qualifier's normalized alternative of type `t`
    /// (`resolve_selected_in`): `t` has such a member the selection can access, a builtin's operator
    /// counting as one, or members unknown (`hasUnknownMembers`: a variable, `Nothing`); with `views`, a view gives a
    /// type that has it (`viewExists`), the std's extensions of a builtin being Predef's views. An
    /// extension method counts in neither.
    pub(super) fn selection_matches(&mut self, t: TypeId, name: Name, views: bool) -> bool {
        let t = self.deref(t);
        if matches!(self.types.get(t), Type::Var(_)) || t == NOTHING {
            return true;
        }
        // A member the selection could not access is none (`isAccessibleFrom`).
        let member = match self.find_member(t, name) {
            Some((sym, _)) => match self.syms.alternatives(sym) {
                Some(alts) => alts.to_vec().into_iter().any(|a| self.is_accessible(a)),
                None => self.is_accessible(sym),
            },
            None => false,
        };
        if member
            || self.universal_selection(t, name)
            || self.prim_op_applies(t, name)
            || self.function_class_member(t, name)
            || self.builtin_member_like(t, name)
        {
            return true;
        }
        if !views {
            return false;
        }
        if self.has_member_like(t, name) {
            return true;
        }
        // A view to a type with the member, unambiguous (`viewExists`): the deeper round's probe
        // (`view_selects`), which leaves the selection's application of a conversion its one
        // caller.
        let span = self.selections.len().checked_sub(1).and_then(|i| self.selection_at(i)).map_or(Span::new(0, 0), |s| s.span);
        let mark = self.attempt();
        let found = self.view_selects(t, name, span, &mut |_, _| true);
        self.retract(mark);
        found
    }

    /// The arguments of the member the selection at `i` (`Worker::selections`) applies, as written.
    fn selection_list(&mut self, i: usize) -> Option<ArgList> {
        let sel = self.sel(i);
        match sel.args {
            SelectionArgs::None => None,
            SelectionArgs::Operand(r) => Some(ArgList { args: self.infix_args(r), using: false, span: sel.span }),
            SelectionArgs::List(l) => {
                Some(ArgList { args: self.cur_ast().expr_list(l).iter().map(|&a| ArgSrc::Ast(a)).collect(), using: false, span: sel.span })
            }
        }
    }

    /// How many layers the selection at `i` holds past its member's name, which the resolution
    /// reveals one at a time (`deepenProto`): its arguments, then its expectation, the selection
    /// on its result with its own layers.
    pub(super) fn selection_levels(&self, i: usize) -> u8 {
        let sel = self.sel(i);
        let own = u8::from(!matches!(sel.args, SelectionArgs::None));
        own + match sel.result {
            SelectionResult::Ignored => 0,
            SelectionResult::Type(_) => 1,
            SelectionResult::Selected if i > 0 => 1 + self.selection_levels(i - 1).min(4),
            SelectionResult::Selected => 0,
        }
    }

    /// `isCompatible` of `t` with the selection at `i` revealed `depth` layers (`depth` at least
    /// 1; ProtoTypes.scala 44): the selection matches it (`selection_matched`), or, where `views`
    /// (the resolution's pass with implicits, Applications.scala 2475), a view converts it to a
    /// type the selection matches (`viewExists`, `selection_view`).
    pub(super) fn selection_deep(&mut self, t: TypeId, i: usize, depth: u8, views: bool) -> bool {
        self.selection_matched(t, i, depth, views) || (views && self.selection_view(t, i, depth))
    }

    /// `viewExists` of `t` to the selection at `i` revealed `depth` layers (Implicits.scala 885):
    /// a conversion whose result the selection matches. The search's attempts are taken back, so
    /// the operands are typed before it (`prime_selection_operands`) and it reads their typing.
    #[cold]
    fn selection_view(&mut self, t: TypeId, i: usize, depth: u8) -> bool {
        self.prime_selection_operands(i, depth);
        let (name, span) = (self.sel(i).name, self.sel(i).span);
        let (typed, hoisted) = (self.selection_typed.len(), self.hoisted.len());
        let mark = self.attempt();
        let found = self.view_selects(t, name, span, &mut |w, to| w.selection_matched(to, i, depth, true));
        self.retract(mark);
        self.selection_typed.truncate(typed);
        self.hoisted.truncate(hoisted);
        found
    }

    /// The operands of the selection at `i`, and of the selections its result reveals within
    /// `depth` layers, typed once as `selection_matched` types them (`selection_arg_type`), a
    /// named argument's value for it and a function literal without parameter types left.
    fn prime_selection_operands(&mut self, i: usize, depth: u8) {
        let owner = self.sel(i).span;
        let mut layers = depth;
        if let Some(list) = self.selection_list(i) {
            layers -= 1;
            let ast = self.cur_ast();
            let values: Vec<ExprId> = list
                .args
                .iter()
                .filter_map(|a| match *a {
                    ArgSrc::Ast(e) | ArgSrc::Hoisted(e) => Some(match ast.expr(e) {
                        crate::ast::Expr::NamedArg(_, v) => v,
                        _ => e,
                    }),
                    _ => None,
                })
                .collect();
            for v in values {
                if self.function_shape(v).is_none() {
                    self.selection_arg_type(v, owner);
                }
            }
        }
        if layers >= 2 && i > 0 && matches!(self.sel(i).result, SelectionResult::Selected) {
            self.prime_selection_operands(i - 1, layers - 1);
        }
    }

    /// `SelectionProto.isMatchedBy` of the selection at `i` revealed `depth` layers (`depth` at
    /// least 1; ProtoTypes.scala 257, its member prototype `FunProto(args, IgnoredProto(pt))`
    /// deepened): some accessible alternative of the member of `t`, compatible with them
    /// (`hasAltWith`, `normalizedCompatible`): taking the arguments, each typed alone once
    /// (`selection_arg_type`), its result, once revealed, fitting the member's expectation or
    /// matching the selection on it, through a view where `views`. A method is tested by its first
    /// explicit clause, a value of a function type by its `apply`, another value by its `apply`
    /// members, a builtin operator by the operand types its overloads take.
    fn selection_matched(&mut self, t: TypeId, i: usize, depth: u8, views: bool) -> bool {
        let t = self.deref(t);
        if matches!(self.types.get(t), Type::Var(_)) || t == NOTHING {
            return true;
        }
        let name = self.sel(i).name;
        let list = self.selection_list(i);
        let owner = self.sel(i).span;
        // The layers left for the result once the arguments took one.
        let after_args = if list.is_some() { depth - 1 } else { depth };
        // A member without a symbol of its own: a builtin operator's overloads, `Any`'s members, a
        // function's `apply`, `tupled` and `curried`, each tested as an ordinary member is.
        let alts = if self.prim_op_applies(t, name) {
            Some(self.builtin_operator_alternatives(t, name))
        } else if self.universal_selection(t, name) {
            Some(self.universal_alternatives(name))
        } else if self.function_class_member(t, name) {
            self.function_member_alternatives(t, name)
        } else {
            None
        };
        // A `String`'s own members are the JDK's (dotty selects on `java.lang.String`), whatever
        // signatures the std's implementations of them take.
        let alts = match alts {
            None if self.stands_for_string(t) => Some(self.jdk_string_alternatives(name).unwrap_or_default()),
            alts => alts,
        };
        if let Some(alts) = alts {
            return alts.iter().any(|a| self.sig_alt_matches(a, i, list.as_ref(), after_args, views));
        }
        let Some((sym, _)) = self.find_member(t, name) else { return self.builtin_extension_matched(t, i, list.as_ref(), after_args, views) };
        let members: Vec<SymId> = match self.syms.alternatives(sym) {
            Some(alts) => alts.to_vec(),
            None => vec![sym],
        };
        members.into_iter().any(|m| {
            if !self.is_accessible(m) {
                return false;
            }
            let result = match &list {
                Some(l) => {
                    if !self.member_applies(m, t, l, owner) {
                        return false;
                    }
                    if after_args == 0 {
                        return true;
                    }
                    self.member_result(m, t, true)
                }
                None => self.member_result(m, t, false),
            };
            match result {
                Some(r) => self.result_matches(r, i, after_args, views),
                None => true,
            }
        })
    }

    /// Whether the result `r` of the selection at `i`'s member fits what is expected of it, its
    /// expectation revealed `layers` deep (at least 1): an expected type it is compatible with, or
    /// the selection on it, matched at the depth left, through a view where `views`.
    fn result_matches(&mut self, r: TypeId, i: usize, layers: u8, views: bool) -> bool {
        match self.sel(i).result {
            SelectionResult::Ignored => true,
            SelectionResult::Type(expected) => {
                let mark = self.snapshot();
                let ok = self.is_compatible(r, expected);
                self.rollback(mark);
                ok
            }
            SelectionResult::Selected if i > 0 => match layers {
                1 => {
                    let name = self.sel(i - 1).name;
                    self.selection_matches(r, name, views)
                }
                _ => self.selection_deep(r, i - 1, layers - 1, views),
            },
            SelectionResult::Selected => true,
        }
    }

    /// Whether the member `m` of `t` takes the arguments of `list`: a method by its first explicit
    /// clause, a value by its type's `apply`.
    fn member_applies(&mut self, m: SymId, t: TypeId, list: &ArgList, owner: Span) -> bool {
        if self.syms.sym(m).kind == SymKind::Def {
            return self.method_applies(m, t, list, owner);
        }
        let value = self.member_value_type(m, t);
        self.value_applies(value, list, owner)
    }

    /// The type of the value member `m` seen from `t`.
    fn member_value_type(&mut self, m: SymId, t: TypeId) -> TypeId {
        let sig = self.sig_arc(m);
        let owner_ty = match self.syms.sym(m).owner {
            Owner::Class(c) => self.base_type(t, c).unwrap_or(t),
            _ => t,
        };
        let subst = self.owner_subst(owner_ty);
        self.types.subst(sig.ret, &subst)
    }

    /// The member `m` of `t` as the deeper selection compares it with its expectation, `applied`
    /// to its first explicit clause or not: a method's result past that clause, or its normalized
    /// value (`ProtoTypes.normalize`: a using clause gives way to its result, an explicit clause
    /// with parameters becomes its function type, a context function gives way to its result); a
    /// value's type, or its function's result where applied. `None` where it is not known.
    fn member_result(&mut self, m: SymId, t: TypeId, applied: bool) -> Option<TypeId> {
        if self.syms.sym(m).kind != SymKind::Def {
            let value = self.member_value_type(m, t);
            if !applied {
                return Some(value);
            }
            return self.as_function(value).map(|(_, r)| r);
        }
        let sig = self.sig_arc(m);
        let owner_ty = match self.syms.sym(m).owner {
            Owner::Class(c) => self.base_type(t, c).unwrap_or(t),
            _ => t,
        };
        let subst = self.boundary_vars(&sig, owner_ty);
        let explicit: Vec<&ClauseSig> = sig.clauses.iter().filter(|c| !c.is_using).skip(usize::from(applied)).collect();
        Some(self.clauses_result(&sig, &explicit, &subst))
    }

    /// The result of `sig` past its clauses but `explicit`, normalized as `member_result` has it:
    /// a context function's result followed, each explicit clause with parameters a function.
    fn clauses_result(&mut self, sig: &MethodSig, explicit: &[&ClauseSig], subst: &Subst) -> TypeId {
        let mut ty = self.types.subst(sig.ret, subst);
        for _ in 0..8 {
            let Some((_, result)) = self.as_context_function(ty) else { break };
            ty = result;
        }
        for clause in explicit.iter().rev() {
            if clause.params.is_empty() {
                continue;
            }
            let params: Vec<TypeId> = clause.params.iter().map(|p| self.types.subst(p.ty, subst)).collect();
            ty = self.fun_type(&params, ty);
        }
        ty
    }

    /// `isMatchedBy` of a builtin's member that the std gives as an extension (`builtin_member_like`:
    /// `Array.update`, a `String`'s JDK method) at the selection at `i`, as of a class's own
    /// member: the receiver bound to `t`, the extension's first explicit clause past the
    /// receiver's taking the arguments of `list`, and its result, past the layers they took,
    /// matching what is revealed of the member's (`result_matches`).
    fn builtin_extension_matched(&mut self, t: TypeId, i: usize, list: Option<&ArgList>, after_args: u8, views: bool) -> bool {
        let name = self.sel(i).name;
        let owner = self.sel(i).span;
        let candidates = self.builtin_member_candidates(t, name);
        for sym in candidates {
            if !self.is_builtin_member(sym, t) {
                continue;
            }
            let sig = self.sig_arc(sym);
            let ext = self.syms.sym(sym).ext_clauses as usize;
            let Some(recv) = sig.clauses.iter().take(ext).find(|c| !c.is_using).and_then(|c| c.params.first()).map(|p| p.ty) else { continue };
            let mark = self.snapshot();
            let subst = self.boundary_vars(&sig, t);
            let recv = self.types.subst(recv, &subst);
            let own: Vec<&ClauseSig> = sig.clauses.iter().skip(ext).filter(|c| !c.is_using).collect();
            let ok = self.is_sub(t, recv)
                && match list {
                    Some(l) => match own.first() {
                        Some(clause) => match self.boundary_args(clause, l) {
                            Some((args, true, _)) => {
                                let types = self.selection_arg_types(&args, owner);
                                self.applicable_to_types(clause, &args, &types, &subst) && (after_args == 0 || {
                                    let r = self.clauses_result(&sig, &own[1..], &subst);
                                    self.result_matches(r, i, after_args, views)
                                })
                            }
                            _ => false,
                        },
                        None => false,
                    },
                    None => {
                        let r = self.clauses_result(&sig, &own, &subst);
                        self.result_matches(r, i, after_args, views)
                    }
                };
            self.rollback(mark);
            if ok {
                return true;
            }
        }
        false
    }

    /// Whether the method `m`, a member of `t`, applies to the arguments of `list` by its first
    /// explicit clause, the arguments typed once for the selection at `owner`
    /// (`applicable_to_types`).
    fn method_applies(&mut self, m: SymId, t: TypeId, list: &ArgList, owner: Span) -> bool {
        let sig = self.sig_arc(m);
        let Some(clause) = sig.clauses.iter().find(|c| !c.is_using) else { return false };
        let Some((args, matched, _)) = self.boundary_args(clause, list) else { return false };
        if !matched {
            return false;
        }
        let types = self.selection_arg_types(&args, owner);
        let owner_ty = match self.syms.sym(m).owner {
            Owner::Class(c) => self.base_type(t, c).unwrap_or(t),
            _ => t,
        };
        let mark = self.snapshot();
        let subst = self.boundary_vars(&sig, owner_ty);
        let ok = self.applicable_to_types(clause, &args, &types, &subst);
        self.rollback(mark);
        ok
    }

    /// The types of the selection's arguments `args`, typed once for the selection at `owner`: a
    /// function literal without parameter types by its shape (`FunProto.typedArgs` leaves it so).
    fn selection_arg_types(&mut self, args: &[BoundaryArg], owner: Span) -> Vec<Option<TypeId>> {
        args.iter()
            .map(|a| match (a.pretyped, a.value) {
                (Some((_, ty)), _) => Some(ty),
                (None, Some(value)) => self.function_shape(value).or_else(|| self.selection_arg_type(value, owner)),
                (None, None) => None,
            })
            .collect()
    }

    /// Whether a value of type `t` applies to the arguments of `list`: a function by its
    /// parameters, another value by an accessible `apply` member that does.
    fn value_applies(&mut self, t: TypeId, list: &ArgList, owner: Span) -> bool {
        if let Some((params, _)) = self.as_function(t) {
            return self.function_takes(&params, list, owner);
        }
        let Some((sym, _)) = self.find_member(t, names::APPLY) else { return false };
        let members: Vec<SymId> = match self.syms.alternatives(sym) {
            Some(alts) => alts.to_vec(),
            None => vec![sym],
        };
        members.into_iter().any(|m| self.is_accessible(m) && self.syms.sym(m).kind == SymKind::Def && self.method_applies(m, t, list, owner))
    }

    /// Whether a function of the parameters `params` takes the positional arguments of `list`
    /// (`sig_alt_takes`).
    fn function_takes(&mut self, params: &[TypeId], list: &ArgList, owner: Span) -> bool {
        let params: Vec<Option<TypeId>> = params.iter().copied().map(Some).collect();
        self.sig_alt_takes(&params, false, list, owner)
    }

    /// One alternative of a member that has no symbol of its own (`selection_matched`): a builtin
    /// operator's overload as scala's `Int.scala` declares it (`+(x: Long): Long`), a member of
    /// `Any`, a function's `apply`, a method of the JDK's `String`. The types of its one clause's
    /// parameters, `None` where it takes no list, each `None` where it is not known, whether the
    /// last is repeated, and its result, `None` where it is not known.
    pub(super) fn sig_alt(params: Option<Vec<Option<TypeId>>>, repeated: bool, result: Option<TypeId>) -> SigAlt {
        SigAlt { params, repeated, result }
    }

    /// `normalizedCompatible` of the alternative `alt` with the selection at `i` revealed past its
    /// name (`selection_matched`): the arguments of `list`, typed once (`selection_arg_type`), fit
    /// its parameters, and its result, once the layers the arguments took are revealed, fits what
    /// is expected of the member (`result_matches`). Unapplied, a member with parameters is the
    /// function it expands to, one with an empty clause its result unless that function fits
    /// (`ProtoTypes.normalize`); applied without a list of its own, its result's `apply` takes it.
    fn sig_alt_matches(&mut self, alt: &SigAlt, i: usize, list: Option<&ArgList>, after_args: u8, views: bool) -> bool {
        let owner = self.sel(i).span;
        match (list, &alt.params) {
            (Some(l), Some(params)) => {
                self.sig_alt_takes(params, alt.repeated, l, owner)
                    && (after_args == 0 || alt.result.map_or(true, |r| self.result_matches(r, i, after_args, views)))
            }
            (Some(l), None) => {
                let Some(r) = alt.result else { return true };
                match self.as_function(r) {
                    Some((params, result)) => {
                        let params: Vec<Option<TypeId>> = params.into_iter().map(Some).collect();
                        self.sig_alt_takes(&params, false, l, owner) && (after_args == 0 || self.result_matches(result, i, after_args, views))
                    }
                    None => self.value_applies(r, l, owner),
                }
            }
            (None, params) => {
                let Some(r) = alt.result else { return true };
                match params {
                    None => self.result_matches(r, i, after_args, views),
                    Some(params) if params.is_empty() => {
                        self.result_matches(r, i, after_args, views) || {
                            let f = self.fun_type(&[], r);
                            self.result_matches(f, i, after_args, views)
                        }
                    }
                    Some(params) => match params.iter().copied().collect::<Option<Vec<TypeId>>>() {
                        Some(params) if !alt.repeated => {
                            let f = self.fun_type(&params, r);
                            self.result_matches(f, i, after_args, views)
                        }
                        _ => true,
                    },
                }
            }
        }
    }

    /// Whether the parameters `params` (`SigAlt`) take the positional arguments of `list`, typed
    /// once for the selection at `owner`, the last repeated where `repeated`; a parameter or an
    /// argument whose type is not known takes or is taken.
    fn sig_alt_takes(&mut self, params: &[Option<TypeId>], repeated: bool, list: &ArgList, owner: Span) -> bool {
        let fixed = if repeated { params.len() - 1 } else { params.len() };
        if list.args.len() < fixed || (!repeated && list.args.len() > fixed) {
            return false;
        }
        let exprs: Vec<Option<ExprId>> = list.args.iter().map(|a| match *a {
            ArgSrc::Ast(e) | ArgSrc::Hoisted(e) => Some(e),
            _ => None,
        }).collect();
        for (k, e) in exprs.into_iter().enumerate() {
            let Some(e) = e else { return false };
            let Some(p) = params[k.min(params.len() - 1)] else { continue };
            let ty = match self.function_shape(e) {
                Some(shape) => shape,
                None => match self.selection_arg_type(e, owner) {
                    Some(ty) => ty,
                    None => continue,
                },
            };
            let mark = self.snapshot();
            let fits = self.fits_boundary(ty, p);
            self.rollback(mark);
            if !fits {
                return false;
            }
        }
        true
    }

    /// The overloads of the builtin operator `name` of the primitive or `String` type `t`, as
    /// scala's `Int.scala` and its siblings declare them: a number's arithmetic one per numeric
    /// operand type, the result the wider of the two and `Int` at least, its `+` a `String`'s too,
    /// its comparisons a `Boolean`, its bit operators one per integral operand type, its shifts an
    /// `Int`'s and a `Long`'s, the result the receiver's promoted type, its prefix operators and
    /// conversions none; a `Boolean`'s operators a `Boolean`'s; a `String`'s `+` anything, its
    /// comparisons a `String`.
    fn builtin_operator_alternatives(&mut self, t: TypeId, name: Name) -> Vec<SigAlt> {
        use super::prims::{R_BYTE, R_CHAR, R_DOUBLE, R_INT, R_LONG, R_SHORT};
        let own = self.dealias(t);
        let own = self.operator_type(own);
        let own = self.dealias(own);
        let (b_bool, b_string) = (self.b.t_boolean, self.b.t_string);
        let unary = |r: TypeId| vec![Self::sig_alt(None, false, Some(r))];
        let binary = |p: TypeId, r: TypeId| Self::sig_alt(Some(vec![Some(p)]), false, Some(r));
        if own == b_bool {
            return match name {
                names::UNARY_BANG => unary(b_bool),
                _ => vec![binary(b_bool, b_bool)],
            };
        }
        if own == b_string {
            return match name {
                names::PLUS => vec![binary(ANY, b_string)],
                _ => vec![binary(b_string, b_bool)],
            };
        }
        let Some(rank) = self.is_numeric(own) else { return Vec::new() };
        let promoted = self.rank_type(rank.max(R_INT));
        let text = self.name_ref(name).to_string();
        if let Some(at) = super::prims::NUMERIC_CONVERSIONS.iter().position(|&c| c == text) {
            let to = [R_BYTE, R_SHORT, R_CHAR, R_INT, R_LONG, super::prims::R_FLOAT, R_DOUBLE][at];
            return unary(self.rank_type(to));
        }
        let ranks = |upto: u8| (R_BYTE..=upto).collect::<Vec<u8>>();
        match name {
            names::UNARY_MINUS | names::UNARY_PLUS | names::UNARY_TILDE => unary(promoted),
            names::LT | names::LE | names::GT | names::GE => ranks(R_DOUBLE).into_iter().map(|p| binary(self.rank_type(p), b_bool)).collect(),
            names::AMP | names::BAR | names::CARET => ranks(R_LONG).into_iter().map(|p| binary(self.rank_type(p), self.rank_type(rank.max(p).max(R_INT)))).collect(),
            names::SHL | names::SHR | names::USHR => [R_INT, R_LONG].into_iter().map(|p| binary(self.rank_type(p), promoted)).collect(),
            _ => {
                let mut alts: Vec<SigAlt> = ranks(R_DOUBLE).into_iter().map(|p| binary(self.rank_type(p), self.rank_type(rank.max(p).max(R_INT)))).collect();
                if name == names::PLUS {
                    alts.push(binary(b_string, b_string));
                }
                alts
            }
        }
    }

    /// The members of `Any` and `AnyRef` (`universal_selection`) as `Any.scala` and `Object`
    /// declare them: `==`, `!=` and `equals` take an `Any`, `eq` and `ne` an `AnyRef`, each a
    /// `Boolean`; `hashCode()` and `##` an `Int`, `toString()` a `String`; the others, whose
    /// signatures take a type argument (`isInstanceOf`, `asInstanceOf`, `synchronized`) or name
    /// a class (`getClass`), of a result not known.
    fn universal_alternatives(&mut self, name: Name) -> Vec<SigAlt> {
        let (b_bool, b_int, b_string, any_ref) = (self.b.t_boolean, self.b.t_int, self.b.t_string, self.b.t_any_ref);
        vec![match self.name_ref(name) {
            "==" | "!=" | "equals" => Self::sig_alt(Some(vec![Some(ANY)]), false, Some(b_bool)),
            "eq" | "ne" => Self::sig_alt(Some(vec![Some(any_ref)]), false, Some(b_bool)),
            "hashCode" => Self::sig_alt(Some(Vec::new()), false, Some(b_int)),
            "##" => Self::sig_alt(None, false, Some(b_int)),
            "toString" => Self::sig_alt(Some(Vec::new()), false, Some(b_string)),
            "getClass" => Self::sig_alt(Some(Vec::new()), false, None),
            _ => Self::sig_alt(None, false, None),
        }]
    }

    /// The members of the function type `t` that `FunctionN` declares and teq's typer gives
    /// (`function_class_member`): `apply` of its parameters and result, `tupled` a function of
    /// their tuple, `curried` a function per parameter; `None` for `andThen` and `compose`, which
    /// std/core.scala gives as extensions (`builtin_extension_matched`).
    fn function_member_alternatives(&mut self, t: TypeId, name: Name) -> Option<Vec<SigAlt>> {
        let (params, result) = self.as_function(t)?;
        Some(vec![match name {
            names::APPLY => Self::sig_alt(Some(params.into_iter().map(Some).collect()), false, Some(result)),
            names::TUPLED => {
                let tuple = self.tuple_type(&params);
                Self::sig_alt(None, false, Some(self.fun_type(&[tuple], result)))
            }
            names::CURRIED => {
                let curried = params.iter().rev().fold(result, |r, &p| self.fun_type(&[p], r));
                Self::sig_alt(None, false, Some(curried))
            }
            _ => return None,
        }])
    }

    /// The public instance methods `name` of the JDK's `java.lang.String`
    /// (`loader::javaclass::jdk_string_class`) as a selection on a `String` tests them (`SigAlt`):
    /// dotty selects on the receiver's own class and checks its alternatives (Typer.scala 1081,
    /// ProtoTypes.scala 262), whose signatures are the JDK's (`contains(CharSequence)`, `trim()`),
    /// where the std's implementations may take wider ones (`contains(String | Char)`,
    /// `trim(Unit = ())`). A type the JDK names that teq does not know is taken as not known.
    /// `None` where the class declares no such method.
    fn jdk_string_alternatives(&mut self, name: Name) -> Option<Vec<SigAlt>> {
        use crate::classfile::{ACC_PUBLIC, ACC_STATIC, ACC_VARARGS};
        let cf = super::loader::javaclass::jdk_string_class()?;
        let text = self.name_ref(name).to_string();
        let mut alts = Vec::new();
        for m in cf.methods.iter().filter(|m| m.name == text && m.access & ACC_PUBLIC != 0 && m.access & ACC_STATIC == 0) {
            let Ok(sig) = crate::classfile::sig::method_sig(m) else { continue };
            let repeated = m.access & ACC_VARARGS != 0 && !sig.params.is_empty();
            let count = sig.params.len();
            let params = sig
                .params
                .iter()
                .enumerate()
                .map(|(k, p)| match (repeated && k + 1 == count, p) {
                    (true, crate::classfile::sig::JType::Array(elem)) => self.jdk_type(elem),
                    _ => self.jdk_type(p),
                })
                .collect();
            let result = self.jdk_type(&sig.ret);
            alts.push(Self::sig_alt(Some(params), repeated, result));
        }
        (!alts.is_empty()).then_some(alts)
    }

    /// The type of a JDK signature's type `j` as Scala reads a Java member's (`map_jtype`): a
    /// primitive the builtin, `void` `Unit`, `String` the builtin, `Object` `Any`, an array an
    /// `Array`, another class without type arguments the one of its path; `None` for a type
    /// variable, a class with arguments or one teq does not have.
    fn jdk_type(&mut self, j: &crate::classfile::sig::JType) -> Option<TypeId> {
        use crate::classfile::sig::{JType, Prim};
        Some(match j {
            JType::Prim(p) => match p {
                Prim::Byte => self.b.t_byte,
                Prim::Char => self.b.t_char,
                Prim::Double => self.b.t_double,
                Prim::Float => self.b.t_float,
                Prim::Int => self.b.t_int,
                Prim::Long => self.b.t_long,
                Prim::Short => self.b.t_short,
                Prim::Boolean => self.b.t_boolean,
            },
            JType::Void => self.b.t_unit,
            JType::Array(elem) => {
                let elem = self.jdk_type(elem)?;
                self.types.class(self.b.array, &[elem])
            }
            JType::Class(c) if c.args.is_empty() && c.outer.is_none() => match c.name.as_str() {
                "java/lang/String" => self.b.t_string,
                "java/lang/Object" => ANY,
                path => {
                    let parts: Vec<&str> = path.split('/').collect();
                    let class = self.class_at(&parts)?;
                    if !self.syms.class(class).tparams.is_empty() {
                        return None;
                    }
                    self.types.class(class, &[])
                }
            },
            _ => return None,
        })
    }

    /// Whether `name` is a member every value of type `t` has: `Any`'s, and `AnyRef`'s but on a
    /// primitive, which teq gives through `universal_member` rather than as members a lookup finds
    /// (`names::ANY_MEMBERS`); dotty's `member` finds them on any type through `Any` and `Object`.
    fn universal_selection(&mut self, t: TypeId, name: Name) -> bool {
        let text = self.interner.get(name);
        let any_ref = matches!(text, "eq" | "ne" | "synchronized");
        if !crate::names::ANY_MEMBERS.contains(&text) {
            return false;
        }
        if !any_ref {
            return true;
        }
        let t = self.dealias(t);
        !self.is_primitive(t)
    }

    /// Whether `name` is a member scala-library's `FunctionN` declares beside `Any`'s, for the
    /// function type `t`: `apply`, `Function1`'s `andThen` and `compose`, the `tupled` and
    /// `curried` of two to 22 parameters. teq's std gives a function value those as builtins of
    /// the typer (`apply`, `tupled`, `curried`, `apply_member`) and as extensions (`andThen` and
    /// `compose`, std/core.scala), a function being a JavaScript function, so no member lookup
    /// finds them.
    fn function_class_member(&mut self, t: TypeId, name: Name) -> bool {
        let Some((params, _)) = self.as_function(t) else { return false };
        match name {
            names::APPLY => true,
            names::TUPLED | names::CURRIED => (2..=22).contains(&params.len()),
            _ => params.len() == 1 && matches!(self.name_ref(name), "andThen" | "compose"),
        }
    }

    /// The deeper selection's typing of the operand `e` (`SelectionTyped`), if it typed it.
    pub(super) fn selection_typed_of(&self, e: ExprId) -> Option<Option<(TExprId, TypeId)>> {
        if self.selection_typed.is_empty() {
            return None;
        }
        let file = self.env.file;
        self.selection_typed.iter().rev().find(|s| s.file == file && s.expr == e).map(|s| s.typed)
    }

    /// The type the candidates of the deeper selection test the operand `e` by, typed once as an
    /// overloaded application types its arguments before an alternative is chosen
    /// (`pretype_value`: a written literal by its literal type) and kept for the member's
    /// application (`SelectionTyped`); `None` where the typing reported an error, which is
    /// retracted, or where the operand is a name of an overloaded method, left for the member's
    /// application.
    fn selection_arg_type(&mut self, e: ExprId, owner: Span) -> Option<TypeId> {
        if let Some(s) = self.selection_typed.iter().rev().find(|s| s.file == self.env.file && s.expr == e) {
            return s.typed.map(|(_, ty)| s.literal.unwrap_or(ty));
        }
        let hoisted = self.hoisted.len();
        let m = self.attempt();
        let pretyped = self.pretype_value(e, None, false);
        let (typed, literal) = match pretyped {
            Some((te, ty, literal)) if !self.attempt_failed(&m) && !self.types.contains_error(ty) => {
                self.close(m);
                (Some((te, ty)), literal)
            }
            _ => {
                self.retract(m);
                self.hoisted.truncate(hoisted);
                (None, None)
            }
        };
        self.selection_typed.push(SelectionTyped { file: self.env.file, owner, expr: e, typed, literal });
        typed.map(|(_, ty)| literal.unwrap_or(ty))
    }

    /// The member of the selection on the qualifier at `owner` is applied: the operands its
    /// deeper selection typed are gone.
    #[inline]
    pub(super) fn drop_selection_typed(&mut self, owner: Option<Span>) {
        if let (Some(owner), false) = (owner, self.selection_typed.is_empty()) {
            let file = self.env.file;
            self.selection_typed.retain(|t| t.file != file || t.owner != owner);
        }
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
        if spread {
            self.spread_was_array = self.array_element(ty).is_some();
            if self.spread_was_array {
                self.prog.mark_spread(te);
            }
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
        match self.rank_extension_givens(&in_scope, name, recv, recv_ty, targs, lists, expected, span) {
            Ranked::One(i, probe) => {
                let ((given, given_ty), _) = in_scope[i];
                if let Some(call) = self.given_extension_for(given, given_ty, name, recv, recv_ty, lists, expected, span, probe) {
                    self.attribute(given.0);
                    self.use_sym(given.0, Span::default());
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
        let from_given = match self.rank_extension_givens(&beyond, name, recv, recv_ty, targs, lists, expected, span) {
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
                self.use_sym(given, Span::default());
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
        targs: Option<ListRef>,
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
            std::cmp::Ordering::Equal => w.compare_given_refs(cands[a].0 .0, cands[b].0 .0),
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
            // `f(qual)` applies the using clauses before the extension's own parameters
            // (`tryExtension`): one without an instance passes the given over. Type arguments the
            // call writes for the method's own type parameters leave its clauses to the
            // application (`pf.asExprOf[X]`, whose `using Type[X]` the `[X]` fixes).
            if !self.prefix_resolves(&call, recv_ty, targs, lists, span, false) {
                continue;
            }
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
                        self.ext_modules.push((s, GivenScope::Module(m)));
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

    /// Whether the extension `e` may take `recv_ty` as its receiver: a top-level one whose receiver
    /// clause takes it (`receiver_fits`), any other, whose receiver its owner's instance may fix.
    fn extension_may_take(&mut self, e: SymId, recv: TExprId, recv_ty: TypeId) -> bool {
        if !matches!(self.syms.sym(e).owner, Owner::Package(_)) {
            return true;
        }
        let call = MethodCall { recv: None, sym: e, owner_subst: Vec::new(), ext_recv: Some((recv, recv_ty)), prefix: None, refined: None };
        self.receiver_fits(&call, recv_ty)
    }

    /// Whether the extension `call`'s receiver clause takes `recv_ty`: what dotty's `tryExtension`
    /// types as `f(qual)` (Typer.scala 4285), which selects the extension under a member's retry.
    /// Nothing else is resolved ahead of the application.
    fn receiver_fits(&mut self, call: &MethodCall, recv_ty: TypeId) -> bool {
        let mark = self.snapshot();
        let fits = match self.receiver_param_type(call, true) {
            Some(pty) => self.is_sub(recv_ty, pty),
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
        self.prefix_resolves(call, recv_ty, targs, lists, span, true)
    }

    /// `prefix_holds`; with `keep` the givens it resolved are the application's, else dropped
    /// (a given's extension weighed among others, which is applied later if selected).
    fn prefix_resolves(&mut self, call: &MethodCall, recv_ty: TypeId, targs: Option<ListRef>, lists: &Option<Vec<ArgList>>, span: Span, keep: bool) -> bool {
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
        if hold && keep {
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
        let t = self.view_scope_type(t);
        for found in self.implicit_scope_objects(t) {
            let m = match found {
                GivenScope::Module(m) => m,
                // A path of the scope: the extensions of its class and bases, read on it (dotty's
                // `OfTypeImplicits.refs` of the `addPath` references, `filterMatching`).
                GivenScope::Path(p) if self.scope_accessible(found) => {
                    let under = self.path_underlying(p);
                    let Some(c) = self.class_of(under) else { continue };
                    self.complete_class(c);
                    let start = out.len();
                    let bases: Vec<ClassId> = self.syms.class(c).base_types.iter().map(|&(b, _)| b).collect();
                    for b in bases {
                        self.complete_class(b);
                        out.extend(self.syms.class(b).extensions.iter().copied().filter(|&s| self.syms.sym(s).name == name));
                    }
                    let mut k = start;
                    while k < out.len() {
                        if self.is_given_accessible(out[k]) {
                            k += 1;
                        } else {
                            out.remove(k);
                        }
                    }
                    for &s in &out[start..] {
                        self.ext_modules.push((s, GivenScope::Path(p)));
                    }
                    continue;
                }
                _ => continue,
            };
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
                        self.ext_modules.push((s, GivenScope::Module(m)));
                    }
                }
            }
        }
        out
    }

    /// The substitution of the owner `c`'s parameters and the prefix the signature is seen from,
    /// of an extension the implicit scope reaches through `scope`: an object inheriting it, or a
    /// path whose class has it (`o`'s `extension (i: I)` for an `o.I`).
    pub(super) fn scope_extension_site(&mut self, scope: GivenScope, c: ClassId) -> (Subst, Option<TypeId>) {
        match scope {
            GivenScope::Module(m) => (self.trait_member_subst(super::exports::TraitMemberSite::Module(m), c), self.inherited_extension_prefix(Some(m))),
            GivenScope::Path(p) => (self.trait_member_subst(super::exports::TraitMemberSite::Path(p), c), Some(p)),
            _ => (Vec::new(), None),
        }
    }

    /// The object of the implicit scope the `i`-th of `candidates`, an extension inherited from
    /// a trait, was found in. One method reached through two objects (`Html` and `Svg` of one
    /// trait) is two candidates in the order the search pushed their objects, which are the last
    /// entries for it.
    fn extension_module_at(&self, candidates: &[SymId], i: usize) -> Option<GivenScope> {
        let i = i % candidates.len();
        let sym = candidates[i];
        let total = candidates.iter().filter(|&&s| s == sym).count();
        let before = candidates[..i].iter().filter(|&&s| s == sym).count();
        let found: Vec<GivenScope> = self.ext_modules.iter().filter(|&&(s, _)| s == sym).map(|&(_, m)| m).collect();
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
                        Some(GivenScope::Module(m)) => super::exports::TraitMemberSite::Module(m),
                        Some(GivenScope::Path(p)) => super::exports::TraitMemberSite::Path(p),
                        _ => self.trait_member_site(sym, c),
                    };
                    owner_subst = self.trait_member_subst(site, c);
                    prefix = match module {
                        Some(GivenScope::Path(p)) => Some(p),
                        _ => self.inherited_extension_prefix(module.and_then(GivenScope::module)),
                    };
                    let receiver = self.trait_member_receiver(site);
                    if prefix.is_none() {
                        prefix = self.lexical_extension_prefix(sym, site, Some(receiver));
                    }
                    Some(receiver)
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
            let call = MethodCall { recv: owner, sym, owner_subst, ext_recv: Some((recv, recv_ty)), prefix, refined: None };
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
            let prefix = match module {
                Some(GivenScope::Path(p)) => Some(p),
                _ => self.inherited_extension_prefix(module.and_then(GivenScope::module)),
            };
            let call = MethodCall { recv: None, sym, owner_subst: Vec::new(), ext_recv: None, prefix, refined: None };
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
        let bases: Vec<(ClassId, TypeId)> = self.met_base_types(gc);
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
                let call = MethodCall { recv: Some(instance), sym: ext, owner_subst, ext_recv: Some((recv, recv_ty)), prefix: None, refined: None };
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
        let modules: Vec<Option<GivenScope>> = (0..from_scope.len()).map(|i| self.extension_module_at(&from_scope, i)).collect();
        let candidates: Vec<(SymId, Option<GivenScope>)> = lexical.into_iter().map(|s| (s, None)).chain(from_scope.into_iter().zip(modules)).collect();
        self.ext_modules.truncate(mark);
        let declared = candidates.into_iter().any(|(sym, module)| {
            let (owner_subst, prefix) = match (module, self.syms.sym(sym).owner) {
                (Some(scope), Owner::Class(c)) => self.scope_extension_site(scope, c),
                (Some(GivenScope::Module(m)), _) => (Vec::new(), self.inherited_extension_prefix(Some(m))),
                _ => (Vec::new(), None),
            };
            let call = MethodCall { recv: None, sym, owner_subst, ext_recv: None, prefix, refined: None };
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
            let call = MethodCall { recv: None, sym, owner_subst: Vec::new(), ext_recv: None, prefix: None, refined: None };
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
    /// most candidates without instantiating them; but a `*:` chain, which a tuple class conforms
    /// to by the subtype check's rule and not by its base classes.
    fn receiver_class_rules_out(&mut self, sym: SymId, recv_ty: TypeId) -> bool {
        let ext_clauses = self.syms.sym(sym).ext_clauses as usize;
        let Some(recv_param) = self.sig_of(sym).clauses.iter().take(ext_clauses).find(|c| !c.is_using).map(|c| c.params[0].ty) else { return false };
        let Type::Class(wanted, _) = self.types.get(recv_param) else { return false };
        if Some(wanted) == self.b.cons_tuple {
            return false;
        }
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

    /// The prefix an extension a trait defines is seen from where it is found lexically and its
    /// signature names a path: the path of its site's receiver, `C.this` of the enclosing class
    /// that inherits it (`page` of `CtrlPage` in a subclass, whose `Status` is `C.this.Status`),
    /// the object or the value whose import brings it (`"f".in(2)` of `import dsl.*`), as dotty's
    /// lexical reference `C.this.page` or `dsl.in` is seen from its prefix (`Typer.findRef`,
    /// `TypeOps.asSeenFrom`). Completion and signature help, which build no receiver, see an
    /// enclosing class's `this` and an object's.
    pub(super) fn lexical_extension_prefix(&mut self, sym: SymId, site: super::exports::TraitMemberSite, receiver: Option<TExprId>) -> Option<TypeId> {
        use super::exports::TraitMemberSite;
        if !matches!(site, TraitMemberSite::This(Some(_)) | TraitMemberSite::Module(_) | TraitMemberSite::Value(_)) || !self.sig_names_paths(sym) {
            return None;
        }
        match (site, receiver) {
            (_, Some(receiver)) => self.path_of(receiver),
            (TraitMemberSite::This(Some(k)), None) => Some(self.this_prefix(k)),
            (TraitMemberSite::Module(m), None) => Some(self.types.class(m, &[])),
            _ => None,
        }
    }

    fn receiver_param_type(&mut self, call: &MethodCall, fresh: bool) -> Option<TypeId> {
        let sig = self.sig_arc(call.sym);
        let mut stand_in = None;
        let sig = match call.prefix {
            Some(prefix) => {
                stand_in = self.prefix_stand_in(prefix, &call.owner_subst);
                let inserted = stand_in.map_or(prefix, |(param, _)| param);
                self.sig_seen_as(sig, prefix, inserted, call.sym)
            }
            None => sig,
        };
        let ext_clauses = self.syms.sym(call.sym).ext_clauses as usize;
        let idx = sig.clauses.iter().take(ext_clauses).position(|c| !c.is_using)?;
        let mut subst = call.owner_subst.clone();
        subst.extend(stand_in.map(|(_, entry)| entry));
        if !fresh {
            return Some(self.types.subst(sig.clauses[idx].params[0].ty, &subst));
        }
        for &tp in &sig.tparams {
            let v = self.fresh_var();
            subst.push((tp, v));
        }
        let pty = self.types.subst(sig.clauses[idx].params[0].ty, &subst);
        // A receiver's type through a leading using parameter (`self: c.Term` of
        // `extension (using c: Ctx)`) is open until that given is found.
        let leading: Vec<SymId> = sig.clauses[..idx].iter().flat_map(|c| c.params.iter().map(|p| p.sym)).collect();
        Some(self.approx_paths(pty, &leading, &mut Vec::new()))
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
